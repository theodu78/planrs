//! Vues JSON partagées par la CLI (`--json`), le MCP et le canal
//! (ADR-0012 D1, lot L6).
//!
//! Contrats de surface :
//! - **C-MCP-01** : toute date et toute marge viennent du `Schedule` du
//!   moteur (règle 3) ; ce module ne fait que les recopier en texte ISO
//!   local. Seule mesure faite ici, `DiffView::finish_delta` : le temps
//!   ouvré entre les deux fins du projet, compté par le calendrier compilé
//!   du cœur (R-020), jamais estimé ;
//! - **C-MCP-02** : forme JSON figée par les fichiers de référence
//!   `tests/golden/views/*.json` (vues résumées) ;
//! - **C-MCP-03** : sortie bornée : [`MAX_LIST_TASKS`] tâches par page,
//!   [`MAX_DIFF_TASKS`] tâches de diff, [`MAX_VIEW_DIAGNOSTICS`]
//!   diagnostics, messages d'au plus [`MAX_MESSAGE_CHARS`] caractères.
//!
//! Durées : la forme unique `{"minutes","unit","text"}` de [`DurationJson`]
//! (une durée de sortie est une entrée valide) ; une marge négative a des
//! minutes négatives et un texte précédé de « - » (sortie seulement).

use std::collections::BTreeMap;

use planrs_core::calendar::CompiledCalendar;
use planrs_core::{
    Diagnostic, Duration, DurationUnit, LagValue, Minutes, Moment, Project, Schedule, Severity, Subject,
    Task, TaskRole, TaskSchedule, TaskUid, TimeBasis,
};
use serde::Serialize;

use crate::command::params::TaskListParams;
use crate::diff::{ScheduleDiff, TaskChange};
use crate::error::{CommandError, CommandErrorKind, MAX_MESSAGE_CHARS, codes};
use crate::json::{
    ConstraintKindJson, DurationJson, GranularityJson, LagJson, LinkTypeJson, ResourceKindJson, UnitsJson,
};
use crate::outcome::{ApplyOutcome, SideEffect};
use crate::session::Session;
use crate::text::TextContext;

/// Tâches d'une page de `task.list` au plus (C-MCP-03).
pub const MAX_LIST_TASKS: usize = 500;
/// Tâches d'une page de `task.list` par défaut.
pub const DEFAULT_LIST_TASKS: usize = 50;
/// Tâches d'un diff au plus (C-MCP-03).
pub const MAX_DIFF_TASKS: usize = 1_000;
/// Diagnostics d'une vue au plus (C-MCP-03) ; le total reste compté.
pub const MAX_VIEW_DIAGNOSTICS: usize = 100;

/// Tâche, telle que le moteur l'a planifiée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TaskView {
    /// UID (stable).
    pub uid: u32,
    /// ID (position, change avec l'ordre).
    pub id: u32,
    /// Nom.
    pub name: String,
    /// Niveau hiérarchique (1 : premier niveau).
    pub level: u16,
    /// Récapitulative.
    pub summary: bool,
    /// Jalon.
    pub milestone: bool,
    /// Durée : saisie pour une feuille, calculée pour une récapitulative.
    pub duration: DurationJson,
    /// Début planifié (ISO local) ; absent si la tâche n'est pas planifiée.
    pub start: Option<String>,
    /// Fin planifiée (ISO local).
    pub finish: Option<String>,
    /// Marge totale (unité par défaut du projet).
    pub total_slack: Option<DurationJson>,
    /// Marge libre.
    pub free_slack: Option<DurationJson>,
    /// Critique.
    pub critical: bool,
    /// Contrainte, sauf « dès que possible ».
    pub constraint: Option<ConstraintOut>,
    /// Échéance (ISO local).
    pub deadline: Option<String>,
    /// Liens vers les prédécesseurs.
    pub predecessors: Vec<LinkOut>,
    /// Codes des diagnostics de la tâche.
    pub diagnostics: Vec<String>,
}

/// Contrainte de date d'une tâche.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConstraintOut {
    /// Type.
    pub kind: ConstraintKindJson,
    /// Date (absente pour « dès/le plus tard possible »).
    pub date: Option<String>,
}

/// Lien vers un prédécesseur.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LinkOut {
    /// UID du prédécesseur.
    pub pred: u32,
    /// Type.
    pub kind: LinkTypeJson,
    /// Décalage, s'il n'est pas nul.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lag: Option<LagJson>,
}

/// Résumé du projet (`project.summary`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SummaryView {
    /// Nom du projet.
    pub name: String,
    /// Début planifié (ISO local).
    pub start: String,
    /// Fin planifiée (ISO local).
    pub finish: String,
    /// Durée du projet.
    pub duration: DurationJson,
    /// Tâches (lignes vides exclues).
    pub tasks: usize,
    /// Récapitulatives.
    pub summaries: usize,
    /// Jalons.
    pub milestones: usize,
    /// Tâches critiques.
    pub critical: usize,
    /// Liens.
    pub links: usize,
    /// Diagnostics de gravité erreur.
    pub errors: usize,
    /// Avertissements.
    pub warnings: usize,
    /// Document modifié depuis l'enregistrement.
    pub dirty: bool,
    /// Entrées d'historique.
    pub history_len: usize,
    /// Chemin du fichier.
    pub path: Option<String>,
}

/// Page de `task.list`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ListView {
    /// Tâches de la page, dans l'ordre des ID.
    pub tasks: Vec<TaskView>,
    /// UID à passer en `cursor` pour la page suivante ; absent à la fin.
    pub next_cursor: Option<u32>,
    /// Tâches retenues par le filtre, toutes pages comprises.
    pub total: usize,
}

/// Diagnostic, lisible tel quel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiagView {
    /// Code stable.
    pub code: String,
    /// Gravité.
    pub severity: Severity,
    /// Élément concerné.
    pub subject: Option<Subject>,
    /// Message (borné).
    pub message: String,
}

/// Résultat d'un lot, d'une annulation ou d'un rétablissement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OutcomeView {
    /// Vrai si la session a changé (faux en `dry_run`).
    pub committed: bool,
    /// Étiquettes `$x` → UID.
    pub labels: BTreeMap<String, u32>,
    /// UID des tâches créées, dans l'ordre (aussi en `dry_run`).
    pub created_uids: Vec<u32>,
    /// Tâches créées, telles que planifiées (vide en `dry_run` : elles
    /// n'existent que dans la copie jetée ; leurs dates sont dans `diff`).
    pub created: Vec<TaskView>,
    /// Diff de planning.
    pub diff: DiffView,
    /// Avertissements nouveaux.
    pub warnings: Vec<DiagView>,
    /// Effets de bord.
    pub side_effects: Vec<SideEffect>,
    /// Entrées d'historique après l'opération.
    pub history_len: usize,
}

/// Diff de planning (R-163), dates en ISO local.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiffView {
    /// Début du projet avant.
    pub start_before: String,
    /// Début du projet après.
    pub start_after: String,
    /// Fin du projet avant.
    pub finish_before: String,
    /// Fin du projet après.
    pub finish_after: String,
    /// Décalage de la fin du projet : temps ouvré du calendrier du projet,
    /// en jours (minutes exactes dans `minutes`).
    pub finish_delta: DurationJson,
    /// Tâches devenues critiques.
    pub critical_entered: Vec<u32>,
    /// Tâches qui ne sont plus critiques.
    pub critical_left: Vec<u32>,
    /// Tâches changées, en tout.
    pub changed_total: usize,
    /// Tâches changées, les plus décalées d'abord.
    pub changed: Vec<TaskChangeView>,
    /// Vrai si `changed` est coupé.
    pub truncated: bool,
    /// Ressources surutilisées apparues ou résolues (J5, ADR-0013 D9,
    /// signalées dès un `dry_run`) ; absent du JSON quand vide.
    #[serde(skip_serializing_if = "OverallocationView::is_empty")]
    pub overallocated: OverallocationView,
}

/// Surutilisations changées par un lot (J5).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct OverallocationView {
    /// Ressources (UID) devenues surutilisées.
    pub new: Vec<u32>,
    /// Ressources (UID) qui ne le sont plus.
    pub resolved: Vec<u32>,
}

impl OverallocationView {
    /// Vrai si rien n'a changé.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.new.is_empty() && self.resolved.is_empty()
    }
}

/// Ressource, telle que planifiée (J5, `resource.list` ; rempli par le lot
/// L8).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResourceView {
    /// UID.
    pub uid: u32,
    /// Nom.
    pub name: String,
    /// Type.
    pub kind: ResourceKindJson,
    /// Étiquette d'unité (ressource matérielle).
    pub material_label: Option<String>,
    /// Capacité maximale.
    pub max_units: UnitsJson,
    /// Calendrier propre.
    pub calendar: Option<u32>,
    /// Initiales.
    pub initials: String,
    /// Groupe.
    pub group: String,
    /// Travail total calculé par le moteur (R-184).
    pub work: DurationJson,
    /// Surutilisée (R-186).
    pub overallocated: bool,
    /// Inactive.
    pub inactive: bool,
}

/// Page de `resource.list` (J5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResourceListView {
    /// Ressources de la page, dans l'ordre des UID.
    pub resources: Vec<ResourceView>,
    /// UID à passer en `cursor` pour la page suivante ; absent à la fin.
    pub next_cursor: Option<u32>,
    /// Ressources retenues par le filtre, toutes pages comprises.
    pub total: usize,
}

/// Utilisation par période (J5, `analysis.resource_usage`, R-188 ; rempli
/// par le lot L8).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UsageView {
    /// Granularité.
    pub by: GranularityJson,
    /// Début de chaque période (ISO local).
    pub periods: Vec<String>,
    /// Une ligne par ressource.
    pub rows: Vec<UsageRowView>,
    /// Vrai si le rapport est coupé.
    pub truncated: bool,
}

/// Ligne d'une ressource dans [`UsageView`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UsageRowView {
    /// UID de la ressource.
    pub uid: u32,
    /// Nom.
    pub name: String,
    /// Travail de chaque période.
    pub work: Vec<DurationJson>,
    /// Surutilisation de chaque période.
    pub overallocated: Vec<bool>,
}

/// Changement d'une tâche.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TaskChangeView {
    /// UID.
    pub uid: u32,
    /// ID avant (absent : tâche ajoutée).
    pub id_before: Option<u32>,
    /// ID après (absent : tâche supprimée).
    pub id_after: Option<u32>,
    /// Début avant.
    pub start_before: Option<String>,
    /// Début après.
    pub start_after: Option<String>,
    /// Fin avant.
    pub finish_before: Option<String>,
    /// Fin après.
    pub finish_after: Option<String>,
    /// Marge totale avant.
    pub total_slack_before: Option<DurationJson>,
    /// Marge totale après.
    pub total_slack_after: Option<DurationJson>,
    /// Critique avant.
    pub critical_before: bool,
    /// Critique après.
    pub critical_after: bool,
    /// Minutes écoulées entre la fin avant et la fin après.
    pub finish_delta_min: i64,
}

/// Chemin critique (`analysis.critical_path`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CriticalPathView {
    /// Fin du projet.
    pub project_finish: String,
    /// Tâches critiques (feuilles), dans l'ordre du calcul (R-035).
    pub tasks: Vec<TaskView>,
    /// Tâches critiques en tout.
    pub total: usize,
    /// Vrai si `tasks` est coupé à [`MAX_LIST_TASKS`].
    pub truncated: bool,
}

/// Texte ISO local d'un instant (R-075).
fn iso(m: Moment) -> String {
    m.to_string()
}

/// Durée de sortie de `minutes` (signées) dans l'unité `unit`, temps ouvré.
/// Une valeur négative (marge) s'écrit « -2j » ; hors des bornes du modèle,
/// seules les minutes et l'unité sont données.
#[must_use]
pub fn signed_minutes(minutes: Minutes, unit: DurationUnit, ctx: &TextContext) -> DurationJson {
    let (negative, abs) = if minutes.is_negative() {
        (true, minutes.checked_neg())
    } else {
        (false, Some(minutes))
    };
    let parsed = abs.and_then(|a| Duration::new(a, unit, TimeBasis::Working, false).ok());
    let Some(d) = parsed else {
        return DurationJson {
            minutes: Some(minutes.get()),
            unit: Some(unit.into()),
            ..DurationJson::default()
        };
    };
    let mut out = DurationJson::from_duration(&d, ctx);
    out.minutes = Some(minutes.get());
    out.elapsed = None;
    out.estimated = None;
    if negative {
        out.text = out.text.map(|t| format!("-{t}"));
    }
    out
}

/// Coupe `s` à [`MAX_MESSAGE_CHARS`] caractères (C-MCP-03).
fn bounded(s: String) -> String {
    if s.chars().count() <= MAX_MESSAGE_CHARS {
        return s;
    }
    let mut out: String = s.chars().take(MAX_MESSAGE_CHARS.saturating_sub(1)).collect();
    out.push('…');
    out
}

impl DiagView {
    /// Vue d'un diagnostic du cœur.
    #[must_use]
    pub fn from_diagnostic(d: &Diagnostic) -> Self {
        Self {
            code: d.code().to_owned(),
            severity: d.severity(),
            subject: Some(d.subject()),
            message: bounded(d.to_string()),
        }
    }

    /// Vue d'un diagnostic d'import MSPDI.
    #[must_use]
    pub fn from_import(d: &planrs_mspdi::ImportDiagnostic) -> Self {
        Self {
            code: d.code.code(),
            severity: d.severity,
            subject: d.subject,
            message: bounded(d.to_string()),
        }
    }

    /// Vues d'une liste de diagnostics, au plus [`MAX_VIEW_DIAGNOSTICS`].
    pub fn list<'a>(ds: impl IntoIterator<Item = &'a Diagnostic>) -> Vec<Self> {
        ds.into_iter()
            .take(MAX_VIEW_DIAGNOSTICS)
            .map(Self::from_diagnostic)
            .collect()
    }
}

/// Contexte de construction des vues sur un état (projet et planning).
pub struct ViewCtx<'a> {
    project: &'a Project,
    schedule: &'a Schedule,
    text: TextContext,
    /// Codes des diagnostics par tâche (dédoublonnés, triés).
    task_diags: BTreeMap<TaskUid, Vec<String>>,
}

impl<'a> ViewCtx<'a> {
    /// Contexte de l'état courant d'une session.
    #[must_use]
    pub fn new(session: &'a Session) -> Self {
        let mut task_diags: BTreeMap<TaskUid, Vec<String>> = BTreeMap::new();
        for d in session.diagnostics() {
            if let Subject::Task(uid) = d.subject() {
                let codes = task_diags.entry(uid).or_default();
                let code = d.code();
                if !codes.iter().any(|c| c == code) {
                    codes.push(code.to_owned());
                }
            }
        }
        for codes in task_diags.values_mut() {
            codes.sort_unstable();
        }
        Self {
            project: session.project(),
            schedule: session.schedule(),
            text: session.text_context(),
            task_diags,
        }
    }

    /// Contexte du texte (langue de la session).
    #[must_use]
    pub const fn text(&self) -> &TextContext {
        &self.text
    }

    /// Vrai si la tâche à la position `pos` a une sous-tâche : la tâche non
    /// vide suivante est plus profonde, lignes vides ignorées (R-052).
    #[must_use]
    pub fn is_summary_at(&self, pos: usize) -> bool {
        crate::command::outline::is_summary_at(&self.project.tasks, pos)
    }

    /// Vue de la tâche `uid`, si elle existe (lignes vides comprises).
    #[must_use]
    pub fn task(&self, uid: TaskUid) -> Option<TaskView> {
        let pos = self.project.tasks.position(uid)?;
        let task = self.project.tasks.at(pos)?;
        Some(self.task_at(pos, task))
    }

    /// Vue de `task`, à la position `pos`.
    #[must_use]
    pub fn task_at(&self, pos: usize, task: &Task) -> TaskView {
        let uid = task.uid();
        let scheduled = self.schedule.scheduled(uid);
        let id = pos
            .checked_add(1)
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(u32::MAX);
        let slack_unit = self.project.options.default_duration_unit;
        TaskView {
            uid: uid.get(),
            id,
            name: task.name.clone(),
            level: task.outline_level.get(),
            summary: self.is_summary_at(pos),
            milestone: task.milestone,
            duration: self.duration(task, scheduled),
            start: scheduled.map(|s| iso(s.start)),
            finish: scheduled.map(|s| iso(s.finish)),
            total_slack: scheduled.map(|s| signed_minutes(s.total_slack, slack_unit, &self.text)),
            free_slack: scheduled.map(|s| signed_minutes(s.free_slack, slack_unit, &self.text)),
            critical: scheduled.is_some_and(|s| s.critical),
            constraint: constraint_out(task),
            deadline: task.deadline.map(iso),
            predecessors: task
                .predecessors
                .iter()
                .map(|l| LinkOut {
                    pred: l.predecessor.get(),
                    kind: l.kind.into(),
                    lag: lag_out(&l.lag, &self.text),
                })
                .collect(),
            diagnostics: self.task_diags.get(&uid).cloned().unwrap_or_default(),
        }
    }

    /// Durée d'une tâche : saisie (feuille, tâche non planifiée), ou temps
    /// ouvré calculé par le moteur (récapitulative, R-054), dans l'unité de
    /// la tâche.
    fn duration(&self, task: &Task, scheduled: Option<&TaskSchedule>) -> DurationJson {
        match scheduled {
            Some(s) if s.role == TaskRole::Summary => {
                let mut d = signed_minutes(s.duration, task.duration.unit(), &self.text);
                d.elapsed = Some(false);
                d.estimated = Some(false);
                d
            }
            _ => DurationJson::from_duration(&task.duration, &self.text),
        }
    }

    /// Tâches visibles (lignes vides exclues), avec leur position.
    pub fn rows(&self) -> impl Iterator<Item = (usize, &'a Task)> + 'a {
        self.project.tasks.iter().enumerate().filter(|(_, t)| !t.is_null)
    }

    /// Vue de l'ID et du UID d'un changement.
    fn change(&self, c: &TaskChange) -> TaskChangeView {
        let unit = self.project.options.default_duration_unit;
        let slack = |m: Option<Minutes>| m.map(|m| signed_minutes(m, unit, &self.text));
        TaskChangeView {
            uid: c.uid.get(),
            id_before: c.id_before.map(planrs_core::TaskId::get),
            id_after: c.id_after.map(planrs_core::TaskId::get),
            start_before: c.start.0.map(iso),
            start_after: c.start.1.map(iso),
            finish_before: c.finish.0.map(iso),
            finish_after: c.finish.1.map(iso),
            total_slack_before: slack(c.total_slack.0),
            total_slack_after: slack(c.total_slack.1),
            critical_before: c.critical.0,
            critical_after: c.critical.1,
            finish_delta_min: c.finish_delta_min,
        }
    }

    /// Vue d'un diff ; `finish_delta` est mesuré sur le calendrier du
    /// projet de cet état (R-020), à défaut en minutes écoulées.
    #[must_use]
    pub fn diff(&self, d: &ScheduleDiff) -> DiffView {
        let (before, after) = d.project_finish;
        let working = CompiledCalendar::compile(&self.project.calendars, self.project.calendar)
            .ok()
            .and_then(|c| c.working_between(before, after).ok());
        let finish_delta = working.map_or_else(
            || {
                let mut out = signed_minutes(before.minutes_until(after), DurationUnit::Day, &self.text);
                out.elapsed = Some(true);
                out.text = None;
                out
            },
            |m| signed_minutes(m, DurationUnit::Day, &self.text),
        );
        let changed: Vec<TaskChangeView> = d
            .changed
            .iter()
            .take(MAX_DIFF_TASKS)
            .map(|c| self.change(c))
            .collect();
        DiffView {
            start_before: iso(d.project_start.0),
            start_after: iso(d.project_start.1),
            finish_before: iso(before),
            finish_after: iso(after),
            finish_delta,
            critical_entered: d.critical_entered.iter().map(|u| u.get()).collect(),
            critical_left: d.critical_left.iter().map(|u| u.get()).collect(),
            changed_total: d.changed_total,
            truncated: d.truncated || changed.len() < d.changed.len(),
            changed,
            overallocated: OverallocationView::default(),
        }
    }

    /// Vue d'un résultat appliqué à cet état (après un commit, un undo ou
    /// un redo, ou l'état intact d'un `dry_run`).
    #[must_use]
    pub fn outcome(&self, o: &ApplyOutcome) -> OutcomeView {
        let created = if o.committed {
            o.created.iter().filter_map(|u| self.task(*u)).collect()
        } else {
            Vec::new()
        };
        OutcomeView {
            committed: o.committed,
            labels: o.labels.iter().map(|(k, v)| (k.clone(), v.get())).collect(),
            created_uids: o.created.iter().map(|u| u.get()).collect(),
            created,
            diff: self.diff(&o.diff),
            warnings: DiagView::list(&o.warnings),
            side_effects: o.side_effects.clone(),
            history_len: o.history_len,
        }
    }

    /// Chemin critique : feuilles critiques dans l'ordre topologique du
    /// calcul (R-035), au plus [`MAX_LIST_TASKS`].
    #[must_use]
    pub fn critical_path(&self) -> CriticalPathView {
        let mut tasks = Vec::new();
        let mut total = 0usize;
        for uid in &self.schedule.order {
            let Some(s) = self.schedule.scheduled(*uid) else {
                continue;
            };
            if !s.critical || s.role == TaskRole::Summary {
                continue;
            }
            total = total.saturating_add(1);
            if tasks.len() < MAX_LIST_TASKS
                && let Some(v) = self.task(*uid)
            {
                tasks.push(v);
            }
        }
        CriticalPathView {
            project_finish: iso(self.schedule.project_finish),
            truncated: tasks.len() < total,
            tasks,
            total,
        }
    }
}

/// Contrainte d'une tâche, sauf « dès que possible ».
fn constraint_out(task: &Task) -> Option<ConstraintOut> {
    let kind = task.constraint.kind();
    if kind == planrs_core::ConstraintKind::AsSoonAsPossible {
        return None;
    }
    Some(ConstraintOut {
        kind: kind.into(),
        date: task.constraint.date().map(iso),
    })
}

/// Décalage d'un lien, sauf s'il est nul.
fn lag_out(l: &planrs_core::Lag, ctx: &TextContext) -> Option<LagJson> {
    let zero = match l.value {
        LagValue::Time { minutes, .. } => minutes.get() == 0,
        LagValue::Percent { percent } => percent.get() == 0,
    };
    (!zero).then(|| LagJson::from_lag(l, ctx))
}

/// Résumé de l'état courant d'une session.
#[must_use]
pub fn summary(session: &Session) -> SummaryView {
    let project = session.project();
    let schedule = session.schedule();
    let ctx = ViewCtx::new(session);
    let (mut tasks, mut summaries, mut milestones, mut critical, mut links) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    for (pos, t) in ctx.rows() {
        tasks = tasks.saturating_add(1);
        if ctx.is_summary_at(pos) {
            summaries = summaries.saturating_add(1);
        }
        if t.milestone {
            milestones = milestones.saturating_add(1);
        }
        if schedule.scheduled(t.uid()).is_some_and(|s| s.critical) {
            critical = critical.saturating_add(1);
        }
        links = links.saturating_add(t.predecessors.len());
    }
    let count = |sev: Severity| {
        session
            .diagnostics()
            .iter()
            .filter(|d| d.severity() == sev)
            .count()
    };
    let duration_min = schedule.project_summary.as_ref().map_or_else(
        || {
            CompiledCalendar::compile(&project.calendars, project.calendar)
                .ok()
                .and_then(|c| {
                    c.working_between(schedule.project_start, schedule.project_finish)
                        .ok()
                })
                .unwrap_or(Minutes::ZERO)
        },
        |s| s.duration,
    );
    let name = if project.info.name.is_empty() {
        session.origin().name.clone()
    } else {
        project.info.name.clone()
    };
    SummaryView {
        name,
        start: iso(schedule.project_start),
        finish: iso(schedule.project_finish),
        duration: signed_minutes(duration_min, project.options.default_duration_unit, ctx.text()),
        tasks,
        summaries,
        milestones,
        critical,
        links,
        errors: count(Severity::Error),
        warnings: count(Severity::Warning),
        dirty: session.is_dirty(),
        history_len: session.history().entries.len(),
        path: session.origin().path.clone(),
    }
}

/// Erreur `param.invalid` sur le champ `field`.
fn invalid(field: &str, why: &str) -> CommandError {
    CommandError::new(
        codes::PARAM_INVALID,
        CommandErrorKind::InvalidParam(why.to_owned()),
    )
    .with_field(field)
}

/// Page de `task.list` : tâches dans l'ordre des ID (lignes vides
/// exclues), filtrées puis coupées après le curseur.
///
/// Filtres : `critical` (critique selon le moteur), `late` (J4 : fin
/// planifiée après l'échéance), `from`/`to` (dates planifiées qui touchent
/// la plage de jours, bornes comprises : début au plus tard le jour `to`,
/// fin au plus tôt le jour `from`), `outline_max`, `summary`. Une tâche non
/// planifiée n'est retenue par aucun filtre de dates ni par `critical` ou
/// `late`.
///
/// # Errors
///
/// `limit.exceeded` au-delà de [`MAX_LIST_TASKS`] ; `param.invalid` pour
/// `limit` 0, `outline_max` 0, `from` après `to` ; `task.not_found`
/// (champ `cursor`) si le curseur ne désigne aucune tâche ;
/// `command.not_yet_available` (J5) pour `resource`.
pub fn list(session: &Session, p: &TaskListParams) -> Result<ListView, CommandError> {
    if p.resource.is_some() {
        return Err(CommandError::not_yet(5).with_field("resource"));
    }
    let limit = match p.limit {
        None => DEFAULT_LIST_TASKS,
        Some(0) => return Err(invalid("limit", "« limit » doit valoir au moins 1")),
        Some(n) => {
            let n = usize::try_from(n).unwrap_or(usize::MAX);
            if n > MAX_LIST_TASKS {
                return Err(CommandError::new(
                    codes::LIMIT_EXCEEDED,
                    CommandErrorKind::Limit {
                        what: "limit",
                        max: u64::try_from(MAX_LIST_TASKS).unwrap_or(u64::MAX),
                    },
                )
                .with_field("limit"));
            }
            n
        }
    };
    if p.outline_max == Some(0) {
        return Err(invalid("outline_max", "« outline_max » doit valoir au moins 1"));
    }
    if let (Some(from), Some(to)) = (p.from, p.to)
        && from > to
    {
        return Err(invalid("from", "« from » est après « to »"));
    }
    let ctx = ViewCtx::new(session);
    let project = session.project();
    let start_pos = match p.cursor {
        None => 0,
        Some(c) => {
            let uid = TaskUid::new(c);
            let pos = project.tasks.position(uid).ok_or_else(|| {
                CommandError::new(codes::TASK_NOT_FOUND, CommandErrorKind::UnknownTask(uid))
                    .with_field("cursor")
            })?;
            pos.saturating_add(1)
        }
    };
    let schedule = session.schedule();
    let keep = |pos: usize, t: &Task| -> bool {
        let s = schedule.scheduled(t.uid());
        let critical_ok = !p.critical || s.is_some_and(|s| s.critical);
        let late_ok = !p.late || s.is_some_and(|s| t.deadline.is_some_and(|d| s.finish > d));
        let from_ok = p
            .from
            .is_none_or(|from| s.is_some_and(|s| s.finish.day() >= from));
        let to_ok = p.to.is_none_or(|to| s.is_some_and(|s| s.start.day() <= to));
        let outline_ok = p.outline_max.is_none_or(|m| t.outline_level.get() <= m);
        if !(critical_ok && late_ok && from_ok && to_ok && outline_ok) {
            return false;
        }
        p.summary.is_none_or(|want| ctx.is_summary_at(pos) == want)
    };
    let mut total = 0usize;
    let mut tasks = Vec::new();
    let mut next_cursor = None;
    for (pos, t) in ctx.rows() {
        if !keep(pos, t) {
            continue;
        }
        total = total.saturating_add(1);
        if pos < start_pos {
            continue;
        }
        if tasks.len() < limit {
            tasks.push(ctx.task_at(pos, t));
        } else if next_cursor.is_none() {
            next_cursor = tasks.last().map(|v: &TaskView| v.uid);
        }
    }
    Ok(ListView {
        tasks,
        next_cursor,
        total,
    })
}
