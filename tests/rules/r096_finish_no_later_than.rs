//! R-096 — FNLT.
//!
//! Lot K1 (J3, ADR-0010). « Fin au plus tard le » : plafond `LF ≤ D'` de la
//! passe arrière (normalisé par `backward_span`), quelle que soit l'option ;
//! en conflit, option vraie : la tâche est épinglée sur
//! `(P.backward_span(D', d).start, D')` ; option fausse : R-100.

use planrs_core::{Constraint, ConstraintKind, Diagnostic, Minutes};
use planrs_tests::moment;
use planrs_tests::net::{Net, Run};

fn nc(date: &str, honor: bool) -> Run {
    Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("D", 2_400)
        .fs("A", "B")
        .honor(honor)
        .constraint("B", Constraint::FinishNoLaterThan { date: moment(date) })
        .run()
}

#[test]
fn r096_ceiling_in_backward_pass() {
    // FNLT jeu 22/10 17:00 : B mer ; LF(B) jeu 17:00 → TF(B) 480, TF(A) 480.
    for honor in [true, false] {
        let run = nc("2026-10-22T17:00", honor);
        assert_eq!(run.at("B").early.start, moment("2026-10-21T08:00"));
        assert_eq!(run.at("B").late.finish, moment("2026-10-22T17:00"));
        assert_eq!(run.at("B").total_slack, Minutes::new(480));
        assert_eq!(run.at("A").total_slack, Minutes::new(480));
        assert!(run.schedule.diagnostics.is_empty());
    }
}

#[test]
fn r096_conflict_both_options() {
    // FNLT mar 20/10 17:00. Option vraie : B = `backward_span(mar 17:00,
    // 480)` = mar 08:00 – 17:00, lien non respecté, TF(B) 0, TF(A) −480.
    let run = nc("2026-10-20T17:00", true);
    assert_eq!(run.at("B").early.start, moment("2026-10-20T08:00"));
    assert_eq!(run.at("B").early.finish, moment("2026-10-20T17:00"));
    assert_eq!(run.at("B").total_slack, Minutes::ZERO);
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::LinkNotHonoredConstraint {
            predecessor: run.uid("A"),
            successor: run.uid("B"),
        }]
    );
    // Option fausse : B mer, TF(B) = `between(mer 17:00, mar 17:00)` = −480,
    // FS(B) = −480 (non tronquée), TF(A) −480, conflit.
    let run = nc("2026-10-20T17:00", false);
    assert_eq!(run.at("B").early.finish, moment("2026-10-21T17:00"));
    assert_eq!(run.at("B").total_slack, Minutes::new(-480));
    assert_eq!(run.at("B").free_slack, Minutes::new(-480));
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    assert!(run.at("B").critical && run.at("A").critical);
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskConstraintConflict {
            task: run.uid("B"),
            constraint: ConstraintKind::FinishNoLaterThan,
            date: moment("2026-10-20T17:00"),
            honored: false,
        }]
    );
}
