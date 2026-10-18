//! R-095 — SNLT.
//!
//! Lot K1 (J3, ADR-0010). « Début au plus tard le » : plafond de la passe
//! arrière, quelle que soit l'option : `LF ≤ Sc.finish` (fin de
//! `P.forward_span(D, d)`), donc `LS ≤ D'`. Sans effet sur la passe avant
//! tant que la tâche peut commencer à temps ; en conflit, R-099 (option
//! vraie, la tâche est épinglée sur `Sc`) ou R-100 (option fausse).

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
        .constraint("B", Constraint::StartNoLaterThan { date: moment(date) })
        .run()
}

#[test]
fn r095_ceiling_in_backward_pass() {
    // SNLT jeu 22/10 08:00 : B reste mer ; LS(B) = jeu 08:00 → TF(B) 480 ;
    // LF(A) = mer 17:00, LS(A) mar 08:00 → TF(A) 480, FS(A) 0.
    for honor in [true, false] {
        let run = nc("2026-10-22T08:00", honor);
        let b = run.at("B");
        assert_eq!(b.early.start, moment("2026-10-21T08:00"));
        assert_eq!(b.late.start, moment("2026-10-22T08:00"));
        assert_eq!(b.total_slack, Minutes::new(480));
        assert_eq!(run.at("A").late.finish, moment("2026-10-21T17:00"));
        assert_eq!(run.at("A").total_slack, Minutes::new(480));
        assert_eq!(run.at("A").free_slack, Minutes::ZERO);
        assert!(run.schedule.diagnostics.is_empty());
    }
}

#[test]
fn r095_conflict_both_options() {
    // SNLT mar 20/10 08:00, le lien demande mer 08:00.
    // Option vraie : B épinglée mar 08:00 – 17:00, lien A → B non respecté,
    // TF(B) 0, TF(A) −480 (LS(A) ven 16/10 08:00).
    let run = nc("2026-10-20T08:00", true);
    let b = run.at("B");
    assert_eq!(b.early.start, moment("2026-10-20T08:00"));
    assert_eq!(
        b.driver,
        Driver::Constraint {
            constraint: ConstraintKind::StartNoLaterThan
        }
    );
    assert_eq!(b.total_slack, Minutes::ZERO);
    assert_eq!(run.at("A").late.start, moment("2026-10-16T08:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::LinkNotHonoredConstraint {
            predecessor: run.uid("A"),
            successor: run.uid("B"),
        }]
    );
    // Option fausse : B mer, TF(B) = TF(A) = −480, conflit signalé.
    let run = nc("2026-10-20T08:00", false);
    assert_eq!(run.at("B").early.start, moment("2026-10-21T08:00"));
    assert_eq!(run.at("B").total_slack, Minutes::new(-480));
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskConstraintConflict {
            task: run.uid("B"),
            constraint: ConstraintKind::StartNoLaterThan,
            date: moment("2026-10-20T08:00"),
            honored: false,
        }]
    );
}
