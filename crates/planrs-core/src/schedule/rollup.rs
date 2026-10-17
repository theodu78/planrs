//! Agrégats des récapitulatives et assemblage du résultat (spécification J2
//! § 3.10).
//!
//! Mécanique des récapitulatives en place ici : dates (R-053), durée
//! (R-054), % achevé (R-055), dates au plus tard agrégées (R-060, dans
//! `backward.rs`). Restent à l'agent des récapitulatives : la récapitulative
//! manuelle à dates fixes (R-064), la récapitulative du projet (R-068), et
//! les fiches et tests des règles R-053 à R-058, R-060.

use std::collections::BTreeMap;

use super::network::{NodeIx, NodeKind, Role, Status, TaskInfo, pos};
use super::{Calc, Exclusion, Schedule, TaskOutcome, TaskRole, TaskSchedule};
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
        Schedule {
            project_start: self.start,
            project_finish: self.finish,
            // R-068 : agent des récapitulatives.
            project_summary: None,
            tasks,
            order: sorted,
            diagnostics: self.diags.finish(),
        }
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
        let Some(early) = state.early else {
            return excluded(if info.role == Role::Summary {
                Exclusion::EmptySummary
            } else {
                Exclusion::Unschedulable
            });
        };
        let mut degraded = state.degraded;
        let late = state.late.unwrap_or_else(|| {
            degraded = true;
            early
        });
        if state.late.is_none() {
            self.diags.push(Diagnostic::ScheduleInternal {
                detail: format!("tâche {} sans dates au plus tard", task.uid()),
            });
        }
        let node = info.exit.and_then(|v| self.net.node(v));
        let (role, manual, duration, percent_complete) = if info.role == Role::Summary {
            let duration = node
                .and_then(|n| self.net.scale(n.place))
                .and_then(|s| s.between(early.start, early.finish).ok());
            if duration.is_none() {
                degraded = true;
            }
            (
                TaskRole::Summary,
                false,
                duration.unwrap_or(Minutes::ZERO),
                self.summary_percent(p, info),
            )
        } else {
            (
                TaskRole::Leaf,
                node.is_some_and(|n| n.fixed.is_some()),
                node.map_or(task.duration.minutes(), |n| n.duration),
                task.progress.percent_complete,
            )
        };
        TaskOutcome::Scheduled(TaskSchedule {
            role,
            manual,
            start: early.start,
            finish: early.finish,
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

    /// R-055 : % achevé d'une récapitulative,
    /// `round_div(Σ réel × 10 000, Σ durée)` sur ses feuilles descendantes
    /// actives planifiées (récapitulatives intermédiaires exclues), réel =
    /// durée × % (R-010) ; Σ durée nulle : moyenne arrondie des % ; aucune
    /// feuille : 0.
    fn summary_percent(&self, p: usize, info: &TaskInfo) -> Percent {
        let (mut actual, mut total, mut pct_sum, mut count) = (0i128, 0i128, 0i128, 0i128);
        let first = p.saturating_add(1);
        for q in first..info.end {
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
