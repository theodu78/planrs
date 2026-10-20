//! Raccourcis clavier : table [`KEYMAP`], vérifiée contre le registre des
//! commandes (ADR-0012 D11).
//!
//! Une entrée associe un contexte, une combinaison de touches et une
//! liaison : une commande du registre ([`Binding::Command`], id à point
//! comme `history.undo`) ou une action propre à l'interface
//! ([`Binding::Ui`]). Les tests vérifient que chaque id existe dans
//! `planrs_automation::REGISTRY` et que chaque entrée déclenche son action.
//!
//! Ordre : egui compare les modificateurs « logiquement » (Maj et Alt en
//! trop ignorés) ; la combinaison la plus précise d'une touche vient donc
//! avant la moins précise (Ctrl+Maj+Z avant Ctrl+Z), ce qu'un test vérifie.
//! Aucun raccourci n'est lu pendant la saisie dans un champ de texte : ses
//! propres Ctrl+Z et Ctrl+Y lui appartiennent.

use egui::{Key, KeyboardShortcut, Modifiers};

use crate::action::Action;
use crate::nav::NavMove;
use crate::structure::StructureOp;

/// Combinaison de touches (`Modifiers::COMMAND` : Ctrl, ou Cmd sur macOS).
pub type Shortcut = KeyboardShortcut;

/// Contexte où un raccourci est actif.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyContext {
    /// Partout (accueil compris), hors saisie de texte.
    Global,
    /// Un document est ouvert, hors saisie de texte.
    Document,
    /// La table a le focus, sans saisie en cours, sans menu ni fenêtre
    /// ouverts (J7, ADR-0015 D2 ; actif à partir du lot L1).
    Table,
}

/// Action propre à l'interface (sans commande du registre).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiBinding {
    /// Ouvre le sélecteur de fichiers.
    OpenFile,
    /// Enregistre sous un autre nom (`project.save` avec un chemin choisi
    /// dans la boîte de dialogue).
    SaveAs,
    /// Quitte l'application (desktop), par le bandeau de confirmation si le
    /// document est modifié. Sur macOS, Cmd+Q n'arrive ici que parce que
    /// l'application retire le menu par défaut de winit (`apps/planrs`).
    Quit,
    /// Déplacement dans la grille (J7, H-UI-13 à H-UI-16) ; `extend` :
    /// étend la sélection de lignes.
    Nav {
        /// Déplacement.
        mv: NavMove,
        /// Extension de la sélection (Maj).
        extend: bool,
    },
    /// « Informations sur la tâche » (Maj+F2, H-UI-19).
    TaskInformation,
    /// Une graduation fine de l'échelle de temps (Alt+←/→, H-UI-18).
    TimescaleStep(i8),
    /// Une largeur visible (Alt+Page haut/bas, H-UI-18).
    TimescalePage(i8),
    /// Début du projet (Alt+Origine, H-UI-18).
    TimescaleToStart,
    /// Fin du projet (Alt+Fin, H-UI-18).
    TimescaleToEnd,
    /// Unités de temps plus fines (Ctrl+/, Ctrl+=, H-UI-18).
    ZoomFiner,
    /// Unités de temps plus grossières (Ctrl+-, H-UI-18).
    ZoomCoarser,
}

/// Liaison d'un raccourci.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// Commande du registre, par son id (`history.undo`).
    Command(&'static str),
    /// Action de l'interface.
    Ui(UiBinding),
}

/// Ctrl/Cmd + Maj.
const COMMAND_SHIFT: Modifiers = Modifiers {
    shift: true,
    ..Modifiers::COMMAND
};

/// Alt + Maj.
const ALT_SHIFT: Modifiers = Modifiers {
    alt: true,
    shift: true,
    ..Modifiers::NONE
};

/// Table des raccourcis, la combinaison la plus précise d'une touche
/// d'abord.
pub const KEYMAP: &[(KeyContext, Shortcut, Binding)] = &[
    (
        KeyContext::Document,
        Shortcut::new(COMMAND_SHIFT, Key::Z),
        Binding::Command("history.redo"),
    ),
    (
        KeyContext::Document,
        Shortcut::new(Modifiers::COMMAND, Key::Z),
        Binding::Command("history.undo"),
    ),
    (
        KeyContext::Document,
        Shortcut::new(Modifiers::COMMAND, Key::Y),
        Binding::Command("history.redo"),
    ),
    (
        KeyContext::Global,
        Shortcut::new(Modifiers::COMMAND, Key::O),
        Binding::Ui(UiBinding::OpenFile),
    ),
    (
        KeyContext::Global,
        Shortcut::new(Modifiers::COMMAND, Key::Q),
        Binding::Ui(UiBinding::Quit),
    ),
    (
        KeyContext::Global,
        Shortcut::new(Modifiers::COMMAND, Key::N),
        Binding::Command("project.new"),
    ),
    (
        KeyContext::Document,
        Shortcut::new(COMMAND_SHIFT, Key::S),
        Binding::Ui(UiBinding::SaveAs),
    ),
    (
        KeyContext::Document,
        Shortcut::new(Modifiers::COMMAND, Key::S),
        Binding::Command("project.save"),
    ),
    // Structure (lot L14), sur la sélection de la vue principale.
    (
        KeyContext::Document,
        Shortcut::new(Modifiers::NONE, Key::Insert),
        Binding::Command("task.add"),
    ),
    (
        KeyContext::Document,
        Shortcut::new(Modifiers::NONE, Key::Delete),
        Binding::Command("task.delete"),
    ),
    // Touche « delete » d'un clavier Mac (Retour arrière) : pas de Suppr
    // sans fn. Après Suppr, qui reste le raccourci affiché.
    (
        KeyContext::Document,
        Shortcut::new(Modifiers::NONE, Key::Backspace),
        Binding::Command("task.delete"),
    ),
    (
        KeyContext::Document,
        Shortcut::new(ALT_SHIFT, Key::ArrowRight),
        Binding::Command("task.indent"),
    ),
    (
        KeyContext::Document,
        Shortcut::new(ALT_SHIFT, Key::ArrowLeft),
        Binding::Command("task.outdent"),
    ),
    (
        KeyContext::Document,
        Shortcut::new(COMMAND_SHIFT, Key::F2),
        Binding::Command("link.remove"),
    ),
    (
        KeyContext::Document,
        Shortcut::new(Modifiers::COMMAND, Key::F2),
        Binding::Command("link.add"),
    ),
];

/// Action de l'interface pour une liaison ; `None` pour un id du registre
/// que l'interface ne sait pas encore exécuter seul (un test garantit
/// qu'aucune entrée de [`KEYMAP`] n'est dans ce cas).
#[must_use]
pub fn binding_action(binding: Binding) -> Option<Action> {
    match binding {
        Binding::Command("history.undo") => Some(Action::Undo),
        Binding::Command("history.redo") => Some(Action::Redo),
        Binding::Command("project.new") => Some(Action::NewProject),
        Binding::Command("project.save") => Some(Action::Save),
        Binding::Command("task.add") => Some(Action::Structure(StructureOp::InsertBefore)),
        Binding::Command("task.delete") => Some(Action::Structure(StructureOp::Delete)),
        Binding::Command("task.indent") => Some(Action::Structure(StructureOp::Indent)),
        Binding::Command("task.outdent") => Some(Action::Structure(StructureOp::Outdent)),
        Binding::Command("link.add") => Some(Action::Structure(StructureOp::Link)),
        Binding::Command("link.remove") => Some(Action::Structure(StructureOp::Unlink)),
        Binding::Command(_) => None,
        Binding::Ui(ui) => Some(ui_action(ui)),
    }
}

/// Action d'une liaison propre à l'interface : toute [`UiBinding`] en a
/// une (`keymap_bindings_resolve`).
#[must_use]
pub const fn ui_action(binding: UiBinding) -> Action {
    match binding {
        UiBinding::OpenFile => Action::OpenDialog,
        UiBinding::SaveAs => Action::SaveAs,
        UiBinding::Quit => Action::Quit,
        UiBinding::Nav { mv, extend } => Action::Nav { mv, extend },
        UiBinding::TaskInformation => Action::ShowTaskInformation,
        UiBinding::TimescaleStep(n) => Action::TimescaleStep(n),
        UiBinding::TimescalePage(n) => Action::TimescalePage(n),
        UiBinding::TimescaleToStart => Action::TimescaleToStart,
        UiBinding::TimescaleToEnd => Action::TimescaleToEnd,
        UiBinding::ZoomFiner => Action::ZoomBy(1),
        UiBinding::ZoomCoarser => Action::ZoomBy(-1),
    }
}

/// Texte d'un raccourci pour les menus (« Ctrl+Z », « ⌘Z »).
#[must_use]
pub fn shortcut_text(ctx: &egui::Context, shortcut: &Shortcut) -> String {
    ctx.format_shortcut(shortcut)
}

/// Premier raccourci de [`KEYMAP`] lié à `binding` (affiché dans les menus).
#[must_use]
pub fn shortcut_for(binding: Binding) -> Option<&'static Shortcut> {
    KEYMAP
        .iter()
        .find(|(_, _, entry)| *entry == binding)
        .map(|(_, shortcut, _)| shortcut)
}

/// Lit les raccourcis de l'image et pousse leurs actions dans `actions`
/// (touches consommées : aucun widget ne les reçoit ensuite). Rien pendant
/// la saisie dans un champ de texte.
pub fn dispatch(ctx: &egui::Context, has_document: bool, actions: &mut Vec<Action>) {
    if ctx.text_edit_focused() {
        return;
    }
    for (context, shortcut, binding) in KEYMAP {
        let active = match context {
            KeyContext::Global => true,
            KeyContext::Document => has_document,
            // Focus de la table : suivi à partir du lot L1 (aucune entrée
            // en J7 L0).
            KeyContext::Table => false,
        };
        if !active {
            continue;
        }
        if ctx.input_mut(|input| input.consume_shortcut(shortcut)) {
            match binding_action(*binding) {
                Some(action) => actions.push(action),
                None => log::warn!("raccourci sans action : {binding:?}"),
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use super::{Binding, KEYMAP, UiBinding, binding_action, ui_action};
    use crate::action::Action;
    use crate::nav::NavMove;

    /// Modificateurs de `a` contenant ceux de `b` (au moins autant de
    /// touches enfoncées).
    fn covers(a: egui::Modifiers, b: egui::Modifiers) -> bool {
        (a.alt || !b.alt)
            && (a.ctrl || !b.ctrl)
            && (a.shift || !b.shift)
            && (a.mac_cmd || !b.mac_cmd)
            && (a.command || !b.command)
    }

    #[test]
    fn keymap_most_specific_first_and_unique() {
        for (i, (_, earlier, _)) in KEYMAP.iter().enumerate() {
            for (_, later, _) in &KEYMAP[i + 1..] {
                assert_ne!(earlier, later, "raccourci en double : {earlier:?}");
                if earlier.logical_key == later.logical_key {
                    assert!(
                        !covers(later.modifiers, earlier.modifiers) || later.modifiers == earlier.modifiers,
                        "{later:?} doit précéder {earlier:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn keymap_every_binding_has_action() {
        for (_, shortcut, binding) in KEYMAP {
            assert!(binding_action(*binding).is_some(), "{shortcut:?} → {binding:?}");
        }
        assert!(binding_action(Binding::Command("task.move")).is_none());
    }

    /// Contrat J7 (lot L0) : chaque `UiBinding` a son action.
    #[test]
    fn keymap_bindings_resolve() {
        let all = [
            UiBinding::OpenFile,
            UiBinding::SaveAs,
            UiBinding::Quit,
            UiBinding::Nav {
                mv: NavMove::Down,
                extend: true,
            },
            UiBinding::TaskInformation,
            UiBinding::TimescaleStep(-1),
            UiBinding::TimescalePage(1),
            UiBinding::TimescaleToStart,
            UiBinding::TimescaleToEnd,
            UiBinding::ZoomFiner,
            UiBinding::ZoomCoarser,
        ];
        for binding in all {
            assert_eq!(binding_action(Binding::Ui(binding)), Some(ui_action(binding)));
        }
        assert_eq!(
            ui_action(UiBinding::Nav {
                mv: NavMove::PageDown,
                extend: false
            }),
            Action::Nav {
                mv: NavMove::PageDown,
                extend: false
            }
        );
        assert_eq!(ui_action(UiBinding::ZoomFiner), Action::ZoomBy(1));
        assert_eq!(ui_action(UiBinding::ZoomCoarser), Action::ZoomBy(-1));
    }
}
