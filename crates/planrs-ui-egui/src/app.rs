//! Application pure (sans eframe) : document ouvert, environnement injecté,
//! réducteur d'`Action`, relève des ouvertures asynchrones.
//!
//! Une image : relève des `OpenEvent` → synchronisation du thème et de la
//! langue → dessin des vues (qui poussent des actions) → réducteur. `App` se
//! construit sans `Frame` ni stockage : les tests kittest l'utilisent telle
//! quelle avec [`Env::fixed`].

use std::sync::mpsc::{self, Receiver, Sender};

use planrs_core::{Moment, Project, Schedule, Severity, TaskUid};
use planrs_mspdi::ImportDiagnostic;

use crate::action::Action;
use crate::document::{self, LoadedDocument, OpenError};
use crate::geom;
use crate::i18n::{Arg, Language, TextId, Texts};
use crate::io::{FileService, NoFileService, OpenEvent};
use crate::rows::RowModel;
use crate::state::{StateLoadError, TABLE_WIDTH_MAX, TABLE_WIDTH_MIN, ThemePref, UiState};
use crate::theme::Palette;
use crate::timescale::{DayRange, TimeAxis, fit_zoom, zoom_anchor};
use crate::view::{self, FrameCtx, FrameStats, Metrics, ViewTransient};

/// Provenance d'un document ouvert.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocOrigin {
    /// Nom affiché (nom de fichier, ou nom traduit de l'exemple).
    pub name: String,
    /// Chemin UTF-8 sur le desktop ; `None` sur le web et pour l'exemple.
    pub path: Option<String>,
    /// Projet d'exemple embarqué.
    pub demo: bool,
}

/// Document ouvert, en lecture seule : planning calculé une seule fois, à
/// l'ouverture.
#[derive(Clone, Debug)]
pub struct OpenDocument {
    /// Modèle lu.
    pub project: Project,
    /// Planning calculé.
    pub schedule: Schedule,
    /// Diagnostics d'import.
    pub import_diagnostics: Vec<ImportDiagnostic>,
    /// Plage de jours de l'échelle de temps.
    pub range: DayRange,
    /// Provenance.
    pub origin: DocOrigin,
    /// Numéro d'ouverture : clé d'invalidation des caches.
    pub generation: u64,
}

/// Environnement injecté : seule porte vers l'horloge, les fichiers et la
/// langue du système.
pub struct Env {
    /// « Maintenant », lu à chaque ouverture (`ReadOptions::current_date`).
    pub now: Box<dyn Fn() -> Moment>,
    /// Langue du système, lue par l'application ; `None` en test (repli :
    /// français).
    pub system_language: Option<Language>,
    /// Sélecteur de fichiers.
    pub files: Box<dyn FileService>,
}

impl Env {
    /// Environnement de test : date fixe, aucun sélecteur, langue du système
    /// absente.
    #[must_use]
    pub fn fixed(now: Moment) -> Self {
        Self {
            now: Box::new(move || now),
            system_language: None,
            files: Box::new(NoFileService),
        }
    }
}

/// Message affiché à l'utilisateur (bandeau).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserMessage {
    /// Gravité.
    pub severity: Severity,
    /// Texte déjà traduit.
    pub text: String,
}

/// Application egui de planrs, indépendante d'eframe.
pub struct App {
    /// Préférences persistées.
    pub state: UiState,
    /// Document ouvert.
    pub document: Option<OpenDocument>,
    /// Tâche sélectionnée (transitoire).
    pub selection: Option<TaskUid>,
    /// Relevé de la dernière image.
    pub last_frame: FrameStats,
    /// Environnement injecté.
    env: Env,
    /// Messages de la langue courante.
    texts: Texts,
    /// Modèle de lignes du document, dans la langue courante.
    rows: Option<RowModel>,
    /// État transitoire de la vue.
    view: ViewTransient,
    /// File des actions en attente.
    actions: Vec<Action>,
    /// Messages affichés.
    messages: Vec<UserMessage>,
    /// Émetteur des résultats d'ouverture (cloné vers le sélecteur).
    tx: Sender<OpenEvent>,
    /// Récepteur des résultats d'ouverture.
    rx: Receiver<OpenEvent>,
    /// Thème appliqué au contexte egui (appel seulement au changement).
    applied_theme: Option<ThemePref>,
    /// Génération du prochain document ouvert.
    next_generation: u64,
    /// Une ouverture par le sélecteur est en cours (une seule à la fois).
    opening: bool,
    /// Abscisse écran du bord gauche du Gantt à la dernière image (ancre du
    /// zoom à la molette).
    gantt_left: f32,
    /// Largeur du Gantt à la dernière image, d'après la mise en page
    /// partagée (repli quand la vue n'a pas encore mesuré).
    gantt_width: f32,
}

impl App {
    /// Crée l'application (état normalisé, aucun effet).
    #[must_use]
    pub fn new(state: UiState, env: Env) -> Self {
        let state = state.normalized();
        let texts = Texts::new(Language::resolve(state.language, env.system_language));
        let (tx, rx) = mpsc::channel();
        Self {
            state,
            document: None,
            selection: None,
            last_frame: FrameStats::default(),
            env,
            texts,
            rows: None,
            view: ViewTransient::default(),
            actions: Vec::new(),
            messages: Vec::new(),
            tx,
            rx,
            applied_theme: None,
            next_generation: 1,
            opening: false,
            gantt_left: 0.0,
            gantt_width: 0.0,
        }
    }

    /// Ouvre le projet d'exemple embarqué (synchrone).
    pub fn open_demo(&mut self) {
        let origin = DocOrigin {
            name: self.texts.get(TextId::DemoFileName),
            path: None,
            demo: true,
        };
        self.open_bytes(crate::DEMO_MSPDI, origin);
    }

    /// Ouvre un document à partir de ses octets (synchrone : tests, exemple).
    /// Un échec devient un message ; le document précédent reste affiché.
    pub fn open_bytes(&mut self, bytes: &[u8], origin: DocOrigin) {
        let now = (self.env.now)();
        match document::load(bytes, now) {
            Ok(loaded) => self.install(loaded, origin),
            Err(error) => self.report_open_error(&error),
        }
    }

    /// Met une action en file ; elle sera appliquée par le réducteur en fin
    /// de la prochaine image (les effets sur le contexte egui, comme le
    /// thème, demandent ce contexte).
    pub fn dispatch(&mut self, action: Action) {
        self.actions.push(action);
    }

    /// Messages affichés.
    #[must_use]
    pub fn messages(&self) -> &[UserMessage] {
        &self.messages
    }

    /// Messages traduits de la langue courante.
    #[must_use]
    pub const fn texts(&self) -> &Texts {
        &self.texts
    }

    /// Modèle de lignes du document ouvert (dans la langue courante).
    #[must_use]
    pub const fn rows(&self) -> Option<&RowModel> {
        self.rows.as_ref()
    }

    /// État transitoire de la vue (défilements, ajustement en attente).
    #[must_use]
    pub const fn view(&self) -> &ViewTransient {
        &self.view
    }

    /// Une ouverture par le sélecteur de fichiers est en cours.
    #[must_use]
    pub const fn is_opening(&self) -> bool {
        self.opening
    }

    /// Signale (une fois) un état d'interface illisible ou trop récent.
    pub fn report_state_load(&mut self, error: &StateLoadError) {
        let text = self.texts.get_args(
            TextId::StateLoadWarning,
            &[("detail", Arg::Str(error.to_string().into()))],
        );
        self.messages.push(UserMessage {
            severity: Severity::Warning,
            text,
        });
    }

    /// Vue remise à zéro pour un nouveau document (ou aucun), en gardant la
    /// largeur mesurée du Gantt pour ajuster le zoom dès la première image.
    fn fresh_view(&self, needs_fit: bool) -> ViewTransient {
        ViewTransient {
            pending_scroll_y: Some(0.0),
            gantt_scroll_x: 0.0,
            zoom_accum: 0.0,
            needs_fit,
            last_gantt_width: self.view.last_gantt_width,
        }
    }

    /// Installe un document chargé et reconstruit les caches.
    fn install(&mut self, loaded: LoadedDocument, origin: DocOrigin) {
        let generation = self.next_generation;
        self.next_generation = generation.saturating_add(1);
        let document = OpenDocument {
            project: loaded.project,
            schedule: loaded.schedule,
            import_diagnostics: loaded.import_diagnostics,
            range: loaded.range,
            origin,
            generation,
        };
        self.rows = Some(RowModel::build(&document, &self.texts.format_labels()));
        self.document = Some(document);
        self.selection = None;
        self.view = self.fresh_view(true);
        // Une ouverture réussie rend caduques les erreurs d'ouverture
        // précédentes.
        self.messages
            .retain(|message| message.severity != Severity::Error);
    }

    /// Transforme une erreur d'ouverture en message traduit (le titre
    /// `open-error-title` est ajouté par le bandeau).
    fn report_open_error(&mut self, error: &OpenError) {
        let (id, detail) = match error {
            OpenError::Read(detail) => (TextId::OpenErrorRead, detail.clone()),
            OpenError::Mspdi(error) => (TextId::OpenErrorMspdi, error.to_string()),
        };
        let text = self.texts.get_args(id, &[("detail", Arg::Str(detail.into()))]);
        self.messages.push(UserMessage {
            severity: Severity::Error,
            text,
        });
    }

    /// Relève les résultats d'ouverture arrivés depuis l'image précédente.
    fn poll_io(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            self.opening = false;
            match event {
                OpenEvent::Cancelled => {}
                OpenEvent::Opened { origin, result } => match result {
                    Ok(loaded) => self.install(loaded, origin),
                    Err(error) => self.report_open_error(&error),
                },
            }
        }
    }

    /// Applique la préférence de thème au contexte egui, seulement si elle a
    /// changé depuis le dernier appel.
    fn apply_theme(&mut self, ctx: &egui::Context) {
        if self.applied_theme != Some(self.state.theme) {
            ctx.set_theme(match self.state.theme {
                ThemePref::System => egui::ThemePreference::System,
                ThemePref::Light => egui::ThemePreference::Light,
                ThemePref::Dark => egui::ThemePreference::Dark,
            });
            self.applied_theme = Some(self.state.theme);
        }
    }

    /// Recrée les messages et le modèle de lignes si la langue résolue a
    /// changé.
    fn apply_language(&mut self) {
        let language = Language::resolve(self.state.language, self.env.system_language);
        if language == self.texts.language() {
            return;
        }
        self.texts = Texts::new(language);
        if let Some(document) = &mut self.document {
            if document.origin.demo {
                document.origin.name = self.texts.get(TextId::DemoFileName);
            }
            self.rows = Some(RowModel::build(document, &self.texts.format_labels()));
        }
    }

    /// Largeur courante du Gantt : mesure de la vue, sinon mise en page.
    fn current_gantt_width(&self) -> f32 {
        let measured = self.view.last_gantt_width;
        if measured.is_finite() && measured > 0.0 {
            measured
        } else {
            self.gantt_width
        }
    }

    /// Défilement horizontal maximal du Gantt au zoom courant.
    fn max_gantt_scroll(&self) -> f32 {
        let Some(document) = &self.document else {
            return 0.0;
        };
        let content = TimeAxis::new(document.range, self.state.zoom, 0.0).content_width();
        let max = content - self.current_gantt_width();
        if max.is_finite() { max.max(0.0) } else { 0.0 }
    }

    /// Ramène le défilement horizontal dans `[0, max]`.
    fn clamp_gantt_scroll(&mut self) {
        let max = self.max_gantt_scroll();
        let x = self.view.gantt_scroll_x;
        self.view.gantt_scroll_x = if x.is_finite() { x.clamp(0.0, max) } else { 0.0 };
    }

    /// Ajuste le zoom à la plage du document si c'est demandé et que la
    /// largeur du Gantt est connue. Rend `true` si le zoom a été ajusté.
    fn fit_if_needed(&mut self) -> bool {
        if !self.view.needs_fit {
            return false;
        }
        let Some(document) = &self.document else {
            self.view.needs_fit = false;
            return false;
        };
        let width = self.current_gantt_width();
        if !(width.is_finite() && width > 0.0) {
            return false;
        }
        let zoom = fit_zoom(document.range, width);
        self.view.needs_fit = false;
        let changed = zoom != self.state.zoom;
        self.state.zoom = zoom;
        self.clamp_gantt_scroll();
        changed
    }

    /// Réducteur : applique une action.
    fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::OpenDemo => self.open_demo(),
            Action::OpenDialog => self.open_dialog(ctx),
            Action::CloseDocument => {
                self.document = None;
                self.rows = None;
                self.selection = None;
                self.view = self.fresh_view(false);
            }
            Action::Select(selection) => self.selection = selection,
            Action::SetLanguage(language) => {
                self.state.language = language;
                self.apply_language();
            }
            Action::SetTheme(theme) => {
                self.state.theme = theme;
                self.apply_theme(ctx);
            }
            Action::SetZoom(zoom) => {
                self.state.zoom = zoom;
                self.view.needs_fit = false;
                self.clamp_gantt_scroll();
            }
            Action::FitZoom => {
                self.view.needs_fit = self.document.is_some();
                self.fit_if_needed();
            }
            Action::ZoomStep { steps, anchor_x } => self.zoom_step(steps, anchor_x),
            Action::SetTableWidth(width) => {
                self.state.table_width = width.clamp(TABLE_WIDTH_MIN, TABLE_WIDTH_MAX);
            }
            Action::ScrollGanttX(delta) => {
                if delta.is_finite() {
                    self.view.gantt_scroll_x += delta;
                }
                self.clamp_gantt_scroll();
            }
            Action::ScrollToRow(index) => {
                let len = self.rows.as_ref().map_or(0, RowModel::len);
                let row = index.min(len.saturating_sub(1));
                self.view.pending_scroll_y = Some(geom::index_to_f32(row) * view::ROW_H);
            }
            Action::ToggleDiagnostics => self.state.diagnostics_open = !self.state.diagnostics_open,
            Action::DismissMessage => self.messages.clear(),
        }
    }

    /// Lance le sélecteur de fichiers (une seule ouverture à la fois).
    fn open_dialog(&mut self, ctx: &egui::Context) {
        if self.opening {
            let text = self.texts.get(TextId::OpenInProgress);
            if !self.messages.iter().any(|message| message.text == text) {
                self.messages.push(UserMessage {
                    severity: Severity::Info,
                    text,
                });
            }
            return;
        }
        self.opening = true;
        let now = (self.env.now)();
        let title = self.texts.get(TextId::DialogOpenTitle);
        let filter = self.texts.get(TextId::FileFilterMspdi);
        self.env
            .files
            .pick_and_open(ctx, now, title, filter, self.tx.clone());
    }

    /// Pas de zoom ancré sous le pointeur (abscisse écran `anchor_x`).
    fn zoom_step(&mut self, steps: i8, anchor_x: f32) {
        let from = self.state.zoom;
        let mut to = from;
        for _ in 0..steps.unsigned_abs() {
            to = if steps > 0 { to.finer() } else { to.coarser() };
        }
        if to == from {
            return;
        }
        let offset = anchor_x - self.gantt_left;
        let offset = if offset.is_finite() {
            offset.clamp(0.0, self.current_gantt_width().max(0.0))
        } else {
            0.0
        };
        self.view.gantt_scroll_x = zoom_anchor(self.view.gantt_scroll_x, offset, from, to);
        self.state.zoom = to;
        self.view.needs_fit = false;
        self.clamp_gantt_scroll();
    }

    /// Une image complète dans `ui` (appelée par [`crate::ui`]) : relève des
    /// ouvertures, réglages, panneaux, vue centrale, puis réducteur.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.poll_io();
        self.apply_theme(&ctx);
        self.apply_language();
        let palette = Palette::for_theme(ctx.theme());
        let metrics = Metrics::default();
        let mut actions = std::mem::take(&mut self.actions);
        let mut stats = FrameStats::default();
        let mut layout_gantt: Option<(f32, f32)> = None;
        {
            // Champs disjoints : le contexte d'image emprunte l'état en
            // lecture, la vue transitoire est empruntée à part.
            let mut fc = FrameCtx {
                state: &self.state,
                doc: self.document.as_ref(),
                rows: self.rows.as_ref(),
                texts: &self.texts,
                palette: &palette,
                metrics: &metrics,
                selection: self.selection,
                actions: &mut actions,
                stats: &mut stats,
            };
            view::chrome::menu_bar(ui, &mut fc);
            view::chrome::toolbar(ui, &mut fc);
            view::chrome::status_bar(ui, &mut fc, &self.messages);
            if self.state.diagnostics_open {
                view::diagnostics::panel(ui, &mut fc);
            }
            let view_state = &mut self.view;
            if fc.doc.is_some() {
                let frame = egui::Frame::central_panel(ui.style()).inner_margin(0.0);
                egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                    let avail = ui.available_rect_before_wrap();
                    let layout = geom::main_layout(avail, f32::from(fc.state.table_width), fc.metrics);
                    layout_gantt = Some((layout.gantt_clip.left(), layout.gantt_clip.width()));
                    view::main_view::show(ui, &mut fc, view_state);
                });
            } else {
                egui::CentralPanel::default().show(ui, |ui| {
                    view::chrome::welcome(ui, &mut fc);
                });
            }
        }
        self.last_frame = stats;
        if let Some((left, width)) = layout_gantt
            && left.is_finite()
            && width.is_finite()
        {
            self.gantt_left = left;
            self.gantt_width = width.max(0.0);
        }
        // Les actions sont appliquées après le dessin : une image de plus
        // montre leur effet (langue, zoom, document…).
        let applied = !actions.is_empty();
        for action in actions {
            self.apply(action, &ctx);
        }
        // Ajustement du zoom après une ouverture, dès que la largeur du
        // Gantt est connue : là aussi, une image de plus au bon zoom.
        let fitted = self.fit_if_needed();
        if applied || fitted {
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use planrs_core::Moment;

    use super::{App, Env};
    use crate::action::Action;
    use crate::i18n::Language;
    use crate::state::{LanguagePref, ThemePref, UiState};

    /// Application de test : date fixe, français, thème clair.
    fn test_app() -> App {
        let now: Moment = "2027-01-15T08:00".parse().unwrap();
        let state = UiState {
            language: LanguagePref::Fr,
            theme: ThemePref::Light,
            ..UiState::default()
        };
        App::new(state, Env::fixed(now))
    }

    /// Une image sans GPU (textures ignorées : aucun moteur de rendu).
    fn frame(ctx: &egui::Context, app: &mut App) {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| crate::ui(ui, app));
        output.textures_delta.clear();
    }

    #[test]
    fn open_demo_loads_document() {
        let mut app = test_app();
        app.open_demo();
        let document = app.document.as_ref().expect("exemple chargé");
        assert!(document.origin.demo);
        assert_eq!(document.generation, 1);
        assert!(app.messages().is_empty(), "{:?}", app.messages());
    }

    #[test]
    fn invalid_bytes_become_message() {
        let mut app = test_app();
        app.open_bytes(b"pas du MSPDI", super::DocOrigin::default());
        assert!(app.document.is_none());
        assert_eq!(app.messages().len(), 1);
    }

    #[test]
    fn welcome_frame_and_reducer() {
        let ctx = egui::Context::default();
        let mut app = test_app();
        frame(&ctx, &mut app);
        assert_eq!(ctx.theme(), egui::Theme::Light);
        app.dispatch(Action::SetLanguage(LanguagePref::En));
        app.dispatch(Action::SetTheme(ThemePref::Dark));
        frame(&ctx, &mut app);
        frame(&ctx, &mut app);
        assert_eq!(app.texts().language(), Language::En);
        assert_eq!(ctx.theme(), egui::Theme::Dark);
        app.dispatch(Action::OpenDemo);
        frame(&ctx, &mut app);
        assert!(app.document.is_some());
        frame(&ctx, &mut app);
    }
}
