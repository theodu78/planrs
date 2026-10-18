//! R-099 — Conflit, contraintes respectées (`honor` vrai).
//!
//! Lot K1 (J3, ADR-0010). Option « toujours respecter les dates de
//! contrainte » (défaut) : quand un lien demande une date que le plafond
//! SNLT, FNLT, MSO ou MFO interdit, la contrainte gagne, la tâche est placée
//! sur sa date, chaque lien entrant violé donne `link.not_honored_constraint`
//! et la passe arrière propage le plafond : les prédécesseurs portent une
//! marge négative (la tâche, TF 0) et deviennent critiques.
//! Réseau N : A 2 j → B 1 j → C 1 j (FS), X 6 j seule (fin lun 26/10 17:00).

use planrs_core::{Constraint, Diagnostic, Minutes};
use planrs_tests::moment;
use planrs_tests::net::{Net, Run};

fn n(constraint: Constraint) -> Run {
    Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("C", 480)
        .task("X", 2_880)
        .task("D", 480)
        .fs("A", "B")
        .fs("B", "C")
        .fs("C", "D")
        .constraint("C", constraint)
        .run()
}

#[test]
fn r099_must_start_on_chain() {
    // MSO mer 21/10 08:00 sur C : C mer 08:00 – 17:00 (chevauche B), lien
    // B → C non respecté ; TF(C) 0 ; LF(B) = mer 08:00 → B au plus tard mar
    // 08:00 – 17:00, TF(B) = FS(B) = −480 ; LF(A) mar 08:00 → LS(A) ven 16/10
    // 08:00, TF(A) −480 ; A, B, C critiques.
    let run = n(Constraint::MustStartOn {
        date: moment("2026-10-21T08:00"),
    });
    assert_eq!(run.at("C").early.start, moment("2026-10-21T08:00"));
    assert_eq!(run.at("C").total_slack, Minutes::ZERO);
    assert_eq!(run.at("B").late.start, moment("2026-10-20T08:00"));
    assert_eq!(run.at("B").total_slack, Minutes::new(-480));
    assert_eq!(run.at("B").free_slack, Minutes::new(-480));
    assert_eq!(run.at("A").late.start, moment("2026-10-16T08:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(-480));
    for t in ["A", "B", "C"] {
        assert!(run.at(t).critical, "{t}");
    }
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::LinkNotHonoredConstraint {
            predecessor: run.uid("B"),
            successor: run.uid("C"),
        }]
    );
}

#[test]
fn r099_finish_no_later_than_chain() {
    // FNLT mer 21/10 12:00 sur C : épinglée mar 13:00 – mer 12:00 ; TF(C) 0 ;
    // LF(B) = mar 12:00 → LS(B) lun 13:00, TF(B) −720 ; LS(A) jeu 15/10
    // 13:00, TF(A) −720 ; D (FS de C) lit la fin épinglée : mer 13:00 – jeu
    // 12:00.
    let run = n(Constraint::FinishNoLaterThan {
        date: moment("2026-10-21T12:00"),
    });
    assert_eq!(run.at("C").early.start, moment("2026-10-20T13:00"));
    assert_eq!(run.at("C").early.finish, moment("2026-10-21T12:00"));
    assert_eq!(run.at("C").total_slack, Minutes::ZERO);
    assert_eq!(run.at("B").late.start, moment("2026-10-19T13:00"));
    assert_eq!(run.at("B").total_slack, Minutes::new(-720));
    assert_eq!(run.at("A").late.start, moment("2026-10-15T13:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(-720));
    assert_eq!(run.at("D").early.start, moment("2026-10-21T13:00"));
    assert_eq!(run.at("D").early.finish, moment("2026-10-22T12:00"));
    assert!(
        run.schedule
            .diagnostics
            .contains(&Diagnostic::LinkNotHonoredConstraint {
                predecessor: run.uid("B"),
                successor: run.uid("C"),
            })
    );
}

/// Fixture `constraints.xml` lue, option telle qu'écrite (`HonorConstraints`
/// 1) ou basculée.
pub(crate) fn fixture(honor: bool) -> planrs_core::Schedule {
    let bytes = std::fs::read(planrs_tests::tests_path("fixtures/mspdi/constraints.xml")).unwrap();
    let doc = planrs_mspdi::read(
        &bytes,
        &planrs_mspdi::ReadOptions::new(moment("2026-01-01T00:00")),
    )
    .unwrap();
    assert!(doc.project.options.honor_constraints, "HonorConstraints 1 lu");
    let mut p = doc.project;
    p.options.honor_constraints = honor;
    planrs_core::schedule(&p)
}

/// Début et fin planifiés de la tâche `uid`.
pub(crate) fn dates(s: &planrs_core::Schedule, uid: u32) -> (planrs_core::Moment, planrs_core::Moment) {
    let t = s.scheduled(planrs_core::TaskUid::new(uid)).unwrap();
    (t.start, t.finish)
}

#[test]
fn r099_fixture_constraints_xml() {
    // Fixture écrite à la main (Standard, début lun 19/10/2026), option
    // vraie. Valeurs calculées à la main (voir l'en-tête du fichier).
    let s = fixture(true);
    let d = |a: &str, b: &str| (moment(a), moment(b));
    let uid = planrs_core::TaskUid::new;
    for (u, want) in [
        (1, d("2026-10-19T08:00", "2026-10-20T17:00")),
        (3, d("2026-10-22T08:00", "2026-10-22T17:00")),
        (4, d("2026-10-22T08:00", "2026-10-22T17:00")),
        // SNLT mar 08:00 en conflit : épinglée.
        (5, d("2026-10-20T08:00", "2026-10-20T17:00")),
        (6, d("2026-10-21T08:00", "2026-10-21T17:00")),
        // MSO lun 12:30 → 13:00 (déplacée), épinglée.
        (7, d("2026-10-19T13:00", "2026-10-20T12:00")),
        // MFO mar 17:00 en conflit : épinglée.
        (8, d("2026-10-20T08:00", "2026-10-20T17:00")),
        (9, d("2026-10-26T08:00", "2026-10-26T17:00")),
        (10, d("2026-10-19T08:00", "2026-10-19T17:00")),
        (11, d("2026-10-21T08:00", "2026-10-21T17:00")),
        // Forme O14 : fin brute lun 26/10 08:00.
        (12, d("2026-10-23T08:00", "2026-10-26T08:00")),
        // Date sans heure illisible (R-075) et ASAP daté : le plus tôt possible.
        (13, d("2026-10-19T08:00", "2026-10-19T17:00")),
        (14, d("2026-10-19T08:00", "2026-10-19T17:00")),
        (16, d("2026-10-21T08:00", "2026-10-22T17:00")),
        // Récapitulative SNET : sous-tâche mer.
        (18, d("2026-10-21T08:00", "2026-10-21T17:00")),
        (20, d("2026-10-19T08:00", "2026-10-20T17:00")),
        (22, d("2026-10-19T08:00", "2026-10-19T17:00")),
        // Manuelle : dates saisies, contrainte ignorée.
        (23, d("2026-10-19T08:00", "2026-10-19T17:00")),
        (24, d("2026-10-19T17:00", "2026-10-19T17:00")),
    ] {
        assert_eq!(dates(&s, u), want, "UID {u}");
    }
    assert_eq!(s.project_finish, moment("2026-10-26T17:00"));
    assert_eq!(
        s.scheduled(uid(20)).unwrap().total_slack,
        planrs_core::Minutes::ZERO
    );
    // Inactive : affichée mer 08:00 – 17:00, sans diagnostic.
    assert!(matches!(
        s.get(uid(25)),
        Some(planrs_core::TaskOutcome::Excluded { display: Some(sp), .. })
            if sp.start == moment("2026-10-21T08:00")
    ));
    let moved = |t, from: &str, to: &str| Diagnostic::TaskConstraintMoved {
        task: uid(t),
        from: moment(from),
        to: moment(to),
    };
    let mut want = vec![
        moved(7, "2026-10-19T12:30", "2026-10-19T13:00"),
        moved(9, "2026-10-24T10:00", "2026-10-26T08:00"),
        moved(10, "2026-10-25T12:00", "2026-10-23T17:00"),
        moved(11, "2026-10-21T00:00", "2026-10-20T17:00"),
        Diagnostic::LinkNotHonoredConstraint {
            predecessor: uid(1),
            successor: uid(5),
        },
        Diagnostic::LinkNotHonoredConstraint {
            predecessor: uid(1),
            successor: uid(8),
        },
        Diagnostic::TaskDeadlineMissed {
            task: uid(16),
            deadline: moment("2026-10-21T17:00"),
            finish: moment("2026-10-22T17:00"),
        },
        Diagnostic::TaskDeadlineMissed {
            task: uid(23),
            deadline: moment("2026-10-19T12:00"),
            finish: moment("2026-10-19T17:00"),
        },
        Diagnostic::TaskConstraintOnSummary {
            task: uid(21),
            constraint: planrs_core::ConstraintKind::MustStartOn,
        },
    ];
    want.sort();
    assert_eq!(s.diagnostics, want);
}
