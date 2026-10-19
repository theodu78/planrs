//! Registre des commandes (ADR-0012 D7) : table triée par id, source unique
//! des outils MCP, des commandes de la CLI, du canal et des raccourcis de
//! l'interface.
//!
//! Entrées J4 (disponibles, listées) et entrées réservées aux jalons J5-J6
//! (non listées, refusées `command.not_yet_available`). Le schéma de chaque
//! entrée est celui de ses paramètres (JSON Schema 2020-12, sous-schémas en
//! ligne, sans `$ref`) ; les surfaces y ajoutent `dry_run` ou `detail`.
//! Schémas de référence : `tests/golden/schemas/<id>.json`.

use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema, json_schema};

use crate::command::params::{
    ApplyBatchParams, ExportMspdiParams, ImportCsvParams, NewProjectParams, NoParams, OpenProjectParams,
    RenderGanttParams, SaveProjectParams, TaskGetParams, TaskListParams,
};
use crate::command::{
    AnchorSpec, CalendarException, CalendarExceptionKey, LinkKey, LinkSpec, ProjectInfoPatch, SetConstraint,
    SetDeadline, SetStatusDate, TaskAdd, TaskDelete, TaskMove, TaskOne, TaskUpdate,
};
use crate::text::Language;

/// Genre d'une commande.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Lecture.
    Read,
    /// Écriture (lot, annulable).
    Write,
    /// Analyse (lecture calculée).
    Analysis,
    /// Historique (annuler, rétablir, lister).
    History,
    /// Fichiers (ouvrir, enregistrer, importer).
    File,
}

/// Disponibilité d'une commande.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    /// Disponible et listée.
    Now,
    /// Réservée au jalon donné (5 ou 6), non listée.
    Milestone(u8),
}

/// Entrée du registre.
#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    /// Id à point (`task.add`).
    pub id: &'static str,
    /// Nom de l'outil MCP (`add_task`, motif `^[a-zA-Z0-9_-]{1,64}$`).
    pub mcp_name: &'static str,
    /// Genre.
    pub kind: Kind,
    /// Accepte `dry_run`.
    pub dry_run: bool,
    /// Crée une entrée d'historique.
    pub undoable: bool,
    /// Disponibilité.
    pub available: Availability,
    /// Première version du format qui la connaît.
    pub since_format: u32,
    /// Schéma JSON des paramètres.
    pub schema: fn() -> Schema,
    /// Résumé en français.
    pub summary_fr: &'static str,
    /// Résumé en anglais.
    pub summary_en: &'static str,
}

impl CommandSpec {
    /// Résumé dans la langue `lang`.
    #[must_use]
    pub const fn summary(&self, lang: Language) -> &'static str {
        match lang {
            Language::Fr => self.summary_fr,
            Language::En => self.summary_en,
        }
    }

    /// Vrai si la commande est disponible (listée par les surfaces).
    #[must_use]
    pub const fn is_listed(&self) -> bool {
        matches!(self.available, Availability::Now)
    }
}

/// Schéma JSON (2020-12) des paramètres `T`, tel que lu (contrat de
/// désérialisation), sous-schémas en ligne : un client qui ignore `$ref`
/// le lit tel quel.
#[must_use]
pub fn schema_of<T: JsonSchema>() -> Schema {
    SchemaSettings::draft2020_12()
        .with(|s| s.inline_subschemas = true)
        .for_deserialize()
        .into_generator()
        .into_root_schema_for::<T>()
}

/// Entrée disponible en J4.
const fn now(
    id: &'static str,
    mcp_name: &'static str,
    kind: Kind,
    schema: fn() -> Schema,
    summary_fr: &'static str,
    summary_en: &'static str,
) -> CommandSpec {
    let write = matches!(kind, Kind::Write);
    CommandSpec {
        id,
        mcp_name,
        kind,
        dry_run: write,
        undoable: write,
        available: Availability::Now,
        since_format: 1,
        schema,
        summary_fr,
        summary_en,
    }
}

/// Schéma d'une commande réservée : objet quelconque.
fn reserved_schema() -> Schema {
    json_schema!({ "type": "object" })
}

/// Entrée réservée (non listée).
const fn reserved(
    id: &'static str,
    mcp_name: &'static str,
    kind: Kind,
    milestone: u8,
    summary_fr: &'static str,
    summary_en: &'static str,
) -> CommandSpec {
    let write = matches!(kind, Kind::Write);
    CommandSpec {
        id,
        mcp_name,
        kind,
        dry_run: write,
        undoable: write,
        available: Availability::Milestone(milestone),
        since_format: 1,
        schema: reserved_schema,
        summary_fr,
        summary_en,
    }
}

/// Registre, trié par id (vérifié par test).
pub static REGISTRY: &[CommandSpec] = &[
    reserved(
        "analysis.baseline_variance",
        "baseline_variance",
        Kind::Analysis,
        6,
        "Écarts à la ligne de base.",
        "Baseline variance.",
    ),
    now(
        "analysis.critical_path",
        "critical_path",
        Kind::Read,
        schema_of::<NoParams>,
        "Chemin critique : tâches critiques dans l'ordre, fin du projet.",
        "Critical path: critical tasks in order, project finish.",
    ),
    reserved(
        "analysis.explain_date",
        "explain_date",
        Kind::Analysis,
        6,
        "Explique la date d'une tâche.",
        "Explain a task date.",
    ),
    reserved(
        "analysis.resource_usage",
        "resource_usage",
        Kind::Analysis,
        5,
        "Charge des ressources.",
        "Resource usage.",
    ),
    reserved(
        "analysis.what_if",
        "what_if",
        Kind::Analysis,
        6,
        "Simule un lot sans l'appliquer.",
        "Simulate a batch without applying it.",
    ),
    reserved(
        "assignment.add",
        "assign_resource",
        Kind::Write,
        5,
        "Affecte une ressource à une tâche.",
        "Assign a resource to a task.",
    ),
    reserved(
        "assignment.update",
        "update_assignment",
        Kind::Write,
        5,
        "Modifie une affectation.",
        "Update an assignment.",
    ),
    reserved(
        "baseline.set",
        "set_baseline",
        Kind::Write,
        6,
        "Enregistre une ligne de base.",
        "Set a baseline.",
    ),
    now(
        "batch.apply",
        "apply_batch",
        Kind::Write,
        schema_of::<ApplyBatchParams>,
        "Applique un lot de commandes en une seule transaction (étiquettes \"$x\" entre commandes).",
        "Apply a batch of commands as one transaction (\"$x\" labels across commands).",
    ),
    now(
        "calendar.remove_exception",
        "remove_calendar_exception",
        Kind::Write,
        schema_of::<CalendarExceptionKey>,
        "Retire une exception de calendrier.",
        "Remove a calendar exception.",
    ),
    now(
        "calendar.set_exception",
        "edit_calendar",
        Kind::Write,
        schema_of::<CalendarException>,
        "Crée ou remplace une exception de calendrier (jours chômés ou horaires).",
        "Create or replace a calendar exception (nonworking days or working times).",
    ),
    now(
        "history.list",
        "history",
        Kind::History,
        schema_of::<NoParams>,
        "Historique des modifications annulables.",
        "History of undoable changes.",
    ),
    now(
        "history.redo",
        "redo",
        Kind::History,
        schema_of::<NoParams>,
        "Rétablit la dernière modification annulée.",
        "Redo the last undone change.",
    ),
    now(
        "history.undo",
        "undo",
        Kind::History,
        schema_of::<NoParams>,
        "Annule la dernière modification.",
        "Undo the last change.",
    ),
    now(
        "link.add",
        "add_link",
        Kind::Write,
        schema_of::<LinkSpec>,
        "Ajoute un lien entre deux tâches (FS par défaut, décalage facultatif).",
        "Add a link between two tasks (FS by default, optional lag).",
    ),
    now(
        "link.remove",
        "remove_link",
        Kind::Write,
        schema_of::<LinkKey>,
        "Retire un lien.",
        "Remove a link.",
    ),
    now(
        "link.update",
        "update_link",
        Kind::Write,
        schema_of::<LinkSpec>,
        "Modifie le type ou le décalage d'un lien.",
        "Change the type or lag of a link.",
    ),
    reserved(
        "progress.update",
        "update_progress",
        Kind::Write,
        6,
        "Met à jour l'avancement.",
        "Update progress.",
    ),
    now(
        "project.export_mspdi",
        "export_mspdi",
        Kind::File,
        schema_of::<ExportMspdiParams>,
        "Écrit une copie du projet au format MSPDI (XML de MS Project).",
        "Write a copy of the project as MSPDI (MS Project XML).",
    ),
    now(
        "project.import_csv",
        "import_csv",
        Kind::File,
        schema_of::<ImportCsvParams>,
        "Ouvre un projet construit depuis un fichier CSV de tâches.",
        "Open a project built from a CSV file of tasks.",
    ),
    now(
        "project.new",
        "new_project",
        Kind::File,
        schema_of::<NewProjectParams>,
        "Crée un projet vide, planifié depuis une date de début ou de fin.",
        "Create an empty project, scheduled from a start or finish date.",
    ),
    now(
        "project.open",
        "open_project",
        Kind::File,
        schema_of::<OpenProjectParams>,
        "Ouvre un fichier MSPDI (XML de MS Project).",
        "Open an MSPDI file (MS Project XML).",
    ),
    now(
        "project.save",
        "save_project",
        Kind::File,
        schema_of::<SaveProjectParams>,
        "Enregistre le projet (MSPDI), sur son fichier ou sous un autre chemin.",
        "Save the project (MSPDI), to its file or another path.",
    ),
    now(
        "project.set_anchor",
        "set_project_start",
        Kind::Write,
        schema_of::<AnchorSpec>,
        "Fixe la date de début du projet, ou sa date de fin (planification à rebours).",
        "Set the project start date, or its finish date (backward scheduling).",
    ),
    now(
        "project.set_info",
        "set_project_info",
        Kind::Write,
        schema_of::<ProjectInfoPatch>,
        "Modifie les informations du projet (nom, titre, auteur…).",
        "Update the project information (name, title, author…).",
    ),
    now(
        "project.set_status_date",
        "set_status_date",
        Kind::Write,
        schema_of::<SetStatusDate>,
        "Pose ou retire la date d'état.",
        "Set or clear the status date.",
    ),
    now(
        "project.summary",
        "project_summary",
        Kind::Read,
        schema_of::<NoParams>,
        "Résumé du projet : dates, nombre de tâches, tâches critiques, erreurs.",
        "Project summary: dates, task counts, critical tasks, errors.",
    ),
    now(
        "render.gantt",
        "render_gantt",
        Kind::Read,
        schema_of::<RenderGanttParams>,
        "Diagramme de Gantt en SVG, avec plage, échelle et filtre.",
        "Gantt chart as SVG, with range, zoom and filter.",
    ),
    reserved(
        "resource.add",
        "add_resource",
        Kind::Write,
        5,
        "Ajoute une ressource.",
        "Add a resource.",
    ),
    now(
        "task.add",
        "add_task",
        Kind::Write,
        schema_of::<TaskAdd>,
        "Ajoute une tâche (placement, durée, jalon, notes ; étiquette \"as\" pour un lot).",
        "Add a task (placement, duration, milestone, notes; \"as\" label for a batch).",
    ),
    now(
        "task.delete",
        "delete_task",
        Kind::Write,
        schema_of::<TaskDelete>,
        "Supprime une tâche, avec ses liens (cascade obligatoire pour une récapitulative).",
        "Delete a task and its links (cascade required for a summary task).",
    ),
    now(
        "task.get",
        "get_task",
        Kind::Read,
        schema_of::<TaskGetParams>,
        "Détail d'une tâche : dates du moteur, marges, liens, contrainte.",
        "Task details: engine dates, slack, links, constraint.",
    ),
    now(
        "task.indent",
        "indent_task",
        Kind::Write,
        schema_of::<TaskOne>,
        "Indente une tâche et ses sous-tâches.",
        "Indent a task and its subtasks.",
    ),
    now(
        "task.list",
        "list_tasks",
        Kind::Read,
        schema_of::<TaskListParams>,
        "Liste des tâches, filtrée (critiques, en retard, plage de dates) et paginée.",
        "List tasks, filtered (critical, late, date range) and paginated.",
    ),
    now(
        "task.move",
        "move_task",
        Kind::Write,
        schema_of::<TaskMove>,
        "Déplace une tâche et ses sous-tâches (ordre et hiérarchie).",
        "Move a task and its subtasks (order and hierarchy).",
    ),
    now(
        "task.outdent",
        "outdent_task",
        Kind::Write,
        schema_of::<TaskOne>,
        "Désindente une tâche et ses sous-tâches.",
        "Outdent a task and its subtasks.",
    ),
    now(
        "task.set_constraint",
        "set_constraint",
        Kind::Write,
        schema_of::<SetConstraint>,
        "Pose une contrainte de date sur une tâche.",
        "Set a date constraint on a task.",
    ),
    now(
        "task.set_deadline",
        "set_deadline",
        Kind::Write,
        schema_of::<SetDeadline>,
        "Pose ou retire l'échéance d'une tâche.",
        "Set or clear a task deadline.",
    ),
    now(
        "task.update",
        "update_task",
        Kind::Write,
        schema_of::<TaskUpdate>,
        "Modifie une tâche (nom, durée, jalon, notes, début ou fin saisis, active).",
        "Update a task (name, duration, milestone, notes, entered start or finish, active).",
    ),
];

/// Entrées disponibles, dans l'ordre des ids.
pub fn listed() -> impl Iterator<Item = &'static CommandSpec> {
    REGISTRY.iter().filter(|s| s.is_listed())
}

/// Entrée d'id `id` (recherche dichotomique).
#[must_use]
pub fn lookup(id: &str) -> Option<&'static CommandSpec> {
    REGISTRY
        .binary_search_by(|s| s.id.cmp(id))
        .ok()
        .and_then(|i| REGISTRY.get(i))
}

/// Entrée de nom MCP `n`.
#[must_use]
pub fn by_mcp_name(n: &str) -> Option<&'static CommandSpec> {
    REGISTRY.iter().find(|s| s.mcp_name == n)
}
