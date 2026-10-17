//! R-044 — Plusieurs prédécesseurs et pilote. ES0 = max brut des bornes ;
//! pilote = première borne maximale dans l'ordre : début du projet, porte du
//! parent, liens dans l'ordre de stockage. Source : S22 ; départage :
//! hypothèse.

use planrs_core::{Driver, LinkType};
use planrs_tests::moment;
use planrs_tests::net::Net;

#[test]
fn r044_latest_predecessor_drives() {
    // A 1 j (lun), B 2 j (lun – mar), C FS de A et de B, 1 j → mer 08:00 –
    // 17:00, pilote B.
    let run = Net::new("2026-10-19T08:00")
        .task("A", 480)
        .task("B", 960)
        .task("C", 480)
        .fs("A", "C")
        .fs("B", "C")
        .run();
    assert_eq!(run.at("C").early.start, moment("2026-10-21T08:00"));
    assert_eq!(
        run.at("C").driver,
        Driver::Link {
            predecessor: run.uid("B"),
            link: LinkType::FinishToStart
        }
    );
}

#[test]
fn r044_tie_keeps_first_stored_link() {
    // A et B 1 j, mêmes bornes : le pilote est le premier lien stocké.
    let net = || {
        Net::new("2026-10-19T08:00")
            .task("A", 480)
            .task("B", 480)
            .task("C", 480)
    };
    let ab = net().fs("A", "C").fs("B", "C").run();
    let ba = net().fs("B", "C").fs("A", "C").run();
    let driver = |r: &planrs_tests::net::Run, n: &str| Driver::Link {
        predecessor: r.uid(n),
        link: LinkType::FinishToStart,
    };
    assert_eq!(ab.at("C").driver, driver(&ab, "A"));
    assert_eq!(ba.at("C").driver, driver(&ba, "B"));
    assert_eq!(ab.at("C").early, ba.at("C").early);
}
