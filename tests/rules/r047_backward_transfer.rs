//! R-047 — Passe arrière : transfert adjoint. Plafond de fin FS `h(LS_s)`,
//! FF `h(LF_s)` ; plafond de début SS `h(LS_s)`, SF `h(LF_s)` converti par
//! `forward_span(y, d).finish` ; `late = backward_span(min, d)` ;
//! `h(y) = next_working_σ(add_σ(y, −L))`. Source : S22 ; adjoint :
//! hypothèse (aucun décalage dans le corpus).

use planrs_core::{LinkType, Minutes};
use planrs_tests::moment;
use planrs_tests::net::{Net, lag_e, lag_w};

#[test]
fn r047_fs_network() {
    // N1 : C mer 08:00 – 17:00 ; B plafond h(mer 08:00) = mer 08:00 →
    // backward_span(mer 08:00, 960) = lun 08:00 – mar 17:00 ; A mar 08:00 –
    // 17:00.
    let run = Net::new("2026-10-19T08:00")
        .task("A", 480)
        .task("B", 960)
        .task("C", 480)
        .task("D", 480)
        .fs("A", "C")
        .fs("B", "C")
        .run();
    assert_eq!(run.at("C").late.start, moment("2026-10-21T08:00"));
    assert_eq!(run.at("B").late.start, moment("2026-10-19T08:00"));
    assert_eq!(run.at("B").late.finish, moment("2026-10-20T17:00"));
    assert_eq!(run.at("A").late.start, moment("2026-10-20T08:00"));
    assert_eq!(run.at("A").late.finish, moment("2026-10-20T17:00"));
}

#[test]
fn r047_mixed_calendars_with_lead() {
    // Début ven 23/10 08:00 ; P (24 heures) 960 min : ven 08:00 → sam 24/10
    // 00:00 ; Q (Standard) 1 j FS −60 min : add_std(sam 00:00, −60) =
    // ven 16:00 → Q ven 16:00 – lun 26/10 16:00 = fin du projet.
    // Retour : h(ven 16:00) = next_std(add_std(ven 16:00, +60)) =
    // next_std(ven 17:00) = lun 08:00 → P LF lun 08:00, TF(P) = 3 360 (56 h).
    // Le miroir naïf donnerait LF ven 17:00 et TF −420.
    let run = Net::new("2026-10-23T08:00")
        .task("P", 960)
        .calendar("P", 2)
        .task("Q", 480)
        .link("P", "Q", LinkType::FinishToStart, lag_w(-60))
        .run();
    assert_eq!(run.at("P").early.finish, moment("2026-10-24T00:00"));
    assert_eq!(run.at("Q").early.start, moment("2026-10-23T16:00"));
    assert_eq!(run.at("Q").early.finish, moment("2026-10-26T16:00"));
    assert_eq!(run.at("P").late.finish, moment("2026-10-26T08:00"));
    assert_eq!(run.at("P").total_slack, Minutes::new(3_360));
    assert_eq!(run.at("Q").total_slack, Minutes::ZERO);
}

#[test]
fn r047_ff_cap() {
    // A 2 j ; B FF 1 j (mar 08:00 – 17:00) ; D 3 j seul : fin du projet
    // mer 17:00. B LF mer 17:00 ; A plafond h(LF_B) = mer 17:00 →
    // A mar 08:00 – mer 17:00, TF(A) = 480.
    let run = Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("D", 1_440)
        .link("A", "B", LinkType::FinishToFinish, lag_w(0))
        .run();
    assert_eq!(run.at("B").late.finish, moment("2026-10-21T17:00"));
    assert_eq!(run.at("A").late.start, moment("2026-10-20T08:00"));
    assert_eq!(run.at("A").late.finish, moment("2026-10-21T17:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(480));
}

#[test]
fn r047_ss_cap_final_successor() {
    // A 1 j ; B SS+4 h 1 j (lun 13:00 – mar 12:00), tâche finale : fin du
    // projet mar 12:00. B LS lun 13:00 ; A plafond de début
    // h(lun 13:00) = next(add(lun 13:00, −240)) = lun 08:00 → plafond de fin
    // forward_span(lun 08:00, 480).finish = lun 17:00 → A lun 08:00 – 17:00,
    // TF 0.
    let run = Net::new("2026-10-19T08:00")
        .task("A", 480)
        .task("B", 480)
        .link("A", "B", LinkType::StartToStart, lag_w(240))
        .run();
    assert_eq!(run.at("B").late.start, moment("2026-10-19T13:00"));
    assert_eq!(run.at("A").late.finish, moment("2026-10-19T17:00"));
    assert_eq!(run.at("A").total_slack, Minutes::ZERO);
}

#[test]
fn r047_ss_cap_with_slack() {
    // Même lien, D 3 j seul : fin du projet mer 17:00. B LS = mer 08:00 ;
    // A plafond de début h(mer 08:00) = add(mer 08:00, −240) = mar 13:00 →
    // plafond de fin forward_span(mar 13:00, 480).finish = mer 12:00 →
    // A mar 13:00 – mer 12:00, TF(A) = 720 = TF(B).
    let run = Net::new("2026-10-19T08:00")
        .task("A", 480)
        .task("B", 480)
        .task("D", 1_440)
        .link("A", "B", LinkType::StartToStart, lag_w(240))
        .run();
    assert_eq!(run.at("A").late.start, moment("2026-10-20T13:00"));
    assert_eq!(run.at("A").late.finish, moment("2026-10-21T12:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(720));
    assert_eq!(run.at("B").total_slack, Minutes::new(720));
}

#[test]
fn r047_sf_cap() {
    // A 2 j ; B SF+3 j 1 j (mer 08:00 – 17:00) ; D 4 j seul : fin du projet
    // jeu 22/10 17:00. B LF jeu 17:00 ; A plafond de début
    // h(jeu 17:00) = add(jeu 17:00, −1 440) = mar 08:00 → plafond de fin
    // forward_span(mar 08:00, 960).finish = mer 17:00 → A mar 08:00 –
    // mer 17:00, TF(A) = 480 = TF(B).
    let run = Net::new("2026-10-19T08:00")
        .task("A", 960)
        .task("B", 480)
        .task("D", 1_920)
        .link("A", "B", LinkType::StartToFinish, lag_w(1_440))
        .run();
    assert_eq!(run.at("A").late.start, moment("2026-10-20T08:00"));
    assert_eq!(run.at("A").late.finish, moment("2026-10-21T17:00"));
    assert_eq!(run.at("A").total_slack, Minutes::new(480));
    assert_eq!(run.at("B").total_slack, Minutes::new(480));
}

#[test]
fn r047_elapsed_lag_cap() {
    // Début ven 23/10 08:00 ; A 1 j ; B FS+2 ej (lun 26/10 08:00 – 17:00).
    // Retour : h(lun 08:00) = lun 08:00 − 2 880 = sam 24/10 08:00 →
    // backward_span(sam 08:00, 480) = ven 08:00 – 17:00 : TF(A) = 0.
    let run = Net::new("2026-10-23T08:00")
        .task("A", 480)
        .task("B", 480)
        .link("A", "B", LinkType::FinishToStart, lag_e(2_880))
        .run();
    assert_eq!(run.at("A").late.finish, moment("2026-10-23T17:00"));
    assert_eq!(run.at("A").total_slack, Minutes::ZERO);
    assert_eq!(run.at("A").free_slack, Minutes::ZERO);
}

#[test]
fn r047_ff_elapsed_lag_into_working_task() {
    // A 1 j ; B FF+6 em 1 j : borne lun 17:06 (temps civil), B =
    // backward_span(lun 17:06, 480) = lun 08:00 – 17:00 : sa fin est
    // ramenée à 17:00, de même temps ouvré que la borne. Retour : l'ancre
    // de B est portée à next(lun 17:00) = mar 08:00, h = mar 07:54 →
    // TF(A) = FS(A) = 0 (sans ce report : −6).
    let run = Net::new("2026-10-19T08:00")
        .task("A", 480)
        .task("B", 480)
        .link("A", "B", LinkType::FinishToFinish, lag_e(6))
        .run();
    assert_eq!(run.at("B").early.finish, moment("2026-10-19T17:00"));
    assert_eq!(run.at("A").late.finish, moment("2026-10-19T17:00"));
    assert_eq!(run.at("A").total_slack, Minutes::ZERO);
    assert_eq!(run.at("A").free_slack, Minutes::ZERO);
}
