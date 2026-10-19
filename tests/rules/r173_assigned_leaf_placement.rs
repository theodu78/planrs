//! R-173 — Placement d'une feuille affectée : chaque affectation de travail
//! est placée sur son calendrier effectif (R-171) depuis le début au plus tôt
//! du réseau, décalée de son délai (R-177) ; début de la tâche = premier
//! instant où une ressource affectée peut travailler ; fin = la plus tardive
//! des fins d'affectation ; durée calculée = temps ouvré de l'union des
//! intervalles travaillés (et non l'enveloppe). Source : S45, S14 (« total
//! span of active working time »), S8 ; O23, O26.

use planrs_core::{LinkType, Minutes, TaskSchedule};
use planrs_tests::moment;
use planrs_tests::net::{Net, lag_w};

use crate::r171_assignment_effective_calendar::{assign, assign_full, placed, resource};

#[test]
fn r173_union_not_envelope() {
    // R1 1 j, R2 1 j avec délai 2 j ⇒ R1 lun, R2 mer ; tâche lun 08:00 –
    // mer 17:00, durée 960 (union) et non 1 440 (enveloppe).
    let net = Net::new("2026-10-19T08:00").task("A", 480);
    let net = resource(resource(net, 1, None), 2, None);
    let net = assign(net, 1, "A", 1, 480);
    let run = assign_full(net, 2, "A", Some(2), (480, 10_000, 960)).run();
    let a = run.at("A");
    assert_eq!(a.start, moment("2026-10-19T08:00"));
    assert_eq!(a.finish, moment("2026-10-21T17:00"));
    assert_eq!(a.duration, Minutes::new(960));
    assert_eq!(placed(&run, 1).span.finish, moment("2026-10-19T17:00"));
    assert_eq!(placed(&run, 2).span.start, moment("2026-10-21T08:00"));
    assert_eq!(placed(&run, 2).span.finish, moment("2026-10-21T17:00"));
}

#[test]
fn r173_two_finishes() {
    // R1 8 h décalée d'un jour, R2 8 h (A6 UID 7, O26, où le décalage est
    // un retard de nivellement) ⇒ R2 lun, R1 mar ; tâche lun 08:00 – mar
    // 17:00, durée 960 ; le successeur part de la fin la plus tardive.
    let net = Net::new("2026-10-19T08:00")
        .task("A", 480)
        .task("B", 480)
        .fs("A", "B");
    let net = resource(resource(net, 1, None), 2, None);
    let net = assign_full(net, 1, "A", Some(1), (480, 10_000, 480));
    let run = assign(net, 2, "A", 2, 480).run();
    assert_eq!(run.at("A").start, moment("2026-10-19T08:00"));
    assert_eq!(run.at("A").finish, moment("2026-10-20T17:00"));
    assert_eq!(run.at("A").duration, Minutes::new(960));
    assert_eq!(run.at("B").start, moment("2026-10-21T08:00"));
    assert_eq!(placed(&run, 1).span.start, moment("2026-10-20T08:00"));
}

/// Champs de date et de marge d'une tâche planifiée.
fn dates(s: &TaskSchedule) -> (TaskSchedule, Minutes) {
    let mut c = *s;
    c.driver = planrs_core::Driver::ProjectStart;
    c.late_driver = planrs_core::LateDriver::ProjectFinish;
    (c, s.duration)
}

#[test]
fn r173_single_assignment_equals_j4() {
    // Une affectation, ressource sans calendrier : 20 h à 50 % ⇒ d_a =
    // 2 400 ; mêmes dates, marges et durée que la tâche de 2 400 min sans
    // affectation (P34), lien à décalage compris.
    let build = |minutes: i64| {
        Net::new("2026-10-19T08:00")
            .task("P", 480)
            .task("A", minutes)
            .task("Z", 4_800)
            .link("P", "A", LinkType::FinishToStart, lag_w(120))
            .with_task("A", |t| t.deadline = Some(moment("2026-10-30T17:00")))
    };
    let oracle = build(2_400).run();
    let net = resource(build(480), 1, None);
    let run = assign_full(net, 1, "A", Some(1), (1_200, 5_000, 0)).run();
    assert_eq!(dates(run.at("A")), dates(oracle.at("A")));
    assert_eq!(placed(&run, 1).span, run.at("A").early);
    assert_eq!(placed(&run, 1).late, Some(run.at("A").late));
}
