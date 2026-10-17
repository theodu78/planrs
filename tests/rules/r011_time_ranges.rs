//! R-011 — Plages horaires d'un jour : [début, fin) à la minute, 1 à 5 par
//! jour ouvré, triées, sans chevauchement ; contiguës admises. Cas au niveau
//! du jour ; les cas de conversion s'ajoutent avec les calendriers compilés.

use planrs_core::calendar::{DayHours, TimeRange};
use planrs_core::{CalendarModelError, Minutes, TimeOfDay};

fn t(h: u16) -> TimeOfDay {
    TimeOfDay::hm(h, 0).unwrap()
}

fn r(a: u16, b: u16) -> TimeRange {
    TimeRange::new(t(a), t(b)).unwrap()
}

#[test]
fn r011_two_ranges_with_lunch_break() {
    // 4 h + 4 h = 480 min.
    assert_eq!(
        DayHours::new(vec![r(8, 12), r(13, 17)]).unwrap().total(),
        Minutes::new(480)
    );
}

#[test]
fn r011_contiguous_ranges_are_allowed() {
    // [08:00,12:00) + [12:00,17:00) : 4 h + 5 h = 540 min, contiguës admises.
    let h = DayHours::new(vec![r(8, 12), r(12, 17)]).unwrap();
    assert_eq!(h.total(), Minutes::new(540));
    assert_eq!(h.ranges().len(), 2);
}

#[test]
fn r011_invalid_ranges_refused() {
    // [12:00,08:00) : début après la fin.
    assert_eq!(TimeRange::new(t(12), t(8)), Err(CalendarModelError::EmptyRange));
    // [08:00,12:00) + [11:00,13:00) : chevauchement d'une heure.
    assert_eq!(
        DayHours::new(vec![r(8, 12), r(11, 13)]),
        Err(CalendarModelError::OverlappingRanges)
    );
    // 6 plages : au plus 5 (S1).
    assert_eq!(
        DayHours::new(vec![r(0, 1), r(2, 3), r(4, 5), r(6, 7), r(8, 9), r(10, 11)]),
        Err(CalendarModelError::TooManyRanges { count: 6 })
    );
    assert_eq!(DayHours::new(vec![]), Err(CalendarModelError::NoRanges));
}

#[test]
fn r011_full_day_until_midnight() {
    // [00:00,24:00) = 24 × 60 = 1 440 min.
    let full = TimeRange::new(TimeOfDay::MIDNIGHT, TimeOfDay::END_OF_DAY).unwrap();
    assert_eq!(DayHours::new(vec![full]).unwrap().total(), Minutes::new(1_440));
}
