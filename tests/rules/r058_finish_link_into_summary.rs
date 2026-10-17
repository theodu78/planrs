//! R-058 — Lien FF ou SF vers une récapitulative : déplié en borne de fin
//! sur chaque feuille active descendante, avec l'avertissement
//! `link.finish_to_summary`. Source : hypothèse (MS Project le refuserait
//! peut-être à la saisie).

use planrs_core::{Diagnostic, LinkType};
use planrs_tests::moment;
use planrs_tests::net::{Net, lag_w};

#[test]
fn r058_finish_to_finish_unfolded_on_leaves() {
    // X 3 j (lun – mer) FF → S {A 1 j, B 1 j} → A, B mer 08:00 – 17:00.
    let run = Net::new("2026-10-19T08:00")
        .task("X", 1_440)
        .task("S", 0)
        .task("A", 480)
        .level("A", 2)
        .task("B", 480)
        .level("B", 2)
        .link("X", "S", LinkType::FinishToFinish, lag_w(0))
        .run();
    for name in ["A", "B"] {
        assert_eq!(run.at(name).early.start, moment("2026-10-21T08:00"), "{name}");
        assert_eq!(run.at(name).early.finish, moment("2026-10-21T17:00"), "{name}");
    }
    assert!(
        run.schedule
            .diagnostics
            .contains(&Diagnostic::LinkFinishToSummary {
                predecessor: run.uid("X"),
                successor: run.uid("S"),
            })
    );
}

#[test]
fn r058_start_to_finish_unfolded_on_nested_leaves() {
    // X 1 j SF+2 j → S1 {S2 {A 1 j}, B 1 j} : borne de fin add(lun 08:00,
    // 960) = mar 17:00 → A et B mar 08:00 – 17:00 (feuilles à tout niveau).
    let run = Net::new("2026-10-19T08:00")
        .task("X", 480)
        .task("S1", 0)
        .task("S2", 0)
        .level("S2", 2)
        .task("A", 480)
        .level("A", 3)
        .task("B", 480)
        .level("B", 2)
        .link("X", "S1", LinkType::StartToFinish, lag_w(960))
        .run();
    for name in ["A", "B"] {
        assert_eq!(run.at(name).early.start, moment("2026-10-20T08:00"), "{name}");
    }
    assert!(run.codes().contains(&"link.finish_to_summary"));
}
