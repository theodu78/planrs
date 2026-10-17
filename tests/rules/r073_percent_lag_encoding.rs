//! R-073 — Décalage en pourcentage : `LinkLag` = pourcentage × 10 (hypothèse
//! pure, import MSPDI).

use planrs_mspdi::scalar::percent_lag_from_link_lag;

#[test]
fn r073_percent_lag_is_link_lag_times_ten() {
    // 500 → 5 000 en 1/10 000 = 50 % ; −250 → −2 500 = −25 %.
    assert_eq!(percent_lag_from_link_lag(500), Some(5_000));
    assert_eq!(percent_lag_from_link_lag(-250), Some(-2_500));
    assert_eq!(percent_lag_from_link_lag(0), Some(0));
    // Débordement : aucune valeur (l'import signale et applique 0).
    assert_eq!(percent_lag_from_link_lag(i64::MAX), None);
}
