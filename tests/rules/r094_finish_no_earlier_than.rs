//! R-094 — FNET (O14 : `EF = max(fin calculée, D')`).
//!
//! Lot K1 (J3, ADR-0010). « Fin au plus tôt le » : `D' =
//! P.backward_span(D, 0).finish` (R-090) ; plancher de début
//! `P.backward_span(D', d).start` (même conversion qu'une borne FF, R-039),
//! après les liens ; fin au plus tôt `EF = max(P.forward_span(ES, d).finish,
//! D')` en `Moment` brut : quand la FNET pilote, la tâche finit exactement
//! sur `D'` (même temps ouvré que la fin calculée, donc marges inchangées).
//! Observé sur le corpus A7 (UID 15 à 21 et 32, O14).

use planrs_core::calendar::{Calendar, DaySetting};
use planrs_core::{CalendarUid, Constraint, ConstraintKind, Driver, Minutes, Week, Weekday};
use planrs_tests::net::{Net, Run};
use planrs_tests::{moment, working};

fn fnet(date: &str) -> Constraint {
    Constraint::FinishNoEarlierThan { date: moment(date) }
}

fn nc(date: &str) -> Run {
    Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("D", 2_400)
        .fs("A", "B")
        .constraint("B", fnet(date))
        .run()
}

#[test]
fn r094_binding_floor_at_end_of_day() {
    // FNET jeu 22/10 17:00 → B jeu 08:00 – 17:00, pilote `Constraint`,
    // TF(B) 480.
    let run = nc("2026-10-22T17:00");
    let b = run.at("B");
    assert_eq!(b.early.start, moment("2026-10-22T08:00"));
    assert_eq!(b.early.finish, moment("2026-10-22T17:00"));
    assert_eq!(
        b.driver,
        Driver::Constraint {
            constraint: ConstraintKind::FinishNoEarlierThan
        }
    );
    assert_eq!(b.total_slack, Minutes::new(480));
}

#[test]
fn r094_binding_floor_at_range_start_finishes_on_raw_date() {
    // FNET lun 26/10 08:00 : début `backward_span(lun 08:00, 480)` = ven 23
    // 08:00 ; fin = max(ven 17:00, lun 08:00) = lun 26/10 08:00 brute ; fin
    // du projet lun 08:00 (R-045) ; TF(B) = TF(D) = 0 ; TF(A) = 960.
    let run = nc("2026-10-26T08:00");
    let b = run.at("B");
    assert_eq!(b.early.start, moment("2026-10-23T08:00"));
    assert_eq!(b.early.finish, moment("2026-10-26T08:00"));
    assert_eq!(b.finish, moment("2026-10-26T08:00"));
    assert_eq!(run.schedule.project_finish, moment("2026-10-26T08:00"));
    assert_eq!(b.total_slack, Minutes::ZERO);
    assert_eq!(run.at("D").total_slack, Minutes::ZERO);
    assert_eq!(run.at("A").total_slack, Minutes::new(960));
}

#[test]
fn r094_non_binding_floor_keeps_computed_finish() {
    // FNET mar 20/10 17:00 (forme A6 UID 7, non liante) : B reste mer
    // 08:00 – 17:00, fin calculée, pilote le lien.
    let run = nc("2026-10-20T17:00");
    assert_eq!(run.at("B").early.finish, moment("2026-10-21T17:00"));
    assert!(matches!(run.at("B").driver, Driver::Link { .. }));
}

/// Calendrier de la forme de A7 (UID 5) : vendredi 18:00-20:00 et
/// 21:00-23:00, samedi 09:00-13:00 et 14:00-18:00 (12 h par semaine).
fn weekend_calendar() -> Calendar {
    let week = Week::from_fn(|d| match d {
        Weekday::Friday => working(&[("18:00", "20:00"), ("21:00", "23:00")]),
        Weekday::Saturday => working(&[("09:00", "13:00"), ("14:00", "18:00")]),
        _ => DaySetting::NonWorking,
    });
    Calendar::new_base(CalendarUid::new(5), "Week-end".to_owned(), week)
}

#[test]
fn r094_corpus_o14_shape() {
    // Forme de A7 UID 15 et 21, sans lien. T 480 FNET ven 18/10/2024 18:00
    // (début d'une plage) : début sam 12/10 09:00, fin ven 18/10 18:00 (et
    // non sam 12/10 18:00). U 720 FNET ven 29/11 18:00 : début ven 22/11
    // 18:00, fin ven 29/11 18:00 ; fin du projet ven 29/11 18:00. LF(T) =
    // `backward_span(ven 29/11 18:00, 480)` = sam 23/11 09:00 – 18:00 → TF(T)
    // = 6 semaines × 720 = 4 320. Aucun `constraint_moved`.
    let run = Net::new("2024-10-04T08:00")
        .project(|p| {
            p.calendars.push(weekend_calendar()).unwrap();
            p.calendar = CalendarUid::new(5);
        })
        .task("T", 480)
        .task("U", 720)
        .constraint("T", fnet("2024-10-18T18:00"))
        .constraint("U", fnet("2024-11-29T18:00"))
        .run();
    let t = run.at("T");
    assert_eq!(t.early.start, moment("2024-10-12T09:00"));
    assert_eq!(t.early.finish, moment("2024-10-18T18:00"));
    assert_eq!(t.late.start, moment("2024-11-23T09:00"));
    assert_eq!(t.late.finish, moment("2024-11-23T18:00"));
    assert_eq!(t.total_slack, Minutes::new(4_320));
    let u = run.at("U");
    assert_eq!(u.early.start, moment("2024-11-22T18:00"));
    assert_eq!(u.early.finish, moment("2024-11-29T18:00"));
    assert_eq!(u.total_slack, Minutes::ZERO);
    assert_eq!(run.schedule.project_finish, moment("2024-11-29T18:00"));
    assert!(
        run.schedule.diagnostics.is_empty(),
        "{:?}",
        run.schedule.diagnostics
    );
}
