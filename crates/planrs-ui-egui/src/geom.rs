//! Géométrie pure de la vue principale : conversions numériques, plage de
//! lignes visibles, mise en page.
//!
//! Toutes les conversions entre entiers et `f32` sont regroupées ici, pour
//! que les lints pedantic `cast_*` ne soient levées qu'à un seul endroit, avec
//! leur raison. Aucune conversion ne produit une date à partir d'un pixel.

use std::ops::Range;

use egui::{Rect, pos2};

use crate::view::Metrics;

/// Indice de ligne → pixels.
#[expect(
    clippy::cast_precision_loss,
    reason = "indice de ligne → pixels ; exact sous 2^24 lignes"
)]
#[must_use]
pub fn index_to_f32(i: usize) -> f32 {
    i as f32
}

/// Pixels → indice de ligne : 0 pour NaN ou une valeur négative, saturé
/// au-delà de `usize::MAX`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "`as` sature et donne 0 pour NaN ; négatifs filtrés"
)]
#[must_use]
pub fn f32_to_index(v: f32) -> usize {
    if v.is_nan() || v <= 0.0 { 0 } else { v as usize }
}

/// Minutes (écart déjà borné par l'appelant) → `f32`, pour le dessin.
#[expect(
    clippy::cast_precision_loss,
    reason = "dessin : écart borné à 20 ans ≈ 10,5 M min < 2^24, exact en f32"
)]
#[must_use]
pub fn i64_to_f32(v: i64) -> f32 {
    v as f32
}

/// Lignes qui coupent `[top, bottom)` (coordonnées de contenu), élargies de
/// `overscan`. Jamais de panique : résultat inclus dans `0..len`, avec
/// `start ≤ end` ; plage vide pour NaN, infinis ou hauteur nulle.
#[must_use]
pub fn visible_rows(top: f32, bottom: f32, row_h: f32, len: usize, overscan: usize) -> Range<usize> {
    if len == 0 || row_h.is_nan() || row_h <= 0.0 || !top.is_finite() || !bottom.is_finite() || bottom <= top
    {
        return 0..0;
    }
    let first = f32_to_index((top.max(0.0) / row_h).floor()).min(len);
    let end = f32_to_index((bottom / row_h).ceil()).min(len);
    let start = first.saturating_sub(overscan);
    let end = end.saturating_add(overscan).min(len);
    start.min(end)..end
}

/// Rectangles de la vue principale (coordonnées écran).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MainLayout {
    /// En-tête de la table (titres de colonnes).
    pub table_header: Rect,
    /// En-tête du Gantt (échelle de temps à deux niveaux).
    pub gantt_header: Rect,
    /// Séparateur déplaçable, pleine hauteur.
    pub splitter: Rect,
    /// Rectangle donné à la `ScrollArea` verticale unique.
    pub body: Rect,
    /// Découpe du corps de la table.
    pub table_clip: Rect,
    /// Découpe du corps du Gantt.
    pub gantt_clip: Rect,
    /// Barre de défilement horizontale du Gantt.
    pub gantt_hbar: Rect,
}

/// Mise en page de la vue principale dans `avail`, table de largeur
/// `table_width` (bornée pour laisser de la place au Gantt).
#[must_use]
pub fn main_layout(avail: Rect, table_width: f32, m: &Metrics) -> MainLayout {
    let max_table = (avail.width() - m.splitter_w).max(0.0);
    let table_w = if table_width.is_finite() {
        table_width.max(0.0).min(max_table)
    } else {
        0.0
    };
    let split_left = avail.left() + table_w;
    let split_right = (split_left + m.splitter_w).min(avail.right());
    let header_bottom = (avail.top() + m.header_h).min(avail.bottom());
    let hbar_top = (avail.bottom() - m.hbar_h).max(header_bottom);
    let table_header = Rect::from_min_max(avail.min, pos2(split_left, header_bottom));
    let gantt_header = Rect::from_min_max(pos2(split_right, avail.top()), pos2(avail.right(), header_bottom));
    let splitter = Rect::from_min_max(pos2(split_left, avail.top()), pos2(split_right, avail.bottom()));
    let body = Rect::from_min_max(pos2(avail.left(), header_bottom), pos2(avail.right(), hbar_top));
    let table_clip = Rect::from_min_max(body.min, pos2(split_left, body.bottom()));
    let gantt_clip = Rect::from_min_max(pos2(split_right, body.top()), body.max);
    let gantt_hbar = Rect::from_min_max(pos2(split_right, hbar_top), avail.max);
    MainLayout {
        table_header,
        gantt_header,
        splitter,
        body,
        table_clip,
        gantt_clip,
        gantt_hbar,
    }
}
