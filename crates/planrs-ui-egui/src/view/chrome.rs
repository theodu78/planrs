//! Habillage : barre de menus, écran d'accueil, barre d'état et bandeau de
//! messages (le ruban à onglets est dans `view::ribbon`, ADR-0017 ; il
//! remplace depuis le lot L5 l'ancienne barre d'outils de J7 et J8).
//!
//! `menu_bar` et `status_bar` reçoivent l'`Ui` racine et posent leur propre
//! panneau (`egui::Panel`) ; `welcome` dessine dans le panneau central.
//! Aucune chaîne en dur : tout passe par `TextId`.

use planrs_core::Severity;

use crate::action::Action;
use crate::action::ConfirmChoice;
use crate::app::{MAX_MESSAGES, UserMessage};
use crate::column::Column;
use crate::dialog::filter::FilterDialogMode;
use crate::format::format_date;
use crate::i18n::{Arg, TextId};
use crate::io::SaveKind;
use crate::keymap::{Binding, Shortcut, UiBinding, shortcut_for_os, shortcut_text};
use crate::ribbon::spec::{PERCENT_STEPS, RibbonCmd, percent_value};
use crate::state::{FilterChoice, MAX_OUTLINE_LEVEL};
use crate::state::{LanguagePref, ThemePref};
use crate::theme::Palette;
use crate::timescale::TimeScaleZoom;
use crate::view::task_view::TaskViewOp;

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

/// Vues proposées (J7 lots L7, L10, L11 : Gantt, Gantt de suivi, feuille
/// et utilisation des ressources), dans l'ordre du menu et de la liste
/// « Diagramme de Gantt ▾ » du ruban.
const VIEW_CHOICES: [RibbonCmd; 4] = [
    RibbonCmd::GanttChart,
    RibbonCmd::TrackingGantt,
    RibbonCmd::ResourceSheet,
    RibbonCmd::ResourceUsage,
];

/// Clé de la mémoire d'egui où la barre de menus dépose l'état du volet
/// latéral de l'image, relu par le ruban (case « Volet Opérations IA »).
fn side_open_id() -> egui::Id {
    egui::Id::unique("planrs.side-open")
}

/// État du volet latéral déposé par [`menu_bar`] à cette image.
fn side_open(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(side_open_id()))
        .unwrap_or(false)
}

/// État d'une case ou d'une bascule (H-UI-74) : [`RibbonCmd::checked`],
/// sauf le volet Opérations IA, dont l'état est relu ici (même source pour
/// le menu et le ruban).
pub(crate) fn item_checked(ctx: &egui::Context, fc: &FrameCtx<'_>, cmd: RibbonCmd) -> Option<bool> {
    match cmd {
        RibbonCmd::AiPanel => Some(side_open(ctx)),
        _ => cmd.checked(fc),
    }
}

/// Pousse l'action d'une commande partagée par le ruban et la barre de
/// menus (H-UI-70) : [`RibbonCmd::action`], puis le style du Gantt pour les
/// vues Gantt (comme le menu Affichage depuis J7).
pub(crate) fn push_cmd(fc: &mut FrameCtx<'_>, cmd: RibbonCmd) {
    if let Some(action) = cmd.action(fc) {
        fc.actions.push(action);
    }
    if let Some(style) = cmd.gantt_style() {
        fc.actions.push(Action::SetGanttStyle(style));
    }
}

/// Raccourci affiché d'une liaison de `KEYMAP` pour le système courant.
fn binding_shortcut(ui: &egui::Ui, binding: Option<Binding>) -> Option<Shortcut> {
    binding.and_then(|binding| shortcut_for_os(binding, ui.ctx().os()).copied())
}

/// Entrée de menu jumelle d'un élément du ruban (H-UI-70, ADR-0017 D2) :
/// libellé du jumeau ([`RibbonCmd::menu_label`]), activation et action de
/// la commande partagée ; raccourci de `binding`.
fn twin(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, cmd: RibbonCmd, binding: Option<Binding>) {
    let shortcut = binding_shortcut(ui, binding);
    let text = twin_text(fc, cmd);
    twin_item(ui, fc, cmd, text, shortcut);
}

/// Libellé de l'entrée jumelle : [`RibbonCmd::menu_label`] ; « Retirer de
/// la chronologie » quand la tâche active y est (H-UI-79).
fn twin_text(fc: &FrameCtx<'_>, cmd: RibbonCmd) -> String {
    match cmd {
        RibbonCmd::AddToTimeline if cmd.checked(fc) == Some(true) => fc.texts.get(TextId::TimelineRemove),
        RibbonCmd::MarkPercent(step) => fc.texts.get_args(
            TextId::MenuMarkPercent,
            &[("percent", Arg::Count(percent_value(step)))],
        ),
        _ => fc.texts.get(cmd.menu_label()),
    }
}

/// Entrée de menu d'une commande partagée, libellé `text` : case pour les
/// préférences d'affichage ([`RibbonCmd::menu_is_check`]), bouton sinon ;
/// grisée selon [`RibbonCmd::enabled`], raison en infobulle (H-UI-73).
pub(crate) fn twin_item(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    cmd: RibbonCmd,
    text: String,
    shortcut: Option<Shortcut>,
) {
    command_item(ui, fc, cmd, text, shortcut, cmd.menu_is_check());
}

/// Entrée d'une commande partagée : case si `check` (état de
/// [`item_checked`]), bouton avec `shortcut` sinon ; grisée selon
/// [`RibbonCmd::enabled`], raison en infobulle (H-UI-73) ; un clic pousse
/// l'action de la commande ([`push_cmd`]).
fn command_item(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    cmd: RibbonCmd,
    text: String,
    shortcut: Option<Shortcut>,
    check: bool,
) {
    let enabled = cmd.enabled(fc);
    let response = if check {
        let mut on = item_checked(ui.ctx(), fc, cmd).unwrap_or(false);
        ui.add_enabled(enabled.yes, egui::Checkbox::new(&mut on, text))
    } else {
        let mut button = egui::Button::new(text);
        if let Some(shortcut) = shortcut {
            button = button.shortcut_text(shortcut_text(ui.ctx(), &shortcut));
        }
        ui.add_enabled(enabled.yes, button)
    };
    let response = match enabled.reason {
        Some(reason) => response.on_disabled_hover_text(fc.texts.get(reason)),
        None => response,
    };
    if response.clicked() {
        push_cmd(fc, cmd);
    }
}

/// Entrée d'un menu contextuel (table, Gantt, en-tête de l'échelle ;
/// H-UI-100, ADR-0018 D10) : la commande du ruban, libellé
/// [`RibbonCmd::context_label`] (« Retirer de la chronologie » quand la
/// tâche active y est), case pour les bascules de tâche (H-UI-74),
/// raccourci [`RibbonCmd::shortcut`] ; même activation et même action que
/// le ruban et la barre de menus.
pub(crate) fn context_item(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, cmd: RibbonCmd) {
    let text = match cmd {
        RibbonCmd::AddToTimeline | RibbonCmd::MarkPercent(_) => twin_text(fc, cmd),
        _ => fc.texts.get(cmd.context_label()),
    };
    let shortcut = cmd.shortcut(ui.ctx().os());
    command_item(
        ui,
        fc,
        cmd,
        text,
        shortcut,
        cmd.context_is_check() || cmd.menu_is_check(),
    );
}

/// Entrée jumelle de la barre de menus avec le raccourci de la commande
/// ([`RibbonCmd::shortcut`], le même qu'au ruban et au menu contextuel).
fn twin_cmd(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, cmd: RibbonCmd) {
    let shortcut = cmd.shortcut(ui.ctx().os());
    let text = twin_text(fc, cmd);
    twin_item(ui, fc, cmd, text, shortcut);
}

/// Barre de menus (Fichier, Édition, Affichage, Tâche, Projet, Format,
/// Aide) ; `side_open` : volet latéral ouvert (case du menu Affichage, J8
/// lot L12), déposé aussi pour le ruban de la même image.
pub fn menu_bar(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, side_open: bool) {
    let texts = fc.texts;
    ui.ctx()
        .data_mut(|data| data.insert_temp(side_open_id(), side_open));
    egui::Panel::top("planrs_menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(texts.get(TextId::MenuFile), |ui| file_menu(ui, fc));
            ui.menu_button(texts.get(TextId::MenuEdit), |ui| edit_menu(ui, fc));
            view_menu(ui, fc, side_open);
            // --- J8 L15 : menus Tâche, Projet, Format, Aide ---
            ui.menu_button(texts.get(TextId::MenuTask), |ui| task_menu(ui, fc));
            ui.menu_button(texts.get(TextId::MenuProject), |ui| project_menu(ui, fc));
            ui.menu_button(texts.get(TextId::MenuFormat), |ui| format_menu(ui, fc));
            ui.menu_button(texts.get(TextId::MenuHelp), |ui| {
                let label = texts.get_args(
                    TextId::MenuAbout,
                    &[("product", Arg::Str(planrs_core::PRODUCT_NAME.into()))],
                );
                if ui.button(label).clicked() {
                    fc.actions.push(Action::About);
                }
            });
        });
    });
}

/// Menu Affichage : vues, zoom, thème, langue, vue des tâches, volet IA,
/// graphique seul, surbrillance du critique, diagnostics ; chronologie et
/// repli du ruban (ADR-0017, jumeaux de l'onglet Affichage).
fn view_menu(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, _side_open: bool) {
    let texts = fc.texts;
    let has_doc = fc.doc.is_some();
    ui.menu_button(texts.get(TextId::MenuView), |ui| {
        ui.add_enabled_ui(has_doc, |ui| view_choices(ui, fc));
        ui.separator();
        ui.menu_button(texts.get(TextId::MenuZoom), |ui| {
            ui.add_enabled_ui(has_doc, |ui| {
                zoom_choices(ui, fc);
                ui.separator();
                twin(ui, fc, RibbonCmd::EntireProject, None);
                // Jumeaux de Zoom avant / Zoom arrière (ADR-0017).
                twin(ui, fc, RibbonCmd::ZoomIn, Some(Binding::Ui(UiBinding::ZoomFiner)));
                twin(
                    ui,
                    fc,
                    RibbonCmd::ZoomOut,
                    Some(Binding::Ui(UiBinding::ZoomCoarser)),
                );
            });
        });
        ui.menu_button(texts.get(TextId::MenuTheme), |ui| theme_choices(ui, fc));
        ui.menu_button(texts.get(TextId::MenuLanguage), |ui| language_choices(ui, fc));
        // --- J8 L9 : filtres, tri, niveau ---
        super::task_view::view_menu(ui, fc);
        // --- J8 L11 : surbrillance du critique ---
        twin(ui, fc, RibbonCmd::CriticalTasks, None);
        // --- J8 L15 : graphique seul (H-UI-64) ---
        twin(ui, fc, RibbonCmd::ChartOnly, None);
        // --- J8 L12 : volet IA, « Pourquoi cette date ? » ---
        // Bascule du volet latéral (H-UI-41) : sans document aussi
        // (l'historique de session reste lisible).
        twin(ui, fc, RibbonCmd::AiPanel, None);
        // --- J8 L15 : galerie, mise en page ---
        ui.separator();
        // Bascule du panneau des diagnostics : les compteurs de la
        // barre d'état l'ouvrent aussi, mais rien ne les signale comme
        // cliquables. Toujours active : un panneau resté ouvert après
        // « Fermer » doit pouvoir se refermer.
        twin(ui, fc, RibbonCmd::Diagnostics, None);
        // --- Ruban L2 : chronologie (H-UI-78), repli du ruban (H-UI-72).
        twin(ui, fc, RibbonCmd::TimelineVisible, None);
        twin(
            ui,
            fc,
            RibbonCmd::CollapseRibbon,
            Some(Binding::Ui(UiBinding::ToggleRibbon)),
        );
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
    twin(ui, fc, RibbonCmd::Save, Some(Binding::Command("project.save")));
    let save_as = Binding::Ui(UiBinding::SaveAs);
    menu_item(ui, fc, TextId::MenuSaveAs, Some(save_as), has_doc, Action::SaveAs);
    // --- J8 L7 : Exporter en PDF ---
    // --- J8 L15 : mise en page, mode impression (H-UI-68) ---
    twin(ui, fc, RibbonCmd::PageSetup, None);
    twin(ui, fc, RibbonCmd::PrintMode, None);
    // --- J8 L10 : Importer CSV ---
    // Fichier → Importer CSV… (H-UI-40) : sans document aussi (le résultat
    // est un nouveau document).
    menu_item(ui, fc, TextId::MenuImportCsv, None, true, Action::ImportCsv);
    // --- J8 L13 ---
    // Fichier → Exporter (PDF, SVG, CSV) : téléchargement sur le web,
    // boîte « Enregistrer sous » sur le desktop.
    let texts = fc.texts;
    ui.add_enabled_ui(has_doc, |ui| {
        ui.menu_button(texts.get(TextId::MenuExport), |ui| {
            twin(ui, fc, RibbonCmd::ExportPdf, None);
            for (id, kind) in [
                (TextId::MenuExportSvg, SaveKind::Svg),
                (TextId::MenuExportCsv, SaveKind::Csv),
            ] {
                menu_item(ui, fc, id, None, true, Action::Export(kind));
            }
        });
    });
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

/// Menu Projet (J7, lot L12 ; dans la barre depuis J8 lot L15) :
/// informations sur le projet, temps de travail, ligne de base,
/// replanifier le travail restant ; grisé sans document.
fn project_menu(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    for cmd in [
        RibbonCmd::ProjectInformation,
        RibbonCmd::WorkingTime,
        RibbonCmd::SetBaseline,
        RibbonCmd::UpdateProject,
    ] {
        twin(ui, fc, cmd, None);
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
/// J4, F5) ; enfin le sous-menu Projet (J7, lot L12). Jumeau du ruban
/// (ADR-0017) : défiler jusqu'à la tâche.
fn edit_menu(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let texts = fc.texts;
    let history = fc.doc.map(|doc| doc.session.history());
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
    let undo = binding_shortcut(ui, Some(Binding::Command("history.undo")));
    let undo_text = agent_undo.unwrap_or_else(|| texts.get(TextId::MenuUndo));
    twin_item(ui, fc, RibbonCmd::Undo, undo_text, undo);
    twin(ui, fc, RibbonCmd::Redo, Some(Binding::Command("history.redo")));
    ui.separator();
    // Six commandes de structure (relecture finale J4, F5), jumelles du
    // menu contextuel (H-UI-100) : sous tri ou filtre, hausser et abaisser
    // sont grisés (H-UI-32, `RibbonCmd::enabled`).
    for cmd in [
        RibbonCmd::InsertTask,
        RibbonCmd::DeleteTask,
        RibbonCmd::Indent,
        RibbonCmd::Outdent,
        RibbonCmd::Link,
        RibbonCmd::Unlink,
    ] {
        twin_cmd(ui, fc, cmd);
    }
    ui.separator();
    // « Affecter des ressources » (J7, lot L13) : aucun raccourci (S61).
    twin(ui, fc, RibbonCmd::AssignResources, None);
    // --- J8 L9 : rechercher ---
    ui.separator();
    twin(ui, fc, RibbonCmd::Find, Some(Binding::Ui(UiBinding::Find)));
    // --- Ruban L2 : défiler jusqu'à la tâche (H-UI-84).
    twin(ui, fc, RibbonCmd::ScrollToTask, None);
    // --- J8 L10 : couper, copier, coller ---
    ui.separator();
    for cmd in [
        RibbonCmd::Cut,
        RibbonCmd::Copy,
        RibbonCmd::Paste,
        RibbonCmd::PasteSpecial,
    ] {
        twin_cmd(ui, fc, cmd);
    }
    // --- Passe UX (ADR-0018 D10, H-UI-100) : remplir, effacer, tout
    // sélectionner (groupe Editing de Project, S80).
    ui.separator();
    for cmd in [
        RibbonCmd::FillDown,
        RibbonCmd::ClearContents,
        RibbonCmd::SelectAll,
    ] {
        twin_cmd(ui, fc, cmd);
    }
    // --- J8 L15 : menu Tâche (%, mode, actif) ---
}

/// Menu Tâche (J8 lot L15) : marquer n %, basculer le mode, activer ou
/// désactiver, informations, lier et délier ; grisé sans sélection. Jumeaux
/// du ruban (ADR-0017) : insérer une récapitulative, un jalon, notes,
/// ajouter à la chronologie.
fn task_menu(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    for (step, _) in PERCENT_STEPS {
        twin(ui, fc, RibbonCmd::MarkPercent(step), None);
    }
    ui.separator();
    twin(ui, fc, RibbonCmd::ManualSchedule, None);
    twin(ui, fc, RibbonCmd::Inactivate, None);
    ui.separator();
    twin(
        ui,
        fc,
        RibbonCmd::TaskInformation,
        Some(Binding::Ui(UiBinding::TaskInformation)),
    );
    twin_cmd(ui, fc, RibbonCmd::Link);
    twin_cmd(ui, fc, RibbonCmd::Unlink);
    // --- Ruban L2 : récapitulative et jalon (H-UI-83 ; au menu Tâche, comme
    // le groupe Insérer de l'onglet Tâche : au menu Édition, leur libellé
    // commencerait par celui d'« Insérer une tâche »), notes (H-UI-84),
    // chronologie (H-UI-79).
    ui.separator();
    twin(ui, fc, RibbonCmd::InsertSummary, None);
    twin(ui, fc, RibbonCmd::InsertMilestone, None);
    twin(ui, fc, RibbonCmd::TaskNotes, None);
    twin(ui, fc, RibbonCmd::AddToTimeline, None);
}

/// Menu Format (J8 lot L15) : galerie des modèles, mise en forme des
/// barres, mois relatifs, mise en page ; colonnes (jumeau du ruban,
/// ADR-0017 ; aussi au clic droit sur un titre de colonne).
fn format_menu(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    twin(ui, fc, RibbonCmd::Gallery, None);
    // Type de la barre de la tâche active, sinon les barres de tâches.
    twin(ui, fc, RibbonCmd::FormatBars, None);
    // H-UI-67 : mois relatifs en option, quel que soit le modèle (relecture
    // finale de J8 ; seul « Planning travaux » les avait).
    twin(ui, fc, RibbonCmd::RelativeMonths, None);
    ui.separator();
    twin(ui, fc, RibbonCmd::PageSetup, None);
    // --- Ruban L2 : colonnes.
    let has_doc = fc.doc.is_some();
    let texts = fc.texts;
    ui.add_enabled_ui(has_doc, |ui| {
        ui.menu_button(texts.get(TextId::RibbonColumns), |ui| column_choices(ui, fc));
    });
}

/// Contenu de la liste déroulante d'un élément du ruban : mêmes entrées
/// (mêmes actions) que la barre de menus (H-UI-70).
pub(crate) fn dropdown_contents(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, cmd: RibbonCmd) {
    let has_doc = fc.doc.is_some();
    match cmd {
        RibbonCmd::ViewMenu => {
            ui.add_enabled_ui(has_doc, |ui| view_choices(ui, fc));
        }
        RibbonCmd::SortMenu => sort_entries(ui, fc),
        RibbonCmd::OutlineMenu => outline_entries(ui, fc),
        RibbonCmd::FilterMenu => filter_entries(ui, fc),
        RibbonCmd::TimescaleMenu => {
            ui.add_enabled_ui(has_doc, |ui| zoom_choices(ui, fc));
        }
        RibbonCmd::ThemeMenu => theme_choices(ui, fc),
        RibbonCmd::LanguageMenu => language_choices(ui, fc),
        RibbonCmd::ColumnsMenu => column_choices(ui, fc),
        // Remplir ▾, Effacer ▾ (passe UX, lot L5) : une entrée, la commande
        // elle-même (même action, même prédicat, même raccourci).
        RibbonCmd::FillDown | RibbonCmd::ClearContents => {
            if let Some(label) = cmd.dropdown_entry() {
                let shortcut = cmd.shortcut(ui.ctx().os());
                twin_item(ui, fc, cmd, fc.texts.get(label), shortcut);
            }
        }
        _ => {}
    }
}

/// Une case par colonne de [`Column::ALL`] (ID et Nom grisées), comme le
/// menu du titre de colonne (J7, lot L6).
fn column_choices(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    for column in Column::ALL {
        let mut visible = fc.state.column(column).is_some_and(|state| state.visible);
        let check = egui::Checkbox::new(&mut visible, fc.texts.get(column.title()));
        if ui.add_enabled(column.hideable(), check).clicked() {
            fc.actions.push(Action::ToggleColumn(column));
        }
    }
}

/// Entrée d'un sous-menu de la vue des tâches, raccourci de `binding`.
fn view_entry(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    label: TextId,
    binding: Option<UiBinding>,
    enabled: bool,
    op: TaskViewOp,
) {
    let mut button = egui::Button::new(fc.texts.get(label));
    if let Some(shortcut) = binding_shortcut(ui, binding.map(Binding::Ui)) {
        button = button.shortcut_text(shortcut_text(ui.ctx(), &shortcut));
    }
    if ui.add_enabled(enabled, button).clicked() {
        fc.actions.push(Action::TaskView(op));
    }
}

/// Liste « Trier ▾ » : Trier…, Trier par ID (entrées du menu Affichage).
fn sort_entries(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let has_doc = fc.doc.is_some();
    let sorted = fc.task_view.state.is_sorted();
    view_entry(
        ui,
        fc,
        TextId::MenuSort,
        None,
        has_doc,
        TaskViewOp::OpenSortDialog,
    );
    view_entry(
        ui,
        fc,
        TextId::MenuSortById,
        Some(UiBinding::SortById),
        has_doc && sorted,
        TaskViewOp::ResetSort,
    );
}

/// Liste « Mode plan ▾ » : niveaux 1 à 9, Tous, masquer et afficher les
/// sous-tâches (entrées du menu Affichage).
fn outline_entries(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let has_doc = fc.doc.is_some();
    let texts = fc.texts;
    ui.add_enabled_ui(has_doc, |ui| {
        let current = fc.task_view.state.outline_level;
        for level in 1..=MAX_OUTLINE_LEVEL {
            let label = texts.get_args(TextId::OutlineLevelN, &[("count", Arg::Count(i64::from(level)))]);
            if ui.selectable_label(current == Some(level), label).clicked() {
                fc.actions
                    .push(Action::TaskView(TaskViewOp::SetOutlineLevel(Some(level))));
            }
        }
        if ui
            .selectable_label(current.is_none(), texts.get(TextId::OutlineAll))
            .clicked()
        {
            fc.actions
                .push(Action::TaskView(TaskViewOp::SetOutlineLevel(None)));
        }
    });
    ui.separator();
    let hide = Some(UiBinding::HideSubtasks);
    view_entry(
        ui,
        fc,
        TextId::MenuHideSubtasks,
        hide,
        has_doc,
        TaskViewOp::HideSubtasks,
    );
    let show = Some(UiBinding::ShowSubtasks);
    view_entry(
        ui,
        fc,
        TextId::MenuShowSubtasks,
        show,
        has_doc,
        TaskViewOp::ShowSubtasks,
    );
}

/// Liste « Filtre ▾ » : préréglages, filtres à fenêtre, réappliquer,
/// effacer (entrées du menu Affichage → Filtre).
fn filter_entries(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let has_doc = fc.doc.is_some();
    let texts = fc.texts;
    ui.add_enabled_ui(has_doc, |ui| {
        for (preset, label) in super::task_view::PRESETS {
            let current = fc.task_view.state.filter == FilterChoice::Preset(preset.clone());
            if ui
                .selectable_label(has_doc && current, texts.get(label))
                .clicked()
            {
                fc.actions
                    .push(Action::TaskView(TaskViewOp::SetFilter(FilterChoice::Preset(
                        preset,
                    ))));
            }
        }
    });
    ui.separator();
    for (label, mode) in [
        (TextId::FilterUsingResource, FilterDialogMode::Resource),
        (TextId::FilterDateRange, FilterDialogMode::DateRange),
        (TextId::FilterCustom, FilterDialogMode::Custom),
    ] {
        view_entry(ui, fc, label, None, has_doc, TaskViewOp::OpenFilterDialog(mode));
    }
    ui.separator();
    let filtered = has_doc && fc.task_view.state.is_filtered();
    view_entry(
        ui,
        fc,
        TextId::FilterReapply,
        None,
        filtered,
        TaskViewOp::ReapplyFilter,
    );
    let clear = Some(UiBinding::ClearFilter);
    view_entry(
        ui,
        fc,
        TextId::FilterClear,
        clear,
        filtered,
        TaskViewOp::ClearFilter,
    );
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
/// n'est surlignée sans document. Menu Affichage et liste « Diagramme de
/// Gantt ▾ » du ruban (commandes partagées, H-UI-70).
fn view_choices(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    for cmd in VIEW_CHOICES {
        if choice(ui, cmd.view_selected(fc), &fc.texts.get(cmd.menu_label())) {
            push_cmd(fc, cmd);
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
            // --- J8 L9 : « Filtre : Critiques » (rien sans filtre).
            super::task_view::status_indicator(ui, fc);
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

// --- J8 L10 : couper, copier, coller ---

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
