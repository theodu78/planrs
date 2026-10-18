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
    // UID 9 : seul `ManualStart` (mer 21/10 08:00). Amendement J3 : un
    // `Manual*` présent interdit tout repli ; ni `Finish` ni `Duration`
    // (16 h) ne sont lus comme saisis. La fin est laissée au moteur (R-063),
    // la durée de la tâche reste `Duration`.
    let doc = fixture();
    assert_eq!(manual(&doc, 9), (Some(moment("2026-10-21T08:00")), None, None));
    let t = doc.project.tasks.get(TaskUid::new(9)).unwrap();
    assert_eq!(t.duration.minutes(), Minutes::new(960));
}

#[test]
fn r085_amended_no_fallback_when_any_manual_field_is_present() {
    // Seul `ManualDuration` (PT23H0M0S = 1 380 min) : `Start` et `Finish`
    // (lun 19/10 08:00 – mar 20/10 17:00) ne sont pas lus comme saisis.
    let xml = "<Project xmlns=\"http://schemas.microsoft.com/project\">\
               <StartDate>2026-10-19T08:00:00</StartDate><CalendarUID>1</CalendarUID>\
               <Tasks><Task><UID>1</UID><ID>1</ID><Name>m</Name><Manual>1</Manual>\
               <Start>2026-10-19T08:00:00</Start><Finish>2026-10-20T17:00:00</Finish>\
               <Duration>PT16H0M0S</Duration><ManualDuration>PT23H0M0S</ManualDuration>\
               </Task></Tasks></Project>";
    let doc = read(xml.as_bytes(), &ReadOptions::new(moment("2026-01-01T00:00"))).unwrap();
    assert_eq!(manual(&doc, 1), (None, None, Some(1_380)));
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
