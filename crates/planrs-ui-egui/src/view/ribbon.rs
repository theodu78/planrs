//! Moteur de dessin du ruban (ADR-0017 D2, H-UI-69 à H-UI-77) : barre
//! d'accès rapide, bande d'onglets, corps des groupes de
//! [`crate::ribbon::spec::RIBBON`], repli et surimpression.
//!
//! Lot L0 (contrat) : squelette sans dessin, appelé par `App::show` à la
//! place de la barre d'outils quand l'habillage est
//! [`crate::ribbon::Chrome::Ribbon`] (tests seulement avant L5) ; le
//! moteur vient en L2.

use crate::ribbon::{RibbonPrefs, RibbonTab};

use super::FrameCtx;

/// Dessine le ruban sous la barre de menus. `prefs` : onglet et repli
/// persistés ; `overlay` : onglet ouvert en surimpression (ruban replié).
///
/// Lot L0 : rien n'est dessiné.
pub fn show(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, prefs: &RibbonPrefs, overlay: Option<RibbonTab>) {
    let _ = (ui, fc, prefs, overlay);
}
