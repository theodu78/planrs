//! Durées, travail et décalages en minutes entières (ADR-0005, ADR-0008).

use serde::{Deserialize, Serialize};

/// Quantité de minutes, signée (durée, écart, décalage) ; entier JSON.
///
/// Pas d'opérateurs `+`/`-` : ils devraient paniquer ou saturer sans le dire
/// (règle 10). Les opérations passent par `checked_*`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Minutes(i64);

impl Minutes {
    /// Zéro minute.
    pub const ZERO: Self = Self(0);

    /// Quantité de minutes.
    #[must_use]
    pub const fn new(v: i64) -> Self {
        Self(v)
    }

    /// Valeur en minutes.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }

    /// Somme, `None` en cas de débordement.
    #[must_use]
    pub fn checked_add(self, o: Self) -> Option<Self> {
        self.0.checked_add(o.0).map(Self)
    }

    /// Différence, `None` en cas de débordement.
    #[must_use]
    pub fn checked_sub(self, o: Self) -> Option<Self> {
        self.0.checked_sub(o.0).map(Self)
    }

    /// Opposé, `None` pour `i64::MIN`.
    #[must_use]
    pub fn checked_neg(self) -> Option<Self> {
        self.0.checked_neg().map(Self)
    }

    /// Vrai si strictement négatif.
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }
}

#[cfg(test)]
mod tests {
    use super::Minutes;

    #[test]
    fn minutes_checked_ops() {
        assert_eq!(
            Minutes::new(5).checked_add(Minutes::new(-7)),
            Some(Minutes::new(-2))
        );
        assert_eq!(Minutes::new(i64::MAX).checked_add(Minutes::new(1)), None);
        assert_eq!(Minutes::new(i64::MIN).checked_sub(Minutes::new(1)), None);
        assert_eq!(Minutes::new(i64::MIN).checked_neg(), None);
        assert_eq!(Minutes::new(3).checked_neg(), Some(Minutes::new(-3)));
        assert!(Minutes::new(-1).is_negative());
        assert!(!Minutes::ZERO.is_negative());
    }
}
