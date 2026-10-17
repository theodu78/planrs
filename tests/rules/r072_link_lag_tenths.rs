//! R-072 — `LinkLag` en dixièmes de minute → minutes (import MSPDI).

use planrs_core::Minutes;
use planrs_mspdi::scalar::tenths_to_minutes;

#[test]
fn r072_tenths_divided_by_ten() {
    // 4 800 dixièmes = 480 min (1 j de 480) ; −4 800 → −480 (avance).
    assert_eq!(tenths_to_minutes(4_800), (Minutes::new(480), false));
    assert_eq!(tenths_to_minutes(-4_800), (Minutes::new(-480), false));
    // 28 800 dixièmes = 2 880 min (2 ej).
    assert_eq!(tenths_to_minutes(28_800), (Minutes::new(2_880), false));
}

#[test]
fn r072_remainder_rounded_by_r001() {
    // 25 dixièmes = 2,5 min → 3 (moitié loin de zéro), arrondi signalé ;
    // −25 → −3 ; 24 → 2.
    assert_eq!(tenths_to_minutes(25), (Minutes::new(3), true));
    assert_eq!(tenths_to_minutes(-25), (Minutes::new(-3), true));
    assert_eq!(tenths_to_minutes(24), (Minutes::new(2), true));
}
