//! R-100 — Conflit, liens prioritaires (`honor` faux ; marge négative sur la tâche et en amont).
//!
//! Lot K1 (J3, ADR-0010). Option décochée : la tâche est placée par ses
//! liens, la contrainte n'est qu'un plafond de la passe arrière ; la tâche
//! porte une marge négative, et ses prédécesseurs aussi (le plafond se
//! propage comme tout plafond : précision de l'ADR, hypothèse) ;
//! `task.constraint_conflict { honored: false }`. Réseau N (voir R-099).

use planrs_core::{Constraint, ConstraintKind, Diagnostic, Minutes};
use planrs_tests::moment;
use planrs_tests::net::Net;

#[test]
fn r100_must_start_on_chain_links_win() {
    // MSO mer 21/10 08:00 sur C : C jeu 22 08:00 – 17:00 (plancher mer non
    // liant) ; LS(C) = mer 08:00 → TF(C) −480 ; TF(B) = TF(A) = −480.
    let run = Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("C", 480)
        .task("X", 2_880)
        .fs("A", "B")
        .fs("B", "C")
        .honor(false)
        .constraint(
            "C",
            Constraint::MustStartOn {
                date: moment("2026-10-21T08:00"),
            },
        )
        .run();
    assert_eq!(run.at("C").early.start, moment("2026-10-22T08:00"));
    assert_eq!(run.at("C").late.start, moment("2026-10-21T08:00"));
    for t in ["A", "B", "C"] {
        assert_eq!(run.at(t).total_slack, Minutes::new(-480), "{t}");
        assert!(run.at(t).critical, "{t}");
    }
    assert_eq!(run.at("X").total_slack, Minutes::ZERO);
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskConstraintConflict {
            task: run.uid("C"),
            constraint: ConstraintKind::MustStartOn,
            date: moment("2026-10-21T08:00"),
            honored: false,
        }]
    );
}

#[test]
fn r100_fixture_constraints_xml_links_win() {
    // Même fixture, option basculée : les tâches 5 (SNLT) et 8 (MFO) sont
    // placées par leur lien (mer 21/10), marge −480 sur elles et sur P,
    // `task.constraint_conflict { honored: false }` ; plus aucun lien non
    // respecté ; le reste est inchangé.
    use super::r099_conflict_constraints_honored::{dates, fixture};
    let s = fixture(false);
    let honored = fixture(true);
    let uid = planrs_core::TaskUid::new;
    for u in [5, 8] {
        assert_eq!(
            dates(&s, u),
            (moment("2026-10-21T08:00"), moment("2026-10-21T17:00")),
            "UID {u}"
        );
        assert_eq!(s.scheduled(uid(u)).unwrap().total_slack, Minutes::new(-480));
    }
    assert_eq!(s.scheduled(uid(1)).unwrap().total_slack, Minutes::new(-480));
    for u in [1, 3, 4, 6, 7, 9, 10, 11, 12, 16, 18, 20, 23, 24] {
        assert_eq!(dates(&s, u), dates(&honored, u), "UID {u}");
    }
    let conflicts: Vec<_> = s
        .diagnostics
        .iter()
        .filter(|d| matches!(d, Diagnostic::TaskConstraintConflict { .. }))
        .collect();
    assert_eq!(
        conflicts,
        [
            &Diagnostic::TaskConstraintConflict {
                task: uid(5),
                constraint: ConstraintKind::StartNoLaterThan,
                date: moment("2026-10-20T08:00"),
                honored: false,
            },
            &Diagnostic::TaskConstraintConflict {
                task: uid(8),
                constraint: ConstraintKind::MustFinishOn,
                date: moment("2026-10-20T17:00"),
                honored: false,
            },
        ]
    );
    assert!(
        !s.diagnostics
            .iter()
            .any(|d| d.code() == "link.not_honored_constraint")
    );
}
