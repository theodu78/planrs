//! Corps du Gantt : grille, barres, jalons, récapitulatives, chemin
//! critique, date d'état et liens.
//!
//! Ordre des couches : fond → grille → sélection → date d'état → liens →
//! barres. Le tracé d'un lien ([`link_path`]) est une fonction pure, testée
//! sans egui.

use std::ops::Range;

use egui::{Pos2, Rect, pos2};
use planrs_core::LinkType;

use super::FrameCtx;
use crate::timescale::TimeAxis;

/// Extrémités écran d'une barre (ou d'un jalon : `start_x == finish_x`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarEnds {
    /// Abscisse du début.
    pub start_x: f32,
    /// Abscisse de la fin.
    pub finish_x: f32,
    /// Ordonnée du centre vertical de la ligne.
    pub y: f32,
}

/// Mesures du tracé des liens, en points logiques.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkMetrics {
    /// Longueur du segment horizontal qui sort d'une barre.
    pub stub: f32,
    /// Taille de la pointe de flèche.
    pub arrow: f32,
    /// Hauteur d'une ligne.
    pub row_h: f32,
    /// Demi-hauteur d'une barre.
    pub bar_half_h: f32,
}

impl Default for LinkMetrics {
    fn default() -> Self {
        Self {
            stub: 6.0,
            arrow: 4.0,
            row_h: super::ROW_H,
            bar_half_h: 5.0,
        }
    }
}

/// Direction de la pointe de flèche (vers la barre du successeur).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArrowDir {
    /// Vers la droite (entrée par le début).
    #[default]
    Right,
    /// Vers la gauche (entrée par la fin).
    Left,
    /// Vers le bas (entrée par le dessus).
    Down,
    /// Vers le haut (entrée par le dessous).
    Up,
}

/// Tracé d'un lien : polyligne d'au plus 6 points et pointe de flèche.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkPath {
    /// Points de la polyligne ; seuls les `len` premiers comptent.
    pub points: [Pos2; 6],
    /// Nombre de points utiles (0 à 6).
    pub len: usize,
    /// Pointe de la flèche.
    pub arrow_tip: Pos2,
    /// Direction de la flèche.
    pub arrow_dir: ArrowDir,
}

impl LinkPath {
    /// Points utiles de la polyligne.
    #[must_use]
    pub fn points(&self) -> &[Pos2] {
        self.points.get(..self.len).unwrap_or(&[])
    }
}

/// Tracé d'un lien de type `kind` du prédécesseur `p` au successeur `s`
/// (cas direct et cas replié quand le successeur commence avant).
///
/// Corps provisoire : tracé vide, flèche au début du successeur.
#[must_use]
pub fn link_path(_kind: LinkType, _p: BarEnds, s: BarEnds, _m: &LinkMetrics) -> LinkPath {
    let tip = pos2(s.start_x, s.y);
    LinkPath {
        points: [tip; 6],
        len: 0,
        arrow_tip: tip,
        arrow_dir: ArrowDir::Right,
    }
}

/// Corps du Gantt : lignes `range`, `origin` étant le coin haut-gauche écran
/// de la ligne 0, découpe `clip`, projection `axis`.
///
/// Corps provisoire : rien n'est dessiné.
pub fn paint_body(
    _ui: &mut egui::Ui,
    _fc: &mut FrameCtx<'_>,
    _axis: &TimeAxis,
    _origin: Pos2,
    _range: Range<usize>,
    _clip: Rect,
) {
}
