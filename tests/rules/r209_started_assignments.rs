//! R-209 — Affectations d'une tâche commencée (partie achevée, lot L1) :
//! affectations de travail d'une feuille achevée placées depuis le début
//! réel sur leur échelle (R-171, R-173), partie réelle `actual` = `span` ;
//! affectation à travail nul : span vide au début réel brut. La feuille
//! achevée sort de `sync_assigned_durations` et de `placed_by_assignments`
//! (R-173 amendée) : sa durée est celle de l'avancement. Source : O39 (A7
//! affectations 38 à 45).

use planrs_core::schedule::{placed_by_assignments, sync_assigned_durations};
use planrs_core::{Minutes, Span};
use planrs_tests::moment;
use planrs_tests::net::Net;

use crate::r171_assignment_effective_calendar::{assign, assign_full, placed, resource};

/// B 1 j achevée, début réel lun 13:00, fin réelle mar 12:00 (D 480).
fn completed_b() -> Net {
    Net::new("2026-10-19T08:00").task("B", 480).with_task("B", |t| {
        t.progress.actual_start = Some(moment("2026-10-19T13:00"));
        t.progress.actual_finish = Some(moment("2026-10-20T12:00"));
    })
}

#[test]
fn r209_completed_assignments_from_actual_start() {
    // R1 100 % 8 h ⇒ lun 13:00 – mar 12:00, partie réelle = span ; dates au
    // plus tard = réelles.
    let run = assign(resource(completed_b(), 1, None), 1, "B", 1, 480).run();
    let a = placed(&run, 1);
    let expected = Span {
        start: moment("2026-10-19T13:00"),
        finish: moment("2026-10-20T12:00"),
    };
    assert_eq!(a.span, expected);
    assert_eq!(a.actual, Some(expected));
    assert_eq!(a.late, Some(expected));
    assert_eq!(a.remaining_start, None);
    // Avec un délai d'un jour, R2 part du début réel décalé (mar 13:00 –
    // mer 12:00), la tâche garde ses dates réelles.
    let net = resource(resource(completed_b(), 1, None), 2, None);
    let run = assign_full(assign(net, 1, "B", 1, 480), 2, "B", Some(2), (480, 10_000, 480)).run();
    assert_eq!(placed(&run, 2).span.start, moment("2026-10-20T13:00"));
    assert_eq!(placed(&run, 2).actual, Some(placed(&run, 2).span));
    assert_eq!(run.at("B").finish, moment("2026-10-20T12:00"));
}

#[test]
fn r209_zero_work_at_actual_start() {
    // Début réel lun 12:30 (pause, chômé) : R1 8 h placée depuis le début
    // réel (lun 13:00 – mar 12:00), R2 à travail nul datée au début réel
    // brut (lun 12:30 – 12:30, O39 : A7 affectations 39 à 41 à ven 08:00).
    let net = Net::new("2026-10-19T08:00").task("B", 480).with_task("B", |t| {
        t.progress.actual_start = Some(moment("2026-10-19T12:30"));
        t.progress.actual_finish = Some(moment("2026-10-20T12:00"));
    });
    let net = resource(resource(net, 1, None), 2, None);
    let run = assign(assign(net, 1, "B", 1, 480), 2, "B", 2, 0).run();
    assert_eq!(placed(&run, 1).span.start, moment("2026-10-19T13:00"));
    assert_eq!(placed(&run, 1).span.finish, moment("2026-10-20T12:00"));
    let zero = Span {
        start: moment("2026-10-19T12:30"),
        finish: moment("2026-10-19T12:30"),
    };
    assert_eq!(placed(&run, 2).span, zero);
    assert_eq!(placed(&run, 2).actual, Some(zero));
}

#[test]
fn r209_excluded_from_sync() {
    // B achevée avec R1 1 j et R2 1 j décalée de 2 j : l'union des
    // affectations (960) ne remplace pas la durée de l'avancement (480).
    let net = resource(resource(completed_b(), 1, None), 2, None);
    let mut project = assign_full(assign(net, 1, "B", 1, 480), 2, "B", Some(2), (480, 10_000, 960)).build();
    let s = planrs_core::schedule(&project);
    let b = s.scheduled(planrs_core::TaskUid::new(1)).unwrap();
    assert_eq!(b.duration, Minutes::new(480));
    assert_eq!(placed_by_assignments(&project, &s).len(), 0);
    assert_eq!(sync_assigned_durations(&mut project, &s), Vec::new());
    assert_eq!(
        project
            .tasks
            .get(planrs_core::TaskUid::new(1))
            .unwrap()
            .duration
            .minutes(),
        Minutes::new(480)
    );
    // Non achevée : la même feuille suit l'union (R-173).
    let net = resource(
        resource(Net::new("2026-10-19T08:00").task("B", 480), 1, None),
        2,
        None,
    );
    let project = assign_full(assign(net, 1, "B", 1, 480), 2, "B", Some(2), (480, 10_000, 960)).build();
    let s = planrs_core::schedule(&project);
    assert_eq!(placed_by_assignments(&project, &s).len(), 1);
}
