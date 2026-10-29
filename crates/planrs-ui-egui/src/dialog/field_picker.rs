//! Sélecteur de champ « Insérer une colonne » (H-UI-114, S92, lot L6) :
//! zone de recherche focalisée (sous-chaîne sans accents ni casse, titre et
//! alias, [`crate::field::search`]), liste groupée par catégorie, ↑/↓ pour
//! choisir, Entrée insère, Échap ferme ; champ déjà affiché marqué (l'insérer
//! le déplace) ; champ indisponible grisé avec sa raison.
//!
//! Une préférence d'affichage : l'insertion passe par `ColumnOp::Insert`
//! (`UiState.columns`), aucune entrée d'annulation. Hébergé par
//! `ViewTransient::picker`, qui porte aussi « Paramètres de champ »
//! ([`super::field_settings`]) : [`show`] dessine l'un ou l'autre.

use std::collections::BTreeMap;

use planrs_core::model::custom::CustomFieldId;

use crate::action::{Action, ColumnOp};
use crate::column::Column;
use crate::field::{Availability, FieldCategory, FieldPicker, category, choices, describe, search};
use crate::i18n::{TextId, Texts};
use crate::view::FrameCtx;

use super::field_settings::{self, SettingsOutcome};

/// Identifiant de la fenêtre (couche modale d'egui).
#[must_use]
pub fn modal_id() -> egui::Id {
    egui::Id::unique("planrs.field_picker")
}

/// Identifiant de la zone de recherche (focalisée à l'ouverture).
#[must_use]
pub fn search_id() -> egui::Id {
    egui::Id::unique("planrs.field_picker.search")
}

/// Alias des champs personnalisés cherchés par le sélecteur (servis par le
/// lot L9 ; vides jusque-là).
fn aliases(_fc: &FrameCtx<'_>) -> BTreeMap<CustomFieldId, String> {
    BTreeMap::new()
}

/// Libellé d'une entrée : titre du champ, « déjà affiché » s'il l'est.
fn entry_label(texts: &Texts, column: Column, shown: bool) -> String {
    let title = column.label(texts);
    if shown {
        format!("{title} — {}", texts.get(TextId::FieldPickerShown))
    } else {
        title
    }
}

/// Réponse du sélecteur pour une image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickOutcome {
    /// Rien.
    Open,
    /// Échap ou « Annuler ».
    Cancel,
    /// Champ choisi (clic, ou Entrée sur l'entrée en surbrillance).
    Insert(Column),
}

/// Touches du sélecteur, lues **avant** la zone de recherche (elle les
/// prendrait) : ↑/↓ déplacent la surbrillance (bornée), Entrée choisit,
/// Échap ferme.
fn picker_keys(ctx: &egui::Context, picker: &mut FieldPicker, list: &[Column]) -> PickOutcome {
    let (up_presses, down_presses, enter, escape) = ctx.input_mut(|i| {
        (
            i.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
            i.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
            i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
        )
    });
    let max_rank = list.len().saturating_sub(1);
    picker.highlighted = picker
        .highlighted
        .saturating_add(down_presses)
        .saturating_sub(up_presses)
        .min(max_rank);
    if escape {
        return PickOutcome::Cancel;
    }
    if enter && let Some(column) = list.get(picker.highlighted) {
        return PickOutcome::Insert(*column);
    }
    PickOutcome::Open
}

/// Dessine le sélecteur (une image) ; rend la réponse.
fn show_picker(ctx: &egui::Context, fc: &FrameCtx<'_>, picker: &mut FieldPicker) -> PickOutcome {
    let texts = fc.texts;
    let lang = texts.language();
    let aliases = aliases(fc);
    let list = choices(&picker.query, lang, &aliases);
    let mut outcome = picker_keys(ctx, picker, &list);
    let highlighted = list.get(picker.highlighted).copied();
    let found = search(&picker.query, lang, &aliases);
    let focus = std::mem::take(&mut picker.focus_pending);
    egui::Modal::new(modal_id()).show(ctx, |ui| {
        ui.set_width(340.0);
        ui.heading(texts.get(TextId::MenuInsertColumn));
        let hint = texts.get(TextId::FieldPickerSearch);
        let before = picker.query.clone();
        let response = ui.add(
            egui::TextEdit::singleline(&mut picker.query)
                .id(search_id())
                .hint_text(hint.clone())
                .desired_width(f32::INFINITY),
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::SearchInput, true, hint.clone()));
        if focus {
            response.request_focus();
        }
        if picker.query != before {
            picker.highlighted = 0;
        }
        ui.separator();
        egui::ScrollArea::vertical()
            .max_height(360.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if found.is_empty() {
                    ui.weak(texts.get(TextId::FieldPickerEmpty));
                }
                for cat in FieldCategory::ALL {
                    let mut entries = found.iter().copied().filter(|c| category(*c) == cat).peekable();
                    if entries.peek().is_none() {
                        continue;
                    }
                    ui.label(egui::RichText::new(texts.get(cat.title())).strong());
                    for column in entries {
                        let shown = fc.state.shown(column);
                        let label = entry_label(texts, column, shown);
                        let reason = match describe(column).map(|d| d.availability) {
                            Some(Availability::Unavailable(reason)) => Some(reason),
                            _ => None,
                        };
                        let selected = highlighted == Some(column);
                        let button = egui::Button::selectable(selected, label);
                        let response = match reason {
                            Some(reason) => ui
                                .add_enabled(false, button)
                                .on_disabled_hover_text(texts.get(reason)),
                            None => ui.add(button),
                        };
                        if selected {
                            response.scroll_to_me(None);
                        }
                        if response.clicked() {
                            outcome = PickOutcome::Insert(column);
                        }
                    }
                }
            });
        ui.separator();
        if ui.button(texts.get(TextId::TaskInfoCancel)).clicked() {
            outcome = PickOutcome::Cancel;
        }
    });
    outcome
}

/// Fenêtre de colonne ouverte (`picker`, contrat L0 : `ViewTransient::picker`)
/// : « Paramètres de champ » si son brouillon est posé, sinon le sélecteur.
/// Les réponses deviennent des `Action::Column` (ou `FitColumn`) poussées
/// dans la file de l'image ; la fenêtre se ferme à la réponse.
pub fn show(ctx: &egui::Context, fc: &mut FrameCtx<'_>, picker: &mut Option<FieldPicker>) {
    let Some(open) = picker.as_mut() else {
        return;
    };
    if fc.doc.is_none() {
        *picker = None;
        return;
    }
    if let Some(draft) = open.settings.as_mut() {
        let shown: Vec<Column> = fc.state.columns.iter().map(|c| c.column).collect();
        let focus = std::mem::take(&mut open.focus_pending);
        let outcome = field_settings::show(ctx, fc.texts, draft, &shown, focus);
        if outcome != SettingsOutcome::Open {
            fc.actions
                .extend(field_settings::actions(outcome, open.at, draft));
            *picker = None;
            fc.actions.push(Action::FocusPane(crate::view::Pane::Table));
        }
        return;
    }
    match show_picker(ctx, fc, open) {
        PickOutcome::Open => {}
        PickOutcome::Cancel => {
            *picker = None;
            fc.actions.push(Action::FocusPane(crate::view::Pane::Table));
        }
        PickOutcome::Insert(column) => {
            fc.actions
                .push(Action::Column(ColumnOp::Insert { at: open.at, column }));
            *picker = None;
            fc.actions.push(Action::FocusPane(crate::view::Pane::Table));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::entry_label;
    use crate::column::Column;
    use crate::i18n::{Language, Texts};

    #[test]
    fn h_ui_114_entry_label_marks_shown() {
        let texts = Texts::new(Language::Fr);
        assert_eq!(entry_label(&texts, Column::Work, false), "Travail");
        assert_eq!(
            entry_label(&texts, Column::Duration, true),
            "Durée — déjà affiché"
        );
    }
}
