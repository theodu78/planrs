//! Passe arrière (spécification J2 § 3.8, R-046, R-047, R-051, R-057,
//! R-061, R-069), dans l'ordre topologique inverse.
//!
//! Les plafonds passent par le transfert adjoint `h` (R-047). Un agrégat est
//! traité avant ses enfants (il les suit dans l'ordre topologique) : il leur
//! transmet ses plafonds hérités `C(S)` (fin, liens FS/FF sortants) et
//! `K(S)` (début, liens SS/SF sortants, appliqué aux seules feuilles de même
//! début, A11). Une porte est traitée après ses enfants : `LG(S)` = min des
//! débuts au plus tard de ses enfants actifs, lu par les liens FS/SS qui la
//! visent.

use super::constraint::{ConstraintRule, Mode};
use super::network::{Fail, NodeIx, NodeKind, pos};
use super::{Calc, LateDriver, Pass, PassRole, transfer};
use crate::calendar::Span;
use crate::ids::TaskUid;
use crate::model::LinkType;
use crate::time::Moment;

/// Plafonds lus sur les arcs de lien sortants d'un nœud, avec leur cause
/// (R-047).
#[derive(Default)]
struct Caps {
    /// Plus petit plafond de fin (FS, FF) ; à égalité, le premier arc.
    finish: Option<(Moment, LateDriver)>,
    /// Plafonds de début (SS, SF), à convertir pour une feuille.
    starts: Vec<(Moment, LateDriver)>,
    /// Un successeur est peu sûr (dégradé, en échec) : dates au plus tard
    /// de repli (R-069).
    unreliable: bool,
}

/// Minimum d'un plafond optionnel et d'un plafond.
fn min_opt(a: Option<Moment>, b: Moment) -> Moment {
    a.map_or(b, |a| a.min(b))
}

/// Plus petit de deux plafonds avec leur cause ; à égalité, le premier
/// garde la sienne.
fn min_cap<T: Copy>(a: Option<(Moment, T)>, b: (Moment, T)) -> (Moment, T) {
    match a {
        Some(a) if a.0 <= b.0 => a,
        _ => b,
    }
}

/// Plafond de fin en cours de calcul et sa cause (R-047) : le premier
/// minimum strict garde sa cause.
struct LateCap {
    finish: Moment,
    driver: LateDriver,
}

impl LateCap {
    /// Applique le plafond `m` de cause `driver` s'il est strictement plus
    /// petit.
    fn apply(&mut self, m: Moment, driver: LateDriver) {
        if m < self.finish {
            self.finish = m;
            self.driver = driver;
        }
    }
}

impl Calc<'_, '_> {
    /// Passe arrière complète. Depuis le début, c'est la passe secondaire
    /// (`role` = `Secondary`) : elle ne visite que les nœuds atteints par la
    /// passe avant (`reached`).
    pub(super) fn backward(&mut self, order: &[NodeIx], role: PassRole) {
        self.role = role;
        let has_succ = self.has_successors();
        for &v in order.iter().rev() {
            let Some(node) = self.net.node(v) else { continue };
            if node.inactive || !self.reached(v) {
                continue;
            }
            let (kind, uid) = (node.kind, node.uid);
            let Some(early) = self.state(v).and_then(|s| s.early) else {
                continue;
            };
            // R-064 : une récapitulative manuelle datée se traite comme une
            // feuille manuelle ; ses enfants n'héritent d'aucun plafond.
            let leaf_like = kind == NodeKind::Leaf || node.fixed.is_some();
            if leaf_like {
                let own_succ = has_succ.get(node.task).copied().unwrap_or(false);
                match self.leaf_late(v, early, own_succ) {
                    Ok((late, driver)) => {
                        if let Some(s) = self.state_mut(v) {
                            s.late = Some(late);
                            s.late_driver = driver;
                        }
                    }
                    Err(fail) => {
                        // R-069 : dates au plus tard de repli.
                        if let Some(s) = self.state_mut(v) {
                            s.late = Some(early);
                            s.late_driver = LateDriver::Fallback;
                            s.degraded = true;
                        }
                        self.report(uid, Pass::Backward, fail);
                    }
                }
            } else if kind == NodeKind::Rollup {
                if let Err(fail) = self.rollup_caps(v, early) {
                    if let Some(s) = self.state_mut(v) {
                        s.degraded = true;
                    }
                    self.report(uid, Pass::Backward, fail);
                }
            } else {
                self.gate_late(v);
            }
        }
    }

    /// R-051 : vrai, par position, si la tâche a un successeur effectif
    /// (lien non coupé vers une tâche active et planifiée), en propre ou
    /// hérité d'une récapitulative ancêtre (sauf à travers une
    /// récapitulative manuelle datée, R-064). Parents avant enfants : un seul
    /// passage, après la passe avant. Un lien vers une cible sans dates au
    /// plus tôt (non planifiable, récapitulative vide dont l'agrégat n'a pas
    /// de dates, porte en échec) a été ignoré (`link.excluded_endpoint`) :
    /// ce n'est pas un successeur, même filtre que [`Self::out_caps`].
    fn has_successors(&self) -> Vec<bool> {
        let mut out = vec![false; self.net.tasks.len()];
        for p in 0..self.net.tasks.len() {
            let Some(info) = self.net.task(p) else { continue };
            let inherited = info
                .parent
                .filter(|&par| {
                    !self
                        .net
                        .task(par)
                        .and_then(|t| t.exit)
                        .is_some_and(|r| self.net.is_fixed_rollup(r))
                })
                .and_then(|par| out.get(par).copied())
                .unwrap_or(false);
            let own = info
                .exit
                .and_then(|v| self.net.outgoing(v))
                .is_some_and(|out_arcs| {
                    out_arcs.iter().any(|&a| {
                        !self.net.is_cut(a)
                            && !self.is_dropped(a)
                            && self.net.arc(a).is_some_and(|arc| {
                                arc.link.is_some_and(|l| !l.to_inactive) && self.applied_target(arc.to)
                            })
                    })
                });
            if let Some(slot) = out.get_mut(p) {
                *slot = own || inherited;
            }
        }
        out
    }

    /// Cible d'un arc de lien effectivement appliqué en passe avant : elle a
    /// des dates au plus tôt et, pour une porte, l'agrégat jumeau aussi (une
    /// récapitulative vide n'en a pas, R-053).
    fn applied_target(&self, target: NodeIx) -> bool {
        let Some(node) = self.net.node(target) else {
            return false;
        };
        self.reached(target) && (node.kind != NodeKind::Gate || node.twin.is_some_and(|r| self.reached(r)))
    }

    /// Plafonds des arcs de lien sortants du nœud `v` (R-047) : FS
    /// `h(LS_s)`, FF `h(LF_s)` en fin ; SS `h(LS_s)`, SF `h(LF_s)` en début.
    /// Les arcs coupés, vers une tâche inactive ou vers un nœud sans dates
    /// (au plus tôt ou au plus tard) sont ignorés.
    fn out_caps(&mut self, v: NodeIx) -> Result<Caps, Fail> {
        let mut caps = Caps::default();
        // Référence au réseau copiée : aucune copie des arcs sortants.
        let net = self.net;
        for &a in net
            .outgoing(v)
            .ok_or(Fail::Internal("arcs sortants hors bornes"))?
        {
            if net.is_cut(a) || self.is_dropped(a) {
                continue;
            }
            let arc = *net.arc(a).ok_or(Fail::Internal("arc hors bornes"))?;
            let Some(l) = arc.link else { continue };
            if l.to_inactive {
                continue;
            }
            caps.unreliable |= self.unreliable(arc.to);
            // Cible sans dates au plus tard, ou porte dont la borne n'a pas pu
            // se calculer en passe avant (R-069) : le lien n'a pas été
            // appliqué, il ne plafonne rien.
            let Some(tl) = self
                .state(arc.to)
                .filter(|_| self.reached(arc.to))
                .and_then(|s| s.late)
            else {
                continue;
            };
            let lag = self.lag_minutes(a)?;
            let sigma = net.scale(l.sigma).ok_or(Fail::Internal("échelle hors bornes"))?;
            let anchor = match l.ty {
                LinkType::FinishToStart | LinkType::StartToStart => tl.start,
                LinkType::FinishToFinish | LinkType::StartToFinish => self.finish_anchor(arc.to, tl.finish),
            };
            let y = transfer::backward(*sigma, anchor, lag, self.reads_raw(arc.to, l.sigma))?;
            let driver = LateDriver::Successor {
                successor: l.succ,
                link: l.ty,
            };
            match l.ty {
                LinkType::FinishToStart | LinkType::FinishToFinish => {
                    caps.finish = Some(min_cap(caps.finish, (y, driver)));
                }
                LinkType::StartToStart | LinkType::StartToFinish => caps.starts.push((y, driver)),
            }
        }
        Ok(caps)
    }

    /// Dates au plus tard d'une feuille active (automatique ou manuelle) ou
    /// d'une récapitulative manuelle datée (R-064), et leur cause :
    /// plafond de fin = min(fin du projet ; EF si plusieurs chemins critiques
    /// et aucun successeur ; `C(parent)` ; liens FS/FF) ; plafonds de début
    /// (`K(parent)` si `k_applies`, liens SS/SF) convertis par
    /// `forward_span(y, d).finish` ; `late = backward_span(plafond, d)`
    /// (R-030, R-031 pour un jalon, R-061). Le premier minimum strict, dans
    /// cet ordre, donne la cause (`LateDriver`, R-047).
    fn leaf_late(&mut self, v: NodeIx, early: Span, has_succ: bool) -> Result<(Span, LateDriver), Fail> {
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let (place, duration, parent, constraint, deadline) = (
            node.place,
            node.duration,
            node.parent_rollup,
            node.constraint,
            node.deadline,
        );
        // R-097, R-098 : une feuille épinglée (MSO, MFO, option active) a
        // ses dates au plus tard sur l'ancrage, sans plafond : une borne
        // violée s'exprime par la marge négative de l'amont.
        if let Some(c) = constraint
            && let ConstraintRule::Pin(_) = c.rule
        {
            return Ok((c.span()?, LateDriver::Constraint { kind: c.kind }));
        }
        let scale = *self
            .net
            .scale(place)
            .ok_or(Fail::Internal("échelle hors bornes"))?;
        // Plafond de début `y` : la tâche doit commencer au plus tard en `y`
        // en brut ; `y` peut tomber hors du temps ouvré de P(t) (décalage
        // écoulé, successeur sur un autre calendrier, K(S)) : dernier début
        // plaçable avant la conversion (R-047).
        let start_cap = |y: Moment| -> Result<Moment, Fail> {
            let x = scale.latest_start(y, duration)?;
            Ok(scale.forward_span(x, duration)?.finish)
        };
        let mut lf = LateCap {
            finish: self.finish,
            driver: LateDriver::ProjectFinish,
        };
        if self.project.options.multiple_critical_paths && !has_succ {
            lf.apply(early.finish, LateDriver::MultipleCriticalPaths);
        }
        let mut caps = self.out_caps(v)?;
        if let Some(pr) = parent {
            let ps = *self.state(pr).ok_or(Fail::Internal("nœud hors bornes"))?;
            if let Some((c, summary)) = ps.cap_finish {
                lf.apply(c, LateDriver::SummaryCap { summary });
            }
            if let Some((k, summary)) = ps.cap_start
                && self.k_applies(ps.early.map(|e| e.start), early.start)
            {
                lf.apply(start_cap(k)?, LateDriver::SummaryCap { summary });
            }
            caps.unreliable |= ps.late_degraded && !self.net.is_fixed_rollup(pr);
        }
        if caps.unreliable
            && let Some(s) = self.state_mut(v)
        {
            s.late_degraded = true;
        }
        if let Some((c, driver)) = caps.finish {
            lf.apply(c, driver);
        }
        for (y, driver) in caps.starts {
            lf.apply(start_cap(y)?, driver);
        }
        // Contrainte côté plafond (R-095, R-096) : `A.finish`, où `A` est
        // l'ancrage R-090 (pour SNLT, `A.finish = forward_span(s*, d).finish`,
        // le plafond de fin d'un début au plus tard en `s*`).
        if let Some(c) = constraint
            && self.caps_late(c.rule)
        {
            lf.apply(c.span()?.finish, LateDriver::Constraint { kind: c.kind });
        }
        // R-102 : échéance, plafond brut (comme les autres plafonds, avant
        // `backward_span`), propagé à l'amont par les liens.
        if let Some(d) = deadline {
            lf.apply(d, LateDriver::Deadline);
        }
        Ok((scale.backward_span(lf.finish, duration)?, lf.driver))
    }

    /// La règle `rule` plafonne la fin au plus tard dans la passe arrière
    /// secondaire depuis le début : SNLT, FNLT (plafond, ou serrage si
    /// l'option est active) ; MSO, MFO option inactive (fenêtre).
    fn caps_late(&self, rule: ConstraintRule) -> bool {
        match rule {
            ConstraintRule::Cap(_) | ConstraintRule::Window(_) => true,
            ConstraintRule::Clamped(_) => self.mode == Mode::FromStart,
            ConstraintRule::None
            | ConstraintRule::Opposite
            | ConstraintRule::Floor(_)
            | ConstraintRule::Pin(_) => false,
        }
    }

    /// Plafonds hérités d'un agrégat (R-057) : `C(S) = min(C(parent), FS/FF
    /// sortants, FNLT et échéance de la récapitulative (R-103))` ; `K(S) = min(SS/SF sortants, K(parent) si ES_S ==
    /// ES_parent)`.
    fn rollup_caps(&mut self, v: NodeIx, early: Span) -> Result<(), Fail> {
        let parent = self
            .net
            .node(v)
            .ok_or(Fail::Internal("nœud hors bornes"))?
            .parent_rollup;
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let (uid, constraint, deadline) = (node.uid, node.constraint, node.deadline);
        let caps = self.out_caps(v)?;
        let mut c = caps.finish.map(|(m, _)| (m, uid));
        // R-103 : FNLT de la récapitulative (`f*` sur son calendrier) et son
        // échéance (brute) contribuent à `C(S)`.
        if let Some(nc) = constraint
            && let ConstraintRule::Cap(_) = nc.rule
        {
            c = Some(min_cap(c, (nc.span()?.finish, uid)));
        }
        if let Some(d) = deadline {
            c = Some(min_cap(c, (d, uid)));
        }
        let mut k: Option<(Moment, TaskUid)> = None;
        for &(y, _) in &caps.starts {
            k = Some(min_cap(k, (y, uid)));
        }
        let mut unreliable = caps.unreliable;
        if let Some(pr) = parent {
            let ps = *self.state(pr).ok_or(Fail::Internal("nœud hors bornes"))?;
            unreliable |= ps.late_degraded && !self.net.is_fixed_rollup(pr);
            if let Some(pc) = ps.cap_finish {
                c = Some(min_cap(c, pc));
            }
            if let Some(pk) = ps.cap_start
                && self.k_applies(ps.early.map(|e| e.start), early.start)
            {
                k = Some(min_cap(k, pk));
            }
        }
        let s = self.state_mut(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        s.cap_finish = c;
        s.cap_start = k;
        s.late_degraded |= unreliable;
        Ok(())
    }

    /// Porte : `LG(S)` = min des débuts au plus tard des enfants actifs
    /// (feuille : LS ; récapitulative : sa porte).
    fn gate_late(&mut self, v: NodeIx) {
        let Some(out) = self.net.outgoing(v) else { return };
        let mut lg: Option<Moment> = None;
        let mut unreliable = false;
        for &a in out {
            let Some(arc) = self.net.arc(a) else { continue };
            if arc.link.is_some() {
                continue;
            }
            let Some(child) = self.net.node(arc.to) else {
                continue;
            };
            if child.inactive {
                continue;
            }
            unreliable |= self.unreliable(arc.to);
            if let Some(l) = self.state(arc.to).and_then(|s| s.late) {
                lg = Some(min_opt(lg, l.start));
            }
        }
        if let Some(s) = self.state_mut(v) {
            s.late_degraded |= unreliable;
            if let Some(lg) = lg {
                s.late = Some(Span {
                    start: lg,
                    finish: lg,
                });
            }
        }
    }

    /// Dates au plus tard des récapitulatives actives (R-060) : LS = min LS,
    /// LF = max LF des enfants actifs, des plus intérieures aux plus
    /// extérieures (positions décroissantes). Une récapitulative manuelle
    /// datée garde les siennes, calculées comme pour une feuille (R-064).
    pub(super) fn summary_late(&mut self) {
        for p in (0..self.net.tasks.len()).rev() {
            let Some(info) = self.net.task(p) else { continue };
            if info.role != super::network::Role::Summary || info.inactive() {
                continue;
            }
            let Some(rollup) = info.exit else { continue };
            if self.net.is_fixed_rollup(rollup) {
                continue;
            }
            let mut late: Option<Span> = None;
            let mut unreliable = false;
            let Some(incoming) = self.net.incoming(rollup) else {
                continue;
            };
            for a in incoming {
                let Some(arc) = self.net.arc(a) else { continue };
                unreliable |= self.state(arc.from).is_some_and(|s| s.late_degraded);
                if let Some(l) = self.state(arc.from).and_then(|s| s.late) {
                    late = Some(match late {
                        None => l,
                        Some(s) => Span {
                            start: s.start.min(l.start),
                            finish: s.finish.max(l.finish),
                        },
                    });
                }
            }
            if let Some(s) = pos(rollup).and_then(|i| self.st.get_mut(i)) {
                s.late = late;
                s.late_degraded |= unreliable;
            }
        }
    }
}
