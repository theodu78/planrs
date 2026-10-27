//! Chronologie à l'écran et à l'impression (ADR-0017 D7, D8, H-UI-78 à
//! H-UI-82) : module posé par le contrat (lot L0), rempli par le lot L4.
//!
//! L0 : préférence de visibilité et clic sur un élément (sélection).

use planrs_ui_egui::Action;
use planrs_ui_egui::action::TimelineOp;

use crate::{harness, open_demo};

/// H-UI-78 (préférence, lot L0) : visible par défaut, `SetVisible` la
/// change ; `Activate` sélectionne la tâche.
#[test]
fn timeline_l0_visibility_and_activate() {
    let mut harness = harness();
    assert!(harness.state().state.timeline_visible);
    harness
        .state_mut()
        .dispatch(Action::Timeline(TimelineOp::SetVisible(false)));
    harness.run();
    assert!(!harness.state().state.timeline_visible);
    open_demo(&mut harness);
    let uid = harness
        .state()
        .document
        .as_ref()
        .unwrap()
        .project()
        .tasks
        .iter()
        .nth(2)
        .map(planrs_core::HasUid::uid)
        .unwrap();
    harness
        .state_mut()
        .dispatch(Action::Timeline(TimelineOp::Activate(uid)));
    harness.run();
    assert_eq!(harness.state().selection, Some(uid));
}
