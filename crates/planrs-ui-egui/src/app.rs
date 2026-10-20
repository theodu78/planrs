//! Application pure (sans eframe) : document ouvert (une `Session`
//! d'édition), environnement injecté, réducteur d'`Action`, relève des
//! ouvertures asynchrones.
//!
//! Une image : relève des `OpenEvent` → synchronisation du thème et de la
//! langue → raccourcis clavier ([`crate::keymap::KEYMAP`]) → dessin des vues
//! (qui poussent des actions) → réducteur → titre de la fenêtre. `App` se
//! construit sans `Frame` ni stockage : les tests kittest l'utilisent telle
//! quelle avec [`Env::fixed`].
//!
//! Toute édition passe par [`App::execute`] (lot de commandes du registre,
//! ADR-0012 D11), [`App::undo`] ou [`App::redo`] ; après chacune,
//! [`App::refresh`] reconstruit les caches d'affichage.
//!
//! Fichiers (lot L13) : « Nouveau » ([`App::new_project`]), « Enregistrer »
//! et « Enregistrer sous » (octets de `Session::save_bytes` remis au
//! [`FileService`] ; `mark_saved` seulement à la réception du succès, et
//! seulement si le document n'a pas changé entre-temps). Fermer, ouvrir un
//! autre document, créer un projet ou quitter avec un document modifié
//! affiche un bandeau non bloquant (Enregistrer / Ne pas enregistrer /
//! Annuler) ; sur le desktop, la fermeture de la fenêtre est annulée
//! (`ViewportCommand::CancelClose`) jusqu'à la réponse.

use std::sync::mpsc::{self, Receiver, Sender};

use planrs_automation::error::codes;
use planrs_automation::{
    AnchorSpec, ApplyMode, ApplyOutcome, Batch, CommandError, CommandErrorKind, HistoryError, NewProject,
    Session,
};
use planrs_core::{Day, Moment, PRODUCT_NAME, Project, Schedule, Severity, TaskUid, TimeOfDay, Weekday};
use planrs_mspdi::ImportDiagnostic;

use crate::action::{Action, ConfirmChoice};
use crate::document::{self, LoadedDocument, OpenError};
use crate::edit::TableEdit;
use crate::geom;
use crate::i18n::{Arg, Language, TextId, Texts, error_text_id};
use crate::io::{
    FileService, NoFileService, OpenEvent, SaveEvent, SaveRequest, SaveTarget, suggested_file_name,
};
use crate::rows::RowModel;
use crate::selection::Selection;
use crate::state::{StateLoadError, TABLE_WIDTH_MAX, TABLE_WIDTH_MIN, ThemePref, UiState};
use crate::structure::PendingDelete;
use crate::theme::Palette;
use crate::timescale::{
    MAX_RANGE_DAYS, TimeAxis, TimeScaleZoom, ViewSpan, default_range, fit_zoom, zoom_anchor,
};
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

impl DocOrigin {
    /// Origine de la session (nom et chemin, sans le drapeau d'exemple).
    #[must_use]
    pub fn to_session(&self) -> planrs_automation::DocOrigin {
        planrs_automation::DocOrigin {
            name: self.name.clone(),
            path: self.path.clone(),
        }
    }
}

/// Document ouvert : une session d'édition (modèle, planning calculé par le
/// moteur, historique), recalculée à chaque lot de commandes.
#[derive(Clone, Debug)]
pub struct OpenDocument {
    /// Session d'édition. Ne pas la modifier directement : passer par
    /// [`App::execute`], [`App::undo`] et [`App::redo`], qui reconstruisent
    /// les caches d'affichage.
    pub session: Session,
    /// Diagnostics d'import.
    pub import_diagnostics: Vec<ImportDiagnostic>,
    /// Plage de jours de l'échelle de temps.
    pub range: ViewSpan,
    /// Provenance (nom et chemin aussi tenus par la session, voir
    /// [`OpenDocument::set_origin`]).
    pub origin: DocOrigin,
    /// Numéro d'ouverture (1, 2…) : avec [`OpenDocument::generation`], clé
    /// d'invalidation des caches.
    pub opening: u64,
}

impl OpenDocument {
    /// Document d'un chargement réussi : l'origine est aussi posée sur la
    /// session.
    #[must_use]
    pub fn new(loaded: LoadedDocument, origin: DocOrigin, opening: u64) -> Self {
        let mut session = loaded.session;
        session.set_origin(origin.to_session());
        Self {
            session,
            import_diagnostics: loaded.import_diagnostics,
            range: loaded.range,
            origin,
            opening,
        }
    }

    /// Modèle courant.
    #[must_use]
    pub const fn project(&self) -> &Project {
        self.session.project()
    }

    /// Planning courant (calculé par le moteur).
    #[must_use]
    pub const fn schedule(&self) -> &Schedule {
        self.session.schedule()
    }

    /// Génération de la session : +1 à chaque commit, annulation ou
    /// rétablissement.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.session.generation()
    }

    /// Clé des caches d'affichage : (numéro d'ouverture, génération).
    #[must_use]
    pub const fn cache_key(&self) -> (u64, u64) {
        (self.opening, self.generation())
    }

    /// Le document diffère de son dernier enregistrement (R-164).
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.session.is_dirty()
    }

    /// Change l'origine (« Enregistrer sous ») ici et dans la session.
    pub fn set_origin(&mut self, origin: DocOrigin) {
        self.session.set_origin(origin.to_session());
        self.origin = origin;
    }
}

/// Provenance d'une édition passée à [`App::execute`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditOrigin {
    /// L'utilisateur (menu, raccourci, saisie) : un refus s'affiche dans
    /// le bandeau.
    User,
    /// Un agent (canal de contrôle, J7) : le refus est rendu à l'appelant,
    /// qui le relaie ; rien ne s'affiche.
    Agent,
}

/// Observateur du document ouvert (application desktop : rendez-vous du
/// canal de contrôle, ADR-0002). Appelé après chaque ouverture, fermeture,
/// commit, annulation ou rétablissement ; jamais pendant le dessin.
pub trait DocumentObserver {
    /// Le document courant a changé (`None` : aucun document).
    fn document_changed(&mut self, document: Option<&OpenDocument>);
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

/// Opération suspendue au bandeau de confirmation (document modifié).
#[derive(Debug)]
enum Pending {
    /// Fermer le document.
    Close,
    /// Ouvrir l'exemple.
    OpenDemo,
    /// Lancer le sélecteur de fichiers.
    OpenDialog,
    /// Créer un projet vide.
    NewProject,
    /// Fermer la fenêtre (desktop).
    Quit,
    /// Installer un document arrivé du sélecteur alors que le document
    /// courant a été modifié depuis la demande.
    Install(Box<(LoadedDocument, DocOrigin)>),
}

/// Enregistrement en cours.
#[derive(Debug)]
struct SaveJob {
    /// Clé du document enregistré (numéro d'ouverture, génération) au
    /// moment où ses octets ont été produits.
    key: (u64, u64),
    /// Opération à poursuivre après un enregistrement réussi.
    then: Option<Pending>,
    /// Nombre de valeurs ajustées au format MSPDI (diagnostics
    /// d'écriture), signalé après le succès.
    adjusted: usize,
}

/// Message affiché à l'utilisateur (bandeau).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserMessage {
    /// Gravité.
    pub severity: Severity,
    /// Titre affiché au-dessus du message (« Impossible d'ouvrir le
    /// fichier », « Modification refusée »), s'il y en a un.
    pub title: Option<TextId>,
    /// Texte déjà traduit.
    pub text: String,
}

/// Application egui de planrs, indépendante d'eframe.
pub struct App {
    /// Préférences persistées.
    pub state: UiState,
    /// Document ouvert.
    pub document: Option<OpenDocument>,
    /// Tâche sélectionnée (transitoire) : la tâche active de
    /// [`App::selected`].
    pub selection: Option<TaskUid>,
    /// Tâches sélectionnées (lot L14), resynchronisées sur
    /// [`App::selection`] quand elles divergent.
    pub(crate) selected: Selection,
    /// Suppression de récapitulative en attente de confirmation (lot L14).
    pub(crate) pending_delete: Option<PendingDelete>,
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
    /// l'ouverture, à chaque édition et au changement de langue).
    diagnostics: DiagnosticsView,
    /// Observateur du document (desktop), s'il y en a un.
    observer: Option<Box<dyn DocumentObserver>>,
    /// Dernier titre de fenêtre envoyé à eframe.
    sent_title: String,
    /// Opération en attente de la réponse au bandeau de confirmation.
    confirm: Option<Pending>,
    /// Enregistrement en cours (un seul à la fois).
    saving: Option<SaveJob>,
    /// Émetteur des résultats d'enregistrement (cloné vers le service).
    save_tx: Sender<SaveEvent>,
    /// Récepteur des résultats d'enregistrement.
    save_rx: Receiver<SaveEvent>,
    /// Fermeture de la fenêtre acceptée (« Ne pas enregistrer », ou
    /// enregistrement réussi) : la prochaine demande n'est plus annulée.
    quit_confirmed: bool,
    /// Clé du document au lancement du sélecteur : un document arrivé alors
    /// que le document courant a changé depuis redemande confirmation.
    open_confirmed_for: Option<(u64, u64)>,
}

impl App {
    /// Crée l'application (état normalisé, aucun effet).
    #[must_use]
    pub fn new(state: UiState, env: Env) -> Self {
        let state = state.normalized();
        let texts = Texts::new(Language::resolve(state.language, env.system_language));
        let (tx, rx) = mpsc::channel();
        let (save_tx, save_rx) = mpsc::channel();
        Self {
            state,
            document: None,
            selection: None,
            selected: Selection::default(),
            pending_delete: None,
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
            observer: None,
            sent_title: PRODUCT_NAME.to_owned(),
            confirm: None,
            saving: None,
            save_tx,
            save_rx,
            quit_confirmed: false,
            open_confirmed_for: None,
        }
    }

    /// Installe l'observateur du document (remplace le précédent) et lui
    /// signale aussitôt le document courant.
    pub fn set_observer(&mut self, observer: Box<dyn DocumentObserver>) {
        self.observer = Some(observer);
        self.notify_observer();
    }

    /// Signale le document courant à l'observateur.
    fn notify_observer(&mut self) {
        if let Some(observer) = &mut self.observer {
            observer.document_changed(self.document.as_ref());
        }
    }

    /// Langue des textes du moteur (messages d'erreur des commandes).
    fn text_language(&self) -> planrs_automation::text::Language {
        match self.texts.language() {
            Language::Fr => planrs_automation::text::Language::Fr,
            Language::En => planrs_automation::text::Language::En,
        }
    }

    /// Applique un lot de commandes au document (ADR-0012 D11) puis
    /// reconstruit l'affichage ([`App::refresh`]). Un refus laisse le
    /// document intact ; venant de l'utilisateur, il s'affiche dans le
    /// bandeau, traduit par son code (`error-<code>` des fichiers `.ftl`).
    ///
    /// # Errors
    ///
    /// Le refus du lot ([`CommandError`]), ou `session.none` sans document.
    pub fn execute(&mut self, batch: &Batch, origin: EditOrigin) -> Result<ApplyOutcome, CommandError> {
        let result = match &mut self.document {
            Some(document) => {
                let before = document.generation();
                document
                    .session
                    .apply(batch, ApplyMode::Commit)
                    .map(|outcome| (outcome, document.generation() != before))
            }
            None => Err(CommandError::new(
                codes::SESSION_NONE,
                CommandErrorKind::InvalidParam(match self.texts.language() {
                    Language::Fr => "aucun document ouvert".to_owned(),
                    Language::En => "no open document".to_owned(),
                }),
            )),
        };
        match result {
            Ok((outcome, changed)) => {
                self.clear_titled(TextId::CommandErrorTitle);
                if changed {
                    self.refresh();
                }
                Ok(outcome)
            }
            Err(error) => {
                if origin == EditOrigin::User {
                    self.report_command_error(&error);
                }
                Err(error)
            }
        }
    }

    /// Annule le dernier lot (R-164) puis reconstruit l'affichage. Rien à
    /// annuler : aucun effet, aucun message.
    ///
    /// # Errors
    ///
    /// [`HistoryError::Empty`] (rien à annuler, ou aucun document) ;
    /// [`HistoryError::Inconsistent`], aussi affiché dans le bandeau.
    pub fn undo(&mut self) -> Result<ApplyOutcome, HistoryError> {
        self.history_step(Session::undo)
    }

    /// Rétablit le dernier lot annulé (R-164) puis reconstruit
    /// l'affichage. Rien à rétablir : aucun effet, aucun message.
    ///
    /// # Errors
    ///
    /// Comme [`App::undo`].
    pub fn redo(&mut self) -> Result<ApplyOutcome, HistoryError> {
        self.history_step(Session::redo)
    }

    /// Annulation ou rétablissement.
    fn history_step(
        &mut self,
        step: fn(&mut Session) -> Result<ApplyOutcome, HistoryError>,
    ) -> Result<ApplyOutcome, HistoryError> {
        let Some(document) = &mut self.document else {
            return Err(HistoryError::Empty);
        };
        let result = step(&mut document.session);
        match &result {
            Ok(_) => {
                self.clear_titled(TextId::CommandErrorTitle);
                self.refresh();
            }
            Err(HistoryError::Empty) => {}
            Err(error) => {
                let text = self.texts.get_args(
                    error_text_id(error.code()),
                    &[("detail", Arg::Str(error.to_string().into()))],
                );
                self.push_titled(Severity::Error, Some(TextId::CommandErrorTitle), text);
            }
        }
        result
    }

    /// Reconstruit les caches d'affichage après une édition : modèle de
    /// lignes, diagnostics, plage = union de l'ancienne et de la nouvelle
    /// plage par défaut (la nouvelle seule si l'union dépasse
    /// [`MAX_RANGE_DAYS`]). La sélection est gardée par UID (retirée si la
    /// tâche n'existe plus) ; les défilements sont gardés (bornés).
    pub fn refresh(&mut self) {
        let Some(document) = &mut self.document else {
            return;
        };
        let range = default_range(document.project(), document.schedule()).span;
        document.range = union_span(document.range, range);
        self.rows = Some(RowModel::build(document, self.texts.format_labels()));
        self.diagnostics = DiagnosticsView::build(document);
        if self
            .selection
            .is_some_and(|uid| !document.project().tasks.contains(uid))
        {
            self.selection = None;
        }
        self.selected.retain(|uid| document.project().tasks.contains(uid));
        self.sync_selection();
        self.clamp_gantt_scroll();
        self.notify_observer();
    }

    /// Tâches sélectionnées (lot L14).
    #[must_use]
    pub const fn selected(&self) -> &Selection {
        &self.selected
    }

    /// Ramène [`App::selected`] à la seule [`App::selection`] si la tâche
    /// active a changé par ailleurs (clic simple, tâche supprimée, nouveau
    /// document).
    pub(crate) fn sync_selection(&mut self) {
        if self.selected.focus() != self.selection {
            self.selected = Selection::single(self.selection);
        }
    }

    /// Titre de la fenêtre : nom du document précédé de « * » s'il est
    /// modifié, puis nom du produit.
    #[must_use]
    pub fn window_title(&self) -> String {
        window_title_for(self.document.as_ref())
    }

    /// Crée un projet vide (R-032) : nom traduit « Projet1 », planifié
    /// depuis le lundi qui suit « maintenant », 08:00 (hypothèse, voir
    /// [`next_monday_morning`]). Ne demande aucune confirmation : passer
    /// par [`Action::NewProject`] pour protéger un document modifié.
    pub fn new_project(&mut self) {
        let now = (self.env.now)();
        let name = self.texts.get(TextId::NewProjectName);
        let init = NewProject {
            name: name.clone(),
            anchor: AnchorSpec::Start(next_monday_morning(now)),
            lang: Some(self.text_language()),
        };
        match Session::new_project(init, now) {
            Ok(session) => {
                let range = default_range(session.project(), session.schedule());
                let loaded = LoadedDocument {
                    session,
                    import_diagnostics: Vec::new(),
                    range: range.span,
                    range_truncated: range.truncated,
                };
                let origin = DocOrigin {
                    name,
                    path: None,
                    demo: false,
                };
                self.install(loaded, origin);
            }
            Err(error) => self.report_command_error(&error),
        }
    }

    /// Un bandeau « Enregistrer les modifications ? » attend une réponse.
    #[must_use]
    pub const fn confirmation_pending(&self) -> bool {
        self.confirm.is_some()
    }

    /// Un enregistrement est en cours.
    #[must_use]
    pub const fn is_saving(&self) -> bool {
        self.saving.is_some()
    }

    /// Le document courant est modifié depuis son dernier enregistrement.
    fn document_dirty(&self) -> bool {
        self.document.as_ref().is_some_and(OpenDocument::is_dirty)
    }

    /// Demande une opération qui remplacerait le document : bandeau de
    /// confirmation si le document est modifié, exécution sinon.
    fn request(&mut self, pending: Pending, ctx: &egui::Context) {
        if self.document_dirty() {
            self.confirm = Some(pending);
        } else {
            self.perform(pending, ctx);
        }
    }

    /// Exécute une opération confirmée (ou sans rien à perdre).
    fn perform(&mut self, pending: Pending, ctx: &egui::Context) {
        match pending {
            Pending::Close => self.close_document(),
            Pending::OpenDemo => self.open_demo(),
            Pending::OpenDialog => {
                self.open_confirmed_for = self.document.as_ref().map(OpenDocument::cache_key);
                self.open_dialog(ctx);
            }
            Pending::NewProject => self.new_project(),
            Pending::Quit => {
                self.quit_confirmed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Pending::Install(boxed) => {
                let (loaded, origin) = *boxed;
                self.install(loaded, origin);
            }
        }
    }

    /// Réponse au bandeau de confirmation.
    fn answer(&mut self, choice: ConfirmChoice, ctx: &egui::Context) {
        let Some(pending) = self.confirm.take() else {
            return;
        };
        match choice {
            ConfirmChoice::Save => self.start_save(false, Some(pending), ctx),
            ConfirmChoice::Discard => self.perform(pending, ctx),
            ConfirmChoice::Cancel => {}
        }
    }

    /// Ferme le document courant, sans confirmation.
    fn close_document(&mut self) {
        self.document = None;
        self.rows = None;
        self.diagnostics = DiagnosticsView::default();
        self.selection = None;
        self.view = self.fresh_view(false);
        self.notify_observer();
    }

    /// Lance un enregistrement : octets produits ici (`save_bytes`, qui
    /// n'altère rien), écrits par le [`FileService`]. Chemin connu et pas
    /// « Enregistrer sous » : écriture directe ; sinon boîte de dialogue
    /// (desktop) ou téléchargement (web). Un seul enregistrement à la fois
    /// sur le desktop (deux écritures du même fichier pourraient finir dans
    /// le désordre) ; sur le web, une demande restée sans réponse est
    /// abandonnée, comme pour l'ouverture.
    fn start_save(&mut self, save_as: bool, then: Option<Pending>, ctx: &egui::Context) {
        if self.saving.is_some() {
            if cfg!(target_arch = "wasm32") {
                log::warn!("enregistrement précédent sans réponse : abandonné");
                let (save_tx, save_rx) = mpsc::channel();
                self.save_tx = save_tx;
                self.save_rx = save_rx;
                self.saving = None;
            } else {
                let text = self.texts.get(TextId::SaveInProgress);
                self.push_message(Severity::Info, text);
                return;
            }
        }
        let Some(document) = &self.document else {
            return;
        };
        let written = document
            .session
            .save_bytes(&planrs_mspdi::write::WriteOptions::default());
        let target = match (&document.origin.path, save_as) {
            (Some(path), false) => SaveTarget::Path(path.clone()),
            _ => SaveTarget::Dialog {
                suggested_name: suggested_file_name(&document.origin.name),
                title: self.texts.get(TextId::DialogSaveTitle),
                filter: self.texts.get(TextId::FileFilterMspdi),
            },
        };
        self.saving = Some(SaveJob {
            key: document.cache_key(),
            then,
            adjusted: written.diagnostics.len(),
        });
        let request = SaveRequest {
            bytes: written.xml,
            target,
            language: self.texts.language(),
        };
        self.env.files.save(ctx, request, self.save_tx.clone());
    }

    /// Relève les résultats d'enregistrement arrivés depuis l'image
    /// précédente.
    fn poll_save(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.save_rx.try_recv() {
            let Some(job) = self.saving.take() else {
                continue;
            };
            match event {
                // Annulé : l'opération suspendue est abandonnée, le
                // document reste modifié.
                SaveEvent::Cancelled => {}
                SaveEvent::Failed(detail) => {
                    let text = self
                        .texts
                        .get_args(TextId::SaveErrorWrite, &[("detail", Arg::Str(detail.into()))]);
                    self.push_titled(Severity::Error, Some(TextId::SaveErrorTitle), text);
                }
                SaveEvent::Saved { name, path } => self.finish_save(job, name, path, ctx),
            }
        }
    }

    /// Enregistrement réussi : nouvelle origine, `mark_saved` si le document
    /// est resté celui dont les octets ont été écrits, puis opération
    /// suspendue éventuelle. Un document fermé ou remplacé entre-temps
    /// n'est pas touché.
    fn finish_save(&mut self, job: SaveJob, name: String, path: Option<String>, ctx: &egui::Context) {
        let Some(document) = &mut self.document else {
            return;
        };
        if document.opening != job.key.0 {
            return;
        }
        document.set_origin(DocOrigin {
            name,
            path,
            demo: false,
        });
        let saved = document.generation() == job.key.1;
        if saved {
            document.session.mark_saved();
        }
        self.clear_titled(TextId::SaveErrorTitle);
        if job.adjusted > 0 {
            let count = i64::try_from(job.adjusted).unwrap_or(i64::MAX);
            let text = self
                .texts
                .get_args(TextId::SaveAdjusted, &[("count", Arg::Count(count))]);
            self.push_message(Severity::Info, text);
        }
        self.notify_observer();
        if saved && let Some(pending) = job.then {
            self.perform(pending, ctx);
        }
    }

    /// Fermeture de la fenêtre demandée (desktop) : annulée si le document
    /// est modifié (bandeau de confirmation) ou si un enregistrement est en
    /// cours (la fenêtre se ferme quand il réussit).
    fn guard_close(&mut self, ctx: &egui::Context) {
        if self.quit_confirmed || !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        if let Some(job) = &mut self.saving {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if job.then.is_none() {
                job.then = Some(Pending::Quit);
            }
        } else if self.document_dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm = Some(Pending::Quit);
        }
    }

    /// Quitter demandé par l'application (Ctrl/Cmd+Q, menu Fichier), et non
    /// par la fenêtre : même garde que [`App::guard_close`]. Pendant un
    /// enregistrement, la fermeture suit son succès ; document modifié :
    /// bandeau ; sinon la fenêtre se ferme. Sans effet sur le web (une page
    /// ne ferme pas son onglet).
    fn request_quit(&mut self, ctx: &egui::Context) {
        if cfg!(target_arch = "wasm32") {
            return;
        }
        if let Some(job) = &mut self.saving {
            if job.then.is_none() {
                job.then = Some(Pending::Quit);
            }
        } else {
            self.request(Pending::Quit, ctx);
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
        self.push_titled(severity, None, text);
    }

    /// Comme [`App::push_message`], avec un titre.
    fn push_titled(&mut self, severity: Severity, title: Option<TextId>, text: String) {
        self.messages.retain(|message| message.text != text);
        self.messages.push(UserMessage {
            severity,
            title,
            text,
        });
        let excess = self.messages.len().saturating_sub(MAX_MESSAGES);
        if excess > 0 {
            self.messages.drain(..excess);
        }
    }

    /// Retire les messages de titre `title` (refus devenus caducs).
    fn clear_titled(&mut self, title: TextId) {
        self.messages.retain(|message| message.title != Some(title));
    }

    /// Affiche un refus de commande : message traduit par son code, détail
    /// du moteur dans la langue de l'interface.
    fn report_command_error(&mut self, error: &CommandError) {
        let detail = error.message(self.text_language());
        let text = self
            .texts
            .get_args(error_text_id(error.code), &[("detail", Arg::Str(detail.into()))]);
        self.push_titled(Severity::Error, Some(TextId::CommandErrorTitle), text);
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
            edit: TableEdit::default(),
            ..ViewTransient::default()
        }
    }

    /// Installe un document chargé et reconstruit les caches.
    fn install(&mut self, loaded: LoadedDocument, origin: DocOrigin) {
        let opening = self.next_generation;
        self.next_generation = opening.saturating_add(1);
        let range_truncated = loaded.range_truncated;
        let document = OpenDocument::new(loaded, origin, opening);
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
        if range_truncated {
            self.push_message(Severity::Warning, truncated);
        }
        self.notify_observer();
    }

    /// Transforme une erreur d'ouverture en message traduit (le titre
    /// `open-error-title` est ajouté par le bandeau).
    fn report_open_error(&mut self, error: &OpenError) {
        let (id, detail) = match error {
            OpenError::Read(detail) => (TextId::OpenErrorRead, detail.clone()),
            OpenError::Mspdi(error) => (TextId::OpenErrorMspdi, error.to_string()),
        };
        let text = self.texts.get_args(id, &[("detail", Arg::Str(detail.into()))]);
        self.push_titled(Severity::Error, Some(TextId::OpenErrorTitle), text);
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
                    Ok(loaded) => {
                        // Le document courant a été modifié depuis la
                        // demande (ou la confirmation) : nouvelle
                        // confirmation avant de le remplacer.
                        let confirmed = self.document.as_ref().is_none_or(|document| {
                            !document.is_dirty() || Some(document.cache_key()) == self.open_confirmed_for
                        });
                        if confirmed {
                            self.install(loaded, origin);
                        } else {
                            self.confirm = Some(Pending::Install(Box::new((loaded, origin))));
                        }
                    }
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
                let mut origin = document.origin.clone();
                origin.name = self.texts.get(TextId::DemoFileName);
                document.set_origin(origin);
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
            Action::OpenDemo => self.request(Pending::OpenDemo, ctx),
            Action::OpenDialog => self.request(Pending::OpenDialog, ctx),
            Action::CloseDocument => self.request(Pending::Close, ctx),
            Action::NewProject => self.request(Pending::NewProject, ctx),
            Action::Save => self.start_save(false, None, ctx),
            Action::SaveAs => self.start_save(true, None, ctx),
            Action::Quit => self.request_quit(ctx),
            Action::Confirm(choice) => self.answer(choice, ctx),
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
            // Le refus est affiché par `execute` ; rien d'autre à faire ici.
            Action::Exec(batch) => drop(self.execute(&batch, EditOrigin::User)),
            Action::ExecOn { opening, batch } => {
                if self
                    .document
                    .as_ref()
                    .is_some_and(|document| document.opening == opening)
                {
                    drop(self.execute(&batch, EditOrigin::User));
                } else {
                    log::warn!("saisie abandonnée : le document visé n'est plus ouvert");
                }
            }
            Action::Undo => drop(self.undo()),
            Action::Redo => drop(self.redo()),
            Action::SelectWith { uid, mode } => {
                self.sync_selection();
                if let Some(rows) = &self.rows {
                    self.selected.click(uid, mode, rows);
                    self.selection = self.selected.focus();
                }
            }
            Action::Structure(op) => self.run_structure(op),
            Action::ConfirmDelete(confirmed) => self.answer_delete(confirmed),
            // Contrat J7 (lot L0) : actions câblées par leur lot.
            other @ (Action::Nav { .. }
            | Action::FocusPane(_)
            | Action::SetView(_)
            | Action::SetGanttStyle(_)
            | Action::ShowDialog(_)
            | Action::ShowTaskInformation
            | Action::CloseDialog
            | Action::ToggleColumn(_)
            | Action::SetColumnWidth { .. }
            | Action::ScrollTableX(_)
            | Action::TimescaleStep(_)
            | Action::TimescalePage(_)
            | Action::TimescaleToStart
            | Action::TimescaleToEnd
            | Action::ZoomBy(_)
            | Action::Approve(_)
            | Action::Reject(_)
            | Action::OpenRecent(_)
            | Action::Recover(_)
            | Action::Ui(_)) => log::debug!("action J7 pas encore câblée : {other:?}"),
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
        self.poll_save(&ctx);
        self.guard_close(&ctx);
        self.apply_theme(&ctx);
        self.apply_language();
        let palette = Palette::for_theme(ctx.theme());
        let metrics = Metrics::default();
        self.sync_selection();
        self.drop_stale_delete();
        let mut actions = std::mem::take(&mut self.actions);
        // Raccourcis avant le dessin : la touche consommée n'atteint aucun
        // widget.
        crate::keymap::dispatch(&ctx, self.document.is_some(), &mut actions);
        let mut stats = FrameStats::default();
        // Nom du document du bandeau de confirmation, s'il y en a un.
        let confirm_name = self.confirm.as_ref().map(|_| {
            self.document
                .as_ref()
                .map(|document| document.origin.name.clone())
                .unwrap_or_default()
        });
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
                selected: &self.selected,
                actions: &mut actions,
                stats: &mut stats,
            };
            view::chrome::menu_bar(ui, &mut fc);
            view::chrome::toolbar(ui, &mut fc);
            view::chrome::status_bar(ui, &mut fc, &self.messages);
            view::chrome::confirm_banner(ui, &mut fc, confirm_name.as_deref());
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
            if let Some(pending) = &self.pending_delete {
                crate::structure::confirm_modal(&ctx, &self.texts, pending, fc.actions);
            }
        }
        self.last_frame = stats;
        // Les actions sont appliquées après le dessin : une image de plus
        // montre leur effet (langue, zoom, document…). Les lots d'édition
        // d'abord (partition stable) : une saisie validée par la perte du
        // focus arrive après le bouton cliqué dans la même image (barre
        // d'outils, menus et bandeau sont dessinés avant la table) ; elle
        // doit modifier le document **avant** qu'une ouverture, un « Nouveau »
        // ou une réponse au bandeau le remplace, pour que la confirmation la
        // voie (relecture finale J4, F2).
        let applied = !actions.is_empty();
        let (edits, others): (Vec<Action>, Vec<Action>) = actions
            .into_iter()
            .partition(|action| matches!(action, Action::Exec(_) | Action::ExecOn { .. }));
        for action in edits.into_iter().chain(others) {
            self.apply(action, &ctx);
        }
        self.sync_selection();
        // Ajustement du zoom après une ouverture, dès que la largeur du
        // Gantt est connue : là aussi, une image de plus au bon zoom.
        let fitted = self.fit_if_needed();
        if applied || fitted {
            ctx.request_repaint();
        }
        // Titre de la fenêtre (« * » si modifié), envoyé seulement au
        // changement.
        let title = self.window_title();
        if title != self.sent_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.sent_title = title;
        }
    }
}

/// Titre de la fenêtre pour `document` (voir [`App::window_title`]). Le web
/// le pose aussi sur l'onglet (`document.title`) depuis son observateur :
/// eframe web ignore `ViewportCommand::Title`.
#[must_use]
pub fn window_title_for(document: Option<&OpenDocument>) -> String {
    match document {
        None => PRODUCT_NAME.to_owned(),
        Some(document) => format!("{} \u{2014} {PRODUCT_NAME}", document_label(document)),
    }
}

/// Nom du document précédé de « * » s'il est modifié (titre de la fenêtre
/// et barre d'état).
#[must_use]
pub fn document_label(document: &OpenDocument) -> String {
    let star = if document.is_dirty() { "*" } else { "" };
    format!("{star}{}", document.origin.name)
}

/// Début d'un projet neuf : le lundi strictement après le jour de `now`,
/// à 08:00 (hypothèse : début de journée du calendrier Standard ; MS
/// Project propose la date du jour). Repli sur `now` hors du domaine des
/// dates.
#[must_use]
pub fn next_monday_morning(now: Moment) -> Moment {
    let day = now.day();
    let ahead = match day.weekday() {
        Weekday::Sunday => 1,
        Weekday::Monday => 7,
        Weekday::Tuesday => 6,
        Weekday::Wednesday => 5,
        Weekday::Thursday => 4,
        Weekday::Friday => 3,
        Weekday::Saturday => 2,
    };
    // 480 minutes : 08:00.
    day.checked_add_days(ahead)
        .ok()
        .zip(TimeOfDay::new(480).ok())
        .and_then(|(monday, morning)| Moment::at(monday, morning).ok())
        .unwrap_or(now)
}

/// Nombre de jours d'une plage (0 si vide ou inversée).
fn span_days(span: ViewSpan) -> i64 {
    i64::from(span.end.index())
        .saturating_sub(i64::from(span.start.index()))
        .max(0)
}

/// Union de deux plages affichées ; `new` seule si l'union dépasse
/// [`MAX_RANGE_DAYS`] (largeur du contenu représentable en `f32`).
fn union_span(old: ViewSpan, new: ViewSpan) -> ViewSpan {
    let start: Day = old.start.min(new.start);
    let end: Day = old.end.max(new.end);
    let union = ViewSpan { start, end };
    if span_days(union) <= i64::from(MAX_RANGE_DAYS) {
        union
    } else {
        new
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
        assert_eq!(document.opening, 1);
        assert_eq!(document.generation(), 0);
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
        assert_eq!(app.document.as_ref().unwrap().opening, 1);
        app.open_bytes(crate::DEMO_MSPDI, super::DocOrigin::default());
        assert_eq!(app.document.as_ref().unwrap().opening, 2);
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

    /// Projet neuf : lundi strictement suivant, 08:00.
    #[test]
    fn next_monday_morning_is_strictly_after() {
        let cases = [
            ("2027-01-15T08:00", "2027-01-18T08:00"), // vendredi
            ("2027-01-17T23:59", "2027-01-18T08:00"), // dimanche
            ("2027-01-18T07:00", "2027-01-25T08:00"), // lundi
            ("2027-01-16T12:00", "2027-01-18T08:00"), // samedi
        ];
        for (now, expected) in cases {
            let now: Moment = now.parse().unwrap();
            assert_eq!(
                super::next_monday_morning(now),
                expected.parse().unwrap(),
                "{now}"
            );
        }
    }

    /// Relecture finale J4, F2 : un lot visant un autre document (numéro
    /// d'ouverture différent) est abandonné ; le bon numéro l'applique.
    #[test]
    fn exec_on_other_opening_is_dropped() {
        let ctx = egui::Context::default();
        let mut app = test_app();
        app.open_demo();
        let opening = app.document.as_ref().unwrap().opening;
        let batch = planrs_automation::Batch::from_json_str(
            r#"{"format_version":1,"ops":[{"op":"task.update","uid":3,"name":"X"}]}"#,
        )
        .unwrap();
        app.dispatch(Action::ExecOn {
            opening: opening.saturating_add(1),
            batch: batch.clone(),
        });
        frame(&ctx, &mut app);
        assert_eq!(app.document.as_ref().unwrap().generation(), 0);
        assert!(app.messages().is_empty(), "{:?}", app.messages());
        app.dispatch(Action::ExecOn { opening, batch });
        frame(&ctx, &mut app);
        assert_eq!(app.document.as_ref().unwrap().generation(), 1);
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
