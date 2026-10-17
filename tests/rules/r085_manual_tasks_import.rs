//! R-085 — Tâches en mode manuel (import MSPDI).

use planrs_core::{Minutes, SchedulingMode, TaskUid};
use planrs_mspdi::{MspdiDocument, ReadOptions, read};
use planrs_tests::{moment, tests_path};

fn fixture() -> MspdiDocument {
    let bytes = std::fs::read(tests_path("fixtures/mspdi/manual.xml")).unwrap();
    read(&bytes, &ReadOptions::new(moment("2026-01-01T00:00"))).unwrap()
}

fn manual(
    doc: &MspdiDocument,
    uid: u32,
) -> (
    Option<planrs_core::Moment>,
    Option<planrs_core::Moment>,
    Option<i64>,
) {
    match doc.project.tasks.get(TaskUid::new(uid)).unwrap().mode {
        SchedulingMode::Manual {
            start,
            finish,
            duration,
        } => (start, finish, duration.map(|d| d.minutes().get())),
        SchedulingMode::Auto => panic!("tâche {uid} automatique"),
    }
}

#[test]
fn r085_manual_fields_win() {
    // UID 7 : `ManualStart` mer 21/10 09:00, `ManualFinish` ven 23/10 16:00,
    // `ManualDuration` 23 h (1 380 min) ; `Duration` (16 h) reste la durée
    // de la tâche.
    let doc = fixture();
    assert_eq!(
        manual(&doc, 7),
        (
            Some(moment("2026-10-21T09:00")),
            Some(moment("2026-10-23T16:00")),
            Some(1_380)
        )
    );
    let t = doc.project.tasks.get(TaskUid::new(7)).unwrap();
    assert_eq!(t.duration.minutes(), Minutes::new(960));
}

#[test]
fn r085_fallback_to_start_finish_duration() {
    // UID 8 : sans `Manual*` → `Start`, `Finish`, `Duration`.
    let doc = fixture();
    assert_eq!(
        manual(&doc, 8),
        (
            Some(moment("2026-10-26T08:00")),
            Some(moment("2026-10-27T17:00")),
            Some(960)
        )
    );
}

#[test]
fn r085_start_only() {
    // UID 9 : seul `ManualStart` (mer 21/10 08:00) ; la fin est laissée au
    // moteur (R-063), la durée vient de `Duration` (16 h).
    let doc = fixture();
    assert_eq!(
        manual(&doc, 9),
        (Some(moment("2026-10-21T08:00")), None, Some(960))
    );
}

#[test]
fn r085_manual_summary_keeps_its_dates_and_duration() {
    // UID 1 : récapitulative manuelle mar 20/10 08:00 – ven 23/10 17:00,
    // 32 h ; sa durée saisie reste dans le modèle (R-064).
    let doc = fixture();
    assert_eq!(
        manual(&doc, 1),
        (
            Some(moment("2026-10-20T08:00")),
            Some(moment("2026-10-23T17:00")),
            Some(1_920)
        )
    );
    let s = doc.project.tasks.get(TaskUid::new(1)).unwrap();
    assert_eq!(s.duration.minutes(), Minutes::new(1_920));
    // Sa sous-tâche reste automatique.
    assert_eq!(
        doc.project.tasks.get(TaskUid::new(2)).unwrap().mode,
        SchedulingMode::Auto
    );
}
