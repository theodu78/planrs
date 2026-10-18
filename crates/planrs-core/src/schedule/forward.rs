//! Passe avant (spécification J2 § 3.7, R-036 à R-045, R-062, R-063,
//! R-069), dans l'ordre topologique.
//!
//! Chaque arc entrant donne une borne candidate ; la plus grande en
//! `Moment` brut gagne, et à égalité la première atteinte reste pilote
//! (R-044 : début du projet, porte du parent, liens dans l'ordre de
//! stockage). Le plancher d'une contrainte est évalué après eux, sur le
//! début placé : il ne pilote que s'il place la tâche strictement plus tard
//! (une borne de lien brute, par exemple mar 17:00, et un plancher normalisé
//! mer 08:00 donnent le même début : le lien reste pilote).
//!
//! Contraintes (R-090 à R-103) : les planchers (SNET, FNET, MSO, MFO)
//! s'ajoutent aux bornes, quelle que soit l'option ; avec l'option « toujours
//! respecter les dates de contrainte », MSO et MFO, et SNLT ou FNLT quand ils
//! mordent, épinglent la tâche sur son placement de référence, et chaque
//! lien violé est signalé (`link.not_honored_constraint`) ; sans l'option,
//! un plafond violé donne `task.constraint_conflict` (la passe arrière en
//! fait une marge négative).

use super::network::{DateConstraint, Fail, NodeIx, NodeKind};
use super::{Calc, Driver, Pass, transfer};
use crate::calendar::Span;
use crate::diagnostic::Diagnostic;
use crate::model::{ConstraintKind, LinkType};
use crate::time::Moment;

/// Bornes d'un nœud : plus grande borne de début et son pilote, drapeau
/// `degraded` transmis par les arcs entrants.
struct Bounds {
    start: Moment,
    driver: Driver,
    degraded: bool,
    /// Placement épinglé par une contrainte : une borne de porte (arc de
    /// hiérarchie) le dépasse (R-101).
    gate_violated: bool,
}

/// Nature d'un placement imposé, vérifié contre les bornes sans être
/// déplacé par elles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PinKind {
    /// Dates saisies d'une tâche manuelle (R-063) : `link.manual_not_honored`.
    Manual,
    /// Contrainte respectée (R-099) : `link.not_honored_constraint`.
    Constraint,
}

/// Placement imposé d'une feuille.
#[derive(Clone, Copy, Debug)]
pub(super) struct Pin {
    /// Dates imposées.
    pub span: Span,
    /// Nature.
    pub kind: PinKind,
}

/// Une borne candidate.
enum Candidate {
    /// Borne de début (porte, FS, SS).
    Start(Moment),
    /// Borne de fin (FF, SF), à convertir en début (R-039).
    Finish(Moment),
}

impl Calc<'_, '_> {
    /// Passe avant complète, puis fin du projet (R-045).
    pub(super) fn forward(&mut self, order: &[NodeIx]) {
        for &v in order {
            let Some(node) = self.net.node(v) else {
                self.diags.push(Diagnostic::ScheduleInternal {
                    detail: "nœud hors bornes en passe avant".to_owned(),
                });
                continue;
            };
            let (kind, uid) = (node.kind, node.uid);
            let result = match kind {
                NodeKind::Gate => self.gate(v),
                NodeKind::Leaf => self.leaf(v),
                NodeKind::Rollup => self.rollup(v),
            };
            if let Err(fail) = result {
                if let Some(s) = self.state_mut(v) {
                    s.early = None;
                    s.failed = true;
                }
                self.report(uid, Pass::Forward, fail);
            }
        }
        // R-045 : fin du projet = max des fins au plus tôt des feuilles
        // actives planifiées (manuelles comprises, inactives exclues) et des
        // récapitulatives manuelles datées (R-064).
        let mut finish = self.start;
        for (node, s) in self.net.nodes.iter().zip(&self.st) {
            if (node.kind == NodeKind::Leaf || node.fixed.is_some())
                && !node.inactive
                && let Some(e) = s.early
            {
                finish = finish.max(e.finish);
            }
        }
        self.finish = finish;
    }

    /// Bornes de début du nœud `v` (R-036, R-044). Pour une feuille au
    /// placement imposé (`pin`), aucune borne ne la déplace et aucune
    /// conversion n'est faite : chaque lien violé donne
    /// `link.manual_not_honored` (manuelle, R-063) ou
    /// `link.not_honored_constraint` (contrainte respectée, R-099) ; une
    /// porte violée est rendue dans `gate_violated` (R-101).
    fn bounds(&mut self, v: NodeIx, pin: Option<Pin>) -> Result<Bounds, Fail> {
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let (place, duration, kind) = (node.place, node.duration, node.kind);
        let mut b = Bounds {
            start: self.start,
            driver: Driver::ProjectStart,
            degraded: false,
            gate_violated: false,
        };
        let incoming = self
            .net
            .incoming(v)
            .ok_or(Fail::Internal("arcs entrants hors bornes"))?;
        for a in incoming {
            if self.net.is_cut(a) {
                continue;
            }
            let arc = *self.net.arc(a).ok_or(Fail::Internal("arc hors bornes"))?;
            let src = *self.state(arc.from).ok_or(Fail::Internal("nœud hors bornes"))?;
            let Some(se) = src.early else {
                // Origine non planifiée (R-069) ou récapitulative vide.
                if let Some(l) = arc.link {
                    self.diags.push(Diagnostic::LinkExcludedEndpoint {
                        predecessor: l.pred,
                        successor: l.succ,
                    });
                }
                b.degraded |= src.failed || src.degraded;
                continue;
            };
            b.degraded |= src.degraded;
            let (cand, driver) = match arc.link {
                None => {
                    let summary = self
                        .net
                        .node(arc.from)
                        .ok_or(Fail::Internal("nœud hors bornes"))?
                        .uid;
                    (Candidate::Start(se.start), Driver::SummaryGate { summary })
                }
                Some(l) => {
                    let x = match self.link_bound(a, l, se) {
                        Ok(x) => x,
                        // R-069 : la borne d'un lien vers une récapitulative
                        // ne se calcule pas : le lien est ignoré, la porte
                        // garde ses autres bornes, dégradée.
                        Err(Fail::Conversion(cause)) if kind == NodeKind::Gate => {
                            self.drop_link(a, l, cause);
                            b.degraded = true;
                            continue;
                        }
                        Err(fail) => return Err(fail),
                    };
                    let cand = match l.ty {
                        LinkType::FinishToStart | LinkType::StartToStart => Candidate::Start(x),
                        LinkType::FinishToFinish | LinkType::StartToFinish => Candidate::Finish(x),
                    };
                    (
                        cand,
                        Driver::Link {
                            predecessor: l.pred,
                            link: l.ty,
                        },
                    )
                }
            };
            let scale = self
                .net
                .scale(place)
                .ok_or(Fail::Internal("échelle hors bornes"))?;
            let start = match (cand, pin) {
                // Placement imposé : la borne ne le déplace pas (R-063,
                // R-099) ; borne de début en brut, borne de fin en temps
                // ouvré de P(t).
                (Candidate::Start(x), Some(p)) => {
                    if x > p.span.start {
                        self.pin_violated(p.kind, arc.link, &mut b);
                    }
                    continue;
                }
                (Candidate::Finish(x), Some(p)) => {
                    if scale.between(p.span.finish, x)?.get() > 0 {
                        self.pin_violated(p.kind, arc.link, &mut b);
                    }
                    continue;
                }
                // R-039, R-040 : borne de fin convertie en borne de début.
                (Candidate::Finish(x), None) if kind == NodeKind::Leaf => {
                    scale.backward_span(x, duration)?.start
                }
                (Candidate::Start(x) | Candidate::Finish(x), None) => x,
            };
            if start > b.start {
                b.start = start;
                b.driver = driver;
            }
        }
        Ok(b)
    }

    /// Borne violée par un placement imposé : diagnostic du lien selon la
    /// nature du placement ; porte violée notée dans `b`.
    fn pin_violated(&mut self, kind: PinKind, link: Option<super::network::LinkArc>, b: &mut Bounds) {
        match (kind, link) {
            (PinKind::Manual, Some(l)) => self.diags.push(Diagnostic::LinkManualNotHonored {
                predecessor: l.pred,
                successor: l.succ,
            }),
            (PinKind::Constraint, Some(l)) => self.diags.push(Diagnostic::LinkNotHonoredConstraint {
                predecessor: l.pred,
                successor: l.succ,
            }),
            (PinKind::Constraint, None) => b.gate_violated = true,
            (PinKind::Manual, None) => {}
        }
    }

    /// Borne `g(ancre)` de l'arc de lien `a` (R-037 à R-043) : fin ou début
    /// du prédécesseur selon le type, transférée par le décalage.
    fn link_bound(&mut self, a: u32, l: super::network::LinkArc, se: Span) -> Result<Moment, Fail> {
        let lag = self.lag_minutes(a)?;
        let sigma = self
            .net
            .scale(l.sigma)
            .ok_or(Fail::Internal("échelle hors bornes"))?;
        let anchor = match l.ty {
            LinkType::FinishToStart | LinkType::FinishToFinish => se.finish,
            LinkType::StartToStart | LinkType::StartToFinish => se.start,
        };
        Ok(transfer::forward(*sigma, anchor, lag)?)
    }

    /// R-069 : arc de lien `a` vers une porte non appliqué
    /// (`link.unschedulable`).
    fn drop_link(&mut self, a: u32, l: super::network::LinkArc, cause: &str) {
        if let Some(slot) = super::network::pos(a).and_then(|i| self.dropped.get_mut(i)) {
            *slot = true;
        }
        self.diags.push(Diagnostic::LinkUnschedulable {
            predecessor: l.pred,
            successor: l.succ,
            cause: cause.to_owned(),
        });
    }

    /// Porte d'une récapitulative : plus grande borne de début (début du
    /// projet, porte du parent, liens FS/SS entrants, puis SNET de la
    /// récapitulative, R-102), non recalée.
    fn gate(&mut self, v: NodeIx) -> Result<(), Fail> {
        let constraint = self
            .net
            .node(v)
            .ok_or(Fail::Internal("nœud hors bornes"))?
            .constraint;
        let mut b = self.bounds(v, None)?;
        if let Some(c) = constraint.filter(|c| c.kind == ConstraintKind::StartNoEarlierThan)
            && c.date > b.start
        {
            b.start = c.date;
            b.driver = Driver::Constraint { constraint: c.kind };
        }
        let s = self.state_mut(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        s.early = Some(Span {
            start: b.start,
            finish: b.start,
        });
        s.driver = b.driver;
        s.degraded = b.degraded;
        Ok(())
    }

    /// Feuille : automatique (R-030, R-031 sur la borne, contraintes R-090
    /// à R-101), manuelle (dates saisies, R-063, contrainte ignorée, R-103)
    /// ou inactive (même calcul qu'une automatique, contrainte comprise,
    /// pour l'affichage, sans diagnostic, R-062, R-103).
    fn leaf(&mut self, v: NodeIx) -> Result<(), Fail> {
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let (place, duration, uid, inactive) = (node.place, node.duration, node.uid, node.inactive);
        let fixed = match node.fixed {
            Some(r) => Some(r?),
            None => None,
        };
        if let Some(f) = fixed {
            let b = self.bounds(
                v,
                Some(Pin {
                    span: f,
                    kind: PinKind::Manual,
                }),
            )?;
            let s = self.state_mut(v).ok_or(Fail::Internal("nœud hors bornes"))?;
            s.early = Some(f);
            s.driver = Driver::Manual;
            s.degraded = b.degraded;
            return Ok(());
        }
        let scale = *self
            .net
            .scale(place)
            .ok_or(Fail::Internal("échelle hors bornes"))?;
        let norm = self.net.normalized(v)?;
        let b = self.bounds(v, None)?;
        let mut early = scale.forward_span(b.start, duration)?;
        let mut driver = b.driver;
        let degraded = b.degraded;
        // Plancher (R-093, R-094, R-097, R-098), après les liens (R-044) :
        // début `D'`, ou début de `backward_span(D', d)` pour une fin, déjà
        // placés sur P(t) ; il ne pilote que s'il retarde le début placé.
        if let Some(n) = norm.filter(|n| n.c.floor()) {
            let floor = if n.c.on_start() { n.at } else { n.span.start };
            if floor > early.start {
                early = scale.forward_span(floor, duration)?;
                driver = Driver::Constraint { constraint: n.c.kind };
            }
        }
        if let Some(n) = norm {
            // R-094, R-098 (O14) : une fin au plus tôt contrainte finit sur
            // `D'` brute quand elle pilote (même temps ouvré que la fin
            // calculée, qui n'est jamais plus tardive en temps ouvré).
            if n.c.floor() && !n.c.on_start() && n.at > early.finish {
                early.finish = n.at;
            }
            // Plafond dépassé (R-095, R-096), en temps ouvré de P(t).
            let exceeded = n.c.ceiling()
                && if n.c.on_start() {
                    scale.between(n.at, early.start)?.get() > 0
                } else {
                    scale.between(n.at, early.finish)?.get() > 0
                };
            if self.project.options.honor_constraints && (n.c.must() || exceeded) {
                // R-097 à R-099 : la contrainte l'emporte.
                if early != n.span {
                    early = n.span;
                    driver = Driver::Constraint { constraint: n.c.kind };
                }
                if !inactive {
                    let rescan = self.bounds(
                        v,
                        Some(Pin {
                            span: early,
                            kind: PinKind::Constraint,
                        }),
                    )?;
                    // R-101 : début du projet ou porte d'une récapitulative
                    // violés.
                    if rescan.gate_violated || early.start < self.start {
                        self.conflict(uid, n.c, true);
                    }
                }
            } else if exceeded && !inactive {
                // R-100 : les liens l'emportent.
                self.conflict(uid, n.c, false);
            }
            if n.at != n.c.date && !inactive {
                self.diags.push(Diagnostic::TaskConstraintMoved {
                    task: uid,
                    from: n.c.date,
                    to: n.at,
                });
            }
        }
        let s = self.state_mut(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        s.early = Some(early);
        s.driver = driver;
        s.degraded = degraded;
        Ok(())
    }

    /// `task.constraint_conflict` (R-099 à R-102).
    fn conflict(&mut self, task: crate::ids::TaskUid, c: DateConstraint, honored: bool) {
        self.diags.push(Diagnostic::TaskConstraintConflict {
            task,
            constraint: c.kind,
            date: c.date,
            honored,
        });
    }

    /// Agrégat d'une récapitulative (R-053) : min des débuts, max des fins
    /// des enfants planifiés reliés (actifs, ou tous inactifs sous une
    /// récapitulative inactive). Aucun : pas de dates (`EmptySummary`).
    /// Récapitulative manuelle datée (R-064) : ses dates saisies, pilote
    /// `Manual`, quelles que soient celles de ses enfants.
    fn rollup(&mut self, v: NodeIx) -> Result<(), Fail> {
        let fixed = match self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?.fixed {
            Some(r) => Some(r?),
            None => None,
        };
        let incoming = self
            .net
            .incoming(v)
            .ok_or(Fail::Internal("arcs entrants hors bornes"))?;
        let mut span: Option<Span> = None;
        let (mut degraded, mut failed) = (false, false);
        for a in incoming {
            let arc = self.net.arc(a).ok_or(Fail::Internal("arc hors bornes"))?;
            let child = self.state(arc.from).ok_or(Fail::Internal("nœud hors bornes"))?;
            degraded |= child.degraded || child.failed;
            failed |= child.failed;
            if let Some(e) = child.early {
                span = Some(match span {
                    None => e,
                    Some(s) => Span {
                        start: s.start.min(e.start),
                        finish: s.finish.max(e.finish),
                    },
                });
            }
        }
        let s = self.state_mut(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        s.children = span;
        if let Some(f) = fixed {
            s.early = Some(f);
            s.driver = Driver::Manual;
            return Ok(());
        }
        s.early = span;
        s.driver = Driver::Rollup;
        s.degraded = degraded;
        // Tous les enfants en échec : l'agrégat compte comme non planifiable
        // pour ses successeurs (dégradés, R-069).
        s.failed = span.is_none() && failed;
        // R-102 : FNLT d'une récapitulative, jamais épinglée ; dépassée par
        // sa fin agrégée (en temps ouvré de son calendrier) → conflit. Un
        // échec de mesure (jamais attendu) ne rend pas l'agrégat non
        // planifiable : pas de diagnostic.
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        if let (Some(c), Some(e)) = (node.constraint, span)
            && c.kind == ConstraintKind::FinishNoLaterThan
            && !node.inactive
            && self
                .net
                .scale(node.place)
                .and_then(|m| m.between(c.date, e.finish).ok())
                .is_some_and(|m| m.get() > 0)
        {
            let uid = node.uid;
            self.conflict(uid, c, false);
        }
        Ok(())
    }
}
