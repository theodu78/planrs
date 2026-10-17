//! Transfert d'une date à travers le décalage d'un lien (spécification J2
//! § 3.0, R-041 à R-043, R-047).
//!
//! Seul point d'entrée des décalages pour les passes : si un oracle
//! contredit l'adjoint (risque n° 1 de l'ADR-0009), c'est la seule fonction
//! à changer.

use crate::calendar::{Direction, TimeScale};
use crate::duration::Minutes;
use crate::error::CalendarError;
use crate::time::Moment;

/// Transfert avant `g(x)` : `x` si `L = 0` (aucun recalage), sinon
/// `σ.add(x, L)` (R-021, R-022, R-028).
///
/// # Errors
///
/// Erreurs de conversion de l'échelle `σ`.
pub(super) fn forward(sigma: TimeScale<'_>, x: Moment, lag: Minutes) -> Result<Moment, CalendarError> {
    if lag == Minutes::ZERO {
        return Ok(x);
    }
    sigma.add(x, lag)
}

/// Transfert arrière, adjoint de [`forward`] (R-047) : `y` si `L = 0` ;
/// temps écoulé : `y − L` ; temps ouvré :
/// `σ.next_working_moment(σ.add(y, −L))`. `h(y)` est le plus grand `x` tel
/// que `W(g(x)) ≤ W(y)` : la passe arrière ne rend jamais une marge négative
/// sans contrainte, même avec une avance entre calendriers différents.
///
/// # Errors
///
/// Erreurs de conversion de l'échelle `σ`.
pub(super) fn backward(sigma: TimeScale<'_>, y: Moment, lag: Minutes) -> Result<Moment, CalendarError> {
    if lag == Minutes::ZERO {
        return Ok(y);
    }
    let back = lag.checked_neg().ok_or(match sigma {
        TimeScale::Working(c) => CalendarError::OutOfDomain {
            calendar: c.uid(),
            direction: Direction::After,
        },
        TimeScale::Elapsed => CalendarError::ElapsedOutOfDomain {
            direction: Direction::After,
        },
    })?;
    match sigma {
        TimeScale::Elapsed => sigma.add(y, back),
        TimeScale::Working(c) => c.next_working_moment(c.add_working(y, back)?),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::{backward, forward};
    use crate::arena::Arena;
    use crate::calendar::builtin::standard;
    use crate::calendar::compiled::CompiledCalendar;
    use crate::calendar::{Direction, TimeScale};
    use crate::duration::Minutes;
    use crate::error::CalendarError;
    use crate::ids::CalendarUid;
    use crate::time::Moment;

    fn m(s: &str) -> Moment {
        s.parse().unwrap()
    }

    fn std_cal() -> CompiledCalendar {
        let a = Arena::try_from(vec![standard(CalendarUid::new(1), "Standard".to_owned())]).unwrap();
        CompiledCalendar::compile(&a, CalendarUid::new(1)).unwrap()
    }

    #[test]
    fn zero_lag_is_identity_without_realignment() {
        let c = std_cal();
        let w = TimeScale::Working(&c);
        // Sam 24/10 10:00 n'est pas ouvré : un décalage nul ne recale pas.
        let t = m("2026-10-24T10:00");
        assert_eq!(forward(w, t, Minutes::ZERO), Ok(t));
        assert_eq!(backward(w, t, Minutes::ZERO), Ok(t));
    }

    #[test]
    fn adjoint_examples() {
        let c = std_cal();
        let w = TimeScale::Working(&c);
        // Avance d'une heure (R-047) : h(ven 16:00) = next(ven 17:00) = lun 08:00.
        assert_eq!(
            backward(w, m("2026-10-23T16:00"), Minutes::new(-60)),
            Ok(m("2026-10-26T08:00"))
        );
        // Décalage d'un jour : g(lun 17:00) = mar 17:00 ; h(mer 08:00) =
        // next(add(mer 08:00, −480)) = next(mar 08:00) = mar 08:00.
        assert_eq!(
            forward(w, m("2026-10-19T17:00"), Minutes::new(480)),
            Ok(m("2026-10-20T17:00"))
        );
        assert_eq!(
            backward(w, m("2026-10-21T08:00"), Minutes::new(480)),
            Ok(m("2026-10-20T08:00"))
        );
        // Temps écoulé : y − L.
        assert_eq!(
            backward(TimeScale::Elapsed, m("2026-10-25T17:00"), Minutes::new(2_880)),
            Ok(m("2026-10-23T17:00"))
        );
        // Débordement de l'opposé : erreur, jamais de panique.
        assert_eq!(
            backward(w, m("2026-10-25T17:00"), Minutes::new(i64::MIN)),
            Err(CalendarError::OutOfDomain {
                calendar: CalendarUid::new(1),
                direction: Direction::After
            })
        );
        assert!(backward(TimeScale::Elapsed, Moment::MIN, Minutes::new(i64::MIN)).is_err());
    }
}
