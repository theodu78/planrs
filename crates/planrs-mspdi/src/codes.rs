//! Tables des codes numériques MSPDI → types du cœur (ADR-0008 : aucun code
//! MSPDI dans `planrs-core`), et leurs inverses pour l'écriture (R-089, J3).
//!
//! Source : schéma `mspdi_pj15.xsd` (Project 2013 SDK) et pages
//! learn.microsoft.com « XML Schema for the Tasks / Calendars / Project
//! Element » (S1 à S4). Une fonction par table ; un code hors liste rend
//! `None`, que l'appelant signale (`mspdi.unknown_code`) avant d'appliquer la
//! valeur par défaut.
//!
//! Chaque table a son inverse `*_code` (R-089), totale sur le domaine du
//! type : pour tout code valide `c`, `encode(decode(c)) == c`, et pour toute
//! valeur `v`, `decode(encode(v)) == Some(v)` (bijection testée de façon
//! exhaustive).

use planrs_core::calendar::{MonthItem, MonthPosition};
use planrs_core::{DurationUnit, Lag, LagValue, LinkType, Month, TaskType, TimeBasis, Week, Weekday};

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

/// Type de contrainte, sans sa date (`ConstraintType`, 0 à 7) : le type du
/// cœur, réexporté (J3, lot L0 ; l'ancienne énumération locale avait les
/// mêmes variantes).
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

// ---------------------------------------------------------------------------
// Inverses (écriture, R-089) : une fonction par table, totale.
// ---------------------------------------------------------------------------

/// Inverse de [`link_type`] : FF 0, FS 1, SF 2, SS 3.
#[must_use]
pub const fn link_type_code(kind: LinkType) -> i64 {
    match kind {
        LinkType::FinishToFinish => 0,
        LinkType::FinishToStart => 1,
        LinkType::StartToFinish => 2,
        LinkType::StartToStart => 3,
    }
}

/// Code de base (non estimé) d'une unité de temps : 3 m … 12 emois.
const fn time_code(unit: DurationUnit, basis: TimeBasis) -> i64 {
    match (unit, basis) {
        (DurationUnit::Minute, TimeBasis::Working) => 3,
        (DurationUnit::Minute, TimeBasis::Elapsed) => 4,
        (DurationUnit::Hour, TimeBasis::Working) => 5,
        (DurationUnit::Hour, TimeBasis::Elapsed) => 6,
        (DurationUnit::Day, TimeBasis::Working) => 7,
        (DurationUnit::Day, TimeBasis::Elapsed) => 8,
        (DurationUnit::Week, TimeBasis::Working) => 9,
        (DurationUnit::Week, TimeBasis::Elapsed) => 10,
        (DurationUnit::Month, TimeBasis::Working) => 11,
        (DurationUnit::Month, TimeBasis::Elapsed) => 12,
    }
}

/// Inverse de [`duration_format`] : code de base, + 32 pour la variante
/// estimée. Exemples : jour ouvré → 7 ; jour ouvré estimé → 39 ; `e%` → 20 ;
/// nul estimé → 53.
#[must_use]
pub const fn duration_format_code(code: DurationCode) -> i64 {
    let (base, estimated) = match code {
        DurationCode::Time {
            unit,
            basis,
            estimated,
        } => (time_code(unit, basis), estimated),
        DurationCode::Percent {
            basis: TimeBasis::Working,
            estimated,
        } => (19, estimated),
        DurationCode::Percent {
            basis: TimeBasis::Elapsed,
            estimated,
        } => (20, estimated),
        DurationCode::Null { estimated } => (21, estimated),
    };
    if estimated {
        // Constantes : 21 + 32 au plus, aucun débordement possible.
        base.saturating_add(ESTIMATED_OFFSET)
    } else {
        base
    }
}

/// `DurationFormat` d'une durée de tâche : code de son unité et de sa base,
/// + 32 si elle est estimée (raccourci de [`duration_format_code`]).
#[must_use]
pub const fn duration_code(unit: DurationUnit, basis: TimeBasis, estimated: bool) -> i64 {
    duration_format_code(DurationCode::Time {
        unit,
        basis,
        estimated,
    })
}

/// `LagFormat` d'un décalage : code de durée non estimé de son unité et de sa
/// base (temps), 19 (`%`) ou 20 (`e%`) pour un pourcentage. Un décalage nul
/// en jours ouvrés s'écrit 7 (observé : `LinkLag 0`, `LagFormat 7`).
#[must_use]
pub const fn lag_format_code(lag: &Lag) -> i64 {
    match lag.value {
        LagValue::Time { unit, .. } => duration_code(unit, lag.basis, false),
        LagValue::Percent { .. } => duration_format_code(DurationCode::Percent {
            basis: lag.basis,
            estimated: false,
        }),
    }
}

/// Inverse de [`constraint_type`] : 0 ASAP, 1 ALAP, 2 MSO, 3 MFO, 4 SNET,
/// 5 SNLT, 6 FNET, 7 FNLT.
#[must_use]
pub const fn constraint_type_code(kind: ConstraintKind) -> i64 {
    match kind {
        ConstraintKind::AsSoonAsPossible => 0,
        ConstraintKind::AsLateAsPossible => 1,
        ConstraintKind::MustStartOn => 2,
        ConstraintKind::MustFinishOn => 3,
        ConstraintKind::StartNoEarlierThan => 4,
        ConstraintKind::StartNoLaterThan => 5,
        ConstraintKind::FinishNoEarlierThan => 6,
        ConstraintKind::FinishNoLaterThan => 7,
    }
}

/// Inverse de [`task_type`] : 0 unités fixes, 1 durée fixe, 2 travail fixe.
#[must_use]
pub const fn task_type_code(kind: TaskType) -> i64 {
    match kind {
        TaskType::FixedUnits => 0,
        TaskType::FixedDuration => 1,
        TaskType::FixedWork => 2,
    }
}

/// Rang d'un jour de semaine depuis le dimanche (0 dimanche … 6 samedi).
const fn weekday_index(d: Weekday) -> i64 {
    match d {
        Weekday::Sunday => 0,
        Weekday::Monday => 1,
        Weekday::Tuesday => 2,
        Weekday::Wednesday => 3,
        Weekday::Thursday => 4,
        Weekday::Friday => 5,
        Weekday::Saturday => 6,
    }
}

/// Inverse de [`day_type`] : 0 ancienne exception, 1 dimanche … 7 samedi.
#[must_use]
pub const fn day_type_code(day: DayType) -> i64 {
    match day {
        DayType::LegacyException => 0,
        // Rang 0 à 6, + 1 : aucun débordement possible.
        DayType::Weekday(d) => weekday_index(d).saturating_add(1),
    }
}

/// Inverse de [`week_start_day`] : 0 dimanche … 6 samedi.
#[must_use]
pub const fn week_start_day_code(day: Weekday) -> i64 {
    weekday_index(day)
}

/// Inverse de [`exception_type`] : 1 à 9.
#[must_use]
pub const fn exception_type_code(kind: ExceptionType) -> i64 {
    match kind {
        ExceptionType::Daily => 1,
        ExceptionType::YearlyByDay => 2,
        ExceptionType::YearlyByPosition => 3,
        ExceptionType::MonthlyByDay => 4,
        ExceptionType::MonthlyByPosition => 5,
        ExceptionType::Weekly => 6,
        ExceptionType::ByDayCount => 7,
        ExceptionType::ByWeekdayCount => 8,
        ExceptionType::Period => 9,
    }
}

/// Inverse de [`month`] : 0 janvier … 11 décembre.
#[must_use]
pub const fn month_code(m: Month) -> i64 {
    match m {
        Month::January => 0,
        Month::February => 1,
        Month::March => 2,
        Month::April => 3,
        Month::May => 4,
        Month::June => 5,
        Month::July => 6,
        Month::August => 7,
        Month::September => 8,
        Month::October => 9,
        Month::November => 10,
        Month::December => 11,
    }
}

/// Inverse de [`month_item`] : 0 jour, 1 jour ouvré, 2 jour de week-end,
/// 3 dimanche … 9 samedi.
#[must_use]
pub const fn month_item_code(item: MonthItem) -> i64 {
    match item {
        MonthItem::Day => 0,
        MonthItem::Weekday => 1,
        MonthItem::WeekendDay => 2,
        // Rang 0 à 6, + 3 : aucun débordement possible.
        MonthItem::On(d) => weekday_index(d).saturating_add(3),
    }
}

/// Inverse de [`month_position`] : 0 premier … 3 quatrième, 4 dernier.
#[must_use]
pub const fn month_position_code(position: MonthPosition) -> i64 {
    match position {
        MonthPosition::First => 0,
        MonthPosition::Second => 1,
        MonthPosition::Third => 2,
        MonthPosition::Fourth => 3,
        MonthPosition::Last => 4,
    }
}

/// Bit `value` si `set`, sinon 0.
const fn bit(set: bool, value: i64) -> i64 {
    if set { value } else { 0 }
}

/// Inverse de [`days_of_week`] : masque de bits, 1 = dimanche, 2 = lundi …
/// 64 = samedi (0 à 127). Exemple : dimanche, mardi, samedi → 69.
#[must_use]
pub const fn days_of_week_mask(days: &Week<bool>) -> i64 {
    bit(days.sunday, 1)
        | bit(days.monday, 2)
        | bit(days.tuesday, 4)
        | bit(days.wednesday, 8)
        | bit(days.thursday, 16)
        | bit(days.friday, 32)
        | bit(days.saturday, 64)
}

/// Inverse de [`work_format`] : 1 m, 2 h, 3 j, 4 sem, 5 mois.
#[must_use]
pub const fn work_format_code(unit: DurationUnit) -> i64 {
    match unit {
        DurationUnit::Minute => 1,
        DurationUnit::Hour => 2,
        DurationUnit::Day => 3,
        DurationUnit::Week => 4,
        DurationUnit::Month => 5,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::arithmetic_side_effects,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::{
        ConstraintKind, DayType, DurationCode, ExceptionType, constraint_type, constraint_type_code,
        day_type, day_type_code, days_of_week, days_of_week_mask, duration_code, duration_format,
        duration_format_code, exception_type, exception_type_code, lag_format_code, link_type,
        link_type_code, month, month_code, month_item, month_item_code, month_position, month_position_code,
        task_type, task_type_code, week_start_day, week_start_day_code, work_format, work_format_code,
    };
    use planrs_core::calendar::{MonthItem, MonthPosition};
    use planrs_core::{DurationUnit, LinkType, Month, TaskType, TimeBasis, Weekday};
    use planrs_core::{Lag, LagValue, Minutes, SignedPercent};

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

    /// Codes parcourus pour la bijection : tout code valide de chaque table
    /// est dans `-5..=200`, plus des valeurs extrêmes.
    fn probe_codes() -> impl Iterator<Item = i64> {
        (-5_i64..=200).chain([i64::MIN, i64::MAX, 1_000])
    }

    /// `encode(decode(c)) == c` pour tout code valide ; rend le nombre de
    /// codes valides vus.
    fn assert_encode_decode<T>(decode: impl Fn(i64) -> Option<T>, encode: impl Fn(T) -> i64) -> usize {
        let mut valid = 0;
        for c in probe_codes() {
            if let Some(v) = decode(c) {
                assert_eq!(encode(v), c, "code {c}");
                valid += 1;
            }
        }
        valid
    }

    #[test]
    fn inverse_tables_are_bijections() {
        // encode ∘ decode = id sur les codes valides (et le nombre de codes
        // valides est celui de la table : aucun code oublié).
        assert_eq!(assert_encode_decode(link_type, link_type_code), 4);
        assert_eq!(assert_encode_decode(duration_format, duration_format_code), 26);
        assert_eq!(assert_encode_decode(constraint_type, constraint_type_code), 8);
        assert_eq!(assert_encode_decode(task_type, task_type_code), 3);
        assert_eq!(assert_encode_decode(day_type, day_type_code), 8);
        assert_eq!(assert_encode_decode(week_start_day, week_start_day_code), 7);
        assert_eq!(assert_encode_decode(exception_type, exception_type_code), 9);
        assert_eq!(assert_encode_decode(month, month_code), 12);
        assert_eq!(assert_encode_decode(month_item, month_item_code), 10);
        assert_eq!(assert_encode_decode(month_position, month_position_code), 5);
        assert_eq!(assert_encode_decode(work_format, work_format_code), 5);
        assert_eq!(assert_encode_decode(days_of_week, |w| days_of_week_mask(&w)), 128);
        // decode ∘ encode = id sur le domaine : chaque valeur décodée a un
        // code distinct (injectivité de l'encodage) — vérifié par la
        // fonction ci-dessus — et toute valeur du domaine est atteinte :
        // les comptes ci-dessus égalent le cardinal de chaque type.
    }

    #[test]
    fn duration_and_lag_codes() {
        use DurationUnit::{Day, Hour, Minute, Month as Mo, Week};
        use TimeBasis::{Elapsed, Working};
        assert_eq!(duration_code(Day, Working, false), 7);
        assert_eq!(duration_code(Day, Working, true), 39);
        assert_eq!(duration_code(Minute, Working, false), 3);
        assert_eq!(duration_code(Mo, Elapsed, true), 44);
        for unit in [Minute, Hour, Day, Week, Mo] {
            for basis in [Working, Elapsed] {
                for estimated in [false, true] {
                    let v = DurationCode::Time {
                        unit,
                        basis,
                        estimated,
                    };
                    assert_eq!(duration_format(duration_format_code(v)), Some(v));
                }
            }
        }
        // Décalage nul observé : `LinkLag 0`, `LagFormat 7`.
        assert_eq!(lag_format_code(&Lag::ZERO), 7);
        let hours = Lag::new(
            Elapsed,
            LagValue::Time {
                minutes: Minutes::new(120),
                unit: Hour,
            },
        )
        .unwrap();
        assert_eq!(lag_format_code(&hours), 6);
        let percent = |basis| {
            Lag::new(
                basis,
                LagValue::Percent {
                    percent: SignedPercent::new(5_000).unwrap(),
                },
            )
            .unwrap()
        };
        assert_eq!(lag_format_code(&percent(Working)), 19);
        assert_eq!(lag_format_code(&percent(Elapsed)), 20);
    }

    #[test]
    fn masks_and_weekdays() {
        let w = days_of_week(1 | 4 | 64).unwrap();
        assert_eq!(days_of_week_mask(&w), 69);
        assert_eq!(days_of_week_mask(&planrs_core::Week::default()), 0);
        for (code, d) in (1_i64..).zip(Weekday::ALL) {
            assert_eq!(day_type_code(DayType::Weekday(d)), code);
            assert_eq!(week_start_day_code(d), code - 1);
            assert_eq!(month_item_code(MonthItem::On(d)), code + 2);
        }
        assert_eq!(day_type_code(DayType::LegacyException), 0);
        assert_eq!(exception_type_code(ExceptionType::Period), 9);
        assert_eq!(constraint_type_code(ConstraintKind::FinishNoLaterThan), 7);
        assert_eq!(month_code(Month::December), 11);
        assert_eq!(month_position_code(MonthPosition::Last), 4);
        assert_eq!(task_type_code(TaskType::FixedDuration), 1);
        assert_eq!(work_format_code(DurationUnit::Week), 4);
        assert_eq!(link_type_code(LinkType::StartToStart), 3);
    }
}
