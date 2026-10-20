//! Intentions de l'utilisateur.
//!
//! Les vues ne modifient jamais l'application : elles poussent des [`Action`]
//! dans une file, appliquée par le réducteur unique d'`App` en fin d'image.
//! Une action sérialisée préfigure aussi le canal de contrôle (ADR-0002).

use planrs_automation::Batch;
use planrs_core::TaskUid;

use crate::column::Column;
use crate::nav::NavMove;
use crate::selection::SelectMode;
use crate::state::{GanttStyle, LanguagePref, ThemePref, ViewKind};
use crate::structure::StructureOp;
use crate::timescale::TimeScaleZoom;
use crate::view::Pane;

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
    /// Comme [`Action::Exec`], pour le document de numéro d'ouverture
    /// `opening` seulement (saisie de cellule) : abandonné si un autre
    /// document l'a remplacé entre-temps (relecture finale J4, F2).
    ExecOn {
        /// Numéro d'ouverture du document visé (`OpenDocument::opening`).
        opening: u64,
        /// Lot de commandes.
        batch: Batch,
    },
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
    /// Quitte l'application (desktop : Ctrl/Cmd+Q, menu Fichier), après
    /// confirmation si le document est modifié ; sans effet sur le web.
    Quit,
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
    // --- J7 (contrat L0, ADR-0015) : sans effet tant que leur lot ne les
    // câble pas (réducteur d'`App`).
    /// Déplacement au clavier dans la grille (H-UI-13 à H-UI-16, lot L1) ;
    /// `extend` : étend la sélection de lignes depuis l'ancre.
    Nav {
        /// Déplacement.
        mv: NavMove,
        /// Extension de la sélection (Maj).
        extend: bool,
    },
    /// Donne le focus clavier à un volet (lot L1).
    FocusPane(Pane),
    /// Change la vue principale (lots L10, L11).
    SetView(ViewKind),
    /// Change le style du Gantt (lot L7).
    SetGanttStyle(GanttStyle),
    /// Ouvre une fenêtre (lots L8, L9, L12, L13).
    ShowDialog(DialogKind),
    /// « Informations sur la tâche » de la tâche active (Maj+F2, H-UI-19,
    /// lot L9) : la tâche est résolue par le réducteur.
    ShowTaskInformation,
    /// Ferme la fenêtre ouverte (Échap, Annuler).
    CloseDialog,
    /// Affiche ou masque une colonne (lot L6 ; ID et Nom restent visibles).
    ToggleColumn(Column),
    /// Largeur d'une colonne (bord de l'en-tête glissé, lot L6), bornée par
    /// `UiState::normalized`.
    SetColumnWidth {
        /// Colonne.
        column: Column,
        /// Largeur en points logiques.
        width: u16,
    },
    /// Défilement horizontal de la table, delta en points (lot L6).
    ScrollTableX(f32),
    /// Alt+←/→ : une graduation fine de l'échelle (H-UI-18, lot L1).
    TimescaleStep(i8),
    /// Alt+Page haut/bas : une largeur visible du Gantt (H-UI-18, lot L1).
    TimescalePage(i8),
    /// Alt+Origine : début du projet (H-UI-18, lot L1).
    TimescaleToStart,
    /// Alt+Fin : fin du projet (H-UI-18, lot L1).
    TimescaleToEnd,
    /// Zoom au pas voisin, ancré au centre du Gantt (positif : plus fin ;
    /// H-UI-18, lot L1).
    ZoomBy(i8),
    /// Accepte l'écriture relayée `id` (bandeau, H-UI-27, lot L3).
    Approve(u64),
    /// Refuse l'écriture relayée `id` (lot L3).
    Reject(u64),
    /// Ouvre le fichier récent d'indice donné (H-UI-28, lot L14).
    OpenRecent(usize),
    /// Réponse au bandeau de reprise après plantage (lot L15).
    Recover(RecoverChoice),
    /// Action d'interface demandée par un agent (`app.ui`, lot L3).
    Ui(UiOp),
}

/// Fenêtre à ouvrir (J7, ADR-0015 D6 ; `Action::OpenDialog` reste le
/// sélecteur de fichiers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogKind {
    /// « Informations sur la tâche » (lot L9).
    TaskInformation(TaskUid),
    /// Boîte d'un lien (H-UI-21, lot L8).
    Link {
        /// Prédécesseur.
        predecessor: TaskUid,
        /// Successeur.
        successor: TaskUid,
    },
    /// « Informations sur le projet » (lot L12).
    ProjectInformation,
    /// « Modifier le temps de travail » (lot L12).
    WorkingTime,
    /// « Ligne de base » : définir ou effacer (lot L12).
    Baseline,
    /// « Replanifier le reste » (lot L12).
    Reschedule,
    /// « Affecter des ressources » (lot L13).
    AssignResources,
}

/// Action d'interface d'un agent (`app.ui`, J7) : sans historique, sans
/// effet sur le projet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiOp {
    /// Sélectionne la tâche d'UID donné (ou aucune).
    Select(Option<u32>),
    /// Fait défiler jusqu'à la tâche d'UID donné.
    ScrollTo(u32),
    /// Zoom de l'échelle de temps.
    Zoom(TimeScaleZoom),
    /// Vue principale.
    View(ViewKind),
    /// Thème.
    Theme(ThemePref),
    /// Langue.
    Language(LanguagePref),
}

/// Réponse au bandeau de reprise après plantage (J7, lot L15).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoverChoice {
    /// Restaure l'orphelin d'identifiant donné.
    Restore(String),
    /// Jette l'orphelin d'identifiant donné.
    Discard(String),
    /// Décide plus tard (le bandeau revient au prochain démarrage).
    Later,
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
