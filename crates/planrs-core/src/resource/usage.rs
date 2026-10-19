//! Utilisation des ressources par période (ADR-0013 D6 ; règle R-188).
//! Contrat J5 (lot L0) ; calcul rempli par le lot L5.
//!
//! Calculée à la demande depuis un [`Schedule`] (jamais stockée) : pour
//! chaque ressource demandée et chaque période de `[from, to)`, travail
//! planifié dans la période et surutilisation. Bornée (règle 10) :
//! [`MAX_PERIODS`] périodes, [`MAX_RESOURCES`] ressources.
//!
//! Squelette L0 : contrôle des bornes sur les ressources, rapport vide.

use crate::duration::Work;
use crate::ids::ResourceUid;
use crate::model::Project;
use crate::schedule::Schedule;
use crate::time::Moment;

/// Périodes d'un rapport au plus.
pub const MAX_PERIODS: usize = 1_000;
/// Ressources d'un rapport au plus.
pub const MAX_RESOURCES: usize = 1_000;

/// Granularité des périodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Granularity {
    /// Jour civil.
    Day,
    /// Semaine (premier jour : `WeekStartDay` du projet).
    Week,
    /// Mois civil.
    Month,
}

/// Rapport d'utilisation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageReport {
    /// Début de chaque période, dans l'ordre.
    pub periods: Vec<Moment>,
    /// Une ligne par ressource demandée, dans l'ordre demandé.
    pub rows: Vec<UsageRow>,
    /// Vrai si le rapport est coupé.
    pub truncated: bool,
}

/// Ligne d'une ressource : une valeur par période.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageRow {
    /// Ressource.
    pub resource: ResourceUid,
    /// Travail planifié dans chaque période.
    pub work: Vec<Work>,
    /// Surutilisation dans chaque période.
    pub overallocated: Vec<bool>,
}

/// Rapport refusé.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UsageError {
    /// Trop de périodes ou de ressources.
    #[error("rapport trop grand : au plus {periods} périodes et {resources} ressources")]
    Limit {
        /// Périodes au plus.
        periods: usize,
        /// Ressources au plus.
        resources: usize,
    },
}

impl UsageError {
    /// Code stable.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Limit { .. } => "usage.limit",
        }
    }
}

/// Utilisation des ressources `resources` sur `[from, to)`, par périodes
/// `by`.
///
/// # Errors
///
/// [`UsageError::Limit`] au-delà de [`MAX_PERIODS`] périodes ou de
/// [`MAX_RESOURCES`] ressources.
pub fn usage(
    p: &Project,
    s: &Schedule,
    from: Moment,
    to: Moment,
    by: Granularity,
    resources: &[ResourceUid],
) -> Result<UsageReport, UsageError> {
    let _ = (p, s, from, to, by);
    if resources.len() > MAX_RESOURCES {
        return Err(UsageError::Limit {
            periods: MAX_PERIODS,
            resources: MAX_RESOURCES,
        });
    }
    Ok(UsageReport {
        periods: Vec::new(),
        rows: Vec::new(),
        truncated: false,
    })
}
