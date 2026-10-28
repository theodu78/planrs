//! Volet de détails « Formulaire tâche » (seconde passe UX, ADR-0019 D6,
//! H-UI-112, H-UI-113).
//!
//! Contrat posé par le lot L0 : squelette. [`show`] dessine « Sélectionnez
//! une tâche » dans le rectangle donné ; `App::details_op` ne fait rien. Le
//! lot L4 place le volet sous la vue (cadre, en-tête, ×), le lot L5 le
//! remplit (champs du haut, grilles des prédécesseurs, successeurs et
//! ressources, OK et Annuler : un lot, une entrée d'annulation).

use crate::action::DetailsOp;
use crate::i18n::TextId;
use crate::view::FrameCtx;

/// État transitoire du volet (brouillon du formulaire, servi par le lot
/// L5) : tâche suivie et génération du document lue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DetailsTransient {
    /// Tâche suivie (celle de la cellule active), s'il y en a une.
    pub task: Option<planrs_core::TaskUid>,
    /// Clé du document (ouverture, génération) du brouillon.
    pub doc_key: Option<(u64, u64)>,
    /// Le brouillon diffère du document.
    pub dirty: bool,
}

/// Dessine le volet dans `rect` (L0 : invite « Sélectionnez une tâche »,
/// centrée, dans les couleurs du thème).
pub fn show(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, rect: egui::Rect) {
    if !rect.is_finite() || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let text = fc.texts.get(TextId::DetailsSelectTask);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::TextStyle::Body.resolve(ui.style()),
        ui.visuals().weak_text_color(),
    );
}

impl crate::App {
    /// OK ou Annuler du volet (`Action::Details`). L0 : rien (lot L5).
    pub(crate) fn details_op(&mut self, op: DetailsOp) {
        let _ = (op, &self.view.details);
    }
}

/// Textes posés par le lot L0 pour le volet, servis par les lots L4 et L5.
pub const L5_TEXTS: [TextId; 25] = [
    TextId::DetailsTitle,
    TextId::DetailsFormResourcesPredecessors,
    TextId::DetailsFormResourcesSuccessors,
    TextId::DetailsFormPredecessorsSuccessors,
    TextId::DetailsName,
    TextId::DetailsDuration,
    TextId::DetailsEffortDriven,
    TextId::DetailsManual,
    TextId::DetailsStart,
    TextId::DetailsFinish,
    TextId::DetailsTaskType,
    TextId::DetailsPercent,
    TextId::DetailsConstraint,
    TextId::DetailsConstraintDate,
    TextId::DetailsPredecessors,
    TextId::DetailsSuccessors,
    TextId::DetailsResources,
    TextId::DetailsColId,
    TextId::DetailsColName,
    TextId::DetailsColType,
    TextId::DetailsColLag,
    TextId::DetailsColUnits,
    TextId::DetailsColWork,
    TextId::DetailsOk,
    TextId::DetailsCancel,
];

/// Textes posés par le lot L0 pour le volet (suite) : libellé du lot
/// d'annulation, raison des champs grisés d'une récapitulative, menu
/// Affichage → Détails, tâche inconnue, brouillon gardé.
pub const L5_MORE_TEXTS: [TextId; 5] = [
    TextId::DetailsBatchLabel,
    TextId::DetailsSummaryReason,
    TextId::MenuDetailsPane,
    TextId::DetailsUnknownTask,
    TextId::DetailsDraftKept,
];
