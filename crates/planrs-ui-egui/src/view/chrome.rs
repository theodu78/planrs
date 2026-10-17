//! Habillage : barre de menus, barre d'outils, écran d'accueil, barre d'état
//! et bandeau de messages.
//!
//! `menu_bar`, `toolbar` et `status_bar` reçoivent l'`Ui` racine et posent
//! leur propre panneau (`egui::Panel`) ; `welcome` dessine dans le panneau
//! central. Aucune chaîne en dur : tout passe par `TextId`.

use crate::action::Action;
use crate::app::UserMessage;
use crate::i18n::{Arg, TextId};
use crate::state::{LanguagePref, ThemePref};

use super::FrameCtx;

/// Choix de langue proposés, avec leur libellé.
const LANGUAGE_CHOICES: [(LanguagePref, TextId); 3] = [
    (LanguagePref::System, TextId::LangSystem),
    (LanguagePref::Fr, TextId::LangFr),
    (LanguagePref::En, TextId::LangEn),
];

/// Choix de thème proposés, avec leur libellé.
const THEME_CHOICES: [(ThemePref, TextId); 3] = [
    (ThemePref::System, TextId::ThemeSystem),
    (ThemePref::Light, TextId::ThemeLight),
    (ThemePref::Dark, TextId::ThemeDark),
];

/// Barre de menus (Fichier, Affichage).
///
/// Corps provisoire : rien n'est affiché.
pub fn menu_bar(_ui: &mut egui::Ui, _fc: &mut FrameCtx<'_>) {}

/// Barre d'outils : exemple, ouvrir, zoom, ajuster, thème, langue.
///
/// Corps provisoire : rien n'est affiché.
pub fn toolbar(_ui: &mut egui::Ui, _fc: &mut FrameCtx<'_>) {}

/// Écran d'accueil (aucun document ouvert) : titre, boutons d'ouverture,
/// choix de la langue et du thème.
pub fn welcome(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let texts = fc.texts;
    ui.vertical_centered(|ui| {
        ui.add_space(fc.metrics.header_h);
        ui.heading(texts.get_args(
            TextId::WelcomeTitle,
            &[("product", Arg::Str(planrs_core::PRODUCT_NAME.into()))],
        ));
        ui.label(texts.get(TextId::WelcomeSubtitle));
        ui.add_space(fc.metrics.row_h);
        if ui.button(texts.get(TextId::WelcomeOpenDemo)).clicked() {
            fc.actions.push(Action::OpenDemo);
        }
        if ui.button(texts.get(TextId::WelcomeOpenFile)).clicked() {
            fc.actions.push(Action::OpenDialog);
        }
        #[cfg(target_arch = "wasm32")]
        ui.label(texts.get(TextId::WelcomeWebNote));
        ui.add_space(fc.metrics.row_h);
        ui.label(texts.get(TextId::WelcomeLanguage));
        ui.horizontal_wrapped(|ui| {
            for (pref, label) in LANGUAGE_CHOICES {
                if ui
                    .selectable_label(fc.state.language == pref, texts.get(label))
                    .clicked()
                {
                    fc.actions.push(Action::SetLanguage(pref));
                }
            }
        });
        ui.label(texts.get(TextId::WelcomeTheme));
        ui.horizontal_wrapped(|ui| {
            for (pref, label) in THEME_CHOICES {
                if ui
                    .selectable_label(fc.state.theme == pref, texts.get(label))
                    .clicked()
                {
                    fc.actions.push(Action::SetTheme(pref));
                }
            }
        });
    });
}

/// Barre d'état (nom, nombre de tâches, plage, diagnostics) et bandeau des
/// messages.
///
/// Corps provisoire : rien n'est affiché.
pub fn status_bar(_ui: &mut egui::Ui, _fc: &mut FrameCtx<'_>, _messages: &[UserMessage]) {}
