//! R-019 — Semaines de travail datées : sur leur période (jours inclus),
//! elles remplacent la semaine par défaut du même calendrier pour les jours
//! qu'elles définissent ; `Inherit` renvoie à la semaine par défaut.
//! Cas au niveau du jour.

use planrs_core::calendar::{Calendar, DayRule, WorkWeek};
use planrs_core::{Week, Weekday};
use planrs_tests::{calendars, day_minutes, days, hours, standard};

/// Standard avec une semaine datée sur `[from, to]`.
fn with_work_week(from: &str, to: &str, rule: impl Fn(Weekday) -> DayRule) -> Calendar {
    let mut c = standard();
    c.add_work_week(WorkWeek {
        name: "Été".to_owned(),
        period: days(from, to),
        days: Week::from_fn(rule),
    })
    .unwrap();
    c
}

#[test]
fn r019_summer_hours() {
    // Du 01/07/2026 au 31/08/2026, lun-ven [08:00,15:00) = 420.
    let c = with_work_week("2026-07-01", "2026-08-31", |d| match d {
        Weekday::Saturday | Weekday::Sunday => DayRule::Inherit,
        _ => DayRule::Working(hours(&[("08:00", "15:00")])),
    });
    let a = calendars(vec![c]);
    // Mar 30/06 : avant la période, 480. Mer 01/07 (borne) : 420.
    assert_eq!(day_minutes(&a, 1, "2026-06-30"), 480);
    assert_eq!(day_minutes(&a, 1, "2026-07-01"), 420);
    assert_eq!(day_minutes(&a, 1, "2026-07-06"), 420);
    // Lun 31/08 (borne) : 420 ; mar 01/09 : 480.
    assert_eq!(day_minutes(&a, 1, "2026-08-31"), 420);
    assert_eq!(day_minutes(&a, 1, "2026-09-01"), 480);
    // Samedi hérité de la semaine par défaut : chômé.
    assert_eq!(day_minutes(&a, 1, "2026-07-04"), 0);
}

#[test]
fn r019_inherit_falls_back_to_default_week() {
    // Du 01/07/2027 au 31/08/2027 : lun-jeu hérités (Standard, 480),
    // vendredi [08:00,12:00) = 240.
    let c = with_work_week("2027-07-01", "2027-08-31", |d| match d {
        Weekday::Friday => DayRule::Working(hours(&[("08:00", "12:00")])),
        _ => DayRule::Inherit,
    });
    let a = calendars(vec![c]);
    // Semaine du dim 04/07/2027 au sam 10/07 : 4 × 480 + 240 = 2 160.
    let week: i64 = [
        "2027-07-04",
        "2027-07-05",
        "2027-07-06",
        "2027-07-07",
        "2027-07-08",
        "2027-07-09",
        "2027-07-10",
    ]
    .iter()
    .map(|d| day_minutes(&a, 1, d))
    .sum();
    assert_eq!(week, 2_160);
}
