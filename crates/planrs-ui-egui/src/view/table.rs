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

use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat, TextWrapping};
use egui::{
    Color32, Event, EventFilter, FontId, Galley, Id, Key, Modifiers, Order, Painter, Pos2, Rect, Response,
    Sense, Stroke, StrokeKind, UiBuilder, pos2, vec2,
};
use planrs_core::TaskUid;

use super::{FrameCtx, Pane};
use crate::action::Action;
use crate::column::{CellAlign, Column};
use crate::edit::{EditState, TableEdit, commit_edit, editable, initial_text, text_language};
use crate::geom::index_to_f32;
use crate::i18n::{Arg, TextId};
use crate::nav::{CellCursor, NavMove, RowRef};
use crate::rows::{Row, RowKind, RowModel};
use crate::theme::Palette;

/// Curseur de la table (cellule courante).
pub type TableCursor = CellCursor<TaskUid, Column>;

/// Identifiant focalisable de la table (J7, lot L1) : tant qu'il a le
/// focus clavier, les raccourcis de navigation (`KeyContext::Table`) sont
/// actifs.
#[must_use]
pub fn table_id() -> Id {
    Id::unique("planrs.table")
}

/// Verrou de focus de la table, rappelé à chaque image : Tab et les
/// flèches restent à la table (egui les traite en début d'image, avant
/// l'application) ; Échap rend le focus (ADR-0015 D2).
const NAV_FOCUS_FILTER: EventFilter = EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: false,
};

/// Épaisseur du cadre de la cellule courante, en points.
const CURSOR_STROKE: f32 = 2.0;

/// Vrai si un autre widget que la table a le focus clavier (zone de
/// saisie, bouton parcouru au clavier) : la frappe ne s'adresse pas à la
/// table.
fn focus_elsewhere(ctx: &egui::Context) -> bool {
    ctx.memory(|memory| memory.focused().is_some_and(|id| id != table_id()))
}

/// Largeur en dessous de laquelle une cellule n'affiche pas de texte.
const MIN_TEXT_WIDTH: f32 = 4.0;

/// Décalage du second tracé qui simule le gras des récapitulatives.
const FAUX_BOLD_DX: f32 = 0.6;

/// Largeur de la colonne Nom toujours laissée au texte, quel que soit le
/// niveau hiérarchique : l'indentation est plafonnée à `largeur − 40` (un
/// nom au niveau 200 reste lisible).
const NAME_MIN_TEXT_ROOM: f32 = 40.0;

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

/// Couleur du texte d'une cellule. Comme MS Project, la table n'est pas
/// colorée par la criticité (D-fix-7) : seule la colonne « Critique » d'une
/// tâche critique est en rouge (la barre du Gantt l'est aussi) ; les
/// récapitulatives sont en texte appuyé (et en gras simulé au dessin).
fn cell_text_color(row: &Row, column: Column, palette: &Palette) -> Color32 {
    match row.kind {
        RowKind::Null | RowKind::Excluded => palette.text_weak,
        _ if row.critical && column == Column::Critical => palette.critical,
        RowKind::Summary => palette.text_strong,
        RowKind::Leaf | RowKind::Milestone => palette.text,
    }
}

/// Retrait du nom au niveau `level`, plafonné pour laisser
/// [`NAME_MIN_TEXT_ROOM`] au texte dans une colonne de largeur `w`.
fn name_indent(indent_per_level: f32, level: u16, w: f32) -> f32 {
    let wanted = indent_per_level * f32::from(level.saturating_sub(1));
    let cap = (w - NAME_MIN_TEXT_ROOM).max(0.0);
    if wanted.is_finite() { wanted.min(cap) } else { 0.0 }
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
            name_indent(metrics.indent, row.level, w)
        } else {
            0.0
        };
        // Troncature au bord visible de la table si la colonne le dépasse.
        let visible_w = w.min(right - cell_left);
        let max_w = visible_w - 2.0 * pad - indent;
        if !max_w.is_finite() || max_w < MIN_TEXT_WIDTH {
            continue;
        }
        let galley = truncated_galley(painter, text, font.clone(), max_w);
        let size = galley.size();
        let text_x = aligned_x(state.column.align(), cell_left + indent, w - indent, pad, size.x);
        let text_y = row_rect.top() + (row_rect.height() - size.y) * 0.5;
        let color = cell_text_color(row, state.column, fc.palette);
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
/// Fond alterné léger, ligne sélectionnée surlignée, texte en couleur
/// normale (rouge seulement dans la colonne « Critique »), nom indenté par
/// niveau, récapitulative en texte appuyé, ligne vide sans texte. Chaque
/// ligne est une zone cliquable (`Action::Select`, colonne courante posée
/// dans `cursor`) qui porte le libellé d'accessibilité `row-label`. La ligne
/// d'indice `rows.len()` est la ligne vide de fin (création d'une tâche,
/// libellé `blank-row-label`). La table entière est un widget focalisable
/// ([`table_id`]) ; la cellule courante porte un cadre de 2 px et un nœud
/// d'accessibilité (« Ligne 3, Durée, 5 jours »). Saisie des cellules : voir
/// [`edit_cells`]. Écrit `FrameStats::table_rows` (et lui seul : le Gantt
/// écrit `gantt_rows`).
pub fn paint_body(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    edit: &mut TableEdit,
    cursor: &mut TableCursor,
    origin: Pos2,
    range: Range<usize>,
    clip: Rect,
) {
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
    focus_widget(ui, fc, clip);
    open_from_keys(ui, fc, rows, edit, cursor);
    fc.stats.table_rows = range.clone();
    let left = clip.left();
    let blank = rows.len();
    for i in range.clone() {
        let top = origin.y + index_to_f32(i) * row_h;
        let row_rect = Rect::from_min_size(pos2(left, top), vec2(clip.width(), row_h));
        if !row_rect.is_finite() {
            continue;
        }
        let row = rows.row(i);
        if row.is_none() && i != blank {
            continue;
        }
        let selected = match row {
            Some(row) => fc.selected.contains(row.uid),
            None => fc.selection.is_none(),
        };
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

        if let Some(row) = row
            && row.kind != RowKind::Null
        {
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
            response.widget_info(|| {
                let label = match row {
                    Some(row) => texts.get_args(
                        TextId::RowLabel,
                        &[
                            ("id", Arg::Count(i64::from(row.id))),
                            ("name", Arg::Str(rows.text(i, Column::Name).into())),
                        ],
                    ),
                    None => texts.get(TextId::BlankRowLabel),
                };
                egui::WidgetInfo::selected(egui::Role::Row, true, selected, label)
            });
            row_clicked(fc, rows, edit, cursor, &response, i, left);
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
    shapes = shapes.saturating_add(paint_cursor(ui, &painter, fc, rows, edit, cursor, origin, &range));
    fc.stats.shapes = fc.stats.shapes.saturating_add(shapes);
    edit_cells(ui, fc, rows, edit, cursor, origin, &range, clip);
}

/// Widget focalisable de la table (sans clic : les lignes, dessinées
/// au-dessus, reçoivent les clics) ; verrou de focus rappelé tant qu'il a
/// le focus.
fn focus_widget(ui: &egui::Ui, fc: &FrameCtx<'_>, clip: Rect) {
    let response = ui.interact(clip, table_id(), Sense::focusable_noninteractive());
    let texts = fc.texts;
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Table, true, texts.get(TextId::TableLabel)));
    if response.has_focus() {
        ui.memory_mut(|memory| memory.set_focus_lock_filter(table_id(), NAV_FOCUS_FILTER));
    }
}

/// Indice de la ligne du curseur (ligne vide : `rows.len()`) ; `None` si
/// sa tâche n'est plus dans le modèle.
fn cursor_index(rows: &RowModel, cursor: &TableCursor) -> Option<usize> {
    match cursor.row {
        RowRef::Item(uid) => rows.index_of(uid),
        RowRef::Blank => Some(rows.len()),
    }
}

/// Cadre de 2 px de la cellule courante (couleur `Palette::cursor`) et
/// son nœud d'accessibilité, si elle est dans la plage dessinée, sa colonne
/// visible et aucune saisie ouverte ; rend le nombre de formes ajoutées.
#[allow(
    clippy::too_many_arguments,
    reason = "contexte d'image découpé en emprunts disjoints"
)]
fn paint_cursor(
    ui: &egui::Ui,
    painter: &Painter,
    fc: &FrameCtx<'_>,
    rows: &RowModel,
    edit: &TableEdit,
    cursor: &TableCursor,
    origin: Pos2,
    range: &Range<usize>,
) -> usize {
    if edit.editing.is_some() {
        return 0;
    }
    let clip = painter.clip_rect();
    let Some(index) = cursor_index(rows, cursor).filter(|i| range.contains(i)) else {
        return 0;
    };
    let Some((left, w)) = column_span(fc, clip.left(), cursor.column) else {
        return 0;
    };
    let top = origin.y + index_to_f32(index) * fc.metrics.row_h;
    let rect = Rect::from_min_size(pos2(left, top), vec2(w, fc.metrics.row_h));
    if !rect.is_finite() || !rect.is_positive() {
        return 0;
    }
    painter.rect_stroke(
        rect.shrink(CURSOR_STROKE * 0.5),
        0.0,
        Stroke::new(CURSOR_STROKE, fc.palette.cursor),
        StrokeKind::Inside,
    );
    let hit = rect.intersect(clip);
    if hit.is_positive() {
        let response = ui.interact(hit, Id::unique("planrs.table.cursor"), Sense::hover());
        let texts = fc.texts;
        let column = texts.get(cursor.column.title());
        response.widget_info(|| {
            let label = match rows.row(index) {
                Some(row) => {
                    let value = rows.text(index, cursor.column);
                    let value = if value.is_empty() {
                        texts.get(TextId::NavCursorEmptyValue)
                    } else {
                        value.to_owned()
                    };
                    texts.get_args(
                        TextId::NavCursorLabel,
                        &[
                            ("id", Arg::Count(i64::from(row.id))),
                            ("column", Arg::Str(column.clone().into())),
                            ("value", Arg::Str(value.into())),
                        ],
                    )
                }
                None => texts.get_args(
                    TextId::NavCursorBlankLabel,
                    &[("column", Arg::Str(column.clone().into()))],
                ),
            };
            egui::WidgetInfo::labeled(egui::Role::Cell, true, label)
        });
    }
    1
}

/// Colonne visible sous l'abscisse écran `x`, la table commençant à `left`.
fn column_at(fc: &FrameCtx<'_>, left: f32, x: f32) -> Option<Column> {
    let mut l = left;
    for state in fc.state.visible_columns() {
        let w = f32::from(state.width);
        if x >= l && x < l + w {
            return Some(state.column);
        }
        l += w;
    }
    None
}

/// Bord gauche et largeur de la colonne visible `column`.
fn column_span(fc: &FrameCtx<'_>, left: f32, column: Column) -> Option<(f32, f32)> {
    let mut l = left;
    for state in fc.state.visible_columns() {
        let w = f32::from(state.width);
        if state.column == column {
            return Some((l, w));
        }
        l += w;
    }
    None
}

/// Clic sur la ligne `i` (ou sur la ligne vide) : sélection, focus de la
/// table, colonne courante sous le pointeur ; double-clic : ouverture de la
/// saisie.
fn row_clicked(
    fc: &mut FrameCtx<'_>,
    rows: &RowModel,
    edit: &mut TableEdit,
    cursor: &mut TableCursor,
    response: &Response,
    i: usize,
    left: f32,
) {
    if !response.clicked() {
        return;
    }
    let uid = rows.row(i).map(|row| row.uid);
    fc.actions.push(Action::Select(uid));
    fc.actions.push(Action::FocusPane(Pane::Table));
    if let Some(column) = response
        .interact_pointer_pos()
        .and_then(|p| column_at(fc, left, p.x))
    {
        cursor.column = column;
    }
    // Double-clic (ou triple : deux clics suivant de près un premier).
    if (response.double_clicked() || response.triple_clicked()) && edit.editing.is_none() {
        open_editor(fc, rows, edit, cursor.column, uid, i);
    }
}

/// Ouvre la saisie de la cellule (`uid`, `column`) avec son texte initial
/// (F2, double-clic) ; rien sur une colonne calculée.
fn open_editor(
    fc: &FrameCtx<'_>,
    rows: &RowModel,
    edit: &mut TableEdit,
    column: Column,
    uid: Option<TaskUid>,
    i: usize,
) {
    let Some(doc) = fc.doc else { return };
    if !editable(column) {
        return;
    }
    let text = initial_text(doc, rows, i, column, fc.texts.language());
    edit.start(EditState::open(uid, column, text));
}

/// Ligne courante pour la saisie : tâche sélectionnée, ou ligne vide sans
/// sélection.
fn current_row(fc: &FrameCtx<'_>, rows: &RowModel) -> Option<(Option<TaskUid>, usize)> {
    match fc.selection {
        Some(uid) => rows.index_of(uid).map(|i| (Some(uid), i)),
        None => Some((None, rows.len())),
    }
}

/// F2 ou frappe d'un caractère (hors champ de texte ou autre widget
/// focalisé, menu ouvert ou modificateur Ctrl/Cmd) : ouvre la saisie de la
/// cellule courante. Le caractère frappé remplace la valeur.
fn open_from_keys(
    ui: &egui::Ui,
    fc: &FrameCtx<'_>,
    rows: &RowModel,
    edit: &mut TableEdit,
    cursor: &TableCursor,
) {
    let ctx = ui.ctx();
    if edit.editing.is_some() || focus_elsewhere(ctx) || ctx.any_popup_open() {
        return;
    }
    let Some((uid, i)) = current_row(fc, rows) else {
        return;
    };
    let column = cursor.column;
    if !editable(column) || !fc.state.visible_columns().any(|state| state.column == column) {
        return;
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F2)) {
        open_editor(fc, rows, edit, column, uid, i);
        return;
    }
    let typed = ctx.input_mut(|input| {
        let mut typed = String::new();
        if input.modifiers.command || input.modifiers.ctrl || input.modifiers.mac_cmd {
            return typed;
        }
        input.events.retain(|event| match event {
            Event::Text(text) if !text.is_empty() && !text.chars().any(char::is_control) => {
                typed.push_str(text);
                false
            }
            _ => true,
        });
        typed
    });
    if !typed.is_empty() {
        edit.start(EditState::typed(uid, column, &typed));
    }
}

/// Identifiant de la zone de saisie (une seule saisie à la fois).
fn editor_id() -> Id {
    Id::unique("planrs-cell-editor")
}

/// Fin d'une saisie demandée par l'utilisateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Finish {
    /// Échap : saisie abandonnée.
    Cancel,
    /// Validation, puis déplacement de la cellule courante.
    Commit(Move),
}

/// Déplacement après validation (H-UI-14 : Tab passe à la colonne
/// **visible** voisine, colonnes calculées comprises).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Move {
    /// Entrée, flèche bas : ligne suivante.
    Down,
    /// Flèche haut : ligne précédente.
    Up,
    /// Tab : colonne visible suivante.
    Next,
    /// Maj+Tab : colonne visible précédente.
    Previous,
    /// Défilement, perte du focus : la cellule reste.
    Stay,
}

impl Move {
    /// Déplacement du curseur correspondant (`None` : la cellule reste).
    const fn nav(self) -> Option<NavMove> {
        match self {
            Self::Down => Some(NavMove::Down),
            Self::Up => Some(NavMove::Up),
            Self::Next => Some(NavMove::Right),
            Self::Previous => Some(NavMove::Left),
            Self::Stay => None,
        }
    }
}

/// Touches de fin de saisie, consommées avant la zone de saisie (qui ne
/// les voit donc pas). Les plus précises d'abord (Maj+Tab avant Tab).
fn finish_keys(ui: &egui::Ui) -> Option<Finish> {
    ui.input_mut(|input| {
        if input.consume_key(Modifiers::NONE, Key::Escape) {
            Some(Finish::Cancel)
        } else if input.consume_key(Modifiers::SHIFT, Key::Tab) {
            Some(Finish::Commit(Move::Previous))
        } else if input.consume_key(Modifiers::NONE, Key::Tab) {
            Some(Finish::Commit(Move::Next))
        } else if input.consume_key(Modifiers::NONE, Key::Enter)
            || input.consume_key(Modifiers::NONE, Key::ArrowDown)
        {
            Some(Finish::Commit(Move::Down))
        } else if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
            Some(Finish::Commit(Move::Up))
        } else {
            None
        }
    })
}

/// Saisie de la cellule ouverte : zone de saisie posée sur la cellule,
/// message d'erreur dessous, fin de saisie. Entrée valide et descend, Tab
/// valide et passe à la cellule visible suivante, Échap abandonne ; un
/// défilement ou la perte du focus valident sur place. Une saisie refusée
/// ([`commit_edit`]) reste ouverte avec son message ; acceptée, son lot est
/// poussé dans `Action::ExecOn` (refus du registre : bandeau), suivi du
/// déplacement du curseur (`Action::Nav`, appliqué après le lot).
#[allow(
    clippy::too_many_arguments,
    reason = "contexte d'image découpé en emprunts disjoints"
)]
fn edit_cells(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    rows: &RowModel,
    edit: &mut TableEdit,
    cursor: &mut TableCursor,
    origin: Pos2,
    range: &Range<usize>,
    clip: Rect,
) {
    let Some(state) = edit.editing.as_ref() else {
        return;
    };
    let index = match state.uid {
        Some(uid) => rows.index_of(uid),
        None => Some(rows.len()),
    };
    let span = column_span(fc, clip.left(), state.column);
    let (Some(index), Some((cell_left, cell_w))) = (index, span) else {
        // Tâche disparue ou colonne masquée : saisie abandonnée.
        close_editor(ui, edit);
        return;
    };
    let mut finish = finish_keys(ui);
    let scrolled = ui.input(|i| {
        i.smooth_scroll_delta.y.abs() > 0.0
            || i.events
                .iter()
                .any(|e| matches!(e, Event::MouseWheel { delta, .. } if delta.y.abs() > 0.0))
    });
    if finish.is_none() && scrolled {
        finish = Some(Finish::Commit(Move::Stay));
    }
    if range.contains(&index) {
        let top = origin.y + index_to_f32(index) * fc.metrics.row_h;
        let rect = Rect::from_min_size(pos2(cell_left, top), vec2(cell_w, fc.metrics.row_h));
        let lost = show_editor(ui, fc, edit, rect, clip);
        if finish.is_none() && lost {
            finish = Some(Finish::Commit(Move::Stay));
        }
    }
    match finish {
        None => {}
        Some(Finish::Cancel) => {
            close_editor(ui, edit);
            // Échap rend la main à la table (navigation au clavier).
            fc.actions.push(Action::FocusPane(Pane::Table));
        }
        Some(Finish::Commit(motion)) => commit(ui, fc, edit, cursor, motion),
    }
}

/// Dessine la zone de saisie dans `rect` (et l'erreur dessous) ; rend vrai
/// si elle vient de perdre le focus.
fn show_editor(ui: &mut egui::Ui, fc: &FrameCtx<'_>, edit: &mut TableEdit, rect: Rect, clip: Rect) -> bool {
    let TableEdit {
        editing,
        focus_pending,
        ..
    } = edit;
    let Some(state) = editing.as_mut() else {
        return false;
    };
    if !rect.is_finite() || !rect.is_positive() {
        return false;
    }
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
    child.set_clip_rect(clip.intersect(ui.clip_rect()));
    let label = fc.texts.get_args(
        TextId::CellEditLabel,
        &[("column", Arg::Str(fc.texts.get(state.column.title()).into()))],
    );
    let response = child.add_sized(
        rect.size(),
        egui::TextEdit::singleline(&mut state.buffer)
            .id(editor_id())
            .lock_focus(true)
            .font(egui::TextStyle::Body)
            .margin(egui::Margin::symmetric(3, 2)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, label.clone()));
    if *focus_pending {
        response.request_focus();
        // Curseur en fin de texte (F2 comme la frappe d'un caractère).
        if let Some(mut text_state) = egui::text_edit::TextEditState::load(ui.ctx(), editor_id()) {
            let end = CCursor::new(state.buffer.chars().count());
            text_state.cursor.set_char_range(Some(CCursorRange::one(end)));
            text_state.store(ui.ctx(), editor_id());
        }
        *focus_pending = false;
    }
    if response.changed() {
        state.error = None;
    }
    if let Some(error) = &state.error {
        let message = error.message(fc.texts);
        let color = fc.palette.critical;
        egui::Area::new(Id::unique("planrs-cell-error"))
            .order(Order::Foreground)
            .fixed_pos(pos2(rect.left(), rect.bottom()))
            .interactable(false)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(egui::RichText::new(message).color(color));
                });
            });
    }
    response.lost_focus()
}

/// Ferme la saisie et rend le focus (les raccourcis redeviennent actifs).
fn close_editor(ui: &egui::Ui, edit: &mut TableEdit) {
    edit.editing = None;
    edit.focus_pending = false;
    ui.memory_mut(|memory| memory.surrender_focus(editor_id()));
}

/// Valide la saisie : refus → message sous la cellule, saisie gardée ;
/// accord → lot poussé (s'il n'est pas vide), saisie fermée, curseur posé
/// sur la cellule éditée puis déplacé (`Action::Nav`, après le lot : une
/// tâche créée par la ligne vide la précède, la ligne vide reste la ligne
/// courante pour saisir des tâches à la suite).
fn commit(
    ui: &egui::Ui,
    fc: &mut FrameCtx<'_>,
    edit: &mut TableEdit,
    cursor: &mut TableCursor,
    motion: Move,
) {
    let Some(doc) = fc.doc else {
        close_editor(ui, edit);
        return;
    };
    let Some(state) = edit.editing.as_mut() else {
        return;
    };
    let result = commit_edit(state, &doc.session, text_language(fc.texts.language()));
    let batch = match result {
        Ok(batch) => batch,
        Err(error) => {
            state.error = Some(error);
            edit.focus_pending = true;
            return;
        }
    };
    let column = state.column;
    close_editor(ui, edit);
    if !batch.ops.is_empty() {
        fc.actions.push(Action::ExecOn {
            opening: doc.opening,
            batch,
        });
    }
    cursor.column = column;
    // La ligne éditée est la ligne courante (sélection, ou ligne vide) :
    // le curseur en part.
    if let Some(mv) = motion.nav() {
        fc.actions.push(Action::Nav { mv, extend: false });
        fc.actions.push(Action::FocusPane(Pane::Table));
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    reason = "tests : un échec doit s'arrêter net et lisiblement ; valeurs exactes"
)]
mod tests {
    use planrs_core::TaskUid;

    use super::{NAME_MIN_TEXT_ROOM, cell_text_color, name_indent};
    use crate::column::Column;
    use crate::rows::{Row, RowKind};
    use crate::theme::Palette;

    fn row(kind: RowKind, critical: bool) -> Row {
        Row {
            uid: TaskUid::new(1),
            id: 1,
            level: 1,
            kind,
            critical,
            bar: None,
        }
    }

    /// D-fix-7 : le texte d'une tâche critique n'est rouge que dans la
    /// colonne « Critique ».
    #[test]
    fn table_critical_red_only_in_critical_column() {
        let p = Palette::LIGHT;
        let leaf = row(RowKind::Leaf, true);
        for column in Column::ALL {
            let expected = if column == Column::Critical {
                p.critical
            } else {
                p.text
            };
            assert_eq!(cell_text_color(&leaf, column, &p), expected, "{column:?}");
        }
        let summary = row(RowKind::Summary, true);
        assert_eq!(cell_text_color(&summary, Column::Name, &p), p.text_strong);
        assert_eq!(cell_text_color(&summary, Column::Critical, &p), p.critical);
        assert_eq!(
            cell_text_color(&row(RowKind::Leaf, false), Column::Critical, &p),
            p.text
        );
        assert_eq!(
            cell_text_color(&row(RowKind::Excluded, true), Column::Critical, &p),
            p.text_weak
        );
    }

    /// Mineur A/M4 : l'indentation est plafonnée, le nom reste visible au
    /// niveau 200 ou 65535.
    #[test]
    fn table_name_indent_is_capped() {
        assert_eq!(name_indent(14.0, 1, 220.0), 0.0);
        assert_eq!(name_indent(14.0, 3, 220.0), 28.0);
        for level in [46, 200, u16::MAX] {
            assert_eq!(name_indent(14.0, level, 220.0), 220.0 - NAME_MIN_TEXT_ROOM);
        }
        // Colonne plus étroite que la réserve : aucun retrait négatif.
        assert_eq!(name_indent(14.0, 10, 24.0), 0.0);
    }
}
