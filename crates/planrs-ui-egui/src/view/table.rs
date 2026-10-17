//! Table des tâches dessinée au `Painter` (en-tête et corps virtualisé).
//!
//! Seules les lignes de la plage visible sont dessinées ; chaque ligne déclare
//! un nœud d'accessibilité (`row-label`) pour être retrouvée par les tests.
//! Les textes viennent tout formatés du `RowModel` : aucune mise en forme ni
//! aucun parcours proportionnel au nombre de tâches pendant l'image. Chaque
//! cellule est une galley du cache d'egui (`LayoutJob` tronqué par « … »),
//! colorée au dessin (`Color32::PLACEHOLDER`), donc partagée entre les thèmes.

use std::ops::Range;
use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, FontId, Galley, Painter, Pos2, Rect, Sense, Stroke, pos2, vec2};

use super::FrameCtx;
use crate::action::Action;
use crate::column::{CellAlign, Column};
use crate::geom::index_to_f32;
use crate::i18n::{Arg, TextId};
use crate::rows::{Row, RowKind, RowModel};
use crate::theme::Palette;

/// Largeur en dessous de laquelle une cellule n'affiche pas de texte.
const MIN_TEXT_WIDTH: f32 = 4.0;

/// Décalage du second tracé qui simule le gras des récapitulatives.
const FAUX_BOLD_DX: f32 = 0.6;

/// Police des cellules et des titres.
fn cell_font(ui: &egui::Ui) -> FontId {
    egui::TextStyle::Body.resolve(ui.style())
}

/// Galley d'un texte tronqué à `max_w` (« … »), couleur fixée au dessin.
fn truncated_galley(painter: &Painter, text: &str, font: FontId, max_w: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), TextFormat::simple(font, Color32::PLACEHOLDER));
    job.wrap = TextWrapping::truncate_at_width(max_w);
    painter.layout_job(job)
}

/// Abscisse d'un texte de largeur `text_w` dans la cellule `[left, left + w)`.
fn aligned_x(align: CellAlign, left: f32, w: f32, pad: f32, text_w: f32) -> f32 {
    match align {
        CellAlign::Left => left + pad,
        CellAlign::Right => (left + w - pad - text_w).max(left + pad),
        CellAlign::Center => (left + (w - text_w) * 0.5).max(left + pad),
    }
}

/// Couleur du texte d'une ligne.
fn row_text_color(row: &Row, palette: &Palette) -> Color32 {
    match row.kind {
        RowKind::Null | RowKind::Excluded => palette.text_weak,
        _ if row.critical => palette.critical,
        RowKind::Summary => palette.text_strong,
        RowKind::Leaf | RowKind::Milestone => palette.text,
    }
}

/// En-tête de la table : fond, titres des colonnes visibles (centrés,
/// tronqués), séparateurs verticaux et trait bas.
pub fn paint_header(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, rect: Rect) {
    if !rect.is_finite() || !rect.is_positive() {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let palette = fc.palette;
    let pad = fc.metrics.pad_x;
    let font = cell_font(ui);
    let mut shapes = 0_usize;
    painter.rect_filled(rect, 0.0, palette.header_bg);
    shapes = shapes.saturating_add(1);
    let mut left = rect.left();
    for state in fc.state.visible_columns() {
        if left >= rect.right() {
            break;
        }
        let w = f32::from(state.width);
        let right = left + w;
        let max_w = w - 2.0 * pad;
        if max_w >= MIN_TEXT_WIDTH {
            let title = fc.texts.get(state.column.title());
            let galley = truncated_galley(&painter, &title, font.clone(), max_w);
            let size = galley.size();
            let x = aligned_x(CellAlign::Center, left, w, pad, size.x);
            let y = rect.center().y - size.y * 0.5;
            painter.galley(pos2(x, y), galley, palette.text_strong);
            shapes = shapes.saturating_add(1);
        }
        painter.vline(right - 0.5, rect.y_range(), Stroke::new(1.0, palette.grid_major));
        shapes = shapes.saturating_add(1);
        left = right;
    }
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, palette.grid_major),
    );
    shapes = shapes.saturating_add(1);
    fc.stats.shapes = fc.stats.shapes.saturating_add(shapes);
}

/// Textes des cellules visibles d'une ligne (nom indenté, troncature,
/// alignement de la colonne) ; rend le nombre de formes ajoutées.
fn paint_cells(
    painter: &Painter,
    fc: &FrameCtx<'_>,
    rows: &RowModel,
    i: usize,
    row_rect: Rect,
    font: &FontId,
) -> usize {
    let Some(row) = rows.row(i) else { return 0 };
    let metrics = fc.metrics;
    let pad = metrics.pad_x;
    let color = row_text_color(row, fc.palette);
    let right = painter.clip_rect().right();
    let mut shapes = 0_usize;
    let mut x = row_rect.left();
    for state in fc.state.visible_columns() {
        if x >= right {
            break;
        }
        let w = f32::from(state.width);
        let cell_left = x;
        x += w;
        let text = rows.text(i, state.column);
        if text.is_empty() {
            continue;
        }
        let indent = if state.column == Column::Name {
            metrics.indent * f32::from(row.level.saturating_sub(1))
        } else {
            0.0
        };
        let max_w = w - 2.0 * pad - indent;
        if !max_w.is_finite() || max_w < MIN_TEXT_WIDTH {
            continue;
        }
        let galley = truncated_galley(painter, text, font.clone(), max_w);
        let size = galley.size();
        let text_x = aligned_x(state.column.align(), cell_left + indent, w - indent, pad, size.x);
        let text_y = row_rect.top() + (row_rect.height() - size.y) * 0.5;
        if row.kind == RowKind::Summary {
            // Gras simulé : la police par défaut n'a pas de graisse grasse.
            painter.galley(pos2(text_x + FAUX_BOLD_DX, text_y), Arc::clone(&galley), color);
            shapes = shapes.saturating_add(1);
        }
        painter.galley(pos2(text_x, text_y), galley, color);
        shapes = shapes.saturating_add(1);
    }
    shapes
}

/// Corps de la table : lignes `range`, `origin` étant le coin haut-gauche
/// écran de la ligne 0, découpe `clip`.
///
/// Fond alterné léger, ligne sélectionnée surlignée, texte rouge pour une
/// ligne critique, nom indenté par niveau, récapitulative en texte appuyé,
/// ligne vide sans texte. Chaque ligne est une zone cliquable
/// (`Action::Select`) qui porte le libellé d'accessibilité `row-label`.
pub fn paint_body(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, origin: Pos2, range: Range<usize>, clip: Rect) {
    let clip = clip.intersect(ui.clip_rect());
    if !clip.is_finite() || !clip.is_positive() || !origin.x.is_finite() || !origin.y.is_finite() {
        return;
    }
    let painter = ui.painter().with_clip_rect(clip);
    let palette = fc.palette;
    let metrics = fc.metrics;
    let row_h = metrics.row_h;
    let font = cell_font(ui);
    let mut shapes = 0_usize;

    painter.rect_filled(clip, 0.0, palette.background);
    shapes = shapes.saturating_add(1);
    let zebra = palette.background.lerp_to_gamma(palette.header_bg, 0.6);

    let Some(rows) = fc.rows else {
        fc.stats.shapes = fc.stats.shapes.saturating_add(shapes);
        return;
    };
    let left = clip.left();
    for i in range {
        let Some(row) = rows.row(i) else { continue };
        let top = origin.y + index_to_f32(i) * row_h;
        let row_rect = Rect::from_min_size(pos2(left, top), vec2(clip.width(), row_h));
        if !row_rect.is_finite() {
            continue;
        }
        let selected = fc.selection == Some(row.uid);
        if selected {
            painter.rect_filled(row_rect, 0.0, palette.selection);
            shapes = shapes.saturating_add(1);
        } else if !i.is_multiple_of(2) {
            painter.rect_filled(row_rect, 0.0, zebra);
            shapes = shapes.saturating_add(1);
        }
        painter.hline(
            row_rect.x_range(),
            row_rect.bottom() - 0.5,
            Stroke::new(1.0, palette.grid),
        );
        shapes = shapes.saturating_add(1);

        if row.kind != RowKind::Null {
            shapes = shapes.saturating_add(paint_cells(&painter, fc, rows, i, row_rect, &font));
        }

        let hit = row_rect.intersect(clip);
        if hit.is_positive() {
            let response = ui.interact(
                hit,
                ui.make_persistent_id(("planrs-table-row", i)),
                Sense::click(),
            );
            let texts = fc.texts;
            let id = row.id;
            let name = rows.text(i, Column::Name);
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::Role::Row,
                    true,
                    selected,
                    texts.get_args(
                        TextId::RowLabel,
                        &[("id", Arg::Count(i64::from(id))), ("name", Arg::Str(name.into()))],
                    ),
                )
            });
            if response.clicked() {
                fc.actions.push(Action::Select(Some(row.uid)));
            }
        }
    }

    // Séparateurs verticaux de colonnes, sur toute la hauteur visible.
    let mut x = left;
    for state in fc.state.visible_columns() {
        x += f32::from(state.width);
        if x > clip.right() {
            break;
        }
        painter.vline(x - 0.5, clip.y_range(), Stroke::new(1.0, palette.grid));
        shapes = shapes.saturating_add(1);
    }
    fc.stats.shapes = fc.stats.shapes.saturating_add(shapes);
}
