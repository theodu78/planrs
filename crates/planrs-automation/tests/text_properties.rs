//! P29 — Texte : `parse(format(x)) == x` pour les durées et les décalages,
//! en français et en anglais, avec des facteurs de projet quelconques ; une
//! chaîne arbitraire donne un `Result`, jamais une panique (règle 10).
//!
//! Nombre de cas fixé ici (2 048) : la variable `PROPTEST_CASES` de la CI
//! (10 000) vise les propriétés du moteur.

#![allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

use planrs_automation::text::{
    Language, MAX_INPUT_CHARS, TextContext, TextError, format_duration, format_lag, parse_duration, parse_lag,
};
use planrs_core::{
    ConversionFactors, Duration, DurationUnit, Lag, LagValue, Minutes, ProjectOptions, SignedPercent,
    TimeBasis,
};
use proptest::prelude::*;

fn factors() -> impl Strategy<Value = ConversionFactors> {
    prop_oneof![
        Just(ConversionFactors::default()),
        (1_u32..=1_440, 1_u32..=10_080, 1_u32..=31)
            .prop_map(|(d, w, m)| ConversionFactors::new(d, w, m).unwrap()),
    ]
}

fn unit() -> impl Strategy<Value = DurationUnit> {
    prop_oneof![
        Just(DurationUnit::Minute),
        Just(DurationUnit::Hour),
        Just(DurationUnit::Day),
        Just(DurationUnit::Week),
        Just(DurationUnit::Month),
    ]
}

fn basis() -> impl Strategy<Value = TimeBasis> {
    prop_oneof![Just(TimeBasis::Working), Just(TimeBasis::Elapsed)]
}

fn language() -> impl Strategy<Value = Language> {
    prop_oneof![Just(Language::Fr), Just(Language::En)]
}

fn minutes() -> impl Strategy<Value = i64> {
    prop_oneof![0_i64..=10_000, 0_i64..=Duration::MAX_MINUTES]
}

fn context(f: ConversionFactors, language: Language) -> TextContext {
    let options = ProjectOptions {
        factors: f,
        ..ProjectOptions::default()
    };
    TextContext::for_options(&options, language)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_048))]

    #[test]
    fn p29_duration_round_trip(
        f in factors(), lang in language(), m in minutes(), u in unit(), b in basis(),
        est in any::<bool>(),
    ) {
        let d = Duration::new(Minutes::new(m), u, b, est).unwrap();
        let ctx = context(f, lang);
        let text = format_duration(&d, &ctx);
        prop_assert_eq!(parse_duration(&text, &ctx), Ok(d), "{}", text);
        // La lecture accepte l'autre langue (marque et abréviations).
        let other = context(f, match lang { Language::Fr => Language::En, Language::En => Language::Fr });
        prop_assert_eq!(parse_duration(&text, &other), Ok(d), "{}", text);
    }

    #[test]
    fn p29_lag_round_trip(
        f in factors(), lang in language(), m in minutes(), neg in any::<bool>(), u in unit(),
        b in basis(), pct in -1_000_000_i64..=1_000_000, is_pct in any::<bool>(),
    ) {
        let value = if is_pct {
            LagValue::Percent { percent: SignedPercent::new(pct).unwrap() }
        } else {
            LagValue::Time { minutes: Minutes::new(if neg { -m } else { m }), unit: u }
        };
        let l = Lag::new(b, value).unwrap();
        let ctx = context(f, lang);
        let text = format_lag(&l, &ctx);
        prop_assert!(text.starts_with('+') || text.starts_with('-'), "{}", text);
        prop_assert_eq!(parse_lag(&text, &ctx), Ok(l), "{}", text);
    }

    #[test]
    fn p29_arbitrary_text_never_panics(s in "\\PC{0,40}", lang in language()) {
        let ctx = context(ConversionFactors::default(), lang);
        let _ = parse_duration(&s, &ctx);
        let _ = parse_lag(&s, &ctx);
    }

    #[test]
    fn p29_text_like_input_never_panics(
        s in "[ +\\-0-9,.\u{a0}\u{202f}?%eéjdhmswoJDHMSWOÉ]{0,24}", lang in language(),
    ) {
        let ctx = context(ConversionFactors::default(), lang);
        if let Ok(d) = parse_duration(&s, &ctx) {
            // Ce qui est lu se réécrit et se relit à l'identique.
            prop_assert_eq!(parse_duration(&format_duration(&d, &ctx), &ctx), Ok(d));
        }
        if let Ok(l) = parse_lag(&s, &ctx) {
            prop_assert_eq!(parse_lag(&format_lag(&l, &ctx), &ctx), Ok(l));
        }
    }
}

#[test]
fn p29_long_input_is_refused() {
    let ctx = context(ConversionFactors::default(), Language::Fr);
    let long = "9".repeat(MAX_INPUT_CHARS + 1);
    assert_eq!(parse_duration(&long, &ctx), Err(TextError::TooLong));
    assert_eq!(parse_lag(&long, &ctx), Err(TextError::TooLong));
}
