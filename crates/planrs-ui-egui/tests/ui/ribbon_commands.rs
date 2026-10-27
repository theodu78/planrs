//! Commandes nouvelles du ruban (ADR-0017 D4, H-UI-79, H-UI-83, H-UI-84) :
//! module posé par le contrat (lot L0), rempli par le lot L3.
//!
//! L0 : squelettes branchés au réducteur ; « pas encore disponible », rien
//! d'écrit dans le document.

use planrs_ui_egui::action::{DialogKind, TimelineOp};
use planrs_ui_egui::i18n::TextId;
use planrs_ui_egui::structure::StructureOp;
use planrs_ui_egui::{Action, App};

use crate::{fr, harness, open_demo};

/// Dernier message affiché.
fn last_message(app: &App) -> Option<String> {
    app.messages().last().map(|m| m.text.clone())
}

/// L0 : Insérer → Jalon et Récapitulative, Ajouter à la chronologie ⇒
/// message, document inchangé (aucune entrée d'annulation).
#[test]
fn ribbon_l0_new_commands_not_available() {
    let mut harness = harness();
    open_demo(&mut harness);
    let first = harness
        .state()
        .document
        .as_ref()
        .unwrap()
        .project()
        .tasks
        .iter()
        .next()
        .map(planrs_core::HasUid::uid);
    harness.state_mut().dispatch(Action::Select(first));
    harness.run();
    let before = harness.state().document.as_ref().unwrap().project().clone();
    for action in [
        Action::Structure(StructureOp::InsertMilestone),
        Action::Structure(StructureOp::InsertSummary),
        Action::Timeline(TimelineOp::ToggleSelection),
    ] {
        harness.state_mut().dispatch(Action::DismissMessage);
        harness.state_mut().dispatch(action.clone());
        harness.run();
        assert_eq!(
            last_message(harness.state()),
            Some(fr(TextId::NotAvailableYet)),
            "{action:?}"
        );
        assert_eq!(
            harness.state().document.as_ref().unwrap().project(),
            &before,
            "{action:?}"
        );
    }
    let history = harness.state().document.as_ref().unwrap().session.history();
    assert_eq!(history.entries.len(), 0, "aucune entrée d'annulation");
}

/// L0 : Notes ouvre « Informations sur la tâche » de la tâche active ;
/// Défiler jusqu'à la tâche ne fait rien (squelette), sans panique.
#[test]
fn ribbon_l0_notes_and_scroll_skeletons() {
    let mut harness = harness();
    open_demo(&mut harness);
    let uid = harness
        .state()
        .document
        .as_ref()
        .unwrap()
        .project()
        .tasks
        .iter()
        .next()
        .map(planrs_core::HasUid::uid);
    harness.state_mut().dispatch(Action::Select(uid));
    harness.run();
    let scroll = harness.state().view().gantt_scroll_x;
    harness.state_mut().dispatch(Action::ScrollToTask);
    harness.run();
    assert!((harness.state().view().gantt_scroll_x - scroll).abs() < f32::EPSILON);
    harness.state_mut().dispatch(Action::ShowTaskNotes);
    harness.run();
    let kind = harness.state().view().dialog.as_ref().map(|d| d.kind);
    assert_eq!(kind, uid.map(DialogKind::TaskInformation));
}
