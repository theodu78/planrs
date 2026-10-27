//! Sélection au clavier (passe UX, ADR-0018) : raccourcis de H-UI-92,
//! H-UI-96, H-UI-97 (contrat L0 : entrées de `KEYMAP` vérifiées par
//! `check_keymap_entry`, appelé par `session.rs`) ; le lot L1 y ajoute les
//! tests `h_ui_92_*`, `h_ui_93_*`, `h_ui_101_*`.
//!
//! En L0, Ctrl/Cmd+A sélectionne tout en lignes entières (servi par le
//! modèle) ; Maj+Espace, Ctrl+Espace et Maj+Retour arrière sont lus mais
//! sans effet (lot L1) ; Ctrl/Cmd+D et Ctrl/Cmd+Suppr affichent « pas
//! encore disponible » sans rien écrire (lot L2).

use egui_kittest::Harness;
use planrs_ui_egui::i18n::{Language, TextId, Texts};
use planrs_ui_egui::keymap::{Binding, KEYMAP, KeyContext, UiBinding};
use planrs_ui_egui::selection::SelectionKind;
use planrs_ui_egui::view::Pane;
use planrs_ui_egui::view::table::table_id;
use planrs_ui_egui::{Action, App};

use crate::{harness, open_demo};

/// Exemple ouvert, ligne `i` sélectionnée, table focalisée.
fn demo_on_row(i: usize) -> Harness<'static, App> {
    let mut harness = harness();
    open_demo(&mut harness);
    let uid = harness.state().rows().unwrap().row(i).unwrap().uid;
    harness.state_mut().dispatch(Action::Select(Some(uid)));
    harness.state_mut().dispatch(Action::FocusPane(Pane::Table));
    harness.run();
    assert!(harness.ctx.memory(|memory| memory.has_focus(table_id())));
    harness
}

/// Vrai si l'entrée de [`KEYMAP`] est vérifiée par [`check_keymap_entry`].
pub fn is_ux_entry(binding: Binding) -> bool {
    matches!(
        binding,
        Binding::Ui(
            UiBinding::SelectAll
                | UiBinding::SelectRows
                | UiBinding::SelectColumns
                | UiBinding::CollapseSelection
                | UiBinding::FillDown
                | UiBinding::ClearCells
        )
    )
}

/// Vérifie l'entrée `i` de [`KEYMAP`] de la passe UX (contexte `Table`) :
/// son effet en L0, et rien sans le focus de la table.
pub fn check_keymap_entry(i: usize) {
    let (context, shortcut, binding) = KEYMAP[i];
    assert_eq!(context, KeyContext::Table, "{shortcut:?}");
    let mut harness = demo_on_row(3);
    let before = (harness.state().selected().clone(), harness.state().view().cursor);
    harness.key_press_modifiers(shortcut.modifiers, shortcut.logical_key);
    harness.run();
    let app = harness.state();
    assert!(app.view().edit.editing.is_none(), "{shortcut:?}");
    assert_eq!(app.document.as_ref().unwrap().generation(), 0, "{shortcut:?}");
    match binding {
        Binding::Ui(UiBinding::SelectAll) => {
            let rows = app.rows().unwrap().len();
            assert_eq!(app.selected().len(), rows, "{shortcut:?}");
            assert_eq!(app.selected().kind(), SelectionKind::Rows);
            assert_eq!(app.selection, Some(app.rows().unwrap().row(0).unwrap().uid));
        }
        Binding::Ui(UiBinding::SelectRows | UiBinding::SelectColumns | UiBinding::CollapseSelection) => {
            assert_eq!(
                (app.selected().clone(), app.view().cursor),
                before,
                "{shortcut:?}"
            );
        }
        Binding::Ui(UiBinding::FillDown | UiBinding::ClearCells) => {
            let text = Texts::new(Language::Fr).get(TextId::UxNotAvailable);
            assert!(
                app.messages().iter().any(|m| m.text == text),
                "{shortcut:?} : {:?}",
                app.messages()
            );
        }
        other => panic!("entrée {i} hors de la passe UX : {other:?}"),
    }
    // Sans le focus de la table : rien.
    let mut harness = demo_on_row(3);
    harness.state_mut().dispatch(Action::FocusPane(Pane::Gantt));
    harness.run();
    let before = (
        harness.state().selected().clone(),
        harness.state().messages().len(),
    );
    harness.key_press_modifiers(shortcut.modifiers, shortcut.logical_key);
    harness.run();
    let app = harness.state();
    assert_eq!(
        (app.selected().clone(), app.messages().len()),
        before,
        "{shortcut:?} sans focus"
    );
}
