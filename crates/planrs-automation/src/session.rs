//! Session d'édition headless : un projet ouvert, son planning, son
//! historique (contrat J4, ADR-0012 D1-D3 ; propriétaire : L2).
//!
//! Aucune entrée/sortie, compilable en wasm : des octets en entrée, des
//! octets en sortie. « Maintenant » est injecté.

use std::collections::BTreeMap;

use planrs_core::calendar::BuiltinCalendarNames;
use planrs_core::{Diagnostic, Moment, Project, ProjectInit, Schedule, ScheduleAnchor, Subject, TaskUid};
use planrs_mspdi::write::{WriteOptions, Written, choose_computed};
use planrs_mspdi::{ImportDiagnostic, ReadOptions, RecordedSchedule};
use serde::{Deserialize, Serialize};

pub use crate::command::AnchorSpec;
use crate::command::Batch;
use crate::command::exec::{Ctx, Exec};
use crate::delta::{Dir, apply_delta, approx_bytes, diff_projects, json_len};
use crate::diag_delta::{blame, diagnostics_delta, state_diagnostics};
pub use crate::diff::DEFAULT_DIFF_TASKS;
use crate::diff::{ScheduleDiff, diff_schedules};
use crate::error::{CommandError, CommandErrorKind, HistoryError, OpenError, codes};
use crate::history::{History, HistoryView};
use crate::outcome::{ApplyOutcome, SideEffect};
use crate::text::{Language, TextContext};
use crate::uid::UidAlloc;

/// Session d'édition.
#[derive(Clone, Debug)]
pub struct Session {
    project: Project,
    schedule: Schedule,
    /// Diagnostics de l'état courant (`validate` ∪ planning), base du delta
    /// de R-160, mis à jour à chaque commit, undo ou redo.
    diagnostics: Vec<Diagnostic>,
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

/// Options d'application d'un lot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApplyOptions {
    /// Diff détaillé : jusqu'à `diff_detail_max` tâches au lieu de
    /// [`DEFAULT_DIFF_TASKS`] (R-163).
    pub detail: bool,
}

/// Opération de test : closure sur le contexte d'exécution (voir
/// [`Session::apply_edits`]).
#[doc(hidden)]
pub type Edit = Box<dyn Fn(&mut EditCtx<'_, '_>) -> Result<(), CommandError>>;

/// Contexte d'une opération de test : accès au clone du projet et aux
/// services du pipeline (UID, étiquettes, sujets touchés, effets).
#[doc(hidden)]
pub struct EditCtx<'a, 'b> {
    ctx: &'a mut Ctx<'b>,
}

impl EditCtx<'_, '_> {
    /// Clone du projet, modifié en place.
    pub fn project(&mut self) -> &mut Project {
        self.ctx.project
    }

    /// UID suivant (R-151).
    ///
    /// # Errors
    ///
    /// `limit.exceeded` au-delà de `u32::MAX`.
    pub fn alloc_uid(&mut self) -> Result<TaskUid, CommandError> {
        self.ctx.uids.alloc()
    }

    /// Marque `subject` comme touché par l'opération courante (R-160).
    pub fn touch(&mut self, subject: Subject) {
        self.ctx.touched.insert(subject, self.ctx.op_index);
    }

    /// Index de l'opération courante.
    #[must_use]
    pub fn op_index(&self) -> usize {
        self.ctx.op_index
    }

    /// Déclare une tâche créée (et son étiquette éventuelle).
    pub fn created(&mut self, uid: TaskUid, label: Option<String>) {
        self.ctx.created.push(uid);
        if let Some(l) = label {
            self.ctx.labels.insert(l, uid);
        }
    }

    /// Signale un effet de bord.
    pub fn effect(&mut self, e: SideEffect) {
        self.ctx.effects.push(e);
    }
}

impl Exec for Edit {
    fn exec(&self, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
        let mut e = EditCtx { ctx };
        self(&mut e)
    }
}

/// Recalcul d'un lot (R-173 amendée) : la durée du modèle de chaque feuille
/// placée par ses affectations prend la durée calculée (mesure de l'union
/// des intervalles travaillés, S14), celle qu'écrit l'écrivain MSPDI : la
/// session et le fichier relu portent la même durée, et les éditions
/// suivantes (R-189 à R-194) partent de cette valeur. Hors durée fixe et
/// unités nulles, la durée du modèle ne place pas la feuille (`d_a` = W ÷ U,
/// R-170) : le second recalcul, quand une durée a changé, redonne les mêmes
/// dates et garde le planning cohérent avec le modèle.
fn synced_schedule(work: &mut Project) -> Schedule {
    let schedule = planrs_core::schedule(work);
    if planrs_core::schedule::sync_assigned_durations(work, &schedule).is_empty() {
        schedule
    } else {
        planrs_core::schedule(work)
    }
}

/// Erreur `batch.would_create_errors` (R-160) : `op_index` = plus petit des
/// index imputés aux erreurs nouvelles (dernière opération qui a touché le
/// sujet, sinon la dernière du lot).
fn would_create_errors(
    errors: Vec<Diagnostic>,
    touched: &BTreeMap<Subject, usize>,
    n_ops: usize,
) -> CommandError {
    let last = n_ops.saturating_sub(1);
    let op_index = errors
        .iter()
        .map(|d| blame(d, touched).unwrap_or(last))
        .min()
        .unwrap_or(last);
    CommandError {
        code: codes::BATCH_WOULD_CREATE_ERRORS,
        op_index: Some(op_index),
        field: None,
        subject: errors.first().map(Diagnostic::subject),
        kind: CommandErrorKind::WouldCreateErrors(errors),
    }
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

    /// Session sur un projet déjà construit (import CSV, tests) : planning
    /// calculé, historique vide, document non modifié, sans valeurs relues.
    #[must_use]
    pub fn from_project(project: Project, origin: DocOrigin, now: Moment, lang: Language) -> Self {
        Self::from_parts(project, None, origin, now, lang)
    }

    /// Session ouverte : la durée du modèle des feuilles placées par leurs
    /// affectations est alignée sur l'union dès l'ouverture, comme après un
    /// lot (R-173 amendée) ; une durée relue différente (fichier d'un autre
    /// outil) n'attend pas le premier lot pour être corrigée, et ne
    /// s'inscrit pas dans l'entrée d'historique d'une commande sans rapport.
    /// Le document n'est pas marqué modifié.
    fn from_parts(
        mut project: Project,
        recorded: Option<RecordedSchedule>,
        origin: DocOrigin,
        now: Moment,
        lang: Language,
    ) -> Self {
        let schedule = synced_schedule(&mut project);
        let diagnostics = state_diagnostics(&project, &schedule);
        let uids = UidAlloc::from_project(&project);
        Self {
            project,
            schedule,
            diagnostics,
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

    /// Applique un lot (ADR-0012 D2, D3) : diff résumé à
    /// [`DEFAULT_DIFF_TASKS`] tâches. Voir [`Session::apply_with`].
    ///
    /// # Errors
    ///
    /// Toute commande refusée, ou une erreur nouvelle (R-160) ; l'état reste
    /// intact.
    pub fn apply(&mut self, batch: &Batch, mode: ApplyMode) -> Result<ApplyOutcome, CommandError> {
        self.apply_with(batch, mode, ApplyOptions::default())
    }

    /// Applique un lot : sur une copie du projet, commande par commande,
    /// puis un recalcul, le contrôle du delta de diagnostics (R-160) et le
    /// diff (R-163) ; `Commit` échange la copie et empile une entrée
    /// d'historique (R-164), `DryRun` la jette. Lot vide : rien ne change,
    /// aucune entrée d'historique.
    ///
    /// # Errors
    ///
    /// `batch.unsupported_version`, `limit.exceeded` (`ops`), l'erreur de la
    /// première commande refusée (avec son `op_index`), ou
    /// `batch.would_create_errors` ; l'état reste intact.
    pub fn apply_with(
        &mut self,
        batch: &Batch,
        mode: ApplyMode,
        opts: ApplyOptions,
    ) -> Result<ApplyOutcome, CommandError> {
        if batch.format_version != 1 {
            return Err(CommandError::new(
                codes::BATCH_UNSUPPORTED_VERSION,
                CommandErrorKind::UnsupportedVersion(batch.format_version),
            ));
        }
        self.run(&batch.ops, batch, mode, opts)
    }

    /// Applique des opérations de test (closures sur le contexte
    /// d'exécution) par le même pipeline que [`Session::apply`]. Réservé
    /// aux tests du pipeline (P30, R-151 à R-164) tant que les commandes
    /// réelles ne sont pas toutes écrites ; l'entrée d'historique porte
    /// `label` et aucune opération sérialisée.
    ///
    /// # Errors
    ///
    /// Comme [`Session::apply_with`].
    #[doc(hidden)]
    pub fn apply_edits(
        &mut self,
        label: Option<String>,
        edits: &[Edit],
        mode: ApplyMode,
        opts: ApplyOptions,
    ) -> Result<ApplyOutcome, CommandError> {
        let record = Batch {
            format_version: 1,
            lang: None,
            ops: Vec::new(),
            label,
        };
        self.run(edits, &record, mode, opts)
    }

    /// Pipeline commun (ordre exact du cahier J4, L2).
    fn run<E: Exec>(
        &mut self,
        ops: &[E],
        record: &Batch,
        mode: ApplyMode,
        options: ApplyOptions,
    ) -> Result<ApplyOutcome, CommandError> {
        if ops.len() > self.limits.max_ops_per_batch {
            return Err(CommandError::new(
                codes::LIMIT_EXCEEDED,
                CommandErrorKind::Limit {
                    what: "ops",
                    max: u64::try_from(self.limits.max_ops_per_batch).unwrap_or(u64::MAX),
                },
            ));
        }
        if ops.is_empty() {
            return Ok(self.unchanged_outcome());
        }
        let text = TextContext::for_project(&self.project, record.lang.unwrap_or(self.lang));
        let mut work = self.project.clone();
        let mut uids = self.uids.clone();
        let mut labels = BTreeMap::new();
        let mut effects = Vec::new();
        let mut touched = BTreeMap::new();
        let mut created = Vec::new();
        let mut resource_labels = BTreeMap::new();
        let mut created_resources = Vec::new();
        for (i, op) in ops.iter().enumerate() {
            let mut ctx = Ctx {
                project: &mut work,
                uids: &mut uids,
                labels: &mut labels,
                effects: &mut effects,
                touched: &mut touched,
                op_index: i,
                text: &text,
                created: &mut created,
                resource_labels: &mut resource_labels,
                created_resources: &mut created_resources,
            };
            op.exec(&mut ctx).map_err(|e| e.with_op_index(i))?;
        }
        let schedule = synced_schedule(&mut work);
        let diagnostics = state_diagnostics(&work, &schedule);
        let delta = diagnostics_delta(&self.diagnostics, &diagnostics);
        if !delta.errors.is_empty() {
            return Err(would_create_errors(delta.errors, &touched, ops.len()));
        }
        let n = if options.detail {
            self.limits.diff_detail_max
        } else {
            DEFAULT_DIFF_TASKS
        };
        let diff = diff_schedules(&self.project, &self.schedule, &work, &schedule, n);
        if mode == ApplyMode::DryRun {
            return Ok(ApplyOutcome {
                committed: false,
                labels,
                created,
                diff,
                warnings: delta.warnings,
                side_effects: effects,
                history_len: self.history.len(),
                resource_labels,
                created_resources,
            });
        }
        let entity_delta = diff_projects(&self.project, &work);
        let bytes = approx_bytes(&entity_delta).saturating_add(json_len(record));
        uids.observe(&work);
        self.project = work;
        self.schedule = schedule;
        self.diagnostics = diagnostics;
        self.uids = uids;
        if self.recorded.take().is_some() {
            // R-162 : valeurs relues jetées au premier commit.
            effects.push(SideEffect::RecordedDiscarded);
        }
        self.history.push(
            record.label.clone(),
            record.clone(),
            entity_delta,
            bytes,
            &self.limits,
        );
        self.forget_unreachable_save();
        self.generation = self.generation.saturating_add(1);
        Ok(ApplyOutcome {
            committed: true,
            labels,
            created,
            diff,
            warnings: delta.warnings,
            side_effects: effects,
            history_len: self.history.len(),
            resource_labels,
            created_resources,
        })
    }

    /// Résultat d'un lot vide : rien ne change.
    fn unchanged_outcome(&self) -> ApplyOutcome {
        ApplyOutcome {
            committed: false,
            labels: BTreeMap::new(),
            created: Vec::new(),
            diff: ScheduleDiff::unchanged(&self.schedule),
            warnings: Vec::new(),
            side_effects: Vec::new(),
            history_len: self.history.len(),
            resource_labels: BTreeMap::new(),
            created_resources: Vec::new(),
        }
    }

    /// `saved_at = None` si l'état enregistré n'est plus atteignable
    /// (abandonné par le budget, ou branche de rétablissement vidée).
    fn forget_unreachable_save(&mut self) {
        if self.saved_at.is_some_and(|id| !self.history.reachable(id)) {
            self.saved_at = None;
        }
    }

    /// Remplace l'état par `project` (déjà modifié par un delta) : recalcul,
    /// diff, avertissements nouveaux, génération.
    fn replace_state(&mut self, project: Project) -> ApplyOutcome {
        let schedule = planrs_core::schedule(&project);
        let diagnostics = state_diagnostics(&project, &schedule);
        let warnings = diagnostics_delta(&self.diagnostics, &diagnostics).warnings;
        let diff = diff_schedules(
            &self.project,
            &self.schedule,
            &project,
            &schedule,
            DEFAULT_DIFF_TASKS,
        );
        self.uids.observe(&project);
        self.project = project;
        self.schedule = schedule;
        self.diagnostics = diagnostics;
        self.generation = self.generation.saturating_add(1);
        ApplyOutcome {
            committed: true,
            labels: BTreeMap::new(),
            created: Vec::new(),
            diff,
            warnings,
            side_effects: Vec::new(),
            history_len: 0,
            resource_labels: BTreeMap::new(),
            created_resources: Vec::new(),
        }
    }

    /// Annule le dernier lot (R-164) : pré-images, recalcul. Le compteur
    /// d'UID ne recule pas (R-151) ; les valeurs relues ne reviennent pas
    /// (R-162).
    ///
    /// # Errors
    ///
    /// [`HistoryError::Empty`] si rien n'est annulable ;
    /// [`HistoryError::Inconsistent`] si le delta est inapplicable (session
    /// intacte).
    pub fn undo(&mut self) -> Result<ApplyOutcome, HistoryError> {
        let entry = self.history.pop_undo().ok_or(HistoryError::Empty)?;
        let mut project = self.project.clone();
        if let Err(e) = apply_delta(&mut project, &entry.delta, Dir::Undo) {
            self.history.push_undo_back(entry);
            return Err(HistoryError::Inconsistent(e));
        }
        let mut outcome = self.replace_state(project);
        self.history.push_redo(entry);
        outcome.history_len = self.history.len();
        Ok(outcome)
    }

    /// Rétablit le dernier lot annulé (R-164) : post-images (mêmes UID).
    ///
    /// # Errors
    ///
    /// [`HistoryError::Empty`] si rien n'est rétablissable ;
    /// [`HistoryError::Inconsistent`] si le delta est inapplicable (session
    /// intacte).
    pub fn redo(&mut self) -> Result<ApplyOutcome, HistoryError> {
        let entry = self.history.pop_redo().ok_or(HistoryError::Empty)?;
        let mut project = self.project.clone();
        if let Err(e) = apply_delta(&mut project, &entry.delta, Dir::Redo) {
            self.history.push_redo(entry);
            return Err(HistoryError::Inconsistent(e));
        }
        let mut outcome = self.replace_state(project);
        self.history.push_undo_back(entry);
        outcome.history_len = self.history.len();
        Ok(outcome)
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

    /// « Maintenant » injecté à l'ouverture.
    #[must_use]
    pub const fn now(&self) -> Moment {
        self.now
    }

    /// Bornes de la session.
    #[must_use]
    pub const fn limits(&self) -> SessionLimits {
        self.limits
    }

    /// Change les bornes ; le budget d'historique s'applique au commit
    /// suivant.
    pub fn set_limits(&mut self, limits: SessionLimits) {
        self.limits = limits;
    }

    /// Diagnostics de l'état courant (`validate` ∪ planning), triés.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Contexte du texte du projet dans la langue de la session.
    #[must_use]
    pub fn text_context(&self) -> TextContext {
        TextContext::for_project(&self.project, self.lang)
    }
}
