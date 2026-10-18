//! Formes JSON des valeurs du modèle dans les commandes et les vues
//! (contrat J4, ADR-0012 D6).
//!
//! Une seule forme de durée en entrée et en sortie :
//! `{"minutes":2400,"unit":"day","text":"5j"}` ; en entrée, `text` seul ou
//! `minutes` seul suffit, les deux doivent concorder. Les conversions vers le
//! modèle (R-150 et la partie « lecture » de R-152/R-158) sont écrites par le
//! lot L3.

use planrs_core::{ConstraintKind, DurationUnit, LinkType};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Durée en JSON.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DurationJson {
    /// Minutes (ouvrées, ou écoulées si `elapsed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes: Option<i64>,
    /// Unité d'affichage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<DurationUnitJson>,
    /// Texte à la manière de MS Project (« 5j », « 2d? »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Durée écoulée (« 5ej »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed: Option<bool>,
    /// Durée estimée (« 5j? »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated: Option<bool>,
}

/// Décalage d'un lien en JSON : minutes et unité, ou pourcentage de la durée
/// du prédécesseur, ou texte (« +2j », « 50% »).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LagJson {
    /// Minutes (signées).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes: Option<i64>,
    /// Pourcentage en 1/100 de % (5 000 = 50 %).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent: Option<i64>,
    /// Unité d'affichage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<DurationUnitJson>,
    /// Texte (« +2j », « -1d », « 50% »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Décalage écoulé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed: Option<bool>,
}

/// Unité d'une durée en JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DurationUnitJson {
    /// Minute.
    Minute,
    /// Heure.
    Hour,
    /// Jour.
    Day,
    /// Semaine.
    Week,
    /// Mois.
    Month,
}

impl From<DurationUnit> for DurationUnitJson {
    fn from(u: DurationUnit) -> Self {
        match u {
            DurationUnit::Minute => Self::Minute,
            DurationUnit::Hour => Self::Hour,
            DurationUnit::Day => Self::Day,
            DurationUnit::Week => Self::Week,
            DurationUnit::Month => Self::Month,
        }
    }
}

impl From<DurationUnitJson> for DurationUnit {
    fn from(u: DurationUnitJson) -> Self {
        match u {
            DurationUnitJson::Minute => Self::Minute,
            DurationUnitJson::Hour => Self::Hour,
            DurationUnitJson::Day => Self::Day,
            DurationUnitJson::Week => Self::Week,
            DurationUnitJson::Month => Self::Month,
        }
    }
}

/// Type de lien en JSON (codes anglais de MS Project).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinkTypeJson {
    /// Fin à début.
    #[default]
    Fs,
    /// Début à début.
    Ss,
    /// Fin à fin.
    Ff,
    /// Début à fin.
    Sf,
}

impl From<LinkType> for LinkTypeJson {
    fn from(k: LinkType) -> Self {
        match k {
            LinkType::FinishToStart => Self::Fs,
            LinkType::StartToStart => Self::Ss,
            LinkType::FinishToFinish => Self::Ff,
            LinkType::StartToFinish => Self::Sf,
        }
    }
}

impl From<LinkTypeJson> for LinkType {
    fn from(k: LinkTypeJson) -> Self {
        match k {
            LinkTypeJson::Fs => Self::FinishToStart,
            LinkTypeJson::Ss => Self::StartToStart,
            LinkTypeJson::Ff => Self::FinishToFinish,
            LinkTypeJson::Sf => Self::StartToFinish,
        }
    }
}

/// Type de contrainte en JSON (sigles anglais de MS Project).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintKindJson {
    /// Le plus tôt possible.
    Asap,
    /// Le plus tard possible.
    Alap,
    /// Doit commencer le.
    Mso,
    /// Doit finir le.
    Mfo,
    /// Ne pas commencer avant le.
    Snet,
    /// Ne pas commencer après le.
    Snlt,
    /// Ne pas finir avant le.
    Fnet,
    /// Ne pas finir après le.
    Fnlt,
}

impl From<ConstraintKind> for ConstraintKindJson {
    fn from(k: ConstraintKind) -> Self {
        match k {
            ConstraintKind::AsSoonAsPossible => Self::Asap,
            ConstraintKind::AsLateAsPossible => Self::Alap,
            ConstraintKind::MustStartOn => Self::Mso,
            ConstraintKind::MustFinishOn => Self::Mfo,
            ConstraintKind::StartNoEarlierThan => Self::Snet,
            ConstraintKind::StartNoLaterThan => Self::Snlt,
            ConstraintKind::FinishNoEarlierThan => Self::Fnet,
            ConstraintKind::FinishNoLaterThan => Self::Fnlt,
        }
    }
}

impl From<ConstraintKindJson> for ConstraintKind {
    fn from(k: ConstraintKindJson) -> Self {
        match k {
            ConstraintKindJson::Asap => Self::AsSoonAsPossible,
            ConstraintKindJson::Alap => Self::AsLateAsPossible,
            ConstraintKindJson::Mso => Self::MustStartOn,
            ConstraintKindJson::Mfo => Self::MustFinishOn,
            ConstraintKindJson::Snet => Self::StartNoEarlierThan,
            ConstraintKindJson::Snlt => Self::StartNoLaterThan,
            ConstraintKindJson::Fnet => Self::FinishNoEarlierThan,
            ConstraintKindJson::Fnlt => Self::FinishNoLaterThan,
        }
    }
}
