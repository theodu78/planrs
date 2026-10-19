//! Intentions de l'utilisateur.
//!
//! Les vues ne modifient jamais l'application : elles poussent des [`Action`]
//! dans une file, appliquée par le réducteur unique d'`App` en fin d'image.
//! Une action sérialisée préfigure aussi le canal de contrôle (ADR-0002).

use planrs_automation::Batch;
use planrs_core::TaskUid;

use crate::selection::SelectMode;
use crate::state::{LanguagePref, ThemePref};
use crate::structure::StructureOp;
use crate::timescale::TimeScaleZoom;

/// Intention de l'utilisateur, appliquée en fin d'image. Pas `Copy` :
/// [`Action::Exec`] porte un lot de commandes (ADR-0012 D11).
#[derive(Clone, Debug, PartialEq)]
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
    /// Change le zoom de l'échelle de temps, ancré au centre du Gantt
    /// visible (la zone affichée reste à l'écran).
    SetZoom(TimeScaleZoom),
    /// Réapplique le zoom ajusté à la plage du document, ancré au centre du
    /// Gantt visible.
    FitZoom,
    /// Pas de zoom (Ctrl/Cmd + molette), qui garde sous le pointeur le même
    /// point du contenu ; arithmétique en pixels seulement.
    ZoomStep {
        /// Nombre de pas : positif vers le plus fin, négatif vers le plus
        /// grossier.
        steps: i8,
        /// Abscisse du pointeur **relative au bord gauche du Gantt**
        /// visible (0 = bord gauche), bornée par le réducteur à la largeur
        /// du Gantt ; une valeur non finie vaut 0.
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
    /// Applique un lot de commandes à la session du document (toute
    /// édition passe par le registre). Sans effet jusqu'au lot L11.
    Exec(Batch),
    /// Annule le dernier lot. Sans effet jusqu'au lot L11.
    Undo,
    /// Rétablit le dernier lot annulé. Sans effet jusqu'au lot L11.
    Redo,
    /// Crée un projet vide (« Projet1 »), après confirmation si le
    /// document courant est modifié.
    NewProject,
    /// Enregistre sur le fichier du document, ou demande un chemin s'il
    /// n'en a pas.
    Save,
    /// Enregistre sous un autre nom (boîte de dialogue ou téléchargement).
    SaveAs,
    /// Réponse au bandeau « Enregistrer les modifications ? ».
    Confirm(ConfirmChoice),
    /// Clic de sélection avec modificateur (Ctrl/Cmd : ajout ou retrait ;
    /// Maj : plage), lot L14.
    SelectWith {
        /// Tâche cliquée.
        uid: TaskUid,
        /// Mode du clic.
        mode: SelectMode,
    },
    /// Commande de structure sur la sélection (Inser, Suppr, indentation,
    /// liens), lot L14.
    Structure(StructureOp),
    /// Réponse à la confirmation d'une suppression de récapitulative.
    ConfirmDelete(bool),
}

/// Réponse au bandeau de confirmation d'un document modifié (fermer, ouvrir
/// un autre document, créer un projet, quitter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmChoice {
    /// Enregistrer, puis poursuivre si l'enregistrement réussit.
    Save,
    /// Poursuivre sans enregistrer : les modifications sont perdues.
    Discard,
    /// Renoncer : le document reste ouvert, rien n'est perdu.
    Cancel,
}
