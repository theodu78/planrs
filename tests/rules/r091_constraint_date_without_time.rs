//! R-091 — Date de contrainte sans heure (`Constraint::on_day`, R-033).
//!
//! Lot K1 (J3, ADR-0010). Un jour saisi sans heure reçoit l'heure de début
//! par défaut pour une contrainte de début (MSO, SNET, SNLT), l'heure de fin
//! par défaut pour une contrainte de fin (MFO, FNET, FNLT) ; ASAP et ALAP
//! ignorent le jour. Une échéance sans heure suit la convention d'une fin
//! (`ProjectOptions::finish_of`). La normalisation sur le calendrier vient
//! au calcul (R-090).

use planrs_core::{Constraint, ConstraintKind, Diagnostic, ProjectOptions, TimeOfDay};
use planrs_tests::net::Net;
use planrs_tests::{day, moment};

#[test]
fn r091_start_and_finish_constraints_get_default_times() {
    let o = ProjectOptions::default();
    // SNET le 2026-10-21 → mer 08:00 ; FNLT le 2026-10-22 → jeu 17:00.
    assert_eq!(
        Constraint::on_day(ConstraintKind::StartNoEarlierThan, day("2026-10-21"), &o),
        Ok(Constraint::StartNoEarlierThan {
            date: moment("2026-10-21T08:00")
        })
    );
    assert_eq!(
        Constraint::on_day(ConstraintKind::FinishNoLaterThan, day("2026-10-22"), &o),
        Ok(Constraint::FinishNoLaterThan {
            date: moment("2026-10-22T17:00")
        })
    );
    for (kind, time) in [
        (ConstraintKind::MustStartOn, "08:00"),
        (ConstraintKind::StartNoLaterThan, "08:00"),
        (ConstraintKind::MustFinishOn, "17:00"),
        (ConstraintKind::FinishNoEarlierThan, "17:00"),
    ] {
        let c = Constraint::on_day(kind, day("2026-10-21"), &o).unwrap();
        assert_eq!(c.kind(), kind);
        assert_eq!(c.date(), Some(moment(&format!("2026-10-21T{time}"))));
    }
    // Échéance : heure de fin.
    assert_eq!(o.finish_of(day("2026-10-21")), Ok(moment("2026-10-21T17:00")));
}

#[test]
fn r091_asap_and_alap_ignore_the_day() {
    let o = ProjectOptions::default();
    assert_eq!(
        Constraint::on_day(ConstraintKind::AsSoonAsPossible, day("2026-10-21"), &o),
        Ok(Constraint::AsSoonAsPossible)
    );
    assert_eq!(
        Constraint::on_day(ConstraintKind::AsLateAsPossible, day("2026-10-21"), &o),
        Ok(Constraint::AsLateAsPossible)
    );
}

#[test]
fn r091_custom_default_start_then_normalized() {
    // Début par défaut 07:00 : SNET le mer 21/10 → 07:00, normalisée à 08:00
    // au calcul (R-090), avec `task.constraint_moved`.
    let o = ProjectOptions {
        default_start_time: TimeOfDay::hm(7, 0).unwrap(),
        ..ProjectOptions::default()
    };
    let c = Constraint::on_day(ConstraintKind::StartNoEarlierThan, day("2026-10-21"), &o).unwrap();
    assert_eq!(c.date(), Some(moment("2026-10-21T07:00")));
    let run = Net::new("2026-10-19T08:00")
        .task("A", 480)
        .constraint("A", c)
        .run();
    assert_eq!(run.at("A").early.start, moment("2026-10-21T08:00"));
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskConstraintMoved {
            task: run.uid("A"),
            from: moment("2026-10-21T07:00"),
            to: moment("2026-10-21T08:00"),
        }]
    );
}
