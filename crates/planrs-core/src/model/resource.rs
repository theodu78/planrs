//! Ressource (ADR-0008, § 5.5).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::arena::HasUid;
use crate::ids::{BaselineNumber, CalendarUid, Guid, ResourceUid};
use crate::model::Extras;
use crate::model::baseline::ResourceBaseline;
use crate::quantity::{Money, Rate, Units};

/// Type de ressource.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResourceKind {
    /// Ressource travail (personne, machine).
    #[default]
    Work,
    /// Ressource matérielle, avec son étiquette d'unité (« m³ »).
    Material {
        /// Étiquette d'unité.
        label: String,
    },
    /// Ressource coût.
    Cost,
}

/// Ressource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    uid: ResourceUid,
    /// GUID conservé tel quel.
    pub guid: Option<Guid>,
    /// Nom.
    pub name: String,
    /// Initiales.
    pub initials: String,
    /// Groupe.
    pub group: String,
    /// Type.
    pub kind: ResourceKind,
    /// Capacité maximale (1/10 000 ; 10 000 = 100 % par défaut, S5).
    pub max_units: Units,
    /// Calendrier propre (dérivé d'un calendrier de base en général).
    pub calendar: Option<CalendarUid>,
    /// Taux standard.
    pub standard_rate: Rate,
    /// Taux des heures supplémentaires.
    pub overtime_rate: Rate,
    /// Coût par utilisation.
    pub cost_per_use: Money,
    /// Ressource générique.
    pub is_generic: bool,
    /// Ressource inactive.
    pub is_inactive: bool,
    /// Notes.
    pub notes: String,
    /// Lignes de base 0 à 10.
    pub baselines: BTreeMap<BaselineNumber, ResourceBaseline>,
    /// Données hors modèle.
    pub extras: Extras,
}

impl HasUid for Resource {
    type Uid = ResourceUid;
    fn uid(&self) -> ResourceUid {
        self.uid
    }
}

impl Resource {
    /// Ressource travail à 100 %, sans calendrier propre ni coût.
    #[must_use]
    pub fn new(uid: ResourceUid, name: String) -> Self {
        Self {
            uid,
            guid: None,
            name,
            initials: String::new(),
            group: String::new(),
            kind: ResourceKind::Work,
            max_units: Units::FULL,
            calendar: None,
            standard_rate: Rate::default(),
            overtime_rate: Rate::default(),
            cost_per_use: Money::ZERO,
            is_generic: false,
            is_inactive: false,
            notes: String::new(),
            baselines: BTreeMap::new(),
            extras: Extras::new(),
        }
    }

    /// UID.
    #[must_use]
    pub const fn uid(&self) -> ResourceUid {
        self.uid
    }
}
