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
use crate::i18n::{Arg, Language, TextId, Texts};
use crate::io::{FileService, NoFileService, OpenEvent};
use crate::rows::RowModel;
use crate::state::{StateLoadError, TABLE_WIDTH_MAX, TABLE_WIDTH_MIN, ThemePref, UiState};
use crate::theme::Palette;
use crate::timescale::DayRange;
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

    /// Met une action en file ; elle sera appliquée en fin d'image.
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
        self.view = ViewTransient {
            needs_fit: true,
            ..ViewTransient::default()
        };
    }

    /// Transforme une erreur d'ouverture en message traduit.
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
            match event {
                OpenEvent::Cancelled => {}
                OpenEvent::Opened { origin, result } => match result {
                    Ok(loaded) => self.install(loaded, origin),
                    Err(error) => self.report_open_error(&error),
                },
            }
        }
    }

    /// Applique le thème (seulement au changement) et la langue résolue.
    fn sync_settings(&mut self, ctx: &egui::Context) {
        if self.applied_theme != Some(self.state.theme) {
            ctx.set_theme(match self.state.theme {
                ThemePref::System => egui::ThemePreference::System,
                ThemePref::Light => egui::ThemePreference::Light,
                ThemePref::Dark => egui::ThemePreference::Dark,
            });
            self.applied_theme = Some(self.state.theme);
        }
        let language = Language::resolve(self.state.language, self.env.system_language);
        if language != self.texts.language() {
            self.texts = Texts::new(language);
            if let Some(document) = &self.document {
                self.rows = Some(RowModel::build(document, &self.texts.format_labels()));
            }
        }
    }

    /// Réducteur : applique une action.
    fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::OpenDemo => self.open_demo(),
            Action::OpenDialog => {
                let now = (self.env.now)();
                let title = self.texts.get(TextId::DialogOpenTitle);
                let filter = self.texts.get(TextId::FileFilterMspdi);
                self.env
                    .files
                    .pick_and_open(ctx, now, title, filter, self.tx.clone());
            }
            Action::CloseDocument => {
                self.document = None;
                self.rows = None;
                self.selection = None;
                self.view = ViewTransient::default();
            }
            Action::Select(selection) => self.selection = selection,
            Action::SetLanguage(language) => self.state.language = language,
            Action::SetTheme(theme) => self.state.theme = theme,
            Action::SetZoom(zoom) => self.state.zoom = zoom,
            Action::SetTableWidth(width) => {
                self.state.table_width = width.clamp(TABLE_WIDTH_MIN, TABLE_WIDTH_MAX);
            }
            Action::ToggleDiagnostics => self.state.diagnostics_open = !self.state.diagnostics_open,
            Action::DismissMessage => self.messages.clear(),
            // Inertes dans le commit de contrat (lots L3, L4, L5).
            Action::FitZoom | Action::ZoomStep { .. } | Action::ScrollGanttX(_) | Action::ScrollToRow(_) => {}
        }
    }

    /// Une image complète dans `ui` (appelée par [`crate::ui`]).
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.poll_io();
        self.sync_settings(&ctx);
        let palette = Palette::for_theme(ctx.theme());
        let metrics = Metrics::default();
        let mut actions = std::mem::take(&mut self.actions);
        let mut stats = FrameStats::default();
        {
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
            egui::CentralPanel::default().show(ui, |ui| {
                if fc.doc.is_some() {
                    view::main_view::show(ui, &mut fc, view_state);
                } else {
                    view::chrome::welcome(ui, &mut fc);
                }
            });
        }
        self.last_frame = stats;
        for action in actions {
            self.apply(action, &ctx);
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
