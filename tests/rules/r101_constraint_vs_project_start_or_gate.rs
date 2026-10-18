//! R-101 — Contrainte contre le début du projet ou une porte.
//!
//! Lot K1 (J3, ADR-0010). Option vraie : une contrainte épinglée (MSO, MFO,
//! SNLT ou FNLT mordant) gagne aussi contre le début du projet et contre la
//! porte d'une récapitulative ancêtre : la tâche peut commencer avant le
//! début du projet (amende R-036) ; `task.constraint_conflict { honored:
//! true }`. Le `Schedule` garde le début du projet saisi ; la récapitulative
//! du projet (UID 0) commence à la tâche. Option fausse : la tâche reste au
//! début du projet, marge négative, conflit `honored: false`.

use planrs_core::{Constraint, ConstraintKind, Diagnostic, Minutes, TaskUid};
use planrs_tests::moment;
use planrs_tests::net::Net;

fn mso_before_start(honor: bool) -> planrs_tests::net::Run {
    Net::new("2026-10-19T08:00")
        .task("A", 480)
        .honor(honor)
        .constraint(
            "A",
            Constraint::MustStartOn {
                date: moment("2026-10-16T08:00"),
            },
        )
        .run()
}

#[test]
fn r101_constraint_before_project_start() {
    // A 1 j MSO ven 16/10 08:00, début du projet lun 19/10 08:00.
    let run = mso_before_start(true);
    assert_eq!(run.at("A").early.start, moment("2026-10-16T08:00"));
    assert_eq!(run.at("A").early.finish, moment("2026-10-16T17:00"));
    assert_eq!(run.schedule.project_start, moment("2026-10-19T08:00"));
    let uid0 = run.schedule.scheduled(TaskUid::PROJECT_SUMMARY).unwrap();
    assert_eq!(uid0.early.start, moment("2026-10-16T08:00"));
    let conflict = |honored| Diagnostic::TaskConstraintConflict {
        task: run.uid("A"),
        constraint: ConstraintKind::MustStartOn,
        date: moment("2026-10-16T08:00"),
        honored,
    };
    assert_eq!(run.schedule.diagnostics, [conflict(true)]);
    // Option fausse : A lun 19/10, LS ven 16/10 08:00 → TF −480.
    let run = mso_before_start(false);
    assert_eq!(run.at("A").early.start, moment("2026-10-19T08:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    assert_eq!(run.schedule.diagnostics, [conflict(false)]);
}

#[test]
fn r101_constraint_against_summary_gate() {
    // X 2 j FS → S {A 1 j} : porte de S mer 08:00. A SNLT mar 20/10 08:00,
    // option vraie : A épinglée mar 08:00 – 17:00, porte violée → conflit
    // `honored: true` (aucun lien de A n'est violé en propre).
    let run = Net::new("2026-10-19T08:00")
        .task("X", 960)
        .task("S", 0)
        .task("A", 480)
        .level("A", 2)
        .fs("X", "S")
        .constraint(
            "A",
            Constraint::StartNoLaterThan {
                date: moment("2026-10-20T08:00"),
            },
        )
        .run();
    assert_eq!(run.at("A").early.start, moment("2026-10-20T08:00"));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskConstraintConflict {
            task: run.uid("A"),
            constraint: ConstraintKind::StartNoLaterThan,
            date: moment("2026-10-20T08:00"),
            honored: true,
        }]
    );
}
