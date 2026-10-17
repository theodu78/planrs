//! Erreurs du cœur : refus d'une opération (ADR-0008, § 7.1).
//!
//! Chaque type d'erreur :
//! - a un message français (`Display`, par `thiserror`) ;
//! - donne un code stable par `code()` : préfixe du module, suffixe de la
//!   variante en `snake_case` (`fixed.too_many_decimals`) ;
//! - se sérialise avec ce code dans le champ `code` (`#[serde(tag = "code")]`,
//!   chaque variante renommée par son code). Un test vérifie que les deux
//!   coïncident pour chaque variante.
//!
//! Les erreurs ne se désérialisent pas : elles sortent du cœur, elles n'y
//! entrent jamais.

use serde::Serialize;

use crate::time::Day;

/// Erreur du point fixe (`fixed.rs`, `quantity.rs`).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "code")]
pub enum FixedError {
    /// Plus de 6 décimales.
    #[serde(rename = "fixed.too_many_decimals")]
    #[error("trop de décimales (au plus 6)")]
    TooManyDecimals,
    /// Valeur hors des bornes du type.
    #[serde(rename = "fixed.out_of_range")]
    #[error("valeur {value} hors des bornes [{min} ; {max}]")]
    OutOfRange {
        /// Borne inférieure incluse.
        min: i64,
        /// Borne supérieure incluse.
        max: i64,
        /// Valeur refusée.
        value: i128,
    },
    /// Dépassement de capacité pendant un calcul.
    #[serde(rename = "fixed.overflow")]
    #[error("dépassement de capacité dans un calcul en point fixe")]
    Overflow,
}

impl FixedError {
    /// Code stable de l'erreur.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::TooManyDecimals => "fixed.too_many_decimals",
            Self::OutOfRange { .. } => "fixed.out_of_range",
            Self::Overflow => "fixed.overflow",
        }
    }
}

/// Erreur du temps (`time.rs`).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "code")]
pub enum TimeError {
    /// Instant ou jour hors du domaine 1984-01-01 00:00 à 2150-01-01 00:00.
    #[serde(rename = "time.out_of_domain")]
    #[error("date hors du domaine (du 1er janvier 1984 au 31 décembre 2149)")]
    OutOfDomain,
    /// Heure au-delà de 24:00, ou minutes >= 60.
    #[serde(rename = "time.invalid_time_of_day")]
    #[error("heure invalide ({minutes} minutes depuis minuit, au plus 24:00)")]
    InvalidTimeOfDay {
        /// Heure refusée, en minutes depuis minuit.
        minutes: u16,
    },
    /// Secondes non nulles dans une saisie stricte.
    #[serde(rename = "time.non_zero_seconds")]
    #[error("les secondes doivent être nulles (résolution à la minute)")]
    NonZeroSeconds,
    /// Texte qui n'a pas la forme attendue.
    #[serde(rename = "time.invalid_format")]
    #[error("format de date ou d'heure invalide : « {input} »")]
    InvalidFormat {
        /// Texte refusé.
        input: String,
    },
    /// Intervalle de jours dont le début suit la fin.
    #[serde(rename = "time.invalid_day_range")]
    #[error("intervalle de jours invalide : du {first} au {last}")]
    InvalidDayRange {
        /// Premier jour.
        first: Day,
        /// Dernier jour, antérieur au premier.
        last: Day,
    },
    /// Dépassement de capacité pendant un calcul.
    #[serde(rename = "time.overflow")]
    #[error("dépassement de capacité dans un calcul de date")]
    Overflow,
}

impl TimeError {
    /// Code stable de l'erreur.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::OutOfDomain => "time.out_of_domain",
            Self::InvalidTimeOfDay { .. } => "time.invalid_time_of_day",
            Self::NonZeroSeconds => "time.non_zero_seconds",
            Self::InvalidFormat { .. } => "time.invalid_format",
            Self::InvalidDayRange { .. } => "time.invalid_day_range",
            Self::Overflow => "time.overflow",
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::{FixedError, TimeError};
    use crate::time::Day;

    /// Vérifie que l'étiquette serde `code` vaut `code()`.
    pub(crate) fn assert_tag(value: &impl serde::Serialize, code: &str) {
        let json = serde_json::to_value(value).unwrap();
        assert_eq!(json["code"].as_str(), Some(code), "{json}");
    }

    #[test]
    fn fixed_error_codes_match_serde_tags() {
        for e in [
            FixedError::TooManyDecimals,
            FixedError::OutOfRange {
                min: 0,
                max: 1,
                value: 2,
            },
            FixedError::Overflow,
        ] {
            assert_tag(&e, e.code());
            assert!(e.code().starts_with("fixed."));
            assert_ne!(e.to_string(), "");
        }
    }

    #[test]
    fn time_error_codes_match_serde_tags() {
        for e in [
            TimeError::OutOfDomain,
            TimeError::InvalidTimeOfDay { minutes: 1441 },
            TimeError::NonZeroSeconds,
            TimeError::InvalidFormat {
                input: "x".to_owned(),
            },
            TimeError::InvalidDayRange {
                first: Day::MAX,
                last: Day::MIN,
            },
            TimeError::Overflow,
        ] {
            assert_tag(&e, e.code());
            assert!(e.code().starts_with("time."));
            assert_ne!(e.to_string(), "");
        }
    }
}
