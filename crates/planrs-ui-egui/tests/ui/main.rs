//! Tests d'interface `egui_kittest` **sans rendu** (aucun GPU) : parcours de
//! l'accueil à l'exemple, menus, barre d'outils, langue, thème, zoom,
//! ouverture annulée, bandeau d'erreur, sélection et défilement synchronisé.
//!
//! Cible unique `ui` (un seul binaire lié à wgpu, ADR-0011, décision 12).
//! Les libellés cherchés sont construits par `Texts` (jamais en dur : le
//! français contient des espaces insécables U+00A0 avant « : »). Le harnais
//! panique sur un glyphe absent des polices et sur un widget sans nom
//! accessible : ces deux pannes sont donc couvertes par chaque test.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use planrs_core::Moment;
use planrs_ui_egui::i18n::{Arg, TextId, Texts};
use planrs_ui_egui::{Action, App, Env, Language, LanguagePref, ThemePref, TimeScaleZoom, UiState};

/// « Maintenant » fixe des tests (date d'état de l'exemple).
fn now() -> Moment {
    "2027-01-15T08:00".parse().unwrap()
}

/// Application de test : français, thème clair, aucun sélecteur de fichiers.
fn new_app() -> App {
    let state = UiState {
        language: LanguagePref::Fr,
        theme: ThemePref::Light,
        ..UiState::default()
    };
    App::new(state, Env::fixed(now()))
}

/// Harnais 1280×720, échelle 1, thème clair, sans rendu.
fn harness() -> Harness<'static, App> {
    Harness::builder()
        .with_size(egui::vec2(1280.0, 720.0))
        .with_pixels_per_point(1.0)
        .with_theme(egui::Theme::Light)
        .with_os(egui::os::OperatingSystem::Nix)
        .with_max_steps(8)
        .build_ui_state(planrs_ui_egui::ui, new_app())
}

/// Libellé français d'un message.
fn fr(id: TextId) -> String {
    Texts::new(Language::Fr).get(id)
}

/// Clique sur l'unique widget portant `label`, puis laisse l'interface se
/// stabiliser.
fn click(harness: &mut Harness<'static, App>, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

/// Ouvre l'exemple depuis l'écran d'accueil.
fn open_demo(harness: &mut Harness<'static, App>) {
    click(harness, &fr(TextId::WelcomeOpenDemo));
    assert!(
        harness.state().document.is_some(),
        "exemple non ouvert : {:?}",
        harness.state().messages()
    );
}

#[test]
fn welcome_open_demo_loads_project() {
    let mut harness = harness();
    assert!(harness.state().document.is_none());
    assert!(harness.query_by_label(&fr(TextId::StatusNoDocument)).is_some());
    open_demo(&mut harness);
    let app = harness.state();
    let document = app.document.as_ref().unwrap();
    assert!(document.origin.demo);
    assert_eq!(document.origin.name, fr(TextId::DemoFileName));
    assert_eq!(document.generation, 1);
    assert_eq!(document.project.tasks.len(), 49);
    assert!(app.messages().is_empty(), "{:?}", app.messages());
    assert_eq!(app.last_frame.gantt_rows, app.last_frame.table_rows);
    // L'accueil a disparu ; la barre d'état affiche le nom de l'exemple.
    assert!(harness.query_by_label(&fr(TextId::WelcomeOpenDemo)).is_none());
    assert!(harness.query_by_label(&fr(TextId::DemoFileName)).is_some());
}

#[test]
fn ui_settles() {
    let mut harness = harness();
    assert!(
        harness.try_run().is_ok(),
        "l'accueil redemande des images sans fin"
    );
    open_demo(&mut harness);
    assert!(
        harness.try_run().is_ok(),
        "la vue principale redemande des images sans fin"
    );
}

#[test]
fn language_switch_relabels() {
    let mut harness = harness();
    let file_fr = fr(TextId::MenuFile);
    let file_en = Texts::new(Language::En).get(TextId::MenuFile);
    assert!(harness.query_by_label(&file_fr).is_some());
    harness
        .state_mut()
        .dispatch(Action::SetLanguage(LanguagePref::En));
    harness.run();
    assert_eq!(harness.state().texts().language(), Language::En);
    assert!(harness.query_by_label(&file_en).is_some(), "« File » absent");
    assert!(
        harness.query_by_label(&file_fr).is_none(),
        "« Fichier » encore présent"
    );
}

#[test]
fn language_switch_renames_demo_document() {
    let mut harness = harness();
    open_demo(&mut harness);
    // Bouton « English » de la barre d'outils (seul visible sans menu ouvert).
    click(&mut harness, &fr(TextId::LangEn));
    let app = harness.state();
    assert_eq!(app.state.language, LanguagePref::En);
    let name = &app.document.as_ref().unwrap().origin.name;
    assert_eq!(name, &Texts::new(Language::En).get(TextId::DemoFileName));
}

#[test]
fn zoom_buttons_change_zoom() {
    let mut harness = harness();
    open_demo(&mut harness);
    for zoom in [
        TimeScaleZoom::Day,
        TimeScaleZoom::Week,
        TimeScaleZoom::Month,
        TimeScaleZoom::Quarter,
    ] {
        click(&mut harness, &fr(zoom.label()));
        assert_eq!(harness.state().state.zoom, zoom);
    }
}

#[test]
fn zoom_step_and_horizontal_scroll_are_bounded() {
    let mut harness = harness();
    open_demo(&mut harness);
    click(&mut harness, &fr(TimeScaleZoom::Week.label()));
    harness.state_mut().dispatch(Action::ScrollGanttX(-500.0));
    harness.run();
    assert!(harness.state().view().gantt_scroll_x.abs() < f32::EPSILON);
    harness.state_mut().dispatch(Action::ScrollGanttX(1.0e9));
    harness.run();
    let far = harness.state().view().gantt_scroll_x;
    assert!(far.is_finite() && far >= 0.0, "défilement non borné : {far}");
    harness.state_mut().dispatch(Action::ZoomStep {
        steps: 1,
        anchor_x: 900.0,
    });
    harness.run();
    assert_eq!(harness.state().state.zoom, TimeScaleZoom::Day);
    harness.state_mut().dispatch(Action::ZoomStep {
        steps: -10,
        anchor_x: f32::NAN,
    });
    harness.run();
    assert_eq!(harness.state().state.zoom, TimeScaleZoom::Quarter);
    let scroll = harness.state().view().gantt_scroll_x;
    assert!(scroll.is_finite() && scroll >= 0.0);
}

#[test]
fn theme_switch_changes_ctx_theme() {
    let mut harness = harness();
    open_demo(&mut harness);
    assert_eq!(harness.ctx.theme(), egui::Theme::Light);
    click(&mut harness, &fr(TextId::ThemeDark));
    assert_eq!(harness.state().state.theme, ThemePref::Dark);
    assert_eq!(harness.ctx.theme(), egui::Theme::Dark);
    click(&mut harness, &fr(TextId::ThemeLight));
    assert_eq!(harness.ctx.theme(), egui::Theme::Light);
}

#[test]
fn open_dialog_cancelled_keeps_welcome() {
    let mut harness = harness();
    click(&mut harness, &fr(TextId::MenuOpen));
    let app = harness.state();
    assert!(!app.is_opening(), "l'annulation doit libérer l'ouverture");
    assert!(app.document.is_none());
    assert!(app.messages().is_empty(), "{:?}", app.messages());
}

#[test]
fn open_error_shows_dismissable_banner() {
    let mut harness = harness();
    open_demo(&mut harness);
    harness
        .state_mut()
        .open_bytes(b"pas du MSPDI", planrs_ui_egui::DocOrigin::default());
    harness.run();
    assert_eq!(harness.state().messages().len(), 1);
    // Le document précédent reste affiché.
    assert_eq!(harness.state().document.as_ref().unwrap().generation, 1);
    assert!(harness.query_by_label(&fr(TextId::OpenErrorTitle)).is_some());
    click(&mut harness, &fr(TextId::MenuClose));
    assert!(harness.state().messages().is_empty());
    assert!(harness.query_by_label(&fr(TextId::OpenErrorTitle)).is_none());
}

#[test]
fn close_document_returns_to_welcome() {
    let mut harness = harness();
    open_demo(&mut harness);
    harness.state_mut().dispatch(Action::CloseDocument);
    harness.run();
    assert!(harness.state().document.is_none());
    assert!(harness.query_by_label(&fr(TextId::WelcomeOpenDemo)).is_some());
}

#[test]
fn diagnostics_panel_toggles() {
    let mut harness = harness();
    open_demo(&mut harness);
    let no_errors = Texts::new(Language::Fr).get_args(TextId::StatusErrors, &[("count", Arg::Count(0))]);
    click(&mut harness, &no_errors);
    assert!(harness.state().state.diagnostics_open);
    // Le panneau (exemple sans diagnostic) répète les compteurs.
    assert_eq!(harness.query_all_by_label(&no_errors).count(), 2);
    harness.state_mut().dispatch(Action::ToggleDiagnostics);
    harness.run();
    assert!(!harness.state().state.diagnostics_open);
}

#[test]
fn click_row_selects_task() {
    let mut harness = harness();
    open_demo(&mut harness);
    let label = Texts::new(Language::Fr).get_args(
        TextId::RowLabel,
        &[
            ("id", Arg::Count(3)),
            ("name", Arg::Str("Plans de l'architecte".into())),
        ],
    );
    click(&mut harness, &label);
    let app = harness.state();
    let expected = app.rows().unwrap().row(2).unwrap().uid;
    assert_eq!(app.selection, Some(expected));
}

#[test]
fn wheel_scroll_keeps_panes_aligned() {
    let mut harness = harness();
    open_demo(&mut harness);
    harness.hover_at(egui::pos2(1000.0, 400.0));
    for _ in 0..20 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -5.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
    let stats = &harness.state().last_frame;
    assert!(stats.table_rows.start > 0, "aucun défilement : {stats:?}");
    assert_eq!(stats.table_rows.start, stats.gantt_rows.start);
}
