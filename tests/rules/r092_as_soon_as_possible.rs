//! R-092 — ASAP.
//!
//! Lot K1 (J3, ADR-0010). « Le plus tôt possible » n'a aucun effet : les
//! dates, les marges, le pilote et les diagnostics sont ceux du réseau sans
//! contrainte, quelle que soit l'option « toujours respecter les dates de
//! contrainte ».

use planrs_core::{Constraint, Driver, LinkType, Minutes};
use planrs_tests::moment;
use planrs_tests::net::Net;

fn nc(honor: bool, asap: bool) -> planrs_tests::net::Run {
    let net = Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("D", 2_400)
        .fs("A", "B")
        .honor(honor);
    if asap {
        net.constraint("B", Constraint::AsSoonAsPossible).run()
    } else {
        net.run()
    }
}

#[test]
fn r092_asap_is_neutral() {
    // Nc avec B ASAP explicite : B mer 08:00 – 17:00, pilote `Link { A }`,
    // TF(B) = 960 ; identique au réseau sans contrainte et à l'option près.
    let base = nc(true, false);
    for honor in [true, false] {
        let run = nc(honor, true);
        assert_eq!(run.schedule.tasks, base.schedule.tasks);
        assert!(run.schedule.diagnostics.is_empty());
        let b = run.at("B");
        assert_eq!(b.early.start, moment("2026-10-21T08:00"));
        assert_eq!(b.total_slack, Minutes::new(960));
        assert_eq!(
            b.driver,
            Driver::Link {
                predecessor: run.uid("A"),
                link: LinkType::FinishToStart
            }
        );
    }
}
