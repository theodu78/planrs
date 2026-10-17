//! R-005 — Heure locale sans fuseau ni heure d'été : chaque jour dure 1 440
//! minutes. Partie « temps civil » ; le cas sur le calendrier 24 heures
//! s'ajoute avec les conversions ouvrées.

use planrs_core::Minutes;
use planrs_tests::moment;

#[test]
fn r005_dst_day_lasts_1440_minutes() {
    // 29/03/2026 : passage à l'heure d'été en France. Sans fuseau, le jour
    // dure 1 440 minutes comme les autres.
    let a = moment("2026-03-29T00:00");
    let b = moment("2026-03-30T00:00");
    assert_eq!(a.minutes_until(b), Minutes::new(1_440));
    assert_eq!(b.minutes() - a.minutes(), 1_440);
    // 02:30 existe le jour du changement d'heure (heure inexistante en France).
    assert_eq!(moment("2026-03-29T02:30").minutes() - a.minutes(), 150);
}

#[test]
fn r005_autumn_change_day_lasts_1440_minutes() {
    // 25/10/2026 : retour à l'heure d'hiver ; toujours 1 440 minutes.
    let a = moment("2026-10-25T00:00");
    assert_eq!(a.checked_add(Minutes::new(1_440)), Ok(moment("2026-10-26T00:00")));
}
