//! R-051 — Plusieurs chemins critiques : si l'option est active, LF = EF pour
//! toute tâche sans successeur effectif (hérités compris). Source : S20 ;
//! « par tâche finale » : hypothèse ; hypothèse concurrente consignée :
//! « par réseau indépendant ».

use planrs_core::Minutes;
use planrs_tests::net::Net;

fn run(multiple: bool) -> planrs_tests::net::Run {
    // A 1 j → B 1 j (FS) ; C 3 j seul : fin du projet mer 21/10 17:00.
    Net::new("2026-10-19T08:00")
        .task("A", 480)
        .task("B", 480)
        .task("C", 1_440)
        .fs("A", "B")
        .options(|o| o.multiple_critical_paths = multiple)
        .run()
}

#[test]
fn r051_single_critical_path_by_default() {
    let r = run(false);
    // B peut finir mer 17:00 : TF(A) = TF(B) = 480.
    assert_eq!(r.at("A").total_slack, Minutes::new(480));
    assert_eq!(r.at("B").total_slack, Minutes::new(480));
    assert!(!r.at("A").critical && !r.at("B").critical && r.at("C").critical);
}

#[test]
fn r051_each_final_task_closes_its_path() {
    // Option active : B n'a pas de successeur → LF(B) = EF(B) = mar 17:00 ;
    // TF(A) = TF(B) = 0 ; A, B, C critiques.
    let r = run(true);
    assert_eq!(r.at("B").late, r.at("B").early);
    assert_eq!(r.at("A").total_slack, Minutes::ZERO);
    assert_eq!(r.at("B").total_slack, Minutes::ZERO);
    assert!(r.at("A").critical && r.at("B").critical && r.at("C").critical);
}
