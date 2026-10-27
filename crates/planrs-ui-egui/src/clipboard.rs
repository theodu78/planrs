//! Couper, copier, coller de lignes dans l'interface (H-UI-39, ADR-0016
//! D3). egui livre `Event::Copy`, `Event::Cut` et `Event::Paste(String)`
//! (jamais Ctrl/Cmd+C/X/V comme touches) ; le tampon interne et son
//! empreinte permettent de reconnaître au collage le dernier texte émis
//! (collage complet, R-244), sinon le collage externe réduit (R-256).
//! Contrat posé par le lot L0, câblé en L10.
//!
//! - Les trois événements sont pris par l'application seulement quand la
//!   table a le focus, sans saisie, menu ni fenêtre ouverts
//!   ([`App::take_clipboard_events`]) ; sinon ils restent à l'éditeur de
//!   texte qui a le focus.
//! - Copier : [`RowClip::capture`] de la sélection (descendants des
//!   récapitulatives inclus), texte tabulé des colonnes visibles
//!   ([`tsv_columns`]) écrit par `ctx.copy_text`, [`ClipboardSlot`] gardé
//!   par l'application (il survit à l'ouverture d'un autre document).
//! - Couper : copier, puis la suppression de la structure dans un lot à
//!   part (une entrée d'annulation), sans confirmation pour une
//!   récapitulative : son bloc est dans le tampon.
//! - Coller : empreinte du texte égale à celle du tampon ⇒
//!   [`paste_batch`] (options par défaut, ou celles de « Collage
//!   spécial… ») ; sinon [`paste_text`]. Insertion avant la tâche active
//!   (fin du projet sur la ligne vide) ; un lot, compte rendu au bandeau.
//! - Menu Édition : Couper et Copier agissent directement (rien n'attend un
//!   événement du système) ; Coller demande le texte du presse-papiers
//!   (`ViewportCommand::RequestPaste`, desktop), dont l'événement est
//!   accepté à l'image suivante même si la table n'a plus le focus. Le
//!   navigateur ne rend pas le presse-papiers à un menu : sur le web, Coller
//!   colle le tampon interne, ou invite à Ctrl+V.

use planrs_automation::clip::{
    self, ClipError, MAX_PASTE_BYTES, PasteOptions, PasteReport, RowClip, paste_batch, paste_text,
};
use planrs_automation::csv::CsvColumn;
use planrs_automation::text::TextContext;
use planrs_automation::{Batch, Placement, TaskRef};
use planrs_core::{Severity, TaskUid};

use crate::app::{App, EditOrigin};
use crate::column::Column;
use crate::i18n::{Arg, TextId, error_text_id};
use crate::selection::Selection;
use crate::state::UiState;
use crate::structure::{self, StructureOp};

/// Dernière copie de l'application : tampon et empreinte du texte écrit
/// dans le presse-papiers du système.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClipboardSlot {
    /// Tampon interne.
    pub clip: RowClip,
    /// Empreinte FNV-1a du texte émis (`planrs_automation::clip::fingerprint`).
    pub fingerprint: u64,
    /// Ce qui a été copié : lignes entières (tampon `clip`, H-UI-39) ou
    /// cellules d'un bloc (texte seul, H-UI-95 ; ADR-0018 D3, lot L2).
    pub kind: ClipKind,
}

/// Contenu de la dernière copie (ADR-0018 D3, H-UI-95).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClipKind {
    /// Lignes entières : le tampon se colle en lignes (R-244).
    #[default]
    Rows,
    /// Cellules d'un bloc : le texte se colle en cellules (R-257).
    Cells,
}

/// Commande du presse-papiers (menu Édition).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardOp {
    /// Couper la sélection.
    Cut,
    /// Copier la sélection.
    Copy,
    /// Coller le presse-papiers.
    Paste,
    /// « Collage spécial… » : options du collage du tampon interne.
    PasteSpecial,
}

/// Colonne du texte tabulé pour une colonne de la table : seules les
/// colonnes qu'un import CSV sait relire (ID, Nom, Durée, Début, Fin,
/// Prédécesseurs, % achevé) ; les autres (marge, critique, ressources,
/// ligne de base, variation) sont calculées ou propres à l'affichage.
#[must_use]
pub const fn tsv_column(column: Column) -> Option<CsvColumn> {
    match column {
        Column::Id => Some(CsvColumn::Id),
        Column::Name => Some(CsvColumn::Name),
        Column::Duration => Some(CsvColumn::Duration),
        Column::Start => Some(CsvColumn::Start),
        Column::Finish => Some(CsvColumn::Finish),
        Column::Predecessors => Some(CsvColumn::Predecessors),
        Column::PercentComplete => Some(CsvColumn::PercentComplete),
        Column::TotalSlack
        | Column::Critical
        | Column::ResourceNames
        | Column::BaselineStart
        | Column::BaselineFinish
        | Column::FinishVariance
        | Column::Mode => None,
    }
}

/// Colonnes du texte tabulé : colonnes visibles de la table, dans leur
/// ordre, réduites à celles de [`tsv_column`].
#[must_use]
pub fn tsv_columns(state: &UiState) -> Vec<CsvColumn> {
    state
        .visible_columns()
        .filter_map(|c| tsv_column(c.column))
        .collect()
}

/// Images après une demande de collage par le menu pendant lesquelles
/// l'événement `Paste` est accepté hors du focus de la table.
const MENU_PASTE_FRAMES: u64 = 2;

impl App {
    /// Dernière copie (tampon interne et empreinte), s'il y en a une.
    #[must_use]
    pub const fn clipboard(&self) -> Option<&ClipboardSlot> {
        self.clipboard.as_ref()
    }

    /// Retire de l'image et traite `Event::Copy`, `Event::Cut` et
    /// `Event::Paste` quand `table` (la table a le focus, sans saisie, menu
    /// ni fenêtre) ou juste après un « Coller » du menu ; sinon ils restent
    /// aux widgets (éditeur de texte).
    pub(crate) fn take_clipboard_events(&mut self, ctx: &egui::Context, table: bool) {
        let frame = ctx.cumulative_frame_nr();
        let menu_paste = self
            .menu_paste_at
            .is_some_and(|at| frame <= at.saturating_add(MENU_PASTE_FRAMES))
            && self.document.is_some()
            && self.view.dialog.is_none()
            && self.view.edit.editing.is_none()
            && !ctx.text_edit_focused();
        if self
            .menu_paste_at
            .is_some_and(|at| frame > at.saturating_add(MENU_PASTE_FRAMES))
        {
            self.menu_paste_at = None;
        }
        if !table && !menu_paste {
            return;
        }
        let mut taken = Vec::new();
        ctx.input_mut(|input| {
            input.events.retain(|event| match event {
                egui::Event::Copy | egui::Event::Cut if table => {
                    taken.push(event.clone());
                    false
                }
                egui::Event::Paste(_) => {
                    taken.push(event.clone());
                    false
                }
                _ => true,
            });
        });
        for event in taken {
            match event {
                egui::Event::Copy => {
                    self.copy_selection(ctx);
                }
                egui::Event::Cut => self.cut_selection(ctx),
                egui::Event::Paste(text) => {
                    self.menu_paste_at = None;
                    self.paste_clipboard_text(&text);
                }
                _ => {}
            }
        }
    }

    /// Commande du menu Édition.
    pub(crate) fn clipboard_op(&mut self, op: ClipboardOp, ctx: &egui::Context) {
        if self.document.is_none() {
            return;
        }
        match op {
            ClipboardOp::Copy => {
                self.copy_selection(ctx);
            }
            ClipboardOp::Cut => self.cut_selection(ctx),
            ClipboardOp::Paste => {
                if cfg!(target_arch = "wasm32") {
                    // Le navigateur ne rend le presse-papiers qu'à un
                    // événement `paste` (Ctrl+V) : tampon interne, sinon
                    // invitation au raccourci.
                    if self.clipboard.is_some() {
                        self.paste_internal(PasteOptions::default());
                    } else {
                        let text = self.texts.get(TextId::PasteUseShortcut);
                        self.push_titled(Severity::Info, Some(TextId::PasteReportTitle), text);
                    }
                } else {
                    self.menu_paste_at = Some(ctx.cumulative_frame_nr());
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                }
            }
            ClipboardOp::PasteSpecial => self.open_paste_special(),
        }
    }

    /// Copie la sélection : tampon interne et texte tabulé des colonnes
    /// visibles écrit dans le presse-papiers. Rend vrai si quelque chose a
    /// été copié (sélection vide ou ligne vide : rien).
    pub(crate) fn copy_selection(&mut self, ctx: &egui::Context) -> bool {
        self.sync_selection();
        let lang = self.text_language();
        let Some(document) = &self.document else {
            return false;
        };
        let uids: Vec<TaskUid> = self.selected.uids().iter().copied().collect();
        if uids.is_empty() {
            return false;
        }
        match RowClip::capture(&document.session, &uids) {
            Ok(clip) if clip.tasks.is_empty() => false,
            Ok(clip) => {
                let text_ctx = TextContext::for_project(document.project(), lang);
                let text = clip.to_tsv(&tsv_columns(&self.state), &text_ctx);
                let fingerprint = clip::fingerprint(&text);
                ctx.copy_text(text);
                self.clipboard = Some(ClipboardSlot {
                    clip,
                    fingerprint,
                    kind: ClipKind::Rows,
                });
                true
            }
            Err(error) => {
                self.report_clip_error(&error);
                false
            }
        }
    }

    /// Coupe la sélection : copie, puis suppression dans un lot à part
    /// (annulable seul). Une récapitulative part avec son bloc, sans
    /// confirmation (le bloc est dans le tampon).
    pub(crate) fn cut_selection(&mut self, ctx: &egui::Context) {
        if !self.copy_selection(ctx) {
            return;
        }
        let Some(document) = &self.document else {
            return;
        };
        let Some(plan) = structure::plan(StructureOp::Delete, document.project(), &self.selected) else {
            return;
        };
        // Un refus est affiché par `execute`.
        drop(self.execute(&plan.batch, EditOrigin::User));
    }

    /// Colle un texte reçu du presse-papiers du système : tampon interne si
    /// l'empreinte est celle du dernier texte émis, sinon collage externe.
    pub(crate) fn paste_clipboard_text(&mut self, text: &str) {
        let Some(document) = &self.document else {
            return;
        };
        if text.len() > MAX_PASTE_BYTES {
            self.report_clip_error(&ClipError::TooLarge);
            return;
        }
        let at = self.paste_placement();
        let print = clip::fingerprint(text);
        let result = match self.clipboard.as_ref().filter(|slot| slot.fingerprint == print) {
            Some(slot) => paste_batch(&document.session, &slot.clip, &at, &PasteOptions::default()),
            None => paste_text(&document.session, text, &at, self.text_language()),
        };
        self.apply_paste(result);
    }

    /// Colle le tampon interne avec `options` (« Collage spécial… », menu
    /// Coller sur le web). Sans tampon : rien.
    pub(crate) fn paste_internal(&mut self, options: PasteOptions) {
        let (Some(document), Some(slot)) = (&self.document, &self.clipboard) else {
            return;
        };
        let at = self.paste_placement();
        let result = paste_batch(&document.session, &slot.clip, &at, &options);
        self.apply_paste(result);
    }

    /// Point d'insertion du collage : avant la tâche active, fin du projet
    /// sur la ligne vide (R-244).
    fn paste_placement(&self) -> Placement {
        self.selection
            .map_or(Placement::End, |uid| Placement::Before(TaskRef::Uid(uid)))
    }

    /// Applique le lot d'un collage (une entrée d'annulation), sélectionne
    /// la première tâche collée et affiche le compte rendu ; un refus
    /// s'affiche au bandeau.
    fn apply_paste(&mut self, result: Result<(Batch, PasteReport), ClipError>) {
        let (batch, report) = match result {
            Ok(pasted) => pasted,
            Err(error) => {
                self.report_clip_error(&error);
                return;
            }
        };
        // Un refus du registre est affiché par `execute`.
        let Ok(outcome) = self.execute(&batch, EditOrigin::User) else {
            return;
        };
        if let Some(first) = outcome.created.first() {
            self.selection = Some(*first);
            self.selected = Selection::single(Some(*first));
            self.sync_selection();
        }
        self.report_paste(&report);
    }

    /// Compte rendu d'un collage au bandeau (titre « Collage ») :
    /// avertissement si des liens, des affectations ou le suivi (avancement,
    /// ligne de base, calendrier, priorité ; relecture finale de J8, F6) n'ont
    /// pas été recopiés, information sinon ; remplace le compte rendu précédent.
    fn report_paste(&mut self, report: &PasteReport) {
        let count = |n: u32| Arg::Count(i64::from(n));
        let mut parts = vec![
            self.texts
                .get_args(TextId::PasteReportTasks, &[("count", count(report.tasks))]),
        ];
        if report.links_dropped > 0 {
            parts.push(self.texts.get_args(
                TextId::PasteReportLinks,
                &[("count", count(report.links_dropped))],
            ));
        }
        if report.assignments_dropped > 0 {
            parts.push(self.texts.get_args(
                TextId::PasteReportAssignments,
                &[("count", count(report.assignments_dropped))],
            ));
        }
        if report.tracking_dropped > 0 {
            parts.push(self.texts.get_args(
                TextId::PasteReportTracking,
                &[("count", count(report.tracking_dropped))],
            ));
        }
        if report.resources_created > 0 {
            parts.push(self.texts.get_args(
                TextId::PasteReportResources,
                &[("count", count(report.resources_created))],
            ));
        }
        let severity =
            if report.links_dropped > 0 || report.assignments_dropped > 0 || report.tracking_dropped > 0 {
                Severity::Warning
            } else {
                Severity::Info
            };
        self.clear_titled(TextId::PasteReportTitle);
        self.push_titled(severity, Some(TextId::PasteReportTitle), parts.join(", "));
    }

    /// Refus d'une copie ou d'un collage au bandeau, traduit par son code
    /// (`paste.unrecognized`, `paste.too_large`…).
    fn report_clip_error(&mut self, error: &ClipError) {
        let detail = match error {
            ClipError::Unrecognized => self.texts.get(TextId::PasteHintUnrecognized),
            ClipError::TooLarge => self.texts.get(TextId::PasteHintTooLarge),
            ClipError::UnknownTask(_) | ClipError::NotYetAvailable => error.to_string(),
        };
        let text = self.texts.get_args(
            error_text_id(error.code()),
            &[("detail", Arg::Str(detail.into()))],
        );
        // Comme un refus du registre : remplace le refus précédent (un lot
        // réussi les retire de même, `App::execute`).
        self.clear_titled(TextId::PasteReportTitle);
        self.clear_titled(TextId::CommandErrorTitle);
        self.push_titled(Severity::Error, Some(TextId::CommandErrorTitle), text);
    }
}
