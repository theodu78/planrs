//! Habillage : barre de menus, barre d'outils, écran d'accueil, barre d'état
//! et bandeau de messages.
//!
//! `menu_bar`, `toolbar` et `status_bar` reçoivent l'`Ui` racine et posent
//! leur propre panneau (`egui::Panel`) ; `welcome` dessine dans le panneau
//! central. Aucune chaîne en dur : tout passe par `TextId`.

use planrs_core::Severity;

use crate::action::Action;
use crate::app::UserMessage;
use crate::format::format_date;
use crate::i18n::{Arg, Language, TextId};
use crate::state::{LanguagePref, ThemePref};
use crate::timescale::TimeScaleZoom;

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
pub fn menu_bar(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let texts = fc.texts;
    let has_doc = fc.doc.is_some();
    egui::Panel::top("planrs_menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(texts.get(TextId::MenuFile), |ui| {
                if ui.button(texts.get(TextId::MenuOpenDemo)).clicked() {
                    fc.actions.push(Action::OpenDemo);
                }
                if ui.button(texts.get(TextId::MenuOpen)).clicked() {
                    fc.actions.push(Action::OpenDialog);
                }
                ui.separator();
                if ui
                    .add_enabled(has_doc, egui::Button::new(texts.get(TextId::MenuClose)))
                    .clicked()
                {
                    fc.actions.push(Action::CloseDocument);
                }
            });
            ui.menu_button(texts.get(TextId::MenuView), |ui| {
                ui.menu_button(texts.get(TextId::MenuZoom), |ui| {
                    ui.add_enabled_ui(has_doc, |ui| {
                        zoom_choices(ui, fc);
                        ui.separator();
                        if ui.button(texts.get(TextId::ToolbarFit)).clicked() {
                            fc.actions.push(Action::FitZoom);
                        }
                    });
                });
                ui.menu_button(texts.get(TextId::MenuTheme), |ui| theme_choices(ui, fc));
                ui.menu_button(texts.get(TextId::MenuLanguage), |ui| language_choices(ui, fc));
            });
        });
    });
}

/// Barre d'outils : exemple, ouvrir, zoom, ajuster, thème, langue. Toujours
/// visible (le zoom est grisé sans document) : le zoom reste découvrable
/// sans connaître Ctrl+molette.
pub fn toolbar(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let texts = fc.texts;
    let has_doc = fc.doc.is_some();
    egui::Panel::top("planrs_toolbar").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if ui.button(texts.get(TextId::MenuOpenDemo)).clicked() {
                fc.actions.push(Action::OpenDemo);
            }
            if ui.button(texts.get(TextId::MenuOpen)).clicked() {
                fc.actions.push(Action::OpenDialog);
            }
            ui.separator();
            ui.add_enabled_ui(has_doc, |ui| {
                zoom_choices(ui, fc);
                if ui.button(texts.get(TextId::ToolbarFit)).clicked() {
                    fc.actions.push(Action::FitZoom);
                }
            });
            ui.separator();
            theme_choices(ui, fc);
            ui.separator();
            for (pref, label) in LANGUAGE_CHOICES {
                // « Système » n'a sa place que dans le menu et l'accueil :
                // la barre d'outils bascule directement entre FR et EN.
                // Sélectionné : la langue effective (aussi quand elle vient
                // du système).
                let selected = texts.language() == Language::resolve(pref, None);
                if pref != LanguagePref::System && choice(ui, selected, &texts.get(label)) {
                    fc.actions.push(Action::SetLanguage(pref));
                }
            }
        });
    });
}

/// Choix exclusif : une étiquette sélectionnable ; rend `true` au clic. Le
/// nom accessible est le libellé traduit, avec l'état sélectionné.
fn choice(ui: &mut egui::Ui, selected: bool, text: &str) -> bool {
    let enabled = ui.is_enabled();
    let response = ui.selectable_label(selected, text);
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, enabled, selected, text));
    response.clicked()
}

/// Zoom exclusif Jour / Semaine / Mois / Trimestre.
fn zoom_choices(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    for zoom in TimeScaleZoom::ALL {
        if choice(ui, fc.state.zoom == zoom, &fc.texts.get(zoom.label())) {
            fc.actions.push(Action::SetZoom(zoom));
        }
    }
}

/// Thème exclusif Système / Clair / Sombre.
fn theme_choices(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    for (pref, label) in THEME_CHOICES {
        if choice(ui, fc.state.theme == pref, &fc.texts.get(label)) {
            fc.actions.push(Action::SetTheme(pref));
        }
    }
}

/// Langue exclusive Système / Français / English.
fn language_choices(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    for (pref, label) in LANGUAGE_CHOICES {
        if choice(ui, fc.state.language == pref, &fc.texts.get(label)) {
            fc.actions.push(Action::SetLanguage(pref));
        }
    }
}

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
        let big = egui::vec2(fc.metrics.header_h * 8.0, fc.metrics.header_h);
        let demo = ui.add(egui::Button::new(texts.get(TextId::WelcomeOpenDemo)).min_size(big));
        // Focus par défaut sur « Ouvrir l'exemple » (Entrée l'active), sans
        // le voler à un autre widget déjà focalisé.
        if ui.memory(|memory| memory.focused().is_none()) {
            demo.request_focus();
        }
        if demo.clicked() {
            fc.actions.push(Action::OpenDemo);
        }
        if ui
            .add(egui::Button::new(texts.get(TextId::WelcomeOpenFile)).min_size(big))
            .clicked()
        {
            fc.actions.push(Action::OpenDialog);
        }
        if cfg!(target_arch = "wasm32") {
            ui.label(texts.get(TextId::WelcomeWebNote));
        }
        ui.add_space(fc.metrics.row_h);
        ui.label(texts.get(TextId::WelcomeLanguage));
        let labels = LANGUAGE_CHOICES.map(|(_, label)| texts.get(label));
        centered_row(ui, &labels, |ui| language_choices(ui, fc));
        ui.label(texts.get(TextId::WelcomeTheme));
        let labels = THEME_CHOICES.map(|(_, label)| texts.get(label));
        centered_row(ui, &labels, |ui| theme_choices(ui, fc));
    });
}

/// Rangée de choix centrée : egui ne connaît la largeur d'une rangée
/// horizontale qu'après l'avoir dessinée ; on la mesure donc d'avance
/// (galleys des libellés, marges des boutons, espacement).
fn centered_row(ui: &mut egui::Ui, labels: &[String], add_contents: impl FnOnce(&mut egui::Ui)) {
    let spacing = ui.spacing().item_spacing.x;
    let padding = ui.spacing().button_padding.x * 2.0;
    let mut width = 0.0_f32;
    for label in labels {
        let galley = egui::WidgetText::from(label.as_str()).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        );
        width += galley.size().x + padding + spacing;
    }
    ui.horizontal(|ui| {
        let pad = f32::midpoint(ui.available_width() - width, spacing).max(0.0);
        if pad.is_finite() {
            ui.add_space(pad);
        }
        add_contents(ui);
    });
}

/// Nombre de diagnostics (import et calcul) par gravité : (erreurs,
/// avertissements).
fn diagnostic_counts(fc: &FrameCtx<'_>) -> (i64, i64) {
    let Some(doc) = fc.doc else {
        return (0, 0);
    };
    let severities = doc
        .import_diagnostics
        .iter()
        .map(|diagnostic| diagnostic.severity)
        .chain(
            doc.schedule
                .diagnostics
                .iter()
                .map(planrs_core::Diagnostic::severity),
        );
    let (mut errors, mut warnings) = (0_i64, 0_i64);
    for severity in severities {
        match severity {
            Severity::Error => errors = errors.saturating_add(1),
            Severity::Warning => warnings = warnings.saturating_add(1),
            Severity::Info => {}
        }
    }
    (errors, warnings)
}

/// Barre d'état (nom, nombre de tâches, plage, diagnostics) et bandeau des
/// messages (sous la barre d'outils).
pub fn status_bar(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, messages: &[UserMessage]) {
    let texts = fc.texts;
    egui::Panel::bottom("planrs_status_bar").show(ui, |ui| {
        ui.horizontal(|ui| {
            let Some(doc) = fc.doc else {
                ui.label(texts.get(TextId::StatusNoDocument));
                return;
            };
            ui.strong(doc.origin.name.as_str());
            ui.separator();
            let count = fc
                .rows
                .map(crate::rows::RowModel::len)
                .filter(|&len| len > 0)
                .unwrap_or_else(|| doc.project.tasks.len());
            let count = i64::try_from(count).unwrap_or(i64::MAX);
            ui.label(texts.get_args(TextId::StatusTasks, &[("count", Arg::Count(count))]));
            ui.separator();
            let labels = texts.format_labels();
            ui.label(texts.get_args(
                TextId::StatusRange,
                &[
                    (
                        "start",
                        Arg::Str(format_date(doc.schedule.project_start, &labels).into()),
                    ),
                    (
                        "finish",
                        Arg::Str(format_date(doc.schedule.project_finish, &labels).into()),
                    ),
                ],
            ));
            ui.separator();
            // Cliquer sur un compteur ouvre ou referme le panneau des
            // diagnostics.
            let (errors, warnings) = diagnostic_counts(fc);
            let counters = [
                (TextId::StatusErrors, errors, ui.visuals().error_fg_color),
                (TextId::StatusWarnings, warnings, ui.visuals().warn_fg_color),
            ];
            for (id, count, color) in counters {
                let mut text = egui::RichText::new(texts.get_args(id, &[("count", Arg::Count(count))]));
                if count > 0 {
                    text = text.color(color);
                }
                if ui.selectable_label(fc.state.diagnostics_open, text).clicked() {
                    fc.actions.push(Action::ToggleDiagnostics);
                }
            }
        });
    });
    message_banner(ui, fc, messages);
}

/// Bandeau des messages (erreur en rouge), refermable ; jamais de modale.
fn message_banner(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, messages: &[UserMessage]) {
    if messages.is_empty() {
        return;
    }
    let texts = fc.texts;
    let has_error = messages.iter().any(|message| message.severity == Severity::Error);
    let visuals = ui.visuals();
    let (fill, fg) = if has_error {
        (
            visuals.error_fg_color.gamma_multiply(0.18),
            visuals.error_fg_color,
        )
    } else {
        (visuals.warn_fg_color.gamma_multiply(0.18), visuals.text_color())
    };
    let frame = egui::Frame::side_top_panel(ui.style()).fill(fill);
    egui::Panel::top("planrs_message_banner")
        .frame(frame)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    if has_error {
                        ui.label(
                            egui::RichText::new(texts.get(TextId::OpenErrorTitle))
                                .strong()
                                .color(fg),
                        );
                    }
                    for message in messages {
                        ui.label(egui::RichText::new(message.text.as_str()).color(fg));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(texts.get(TextId::MenuClose)).clicked() {
                        fc.actions.push(Action::DismissMessage);
                    }
                });
            });
        });
}
