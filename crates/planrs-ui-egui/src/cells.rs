//! Commandes de cellule sur le bloc de la sélection (ADR-0018 D3 à D5 ;
//! R-257 coller, R-258 remplir vers le bas, R-259 effacer ; H-UI-95 à
//! H-UI-98).
//!
//! Contrat L0 : types et signatures figés, squelettes qui refusent
//! ([`CellError::NotAvailable`], jamais de panique) ; corps écrits par le
//! lot L2. Chaque cellule écrite passera par la lecture de la saisie
//! (`edit::commit_edit`) puis le registre, opérations concaténées dans un
//! seul lot (une entrée d'annulation) ; copier écrit les textes du moteur
//! (`edit::initial_text`, `RowModel`), jamais une date calculée ici
//! (règle 3).
//!
//! `App::cells_op` (réducteur, `Action::Cells`) : en L0, Suppr
//! (`CellOp::DeleteKey`) supprime les tâches comme avant
//! (`Action::Structure(Delete)`) ; les autres commandes affichent « pas
//! encore disponible ».

use planrs_automation::Batch;
use planrs_automation::Session;
use planrs_automation::text::Language as TextLanguage;

use crate::action::{Action, CellOp};
use crate::app::OpenDocument;
use crate::block::Block;
use crate::column::Column;
use crate::i18n::TextId;
use crate::row_view::VisibleRows;
use crate::structure::StructureOp;

use crate::selection::SelectionKind;

/// Lot préparé par une commande de cellule.
#[derive(Clone, Debug, PartialEq)]
pub struct CellPlan {
    /// Lot unique (une entrée d'annulation).
    pub batch: Batch,
    /// Cellules écrites.
    pub written: usize,
    /// Cellules sautées (non modifiables, hors des colonnes visibles…),
    /// comptées au bandeau.
    pub skipped: usize,
}

/// Refus d'une commande de cellule : rien n'est écrit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellError {
    /// Commande pas encore servie (squelette du contrat L0).
    NotAvailable,
    /// Bloc de plus de [`crate::block::MAX_BLOCK_CELLS`] cellules, ou texte
    /// collé au-delà des bornes de R-256.
    TooLarge,
    /// Texte illisible pour une cellule (première cellule fautive).
    Parse {
        /// ID de la ligne.
        id: u32,
        /// Colonne.
        column: Column,
        /// Détail de la lecture refusée.
        detail: String,
    },
    /// Lignes au-delà de la dernière tâche sous un tri ou un filtre.
    PastEnd,
    /// Remplir vers le bas sur une ligne sans ligne affichée au-dessus.
    NothingAbove,
}

impl CellError {
    /// Message du refus.
    #[must_use]
    pub const fn text_id(&self) -> TextId {
        match self {
            Self::NotAvailable => TextId::UxNotAvailable,
            Self::TooLarge => TextId::CellsTooLarge,
            Self::Parse { .. } => TextId::CellsParse,
            Self::PastEnd => TextId::CellsPastEnd,
            Self::NothingAbove => TextId::CellsNothingAbove,
        }
    }
}

/// Compte rendu au bandeau d'une commande de cellule acceptée (L2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellNotice {
    /// Cellules non modifiables sautées.
    Skipped(usize),
    /// Cellules qui ne peuvent pas être vides (R-259).
    NotClearable(usize),
    /// Tâches supprimées par Suppr (H-UI-97), annulables.
    TasksDeleted(usize),
}

impl CellNotice {
    /// Message du compte rendu (argument `count`).
    #[must_use]
    pub const fn text_id(self) -> TextId {
        match self {
            Self::Skipped(_) => TextId::CellsSkipped,
            Self::NotClearable(_) => TextId::CellsNotClearable,
            Self::TasksDeleted(_) => TextId::TasksDeleted,
        }
    }

    /// Nombre compté.
    #[must_use]
    pub const fn count(self) -> usize {
        match self {
            Self::Skipped(n) | Self::NotClearable(n) | Self::TasksDeleted(n) => n,
        }
    }
}

/// Libellés du groupe Édition du ruban (Remplir ▾ Vers le bas, Effacer ▾
/// Contenu, ADR-0018 D10) et leurs info-bulles, posés en L0 et placés dans
/// la table `RIBBON` par le lot L5.
pub const RIBBON_EDITING_TEXTS: [TextId; 6] = [
    TextId::RibbonFill,
    TextId::RibbonFillDown,
    TextId::RibbonClear,
    TextId::RibbonClearContents,
    TextId::RibbonTipFill,
    TextId::RibbonTipClear,
];

impl CellOp {
    /// Libellé du jumeau de menu (menu Édition et menus contextuels,
    /// H-UI-100) ; `None` pour les gestes sans entrée de menu.
    #[must_use]
    pub const fn menu_label(&self) -> Option<TextId> {
        match self {
            Self::FillDown => Some(TextId::MenuFillDown),
            Self::Clear => Some(TextId::MenuClearContents),
            Self::DeleteKey | Self::FillTyped(_) => None,
        }
    }
}

/// Suppr selon la cible (H-UI-97, ADR-0018 D5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteTarget {
    /// Supprimer les tâches des lignes de la sélection.
    Tasks,
    /// Effacer le contenu du bloc (R-259).
    Contents,
}

/// Cible de Suppr : lignes entières, ou cellule active dans l'ID ou le Nom
/// ⇒ supprimer les tâches ; sinon ⇒ effacer le contenu du bloc (H-UI-97).
#[must_use]
pub fn delete_target(block: &Block) -> DeleteTarget {
    if block.kind == SelectionKind::Rows || matches!(block.active.1, Column::Id | Column::Name) {
        DeleteTarget::Tasks
    } else {
        DeleteTarget::Contents
    }
}

/// Texte tabulé des cellules du bloc (H-UI-95) : texte de saisie pour une
/// colonne modifiable, texte affiché sinon. Squelette L0 : texte vide.
#[must_use]
pub fn copy_text(_block: &Block, _doc: &OpenDocument, _rows: VisibleRows<'_>, _lang: TextLanguage) -> String {
    String::new()
}

/// Coller un texte dans les cellules à partir du coin du bloc (R-257).
///
/// # Errors
/// [`CellError`] : rien n'est écrit (squelette L0 : `NotAvailable`).
pub fn paste_plan(
    _block: &Block,
    _text: &str,
    _s: &Session,
    _rows: VisibleRows<'_>,
    _lang: TextLanguage,
) -> Result<CellPlan, CellError> {
    Err(CellError::NotAvailable)
}

/// Remplir vers le bas (R-258) ; `Ok(None)` : rien à écrire.
///
/// # Errors
/// [`CellError`] : rien n'est écrit (squelette L0 : `NotAvailable`).
pub fn fill_down_plan(
    _block: &Block,
    _s: &Session,
    _rows: VisibleRows<'_>,
    _lang: TextLanguage,
) -> Result<Option<CellPlan>, CellError> {
    Err(CellError::NotAvailable)
}

/// Ctrl/Cmd+Entrée : le texte saisi dans toutes les cellules modifiables du
/// bloc (H-UI-98, R-258).
///
/// # Errors
/// [`CellError`] : rien n'est écrit (squelette L0 : `NotAvailable`).
pub fn fill_with(
    _block: &Block,
    _text: &str,
    _s: &Session,
    _rows: VisibleRows<'_>,
    _lang: TextLanguage,
) -> Result<CellPlan, CellError> {
    Err(CellError::NotAvailable)
}

/// Effacer le contenu du bloc (R-259) ; `Ok(None)` : rien à effacer.
///
/// # Errors
/// [`CellError`] : rien n'est écrit (squelette L0 : `NotAvailable`).
pub fn clear_plan(
    _block: &Block,
    _s: &Session,
    _rows: VisibleRows<'_>,
    _lang: TextLanguage,
) -> Result<Option<CellPlan>, CellError> {
    Err(CellError::NotAvailable)
}

impl crate::App {
    /// Commande de cellule (`Action::Cells`). L0 : Suppr supprime les
    /// tâches de la sélection comme avant (même chemin que
    /// `Action::Structure(Delete)`, gardes du réducteur comprises) ; les
    /// autres commandes affichent « pas encore disponible » (lot L2).
    #[allow(
        clippy::needless_pass_by_value,
        reason = "contrat L0 : le lot L2 consomme le texte de `FillTyped`"
    )]
    pub(crate) fn cells_op(&mut self, op: CellOp, ctx: &egui::Context) {
        match op {
            CellOp::DeleteKey => self.apply(Action::Structure(StructureOp::Delete), ctx),
            CellOp::FillDown | CellOp::Clear | CellOp::FillTyped(_) => {
                if self.document.is_none() {
                    return;
                }
                let text = self.texts.get(CellError::NotAvailable.text_id());
                self.push_message(planrs_core::Severity::Info, text);
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use planrs_core::TaskUid;

    use super::{CellError, CellNotice, DeleteTarget, RIBBON_EDITING_TEXTS, delete_target};
    use crate::action::CellOp;
    use crate::block::Block;
    use crate::column::Column;
    use crate::nav::RowRef;
    use crate::row_view::DisplayRow;
    use crate::selection::SelectionKind;

    /// H-UI-97 : lignes entières, ou cellule active dans l'ID ou le Nom ⇒
    /// supprimer les tâches ; Durée (et toute autre colonne) ⇒ effacer.
    #[test]
    fn h_ui_97_delete_target() {
        let block = |kind, column| Block {
            kind,
            rows: vec![DisplayRow(3)],
            columns: vec![column],
            active: (RowRef::Item(TaskUid::new(3)), column),
        };
        assert_eq!(
            delete_target(&block(SelectionKind::Rows, Column::Duration)),
            DeleteTarget::Tasks
        );
        assert_eq!(
            delete_target(&block(SelectionKind::Cells, Column::Id)),
            DeleteTarget::Tasks
        );
        assert_eq!(
            delete_target(&block(SelectionKind::Cells, Column::Name)),
            DeleteTarget::Tasks
        );
        for column in [
            Column::Duration,
            Column::Start,
            Column::Predecessors,
            Column::PercentComplete,
        ] {
            assert_eq!(
                delete_target(&block(SelectionKind::Cells, column)),
                DeleteTarget::Contents
            );
        }
    }

    /// Contrat L0 : chaque refus et chaque compte rendu a son message ;
    /// les squelettes refusent sans paniquer.
    #[test]
    fn cells_contract_texts_and_skeletons() {
        let errors = [
            CellError::NotAvailable,
            CellError::TooLarge,
            CellError::Parse {
                id: 3,
                column: crate::column::Column::Duration,
                detail: String::new(),
            },
            CellError::PastEnd,
            CellError::NothingAbove,
        ];
        let mut ids: Vec<_> = errors.iter().map(CellError::text_id).collect();
        ids.extend(
            [
                CellNotice::Skipped(1),
                CellNotice::NotClearable(2),
                CellNotice::TasksDeleted(3),
            ]
            .map(CellNotice::text_id),
        );
        ids.extend(RIBBON_EDITING_TEXTS);
        let before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), before, "un message par cas");
        assert_eq!(CellNotice::TasksDeleted(3).count(), 3);
        assert!(CellOp::FillDown.menu_label().is_some());
        assert!(CellOp::FillTyped("2".into()).menu_label().is_none());
    }
}
