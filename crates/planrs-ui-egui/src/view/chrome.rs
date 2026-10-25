//! Habillage : barre de menus, barre d'outils, écran d'accueil, barre d'état
//! et bandeau de messages.
//!
//! `menu_bar`, `toolbar` et `status_bar` reçoivent l'`Ui` racine et posent
//! leur propre panneau (`egui::Panel`) ; `welcome` dessine dans le panneau
//! central. Aucune chaîne en dur : tout passe par `TextId`.

use planrs_core::Severity;

use crate::action::Action;
use crate::action::{ConfirmChoice, DialogKind};
use crate::app::{MAX_MESSAGES, UserMessage};
use crate::format::format_date;
use crate::i18n::{Arg, Language, TextId};
use crate::keymap::{Binding, UiBinding, shortcut_for_os, shortcut_text};
use crate::state::{GanttStyle, LanguagePref, ThemePref, ViewKind};
use crate::theme::Palette;
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

/// Styles du Gantt proposés, avec leur libellé (J7, lot L7).
const GANTT_STYLE_CHOICES: [(GanttStyle, TextId); 2] = [
    (GanttStyle::Standard, TextId::MenuGanttStandard),
    (GanttStyle::Tracking, TextId::MenuGanttTracking),
];

/// Barre de menus (Fichier, Édition, Affichage).
pub fn menu_bar(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let texts = fc.texts;
    let has_doc = fc.doc.is_some();
    egui::Panel::top("planrs_menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(texts.get(TextId::MenuFile), |ui| file_menu(ui, fc));
            ui.menu_button(texts.get(TextId::MenuEdit), |ui| edit_menu(ui, fc));
            ui.menu_button(texts.get(TextId::MenuView), |ui| {
                ui.add_enabled_ui(has_doc, |ui| gantt_style_choices(ui, fc));
                ui.separator();
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
                ui.separator();
                // Bascule du panneau des diagnostics : les compteurs de la
                // barre d'état l'ouvrent aussi, mais rien ne les signale comme
                // cliquables. Toujours active : un panneau resté ouvert après
                // « Fermer » doit pouvoir se refermer.
                let mut open = fc.state.diagnostics_open;
                if ui
                    .checkbox(&mut open, texts.get(TextId::MenuDiagnostics))
                    .changed()
                {
                    fc.actions.push(Action::ToggleDiagnostics);
                }
            });
        });
    });
}

/// Menu Fichier : Nouveau, exemple, Ouvrir, Enregistrer, Enregistrer
/// sous, Fermer, Quitter (desktop) ; raccourcis de
/// [`crate::keymap::KEYMAP`] affichés. Enregistrer et Fermer demandent un
/// document.
fn file_menu(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let has_doc = fc.doc.is_some();
    let new = Binding::Command("project.new");
    menu_item(ui, fc, TextId::MenuNew, Some(new), true, Action::NewProject);
    menu_item(ui, fc, TextId::MenuOpenDemo, None, true, Action::OpenDemo);
    let open = Binding::Ui(UiBinding::OpenFile);
    menu_item(ui, fc, TextId::MenuOpen, Some(open), true, Action::OpenDialog);
    // Fichiers récents (H-UI-28) : chemins du desktop seulement.
    if !cfg!(target_arch = "wasm32") {
        let texts = fc.texts;
        ui.menu_button(texts.get(TextId::MenuRecent), |ui| recent_menu(ui, fc));
    }
    ui.separator();
    let save = Binding::Command("project.save");
    menu_item(ui, fc, TextId::MenuSave, Some(save), has_doc, Action::Save);
    let save_as = Binding::Ui(UiBinding::SaveAs);
    menu_item(ui, fc, TextId::MenuSaveAs, Some(save_as), has_doc, Action::SaveAs);
    ui.separator();
    menu_item(ui, fc, TextId::MenuClose, None, has_doc, Action::CloseDocument);
    // Quitter : le navigateur ne laisse pas une page fermer son onglet.
    if !cfg!(target_arch = "wasm32") {
        ui.separator();
        let quit = Binding::Ui(UiBinding::Quit);
        menu_item(ui, fc, TextId::MenuQuit, Some(quit), true, Action::Quit);
    }
}

/// Sous-menu Fichier → Fichiers récents (H-UI-28) : le plus récent en
/// tête, nom affiché et chemin en infobulle ; un fichier introuvable est
/// grisé (« (introuvable) ») mais reste cliquable : la tentative
/// d'ouverture le retire de la liste. L'existence n'est relue que pendant
/// que le sous-menu est ouvert.
fn recent_menu(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let texts = fc.texts;
    if fc.state.recent.is_empty() {
        ui.add_enabled(false, egui::Button::new(texts.get(TextId::MenuRecentEmpty)));
        return;
    }
    for (index, recent) in fc.state.recent.iter().enumerate() {
        let exists = std::path::Path::new(&recent.path).exists();
        let label = if exists {
            egui::RichText::new(recent.name.as_str())
        } else {
            let text = texts.get_args(
                TextId::MenuRecentMissing,
                &[("name", Arg::Str(recent.name.as_str().into()))],
            );
            egui::RichText::new(text).weak()
        };
        if ui
            .add(egui::Button::new(label))
            .on_hover_text(recent.path.as_str())
            .clicked()
        {
            fc.actions.push(Action::OpenRecent(index));
        }
    }
}

/// Sous-menu Édition → Projet (J7, lot L12) : informations sur le projet,
/// temps de travail, ligne de base, replanifier le travail restant ; grisé
/// sans document. Sous-menu plutôt qu'un menu de la barre : la barre de
/// menus figure dans toutes les captures de référence (écart consigné).
fn project_menu(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let has_doc = fc.doc.is_some();
    for (label, kind) in [
        (TextId::MenuProjectInfo, DialogKind::ProjectInformation),
        (TextId::MenuWorkingTime, DialogKind::WorkingTime),
        (TextId::MenuBaseline, DialogKind::Baseline),
        (TextId::MenuReschedule, DialogKind::Reschedule),
    ] {
        menu_item(ui, fc, label, None, has_doc, Action::ShowDialog(kind));
    }
}

/// Entrée de menu : libellé traduit, raccourci éventuel de la liaison,
/// action poussée au clic.
fn menu_item(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    label: TextId,
    binding: Option<Binding>,
    enabled: bool,
    action: Action,
) {
    let mut button = egui::Button::new(fc.texts.get(label));
    if let Some(shortcut) = binding.and_then(|binding| shortcut_for_os(binding, ui.ctx().os())) {
        button = button.shortcut_text(shortcut_text(ui.ctx(), shortcut));
    }
    if ui.add_enabled(enabled, button).clicked() {
        fc.actions.push(action);
    }
}

/// Menu Édition : Annuler, Rétablir, actifs seulement s'il y a un lot à
/// annuler ou à rétablir ; puis les six commandes de structure du menu
/// contextuel (grisées sans document, et sans sélection sauf « Insérer une
/// tâche ») ; raccourcis de [`crate::keymap::KEYMAP`] affichés. Rend la
/// structure découvrable sans clic droit ni touche Inser (relecture finale
/// J4, F5) ; enfin le sous-menu Projet (J7, lot L12).
fn edit_menu(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let texts = fc.texts;
    let history = fc.doc.map(|doc| doc.session.history());
    let can_undo = history.as_ref().is_some_and(|h| !h.entries.is_empty());
    let can_redo = history.as_ref().is_some_and(|h| !h.redo.is_empty());
    let items = [
        (TextId::MenuUndo, "history.undo", can_undo, Action::Undo),
        (TextId::MenuRedo, "history.redo", can_redo, Action::Redo),
    ];
    // Dernier lot annulable venu d'un agent (relais J7) : « Annuler :
    // agent — libellé » (ou le nombre d'opérations).
    let agent_undo = history
        .as_ref()
        .and_then(|h| h.entries.last())
        .filter(|entry| entry.source == planrs_automation::EditSource::Agent)
        .map(|entry| {
            let detail = entry.label.clone().unwrap_or_else(|| {
                let count = i64::try_from(entry.ops).unwrap_or(i64::MAX);
                texts.get_args(TextId::MenuUndoAgentOps, &[("count", Arg::Count(count))])
            });
            texts.get_args(TextId::MenuUndoAgent, &[("detail", Arg::Str(detail.into()))])
        });
    for (label, id, enabled, action) in items {
        let text = match (&agent_undo, label) {
            (Some(text), TextId::MenuUndo) => text.clone(),
            _ => texts.get(label),
        };
        let mut button = egui::Button::new(text);
        if let Some(shortcut) = shortcut_for_os(Binding::Command(id), ui.ctx().os()) {
            button = button.shortcut_text(shortcut_text(ui.ctx(), shortcut));
        }
        if ui.add_enabled(enabled, button).clicked() {
            fc.actions.push(action);
        }
    }
    ui.separator();
    let has_doc = fc.doc.is_some();
    let has_selection = fc.selection.is_some();
    for (op, label, id, needs_selection) in super::main_view::CONTEXT_ENTRIES {
        let mut button = egui::Button::new(texts.get(label));
        if let Some(shortcut) = shortcut_for_os(Binding::Command(id), ui.ctx().os()) {
            button = button.shortcut_text(shortcut_text(ui.ctx(), shortcut));
        }
        let enabled = has_doc && (has_selection || !needs_selection);
        if ui.add_enabled(enabled, button).clicked() {
            fc.actions.push(Action::Structure(op));
        }
    }
    ui.separator();
    // « Affecter des ressources » (J7, lot L13) : aucun raccourci (S61).
    let assign = Action::ShowDialog(DialogKind::AssignResources);
    menu_item(ui, fc, TextId::MenuAssignResources, None, has_doc, assign);
    ui.menu_button(texts.get(TextId::MenuProject), |ui| project_menu(ui, fc));
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
            // Libellé court (le menu et l'accueil portent le libellé long) :
            // la barre d'outils reste sur une ligne, et le nom accessible ne
            // double pas le bouton de l'accueil.
            if ui.button(texts.get(TextId::ToolbarOpen)).clicked() {
                fc.actions.push(Action::OpenDialog);
            }
            ui.separator();
            ui.add_enabled_ui(has_doc, |ui| {
                zoom_choices(ui, fc);
                // « Ajuster » est une action, pas un choix exclusif : séparé
                // des étiquettes de zoom pour ne pas passer pour une bascule
                // enfoncée.
                ui.separator();
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

/// Zoom exclusif Jour / Semaine / Mois / Trimestre ; aucun n'est surligné
/// sans document (l'ouverture choisira le zoom ajusté).
fn zoom_choices(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let has_doc = fc.doc.is_some();
    for zoom in TimeScaleZoom::ALL {
        if choice(ui, has_doc && fc.state.zoom == zoom, &fc.texts.get(zoom.label())) {
            fc.actions.push(Action::SetZoom(zoom));
        }
    }
}

/// Vue exclusive : Diagramme de Gantt / Gantt de suivi (J7, lot L7,
/// H-UI-22 ; style du Gantt, qui ramène à la vue Gantt) / Feuille des
/// ressources (lot L10) / Utilisation des ressources (lot L11) ; aucune
/// n'est surlignée sans document.
fn gantt_style_choices(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let has_doc = fc.doc.is_some();
    let gantt = fc.state.view == ViewKind::Gantt;
    for (style, label) in GANTT_STYLE_CHOICES {
        if choice(
            ui,
            has_doc && gantt && fc.state.gantt_style == style,
            &fc.texts.get(label),
        ) {
            fc.actions.push(Action::SetView(ViewKind::Gantt));
            fc.actions.push(Action::SetGanttStyle(style));
        }
    }
    let sheet = fc.state.view == ViewKind::ResourceSheet;
    if choice(ui, has_doc && sheet, &fc.texts.get(TextId::MenuViewResourceSheet)) {
        fc.actions.push(Action::SetView(ViewKind::ResourceSheet));
    }
    let usage = fc.state.view == ViewKind::ResourceUsage;
    if choice(ui, has_doc && usage, &fc.texts.get(TextId::MenuViewResourceUsage)) {
        fc.actions.push(Action::SetView(ViewKind::ResourceUsage));
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

/// Écran d'accueil (aucun document ouvert) : titre, boutons d'ouverture et
/// de création,
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
        // « Nouveau projet » : sur le web, Ctrl+N appartient au navigateur
        // (relecture finale J4, F5).
        if ui
            .add(egui::Button::new(texts.get(TextId::WelcomeNewProject)).min_size(big))
            .clicked()
        {
            fc.actions.push(Action::NewProject);
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

/// Barre d'état (nom précédé de « * » si le document est modifié, nombre de
/// tâches, plage, diagnostics) et bandeau des
/// messages (sous la barre d'outils).
pub fn status_bar(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, messages: &[UserMessage]) {
    let texts = fc.texts;
    egui::Panel::bottom("planrs_status_bar").show(ui, |ui| {
        ui.horizontal(|ui| {
            let Some(doc) = fc.doc else {
                ui.label(texts.get(TextId::StatusNoDocument));
                return;
            };
            // « * » si modifié, comme le titre de la fenêtre : sur le web,
            // seul indicateur visible dans la page (relecture finale J4, F4).
            ui.strong(crate::app::document_label(doc));
            ui.separator();
            let count = fc
                .rows
                .map(crate::rows::RowModel::len)
                .filter(|&len| len > 0)
                .unwrap_or_else(|| doc.project().tasks.len());
            let count = i64::try_from(count).unwrap_or(i64::MAX);
            ui.label(texts.get_args(TextId::StatusTasks, &[("count", Arg::Count(count))]));
            ui.separator();
            let labels = texts.format_labels();
            ui.label(texts.get_args(
                TextId::StatusRange,
                &[
                    (
                        "start",
                        Arg::Str(format_date(doc.schedule().project_start, labels).into()),
                    ),
                    (
                        "finish",
                        Arg::Str(format_date(doc.schedule().project_finish, labels).into()),
                    ),
                ],
            ));
            ui.separator();
            // Cliquer sur un compteur ouvre ou referme le panneau des
            // diagnostics. Compteurs calculés une fois par document (cache
            // d'`App`, `DiagnosticsView`).
            let errors = i64::try_from(fc.diagnostics.errors).unwrap_or(i64::MAX);
            let warnings = i64::try_from(fc.diagnostics.warnings).unwrap_or(i64::MAX);
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

/// Bandeau « Enregistrer les modifications de « nom » ? » (Enregistrer /
/// Ne pas enregistrer / Annuler), non bloquant : rien n'est dessiné si
/// `pending` vaut `None`. `pending` est le nom du document modifié.
pub fn confirm_banner(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, pending: Option<&str>) {
    let Some(name) = pending else {
        return;
    };
    let texts = fc.texts;
    let fg = fc.palette.text_strong;
    let frame = egui::Frame::side_top_panel(ui.style()).fill(fc.palette.message_warning_bg);
    egui::Panel::top("planrs_confirm_banner")
        .frame(frame)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let title = texts.get_args(TextId::ConfirmUnsavedTitle, &[("name", Arg::Str(name.into()))]);
                ui.label(egui::RichText::new(title).strong().color(fg));
                let choices = [
                    (TextId::ConfirmSave, ConfirmChoice::Save),
                    (TextId::ConfirmDiscard, ConfirmChoice::Discard),
                    (TextId::ConfirmCancel, ConfirmChoice::Cancel),
                ];
                for (label, choice) in choices {
                    if ui.button(texts.get(label)).clicked() {
                        fc.actions.push(Action::Confirm(choice));
                    }
                }
            });
        });
}

/// Les [`MAX_MESSAGES`] messages les plus récents (fin de la liste) : `App`
/// borne déjà sa liste, le bandeau la borne aussi à l'affichage pour ne
/// jamais manger la vue (D-fix-9).
fn recent_messages(messages: &[UserMessage]) -> &[UserMessage] {
    let skip = messages.len().saturating_sub(MAX_MESSAGES);
    messages.get(skip..).unwrap_or(messages)
}

/// Fond et liseré du bandeau selon la gravité la plus haute affichée.
fn banner_colors(
    has_error: bool,
    palette: &Palette,
    visuals: &egui::Visuals,
) -> (egui::Color32, egui::Color32) {
    if has_error {
        (palette.message_error_bg, palette.critical)
    } else {
        (palette.message_warning_bg, visuals.warn_fg_color)
    }
}

/// Bandeau des messages, refermable ; jamais de modale. Texte fort
/// (`text_strong`) sur fond teinté (contraste ≥ 4,5:1, testé dans `theme`),
/// liseré gauche coloré selon la gravité ; au plus [`MAX_MESSAGES`]
/// messages, les plus récents.
fn message_banner(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, messages: &[UserMessage]) {
    let messages = recent_messages(messages);
    if messages.is_empty() {
        return;
    }
    let texts = fc.texts;
    let palette = fc.palette;
    let has_error = messages.iter().any(|message| message.severity == Severity::Error);
    // Titre de chaque message (« Impossible d'ouvrir le fichier »,
    // « Modification refusée »), une fois par suite de messages de même
    // titre.
    let titled: Vec<(Option<String>, &UserMessage)> = messages
        .iter()
        .scan(None, |previous: &mut Option<TextId>, message| {
            let title = message
                .title
                .filter(|title| *previous != Some(*title))
                .map(|title| texts.get(title));
            *previous = message.title;
            Some((title, message))
        })
        .collect();
    let (fill, accent) = banner_colors(has_error, palette, ui.visuals());
    let fg = palette.text_strong;
    let frame = egui::Frame::side_top_panel(ui.style()).fill(fill);
    egui::Panel::top("planrs_message_banner")
        .frame(frame)
        .show(ui, |ui| {
            let rect = ui.max_rect();
            if rect.is_finite() {
                let bar =
                    egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.left() + 3.0, rect.bottom()));
                ui.painter()
                    .rect_filled(bar.expand2(egui::vec2(0.0, 2.0)), 0.0, accent);
            }
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    for (title, message) in titled {
                        if let Some(title) = title {
                            ui.label(egui::RichText::new(title).strong().size(15.0).color(fg));
                        }
                        ui.label(egui::RichText::new(message.text.as_str()).color(fg));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(texts.get(TextId::MessageDismiss)).clicked() {
                        fc.actions.push(Action::DismissMessage);
                    }
                });
            });
        });
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use planrs_core::Severity;

    use super::recent_messages;
    use crate::app::{MAX_MESSAGES, UserMessage};

    fn message(n: usize) -> UserMessage {
        UserMessage {
            severity: Severity::Warning,
            title: None,
            text: n.to_string(),
        }
    }

    #[test]
    fn banner_keeps_most_recent_messages() {
        let none: Vec<UserMessage> = Vec::new();
        assert_eq!(recent_messages(&none).len(), 0);
        let two: Vec<UserMessage> = (0..2).map(message).collect();
        assert_eq!(recent_messages(&two), two.as_slice());
        let forty: Vec<UserMessage> = (0..40).map(message).collect();
        let shown = recent_messages(&forty);
        assert_eq!(shown.len(), MAX_MESSAGES);
        assert_eq!(shown.first().unwrap().text, (40 - MAX_MESSAGES).to_string());
        assert_eq!(shown.last().unwrap().text, "39");
    }
}
