//! R-113 — Calendrier du projet dérivé : le calendrier du projet est un
//! calendrier de base (S41, S42). Quand `Project::calendar` désigne un
//! calendrier dérivé (calendrier de ressource), sa base mesure la durée des
//! récapitulatives (R-054), les marges (R-048, R-049) et les décalages
//! (R-041) des tâches sans calendrier propre ; le calendrier désigné les
//! place (substitut des calendriers de ressource, J5). Observé : O13 (A4,
//! 20 cas reproduits). Hypothèse pour le placement et les décalages.

use planrs_core::calendar::{Calendar, DayRule, DaySetting};
use planrs_core::{CalendarUid, LinkType, Minutes, Week, Weekday};
use planrs_tests::net::{Net, lag_w};
use planrs_tests::{hours, moment, working};

const BASE: u32 = 10;
const DERIVED: u32 = 11;

fn uid(v: u32) -> CalendarUid {
    CalendarUid::new(v)
}

fn weekdays(day: impl Fn() -> DaySetting) -> Week<DaySetting> {
    Week::from_fn(|d| match d {
        Weekday::Saturday | Weekday::Sunday => DaySetting::NonWorking,
        _ => day(),
    })
}

/// Base sur le modèle de A4 (UID 2, O13) : lun-ven 09:00-13:00, 15:00-19:00.
fn base() -> Calendar {
    Calendar::new_base(
        uid(BASE),
        "Base".to_owned(),
        weekdays(|| working(&[("09:00", "13:00"), ("15:00", "19:00")])),
    )
}

/// Dérivé (calendrier de ressource, A4 UID 1) : lun-ven 08:00-12:00,
/// 13:00-17:00.
fn derived(base_uid: u32) -> Calendar {
    let mut c = Calendar::new_derived(uid(DERIVED), "Ressource".to_owned(), uid(base_uid));
    for d in [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
    ] {
        c.set_day(
            d,
            DayRule::Working(hours(&[("08:00", "12:00"), ("13:00", "17:00")])),
        )
        .unwrap();
    }
    c
}

/// Projet dont le calendrier est le dérivé, début lun 19/10/2026 08:00.
fn net() -> Net {
    Net::new("2026-10-19T08:00").project(|p| {
        p.calendars.push(base()).unwrap();
        p.calendars.push(derived(BASE)).unwrap();
        p.calendar = uid(DERIVED);
    })
}

#[test]
fn r113_leaves_placed_on_the_designated_calendar() {
    // A 2 j : placée sur le dérivé, lun 08:00 – mar 17:00 (la base la
    // placerait de lun 09:00 à mar 19:00).
    let run = net().task("A", 960).run();
    assert_eq!(run.at("A").early.start, moment("2026-10-19T08:00"));
    assert_eq!(run.at("A").early.finish, moment("2026-10-20T17:00"));
}

#[test]
fn r113_summary_duration_measured_on_the_base() {
    // S {A 2 j ; B 1 j FS} : A lun – mar, B mer 08:00 – 17:00 ; durée de S
    // sur la base = 8 h + 8 h + 6 h (mer 09:00-13:00, 15:00-17:00) = 1 320,
    // 120 de moins que sur le dérivé (O13 : A4 UID 1, 9 480 contre 9 600).
    let run = net()
        .task("S", 0)
        .task("A", 960)
        .level("A", 2)
        .task("B", 480)
        .level("B", 2)
        .fs("A", "B")
        .run();
    assert_eq!(run.at("S").early.start, moment("2026-10-19T08:00"));
    assert_eq!(run.at("S").early.finish, moment("2026-10-21T17:00"));
    assert_eq!(run.at("S").duration, Minutes::new(1_320));
}

#[test]
fn r113_free_slack_measured_on_the_base() {
    // A 1 j → B 1 j (FS) ; C 3 j seule. A lun, B mar, fin mer 17:00 ;
    // FS(A) = base.between(lun 17:00, mar 08:00) = 120 (lun 17:00-19:00),
    // borné par TF(A) = 480 : 120, comme A4 UID 14, 20, 21 (FreeSlack
    // 1 200 dixièmes). Marge totale inchangée (deux fins à 17:00).
    let run = net()
        .task("A", 480)
        .task("B", 480)
        .task("C", 1_440)
        .fs("A", "B")
        .run();
    assert_eq!(run.at("B").early.start, moment("2026-10-20T08:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(480));
    assert_eq!(run.at("A").free_slack, Minutes::new(120));
    assert_eq!(run.at("B").total_slack, Minutes::new(480));
    assert_eq!(run.at("B").free_slack, Minutes::new(480));
}

#[test]
fn r113_lag_counted_on_the_base() {
    // A 1 j → B FS+2 h : base.add(lun 17:00, 120) = lun 19:00, placé sur le
    // dérivé → mar 08:00 – 17:00 (sur le dérivé, le décalage donnerait
    // mar 10:00). R-041 : sans calendrier propre du successeur, le décalage
    // se compte sur le calendrier du projet, qui est la base.
    let run = net()
        .task("A", 480)
        .task("B", 480)
        .link("A", "B", LinkType::FinishToStart, lag_w(120))
        .run();
    assert_eq!(run.at("B").early.start, moment("2026-10-20T08:00"));
    assert_eq!(run.at("B").early.finish, moment("2026-10-20T17:00"));
}

#[test]
fn r113_base_project_calendar_unchanged() {
    // Calendrier du projet de base (la base elle-même) : placement et mesure
    // sur lui seul ; S {A 2 j ; B 1 j FS} : A lun 09:00 – mar 19:00, B mer
    // 09:00 – 19:00, durée de S 1 440.
    let run = Net::new("2026-10-19T08:00")
        .project(|p| {
            p.calendars.push(base()).unwrap();
            p.calendar = uid(BASE);
        })
        .task("S", 0)
        .task("A", 960)
        .level("A", 2)
        .task("B", 480)
        .level("B", 2)
        .fs("A", "B")
        .run();
    assert_eq!(run.at("A").early.start, moment("2026-10-19T09:00"));
    assert_eq!(run.at("B").early.finish, moment("2026-10-21T19:00"));
    assert_eq!(run.at("S").duration, Minutes::new(1_440));
}

#[test]
fn r113_task_calendar_measures_its_own_slack() {
    // Une tâche de calendrier propre (le dérivé désigné explicitement) garde
    // son calendrier pour la mesure : FS(A) = 0, comme avant R-113.
    let run = net()
        .task("A", 480)
        .calendar("A", DERIVED)
        .task("B", 480)
        .task("C", 1_440)
        .fs("A", "B")
        .run();
    assert_eq!(run.at("A").free_slack, Minutes::ZERO);
}

#[test]
fn r113_unusable_base_keeps_the_designated_calendar() {
    // Base sans temps ouvré : le calendrier désigné sert à tout (repli),
    // durée de S 1 440 sur le dérivé.
    let empty = Calendar::new_base(
        uid(BASE),
        "Vide".to_owned(),
        Week::from_fn(|_| DaySetting::NonWorking),
    );
    let run = Net::new("2026-10-19T08:00")
        .project(|p| {
            p.calendars.push(empty).unwrap();
            p.calendars.push(derived(BASE)).unwrap();
            p.calendar = uid(DERIVED);
        })
        .task("S", 0)
        .task("A", 960)
        .level("A", 2)
        .task("B", 480)
        .level("B", 2)
        .fs("A", "B")
        .run();
    assert_eq!(run.at("S").early.finish, moment("2026-10-21T17:00"));
    assert_eq!(run.at("S").duration, Minutes::new(1_440));
}
