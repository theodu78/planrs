//! Filtre de tâches partagé (R-251, ADR-0016 D2).
//!
//! Prédicat pur, sans interface : le même pour l'interface (vue filtrée,
//! H-UI-30), `task.list` et l'outil MCP `list_tasks` (une seule sémantique
//! pour l'IA, la CLI et l'interface). Les critères « critique », « en
//! retard » et « inachevée » lisent le planning du moteur (règle 3). Un
//! filtre est une vue : il n'écrit rien (P50).
//!
//! Contrat posé par le lot L0, implémenté en L2 : jusque-là [`TaskFilter`]
//! laisse passer toute tâche et [`fold_text`] replie seulement la casse et
//! les accents latins usuels.

use planrs_core::{Day, Moment, Project, Schedule, Task};

/// Critères d'un filtre ; une tâche passe si elle satisfait **tous** les
/// critères posés (`None` : critère absent).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskFilter {
    /// Critique du moteur.
    pub critical: Option<bool>,
    /// En retard (sémantique de `task.list`, `view.rs`).
    pub late: Option<bool>,
    /// Feuille, non exclue, % achevé < 100 %.
    pub incomplete: Option<bool>,
    /// Jalon.
    pub milestone: Option<bool>,
    /// Récapitulative.
    pub summary: Option<bool>,
    /// Active.
    pub active: Option<bool>,
    /// Affectée à la ressource de ce nom (comparaison exacte).
    pub resource: Option<String>,
    /// Dates planifiées touchant ce jour ou un jour suivant.
    pub from: Option<Day>,
    /// Dates planifiées touchant ce jour ou un jour précédent.
    pub to: Option<Day>,
    /// Niveau hiérarchique maximal.
    pub outline_max: Option<u16>,
    /// Sous-chaîne du nom, sans casse ni accents ([`fold_text`]).
    pub name_contains: Option<String>,
    /// Ajoute les récapitulatives ancêtres d'une tâche retenue (vrai par
    /// défaut dans l'interface, faux pour `task.list`).
    pub show_related_summaries: bool,
}

/// Filtres prédéfinis du menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilterPreset {
    /// Toutes les tâches.
    All,
    /// Tâches critiques.
    Critical,
    /// Tâches en retard.
    Late,
    /// Tâches inachevées.
    Incomplete,
    /// Jalons.
    Milestones,
    /// Récapitulatives.
    Summaries,
    /// Tâches affectées à une ressource (nom).
    UsingResource(String),
    /// Tâches touchant une plage de jours.
    DateRange {
        /// Premier jour.
        from: Day,
        /// Dernier jour.
        to: Day,
    },
}

/// Contexte d'évaluation : projet, planning du moteur, date d'état.
#[derive(Clone, Copy, Debug)]
pub struct FilterCtx<'a> {
    /// Projet.
    pub project: &'a Project,
    /// Planning du moteur.
    pub schedule: &'a Schedule,
    /// Date d'état effective.
    pub status: Moment,
}

/// Résultat d'un filtre sur tout le projet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilterHits {
    /// Ligne montrée, par position dans le projet.
    pub shown: Vec<bool>,
    /// Tâches retenues par les critères.
    pub direct: u32,
    /// Récapitulatives ajoutées par `show_related_summaries`.
    pub related: u32,
}

impl TaskFilter {
    /// Filtre d'un préréglage (récapitulatives liées affichées).
    #[must_use]
    pub fn from_preset(p: &FilterPreset) -> Self {
        let mut f = Self {
            show_related_summaries: true,
            ..Self::default()
        };
        match p {
            FilterPreset::All => {}
            FilterPreset::Critical => f.critical = Some(true),
            FilterPreset::Late => f.late = Some(true),
            FilterPreset::Incomplete => f.incomplete = Some(true),
            FilterPreset::Milestones => f.milestone = Some(true),
            FilterPreset::Summaries => f.summary = Some(true),
            FilterPreset::UsingResource(name) => f.resource = Some(name.clone()),
            FilterPreset::DateRange { from, to } => {
                f.from = Some(*from);
                f.to = Some(*to);
            }
        }
        f
    }

    /// Vrai si la tâche `t` (position `pos`) satisfait tous les critères.
    /// Squelette L0 : toujours vrai (filtre transparent) jusqu'à L2.
    #[must_use]
    pub fn matches(&self, ctx: &FilterCtx<'_>, pos: usize, t: &Task) -> bool {
        let _ = (ctx, pos, t);
        true
    }

    /// Lignes montrées sur tout le projet. Squelette L0 : toutes.
    #[must_use]
    pub fn hits(&self, ctx: &FilterCtx<'_>) -> FilterHits {
        let len = ctx.project.tasks.len();
        FilterHits {
            shown: vec![true; len],
            direct: u32::try_from(len).unwrap_or(u32::MAX),
            related: 0,
        }
    }
}

/// Texte replié pour la recherche : sans casse ni accents (L2 : NFD par
/// `unicode-normalization`, marques combinantes retirées, minuscules ;
/// squelette L0 : repli des accents latins usuels).
#[must_use]
pub fn fold_text(s: &str) -> String {
    crate::text::lex::fold(s.chars())
}
