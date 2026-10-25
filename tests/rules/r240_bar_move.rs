//! R-240 — Déplacer une barre : début planifié + n × 1 440 min (jours
//! calendaires, heure gardée) ⇒ `task.update { start }`, qui pose la
//! contrainte de la saisie (R-158 : SNET depuis le début, SNLT à rebours) ;
//! n = 0 ⇒ aucun lot ; tâche commencée ou récapitulative ⇒
//! `gesture.not_draggable` ; une date chômée passe telle quelle, normalisée
//! par le moteur (R-090). Repère : Standard, début lun 19/10/2026 08:00.

use planrs_automation::error::codes;
use planrs_automation::gesture::{self, GestureError, GestureKind, NotDraggable};
use planrs_automation::{ApplyMode, Batch, Session};
use planrs_core::{
    BuiltinCalendarNames, CalendarSet, Constraint, Project, ProjectInit, ScheduleAnchor, SchedulingMode,
    TaskUid,
};
use planrs_tests::moment;
use planrs_tests::resource_edit::with_tasks;
use planrs_tests::session::{push_tasks, session};
use planrs_tests::task_edit::{apply, get, try_apply};
use serde_json::json;

const T: TaskUid = TaskUid::new(1);

fn commit(s: &mut Session, b: &Batch) {
    s.apply(b, ApplyMode::Commit).unwrap();
}

fn dates(s: &Session, uid: TaskUid) -> (planrs_core::Moment, planrs_core::Moment) {
    let t = s.schedule().scheduled(uid).unwrap();
    (t.start, t.finish)
}

#[test]
fn r240_move_sets_snet() {
    // T 2 j lun 19/10 08:00 – mar 20/10 17:00 ; n = 3 ⇒ début jeu 22/10
    // 08:00, SNET 22/10 08:00, fin ven 23/10 17:00.
    let mut s = with_tasks(&[("T", "2j")]);
    assert_eq!(
        dates(&s, T),
        (moment("2026-10-19T08:00"), moment("2026-10-20T17:00"))
    );
    let b = gesture::bar_move(&s, T, 3).unwrap().unwrap();
    assert_eq!(b.ops.len(), 1);
    assert_eq!(b.ops[0].id(), "task.update");
    let set = CalendarSet::compile(s.project()).0;
    let p = gesture::preview(&s, &set, T, GestureKind::Move, 3).unwrap();
    commit(&mut s, &b);
    assert_eq!(
        get(&s, 1).constraint,
        Constraint::StartNoEarlierThan {
            date: moment("2026-10-22T08:00")
        }
    );
    assert_eq!(
        dates(&s, T),
        (moment("2026-10-22T08:00"), moment("2026-10-23T17:00"))
    );
    assert_eq!((p.start, p.finish), dates(&s, T));
}

#[test]
fn r240_move_backward_project_snlt() {
    let mut p = Project::new(ProjectInit {
        name: "P".to_owned(),
        anchor: ScheduleAnchor::FromFinish(moment("2026-11-30T17:00")),
        current_date: moment("2026-10-17T08:00"),
        calendar_names: BuiltinCalendarNames::default(),
    });
    push_tasks(&mut p, &[1], 1);
    p.tasks.get_mut(T).unwrap().constraint = Constraint::AsLateAsPossible;
    let mut s = session(p);
    let (start, _) = dates(&s, T);
    assert_eq!(start, moment("2026-11-30T08:00"));
    let b = gesture::bar_move(&s, T, -7).unwrap().unwrap();
    commit(&mut s, &b);
    assert_eq!(
        get(&s, 1).constraint,
        Constraint::StartNoLaterThan {
            date: moment("2026-11-23T08:00")
        }
    );
    assert_eq!(dates(&s, T).0, moment("2026-11-23T08:00"));
}

#[test]
fn r240_move_zero_no_batch() {
    let s = with_tasks(&[("T", "2j")]);
    assert_eq!(gesture::bar_move(&s, T, 0).unwrap(), None);
    // Un jalon se déplace aussi.
    let s = with_tasks(&[("J", "0j")]);
    assert!(get(&s, 1).milestone);
    assert!(gesture::bar_move(&s, T, 1).unwrap().is_some());
}

#[test]
fn r240_started_not_draggable() {
    let mut s = with_tasks(&[("T", "2j"), ("S", "1j"), ("A", "1j")]);
    apply(
        &mut s,
        json!([
            { "op": "progress.update", "task": 1, "percent_complete": { "text": "50%" } },
            { "op": "task.indent", "uid": 3 }
        ]),
    );
    let err = gesture::bar_move(&s, T, 1).unwrap_err();
    assert_eq!(err, GestureError::NotDraggable(T, NotDraggable::Started));
    assert_eq!(err.code(), codes::GESTURE_NOT_DRAGGABLE);
    // S (UID 2) est devenue récapitulative de A.
    assert_eq!(
        gesture::bar_move(&s, TaskUid::new(2), 1).unwrap_err(),
        GestureError::NotDraggable(TaskUid::new(2), NotDraggable::Summary)
    );
    // Tâche inactive : exclue du calcul.
    apply(
        &mut s,
        json!([{ "op": "task.update", "uid": 3, "active": false }]),
    );
    assert_eq!(
        gesture::bar_move(&s, TaskUid::new(3), 1).unwrap_err(),
        GestureError::NotDraggable(TaskUid::new(3), NotDraggable::Excluded)
    );
    assert_eq!(
        gesture::bar_move(&s, TaskUid::new(99), 1).unwrap_err(),
        GestureError::UnknownTask(TaskUid::new(99))
    );
}

#[test]
fn r240_weekend_normalized_by_engine() {
    // n = 5 ⇒ sam 24/10 08:00 passé tel quel, normalisé par le moteur
    // (R-090) en lun 26/10 08:00 ; l'aperçu le dit aussi.
    let mut s = with_tasks(&[("T", "2j")]);
    let set = CalendarSet::compile(s.project()).0;
    let p = gesture::preview(&s, &set, T, GestureKind::Move, 5).unwrap();
    let b = gesture::bar_move(&s, T, 5).unwrap().unwrap();
    commit(&mut s, &b);
    assert_eq!(
        get(&s, 1).constraint,
        Constraint::StartNoEarlierThan {
            date: moment("2026-10-24T08:00")
        }
    );
    assert_eq!(
        dates(&s, T),
        (moment("2026-10-26T08:00"), moment("2026-10-27T17:00"))
    );
    assert_eq!((p.start, p.finish), dates(&s, T));
}

#[test]
fn r240_manual_refused_by_registry() {
    // Écart au cahier (« date saisie d'une manuelle ») : R-157 refuse un
    // début sur une tâche manuelle ; le geste rend le lot, le registre le
    // refuse (bandeau dans l'interface), rien ne change.
    let mut s = with_tasks(&[("M", "2j")]);
    let mut p = s.project().clone();
    p.tasks.get_mut(T).unwrap().mode = SchedulingMode::Manual {
        start: Some(moment("2026-10-19T08:00")),
        finish: None,
        duration: None,
    };
    s = session(p);
    let b = gesture::bar_move(&s, T, 1).unwrap().unwrap();
    let before = s.project().clone();
    let err = s.apply(&b, ApplyMode::Commit).unwrap_err();
    assert_eq!(err.code, codes::TASK_MANUAL_UNSUPPORTED);
    assert_eq!(s.project(), &before);
    // Le même lot écrit à la main est refusé de la même façon.
    assert!(
        try_apply(
            &mut s,
            json!([{ "op": "task.update", "uid": 1, "start": "2026-10-20T08:00" }])
        )
        .is_err()
    );
}

#[test]
fn r240_overflow_refused() {
    let s = with_tasks(&[("T", "2j")]);
    assert_eq!(
        gesture::bar_move(&s, T, i64::MAX).unwrap_err(),
        GestureError::Overflow
    );
    assert_eq!(
        gesture::bar_move(&s, T, -1_000_000).unwrap_err(),
        GestureError::Overflow
    );
}
