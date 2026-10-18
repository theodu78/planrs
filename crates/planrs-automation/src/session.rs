//! Session d'édition headless : un projet ouvert, son planning, son
//! historique (contrat J4, ADR-0012 D1-D3 ; propriétaire : L2).
//!
//! Aucune entrée/sortie, compilable en wasm : des octets en entrée, des
//! octets en sortie. « Maintenant » est injecté.

use planrs_core::calendar::BuiltinCalendarNames;
use planrs_core::{Moment, Project, ProjectInit, Schedule, ScheduleAnchor};
use planrs_mspdi::write::{WriteOptions, Written, choose_computed};
use planrs_mspdi::{ImportDiagnostic, ReadOptions, RecordedSchedule};
use serde::{Deserialize, Serialize};

pub use crate::command::AnchorSpec;
use crate::command::Batch;
use crate::error::{CommandError, HistoryError, OpenError};
use crate::history::{History, HistoryView};
use crate::outcome::ApplyOutcome;
use crate::text::{Language, TextContext};
use crate::uid::UidAlloc;

/// Session d'édition.
#[derive(Clone, Debug)]
pub struct Session {
    project: Project,
    schedule: Schedule,
    /// Valeurs relues du fichier, jetées au premier commit (R-162).
    recorded: Option<RecordedSchedule>,
    origin: DocOrigin,
    history: History,
    uids: UidAlloc,
    /// Position de l'historique au dernier enregistrement (R-164).
    saved_at: Option<u64>,
    now: Moment,
    lang: Language,
    limits: SessionLimits,
    generation: u64,
}

/// Origine du document : nom affiché, chemin s'il vient d'un fichier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocOrigin {
    /// Nom affiché.
    pub name: String,
    /// Chemin du fichier (texte), s'il y en a un.
    pub path: Option<String>,
}

/// Bornes d'une session (règle 10).
#[derive(Clone, Copy, Debug)]
pub struct SessionLimits {
    /// Opérations par lot (10 000).
    pub max_ops_per_batch: usize,
    /// Budget de l'historique en octets (64 Mio).
    pub history_bytes: usize,
    /// Entrées d'historique (1 000).
    pub history_entries: usize,
    /// Tâches du diff détaillé (1 000).
    pub diff_detail_max: usize,
}

impl Default for SessionLimits {
    fn default() -> Self {
        Self {
            max_ops_per_batch: 10_000,
            history_bytes: 64 * 1024 * 1024,
            history_entries: 1_000,
            diff_detail_max: 1_000,
        }
    }
}

/// Mode d'application d'un lot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyMode {
    /// Appliquer.
    Commit,
    /// Simuler sur une copie : la session reste intacte.
    DryRun,
}

/// Paramètres d'un projet neuf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewProject {
    /// Nom.
    pub name: String,
    /// Sens et date pilote.
    pub anchor: AnchorSpec,
    /// Langue (noms des calendriers fournis, textes) ; défaut : français.
    pub lang: Option<Language>,
}

impl Session {
    /// Ouvre un document MSPDI et calcule son planning.
    ///
    /// # Errors
    ///
    /// [`OpenError::Mspdi`] si le document est inexploitable ; le reste
    /// devient un diagnostic d'import.
    pub fn open_bytes(
        bytes: &[u8],
        origin: DocOrigin,
        now: Moment,
        lang: Language,
    ) -> Result<(Self, Vec<ImportDiagnostic>), OpenError> {
        let doc = planrs_mspdi::read(bytes, &ReadOptions::new(now))?;
        let session = Self::from_parts(doc.project, Some(doc.recorded), origin, now, lang);
        Ok((session, doc.diagnostics))
    }

    /// Projet neuf, vide (R-032).
    ///
    /// # Errors
    ///
    /// Aucune en J4 (validation du nom réservée).
    pub fn new_project(init: NewProject, now: Moment) -> Result<Self, CommandError> {
        let lang = init.lang.unwrap_or(Language::Fr);
        // Noms des calendriers fournis : français pour toutes les langues en
        // L0 ; noms anglais à trancher par L2 (hypothèse à consigner).
        let calendar_names = BuiltinCalendarNames::default();
        let anchor = match init.anchor {
            AnchorSpec::Start(m) => ScheduleAnchor::FromStart(m),
            AnchorSpec::Finish(m) => ScheduleAnchor::FromFinish(m),
        };
        let project = Project::new(ProjectInit {
            name: init.name.clone(),
            anchor,
            current_date: now,
            calendar_names,
        });
        let origin = DocOrigin {
            name: init.name,
            path: None,
        };
        Ok(Self::from_parts(project, None, origin, now, lang))
    }

    fn from_parts(
        project: Project,
        recorded: Option<RecordedSchedule>,
        origin: DocOrigin,
        now: Moment,
        lang: Language,
    ) -> Self {
        let schedule = planrs_core::schedule(&project);
        let uids = UidAlloc::from_project(&project);
        Self {
            project,
            schedule,
            recorded,
            origin,
            history: History::default(),
            uids,
            saved_at: Some(0),
            now,
            lang,
            limits: SessionLimits::default(),
            generation: 0,
        }
    }

    /// Applique un lot (pipeline de L2). Squelette L0 :
    /// `command.not_yet_available`, session intacte.
    ///
    /// # Errors
    ///
    /// Toute commande refusée ; l'état reste intact.
    pub fn apply(&mut self, batch: &Batch, mode: ApplyMode) -> Result<ApplyOutcome, CommandError> {
        let _ = (batch, mode, &self.uids, self.now, self.limits);
        Err(CommandError::not_yet(4))
    }

    /// Annule le dernier lot (R-164). Squelette L0 : historique vide.
    ///
    /// # Errors
    ///
    /// [`HistoryError::Empty`] si rien n'est annulable.
    pub fn undo(&mut self) -> Result<ApplyOutcome, HistoryError> {
        Err(HistoryError::Empty)
    }

    /// Rétablit le dernier lot annulé (R-164). Squelette L0.
    ///
    /// # Errors
    ///
    /// [`HistoryError::Empty`] si rien n'est rétablissable.
    pub fn redo(&mut self) -> Result<ApplyOutcome, HistoryError> {
        Err(HistoryError::Empty)
    }

    /// Vue de l'historique.
    #[must_use]
    pub fn history(&self) -> HistoryView {
        self.history.view()
    }

    /// Vrai si l'état diffère du dernier enregistrement (R-164).
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.saved_at != Some(self.history.position())
    }

    /// Marque l'état courant comme enregistré (après une écriture réussie).
    pub fn mark_saved(&mut self) {
        self.saved_at = Some(self.history.position());
    }

    /// Octets MSPDI de l'état courant (R-122, R-162). N'altère rien.
    #[must_use]
    pub fn save_bytes(&self, opts: &WriteOptions) -> Written {
        let (computed, mut diagnostics) =
            choose_computed(&self.project, &self.schedule, self.recorded.as_ref());
        let mut written = planrs_mspdi::write::write(&self.project, computed, opts);
        diagnostics.append(&mut written.diagnostics);
        written.diagnostics = diagnostics;
        written
    }

    /// Projet courant.
    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    /// Planning courant (calculé par le moteur).
    #[must_use]
    pub const fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Origine du document.
    #[must_use]
    pub const fn origin(&self) -> &DocOrigin {
        &self.origin
    }

    /// Change l'origine (« Enregistrer sous »).
    pub fn set_origin(&mut self, o: DocOrigin) {
        self.origin = o;
    }

    /// Génération : +1 à chaque commit, undo ou redo (clé de cache).
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Contexte du texte du projet dans la langue de la session.
    #[must_use]
    pub fn text_context(&self) -> TextContext {
        TextContext::for_project(&self.project, self.lang)
    }
}
