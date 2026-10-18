//! R-102 — Contrainte d'une récapitulative.
//!
//! Lot K1 (J3, ADR-0010). Une récapitulative automatique n'accepte que SNET
//! (borne de sa porte, brute, comme un lien FS entrant, R-056 : ses
//! sous-tâches sont pilotées `SummaryGate`) et FNLT (plafond `C(S)` de
//! toutes ses feuilles, comme un lien FF sortant, R-057 ; jamais épinglée ;
//! fin agrégée au-delà → `task.constraint_conflict { honored: false }`).
//! MSO, MFO, SNLT, FNET et ALAP sont remplacées par ASAP avec
//! `task.constraint_on_summary` (avertissement).

use planrs_core::{Constraint, ConstraintKind, Diagnostic, Driver, Minutes};
use planrs_tests::moment;
use planrs_tests::net::{Net, Run};

fn summary(constraint: Constraint, a: i64) -> Run {
    // S {A, B 1 j}, D 4 j seule (lun – jeu 17:00).
    Net::new("2026-10-19T08:00")
        .task("S", 0)
        .task("A", a)
        .task("B", 480)
        .task("D", 1_920)
        .level("A", 2)
        .level("B", 2)
        .constraint("S", constraint)
        .run()
}

#[test]
fn r102_snet_gates_the_subtasks() {
    // S SNET mer 21/10 08:00 {A 1 j, B 1 j} : A, B mer 08:00 – 17:00,
    // pilote la porte de S.
    let run = summary(
        Constraint::StartNoEarlierThan {
            date: moment("2026-10-21T08:00"),
        },
        480,
    );
    for t in ["A", "B"] {
        assert_eq!(run.at(t).early.start, moment("2026-10-21T08:00"), "{t}");
        assert_eq!(
            run.at(t).driver,
            Driver::SummaryGate {
                summary: run.uid("S")
            }
        );
    }
    assert_eq!(run.at("S").early.start, moment("2026-10-21T08:00"));
    assert!(run.schedule.diagnostics.is_empty());
}

#[test]
fn r102_fnlt_caps_every_leaf() {
    // S FNLT mar 20/10 17:00 {A 2 j, B 1 j} : TF(A) 0, TF(B) 480 (sans la
    // FNLT : 960 et 1 440), TF(S) 0, aucun conflit.
    let run = summary(
        Constraint::FinishNoLaterThan {
            date: moment("2026-10-20T17:00"),
        },
        960,
    );
    assert_eq!(run.at("A").total_slack, Minutes::ZERO);
    assert_eq!(run.at("B").total_slack, Minutes::new(480));
    assert_eq!(run.at("S").total_slack, Minutes::ZERO);
    assert!(run.schedule.diagnostics.is_empty());
    // FNLT lun 19/10 17:00 : TF(A) = `between(mar 17:00, lun 17:00)` = −480,
    // conflit sur S (jamais épinglée).
    let date = moment("2026-10-19T17:00");
    let run = summary(Constraint::FinishNoLaterThan { date }, 960);
    assert_eq!(run.at("A").early.finish, moment("2026-10-20T17:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    assert_eq!(run.at("S").total_slack, Minutes::new(-480));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskConstraintConflict {
            task: run.uid("S"),
            constraint: ConstraintKind::FinishNoLaterThan,
            date,
            honored: false,
        }]
    );
}

#[test]
fn r102_other_constraints_refused() {
    // MSO, MFO, SNLT, FNET, ALAP sur S : planning identique à ASAP, avec
    // `task.constraint_on_summary`.
    let base = summary(Constraint::AsSoonAsPossible, 960);
    let d = moment("2026-10-22T08:00");
    for c in [
        Constraint::MustStartOn { date: d },
        Constraint::MustFinishOn { date: d },
        Constraint::StartNoLaterThan { date: d },
        Constraint::FinishNoEarlierThan { date: d },
        Constraint::AsLateAsPossible,
    ] {
        let run = summary(c, 960);
        assert_eq!(run.schedule.tasks, base.schedule.tasks, "{c:?}");
        assert_eq!(
            run.schedule.diagnostics,
            [Diagnostic::TaskConstraintOnSummary {
                task: run.uid("S"),
                constraint: c.kind(),
            }]
        );
    }
}
