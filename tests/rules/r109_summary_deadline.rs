//! R-109 — Échéance d'une récapitulative.
//!
//! Lot K1 (J3, ADR-0010). L'échéance d'une récapitulative automatique
//! plafonne toutes ses feuilles (`C(S) = min(C(S), échéance)`, comme une
//! FNLT de récapitulative, R-102) ; ses dates au plus tard restent l'agrégat
//! de R-060 ; `task.deadline_missed` sur la récapitulative si sa fin
//! agrégée dépasse l'échéance (temps ouvré de son calendrier).

use planrs_core::{Diagnostic, Minutes};
use planrs_tests::moment;
use planrs_tests::net::Net;

#[test]
fn r109_summary_deadline_caps_every_leaf() {
    // S {A 2 j, B 1 j}, échéance mar 20/10 12:00, D 4 j seule : LS(A) ven
    // 16/10 13:00, TF(A) −240 ; LS(B) lun 13:00, TF(B) 240 ; TF(S) −240 ;
    // échéance dépassée sur S (fin mar 17:00).
    let run = Net::new("2026-10-19T08:00")
        .task("S", 0)
        .task("A", 960)
        .task("B", 480)
        .task("D", 1_920)
        .level("A", 2)
        .level("B", 2)
        .deadline("S", "2026-10-20T12:00")
        .run();
    assert_eq!(run.at("A").late.start, moment("2026-10-16T13:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(-240));
    assert_eq!(run.at("B").late.start, moment("2026-10-19T13:00"));
    assert_eq!(run.at("B").total_slack, Minutes::new(240));
    assert_eq!(run.at("S").total_slack, Minutes::new(-240));
    assert_eq!(run.at("S").late.finish, moment("2026-10-20T12:00"));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskDeadlineMissed {
            task: run.uid("S"),
            deadline: moment("2026-10-20T12:00"),
            finish: moment("2026-10-20T17:00"),
        }]
    );
}

#[test]
fn r109_nested_summary_inherits_the_cap() {
    // S1 {S2 {A 1 j}}, échéance de S1 lun 19/10 12:00 : A plafonnée à travers
    // S2, TF(A) = −240.
    let run = Net::new("2026-10-19T08:00")
        .task("S1", 0)
        .task("S2", 0)
        .task("A", 480)
        .level("S2", 2)
        .level("A", 3)
        .deadline("S1", "2026-10-19T12:00")
        .run();
    assert_eq!(run.at("A").total_slack, Minutes::new(-240));
    assert_eq!(run.codes(), ["task.deadline_missed"]);
}
