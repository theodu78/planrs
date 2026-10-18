//! Registre des commandes (ADR-0012 D7) : table triée par id, source unique
//! des outils MCP, des commandes de la CLI, du canal et des raccourcis de
//! l'interface.
//!
//! Contrat L0 : types et entrées réservées (J5-J6, non listées) ; les
//! entrées J4 et leurs schémas sont ajoutés par le lot L3.

use schemars::{Schema, json_schema};

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
    reserved(
        "progress.update",
        "update_progress",
        Kind::Write,
        6,
        "Met à jour l'avancement.",
        "Update progress.",
    ),
    reserved(
        "resource.add",
        "add_resource",
        Kind::Write,
        5,
        "Ajoute une ressource.",
        "Add a resource.",
    ),
];

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
