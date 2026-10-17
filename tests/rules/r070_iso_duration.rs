//! R-070 — Durée ISO 8601 → minutes (import MSPDI).

use planrs_mspdi::scalar::{IsoDuration, IsoDurationError, parse_iso_duration};

fn minutes(s: &str) -> i64 {
    parse_iso_duration(s).unwrap().minutes
}

#[test]
fn r070_hours_minutes_days() {
    // 80 h = 4 800 min (10 jours de 480) ; 8 h 30 = 510.
    assert_eq!(minutes("PT80H0M0S"), 4_800);
    assert_eq!(minutes("PT8H30M0S"), 510);
    assert_eq!(minutes("PT0H0M0S"), 0);
    // Un jour ISO vaut 24 h, pas `MinutesPerDay` : P2D = 2 880.
    assert_eq!(minutes("P2D"), 2_880);
}

#[test]
fn r070_decimals_in_last_component() {
    // 1,5 h = 90 min ; décimale ailleurs que dans la dernière composante : refus.
    assert_eq!(minutes("PT1.5H"), 90);
    assert_eq!(parse_iso_duration("PT1.5H30M"), Err(IsoDurationError::Syntax));
}

#[test]
fn r070_seconds_rounded_by_r001() {
    // 30 s = 0,5 min → 1 (moitié loin de zéro), arrondi signalé.
    assert_eq!(
        parse_iso_duration("PT0H0M30S"),
        Ok(IsoDuration {
            minutes: 1,
            rounded: true
        })
    );
    // 29 s → 0, arrondi signalé.
    assert_eq!(
        parse_iso_duration("PT0H0M29S"),
        Ok(IsoDuration {
            minutes: 0,
            rounded: true
        })
    );
}

#[test]
fn r070_years_and_months_refused() {
    // Longueur variable : refusés (l'import applique une durée nulle avec un
    // diagnostic `mspdi.invalid_value`).
    assert_eq!(parse_iso_duration("P1M"), Err(IsoDurationError::YearsOrMonths));
    assert_eq!(parse_iso_duration("P1Y"), Err(IsoDurationError::YearsOrMonths));
}

#[test]
fn r070_negative_is_read_signed() {
    // −8 h se lit −480 ; c'est la tâche qui refuse une durée négative.
    assert_eq!(minutes("-PT8H"), -480);
}
