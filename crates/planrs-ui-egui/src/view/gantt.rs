//! Corps du Gantt : grille, jours chômés, barres, jalons, récapitulatives,
//! chemin critique, date d'état et liens ; Gantt de suivi (ligne de base 0,
//! bande d'avancement) et infobulle `explain_date` d'une barre (J7, lot L7).
//!
//! Ordre des couches : fond → jours chômés → grille → sélection → date
//! d'état → liens → lignes de base → barres → bandes d'avancement. Le tracé
//! d'un lien ([`link_path`]), la mise en page d'une ligne de suivi
//! ([`tracking_layout`]) et la bande d'avancement ([`progress_span`]) sont
//! des fonctions pures, testées sans egui. Aucune date n'est calculée ici :
//! début, fin, ligne de base et `Stop` viennent du moteur (règle 3).

use std::ops::Range;

use egui::{Id, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, pos2};
use planrs_automation::command::TaskRef;
use planrs_automation::command::params::ExplainParams;
use planrs_automation::gesture::GestureKind;
use planrs_automation::json::SideJson;
use planrs_core::{LinkType, TaskUid, Weekday};

use super::{BarDrag, FrameCtx};
use crate::app::OpenDocument;
use crate::document::project_calendar;
use crate::edit::text_language;
use crate::geom::{BarZone, GanttHit, bar_zone, index_to_f32, near_polyline, row_at};
use crate::i18n::{Arg, Language, Texts, error_text_id};
use crate::query::{Query, QueryValue};
use crate::row_view::{DisplayRow, VisibleRows};
use crate::rows::{Row, RowKind};
use crate::state::GanttStyle;
use crate::theme::Palette;
use crate::timescale::{TierUnit, TimeAxis, TimeScaleZoom, tier_spans};

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

/// Bandes verticales d'une ligne du Gantt de suivi (H-UI-22), en
/// ordonnées écran `(haut, bas)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackingLayout {
    /// Barre courante : au-dessus du tiers inférieur.
    pub current: (f32, f32),
    /// Barre de la ligne de base 0 : dans le tiers inférieur de la ligne.
    pub baseline: (f32, f32),
}

/// Mise en page d'une ligne de suivi de hauteur `row_h` dont le haut est à
/// `row_top` (hypothèse H-UI-22) : barre courante de `row_top + 3` à
/// 1 point au-dessus du tiers inférieur, ligne de base de 1 point sous le
/// début du tiers inférieur à 2 points du bas. Entrée non finie ou ligne
/// trop basse : bandes vides (haut = bas), jamais de NaN.
#[must_use]
pub fn tracking_layout(row_top: f32, row_h: f32) -> TrackingLayout {
    if !row_top.is_finite() || !row_h.is_finite() || row_h <= 0.0 {
        let top = if row_top.is_finite() { row_top } else { 0.0 };
        return TrackingLayout {
            current: (top, top),
            baseline: (top, top),
        };
    }
    let lower = row_top + 2.0 * row_h / 3.0;
    let band = |top: f32, bottom: f32| {
        let top = top.min(row_top + row_h);
        (top, bottom.max(top))
    };
    TrackingLayout {
        current: band(row_top + 3.0, lower - 1.0),
        baseline: band(lower + 1.0, row_top + row_h - 2.0),
    }
}

/// Abscisses `(gauche, droite)` de la bande d'avancement d'une barre
/// `[x0, x1]` dont la partie réelle s'arrête à `stop_x` (H-UI-22 :
/// [début, `Stop`], bornée à la barre) ; `None` si elle est vide ou si une
/// entrée n'est pas finie.
#[must_use]
pub fn progress_span(x0: f32, x1: f32, stop_x: f32) -> Option<(f32, f32)> {
    if !(x0.is_finite() && x1.is_finite() && stop_x.is_finite()) {
        return None;
    }
    let right = stop_x.min(x1);
    (right > x0).then_some((x0, right))
}

/// Géométrie écran d'une barre, déjà bornée et finie.
#[derive(Clone, Copy, Debug)]
struct BarGeom {
    /// Début.
    x0: f32,
    /// Fin (≥ `x0 + 1` pour une barre, `= x0` pour un jalon).
    x1: f32,
    /// Centre vertical de la barre.
    yc: f32,
    /// Demi-hauteur de la barre.
    half_h: f32,
    /// Nature de la ligne.
    kind: RowKind,
    /// Chemin critique.
    critical: bool,
}

impl BarGeom {
    /// Barre d'une ligne dont le haut est à `top`, ou `None` (ligne vide,
    /// pas de dates, valeurs non finies). Gantt de suivi et ligne de base
    /// enregistrée : barre courante au-dessus du tiers inférieur
    /// (H-UI-22) ; sinon centrée.
    fn of(row: &Row, axis: &TimeAxis, top: f32, m: &LinkMetrics, tracking: bool) -> Option<Self> {
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
        let (yc, half_h) = if tracking && row.baseline.is_some() {
            let (a, b) = tracking_layout(top, m.row_h).current;
            (f32::midpoint(a, b), (b - a) / 2.0)
        } else {
            (top + m.row_h / 2.0, m.bar_half_h)
        };
        (x0.is_finite() && x1.is_finite() && yc.is_finite() && half_h.is_finite()).then_some(Self {
            x0,
            x1,
            yc,
            half_h,
            kind: row.kind,
            critical: row.critical,
        })
    }

    /// Demi-diagonale du losange d'un jalon.
    fn diamond_half(&self) -> f32 {
        self.half_h + 1.0
    }

    /// Extrémités vues par les liens (losange pour un jalon).
    fn ends(&self) -> BarEnds {
        if self.kind == RowKind::Milestone {
            let d = self.diamond_half();
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
    fn culled(&self, clip: Rect) -> bool {
        let d = self.diamond_half();
        self.x1 + d < clip.left() - CULL_MARGIN || self.x0 - d > clip.right() + CULL_MARGIN
    }

    /// Zone de survol de l'infobulle : la barre (losange d'un jalon) et
    /// 2 points autour.
    fn hover_rect(&self) -> Rect {
        let d = if self.kind == RowKind::Milestone {
            self.diamond_half()
        } else {
            0.0
        };
        Rect::from_min_max(
            pos2(self.x0 - d - 2.0, self.yc - self.half_h - 2.0),
            pos2(self.x1 + d + 2.0, self.yc + self.half_h + 2.0),
        )
    }
}

/// Haut écran de la ligne `i`.
fn row_top(origin: Pos2, i: usize, row_h: f32) -> f32 {
    origin.y + index_to_f32(i) * row_h
}

/// Formes d'une barre (sans contrôle de culling).
fn bar_shapes(b: &BarGeom, palette: &Palette, out: &mut Vec<Shape>) {
    let h = b.half_h;
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
                diamond(b.x0, b.yc, b.diamond_half()),
                color,
                Stroke::NONE,
            ));
        }
        RowKind::Excluded => {
            let stroke = Stroke::new(1.0, palette.text_weak);
            if b.x1 - b.x0 <= 1.0 {
                out.push(Shape::closed_line(diamond(b.x0, b.yc, b.diamond_half()), stroke));
            } else {
                let rect = Rect::from_min_max(pos2(b.x0, b.yc - h), pos2(b.x1, b.yc + h));
                out.push(Shape::rect_stroke(rect, 2.0, stroke, StrokeKind::Inside));
            }
        }
    }
}

/// Barre de la ligne de base 0 d'une ligne dont le haut est à `top`
/// (H-UI-22) : rectangle plein dans le tiers inférieur, losange si début et
/// fin de base coïncident ; rien pour une ligne vide ou exclue, sans ligne
/// de base, hors de la découpe ou non finie.
fn baseline_shape(
    row: &Row,
    axis: &TimeAxis,
    top: f32,
    row_h: f32,
    clip: Rect,
    color: egui::Color32,
) -> Option<Shape> {
    if matches!(row.kind, RowKind::Null | RowKind::Excluded) {
        return None;
    }
    let (start, finish) = row.baseline?;
    let (y0, y1) = tracking_layout(top, row_h).baseline;
    let x0 = axis.x(start);
    let point = finish <= start;
    let x1 = if point { x0 } else { axis.x(finish).max(x0 + 1.0) };
    let d = (y1 - y0) / 2.0 + 1.0;
    let finite = [x0, x1, y0, y1, d].iter().all(|v| v.is_finite());
    if !finite || x1 + d < clip.left() - CULL_MARGIN || x0 - d > clip.right() + CULL_MARGIN {
        return None;
    }
    Some(if point {
        Shape::convex_polygon(diamond(x0, f32::midpoint(y0, y1), d), color, Stroke::NONE)
    } else {
        Shape::rect_filled(Rect::from_min_max(pos2(x0, y0), pos2(x1, y1)), 0.0, color)
    })
}

/// Bande d'avancement d'une barre de tâche feuille (H-UI-22) : sur
/// [début, `Stop`] (fin de la partie réelle, R-204), dans la moitié centrale
/// de la barre.
fn progress_shape(row: &Row, bar: &BarGeom, axis: &TimeAxis, color: egui::Color32) -> Option<Shape> {
    if bar.kind != RowKind::Leaf {
        return None;
    }
    let (a, b) = progress_span(bar.x0, bar.x1, axis.x(row.stop?))?;
    let h = (bar.half_h / 2.0).max(1.0);
    let rect = Rect::from_min_max(pos2(a, bar.yc - h), pos2(b, bar.yc + h));
    rect.is_finite().then(|| Shape::rect_filled(rect, 0.0, color))
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

/// Jours chômés du calendrier du projet grisés (N3), aux zooms Jour et
/// Semaine seulement : jours visibles consécutifs fusionnés, donc au plus un
/// rectangle par jour visible. Calendrier compilé une fois par génération
/// (`OpenDocument::calendars`) ; calendrier du projet en échec ou hors
/// budget : rien (le diagnostic du moteur est au panneau). Rend le nombre
/// de rectangles.
fn non_working_shapes(
    fc: &FrameCtx<'_>,
    axis: &TimeAxis,
    week_start: Weekday,
    clip: Rect,
    out: &mut Vec<Shape>,
) -> usize {
    if !matches!(fc.state.zoom, TimeScaleZoom::Day | TimeScaleZoom::Week) {
        return 0;
    }
    let Some(doc) = fc.doc else {
        return 0;
    };
    let set = doc.calendars();
    let Some(calendar) = project_calendar(&set, doc.project()) else {
        return 0;
    };
    let mut count = 0_usize;
    let mut push = |(x0, x1): (f32, f32), out: &mut Vec<Shape>| {
        let rect = Rect::from_min_max(
            pos2(x0.max(clip.left()), clip.top()),
            pos2(x1.min(clip.right()), clip.bottom()),
        );
        if rect.is_finite() && rect.width() > 0.0 {
            out.push(Shape::rect_filled(rect, 0.0, fc.palette.non_working));
            count = count.saturating_add(1);
        }
    };
    let mut run: Option<(f32, f32)> = None;
    for (day, x0, x1) in tier_spans(axis, TierUnit::Day, week_start, (clip.left(), clip.right())) {
        let off = calendar.day_ranges(day).is_ok_and(<[_]>::is_empty);
        if off {
            run = Some(run.map_or((x0, x1), |(a, _)| (a, x1)));
        } else if let Some(span) = run.take() {
            push(span, out);
        }
    }
    if let Some(span) = run {
        push(span, out);
    }
    count
}

/// Fond, jours chômés, grille verticale aux frontières des deux niveaux de
/// l'échelle, bande de sélection et ligne de la date d'état.
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
    non_working_shapes(fc, axis, week_start, clip, out);
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
    if let Some(rows) = fc.visible().filter(|_| !fc.selected.is_empty()) {
        for i in range.clone() {
            if rows
                .row(DisplayRow::at(i))
                .is_some_and(|r| fc.selected.contains(r.uid))
            {
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
    /// Gantt de suivi (H-UI-22).
    tracking: bool,
}

impl Body<'_> {
    /// Barre de la ligne affichée `i`.
    fn bar(&self, rows: VisibleRows<'_>, i: usize) -> Option<BarGeom> {
        rows.row(DisplayRow::at(i)).and_then(|r| {
            BarGeom::of(
                r,
                self.axis,
                row_top(self.origin, i, self.m.row_h),
                &self.m,
                self.tracking,
            )
        })
    }
}

/// Liens qui touchent les lignes `range` (intersection d'intervalles) ;
/// rend le nombre de liens dessinés.
fn link_shapes(
    rows: VisibleRows<'_>,
    palette: &Palette,
    body: &Body<'_>,
    range: &Range<usize>,
    out: &mut Vec<Shape>,
) -> usize {
    let m = &body.m;
    let mut painted = 0_usize;
    for (shown, link) in rows.links_touching(range.clone()) {
        let (from, to) = (shown.from.index(), shown.to.index());
        let (Some(p), Some(s)) = (body.bar(rows, from), body.bar(rows, to)) else {
            continue;
        };
        let path = link_path(link.kind, p.ends(), s.ends(), m);
        if path.len < 2 || !path_visible(&path, body.clip) {
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

/// Clé du cache de l'infobulle : (ouverture, génération, UID, côté).
type TipKey = (u64, u64, TaskUid, SideJson);

/// Cache d'une entrée de l'infobulle `explain_date` (H-UI-26), gardé dans
/// la mémoire temporaire d'egui : la lecture n'est refaite que si le
/// document, sa génération, la tâche ou le côté changent.
#[derive(Clone, Debug, Default)]
pub struct ExplainTipCache {
    /// Clé du texte gardé.
    key: Option<TipKey>,
    /// Texte gardé (résumé du gabarit, ou refus traduit).
    text: String,
    /// Lectures faites (tests).
    runs: u64,
}

impl ExplainTipCache {
    /// Texte de `key`, lu par `run` seulement si la clé change.
    fn text_for(&mut self, key: TipKey, run: impl FnOnce() -> String) -> String {
        if self.key != Some(key) {
            self.text = run();
            self.key = Some(key);
            self.runs = self.runs.saturating_add(1);
        }
        self.text.clone()
    }
}

/// Identifiant du cache de l'infobulle dans la mémoire d'egui.
fn tip_cache_id() -> Id {
    Id::unique("planrs.gantt.explain_tip")
}

/// Résumé `explain_date` du début de la tâche `uid` (gabarit du moteur,
/// R-233, dans la langue de l'interface), par la lecture typée
/// `Query::Explain` (même fonction que `analysis.explain_date`) ; refus du
/// moteur traduit, jamais une date rédigée ici (H-UI-26).
#[must_use]
pub fn explain_tip_text(doc: &OpenDocument, texts: &Texts, uid: TaskUid) -> String {
    let query = Query::Explain(ExplainParams {
        task: TaskRef::Uid(uid),
        side: Some(SideJson::Start),
        depth: Some(1),
    });
    match query.run(&doc.session) {
        Ok(QueryValue::Explain(view)) => match texts.language() {
            Language::Fr => view.summary_fr,
            Language::En => view.summary_en,
        },
        Ok(QueryValue::Usage(_) | QueryValue::Variance(_)) => String::new(),
        Err(error) => {
            let detail = error.message(text_language(texts.language()));
            texts.get_args(error_text_id(error.code), &[("detail", Arg::Str(detail.into()))])
        }
    }
}

/// Infobulle de la barre sous le pointeur (H-UI-26), après le délai
/// d'egui : une zone de survol (sans clic ni glisser, le fond garde
/// sélection et défilement) posée seulement sur la barre survolée.
fn bar_tooltip(
    ui: &mut egui::Ui,
    fc: &FrameCtx<'_>,
    rows: VisibleRows<'_>,
    body: &Body<'_>,
    range: &Range<usize>,
) {
    let Some(doc) = fc.doc else {
        return;
    };
    let Some(p) = ui.ctx().pointer_hover_pos() else {
        return;
    };
    if !ui.rect_contains_pointer(body.clip) {
        return;
    }
    let Some(i) = row_at(p.y - body.origin.y, body.m.row_h, rows.len()).filter(|i| range.contains(i)) else {
        return;
    };
    let (Some(row), Some(bar)) = (rows.row(DisplayRow::at(i)), body.bar(rows, i)) else {
        return;
    };
    let hover = bar.hover_rect();
    if !hover.contains(p) {
        return;
    }
    let uid = row.uid;
    let response = ui.interact(
        hover.intersect(body.clip),
        ui.make_persistent_id(("planrs.gantt.bar_tip", uid)),
        Sense::hover(),
    );
    response.on_hover_ui(|ui| {
        let (opening, generation) = doc.cache_key();
        let key = (opening, generation, uid, SideJson::Start);
        let mut cache = ui
            .ctx()
            .data(|d| d.get_temp::<ExplainTipCache>(tip_cache_id()))
            .unwrap_or_default();
        let text = cache.text_for(key, || explain_tip_text(doc, fc.texts, uid));
        ui.ctx().data_mut(|d| d.insert_temp(tip_cache_id(), cache));
        ui.label(text);
    });
}

/// Lectures faites par le cache de l'infobulle (tests de H-UI-26).
#[must_use]
pub fn explain_tip_runs(ctx: &egui::Context) -> u64 {
    ctx.data(|d| d.get_temp::<ExplainTipCache>(tip_cache_id()))
        .map_or(0, |cache| cache.runs)
}

/// Corps du Gantt : lignes `range`, `origin` étant le coin haut-gauche écran
/// de la ligne 0, découpe `clip`, projection `axis`.
///
/// Couches : fond → jours chômés → grille → sélection → date d'état →
/// liens → lignes de base (suivi) → barres → bandes d'avancement (suivi).
/// Seules les lignes de `range` sont parcourues (liens : ceux dont
/// l'intervalle de lignes coupe `range`) ; une barre hors de la découpe
/// horizontale n'est pas dessinée. Infobulle `explain_date` sur la barre
/// survolée.
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
    let tracking = fc.state.gantt_style == GanttStyle::Tracking;
    let mut shapes = Vec::new();
    background_shapes(fc, axis, origin, &range, clip, &mut shapes);
    let mut links = 0;
    if let Some(rows) = fc.visible() {
        let body = Body {
            axis,
            origin,
            clip,
            m,
            tracking,
        };
        links = link_shapes(rows, fc.palette, &body, &range, &mut shapes);
        for i in range.clone() {
            let Some(row) = rows.row(DisplayRow::at(i)) else {
                continue;
            };
            if tracking
                && let Some(shape) = baseline_shape(
                    row,
                    axis,
                    row_top(origin, i, m.row_h),
                    m.row_h,
                    clip,
                    fc.palette.baseline,
                )
            {
                shapes.push(shape);
            }
            let Some(bar) = body.bar(rows, i) else {
                continue;
            };
            if bar.culled(clip) {
                continue;
            }
            bar_shapes(&bar, fc.palette, &mut shapes);
            if tracking && let Some(shape) = progress_shape(row, &bar, axis, fc.palette.progress) {
                shapes.push(shape);
            }
        }
        bar_tooltip(ui, fc, rows, &body, &range);
    }
    fc.stats.gantt_rows = range;
    fc.stats.links_painted = links;
    fc.stats.shapes = fc.stats.shapes.saturating_add(shapes.len());
    painter.extend(shapes);
}

/// Tolérance du pointeur autour du tracé d'un lien, en points.
const LINK_HIT_TOL: f32 = 3.0;

/// Cible du pointeur `p` (écran) dans le corps du Gantt (H-UI-20,
/// H-UI-21) : zone de la barre de sa ligne (bande verticale de la barre et
/// 2 points autour ; losange d'un jalon), sinon tracé d'un lien qui
/// traverse sa ligne, sinon rien. `origin` : coin haut-gauche écran de la
/// ligne 0. Indices (lignes affichées) et UID seulement : aucune date.
#[must_use]
pub fn hit_test(
    rows: VisibleRows<'_>,
    axis: &TimeAxis,
    origin: Pos2,
    row_h: f32,
    tracking: bool,
    p: Pos2,
) -> GanttHit {
    let Some(i) = row_at(p.y - origin.y, row_h, rows.len()) else {
        return GanttHit::Empty;
    };
    let body = Body {
        axis,
        origin,
        clip: Rect::EVERYTHING,
        m: LinkMetrics {
            row_h,
            ..LinkMetrics::default()
        },
        tracking,
    };
    if let (Some(row), Some(bar)) = (rows.row(DisplayRow::at(i)), body.bar(rows, i)) {
        let band = bar.hover_rect();
        let (left, right) = if bar.kind == RowKind::Milestone {
            (band.left(), band.right())
        } else {
            (bar.x0, bar.x1)
        };
        if band.y_range().contains(p.y) {
            match bar_zone(p.x, left, right, row.kind) {
                Some(BarZone::Progress) => return GanttHit::BarStart(row.uid),
                Some(BarZone::Resize) => return GanttHit::BarEnd(row.uid),
                Some(BarZone::Inside) => return GanttHit::Bar(row.uid),
                None => {}
            }
        }
    }
    for (shown, link) in rows.links_touching(i..i.saturating_add(1)) {
        let (from, to) = (shown.from.index(), shown.to.index());
        let (Some(pred), Some(succ)) = (rows.row(shown.from), rows.row(shown.to)) else {
            continue;
        };
        let (Some(pb), Some(sb)) = (body.bar(rows, from), body.bar(rows, to)) else {
            continue;
        };
        let path = link_path(link.kind, pb.ends(), sb.ends(), &body.m);
        if near_polyline(p, path.points(), LINK_HIT_TOL) || path.arrow_tip.distance(p) <= LINK_HIT_TOL {
            return GanttHit::Link(pred.uid, succ.uid);
        }
    }
    GanttHit::Empty
}

/// Fantôme du glisser en cours (J7, lot L8) : contour de la barre aux
/// dates de l'aperçu (`gesture::preview`, abscisses par `TimeAxis::x`,
/// sens date → pixel), bande d'avancement de l'aperçu, ou trait du lien
/// tracé vers la ligne survolée. Aucune date n'est calculée ici.
pub fn paint_drag(
    ui: &egui::Ui,
    fc: &FrameCtx<'_>,
    axis: &TimeAxis,
    origin: Pos2,
    clip: Rect,
    drag: &BarDrag,
    hover: Option<Pos2>,
) {
    let Some(rows) = fc.visible() else {
        return;
    };
    let Some(d) = rows.display_of(drag.uid) else {
        return;
    };
    let i = d.index();
    let row_h = fc.metrics.row_h;
    let top = row_top(origin, i, row_h);
    let yc = top + row_h / 2.0;
    let half = LinkMetrics::default().bar_half_h + 1.0;
    let painter = ui.painter_at(clip);
    let stroke = Stroke::new(1.5, fc.palette.text_strong);
    let linking = drag.kind == GestureKind::Move && drag.over_row.is_some_and(|r| r != d);
    if linking {
        if let Some(p) = hover.filter(|p| p.is_finite()) {
            let from = pos2(clip.left() + drag.origin_x, yc);
            painter.line_segment([from, p], stroke);
        }
        return;
    }
    let Some(preview) = drag.preview else {
        return;
    };
    let x0 = axis.x(preview.start);
    let x1 = axis.x(preview.finish).max(x0 + 1.0);
    if !(x0.is_finite() && x1.is_finite() && yc.is_finite()) {
        return;
    }
    let rect = Rect::from_min_max(pos2(x0, yc - half), pos2(x1, yc + half));
    painter.rect_stroke(rect, 2.0, stroke, StrokeKind::Outside);
    if let Some(percent) = preview.percent {
        // Bande proportionnelle au % de l'aperçu (dessin seulement).
        let share = index_to_f32(usize::try_from(percent.get().clamp(0, 10_000)).unwrap_or(0)) / 10_000.0;
        let band = Rect::from_min_max(pos2(x0, yc - 1.5), pos2(x0 + (x1 - x0) * share, yc + 1.5));
        painter.rect_filled(band, 0.0, fc.palette.text_strong);
    }
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
    /// H-UI-22 : ligne de base dans le tiers inférieur, barre courante
    /// au-dessus, sans chevauchement ; entrée non finie sans NaN.
    #[test]
    fn h_ui_22_tracking_layout_lower_third() {
        let l = tracking_layout(100.0, 22.0);
        let lower = 100.0 + 22.0 * 2.0 / 3.0;
        assert!(l.baseline.0 >= lower && l.baseline.1 <= 122.0, "{l:?}");
        assert!(l.baseline.1 > l.baseline.0, "{l:?}");
        assert!(l.current.0 >= 100.0 && l.current.1 < lower, "{l:?}");
        assert!(l.current.1 > l.current.0 && l.current.1 < l.baseline.0, "{l:?}");
        for (top, h) in [(f32::NAN, 22.0), (0.0, f32::INFINITY), (0.0, 0.0), (0.0, -3.0)] {
            let l = tracking_layout(top, h);
            for v in [l.current.0, l.current.1, l.baseline.0, l.baseline.1] {
                assert!(v.is_finite(), "{top} {h} : {l:?}");
            }
            assert!(l.current.1 >= l.current.0 && l.baseline.1 >= l.baseline.0);
        }
        // Ligne très basse : bandes bornées à la ligne, jamais inversées.
        let l = tracking_layout(0.0, 3.0);
        assert!(
            l.current.1 >= l.current.0 && l.baseline.1 >= l.baseline.0,
            "{l:?}"
        );
        assert!(l.current.0 <= 3.0 && l.baseline.0 <= 3.0, "{l:?}");
    }

    /// H-UI-22 : bande sur [début, `Stop`], bornée à la barre ; vide avant
    /// le début ; non finie ⇒ rien. Exemple du cahier : barre de 4 jours à
    /// 12 px par jour, `Stop` au soir du 2e jour ⇒ 24 px.
    #[test]
    fn h_ui_22_progress_span_bounded() {
        assert_eq!(progress_span(100.0, 148.0, 124.0), Some((100.0, 124.0)));
        assert_eq!(progress_span(100.0, 148.0, 500.0), Some((100.0, 148.0)));
        assert_eq!(progress_span(100.0, 148.0, 100.0), None);
        assert_eq!(progress_span(100.0, 148.0, 50.0), None);
        assert_eq!(progress_span(f32::NAN, 148.0, 120.0), None);
        assert_eq!(progress_span(100.0, 148.0, f32::INFINITY), None);
    }

    /// H-UI-26 : une entrée, relue seulement quand la clé change.
    #[test]
    fn h_ui_26_tip_cache_one_entry() {
        let mut cache = ExplainTipCache::default();
        let a = (1, 0, TaskUid::new(3), SideJson::Start);
        let b = (1, 1, TaskUid::new(3), SideJson::Start);
        assert_eq!(cache.text_for(a, || "x".to_owned()), "x");
        assert_eq!(cache.text_for(a, || "y".to_owned()), "x");
        assert_eq!(cache.runs, 1);
        assert_eq!(cache.text_for(b, || "z".to_owned()), "z");
        assert_eq!(cache.text_for(a, || "w".to_owned()), "w");
        assert_eq!(cache.runs, 3);
    }

    /// Fumée : en-tête et corps dessinés sur l'exemple, à tous les zooms, y
    /// compris avec une découpe dégénérée, sans panique ni `Rect` non fini
    /// (egui vérifie par `debug_assert!`).
    #[test]
    fn gantt_paint_smoke_demo() {
        use planrs_core::{Day, Moment, TimeOfDay};

        use crate::app::{DocOrigin, OpenDocument};
        use crate::i18n::{Language, Texts};
        use crate::row_view::RowView;
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
        let row_view = RowView::unfiltered(&rows);
        let palette = Palette::LIGHT;
        let metrics = Metrics::default();
        let selection = rows.row(2).map(|r| r.uid);
        let selected = crate::selection::Selection::single(selection);
        let ctx = egui::Context::default();
        let styles = [
            crate::state::GanttStyle::Standard,
            crate::state::GanttStyle::Tracking,
        ];
        for (zoom, gantt_style) in TimeScaleZoom::ALL
            .into_iter()
            .flat_map(|z| styles.map(|s| (z, s)))
        {
            let state = UiState {
                zoom,
                gantt_style,
                ..UiState::default()
            };
            let mut actions = Vec::new();
            let mut stats = FrameStats::default();
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let mut fc = FrameCtx {
                    state: &state,
                    doc: Some(&doc),
                    rows: Some(&rows),
                    row_view: Some(&row_view),
                    texts: &texts,
                    palette: &palette,
                    metrics: &metrics,
                    diagnostics: &crate::view::DiagnosticsView::default(),
                    selection,
                    selected: &selected,
                    actions: &mut actions,
                    stats: &mut stats,
                    window_open: false,
                    task_view: crate::view::task_view::TaskViewFrame::default(),
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
