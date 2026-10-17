//! Intentions de l'utilisateur.
//!
//! Les vues ne modifient jamais l'application : elles poussent des [`Action`]
//! dans une file, appliquée par le réducteur unique d'`App` en fin d'image.
//! Une action sérialisée préfigure aussi le canal de contrôle (ADR-0002).

use planrs_core::TaskUid;

use crate::state::{LanguagePref, ThemePref};
use crate::timescale::TimeScaleZoom;

/// Intention de l'utilisateur, appliquée en fin d'image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// Ouvre le projet d'exemple embarqué.
    OpenDemo,
    /// Ouvre le sélecteur de fichiers (rfd natif ou téléversement web).
    OpenDialog,
    /// Ferme le document courant.
    CloseDocument,
    /// Sélectionne une tâche (ou aucune).
    Select(Option<TaskUid>),
    /// Change la préférence de langue.
    SetLanguage(LanguagePref),
    /// Change la préférence de thème.
    SetTheme(ThemePref),
    /// Change le zoom de l'échelle de temps.
    SetZoom(TimeScaleZoom),
    /// Réapplique le zoom ajusté à la plage du document.
    FitZoom,
    /// Pas de zoom (Ctrl/Cmd + molette), ancré à l'abscisse écran
    /// `anchor_x` ; arithmétique en pixels seulement.
    ZoomStep {
        /// Nombre de pas : positif vers le plus fin, négatif vers le plus
        /// grossier.
        steps: i8,
        /// Abscisse écran du pointeur.
        anchor_x: f32,
    },
    /// Largeur de la table (séparateur déplacé), en points logiques.
    SetTableWidth(u16),
    /// Défilement horizontal du Gantt, delta en points.
    ScrollGanttX(f32),
    /// Fait défiler jusqu'à la ligne d'indice donné.
    ScrollToRow(usize),
    /// Ouvre ou referme le panneau des diagnostics.
    ToggleDiagnostics,
    /// Referme le bandeau de messages.
    DismissMessage,
}
