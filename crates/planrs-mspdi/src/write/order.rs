//! Tables d'ordre des éléments écrits (R-088, J3, lot W1).
//!
//! MSPDI est validé par un schéma XML dont les `xs:sequence` imposent l'ordre
//! des enfants de chaque élément. L'écrivain émet donc les enfants d'une
//! entité dans l'ordre de sa table : valeur du modèle si la balise est
//! régénérée, puis les charges opaques (`Extras` d'origine `mspdi`) de la
//! même clé, à la place de leur clé ; les clés absentes de la table
//! (inconnues du schéma tel que nous le connaissons) vont en fin d'entité,
//! dans leur ordre de stockage.
//!
//! Source unique (ADR-0010, amendements) : ces tables, et rien d'autre (ni
//! fichier d'ordre commité, ni outil de génération). Provenance : ordre
//! d'après `mspdi_pj15.xsd` (Project 2013 SDK, révision 2012-07-18, de
//! mémoire : le schéma n'est pas consulté dans la session) et ordre observé
//! dans les cinq fichiers du corpus (A1, A2, A4, A6, A7 :
//! `tests/corpus/files/*.xml`, fusion des suites d'enfants sans aucun
//! conflit). Les balises suivies du commentaire `hypothèse` ne figurent dans
//! aucun fichier du corpus : leur position vient du schéma de mémoire, à
//! vérifier le jour où le XSD sera disponible. Le test de sous-suite
//! (`r088_*`) vérifie que la suite des enfants de chaque entité de chaque
//! fichier du corpus respecte sa table.

use planrs_core::ExtraEntry;

/// Enfants de `Project`. Ordre d'après `mspdi_pj15.xsd` (de mémoire) et
/// observé dans A1/A2/A4/A6/A7.
pub(crate) const PROJECT_ORDER: &[&str] = &[
    "SaveVersion",
    "BuildNumber",
    "Name",
    "GUID",
    "Title",
    "Subject",  // hypothèse : non observé
    "Category", // hypothèse : non observé
    "Company",
    "Author",
    "Manager",
    "CreationDate",
    "LastSaved",
    "ScheduleFromStart",
    "StartDate",
    "FinishDate",
    "FYStartDate",
    "CriticalSlackLimit",
    "CurrencyDigits",
    "CurrencySymbol",
    "CurrencyCode",
    "CurrencySymbolPosition",
    "CalendarUID",
    "BaselineCalendar",
    "DefaultStartTime",
    "DefaultFinishTime",
    "MinutesPerDay",
    "MinutesPerWeek",
    "DaysPerMonth",
    "DefaultTaskType",
    "DefaultFixedCostAccrual",
    "DefaultStandardRate",
    "DefaultOvertimeRate",
    "DurationFormat",
    "WorkFormat",
    "EditableActualCosts",
    "HonorConstraints",
    "InsertedProjectsLikeSummary",
    "MultipleCriticalPaths",
    "NewTasksEffortDriven",
    "NewTasksEstimated",
    "SplitsInProgressTasks",
    "SpreadActualCost",
    "SpreadPercentComplete",
    "TaskUpdatesResource",
    "FiscalYearStart",
    "WeekStartDay",
    "MoveCompletedEndsBack",
    "MoveRemainingStartsBack",
    "MoveRemainingStartsForward",
    "MoveCompletedEndsForward",
    "BaselineForEarnedValue",
    "AutoAddNewResourcesAndTasks",
    "StatusDate", // hypothèse : non observé
    "CurrentDate",
    "MicrosoftProjectServerURL",
    "Autolink",
    "NewTaskStartDate",
    "NewTasksAreManual",
    "DefaultTaskEVMethod",
    "ProjectExternallyEdited",
    "ExtendedCreationDate",
    "ActualsInSync",
    "RemoveFileProperties",
    "AdminProject",
    "UpdateManuallyScheduledTasksWhenEditingLinks",
    "KeepTaskOnNearestWorkingTimeWhenMadeAutoScheduled",
    "SprintLength",
    "AgileMode",
    "Views",
    "Filters",
    "Groups",
    "Tables",
    "Maps",
    "Reports",
    "Drawings",
    "DataLinks",
    "VBAProjects",
    "OutlineCodes",
    "WBSMasks",
    "ExtendedAttributes",
    "Calendars",
    "Tasks",
    "Resources",
    "Assignments",
    "BoardColumns",
    "Sprints",
];

/// Enfants de `Tasks/Task`. Ordre d'après `mspdi_pj15.xsd` (de mémoire) et
/// observé dans A1/A2/A4/A6/A7 (toutes les balises sont observées).
pub(crate) const TASK_ORDER: &[&str] = &[
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
    "WBSLevel",
    "OutlineNumber",
    "OutlineLevel",
    "Priority",
    "StartText",
    "DurationText",
    "Start",
    "Finish",
    "Duration",
    "ManualStart",
    "ManualFinish",
    "ManualDuration",
    "DurationFormat",
    "FreeformDurationFormat",
    "Work",
    "Stop",
    "Resume",
    "ResumeValid",
    "EffortDriven",
    "Recurring",
    "OverAllocated",
    "Estimated",
    "Milestone",
    "Summary",
    "DisplayAsSummary",
    "Critical",
    "IsSubproject",
    "IsSubprojectReadOnly",
    "ExternalTask",
    "EarlyStart",
    "EarlyFinish",
    "LateStart",
    "LateFinish",
    "StartVariance",
    "FinishVariance",
    "WorkVariance",
    "FreeSlack",
    "TotalSlack",
    "StartSlack",
    "FinishSlack",
    "FixedCost",
    "FixedCostAccrual",
    "PercentComplete",
    "PercentWorkComplete",
    "Cost",
    "OvertimeCost",
    "OvertimeWork",
    "ActualStart",
    "ActualFinish",
    "ActualDuration",
    "ActualCost",
    "ActualOvertimeCost",
    "ActualWork",
    "ActualOvertimeWork",
    "RegularWork",
    "RemainingDuration",
    "RemainingCost",
    "RemainingWork",
    "RemainingOvertimeCost",
    "RemainingOvertimeWork",
    "ACWP",
    "CV",
    "ConstraintType",
    "CalendarUID",
    "ConstraintDate",
    "Deadline",
    "LevelAssignments",
    "LevelingCanSplit",
    "LevelingDelay",
    "LevelingDelayFormat",
    "PreLeveledStart",
    "PreLeveledFinish",
    "IgnoreResourceCalendar",
    "Notes",
    "HideBar",
    "Rollup",
    "BCWS",
    "BCWP",
    "PhysicalPercentComplete",
    "EarnedValueMethod",
    "PredecessorLink",
    "IsPublished",
    "CommitmentType",
    "ExtendedAttribute",
    "BoardStatusColumnOrderingID",
    "SprintColumnOrderingID",
    "NextAvailableBoardStatusColumnOrderingID",
    "NextAvailableSprintOrderingID",
    "SprintUID",
    "BoardColumnUID",
    "Baseline",
    "TimephasedData",
];

/// Enfants de `Task/PredecessorLink`. Ordre d'après `mspdi_pj15.xsd` (de
/// mémoire) et observé dans A1/A2/A4/A6/A7 (127 liens).
pub(crate) const LINK_ORDER: &[&str] = &[
    "PredecessorUID",
    "Type",
    "CrossProject",
    "CrossProjectName", // hypothèse : non observé
    "LinkLag",
    "LagFormat",
];

/// Enfants de `Calendars/Calendar`. Ordre d'après `mspdi_pj15.xsd` (de
/// mémoire) et observé dans A1/A2/A4/A6/A7.
pub(crate) const CALENDAR_ORDER: &[&str] = &[
    "UID",
    "GUID",
    "Name",
    "IsBaseCalendar",
    "IsBaselineCalendar",
    "BaseCalendarUID",
    "WeekDays",
    "Exceptions",
    "WorkWeeks", // hypothèse : non observé
];

/// Enfants de `WeekDays/WeekDay` (d'un calendrier ou d'une semaine datée).
/// Ordre d'après `mspdi_pj15.xsd` (de mémoire) et observé dans
/// A1/A2/A4/A6/A7.
pub(crate) const WEEKDAY_ORDER: &[&str] = &["DayType", "DayWorking", "TimePeriod", "WorkingTimes"];

/// Enfants de `WorkingTimes/WorkingTime`. Ordre d'après `mspdi_pj15.xsd` (de
/// mémoire) et observé dans A1/A2/A4/A6/A7.
pub(crate) const WORKINGTIME_ORDER: &[&str] = &["FromTime", "ToTime"];

/// Enfants de `TimePeriod` (d'un `WeekDay`, d'une `Exception` ou d'une
/// `WorkWeek`). Ordre d'après `mspdi_pj15.xsd` (de mémoire) et observé dans
/// A1/A2/A4/A6/A7.
pub(crate) const TIMEPERIOD_ORDER: &[&str] = &["FromDate", "ToDate"];

/// Enfants de `Exceptions/Exception`. Ordre d'après `mspdi_pj15.xsd` (de
/// mémoire) et observé dans A1/A2/A4/A6/A7.
pub(crate) const EXCEPTION_ORDER: &[&str] = &[
    "EnteredByOccurrences",
    "TimePeriod",
    "Occurrences",
    "Name",
    "Type",
    "Period",        // hypothèse : non observé
    "DaysOfWeek",    // hypothèse : non observé
    "MonthItem",     // hypothèse : non observé
    "MonthPosition", // hypothèse : non observé
    "Month",
    "MonthDay",
    "DayWorking",
    "WorkingTimes",
];

/// Enfants de `WorkWeeks/WorkWeek`. Ordre d'après `mspdi_pj15.xsd` (de
/// mémoire) ; aucune semaine datée dans A1/A2/A4/A6/A7 : les trois
/// positions sont des hypothèses.
pub(crate) const WORKWEEK_ORDER: &[&str] = &[
    "TimePeriod", // hypothèse : non observé
    "Name",       // hypothèse : non observé
    "WeekDays",   // hypothèse : non observé
];

/// Rang de la balise `name` dans `table` (position, à partir de 0) ; `None`
/// si la balise n'y figure pas (inconnue du schéma tel que nous le
/// connaissons).
pub(crate) fn rank(table: &[&str], name: &str) -> Option<usize> {
    table.iter().position(|n| *n == name)
}

/// Forme canonique des charges opaques d'une entité (R-088) : les entrées
/// triées de façon stable par rang de leur clé dans `table`, les clés
/// inconnues au rang `table.len()` (donc en fin, dans leur ordre de
/// stockage). Deux entrées de même clé gardent leur ordre relatif.
///
/// Le filtrage par origine (seules les entrées `mspdi` sont écrites) est à
/// la charge de l'appelant. Coût : O(n × |table|) pour les rangs, puis
/// O(n log n) pour le tri.
pub(crate) fn canonical_extras<'a>(entries: &'a [ExtraEntry], table: &[&str]) -> Vec<&'a ExtraEntry> {
    let mut ranked: Vec<(usize, &'a ExtraEntry)> = entries
        .iter()
        .map(|e| (rank(table, &e.key).unwrap_or(table.len()), e))
        .collect();
    // `sort_by_key` est stable : l'ordre de stockage départage les égaux.
    ranked.sort_by_key(|(r, _)| *r);
    ranked.into_iter().map(|(_, e)| e).collect()
}
