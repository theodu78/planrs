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
use crate::timescale::{TimeAxis, TimeScaleZoom, ViewSpan, fit_zoom, zoom_anchor};
use crate::view::{self, DiagnosticsView, FrameCtx, FrameStats, Metrics, ViewTransient};

/// Nombre maximal de messages conservés dans le bandeau (les plus récents).
pub const MAX_MESSAGES: usize = 3;

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
    pub range: ViewSpan,
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
    /// Une ouverture par le sélecteur est en cours : indicateur seulement,
    /// une nouvelle demande relance le sélecteur.
    opening: bool,
    /// Diagnostics du document triés et mis en texte (cache invalidé à
    /// l'ouverture et au changement de langue).
    diagnostics: DiagnosticsView,
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
            diagnostics: DiagnosticsView::default(),
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

    /// Diagnostics du document ouvert, triés par gravité et mis en texte une
    /// fois par document et par langue, avec leurs compteurs (vide sans
    /// document).
    #[must_use]
    pub const fn diagnostics_view(&self) -> &DiagnosticsView {
        &self.diagnostics
    }

    /// Ajoute un message au bandeau : un texte déjà affiché n'est pas
    /// répété, et seuls les [`MAX_MESSAGES`] plus récents sont gardés.
    fn push_message(&mut self, severity: Severity, text: String) {
        self.messages.retain(|message| message.text != text);
        self.messages.push(UserMessage { severity, text });
        let excess = self.messages.len().saturating_sub(MAX_MESSAGES);
        if excess > 0 {
            self.messages.drain(..excess);
        }
    }

    /// Signale (une fois) un état d'interface illisible ou trop récent.
    pub fn report_state_load(&mut self, error: &StateLoadError) {
        let text = self.texts.get_args(
            TextId::StateLoadWarning,
            &[("detail", Arg::Str(error.to_string().into()))],
        );
        self.push_message(Severity::Warning, text);
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
            last_gantt_left: self.view.last_gantt_left,
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
        self.rows = Some(RowModel::build(&document, self.texts.format_labels()));
        self.diagnostics = DiagnosticsView::build(&document);
        self.document = Some(document);
        self.selection = None;
        self.view = self.fresh_view(true);
        // Une ouverture réussie rend caduques les erreurs et avertissements
        // d'ouverture précédents.
        let truncated = self.texts.get(TextId::RangeTruncated);
        self.messages
            .retain(|message| message.severity != Severity::Error && message.text != truncated);
        if loaded.range_truncated {
            self.push_message(Severity::Warning, truncated);
        }
    }

    /// Transforme une erreur d'ouverture en message traduit (le titre
    /// `open-error-title` est ajouté par le bandeau).
    fn report_open_error(&mut self, error: &OpenError) {
        let (id, detail) = match error {
            OpenError::Read(detail) => (TextId::OpenErrorRead, detail.clone()),
            OpenError::Mspdi(error) => (TextId::OpenErrorMspdi, error.to_string()),
        };
        let text = self.texts.get_args(id, &[("detail", Arg::Str(detail.into()))]);
        self.push_message(Severity::Error, text);
    }

    /// Relève les résultats d'ouverture arrivés depuis l'image précédente.
    /// Seule la demande en cours peut répondre : une demande abandonnée
    /// (voir [`App::open_dialog`]) écrit dans un canal fermé.
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
            self.rows = Some(RowModel::build(document, self.texts.format_labels()));
            self.diagnostics = DiagnosticsView::build(document);
        }
    }

    /// Largeur du Gantt mesurée par la vue à la dernière image (0 tant
    /// qu'aucune image n'a été dessinée).
    fn current_gantt_width(&self) -> f32 {
        let measured = self.view.last_gantt_width;
        if measured.is_finite() && measured > 0.0 {
            measured
        } else {
            0.0
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
    /// largeur du Gantt est connue. Après une ouverture (défilement nul),
    /// le défilement est seulement borné ; pour le bouton « Ajuster »,
    /// [`App::apply`] ancre ensuite au centre. Rend `true` si le zoom a
    /// changé.
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

    /// Change le zoom en gardant à la même place le point du contenu situé
    /// à `anchor` pixels du bord gauche du Gantt (borné à la largeur du
    /// Gantt ; une ancre non finie vaut 0). Arithmétique en pixels
    /// seulement.
    fn zoom_to(&mut self, to: TimeScaleZoom, anchor: f32) {
        let from = self.state.zoom;
        self.view.needs_fit = false;
        if to == from {
            return;
        }
        let anchor = if anchor.is_finite() {
            anchor.clamp(0.0, self.current_gantt_width())
        } else {
            0.0
        };
        self.view.gantt_scroll_x = zoom_anchor(self.view.gantt_scroll_x, anchor, from, to);
        self.state.zoom = to;
        self.clamp_gantt_scroll();
    }

    /// Centre du Gantt visible, ancre des boutons de zoom et d'« Ajuster ».
    fn gantt_center(&self) -> f32 {
        self.current_gantt_width() / 2.0
    }

    /// Réducteur : applique une action.
    fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::OpenDemo => self.open_demo(),
            Action::OpenDialog => self.open_dialog(ctx),
            Action::CloseDocument => {
                self.document = None;
                self.rows = None;
                self.diagnostics = DiagnosticsView::default();
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
            // Boutons et menu de zoom : ancrés au centre du Gantt (la zone
            // affichée reste à l'écran, comme dans MS Project).
            Action::SetZoom(zoom) => self.zoom_to(zoom, self.gantt_center()),
            Action::FitZoom => {
                if let Some(document) = &self.document {
                    let width = self.current_gantt_width();
                    if width > 0.0 {
                        let zoom = fit_zoom(document.range, width);
                        self.zoom_to(zoom, self.gantt_center());
                    } else {
                        // Largeur encore inconnue : ajustement à la
                        // prochaine image.
                        self.view.needs_fit = true;
                    }
                }
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

    /// Lance le sélecteur de fichiers. Si une demande précédente n'a jamais
    /// répondu (sélecteur web sans événement d'annulation, lecture
    /// `FileReader` bloquée), elle est abandonnée : un nouveau canal la
    /// remplace, si bien que son résultat éventuel, envoyé dans l'ancien
    /// canal fermé, est ignoré. Jamais de blocage de « Ouvrir » (relecture
    /// A, I3).
    fn open_dialog(&mut self, ctx: &egui::Context) {
        if self.opening {
            log::warn!("ouverture précédente sans réponse : abandonnée, sélecteur relancé");
            let (tx, rx) = mpsc::channel();
            self.tx = tx;
            self.rx = rx;
        }
        self.opening = true;
        let now = (self.env.now)();
        let title = self.texts.get(TextId::DialogOpenTitle);
        let filter = self.texts.get(TextId::FileFilterMspdi);
        self.env
            .files
            .pick_and_open(ctx, now, title, filter, self.tx.clone());
    }

    /// Pas de zoom (Ctrl/Cmd + molette) ancré sous le pointeur, `anchor_x`
    /// étant relatif au bord gauche du Gantt.
    fn zoom_step(&mut self, steps: i8, anchor_x: f32) {
        let mut to = self.state.zoom;
        for _ in 0..steps.unsigned_abs() {
            to = if steps > 0 { to.finer() } else { to.coarser() };
        }
        if to != self.state.zoom {
            self.zoom_to(to, anchor_x);
        }
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
                diagnostics: &self.diagnostics,
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
                    view::main_view::show(ui, &mut fc, view_state);
                });
            } else {
                egui::CentralPanel::default().show(ui, |ui| {
                    view::chrome::welcome(ui, &mut fc);
                });
            }
        }
        self.last_frame = stats;
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
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use planrs_core::Moment;

    use planrs_core::Severity;

    use super::{App, Env};
    use crate::action::Action;
    use crate::i18n::{Language, TextId};
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

    /// Ouverture synchrone réussie après un échec : nouvelle génération,
    /// erreur purgée, sélection remise à zéro, diagnostics recalculés.
    #[test]
    fn open_bytes_success_installs_new_generation() {
        let mut app = test_app();
        app.open_demo();
        app.selection = app.rows().and_then(|rows| rows.row(2)).map(|row| row.uid);
        app.open_bytes(b"pas du MSPDI", super::DocOrigin::default());
        assert_eq!(app.messages().len(), 1);
        assert_eq!(app.document.as_ref().unwrap().generation, 1);
        app.open_bytes(crate::DEMO_MSPDI, super::DocOrigin::default());
        assert_eq!(app.document.as_ref().unwrap().generation, 2);
        assert!(app.messages().is_empty(), "{:?}", app.messages());
        assert!(app.selection.is_none());
        assert!(app.view().needs_fit);
        assert_eq!(app.diagnostics_view(), &crate::view::DiagnosticsView::default());
    }

    /// Relecture A, B1 : une plage tronquée à 20 ans est signalée par un
    /// avertissement traduit, une seule fois.
    #[test]
    fn truncated_range_warns_once() {
        let mut app = test_app();
        for _ in 0..2 {
            let mut loaded =
                crate::document::load(crate::DEMO_MSPDI, "2027-01-15T08:00".parse().unwrap()).unwrap();
            loaded.range_truncated = true;
            app.install(loaded, super::DocOrigin::default());
        }
        let expected = app.texts().get(TextId::RangeTruncated);
        assert_eq!(app.messages().len(), 1, "{:?}", app.messages());
        assert_eq!(app.messages()[0].severity, Severity::Warning);
        assert_eq!(app.messages()[0].text, expected);
        // Un document non tronqué retire l'avertissement.
        app.open_demo();
        assert!(app.messages().is_empty(), "{:?}", app.messages());
    }

    /// Le bandeau garde au plus `MAX_MESSAGES` messages, sans doublon.
    #[test]
    fn messages_are_capped_and_deduplicated() {
        let mut app = test_app();
        for i in 0..10 {
            app.push_message(Severity::Error, format!("m{i}"));
            app.push_message(Severity::Error, format!("m{i}"));
        }
        let texts: Vec<&str> = app.messages().iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, ["m7", "m8", "m9"]);
        assert_eq!(texts.len(), super::MAX_MESSAGES);
        // Le même échec répété n'empile pas de lignes.
        let mut app = test_app();
        for _ in 0..40 {
            app.open_bytes(b"<x", super::DocOrigin::default());
        }
        assert_eq!(app.messages().len(), 1);
    }

    /// Diagnostics triés et comptés une fois : cache recalculé au changement
    /// de langue, vidé à la fermeture ; « → » rendu en « -> ».
    #[test]
    fn diagnostics_view_is_cached_per_document() {
        let xml = std::str::from_utf8(crate::DEMO_MSPDI).unwrap().replacen(
            "<PredecessorUID>",
            "<PredecessorUID>777",
            1,
        );
        let ctx = egui::Context::default();
        let mut app = test_app();
        app.open_bytes(xml.as_bytes(), super::DocOrigin::default());
        assert!(app.document.is_some(), "{:?}", app.messages());
        let view = app.diagnostics_view().clone();
        assert!(!view.entries.is_empty(), "un lien pendant doit être signalé");
        assert!(view.entries.iter().all(|(_, text)| !text.contains('\u{2192}')));
        let ranks: Vec<u8> = view
            .entries
            .iter()
            .map(|(severity, _)| match severity {
                Severity::Error => 0,
                Severity::Warning => 1,
                Severity::Info => 2,
            })
            .collect();
        assert!(ranks.windows(2).all(|w| w[0] <= w[1]), "{ranks:?}");
        let errors = view.entries.iter().filter(|(s, _)| *s == Severity::Error).count();
        assert_eq!(view.errors, errors);
        app.dispatch(Action::SetLanguage(LanguagePref::En));
        frame(&ctx, &mut app);
        assert_eq!(app.diagnostics_view().entries.len(), view.entries.len());
        app.dispatch(Action::CloseDocument);
        frame(&ctx, &mut app);
        assert_eq!(app.diagnostics_view().entries.len(), 0);
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
