//! R-103 — Contrainte d'une manuelle ou d'une inactive.
//!
//! Lot K1 (J3, ADR-0010). Une tâche manuelle garde ses dates saisies : sa
//! contrainte est ignorée sans diagnostic (son échéance est évaluée,
//! R-108) ; une récapitulative manuelle datée aussi. Une tâche inactive est
//! placée pour l'affichage avec sa contrainte (même calcul qu'une
//! automatique, R-062), sans aucun diagnostic ni échéance. Observé (indice
//! faible) : A7 UID 4, manuelle achevée, fin avant sa date FNET (O19).

use planrs_core::{Constraint, Exclusion, Span, TaskOutcome};
use planrs_tests::moment;
use planrs_tests::net::Net;

#[test]
fn r103_manual_task_ignores_its_constraint() {
    // M manuelle lun 08:00 – 17:00, SNET mer 21/10 : M reste lun, aucun
    // diagnostic, planning identique sans la contrainte.
    let net = || {
        Net::new("2026-10-19T08:00").task("M", 480).manual(
            "M",
            Some("2026-10-19T08:00"),
            Some("2026-10-19T17:00"),
        )
    };
    let with = net()
        .constraint(
            "M",
            Constraint::StartNoEarlierThan {
                date: moment("2026-10-21T08:00"),
            },
        )
        .run();
    let without = net().run();
    assert_eq!(with.at("M").early.start, moment("2026-10-19T08:00"));
    assert_eq!(with.schedule, without.schedule);
}

#[test]
fn r103_inactive_task_display_follows_constraint() {
    // I inactive 1 j SNET mer 21/10 08:00 : exclue, affichée mer 08:00 –
    // 17:00 ; avec une échéance lun 12:00 : aucun diagnostic.
    let run = Net::new("2026-10-19T08:00")
        .task("I", 480)
        .inactive("I")
        .constraint(
            "I",
            Constraint::StartNoEarlierThan {
                date: moment("2026-10-21T08:00"),
            },
        )
        .deadline("I", "2026-10-19T12:00")
        .run();
    assert_eq!(
        *run.outcome("I"),
        TaskOutcome::Excluded {
            reason: Exclusion::Inactive,
            display: Some(Span {
                start: moment("2026-10-21T08:00"),
                finish: moment("2026-10-21T17:00"),
            }),
        }
    );
    assert!(run.schedule.diagnostics.is_empty());
}
