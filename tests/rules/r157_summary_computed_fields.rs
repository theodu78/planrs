//! R-157 — Champs calculés d'une récapitulative ; durée nulle ⇒ jalon ;
//! dates d'une tâche manuelle refusées.

use planrs_automation::CommandErrorKind;
use planrs_automation::error::codes;
use planrs_core::{SchedulingMode, Subject, TaskUid};
use planrs_tests::mspdi::new_project;
use planrs_tests::session::{push_tasks, session};
use planrs_tests::task_edit::{apply, get, push_outline, try_apply};
use serde_json::json;

#[test]
fn r157_summary_duration_refused() {
    // [1 P(1), 2 a(2)] : `update 1 duration 3j` → task.computed_field.
    let mut p = new_project("2026-11-02T08:00");
    push_outline(&mut p, &[(1, 1), (2, 2)]);
    let mut s = session(p);
    let before = s.project().clone();
    for (field, value) in [
        ("duration", json!({ "text": "3j" })),
        ("start", json!("2026-11-04T08:00")),
        ("finish", json!("2026-11-06T17:00")),
    ] {
        let mut op = json!({ "op": "task.update", "uid": 1 });
        op[field] = value;
        let err = try_apply(&mut s, json!([op])).unwrap_err();
        assert_eq!(err.code, codes::TASK_COMPUTED_FIELD, "{field}");
        assert_eq!(err.kind, CommandErrorKind::ComputedField);
        assert_eq!(err.field.as_deref(), Some(field));
        assert_eq!(err.subject, Some(Subject::Task(TaskUid::new(1))));
    }
    assert_eq!(s.project(), &before);
    // Nom et notes d'une récapitulative : modifiables.
    apply(
        &mut s,
        json!([{ "op": "task.update", "uid": 1, "name": "Lot 1", "notes": "n" }]),
    );
    assert_eq!(get(&s, 1).name, "Lot 1");
}

#[test]
fn r157_zero_duration_milestone() {
    let mut p = new_project("2026-11-02T08:00");
    push_tasks(&mut p, &[1], 3);
    let mut s = session(p);
    // Ajout de durée nulle : jalon.
    apply(
        &mut s,
        json!([{ "op": "task.add", "name": "J", "duration": { "minutes": 0 } }]),
    );
    assert!(get(&s, 2).milestone);
    // Durée passée à 0 : jalon.
    apply(
        &mut s,
        json!([{ "op": "task.update", "uid": 1, "duration": { "text": "0j" } }]),
    );
    assert!(get(&s, 1).milestone);
    // `milestone:true` sur une durée > 0 : drapeau posé, durée gardée.
    apply(
        &mut s,
        json!([{ "op": "task.update", "uid": 1, "duration": { "text": "2j" }, "milestone": false }]),
    );
    assert!(!get(&s, 1).milestone);
    apply(
        &mut s,
        json!([{ "op": "task.update", "uid": 1, "milestone": true }]),
    );
    assert!(get(&s, 1).milestone);
    assert_eq!(get(&s, 1).duration.minutes().get(), 960);
    // `milestone:false` explicite l'emporte sur la durée nulle.
    apply(
        &mut s,
        json!([{ "op": "task.add", "name": "K", "duration": { "minutes": 0 }, "milestone": false }]),
    );
    assert!(!get(&s, 3).milestone);
}

#[test]
fn r157_manual_dates_refused() {
    let mut p = new_project("2026-11-02T08:00");
    push_tasks(&mut p, &[1], 1);
    p.tasks.get_mut(TaskUid::new(1)).unwrap().mode = SchedulingMode::Manual {
        start: None,
        finish: None,
        duration: None,
    };
    let mut s = session(p);
    for (field, value) in [
        ("start", json!("2026-11-04T08:00")),
        ("finish", json!(null)),
        ("duration", json!({ "text": "2j" })),
    ] {
        let mut op = json!({ "op": "task.update", "uid": 1 });
        op[field] = value;
        let err = try_apply(&mut s, json!([op])).unwrap_err();
        assert_eq!(err.code, codes::TASK_MANUAL_UNSUPPORTED, "{field}");
        assert_eq!(err.field.as_deref(), Some(field));
    }
    // Le nom reste modifiable.
    apply(&mut s, json!([{ "op": "task.update", "uid": 1, "name": "M" }]));
    assert_eq!(get(&s, 1).name, "M");
}
