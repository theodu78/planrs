//! Placement d'une feuille affectée, affectation par affectation (ADR-0013
//! D1 ; règles R-170, R-171, R-173, R-174, R-176, R-177 ; lot L4a).
//!
//! Trois temps :
//! 1. [`resolve`], avant le réseau : pour chaque affectation, ressource de
//!    travail ou affectation neutre (R-176), calendrier effectif (R-171,
//!    intersections compilées et mises en cache par paire, R-172), durée
//!    `d_a` (R-170) ; pour une tâche dont les affectations de travail ont
//!    plusieurs calendriers effectifs, leur union (échelle représentative,
//!    R-173). Les calendriers combinés sont possédés par [`Resolved`], qui
//!    vit aussi longtemps que le réseau.
//! 2. [`attach`], pendant la construction du réseau : les feuilles
//!    automatiques actives à durée ouvrée reçoivent leurs parts (`Part`,
//!    rangées en CSR par position de tâche, triées par UID d'affectation) ;
//!    chaque calendrier effectif devient une échelle de [`TimeScale`].
//! 3. Les passes placent la feuille par un [`Placer`] : une échelle et une
//!    durée (chemin J4, une seule affectation sans délai comprise), ou ses
//!    parts. La sortie ([`Calc::place_assignments`]) range les
//!    [`AssignmentSchedule`] et la durée calculée des feuilles (mesure de
//!    l'union des intervalles travaillés, R-173).
//!
//! Placement d'une feuille à parts, depuis une borne `x` (R-173, R-177) :
//! - part `a` : `s_a = forward_span_a(add_a(x, delay_a), d_a)` sur son
//!   échelle ;
//! - début de la tâche : `T(x) = min_a forward_span_a(x, d_a).start`, le
//!   premier instant où une ressource affectée peut travailler (les délais ne
//!   décalent pas la tâche, S45) ; `T` est idempotente et les parts placées
//!   depuis `T(x)` sont celles placées depuis `x` (aucune part n'a de temps
//!   ouvré dans `[x, T(x))`) : le début planifié suffit à les retrouver ;
//! - fin : `max_a f_a`.
//!
//! Passe arrière et contraintes de fin (R-174) : `LS = min_a
//! (backward_span_a(y, d_a).start − delay_a)` sur l'échelle de chaque part,
//! dates au plus tard `F(LS)` ; dernier début plaçable sous un plafond de
//! début `y` : `max_a latest_start_a(y, d_a)` (exact pour `T`).

use std::collections::BTreeMap;

use super::constraint::Mode;
use super::network::{ELAPSED, Fail, PROJECT, Role, TaskInfo, ix, pos};
use super::{AssignmentRole, AssignmentSchedule, Calc, EffectiveCalendar};
use crate::calendar::{CalendarSet, CompiledCalendar, Span, TimeScale};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::duration::{Duration, Minutes, TimeBasis, Work};
use crate::error::CalendarError;
use crate::fixed::round_div;
use crate::ids::{AssignmentUid, CalendarUid, ResourceUid, TaskUid};
use crate::model::{
    ConstraintKind, LevelingDelay, Project, ResourceKind, SchedulingMode, Task, TaskType, WorkContour,
};
use crate::quantity::Units;
use crate::time::Moment;

/// Code de l'échec d'un `d_a` hors du domaine des durées (R-170, R-181).
const OUT_OF_DOMAIN: &str = "assignment.duration_out_of_domain";

/// Plafond des unions mises en cache pour la mesure de la durée d'une
/// feuille (règle 10) ; au-delà, la mesure se replie sur le plus grand des
/// temps ouvrés des échelles actives.
const UNION_CACHE_LIMIT: usize = 256;

/// Unions compilées par ensemble trié d'échelles (mesure de la durée d'une
/// feuille, R-173) ; `None` : union trop morcelée.
type UnionCache = BTreeMap<Vec<u32>, Option<CompiledCalendar>>;

/// Calendrier effectif résolu, avant l'indexation des échelles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CalRef {
    /// Calendrier compilé du modèle (`CalendarSet`).
    Set(CalendarUid),
    /// Échelle du calendrier du projet, `PROJECT` (R-175).
    Project,
    /// Temps écoulé, `ELAPSED` (tâche à durée écoulée, R-182).
    Elapsed,
    /// Calendrier combiné possédé par [`Resolved`] (intersection ou union).
    Owned(usize),
}

/// Affectation résolue.
#[derive(Clone, Copy, Debug)]
struct Resolution {
    /// UID de l'affectation.
    uid: AssignmentUid,
    /// Position de la tâche dans le modèle.
    task_pos: usize,
    /// Tâche.
    task: TaskUid,
    /// Ressource (`None` : affectation fictive −65 535).
    resource: Option<ResourceUid>,
    /// Ressource de travail connue : l'affectation peut placer la tâche
    /// (R-176 sinon).
    work_resource: bool,
    /// Calendrier effectif (R-171).
    cal: CalRef,
    /// Calendrier effectif, pour la sortie.
    calendar: EffectiveCalendar,
    /// Calendrier de la tâche seule (affectation neutre, R-176).
    task_calendar: EffectiveCalendar,
    /// `d_a` (R-170), ou le code de son échec.
    duration: Result<Minutes, &'static str>,
    /// Délai d'affectation (R-177).
    delay: Minutes,
    /// Retard de nivellement de l'affectation (R-178).
    leveling: Option<LevelingDelay>,
    /// Profil de travail (R-180).
    contour: WorkContour,
    /// Travail.
    work: Work,
    /// Unités.
    units: Units,
}

/// Résultat de [`resolve`] : décisions par affectation et calendriers
/// combinés possédés.
pub(super) struct Resolved {
    /// Intersections et unions compilées.
    owned: Vec<CompiledCalendar>,
    /// Affectations résolues, triées par (position de tâche, UID).
    items: Vec<Resolution>,
    /// Échelle représentative d'une tâche dont les affectations de travail
    /// ont plusieurs calendriers effectifs (union, R-173), par position.
    unions: BTreeMap<usize, CalRef>,
}

/// Calendrier compilé utilisable (compilé, avec du temps ouvré).
fn usable(set: &CalendarSet, uid: CalendarUid) -> bool {
    set.get(uid).is_ok_and(CompiledCalendar::has_working_time)
}

/// R-170 (moteur) : `d_a = round_div(W × 10 000, U)` pour les unités fixes
/// et le travail fixe ; durée de la tâche pour une feuille à durée fixe et
/// pour des unités nulles (R-181) ; hors du domaine des durées : échec
/// (R-069). Une récapitulative (`summary`) n'a pas de durée tenue : son
/// affectation dure W ÷ U quel que soit son type (R-179 ; toutes les
/// récapitulatives du corpus sont à durée fixe, O32, de durée enregistrée
/// nulle dans A6).
fn assignment_duration(
    task: &Task,
    work: Work,
    units: Units,
    summary: bool,
) -> Result<Minutes, &'static str> {
    if (task.task_type == TaskType::FixedDuration && !summary) || units == Units::ZERO {
        return Ok(task.duration.minutes());
    }
    i128::from(work.minutes().get())
        .checked_mul(10_000)
        .and_then(|num| round_div(num, i128::from(units.get())))
        .filter(|m| (0..=Duration::MAX_MINUTES).contains(m))
        .map(Minutes::new)
        .ok_or(OUT_OF_DOMAIN)
}

/// Construction de [`Resolved`].
struct Resolver<'s> {
    set: &'s CalendarSet,
    /// Calendrier du projet (échelle `PROJECT`).
    project_place: &'s CompiledCalendar,
    owned: Vec<CompiledCalendar>,
    /// Intersections par paire (tâche, ressource) ; `None` : vide ou en
    /// échec.
    pairs: BTreeMap<(CalendarUid, CalendarUid), Option<usize>>,
    /// Unions par ensemble trié de calendriers effectifs.
    unions: BTreeMap<Vec<CalRef>, Option<usize>>,
}

impl Resolver<'_> {
    /// Calendrier compilé de `r`.
    fn compiled(&self, r: CalRef) -> Option<&CompiledCalendar> {
        match r {
            CalRef::Set(uid) => self.set.get(uid).ok(),
            CalRef::Project => Some(self.project_place),
            CalRef::Elapsed => None,
            CalRef::Owned(i) => self.owned.get(i),
        }
    }

    /// Intersection compilée de `own` (tâche) et `res` (ressource), mise en
    /// cache ; `None` si elle est vide ou trop morcelée (diagnostic).
    fn intersection(
        &mut self,
        (own, res): (CalendarUid, CalendarUid),
        assignment: AssignmentUid,
        diags: &mut Diagnostics,
    ) -> Option<usize> {
        if let Some(slot) = self.pairs.get(&(own, res)) {
            if slot.is_none() {
                Self::empty(own, res, assignment, diags);
            }
            return *slot;
        }
        let combined = match (self.set.get(own), self.set.get(res)) {
            (Ok(a), Ok(b)) => a.intersect(b),
            (Err(e), _) | (_, Err(e)) => Err(e),
        };
        let slot = match combined {
            Ok(c) if c.has_working_time() => {
                let i = self.owned.len();
                self.owned.push(c);
                Some(i)
            }
            Ok(_) => {
                Self::empty(own, res, assignment, diags);
                None
            }
            Err(CalendarError::TooManySegments { limit }) => {
                diags.push(Diagnostic::CalendarTooManySegments {
                    calendar: own,
                    other: res,
                    limit,
                });
                None
            }
            Err(_) => None,
        };
        self.pairs.insert((own, res), slot);
        slot
    }

    /// `assignment.calendar_empty` : repli sur le calendrier de la tâche.
    fn empty(own: CalendarUid, res: CalendarUid, assignment: AssignmentUid, diags: &mut Diagnostics) {
        diags.push(Diagnostic::AssignmentCalendarEmpty {
            assignment,
            task_calendar: own,
            resource_calendar: res,
        });
    }

    /// Union compilée des calendriers `refs` (triés, sans doublon, au moins
    /// deux), mise en cache ; `None` si elle est trop morcelée.
    fn union(&mut self, refs: &[CalRef], diags: &mut Diagnostics) -> Option<usize> {
        if let Some(slot) = self.unions.get(refs) {
            return *slot;
        }
        let mut acc: Option<CompiledCalendar> = None;
        let mut failed = false;
        for &r in refs {
            let Some(c) = self.compiled(r) else {
                failed = true;
                break;
            };
            let next = match &acc {
                None => Ok(c.clone()),
                Some(a) => a.union(c),
            };
            match next {
                Ok(n) => acc = Some(n),
                Err(e) => {
                    if let (CalendarError::TooManySegments { limit }, Some(a)) = (e, &acc) {
                        diags.push(Diagnostic::CalendarTooManySegments {
                            calendar: a.uid(),
                            other: c.uid(),
                            limit,
                        });
                    }
                    failed = true;
                    break;
                }
            }
        }
        let slot = match acc.filter(|_| !failed) {
            Some(c) => {
                let i = self.owned.len();
                self.owned.push(c);
                Some(i)
            }
            None => None,
        };
        self.unions.insert(refs.to_vec(), slot);
        slot
    }

    /// R-171 : calendrier effectif d'une affectation de la tâche `task` à
    /// une ressource de calendrier `res_cal`.
    fn effective(
        &mut self,
        task: &Task,
        res_cal: Option<CalendarUid>,
        assignment: AssignmentUid,
        diags: &mut Diagnostics,
    ) -> (CalRef, EffectiveCalendar) {
        let set = self.set;
        let own = task.calendar.filter(|&c| !task.is_null && usable(set, c));
        let project = (
            CalRef::Project,
            EffectiveCalendar::Project(self.project_place.uid()),
        );
        if task.ignore_resource_calendar {
            // S48 : « only the task calendar is considered ».
            return own.map_or(project, |o| (CalRef::Set(o), EffectiveCalendar::Task(o)));
        }
        let res = match res_cal {
            Some(c) if usable(set, c) => Some(c),
            Some(c) => {
                // Calendrier de ressource inutilisable : comme sans
                // calendrier (R-066).
                diags.push(Diagnostic::TaskCalendarFallback {
                    task: task.uid(),
                    calendar: c,
                });
                None
            }
            None => None,
        };
        match (own, res) {
            (Some(o), Some(r)) => {
                let both = EffectiveCalendar::Intersection { task: o, resource: r };
                if o == r {
                    return (CalRef::Set(o), both);
                }
                match self.intersection((o, r), assignment, diags) {
                    Some(i) => (CalRef::Owned(i), both),
                    None => (CalRef::Set(o), EffectiveCalendar::Task(o)),
                }
            }
            (Some(o), None) => (CalRef::Set(o), EffectiveCalendar::Task(o)),
            (None, Some(r)) => (CalRef::Set(r), EffectiveCalendar::Resource(r)),
            (None, None) => project,
        }
    }
}

/// Étape 1 : résout chaque affectation d'une tâche connue (R-170, R-171,
/// R-176) et l'union des calendriers effectifs des tâches qui en ont
/// plusieurs. `project_place` : calendrier du projet, de base (échelle
/// `PROJECT`, R-113, R-175).
pub(super) fn resolve(
    project: &Project,
    set: &CalendarSet,
    project_place: &CompiledCalendar,
    diags: &mut Diagnostics,
) -> Resolved {
    let mut r = Resolver {
        set,
        project_place,
        owned: Vec::new(),
        pairs: BTreeMap::new(),
        unions: BTreeMap::new(),
    };
    let mut items = Vec::with_capacity(project.assignments.len());
    for a in &project.assignments {
        // Tâche inconnue : signalée par la validation, ignorée ici.
        let Some(task_pos) = project.tasks.position(a.task) else {
            continue;
        };
        let Some(task) = project.tasks.at(task_pos) else {
            continue;
        };
        let resource = a.resource.and_then(|uid| project.resources.get(uid));
        let work_resource = resource.is_some_and(|res| res.kind == ResourceKind::Work);
        let task_calendar = if task.duration.basis() == TimeBasis::Elapsed {
            EffectiveCalendar::Elapsed
        } else {
            match task.calendar.filter(|&c| !task.is_null && usable(set, c)) {
                Some(o) => EffectiveCalendar::Task(o),
                None => EffectiveCalendar::Project(project_place.uid()),
            }
        };
        let (cal, calendar) = if work_resource && task_calendar == EffectiveCalendar::Elapsed {
            // R-182 : durée écoulée, calendriers ignorés.
            (CalRef::Elapsed, EffectiveCalendar::Elapsed)
        } else if work_resource {
            r.effective(task, resource.and_then(|res| res.calendar), a.uid(), diags)
        } else {
            (CalRef::Project, task_calendar)
        };
        items.push(Resolution {
            uid: a.uid(),
            task_pos,
            task: a.task,
            resource: a.resource,
            work_resource,
            cal,
            calendar,
            task_calendar,
            // Rôle de la tâche inconnu ici : une récapitulative est
            // corrigée par `attach` (R-179).
            duration: assignment_duration(task, a.work, a.units, false),
            delay: a.delay.max(Minutes::ZERO),
            leveling: a.leveling_delay.filter(|l| l.minutes > Minutes::ZERO),
            contour: a.contour,
            work: a.work,
            units: a.units,
        });
    }
    // Clés uniques (UID d'affectation) : tri instable sans effet sur l'ordre.
    items.sort_unstable_by_key(|i| (i.task_pos, i.uid));
    // Union des calendriers effectifs des affectations de travail, par
    // tâche qui en a plusieurs (échelle représentative, R-173).
    let mut unions = BTreeMap::new();
    let mut k = 0;
    // Tampon réutilisé d'une tâche à l'autre (aucune allocation par tâche).
    let mut refs: Vec<CalRef> = Vec::new();
    while let Some(first) = items.get(k) {
        let p = first.task_pos;
        refs.clear();
        while let Some(i) = items.get(k).filter(|i| i.task_pos == p) {
            if i.work_resource {
                refs.push(i.cal);
            }
            k = k.saturating_add(1);
        }
        refs.sort_unstable();
        refs.dedup();
        if refs.len() >= 2 {
            let rep = r.union(&refs, diags).map(CalRef::Owned);
            if let Some(rep) = rep {
                unions.insert(p, rep);
            }
        }
    }
    Resolved {
        owned: r.owned,
        items,
        unions,
    }
}

/// Part d'une feuille : une affectation de travail qui la place (R-173).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Part {
    /// Affectation.
    pub assignment: AssignmentUid,
    /// Échelle effective (indice dans `Network::scales`).
    pub scale: u32,
    /// Délai d'affectation, ouvré sur l'échelle (R-177).
    pub delay: Minutes,
    /// Retard de nivellement de l'affectation, ajouté au délai (R-178) :
    /// minutes, et vrai s'il est écoulé.
    pub level: (Minutes, bool),
    /// Durée `d_a` (R-170).
    pub d: Minutes,
}

impl Part {
    /// Part sans délai ni retard de nivellement.
    fn undelayed(&self) -> bool {
        self.delay == Minutes::ZERO && self.level.0 == Minutes::ZERO
    }
}

/// Parts (au moins une) sur une même échelle, sans délai, toutes de durée
/// non nulle ou toutes nulles : elles commencent au même instant et la
/// feuille se place comme sa plus longue part ; l'union de leurs
/// intervalles est cette part (mesure = plus grand `d_a`).
fn shared_scale(parts: &[Part]) -> bool {
    match parts {
        [] => false,
        [first, rest @ ..] => {
            first.undelayed()
                && rest.iter().all(|p| {
                    p.scale == first.scale
                        && p.undelayed()
                        && (p.d == Minutes::ZERO) == (first.d == Minutes::ZERO)
                })
        }
    }
}

/// Retard de nivellement d'une affectation, sous la forme d'une part.
fn level_of(l: Option<LevelingDelay>) -> (Minutes, bool) {
    l.map_or((Minutes::ZERO, false), |l| (l.minutes, l.elapsed))
}

/// Affectation à ranger dans le planning, avec la part qui la place.
#[derive(Clone, Copy, Debug)]
struct Output {
    /// Résolution.
    res: Resolution,
    /// Indice dans `Parts::items`, pour une affectation qui place sa
    /// feuille.
    part: Option<usize>,
    /// Échelle effective d'une affectation de travail d'une récapitulative
    /// (R-179) ou d'une tâche manuelle (R-183).
    scale: Option<u32>,
}

/// Parts des feuilles (CSR par position de tâche) et affectations à ranger.
#[derive(Debug, Default)]
pub(super) struct Parts {
    /// Parts de la tâche en position `p` : `parts[offsets[p]..offsets[p + 1]]`.
    offsets: Vec<u32>,
    /// Parts, triées par UID d'affectation dans chaque tâche.
    items: Vec<Part>,
    /// Échelle représentative (union des échelles effectives), par position
    /// de tâche à parts de plusieurs échelles.
    rep: BTreeMap<usize, u32>,
    /// Échec d'un `d_a` (R-069), par position de tâche.
    fail: BTreeMap<usize, &'static str>,
    /// Affectations à ranger (feuilles actives, récapitulatives, manuelles),
    /// triées par position de tâche puis UID.
    outputs: Vec<Output>,
}

impl Parts {
    /// Parts de la tâche en position `p` (vide : chemin J4).
    pub fn of(&self, p: usize) -> &[Part] {
        let range = || {
            let a = pos(*self.offsets.get(p)?)?;
            let b = pos(*self.offsets.get(p.checked_add(1)?)?)?;
            self.items.get(a..b)
        };
        range().unwrap_or_default()
    }

    /// Code d'échec du `d_a` d'une affectation de la tâche en position `p`.
    pub fn failure(&self, p: usize) -> Option<&'static str> {
        self.fail.get(&p).copied()
    }

    /// Échelle de placement d'une feuille à parts : celle de son unique
    /// part, ou l'union de leurs échelles (sinon la première) ; `None` sans
    /// part.
    pub fn place(&self, p: usize) -> Option<u32> {
        let parts = self.of(p);
        let first = parts.first()?;
        Some(self.rep.get(&p).copied().unwrap_or(first.scale))
    }

    /// Durée de la tâche pour le réseau (lags en %, jalon, % achevé) : le
    /// plus grand des `d_a` (convention du lot L2) ; `None` sans part.
    pub fn duration(&self, p: usize) -> Option<Minutes> {
        self.of(p).iter().map(|part| part.d).max()
    }
}

/// Feuille qui se place par ses affectations : active, automatique (R-173 ;
/// durée écoulée sur le temps civil, R-182 ; manuelle : dates saisies,
/// R-183).
fn placed_by_parts(info: &TaskInfo, task: &Task) -> bool {
    info.role == Role::Leaf
        && !info.inactive()
        && !info.manual_undated
        && matches!(task.mode, SchedulingMode::Auto)
}

/// Étape 2 : parts des feuilles et échelles effectives, ajoutées à `scales`.
pub(super) fn attach<'a>(
    project: &Project,
    resolved: &'a Resolved,
    set: &'a CalendarSet,
    scales: &mut Vec<TimeScale<'a>>,
    tasks: &[TaskInfo],
    diags: &mut Diagnostics,
) -> Option<Parts> {
    let mut index: BTreeMap<CalRef, u32> = BTreeMap::new();
    index.insert(CalRef::Project, PROJECT);
    // Un calendrier déjà rangé (calendrier du projet, calendrier d'une
    // tâche) garde son échelle : une affectation sur le calendrier de sa
    // tâche la place exactement comme sans affectation (P34).
    let mut scale_of = |r: CalRef, scales: &mut Vec<TimeScale<'a>>| -> Option<u32> {
        if let Some(&i) = index.get(&r) {
            return Some(i);
        }
        let c = match r {
            CalRef::Set(uid) => set.get(uid).ok()?,
            CalRef::Owned(i) => resolved.owned.get(i)?,
            CalRef::Project => return Some(PROJECT),
            CalRef::Elapsed => return Some(ELAPSED),
        };
        // Une échelle de même forme compilée (calendrier de ressource dérivé
        // sans exception propre, intersection égale au calendrier de la
        // tâche) est réutilisée : mêmes conversions, et les parts qui la
        // partagent se placent comme une durée unique (`Placer::of`).
        let known = scales.iter().position(
            |s| matches!(s, TimeScale::Working(k) if std::ptr::eq(*k, c) || k.same_working_time(c)),
        );
        let i = if let Some(k) = known {
            ix(k)?
        } else {
            let i = ix(scales.len())?;
            scales.push(TimeScale::Working(c));
            i
        };
        index.insert(r, i);
        Some(i)
    };
    let n = tasks.len();
    let mut out = Parts {
        offsets: Vec::with_capacity(n.checked_add(1)?),
        ..Parts::default()
    };
    out.offsets.push(0);
    let mut k = 0usize;
    for p in 0..n {
        let info = tasks.get(p)?;
        let task = project.tasks.at(p)?;
        let by_parts = placed_by_parts(info, task);
        while let Some(res) = resolved.items.get(k).filter(|i| i.task_pos == p) {
            k = k.checked_add(1)?;
            // R-183 : tâche inactive, aucune affectation rangée.
            if info.role == Role::Null || info.inactive() || info.manual_undated {
                continue;
            }
            if res.work_resource {
                notices(res, diags);
            }
            let mut part = None;
            let mut scale = None;
            let mut resolution = *res;
            if by_parts && res.work_resource {
                match res.duration {
                    Ok(d) => {
                        part = Some(out.items.len());
                        out.items.push(Part {
                            assignment: res.uid,
                            scale: scale_of(res.cal, scales)?,
                            delay: res.delay,
                            level: level_of(res.leveling),
                            d,
                        });
                    }
                    Err(code) => {
                        diags.push(Diagnostic::AssignmentDurationOutOfDomain { assignment: res.uid });
                        out.fail.insert(p, code);
                    }
                }
            } else if res.work_resource {
                // R-179, R-183 : échelle effective, pour placer l'affectation
                // d'une récapitulative ou mesurer celle d'une manuelle.
                scale = Some(scale_of(res.cal, scales)?);
                let summary = info.role == Role::Summary && matches!(task.mode, SchedulingMode::Auto);
                if summary {
                    // R-179 : W ÷ U, même pour une récapitulative à durée
                    // fixe.
                    resolution.duration = assignment_duration(task, res.work, res.units, true);
                    if resolution.duration.is_err() {
                        diags.push(Diagnostic::AssignmentDurationOutOfDomain { assignment: res.uid });
                    }
                }
            }
            out.outputs.push(Output {
                res: resolution,
                part,
                scale,
            });
        }
        if let Some(&rep) = resolved.unions.get(&p).filter(|_| by_parts) {
            out.rep.insert(p, scale_of(rep, scales)?);
        }
        out.offsets.push(ix(out.items.len())?);
    }
    Some(out)
}

/// Placement d'une feuille : une échelle et une durée (chemin J4, et une
/// seule affectation sans délai, P34), ou ses parts (R-173, R-174, R-177).
#[derive(Clone, Copy)]
pub(super) enum Placer<'p, 'a> {
    /// Une échelle, une durée.
    Single(TimeScale<'a>, Minutes),
    /// Parts d'une feuille affectée.
    Parts {
        /// Parts, triées par UID d'affectation.
        parts: &'p [Part],
        /// Échelles du réseau.
        scales: &'p [TimeScale<'a>],
        /// Échelle de placement de la feuille (union des échelles de ses
        /// parts) : instant de fin d'une contrainte de fin (R-174).
        place: TimeScale<'a>,
    },
}

impl<'p, 'a> Placer<'p, 'a> {
    /// Placement de la feuille de parts `parts` (vide : J4) sur l'échelle de
    /// placement `place`, de durée `d`.
    pub fn of(scales: &'p [TimeScale<'a>], parts: &'p [Part], place: u32, d: Minutes) -> Result<Self, Fail> {
        // Une échelle commune, sans délai, toutes de durée > 0 (ou toutes
        // nulles) : le placement est celui de la plus longue (`d` = max
        // `d_a`), le chemin J4 suffit.
        let single = parts.is_empty() || shared_scale(parts);
        let scale = *pos(place)
            .and_then(|i| scales.get(i))
            .ok_or(Fail::Internal("échelle hors bornes"))?;
        Ok(if single {
            Self::Single(scale, d)
        } else {
            Self::Parts {
                parts,
                scales,
                place: scale,
            }
        })
    }

    /// Échelle d'une part.
    fn scale(scales: &[TimeScale<'a>], part: &Part) -> Result<TimeScale<'a>, Fail> {
        pos(part.scale)
            .and_then(|i| scales.get(i))
            .copied()
            .ok_or(Fail::Internal("échelle de part hors bornes"))
    }

    /// R-177, R-178 : dates de la part depuis la borne `from` : délai compté
    /// en temps ouvré de son échelle, puis retard de nivellement de
    /// l'affectation (ouvré sur la même échelle, ou écoulé), puis `d_a`
    /// (R-030, R-031).
    pub fn part_span(scales: &[TimeScale<'a>], part: &Part, from: Moment) -> Result<Span, Fail> {
        let scale = Self::scale(scales, part)?;
        let x = scale.add(from, part.delay)?;
        let x = Self::level_scale(scale, part).add(x, part.level.0)?;
        Ok(scale.forward_span(x, part.d)?)
    }

    /// Échelle du retard de nivellement d'une part : la sienne, ou le temps
    /// écoulé (R-178).
    fn level_scale(scale: TimeScale<'a>, part: &Part) -> TimeScale<'a> {
        if part.level.1 { TimeScale::Elapsed } else { scale }
    }

    /// Passe avant depuis la borne `from` (R-173) : début `T(from)`, fin
    /// `max f_a`.
    pub fn forward_span(&self, from: Moment) -> Result<Span, Fail> {
        let (parts, scales) = match self {
            Self::Single(scale, d) => return Ok(scale.forward_span(from, *d)?),
            Self::Parts { parts, scales, .. } => (*parts, *scales),
        };
        let mut span: Option<Span> = None;
        for part in parts {
            let own = Self::scale(scales, part)?.forward_span(from, part.d)?;
            let start = own.start;
            let finish = if part.undelayed() {
                own.finish
            } else {
                Self::part_span(scales, part, from)?.finish
            };
            span = Some(span.map_or(Span { start, finish }, |s| Span {
                start: s.start.min(start),
                finish: s.finish.max(finish),
            }));
        }
        span.ok_or(Fail::Internal("feuille sans part"))
    }

    /// Passe arrière sous le plafond `to` (R-174) : `LS = min_a
    /// (backward_span_a(to, d_a).start − delay_a)`, dates `F(LS)`.
    pub fn backward_span(&self, to: Moment) -> Result<Span, Fail> {
        let (parts, scales) = match self {
            Self::Single(scale, d) => return Ok(scale.backward_span(to, *d)?),
            Self::Parts { parts, scales, .. } => (*parts, *scales),
        };
        let mut start: Option<Moment> = None;
        for part in parts {
            let scale = Self::scale(scales, part)?;
            let b = scale.backward_span(to, part.d)?.start;
            let neg = |m: Minutes| m.checked_neg().ok_or(Fail::Internal("délai hors bornes"));
            let b = Self::level_scale(scale, part).add(b, neg(part.level.0)?)?;
            let origin = scale.add(b, neg(part.delay)?)?;
            start = Some(start.map_or(origin, |s| s.min(origin)));
        }
        let ls = start.ok_or(Fail::Internal("feuille sans part"))?;
        self.forward_span(ls)
    }

    /// Dernier instant dont le placement commence au plus tard en `y`
    /// (R-047) : `max_a latest_start_a(y, d_a)`.
    pub fn latest_start(&self, y: Moment) -> Result<Moment, Fail> {
        let (parts, scales) = match self {
            Self::Single(scale, d) => return Ok(scale.latest_start(y, *d)?),
            Self::Parts { parts, scales, .. } => (*parts, *scales),
        };
        let mut best: Option<Moment> = None;
        for part in parts {
            let x = Self::scale(scales, part)?.latest_start(y, part.d)?;
            best = Some(best.map_or(x, |b| b.max(x)));
        }
        best.ok_or(Fail::Internal("feuille sans part"))
    }

    /// R-090, R-174 : ancrage d'une contrainte datée. Type début : passe
    /// avant depuis la date. Type fin : `f*` = la date recalée sur l'échelle
    /// de placement comme pour une feuille sans affectation (R-031 en passe
    /// arrière), fin de l'ancrage = `f*` (O14), début = min des débuts qui
    /// finissent chaque part en `f*`.
    pub fn anchor(&self, kind: ConstraintKind, date: Moment) -> Result<Span, Fail> {
        match self {
            Self::Single(scale, d) => Ok(super::constraint::anchor(*scale, kind, date, *d)?),
            Self::Parts { .. } if kind.is_start_side() => self.forward_span(date),
            Self::Parts { place, .. } => {
                let finish = place.backward_span(date, Minutes::ZERO)?.finish;
                let start = self.backward_span(finish)?.start;
                Ok(Span { start, finish })
            }
        }
    }
}

impl<'n, 'a> Calc<'n, 'a> {
    /// Placement du nœud `v` : feuille à parts, sinon son échelle de
    /// placement et sa durée ; échec d'un `d_a` hors domaine (R-069).
    pub(super) fn placer(&self, v: super::network::NodeIx) -> Result<Placer<'n, 'a>, Fail> {
        let net: &'n super::network::Network<'a> = self.net;
        let node = net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let parts = if node.kind == super::network::NodeKind::Leaf {
            if let Some(code) = net.parts.failure(node.task) {
                return Err(Fail::Conversion(code));
            }
            net.parts.of(node.task)
        } else {
            &[]
        };
        Placer::of(&net.scales, parts, node.place, node.duration)
    }

    /// Sortie : affectations placées (R-171, R-173, R-176, R-177) et durée
    /// calculée des feuilles à parts (mesure de l'union des intervalles
    /// travaillés, R-173), par position de tâche.
    pub(super) fn place_assignments(
        &mut self,
    ) -> (
        BTreeMap<AssignmentUid, AssignmentSchedule>,
        BTreeMap<usize, Minutes>,
    ) {
        let net = self.net;
        let mut durations = BTreeMap::new();
        let mut unions: UnionCache = BTreeMap::new();
        // Dates de chaque part depuis le début planifié, par indice de part :
        // réutilisées par `assignment_schedule` (même borne, même résultat).
        let mut placed: Vec<Option<(Moment, Span)>> = vec![None; net.parts.items.len()];
        let mut spans: Vec<Span> = Vec::new();
        for p in 0..net.tasks.len() {
            let parts = net.parts.of(p);
            if parts.len() < 2 && parts.first().is_none_or(Part::undelayed) {
                continue;
            }
            let Some(planned) = self.planned_start(p) else {
                continue;
            };
            let first = net.parts.offsets.get(p).and_then(|&o| pos(o));
            spans.clear();
            for (k, part) in parts.iter().enumerate() {
                let Ok(span) = Placer::part_span(&net.scales, part, planned) else {
                    continue;
                };
                spans.push(span);
                if let Some(slot) = first
                    .and_then(|f| f.checked_add(k))
                    .and_then(|i| placed.get_mut(i))
                {
                    *slot = Some((planned, span));
                }
            }
            if spans.len() != parts.len() {
                continue;
            }
            let measured = if parts.len() == 1 || shared_scale(parts) {
                parts.iter().map(|part| part.d).max()
            } else {
                // L'union de toutes les échelles des parts est l'échelle de
                // la feuille (déjà compilée, R-173).
                let mut all: Vec<u32> = parts.iter().map(|part| part.scale).collect();
                all.sort_unstable();
                all.dedup();
                let place = net.parts.place(p).and_then(|i| net.scale(i)).copied();
                let full = place.map(|s| (all.as_slice(), s));
                union_measure(&net.scales, parts, &spans, (full, &mut unions))
            };
            if let Some(m) = measured {
                durations.insert(p, m);
            }
        }
        // Sortie triée par UID puis construite en bloc (plus rapide que des
        // insertions une à une).
        let mut out: Vec<(AssignmentUid, AssignmentSchedule)> = net
            .parts
            .outputs
            .iter()
            .filter_map(|o| Some((o.res.uid, self.assignment_schedule(o, &placed)?)))
            .collect();
        out.sort_unstable_by_key(|(uid, _)| *uid);
        (out.into_iter().collect(), durations)
    }

    /// Début planifié de la tâche en position `p` (au plus tôt depuis le
    /// début, au plus tard à rebours, R-112) ; `None` sans dates.
    fn planned_start(&self, p: usize) -> Option<Moment> {
        let v = self.net.task(p)?.exit?;
        let s = self.state(v)?;
        match self.mode {
            Mode::FromStart => s.early.map(|e| e.start),
            Mode::FromFinish => s.late.map(|l| l.start),
        }
    }

    /// `AssignmentSchedule` d'une affectation, si sa tâche est planifiée.
    fn assignment_schedule(
        &self,
        o: &Output,
        placed: &[Option<(Moment, Span)>],
    ) -> Option<AssignmentSchedule> {
        let net = self.net;
        let info = net.task(o.res.task_pos)?;
        let task = self.project.tasks.at(o.res.task_pos)?;
        let v = info.exit?;
        let state = self.state(v)?;
        let early = state.early?;
        let late = state.late;
        // R-112 : l'affectation est rangée sur les dates planifiées de sa
        // tâche (au plus tôt depuis le début, au plus tard à rebours, repli
        // au plus tôt sans dates au plus tard comme `rollup`) ; ses dates au
        // plus tard restent dans `late`.
        let planned = match self.mode {
            Mode::FromStart => early,
            Mode::FromFinish => late.unwrap_or(early),
        };
        let node = net.node(v)?;
        let res = &o.res;
        // R-183 : une tâche manuelle (feuille ou récapitulative) range ses
        // affectations sur ses dates saisies.
        let role = if !matches!(task.mode, SchedulingMode::Auto) {
            AssignmentRole::Manual
        } else if info.role == Role::Summary {
            AssignmentRole::Summary
        } else if res.work_resource {
            AssignmentRole::Work
        } else {
            AssignmentRole::Neutral
        };
        let parts = net.parts.of(o.res.task_pos);
        let mut effective_units = res.units;
        let span_from = |i: usize, part: &Part, from: Moment| match placed.get(i) {
            Some(Some((at, span))) if *at == from => Some(*span),
            _ => Placer::part_span(&net.scales, part, from).ok(),
        };
        let (span, late, duration, calendar) = match o.part.and_then(|i| Some((i, net.parts.items.get(i)?))) {
            // Une seule part sans délai : la feuille est la part (P34).
            Some((_, part)) if parts.len() == 1 && part.undelayed() => (planned, late, part.d, res.calendar),
            Some((i, part)) => (
                span_from(i, part, planned.start)?,
                late.and_then(|l| span_from(i, part, l.start)),
                part.d,
                res.calendar,
            ),
            None => match (role, o.scale, res.duration) {
                // R-179 : affectation de travail d'une récapitulative : départ
                // au début de la récapitulative, délais compris, `d_a` sur son
                // calendrier effectif, jamais tronquée.
                (AssignmentRole::Summary, Some(scale), Ok(d)) => {
                    let part = Part {
                        assignment: res.uid,
                        scale,
                        delay: res.delay,
                        level: level_of(res.leveling),
                        d,
                    };
                    (
                        Placer::part_span(&net.scales, &part, planned.start).ok()?,
                        late.and_then(|l| Placer::part_span(&net.scales, &part, l.start).ok()),
                        d,
                        res.calendar,
                    )
                }
                // R-183 : affectation de travail d'une tâche manuelle : le
                // span de la tâche, débit = travail ÷ temps ouvré du span.
                (AssignmentRole::Manual, Some(scale), _) => {
                    let measured = net
                        .scale(scale)
                        .and_then(|s| s.between(planned.start, planned.finish).ok())
                        .filter(|m| m.get() > 0);
                    if let Some(m) = measured {
                        effective_units = rate(res.work, m).unwrap_or(res.units);
                    }
                    (planned, late, measured.unwrap_or(node.duration), res.calendar)
                }
                _ => {
                    let calendar = match role {
                        AssignmentRole::Work if task.duration.basis() == TimeBasis::Elapsed => {
                            EffectiveCalendar::Elapsed
                        }
                        AssignmentRole::Work | AssignmentRole::Summary => res.calendar,
                        AssignmentRole::Neutral | AssignmentRole::Manual => res.task_calendar,
                    };
                    (planned, late, node.duration, calendar)
                }
            },
        };
        Some(AssignmentSchedule {
            task: res.task,
            resource: res.resource,
            role,
            span,
            late,
            duration,
            work: res.work,
            effective_units,
            calendar,
        })
    }
}

/// Débit d'une affectation : `round_div(W × 10 000, m)` (R-183, R-186) ;
/// `None` hors du domaine des unités.
fn rate(work: Work, m: Minutes) -> Option<Units> {
    let num = i128::from(work.minutes().get()).checked_mul(10_000)?;
    let u = round_div(num, i128::from(m.get()))?;
    Units::new(u).ok()
}

/// Diagnostics d'une affectation de travail d'une tâche active : unités
/// nulles (R-181), profil non uniforme placé comme uniforme (R-180).
fn notices(res: &Resolution, diags: &mut Diagnostics) {
    if res.units == Units::ZERO {
        diags.push(Diagnostic::AssignmentZeroUnits { assignment: res.uid });
    }
    if res.contour != WorkContour::Flat {
        diags.push(Diagnostic::AssignmentContourUnsupported {
            assignment: res.uid,
            contour: res.contour,
        });
    }
}

/// R-173 : mesure de l'union des intervalles travaillés `[s_a, f_a)` des
/// parts, chacun sur son échelle : balayage des bornes ; sur chaque
/// intervalle élémentaire, temps ouvré de l'union des échelles des parts
/// actives (union compilée mise en cache par ensemble d'échelles). `None`
/// si une conversion échoue.
fn union_measure(
    scales: &[TimeScale<'_>],
    parts: &[Part],
    spans: &[Span],
    (full, cache): (Option<(&[u32], TimeScale<'_>)>, &mut UnionCache),
) -> Option<Minutes> {
    let mut bounds: Vec<Moment> = spans.iter().flat_map(|s| [s.start, s.finish]).collect();
    bounds.sort_unstable();
    bounds.dedup();
    let mut total = Minutes::ZERO;
    for w in bounds.windows(2) {
        let (lo, hi) = (*w.first()?, *w.get(1)?);
        let mut active: Vec<u32> = parts
            .iter()
            .zip(spans)
            .filter(|(_, s)| s.start <= lo && s.finish >= hi)
            .map(|(part, _)| part.scale)
            .collect();
        active.sort_unstable();
        active.dedup();
        let m = match active.as_slice() {
            [] => Minutes::ZERO,
            [one] => scales.get(pos(*one)?)?.between(lo, hi).ok()?,
            many => match full {
                Some((all, place)) if all == many => place.between(lo, hi).ok()?,
                _ => measure_union(scales, many, (lo, hi), cache)?,
            },
        };
        total = total.checked_add(m)?;
    }
    Some(total)
}

/// Temps ouvré de `[lo, hi)` sur l'union des échelles `many` ; repli (union
/// trop morcelée, cache plein) : le plus grand des temps ouvrés.
fn measure_union(
    scales: &[TimeScale<'_>],
    many: &[u32],
    (lo, hi): (Moment, Moment),
    cache: &mut UnionCache,
) -> Option<Minutes> {
    let working = |i: u32| match scales.get(pos(i)?)? {
        TimeScale::Working(c) => Some(*c),
        TimeScale::Elapsed => None,
    };
    if !cache.contains_key(many) && cache.len() < UNION_CACHE_LIMIT {
        let mut acc: Option<CompiledCalendar> = None;
        for &i in many {
            let c = working(i);
            acc = match (acc, c) {
                (None, Some(c)) => Some(c.clone()),
                (Some(a), Some(c)) => a.union(c).ok(),
                _ => None,
            };
            if acc.is_none() {
                break;
            }
        }
        cache.insert(many.to_vec(), acc);
    }
    if let Some(Some(u)) = cache.get(many) {
        return u.working_between(lo, hi).ok();
    }
    let mut best = Minutes::ZERO;
    for &i in many {
        best = best.max(scales.get(pos(i)?)?.between(lo, hi).ok()?);
    }
    Some(best)
}
