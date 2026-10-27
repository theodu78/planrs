//! Géométrie de la chronologie (H-R16, ADR-0017 D7) : frise au-dessus de la
//! vue Gantt et en tête de la première page imprimée.
//!
//! Entrée ([`TimelineInput`]) : dates et textes fournis par le moteur via
//! `planrs_automation::timeline::input` (règle 3, aucune date calculée ici) ;
//! sortie ([`TimelineLayout`]) : formes en entiers de 1/16 px, sans egui,
//! peintes par l'interface, le SVG et le PDF (même principe que H-UI-42).
//!
//! Lot L0 (contrat) : types figés et [`layout`] réduit à la bande de fond ;
//! axe, graduations, barres, jalons et textes en L1 (H-R16, H-UI-80, P58).

use planrs_core::{Moment, TaskUid};

/// Hauteur de la frise, en 1/16 px (72 px).
pub const TIMELINE_HEIGHT16: i64 = 72 * 16;

/// Données de la frise, toutes venues du moteur (règle 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineInput {
    /// Début du projet (bord gauche de l'axe).
    pub start: Moment,
    /// Fin du projet (bord droit de l'axe).
    pub finish: Moment,
    /// Date d'état effective, si le projet en a une.
    pub status: Option<Moment>,
    /// Étiquette du début (« Début lun 19/10/26 »).
    pub start_label: String,
    /// Étiquette de la fin (« Fin mar 17/11/26 »).
    pub finish_label: String,
    /// Étiquette de la date d'état (« État 06/11 »).
    pub status_label: Option<String>,
    /// Graduations : instant et texte (« oct. »).
    pub ticks: Vec<(Moment, String)>,
    /// Tâches de la frise présentes dans le projet.
    pub items: Vec<TimelineItem>,
}

/// Tâche de la frise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineItem {
    /// UID de la tâche.
    pub uid: TaskUid,
    /// ID (ordre de la table), départage des rangées.
    pub id: u32,
    /// Barre ou jalon (dit par le moteur).
    pub kind: TimelineKind,
    /// Début planifié.
    pub start: Moment,
    /// Fin planifiée.
    pub finish: Moment,
    /// Nom de la tâche.
    pub name: String,
    /// Date courte (jalons : « 06/11 »).
    pub date_label: String,
}

/// Sorte d'élément de la frise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelineKind {
    /// Barre, du début à la fin.
    Bar,
    /// Losange sur l'axe.
    Milestone,
}

/// Frise disposée : formes dans l'ordre de dessin (H-UI-80).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineLayout {
    /// Hauteur, en 1/16 px ([`TIMELINE_HEIGHT16`]).
    pub height16: i64,
    /// Formes, dans l'ordre de dessin.
    pub shapes: Vec<TimelineShape>,
    /// Éléments valides non dessinés (au-delà de trois rangées), comptés
    /// « +n ».
    pub hidden: u32,
}

/// Forme de la frise ; rectangles `[x, y, largeur, hauteur]` en 1/16 px,
/// origine en haut à gauche de la frise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimelineShape {
    /// Fond de la frise.
    Band {
        /// Rectangle.
        rect16: [i64; 4],
    },
    /// Barre d'une tâche.
    Bar {
        /// Tâche.
        uid: TaskUid,
        /// Rangée (0 à 2).
        row: u8,
        /// Rectangle.
        rect16: [i64; 4],
    },
    /// Losange d'un jalon.
    Diamond {
        /// Tâche.
        uid: TaskUid,
        /// Centre.
        center16: (i64, i64),
        /// Demi-diagonale.
        half16: i64,
    },
    /// Axe horizontal.
    Axis {
        /// Ordonnée.
        y16: i64,
        /// Abscisses de début et de fin.
        x16: (i64, i64),
    },
    /// Graduation.
    Tick {
        /// Abscisse.
        x16: i64,
    },
    /// Date d'état (trait tireté).
    StatusLine {
        /// Abscisse.
        x16: i64,
    },
    /// Texte placé (ligne de base à `y16`).
    Text {
        /// Abscisse du bord gauche.
        x16: i64,
        /// Ordonnée de la ligne de base.
        y16: i64,
        /// Texte, déjà tronqué (« … », H-R11).
        text: String,
        /// Rôle (couleur, graisse).
        role: TimelineText,
        /// Tâche nommée, s'il y en a une.
        uid: Option<TaskUid>,
    },
}

/// Rôle d'un texte de la frise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelineText {
    /// Étiquette d'extrémité (début, fin).
    Bound,
    /// Graduation.
    Tick,
    /// Date d'état.
    Status,
    /// Nom dans (ou à droite de) une barre.
    BarName,
    /// Nom d'un jalon.
    MilestoneName,
    /// Date d'un jalon.
    MilestoneDate,
    /// Compteur « +n ».
    Overflow,
}

/// Dispose la frise sur `width16` (1/16 px, borné à `[0, i64::MAX / 2]`).
///
/// Lot L0 : bande de fond seule (aucun élément, aucun texte) ; H-R16 en L1.
#[must_use]
pub fn layout(input: &TimelineInput, width16: i64) -> TimelineLayout {
    let _ = input;
    let width16 = width16.clamp(0, i64::MAX / 2);
    TimelineLayout {
        height16: TIMELINE_HEIGHT16,
        shapes: vec![TimelineShape::Band {
            rect16: [0, 0, width16, TIMELINE_HEIGHT16],
        }],
        hidden: 0,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use planrs_core::Moment;

    use super::{TIMELINE_HEIGHT16, TimelineInput, TimelineShape, layout};

    fn input() -> TimelineInput {
        TimelineInput {
            start: Moment::from_minutes(0).unwrap(),
            finish: Moment::from_minutes(60).unwrap(),
            status: None,
            start_label: String::new(),
            finish_label: String::new(),
            status_label: None,
            ticks: Vec::new(),
            items: Vec::new(),
        }
    }

    /// Contrat L0 : bande seule, largeur bornée, jamais de panique.
    #[test]
    fn timeline_skeleton_band_only() {
        for width in [i64::MIN, -1, 0, 16_000, i64::MAX] {
            let out = layout(&input(), width);
            assert_eq!(out.height16, TIMELINE_HEIGHT16);
            assert_eq!(out.hidden, 0);
            assert!(matches!(
                out.shapes.as_slice(),
                [TimelineShape::Band { rect16: [0, 0, w, TIMELINE_HEIGHT16] }] if *w >= 0
            ));
        }
    }
}
