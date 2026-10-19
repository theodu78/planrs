//! Vue principale : en-têtes, séparateur, `ScrollArea` verticale unique pour
//! la table et le Gantt (alignés dans la même image), défilement horizontal
//! du Gantt.
//!
//! Le défilement vertical vit dans la mémoire de la `ScrollArea` ; il n'est
//! réimposé qu'une fois par `ViewTransient::pending_scroll_y`. Le défilement
//! horizontal du Gantt est l'état transitoire `ViewTransient::gantt_scroll_x`,
//! lu par l'en-tête et par le corps dans la même image. Les gestes produisent
//! des `Action` (`ScrollGanttX`, `ZoomStep`, `SetTableWidth`, `Select`) ;
//! aucune conversion pixel → date : seuls des indices de ligne sont déduits
//! d'une ordonnée.
//!
//! Sélection multiple et menu contextuel (lot L14) : un clic de ligne avec
//! Ctrl/Cmd ou Maj devient `SelectWith` (la table et le Gantt ne poussent
//! que `Select`, réécrit ici d'après les modificateurs de l'image) ; un clic
//! droit sur une ligne la sélectionne (si elle ne l'est pas déjà) et ouvre
//! le menu des commandes de structure (`Action::Structure`).

use egui::containers::menu::menu_style;
use egui::scroll_area::ScrollBarVisibility;
use egui::{
    Align, CursorIcon, Id, Layout, PointerButton, Popup, PopupAnchor, PopupKind, Pos2, Rect, Sense,
    SetOpenCommand, Stroke, pos2, vec2,
};

use planrs_core::TaskUid;

use super::{FrameCtx, OVERSCAN, ViewTransient, gantt, table, timescale};
use crate::action::Action;
use crate::geom::{self, MainLayout};
use crate::i18n::TextId;
use crate::keymap::{self, Binding};
use crate::selection::SelectMode;
use crate::structure::StructureOp;
use crate::timescale::TimeAxis;

/// Largeur minimale du curseur de la barre horizontale, en points.
const HBAR_MIN_THUMB: f32 = 16.0;

/// Pas de zoom à la molette : un pas par facteur 1,25 cumulé.
const ZOOM_STEP_FACTOR: f32 = 1.25;

/// Défilement horizontal du Gantt borné à `[0, max_scroll]` (0 si non fini).
fn clamp_scroll(scroll: f32, max_scroll: f32) -> f32 {
    if scroll.is_finite() {
        scroll.clamp(0.0, max_scroll)
    } else {
        0.0
    }
}

/// Défilement horizontal maximal : contenu moins partie visible, jamais
/// négatif ni non fini.
fn max_scroll_x(content_w: f32, visible_w: f32) -> f32 {
    let max = content_w - visible_w;
    if max.is_finite() { max.max(0.0) } else { 0.0 }
}

/// Entrées lues au-dessus du Gantt pour cette image.
#[derive(Clone, Copy, Debug)]
struct GanttInput {
    /// Pointeur au-dessus du Gantt (corps ou en-tête).
    hovered: bool,
    /// Position du pointeur.
    pointer: Option<Pos2>,
    /// Facteur de zoom (Ctrl/Cmd + molette, pincement) ; 1 = rien.
    zoom_delta: f32,
}

/// Affiche la vue principale dans la place restante de `ui`.
pub fn show(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, view: &mut ViewTransient) {
    let avail = ui.available_rect_before_wrap();
    let metrics = *fc.metrics;
    let layout = geom::main_layout(avail, f32::from(fc.state.table_width), &metrics);
    let Some(doc) = fc.doc else {
        ui.advance_cursor_after_rect(avail);
        return;
    };
    let gantt_w = layout.gantt_clip.width();
    view.last_gantt_width = gantt_w;
    view.last_gantt_left = layout.gantt_clip.left();
    let content_w = TimeAxis::new(doc.range, fc.state.zoom, 0.0).content_width();
    let max_scroll = max_scroll_x(content_w, gantt_w);
    let scroll_x = clamp_scroll(view.gantt_scroll_x, max_scroll);
    view.gantt_scroll_x = scroll_x;
    let axis = TimeAxis::new(doc.range, fc.state.zoom, layout.gantt_clip.left() - scroll_x);

    let gantt_zone = layout.gantt_clip.union(layout.gantt_header);
    let hovered = ui.rect_contains_pointer(gantt_zone);
    let input = ui.input(|i| GanttInput {
        hovered,
        pointer: i.pointer.hover_pos(),
        zoom_delta: i.zoom_delta(),
    });

    table::paint_header(ui, fc, layout.table_header);
    timescale::paint_header(ui, fc, &axis, layout.gantt_header);
    paint_corners(ui, fc, &layout, avail);

    let RowsOut {
        dx: mut delta_x,
        context_click,
    } = paint_rows(ui, fc, view, &layout, &axis);
    context_menu(ui, fc, context_click);

    // Molette horizontale (ou Maj + molette) au-dessus du Gantt : la
    // `ScrollArea` verticale ne consomme pas l'axe x (vérifié par le test
    // `shift_wheel_over_gantt_scrolls_x`), on le lit donc après elle.
    if input.hovered {
        let wheel_x = ui.input(|i| i.smooth_scroll_delta.x);
        if wheel_x.is_finite() && wheel_x.abs() > f32::EPSILON {
            delta_x -= wheel_x;
            ui.input_mut(|i| i.smooth_scroll_delta.x = 0.0);
        }
        zoom_from_wheel(fc, view, &layout, input);
    }

    paint_splitter(ui, fc, &layout, avail);
    delta_x += paint_hbar(ui, fc, &layout, scroll_x, content_w, max_scroll);

    let target = clamp_scroll(scroll_x + delta_x, max_scroll);
    let dx = target - scroll_x;
    if dx.abs() > f32::EPSILON {
        fc.actions.push(Action::ScrollGanttX(dx));
    }
    ui.advance_cursor_after_rect(avail);
}

/// Résultat du corps : défilement horizontal demandé et clic droit.
struct RowsOut {
    /// Défilement horizontal demandé par un glisser sur le Gantt.
    dx: f32,
    /// Clic droit sur le corps, s'il y en a eu un.
    context_click: Option<ContextClick>,
}

/// Clic droit sur le corps de la vue.
#[derive(Clone, Copy, Debug)]
struct ContextClick {
    /// Tâche de la ligne cliquée (`None` sous la dernière ligne).
    uid: Option<TaskUid>,
}

/// Réécrit les `Select(Some(uid))` poussés depuis `start` en `SelectWith`
/// si Ctrl/Cmd (ajout ou retrait) ou Maj (plage) est enfoncé.
fn apply_click_modifiers(actions: &mut [Action], start: usize, modifiers: egui::Modifiers) {
    let mode = if modifiers.command {
        SelectMode::Toggle
    } else if modifiers.shift {
        SelectMode::Range
    } else {
        return;
    };
    for action in actions.iter_mut().skip(start) {
        if let Action::Select(Some(uid)) = *action {
            *action = Action::SelectWith { uid, mode };
        }
    }
}

/// `ScrollArea` verticale unique : corps de la table et du Gantt sur la même
/// plage de lignes, fond cliquable (sélection, glisser horizontal du Gantt).
fn paint_rows(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    view: &mut ViewTransient,
    layout: &MainLayout,
    axis: &TimeAxis,
) -> RowsOut {
    let row_h = fc.metrics.row_h;
    // Une ligne de plus que de tâches : la ligne vide de fin (création).
    let len = fc.rows.map_or(0, |rows| rows.len().saturating_add(1));
    let content_h = geom::index_to_f32(len) * row_h;
    let content_h = if content_h.is_finite() {
        content_h.max(0.0)
    } else {
        0.0
    };

    let mut area = egui::ScrollArea::vertical()
        .id_salt("planrs.rows")
        .auto_shrink([false, false])
        .content_margin(egui::Margin::ZERO)
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible);
    if let Some(y) = view.pending_scroll_y.take()
        && y.is_finite()
    {
        // Borné au défilement maximal : un décalage au-delà ferait revenir
        // la zone par un rebond animé sur une dizaine d'images.
        let max_y = (content_h - layout.body.height()).max(0.0);
        let max_y = if max_y.is_finite() { max_y } else { 0.0 };
        area = area.vertical_scroll_offset(y.clamp(0.0, max_y));
    }
    let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(layout.body));
    // Barre verticale pleine, toujours visible (relecture B, I4) : elle
    // occupe `Metrics::vbar_w` à droite du corps, que la mise en page
    // retire au Gantt.
    body_ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
    let table_clip = layout.table_clip;
    let gantt_clip = layout.gantt_clip;
    let edit = &mut view.edit;
    let out = area.show_viewport(&mut body_ui, |ui, viewport| {
        let origin = ui.max_rect().min;
        ui.set_min_size(vec2(ui.available_width().max(0.0), content_h));
        let range = geom::visible_rows(viewport.top(), viewport.bottom(), row_h, len, OVERSCAN);

        // Fond : déclaré avant les lignes, qui passent donc au-dessus.
        let background = ui.interact(
            ui.clip_rect(),
            ui.make_persistent_id("planrs-body-bg"),
            Sense::click_and_drag(),
        );
        let first_action = fc.actions.len();
        table::paint_body(ui, fc, edit, origin, range.clone(), table_clip);
        gantt::paint_body(ui, fc, axis, origin, range, gantt_clip);

        let mut dx = 0.0;
        if background.clicked()
            && let Some(p) = background.interact_pointer_pos()
        {
            let uid = if gantt_clip.contains(p) {
                geom::row_at(p.y - origin.y, row_h, len)
                    .and_then(|i| fc.rows.and_then(|rows| rows.row(i)))
                    .map(|row| row.uid)
            } else {
                None
            };
            fc.actions.push(Action::Select(uid));
        }
        let modifiers = ui.input(|i| i.modifiers);
        apply_click_modifiers(fc.actions, first_action, modifiers);
        let context_click = secondary_click(ui, table_clip.union(gantt_clip)).map(|p| ContextClick {
            uid: geom::row_at(p.y - origin.y, row_h, len)
                .and_then(|i| fc.rows.and_then(|rows| rows.row(i)))
                .map(|row| row.uid),
        });
        let from_gantt = ui
            .input(|i| i.pointer.press_origin())
            .is_some_and(|p| gantt_clip.contains(p));
        if background.dragged_by(PointerButton::Primary) && from_gantt {
            let drag = background.drag_delta().x;
            if drag.is_finite() {
                dx -= drag;
            }
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        }
        RowsOut { dx, context_click }
    });
    // Les plages dessinées sont relevées par chaque volet lui-même.
    out.inner
}

/// Position d'un clic droit relâché dans cette image au-dessus de `zone`, sur
/// la couche de `ui` (un clic dans un menu ou une fenêtre au-dessus ne compte
/// pas).
fn secondary_click(ui: &egui::Ui, zone: Rect) -> Option<Pos2> {
    let pos = ui.input(|i| {
        i.pointer
            .button_clicked(PointerButton::Secondary)
            .then(|| i.pointer.interact_pos())
            .flatten()
    })?;
    let on_layer = ui.ctx().layer_id_at(pos) == Some(ui.layer_id());
    (zone.contains(pos) && ui.clip_rect().contains(pos) && on_layer).then_some(pos)
}

/// Entrées du menu contextuel (et du menu Édition) : commande, libellé,
/// liaison de [`keymap::KEYMAP`] dont le raccourci est affiché, et besoin
/// d'une sélection.
pub(crate) const CONTEXT_ENTRIES: [(StructureOp, TextId, &str, bool); 6] = [
    (
        StructureOp::InsertBefore,
        TextId::StructureInsert,
        "task.add",
        false,
    ),
    (StructureOp::Delete, TextId::StructureDelete, "task.delete", true),
    (StructureOp::Indent, TextId::StructureIndent, "task.indent", true),
    (
        StructureOp::Outdent,
        TextId::StructureOutdent,
        "task.outdent",
        true,
    ),
    (StructureOp::Link, TextId::StructureLink, "link.add", true),
    (StructureOp::Unlink, TextId::StructureUnlink, "link.remove", true),
];

/// Menu contextuel des commandes de structure, ouvert au pointeur par un
/// clic droit sur le corps ; la ligne cliquée est d'abord sélectionnée si
/// elle ne l'est pas (la sélection multiple est gardée sinon).
fn context_menu(ui: &egui::Ui, fc: &mut FrameCtx<'_>, click: Option<ContextClick>) {
    let open = click.map(|ContextClick { uid }| {
        if let Some(uid) = uid
            && !fc.selected.contains(uid)
        {
            fc.actions.push(Action::Select(Some(uid)));
        }
        SetOpenCommand::Bool(true)
    });
    // La sélection change à la fin de l'image : une ligne cliquée compte
    // déjà comme sélectionnée pour griser les entrées.
    let has_selection = fc.selection.is_some() || click.is_some_and(|c| c.uid.is_some());
    let texts = fc.texts;
    let ctx = ui.ctx().clone();
    let mut chosen = None;
    Popup::new(
        Id::unique("planrs.structure-menu"),
        ctx.clone(),
        PopupAnchor::PointerFixed,
        ui.layer_id(),
    )
    .kind(PopupKind::Menu)
    .layout(Layout::top_down_justified(Align::Min))
    .style(menu_style)
    .open_memory(open)
    .show(|ui| {
        for (op, text, command, needs_selection) in CONTEXT_ENTRIES {
            let mut button = egui::Button::new(texts.get(text));
            if let Some(shortcut) = keymap::shortcut_for(Binding::Command(command)) {
                button = button.shortcut_text(keymap::shortcut_text(&ctx, shortcut));
            }
            if ui
                .add_enabled(has_selection || !needs_selection, button)
                .clicked()
            {
                chosen = Some(op);
            }
        }
    });
    if let Some(op) = chosen {
        fc.actions.push(Action::Structure(op));
    }
}

/// Coins à droite de l'en-tête et de la barre horizontale du Gantt, au-dessus
/// et au-dessous de la barre verticale : même fond que l'en-tête.
fn paint_corners(ui: &egui::Ui, fc: &mut FrameCtx<'_>, layout: &MainLayout, avail: Rect) {
    let corners = [
        Rect::from_min_max(
            pos2(layout.gantt_header.right(), layout.gantt_header.top()),
            pos2(avail.right(), layout.gantt_header.bottom()),
        ),
        Rect::from_min_max(
            pos2(layout.gantt_hbar.right(), layout.gantt_hbar.top()),
            pos2(avail.right(), layout.gantt_hbar.bottom()),
        ),
    ];
    for corner in corners {
        if corner.is_finite() && corner.is_positive() {
            ui.painter().rect_filled(corner, 0.0, fc.palette.header_bg);
            fc.stats.shapes = fc.stats.shapes.saturating_add(1);
        }
    }
}

/// Ctrl/Cmd + molette au-dessus du Gantt : cumule `ln(zoom_delta)` et émet un
/// pas de zoom par franchissement de ±ln(1,25), ancré sous le pointeur
/// (abscisse relative au bord gauche du Gantt).
fn zoom_from_wheel(fc: &mut FrameCtx<'_>, view: &mut ViewTransient, layout: &MainLayout, input: GanttInput) {
    let d = input.zoom_delta;
    if !d.is_finite() || d <= 0.0 || (d - 1.0).abs() <= f32::EPSILON {
        return;
    }
    if !view.zoom_accum.is_finite() {
        view.zoom_accum = 0.0;
    }
    view.zoom_accum += d.ln();
    let threshold = ZOOM_STEP_FACTOR.ln();
    let steps: i8 = if view.zoom_accum >= threshold {
        1
    } else if view.zoom_accum <= -threshold {
        -1
    } else {
        return;
    };
    view.zoom_accum = 0.0;
    let clip = layout.gantt_clip;
    let anchor_x = input
        .pointer
        .map_or(0.0, |p| (p.x - clip.left()).clamp(0.0, clip.width().max(0.0)));
    let anchor_x = if anchor_x.is_finite() { anchor_x } else { 0.0 };
    fc.actions.push(Action::ZoomStep { steps, anchor_x });
}

/// Séparateur table / Gantt, glissable : `SetTableWidth` d'après la position
/// du pointeur (pas de perte d'arrondi d'une image à l'autre).
fn paint_splitter(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, layout: &MainLayout, avail: Rect) {
    let rect = layout.splitter;
    if !rect.is_finite() || !rect.is_positive() {
        return;
    }
    let texts = fc.texts;
    let response = ui.interact(rect, Id::unique("planrs.splitter"), Sense::drag());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Splitter, true, texts.get(TextId::SplitterLabel))
    });
    let response = response.on_hover_and_drag_cursor(CursorIcon::ResizeHorizontal);
    let color = if response.hovered() || response.dragged() {
        fc.palette.grid_major
    } else {
        fc.palette.splitter
    };
    ui.painter().rect_filled(rect, 0.0, color);
    fc.stats.shapes = fc.stats.shapes.saturating_add(1);
    if response.dragged() {
        let splitter_w = rect.width();
        let wanted = response.interact_pointer_pos().map_or_else(
            || layout.table_clip.width() + response.drag_delta().x,
            |p| p.x - avail.left() - splitter_w * 0.5,
        );
        let width = geom::clamp_table_width(wanted, avail.width(), splitter_w).round();
        let width = u16::try_from(geom::f32_to_index(width)).unwrap_or(u16::MAX);
        if width != fc.state.table_width {
            fc.actions.push(Action::SetTableWidth(width));
        }
    }
}

/// Barre horizontale minimale du Gantt (piste et curseur proportionnel),
/// glissable ; un clic sur la piste avance d'une page. Rend le défilement
/// demandé, en pixels de contenu.
fn paint_hbar(
    ui: &mut egui::Ui,
    fc: &mut FrameCtx<'_>,
    layout: &MainLayout,
    scroll_x: f32,
    content_w: f32,
    max_scroll: f32,
) -> f32 {
    let palette = fc.palette;
    let painter = ui.painter();
    // Bande sous la table : même fond que la piste.
    let under_table = Rect::from_min_max(
        pos2(layout.table_clip.left(), layout.gantt_hbar.top()),
        pos2(layout.table_clip.right(), layout.gantt_hbar.bottom()),
    );
    if under_table.is_finite() && under_table.is_positive() {
        painter.rect_filled(under_table, 0.0, palette.header_bg);
    }
    let track = layout.gantt_hbar;
    if !track.is_finite() || !track.is_positive() {
        return 0.0;
    }
    painter.rect_filled(track, 0.0, palette.header_bg);
    painter.hline(track.x_range(), track.top() + 0.5, Stroke::new(1.0, palette.grid));
    fc.stats.shapes = fc.stats.shapes.saturating_add(3);

    let visible_w = layout.gantt_clip.width();
    let ratio = if content_w > 0.0 && content_w.is_finite() {
        (visible_w / content_w).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let track_w = track.width();
    let thumb_w = (track_w * ratio).max(HBAR_MIN_THUMB).min(track_w);
    let free = (track_w - thumb_w).max(0.0);
    let t = if max_scroll > 0.0 {
        (scroll_x / max_scroll).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let inset = (track.height() * 0.2).min(2.0);
    let thumb = Rect::from_min_max(
        pos2(track.left() + free * t, track.top() + inset),
        pos2(track.left() + free * t + thumb_w, track.bottom() - inset),
    );

    let texts = fc.texts;
    let response = ui.interact(track, Id::unique("planrs.gantt-hbar"), Sense::click_and_drag());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::ScrollBar, true, texts.get(TextId::GanttScrollLabel))
    });
    let active = response.hovered() || response.dragged();
    if thumb.is_finite() && thumb.is_positive() && max_scroll > 0.0 {
        let color = if active {
            palette.text_weak
        } else {
            palette.grid_major
        };
        ui.painter().rect_filled(thumb, 2.0, color);
        fc.stats.shapes = fc.stats.shapes.saturating_add(1);
    }
    if max_scroll <= 0.0 {
        return 0.0;
    }
    let mut dx = 0.0;
    if response.dragged_by(PointerButton::Primary) && free > 0.0 {
        let drag = response.drag_delta().x * max_scroll / free;
        if drag.is_finite() {
            dx += drag;
        }
    } else if response.clicked()
        && let Some(p) = response.interact_pointer_pos()
    {
        if p.x < thumb.left() {
            dx -= visible_w;
        } else if p.x > thumb.right() {
            dx += visible_w;
        }
    }
    dx
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use egui::{Event, Modifiers, MouseWheelUnit, Pos2, RawInput, Rect, TouchPhase, pos2, vec2};
    use planrs_core::Moment;

    use crate::action::Action;
    use crate::app::{App, Env};
    use crate::i18n::{Language, Texts};
    use crate::state::{LanguagePref, ThemePref, UiState};
    use crate::theme::Palette;
    use crate::view::{DiagnosticsView, FrameCtx, FrameStats, Metrics, ViewTransient};

    fn now() -> Moment {
        "2027-01-15T08:00".parse().unwrap()
    }

    fn state() -> UiState {
        UiState {
            language: LanguagePref::Fr,
            theme: ThemePref::Light,
            ..UiState::default()
        }
    }

    fn input(mut events: Vec<Event>, modifiers: Modifiers) -> RawInput {
        events.insert(0, Event::ModifiersChanged(modifiers));
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 720.0))),
            events,
            ..RawInput::default()
        }
    }

    fn wheel(delta: egui::Vec2, modifiers: Modifiers) -> Event {
        Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta,
            phase: TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers,
        }
    }

    /// Une image de `main_view::show` seule, avec un `FrameCtx` fait main ;
    /// rend les actions émises et le relevé.
    fn frame(
        ctx: &egui::Context,
        app: &App,
        view: &mut ViewTransient,
        raw: RawInput,
    ) -> (Vec<Action>, FrameStats) {
        let texts = Texts::new(Language::Fr);
        let palette = Palette::LIGHT;
        let metrics = Metrics::default();
        let mut actions = Vec::new();
        let mut stats = FrameStats::default();
        let diagnostics = DiagnosticsView::default();
        let mut output = ctx.run_ui(raw, |ui| {
            let mut fc = FrameCtx {
                state: &app.state,
                doc: app.document.as_ref(),
                rows: app.rows(),
                texts: &texts,
                palette: &palette,
                metrics: &metrics,
                diagnostics: &diagnostics,
                selection: None,
                selected: &crate::selection::Selection::default(),
                actions: &mut actions,
                stats: &mut stats,
            };
            super::show(ui, &mut fc, view);
        });
        // Aucun moteur de rendu : les textures sont ignorées.
        output.textures_delta.clear();
        (actions, stats)
    }

    fn demo_app() -> App {
        let mut app = App::new(state(), Env::fixed(now()));
        app.open_demo();
        assert!(app.document.is_some());
        app
    }

    /// Expérience R-6 : la `ScrollArea` verticale laisse l'axe x de la
    /// molette (Maj + molette) au Gantt, qui en fait un `ScrollGanttX`.
    #[test]
    fn shift_wheel_over_gantt_scrolls_x() {
        let ctx = egui::Context::default();
        let mut app = demo_app();
        // Zoom Jour : le contenu dépasse largement la fenêtre.
        app.state.zoom = crate::timescale::TimeScaleZoom::Day;
        let mut view = ViewTransient::default();
        let over_gantt = pos2(1000.0, 300.0);
        let _ = frame(
            &ctx,
            &app,
            &mut view,
            input(vec![Event::PointerMoved(over_gantt)], Modifiers::NONE),
        );
        let mut got = Vec::new();
        for _ in 0..4 {
            let events = vec![
                Event::PointerMoved(over_gantt),
                wheel(vec2(0.0, -50.0), Modifiers::SHIFT),
            ];
            let (actions, _) = frame(&ctx, &app, &mut view, input(events, Modifiers::SHIFT));
            got.extend(actions);
        }
        let dx: f32 = got
            .iter()
            .filter_map(|a| match a {
                Action::ScrollGanttX(dx) => Some(*dx),
                _ => None,
            })
            .sum();
        assert!(
            dx > 0.0,
            "Maj + molette vers le bas doit avancer le Gantt : {got:?}"
        );
    }

    /// Ctrl + molette au-dessus du Gantt : un pas de zoom vers le plus fin.
    #[test]
    fn ctrl_wheel_over_gantt_zooms() {
        let ctx = egui::Context::default();
        let app = demo_app();
        let mut view = ViewTransient::default();
        let over_gantt = pos2(1000.0, 300.0);
        let _ = frame(
            &ctx,
            &app,
            &mut view,
            input(vec![Event::PointerMoved(over_gantt)], Modifiers::NONE),
        );
        let mut got = Vec::new();
        for _ in 0..4 {
            let events = vec![
                Event::PointerMoved(over_gantt),
                wheel(vec2(0.0, 200.0), Modifiers::COMMAND),
            ];
            let (actions, _) = frame(&ctx, &app, &mut view, input(events, Modifiers::COMMAND));
            got.extend(actions);
        }
        // Ancre relative au bord gauche du Gantt (relecture B, I2).
        let expected = over_gantt.x - view.last_gantt_left;
        assert!(expected > 0.0 && expected < view.last_gantt_width);
        assert!(
            got.iter()
                .any(|a| matches!(a, Action::ZoomStep { steps: 1, anchor_x }
                if (*anchor_x - expected).abs() < 0.5)),
            "{got:?}"
        );
    }

    /// Les deux volets, qui relèvent chacun leur plage, dessinent les mêmes
    /// lignes (modèle de lignes réel de l'exemple), en haut puis après un
    /// défilement imposé.
    #[test]
    fn panes_share_row_range() {
        let ctx = egui::Context::default();
        let app = demo_app();
        let mut view = ViewTransient::default();
        let (_, stats) = frame(&ctx, &app, &mut view, input(Vec::new(), Modifiers::NONE));
        assert!(!stats.gantt_rows.is_empty(), "{stats:?}");
        assert_eq!(stats.gantt_rows.start, 0);
        assert_eq!(stats.table_rows, stats.gantt_rows);
        assert!(view.last_gantt_width > 0.0);
        view.pending_scroll_y = Some(20.0 * crate::view::ROW_H);
        let _ = frame(&ctx, &app, &mut view, input(Vec::new(), Modifiers::NONE));
        let (_, stats) = frame(&ctx, &app, &mut view, input(Vec::new(), Modifiers::NONE));
        assert!(stats.gantt_rows.start > 0, "{stats:?}");
        assert_eq!(stats.table_rows, stats.gantt_rows);
    }
}
