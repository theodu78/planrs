//! R-069 — Conversion impossible : en passe avant → `Excluded {
//! Unschedulable }` + `task.unschedulable { Forward }` ; ses liens sortants
//! sont ignorés et ses successeurs, planifiés sans eux, ont `degraded`
//! (transitif) ; en passe arrière → `late = early`, `degraded`. Hypothèse
//! (règle 10).

use planrs_core::{Diagnostic, Exclusion, Pass, TaskOutcome};
use planrs_tests::moment;
use planrs_tests::net::Net;

#[test]
fn r069_out_of_domain_task_excluded_and_successors_degraded() {
    // Début mer 31/12/2149 08:00 ; A 2 j : la fin sort du domaine →
    // `OutOfDomain { After }` → non planifiable ; B FS de A 1 j → mer 08:00
    // – 17:00, dégradée ; C FS de B, dégradée aussi.
    let run = Net::new("2149-12-31T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("C", 0)
        .fs("A", "B")
        .fs("B", "C")
        .run();
    assert_eq!(
        *run.outcome("A"),
        TaskOutcome::Excluded {
            reason: Exclusion::Unschedulable,
            display: None
        }
    );
    assert!(run.schedule.diagnostics.contains(&Diagnostic::TaskUnschedulable {
        task: run.uid("A"),
        pass: Pass::Forward,
        cause: "calendar.out_of_domain".to_owned(),
    }));
    assert_eq!(run.at("B").early.start, moment("2149-12-31T08:00"));
    assert_eq!(run.at("B").early.finish, moment("2149-12-31T17:00"));
    assert!(run.at("B").degraded);
    assert!(run.at("C").degraded);
    assert!(run.codes().contains(&"link.excluded_endpoint"));
    // A n'apparaît pas dans l'ordre des tâches planifiées.
    assert!(!run.schedule.order.contains(&run.uid("A")));
}
