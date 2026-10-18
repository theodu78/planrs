//! R-097 — MSO.
//!
//! Lot K1 (J3, ADR-0010). « Doit commencer le » : option vraie, la tâche est
//! épinglée sur `Sc = P.forward_span(D, d)` dans la passe avant (pilote
//! `Constraint` si elle est déplacée) et plafonnée par `Sc.finish` dans la
//! passe arrière ; option fausse : plancher `D'` (comme SNET) et plafond
//! (comme SNLT).

use planrs_core::{Constraint, ConstraintKind, Diagnostic, Driver, Minutes};
use planrs_tests::moment;
use planrs_tests::net::{Net, Run};

fn nc(date: &str, honor: bool) -> Run {
    Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("D", 2_400)
        .fs("A", "B")
        .honor(honor)
        .constraint("B", Constraint::MustStartOn { date: moment(date) })
        .run()
}

#[test]
fn r097_later_date_both_options() {
    // MSO jeu 22/10 08:00 : B jeu 08:00 – 17:00, pilote `Constraint`, TF(B) 0
    // (critique), LF(A) mer 17:00 → TF(A) 480.
    for honor in [true, false] {
        let run = nc("2026-10-22T08:00", honor);
        let b = run.at("B");
        assert_eq!(b.early.start, moment("2026-10-22T08:00"));
        assert_eq!(
            b.driver,
            Driver::Constraint {
                constraint: ConstraintKind::MustStartOn
            }
        );
        assert_eq!(b.total_slack, Minutes::ZERO);
        assert!(b.critical);
        assert_eq!(run.at("A").total_slack, Minutes::new(480));
        assert!(run.schedule.diagnostics.is_empty());
    }
}

#[test]
fn r097_earlier_date_conflict() {
    // MSO mar 20/10 08:00. Option vraie : B mar 08:00 – 17:00, lien non
    // respecté, TF(B) 0, TF(A) −480. Option fausse : B mer, TF(B) −480,
    // conflit.
    let run = nc("2026-10-20T08:00", true);
    assert_eq!(run.at("B").early.start, moment("2026-10-20T08:00"));
    assert_eq!(run.at("B").total_slack, Minutes::ZERO);
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::LinkNotHonoredConstraint {
            predecessor: run.uid("A"),
            successor: run.uid("B"),
        }]
    );
    let run = nc("2026-10-20T08:00", false);
    assert_eq!(run.at("B").early.start, moment("2026-10-21T08:00"));
    assert_eq!(run.at("B").total_slack, Minutes::new(-480));
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskConstraintConflict {
            task: run.uid("B"),
            constraint: ConstraintKind::MustStartOn,
            date: moment("2026-10-20T08:00"),
            honored: false,
        }]
    );
}
