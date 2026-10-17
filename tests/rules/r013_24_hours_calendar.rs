//! R-013 — Calendrier 24 heures : tous les jours 00:00-24:00. Cas au niveau du
//! jour et de la semaine ; les cas de conversion s'ajoutent avec les
//! calendriers compilés.

use planrs_core::calendar::{CalendarKind, twenty_four_hours};
use planrs_core::{CalendarUid, Weekday};

#[test]
fn r013_24_hours_week() {
    let c = twenty_four_hours(CalendarUid::new(2), "24 heures".to_owned());
    let CalendarKind::Base { week } = c.kind() else {
        panic!("24 heures est un calendrier de base");
    };
    // Chaque jour : 1 440 ; semaine : 7 × 1 440 = 10 080.
    for d in Weekday::ALL {
        assert_eq!(week.get(d).total().get(), 1_440, "{d:?}");
    }
    let total: i64 = Weekday::ALL.iter().map(|d| week.get(*d).total().get()).sum();
    assert_eq!(total, 10_080);
}
