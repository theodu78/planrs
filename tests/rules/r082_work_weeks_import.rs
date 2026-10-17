//! R-082 — Semaines de travail datées (`WorkWeeks`, import MSPDI).

use planrs_mspdi::{MspdiDocument, ReadOptions, read};
use planrs_tests::{day_minutes, moment, tests_path};

fn fixture() -> MspdiDocument {
    let bytes = std::fs::read(tests_path("fixtures/mspdi/calendars.xml")).unwrap();
    read(&bytes, &ReadOptions::new(moment("2026-01-01T00:00"))).unwrap()
}

#[test]
fn r082_work_week_with_inherited_day() {
    // Calendrier 8 : « Winter Break » du mar 22/12 au sam 26/12/2026 23:59,
    // dimanche à vendredi chômés, samedi absent → hérité de la semaine par
    // défaut du calendrier (samedi 08:00-12:00).
    let doc = fixture();
    let cals = &doc.project.calendars;
    for d in ["2026-12-22", "2026-12-23", "2026-12-24", "2026-12-25"] {
        assert_eq!(day_minutes(cals, 8, d), 0, "{d}");
    }
    assert_eq!(day_minutes(cals, 8, "2026-12-26"), 240);
    // Avant et après la période : semaine par défaut.
    assert_eq!(day_minutes(cals, 8, "2026-12-21"), 480);
    assert_eq!(day_minutes(cals, 8, "2026-12-28"), 480);
    let calendar = cals.get(planrs_core::CalendarUid::new(8)).unwrap();
    assert_eq!(calendar.work_weeks().len(), 1);
    assert_eq!(calendar.work_weeks()[0].name, "Winter Break");
}
