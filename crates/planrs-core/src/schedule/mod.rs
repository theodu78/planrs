//! Moteur de planification : chemin critique (ADR-0009, spécification J2
//! § 3, règles R-035 à R-069).
//!
//! [`schedule`] est une fonction pure du projet : aucune horloge, aucune
//! entrée/sortie, aucun état gardé entre deux appels. Elle n'échoue jamais et
//! ne panique jamais : toute anomalie devient un diagnostic, et une tâche
//! qu'on ne sait pas placer est exclue avec sa raison (règles 3 et 10).
//!
//! Étapes :
//! 0. calendriers compilés, replis (R-066), table des échelles ;
//! 1. à 4. réseau ([`network`]) : hiérarchie (R-052), classification
//!    (manuelles R-063, inactives R-062), nœuds canoniques, liens effectifs
//!    (R-059), adjacence CSR ;
//! 5. ordre topologique et cycles ([`order`], R-035, R-065) ;
//! 6. passe avant ([`forward`], R-036 à R-045, R-069) ;
//! 7. passe arrière ([`backward`], R-046, R-047, R-051) ;
//! 8. marges et critique ([`slack`], R-048 à R-050) ;
//! 9. agrégats des récapitulatives et sortie ([`rollup`]).
//!
//! Les décalages passent tous par [`transfer`] (transfert avant `g`,
//! adjoint `h` en passe arrière, R-047).

mod backward;
mod constraint;
mod forward;
mod network;
mod order;
mod rollup;
mod slack;
mod transfer;
mod types;

use std::collections::BTreeMap;

pub use crate::diagnostic::Pass;
pub use types::{Driver, Exclusion, LateDriver, Schedule, TaskOutcome, TaskRole, TaskSchedule};

use crate::arena::Arena;
use crate::calendar::{CalendarSet, CompiledCalendar, Span, builtin};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::duration::Minutes;
use crate::ids::TaskUid;
use crate::model::{Project, ScheduleAnchor};
use crate::time::Moment;
use constraint::{Mode, PassRole};
use network::{Fail, Network, NodeIx, pos};

/// État calculé d'un nœud canonique.
#[derive(Clone, Copy, Debug)]
struct NodeState {
    /// Dates au plus tôt ; porte : `(G, G)` ; agrégat : (min ES, max EF),
    /// ou les dates saisies d'une récapitulative manuelle datée (R-064).
    early: Option<Span>,
    /// Agrégat : (min ES, max EF) des enfants, même pour une récapitulative
    /// manuelle datée (début réel des sous-tâches, lu par la marge libre
    /// d'un lien vers sa porte).
    children: Option<Span>,
    /// Dates au plus tard ; porte : `(LG, LG)` ; agrégat : (min LS, max LF).
    late: Option<Span>,
    /// Cause de l'ES (R-044).
    driver: Driver,
    /// Dates de repli (R-069).
    degraded: bool,
    /// Conversion impossible en passe avant (R-069).
    failed: bool,
    /// Dates au plus tard et marges de repli : elles dépendent d'un
    /// successeur dégradé ou non planifiable (R-069, propagé vers l'amont).
    late_degraded: bool,
    /// Cause de la fin au plus tard (R-047).
    late_driver: LateDriver,
    /// Agrégat : plafond de fin hérité `C(S)` (R-057) et la récapitulative
    /// qui l'a posé.
    cap_finish: Option<(Moment, TaskUid)>,
    /// Agrégat : plafond de début hérité `K(S)` (R-057, A11) et la
    /// récapitulative qui l'a posé.
    cap_start: Option<(Moment, TaskUid)>,
    /// Épingle d'une tâche à placement opposé (ALAP depuis le début,
    /// R-092) : dates fixes de la repasse.
    pin: Option<Span>,
    /// Marge totale (R-048).
    total_slack: Minutes,
    /// Marge libre (R-049).
    free_slack: Minutes,
}

impl NodeState {
    const NEW: Self = Self {
        early: None,
        children: None,
        late: None,
        driver: Driver::ProjectStart,
        degraded: false,
        failed: false,
        late_degraded: false,
        late_driver: LateDriver::ProjectFinish,
        cap_finish: None,
        cap_start: None,
        pin: None,
        total_slack: Minutes::ZERO,
        free_slack: Minutes::ZERO,
    };
}

/// Calcul en cours : réseau, états des nœuds, décalages convertis.
struct Calc<'n, 'a> {
    net: &'n Network<'a>,
    project: &'n Project,
    /// Sens de planification.
    mode: Mode,
    /// Rôle de la passe en cours.
    role: PassRole,
    /// Repasse (R-092) : aucun diagnostic, sauf `schedule.internal`.
    quiet: bool,
    /// Début du projet (brut, R-036).
    start: Moment,
    /// Fin du projet (R-045), connue après la passe avant ; plafond commun
    /// de la passe arrière (R-046).
    finish: Moment,
    /// États, par nœud.
    st: Vec<NodeState>,
    /// Décalage converti de chaque arc de lien, calculé une fois (R-043).
    lag: Vec<Option<Minutes>>,
    /// Arcs de lien vers une porte dont la borne n'a pas pu se calculer
    /// (`link.unschedulable`, R-069) : non appliqués, ils ne plafonnent rien
    /// et ne comptent ni pour la marge libre ni pour R-051.
    dropped: Vec<bool>,
    diags: Diagnostics,
}

impl Calc<'_, '_> {
    /// État du nœud `v`.
    fn state(&self, v: NodeIx) -> Option<&NodeState> {
        self.st.get(pos(v)?)
    }

    /// Le nœud `v` a été atteint par la passe principale : ses dates
    /// servent de borne à ses voisins dans la passe secondaire (depuis le
    /// début : il a des dates au plus tôt). Remplace les tests directs sur
    /// `early` de la passe arrière (ADR-0010).
    fn reached(&self, v: NodeIx) -> bool {
        match self.mode {
            Mode::FromStart | Mode::FromFinish => self.state(v).is_some_and(|s| s.early.is_some()),
        }
    }

    /// Le plafond de début hérité `K(parent)` s'applique au nœud de début
    /// au plus tôt `es` (R-057, A11) : depuis le début, aux seuls nœuds de
    /// même début que la récapitulative ; à rebours, à tous (D10, ADR-0010).
    fn k_applies(&self, parent_start: Option<Moment>, es: Moment) -> bool {
        match self.mode {
            Mode::FromStart => parent_start == Some(es),
            Mode::FromFinish => true,
        }
    }

    /// État modifiable du nœud `v`.
    fn state_mut(&mut self, v: NodeIx) -> Option<&mut NodeState> {
        self.st.get_mut(pos(v)?)
    }

    /// Cible d'arc peu sûre pour la passe arrière et la marge libre (R-069) :
    /// nœud dégradé, en échec ou aux dates au plus tard de repli ; pour une
    /// porte, aussi l'agrégat de ses sous-tâches.
    fn unreliable(&self, v: NodeIx) -> bool {
        let bad = |s: &NodeState| s.degraded || s.failed || s.late_degraded;
        let Some(s) = self.state(v) else { return true };
        let twin = self
            .net
            .node(v)
            .filter(|n| n.kind == network::NodeKind::Gate)
            .and_then(|n| n.twin)
            .and_then(|r| self.state(r));
        bad(s) || twin.is_some_and(bad)
    }

    /// Arc `a` non appliqué en passe avant (R-069).
    fn is_dropped(&self, a: u32) -> bool {
        pos(a).and_then(|i| self.dropped.get(i)).copied().unwrap_or(false)
    }

    /// Ajoute un diagnostic ; en repasse (`quiet`), seul un invariant violé
    /// (`schedule.internal`) est gardé : la passe principale a déjà tout
    /// signalé.
    fn emit(&mut self, d: Diagnostic) {
        if !self.quiet || matches!(d, Diagnostic::ScheduleInternal { .. }) {
            self.diags.push(d);
        }
    }

    /// Signale l'échec du calcul de la tâche `uid` dans la passe `pass`. En
    /// repasse, un échec est un invariant violé (la passe principale a
    /// réussi sur ce nœud).
    fn report(&mut self, uid: TaskUid, pass: Pass, fail: Fail) {
        if self.quiet {
            let detail = match fail {
                Fail::Conversion(cause) => cause,
                Fail::Internal(detail) => detail,
            };
            self.diags.push(Diagnostic::ScheduleInternal {
                detail: format!("tâche {uid} : échec en repasse ({detail})"),
            });
            return;
        }
        match fail {
            Fail::Conversion(cause) => self.diags.push(Diagnostic::TaskUnschedulable {
                task: uid,
                pass,
                cause: cause.to_owned(),
            }),
            Fail::Internal(detail) => self.diags.push(Diagnostic::ScheduleInternal {
                detail: format!("tâche {uid} : {detail}"),
            }),
        }
    }

    /// Minutes du décalage de l'arc `a` (R-043 : pourcentage de la durée du
    /// prédécesseur, durée d'une récapitulative par R-054), mémorisées.
    fn lag_minutes(&mut self, a: u32) -> Result<Minutes, Fail> {
        let k = pos(a).ok_or(Fail::Internal("arc hors bornes"))?;
        if let Some(Some(m)) = self.lag.get(k) {
            return Ok(*m);
        }
        let arc = self.net.arc(a).ok_or(Fail::Internal("arc hors bornes"))?;
        let link = arc
            .link
            .ok_or(Fail::Internal("arc de hiérarchie sans décalage"))?;
        let m = match link.lag {
            network::LagSpec::Time(m) => m,
            network::LagSpec::Percent(pct) => crate::duration::percent_of(self.duration_of(arc.from)?, pct)?,
        };
        if let Some(slot) = self.lag.get_mut(k) {
            *slot = Some(m);
        }
        Ok(m)
    }

    /// Ancre de fin d'un successeur, avant le transfert arrière d'un lien FF
    /// ou SF (R-047) : le dernier instant de même temps ouvré sur l'échelle
    /// de placement du successeur (`next_working_moment`, identité en temps
    /// écoulé). La contrainte FF/SF se lit en temps ouvré de cette échelle
    /// (R-039) : sans ce report, un décalage écoulé vers une feuille ouvrée
    /// donnerait une marge négative sans contrainte (fin du successeur
    /// ramenée à 17:00 alors que la borne tombait à 17:06). Plus aucun temps
    /// ouvré après `y` (fin du domaine) : tout instant jusqu'à la fin du
    /// domaine a le même temps ouvré, l'ancre est `Moment::MAX`.
    ///
    /// Jalon (R-031) : une borne de fin `b` le place en `b` si `b` commence
    /// une minute ouvrée, sinon à l'instant ouvré précédent. Il ne dépasse
    /// donc pas `y` tant que `b < next_working_moment(y)` : l'ancre est `y`
    /// s'il commence une minute ouvrée, sinon `next_working_moment(y) − 1
    /// min` (et non `next_working_moment(y)`, qui le placerait au début de la
    /// plage suivante).
    fn finish_anchor(&self, target: NodeIx, y: Moment) -> Moment {
        let Some(node) = self.net.node(target) else {
            return y;
        };
        let milestone = node.duration == Minutes::ZERO;
        match self.net.scale(node.place) {
            Some(crate::calendar::TimeScale::Working(c)) => match c.next_working_moment(y) {
                Ok(next) if milestone && next != y => next.checked_add(Minutes::new(-1)).unwrap_or(y),
                Ok(next) => next,
                Err(_) => Moment::MAX,
            },
            _ => y,
        }
    }

    /// R-047 : la cible `target` d'un lien d'échelle `sigma` lit sa borne en
    /// `Moment` brut et non en temps ouvré de `σ` : porte d'une
    /// récapitulative (borne non recalée), jalon (R-031 : il reste sur une
    /// fin de minute ouvrée mais avance au début de la minute ouvrée
    /// suivante sinon), feuille placée sur une autre échelle (calendrier
    /// propre différent, durée écoulée). Le transfert arrière doit alors
    /// être l'adjoint en brut ([`transfer::backward`], `raw`). Une feuille de
    /// durée > 0 placée sur `σ` commence à `next_working(borne)`, qui ne
    /// dépend que du temps ouvré de la borne.
    fn reads_raw(&self, target: NodeIx, sigma: u32) -> bool {
        let Some(node) = self.net.node(target) else {
            return true;
        };
        if node.kind == network::NodeKind::Gate || node.duration == Minutes::ZERO {
            return true;
        }
        if node.place == sigma {
            return false;
        }
        match (self.net.scale(node.place), self.net.scale(sigma)) {
            (Some(crate::calendar::TimeScale::Working(a)), Some(crate::calendar::TimeScale::Working(b))) => {
                !std::ptr::eq(*a, *b)
            }
            _ => true,
        }
    }

    /// Durée d'un nœud prédécesseur : feuille (manuelle : durée utilisée) ;
    /// agrégat : temps ouvré entre son début et sa fin sur le calendrier de
    /// la récapitulative (R-054) ; récapitulative manuelle datée : durée
    /// utilisée (R-064).
    fn duration_of(&self, v: NodeIx) -> Result<Minutes, Fail> {
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        if node.fixed.is_some() {
            return Ok(node.duration);
        }
        match node.kind {
            network::NodeKind::Leaf => Ok(node.duration),
            network::NodeKind::Rollup => {
                let e = self
                    .state(v)
                    .and_then(|s| s.early)
                    .ok_or(Fail::Internal("agrégat sans dates"))?;
                let scale = self
                    .net
                    .scale(node.place)
                    .ok_or(Fail::Internal("échelle hors bornes"))?;
                Ok(scale.between(e.start, e.finish)?)
            }
            network::NodeKind::Gate => Err(Fail::Internal("porte prise comme prédécesseur")),
        }
    }
}

/// Calcule le planning complet du projet.
///
/// Fonction pure : aucune horloge, aucune entrée/sortie. N'échoue jamais, ne
/// panique jamais : toute anomalie devient un diagnostic. Les calendriers
/// sont compilés à chaque appel.
#[must_use]
pub fn schedule(project: &Project) -> Schedule {
    let (mode, start) = match project.anchor {
        ScheduleAnchor::FromStart(m) => (Mode::FromStart, m),
        ScheduleAnchor::FromFinish(m) => (Mode::FromFinish, m),
    };
    if mode == Mode::FromFinish {
        return from_finish(project, start);
    }
    let mut diags = Diagnostics::new();
    // Étape 0 : calendriers et replis (R-066).
    let (set, compile_diags) = CalendarSet::compile(project);
    diags.extend(compile_diags);
    let fallback = match set.get(project.calendar) {
        Ok(c) if c.has_working_time() => None,
        _ => {
            diags.push(Diagnostic::ProjectCalendarFallback {
                calendar: project.calendar,
            });
            Some(compile_builtin_standard(project))
        }
    };
    let project_cal: Option<&CompiledCalendar> = match &fallback {
        Some(c) => c.as_ref(),
        None => set.get(project.calendar).ok(),
    };
    let Some(project_cal) = project_cal else {
        return internal_failure(
            project,
            start,
            diags,
            "calendrier Standard intégré non compilable",
        );
    };
    let sc = network::scales(project, &set, project_cal, &mut diags);
    // Étapes 1 à 5.
    let Some(mut net) = network::build(project, sc, mode, &mut diags) else {
        return internal_failure(project, start, diags, "construction du réseau");
    };
    let Some(order) = order::order(&mut net, &mut diags) else {
        return internal_failure(project, start, diags, "ordre topologique");
    };
    let mut calc = Calc {
        net: &net,
        project,
        mode,
        role: PassRole::Primary,
        quiet: false,
        start,
        finish: start,
        st: vec![NodeState::NEW; net.nodes.len()],
        lag: vec![None; net.arcs.len()],
        dropped: vec![false; net.arcs.len()],
        diags,
    };
    // Étapes 6 à 9.
    calc.forward(&order, PassRole::Primary);
    calc.backward(&order, PassRole::Secondary);
    // R-092 : tâches ALAP épinglées sur leurs dates au plus tard, puis une
    // seule repasse de la passe avant, seulement s'il y a une épingle.
    if calc.pin_opposite() {
        calc.forward(&order, PassRole::Replay);
    }
    calc.summary_late();
    calc.slack();
    calc.check_constraints();
    calc.output(&order)
}

/// Calendrier Standard intégré, compilé seul, sous l'UID du calendrier du
/// projet (repli de R-066).
fn compile_builtin_standard(project: &Project) -> Option<CompiledCalendar> {
    let standard = builtin::standard(project.calendar, "Standard".to_owned());
    let arena = Arena::try_from(vec![standard]).ok()?;
    CompiledCalendar::compile(&arena, project.calendar).ok()
}

/// Planification à rebours (J3) : toutes les tâches non vides sont exclues,
/// avec `schedule.from_finish_unsupported` (A13). Jamais de fausses dates.
fn from_finish(project: &Project, at: Moment) -> Schedule {
    let tasks = project
        .tasks
        .iter()
        .map(|t| {
            let reason = if t.is_null {
                Exclusion::NullRow
            } else {
                Exclusion::UnsupportedAnchor
            };
            (
                t.uid(),
                TaskOutcome::Excluded {
                    reason,
                    display: None,
                },
            )
        })
        .collect();
    let mut diags = Diagnostics::new();
    diags.push(Diagnostic::ScheduleFromFinishUnsupported);
    Schedule {
        project_start: at,
        project_finish: at,
        project_summary: None,
        tasks,
        order: Vec::new(),
        diagnostics: diags.finish(),
    }
}

/// Invariant interne violé avant les passes : `schedule.internal`, toutes
/// les tâches non vides exclues comme non planifiables.
fn internal_failure(project: &Project, start: Moment, mut diags: Diagnostics, detail: &str) -> Schedule {
    diags.push(Diagnostic::ScheduleInternal {
        detail: detail.to_owned(),
    });
    let tasks: BTreeMap<_, _> = project
        .tasks
        .iter()
        .map(|t| {
            let reason = if t.is_null {
                Exclusion::NullRow
            } else {
                Exclusion::Unschedulable
            };
            (
                t.uid(),
                TaskOutcome::Excluded {
                    reason,
                    display: None,
                },
            )
        })
        .collect();
    Schedule {
        project_start: start,
        project_finish: start,
        project_summary: None,
        tasks,
        order: Vec::new(),
        diagnostics: diags.finish(),
    }
}

#[cfg(test)]
mod tests {
    //! Mécanique des portes et des agrégats (cas de la spécification J2 ;
    //! les fiches R-053 à R-060 ont leurs tests dans `tests/rules/`).
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::{Driver, Schedule, TaskRole, TaskSchedule, schedule};
    use crate::calendar::BuiltinCalendarNames;
    use crate::duration::{Duration, DurationUnit, Lag, LagValue, Minutes, TimeBasis};
    use crate::ids::TaskUid;
    use crate::model::{Link, LinkType, OutlineLevel, Project, ProjectInit, ScheduleAnchor, Task};
    use crate::quantity::Percent;
    use crate::time::Moment;

    fn m(s: &str) -> Moment {
        s.parse().unwrap()
    }

    /// Projet : (uid, niveau, minutes), liens (pred, succ, type, décalage).
    fn project(tasks: &[(u32, u16, i64)], links: &[(u32, u32, LinkType, i64)]) -> Project {
        let mut p = Project::new(ProjectInit {
            name: "T".to_owned(),
            anchor: ScheduleAnchor::FromStart(m("2026-10-19T08:00")),
            current_date: m("2026-10-19T08:00"),
            calendar_names: BuiltinCalendarNames::default(),
        });
        let o = p.options;
        for &(uid, level, minutes) in tasks {
            let mut t = Task::new(TaskUid::new(uid), String::new(), &o);
            t.outline_level = OutlineLevel::new(level).unwrap();
            t.duration = Duration::new(
                Minutes::new(minutes),
                DurationUnit::Day,
                TimeBasis::Working,
                false,
            )
            .unwrap();
            p.tasks.push(t).unwrap();
        }
        for &(pred, succ, kind, lag) in links {
            let lag = Lag::new(
                TimeBasis::Working,
                LagValue::Time {
                    minutes: Minutes::new(lag),
                    unit: DurationUnit::Day,
                },
            )
            .unwrap();
            p.tasks
                .get_mut(TaskUid::new(succ))
                .unwrap()
                .predecessors
                .push(Link {
                    predecessor: TaskUid::new(pred),
                    kind,
                    lag,
                });
        }
        p
    }

    fn at(s: &Schedule, uid: u32) -> TaskSchedule {
        *s.scheduled(TaskUid::new(uid)).unwrap()
    }

    #[test]
    fn gate_bounds_all_subtasks() {
        // X (1) 2 j FS → S (2) {A (3) 1 j, B (4) 1 j} : A, B mer 08:00 –
        // 17:00, pilote : la porte de S.
        let s = schedule(&project(
            &[(1, 1, 960), (2, 1, 0), (3, 2, 480), (4, 2, 480)],
            &[(1, 2, LinkType::FinishToStart, 0)],
        ));
        for uid in [3, 4] {
            assert_eq!(at(&s, uid).early.start, m("2026-10-21T08:00"));
            assert_eq!(
                at(&s, uid).driver,
                Driver::SummaryGate {
                    summary: TaskUid::new(2)
                }
            );
        }
        let sum = at(&s, 2);
        assert_eq!(sum.role, TaskRole::Summary);
        assert_eq!(sum.driver, Driver::Rollup);
        assert_eq!(sum.early.start, m("2026-10-21T08:00"));
        assert_eq!(sum.duration, Minutes::new(480));
    }

    #[test]
    fn summary_successor_caps_all_leaves() {
        // S (1) {A (2) 1 j, B (3) 2 j} FS → Y (4) 1 j : Y mer ; A LF mar
        // 17:00, TF 480, FS 480 ; B TF 0.
        let s = schedule(&project(
            &[(1, 1, 0), (2, 2, 480), (3, 2, 960), (4, 1, 480)],
            &[(1, 4, LinkType::FinishToStart, 0)],
        ));
        assert_eq!(at(&s, 4).early.start, m("2026-10-21T08:00"));
        assert_eq!(at(&s, 2).late.finish, m("2026-10-20T17:00"));
        assert_eq!(at(&s, 2).total_slack, Minutes::new(480));
        assert_eq!(at(&s, 2).free_slack, Minutes::new(480));
        assert_eq!(at(&s, 3).total_slack, Minutes::ZERO);
    }

    #[test]
    fn summary_start_link_caps_same_start_leaves_only() {
        // S (1) {A (2) 1 j ; B (3) FS de A 1 j} SS+1 j → Y (4) 1 j :
        // Y mar 08:00 – 17:00 ; plafond lun 08:00 sur A seule →
        // TF(A) = TF(B) = TF(Y) = 0.
        let s = schedule(&project(
            &[(1, 1, 0), (2, 2, 480), (3, 2, 480), (4, 1, 480)],
            &[
                (2, 3, LinkType::FinishToStart, 0),
                (1, 4, LinkType::StartToStart, 480),
            ],
        ));
        assert_eq!(at(&s, 4).early.start, m("2026-10-20T08:00"));
        for uid in [2, 3, 4] {
            assert_eq!(at(&s, uid).total_slack, Minutes::ZERO, "{uid}");
            assert!(at(&s, uid).free_slack >= Minutes::ZERO, "{uid}");
        }
    }

    #[test]
    fn summary_slack_on_aggregated_dates() {
        // P (1) 1 j ; S (2) {A (3) 1 j sans lien, B (4) 1 j FS de P} ;
        // Q (5) 1 j FS de B. S : ES lun 08:00, EF mar 17:00, LS = mar 08:00,
        // LF = mer 17:00 → TF(S) = FS(S) = 480, non critique malgré B.
        let s = schedule(&project(
            &[(1, 1, 480), (2, 1, 0), (3, 2, 480), (4, 2, 480), (5, 1, 480)],
            &[
                (1, 4, LinkType::FinishToStart, 0),
                (4, 5, LinkType::FinishToStart, 0),
            ],
        ));
        let sum = at(&s, 2);
        assert_eq!(sum.late.start, m("2026-10-20T08:00"));
        assert_eq!(sum.late.finish, m("2026-10-21T17:00"));
        assert_eq!(sum.total_slack, Minutes::new(480));
        assert_eq!(sum.free_slack, Minutes::new(480));
        assert!(!sum.critical);
        assert!(at(&s, 4).critical);
        assert_eq!(at(&s, 3).total_slack, Minutes::new(960));
    }

    #[test]
    fn summary_percent_weighted_by_leaf_durations() {
        // S1 (1) {S2 (2) {A (3) 960 à 50 %, B (4) 480 à 0 %}, C (5) 240 à
        // 100 %} : (480 + 0 + 240) / 1 680 → 4 286.
        let mut p = project(
            &[(1, 1, 0), (2, 2, 0), (3, 3, 960), (4, 3, 480), (5, 2, 240)],
            &[],
        );
        p.tasks
            .get_mut(TaskUid::new(3))
            .unwrap()
            .progress
            .percent_complete = Percent::new(5_000).unwrap();
        p.tasks
            .get_mut(TaskUid::new(5))
            .unwrap()
            .progress
            .percent_complete = Percent::FULL;
        let s = schedule(&p);
        assert_eq!(at(&s, 1).percent_complete, Percent::new(4_286).unwrap());
    }

    #[test]
    fn from_finish_excludes_every_task() {
        let mut p = project(&[(1, 1, 480)], &[]);
        p.anchor = ScheduleAnchor::FromFinish(m("2026-12-31T17:00"));
        let s = schedule(&p);
        assert_eq!(s.project_finish, m("2026-12-31T17:00"));
        assert!(s.scheduled(TaskUid::new(1)).is_none());
        assert_eq!(s.diagnostics.len(), 1);
        assert_eq!(s.diagnostics[0].code(), "schedule.from_finish_unsupported");
    }

    #[test]
    fn schedule_json_round_trip() {
        let s = schedule(&project(
            &[(1, 1, 0), (2, 2, 480), (3, 1, 480)],
            &[(2, 3, LinkType::FinishToStart, 0)],
        ));
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains(r#""status":"scheduled""#), "{json}");
        assert!(json.contains(r#""kind":"rollup""#), "{json}");
        assert!(
            json.contains(r#""late_driver":{"kind":"project_finish"}"#),
            "{json}"
        );
        assert_eq!(serde_json::from_str::<Schedule>(&json).unwrap(), s);
    }
}
