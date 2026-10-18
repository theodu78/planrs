//! Préparation du réseau (spécification J2 § 3.1 à 3.5) : échelles de temps,
//! hiérarchie (R-052), classification des tâches, nœuds canoniques, liens
//! effectifs et adjacence CSR.
//!
//! Nœuds canoniques, numérotés en un seul parcours en pré-ordre : la porte
//! d'une récapitulative (`Gate`) à son entrée, une feuille à sa position,
//! l'agrégat d'une récapitulative (`Rollup`) à la fermeture de son
//! sous-arbre (le plus intérieur d'abord). Tout arc de hiérarchie va donc
//! vers l'avant (porte → enfants → agrégat) : seuls des liens peuvent
//! remonter la numérotation, ce qui rend la coupure des cycles simple
//! (`order.rs`).
//!
//! Aucune récursion, aucune indexation qui panique : un invariant violé
//! rend `None` et l'appelant le signale (`schedule.internal`).

use std::collections::BTreeMap;
use std::ops::Range;

use crate::calendar::{CalendarSet, CompiledCalendar, Span, TimeScale};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::duration::{LagValue, Minutes, TimeBasis};
use crate::ids::{CalendarUid, TaskUid};
use crate::model::{ConstraintKind, LinkType, Project, SchedulingMode, Task};
use crate::quantity::SignedPercent;
use crate::time::Moment;

/// Indice d'un nœud canonique.
pub(super) type NodeIx = u32;

/// Indice de l'échelle « temps écoulé » dans [`Network::scales`].
pub(super) const ELAPSED: u32 = 0;
/// Indice de l'échelle du calendrier du projet dans [`Network::scales`].
pub(super) const PROJECT: u32 = 1;

/// Échec du calcul d'un nœud : conversion impossible (code stable de
/// l'erreur, R-069) ou invariant interne violé (`schedule.internal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Fail {
    /// Conversion impossible.
    Conversion(&'static str),
    /// Invariant interne violé.
    Internal(&'static str),
}

impl From<crate::error::CalendarError> for Fail {
    fn from(e: crate::error::CalendarError) -> Self {
        Self::Conversion(e.code())
    }
}

impl From<crate::error::DurationError> for Fail {
    fn from(e: crate::error::DurationError) -> Self {
        Self::Conversion(e.code())
    }
}

/// Activité d'une tâche (R-062).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Status {
    /// Active.
    Active,
    /// Inactive.
    Inactive,
    /// Sous une récapitulative inactive.
    InactiveAncestor,
}

/// Rôle d'une ligne (R-052).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    /// Ligne vide.
    Null,
    /// Feuille.
    Leaf,
    /// Récapitulative.
    Summary,
}

/// Classification d'une tâche du modèle, par position.
#[derive(Clone, Debug)]
pub(super) struct TaskInfo {
    /// UID.
    pub uid: TaskUid,
    /// Rôle.
    pub role: Role,
    /// Position du parent (récapitulative), s'il existe.
    pub parent: Option<usize>,
    /// Fin (exclue) du sous-arbre : `position + 1` pour une feuille.
    pub end: usize,
    /// Activité.
    pub status: Status,
    /// Feuille manuelle sans début ni fin (R-063) : aucun nœud.
    pub manual_undated: bool,
    /// Échelle ouvrée de la tâche : calendrier propre utilisable, sinon
    /// calendrier du projet (R-066). Sert au décalage des liens entrants
    /// (R-041) et à la marge d'une récapitulative.
    pub cal: u32,
    /// Nœud d'entrée : la feuille, ou la porte d'une récapitulative.
    pub entry: Option<NodeIx>,
    /// Nœud de sortie : la feuille, ou l'agrégat d'une récapitulative.
    pub exit: Option<NodeIx>,
}

impl TaskInfo {
    /// Inactive ou sous une récapitulative inactive.
    pub fn inactive(&self) -> bool {
        self.status != Status::Active
    }
}

/// Nature d'un nœud canonique.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NodeKind {
    /// Porte d'une récapitulative : borne de début de ses sous-tâches
    /// (R-056).
    Gate,
    /// Feuille.
    Leaf,
    /// Agrégat d'une récapitulative (R-053).
    Rollup,
}

/// Nœud canonique.
#[derive(Clone, Debug)]
pub(super) struct Node {
    /// Nature.
    pub kind: NodeKind,
    /// Position de la tâche dans le modèle.
    pub task: usize,
    /// UID de la tâche.
    pub uid: TaskUid,
    /// Tâche inactive (R-062) : placée pour l'affichage, sans influence.
    pub inactive: bool,
    /// Feuille : échelle de placement `P(t)` (R-030, R-067) ; porte et
    /// agrégat : calendrier de la récapitulative.
    pub place: u32,
    /// Feuille : durée de la tâche ; feuille manuelle : durée utilisée
    /// (R-063) ; agrégat d'une récapitulative manuelle datée : durée
    /// utilisée (R-064) ; autre agrégat, porte : 0.
    pub duration: Minutes,
    /// Feuille manuelle active (R-063) ou agrégat d'une récapitulative
    /// manuelle datée (R-064) : dates saisies, ou l'échec de leur calcul.
    pub fixed: Option<Result<Span, Fail>>,
    /// Contrainte de date effective (R-090 à R-103) : feuille automatique
    /// (active, ou inactive pour son placement d'affichage) à contrainte
    /// datée ; porte et agrégat d'une récapitulative automatique : SNET ou
    /// FNLT seulement (R-102). `None` sinon : ASAP, ALAP (placement, K2),
    /// tâche manuelle ou récapitulative manuelle datée (R-103), contrainte
    /// refusée sur une récapitulative.
    pub constraint: Option<DateConstraint>,
    /// Échéance effective (R-108, R-109) : feuille active (automatique ou
    /// manuelle), agrégat d'une récapitulative active ; `None` sinon
    /// (inactive ignorée).
    pub deadline: Option<Moment>,
    /// Agrégat de la récapitulative parente.
    pub parent_rollup: Option<NodeIx>,
    /// Porte : son agrégat ; agrégat : sa porte.
    pub twin: Option<NodeIx>,
}

/// Contrainte datée d'un nœud : type et date saisie (brute).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DateConstraint {
    /// Type (jamais ASAP ni ALAP).
    pub kind: ConstraintKind,
    /// Date saisie.
    pub date: Moment,
}

impl DateConstraint {
    /// Contrainte de début (SNET, SNLT, MSO) ; sinon de fin (FNET, FNLT,
    /// MFO).
    pub fn on_start(self) -> bool {
        matches!(
            self.kind,
            ConstraintKind::StartNoEarlierThan
                | ConstraintKind::StartNoLaterThan
                | ConstraintKind::MustStartOn
        )
    }

    /// Partie plancher, appliquée en passe avant (SNET, FNET, MSO, MFO).
    pub fn floor(self) -> bool {
        matches!(
            self.kind,
            ConstraintKind::StartNoEarlierThan
                | ConstraintKind::FinishNoEarlierThan
                | ConstraintKind::MustStartOn
                | ConstraintKind::MustFinishOn
        )
    }

    /// Partie plafond, appliquée en passe arrière (SNLT, FNLT, MSO, MFO).
    pub fn ceiling(self) -> bool {
        matches!(
            self.kind,
            ConstraintKind::StartNoLaterThan
                | ConstraintKind::FinishNoLaterThan
                | ConstraintKind::MustStartOn
                | ConstraintKind::MustFinishOn
        )
    }

    /// Date imposée (MSO, MFO) : épinglée quand l'option « toujours
    /// respecter les dates de contrainte » est active (R-097, R-098).
    pub fn must(self) -> bool {
        matches!(
            self.kind,
            ConstraintKind::MustStartOn | ConstraintKind::MustFinishOn
        )
    }
}

/// Contrainte d'une feuille normalisée sur son échelle de placement `P(t)`
/// (R-090).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Normalized {
    /// Contrainte saisie.
    pub c: DateConstraint,
    /// Date effective `D'` : contrainte de début, début de
    /// `P.forward_span(D, d)` (instant ouvré suivant, R-031 pour un jalon) ;
    /// contrainte de fin, `P.backward_span(D, 0).finish` (convention de
    /// jalon arrière : `D` gardée si elle commence une minute ouvrée ou une
    /// plage, sinon l'instant ouvré précédent ; O14). Temps écoulé : `D`.
    pub at: Moment,
    /// Placement de référence : `P.forward_span(D, d)` pour une contrainte de
    /// début ; `(P.backward_span(D', d).start, D')` pour une contrainte de
    /// fin (la fin reste `D'` brute, O14). C'est le placement épinglé (MSO,
    /// MFO, plafond mordant) et la borne de la passe arrière.
    pub span: Span,
}

/// Décalage d'un lien, avant conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LagSpec {
    /// Minutes, signées.
    Time(Minutes),
    /// Pourcentage de la durée du prédécesseur (R-043).
    Percent(SignedPercent),
}

/// Lien effectif porté par un arc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LinkArc {
    /// Type.
    pub ty: LinkType,
    /// Décalage.
    pub lag: LagSpec,
    /// Échelle du décalage `σ` (R-041).
    pub sigma: u32,
    /// Prédécesseur d'origine.
    pub pred: TaskUid,
    /// Successeur d'origine.
    pub succ: TaskUid,
    /// Vers une tâche inactive (R-062) : sert à l'affichage seulement,
    /// ignoré en passe arrière et pour la marge libre.
    pub to_inactive: bool,
}

/// Arc du réseau : hiérarchie (`link == None`) ou lien.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Arc {
    /// Origine.
    pub from: NodeIx,
    /// Cible.
    pub to: NodeIx,
    /// Lien, ou `None` pour un arc de hiérarchie.
    pub link: Option<LinkArc>,
}

/// Réseau prêt pour les passes.
pub(super) struct Network<'a> {
    /// Échelles : `[0]` temps écoulé, `[1]` calendrier du projet, puis les
    /// calendriers de tâche utilisés, dans l'ordre des UID.
    pub scales: Vec<TimeScale<'a>>,
    /// Classification, par position.
    pub tasks: Vec<TaskInfo>,
    /// Nœuds canoniques.
    pub nodes: Vec<Node>,
    /// Arcs triés par cible (tri stable : hiérarchie, puis liens dans
    /// l'ordre MSPDI).
    pub arcs: Vec<Arc>,
    /// Arcs entrants du nœud `v` : `arcs[in_off[v]..in_off[v + 1]]`.
    pub in_off: Vec<u32>,
    /// Arcs sortants du nœud `v` : `out_idx[out_off[v]..out_off[v + 1]]`.
    pub out_off: Vec<u32>,
    /// Indices d'arcs, triés par origine.
    pub out_idx: Vec<u32>,
    /// Arcs coupés pour rompre un cycle (R-065).
    pub cut: Vec<bool>,
}

/// Indice `u32` d'une position (`None` au-delà de `u32::MAX`).
pub(super) fn ix(i: usize) -> Option<u32> {
    u32::try_from(i).ok()
}

/// Position `usize` d'un indice `u32`.
pub(super) fn pos(i: u32) -> Option<usize> {
    usize::try_from(i).ok()
}

impl<'a> Network<'a> {
    /// Nœud `v`.
    pub fn node(&self, v: NodeIx) -> Option<&Node> {
        self.nodes.get(pos(v)?)
    }

    /// Arc `a`.
    pub fn arc(&self, a: u32) -> Option<&Arc> {
        self.arcs.get(pos(a)?)
    }

    /// Arc `a` coupé (R-065).
    pub fn is_cut(&self, a: u32) -> bool {
        pos(a).and_then(|i| self.cut.get(i)).copied().unwrap_or(true)
    }

    /// Échelle `i`.
    pub fn scale(&self, i: u32) -> Option<&TimeScale<'a>> {
        self.scales.get(pos(i)?)
    }

    /// Classification de la tâche en position `p`.
    pub fn task(&self, p: usize) -> Option<&TaskInfo> {
        self.tasks.get(p)
    }

    /// Indices des arcs entrants du nœud `v` (ordre CSR).
    pub fn incoming(&self, v: NodeIx) -> Option<Range<u32>> {
        let i = pos(v)?;
        let a = *self.in_off.get(i)?;
        let b = *self.in_off.get(i.checked_add(1)?)?;
        Some(a..b)
    }

    /// Indices des arcs sortants du nœud `v`.
    pub fn outgoing(&self, v: NodeIx) -> Option<&[u32]> {
        let i = pos(v)?;
        let a = pos(*self.out_off.get(i)?)?;
        let b = pos(*self.out_off.get(i.checked_add(1)?)?)?;
        self.out_idx.get(a..b)
    }

    /// R-090 : contrainte normalisée de la feuille `v`, `None` sans
    /// contrainte datée effective (ou pour un nœud qui n'est pas une
    /// feuille).
    pub fn normalized(&self, v: NodeIx) -> Result<Option<Normalized>, Fail> {
        let node = self.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let Some(c) = node.constraint.filter(|_| node.kind == NodeKind::Leaf) else {
            return Ok(None);
        };
        let scale = self
            .scale(node.place)
            .ok_or(Fail::Internal("échelle hors bornes"))?;
        let (at, span) = if c.on_start() {
            let span = scale.forward_span(c.date, node.duration)?;
            (span.start, span)
        } else {
            let at = scale.backward_span(c.date, Minutes::ZERO)?.finish;
            let start = scale.backward_span(at, node.duration)?.start;
            (at, Span { start, finish: at })
        };
        Ok(Some(Normalized { c, at, span }))
    }

    /// Agrégat d'une récapitulative manuelle datée (R-064) : ses dates ne
    /// dépendent pas de ses enfants, qui n'héritent donc ni de ses liens
    /// sortants ni de ceux de ses ancêtres.
    pub fn is_fixed_rollup(&self, v: NodeIx) -> bool {
        self.node(v)
            .is_some_and(|n| n.kind == NodeKind::Rollup && n.fixed.is_some())
    }

    /// Nombre de nœuds.
    pub fn node_count(&self) -> Option<u32> {
        ix(self.nodes.len())
    }
}

/// Calendriers résolus pour le calcul (§ 3.1, R-066).
pub(super) struct Scales<'a> {
    /// Échelles (voir [`Network::scales`]).
    pub scales: Vec<TimeScale<'a>>,
    /// Échelle ouvrée de chaque tâche, par position.
    pub task_cal: Vec<u32>,
}

/// Table des échelles : temps écoulé, calendrier du projet (déjà résolu,
/// replis compris), puis un calendrier compilé par UID de tâche utilisé.
/// Un calendrier de tâche inutilisable (inconnu, en échec, sans temps ouvré)
/// est remplacé par celui du projet, avec `task.calendar_fallback`.
pub(super) fn scales<'a>(
    project: &Project,
    set: &'a CalendarSet,
    project_cal: &'a CompiledCalendar,
    diags: &mut Diagnostics,
) -> Scales<'a> {
    let mut scales = vec![TimeScale::Elapsed, TimeScale::Working(project_cal)];
    let mut index: BTreeMap<CalendarUid, Option<u32>> = BTreeMap::new();
    let mut task_cal = Vec::with_capacity(project.tasks.len());
    for task in &project.tasks {
        let Some(uid) = task.calendar.filter(|_| !task.is_null) else {
            task_cal.push(PROJECT);
            continue;
        };
        let slot = *index.entry(uid).or_insert_with(|| match set.get(uid) {
            Ok(c) if c.has_working_time() => {
                let i = ix(scales.len());
                if i.is_some() {
                    scales.push(TimeScale::Working(c));
                }
                i
            }
            _ => None,
        });
        if let Some(i) = slot {
            task_cal.push(i);
        } else {
            diags.push(Diagnostic::TaskCalendarFallback {
                task: task.uid(),
                calendar: uid,
            });
            task_cal.push(PROJECT);
        }
    }
    Scales { scales, task_cal }
}

/// Étapes 1 et 2 : hiérarchie (R-052) et activité (R-062). Le parent est la
/// dernière tâche non vide de niveau strictement inférieur ; une tâche est
/// récapitulative si une tâche la désigne comme parent (ce qui équivaut à
/// « la tâche non vide suivante a un niveau supérieur »). Le drapeau
/// `Summary` de MSPDI n'est pas lu.
fn classify(project: &Project, task_cal: &[u32], diags: &mut Diagnostics) -> Option<Vec<TaskInfo>> {
    let n = project.tasks.len();
    let mut tasks: Vec<TaskInfo> = Vec::with_capacity(n);
    // Pile des tâches ouvertes : (niveau, position).
    let mut stack: Vec<(u16, usize)> = Vec::new();
    for (p, task) in project.tasks.iter().enumerate() {
        let cal = task_cal.get(p).copied().unwrap_or(PROJECT);
        let mut info = TaskInfo {
            uid: task.uid(),
            role: Role::Null,
            parent: None,
            end: p.checked_add(1)?,
            status: Status::Active,
            manual_undated: false,
            cal,
            entry: None,
            exit: None,
        };
        if !task.is_null {
            let level = task.outline_level.get();
            while let Some(&(l, top)) = stack.last() {
                if l < level {
                    break;
                }
                stack.pop();
                tasks.get_mut(top)?.end = p;
            }
            info.role = Role::Leaf;
            if let Some(&(_, parent)) = stack.last() {
                info.parent = Some(parent);
                let par = tasks.get_mut(parent)?;
                par.role = Role::Summary;
            }
            stack.push((level, p));
        }
        tasks.push(info);
    }
    for (_, top) in stack {
        tasks.get_mut(top)?.end = n;
    }
    // Activité et tâches manuelles sans date, parents avant enfants.
    for p in 0..n {
        let task = project.tasks.at(p)?;
        let parent_status = match tasks.get(p)?.parent {
            Some(par) => Some(tasks.get(par)?.status),
            None => None,
        };
        let info = tasks.get_mut(p)?;
        if info.role == Role::Null {
            continue;
        }
        info.status = match parent_status {
            Some(s) if s != Status::Active => Status::InactiveAncestor,
            _ if !task.active => Status::Inactive,
            _ => Status::Active,
        };
        if info.role == Role::Leaf
            && info.status == Status::Active
            && let SchedulingMode::Manual {
                start: None,
                finish: None,
                ..
            } = task.mode
        {
            info.manual_undated = true;
            diags.push(Diagnostic::TaskManualUndated { task: task.uid() });
        }
    }
    Some(tasks)
}

/// Échelle de placement `P(t)` d'une feuille : temps écoulé pour une durée
/// écoulée (R-067), sinon le calendrier de la tâche.
fn place_of(task: &Task, cal: u32) -> u32 {
    if task.duration.basis() == TimeBasis::Elapsed {
        ELAPSED
    } else {
        cal
    }
}

/// R-063, R-064 : dates d'une feuille (ou d'une récapitulative) manuelle et
/// durée utilisée. Les dates saisies
/// sont conservées sans recalage ; une seule saisie : l'autre par `P.add`
/// avec la durée saisie, sinon la durée de la tâche ; deux saisies : la
/// durée utilisée est le temps entre elles sur `P` ; fin avant début :
/// `task.manual_dates_inverted` et fin ramenée au début.
fn manual_span(
    task: &Task,
    scale: TimeScale<'_>,
    diags: &mut Diagnostics,
) -> (Option<Result<Span, Fail>>, Minutes) {
    let SchedulingMode::Manual {
        start,
        finish,
        duration,
    } = task.mode
    else {
        return (None, task.duration.minutes());
    };
    let entered = duration.map_or(task.duration.minutes(), |d| d.minutes());
    let span = |s: Moment, f: Moment| Span { start: s, finish: f };
    match (start, finish) {
        (Some(s), Some(f)) => {
            let f = if f < s {
                diags.push(Diagnostic::TaskManualDatesInverted { task: task.uid() });
                s
            } else {
                f
            };
            match scale.between(s, f) {
                Ok(d) => (Some(Ok(span(s, f))), d),
                Err(e) => (Some(Err(e.into())), entered),
            }
        }
        (Some(s), None) => (
            Some(scale.add(s, entered).map(|f| span(s, f)).map_err(Fail::from)),
            entered,
        ),
        (None, Some(f)) => {
            let back = entered.checked_neg().unwrap_or(Minutes::ZERO);
            (
                Some(scale.add(f, back).map(|s| span(s, f)).map_err(Fail::from)),
                entered,
            )
        }
        // Exclue plus haut (`ManualUndated`).
        (None, None) => (Some(Err(Fail::Internal("manuelle sans date"))), entered),
    }
}

/// Récapitulative ouverte pendant la numérotation : position, dates
/// saisies et durée utilisée d'une récapitulative manuelle datée (R-064),
/// échéance effective (R-109).
type OpenSummary = (usize, Option<Result<Span, Fail>>, Minutes, Option<Moment>);

/// R-102, R-103 : contrainte effective d'une récapitulative. Récapitulative
/// manuelle datée : ignorée sans diagnostic ; sinon SNET (porte) et FNLT
/// (plafond de ses feuilles) gardées, MSO, MFO, SNLT, FNET et ALAP
/// remplacées par ASAP avec `task.constraint_on_summary` (active
/// seulement : une inactive n'a aucun diagnostic).
fn summary_constraint(
    task: &Task,
    fixed: bool,
    inactive: bool,
    diags: &mut Diagnostics,
) -> Option<DateConstraint> {
    if fixed {
        return None;
    }
    let kind = task.constraint.kind();
    match (kind, task.constraint.date()) {
        (ConstraintKind::AsSoonAsPossible, _) => None,
        (ConstraintKind::StartNoEarlierThan | ConstraintKind::FinishNoLaterThan, Some(date)) => {
            Some(DateConstraint { kind, date })
        }
        _ => {
            if !inactive {
                diags.push(Diagnostic::TaskConstraintOnSummary {
                    task: task.uid(),
                    constraint: kind,
                });
            }
            None
        }
    }
}

/// R-093 à R-098, R-103 : contrainte effective d'une feuille. Manuelle
/// (même inactive) : ignorée sans diagnostic ; ASAP, ALAP : aucune.
fn leaf_constraint(task: &Task) -> Option<DateConstraint> {
    if matches!(task.mode, SchedulingMode::Manual { .. }) {
        return None;
    }
    task.constraint.date().map(|date| DateConstraint {
        kind: task.constraint.kind(),
        date,
    })
}

/// Étape 3 : nœuds canoniques en pré-ordre (porte, feuille, agrégat).
fn number_nodes(
    project: &Project,
    tasks: &mut [TaskInfo],
    scales: &[TimeScale<'_>],
    diags: &mut Diagnostics,
) -> Option<Vec<Node>> {
    let mut nodes: Vec<Node> = Vec::with_capacity(tasks.len().checked_add(tasks.len() / 4)?);
    // Récapitulatives ouvertes, la plus intérieure en dernier.
    let mut open: Vec<OpenSummary> = Vec::new();
    let n = tasks.len();
    // L'agrégat reprend la porte (tâche, UID, activité, calendrier,
    // contrainte) avec ses propres dates saisies, durée et échéance.
    let close = |(s, fixed, duration, deadline): OpenSummary,
                 tasks: &mut [TaskInfo],
                 nodes: &mut Vec<Node>|
     -> Option<()> {
        let info = tasks.get_mut(s)?;
        let gate = info.entry?;
        let v = ix(nodes.len())?;
        let mut rollup = nodes.get(pos(gate)?)?.clone();
        rollup.kind = NodeKind::Rollup;
        (rollup.duration, rollup.fixed, rollup.deadline) = (duration, fixed, deadline);
        rollup.twin = Some(gate);
        nodes.push(rollup);
        info.exit = Some(v);
        nodes.get_mut(pos(gate)?)?.twin = Some(v);
        Some(())
    };
    for p in 0..=n {
        while let Some(&(s, ..)) = open.last() {
            if tasks.get(s)?.end > p {
                break;
            }
            close(open.pop()?, tasks, &mut nodes)?;
        }
        if p == n {
            break;
        }
        let task = project.tasks.at(p)?;
        let info = tasks.get_mut(p)?;
        if info.role == Role::Null || info.manual_undated {
            continue;
        }
        let v = ix(nodes.len())?;
        if info.role == Role::Summary {
            // R-064 : une récapitulative manuelle active avec au moins une
            // date saisie garde ses dates (comme une feuille manuelle, sur
            // son calendrier) ; sans date, elle est automatique.
            let dated = matches!(
                task.mode,
                SchedulingMode::Manual { start, finish, .. } if start.is_some() || finish.is_some()
            );
            let (fixed, duration) = if dated && !info.inactive() {
                manual_span(task, *scales.get(pos(info.cal)?)?, diags)
            } else {
                (None, Minutes::ZERO)
            };
            let deadline = task.deadline.filter(|_| !info.inactive());
            nodes.push(Node {
                kind: NodeKind::Gate,
                task: p,
                uid: info.uid,
                inactive: info.inactive(),
                place: info.cal,
                duration: Minutes::ZERO,
                fixed: None,
                constraint: summary_constraint(task, fixed.is_some(), info.inactive(), diags),
                deadline: None,
                parent_rollup: None,
                twin: None,
            });
            info.entry = Some(v);
            open.push((p, fixed, duration, deadline));
        } else {
            let place = place_of(task, info.cal);
            let (fixed, duration) = if info.inactive() {
                (None, task.duration.minutes())
            } else {
                manual_span(task, *scales.get(pos(place)?)?, diags)
            };
            nodes.push(Node {
                kind: NodeKind::Leaf,
                task: p,
                uid: info.uid,
                inactive: info.inactive(),
                place,
                duration,
                fixed,
                constraint: leaf_constraint(task),
                deadline: task.deadline.filter(|_| !info.inactive()),
                parent_rollup: None,
                twin: None,
            });
            info.entry = Some(v);
            info.exit = Some(v);
        }
    }
    // Agrégat du parent de chaque nœud de feuille ou de récapitulative.
    for node in &mut nodes {
        if node.kind == NodeKind::Gate {
            continue;
        }
        if let Some(par) = tasks.get(node.task)?.parent {
            node.parent_rollup = tasks.get(par)?.exit;
        }
    }
    Some(nodes)
}

/// Contexte de construction des arcs.
struct ArcBuilder<'p> {
    project: &'p Project,
    arcs: Vec<Arc>,
}

impl ArcBuilder<'_> {
    /// Arcs de hiérarchie : porte du parent → entrée de l'enfant ; sortie de
    /// l'enfant → agrégat du parent, si l'enfant a la même activité que le
    /// parent (une tâche inactive reste hors des agrégats actifs, R-062).
    fn hierarchy(&mut self, tasks: &[TaskInfo]) -> Option<()> {
        for info in tasks {
            let Some(par) = info.parent else { continue };
            let (Some(entry), Some(exit)) = (info.entry, info.exit) else {
                continue;
            };
            let parent = tasks.get(par)?;
            self.arcs.push(Arc {
                from: parent.entry?,
                to: entry,
                link: None,
            });
            if info.inactive() == parent.inactive() {
                self.arcs.push(Arc {
                    from: exit,
                    to: parent.exit?,
                    link: None,
                });
            }
        }
        Some(())
    }

    /// Étape 4 : liens effectifs (§ 3.5), dans l'ordre des successeurs puis
    /// l'ordre MSPDI.
    fn links(&mut self, tasks: &[TaskInfo], diags: &mut Diagnostics) -> Option<()> {
        // Doublons : position du dernier successeur qui a cité chaque
        // prédécesseur (aucune allocation par tâche).
        let mut seen_by: Vec<usize> = vec![usize::MAX; tasks.len()];
        for (s_pos, succ_task) in self.project.tasks.iter().enumerate() {
            let succ = tasks.get(s_pos)?;
            for link in &succ_task.predecessors {
                let p_uid = link.predecessor;
                let s_uid = succ.uid;
                // 1. Inconnu, auto-lien, doublon (A17).
                let Some(p_pos) = self.project.tasks.position(p_uid) else {
                    diags.push(Diagnostic::LinkUnknownTask {
                        predecessor: p_uid,
                        successor: s_uid,
                    });
                    continue;
                };
                if p_uid == s_uid {
                    diags.push(Diagnostic::LinkSelf { task: s_uid });
                    continue;
                }
                let seen = seen_by.get_mut(p_pos)?;
                if *seen == s_pos {
                    diags.push(Diagnostic::LinkDuplicate {
                        predecessor: p_uid,
                        successor: s_uid,
                    });
                    continue;
                }
                *seen = s_pos;
                let pred = tasks.get(p_pos)?;
                let excluded = |t: &TaskInfo| t.role == Role::Null || t.manual_undated;
                // 2. Extrémité non planifiée.
                if excluded(pred) || excluded(succ) {
                    diags.push(Diagnostic::LinkExcludedEndpoint {
                        predecessor: p_uid,
                        successor: s_uid,
                    });
                    continue;
                }
                // 3. Prédécesseur inactif vers un successeur actif : aucun
                //    arc, aucun pont (R-062). Entre deux tâches inactives
                //    (par exemple dans une récapitulative inactive), l'arc
                //    est gardé pour le placement d'affichage, marqué
                //    `to_inactive`.
                if pred.inactive() && !succ.inactive() {
                    diags.push(Diagnostic::LinkInactive {
                        predecessor: p_uid,
                        successor: s_uid,
                    });
                    continue;
                }
                // 4. Récapitulative et descendante (R-059).
                let contains = |anc_pos: usize, anc: &TaskInfo, q: usize| anc_pos < q && q < anc.end;
                if contains(p_pos, pred, s_pos) || contains(s_pos, succ, p_pos) {
                    diags.push(Diagnostic::LinkSummaryDescendant {
                        predecessor: p_uid,
                        successor: s_uid,
                    });
                    continue;
                }
                // 5. Origine : la sortie du prédécesseur (agrégat ou feuille).
                let from = pred.exit?;
                let lag = match link.lag.value {
                    LagValue::Time { minutes, .. } => LagSpec::Time(minutes),
                    LagValue::Percent { percent } => LagSpec::Percent(percent),
                };
                let sigma = match link.lag.basis {
                    TimeBasis::Elapsed => ELAPSED,
                    TimeBasis::Working => succ.cal,
                };
                let arc = |to: NodeIx, to_inactive: bool| Arc {
                    from,
                    to,
                    link: Some(LinkArc {
                        ty: link.kind,
                        lag,
                        sigma,
                        pred: p_uid,
                        succ: s_uid,
                        to_inactive,
                    }),
                };
                // 6. Cible : la porte pour FS/SS vers une récapitulative ;
                //    dépliage sur les feuilles pour FF/SF (R-058).
                let finish_kind = matches!(link.kind, LinkType::FinishToFinish | LinkType::StartToFinish);
                if succ.role == Role::Summary && finish_kind {
                    diags.push(Diagnostic::LinkFinishToSummary {
                        predecessor: p_uid,
                        successor: s_uid,
                    });
                    let first = s_pos.checked_add(1)?;
                    for q in first..succ.end {
                        let leaf = tasks.get(q)?;
                        if leaf.role != Role::Leaf || (leaf.inactive() && !succ.inactive()) {
                            continue;
                        }
                        if let Some(v) = leaf.entry {
                            self.arcs.push(arc(v, leaf.inactive()));
                        }
                    }
                } else {
                    self.arcs.push(arc(succ.entry?, succ.inactive()));
                }
            }
        }
        Some(())
    }
}

/// Adjacence CSR : arcs triés par cible, index entrant et sortant.
struct Csr {
    arcs: Vec<Arc>,
    in_off: Vec<u32>,
    out_off: Vec<u32>,
    out_idx: Vec<u32>,
}

/// Tri par comptage stable des arcs par cible ; index sortant.
fn csr(node_count: usize, arcs: Vec<Arc>) -> Option<Csr> {
    let mut in_off = vec![0u32; node_count.checked_add(1)?];
    for a in &arcs {
        let slot = in_off.get_mut(pos(a.to)?.checked_add(1)?)?;
        *slot = slot.checked_add(1)?;
    }
    for i in 1..in_off.len() {
        let prev = *in_off.get(i.checked_sub(1)?)?;
        let cur = in_off.get_mut(i)?;
        *cur = cur.checked_add(prev)?;
    }
    let mut fill = in_off.clone();
    let mut sorted: Vec<Option<Arc>> = vec![None; arcs.len()];
    for a in arcs {
        let slot = fill.get_mut(pos(a.to)?)?;
        *sorted.get_mut(pos(*slot)?)? = Some(a);
        *slot = slot.checked_add(1)?;
    }
    let sorted: Vec<Arc> = sorted.into_iter().collect::<Option<Vec<Arc>>>()?;
    let mut out_off = vec![0u32; node_count.checked_add(1)?];
    for a in &sorted {
        let slot = out_off.get_mut(pos(a.from)?.checked_add(1)?)?;
        *slot = slot.checked_add(1)?;
    }
    for i in 1..out_off.len() {
        let prev = *out_off.get(i.checked_sub(1)?)?;
        let cur = out_off.get_mut(i)?;
        *cur = cur.checked_add(prev)?;
    }
    let mut fill = out_off.clone();
    let mut out_idx = vec![0u32; sorted.len()];
    for (k, a) in sorted.iter().enumerate() {
        let slot = fill.get_mut(pos(a.from)?)?;
        *out_idx.get_mut(pos(*slot)?)? = ix(k)?;
        *slot = slot.checked_add(1)?;
    }
    Some(Csr {
        arcs: sorted,
        in_off,
        out_off,
        out_idx,
    })
}

/// Étapes 1 à 4 : réseau complet. `None` si un invariant est violé (jamais
/// attendu) ; l'appelant le signale.
pub(super) fn build<'a>(project: &Project, sc: Scales<'a>, diags: &mut Diagnostics) -> Option<Network<'a>> {
    let mut tasks = classify(project, &sc.task_cal, diags)?;
    let nodes = number_nodes(project, &mut tasks, &sc.scales, diags)?;
    let mut builder = ArcBuilder {
        project,
        arcs: Vec::new(),
    };
    builder.hierarchy(&tasks)?;
    builder.links(&tasks, diags)?;
    let arc_count = builder.arcs.len();
    let c = csr(nodes.len(), builder.arcs)?;
    Some(Network {
        scales: sc.scales,
        tasks,
        nodes,
        arcs: c.arcs,
        in_off: c.in_off,
        out_off: c.out_off,
        out_idx: c.out_idx,
        cut: vec![false; arc_count],
    })
}
