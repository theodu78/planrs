//! R-098 — MFO.
//!
//! Lot K1 (J3, ADR-0010). « Doit finir le » : option vraie, la tâche est
//! épinglée sur `(P.backward_span(D', d).start, D')`, fin `D'` brute comme
//! une FNET liante (O14, hypothèse par cohérence), plafonnée par `D'` en
//! passe arrière ; option fausse : plancher (comme FNET) et plafond (comme
//! FNLT).

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
        .constraint("B", Constraint::MustFinishOn { date: moment(date) })
        .run()
}

#[test]
fn r098_later_date_both_options() {
    // MFO jeu 22/10 17:00 : mêmes valeurs qu'une MSO jeu 08:00.
    for honor in [true, false] {
        let run = nc("2026-10-22T17:00", honor);
        assert_eq!(run.at("B").early.start, moment("2026-10-22T08:00"));
        assert_eq!(run.at("B").early.finish, moment("2026-10-22T17:00"));
        assert_eq!(run.at("B").total_slack, Minutes::ZERO);
        assert_eq!(run.at("A").total_slack, Minutes::new(480));
        assert!(run.schedule.diagnostics.is_empty());
    }
}

#[test]
fn r098_earlier_date_conflict() {
    // MFO mar 20/10 17:00 : option vraie, B mar 08:00 – 17:00, TF(A) −480 ;
    // option fausse, B mer, TF(B) −480, conflit.
    let run = nc("2026-10-20T17:00", true);
    assert_eq!(run.at("B").early.start, moment("2026-10-20T08:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    assert_eq!(run.schedule.diagnostics.len(), 1);
    let run = nc("2026-10-20T17:00", false);
    assert_eq!(run.at("B").early.start, moment("2026-10-21T08:00"));
    assert_eq!(run.at("B").total_slack, Minutes::new(-480));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskConstraintConflict {
            task: run.uid("B"),
            constraint: ConstraintKind::MustFinishOn,
            date: moment("2026-10-20T17:00"),
            honored: false,
        }]
    );
}

#[test]
fn r098_pinned_finish_is_raw_date() {
    // MFO lun 26/10 08:00 (début de plage) : B épinglée ven 23 08:00 → lun
    // 26 08:00 brute ; fin du projet lun 08:00.
    let run = nc("2026-10-26T08:00", true);
    assert_eq!(run.at("B").early.start, moment("2026-10-23T08:00"));
    assert_eq!(run.at("B").early.finish, moment("2026-10-26T08:00"));
    assert_eq!(run.schedule.project_finish, moment("2026-10-26T08:00"));
    assert_eq!(run.at("B").total_slack, Minutes::ZERO);
}
