//! Vues : fonctions de dessin qui lisent un contexte d'image ([`FrameCtx`])
//! et poussent des `Action`, sans jamais modifier l'application.
//!
//! Chaque vue se teste seule avec un `FrameCtx` fabriqué à la main. Les vues
//! n'importent pas `app` : c'est `App` qui assemble l'image.

use std::ops::Range;

use planrs_core::{Severity, TaskUid};

use crate::action::Action;
use crate::app::OpenDocument;
use crate::i18n::Texts;
use crate::rows::RowModel;
use crate::selection::Selection;
use crate::state::UiState;
use crate::theme::Palette;

pub mod chrome;
pub mod diagnostics;
pub mod gantt;
pub mod main_view;
pub mod table;
pub mod timescale;

/// Hauteur fixe d'une ligne (table et Gantt), en points logiques.
pub const ROW_H: f32 = 22.0;

/// Hauteur de l'en-tête commun (deux niveaux d'échelle de temps).
pub const HEADER_H: f32 = 44.0;

/// Lignes dessinées en plus au-dessus et au-dessous de la fenêtre.
pub const OVERSCAN: usize = 2;

/// Mesures de la vue principale, en points logiques.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// Hauteur d'une ligne.
    pub row_h: f32,
    /// Hauteur de l'en-tête.
    pub header_h: f32,
    /// Marge horizontale du texte dans une cellule.
    pub pad_x: f32,
    /// Retrait du nom par niveau hiérarchique au-delà du premier.
    pub indent: f32,
    /// Largeur du séparateur table / Gantt.
    pub splitter_w: f32,
    /// Hauteur de la barre de défilement horizontale du Gantt.
    pub hbar_h: f32,
    /// Largeur réservée à droite du corps par la barre de défilement
    /// verticale pleine (`ScrollStyle::solid`) : le Gantt s'arrête avant.
    pub vbar_w: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            row_h: ROW_H,
            header_h: HEADER_H,
            pad_x: 4.0,
            indent: 14.0,
            splitter_w: 5.0,
            hbar_h: 10.0,
            vbar_w: egui::style::ScrollStyle::solid().allocated_width(),
        }
    }
}

/// État transitoire de la vue : jamais sérialisé.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewTransient {
    /// Défilement vertical à imposer une seule fois (restauration,
    /// `ScrollToRow`, tests).
    pub pending_scroll_y: Option<f32>,
    /// Défilement horizontal du Gantt, en pixels depuis le début de la plage.
    pub gantt_scroll_x: f32,
    /// Accumulateur du zoom à la molette (somme de `ln(zoom_delta)`).
    pub zoom_accum: f32,
    /// Ajuster le zoom à la prochaine image (après une ouverture).
    pub needs_fit: bool,
    /// Largeur du Gantt à l'image précédente : référence des ancres de
    /// zoom (centre du Gantt) et de la borne du défilement horizontal.
    pub last_gantt_width: f32,
    /// Abscisse écran du bord gauche du Gantt à l'image précédente (relevé
    /// pour les tests ; le réducteur ne travaille qu'en abscisses relatives
    /// au Gantt).
    pub last_gantt_left: f32,
}

/// Relevé d'une image (tests de synchronisation et de performance).
///
/// Chaque volet écrit son propre champ (`table::paint_body` pour
/// `table_rows`, `gantt::paint_body` pour `gantt_rows`) : comparer les deux
/// compare deux chemins de dessin distincts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    /// Lignes dessinées dans la table (écrit par `table::paint_body`).
    pub table_rows: Range<usize>,
    /// Lignes dessinées dans le Gantt (écrit par `gantt::paint_body`).
    pub gantt_rows: Range<usize>,
    /// Liens dessinés.
    pub links_painted: usize,
    /// Formes émises par la vue principale.
    pub shapes: usize,
}

/// Diagnostics du document prêts à afficher : triés par gravité et mis en
/// texte **une fois** par document et par langue (cache d'`App`), comptés
/// une fois. Aucune image ne refait ce travail proportionnel au nombre de
/// diagnostics (ADR-0011, décision 3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticsView {
    /// Gravité et texte affiché, la plus grave d'abord (ordre d'émission à
    /// gravité égale : diagnostics d'import, puis de calcul).
    pub entries: Vec<(Severity, String)>,
    /// Nombre d'erreurs.
    pub errors: usize,
    /// Nombre d'avertissements.
    pub warnings: usize,
}

/// Rang d'affichage d'une gravité (la plus grave d'abord).
const fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

/// Texte affichable d'un diagnostic : « → » (U+2192), absent des polices
/// d'egui, devient « -> » (voir `docs/ui-needs.md`, N5 : diagnostics non
/// traduits, mis en forme par le cœur).
fn displayable(text: &str) -> String {
    text.replace('\u{2192}', "->")
}

impl DiagnosticsView {
    /// Construit la vue des diagnostics d'import et de calcul d'un document
    /// (tri stable par gravité).
    #[must_use]
    pub fn build(doc: &OpenDocument) -> Self {
        let mut entries: Vec<(Severity, String)> = doc
            .import_diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.severity, displayable(&diagnostic.to_string())))
            .chain(
                doc.schedule()
                    .diagnostics
                    .iter()
                    .map(|diagnostic| (diagnostic.severity(), displayable(&diagnostic.to_string()))),
            )
            .collect();
        entries.sort_by_key(|(severity, _)| severity_rank(*severity));
        let count = |wanted: Severity| entries.iter().filter(|(severity, _)| *severity == wanted).count();
        let (errors, warnings) = (count(Severity::Error), count(Severity::Warning));
        Self {
            entries,
            errors,
            warnings,
        }
    }
}

/// Contexte d'une image : lectures partagées, file d'actions et relevé.
pub struct FrameCtx<'a> {
    /// Préférences d'interface.
    pub state: &'a UiState,
    /// Document ouvert, s'il y en a un.
    pub doc: Option<&'a OpenDocument>,
    /// Modèle de lignes du document (dans la langue courante).
    pub rows: Option<&'a RowModel>,
    /// Messages traduits.
    pub texts: &'a Texts,
    /// Couleurs du thème résolu.
    pub palette: &'a Palette,
    /// Mesures.
    pub metrics: &'a Metrics,
    /// Diagnostics du document, triés et mis en texte (vide sans document).
    pub diagnostics: &'a DiagnosticsView,
    /// Tâche sélectionnée.
    pub selection: Option<TaskUid>,
    /// Tâches sélectionnées (lot L14) ; `selection` en est la tâche active.
    pub selected: &'a Selection,
    /// File des actions de l'image.
    pub actions: &'a mut Vec<Action>,
    /// Relevé de l'image.
    pub stats: &'a mut FrameStats,
}
