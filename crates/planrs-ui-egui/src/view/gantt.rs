//! Corps du Gantt : grille, barres, jalons, récapitulatives, chemin
//! critique, date d'état et liens.
//!
//! Ordre des couches : fond → grille → sélection → date d'état → liens →
//! barres. Le tracé d'un lien ([`link_path`]) est une fonction pure, testée
//! sans egui.

use std::ops::Range;

use egui::{Pos2, Rect, Shape, Stroke, StrokeKind, pos2};
use planrs_core::{LinkType, Weekday};

use super::FrameCtx;
use crate::geom::index_to_f32;
use crate::rows::{Row, RowKind, RowModel};
use crate::theme::Palette;
use crate::timescale::{TimeAxis, tier_spans};

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

/// Tracé vide (entrée non finie) : aucun point, flèche sans effet.
fn empty_path(tip: Pos2) -> LinkPath {
    LinkPath {
        points: [tip; 6],
        len: 0,
        arrow_tip: tip,
        arrow_dir: ArrowDir::Right,
    }
}

/// Construit un tracé à partir de ses points (au plus 6), la flèche étant au
/// dernier point.
fn path_of(points: &[Pos2], dir: ArrowDir) -> LinkPath {
    let tip = points.last().copied().unwrap_or(Pos2::ZERO);
    let mut out = [tip; 6];
    for (slot, p) in out.iter_mut().zip(points) {
        *slot = *p;
    }
    let len = points.len().min(out.len());
    LinkPath {
        points: out,
        len,
        arrow_tip: tip,
        arrow_dir: dir,
    }
}

/// Tracé d'un lien de type `kind` du prédécesseur `p` au successeur `s`
/// (cas direct et cas replié quand le successeur commence avant).
///
/// Polyligne orthogonale (hypothèse d'affichage H-UI, d'après les captures
/// de l'aide Microsoft, non vérifiée au pixel) :
/// - FS : part de la fin de `p` ; direct (successeur assez à droite) :
///   horizontale puis verticale qui entre par le dessus (ou le dessous si le
///   successeur est au-dessus) près du début de `s` ; replié : sortie `stub`,
///   passage dans l'intercolonne sous `p`, retour à gauche de `s`, entrée par
///   le début ;
/// - SS : du début de `p` vers la gauche, verticale, entrée par le début de
///   `s` ;
/// - FF : de la fin de `p` vers la droite, verticale, entrée par la fin de
///   `s` (flèche vers la gauche) ;
/// - SF : du début de `p` vers la gauche ; direct si `s` finit assez à
///   gauche, sinon replié dans l'intercolonne ; entrée par la fin de `s`.
///
/// Un jalon se passe en donnant les extrémités de son losange. Une entrée
/// non finie donne un tracé vide.
#[must_use]
pub fn link_path(kind: LinkType, p: BarEnds, s: BarEnds, m: &LinkMetrics) -> LinkPath {
    let values = [
        p.start_x,
        p.finish_x,
        p.y,
        s.start_x,
        s.finish_x,
        s.y,
        m.stub,
        m.row_h,
        m.bar_half_h,
    ];
    if values.iter().any(|v| !v.is_finite()) {
        return empty_path(pos2(0.0, 0.0));
    }
    let (p0, p1, yp) = (p.start_x, p.finish_x.max(p.start_x), p.y);
    let (s0, s1, ys) = (s.start_x, s.finish_x.max(s.start_x), s.y);
    let stub = m.stub.max(0.0);
    let below = ys >= yp;
    let dir = if below { 1.0 } else { -1.0 };
    // Frontière de ligne côté successeur, juste sous (ou sur) le prédécesseur.
    let ymid = yp + dir * m.row_h / 2.0;
    match kind {
        LinkType::FinishToStart => {
            let xa = s0 + stub.min((s1 - s0) / 2.0);
            if xa >= p1 + stub / 2.0 {
                let edge = ys - dir * m.bar_half_h;
                let arrow = if below { ArrowDir::Down } else { ArrowDir::Up };
                path_of(&[pos2(p1, yp), pos2(xa, yp), pos2(xa, edge)], arrow)
            } else {
                let xo = p1 + stub;
                let xi = s0 - stub;
                path_of(
                    &[
                        pos2(p1, yp),
                        pos2(xo, yp),
                        pos2(xo, ymid),
                        pos2(xi, ymid),
                        pos2(xi, ys),
                        pos2(s0, ys),
                    ],
                    ArrowDir::Right,
                )
            }
        }
        LinkType::StartToStart => {
            let xl = p0.min(s0) - stub;
            path_of(
                &[pos2(p0, yp), pos2(xl, yp), pos2(xl, ys), pos2(s0, ys)],
                ArrowDir::Right,
            )
        }
        LinkType::FinishToFinish => {
            let xr = p1.max(s1) + stub;
            path_of(
                &[pos2(p1, yp), pos2(xr, yp), pos2(xr, ys), pos2(s1, ys)],
                ArrowDir::Left,
            )
        }
        LinkType::StartToFinish => {
            let xl = p0 - stub;
            if s1 + stub <= xl {
                path_of(
                    &[pos2(p0, yp), pos2(xl, yp), pos2(xl, ys), pos2(s1, ys)],
                    ArrowDir::Left,
                )
            } else {
                let xr = s1 + stub;
                path_of(
                    &[
                        pos2(p0, yp),
                        pos2(xl, yp),
                        pos2(xl, ymid),
                        pos2(xr, ymid),
                        pos2(xr, ys),
                        pos2(s1, ys),
                    ],
                    ArrowDir::Left,
                )
            }
        }
    }
}

/// Triangle de la pointe de flèche : pointe en `tip`, orientée selon `dir`,
/// de longueur `size` et de demi-largeur `0,8 × size`.
#[must_use]
pub fn arrow_points(tip: Pos2, dir: ArrowDir, size: f32) -> [Pos2; 3] {
    let half = size * 0.8;
    match dir {
        ArrowDir::Right => [
            tip,
            pos2(tip.x - size, tip.y + half),
            pos2(tip.x - size, tip.y - half),
        ],
        ArrowDir::Left => [
            tip,
            pos2(tip.x + size, tip.y - half),
            pos2(tip.x + size, tip.y + half),
        ],
        ArrowDir::Down => [
            tip,
            pos2(tip.x - half, tip.y - size),
            pos2(tip.x + half, tip.y - size),
        ],
        ArrowDir::Up => [
            tip,
            pos2(tip.x + half, tip.y + size),
            pos2(tip.x - half, tip.y + size),
        ],
    }
}

/// Marge horizontale de culling des barres et des liens.
const CULL_MARGIN: f32 = 16.0;

/// Géométrie écran d'une barre, déjà bornée et finie.
#[derive(Clone, Copy, Debug)]
struct BarGeom {
    /// Début.
    x0: f32,
    /// Fin (≥ `x0 + 1` pour une barre, `= x0` pour un jalon).
    x1: f32,
    /// Centre vertical de la ligne.
    yc: f32,
    /// Nature de la ligne.
    kind: RowKind,
    /// Chemin critique.
    critical: bool,
}

impl BarGeom {
    /// Barre d'une ligne, ou `None` (ligne vide, pas de dates, valeurs non
    /// finies).
    fn of(row: &Row, axis: &TimeAxis, yc: f32) -> Option<Self> {
        let (start, finish) = row.bar?;
        if row.kind == RowKind::Null {
            return None;
        }
        let x0 = axis.x(start);
        let x1 = if row.kind == RowKind::Milestone {
            x0
        } else {
            axis.x(finish).max(x0 + 1.0)
        };
        (x0.is_finite() && x1.is_finite() && yc.is_finite()).then_some(Self {
            x0,
            x1,
            yc,
            kind: row.kind,
            critical: row.critical,
        })
    }

    /// Extrémités vues par les liens (losange pour un jalon).
    fn ends(&self, m: &LinkMetrics) -> BarEnds {
        if self.kind == RowKind::Milestone {
            let d = milestone_half(m);
            BarEnds {
                start_x: self.x0 - d,
                finish_x: self.x0 + d,
                y: self.yc,
            }
        } else {
            BarEnds {
                start_x: self.x0,
                finish_x: self.x1,
                y: self.yc,
            }
        }
    }

    /// Hors de la découpe horizontale (marge comprise).
    fn culled(&self, clip: Rect, m: &LinkMetrics) -> bool {
        let d = milestone_half(m);
        self.x1 + d < clip.left() - CULL_MARGIN || self.x0 - d > clip.right() + CULL_MARGIN
    }
}

/// Demi-diagonale du losange d'un jalon.
fn milestone_half(m: &LinkMetrics) -> f32 {
    m.bar_half_h + 1.0
}

/// Centre vertical écran de la ligne `i`.
fn row_center(origin: Pos2, i: usize, row_h: f32) -> f32 {
    origin.y + index_to_f32(i) * row_h + row_h / 2.0
}

/// Formes d'une barre (sans contrôle de culling).
fn bar_shapes(b: &BarGeom, palette: &Palette, m: &LinkMetrics, out: &mut Vec<Shape>) {
    let h = m.bar_half_h;
    let fill = if b.critical { palette.critical } else { palette.bar };
    match b.kind {
        RowKind::Null => {}
        RowKind::Leaf => {
            let rect = Rect::from_min_max(pos2(b.x0, b.yc - h), pos2(b.x1, b.yc + h));
            let outline = if b.critical {
                palette.critical
            } else {
                palette.bar_outline
            };
            out.push(Shape::rect_filled(rect, 2.0, fill));
            out.push(Shape::rect_stroke(
                rect,
                2.0,
                Stroke::new(1.0, outline),
                StrokeKind::Inside,
            ));
        }
        RowKind::Summary => {
            let top = b.yc - h;
            let thick = (2.0 * h / 3.0).max(2.0);
            let rect = Rect::from_min_max(pos2(b.x0, top), pos2(b.x1, top + thick));
            // Récapitulative critique : rouge sombre (hypothèse H-UI).
            let summary = if b.critical {
                palette.summary_critical
            } else {
                palette.summary
            };
            out.push(Shape::rect_filled(rect, 0.0, summary));
            let w = h.min((b.x1 - b.x0) / 2.0).max(1.0);
            let depth = 1.6 * h;
            let left = vec![pos2(b.x0, top), pos2(b.x0 + w, top), pos2(b.x0, top + depth)];
            let right = vec![pos2(b.x1 - w, top), pos2(b.x1, top), pos2(b.x1, top + depth)];
            out.push(Shape::convex_polygon(left, summary, Stroke::NONE));
            out.push(Shape::convex_polygon(right, summary, Stroke::NONE));
        }
        RowKind::Milestone => {
            let color = if b.critical {
                palette.critical
            } else {
                palette.milestone
            };
            out.push(Shape::convex_polygon(
                diamond(b.x0, b.yc, milestone_half(m)),
                color,
                Stroke::NONE,
            ));
        }
        RowKind::Excluded => {
            let stroke = Stroke::new(1.0, palette.text_weak);
            if b.x1 - b.x0 <= 1.0 {
                out.push(Shape::closed_line(diamond(b.x0, b.yc, milestone_half(m)), stroke));
            } else {
                let rect = Rect::from_min_max(pos2(b.x0, b.yc - h), pos2(b.x1, b.yc + h));
                out.push(Shape::rect_stroke(rect, 2.0, stroke, StrokeKind::Inside));
            }
        }
    }
}

/// Sommets d'un losange centré en `(x, y)`, sens horaire.
fn diamond(x: f32, y: f32, d: f32) -> Vec<Pos2> {
    vec![pos2(x, y - d), pos2(x + d, y), pos2(x, y + d), pos2(x - d, y)]
}

/// Boîte englobante d'un tracé (pointe comprise) qui coupe la découpe.
fn path_visible(path: &LinkPath, clip: Rect) -> bool {
    let mut bbox = Rect::NOTHING;
    for p in path.points() {
        bbox.extend_with(*p);
    }
    bbox.is_finite() && bbox.expand(CULL_MARGIN).intersects(clip)
}

/// Fond, grille verticale aux frontières des deux niveaux de l'échelle,
/// bande de sélection et ligne de la date d'état.
fn background_shapes(
    fc: &FrameCtx<'_>,
    axis: &TimeAxis,
    origin: Pos2,
    range: &Range<usize>,
    clip: Rect,
    out: &mut Vec<Shape>,
) {
    let palette = fc.palette;
    let row_h = fc.metrics.row_h;
    out.push(Shape::rect_filled(clip, 0.0, palette.background));
    let week_start = fc
        .doc
        .map_or(Weekday::Monday, |doc| doc.project().options.week_start_day);
    let (major, minor) = fc.state.zoom.tiers();
    for (unit, color) in [(minor, palette.grid), (major, palette.grid_major)] {
        let stroke = Stroke::new(1.0, color);
        for (_, x0, _) in tier_spans(axis, unit, week_start, (clip.left(), clip.right())) {
            let x = x0.round() + 0.5;
            if x.is_finite() && x > clip.left() && x < clip.right() {
                out.push(Shape::line_segment(
                    [pos2(x, clip.top()), pos2(x, clip.bottom())],
                    stroke,
                ));
            }
        }
    }
    if let Some(rows) = fc.rows.filter(|_| !fc.selected.is_empty()) {
        for i in range.clone() {
            if rows.row(i).is_some_and(|r| fc.selected.contains(r.uid)) {
                let top = origin.y + index_to_f32(i) * row_h;
                let rect = Rect::from_min_max(pos2(clip.left(), top), pos2(clip.right(), top + row_h));
                if rect.is_finite() {
                    out.push(Shape::rect_filled(rect, 0.0, palette.selection));
                }
            }
        }
    }
    if let Some(doc) = fc.doc {
        let x = axis.x(doc.project().effective_status_date()).round() + 0.5;
        if x.is_finite() && x >= clip.left() && x <= clip.right() {
            let line = [pos2(x, clip.top()), pos2(x, clip.bottom())];
            out.extend(Shape::dashed_line(
                &line,
                Stroke::new(1.5, palette.status_date),
                6.0,
                4.0,
            ));
        }
    }
}

/// Repère du corps : projection, coin de la ligne 0, découpe, mesures.
#[derive(Clone, Copy)]
struct Body<'a> {
    /// Projection temps → pixels.
    axis: &'a TimeAxis,
    /// Coin haut-gauche écran de la ligne 0.
    origin: Pos2,
    /// Découpe du corps.
    clip: Rect,
    /// Mesures des barres et des liens.
    m: LinkMetrics,
}

/// Liens qui touchent les lignes `range` (intersection d'intervalles) ;
/// rend le nombre de liens dessinés.
fn link_shapes(
    rows: &RowModel,
    palette: &Palette,
    body: &Body<'_>,
    range: &Range<usize>,
    out: &mut Vec<Shape>,
) -> usize {
    let Body {
        axis,
        origin,
        clip,
        m,
    } = *body;
    let m = &m;
    let mut painted = 0_usize;
    for link in rows.links_touching(range.clone()) {
        let (Ok(from), Ok(to)) = (usize::try_from(link.from), usize::try_from(link.to)) else {
            continue;
        };
        let geom = |i: usize| {
            rows.row(i)
                .and_then(|r| BarGeom::of(r, axis, row_center(origin, i, m.row_h)))
        };
        let (Some(p), Some(s)) = (geom(from), geom(to)) else {
            continue;
        };
        let path = link_path(link.kind, p.ends(m), s.ends(m), m);
        if path.len < 2 || !path_visible(&path, clip) {
            continue;
        }
        let color = if link.critical {
            palette.critical
        } else {
            palette.link
        };
        out.push(Shape::line(path.points().to_vec(), Stroke::new(1.0, color)));
        let tri = arrow_points(path.arrow_tip, path.arrow_dir, m.arrow);
        out.push(Shape::convex_polygon(tri.to_vec(), color, Stroke::NONE));
        painted = painted.saturating_add(1);
    }
    painted
}

/// Corps du Gantt : lignes `range`, `origin` étant le coin haut-gauche écran
/// de la ligne 0, découpe `clip`, projection `axis`.
///
/// Couches : fond → grille → sélection → date d'état → liens → barres. Seules
/// les lignes de `range` sont parcourues (liens : ceux dont l'intervalle de
/// lignes coupe `range`) ; une barre hors de la découpe horizontale n'est pas
/// dessinée.
pub fn paint_body(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    axis: &TimeAxis,
    origin: Pos2,
    range: Range<usize>,
    clip: Rect,
) {
    if !clip.is_finite() || !origin.is_finite() || clip.width() <= 0.0 || clip.height() <= 0.0 {
        return;
    }
    let painter = ui.painter_at(clip);
    let m = LinkMetrics {
        row_h: fc.metrics.row_h,
        ..LinkMetrics::default()
    };
    let mut shapes = Vec::new();
    background_shapes(fc, axis, origin, &range, clip, &mut shapes);
    let mut links = 0;
    if let Some(rows) = fc.rows {
        let body = Body {
            axis,
            origin,
            clip,
            m,
        };
        links = link_shapes(rows, fc.palette, &body, &range, &mut shapes);
        for i in range.clone() {
            let Some(bar) = rows
                .row(i)
                .and_then(|r| BarGeom::of(r, axis, row_center(origin, i, m.row_h)))
            else {
                continue;
            };
            if !bar.culled(clip, &m) {
                bar_shapes(&bar, fc.palette, &m, &mut shapes);
            }
        }
    }
    fc.stats.gantt_rows = range;
    fc.stats.links_painted = links;
    fc.stats.shapes = fc.stats.shapes.saturating_add(shapes.len());
    painter.extend(shapes);
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp,
    clippy::arithmetic_side_effects,
    clippy::similar_names,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use super::*;

    const M: LinkMetrics = LinkMetrics {
        stub: 6.0,
        arrow: 4.0,
        row_h: 22.0,
        bar_half_h: 5.0,
    };

    fn bar(start_x: f32, finish_x: f32, row: u8) -> BarEnds {
        BarEnds {
            start_x,
            finish_x,
            y: 11.0 + 22.0 * f32::from(row),
        }
    }

    /// Vérifications communes : points finis, segments orthogonaux, premier
    /// point sur l'extrémité attendue du prédécesseur, dernier sur
    /// l'extrémité attendue du successeur, flèche au dernier point.
    fn check(path: &LinkPath, first: Pos2, last: Pos2, dir: ArrowDir) {
        let pts = path.points();
        assert!(pts.len() >= 2 && pts.len() <= 6, "{pts:?}");
        assert!(pts.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
        for w in pts.windows(2) {
            assert!(w[0].x == w[1].x || w[0].y == w[1].y, "segment oblique {w:?}");
        }
        assert_eq!(pts[0], first);
        assert_eq!(*pts.last().unwrap(), last);
        assert_eq!(path.arrow_tip, last);
        assert_eq!(path.arrow_dir, dir);
        // Le dernier segment va dans le sens de la flèche.
        let (a, b) = (pts[pts.len() - 2], pts[pts.len() - 1]);
        let ok = match dir {
            ArrowDir::Right => b.x > a.x,
            ArrowDir::Left => b.x < a.x,
            ArrowDir::Down => b.y > a.y,
            ArrowDir::Up => b.y < a.y,
        };
        assert!(ok, "flèche {dir:?} contre le segment {a:?} → {b:?}");
    }

    #[test]
    fn link_path_fs_direct() {
        let p = bar(10.0, 50.0, 0);
        let s = bar(80.0, 140.0, 1);
        let path = link_path(LinkType::FinishToStart, p, s, &M);
        // Entre par le dessus du successeur, à `stub` de son début.
        check(&path, pos2(50.0, 11.0), pos2(86.0, 33.0 - 5.0), ArrowDir::Down);
        assert_eq!(path.len, 3);
    }

    #[test]
    fn link_path_fs_folded() {
        let p = bar(10.0, 50.0, 0);
        let s = bar(30.0, 90.0, 2);
        let path = link_path(LinkType::FinishToStart, p, s, &M);
        check(&path, pos2(50.0, 11.0), pos2(30.0, 55.0), ArrowDir::Right);
        assert_eq!(path.len, 6);
        // Passage dans l'intercolonne, sous la ligne du prédécesseur.
        assert_eq!(path.points[2].y, 22.0);
        assert_eq!(path.points[3], pos2(24.0, 22.0));
    }

    #[test]
    fn link_path_ss() {
        let p = bar(40.0, 90.0, 0);
        let s = bar(60.0, 120.0, 1);
        let path = link_path(LinkType::StartToStart, p, s, &M);
        check(&path, pos2(40.0, 11.0), pos2(60.0, 33.0), ArrowDir::Right);
        assert_eq!(path.points[1].x, 34.0);
    }

    #[test]
    fn link_path_ff() {
        let p = bar(40.0, 90.0, 0);
        let s = bar(20.0, 70.0, 1);
        let path = link_path(LinkType::FinishToFinish, p, s, &M);
        check(&path, pos2(90.0, 11.0), pos2(70.0, 33.0), ArrowDir::Left);
        assert_eq!(path.points[1].x, 96.0);
    }

    #[test]
    fn link_path_sf() {
        // Direct : le successeur finit bien avant le début du prédécesseur.
        let p = bar(100.0, 150.0, 0);
        let s = bar(20.0, 60.0, 1);
        let path = link_path(LinkType::StartToFinish, p, s, &M);
        check(&path, pos2(100.0, 11.0), pos2(60.0, 33.0), ArrowDir::Left);
        assert_eq!(path.len, 4);
        // Replié : le successeur finit après le début du prédécesseur.
        let s = bar(80.0, 130.0, 1);
        let path = link_path(LinkType::StartToFinish, p, s, &M);
        check(&path, pos2(100.0, 11.0), pos2(130.0, 33.0), ArrowDir::Left);
        assert_eq!(path.len, 6);
    }

    #[test]
    fn link_path_successor_above() {
        let p = bar(10.0, 50.0, 3);
        let s = bar(80.0, 140.0, 1);
        let path = link_path(LinkType::FinishToStart, p, s, &M);
        // Entre par le dessous du successeur, flèche vers le haut.
        check(&path, pos2(50.0, 77.0), pos2(86.0, 33.0 + 5.0), ArrowDir::Up);
        // Replié au-dessus : l'intercolonne est la frontière au-dessus de P.
        let s = bar(20.0, 60.0, 1);
        let path = link_path(LinkType::FinishToStart, p, s, &M);
        check(&path, pos2(50.0, 77.0), pos2(20.0, 33.0), ArrowDir::Right);
        assert_eq!(path.points[2].y, 66.0);
    }

    #[test]
    fn link_path_milestone() {
        // Jalon prédécesseur (losange centré en 50, demi-diagonale 6).
        let p = bar(44.0, 56.0, 0);
        let s = bar(90.0, 140.0, 1);
        let path = link_path(LinkType::FinishToStart, p, s, &M);
        check(&path, pos2(56.0, 11.0), pos2(96.0, 28.0), ArrowDir::Down);
        // Jalon successeur : la flèche tombe au centre du losange.
        let p = bar(10.0, 40.0, 0);
        let s = bar(74.0, 86.0, 1);
        let path = link_path(LinkType::FinishToStart, p, s, &M);
        check(&path, pos2(40.0, 11.0), pos2(80.0, 28.0), ArrowDir::Down);
        // Entrée non finie : tracé vide, sans panique.
        let bad = bar(f32::NAN, 10.0, 0);
        assert_eq!(link_path(LinkType::FinishToStart, bad, s, &M).len, 0);
    }

    #[test]
    fn arrow_points_oriented() {
        let tip = pos2(10.0, 10.0);
        let right = arrow_points(tip, ArrowDir::Right, 4.0);
        assert!(right[1].x < tip.x && right[2].x < tip.x);
        let left = arrow_points(tip, ArrowDir::Left, 4.0);
        assert!(left[1].x > tip.x && left[2].x > tip.x);
        let down = arrow_points(tip, ArrowDir::Down, 4.0);
        assert!(down[1].y < tip.y && down[2].y < tip.y);
        let up = arrow_points(tip, ArrowDir::Up, 4.0);
        assert!(up[1].y > tip.y && up[2].y > tip.y);
    }
    /// Fumée : en-tête et corps dessinés sur l'exemple, à tous les zooms, y
    /// compris avec une découpe dégénérée, sans panique ni `Rect` non fini
    /// (egui vérifie par `debug_assert!`).
    #[test]
    fn gantt_paint_smoke_demo() {
        use planrs_core::{Day, Moment, TimeOfDay};

        use crate::app::{DocOrigin, OpenDocument};
        use crate::i18n::{Language, Texts};
        use crate::rows::RowModel;
        use crate::state::UiState;
        use crate::timescale::TimeScaleZoom;
        use crate::view::{FrameStats, Metrics};

        let day = Day::from_naive(chrono::NaiveDate::from_ymd_opt(2027, 1, 15).unwrap()).unwrap();
        let now = Moment::at(day, TimeOfDay::hm(8, 0).unwrap()).unwrap();
        let loaded = crate::document::load(crate::DEMO_MSPDI, now).unwrap();
        let origin = DocOrigin {
            name: String::new(),
            path: None,
            demo: true,
        };
        let doc = OpenDocument::new(loaded, origin, 1);
        let texts = Texts::new(Language::Fr);
        let rows = RowModel::build(&doc, texts.format_labels());
        let palette = Palette::LIGHT;
        let metrics = Metrics::default();
        let selection = rows.row(2).map(|r| r.uid);
        let selected = crate::selection::Selection::single(selection);
        let ctx = egui::Context::default();
        for zoom in TimeScaleZoom::ALL {
            let state = UiState {
                zoom,
                ..UiState::default()
            };
            let mut actions = Vec::new();
            let mut stats = FrameStats::default();
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let mut fc = FrameCtx {
                    state: &state,
                    doc: Some(&doc),
                    rows: Some(&rows),
                    texts: &texts,
                    palette: &palette,
                    metrics: &metrics,
                    diagnostics: &crate::view::DiagnosticsView::default(),
                    selection,
                    selected: &selected,
                    actions: &mut actions,
                    stats: &mut stats,
                };
                let header = Rect::from_min_max(pos2(600.0, 0.0), pos2(1400.0, 44.0));
                let clip = Rect::from_min_max(pos2(600.0, 44.0), pos2(1400.0, 700.0));
                for scroll in [0.0, 500.0, 5_000.0] {
                    let axis = TimeAxis::new(doc.range, zoom, 600.0 - scroll);
                    crate::view::timescale::paint_header(ui, &mut fc, &axis, header);
                    let len = rows.len();
                    paint_body(ui, &mut fc, &axis, pos2(600.0, 44.0), 0..len, clip);
                }
                let axis = TimeAxis::new(doc.range, zoom, 600.0);
                let nan = Rect::from_min_max(pos2(f32::NAN, 0.0), pos2(10.0, 10.0));
                paint_body(ui, &mut fc, &axis, pos2(600.0, 44.0), 0..3, nan);
                crate::view::timescale::paint_header(ui, &mut fc, &axis, nan);
            });
            out.textures_delta.clear();
            assert!(stats.shapes > 0, "{zoom:?}");
            assert!(stats.links_painted <= 46);
        }
    }
}
