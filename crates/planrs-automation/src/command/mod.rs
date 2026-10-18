//! Commandes sérialisées (contrat J4, ADR-0012 D5, D6).
//!
//! Format versionné unique, partagé par `planrs apply`, le MCP, le canal de
//! contrôle et le journal :
//! `{"format_version":1,"lang":"fr","ops":[{"op":"task.add",…}]}`, champs
//! inconnus refusés. Les tâches sont désignées par UID (entier JSON) ou par
//! une étiquette `"$x"` créée par un `task.add` antérieur du même lot
//! (R-150, lot L3). Dates en ISO local (`"2026-11-02T08:00"`).
//!
//! Propriétaires : types et schémas, L3 ; exécution, L4 (`task.rs`) et L5
//! (`link.rs`, `constraint.rs`, `project.rs`, `calendar.rs`) ; contexte
//! d'exécution (`exec.rs`), L2.

mod calendar;
mod constraint;
pub(crate) mod exec;
mod link;
mod project;
mod task;

use std::borrow::Cow;

use planrs_core::{CalendarUid, Day, Moment, TaskUid, TimeOfDay};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use crate::json::{ConstraintKindJson, DurationJson, DurationUnitJson, LagJson, LinkTypeJson};
use crate::text::Language;

/// Lot de commandes : une transaction, un recalcul, une entrée d'historique
/// (ADR-0012 D2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    /// Version du format (1).
    pub format_version: u32,
    /// Langue des textes (durées) ; à défaut, celle de la session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<Language>,
    /// Opérations, dans l'ordre.
    pub ops: Vec<Command>,
    /// Libellé de l'entrée d'historique.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Commande d'écriture. Le nom JSON (`"op"`) est l'id du registre.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op")]
#[non_exhaustive]
pub enum Command {
    /// Ajoute une tâche (R-152).
    #[serde(rename = "task.add")]
    TaskAdd(TaskAdd),
    /// Modifie une tâche (R-157, R-158).
    #[serde(rename = "task.update")]
    TaskUpdate(TaskUpdate),
    /// Supprime une tâche (R-155).
    #[serde(rename = "task.delete")]
    TaskDelete(TaskDelete),
    /// Déplace un sous-arbre (R-154).
    #[serde(rename = "task.move")]
    TaskMove(TaskMove),
    /// Indente (R-153).
    #[serde(rename = "task.indent")]
    TaskIndent(TaskOne),
    /// Désindente (R-153).
    #[serde(rename = "task.outdent")]
    TaskOutdent(TaskOne),
    /// Ajoute un lien (R-156).
    #[serde(rename = "link.add")]
    LinkAdd(LinkSpec),
    /// Modifie un lien (R-156).
    #[serde(rename = "link.update")]
    LinkUpdate(LinkSpec),
    /// Retire un lien (R-156).
    #[serde(rename = "link.remove")]
    LinkRemove(LinkKey),
    /// Pose une contrainte (R-159).
    #[serde(rename = "task.set_constraint")]
    TaskSetConstraint(SetConstraint),
    /// Pose ou retire une échéance (R-159).
    #[serde(rename = "task.set_deadline")]
    TaskSetDeadline(SetDeadline),
    /// Modifie les informations du projet.
    #[serde(rename = "project.set_info")]
    ProjectSetInfo(ProjectInfoPatch),
    /// Change le sens et la date pilote du projet.
    #[serde(rename = "project.set_anchor")]
    ProjectSetAnchor(AnchorSpec),
    /// Pose ou retire la date d'état.
    #[serde(rename = "project.set_status_date")]
    ProjectSetStatusDate(SetStatusDate),
    /// Crée ou remplace une exception de calendrier (R-165).
    #[serde(rename = "calendar.set_exception")]
    CalendarSetException(CalendarException),
    /// Retire une exception de calendrier (R-165).
    #[serde(rename = "calendar.remove_exception")]
    CalendarRemoveException(CalendarExceptionKey),
}

/// Désignation d'une tâche (R-150). JSON : entier → UID ; chaîne `"$x"` →
/// étiquette. Le refus d'une chaîne sans `$` est écrit par le lot L3.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "TaskRefRepr", into = "TaskRefRepr")]
pub enum TaskRef {
    /// UID.
    Uid(TaskUid),
    /// Étiquette `$x` d'une tâche créée plus tôt dans le lot.
    Label(String),
}

/// Forme JSON de [`TaskRef`].
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum TaskRefRepr {
    Uid(u32),
    Label(String),
}

impl From<TaskRefRepr> for TaskRef {
    fn from(r: TaskRefRepr) -> Self {
        match r {
            TaskRefRepr::Uid(u) => Self::Uid(TaskUid::new(u)),
            TaskRefRepr::Label(s) => Self::Label(s),
        }
    }
}

impl From<TaskRef> for TaskRefRepr {
    fn from(r: TaskRef) -> Self {
        match r {
            TaskRef::Uid(u) => Self::Uid(u.get()),
            TaskRef::Label(s) => Self::Label(s),
        }
    }
}

impl JsonSchema for TaskRef {
    fn schema_name() -> Cow<'static, str> {
        "TaskRef".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "oneOf": [
                { "type": "integer", "minimum": 0, "maximum": 4_294_967_295_u32 },
                { "type": "string", "pattern": "^\\$[A-Za-z0-9_-]{1,64}$" }
            ]
        })
    }
}

/// Placement d'une tâche ajoutée ou déplacée (R-152, R-154).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    /// En dernière position, niveau 1.
    #[default]
    End,
    /// Après le sous-arbre de la tâche, au même niveau.
    After(TaskRef),
    /// Juste avant la tâche, au même niveau.
    Before(TaskRef),
    /// Premier enfant de la tâche.
    FirstChildOf(TaskRef),
    /// Dernier enfant de la tâche.
    LastChildOf(TaskRef),
}

/// Modification d'un champ : absent (garder), valeur (poser), `null`
/// (effacer). Les champs `Patch` portent `#[serde(default)]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Patch<T> {
    /// Champ absent : valeur gardée.
    #[default]
    Keep,
    /// Nouvelle valeur.
    Set(T),
    /// `null` : valeur effacée.
    Clear,
}

impl<T> Patch<T> {
    /// Vrai pour [`Patch::Keep`] (champ omis à l'écriture).
    #[must_use]
    pub const fn is_keep(&self) -> bool {
        matches!(self, Self::Keep)
    }
}

impl<T: Serialize> Serialize for Patch<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Set(v) => v.serialize(s),
            Self::Keep | Self::Clear => s.serialize_none(),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Option::<T>::deserialize(d).map(|v| v.map_or(Self::Clear, Self::Set))
    }
}

impl<T: JsonSchema> JsonSchema for Patch<T> {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        format!("Patch_{}", T::schema_name()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        Option::<T>::json_schema(generator)
    }
}

/// `task.add`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskAdd {
    /// Nom.
    pub name: String,
    /// Étiquette `$x` de la tâche créée, pour la suite du lot.
    #[serde(rename = "as", default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Placement (défaut : fin).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Placement>,
    /// Durée (défaut : celle du projet, estimée).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<DurationJson>,
    /// Jalon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub milestone: Option<bool>,
    /// Notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Début saisi (→ contrainte, R-158).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub start: Option<Moment>,
    /// Fin saisie (→ contrainte, R-158).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub finish: Option<Moment>,
}

/// `task.update`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskUpdate {
    /// Tâche.
    pub uid: TaskRef,
    /// Nom (`null` refusé).
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub name: Patch<String>,
    /// Durée.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub duration: Patch<DurationJson>,
    /// Jalon.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub milestone: Patch<bool>,
    /// Notes.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub notes: Patch<String>,
    /// Début saisi (R-158).
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    #[schemars(with = "Patch<String>")]
    pub start: Patch<Moment>,
    /// Fin saisie (R-158).
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    #[schemars(with = "Patch<String>")]
    pub finish: Patch<Moment>,
    /// Tâche active.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub active: Patch<bool>,
}

/// `task.delete`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskDelete {
    /// Tâche.
    pub uid: TaskRef,
    /// Supprimer aussi les descendants (obligatoire pour une récapitulative).
    #[serde(default)]
    pub cascade: bool,
}

/// `task.move`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskMove {
    /// Tâche (avec son sous-arbre).
    pub uid: TaskRef,
    /// Destination.
    pub to: Placement,
}

/// Commande à une seule tâche (`task.indent`, `task.outdent`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskOne {
    /// Tâche.
    pub uid: TaskRef,
}

/// `link.add`, `link.update`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkSpec {
    /// Prédécesseur.
    pub pred: TaskRef,
    /// Successeur.
    pub succ: TaskRef,
    /// Type (défaut FS à l'ajout ; gardé à la modification).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<LinkTypeJson>,
    /// Décalage (défaut 0 à l'ajout ; gardé à la modification).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lag: Option<LagJson>,
}

/// `link.remove`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkKey {
    /// Prédécesseur.
    pub pred: TaskRef,
    /// Successeur.
    pub succ: TaskRef,
}

/// `task.set_constraint`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetConstraint {
    /// Tâche.
    pub uid: TaskRef,
    /// Type.
    pub kind: ConstraintKindJson,
    /// Date (obligatoire pour les types datés, interdite sinon).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub date: Option<Moment>,
}

/// `task.set_deadline`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetDeadline {
    /// Tâche.
    pub uid: TaskRef,
    /// Échéance (`null` : retirée).
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    #[schemars(with = "Patch<String>")]
    pub deadline: Patch<Moment>,
}

/// `project.set_status_date`. Struct plutôt que `Patch<Moment>` nu : une
/// variante d'enum à étiquette interne doit contenir un objet JSON.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetStatusDate {
    /// Date d'état (`null` : NA).
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    #[schemars(with = "Patch<String>")]
    pub date: Patch<Moment>,
}

/// `project.set_info`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectInfoPatch {
    /// Nom.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub name: Patch<String>,
    /// Titre.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub title: Patch<String>,
    /// Objet.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub subject: Patch<String>,
    /// Auteur.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub author: Patch<String>,
    /// Responsable.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub manager: Patch<String>,
    /// Société.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub company: Patch<String>,
    /// Catégorie.
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub category: Patch<String>,
}

/// Sens de planification et date pilote. JSON : `{"start":"…"}` ou
/// `{"finish":"…"}` (un objet, pour l'étiquette interne de [`Command`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "AnchorRepr", into = "AnchorRepr")]
pub enum AnchorSpec {
    /// Depuis la date de début.
    Start(Moment),
    /// À rebours depuis la date de fin.
    Finish(Moment),
}

/// Forme JSON de [`AnchorSpec`] : exactement un des deux champs.
#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AnchorRepr {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    start: Option<Moment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    finish: Option<Moment>,
}

impl TryFrom<AnchorRepr> for AnchorSpec {
    type Error = &'static str;

    fn try_from(r: AnchorRepr) -> Result<Self, Self::Error> {
        match (r.start, r.finish) {
            (Some(s), None) => Ok(Self::Start(s)),
            (None, Some(f)) => Ok(Self::Finish(f)),
            _ => Err("exactement un de « start » ou « finish » attendu"),
        }
    }
}

impl From<AnchorSpec> for AnchorRepr {
    fn from(a: AnchorSpec) -> Self {
        match a {
            AnchorSpec::Start(s) => Self {
                start: Some(s),
                finish: None,
            },
            AnchorSpec::Finish(f) => Self {
                start: None,
                finish: Some(f),
            },
        }
    }
}

/// Calendrier désigné : UID (entier JSON) ou `"project"` (calendrier du
/// projet).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "CalendarRefRepr", into = "CalendarRefRepr")]
pub enum CalendarRef {
    /// Calendrier du projet.
    Project,
    /// UID.
    Uid(CalendarUid),
}

/// Forme JSON de [`CalendarRef`].
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum CalendarRefRepr {
    Uid(u32),
    Name(String),
}

impl TryFrom<CalendarRefRepr> for CalendarRef {
    type Error = &'static str;

    fn try_from(r: CalendarRefRepr) -> Result<Self, Self::Error> {
        match r {
            CalendarRefRepr::Uid(u) => Ok(Self::Uid(CalendarUid::new(u))),
            CalendarRefRepr::Name(s) if s == "project" => Ok(Self::Project),
            CalendarRefRepr::Name(_) => Err("UID de calendrier ou « project » attendu"),
        }
    }
}

impl From<CalendarRef> for CalendarRefRepr {
    fn from(c: CalendarRef) -> Self {
        match c {
            CalendarRef::Project => Self::Name("project".to_owned()),
            CalendarRef::Uid(u) => Self::Uid(u.get()),
        }
    }
}

impl JsonSchema for CalendarRef {
    fn schema_name() -> Cow<'static, str> {
        "CalendarRef".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "oneOf": [
                { "type": "integer", "minimum": 0, "maximum": 4_294_967_295_u32 },
                { "const": "project" }
            ]
        })
    }
}

/// `calendar.set_exception` (R-165) : jours `[from, to]` chômés
/// (`ranges` vide) ou ouvrés sur les plages données.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarException {
    /// Calendrier.
    pub cal: CalendarRef,
    /// Premier jour.
    #[schemars(with = "String")]
    pub from: Day,
    /// Dernier jour (inclus).
    #[schemars(with = "String")]
    pub to: Day,
    /// Plages ouvrées `["08:00","12:00"]` ; vide : jours chômés.
    #[serde(default)]
    #[schemars(with = "Vec<[String; 2]>")]
    pub ranges: Vec<[TimeOfDay; 2]>,
}

/// `calendar.remove_exception` (R-165).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarExceptionKey {
    /// Calendrier.
    pub cal: CalendarRef,
    /// Premier jour de l'exception.
    #[schemars(with = "String")]
    pub from: Day,
}
