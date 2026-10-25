//! Table des tâches dessinée au `Painter` (en-tête et corps virtualisé).
//!
//! Seules les lignes de la plage visible sont dessinées ; chaque ligne déclare
//! un nœud d'accessibilité (`row-label`) pour être retrouvée par les tests.
//! Les textes viennent tout formatés du `RowModel` : aucune mise en forme ni
//! aucun parcours proportionnel au nombre de tâches pendant l'image. Chaque
//! cellule est une galley du cache d'egui (`LayoutJob` tronqué par « … »),
//! colorée au dessin (`Color32::PLACEHOLDER`), donc partagée entre les thèmes.

use std::ops::Range;

use egui::{
    Event, EventFilter, Id, Key, Modifiers, Painter, Pos2, Rect, Response, Sense, Stroke, pos2, vec2,
};
use planrs_automation::text::Language as TextLanguage;
use planrs_automation::{Batch, Session};
use planrs_core::TaskUid;

use super::main_view::HBarSpec;
use super::{FrameCtx, Pane};
use crate::action::{Action, DialogKind};
use crate::column::{CellAlign, Column};
use crate::edit::{EditState, TableEdit, commit_edit, editable, initial_text, text_language};
use crate::geom::index_to_f32;
pub use crate::grid::scroll_to_column;
use crate::grid::{
    self, CellStyle, EditorSpec, Finish, HeaderPlace, Move, RowPlace, Sheet, cell_font, column_at,
    column_span, finish_keys,
};
#[cfg(test)]
use crate::grid::{NAME_MIN_TEXT_ROOM, name_indent};
use crate::i18n::{Arg, TextId, Texts};
use crate::nav::{CellCursor, RowRef};
use crate::rows::{Row, RowKind, RowModel};
use crate::state::UiState;
use crate::theme::Palette;

/// Curseur de la table (cellule courante).
pub type TableCursor = CellCursor<TaskUid, Column>;

/// Identifiant focalisable de la table (J7, lot L1) : tant qu'il a le
/// focus clavier, les raccourcis de navigation (`KeyContext::Table`) sont
/// actifs. La feuille des ressources (lot L10) prend le même identifiant :
/// une seule grille est affichée à la fois.
#[must_use]
pub fn table_id() -> Id {
    Id::unique("planrs.table")
}

/// Verrou de focus de la table, rappelé à chaque image : Tab et les
/// flèches restent à la table (egui les traite en début d'image, avant
/// l'application) ; Échap rend le focus (ADR-0015 D2).
pub(crate) const NAV_FOCUS_FILTER: EventFilter = EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: false,
};

/// Vrai si un autre widget que la table a le focus clavier (zone de
/// saisie, bouton parcouru au clavier) : la frappe ne s'adresse pas à la
/// table.
pub(crate) fn focus_elsewhere(ctx: &egui::Context) -> bool {
    ctx.memory(|memory| memory.focused().is_some_and(|id| id != table_id()))
}

/// Couleur du texte d'une cellule. Comme MS Project, la table n'est pas
/// colorée par la criticité (D-fix-7) : seule la colonne « Critique » d'une
/// tâche critique est en rouge (la barre du Gantt l'est aussi) ; les
/// récapitulatives sont en texte appuyé (et en gras simulé au dessin).
fn cell_text_color(row: &Row, column: Column, palette: &Palette) -> egui::Color32 {
    match row.kind {
        RowKind::Null | RowKind::Excluded => palette.text_weak,
        _ if row.critical && column == Column::Critical => palette.critical,
        RowKind::Summary => palette.text_strong,
        RowKind::Leaf | RowKind::Milestone => palette.text,
    }
}

/// La table des tâches vue comme une grille ([`Sheet`], J7 lot L10) : lignes
/// du `RowModel`, colonnes visibles de l'état d'interface, nom indenté par
/// niveau, récapitulative en gras simulé.
pub struct TaskSheet<'a> {
    /// Lignes et textes pré-formatés.
    rows: &'a RowModel,
    /// Colonnes visibles et leurs largeurs.
    columns: Vec<(Column, u16)>,
    /// Messages (refus d'une saisie).
    texts: &'a Texts,
}

impl<'a> TaskSheet<'a> {
    /// Grille des lignes `rows` avec les colonnes visibles de `state`.
    #[must_use]
    pub fn new(rows: &'a RowModel, state: &UiState, texts: &'a Texts) -> Self {
        Self {
            rows,
            columns: visible_columns(state),
            texts,
        }
    }
}

/// Colonnes visibles de la table et leurs largeurs, dans l'ordre
/// d'affichage.
fn visible_columns(state: &UiState) -> Vec<(Column, u16)> {
    state
        .visible_columns()
        .map(|state| (state.column, state.width))
        .collect()
}

impl Sheet for TaskSheet<'_> {
    type Key = TaskUid;
    type Col = Column;

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn key(&self, row: usize) -> Option<TaskUid> {
        self.rows.row(row).map(|row| row.uid)
    }

    fn index_of(&self, key: TaskUid) -> Option<usize> {
        self.rows.index_of(key)
    }

    fn columns(&self) -> &[(Column, u16)] {
        &self.columns
    }

    fn title(&self, col: Column) -> TextId {
        col.title()
    }

    fn align(&self, col: Column) -> CellAlign {
        col.align()
    }

    fn style(&self, row: usize, col: Column, palette: &Palette) -> CellStyle {
        let Some(r) = self.rows.row(row) else {
            return CellStyle::plain(palette.text);
        };
        CellStyle {
            color: cell_text_color(r, col, palette),
            faux_bold: r.kind == RowKind::Summary,
            level: if col == Column::Name { r.level } else { 0 },
        }
    }

    fn text(&self, row: usize, col: Column) -> &str {
        self.rows.text(row, col)
    }

    fn editable(&self, col: Column) -> bool {
        editable(col)
    }

    /// Même lecture que la saisie de la table (`edit::commit_edit`, frappe
    /// directe : le texte est toujours envoyé) ; le refus est traduit dans
    /// la langue de l'interface.
    fn commit(
        &self,
        row: RowRef<TaskUid>,
        col: Column,
        text: &str,
        s: &Session,
        lang: TextLanguage,
    ) -> Result<Batch, String> {
        commit_edit(&EditState::typed(row.item(), col, text), s, lang).map_err(|e| e.message(self.texts))
    }
}

/// Demi-largeur de la poignée de redimensionnement au bord droit d'un
/// en-tête de colonne, en points.
const RESIZE_GRIP: f32 = 3.0;

/// En-tête de la table : fond, titres des colonnes visibles (centrés,
/// tronqués), séparateurs verticaux et trait bas, décalés de `scroll_x`
/// (défilement horizontal de la table, J7 lot L6).
///
/// Chaque titre est une zone cliquable (« Colonne Durée ») dont le clic
/// droit ouvre le menu des colonnes ([`column_menu`]) ; le bord droit de
/// chaque titre se glisse pour changer la largeur de la colonne
/// (`Action::SetColumnWidth`, bornée par `UiState::set_column_width`).
pub fn paint_header(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, rect: Rect, scroll_x: f32) {
    if !rect.is_finite() || !rect.is_positive() {
        return;
    }
    let scroll_x = if scroll_x.is_finite() { scroll_x } else { 0.0 };
    let clip = rect.intersect(ui.clip_rect());
    let painter = ui.painter().with_clip_rect(clip);
    let palette = fc.palette;
    let font = cell_font(ui);
    let mut shapes = 0_usize;
    painter.rect_filled(rect, 0.0, palette.header_bg);
    shapes = shapes.saturating_add(1);
    let columns = visible_columns(fc.state);
    let texts = fc.texts;
    let (edges, titles) = grid::paint_titles(
        &painter,
        &columns,
        |column: Column| texts.get(column.title()),
        palette,
        HeaderPlace {
            rect,
            left: rect.left() - scroll_x,
            pad: fc.metrics.pad_x,
        },
        &font,
    );
    shapes = shapes.saturating_add(titles);
    for &(column, left, right) in &edges {
        let cell = Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom())).intersect(clip);
        if cell.is_positive() {
            header_cell(ui, fc, column, cell);
        }
    }
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, palette.grid_major),
    );
    shapes = shapes.saturating_add(1);
    fc.stats.shapes = fc.stats.shapes.saturating_add(shapes);
    // Poignées après les titres : elles passent au-dessus pour le pointeur.
    for (column, cell_left, right) in edges {
        resize_grip(ui, fc, column, cell_left, right, rect, clip);
    }
}

/// Zone d'un titre de colonne : nœud d'accessibilité et menu des colonnes
/// au clic droit.
fn header_cell(ui: &egui::Ui, fc: &mut FrameCtx<'_>, column: Column, cell: Rect) {
    let texts = fc.texts;
    let response = ui.interact(
        cell,
        ui.make_persistent_id(("planrs.table.header", column)),
        Sense::click(),
    );
    response.widget_info(|| {
        let label = texts.get_args(
            TextId::ColumnHeaderLabel,
            &[("column", Arg::Str(texts.get(column.title()).into()))],
        );
        egui::WidgetInfo::labeled(egui::Role::ColumnHeader, true, label)
    });
    let mut chosen = None;
    response.context_menu(|ui| chosen = column_menu(ui, fc, column));
    if let Some(action) = chosen {
        fc.actions.push(action);
    }
}

/// Menu des colonnes (clic droit sur un titre) : « Masquer la colonne »
/// (grisé pour l'ID et le Nom, H-UI-12), puis une case par colonne, cochée
/// si elle est visible (ID et Nom cochés et grisés). Rend l'action choisie.
fn column_menu(ui: &mut egui::Ui, fc: &FrameCtx<'_>, clicked: Column) -> Option<Action> {
    let texts = fc.texts;
    let mut chosen = None;
    let hide = egui::Button::new(texts.get(TextId::ColumnMenuHide));
    if ui.add_enabled(clicked.hideable(), hide).clicked() {
        chosen = Some(Action::ToggleColumn(clicked));
    }
    ui.separator();
    for column in Column::ALL {
        let mut visible = fc.state.column(column).is_some_and(|state| state.visible);
        let check = egui::Checkbox::new(&mut visible, texts.get(column.title()));
        if ui.add_enabled(column.hideable(), check).clicked() {
            chosen = Some(Action::ToggleColumn(column));
        }
    }
    chosen
}

/// Poignée du bord droit d'un titre (`right`) : glisser pousse
/// `SetColumnWidth` d'après la position du pointeur (largeur entière ≥ 0,
/// bornée ensuite par l'état).
fn resize_grip(
    ui: &egui::Ui,
    fc: &mut FrameCtx<'_>,
    column: Column,
    cell_left: f32,
    right: f32,
    rect: Rect,
    clip: Rect,
) {
    let grip = Rect::from_min_max(
        pos2(right - RESIZE_GRIP, rect.top()),
        pos2(right + RESIZE_GRIP, rect.bottom()),
    )
    .intersect(clip);
    if !grip.is_finite() || !grip.is_positive() {
        return;
    }
    let texts = fc.texts;
    let response = ui.interact(
        grip,
        ui.make_persistent_id(("planrs.table.resize", column)),
        Sense::drag(),
    );
    response.widget_info(|| {
        let label = texts.get_args(
            TextId::ColumnResizeLabel,
            &[("column", Arg::Str(texts.get(column.title()).into()))],
        );
        egui::WidgetInfo::labeled(egui::Role::Splitter, true, label)
    });
    let response = response.on_hover_and_drag_cursor(egui::CursorIcon::ResizeColumn);
    if !response.dragged() {
        return;
    }
    let Some(p) = response.interact_pointer_pos() else {
        return;
    };
    let Some(width) = width_from_drag(p.x - cell_left) else {
        return;
    };
    let current = fc.state.column(column).map(|state| state.width);
    if current != Some(width) {
        fc.actions.push(Action::SetColumnWidth { column, width });
    }
}

/// Largeur entière d'une colonne d'après la distance entre son bord gauche
/// et le pointeur : arrondie, bornée à `[Column::MIN_WIDTH,
/// Column::MAX_WIDTH]` ; `None` si la distance n'est pas finie.
#[must_use]
pub fn width_from_drag(dx: f32) -> Option<u16> {
    if !dx.is_finite() {
        return None;
    }
    let w = dx
        .round()
        .clamp(f32::from(Column::MIN_WIDTH), f32::from(Column::MAX_WIDTH));
    // Borné à [24, 2 000] et fini : la conversion par `f32_to_index` est
    // exacte.
    u16::try_from(crate::geom::f32_to_index(w)).ok()
}

/// Barre de défilement horizontale de la table, dans la bande sous la
/// table : bande unie (comme avant J7) tant que les colonnes tiennent, barre
/// glissable sinon. Rend le défilement demandé, en points.
pub fn paint_hbar(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, track: Rect, scroll_x: f32, view_w: f32) -> f32 {
    let content_w: f32 = fc
        .state
        .visible_columns()
        .map(|state| f32::from(state.width))
        .sum();
    if !track.is_finite() || !track.is_positive() {
        return 0.0;
    }
    if content_w.is_nan() || content_w <= view_w {
        ui.painter().rect_filled(track, 0.0, fc.palette.header_bg);
        fc.stats.shapes = fc.stats.shapes.saturating_add(1);
        return 0.0;
    }
    super::main_view::hbar(
        ui,
        fc,
        track,
        &HBarSpec {
            id: Id::unique("planrs.table-hbar"),
            label: TextId::TableScrollLabel,
            scroll: scroll_x,
            content_w,
            visible_w: view_w,
        },
    )
}

/// Corps de la table : lignes `range`, `origin` étant le coin haut-gauche
/// écran de la ligne 0, découpe `clip`, colonnes décalées de `scroll_x`
/// (défilement horizontal, J7 lot L6).
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
#[allow(
    clippy::too_many_arguments,
    reason = "contexte d'image découpé en emprunts disjoints"
)]
pub fn paint_body(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    edit: &mut TableEdit,
    cursor: &mut TableCursor,
    origin: Pos2,
    range: Range<usize>,
    clip: Rect,
    scroll_x: f32,
) {
    let clip = clip.intersect(ui.clip_rect());
    let scroll_x = if scroll_x.is_finite() { scroll_x } else { 0.0 };
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
    let sheet = TaskSheet::new(rows, fc.state, fc.texts);
    open_from_keys(ui, fc, &sheet, edit, cursor);
    fc.stats.table_rows = range.clone();
    let left = clip.left();
    let cols_left = left - scroll_x;
    let blank = rows.len();
    let mut place = RowPlace {
        rect: Rect::NOTHING,
        cols_left,
        pad: metrics.pad_x,
        indent: metrics.indent,
    };
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
            place.rect = row_rect;
            shapes = shapes.saturating_add(grid::paint_cells(&painter, &sheet, palette, i, place, &font));
        }

        let hit = row_rect.intersect(clip);
        if hit.is_positive() {
            let response = row_widget(ui, fc, rows, hit, i, selected);
            row_clicked(fc, &sheet, edit, cursor, &response, i, cols_left);
        }
    }

    // Séparateurs verticaux de colonnes, sur toute la hauteur visible.
    shapes = shapes.saturating_add(grid::paint_column_lines(
        &painter,
        sheet.columns(),
        cols_left,
        clip,
        palette.grid,
    ));
    let at = Place {
        origin,
        cols_left,
        range: &range,
    };
    shapes = shapes.saturating_add(paint_cursor(ui, &painter, fc, &sheet, edit, cursor, &at));
    fc.stats.shapes = fc.stats.shapes.saturating_add(shapes);
    edit_cells(ui, fc, &sheet, edit, cursor, &at, clip);
}

/// Zone cliquable de la ligne `i` (ou de la ligne vide) et son libellé
/// d'accessibilité (`row-label`, `blank-row-label`).
fn row_widget(
    ui: &egui::Ui,
    fc: &FrameCtx<'_>,
    rows: &RowModel,
    hit: Rect,
    i: usize,
    selected: bool,
) -> Response {
    let response = ui.interact(
        hit,
        ui.make_persistent_id(("planrs-table-row", i)),
        Sense::click(),
    );
    let texts = fc.texts;
    response.widget_info(|| {
        let label = match rows.row(i) {
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
    response
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

/// Position des lignes et des colonnes à l'écran pour une image.
struct Place<'r> {
    /// Coin haut-gauche écran de la ligne 0.
    origin: Pos2,
    /// Abscisse écran du bord gauche de la première colonne visible
    /// (défilement horizontal déduit).
    cols_left: f32,
    /// Lignes dessinées.
    range: &'r Range<usize>,
}

/// Cadre de 2 px de la cellule courante (couleur `Palette::cursor`) et
/// son nœud d'accessibilité, si elle est dans la plage dessinée, sa colonne
/// visible et aucune saisie ouverte ; rend le nombre de formes ajoutées.
fn paint_cursor(
    ui: &egui::Ui,
    painter: &Painter,
    fc: &FrameCtx<'_>,
    sheet: &TaskSheet<'_>,
    edit: &TableEdit,
    cursor: &TableCursor,
    at: &Place<'_>,
) -> usize {
    if edit.editing.is_some() {
        return 0;
    }
    let clip = painter.clip_rect();
    let Some(index) = grid::row_index(sheet, cursor.row).filter(|i| at.range.contains(i)) else {
        return 0;
    };
    let Some((left, w)) = column_span(sheet.columns(), at.cols_left, cursor.column) else {
        return 0;
    };
    let top = at.origin.y + index_to_f32(index) * fc.metrics.row_h;
    let rect = Rect::from_min_size(pos2(left, top), vec2(w, fc.metrics.row_h));
    if !grid::paint_cursor_frame(painter, rect, fc.palette) {
        return 0;
    }
    let hit = rect.intersect(clip);
    if hit.is_positive() {
        let response = ui.interact(hit, Id::unique("planrs.table.cursor"), Sense::hover());
        let texts = fc.texts;
        let rows = sheet.rows;
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

/// Clic sur la ligne `i` (ou sur la ligne vide) : sélection, focus de la
/// table, colonne courante sous le pointeur ; double-clic : ouverture de la
/// saisie.
fn row_clicked(
    fc: &mut FrameCtx<'_>,
    sheet: &TaskSheet<'_>,
    edit: &mut TableEdit,
    cursor: &mut TableCursor,
    response: &Response,
    i: usize,
    left: f32,
) {
    if !response.clicked() {
        return;
    }
    let uid = sheet.key(i);
    fc.actions.push(Action::Select(uid));
    fc.actions.push(Action::FocusPane(Pane::Table));
    if let Some(column) = response
        .interact_pointer_pos()
        .and_then(|p| column_at(sheet.columns(), left, p.x))
    {
        cursor.column = column;
    }
    // Double-clic (ou triple : deux clics suivant de près un premier) :
    // sur l'ID, « Informations sur la tâche » (H-UI-19, J7 lot L9) ;
    // ailleurs, la saisie de la cellule.
    if (response.double_clicked() || response.triple_clicked()) && edit.editing.is_none() {
        match uid {
            Some(uid) if cursor.column == Column::Id => {
                fc.actions
                    .push(Action::ShowDialog(DialogKind::TaskInformation(uid)));
            }
            _ => open_editor(fc, sheet.rows, edit, cursor.column, uid, i),
        }
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

/// Texte frappé dans cette image hors modificateur Ctrl/Cmd (caractères
/// imprimables seulement), retiré des événements ; vide sinon.
pub(crate) fn take_typed(ctx: &egui::Context) -> String {
    ctx.input_mut(|input| {
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
    })
}

/// F2 ou frappe d'un caractère (hors champ de texte ou autre widget
/// focalisé, menu ouvert ou modificateur Ctrl/Cmd) : ouvre la saisie de la
/// cellule courante. Le caractère frappé remplace la valeur.
fn open_from_keys(
    ui: &egui::Ui,
    fc: &FrameCtx<'_>,
    sheet: &TaskSheet<'_>,
    edit: &mut TableEdit,
    cursor: &TableCursor,
) {
    let ctx = ui.ctx();
    if edit.editing.is_some() || focus_elsewhere(ctx) || ctx.any_popup_open() {
        return;
    }
    let Some((uid, i)) = current_row(fc, sheet.rows) else {
        return;
    };
    let column = cursor.column;
    if !editable(column) || !sheet.columns().iter().any(|(c, _)| *c == column) {
        return;
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F2)) {
        open_editor(fc, sheet.rows, edit, column, uid, i);
        return;
    }
    let typed = take_typed(ctx);
    if !typed.is_empty() {
        edit.start(EditState::typed(uid, column, &typed));
    }
}

/// Saisie de la cellule ouverte : zone de saisie posée sur la cellule,
/// message d'erreur dessous, fin de saisie. Entrée valide et descend, Tab
/// valide et passe à la cellule visible suivante, Échap abandonne ; un
/// défilement ou la perte du focus valident sur place. Une saisie refusée
/// ([`commit_edit`]) reste ouverte avec son message ; acceptée, son lot est
/// poussé dans `Action::ExecOn` (refus du registre : bandeau), suivi du
/// déplacement du curseur (`Action::Nav`, appliqué après le lot).
fn edit_cells(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    sheet: &TaskSheet<'_>,
    edit: &mut TableEdit,
    cursor: &mut TableCursor,
    at: &Place<'_>,
    clip: Rect,
) {
    let Some(state) = edit.editing.as_ref() else {
        return;
    };
    let index = grid::row_index(sheet, RowRef::from_selection(state.uid));
    let span = column_span(sheet.columns(), at.cols_left, state.column);
    let (Some(index), Some((cell_left, cell_w))) = (index, span) else {
        // Tâche disparue ou colonne masquée : saisie abandonnée.
        close_editor(ui, edit);
        return;
    };
    // Zone de saisie sans le focus (pas encore dessinée : ligne hors de la
    // plage, ou focus demandé pour cette image) : la frappe va au tampon,
    // rien n'est perdu (relecture finale J7, F2).
    if !ui.memory(|memory| memory.has_focus(grid::editor_id())) {
        absorb_keys(ui, edit);
    }
    let mut finish = finish_keys(ui);
    if finish.is_none() && grid::scrolled(ui) {
        finish = Some(Finish::Commit(Move::Stay));
    }
    if at.range.contains(&index) {
        let top = at.origin.y + index_to_f32(index) * fc.metrics.row_h;
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

/// Frappe reçue alors que la zone de saisie n'a pas encore le focus :
/// caractères ajoutés au tampon, Retour arrière en retire un par appui.
fn absorb_keys(ui: &egui::Ui, edit: &mut TableEdit) {
    let Some(state) = edit.editing.as_mut() else {
        return;
    };
    let typed = take_typed(ui.ctx());
    let erased = ui.input_mut(|input| input.count_and_consume_key(Modifiers::NONE, Key::Backspace));
    if typed.is_empty() && erased == 0 {
        return;
    }
    state.buffer.push_str(&typed);
    for _ in 0..erased {
        state.buffer.pop();
    }
    state.error = None;
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
    let label = fc.texts.get_args(
        TextId::CellEditLabel,
        &[("column", Arg::Str(fc.texts.get(state.column.title()).into()))],
    );
    let spec = EditorSpec { rect, clip, label };
    let out = grid::show_editor(ui, &mut state.buffer, focus_pending, &spec);
    if out.changed {
        state.error = None;
    }
    if let Some(error) = &state.error {
        grid::show_cell_error(ui.ctx(), rect, error.message(fc.texts), fc.palette.critical);
    }
    out.lost
}

/// Ferme la saisie et rend le focus (les raccourcis redeviennent actifs).
fn close_editor(ui: &egui::Ui, edit: &mut TableEdit) {
    edit.editing = None;
    edit.focus_pending = false;
    grid::surrender_editor(ui);
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

    use super::{NAME_MIN_TEXT_ROOM, cell_text_color, name_indent, scroll_to_column, width_from_drag};
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
            percent: planrs_core::Percent::ZERO,
            baseline: None,
            finish_variance: None,
            stop: None,
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

    /// H-UI-17 horizontal (J7, lot L6) : une colonne de marge quand elle
    /// tient, alignement à gauche d'une colonne plus large que la vue,
    /// bornes et valeurs non finies.
    #[test]
    fn table_scroll_to_column_margin() {
        let widths = [40, 220, 80, 96, 96, 110];
        // Vue de 300 : colonnes 0-1 visibles (0..260).
        assert_eq!(scroll_to_column(&widths, 1, 0.0, 300.0), 0.0);
        // Colonne 2 (260..340) et sa voisine 3 (..436) : bord droit à 436.
        assert_eq!(scroll_to_column(&widths, 2, 0.0, 300.0), 136.0);
        // Retour à la colonne 1 (40..260) avec la colonne 0 en marge.
        assert_eq!(scroll_to_column(&widths, 1, 136.0, 300.0), 0.0);
        // Dernière colonne : maximum (642 − 300).
        assert_eq!(scroll_to_column(&widths, 5, 0.0, 300.0), 342.0);
        // Marge impossible (vue de 120) : la colonne seule.
        assert_eq!(scroll_to_column(&widths, 2, 0.0, 120.0), 220.0);
        // Colonne plus large que la vue : bord gauche.
        assert_eq!(scroll_to_column(&widths, 1, 0.0, 100.0), 40.0);
        // Colonne absente, valeurs non finies : bornées.
        assert_eq!(scroll_to_column(&widths, 9, 1e9, 300.0), 342.0);
        assert_eq!(scroll_to_column(&widths, 0, f32::NAN, f32::INFINITY), 0.0);
        assert_eq!(scroll_to_column(&[], 0, 5.0, 300.0), 0.0);
    }

    #[test]
    fn table_width_from_drag_bounded() {
        assert_eq!(width_from_drag(120.4), Some(120));
        assert_eq!(width_from_drag(-50.0), Some(Column::MIN_WIDTH));
        assert_eq!(width_from_drag(1e9), Some(Column::MAX_WIDTH));
        assert_eq!(width_from_drag(f32::NAN), None);
        assert_eq!(width_from_drag(f32::INFINITY), None);
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
