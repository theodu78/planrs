//! R-104 — FF/SF, chemins critiques multiples, seuil, marge négative ⇒ critique, FS non tronquée.
//!
//! Lot K1 (J3, ADR-0010). Une borne de fin entrante (FF, SF) est vérifiée
//! contre un placement épinglé en temps ouvré de `P(t)` (comme R-063) ; le
//! plafond d'une contrainte se combine avec R-051 (plusieurs chemins
//! critiques : `LF ≤ EF` sans successeur) par le minimum ; une marge totale
//! négative rend la tâche critique (`TF ≤ seuil`, R-050) ; la marge libre
//! n'est jamais tronquée (R-049 : `FS = min(TF, écarts)`, négative si TF
//! l'est ; `FS ≥ 0` reste vrai quand `TF ≥ 0`).

use planrs_core::{Constraint, Diagnostic, LinkType, Minutes};
use planrs_tests::moment;
use planrs_tests::net::{Net, lag_w};

#[test]
fn r104_finish_link_into_pinned_task() {
    // P 2 j (lun – mar), C 1 j FF de P (début mar 08:00) ; C SNLT lun 19/10
    // 08:00, option vraie : C lun 08:00 – 17:00 ; `between(lun 17:00, mar
    // 17:00)` = 480 > 0 → lien P → C non respecté.
    let run = Net::new("2026-10-19T08:00")
        .task("P", 960)
        .task("C", 480)
        .link("P", "C", LinkType::FinishToFinish, lag_w(0))
        .constraint(
            "C",
            Constraint::StartNoLaterThan {
                date: moment("2026-10-19T08:00"),
            },
        )
        .run();
    assert_eq!(run.at("C").early.start, moment("2026-10-19T08:00"));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::LinkNotHonoredConstraint {
            predecessor: run.uid("P"),
            successor: run.uid("C"),
        }]
    );
}

fn n() -> Net {
    Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("C", 480)
        .task("X", 2_880)
        .fs("A", "B")
        .fs("B", "C")
}

#[test]
fn r104_ceiling_with_multiple_critical_paths() {
    // Plusieurs chemins critiques : C sans successeur, LF(C) = min(fin du
    // projet lun 26 17:00, EF jeu 17:00, FNLT mer 17:00) = mer 17:00 →
    // TF(C) = −480, critique.
    let run = n()
        .options(|o| o.multiple_critical_paths = true)
        .honor(false)
        .constraint(
            "C",
            Constraint::FinishNoLaterThan {
                date: moment("2026-10-21T17:00"),
            },
        )
        .run();
    assert_eq!(run.at("C").late.finish, moment("2026-10-21T17:00"));
    assert_eq!(run.at("C").total_slack, Minutes::new(-480));
    assert!(run.at("C").critical);
}

#[test]
fn r104_threshold_and_negative_slack() {
    // Seuil 1 j : C SNLT ven 23/10 08:00 → TF(C) = `between(jeu 08:00, ven
    // 08:00)` = 480 ≤ 480 → critique. Sans seuil, non critique.
    let snlt = Constraint::StartNoLaterThan {
        date: moment("2026-10-23T08:00"),
    };
    let run = n()
        .options(|o| o.critical_slack_limit_days = 1)
        .constraint("C", snlt)
        .run();
    assert_eq!(run.at("C").total_slack, Minutes::new(480));
    assert!(run.at("C").critical);
    let run = n().constraint("C", snlt).run();
    assert!(!run.at("C").critical);
    // TF ≥ 0 ⇒ FS ≥ 0 ; une marge négative est critique et la marge libre
    // n'est pas tronquée (MSO mer 21/10 08:00 sur C : FS(B) = −480).
    let run = n()
        .constraint(
            "C",
            Constraint::MustStartOn {
                date: moment("2026-10-21T08:00"),
            },
        )
        .run();
    assert_eq!(run.at("B").free_slack, Minutes::new(-480));
    assert!(run.at("B").critical);
    assert!(run.at("X").free_slack >= Minutes::ZERO);
}
