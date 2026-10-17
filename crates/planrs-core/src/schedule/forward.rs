//! Passe avant (spécification J2 § 3.7, R-036 à R-045, R-062, R-063,
//! R-069), dans l'ordre topologique.
//!
//! Chaque arc entrant donne une borne candidate ; la plus grande en
//! `Moment` brut gagne, et à égalité la première atteinte reste pilote
//! (R-044 : début du projet, porte du parent, liens dans l'ordre de
//! stockage).

use super::network::{Fail, NodeIx, NodeKind};
use super::{Calc, Driver, Pass, transfer};
use crate::calendar::Span;
use crate::diagnostic::Diagnostic;
use crate::model::LinkType;
use crate::time::Moment;

/// Bornes d'un nœud : plus grande borne de début et son pilote, drapeau
/// `degraded` transmis par les arcs entrants.
struct Bounds {
    start: Moment,
    driver: Driver,
    degraded: bool,
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

    /// Bornes de début du nœud `v` (R-036, R-044). Pour une feuille
    /// manuelle (`fixed`), chaque borne violée donne
    /// `link.manual_not_honored` (R-063) et aucune conversion n'est faite.
    fn bounds(&mut self, v: NodeIx, fixed: Option<Span>) -> Result<Bounds, Fail> {
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let (place, duration, kind) = (node.place, node.duration, node.kind);
        let mut b = Bounds {
            start: self.start,
            driver: Driver::ProjectStart,
            degraded: false,
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
                    let lag = self.lag_minutes(a)?;
                    let sigma = self
                        .net
                        .scale(l.sigma)
                        .ok_or(Fail::Internal("échelle hors bornes"))?;
                    let anchor = match l.ty {
                        LinkType::FinishToStart | LinkType::FinishToFinish => se.finish,
                        LinkType::StartToStart | LinkType::StartToFinish => se.start,
                    };
                    let x = transfer::forward(*sigma, anchor, lag)?;
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
            let start = match (cand, fixed) {
                // Feuille manuelle : la borne ne la déplace pas (R-063).
                (Candidate::Start(x), Some(f)) => {
                    if x > f.start
                        && let Some(l) = arc.link
                    {
                        self.diags.push(Diagnostic::LinkManualNotHonored {
                            predecessor: l.pred,
                            successor: l.succ,
                        });
                    }
                    continue;
                }
                (Candidate::Finish(x), Some(f)) => {
                    if scale.between(f.finish, x)?.get() > 0
                        && let Some(l) = arc.link
                    {
                        self.diags.push(Diagnostic::LinkManualNotHonored {
                            predecessor: l.pred,
                            successor: l.succ,
                        });
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

    /// Porte d'une récapitulative : plus grande borne de début (début du
    /// projet, porte du parent, liens FS/SS entrants), non recalée.
    fn gate(&mut self, v: NodeIx) -> Result<(), Fail> {
        let b = self.bounds(v, None)?;
        let s = self.state_mut(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        s.early = Some(Span {
            start: b.start,
            finish: b.start,
        });
        s.driver = b.driver;
        s.degraded = b.degraded;
        Ok(())
    }

    /// Feuille : automatique (R-030, R-031 sur la borne), manuelle (dates
    /// saisies, R-063) ou inactive (même calcul qu'une automatique, pour
    /// l'affichage, R-062).
    fn leaf(&mut self, v: NodeIx) -> Result<(), Fail> {
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let (place, duration) = (node.place, node.duration);
        let fixed = match node.fixed {
            Some(r) => Some(r?),
            None => None,
        };
        let b = self.bounds(v, fixed)?;
        let (early, driver) = if let Some(f) = fixed {
            (f, Driver::Manual)
        } else {
            let scale = self
                .net
                .scale(place)
                .ok_or(Fail::Internal("échelle hors bornes"))?;
            (scale.forward_span(b.start, duration)?, b.driver)
        };
        let s = self.state_mut(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        s.early = Some(early);
        s.driver = driver;
        s.degraded = b.degraded;
        Ok(())
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
        Ok(())
    }
}
