//! R-111 — Conversion impossible à rebours (amende R-069) : une conversion
//! qui échoue dans la passe arrière principale exclut la tâche
//! (`Unschedulable`, `task.unschedulable { pass: Backward }`) ; ses
//! prédécesseurs ignorent le lien (`link.excluded_endpoint`) et ont des
//! dates de repli (`degraded`, propagé vers l'amont). Source : hypothèse
//! (règle 10).

use planrs_core::{Diagnostic, Exclusion, Pass, TaskOutcome};
use planrs_tests::moment;
use planrs_tests::net::Net;

#[test]
fn r111_out_of_domain_from_finish() {
    // F = mar 03/01/1984 17:00 ; X 1 j FS → A 3 j : A finirait mar 17:00 et
    // commencerait avant le domaine (`calendar.out_of_domain`) → exclue ;
    // X au plus tard mar 03/01/1984 08:00 – 17:00, dégradée.
    let run = Net::new("1984-01-02T08:00")
        .from_finish("1984-01-03T17:00")
        .task("X", 480)
        .task("A", 1_440)
        .fs("X", "A")
        .run();
    assert_eq!(
        *run.outcome("A"),
        TaskOutcome::Excluded {
            reason: Exclusion::Unschedulable,
            display: None
        }
    );
    let x = run.at("X");
    assert_eq!(x.late.start, moment("1984-01-03T08:00"));
    assert_eq!(x.late.finish, moment("1984-01-03T17:00"));
    assert!(x.degraded);
    assert!(run.schedule.diagnostics.contains(&Diagnostic::TaskUnschedulable {
        task: run.uid("A"),
        pass: Pass::Backward,
        cause: "calendar.out_of_domain".to_owned()
    }));
    assert!(
        run.schedule
            .diagnostics
            .contains(&Diagnostic::LinkExcludedEndpoint {
                predecessor: run.uid("X"),
                successor: run.uid("A")
            })
    );
    assert_eq!(run.codes().len(), 2, "{:?}", run.schedule.diagnostics);
}
