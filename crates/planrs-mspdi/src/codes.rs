//! Tables des codes numériques MSPDI → types du cœur (ADR-0008 : aucun code
//! MSPDI dans `planrs-core`).
//!
//! Source : schéma `mspdi_pj15.xsd` (Project 2013 SDK) et pages
//! learn.microsoft.com « XML Schema for the Tasks / Calendars / Project
//! Element » (S1 à S4). Une fonction par table ; un code hors liste rend
//! `None`, que l'appelant signale (`mspdi.unknown_code`) avant d'appliquer la
//! valeur par défaut.

use planrs_core::calendar::{MonthItem, MonthPosition};
use planrs_core::{DurationUnit, LinkType, Month, TaskType, TimeBasis, Week, Weekday};

/// Écart entre un code de durée et sa variante estimée (« ? ») : 32.
const ESTIMATED_OFFSET: i64 = 32;

/// `PredecessorLink/Type` : 0 FF, 1 FS, 2 SF, 3 SS (S4).
#[must_use]
pub const fn link_type(code: i64) -> Option<LinkType> {
    match code {
        0 => Some(LinkType::FinishToFinish),
        1 => Some(LinkType::FinishToStart),
        2 => Some(LinkType::StartToFinish),
        3 => Some(LinkType::StartToStart),
        _ => None,
    }
}

/// Format d'une durée ou d'un décalage (`DurationFormat`, `LagFormat`, S3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurationCode {
    /// Durée dans une unité de temps.
    Time {
        /// Unité d'affichage.
        unit: DurationUnit,
        /// Ouvrée ou écoulée.
        basis: TimeBasis,
        /// Variante estimée (« ? »).
        estimated: bool,
    },
    /// Pourcentage (`%`, `e%`) : n'a de sens que pour un décalage.
    Percent {
        /// Ouvré (`%`) ou écoulé (`e%`).
        basis: TimeBasis,
        /// Variante estimée.
        estimated: bool,
    },
    /// Format nul (21, 53) : unité par défaut du projet.
    Null {
        /// Variante estimée.
        estimated: bool,
    },
}

/// `DurationFormat` / `LagFormat` : 3 m, 4 em, 5 h, 6 eh, 7 j, 8 ej, 9 sem,
/// 10 esem, 11 mois, 12 emois, 19 %, 20 e%, 21 nul ; + 32 pour la variante
/// estimée (35 à 44, 51 à 53).
#[must_use]
pub fn duration_format(code: i64) -> Option<DurationCode> {
    let (base, estimated) = if (35..=53).contains(&code) {
        (code.saturating_sub(ESTIMATED_OFFSET), true)
    } else {
        (code, false)
    };
    let time = |unit, basis| {
        Some(DurationCode::Time {
            unit,
            basis,
            estimated,
        })
    };
    match base {
        3 => time(DurationUnit::Minute, TimeBasis::Working),
        4 => time(DurationUnit::Minute, TimeBasis::Elapsed),
        5 => time(DurationUnit::Hour, TimeBasis::Working),
        6 => time(DurationUnit::Hour, TimeBasis::Elapsed),
        7 => time(DurationUnit::Day, TimeBasis::Working),
        8 => time(DurationUnit::Day, TimeBasis::Elapsed),
        9 => time(DurationUnit::Week, TimeBasis::Working),
        10 => time(DurationUnit::Week, TimeBasis::Elapsed),
        11 => time(DurationUnit::Month, TimeBasis::Working),
        12 => time(DurationUnit::Month, TimeBasis::Elapsed),
        19 => Some(DurationCode::Percent {
            basis: TimeBasis::Working,
            estimated,
        }),
        20 => Some(DurationCode::Percent {
            basis: TimeBasis::Elapsed,
            estimated,
        }),
        21 => Some(DurationCode::Null { estimated }),
        _ => None,
    }
}

/// Type de contrainte, sans sa date : défini dans le cœur (ADR-0010), les
/// codes `ConstraintType` (0 à 7) restent ici (ADR-0008).
pub use planrs_core::ConstraintKind;

/// `ConstraintType` : 0 ASAP, 1 ALAP, 2 MSO, 3 MFO, 4 SNET, 5 SNLT, 6 FNET,
/// 7 FNLT.
#[must_use]
pub const fn constraint_type(code: i64) -> Option<ConstraintKind> {
    match code {
        0 => Some(ConstraintKind::AsSoonAsPossible),
        1 => Some(ConstraintKind::AsLateAsPossible),
        2 => Some(ConstraintKind::MustStartOn),
        3 => Some(ConstraintKind::MustFinishOn),
        4 => Some(ConstraintKind::StartNoEarlierThan),
        5 => Some(ConstraintKind::StartNoLaterThan),
        6 => Some(ConstraintKind::FinishNoEarlierThan),
        7 => Some(ConstraintKind::FinishNoLaterThan),
        _ => None,
    }
}

/// `Type` d'une tâche et `DefaultTaskType` : 0 unités fixes, 1 durée fixe,
/// 2 travail fixe.
#[must_use]
pub const fn task_type(code: i64) -> Option<TaskType> {
    match code {
        0 => Some(TaskType::FixedUnits),
        1 => Some(TaskType::FixedDuration),
        2 => Some(TaskType::FixedWork),
        _ => None,
    }
}

/// `WeekDay/DayType` : un jour de semaine, ou l'ancienne forme d'exception.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayType {
    /// Jour de la semaine par défaut.
    Weekday(Weekday),
    /// 0 : exception datée, ancienne forme (R-081).
    LegacyException,
}

/// `DayType` : 0 ancienne exception, 1 dimanche … 7 samedi (S1).
#[must_use]
pub const fn day_type(code: i64) -> Option<DayType> {
    match code {
        0 => Some(DayType::LegacyException),
        1 => Some(DayType::Weekday(Weekday::Sunday)),
        2 => Some(DayType::Weekday(Weekday::Monday)),
        3 => Some(DayType::Weekday(Weekday::Tuesday)),
        4 => Some(DayType::Weekday(Weekday::Wednesday)),
        5 => Some(DayType::Weekday(Weekday::Thursday)),
        6 => Some(DayType::Weekday(Weekday::Friday)),
        7 => Some(DayType::Weekday(Weekday::Saturday)),
        _ => None,
    }
}

/// `WeekStartDay` : 0 dimanche … 6 samedi (S2).
#[must_use]
pub const fn week_start_day(code: i64) -> Option<Weekday> {
    match code {
        0 => Some(Weekday::Sunday),
        1 => Some(Weekday::Monday),
        2 => Some(Weekday::Tuesday),
        3 => Some(Weekday::Wednesday),
        4 => Some(Weekday::Thursday),
        5 => Some(Weekday::Friday),
        6 => Some(Weekday::Saturday),
        _ => None,
    }
}

/// Type d'une exception de calendrier (`Exception/Type`, 1 à 9, S1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExceptionType {
    /// 1 : tous les `Period` jours.
    Daily,
    /// 2 : chaque année, le `MonthDay` du mois `Month`.
    YearlyByDay,
    /// 3 : chaque année, le n-ième `MonthItem` du mois `Month`.
    YearlyByPosition,
    /// 4 : tous les `Period` mois, le `MonthDay`.
    MonthlyByDay,
    /// 5 : tous les `Period` mois, le n-ième `MonthItem`.
    MonthlyByPosition,
    /// 6 : toutes les `Period` semaines, les jours de `DaysOfWeek`.
    Weekly,
    /// 7 : sémantique non documentée.
    ByDayCount,
    /// 8 : sémantique non documentée.
    ByWeekdayCount,
    /// 9 : chaque jour de la période.
    Period,
}

/// `Exception/Type` : 1 à 9.
#[must_use]
pub const fn exception_type(code: i64) -> Option<ExceptionType> {
    match code {
        1 => Some(ExceptionType::Daily),
        2 => Some(ExceptionType::YearlyByDay),
        3 => Some(ExceptionType::YearlyByPosition),
        4 => Some(ExceptionType::MonthlyByDay),
        5 => Some(ExceptionType::MonthlyByPosition),
        6 => Some(ExceptionType::Weekly),
        7 => Some(ExceptionType::ByDayCount),
        8 => Some(ExceptionType::ByWeekdayCount),
        9 => Some(ExceptionType::Period),
        _ => None,
    }
}

/// `Month` : 0 janvier … 11 décembre.
#[must_use]
pub const fn month(code: i64) -> Option<Month> {
    match code {
        0 => Some(Month::January),
        1 => Some(Month::February),
        2 => Some(Month::March),
        3 => Some(Month::April),
        4 => Some(Month::May),
        5 => Some(Month::June),
        6 => Some(Month::July),
        7 => Some(Month::August),
        8 => Some(Month::September),
        9 => Some(Month::October),
        10 => Some(Month::November),
        11 => Some(Month::December),
        _ => None,
    }
}

/// `MonthItem` : 0 jour, 1 jour ouvré, 2 jour de week-end, 3 dimanche …
/// 9 samedi.
#[must_use]
pub const fn month_item(code: i64) -> Option<MonthItem> {
    match code {
        0 => Some(MonthItem::Day),
        1 => Some(MonthItem::Weekday),
        2 => Some(MonthItem::WeekendDay),
        3 => Some(MonthItem::On(Weekday::Sunday)),
        4 => Some(MonthItem::On(Weekday::Monday)),
        5 => Some(MonthItem::On(Weekday::Tuesday)),
        6 => Some(MonthItem::On(Weekday::Wednesday)),
        7 => Some(MonthItem::On(Weekday::Thursday)),
        8 => Some(MonthItem::On(Weekday::Friday)),
        9 => Some(MonthItem::On(Weekday::Saturday)),
        _ => None,
    }
}

/// `MonthPosition` : 0 premier, 1 deuxième, 2 troisième, 3 quatrième,
/// 4 dernier.
#[must_use]
pub const fn month_position(code: i64) -> Option<MonthPosition> {
    match code {
        0 => Some(MonthPosition::First),
        1 => Some(MonthPosition::Second),
        2 => Some(MonthPosition::Third),
        3 => Some(MonthPosition::Fourth),
        4 => Some(MonthPosition::Last),
        _ => None,
    }
}

/// `DaysOfWeek` : masque de bits, 1 = dimanche, 2 = lundi … 64 = samedi.
/// `None` pour un masque négatif ou portant un bit au-delà de 64 (> 127).
#[must_use]
pub const fn days_of_week(mask: i64) -> Option<Week<bool>> {
    if mask < 0 || mask > 127 {
        return None;
    }
    Some(Week {
        sunday: mask & 1 != 0,
        monday: mask & 2 != 0,
        tuesday: mask & 4 != 0,
        wednesday: mask & 8 != 0,
        thursday: mask & 16 != 0,
        friday: mask & 32 != 0,
        saturday: mask & 64 != 0,
    })
}

/// `WorkFormat` : 1 m, 2 h, 3 j, 4 sem, 5 mois.
#[must_use]
pub const fn work_format(code: i64) -> Option<DurationUnit> {
    match code {
        1 => Some(DurationUnit::Minute),
        2 => Some(DurationUnit::Hour),
        3 => Some(DurationUnit::Day),
        4 => Some(DurationUnit::Week),
        5 => Some(DurationUnit::Month),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::{
        ConstraintKind, DayType, DurationCode, ExceptionType, constraint_type, day_type, days_of_week,
        duration_format, exception_type, link_type, month, month_item, month_position, task_type,
        week_start_day, work_format,
    };
    use planrs_core::calendar::{MonthItem, MonthPosition};
    use planrs_core::{DurationUnit, LinkType, Month, TaskType, TimeBasis, Weekday};

    /// Valeurs voisines de chaque table, hors liste : aucune ne doit passer.
    const OUT: [i64; 6] = [-1, 64, 100, i64::MIN, i64::MAX, 1_000];

    #[test]
    fn link_types() {
        assert_eq!(link_type(0), Some(LinkType::FinishToFinish));
        assert_eq!(link_type(1), Some(LinkType::FinishToStart));
        assert_eq!(link_type(2), Some(LinkType::StartToFinish));
        assert_eq!(link_type(3), Some(LinkType::StartToStart));
        assert_eq!(link_type(4), None);
        assert!(OUT.iter().all(|c| link_type(*c).is_none()));
    }

    #[test]
    fn duration_formats_exhaustive() {
        use DurationUnit::{Day, Hour, Minute, Month as Mo, Week};
        use TimeBasis::{Elapsed as E, Working as W};
        let time = [
            (3, Minute, W),
            (4, Minute, E),
            (5, Hour, W),
            (6, Hour, E),
            (7, Day, W),
            (8, Day, E),
            (9, Week, W),
            (10, Week, E),
            (11, Mo, W),
            (12, Mo, E),
        ];
        for (code, unit, basis) in time {
            for (c, estimated) in [(code, false), (code + 32, true)] {
                assert_eq!(
                    duration_format(c),
                    Some(DurationCode::Time {
                        unit,
                        basis,
                        estimated
                    }),
                    "{c}"
                );
            }
        }
        assert_eq!(
            duration_format(19),
            Some(DurationCode::Percent {
                basis: W,
                estimated: false
            })
        );
        assert_eq!(
            duration_format(52),
            Some(DurationCode::Percent {
                basis: E,
                estimated: true
            })
        );
        assert_eq!(duration_format(21), Some(DurationCode::Null { estimated: false }));
        assert_eq!(duration_format(53), Some(DurationCode::Null { estimated: true }));
        let valid: Vec<i64> = (3..=12).chain(19..=21).chain(35..=44).chain(51..=53).collect();
        for c in -5..=80 {
            assert_eq!(duration_format(c).is_some(), valid.contains(&c), "{c}");
        }
        assert!(OUT.iter().all(|c| duration_format(*c).is_none()));
    }

    #[test]
    fn constraint_and_task_types() {
        let kinds = [
            ConstraintKind::AsSoonAsPossible,
            ConstraintKind::AsLateAsPossible,
            ConstraintKind::MustStartOn,
            ConstraintKind::MustFinishOn,
            ConstraintKind::StartNoEarlierThan,
            ConstraintKind::StartNoLaterThan,
            ConstraintKind::FinishNoEarlierThan,
            ConstraintKind::FinishNoLaterThan,
        ];
        for (code, kind) in (0_i64..).zip(kinds) {
            assert_eq!(constraint_type(code), Some(kind));
        }
        assert_eq!(constraint_type(8), None);
        assert_eq!(task_type(0), Some(TaskType::FixedUnits));
        assert_eq!(task_type(1), Some(TaskType::FixedDuration));
        assert_eq!(task_type(2), Some(TaskType::FixedWork));
        assert_eq!(task_type(3), None);
        assert!(
            OUT.iter()
                .all(|c| constraint_type(*c).is_none() && task_type(*c).is_none())
        );
    }

    #[test]
    fn days_and_weeks() {
        assert_eq!(day_type(0), Some(DayType::LegacyException));
        for (code, d) in (1_i64..).zip(Weekday::ALL) {
            assert_eq!(day_type(code), Some(DayType::Weekday(d)));
        }
        assert_eq!(day_type(8), None);
        for (code, d) in (0_i64..).zip(Weekday::ALL) {
            assert_eq!(week_start_day(code), Some(d));
        }
        assert_eq!(week_start_day(7), None);
        let w = days_of_week(1 | 4 | 64).unwrap();
        assert!(w.sunday && w.tuesday && w.saturday);
        assert!(!w.monday && !w.wednesday && !w.thursday && !w.friday);
        assert_eq!(days_of_week(0).unwrap(), planrs_core::Week::default());
        assert!(days_of_week(127).is_some());
        assert_eq!(days_of_week(128), None);
        assert_eq!(days_of_week(-1), None);
        assert!(
            OUT.iter()
                .all(|c| day_type(*c).is_none() && week_start_day(*c).is_none())
        );
    }

    #[test]
    fn exceptions_and_months() {
        let types = [
            ExceptionType::Daily,
            ExceptionType::YearlyByDay,
            ExceptionType::YearlyByPosition,
            ExceptionType::MonthlyByDay,
            ExceptionType::MonthlyByPosition,
            ExceptionType::Weekly,
            ExceptionType::ByDayCount,
            ExceptionType::ByWeekdayCount,
            ExceptionType::Period,
        ];
        for (code, t) in (1_i64..).zip(types) {
            assert_eq!(exception_type(code), Some(t));
        }
        assert_eq!(exception_type(0), None);
        assert_eq!(exception_type(10), None);
        for (code, mo) in (0_i64..).zip(Month::ALL) {
            assert_eq!(month(code), Some(mo));
        }
        assert_eq!(month(12), None);
        assert_eq!(month_item(0), Some(MonthItem::Day));
        assert_eq!(month_item(1), Some(MonthItem::Weekday));
        assert_eq!(month_item(2), Some(MonthItem::WeekendDay));
        for (code, d) in (3_i64..).zip(Weekday::ALL) {
            assert_eq!(month_item(code), Some(MonthItem::On(d)));
        }
        assert_eq!(month_item(10), None);
        let positions = [
            MonthPosition::First,
            MonthPosition::Second,
            MonthPosition::Third,
            MonthPosition::Fourth,
            MonthPosition::Last,
        ];
        for (code, p) in (0_i64..).zip(positions) {
            assert_eq!(month_position(code), Some(p));
        }
        assert_eq!(month_position(5), None);
        assert!(OUT.iter().all(|c| {
            exception_type(*c).is_none()
                && month(*c).is_none()
                && month_item(*c).is_none()
                && month_position(*c).is_none()
        }));
    }

    #[test]
    fn work_formats() {
        let units = [
            DurationUnit::Minute,
            DurationUnit::Hour,
            DurationUnit::Day,
            DurationUnit::Week,
            DurationUnit::Month,
        ];
        for (code, u) in (1_i64..).zip(units) {
            assert_eq!(work_format(code), Some(u));
        }
        assert_eq!(work_format(0), None);
        assert_eq!(work_format(6), None);
        assert!(OUT.iter().all(|c| work_format(*c).is_none()));
    }
}
