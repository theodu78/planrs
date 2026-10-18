//! R-108 — Échéance : plafond propagé et dépassement (feuille, manuelle, jalon).
//!
//! Lot K1 (J3, ADR-0010). Une échéance ne déplace aucune date planifiée :
//! elle plafonne la fin au plus tard (`LF = min(LF, échéance)`, brute, puis
//! normalisée par `backward_span`), donc la marge totale, qui devient
//! négative en cas de dépassement, et le plafond se propage aux
//! prédécesseurs. `task.deadline_missed` (avertissement) si la fin planifiée
//! dépasse l'échéance en temps ouvré de `P(t)`. Feuille automatique ou
//! manuelle active ; inactive ignorée (R-103). Observé : A7 UID 30
//! (manuelle, `LateFinish` = échéance, O16).

use planrs_core::{Constraint, Diagnostic, Minutes};
use planrs_tests::moment;
use planrs_tests::net::Net;

#[test]
fn r108_deadline_caps_late_finish() {
    // A 1 j (lun), D 4 j seule (lun – jeu) : échéance de A mer 21/10 17:00 →
    // TF(A) 960 (sans : 1 440), aucun diagnostic.
    let run = Net::new("2026-10-19T08:00")
        .task("A", 480)
        .task("D", 1_920)
        .deadline("A", "2026-10-21T17:00")
        .run();
    assert_eq!(run.at("A").early.start, moment("2026-10-19T08:00"));
    assert_eq!(run.at("A").late.finish, moment("2026-10-21T17:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(960));
    assert!(run.schedule.diagnostics.is_empty());
}

#[test]
fn r108_missed_deadline() {
    // A 2 j, échéance lun 19/10 17:00 : A reste lun – mar ; TF = FS = −480,
    // critique, échéance dépassée.
    let run = Net::new("2026-10-19T08:00")
        .task("A", 960)
        .deadline("A", "2026-10-19T17:00")
        .run();
    let a = run.at("A");
    assert_eq!(a.early.finish, moment("2026-10-20T17:00"));
    assert_eq!(a.finish, moment("2026-10-20T17:00"));
    assert_eq!(a.total_slack, Minutes::new(-480));
    assert_eq!(a.free_slack, Minutes::new(-480));
    assert!(a.critical);
    assert_eq!(
        run.schedule.diagnostics,
        [Diagnostic::TaskDeadlineMissed {
            task: run.uid("A"),
            deadline: moment("2026-10-19T17:00"),
            finish: moment("2026-10-20T17:00"),
        }]
    );
}

#[test]
fn r108_deadline_propagates_upstream() {
    // X 2 j → A 1 j (mer), échéance de A lun 19/10 12:00 : LS(A) ven 16/10
    // 13:00, TF(A) −1 200 ; LF(X) ven 16/10 12:00, LS(X) mer 14/10 13:00,
    // TF(X) −1 200.
    let run = Net::new("2026-10-19T08:00")
        .task("X", 960)
        .task("A", 480)
        .fs("X", "A")
        .deadline("A", "2026-10-19T12:00")
        .run();
    assert_eq!(run.at("A").late.start, moment("2026-10-16T13:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(-1_200));
    assert_eq!(run.at("X").late.finish, moment("2026-10-16T12:00"));
    assert_eq!(run.at("X").late.start, moment("2026-10-14T13:00"));
    assert_eq!(run.at("X").total_slack, Minutes::new(-1_200));
}

#[test]
fn r108_manual_task_deadline() {
    // M manuelle lun 08:00 – 17:00, échéance lun 12:00 : LF lun 12:00, LS ven
    // 16/10 13:00 → TF −240, critique, dépassée ; dates saisies inchangées.
    let run = Net::new("2026-10-19T08:00")
        .task("M", 480)
        .manual("M", Some("2026-10-19T08:00"), Some("2026-10-19T17:00"))
        .deadline("M", "2026-10-19T12:00")
        .run();
    assert_eq!(run.at("M").early.finish, moment("2026-10-19T17:00"));
    assert_eq!(run.at("M").total_slack, Minutes::new(-240));
    assert!(run.at("M").critical);
    assert!(run.codes().contains(&"task.deadline_missed"));
}

#[test]
fn r108_milestone_deadline_in_working_time() {
    // A 2 j → B 1 j → M jalon (mer 17:00). Échéance mer 17:00 : non
    // dépassée ; mer 12:00 : dépassée (240 min ouvrées).
    let net = |deadline: &str| {
        Net::new("2026-10-19T08:00")
            .task("A", 960)
            .task("B", 480)
            .task("M", 0)
            .fs("A", "B")
            .fs("B", "M")
            .deadline("M", deadline)
            .run()
    };
    let run = net("2026-10-21T17:00");
    assert_eq!(run.at("M").finish, moment("2026-10-21T17:00"));
    assert!(run.schedule.diagnostics.is_empty());
    let run = net("2026-10-21T12:00");
    assert!(run.codes().contains(&"task.deadline_missed"));
    assert!(run.at("M").total_slack < Minutes::ZERO);
}

#[test]
fn r108_raw_finish_on_same_working_time_is_not_missed() {
    // B FNET lun 26/10 08:00 (fin brute lun 08:00), échéance ven 23/10
    // 17:00 : même temps ouvré, échéance non dépassée, TF(B) 0.
    let run = Net::new("2026-10-19T08:00")
        .task("B", 480)
        .constraint(
            "B",
            Constraint::FinishNoEarlierThan {
                date: moment("2026-10-26T08:00"),
            },
        )
        .deadline("B", "2026-10-23T17:00")
        .run();
    assert_eq!(run.at("B").finish, moment("2026-10-26T08:00"));
    assert_eq!(run.at("B").total_slack, Minutes::ZERO);
    assert!(run.schedule.diagnostics.is_empty());
}
