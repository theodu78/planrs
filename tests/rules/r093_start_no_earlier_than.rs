//! R-093 — SNET.
//!
//! Lot K1 (J3, ADR-0010). « Début au plus tôt le » : plancher de début `D'`
//! (R-090) évalué après les autres bornes de la passe avant (R-044) : il ne
//! pilote que s'il retarde strictement le début placé. Aucun conflit
//! possible ; sans effet sur la passe arrière ; mêmes résultats quelle que
//! soit l'option. Réseau Nc : A 2 j (lun – mar), B 1 j FS de A, D 5 j seule.

use planrs_core::{Constraint, ConstraintKind, Driver, LinkType, Minutes};
use planrs_tests::moment;
use planrs_tests::net::{Net, Run};

fn nc(date: &str, honor: bool) -> Run {
    Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("D", 2_400)
        .fs("A", "B")
        .honor(honor)
        .constraint("B", Constraint::StartNoEarlierThan { date: moment(date) })
        .run()
}

#[test]
fn r093_binding_floor() {
    // SNET jeu 22/10 08:00 → B jeu 08:00 – 17:00, pilote `Constraint` ;
    // LS(B) ven 08:00 → TF(B) 480 ; LF(A) jeu 17:00 → TF(A) 960 ; FS(A) =
    // `between(mar 17:00, jeu 08:00)` = 480. Identique sans l'option.
    for honor in [true, false] {
        let run = nc("2026-10-22T08:00", honor);
        let b = run.at("B");
        assert_eq!(b.early.start, moment("2026-10-22T08:00"));
        assert_eq!(b.early.finish, moment("2026-10-22T17:00"));
        assert_eq!(
            b.driver,
            Driver::Constraint {
                constraint: ConstraintKind::StartNoEarlierThan
            }
        );
        assert_eq!(b.total_slack, Minutes::new(480));
        assert_eq!(run.at("A").total_slack, Minutes::new(960));
        assert_eq!(run.at("A").free_slack, Minutes::new(480));
        assert!(run.schedule.diagnostics.is_empty());
    }
}

#[test]
fn r093_non_binding_and_tied_floor_keep_the_link_driver() {
    // SNET lun 19/10 08:00 : sans effet. SNET mer 21/10 08:00 = début placé
    // par le lien (borne brute mar 17:00) : même début, le lien reste
    // pilote.
    for date in ["2026-10-19T08:00", "2026-10-21T08:00"] {
        let run = nc(date, true);
        let b = run.at("B");
        assert_eq!(b.early.start, moment("2026-10-21T08:00"), "{date}");
        assert_eq!(
            b.driver,
            Driver::Link {
                predecessor: run.uid("A"),
                link: LinkType::FinishToStart
            },
            "{date}"
        );
        assert_eq!(b.total_slack, Minutes::new(960));
    }
}
