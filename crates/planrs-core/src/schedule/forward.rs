//! Passe avant (spécification J2 § 3.7, R-036 à R-045, R-062, R-063,
//! R-069), dans l'ordre topologique.
//!
//! Chaque arc entrant donne une borne candidate ; la plus grande en
//! `Moment` brut gagne, et à égalité la première atteinte reste pilote
//! (R-044 : début du projet, porte du parent, liens dans l'ordre de
//! stockage).

use super::constraint::{ConstraintRule, Mode, Side};
use super::network::{Fail, NodeIx, NodeKind};
use super::{Calc, Driver, Pass, PassRole, transfer};
use crate::calendar::{Span, TimeScale};
use crate::diagnostic::{Diagnostic, OverriddenBound};
use crate::ids::TaskUid;
use crate::model::{ConstraintKind, LinkType};
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
    /// Passe avant complète, puis fin du projet (R-045). Depuis le début,
    /// c'est la passe principale (`role` = `Primary`).
    pub(super) fn forward(&mut self, order: &[NodeIx], role: PassRole) {
        self.role = role;
        self.quiet = role == PassRole::Replay;
        for &v in order {
            let Some(node) = self.net.node(v) else {
                self.emit(Diagnostic::ScheduleInternal {
                    detail: "nœud hors bornes en passe avant".to_owned(),
                });
                continue;
            };
            let (kind, uid) = (node.kind, node.uid);
            // Repasse : un nœud en échec dans la passe principale le reste
            // (ses voisins l'ont déjà ignoré).
            if self.quiet && self.state(v).is_some_and(|s| s.failed) {
                continue;
            }
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
        // Repasse : la fin du projet de la passe principale est gardée (les
        // épingles ne la déplacent pas, R-092).
        if role == PassRole::Replay {
            self.quiet = false;
            return;
        }
        // R-045 : fin du projet = max des fins au plus tôt des feuilles
        // actives planifiées (manuelles comprises, inactives exclues) et des
        // récapitulatives manuelles datées (R-064) ; sans tâche planifiée, le
        // début du projet. Ce max peut précéder le début du projet (tâche
        // manuelle ou contrainte respectée avant lui, R-105).
        let mut finish: Option<Moment> = None;
        for (node, s) in self.net.nodes.iter().zip(&self.st) {
            if (node.kind == NodeKind::Leaf || node.fixed.is_some())
                && !node.inactive
                && let Some(e) = s.early
            {
                finish = Some(finish.map_or(e.finish, |f| f.max(e.finish)));
            }
        }
        self.finish = finish.unwrap_or(self.start);
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
                    self.emit(Diagnostic::LinkExcludedEndpoint {
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
            let start = match (cand, fixed) {
                // Feuille manuelle : la borne ne la déplace pas (R-063).
                (Candidate::Start(x), Some(f)) => {
                    if x > f.start
                        && let Some(l) = arc.link
                    {
                        self.emit(Diagnostic::LinkManualNotHonored {
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
                        self.emit(Diagnostic::LinkManualNotHonored {
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
        self.emit(Diagnostic::LinkUnschedulable {
            predecessor: l.pred,
            successor: l.succ,
            cause: cause.to_owned(),
        });
    }

    /// Porte d'une récapitulative : plus grande borne de début (début du
    /// projet, porte du parent, liens FS/SS entrants, SNET de la
    /// récapitulative), non recalée.
    fn gate(&mut self, v: NodeIx) -> Result<(), Fail> {
        let mut b = self.bounds(v, None)?;
        // R-103 : SNET d'une récapitulative, plancher brut de sa porte (non
        // recalé, comme une borne de lien, R-056), candidat en dernier.
        if let Some(c) = self.net.node(v).and_then(|n| n.constraint)
            && let (ConstraintRule::Floor(Side::Start), Some(date)) = (c.rule, c.date)
            && date > b.start
        {
            b.start = date;
            b.driver = Driver::Constraint { kind: c.kind };
        }
        let replay = self.role == PassRole::Replay;
        let s = self.state_mut(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        s.early = Some(Span {
            start: b.start,
            finish: b.start,
        });
        s.driver = b.driver;
        s.degraded = b.degraded || (replay && s.degraded);
        Ok(())
    }

    /// Feuille : automatique (R-030, R-031 sur la borne), manuelle (dates
    /// saisies, R-063) ou inactive (même calcul qu'une automatique, pour
    /// l'affichage, R-062).
    fn leaf(&mut self, v: NodeIx) -> Result<(), Fail> {
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let (place, duration, constraint) = (node.place, node.duration, node.constraint);
        let fixed = match node.fixed {
            Some(r) => Some(r?),
            None => None,
        };
        let b = self.bounds(v, fixed)?;
        let pin = self.state(v).and_then(|s| s.pin);
        let (early, driver) = if let Some(f) = fixed {
            (f, Driver::Manual)
        } else {
            let scale = *self
                .net
                .scale(place)
                .ok_or(Fail::Internal("échelle hors bornes"))?;
            // R-092 : épingle d'une tâche ALAP, tenue pour fixe en repasse ;
            // une borne violée est un invariant violé.
            if let (Some(p), Some(c)) = (pin, constraint) {
                self.overrides(v, scale, p, c.kind, true)?;
                (p, Driver::Constraint { kind: c.kind })
            } else {
                let natural = scale.forward_span(b.start, duration)?;
                self.constrain(v, scale, natural, b.driver)?
            }
        };
        let replay = self.role == PassRole::Replay;
        let s = self.state_mut(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        s.early = Some(early);
        s.driver = driver;
        // Repasse : les drapeaux posés par la passe arrière sont gardés.
        s.degraded = b.degraded || (replay && s.degraded);
        Ok(())
    }

    /// Contrainte d'une feuille automatique en passe avant (ADR-0010), sur
    /// le placement `natural` de ses bornes (R-044) : un plancher (R-093,
    /// R-094) ne pilote que s'il déplace le début placé ; une date de
    /// contrainte plus grande en brut mais de même temps ouvré ne change
    /// rien.
    /// - plancher de début : `s* > natural.start` (deux débuts placés,
    ///   comparés en brut) → `early = A` ;
    /// - plancher de fin : `P.between(natural.finish, f*) > 0` → `early = A`
    ///   (fin = `f*`, O14, et non `forward_span(A.start, d)`).
    fn constrain(
        &mut self,
        v: NodeIx,
        scale: TimeScale<'_>,
        natural: Span,
        driver: Driver,
    ) -> Result<(Span, Driver), Fail> {
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let Some(c) = node.constraint else {
            return Ok((natural, driver));
        };
        let pinned = Driver::Constraint { kind: c.kind };
        // Passe principale depuis le début, repasse comprise (R-092 : la
        // repasse rejoue la passe principale, contraintes comprises).
        let primary_from_start = self.role != PassRole::Secondary && self.mode == Mode::FromStart;
        match c.rule {
            // R-097, R-098 : option active, MSO et MFO épinglent la feuille
            // sur l'ancrage, quelles que soient ses bornes (R-100).
            ConstraintRule::Pin(_) => {
                let a = c.span()?;
                self.overrides(v, scale, a, c.kind, false)?;
                return Ok((a, pinned));
            }
            // R-095, R-096, R-099 : option active, le plafond (SNLT, FNLT)
            // s'impose en passe principale ; chaque borne violée est
            // signalée (R-100).
            ConstraintRule::Clamped(side) if primary_from_start => {
                let a = c.span()?;
                let late = match side {
                    Side::Start => natural.start > a.start,
                    Side::Finish => scale.between(a.finish, natural.finish)?.get() > 0,
                };
                if late {
                    self.overrides(v, scale, a, c.kind, false)?;
                    return Ok((a, pinned));
                }
            }
            // Plancher : SNET, FNET ; MSO, MFO option inactive (fenêtre,
            // R-097, R-098).
            ConstraintRule::Floor(Side::Start) | ConstraintRule::Window(Side::Start) => {
                let a = c.span()?;
                if a.start > natural.start {
                    return Ok((a, pinned));
                }
            }
            ConstraintRule::Floor(Side::Finish) | ConstraintRule::Window(Side::Finish) => {
                let a = c.span()?;
                if scale.between(natural.finish, a.finish)?.get() > 0 {
                    return Ok((a, pinned));
                }
            }
            ConstraintRule::None
            | ConstraintRule::Opposite
            | ConstraintRule::Cap(_)
            | ConstraintRule::Clamped(_) => {}
        }
        Ok((natural, driver))
    }

    /// R-100 : la feuille `v`, placée de force sur `a` par sa contrainte de
    /// type `kind` (`opposite` : épingle d'une tâche ALAP en repasse, R-092),
    /// viole ces bornes ; un `constraint.override` par borne
    /// violée, en passe principale seulement : début du projet si
    /// `a.start <` début (brut) ; porte du parent si `a.start < G` ; lien
    /// FS/SS si `a.start <` borne (brut) ; lien FF/SF si
    /// `P.between(a.finish, borne) > 0` (R-063, même lecture que pour une
    /// tâche manuelle).
    fn overrides(
        &mut self,
        v: NodeIx,
        scale: TimeScale<'_>,
        a: Span,
        kind: ConstraintKind,
        opposite: bool,
    ) -> Result<(), Fail> {
        // Passe secondaire : jamais ; repasse : seulement pour contrôler les
        // épingles des tâches à placement opposé (les contraintes dures ont
        // déjà été signalées par la passe principale).
        if self.role == PassRole::Secondary || (self.role == PassRole::Replay && !opposite) {
            return Ok(());
        }
        let task = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?.uid;
        if a.start < self.start {
            self.violated(task, kind, OverriddenBound::ProjectStart);
        }
        let incoming = self
            .net
            .incoming(v)
            .ok_or(Fail::Internal("arcs entrants hors bornes"))?;
        for i in incoming {
            if self.net.is_cut(i) || self.is_dropped(i) {
                continue;
            }
            let arc = *self.net.arc(i).ok_or(Fail::Internal("arc hors bornes"))?;
            let Some(se) = self.state(arc.from).and_then(|s| s.early) else {
                continue;
            };
            let bound = match arc.link {
                None => {
                    let summary = self
                        .net
                        .node(arc.from)
                        .ok_or(Fail::Internal("nœud hors bornes"))?
                        .uid;
                    (se.start > a.start).then_some(OverriddenBound::SummaryGate { summary })
                }
                Some(l) => {
                    let x = self.link_bound(i, l, se)?;
                    let violated = match l.ty {
                        LinkType::FinishToStart | LinkType::StartToStart => x > a.start,
                        LinkType::FinishToFinish | LinkType::StartToFinish => {
                            scale.between(a.finish, x)?.get() > 0
                        }
                    };
                    violated.then_some(OverriddenBound::Predecessor {
                        predecessor: l.pred,
                        link: l.ty,
                    })
                }
            };
            if let Some(bound) = bound {
                self.violated(task, kind, bound);
            }
        }
        Ok(())
    }

    /// Borne `bound` violée par la tâche `task` placée de force : en passe
    /// principale, `constraint.override` (R-100) ; en repasse, une épingle
    /// (R-092) ne viole jamais un lien : invariant violé.
    fn violated(&mut self, task: TaskUid, kind: ConstraintKind, bound: OverriddenBound) {
        if self.role == PassRole::Replay {
            self.emit(Diagnostic::ScheduleInternal {
                detail: format!("tâche {task} : épingle en repasse, {bound} est violé(e)"),
            });
        } else {
            self.emit(Diagnostic::ConstraintOverride { task, kind, bound });
        }
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
        let replay = self.role == PassRole::Replay;
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
        s.degraded = degraded || (replay && s.degraded);
        // Tous les enfants en échec : l'agrégat compte comme non planifiable
        // pour ses successeurs (dégradés, R-069).
        s.failed = span.is_none() && failed;
        Ok(())
    }
}
