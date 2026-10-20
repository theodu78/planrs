//! Agrégats des récapitulatives et assemblage du résultat (spécification J2
//! § 3.10).
//!
//! Récapitulatives : dates (R-053, agrégat de la passe avant), durée
//! (R-054), % achevé (R-055), dates au plus tard agrégées (R-060, dans
//! `backward.rs`), récapitulative manuelle datée (R-064), récapitulative du
//! projet (R-068), dates planifiées (R-112 : au plus tôt depuis le début, au
//! plus tard à rebours, enveloppe de celles des enfants).

use std::collections::BTreeMap;
use std::ops::Range;

use super::constraint::Mode;
use super::network::{NodeIx, NodeKind, PROJECT, Role, Status, TaskInfo, pos};
use super::{Calc, Driver, Exclusion, LateDriver, Schedule, TaskOutcome, TaskRole, TaskSchedule};
use crate::calendar::Span;
use crate::diagnostic::Diagnostic;
use crate::duration::{Minutes, Work};
use crate::fixed::round_div;
use crate::ids::TaskUid;
use crate::model::Task;
use crate::quantity::Percent;

impl Calc<'_, '_> {
    /// Assemble le `Schedule` : une entrée par tâche du modèle, l'ordre
    /// topologique des tâches planifiées, les diagnostics triés.
    pub(super) fn output(mut self, order: &[NodeIx]) -> Schedule {
        self.empty_summary_links();
        // J5 : affectations placées et durée calculée des feuilles à parts
        // (R-173).
        let (assignments, durations) = self.place_assignments();
        let sums = match self.strategy {
            super::Strategy::Fast => self.percent_sums(),
            super::Strategy::Reference => None,
        };
        let mut tasks: BTreeMap<TaskUid, TaskOutcome> = BTreeMap::new();
        for (p, task) in self.project.tasks.iter().enumerate() {
            let outcome = if let Some(info) = self.net.task(p).cloned() {
                self.outcome(p, task, &info, (sums.as_deref(), durations.get(&p).copied()))
            } else {
                self.diags.push(Diagnostic::ScheduleInternal {
                    detail: format!("tâche {} non classée", task.uid()),
                });
                TaskOutcome::Excluded {
                    reason: Exclusion::Unschedulable,
                    display: None,
                }
            };
            tasks.insert(task.uid(), outcome);
        }
        let mut sorted = Vec::new();
        for &v in order {
            let Some(node) = self.net.node(v) else { continue };
            if node.kind != NodeKind::Gate && matches!(tasks.get(&node.uid), Some(TaskOutcome::Scheduled(_)))
            {
                sorted.push(node.uid);
            }
        }
        let project_summary = self.project_summary(&tasks, sums.as_deref());
        Schedule {
            project_start: self.start,
            project_finish: self.finish,
            project_summary,
            tasks,
            order: sorted,
            diagnostics: self.diags.finish(),
            assignments,
            resources: BTreeMap::new(),
        }
    }

    /// R-068 : récapitulative du projet (UID 0), agrégat des tâches actives
    /// planifiées de premier niveau (R-053, R-054, R-055, R-060) sur le
    /// calendrier du projet ; sans lien, sa marge libre égale sa marge
    /// totale ; dates planifiées et durée sur l'enveloppe des dates
    /// planifiées de ces tâches (R-112). `None` si aucune tâche n'est
    /// planifiée.
    fn project_summary(
        &self,
        tasks: &BTreeMap<crate::ids::TaskUid, TaskOutcome>,
        sums: Option<&[LeafSums]>,
    ) -> Option<TaskSchedule> {
        let (mut early, mut late, mut planned, mut degraded): (
            Option<Span>,
            Option<Span>,
            Option<Span>,
            bool,
        ) = (None, None, None, false);
        let hull = |a: Option<Span>, b: Span| {
            Some(a.map_or(b, |a| Span {
                start: a.start.min(b.start),
                finish: a.finish.max(b.finish),
            }))
        };
        for info in &self.net.tasks {
            if info.parent.is_some() || info.role == Role::Null || info.inactive() {
                continue;
            }
            let Some(TaskOutcome::Scheduled(ts)) = tasks.get(&info.uid) else {
                continue;
            };
            early = hull(early, ts.early);
            late = hull(late, ts.late);
            planned = hull(
                planned,
                Span {
                    start: ts.start,
                    finish: ts.finish,
                },
            );
            degraded |= ts.degraded;
        }
        let (early, late, planned) = (early?, late?, planned?);
        let scale = self.net.scale(PROJECT);
        let between = |a, b| scale.and_then(|s| s.between(a, b).ok());
        let duration = between(planned.start, planned.finish);
        let tf = match (
            between(early.start, late.start),
            between(early.finish, late.finish),
        ) {
            (Some(a), Some(b)) => Some(a.min(b)),
            _ => None,
        };
        degraded |= duration.is_none() || tf.is_none();
        let tf = tf.unwrap_or(Minutes::ZERO);
        Some(TaskSchedule {
            role: TaskRole::Summary,
            manual: false,
            start: planned.start,
            finish: planned.finish,
            early,
            late,
            duration: duration.unwrap_or(Minutes::ZERO),
            total_slack: tf,
            // R-101 : sans lien, marge libre = marge totale, bornée à 0.
            free_slack: tf.max(Minutes::ZERO),
            critical: tf <= self.critical_limit(),
            percent_complete: self.percent_of(sums, 0..self.net.tasks.len()),
            driver: Driver::Rollup,
            late_driver: LateDriver::Rollup,
            degraded,
            work: Work::ZERO,
            progress: None,
            driver_bound: None,
        })
    }

    /// Liens vers une récapitulative sans enfant planifié (R-053) :
    /// `link.excluded_endpoint` (les liens qui en partent sont signalés en
    /// passe avant).
    fn empty_summary_links(&mut self) {
        for arc in &self.net.arcs {
            let Some(l) = arc.link else { continue };
            let Some(target) = self.net.node(arc.to) else {
                continue;
            };
            if target.kind != NodeKind::Gate || target.inactive {
                continue;
            }
            let empty = target
                .twin
                .and_then(pos)
                .and_then(|r| self.st.get(r))
                .is_none_or(|s| s.early.is_none());
            if empty {
                self.diags.push(Diagnostic::LinkExcludedEndpoint {
                    predecessor: l.pred,
                    successor: l.succ,
                });
            }
        }
    }

    /// Résultat de la tâche en position `p`.
    /// `measured` : durée calculée d'une feuille à parts (R-173).
    #[allow(
        clippy::too_many_lines,
        reason = "construction d'un `TaskSchedule` champ par champ (champs J6 compris)"
    )]
    fn outcome(
        &mut self,
        p: usize,
        task: &Task,
        info: &TaskInfo,
        (sums, measured): (Option<&[LeafSums]>, Option<Minutes>),
    ) -> TaskOutcome {
        let excluded = |reason| TaskOutcome::Excluded {
            reason,
            display: None,
        };
        if info.role == Role::Null {
            return excluded(Exclusion::NullRow);
        }
        if info.manual_undated {
            return excluded(Exclusion::ManualUndated);
        }
        let exit = info.exit.and_then(|v| self.state(v).copied());
        if info.inactive() {
            // R-062 : placement d'affichage.
            let reason = if info.status == Status::Inactive {
                Exclusion::Inactive
            } else {
                Exclusion::InactiveAncestor
            };
            return TaskOutcome::Excluded {
                reason,
                display: exit.and_then(|s| s.early),
            };
        }
        let Some(state) = exit else {
            return excluded(Exclusion::Unschedulable);
        };
        let node = info.exit.and_then(|v| self.net.node(v));
        let fixed = node.is_some_and(|n| n.fixed.is_some());
        let Some(early) = state.early else {
            // Une récapitulative manuelle datée dont les dates ne se
            // calculent pas est non planifiable, pas vide (R-064, R-069).
            return excluded(if info.role == Role::Summary && !fixed {
                Exclusion::EmptySummary
            } else {
                Exclusion::Unschedulable
            });
        };
        let mut degraded = state.degraded || state.late_degraded;
        let late = state.late.unwrap_or_else(|| {
            degraded = true;
            early
        });
        if state.late.is_none() {
            self.diags.push(Diagnostic::ScheduleInternal {
                detail: format!("tâche {} sans dates au plus tard", task.uid()),
            });
        }
        // Cause de la fin au plus tard : agrégat des enfants pour une
        // récapitulative automatique (R-060), repli sans dates au plus tard
        // (R-069).
        let late_driver = if state.late.is_none() {
            LateDriver::Fallback
        } else if info.role == Role::Summary && !fixed {
            LateDriver::Rollup
        } else {
            state.late_driver
        };
        // R-112 : dates planifiées, au plus tôt depuis le début, au plus tard
        // à rebours (enveloppe de celles des enfants pour une récapitulative
        // automatique, par R-053 et R-060).
        let planned = match self.mode {
            Mode::FromStart => early,
            Mode::FromFinish => late,
        };
        // R-178 : le retard de nivellement d'une feuille décale son début
        // planifié, pas son début au plus tôt (O27).
        let early = Span {
            start: state.undelayed.unwrap_or(early.start),
            ..early
        };
        let (role, manual, duration, percent_complete) = if info.role == Role::Summary {
            // R-054, mesurée sur les dates planifiées (R-112) ; récapitulative
            // manuelle datée : durée utilisée (R-064).
            let duration = if fixed {
                node.map(|n| n.duration)
            } else {
                // R-113 : mesurée sur le calendrier du projet (de base) pour
                // une récapitulative sans calendrier propre.
                node.and_then(|n| self.net.scale(n.measure))
                    .and_then(|s| s.between(planned.start, planned.finish).ok())
            };
            if duration.is_none() {
                degraded = true;
            }
            (
                TaskRole::Summary,
                fixed,
                duration.unwrap_or(Minutes::ZERO),
                self.percent_of(sums, p.saturating_add(1)..info.end),
            )
        } else {
            (
                TaskRole::Leaf,
                fixed,
                measured.unwrap_or_else(|| node.map_or(task.duration.minutes(), |n| n.duration)),
                task.progress.percent_complete,
            )
        };
        TaskOutcome::Scheduled(TaskSchedule {
            role,
            manual,
            start: planned.start,
            finish: planned.finish,
            early,
            late,
            duration,
            total_slack: state.total_slack,
            free_slack: state.free_slack,
            critical: state.total_slack <= self.critical_limit(),
            percent_complete,
            driver: state.driver,
            late_driver,
            degraded,
            work: Work::ZERO,
            progress: None,
            driver_bound: None,
        })
    }

    /// R-055 : feuille retenue en position `q` (active, planifiée) :
    /// `(réel, durée, %)`, réel = `round_div(durée × %, 10 000)` (R-010) ;
    /// `None` pour une récapitulative, une tâche inactive ou non planifiée.
    fn leaf_terms(&self, q: usize) -> Option<(i128, i128, i128)> {
        let (leaf, task) = (self.net.task(q)?, self.project.tasks.at(q)?);
        if leaf.role != Role::Leaf || leaf.inactive() {
            return None;
        }
        let v = leaf.exit?;
        let (node, state) = (self.net.node(v)?, self.state(v)?);
        state.early?;
        let d = i128::from(node.duration.get());
        let pct = i128::from(task.progress.percent_complete.get());
        let a = d.checked_mul(pct).and_then(|x| round_div(x, 10_000)).unwrap_or(0);
        Some((i128::from(a), d, pct))
    }

    /// Sommes préfixes de [`Self::leaf_terms`] sur les positions (lot L15) :
    /// `sums[q]` porte les sommes des positions `0..q`. Le sous-arbre d'une
    /// récapitulative est une plage contiguë de positions : son % achevé se
    /// lit en O(1). `None` en cas de débordement (jamais attendu : 2³²
    /// feuilles de durée `i64::MAX` tiennent en `i128`) ; l'appelant passe
    /// alors par la référence.
    fn percent_sums(&self) -> Option<Vec<LeafSums>> {
        let n = self.net.tasks.len();
        let mut sums = Vec::with_capacity(n.checked_add(1)?);
        let mut acc = LeafSums::default();
        sums.push(acc);
        for q in 0..n {
            if let Some((a, d, pct)) = self.leaf_terms(q) {
                acc = LeafSums {
                    actual: acc.actual.checked_add(a)?,
                    total: acc.total.checked_add(d)?,
                    pct_sum: acc.pct_sum.checked_add(pct)?,
                    count: acc.count.checked_add(1)?,
                };
            }
            sums.push(acc);
        }
        Some(sums)
    }

    /// R-055 sur les positions `range` : par les sommes préfixes si elles
    /// existent, sinon par la référence.
    fn percent_of(&self, sums: Option<&[LeafSums]>, range: Range<usize>) -> Percent {
        let Some(sums) = sums else {
            return self.percent_over(range);
        };
        let n = sums.len().saturating_sub(1);
        let end = range.end.min(n);
        let start = range.start.min(end);
        let (Some(hi), Some(lo)) = (sums.get(end), sums.get(start)) else {
            return self.percent_over(range);
        };
        let diff = |a: i128, b: i128| a.checked_sub(b);
        match (
            diff(hi.actual, lo.actual),
            diff(hi.total, lo.total),
            diff(hi.pct_sum, lo.pct_sum),
            diff(hi.count, lo.count),
        ) {
            (Some(actual), Some(total), Some(pct_sum), Some(count)) => {
                percent_value(actual, total, pct_sum, count)
            }
            _ => self.percent_over(range),
        }
    }

    /// R-055 : % achevé d'une récapitulative,
    /// `round_div(Σ réel × 10 000, Σ durée)` sur ses feuilles descendantes
    /// actives planifiées (récapitulatives intermédiaires exclues), réel =
    /// durée × % (R-010) ; Σ durée nulle : moyenne arrondie des % ; aucune
    /// feuille : 0.
    /// Feuilles des positions `range` (le sous-arbre d'une récapitulative,
    /// ou tout le projet pour la récapitulative du projet, R-068).
    /// Référence (J3, sous-arbre parcouru) : oracle des sommes préfixes et
    /// leur repli.
    fn percent_over(&self, range: Range<usize>) -> Percent {
        let (mut actual, mut total, mut pct_sum, mut count) = (0i128, 0i128, 0i128, 0i128);
        for q in range {
            let Some((a, d, pct)) = self.leaf_terms(q) else {
                continue;
            };
            actual = actual.saturating_add(a);
            total = total.saturating_add(d);
            pct_sum = pct_sum.saturating_add(pct);
            count = count.saturating_add(1);
        }
        percent_value(actual, total, pct_sum, count)
    }
}

/// Sommes de R-055 sur une plage de positions.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct LeafSums {
    /// Σ réel.
    actual: i128,
    /// Σ durée.
    total: i128,
    /// Σ %.
    pct_sum: i128,
    /// Nombre de feuilles retenues.
    count: i128,
}

/// R-055 : valeur du % achevé depuis ses sommes.
fn percent_value(actual: i128, total: i128, pct_sum: i128, count: i128) -> Percent {
    let value = if total > 0 {
        actual.checked_mul(10_000).and_then(|x| round_div(x, total))
    } else if count > 0 {
        round_div(pct_sum, count)
    } else {
        Some(0)
    };
    value.and_then(|v| Percent::new(v).ok()).unwrap_or(Percent::ZERO)
}
