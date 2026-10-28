//! Quitter et fermer (seconde passe UX, ADR-0019 D4, H-UI-108, H-UI-109).
//!
//! Contrat posé par le lot L0 : [`App::global_keys`] est appelée en tête de
//! la lecture des raccourcis de chaque image ; en L0 elle ne prend aucune
//! touche (Ctrl/Cmd+Q reste lu par `KEYMAP` hors saisie et hors fenêtre,
//! comme avant la passe, et `egui::Options::quit_shortcuts` est gardé). Le
//! lot L3 y lit Ctrl/Cmd+Q, Ctrl/Cmd+W, Ctrl+F4 et Ctrl/Cmd+P dans **tous**
//! les états (saisie validée d'abord, fenêtre modale annulée) et coupe le
//! raccourci de sortie intégré d'egui. [`App::shell_quit`] est la demande
//! du menu natif macOS (« Quitter planrs », lot L3) : même garde que
//! Ctrl/Cmd+Q.

use crate::action::Action;
use crate::app::App;
use crate::i18n::TextId;

impl App {
    /// Touches globales de sortie et de fermeture, lues avant tout autre
    /// raccourci ; vrai si une touche a été prise (l'image ne lit alors
    /// aucun autre raccourci). L0 : aucune (voir le module).
    #[allow(
        clippy::unused_self,
        reason = "contrat L0 : signature servie par le lot L3 (saisie, fenêtres, garde)"
    )]
    pub(crate) fn global_keys(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) -> bool {
        let _ = (ctx, actions);
        false
    }

    /// « Quitter » du menu applicatif natif (macOS, lot L3) : même chemin
    /// que Ctrl/Cmd+Q (`Action::Quit`, garde des modifications), appliqué
    /// à la fin de la prochaine image.
    pub fn shell_quit(&mut self) {
        self.actions.push(Action::Quit);
    }
}

/// Textes posés par le lot L0 pour le menu natif macOS et « Fermer le
/// document », servis par les lots L3 et L10.
pub const L3_TEXTS: [TextId; 9] = [
    TextId::MacMenuAbout,
    TextId::MacMenuHide,
    TextId::MacMenuHideOthers,
    TextId::MacMenuShowAll,
    TextId::MacMenuQuit,
    TextId::MacMenuWindow,
    TextId::MacMenuMinimize,
    TextId::MacMenuZoom,
    TextId::MenuCloseDocument,
];
