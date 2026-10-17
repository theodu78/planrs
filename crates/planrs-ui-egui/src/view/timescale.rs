//! En-tête à deux niveaux de l'échelle de temps (au-dessus du Gantt).
//!
//! Bande haute : unité grossière du zoom ; bande basse : unité fine. Les
//! cellules viennent de [`tier_cells`] (culling à la largeur visible), les
//! étiquettes sont centrées dans la partie visible de leur cellule et
//! tronquées par egui (`LayoutJob`).

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, FontId, Rect, Shape, Stroke, pos2};
use planrs_core::Weekday;

use super::FrameCtx;
use crate::timescale::{TierCell, TimeAxis, tier_cells};

/// Taille de police des étiquettes de l'en-tête.
const HEADER_FONT_PX: f32 = 12.0;

/// Marge horizontale d'une étiquette dans sa cellule.
const LABEL_PAD: f32 = 2.0;

/// Largeur visible minimale pour dessiner une étiquette (sinon une cellule
/// partielle au bord n'afficherait que « … »).
const MIN_LABEL_W: f32 = 24.0;

/// Dessine les deux niveaux de l'en-tête dans `rect`, selon `axis`.
pub fn paint_header(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, axis: &TimeAxis, rect: Rect) {
    if !rect.is_finite() || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter_at(rect);
    let palette = fc.palette;
    let week_start = fc
        .doc
        .map_or(Weekday::Monday, |doc| doc.project.options.week_start_day);
    let (top_unit, bottom_unit) = fc.state.zoom.tiers();
    let labels = fc.texts.format_labels();
    let mid_y = rect.center().y;
    let top = Rect::from_min_max(rect.min, pos2(rect.right(), mid_y));
    let bottom = Rect::from_min_max(pos2(rect.left(), mid_y), rect.max);

    let mut shapes = vec![Shape::rect_filled(rect, 0.0, palette.header_bg)];
    let bands = [
        (top, top_unit, palette.grid_major),
        (bottom, bottom_unit, palette.grid),
    ];
    let mut texts = Vec::new();
    for (band, unit, line_color) in bands {
        let cells = tier_cells(axis, unit, week_start, (band.left(), band.right()), &labels);
        band_shapes(band, &cells, line_color, &mut shapes);
        collect_labels(band, cells, &mut texts);
    }
    // Séparation des deux bandes et bord inférieur de l'en-tête.
    let stroke = Stroke::new(1.0, palette.grid_major);
    shapes.push(Shape::line_segment(
        [pos2(rect.left(), mid_y), pos2(rect.right(), mid_y)],
        stroke,
    ));
    let bottom_y = rect.bottom() - 0.5;
    shapes.push(Shape::line_segment(
        [pos2(rect.left(), bottom_y), pos2(rect.right(), bottom_y)],
        stroke,
    ));

    let text_color = palette.text;
    for (band, x0, x1, label) in texts {
        let width = x1 - x0 - 2.0 * LABEL_PAD;
        if !width.is_finite() || width < MIN_LABEL_W {
            continue;
        }
        let mut job = LayoutJob::single_section(
            label,
            TextFormat::simple(FontId::proportional(HEADER_FONT_PX), Color32::PLACEHOLDER),
        );
        job.wrap = TextWrapping::truncate_at_width(width);
        let galley = painter.layout_job(job);
        let size = galley.size();
        let x = f32::midpoint(x0, x1) - size.x / 2.0;
        let y = band.center().y - size.y / 2.0;
        if x.is_finite() && y.is_finite() && !galley.is_empty() {
            shapes.push(Shape::galley(pos2(x, y), galley, text_color));
        }
    }
    fc.stats.shapes = fc.stats.shapes.saturating_add(shapes.len());
    painter.extend(shapes);
}

/// Séparateurs verticaux au bord gauche de chaque cellule visible.
fn band_shapes(band: Rect, cells: &[TierCell], color: Color32, shapes: &mut Vec<Shape>) {
    let stroke = Stroke::new(1.0, color);
    for cell in cells {
        let x = cell.x0.round() + 0.5;
        if x.is_finite() && x > band.left() && x < band.right() {
            shapes.push(Shape::line_segment(
                [pos2(x, band.top()), pos2(x, band.bottom())],
                stroke,
            ));
        }
    }
}

/// Étiquettes à dessiner : partie visible de chaque cellule (étiquette
/// « collante » d'une cellule partiellement visible).
fn collect_labels(band: Rect, cells: Vec<TierCell>, out: &mut Vec<(Rect, f32, f32, String)>) {
    for cell in cells {
        if cell.label.is_empty() {
            continue;
        }
        let x0 = cell.x0.max(band.left());
        let x1 = cell.x1.min(band.right());
        if x0.is_finite() && x1.is_finite() && x1 > x0 {
            out.push((band, x0, x1, cell.label));
        }
    }
}
