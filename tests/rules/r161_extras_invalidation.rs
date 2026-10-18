//! R-161 — Invalidation des `Extras` d'une tâche modifiée (partie tâches) :
//! les éléments MSPDI qui dépendent des dates calculées sont jetés quand ce
//! dont elles dépendent change ; le nom et les notes n'en jettent aucun.

use planrs_automation::SideEffect;
use planrs_core::{Assignment, AssignmentUid, ExtraEntry, Project, Subject, TaskUid};
use planrs_mspdi::write::WriteOptions;
use planrs_tests::mspdi::new_project;
use planrs_tests::session::{push_tasks, session};
use planrs_tests::task_edit::{apply, get};
use serde_json::json;

fn entry(key: &str, payload: &str) -> ExtraEntry {
    ExtraEntry {
        origin: "mspdi".to_owned(),
        key: key.to_owned(),
        payload: payload.to_owned(),
    }
}

/// Tâche 1 relue avec `LevelingDelay` et `Hyperlink` en Extras, une
/// affectation avec sa répartition.
fn project() -> Project {
    let mut p = new_project("2026-11-02T08:00");
    push_tasks(&mut p, &[1, 2], 2);
    let t = p.tasks.get_mut(TaskUid::new(1)).unwrap();
    t.extras.0.push(entry("Hyperlink", "<Hyperlink>plan</Hyperlink>"));
    t.extras
        .0
        .push(entry("LevelingDelay", "<LevelingDelay>4800</LevelingDelay>"));
    t.extras.0.push(entry(
        "LevelingDelayFormat",
        "<LevelingDelayFormat>7</LevelingDelayFormat>",
    ));
    let mut a = Assignment::new(AssignmentUid::new(1), TaskUid::new(1), None);
    a.extras.0.push(entry(
        "TimephasedData",
        "<TimephasedData><Type>2</Type><UID>1</UID></TimephasedData>",
    ));
    p.assignments.push(a).unwrap();
    p
}

fn mspdi(s: &planrs_automation::Session) -> String {
    String::from_utf8(s.save_bytes(&WriteOptions::default()).xml).unwrap()
}

#[test]
fn r161_duration_drops_leveling_extras() {
    let mut s = session(project());
    assert!(mspdi(&s).contains("<LevelingDelay>4800</LevelingDelay>"));
    let o = apply(
        &mut s,
        json!([{ "op": "task.update", "uid": 1, "duration": { "text": "3j" } }]),
    );
    assert!(o.side_effects.contains(&SideEffect::ExtrasDropped {
        subject: Subject::Task(TaskUid::new(1)),
        keys: vec!["LevelingDelay".to_owned(), "LevelingDelayFormat".to_owned()],
    }));
    assert!(o.side_effects.contains(&SideEffect::ExtrasDropped {
        subject: Subject::Assignment(AssignmentUid::new(1)),
        keys: vec!["TimephasedData".to_owned()],
    }));
    let keys: Vec<_> = get(&s, 1).extras.0.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, ["Hyperlink"]);
    let xml = mspdi(&s);
    assert!(!xml.contains("LevelingDelay"), "{xml}");
    assert!(xml.contains("<Hyperlink>plan</Hyperlink>"));
    // Même valeur : rien à jeter, aucun effet.
    let o = apply(
        &mut s,
        json!([{ "op": "task.update", "uid": 1, "duration": { "text": "3j" } }]),
    );
    assert!(
        o.side_effects
            .iter()
            .all(|e| !matches!(e, SideEffect::ExtrasDropped { .. }))
    );
}

#[test]
fn r161_rename_keeps_extras() {
    let mut s = session(project());
    let o = apply(
        &mut s,
        json!([{ "op": "task.update", "uid": 1, "name": "Autre", "notes": "note" }]),
    );
    assert!(
        o.side_effects
            .iter()
            .all(|e| !matches!(e, SideEffect::ExtrasDropped { .. }))
    );
    assert_eq!(get(&s, 1).extras.0.len(), 3);
    assert_eq!(s.project().assignments.iter().next().unwrap().extras.0.len(), 1);
}

#[test]
fn r161_structure_and_dates_drop_leveling_extras() {
    // Début saisi, jalon, activité, hiérarchie : mêmes clés jetées ; les
    // affectations ne perdent leur répartition que sur un changement de
    // durée.
    for op in [
        json!({ "op": "task.update", "uid": 1, "start": "2026-11-04T08:00" }),
        json!({ "op": "task.update", "uid": 1, "milestone": true }),
        json!({ "op": "task.update", "uid": 1, "active": false }),
        json!({ "op": "task.move", "uid": 1, "to": "end" }),
    ] {
        let mut s = session(project());
        let o = apply(&mut s, json!([op.clone()]));
        let keys: Vec<_> = get(&s, 1).extras.0.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["Hyperlink"], "{op}");
        assert!(
            o.side_effects.iter().any(|e| matches!(
                e,
                SideEffect::ExtrasDropped { subject: Subject::Task(u), .. } if u.get() == 1
            )),
            "{op}"
        );
        assert_eq!(
            s.project().assignments.iter().next().unwrap().extras.0.len(),
            1,
            "{op}"
        );
    }
    // Une feuille qui devient récapitulative (ajout d'un enfant) perd aussi
    // ses Extras de planning.
    let mut s = session(project());
    apply(
        &mut s,
        json!([{ "op": "task.add", "name": "Enfant", "at": { "first_child_of": 1 } }]),
    );
    let keys: Vec<_> = get(&s, 1).extras.0.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, ["Hyperlink"]);
}
