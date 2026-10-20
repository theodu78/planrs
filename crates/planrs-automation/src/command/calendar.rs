//! Exécution des exceptions de calendrier (R-165, R-161).
//!
//! Une exception posée par commande couvre chaque jour de `[from, to]`
//! (récurrence « période ») ; elle remplace une exception de même période,
//! et toute autre exception qui partage un jour avec elle est un refus
//! (pas de découpage implicite).
//!
//! Contrat J7 (lot L0, ADR-0015 D8) : `calendar.add`, `calendar.update`,
//! `calendar.set_week`, `calendar.delete` (R-247, R-248), squelettes
//! refusés `command.not_yet_available` jusqu'au lot L5 ; nom d'exception
//! (R-165 amendée) refusé de même.

use planrs_core::calendar::{
    Calendar, CalendarException as ModelException, CalendarKind, DayHours, DaySetting, Recurrence, TimeRange,
};
use planrs_core::{
    Assignment, AssignmentUid, CalendarModelError, CalendarUid, DayRange, Project, Subject, TimeOfDay,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::exec::Ctx;
use super::extras_invalidation::{ASSIGNMENT_SCHEDULE_KEYS, drop_keys};
use super::{CalendarException, CalendarExceptionKey, CalendarRef, Patch};
use crate::error::{CommandError, CommandErrorKind, codes};
use crate::json::DaySettingJson;
use crate::outcome::SideEffect;

/// `param.invalid` sur `field`.
fn invalid(why: String, field: &str) -> CommandError {
    CommandError::new(codes::PARAM_INVALID, CommandErrorKind::InvalidParam(why)).with_field(field)
}

/// UID du calendrier désigné, qui doit exister.
fn resolve_calendar(p: &Project, r: CalendarRef) -> Result<CalendarUid, CommandError> {
    let uid = match r {
        CalendarRef::Project => p.calendar,
        CalendarRef::Uid(u) => u,
    };
    if p.calendars.contains(uid) {
        Ok(uid)
    } else {
        Err(CommandError::new(
            codes::CALENDAR_NOT_FOUND,
            CommandErrorKind::InvalidParam(format!("aucun calendrier d'UID {uid}")),
        )
        .with_field("cal")
        .with_subject(Subject::Calendar(uid)))
    }
}

/// Réglage des jours : chômés sans plage, sinon ouvrés sur les plages
/// (validées par le modèle : 1 à 5, non vides, sans chevauchement).
fn setting(ranges: &[[TimeOfDay; 2]]) -> Result<DaySetting, CommandError> {
    if ranges.is_empty() {
        return Ok(DaySetting::NonWorking);
    }
    let bad = |e: &CalendarModelError| invalid(e.to_string(), "ranges");
    let ranges = ranges
        .iter()
        .map(|[from, to]| TimeRange::new(*from, *to))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| bad(&e))?;
    DayHours::new(ranges)
        .map(DaySetting::Working)
        .map_err(|e| bad(&e))
}

/// `calendar.exception_overlap` : chevauchement avec l'exception `existing`.
fn overlap(cal: CalendarUid, existing: usize) -> CommandError {
    CommandError::new(
        codes::CALENDAR_EXCEPTION_OVERLAP,
        CommandErrorKind::InvalidParam(format!(
            "l'exception chevauche l'exception n° {existing} du calendrier {cal} sans la recouvrir exactement"
        )),
    )
    .with_field("from")
    .with_subject(Subject::Calendar(cal))
}

/// R-161 : jette la répartition (`TimephasedData`) des affectations
/// planifiées par le calendrier `cal` ou un calendrier qui en dérive : celles
/// des tâches qui l'utilisent (calendrier propre, ou celui du projet à
/// défaut) et celles des ressources qui l'utilisent. Un seul effet résumé,
/// sujet le calendrier.
fn drop_assignment_extras(ctx: &mut Ctx<'_>, cal: CalendarUid) {
    let p = &mut *ctx.project;
    let affected: Vec<CalendarUid> = p
        .calendars
        .iter()
        .filter(|c| c.uid() == cal || matches!(c.kind(), CalendarKind::Derived { base, .. } if *base == cal))
        .map(Calendar::uid)
        .collect();
    let project_calendar = p.calendar;
    let uses = |c: Option<CalendarUid>| affected.contains(&c.unwrap_or(project_calendar));
    let targets: Vec<AssignmentUid> = p
        .assignments
        .iter()
        .filter(|a| {
            let by_task = p.tasks.get(a.task).is_some_and(|t| uses(t.calendar));
            let by_resource = a
                .resource
                .and_then(|r| p.resources.get(r))
                .is_some_and(|r| r.calendar.is_some_and(|c| affected.contains(&c)));
            (by_task || by_resource) && !a.extras.is_empty()
        })
        .map(Assignment::uid)
        .collect();
    let mut dropped = Vec::new();
    for uid in targets {
        if let Some(a) = p.assignments.get_mut(uid) {
            dropped.extend(drop_keys(&mut a.extras, ASSIGNMENT_SCHEDULE_KEYS));
        }
    }
    dropped.sort();
    dropped.dedup();
    if !dropped.is_empty() {
        ctx.effects.push(SideEffect::ExtrasDropped {
            subject: Subject::Calendar(cal),
            keys: dropped,
        });
    }
}

/// `calendar.set_exception` (R-165).
pub(super) fn set_exception(op: &CalendarException, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    // Nom d'exception (R-165 amendée) : contrat J7 L0, lot L5.
    if op.name.is_some() {
        return Err(CommandError::not_yet(7).with_field("name"));
    }
    let cal = resolve_calendar(ctx.project, op.cal)?;
    let period = DayRange::new(op.from, op.to).map_err(|e| invalid(e.to_string(), "to"))?;
    let setting = setting(&op.ranges)?;
    ctx.touched.insert(Subject::Calendar(cal), ctx.op_index);
    let calendar = ctx.project.calendars.get_mut(cal).ok_or_else(|| {
        CommandError::new(
            codes::CALENDAR_NOT_FOUND,
            CommandErrorKind::InvalidParam(String::new()),
        )
        .with_field("cal")
    })?;
    // Remplacement : l'exception de même période est retirée (son nom est
    // gardé), puis la nouvelle ajoutée ; le modèle refuse tout autre
    // chevauchement (jours dépliés, R-017, R-018).
    let mut name = String::new();
    if let Some(i) = calendar.exceptions().iter().position(|e| e.period == period) {
        let old = calendar
            .remove_exception(i)
            .map_err(|e| invalid(e.to_string(), "from"))?;
        name = old.name;
    }
    let exception = ModelException {
        name,
        period,
        setting,
        recurrence: Recurrence::Period,
        occurrences: None,
    };
    match calendar.add_exception(exception) {
        Ok(_) => {}
        Err(CalendarModelError::OverlappingException { existing }) => return Err(overlap(cal, existing)),
        Err(e) => return Err(invalid(e.to_string(), "from")),
    }
    drop_assignment_extras(ctx, cal);
    Ok(())
}

/// `calendar.remove_exception` (R-165) : retire l'exception qui commence le
/// jour `from` (la première dans l'ordre de stockage).
pub(super) fn remove_exception(op: &CalendarExceptionKey, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let cal = resolve_calendar(ctx.project, op.cal)?;
    ctx.touched.insert(Subject::Calendar(cal), ctx.op_index);
    let calendar = ctx.project.calendars.get_mut(cal).ok_or_else(|| {
        CommandError::new(
            codes::CALENDAR_NOT_FOUND,
            CommandErrorKind::InvalidParam(String::new()),
        )
        .with_field("cal")
    })?;
    let i = calendar
        .exceptions()
        .iter()
        .position(|e| e.period.first() == op.from)
        .ok_or_else(|| {
            CommandError::new(
                codes::CALENDAR_EXCEPTION_NOT_FOUND,
                CommandErrorKind::InvalidParam(format!(
                    "aucune exception du calendrier {cal} ne commence le {}",
                    op.from
                )),
            )
            .with_field("from")
            .with_subject(Subject::Calendar(cal))
        })?;
    calendar
        .remove_exception(i)
        .map_err(|e| invalid(e.to_string(), "from"))?;
    drop_assignment_extras(ctx, cal);
    Ok(())
}

/// `calendar.add` (J7, R-248) : nouveau calendrier de base (aucun de
/// `base`, `copy_of`), dérivé de `base`, ou copie de `copy_of` (les deux :
/// `param.invalid`). Contrat J7 L0 ; exécution : lot L5.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarAdd {
    /// Nom (non vide, unique sans tenir compte de la casse).
    pub name: String,
    /// Calendrier de base dont le nouveau dérive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<u32>,
    /// Calendrier copié (semaine, exceptions et base).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_of: Option<u32>,
}

/// `calendar.update` (J7, R-248) : nom et base ; absent gardé, `null` sur
/// `base` : le calendrier devient un calendrier de base.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarUpdate {
    /// Calendrier.
    pub uid: u32,
    /// Nom (`null` refusé).
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub name: Patch<String>,
    /// Base.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub base: Patch<u32>,
}

/// `calendar.set_week` (J7, R-247) : réglage de jours de la semaine type ;
/// jours non cités inchangés.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarSetWeek {
    /// Calendrier (UID ou `"project"`).
    pub calendar: CalendarRef,
    /// Jours réglés.
    pub days: Vec<DaySettingJson>,
}

/// `calendar.delete` (J7, R-248) : refusé `calendar.in_use` si le
/// calendrier sert (projet, tâche, ressource, base).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarDelete {
    /// Calendrier.
    pub uid: u32,
}

/// `calendar.add` : squelette J7 L0.
pub(super) fn add(op: &CalendarAdd, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(7))
}

/// `calendar.update` : squelette J7 L0.
pub(super) fn update(op: &CalendarUpdate, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(7))
}

/// `calendar.set_week` : squelette J7 L0.
pub(super) fn set_week(op: &CalendarSetWeek, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(7))
}

/// `calendar.delete` : squelette J7 L0.
pub(super) fn delete(op: &CalendarDelete, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(7))
}
