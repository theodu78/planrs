//! Captures de référence (`tests/snapshots/*.png`), comparées seulement si
//! `PLANRS_UI_SNAPSHOTS=1` (voir `common`). Chaque test vérifie aussi l'état
//! obtenu, dans tous les modes.
//!
//! J7 lot L16 (critère de fin « 5 000 tâches fluide à l'écran, vérifié par
//! captures ») : `net5000_*`, le réseau généré de 5 000 tâches et 10 000
//! liens (le même que `tests/perf.rs` et `snapshot --generate 5000`),
//! début, milieu, fin × Semaine, Trimestre × clair, sombre, 1280×720,
//! « maintenant » fixe (début du réseau) ; la ligne visée est sélectionnée
//! et le Gantt défilé au début de sa barre, comme `--row` de l'exemple.
//!
//! ADR-0017 lot L5 : ruban (un onglet par capture, anglais sombre, replié,
//! surimpression, 800 px, deux pixels par point) et chronologie (« P » avec 2, 3, 4, 6
//! et 7 sur la frise, à l'écran et en mode impression) ; toutes les autres
//! captures montrent désormais le ruban à la place des deux rangées
//! d'outils, et une frise (vide hors « P ») en tête des vues Gantt.
//!
//! Mise à jour des références (Linux, lavapipe) :
//! `PLANRS_UI_SNAPSHOTS=1 UPDATE_SNAPSHOTS=1 WGPU_BACKEND=vulkan cargo test -p planrs-ui-egui --test ui`.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use planrs_core::Moment;
use planrs_tests::generate::ui_network;
use planrs_ui_egui::i18n::{TextId, Texts};
use planrs_ui_egui::ribbon::{RibbonOp, RibbonTab};
use planrs_ui_egui::timescale::TimeAxis;
use planrs_ui_egui::view::ribbon::drawn;
use planrs_ui_egui::{
    Action, App, DocOrigin, Env, Language, LanguagePref, ThemePref, TimeScaleZoom, UiState,
};

use crate::common::{harness, harness_sized, harness_with, snapshot};

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

/// H-UI-85 (correctif de la saisie de cellule, 2026-10-27) : cellule en
/// saisie ouverte par F2, dessinée dans la cellule (police, graisse,
/// alignement et retrait de la cellule, fond uni, cadre du curseur),
/// curseur de texte en fin de texte (clignotement coupé pour une capture
/// stable). Clair : nom indenté d'une tâche ; sombre : durée alignée à
/// droite d'une récapitulative (gras simulé).
fn cell_edit_capture(theme: ThemePref, uid: u32, column: planrs_ui_egui::column::Column, name: &str) {
    let mut harness = demo(LanguagePref::Fr, theme);
    harness
        .ctx
        .all_styles_mut(|style| style.visuals.text_cursor.blink = false);
    let uid = planrs_core::TaskUid::new(uid);
    harness.state_mut().dispatch(Action::Select(Some(uid)));
    harness.run();
    harness.state_mut().view_mut().cursor.column = column;
    harness.key_press(egui::Key::F2);
    harness.run();
    let app = harness.state();
    let editing = app.view().edit.editing.as_ref().expect("saisie ouverte");
    assert_eq!((editing.uid, editing.column), (Some(uid), column));
    assert!(
        harness
            .ctx
            .memory(|memory| memory.has_focus(planrs_ui_egui::grid::editor_id()))
    );
    snapshot(&mut harness, name);
}

#[test]
fn snapshot_cell_edit_fr_light() {
    cell_edit_capture(
        ThemePref::Light,
        3,
        planrs_ui_egui::column::Column::Name,
        "cell_edit_fr_light",
    );
}

#[test]
fn snapshot_cell_edit_fr_dark() {
    cell_edit_capture(
        ThemePref::Dark,
        1,
        planrs_ui_egui::column::Column::Duration,
        "cell_edit_fr_dark",
    );
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

/// Récapitulatives × (1 + feuilles) du réseau de 5 000 tâches (`perf.rs`).
const NET_SUMMARIES: usize = 250;
const NET_LEAVES: usize = 19;
const NET_LINKS: usize = 10_000;

/// Position de la ligne visée dans le réseau.
#[derive(Clone, Copy, Debug)]
enum Position {
    Start,
    Middle,
    End,
}

impl Position {
    const fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Middle => "middle",
            Self::End => "end",
        }
    }

    const fn row(self, len: usize) -> usize {
        match self {
            Self::Start => 0,
            Self::Middle => len / 2,
            Self::End => len.saturating_sub(1),
        }
    }
}

/// « Maintenant » du réseau généré : son début (comme `snapshot --generate`).
fn net_now() -> Moment {
    "2026-01-05T08:00".parse().unwrap()
}

/// Réseau de 5 000 tâches ouvert (français, `theme`), premier dessin fait.
fn net5000(theme: ThemePref) -> Harness<'static, App> {
    let state = UiState {
        language: LanguagePref::Fr,
        theme,
        ..UiState::default()
    };
    let mut app = App::new(state, Env::fixed(net_now()));
    let origin = DocOrigin {
        name: "reseau-5000.xml".to_owned(),
        path: None,
        demo: false,
    };
    app.open_project(ui_network(NET_SUMMARIES, NET_LEAVES, NET_LINKS), origin);
    let mut harness = harness_with(app, theme);
    harness.run();
    let app = harness.state();
    let doc = app
        .document
        .as_ref()
        .unwrap_or_else(|| panic!("réseau refusé : {:?}", app.messages()));
    assert_eq!(doc.project().tasks.len(), NET_SUMMARIES * (NET_LEAVES + 1));
    assert!(
        doc.schedule().diagnostics.is_empty(),
        "{:?}",
        doc.schedule().diagnostics.first()
    );
    assert!(app.messages().is_empty(), "{:?}", app.messages());
    harness
}

/// Capture `net5000_<position>_<zoom>_<thème>` : zoom imposé, ligne visée
/// sélectionnée et montrée, Gantt défilé au début de sa barre moins deux
/// jours (abscisse par l'axe de l'interface, jamais calculée ici).
fn net5000_capture(position: Position, zoom: TimeScaleZoom, theme: ThemePref) {
    let mut harness = net5000(theme);
    set_zoom(&mut harness, zoom);
    let len = harness.state().rows().unwrap().len();
    let row = position.row(len);
    let (uid, bar) = {
        let r = harness.state().rows().unwrap().row(row).unwrap();
        (r.uid, r.bar)
    };
    harness.state_mut().dispatch(Action::Select(Some(uid)));
    harness.state_mut().dispatch(Action::ScrollToRow(row));
    harness.run();
    let app = harness.state();
    let doc = app.document.as_ref().unwrap();
    let axis = TimeAxis::new(doc.range, zoom, 0.0);
    let (start, _) = bar.expect("ligne visée sans barre");
    let target = (axis.x(start) - 2.0 * axis.px_per_day()).max(0.0);
    let delta = target - app.view().gantt_scroll_x;
    harness.state_mut().dispatch(Action::ScrollGanttX(delta));
    harness.run();
    let app = harness.state();
    assert_eq!(app.selection, Some(uid));
    assert_eq!(app.state.zoom, zoom);
    assert!(
        app.last_frame.table_rows.contains(&row),
        "ligne {row} non peinte : {:?}",
        app.last_frame.table_rows
    );
    assert_eq!(app.last_frame.table_rows, app.last_frame.gantt_rows);
    let theme_name = match theme {
        ThemePref::Dark => "dark",
        ThemePref::Light | ThemePref::System => "light",
    };
    let zoom_name = format!("{zoom:?}").to_lowercase();
    snapshot(
        &mut harness,
        &format!("net5000_{}_{zoom_name}_{theme_name}", position.name()),
    );
}

macro_rules! net5000_snapshots {
    ($($name:ident: $position:ident, $zoom:ident, $theme:ident;)*) => {
        $(
            #[test]
            fn $name() {
                net5000_capture(Position::$position, TimeScaleZoom::$zoom, ThemePref::$theme);
            }
        )*
    };
}

net5000_snapshots! {
    snapshot_net5000_start_week_light: Start, Week, Light;
    snapshot_net5000_start_week_dark: Start, Week, Dark;
    snapshot_net5000_start_quarter_light: Start, Quarter, Light;
    snapshot_net5000_start_quarter_dark: Start, Quarter, Dark;
    snapshot_net5000_middle_week_light: Middle, Week, Light;
    snapshot_net5000_middle_week_dark: Middle, Week, Dark;
    snapshot_net5000_middle_quarter_light: Middle, Quarter, Light;
    snapshot_net5000_middle_quarter_dark: Middle, Quarter, Dark;
    snapshot_net5000_end_week_light: End, Week, Light;
    snapshot_net5000_end_week_dark: End, Week, Dark;
    snapshot_net5000_end_quarter_light: End, Quarter, Light;
    snapshot_net5000_end_quarter_dark: End, Quarter, Dark;
}

// --- ADR-0017, lot L5 : ruban et chronologie ---

/// « P » ouvert (`language`, `theme`), fenêtre `size` à `ppp`, tâche 3
/// sélectionnée (éléments de sélection actifs), tâches `timeline` sur la
/// frise (un lot `style.apply`, le geste « Ajouter à la chronologie »).
fn p_capture(
    language: LanguagePref,
    theme: ThemePref,
    size: egui::Vec2,
    ppp: f32,
    timeline: &[u32],
) -> Harness<'static, App> {
    let state = UiState {
        language,
        theme,
        ..UiState::default()
    };
    let mut app = App::new(state, Env::fixed(crate::now()));
    app.open_project(
        crate::filters::project_p(),
        DocOrigin {
            name: "P.xml".to_owned(),
            path: None,
            demo: false,
        },
    );
    let mut harness = harness_sized(app, theme, size, ppp);
    harness.run();
    if !timeline.is_empty() {
        let json = serde_json::json!({
            "format_version": 1,
            "ops": [{
                "op": "style.apply",
                "template": harness.state().state.default_template.key(),
                "patch": { "timeline": { "tasks": timeline } }
            }]
        });
        let batch = planrs_automation::Batch::from_json_str(&json.to_string()).unwrap();
        harness.state_mut().dispatch(Action::Exec(batch));
    }
    harness
        .state_mut()
        .dispatch(Action::Select(Some(planrs_core::TaskUid::new(3))));
    harness.run();
    let app = harness.state();
    assert!(app.document.is_some(), "{:?}", app.messages());
    assert_eq!(app.selection, Some(planrs_core::TaskUid::new(3)));
    harness
}

/// « P » 1 280 × 720, français, clair, frise vide.
fn p_fr_light() -> Harness<'static, App> {
    p_capture(
        LanguagePref::Fr,
        ThemePref::Light,
        egui::vec2(1280.0, 720.0),
        1.0,
        &[],
    )
}

/// Choisit l'onglet `tab` du ruban.
fn select_tab(harness: &mut Harness<'static, App>, tab: RibbonTab) {
    harness
        .state_mut()
        .dispatch(Action::Ribbon(RibbonOp::Select(tab)));
    harness.run();
    assert_eq!(harness.state().state.ribbon.tab, tab);
    assert!(!drawn(&harness.ctx).items.is_empty(), "ruban vide sur {tab:?}");
}

macro_rules! ribbon_tab_snapshots {
    ($($name:ident: $tab:ident, $file:literal;)*) => {
        $(
            #[test]
            fn $name() {
                let mut harness = p_fr_light();
                select_tab(&mut harness, RibbonTab::$tab);
                snapshot(&mut harness, $file);
            }
        )*
    };
}

ribbon_tab_snapshots! {
    snapshot_ribbon_task_fr_light: Task, "ribbon_task_fr_light";
    snapshot_ribbon_resource_fr_light: Resource, "ribbon_resource_fr_light";
    snapshot_ribbon_project_fr_light: Project, "ribbon_project_fr_light";
    snapshot_ribbon_view_fr_light: View, "ribbon_view_fr_light";
    snapshot_ribbon_format_fr_light: Format, "ribbon_format_fr_light";
}

#[test]
fn snapshot_ribbon_task_en_dark() {
    let mut harness = p_capture(
        LanguagePref::En,
        ThemePref::Dark,
        egui::vec2(1280.0, 720.0),
        1.0,
        &[],
    );
    assert_eq!(harness.ctx.theme(), egui::Theme::Dark);
    assert_eq!(harness.state().texts().language(), Language::En);
    select_tab(&mut harness, RibbonTab::Task);
    snapshot(&mut harness, "ribbon_task_en_dark");
}

/// H-UI-72 : replié, seule la bande d'onglets reste.
#[test]
fn snapshot_ribbon_collapsed_fr_light() {
    let mut harness = p_fr_light();
    harness
        .state_mut()
        .dispatch(Action::Ribbon(RibbonOp::ToggleCollapsed));
    harness.run();
    assert!(harness.state().state.ribbon.collapsed);
    assert!(drawn(&harness.ctx).body.is_none());
    snapshot(&mut harness, "ribbon_collapsed_fr_light");
}

/// H-UI-72 : replié, onglet Projet ouvert en surimpression sur la vue.
#[test]
fn snapshot_ribbon_overlay_fr_light() {
    let mut harness = p_fr_light();
    harness
        .state_mut()
        .dispatch(Action::Ribbon(RibbonOp::ToggleCollapsed));
    harness
        .state_mut()
        .dispatch(Action::Ribbon(RibbonOp::OpenOverlay(RibbonTab::Project)));
    harness.run();
    assert_eq!(harness.state().view().ribbon_overlay, Some(RibbonTab::Project));
    assert!(drawn(&harness.ctx).body.is_some());
    // Capture sans pointeur : la surimpression reste ouverte (seuls une
    // action, Échap ou un clic la referment).
    harness.remove_cursor();
    harness.run();
    assert_eq!(harness.state().view().ribbon_overlay, Some(RibbonTab::Project));
    snapshot(&mut harness, "ribbon_overlay_fr_light");
}

/// H-UI-75 : 800 px, groupes réduits de droite à gauche.
#[test]
fn snapshot_ribbon_800px_fr_light() {
    let mut harness = p_capture(
        LanguagePref::Fr,
        ThemePref::Light,
        egui::vec2(800.0, 600.0),
        1.0,
        &[],
    );
    select_tab(&mut harness, RibbonTab::Task);
    let d = drawn(&harness.ctx);
    assert!(
        d.items.iter().all(|i| d.panel.contains_rect(i.rect)),
        "débordement à 800 px"
    );
    snapshot(&mut harness, "ribbon_800px_fr_light");
}

/// H-UI-77 : icônes vectorielles nettes à 2 pixels par point.
#[test]
fn snapshot_ribbon_hidpi_fr_light() {
    let mut harness = p_capture(
        LanguagePref::Fr,
        ThemePref::Light,
        egui::vec2(1024.0, 600.0),
        2.0,
        &[],
    );
    assert!((harness.ctx.pixels_per_point() - 2.0).abs() < f32::EPSILON);
    select_tab(&mut harness, RibbonTab::Task);
    snapshot(&mut harness, "ribbon_hidpi_fr_light");
}

/// Tâches de la frise des captures (cahier du ruban, lot L5).
const TIMELINE: [u32; 5] = [2, 3, 4, 6, 7];

/// H-UI-78, H-UI-80 : frise de « P » avec 2, 3, 4, 6 et 7.
fn timeline_capture(language: LanguagePref, theme: ThemePref, name: &str) {
    let mut harness = p_capture(language, theme, egui::vec2(1280.0, 720.0), 1.0, &TIMELINE);
    let app = harness.state();
    let doc = app.document.as_ref().unwrap();
    assert_eq!(
        app.state.effective_style(doc.project()).timeline.tasks,
        TIMELINE.to_vec()
    );
    let drawn = app.view().timeline.drawn.clone().expect("frise dessinée");
    assert_eq!(drawn.elements.len(), TIMELINE.len(), "{:?}", drawn.elements);
    snapshot(&mut harness, name);
}

#[test]
fn snapshot_timeline_fr_light() {
    timeline_capture(LanguagePref::Fr, ThemePref::Light, "timeline_fr_light");
}

#[test]
fn snapshot_timeline_en_dark() {
    timeline_capture(LanguagePref::En, ThemePref::Dark, "timeline_en_dark");
}

/// H-UI-82 : mode impression, frise affichée ⇒ case cochée d'office, frise
/// en tête de la page 1 de l'aperçu.
#[test]
fn snapshot_print_timeline_fr_light() {
    use planrs_ui_egui::action::DialogKind;
    use planrs_ui_egui::dialog::DialogBody;
    let mut harness = p_capture(
        LanguagePref::Fr,
        ThemePref::Light,
        egui::vec2(1280.0, 720.0),
        1.0,
        &TIMELINE,
    );
    harness
        .state_mut()
        .dispatch(Action::ShowDialog(DialogKind::PageSetup { print_mode: true }));
    harness.run();
    let Some(DialogBody::PageSetup(state)) = harness.state().view().dialog.as_ref().map(|d| &d.body) else {
        panic!("mode impression non ouvert");
    };
    assert!(state.page.timeline, "frise cochée d'office");
    snapshot(&mut harness, "print_timeline_fr_light");
}
