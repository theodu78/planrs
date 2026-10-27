//! Ruban (ADR-0017, H-UI-69 à H-UI-77) : module posé par le contrat (lot
//! L0), rempli par le lot L2 (moteur, contenu, menus jumeaux).
//!
//! L0 : commutateur d'habillage (aucun changement visible par défaut) et
//! Ctrl/Cmd+F1.

use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable as _;
use planrs_ui_egui::i18n::TextId;
use planrs_ui_egui::ribbon::{Chrome, RibbonOp, RibbonTab};
use planrs_ui_egui::{Action, App};

use crate::{fr, harness, harness_for, new_app};

/// ADR-0017 D9 : barre d'outils de J8 par défaut ; le ruban la remplace
/// seulement sur demande (`set_chrome`), barre de menus gardée.
#[test]
fn ribbon_l0_chrome_switch() {
    let mut harness = harness();
    assert_eq!(harness.state().chrome(), Chrome::Toolbar);
    harness.run();
    assert!(harness.query_by_label(&fr(TextId::ToolbarOpen)).is_some());
    let mut app: App = new_app();
    app.set_chrome(Chrome::Ribbon);
    let mut harness = harness_for(app);
    harness.run();
    assert_eq!(harness.state().chrome(), Chrome::Ribbon);
    assert!(harness.query_by_label(&fr(TextId::ToolbarOpen)).is_none());
    assert!(harness.query_by_label(&fr(TextId::MenuFile)).is_some());
}

/// H-UI-72 (touche, lot L0) : Ctrl+F1 replie puis déplie le ruban, accueil
/// compris ; l'état est celui des préférences persistées.
#[test]
fn ribbon_l0_ctrl_f1_toggles_collapsed() {
    let mut harness = harness();
    harness.run();
    assert!(!harness.state().state.ribbon.collapsed);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F1);
    harness.run();
    assert!(harness.state().state.ribbon.collapsed);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F1);
    harness.run();
    assert!(!harness.state().state.ribbon.collapsed);
}

/// H-UI-69, H-UI-72 : onglet retenu dans les préférences ; surimpression
/// ouverte seulement ruban replié.
#[test]
fn ribbon_l0_reducer_arms() {
    let mut harness = harness();
    harness
        .state_mut()
        .dispatch(Action::Ribbon(RibbonOp::Select(RibbonTab::Format)));
    harness.run();
    assert_eq!(harness.state().state.ribbon.tab, RibbonTab::Format);
    harness
        .state_mut()
        .dispatch(Action::Ribbon(RibbonOp::ToggleCollapsed));
    harness
        .state_mut()
        .dispatch(Action::Ribbon(RibbonOp::OpenOverlay(RibbonTab::Project)));
    harness.run();
    assert_eq!(harness.state().view().ribbon_overlay, Some(RibbonTab::Project));
    harness
        .state_mut()
        .dispatch(Action::Ribbon(RibbonOp::CloseOverlay));
    harness.run();
    assert_eq!(harness.state().view().ribbon_overlay, None);
}
