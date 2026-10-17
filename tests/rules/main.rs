//! Tests des règles de calcul `R-xxx` (une règle = au moins un test).
//!
//! Convention (voir `tests/rules/README.md`) : un fichier par règle, nommé
//! `rNNN_titre_court.rs`, déclaré ci-dessous par `mod rNNN_titre_court;`.
//! Chaque test porte l'identifiant de sa règle dans son nom et commente ses
//! valeurs calculées à la main.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

// Modules des règles, un par fichier, dans l'ordre des identifiants.
mod r001_rounding_half_away_from_zero;
mod r002_fixed_point_quantities;
mod r003_date_domain;
mod r004_seconds_truncated;
mod r005_local_time_no_dst;
mod r006_working_unit_factors;
mod r007_elapsed_unit_factors;
mod r008_duration_sign;
mod r009_stored_minutes_options;
mod r010_percent_lag_conversion;
mod r011_time_ranges;
mod r012_standard_calendar;
mod r013_24_hours_calendar;
mod r014_night_shift_calendar;
mod r015_calendar_inheritance;
mod r016_resolution_precedence;
mod r017_calendar_exceptions;
mod r018_recurring_exceptions;
mod r019_work_weeks;
mod r020_working_between;
mod r021_add_working_forward;
mod r022_add_working_backward;
mod r023_add_working_zero;
mod r024_next_working_moment;
mod r025_previous_working_moment;
mod r026_day_uses_minutes_per_day;
mod r027_weekend_and_holiday_skipped;
mod r028_elapsed_time;
mod r029_no_working_time_and_domain;
mod r030_task_span;
mod r031_milestone_placement;
mod r032_new_project_defaults;
mod r033_date_without_time;
mod r034_effective_status_date;

/// Vérifie que chaque fichier de `tests/rules/` suit la convention
/// `rNNN_titre.rs` et qu'il est bien déclaré comme module dans `main.rs`
/// (sinon ses tests ne tourneraient jamais).
#[test]
fn rule_files_follow_naming_convention_and_are_declared() {
    let dir = planrs_tests::tests_path("rules");
    let main = std::fs::read_to_string(dir.join("main.rs")).expect("lecture de rules/main.rs");
    for entry in std::fs::read_dir(&dir).expect("lecture de tests/rules") {
        let path = entry.expect("entrée de dossier").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("nom de fichier UTF-8");
        if stem == "main" {
            continue;
        }
        assert!(
            is_rule_module_name(stem),
            "{stem}.rs ne suit pas la convention rNNN_titre.rs"
        );
        let declaration = format!("\nmod {stem};");
        assert!(
            main.contains(&declaration),
            "{stem}.rs n'est pas déclaré dans rules/main.rs"
        );
    }
}

/// `r` + trois chiffres + `_` + titre en minuscules ASCII, chiffres ou `_`.
fn is_rule_module_name(stem: &str) -> bool {
    let bytes = stem.as_bytes();
    bytes.len() > 5
        && bytes[0] == b'r'
        && bytes[1..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'_'
        && bytes[5..]
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
}

#[test]
fn naming_convention_examples() {
    assert!(is_rule_module_name("r001_fs_link_lag"));
    assert!(!is_rule_module_name("r1_fs"));
    assert!(!is_rule_module_name("R001_fs"));
    assert!(!is_rule_module_name("r001"));
    assert!(!is_rule_module_name("r001_Titre"));
}
