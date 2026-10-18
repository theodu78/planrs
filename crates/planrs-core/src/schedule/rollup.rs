//! Agrégats des récapitulatives et assemblage du résultat (spécification J2
//! § 3.10).
//!
//! Récapitulatives : dates (R-053, agrégat de la passe avant), durée
//! (R-054), % achevé (R-055), dates au plus tard agrégées (R-060, dans
//! `backward.rs`), récapitulative manuelle datée (R-064), récapitulative du
//! projet (R-068), échéance dépassée (R-108, R-109).

use std::collections::BTreeMap;
use std::ops::Range;

use super::network::{NodeIx, NodeKind, PROJECT, Role, Status, TaskInfo, pos};
use super::{Calc, Driver, Exclusion, Schedule, TaskOutcome, TaskRole, TaskSchedule};
use crate::calendar::Span;
use crate::diagnostic::Diagnostic;
use crate::duration::Minutes;
use crate::fixed::round_div;
use crate::ids::TaskUid;
use crate::model::Task;
use crate::quantity::Percent;

impl Calc<'_, '_> {
    /// Assemble le `Schedule` : une entrée par tâche du modèle, l'ordre
    /// topologique des tâches planifiées, les diagnostics triés.
    pub(super) fn output(mut self, order: &[NodeIx]) -> Schedule {
        self.empty_summary_links();
        let mut tasks: BTreeMap<TaskUid, TaskOutcome> = BTreeMap::new();
        for (p, task) in self.project.tasks.iter().enumerate() {
            let outcome = if let Some(info) = self.net.task(p).cloned() {
                self.outcome(p, task, &info)
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
        let project_summary = self.project_summary(&tasks);
        Schedule {
            project_start: self.start,
            project_finish: self.finish,
            project_summary,
            tasks,
            order: sorted,
            diagnostics: self.diags.finish(),
        }
    }

    /// R-068 : récapitulative du projet (UID 0), agrégat des tâches actives
    /// planifiées de premier niveau (R-053, R-054, R-055, R-060) sur le
    /// calendrier du projet ; sans lien, sa marge libre égale sa marge
    /// totale. `None` si aucune tâche n'est planifiée.
    fn project_summary(&self, tasks: &BTreeMap<crate::ids::TaskUid, TaskOutcome>) -> Option<TaskSchedule> {
        let (mut early, mut late, mut degraded): (Option<Span>, Option<Span>, bool) = (None, None, false);
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
            degraded |= ts.degraded;
        }
        let (early, late) = (early?, late?);
        let scale = self.net.scale(PROJECT);
        let between = |a, b| scale.and_then(|s| s.between(a, b).ok());
        let duration = between(early.start, early.finish);
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
            start: early.start,
            finish: early.finish,
            early,
            late,
            duration: duration.unwrap_or(Minutes::ZERO),
            total_slack: tf,
            free_slack: tf,
            critical: tf <= self.critical_limit(),
            percent_complete: self.percent_over(0..self.net.tasks.len()),
            driver: Driver::Rollup,
            degraded,
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
    fn outcome(&mut self, p: usize, task: &Task, info: &TaskInfo) -> TaskOutcome {
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
        let (role, manual, duration, percent_complete) = if info.role == Role::Summary {
            // R-054 ; récapitulative manuelle datée : durée utilisée (R-064).
            let duration = if fixed {
                node.map(|n| n.duration)
            } else {
                node.and_then(|n| self.net.scale(n.place))
                    .and_then(|s| s.between(early.start, early.finish).ok())
            };
            if duration.is_none() {
                degraded = true;
            }
            (
                TaskRole::Summary,
                fixed,
                duration.unwrap_or(Minutes::ZERO),
                self.percent_over(p.saturating_add(1)..info.end),
            )
        } else {
            (
                TaskRole::Leaf,
                fixed,
                node.map_or(task.duration.minutes(), |n| n.duration),
                task.progress.percent_complete,
            )
        };
        let (start, finish) = (early.start, early.finish);
        self.deadline_missed(node.and_then(|n| n.deadline), node.map(|n| n.place), task, finish);
        TaskOutcome::Scheduled(TaskSchedule {
            role,
            manual,
            start,
            finish,
            early,
            late,
            duration,
            total_slack: state.total_slack,
            free_slack: state.free_slack,
            critical: state.total_slack <= self.critical_limit(),
            percent_complete,
            driver: state.driver,
            degraded,
        })
    }

    /// R-108, R-109 : `task.deadline_missed` si la fin planifiée dépasse
    /// l'échéance effective du nœud de sortie, en temps ouvré de son échelle
    /// (une fin `D'` brute posée sur le même temps ouvré que l'échéance ne la
    /// dépasse pas). Aucun effet sur les dates.
    fn deadline_missed(
        &mut self,
        deadline: Option<crate::time::Moment>,
        place: Option<u32>,
        task: &Task,
        finish: crate::time::Moment,
    ) {
        let Some(deadline) = deadline else { return };
        let late = place
            .and_then(|p| self.net.scale(p))
            .and_then(|s| s.between(deadline, finish).ok())
            .is_some_and(|m| m.get() > 0);
        if late {
            self.diags.push(Diagnostic::TaskDeadlineMissed {
                task: task.uid(),
                deadline,
                finish,
            });
        }
    }

    /// R-055 : % achevé d'une récapitulative,
    /// `round_div(Σ réel × 10 000, Σ durée)` sur ses feuilles descendantes
    /// actives planifiées (récapitulatives intermédiaires exclues), réel =
    /// durée × % (R-010) ; Σ durée nulle : moyenne arrondie des % ; aucune
    /// feuille : 0.
    /// Feuilles des positions `range` (le sous-arbre d'une récapitulative,
    /// ou tout le projet pour la récapitulative du projet, R-068).
    fn percent_over(&self, range: Range<usize>) -> Percent {
        let (mut actual, mut total, mut pct_sum, mut count) = (0i128, 0i128, 0i128, 0i128);
        for q in range {
            let (Some(leaf), Some(task)) = (self.net.task(q), self.project.tasks.at(q)) else {
                continue;
            };
            if leaf.role != Role::Leaf || leaf.inactive() {
                continue;
            }
            let Some(v) = leaf.exit else { continue };
            let (Some(node), Some(state)) = (self.net.node(v), self.state(v)) else {
                continue;
            };
            if state.early.is_none() {
                continue;
            }
            let d = i128::from(node.duration.get());
            let pct = i128::from(task.progress.percent_complete.get());
            let a = d.checked_mul(pct).and_then(|x| round_div(x, 10_000)).unwrap_or(0);
            actual = actual.saturating_add(i128::from(a));
            total = total.saturating_add(d);
            pct_sum = pct_sum.saturating_add(pct);
            count = count.saturating_add(1);
        }
        let value = if total > 0 {
            actual.checked_mul(10_000).and_then(|x| round_div(x, total))
        } else if count > 0 {
            round_div(pct_sum, count)
        } else {
            Some(0)
        };
        value.and_then(|v| Percent::new(v).ok()).unwrap_or(Percent::ZERO)
    }
}
