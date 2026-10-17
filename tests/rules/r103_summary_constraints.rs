//! R-103 — Contraintes et échéance d'une récapitulative automatique : SNET
//! → plancher brut de sa porte (hérité par toutes ses sous-tâches) ; FNLT →
//! plafond `C(S)` (ancrage `f*` sur son calendrier) ; échéance → plafond
//! `C(S)` brut, dépassement jugé sur sa fin agrégée ; MSO, MFO, SNLT, FNET
//! → ignorées avec `constraint.summary_ignored` ; ASAP, ALAP → sans effet.
//! Jamais d'épingle ni de serrage sur une récapitulative. Sources : S24 (« If
//! a Summary task (at any level) has a predecessor or a constraint, the
//! subtask can't be scheduled any earlier than the summary task »), S8
//! (échéances sur les récapitulatives), Q11.

use planrs_core::{Constraint, ConstraintKind, Diagnostic, Driver, LateDriver, Minutes};
use planrs_tests::moment;
use planrs_tests::net::Net;

/// S {A 1 j ; B 1 j FS de A}, C 4 j seule.
fn net() -> Net {
    Net::new("2026-10-19T08:00")
        .task("S", 0)
        .task("A", 480)
        .task("B", 480)
        .task("C", 1_920)
        .level("A", 2)
        .level("B", 2)
        .fs("A", "B")
}

#[test]
fn r103_snet_gates_every_subtask() {
    // SNET mer 21/10 08:00 sur S → A mer, B jeu ; pilote de A la porte de S.
    let run = net()
        .constraint(
            "S",
            Constraint::StartNoEarlierThan {
                date: moment("2026-10-21T08:00"),
            },
        )
        .run();
    assert_eq!(run.at("A").early.start, moment("2026-10-21T08:00"));
    assert_eq!(run.at("B").early.start, moment("2026-10-22T08:00"));
    assert_eq!(
        run.at("A").driver,
        Driver::SummaryGate {
            summary: run.uid("S")
        }
    );
    assert_eq!(run.at("S").early.start, moment("2026-10-21T08:00"));
    assert_eq!(run.codes(), [""; 0]);
}

#[test]
fn r103_fnlt_caps_every_leaf() {
    // FNLT mar 20/10 17:00 sur S → C(S) mar 17:00 → TF(A) = TF(B) = 0 (960
    // sans) ; cause de B : le plafond de S.
    let fnlt = Constraint::FinishNoLaterThan {
        date: moment("2026-10-20T17:00"),
    };
    let run = net().constraint("S", fnlt).run();
    for name in ["A", "B"] {
        assert_eq!(run.at(name).total_slack, Minutes::ZERO, "{name}");
    }
    assert_eq!(
        run.at("B").late_driver,
        LateDriver::SummaryCap {
            summary: run.uid("S")
        }
    );
    assert_eq!(run.codes(), [""; 0]);
    let run = net().run();
    assert_eq!(run.at("A").total_slack, Minutes::new(960));
    // S {A 2 j} FNLT lun 19/10 17:00 : la fin agrégée (mar 17:00) dépasse →
    // `constraint.not_honored { S, FNLT, lun 17:00 }` (aucun serrage).
    let run = Net::new("2026-10-19T08:00")
        .task("S", 0)
        .task("A", 960)
        .level("A", 2)
        .constraint(
            "S",
            Constraint::FinishNoLaterThan {
                date: moment("2026-10-19T17:00"),
            },
        )
        .run();
    assert_eq!(run.at("A").early.finish, moment("2026-10-20T17:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::ConstraintNotHonored {
            task: run.uid("S"),
            kind: ConstraintKind::FinishNoLaterThan,
            date: moment("2026-10-19T17:00"),
        }]
    );
}

#[test]
fn r103_summary_deadline() {
    // Échéance lun 19/10 17:00 sur S → TF(A) = TF(B) = −480 ;
    // `task.deadline_missed { S, lun 17:00, mar 17:00 }` (fin agrégée).
    let run = net().deadline("S", "2026-10-19T17:00").run();
    for name in ["A", "B", "S"] {
        assert_eq!(run.at(name).total_slack, Minutes::new(-480), "{name}");
    }
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskDeadlineMissed {
            task: run.uid("S"),
            deadline: moment("2026-10-19T17:00"),
            finish: moment("2026-10-20T17:00"),
        }]
    );
}

#[test]
fn r103_other_kinds_are_ignored() {
    // MSO, MFO, SNLT, FNET sur S : ignorées, `constraint.summary_ignored` ;
    // dates inchangées. ASAP, ALAP : sans effet ni diagnostic.
    let base = net().run();
    let d = moment("2026-10-22T08:00");
    for (c, kind) in [
        (Constraint::MustStartOn { date: d }, ConstraintKind::MustStartOn),
        (Constraint::MustFinishOn { date: d }, ConstraintKind::MustFinishOn),
        (
            Constraint::StartNoLaterThan { date: d },
            ConstraintKind::StartNoLaterThan,
        ),
        (
            Constraint::FinishNoEarlierThan { date: d },
            ConstraintKind::FinishNoEarlierThan,
        ),
    ] {
        let run = net().constraint("S", c).run();
        assert_eq!(
            run.schedule.diagnostics,
            [Diagnostic::ConstraintSummaryIgnored {
                task: run.uid("S"),
                kind
            }],
            "{kind:?}"
        );
        assert_eq!(run.schedule.tasks, base.schedule.tasks, "{kind:?}");
    }
    for c in [Constraint::AsSoonAsPossible, Constraint::AsLateAsPossible] {
        let run = net().constraint("S", c).run();
        assert_eq!(run.schedule, base.schedule, "{c:?}");
    }
}

#[test]
fn r103_inactive_summary_ignores_everything() {
    // S inactive, SNET et échéance : ignorées, sans diagnostic propre.
    let run = net()
        .inactive("S")
        .constraint(
            "S",
            Constraint::MustStartOn {
                date: moment("2026-10-22T08:00"),
            },
        )
        .deadline("S", "2026-10-19T17:00")
        .run();
    assert!(
        !run.codes()
            .iter()
            .any(|c| c.starts_with("constraint.") || *c == "task.deadline_missed")
    );
}
