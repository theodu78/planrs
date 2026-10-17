//! Résolution d'un calendrier (ADR-0008, § 6.3), chemin froid.
//!
//! État : chaîne d'héritage et détection de ses erreurs (R-015). À compléter
//! avec les calendriers : `resolve_day` (précédence R-016) et
//! `expand_exception` (dépliage des récurrences, R-018).

use crate::arena::Arena;
use crate::calendar::model::{Calendar, CalendarKind};
use crate::error::CalendarError;
use crate::ids::CalendarUid;

/// Borne défensive du suivi d'une chaîne de bases dérivées.
const MAX_CHAIN_STEPS: usize = 8;

/// Chaîne d'héritage valide : `[c]` (base) ou `[c, base]` (dérivé).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarChain<'a> {
    /// Calendrier de base.
    Base(&'a Calendar),
    /// Calendrier dérivé et sa base (qui est un calendrier de base).
    Derived {
        /// Calendrier dérivé.
        calendar: &'a Calendar,
        /// Sa base.
        base: &'a Calendar,
    },
}

impl<'a> CalendarChain<'a> {
    /// Calendrier demandé.
    #[must_use]
    pub const fn calendar(&self) -> &'a Calendar {
        match *self {
            Self::Base(c) | Self::Derived { calendar: c, .. } => c,
        }
    }

    /// Calendrier de base de la chaîne (lui-même pour une base).
    #[must_use]
    pub const fn base(&self) -> &'a Calendar {
        match *self {
            Self::Base(c) | Self::Derived { base: c, .. } => c,
        }
    }
}

/// Chaîne d'héritage du calendrier `uid` (R-015).
///
/// Pour un calendrier dérivé `c` : sa base doit exister et être un calendrier
/// de base. Si la base est elle-même dérivée, la chaîne des bases est suivie
/// (au plus 8 pas) pour distinguer une boucle (`InheritanceCycle`, chemin
/// complet avec le premier UID répété en fin) d'une simple base dérivée
/// (`BaseNotBase`). Un calendrier dont la base est lui-même donne
/// `InheritanceCycle { chain: [c, c] }`.
///
/// # Errors
///
/// [`CalendarError::UnknownCalendar`], [`CalendarError::BaseMissing`],
/// [`CalendarError::BaseNotBase`], [`CalendarError::InheritanceCycle`].
pub fn chain(
    calendars: &Arena<CalendarUid, Calendar>,
    uid: CalendarUid,
) -> Result<CalendarChain<'_>, CalendarError> {
    let calendar = calendars
        .get(uid)
        .ok_or(CalendarError::UnknownCalendar { calendar: uid })?;
    let base_uid = match calendar.kind() {
        CalendarKind::Base { .. } => return Ok(CalendarChain::Base(calendar)),
        CalendarKind::Derived { base, .. } => *base,
    };
    if base_uid == uid {
        return Err(CalendarError::InheritanceCycle {
            chain: vec![uid, uid],
        });
    }
    let base = calendars.get(base_uid).ok_or(CalendarError::BaseMissing {
        calendar: uid,
        base: base_uid,
    })?;
    if base.is_base() {
        return Ok(CalendarChain::Derived { calendar, base });
    }
    // La base est dérivée : boucle ou simple base non-base ?
    let not_base = CalendarError::BaseNotBase {
        calendar: uid,
        base: base_uid,
    };
    let mut path = vec![uid, base_uid];
    let mut current = base;
    for _ in 0..MAX_CHAIN_STEPS {
        let next = match current.kind() {
            CalendarKind::Derived { base, .. } => *base,
            CalendarKind::Base { .. } => return Err(not_base),
        };
        let seen = path.contains(&next);
        path.push(next);
        if seen {
            return Err(CalendarError::InheritanceCycle { chain: path });
        }
        match calendars.get(next) {
            Some(c) => current = c,
            None => return Err(not_base),
        }
    }
    Err(not_base)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::{CalendarChain, chain};
    use crate::arena::Arena;
    use crate::calendar::builtin::standard;
    use crate::calendar::model::Calendar;
    use crate::error::CalendarError;
    use crate::ids::CalendarUid;

    fn uid(v: u32) -> CalendarUid {
        CalendarUid::new(v)
    }

    fn derived(v: u32, base: u32) -> Calendar {
        Calendar::new_derived(uid(v), format!("D{v}"), uid(base))
    }

    fn arena(items: Vec<Calendar>) -> Arena<CalendarUid, Calendar> {
        Arena::try_from(items).unwrap()
    }

    #[test]
    fn valid_chains() {
        let a = arena(vec![standard(uid(1), "Standard".to_owned()), derived(4, 1)]);
        let c = chain(&a, uid(1)).unwrap();
        assert!(matches!(c, CalendarChain::Base(_)));
        assert_eq!(c.base().uid(), uid(1));
        let c = chain(&a, uid(4)).unwrap();
        assert_eq!((c.calendar().uid(), c.base().uid()), (uid(4), uid(1)));
    }

    #[test]
    fn chain_errors() {
        let a = arena(vec![
            standard(uid(1), "Standard".to_owned()),
            derived(4, 1),
            derived(5, 6),
            derived(6, 5),
            derived(7, 4),
            derived(8, 99),
            derived(9, 9),
            derived(10, 11),
            derived(11, 12),
        ]);
        assert_eq!(
            chain(&a, uid(2)).unwrap_err(),
            CalendarError::UnknownCalendar { calendar: uid(2) }
        );
        assert_eq!(
            chain(&a, uid(8)).unwrap_err(),
            CalendarError::BaseMissing {
                calendar: uid(8),
                base: uid(99)
            }
        );
        // 7 → 4 → 1 : 4 est dérivé, pas une boucle.
        assert_eq!(
            chain(&a, uid(7)).unwrap_err(),
            CalendarError::BaseNotBase {
                calendar: uid(7),
                base: uid(4)
            }
        );
        // 10 → 11 → 12 (absent) : base dérivée.
        assert_eq!(
            chain(&a, uid(10)).unwrap_err(),
            CalendarError::BaseNotBase {
                calendar: uid(10),
                base: uid(11)
            }
        );
        assert_eq!(
            chain(&a, uid(5)).unwrap_err(),
            CalendarError::InheritanceCycle {
                chain: vec![uid(5), uid(6), uid(5)]
            }
        );
        assert_eq!(
            chain(&a, uid(9)).unwrap_err(),
            CalendarError::InheritanceCycle {
                chain: vec![uid(9), uid(9)]
            }
        );
    }

    #[test]
    fn long_derived_chain_is_bounded() {
        // 20 → 21 → … → 39 → 40 (absent) : plus de 8 pas, BaseNotBase.
        let mut items = vec![];
        for v in 20..40 {
            items.push(derived(v, v + 1));
        }
        let a = arena(items);
        assert_eq!(
            chain(&a, uid(20)).unwrap_err(),
            CalendarError::BaseNotBase {
                calendar: uid(20),
                base: uid(21)
            }
        );
    }

    #[test]
    fn cycle_not_through_start() {
        // 30 → 31 → 32 → 31 : boucle qui ne repasse pas par 30.
        let a = arena(vec![derived(30, 31), derived(31, 32), derived(32, 31)]);
        assert_eq!(
            chain(&a, uid(30)).unwrap_err(),
            CalendarError::InheritanceCycle {
                chain: vec![uid(30), uid(31), uid(32), uid(31)]
            }
        );
    }
}
