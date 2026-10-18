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
mod r035_stable_topological_order;
mod r036_project_start_floor;
mod r037_finish_to_start;
mod r038_start_to_start;
mod r039_finish_to_finish;
mod r040_start_to_finish;
mod r041_lag_calendar;
mod r042_negative_lag;
mod r043_percent_lag;
mod r044_multiple_predecessors;
mod r045_project_finish;
mod r046_late_finish_without_successor;
mod r047_backward_transfer;
mod r048_total_slack;
mod r049_free_slack;
mod r050_critical_threshold;
mod r051_multiple_critical_paths;
mod r052_outline_hierarchy;
mod r053_summary_dates;
mod r054_summary_duration;
mod r055_summary_percent_complete;
mod r056_summary_predecessor_gates_subtasks;
mod r057_summary_successor;
mod r058_finish_link_into_summary;
mod r059_summary_descendant_link;
mod r060_summary_slack_and_critical;
mod r061_milestone_in_network;
mod r062_inactive_task;
mod r063_manual_task;
mod r064_manual_summary;
mod r065_link_cycle;
mod r066_task_calendar_fallback;
mod r067_elapsed_duration_task;
mod r068_project_summary;
mod r069_unschedulable_task;
mod r070_iso_duration;
mod r071_duration_format;
mod r072_link_lag_tenths;
mod r073_percent_lag_encoding;
mod r074_link_type;
mod r075_datetime_without_timezone;
mod r076_booleans_and_schema_defaults;
mod r077_working_times;
mod r078_week_days;
mod r079_base_or_derived_calendar;
mod r080_calendar_exceptions_import;
mod r081_legacy_exceptions;
mod r082_work_weeks_import;
mod r083_recorded_slack_tenths;
mod r084_project_summary_and_task_identity;
mod r085_manual_tasks_import;
mod r086_cpm_options_import;
mod r087_document_header_and_canonical_form;
mod r088_element_order;
mod r089_scalar_formats_and_inverse_codes;
mod r090_constraint_date_normalized;
mod r091_constraint_date_without_time;
mod r092_as_soon_as_possible;
mod r093_start_no_earlier_than;
mod r094_finish_no_earlier_than;
mod r095_start_no_later_than;
mod r096_finish_no_later_than;
mod r097_must_start_on;
mod r098_must_finish_on;
mod r099_conflict_constraints_honored;
mod r100_conflict_links_win;
mod r101_constraint_vs_project_start_or_gate;
mod r102_summary_constraint;
mod r103_manual_or_inactive_constraint;
mod r104_constraint_links_and_critical_paths;
mod r105_alap_placement_pass;
mod r106_placement_without_slack_and_summaries;
mod r107_planned_start_driver;
mod r108_deadline;
mod r109_summary_deadline;
mod r110_backward_chaining;
mod r111_backward_project_bounds;
mod r112_backward_placement_by_constraint;
mod r113_backward_constraints_and_conflicts;
mod r114_backward_summaries;
mod r115_backward_manual_and_inactive;
mod r116_backward_conversion_failure;
mod r117_backward_multiple_critical_paths;
mod r118_backward_reschedule_from_start;
mod r119_from_finish_anchor_mspdi;
mod r120_project_fields_written;
mod r121_computed_task_values_written;
mod r122_identity_hierarchy_special_rows_written;
mod r123_manual_tasks_written;
mod r124_constraints_and_deadline_written;
mod r125_links_written;
mod r126_calendars_written;
mod r127_unrepresentable_values_and_losses;
mod r128_opaque_capture_and_cap;
mod r129_reemission_validated_and_bounded;

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
