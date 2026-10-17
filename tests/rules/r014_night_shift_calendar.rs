//! R-014 — Calendrier Équipe de nuit : la plage de 23:00 appartient au jour
//! qui commence. Cas au niveau du jour et de la semaine ; les cas de
//! conversion s'ajoutent avec les calendriers compilés.

use planrs_core::calendar::{CalendarKind, DaySetting, night_shift};
use planrs_core::{CalendarUid, Weekday};

fn ranges(setting: &DaySetting) -> Vec<(u16, u16)> {
    match setting {
        DaySetting::NonWorking => vec![],
        DaySetting::Working(h) => h
            .ranges()
            .iter()
            .map(|r| (r.from().minutes() / 60, r.to().minutes() / 60))
            .collect(),
    }
}

#[test]
fn r014_night_shift_week() {
    let c = night_shift(CalendarUid::new(3), "Équipe de nuit".to_owned());
    let CalendarKind::Base { week } = c.kind() else {
        panic!("Équipe de nuit est un calendrier de base");
    };
    // Dimanche chômé ; lundi 23-24 (60) ; mardi à vendredi 0-3, 4-8, 23-24
    // (180 + 240 + 60 = 480) ; samedi 0-3, 4-8 (180 + 240 = 420).
    assert_eq!(ranges(week.get(Weekday::Sunday)), []);
    assert_eq!(ranges(week.get(Weekday::Monday)), [(23, 24)]);
    for d in [
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
    ] {
        assert_eq!(ranges(week.get(d)), [(0, 3), (4, 8), (23, 24)], "{d:?}");
        assert_eq!(week.get(d).total().get(), 480);
    }
    assert_eq!(ranges(week.get(Weekday::Saturday)), [(0, 3), (4, 8)]);
    // Semaine : 60 + 4 × 480 + 420 = 2 400.
    let total: i64 = Weekday::ALL.iter().map(|d| week.get(*d).total().get()).sum();
    assert_eq!(total, 2_400);
}
