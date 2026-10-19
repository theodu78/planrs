//! Captures de référence (`tests/snapshots/*.png`), comparées seulement si
//! `PLANRS_UI_SNAPSHOTS=1` (voir `common`). Chaque test vérifie aussi l'état
//! obtenu, dans tous les modes.
//!
//! Mise à jour des références (Linux, lavapipe) :
//! `PLANRS_UI_SNAPSHOTS=1 UPDATE_SNAPSHOTS=1 WGPU_BACKEND=vulkan cargo test -p planrs-ui-egui --test ui`.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use planrs_ui_egui::i18n::{TextId, Texts};
use planrs_ui_egui::timescale::TimeAxis;
use planrs_ui_egui::{Action, App, DocOrigin, Language, LanguagePref, ThemePref, TimeScaleZoom};

use crate::common::{harness, snapshot};

/// Ouvre l'exemple et laisse le premier dessin ajuster le zoom.
fn demo(language: LanguagePref, theme: ThemePref) -> Harness<'static, App> {
    let mut harness = harness(language, theme);
    harness.run();
    harness.state_mut().open_demo();
    harness.run();
    let app = harness.state();
    assert!(
        app.document.is_some(),
        "exemple non ouvert : {:?}",
        app.messages()
    );
    assert!(!app.view().needs_fit, "zoom ajusté pas encore appliqué");
    assert!(app.messages().is_empty(), "{:?}", app.messages());
    assert_eq!(app.last_frame.table_rows, app.last_frame.gantt_rows);
    assert!(app.last_frame.table_rows.end > app.last_frame.table_rows.start);
    harness
}

/// Change le zoom et laisse l'interface se stabiliser.
fn set_zoom(harness: &mut Harness<'static, App>, zoom: TimeScaleZoom) {
    harness.state_mut().dispatch(Action::SetZoom(zoom));
    harness.run();
    assert_eq!(harness.state().state.zoom, zoom);
}

#[test]
fn snapshot_welcome_fr_light() {
    let mut harness = harness(LanguagePref::Fr, ThemePref::Light);
    harness.run();
    assert!(harness.state().document.is_none());
    let open_demo = Texts::new(Language::Fr).get(TextId::WelcomeOpenDemo);
    assert!(harness.query_by_label(&open_demo).is_some());
    snapshot(&mut harness, "welcome_fr_light");
}

#[test]
fn snapshot_main_fr_light() {
    let mut harness = demo(LanguagePref::Fr, ThemePref::Light);
    assert_eq!(harness.state().state.zoom, TimeScaleZoom::Month);
    snapshot(&mut harness, "main_fr_light");
}

#[test]
fn snapshot_main_en_dark() {
    let mut harness = demo(LanguagePref::En, ThemePref::Dark);
    assert_eq!(harness.ctx.theme(), egui::Theme::Dark);
    assert_eq!(harness.state().texts().language(), Language::En);
    snapshot(&mut harness, "main_en_dark");
}

#[test]
fn snapshot_main_fr_light_week() {
    let mut harness = demo(LanguagePref::Fr, ThemePref::Light);
    set_zoom(&mut harness, TimeScaleZoom::Week);
    snapshot(&mut harness, "main_fr_light_week");
}

#[test]
fn snapshot_main_fr_light_day() {
    let mut harness = demo(LanguagePref::Fr, ThemePref::Light);
    set_zoom(&mut harness, TimeScaleZoom::Day);
    // Défile jusqu'à deux jours avant la date d'état (abscisse calculée par
    // l'axe de l'interface, jamais par le test).
    let doc = harness.state().document.as_ref().unwrap();
    let axis = TimeAxis::new(doc.range, TimeScaleZoom::Day, 0.0);
    let target = axis.x(doc.project().effective_status_date()) - 2.0 * axis.px_per_day();
    let target = target.max(0.0);
    harness.state_mut().dispatch(Action::ScrollGanttX(target));
    harness.run();
    let scroll = harness.state().view().gantt_scroll_x;
    assert!(scroll.is_finite() && scroll > 0.0, "aucun défilement : {scroll}");
    snapshot(&mut harness, "main_fr_light_day");
}

#[test]
fn snapshot_main_fr_light_selected_scrolled() {
    let mut harness = demo(LanguagePref::Fr, ThemePref::Light);
    let uid = harness.state().rows().unwrap().row(29).unwrap().uid;
    harness.state_mut().dispatch(Action::ScrollToRow(29));
    harness.state_mut().dispatch(Action::Select(Some(uid)));
    harness.run();
    let app = harness.state();
    assert_eq!(app.selection, Some(uid));
    let rows = &app.last_frame.table_rows;
    assert!(
        rows.start > 0 && rows.contains(&29),
        "ligne 30 non visible : {rows:?}"
    );
    assert_eq!(app.last_frame.table_rows, app.last_frame.gantt_rows);
    snapshot(&mut harness, "main_fr_light_selected_scrolled");
}

#[test]
fn snapshot_diagnostics_open_fr_light() {
    let mut harness = harness(LanguagePref::Fr, ThemePref::Light);
    harness.run();
    harness.state_mut().open_bytes(b"<x>", DocOrigin::default());
    harness.run();
    assert_eq!(harness.state().messages().len(), 1, "bandeau d'erreur attendu");
    let title = Texts::new(Language::Fr).get(TextId::OpenErrorTitle);
    assert!(harness.query_by_label(&title).is_some());
    harness.state_mut().dispatch(Action::ToggleDiagnostics);
    harness.run();
    assert!(harness.state().state.diagnostics_open);
    snapshot(&mut harness, "diagnostics_open_fr_light");
}
