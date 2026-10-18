//! R-088 — Ordre des éléments, charge opaque à la place de sa clé, clé inconnue en fin.
//!
//! Lot W1 (unité), W4 (corpus) (J3, ADR-0010). Les tables d'ordre vivent dans
//! `crates/planrs-mspdi/src/write/order.rs` (source unique, module privé
//! jusqu'à W3) : ce fichier les compile tel quel par `#[path]`, sans copie.
//! Le test de sous-suite sur les cinq fichiers du corpus (lecture quick-xml)
//! et le test « toute balise des listes blanches du lecteur a un rang » sont
//! des tests unitaires de `planrs-mspdi` (`write/mod.rs`,
//! `r088_corpus_children_follow_order_tables`,
//! `r088_reader_whitelists_are_ranked`) : ils lisent des éléments privés de
//! la crate et quick-xml, qui n'est pas une dépendance de `planrs-tests`.

#[allow(dead_code, reason = "tables lues en partie seulement par ces tests")]
#[path = "../../crates/planrs-mspdi/src/write/order.rs"]
mod order;

use order::{
    CALENDAR_ORDER, EXCEPTION_ORDER, LINK_ORDER, PROJECT_ORDER, TASK_ORDER, WEEKDAY_ORDER, WORKWEEK_ORDER,
    canonical_extras, rank,
};
use planrs_core::ExtraEntry;

fn entry(key: &str, payload: &str) -> ExtraEntry {
    ExtraEntry {
        origin: "mspdi".to_owned(),
        key: key.to_owned(),
        payload: payload.to_owned(),
    }
}

fn keys_and_payloads<'a>(entries: &[&'a ExtraEntry]) -> Vec<(&'a str, &'a str)> {
    entries
        .iter()
        .map(|e| (e.key.as_str(), e.payload.as_str()))
        .collect()
}

/// Les balises écrites par l'écrivain se suivent dans l'ordre observé
/// (conception B § 4.3 : `UID`, `ID`, `Name`, …, `Start`, `Finish`,
/// `Duration`, `ManualStart`, …, `ConstraintType`, `CalendarUID`,
/// `ConstraintDate`, `Deadline`, …, `PredecessorLink`) : rangs croissants.
#[test]
fn r088_written_task_fields_follow_table() {
    let written = [
        "UID",
        "GUID",
        "ID",
        "Name",
        "Active",
        "Manual",
        "Type",
        "IsNull",
        "CreateDate",
        "WBS",
        "OutlineNumber",
        "OutlineLevel",
        "Priority",
        "Start",
        "Finish",
        "Duration",
        "ManualStart",
        "ManualFinish",
        "ManualDuration",
        "DurationFormat",
        "EffortDriven",
        "Recurring",
        "Estimated",
        "Milestone",
        "Summary",
        "Critical",
        "EarlyStart",
        "EarlyFinish",
        "LateStart",
        "LateFinish",
        "FreeSlack",
        "TotalSlack",
        "PercentComplete",
        "PercentWorkComplete",
        "ActualStart",
        "ActualFinish",
        "ActualDuration",
        "RemainingDuration",
        "ConstraintType",
        "CalendarUID",
        "ConstraintDate",
        "Deadline",
        "IgnoreResourceCalendar",
        "Notes",
        "PhysicalPercentComplete",
        "PredecessorLink",
    ];
    let ranks: Vec<usize> = written.iter().map(|n| rank(TASK_ORDER, n).unwrap()).collect();
    assert!(ranks.windows(2).all(|w| w[0] < w[1]), "{ranks:?}");
    // Projet : `SaveVersion` en tête, `Calendars` avant `Tasks`, `Tasks`
    // avant les sections opaques `Resources` et `Assignments`.
    assert_eq!(rank(PROJECT_ORDER, "SaveVersion"), Some(0));
    let p = |n| rank(PROJECT_ORDER, n).unwrap();
    assert!(p("Name") < p("ScheduleFromStart"));
    assert!(p("CurrentDate") < p("Calendars"));
    assert!(p("Calendars") < p("Tasks") && p("Tasks") < p("Resources"));
    assert!(p("Resources") < p("Assignments"));
    // Lien, calendrier, jour, exception, semaine datée.
    assert_eq!(rank(LINK_ORDER, "PredecessorUID"), Some(0));
    assert_eq!(rank(LINK_ORDER, "LagFormat"), Some(LINK_ORDER.len() - 1));
    let c = |n| rank(CALENDAR_ORDER, n).unwrap();
    assert!(c("UID") < c("Name") && c("BaseCalendarUID") < c("WeekDays"));
    assert!(c("WeekDays") < c("Exceptions") && c("Exceptions") < c("WorkWeeks"));
    assert_eq!(
        WEEKDAY_ORDER,
        ["DayType", "DayWorking", "TimePeriod", "WorkingTimes"]
    );
    let x = |n| rank(EXCEPTION_ORDER, n).unwrap();
    assert!(x("EnteredByOccurrences") < x("TimePeriod") && x("Name") < x("Type"));
    assert!(x("Type") < x("Month") && x("DayWorking") < x("WorkingTimes"));
    assert_eq!(WORKWEEK_ORDER, ["TimePeriod", "Name", "WeekDays"]);
    assert_eq!(rank(TASK_ORDER, "FutureX"), None);
}

/// Charge opaque à la place de sa clé, clé inconnue en fin, ordre de
/// stockage conservé entre égaux. Entrées stockées : `Baseline` (rang 100),
/// `FutureX` (inconnue), `ExtendedAttribute` n° 1 (rang 93), `Cost`
/// (rang 54), `ExtendedAttribute` n° 2, `FutureY` (inconnue). Ordre
/// canonique attendu, à la main : `Cost` (54), `ExtendedAttribute` 1 et 2
/// (93, dans leur ordre), `Baseline` (100), puis `FutureX` et `FutureY`
/// (rang 102 = longueur de la table, dans leur ordre).
#[test]
fn r088_opaque_payload_at_key_rank_unknown_last() {
    assert_eq!(rank(TASK_ORDER, "Cost"), Some(54));
    assert_eq!(rank(TASK_ORDER, "ExtendedAttribute"), Some(93));
    assert_eq!(rank(TASK_ORDER, "Baseline"), Some(100));
    assert_eq!(TASK_ORDER.len(), 102);
    let stored = [
        entry("Baseline", "<Baseline>b</Baseline>"),
        entry("FutureX", "<FutureX/>"),
        entry("ExtendedAttribute", "<ExtendedAttribute>1</ExtendedAttribute>"),
        entry("Cost", "<Cost>0</Cost>"),
        entry("ExtendedAttribute", "<ExtendedAttribute>2</ExtendedAttribute>"),
        entry("FutureY", "<FutureY/>"),
    ];
    let canonical = canonical_extras(&stored, TASK_ORDER);
    assert_eq!(
        keys_and_payloads(&canonical),
        [
            ("Cost", "<Cost>0</Cost>"),
            ("ExtendedAttribute", "<ExtendedAttribute>1</ExtendedAttribute>"),
            ("ExtendedAttribute", "<ExtendedAttribute>2</ExtendedAttribute>"),
            ("Baseline", "<Baseline>b</Baseline>"),
            ("FutureX", "<FutureX/>"),
            ("FutureY", "<FutureY/>"),
        ]
    );
    // Idempotence : la forme canonique d'une forme canonique est elle-même.
    let again: Vec<ExtraEntry> = canonical.iter().map(|e| (*e).clone()).collect();
    assert_eq!(
        keys_and_payloads(&canonical_extras(&again, TASK_ORDER)),
        keys_and_payloads(&canonical)
    );
    // Vide → vide ; table vide → ordre de stockage inchangé.
    assert!(canonical_extras(&[], TASK_ORDER).is_empty());
    assert_eq!(
        keys_and_payloads(&canonical_extras(&stored, &[])),
        keys_and_payloads(&stored.iter().collect::<Vec<_>>())
    );
}

/// Projet : `Resources` et `Assignments` (charges opaques entières, R-128)
/// se rangent après `Tasks`, `Views` avant `Calendars` ; une section
/// inconnue va en fin.
#[test]
fn r088_project_sections_ordered() {
    let stored = [
        entry("Assignments", "<Assignments/>"),
        entry("Sprints2030", "<Sprints2030/>"),
        entry("Resources", "<Resources/>"),
        entry("Views", "<Views/>"),
        entry("LastSaved", "<LastSaved>2026-10-18T08:00:00</LastSaved>"),
    ];
    let keys: Vec<&str> = canonical_extras(&stored, PROJECT_ORDER)
        .iter()
        .map(|e| e.key.as_str())
        .collect();
    assert_eq!(
        keys,
        ["LastSaved", "Views", "Resources", "Assignments", "Sprints2030"]
    );
}
