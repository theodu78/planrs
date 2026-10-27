//! Chronologie à l'écran (ADR-0017 D7, H-UI-78 à H-UI-81) : frise de 72 px
//! au-dessus de la table et du Gantt, disposée par
//! `planrs_automation::timeline::input` puis `planrs_render::timeline::layout`
//! (aucune date calculée ici, règle 3), mise en cache.
//!
//! Lot L0 (contrat) : état de la vue et squelette sans dessin ; la frise
//! vient en L4.

use planrs_render::timeline::TimelineLayout;

use crate::i18n::{Language, TextId};

use super::FrameCtx;

/// Entrées du menu contextuel de la table, du Gantt et de la frise
/// (H-UI-79, H-UI-81) : ajouter, retirer.
pub const CONTEXT_LABELS: (TextId, TextId) = (TextId::TimelineAdd, TextId::TimelineRemove);

/// Clé du cache de la disposition : génération du document, empreinte du
/// style effectif, largeur arrondie au pixel, langue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimelineKey {
    /// Génération du document.
    pub generation: u64,
    /// Empreinte du style effectif.
    pub style: u64,
    /// Largeur, en pixels entiers.
    pub width_px: u32,
    /// Langue des textes.
    pub language: Language,
}

/// État transitoire de la frise (jamais persisté).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelineView {
    /// Dernière disposition et sa clé.
    pub cache: Option<(TimelineKey, TimelineLayout)>,
    /// Dispositions calculées depuis la création (tests du cache).
    pub layouts_computed: u64,
}

/// Dessine la frise en tête de la vue Gantt (habillage ruban, frise
/// visible, vue Gantt ou Gantt de suivi).
///
/// Lot L0 : rien n'est dessiné.
pub fn show(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, view: &mut TimelineView) {
    let _ = (ui, fc, view);
}
