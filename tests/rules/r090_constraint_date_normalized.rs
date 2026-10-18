//! R-090 — Date de contrainte normalisée (début : moment ouvré suivant ; fin : convention de jalon arrière), `constraint_moved`.
//!
//! Lot K1 (J3, ADR-0010). Une contrainte de début (SNET, SNLT, MSO) vaut
//! le début de `P.forward_span(D, d)` ; une contrainte de fin (FNET, FNLT,
//! MFO) vaut `D' = P.backward_span(D, 0).finish` (D gardée si elle commence
//! une minute ouvrée ou une plage, sinon l'instant ouvré précédent) ; temps
//! écoulé : D. `task.constraint_moved` (information) seulement si `D' ≠ D`.
//! Calendrier Standard, début du projet lun 19/10/2026 08:00. Réseau Nc :
//! A 2 j (lun – mar), B 1 j FS de A (mer), D 5 j seule (lun – ven 23/10).

use planrs_core::{Constraint, ConstraintKind, Diagnostic, Driver, Span, TaskOutcome};
use planrs_tests::moment;
use planrs_tests::net::{Net, Run};

fn nc(constraint: Constraint) -> Run {
    Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("D", 2_400)
        .fs("A", "B")
        .constraint("B", constraint)
        .run()
}

fn moved(run: &Run) -> Vec<&Diagnostic> {
    run.schedule
        .diagnostics
        .iter()
        .filter(|d| d.code() == "task.constraint_moved")
        .collect()
}

#[test]
fn r090_start_constraint_moved_to_next_working_moment() {
    // SNET sam 24/10 10:00 → D' = lun 26/10 08:00 : B lun 26 08:00 – 17:00,
    // date déplacée (information).
    let run = nc(Constraint::StartNoEarlierThan {
        date: moment("2026-10-24T10:00"),
    });
    let b = run.at("B");
    assert_eq!(b.early.start, moment("2026-10-26T08:00"));
    assert_eq!(b.early.finish, moment("2026-10-26T17:00"));
    assert_eq!(
        moved(&run),
        [&Diagnostic::TaskConstraintMoved {
            task: run.uid("B"),
            from: moment("2026-10-24T10:00"),
            to: moment("2026-10-26T08:00"),
        }]
    );
}

#[test]
fn r090_finish_constraint_at_midnight_moves_back() {
    // FNET mer 21/10 00:00 (piège de minuit) → D' = mar 20/10 17:00 (instant
    // ouvré précédent) ; plancher non liant : B reste mer 08:00 – 17:00.
    let run = nc(Constraint::FinishNoEarlierThan {
        date: moment("2026-10-21T00:00"),
    });
    assert_eq!(run.at("B").early.start, moment("2026-10-21T08:00"));
    assert_eq!(run.at("B").early.finish, moment("2026-10-21T17:00"));
    assert_eq!(
        moved(&run),
        [&Diagnostic::TaskConstraintMoved {
            task: run.uid("B"),
            from: moment("2026-10-21T00:00"),
            to: moment("2026-10-20T17:00"),
        }]
    );
}

#[test]
fn r090_finish_at_range_start_is_kept() {
    // FNET lun 26/10 08:00 (début de plage, forme O14) : gardée, aucun
    // diagnostic ; B ven 23 08:00 → fin lun 26 08:00 brute.
    let run = nc(Constraint::FinishNoEarlierThan {
        date: moment("2026-10-26T08:00"),
    });
    assert!(moved(&run).is_empty(), "{:?}", run.schedule.diagnostics);
    assert_eq!(run.at("B").early.start, moment("2026-10-23T08:00"));
    assert_eq!(run.at("B").early.finish, moment("2026-10-26T08:00"));
}

#[test]
fn r090_working_date_not_reported() {
    // SNET jeu 22/10 08:00, en temps ouvré : aucun diagnostic.
    let run = nc(Constraint::StartNoEarlierThan {
        date: moment("2026-10-22T08:00"),
    });
    assert!(
        run.schedule.diagnostics.is_empty(),
        "{:?}",
        run.schedule.diagnostics
    );
}

#[test]
fn r090_milestone_conventions() {
    // Jalon M : MSO lun 19/10 17:00 → gardée (fin de minute ouvrée, R-031
    // avant) ; MFO dim 25/10 12:00 → ven 23/10 17:00 (déplacée).
    let mso = Net::new("2026-10-19T08:00")
        .task("M", 0)
        .constraint(
            "M",
            Constraint::MustStartOn {
                date: moment("2026-10-19T17:00"),
            },
        )
        .run();
    assert_eq!(mso.at("M").early.start, moment("2026-10-19T17:00"));
    assert!(moved(&mso).is_empty());
    let mfo = Net::new("2026-10-19T08:00")
        .task("M", 0)
        .constraint(
            "M",
            Constraint::MustFinishOn {
                date: moment("2026-10-25T12:00"),
            },
        )
        .run();
    assert_eq!(mfo.at("M").early.finish, moment("2026-10-23T17:00"));
    assert_eq!(
        moved(&mfo),
        [&Diagnostic::TaskConstraintMoved {
            task: mfo.uid("M"),
            from: moment("2026-10-25T12:00"),
            to: moment("2026-10-23T17:00"),
        }]
    );
}

#[test]
fn r090_elapsed_task_keeps_raw_date() {
    // Tâche écoulée E 1 440 em, SNET sam 24/10 10:00 : temps civil, la date
    // est gardée telle quelle, aucun diagnostic.
    let run = Net::new("2026-10-19T08:00")
        .elapsed("E", 1_440)
        .constraint(
            "E",
            Constraint::StartNoEarlierThan {
                date: moment("2026-10-24T10:00"),
            },
        )
        .run();
    assert_eq!(
        run.at("E").early,
        Span {
            start: moment("2026-10-24T10:00"),
            finish: moment("2026-10-25T10:00"),
        }
    );
    assert_eq!(
        run.at("E").driver,
        Driver::Constraint {
            constraint: ConstraintKind::StartNoEarlierThan
        }
    );
    assert!(moved(&run).is_empty());
}

#[test]
fn r090_inactive_task_is_not_reported() {
    // Inactive : contrainte appliquée à l'affichage, sans diagnostic (R-103).
    let run = Net::new("2026-10-19T08:00")
        .task("I", 480)
        .inactive("I")
        .constraint(
            "I",
            Constraint::StartNoEarlierThan {
                date: moment("2026-10-24T10:00"),
            },
        )
        .run();
    assert!(matches!(
        run.outcome("I"),
        TaskOutcome::Excluded { display: Some(s), .. } if s.start == moment("2026-10-26T08:00")
    ));
    assert!(run.schedule.diagnostics.is_empty());
}
