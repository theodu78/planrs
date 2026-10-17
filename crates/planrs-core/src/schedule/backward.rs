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

use super::network::{Fail, NodeIx, NodeKind, pos};
use super::{Calc, Pass, transfer};
use crate::calendar::Span;
use crate::model::LinkType;
use crate::time::Moment;

/// Plafonds lus sur les arcs de lien sortants d'un nœud.
#[derive(Default)]
struct Caps {
    /// Plus petit plafond de fin (FS, FF).
    finish: Option<Moment>,
    /// Plafonds de début (SS, SF), à convertir pour une feuille.
    starts: Vec<Moment>,
}

/// Minimum d'un plafond optionnel et d'un plafond.
fn min_opt(a: Option<Moment>, b: Moment) -> Moment {
    a.map_or(b, |a| a.min(b))
}

impl Calc<'_, '_> {
    /// Passe arrière complète.
    pub(super) fn backward(&mut self, order: &[NodeIx]) {
        let has_succ = self.has_successors();
        for &v in order.iter().rev() {
            let Some(node) = self.net.node(v) else { continue };
            if node.inactive {
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
                    Ok(late) => {
                        if let Some(s) = self.state_mut(v) {
                            s.late = Some(late);
                        }
                    }
                    Err(fail) => {
                        // R-069 : dates au plus tard de repli.
                        if let Some(s) = self.state_mut(v) {
                            s.late = Some(early);
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
    /// (lien non coupé vers une tâche active), en propre ou hérité d'une
    /// récapitulative ancêtre (sauf à travers une récapitulative manuelle
    /// datée, R-064). Parents avant enfants : un seul passage.
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
                            && self
                                .net
                                .arc(a)
                                .and_then(|arc| arc.link)
                                .is_some_and(|l| !l.to_inactive)
                    })
                });
            if let Some(slot) = out.get_mut(p) {
                *slot = own || inherited;
            }
        }
        out
    }

    /// Plafonds des arcs de lien sortants du nœud `v` (R-047) : FS
    /// `h(LS_s)`, FF `h(LF_s)` en fin ; SS `h(LS_s)`, SF `h(LF_s)` en début.
    /// Les arcs coupés, vers une tâche inactive ou vers un nœud sans dates
    /// au plus tard sont ignorés.
    fn out_caps(&mut self, v: NodeIx) -> Result<Caps, Fail> {
        let mut caps = Caps::default();
        // Référence au réseau copiée : aucune copie des arcs sortants.
        let net = self.net;
        for &a in net
            .outgoing(v)
            .ok_or(Fail::Internal("arcs sortants hors bornes"))?
        {
            if net.is_cut(a) {
                continue;
            }
            let arc = *net.arc(a).ok_or(Fail::Internal("arc hors bornes"))?;
            let Some(l) = arc.link else { continue };
            if l.to_inactive {
                continue;
            }
            let Some(tl) = self.state(arc.to).and_then(|s| s.late) else {
                continue;
            };
            let lag = self.lag_minutes(a)?;
            let sigma = net.scale(l.sigma).ok_or(Fail::Internal("échelle hors bornes"))?;
            let anchor = match l.ty {
                LinkType::FinishToStart | LinkType::StartToStart => tl.start,
                LinkType::FinishToFinish | LinkType::StartToFinish => self.finish_anchor(arc.to, tl.finish),
            };
            let y = transfer::backward(*sigma, anchor, lag)?;
            match l.ty {
                LinkType::FinishToStart | LinkType::FinishToFinish => {
                    caps.finish = Some(min_opt(caps.finish, y));
                }
                LinkType::StartToStart | LinkType::StartToFinish => caps.starts.push(y),
            }
        }
        Ok(caps)
    }

    /// Dates au plus tard d'une feuille active (automatique ou manuelle) ou
    /// d'une récapitulative manuelle datée (R-064) :
    /// plafond de fin = min(fin du projet ; EF si plusieurs chemins critiques
    /// et aucun successeur ; `C(parent)` ; liens FS/FF) ; plafonds de début
    /// (liens SS/SF, `K(parent)` si même début que le parent) convertis par
    /// `forward_span(y, d).finish` ; `late = backward_span(plafond, d)`
    /// (R-030, R-031 pour un jalon, R-061).
    fn leaf_late(&mut self, v: NodeIx, early: Span, has_succ: bool) -> Result<Span, Fail> {
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let (place, duration, parent) = (node.place, node.duration, node.parent_rollup);
        let mut lf = self.finish;
        if self.project.options.multiple_critical_paths && !has_succ {
            lf = lf.min(early.finish);
        }
        let mut caps = self.out_caps(v)?;
        if let Some(pr) = parent {
            let ps = *self.state(pr).ok_or(Fail::Internal("nœud hors bornes"))?;
            if let Some(c) = ps.cap_finish {
                lf = lf.min(c);
            }
            if let (Some(k), Some(pe)) = (ps.cap_start, ps.early)
                && pe.start == early.start
            {
                caps.starts.push(k);
            }
        }
        if let Some(c) = caps.finish {
            lf = lf.min(c);
        }
        let scale = self
            .net
            .scale(place)
            .ok_or(Fail::Internal("échelle hors bornes"))?;
        for y in caps.starts {
            lf = lf.min(scale.forward_span(y, duration)?.finish);
        }
        Ok(scale.backward_span(lf, duration)?)
    }

    /// Plafonds hérités d'un agrégat (R-057) : `C(S) = min(C(parent), FS/FF
    /// sortants)` ; `K(S) = min(SS/SF sortants, K(parent) si ES_S ==
    /// ES_parent)`.
    fn rollup_caps(&mut self, v: NodeIx, early: Span) -> Result<(), Fail> {
        let parent = self
            .net
            .node(v)
            .ok_or(Fail::Internal("nœud hors bornes"))?
            .parent_rollup;
        let caps = self.out_caps(v)?;
        let mut c = caps.finish;
        let mut k = caps.starts.iter().copied().min();
        if let Some(pr) = parent {
            let ps = *self.state(pr).ok_or(Fail::Internal("nœud hors bornes"))?;
            if let Some(pc) = ps.cap_finish {
                c = Some(min_opt(c, pc));
            }
            if let (Some(pk), Some(pe)) = (ps.cap_start, ps.early)
                && pe.start == early.start
            {
                k = Some(min_opt(k, pk));
            }
        }
        let s = self.state_mut(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        s.cap_finish = c;
        s.cap_start = k;
        Ok(())
    }

    /// Porte : `LG(S)` = min des débuts au plus tard des enfants actifs
    /// (feuille : LS ; récapitulative : sa porte).
    fn gate_late(&mut self, v: NodeIx) {
        let Some(out) = self.net.outgoing(v) else { return };
        let mut lg: Option<Moment> = None;
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
            if let Some(l) = self.state(arc.to).and_then(|s| s.late) {
                lg = Some(min_opt(lg, l.start));
            }
        }
        if let (Some(lg), Some(s)) = (lg, self.state_mut(v)) {
            s.late = Some(Span {
                start: lg,
                finish: lg,
            });
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
            let Some(incoming) = self.net.incoming(rollup) else {
                continue;
            };
            for a in incoming {
                let Some(arc) = self.net.arc(a) else { continue };
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
            }
        }
    }
}
