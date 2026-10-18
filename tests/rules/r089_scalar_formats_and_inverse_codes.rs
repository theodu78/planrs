//! R-089 — Formats scalaires et codes inverses.
//!
//! Lot W1 (J3, ADR-0010). Formateurs de `planrs_mspdi::scalar` (`push_*` dans
//! un tampon, `format_*` en `String`) et inverses `*_code` de
//! `planrs_mspdi::codes`. Valeurs attendues calculées à la main ; chaque
//! formateur est vérifié contre le lecteur correspondant (R-070, R-072,
//! R-073, R-075, R-077) : la relecture rend la valeur écrite.

use planrs_core::calendar::{MonthItem, MonthPosition};
use planrs_core::{
    ConstraintKind, DurationUnit, Lag, LagValue, LinkType, Minutes, Moment, Month, SignedPercent, TaskType,
    TimeBasis, TimeOfDay, Week, Weekday,
};
use planrs_mspdi::codes::{
    self, DayType, DurationCode, ExceptionType, constraint_type_code, day_type_code, days_of_week_mask,
    duration_code, duration_format_code, exception_type_code, lag_format_code, link_type_code, month_code,
    month_item_code, month_position_code, task_type_code, week_start_day_code, work_format_code,
};
use planrs_mspdi::scalar::{
    format_bool, format_datetime, format_int, format_iso_duration, format_time, link_lag_from_percent_lag,
    minutes_to_tenths, parse_datetime, parse_iso_duration, parse_time, percent_lag_from_link_lag,
    percent_to_whole, push_datetime, push_escaped_text, push_iso_duration, tenths_to_minutes,
};

fn m(s: &str) -> Moment {
    s.parse().unwrap()
}

/// Dates `AAAA-MM-JJTHH:MM:00`, `Moment::MAX` → `2150-01-01T00:00:00`.
#[test]
fn r089_datetimes_written_with_zero_seconds() {
    // lun 19/10/2026 08:00.
    assert_eq!(format_datetime(m("2026-10-19T08:00")), "2026-10-19T08:00:00");
    // ven 30/10/2026 17:00 (fin d'une chaîne de 10 jours, conception B § 4.2).
    assert_eq!(format_datetime(m("2026-10-30T17:00")), "2026-10-30T17:00:00");
    assert_eq!(format_datetime(Moment::MIN), "1984-01-01T00:00:00");
    // 31/12/2149 à 24:00 = 2150-01-01 00:00, relu comme `Moment::MAX` (R-075).
    assert_eq!(format_datetime(Moment::MAX), "2150-01-01T00:00:00");
    assert_eq!(parse_datetime("2150-01-01T00:00:00").unwrap().moment, Moment::MAX);
    // Tampon réutilisé : les écritures s'ajoutent.
    let mut buf = String::new();
    push_datetime(&mut buf, m("2026-12-24T00:00"));
    buf.push('|');
    push_datetime(&mut buf, m("2026-12-26T23:59"));
    assert_eq!(buf, "2026-12-24T00:00:00|2026-12-26T23:59:00");
    // Aller-retour : chaque jour du domaine, à 00:00, 08:00, 17:30, 23:59.
    for day in 0..60_631_i64 {
        for t in [0_i64, 480, 1_050, 1_439] {
            let mo = Moment::from_minutes(day * 1_440 + t).unwrap();
            let p = parse_datetime(&format_datetime(mo)).unwrap();
            assert_eq!(p.moment, mo);
            assert!(!p.rounded && !p.zone_ignored);
        }
    }
}

/// Heures `HH:MM:00` ; `ToTime` 24:00 → `00:00:00` (inverse de R-077).
#[test]
fn r089_times_written_end_of_day_as_midnight() {
    assert_eq!(format_time(TimeOfDay::hm(8, 0).unwrap()), "08:00:00");
    assert_eq!(format_time(TimeOfDay::hm(17, 0).unwrap()), "17:00:00");
    assert_eq!(format_time(TimeOfDay::MIDNIGHT), "00:00:00");
    assert_eq!(format_time(TimeOfDay::END_OF_DAY), "00:00:00");
    for minutes in 0..1_440_u16 {
        let t = TimeOfDay::new(minutes).unwrap();
        assert_eq!(parse_time(&format_time(t)), Some((t, false)));
    }
}

/// Durées `PT{m/60}H{m%60}M0S`.
#[test]
fn r089_iso_durations_hours_minutes() {
    // 510 = 8 × 60 + 30.
    assert_eq!(format_iso_duration(Minutes::new(510)), "PT8H30M0S");
    // 4 800 = 80 × 60 (10 jours de 8 h).
    assert_eq!(format_iso_duration(Minutes::new(4_800)), "PT80H0M0S");
    // 87 308 640 = 1 455 144 × 60 (borne du domaine).
    assert_eq!(format_iso_duration(Minutes::new(87_308_640)), "PT1455144H0M0S");
    assert_eq!(format_iso_duration(Minutes::ZERO), "PT0H0M0S");
    assert_eq!(format_iso_duration(Minutes::new(-480)), "-PT8H0M0S");
    let mut buf = String::from("x");
    push_iso_duration(&mut buf, Minutes::new(61));
    assert_eq!(buf, "xPT1H1M0S");
    for v in (0..=87_308_640_i64).step_by(9_973).chain([87_308_640]) {
        let back = parse_iso_duration(&format_iso_duration(Minutes::new(v))).unwrap();
        assert_eq!((back.minutes, back.rounded), (v, false));
    }
}

/// Dixièmes = minutes × 10 ; pourcentages par R-001 (perte signalée) ;
/// décalage en % : `LinkLag = round_div(p, 10)` (inverse de R-073).
#[test]
fn r089_tenths_and_percentages() {
    assert_eq!(minutes_to_tenths(Minutes::new(960)), Some(9_600));
    assert_eq!(minutes_to_tenths(Minutes::new(-2_880)), Some(-28_800));
    assert_eq!(format_int(-28_800), "-28800");
    for v in [-87_308_640_i64, -1, 0, 1, 960, 87_308_640] {
        let tenths = minutes_to_tenths(Minutes::new(v)).unwrap();
        assert_eq!(tenths_to_minutes(tenths), (Minutes::new(v), false));
    }
    // 5 000 / 100 = 50 ; 5 050 / 100 = 50,5 → 51 (moitié loin de zéro), perte.
    assert_eq!(percent_to_whole(5_000), (50, false));
    assert_eq!(percent_to_whole(5_050), (51, true));
    // 5 000 / 10 = 500 ; 5 005 / 10 = 500,5 → 501, perte.
    assert_eq!(link_lag_from_percent_lag(5_000), (500, false));
    assert_eq!(link_lag_from_percent_lag(5_005), (501, true));
    assert_eq!(percent_lag_from_link_lag(500), Some(5_000));
    assert_eq!(format_bool(true), "1");
    assert_eq!(format_bool(false), "0");
}

/// Texte : `&`, `<`, `>` échappés, `\r` → `&#13;`, caractère interdit →
/// U+FFFD avec perte.
#[test]
fn r089_text_escaped() {
    let mut out = String::new();
    assert!(!push_escaped_text(&mut out, "R&D <phase 1>\r\nfin"));
    assert_eq!(out, "R&amp;D &lt;phase 1&gt;&#13;\nfin");
    let mut out = String::new();
    assert!(push_escaped_text(&mut out, "a\u{1}b"));
    assert_eq!(out, "a\u{FFFD}b");
}

/// Probe : tout code valide est dans `-5..=200`.
fn probe() -> impl Iterator<Item = i64> {
    (-5_i64..=200).chain([i64::MIN, -1_000, 1_000, i64::MAX])
}

/// `encode(decode(c)) == c` pour tout code valide ; rend les valeurs
/// décodées (pour vérifier `decode(encode(v)) == Some(v)` sur le domaine).
fn check_table<T: Copy + PartialEq + std::fmt::Debug>(
    decode: impl Fn(i64) -> Option<T>,
    encode: impl Fn(T) -> i64,
) -> Vec<T> {
    let mut values = Vec::new();
    for c in probe() {
        if let Some(v) = decode(c) {
            assert_eq!(encode(v), c, "code {c} : {v:?}");
            values.push(v);
        }
    }
    values
}

/// Bijection de chaque table : `encode ∘ decode = id` sur les codes valides,
/// `decode ∘ encode = id` sur tout le domaine du type.
#[test]
fn r089_inverse_codes_are_bijections() {
    // Liens : FF 0, FS 1, SF 2, SS 3.
    let links = [
        LinkType::FinishToStart,
        LinkType::StartToStart,
        LinkType::FinishToFinish,
        LinkType::StartToFinish,
    ];
    assert_eq!(check_table(codes::link_type, link_type_code).len(), links.len());
    for v in links {
        assert_eq!(codes::link_type(link_type_code(v)), Some(v));
    }
    assert_eq!(link_type_code(LinkType::FinishToStart), 1);

    // Contraintes : 0 à 7.
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
    assert_eq!(check_table(codes::constraint_type, constraint_type_code).len(), 8);
    for (code, v) in (0_i64..).zip(kinds) {
        assert_eq!(constraint_type_code(v), code);
        assert_eq!(codes::constraint_type(code), Some(v));
    }

    // Types de tâche.
    for v in [TaskType::FixedUnits, TaskType::FixedDuration, TaskType::FixedWork] {
        assert_eq!(codes::task_type(task_type_code(v)), Some(v));
    }
    assert_eq!(check_table(codes::task_type, task_type_code).len(), 3);

    // Formats de durée : 26 codes (3-12, 19-21 et leurs variantes + 32).
    let formats = check_table(codes::duration_format, duration_format_code);
    assert_eq!(formats.len(), 26);
    let units = [
        DurationUnit::Minute,
        DurationUnit::Hour,
        DurationUnit::Day,
        DurationUnit::Week,
        DurationUnit::Month,
    ];
    let mut domain = Vec::new();
    for estimated in [false, true] {
        for basis in [TimeBasis::Working, TimeBasis::Elapsed] {
            for unit in units {
                domain.push(DurationCode::Time {
                    unit,
                    basis,
                    estimated,
                });
            }
            domain.push(DurationCode::Percent { basis, estimated });
        }
        domain.push(DurationCode::Null { estimated });
    }
    assert_eq!(domain.len(), 26);
    for v in domain {
        assert_eq!(codes::duration_format(duration_format_code(v)), Some(v));
        assert!(formats.contains(&v));
    }
    // Jour ouvré 7, jour ouvré estimé 39.
    assert_eq!(duration_code(DurationUnit::Day, TimeBasis::Working, false), 7);
    assert_eq!(duration_code(DurationUnit::Day, TimeBasis::Working, true), 39);
}

/// Bijection des tables de calendrier : jours, semaine, mois, rangs,
/// exceptions, masque des jours, et `WorkFormat`.
#[test]
fn r089_calendar_codes_are_bijections() {
    let units = [
        DurationUnit::Minute,
        DurationUnit::Hour,
        DurationUnit::Day,
        DurationUnit::Week,
        DurationUnit::Month,
    ];
    // Jours, semaine, mois, rangs.
    assert_eq!(check_table(codes::day_type, day_type_code).len(), 8);
    assert_eq!(check_table(codes::week_start_day, week_start_day_code).len(), 7);
    assert_eq!(check_table(codes::month, month_code).len(), 12);
    assert_eq!(check_table(codes::month_item, month_item_code).len(), 10);
    assert_eq!(check_table(codes::month_position, month_position_code).len(), 5);
    assert_eq!(check_table(codes::work_format, work_format_code).len(), 5);
    for d in Weekday::ALL {
        assert_eq!(
            codes::day_type(day_type_code(DayType::Weekday(d))),
            Some(DayType::Weekday(d))
        );
        assert_eq!(codes::week_start_day(week_start_day_code(d)), Some(d));
        assert_eq!(
            codes::month_item(month_item_code(MonthItem::On(d))),
            Some(MonthItem::On(d))
        );
    }
    for mo in Month::ALL {
        assert_eq!(codes::month(month_code(mo)), Some(mo));
    }
    for p in [
        MonthPosition::First,
        MonthPosition::Second,
        MonthPosition::Third,
        MonthPosition::Fourth,
        MonthPosition::Last,
    ] {
        assert_eq!(codes::month_position(month_position_code(p)), Some(p));
    }
    for u in units {
        assert_eq!(codes::work_format(work_format_code(u)), Some(u));
    }

    // Exceptions : 1 à 9 ; `Period` → 9.
    assert_eq!(check_table(codes::exception_type, exception_type_code).len(), 9);
    assert_eq!(exception_type_code(ExceptionType::Period), 9);
    assert_eq!(exception_type_code(ExceptionType::Daily), 1);

    // Masque des jours : les 128 combinaisons.
    let masks = check_table(codes::days_of_week, |w: Week<bool>| days_of_week_mask(&w));
    assert_eq!(masks.len(), 128);
    // Dimanche (1) + mardi (4) + samedi (64) = 69.
    let w = Week {
        sunday: true,
        tuesday: true,
        saturday: true,
        ..Week::default()
    };
    assert_eq!(days_of_week_mask(&w), 69);
}

/// `LagFormat` : code de durée non estimé (temps), 19 ou 20 (pourcentage).
#[test]
fn r089_lag_format_code() {
    // Décalage nul observé : `LinkLag 0`, `LagFormat 7`.
    assert_eq!(lag_format_code(&Lag::ZERO), 7);
    let elapsed_days = Lag::new(
        TimeBasis::Elapsed,
        LagValue::Time {
            minutes: Minutes::new(2_880),
            unit: DurationUnit::Day,
        },
    )
    .unwrap();
    assert_eq!(lag_format_code(&elapsed_days), 8);
    let half = |basis| {
        Lag::new(
            basis,
            LagValue::Percent {
                percent: SignedPercent::new(5_000).unwrap(),
            },
        )
        .unwrap()
    };
    assert_eq!(lag_format_code(&half(TimeBasis::Working)), 19);
    assert_eq!(lag_format_code(&half(TimeBasis::Elapsed)), 20);
    // Relu : pourcentage de même base.
    assert_eq!(
        codes::duration_format(lag_format_code(&half(TimeBasis::Elapsed))),
        Some(DurationCode::Percent {
            basis: TimeBasis::Elapsed,
            estimated: false
        })
    );
}
