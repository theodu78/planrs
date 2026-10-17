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

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::FixedError;

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
}
