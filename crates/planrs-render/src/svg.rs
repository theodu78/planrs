//! Écriture SVG (lot L7).
//!
//! Document déterministe : ordre des éléments = ordre des ID, coordonnées
//! entières en 1/16 px écrites avec quatre décimales fixes, aucune police
//! système (largeurs estimées au nombre de caractères), tous les textes
//! échappés. Bornes : `max_rows` lignes, `max_days` jours, `max_bytes` octets
//! (arrêt à la ligne entière précédente). Hypothèses d'affichage H-R
//! (`docs/features/render.md`).

use std::fmt::{self, Write as _};

use planrs_core::{Project, Schedule};

use crate::axis::{self, Axis, DaySpan, SUB, push_fixed16};
use crate::request::{GanttRequest, Lang, RenderReport, Theme};
use crate::scene::{self, BAR_HALF, Ends, MILESTONE_HALF, ROW_H, RowKind, RowScene, Scene};

/// Hauteur de l'en-tête (deux niveaux de 22 px).
const HEADER_H: i64 = 44 * SUB;
/// Hauteur d'un niveau de l'en-tête.
const TIER_H: i64 = 22 * SUB;
/// Largeur de la colonne ID.
const ID_W: i64 = 40 * SUB;
/// Largeur de la table : colonne ID et colonne Nom de 240 px (début du
/// Gantt).
const TABLE_W: i64 = ID_W + 240 * SUB;
/// Retrait par niveau hiérarchique (12 px), plafonné à [`MAX_INDENT`].
const INDENT: i64 = 12 * SUB;
/// Retrait maximal d'un nom (120 px).
const MAX_INDENT: i64 = 120 * SUB;
/// Marge intérieure d'un texte (4 px).
const PAD: i64 = 4 * SUB;
/// Ligne de base d'un texte dans sa ligne (15 px sous le haut).
const BASELINE: i64 = 15 * SUB;
/// Ligne de base des titres de colonnes (27 px, milieu de l'en-tête).
const HEADER_BASELINE: i64 = 27 * SUB;

/// Couleur RVB, écrite `#RRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rgb(u8, u8, u8);

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

/// Couleurs du rendu, reprises de `Palette::LIGHT` / `Palette::DARK` de
/// l'interface (duplication consignée : `render` ne dépend pas d'egui).
#[derive(Clone, Copy, Debug)]
struct Colors {
    background: Rgb,
    text: Rgb,
    text_strong: Rgb,
    text_weak: Rgb,
    grid: Rgb,
    grid_major: Rgb,
    header_bg: Rgb,
    bar: Rgb,
    bar_outline: Rgb,
    critical: Rgb,
    milestone: Rgb,
    summary: Rgb,
    link: Rgb,
    status_date: Rgb,
    splitter: Rgb,
    summary_critical: Rgb,
}

const LIGHT: Colors = Colors {
    background: Rgb(0xFF, 0xFF, 0xFF),
    text: Rgb(0x1F, 0x1F, 0x1F),
    text_strong: Rgb(0x00, 0x00, 0x00),
    text_weak: Rgb(0x6B, 0x70, 0x78),
    grid: Rgb(0xE3, 0xE5, 0xE8),
    grid_major: Rgb(0xC4, 0xC8, 0xCE),
    header_bg: Rgb(0xF3, 0xF4, 0xF6),
    bar: Rgb(0x3F, 0x7C, 0xC4),
    bar_outline: Rgb(0x2B, 0x5C, 0x99),
    critical: Rgb(0xC9, 0x30, 0x2C),
    milestone: Rgb(0x26, 0x26, 0x26),
    summary: Rgb(0x26, 0x26, 0x26),
    link: Rgb(0x4A, 0x4F, 0x57),
    status_date: Rgb(0x7B, 0x3F, 0xB8),
    splitter: Rgb(0xC4, 0xC8, 0xCE),
    summary_critical: Rgb(0x8E, 0x1F, 0x1C),
};

const DARK: Colors = Colors {
    background: Rgb(0x1E, 0x1F, 0x22),
    text: Rgb(0xE6, 0xE6, 0xE6),
    text_strong: Rgb(0xFF, 0xFF, 0xFF),
    text_weak: Rgb(0x9A, 0xA0, 0xA8),
    grid: Rgb(0x33, 0x36, 0x3C),
    grid_major: Rgb(0x4A, 0x4E, 0x56),
    header_bg: Rgb(0x26, 0x28, 0x2C),
    bar: Rgb(0x6F, 0xA8, 0xE8),
    bar_outline: Rgb(0x4A, 0x86, 0xC8),
    critical: Rgb(0xFF, 0x6B, 0x66),
    milestone: Rgb(0xE6, 0xE6, 0xE6),
    summary: Rgb(0xE6, 0xE6, 0xE6),
    link: Rgb(0xB5, 0xBA, 0xC2),
    status_date: Rgb(0xC4, 0x9B, 0xFF),
    splitter: Rgb(0x4A, 0x4E, 0x56),
    summary_critical: Rgb(0xFF, 0x8A, 0x85),
};

/// Feuille de style : une classe par rôle.
fn style(c: &Colors) -> String {
    format!(
        "<style>\
.bg{{fill:{bg}}}.hd{{fill:{hd}}}\
text{{font-family:sans-serif;font-size:12px;fill:{t}}}\
.ts{{fill:{ts};font-weight:bold}}.tw{{fill:{tw}}}.tc{{fill:{tw};font-style:italic}}\
.gr{{stroke:{gr};fill:none}}.gm{{stroke:{gm};fill:none}}.sp{{stroke:{sp};fill:none}}\
.sd{{stroke:{sd};stroke-width:1.5;stroke-dasharray:6 4;fill:none}}\
.b{{fill:{b};stroke:{bo}}}.bc{{fill:{c};stroke:{c}}}\
.s{{fill:{s}}}.sc{{fill:{sc}}}.m{{fill:{m}}}.mc{{fill:{c}}}.x{{fill:none;stroke:{tw}}}\
.l{{fill:none;stroke:{l}}}.lc{{fill:none;stroke:{c}}}.la{{fill:{l}}}.lac{{fill:{c}}}\
</style>",
        bg = c.background,
        hd = c.header_bg,
        t = c.text,
        ts = c.text_strong,
        tw = c.text_weak,
        gr = c.grid,
        gm = c.grid_major,
        sp = c.splitter,
        sd = c.status_date,
        b = c.bar,
        bo = c.bar_outline,
        c = c.critical,
        s = c.summary,
        sc = c.summary_critical,
        m = c.milestone,
        l = c.link,
    )
}

/// Écrit `v` (1/16 px).
fn n(out: &mut String, v: i64) {
    push_fixed16(out, v);
}

/// Écrit une liste de points `x,y x,y …`.
fn points(out: &mut String, pts: &[(i64, i64)]) {
    for (i, (x, y)) in pts.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        n(out, *x);
        out.push(',');
        n(out, *y);
    }
}

/// Caractère échappé pour un texte XML : `&<>"'` en entités, tabulation et
/// fins de ligne en espace, caractères interdits par XML 1.0 en U+FFFD.
fn push_escaped_char(out: &mut String, c: char) {
    match c {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '"' => out.push_str("&quot;"),
        '\'' => out.push_str("&apos;"),
        '\t' | '\n' | '\r' => out.push(' '),
        c if c < ' ' || c == '\u{FFFE}' || c == '\u{FFFF}' => out.push('\u{FFFD}'),
        c => out.push(c),
    }
}

/// Texte échappé, tronqué à `max_chars` caractères suivis de « … ».
fn push_text(out: &mut String, s: &str, max_chars: usize) {
    for (i, c) in s.chars().enumerate() {
        if i >= max_chars {
            out.push('…');
            return;
        }
        push_escaped_char(out, c);
    }
}

/// Fragments d'une ligne, rangés par couche.
#[derive(Clone, Debug, Default)]
struct RowFrag {
    /// Liens dont cette ligne est la plus basse.
    links: String,
    /// Barre.
    bar: String,
    /// ID et nom.
    text: String,
}

impl RowFrag {
    fn len(&self) -> usize {
        self.links
            .len()
            .saturating_add(self.bar.len())
            .saturating_add(self.text.len())
    }
}

/// Ordonnée du haut de la ligne `i`.
fn row_top(i: usize) -> i64 {
    let i = i64::try_from(i).unwrap_or(i64::MAX);
    HEADER_H.saturating_add(i.saturating_mul(ROW_H))
}

/// Ordonnée du centre de la ligne `i`.
fn row_center(i: usize) -> i64 {
    row_top(i).saturating_add(ROW_H / 2)
}

/// Barre d'une ligne.
fn bar_frag(out: &mut String, row: &RowScene, yc: i64) {
    let Some((x0, x1)) = row.bar else {
        return;
    };
    let top = yc.saturating_sub(BAR_HALF);
    let h = BAR_HALF.saturating_mul(2);
    match row.kind {
        RowKind::Null => {}
        RowKind::Leaf => {
            let class = if row.critical { "bc" } else { "b" };
            let w = x1.saturating_sub(x0).max(SUB);
            out.push_str("<rect class=\"");
            out.push_str(class);
            out.push_str("\" x=\"");
            n(out, x0);
            out.push_str("\" y=\"");
            n(out, top);
            out.push_str("\" width=\"");
            n(out, w);
            out.push_str("\" height=\"");
            n(out, h);
            out.push_str("\" rx=\"2\"/>");
        }
        RowKind::Summary => {
            let class = if row.critical { "sc" } else { "s" };
            let x1 = x1.max(x0.saturating_add(SUB));
            let thick = (h / 3).max(2 * SUB);
            let w = BAR_HALF.min(x1.saturating_sub(x0) / 2).max(SUB);
            let depth = BAR_HALF.saturating_mul(8) / 5;
            let bottom = top.saturating_add(thick);
            let tip = top.saturating_add(depth);
            out.push_str("<path class=\"");
            out.push_str(class);
            out.push_str("\" d=\"M");
            points(out, &[(x0, top), (x1, top), (x1, bottom), (x0, bottom)]);
            out.push_str("Z M");
            points(out, &[(x0, top), (x0.saturating_add(w), top), (x0, tip)]);
            out.push_str("Z M");
            points(out, &[(x1.saturating_sub(w), top), (x1, top), (x1, tip)]);
            out.push_str("Z\"/>");
        }
        RowKind::Milestone => {
            let class = if row.critical { "mc" } else { "m" };
            diamond(out, class, x0, yc);
        }
        RowKind::Excluded => {
            if x1.saturating_sub(x0) <= SUB {
                diamond(out, "x", x0, yc);
            } else {
                out.push_str("<rect class=\"x\" x=\"");
                n(out, x0);
                out.push_str("\" y=\"");
                n(out, top);
                out.push_str("\" width=\"");
                n(out, x1.saturating_sub(x0));
                out.push_str("\" height=\"");
                n(out, h);
                out.push_str("\" rx=\"2\"/>");
            }
        }
    }
}

/// Losange centré en `(x, y)`.
fn diamond(out: &mut String, class: &str, x: i64, y: i64) {
    let d = MILESTONE_HALF;
    out.push_str("<polygon class=\"");
    out.push_str(class);
    out.push_str("\" points=\"");
    points(
        out,
        &[
            (x, y.saturating_sub(d)),
            (x.saturating_add(d), y),
            (x, y.saturating_add(d)),
            (x.saturating_sub(d), y),
        ],
    );
    out.push_str("\"/>");
}

/// ID et nom d'une ligne.
fn text_frag(out: &mut String, row: &RowScene, i: usize, name_chars: usize) {
    let y = row_top(i).saturating_add(BASELINE);
    let class = match row.kind {
        RowKind::Summary => " class=\"ts\"",
        RowKind::Null | RowKind::Excluded => " class=\"tw\"",
        RowKind::Leaf | RowKind::Milestone => "",
    };
    let _ = write!(out, "<text{class} x=\"");
    n(out, ID_W.saturating_sub(PAD));
    out.push_str("\" y=\"");
    n(out, y);
    let _ = write!(out, "\" text-anchor=\"end\">{}</text>", row.id);
    if row.name.is_empty() {
        return;
    }
    let level = i64::from(row.level.saturating_sub(1));
    let indent = level.saturating_mul(INDENT).min(MAX_INDENT);
    let _ = write!(out, "<text{class} x=\"");
    n(out, ID_W.saturating_add(PAD).saturating_add(indent));
    out.push_str("\" y=\"");
    n(out, y);
    out.push_str("\">");
    push_text(out, &row.name, name_chars);
    out.push_str("</text>");
}

/// Liens d'un lien de la scène.
fn link_frag(out: &mut String, scene: &Scene, link: &scene::LinkScene) {
    let ends = |i: usize| scene.rows.get(i).and_then(|r| Ends::of(r, row_center(i)));
    let (Some(p), Some(s)) = (ends(link.from), ends(link.to)) else {
        return;
    };
    let path = scene::link_path(link.kind, p, s);
    let Some(&tip) = path.points.last() else {
        return;
    };
    let (stroke, head) = if link.critical { ("lc", "lac") } else { ("l", "la") };
    let _ = write!(out, "<polyline class=\"{stroke}\" points=\"");
    points(out, &path.points);
    let _ = write!(out, "\"/><polygon class=\"{head}\" points=\"");
    points(out, &scene::arrow_points(tip, path.dir));
    out.push_str("\"/>");
}

/// Libellés fixes.
struct Labels {
    id: &'static str,
    name: &'static str,
}

const fn labels(lang: Lang) -> Labels {
    match lang {
        Lang::Fr => Labels {
            id: "N°",
            name: "Nom de la tâche",
        },
        Lang::En => Labels {
            id: "ID",
            name: "Task Name",
        },
    }
}

/// Ligne de troncature : « … N tâches non affichées ».
fn truncation_text(lang: Lang, count: usize) -> String {
    match (lang, count) {
        (Lang::Fr, 1) => "… 1 tâche non affichée".to_owned(),
        (Lang::Fr, c) => format!("… {c} tâches non affichées"),
        (Lang::En, 1) => "… 1 task not shown".to_owned(),
        (Lang::En, c) => format!("… {c} tasks not shown"),
    }
}

/// Contexte d'écriture du document.
struct Doc<'a> {
    project: &'a Project,
    req: &'a GanttRequest,
    span: DaySpan,
    axis: Axis,
    colors: &'static Colors,
    name_chars: usize,
}

impl Doc<'_> {
    /// Document complet : `frags` écrits (toutes leurs couches), hauteur de
    /// `height_rows` lignes, ligne de troncature `cut` (nombre de tâches non
    /// affichées) sur la dernière d'entre elles.
    fn write(&self, frags: &[RowFrag], height_rows: usize, cut: Option<usize>) -> String {
        let mut out = String::new();
        let width = TABLE_W.saturating_add(self.axis.width16());
        let height = row_top(height_rows);
        out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"");
        n(&mut out, width);
        out.push_str("\" height=\"");
        n(&mut out, height);
        out.push_str("\" viewBox=\"0 0 ");
        n(&mut out, width);
        out.push(' ');
        n(&mut out, height);
        out.push_str("\"><title>");
        push_text(&mut out, &self.project.info.name, self.name_chars);
        out.push_str("</title>");
        out.push_str(&style(self.colors));
        // Découpes : textes dans la table, Gantt à droite de la table.
        out.push_str("<defs><clipPath id=\"n\"><rect x=\"0.0000\" y=\"0.0000\" width=\"");
        n(&mut out, TABLE_W);
        out.push_str("\" height=\"");
        n(&mut out, height);
        out.push_str("\"/></clipPath><clipPath id=\"g\"><rect x=\"");
        n(&mut out, TABLE_W);
        out.push_str("\" y=\"0.0000\" width=\"");
        n(&mut out, self.axis.width16());
        out.push_str("\" height=\"");
        n(&mut out, height);
        out.push_str("\"/></clipPath></defs>");
        // Fond et en-tête.
        out.push_str("<rect class=\"bg\" width=\"100%\" height=\"100%\"/><rect class=\"hd\" width=\"");
        n(&mut out, width);
        out.push_str("\" height=\"");
        n(&mut out, HEADER_H);
        out.push_str("\"/>");
        let l = labels(self.req.language);
        let header_y = HEADER_BASELINE;
        out.push_str("<text class=\"ts\" x=\"");
        n(&mut out, ID_W.saturating_sub(PAD));
        out.push_str("\" y=\"");
        n(&mut out, header_y);
        let _ = write!(
            out,
            "\" text-anchor=\"end\">{}</text><text class=\"ts\" x=\"",
            l.id
        );
        n(&mut out, ID_W.saturating_add(PAD));
        out.push_str("\" y=\"");
        n(&mut out, header_y);
        let _ = write!(out, "\">{}</text>", l.name);
        self.write_timescale(&mut out, height);
        self.write_status_date(&mut out, height);
        // Séparateurs de la table et du bas de l'en-tête.
        out.push_str("<path class=\"sp\" d=\"M");
        points(&mut out, &[(ID_W, 0)]);
        out.push('V');
        n(&mut out, height);
        out.push_str(" M");
        points(&mut out, &[(TABLE_W, 0)]);
        out.push('V');
        n(&mut out, height);
        out.push_str(" M");
        points(&mut out, &[(0, HEADER_H)]);
        out.push('H');
        n(&mut out, width);
        out.push_str("\"/>");
        // Couches des lignes : liens, barres (découpe du Gantt), textes.
        out.push_str("<g clip-path=\"url(#g)\">");
        for f in frags {
            out.push_str(&f.links);
        }
        for f in frags {
            out.push_str(&f.bar);
        }
        out.push_str("</g><g clip-path=\"url(#n)\">");
        for f in frags {
            out.push_str(&f.text);
        }
        out.push_str("</g>");
        if let Some(count) = cut {
            out.push_str("<text class=\"tc\" x=\"");
            n(&mut out, PAD);
            out.push_str("\" y=\"");
            n(
                &mut out,
                row_top(height_rows.saturating_sub(1)).saturating_add(BASELINE),
            );
            out.push_str("\">");
            push_text(&mut out, &truncation_text(self.req.language, count), usize::MAX);
            out.push_str("</text>");
        }
        out.push_str("</svg>");
        out
    }

    /// En-tête à deux niveaux et grille verticale du corps.
    fn write_timescale(&self, out: &mut String, height: i64) {
        let (major, minor) = self.axis_scale().tiers();
        let week_start = self.project.options.week_start_day;
        for (unit, class, y0) in [(major, "gm", 0), (minor, "gr", TIER_H)] {
            let cells = axis::tier_cells(&self.axis, self.span, unit, week_start, self.req.language);
            // Grille : une seule `path` par niveau.
            let mut grid = String::new();
            for c in &cells {
                if c.boundary && c.x0 > self.axis.left16() {
                    grid.push('M');
                    points(&mut grid, &[(c.x0, y0)]);
                    grid.push('V');
                    n(&mut grid, height);
                }
            }
            if !grid.is_empty() {
                let _ = write!(out, "<path class=\"{class}\" d=\"{grid}\"/>");
            }
            out.push_str("<g clip-path=\"url(#g)\">");
            // Une étiquette qui ne tient pas dans sa cellule rognée (bord de
            // plage) n'est pas écrite.
            let fits = |c: &&axis::Cell| {
                !c.label.is_empty() && axis::estimated_width16(&c.label) <= c.x1.saturating_sub(c.x0)
            };
            for c in cells.iter().filter(fits) {
                out.push_str("<text x=\"");
                n(out, c.x0.saturating_add(3 * SUB));
                out.push_str("\" y=\"");
                n(out, y0.saturating_add(BASELINE));
                out.push_str("\">");
                push_text(out, &c.label, usize::MAX);
                out.push_str("</text>");
            }
            out.push_str("</g>");
        }
    }

    /// Ligne en tirets de la date d'état, si elle est dans la plage.
    fn write_status_date(&self, out: &mut String, height: i64) {
        let x = self.axis.x16(self.project.effective_status_date());
        if x >= self.axis.left16() && x <= self.axis.right16() {
            out.push_str("<path class=\"sd\" d=\"M");
            points(out, &[(x, HEADER_H)]);
            out.push('V');
            n(out, height);
            out.push_str("\"/>");
        }
    }

    fn axis_scale(&self) -> axis::Scale {
        axis::resolve_scale(self.req.zoom, self.span.days())
    }
}

/// Plus petit document : rendu quand même l'en-tête dépasse `max_bytes`.
const MINIMAL: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"0\" height=\"0\"/>";

/// SVG du Gantt et rapport. Infaillible : toute borne atteinte est signalée
/// dans le rapport.
///
/// Lignes : celles du filtre, dans l'ordre des ID, au plus `max_rows` ; au
/// delà, une dernière ligne « … N tâches non affichées ». Si le document
/// dépasse `max_bytes`, il s'arrête à la dernière ligne entière qui tient,
/// suivie de la ligne de troncature ; si même l'en-tête ne tient pas, un
/// document vide (`width="0"`) est rendu, `truncated_bytes` vrai.
#[must_use]
pub fn svg(project: &Project, schedule: &Schedule, req: &GanttRequest) -> (String, RenderReport) {
    let span = axis::resolve_range(req.range, project, schedule, req.limits.max_days);
    let scale = axis::resolve_scale(req.zoom, span.days());
    let axis = Axis::new(span, scale, TABLE_W);
    let scene = scene::build(project, schedule, &req.filter, &axis);
    let name_chars = usize::from(req.limits.name_chars);
    let max_rows = usize::try_from(req.limits.max_rows).unwrap_or(usize::MAX);
    let candidates = scene.rows.len().min(max_rows);

    let mut frags: Vec<RowFrag> = vec![RowFrag::default(); candidates];
    for (i, (row, frag)) in scene.rows.iter().zip(frags.iter_mut()).enumerate() {
        bar_frag(&mut frag.bar, row, row_center(i));
        text_frag(&mut frag.text, row, i, name_chars);
    }
    for link in &scene.links {
        if let Some(frag) = frags.get_mut(link.last_row()) {
            link_frag(&mut frag.links, &scene, link);
        }
    }

    let doc = Doc {
        project,
        req,
        span,
        axis,
        colors: match req.theme {
            Theme::Light => &LIGHT,
            Theme::Dark => &DARK,
        },
        name_chars,
    };
    let max_bytes = req.limits.max_bytes;
    let row_cut = scene.rows.len().saturating_sub(candidates);
    let total = scene.total;
    let report = |out: &str, shown: usize, truncated_bytes: bool| RenderReport {
        shown: u32::try_from(shown).unwrap_or(u32::MAX),
        hidden: u32::try_from(total.saturating_sub(shown)).unwrap_or(u32::MAX),
        truncated_days: span.truncated,
        truncated_bytes,
        bytes: out.len(),
    };

    // Tout tient ?
    let cut = (row_cut > 0).then_some(row_cut);
    let height = candidates.saturating_add(usize::from(cut.is_some()));
    let full = doc.write(&frags, height, cut);
    if full.len() <= max_bytes {
        let r = report(&full, candidates, false);
        return (full, r);
    }

    // Coupe à l'octet : majorant de la partie fixe (hauteur et compteur
    // maximaux), puis plus grand préfixe de lignes qui tient.
    let fixed = doc.write(&[], candidates.saturating_add(1), Some(total)).len();
    let mut used = fixed;
    let mut k = 0_usize;
    if fixed <= max_bytes {
        for f in &frags {
            let next = used.saturating_add(f.len());
            if next > max_bytes {
                break;
            }
            used = next;
            k = k.saturating_add(1);
        }
    } else {
        let out = MINIMAL.to_owned();
        let r = report(&out, 0, true);
        return (out, r);
    }
    let shown = frags.get(..k).unwrap_or(&[]);
    let hidden_by_bounds = scene.rows.len().saturating_sub(k);
    let out = doc.write(shown, k.saturating_add(1), Some(hidden_by_bounds));
    let r = report(&out, k, true);
    (out, r)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use super::*;

    #[test]
    fn escape_and_truncate() {
        let mut s = String::new();
        push_text(&mut s, "a<b>&\"'\u{1}\u{FFFF}\n", 100);
        assert_eq!(s, "a&lt;b&gt;&amp;&quot;&apos;\u{FFFD}\u{FFFD} ");
        let mut s = String::new();
        push_text(&mut s, "abcdef", 3);
        assert_eq!(s, "abc…");
        let mut s = String::new();
        push_text(&mut s, "abc", 3);
        assert_eq!(s, "abc");
    }

    #[test]
    fn colors_are_hex() {
        assert_eq!(LIGHT.critical.to_string(), "#C9302C");
        assert_eq!(scene::ARROW_HALF, 51);
    }
}
