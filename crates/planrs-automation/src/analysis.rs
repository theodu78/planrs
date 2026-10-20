//! Analyses J6 (lot L8) : `analysis.what_if` (R-234).
//!
//! Contrat de surface (C-MCP-01, règle 3) : toute date, durée et tout écart
//! viennent du moteur — planning simulé par [`Session::simulate`], écarts
//! de `planrs_core::baseline::variance` (R-224, R-225), temps ouvré compté
//! par le calendrier compilé du cœur (R-020). Ce module recopie et met en
//! forme ; il ne calcule aucune date.
//!
//! Séparé de `view.rs` (vues des tâches et du projet) pour garder chaque
//! fichier lisible ; les deux partagent [`ViewCtx`].

use std::collections::BTreeMap;

use planrs_core::schedule::explain::{DEFAULT_DEPTH, Side, explain};
use planrs_core::{
    BaselineNumber, CalendarSet, CompiledCalendar, Minutes, Moment, Project, Schedule, TaskBaseline, TaskUid,
};
use serde::Serialize;

use crate::command::params::WhatIfParams;
use crate::command::{Batch, TaskRef, resolve_task};
use crate::error::{CommandError, CommandErrorKind, codes};
use crate::explain::{ExplainView, explain_error, explain_view};
use crate::json::DurationJson;
use crate::outcome::SideEffect;
use crate::session::{ApplyOptions, Session};
use crate::text::{Language, TextContext};
use crate::view::{DiagView, DiffView, ViewCtx, signed_minutes};
use crate::workspace::ToolError;

/// Tâches expliquées par un `what_if` au plus (R-234).
pub const MAX_WHAT_IF_EXPLAIN: usize = 5;

/// Résultat d'un `what_if` (R-234).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WhatIfView {
    /// Vrai si le lot simulé serait accepté.
    pub accepted: bool,
    /// Refus du lot, comme `apply_batch` les rendrait (absent si accepté).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ToolError>,
    /// Diff du planning, celui d'un `dry_run` du même lot (absent si
    /// refusé).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffView>,
    /// Étiquettes `$x` des tâches que le lot créerait → UID simulé.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, u32>,
    /// Avertissements nouveaux du lot simulé.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<DiagView>,
    /// Effets de bord du lot simulé.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<SideEffect>,
    /// Écart de fin du projet après le lot, à la ligne de base demandée
    /// (R-224 ; unité par défaut du projet) ; absent sans `baseline`, si
    /// aucune tâche n'a cette ligne, ou si le lot est refusé.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_finish_variance: Option<DurationJson>,
    /// Fin du projet dans cette ligne de base (ISO local).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_finish: Option<String>,
    /// Vrai si la ligne du projet est déduite des tâches de premier niveau
    /// (projet sans récapitulative stockée, R-228).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub baseline_derived: bool,
    /// Explications du début des tâches demandées, sur l'état simulé
    /// (R-230 à R-233).
    pub explanations: Vec<ExplainView>,
}

/// Erreur `param.invalid` sur `field`.
fn invalid(field: &str, why: &str) -> CommandError {
    CommandError::new(
        codes::PARAM_INVALID,
        CommandErrorKind::InvalidParam(why.to_owned()),
    )
    .with_field(field)
}

/// Erreur `limit.exceeded` sur `field`.
fn limit(what: &'static str, max: usize, field: &str) -> CommandError {
    CommandError::new(
        codes::LIMIT_EXCEEDED,
        CommandErrorKind::Limit {
            what,
            max: u64::try_from(max).unwrap_or(u64::MAX),
        },
    )
    .with_field(field)
}

/// Numéro de ligne de base (0 à 10), sinon `param.invalid` sur `baseline`.
fn baseline_number(n: u8) -> Result<BaselineNumber, CommandError> {
    BaselineNumber::new(n).map_err(|_| invalid("baseline", "« baseline » : 0 à 10"))
}

/// Ligne de base du projet et ses écarts de dates.
struct ProjectLine {
    /// Valeurs de la ligne (stockée sur l'UID 0, ou déduite).
    baseline: TaskBaseline,
    /// Vrai si déduite des tâches de premier niveau.
    derived: bool,
    /// Écarts : ceux du moteur (ligne stockée) ; dates seules (déduite).
    variance: planrs_core::baseline::Variance,
}

/// Minutes signées de `a` à `b` sur le calendrier du projet (R-020), à
/// défaut minutes civiles (jamais de panique).
fn project_between(p: &Project, a: Moment, b: Moment) -> Minutes {
    CompiledCalendar::compile(&p.calendars, p.calendar)
        .ok()
        .and_then(|c| c.working_between(a, b).ok())
        .unwrap_or_else(|| a.minutes_until(b))
}

/// Ligne de base `number` du projet sur l'état (`p`, `s`) : celle de la
/// récapitulative stockée (UID 0) et ses écarts du moteur (R-224, R-225) ;
/// sans récapitulative stockée (projet créé par planrs, ADR-0014 lot L4),
/// déduite des tâches de premier niveau dotées de la ligne (début min, fin
/// max), écarts de début et de fin mesurés sur le calendrier du projet.
/// `None` si aucune ligne n'est trouvée.
fn project_line(p: &Project, s: &Schedule, set: &CalendarSet, number: BaselineNumber) -> Option<ProjectLine> {
    let v = planrs_core::baseline::variance(p, s, set, number);
    if let Some(baseline) = v.project.baseline {
        return Some(ProjectLine {
            baseline,
            derived: false,
            variance: v.project.variance,
        });
    }
    if p.summary_task.is_some() {
        return None;
    }
    let mut start: Option<Moment> = None;
    let mut finish: Option<Moment> = None;
    for t in p
        .tasks
        .iter()
        .filter(|t| !t.is_null && t.outline_level.get() == 1)
    {
        let Some(b) = t.baselines.get(&number) else {
            continue;
        };
        if let Some(bs) = b.start {
            start = Some(start.map_or(bs, |x| x.min(bs)));
        }
        if let Some(bf) = b.finish {
            finish = Some(finish.map_or(bf, |x| x.max(bf)));
        }
    }
    if start.is_none() && finish.is_none() {
        return None;
    }
    let (now_start, now_finish) = s
        .project_summary
        .as_ref()
        .map_or((s.project_start, s.project_finish), |ps| (ps.start, ps.finish));
    let date = |base: Option<Moment>, now: Moment| base.map_or(Minutes::ZERO, |b| project_between(p, b, now));
    Some(ProjectLine {
        baseline: TaskBaseline {
            start,
            finish,
            ..TaskBaseline::default()
        },
        derived: true,
        variance: planrs_core::baseline::Variance {
            start: date(start, now_start),
            finish: date(finish, now_finish),
            duration: Minutes::ZERO,
            work: Minutes::ZERO,
        },
    })
}

/// `analysis.what_if` (R-234) : simule `params.ops` sur une copie
/// (`Session::simulate`, même pipeline que `dry_run`) ; la session ne
/// change pas. Un lot refusé est rendu comme donnée (`accepted: false`),
/// message dans `lang` ; textes des durées dans la langue de la session.
///
/// # Errors
///
/// Paramètres seulement : `limit.exceeded` (plus de 5 tâches à
/// expliquer), `param.invalid` (`baseline` hors de 0 à 10),
/// `task.not_found`, `batch.unknown_label` ou `explain.not_scheduled`
/// (champ `explain`) pour une tâche à expliquer sur l'état simulé.
pub fn what_if(session: &Session, params: &WhatIfParams, lang: Language) -> Result<WhatIfView, CommandError> {
    if params.explain.len() > MAX_WHAT_IF_EXPLAIN {
        return Err(limit("explain", MAX_WHAT_IF_EXPLAIN, "explain"));
    }
    let number = params.baseline.map(baseline_number).transpose()?;
    let batch = Batch {
        format_version: Batch::FORMAT_VERSION,
        lang: None,
        ops: params.ops.clone(),
        label: None,
    };
    let options = ApplyOptions {
        detail: params.detail,
    };
    let sim = match session.simulate(&batch, options) {
        Ok(sim) => sim,
        Err(e) => {
            return Ok(WhatIfView {
                accepted: false,
                errors: vec![ToolError::from_command(&e, lang)],
                diff: None,
                labels: BTreeMap::new(),
                warnings: Vec::new(),
                side_effects: Vec::new(),
                project_finish_variance: None,
                baseline_finish: None,
                baseline_derived: false,
                explanations: Vec::new(),
            });
        }
    };
    let ctx = ViewCtx::new(session);
    let text = TextContext::for_project(&sim.project, session.lang());
    let line = number.and_then(|n| {
        let (set, _) = CalendarSet::compile(&sim.project);
        project_line(&sim.project, &sim.schedule, &set, n)
    });
    let unit = sim.project.options.default_duration_unit;
    let mut explanations = Vec::with_capacity(params.explain.len());
    for r in &params.explain {
        let uid = match r {
            TaskRef::Uid(u) if *u == TaskUid::PROJECT_SUMMARY => *u,
            r => resolve_task(r, &sim.outcome.labels, &sim.project, "explain")?,
        };
        let x = explain(&sim.project, &sim.schedule, uid, Side::Start, DEFAULT_DEPTH)
            .map_err(|e| explain_error(e).with_field("explain"))?;
        explanations.push(explain_view(&sim.project, &x, &text));
    }
    let o = &sim.outcome;
    Ok(WhatIfView {
        accepted: true,
        errors: Vec::new(),
        diff: Some(ctx.diff(&o.diff)),
        labels: o.labels.iter().map(|(k, v)| (k.clone(), v.get())).collect(),
        warnings: DiagView::list(&o.warnings),
        side_effects: o.side_effects.clone(),
        project_finish_variance: line
            .as_ref()
            .map(|l| signed_minutes(l.variance.finish, unit, &text)),
        baseline_finish: line
            .as_ref()
            .and_then(|l| l.baseline.finish)
            .map(|m| m.to_string()),
        baseline_derived: line.as_ref().is_some_and(|l| l.derived),
        explanations,
    })
}
