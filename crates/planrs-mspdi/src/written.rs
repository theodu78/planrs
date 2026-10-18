//! Ensembles W : balises que l'écrivain régénère, par entité (J3,
//! ADR-0010 amendé, R-128).
//!
//! Contrat partagé entre la capture opaque du lecteur (`xml.rs`) et
//! l'écrivain (`write/`, lot W3) : **est capturé tout enfant direct de
//! `Project`, `Tasks/Task` ou `Calendars/Calendar` dont le nom local n'est
//! pas dans l'ensemble W de son entité** ; l'écrivain émet exactement les
//! balises de W depuis le modèle ou le `Schedule`, puis réémet les charges
//! capturées. Une balise dans W et capturée serait émise deux fois ; une
//! balise ni dans W ni capturée serait perdue. Toute modification d'un de
//! ces ensembles se fait donc ici seulement, des deux côtés à la fois.
//!
//! W n'est pas la liste blanche du lecteur : `LevelingDelay` et
//! `LevelingDelayFormat` sont lus (valeur enregistrée, `mspdi.unsupported`)
//! sans être régénérés, donc capturés ; `OutlineNumber` est régénéré (valeur
//! dérivée de la hiérarchie) sans être lu.
//!
//! Les ensembles sont publics (lecture seule) pour les propriétés
//! d'aller-retour (P19, P20) des autres paquets de test.
//!
//! Clés : une entrée capturée a pour clé le nom **qualifié** de l'élément,
//! tel qu'écrit (`x:Foo`) ; l'appartenance à W se teste sur le nom local
//! (un élément préfixé dont le nom local est dans W est lu, jamais
//! capturé). Seule clé de W admise en `Extras` : `PredecessorLink` sur une
//! tâche (lien abandonné, [`TASK_CAPTURED_WRITTEN`]).

/// Balises de `Project` régénérées : les champs scalaires lus
/// (`PROJECT_FIELDS` du lecteur) et les conteneurs `Calendars`, `Tasks`.
/// `Resources` et `Assignments` n'y sont pas : ils sont capturés entiers.
pub const PROJECT_WRITTEN: &[&str] = &[
    "SaveVersion",
    "GUID",
    "Name",
    "Title",
    "Subject",
    "Category",
    "Company",
    "Manager",
    "Author",
    "CreationDate",
    "ScheduleFromStart",
    "StartDate",
    "FinishDate",
    "CalendarUID",
    "DefaultStartTime",
    "DefaultFinishTime",
    "MinutesPerDay",
    "MinutesPerWeek",
    "DaysPerMonth",
    "DurationFormat",
    "WorkFormat",
    "DefaultTaskType",
    "NewTasksEffortDriven",
    "NewTasksEstimated",
    "HonorConstraints",
    "WeekStartDay",
    "StatusDate",
    "CurrentDate",
    "CriticalSlackLimit",
    "MultipleCriticalPaths",
    "Calendars",
    "Tasks",
];

/// Balises de `Tasks/Task` régénérées (conception B § 4.3) : champs du
/// modèle, valeurs calculées tirées du `Schedule`, `OutlineNumber` dérivé de
/// la hiérarchie, liens du modèle. Un `PredecessorLink` abandonné par le
/// lecteur pour une cause non fautive (`CrossProject 1`, `Type` inconnu)
/// est néanmoins capturé entier sur la tâche successeur (exception unique,
/// R-128) ; l'écrivain le réémet après les liens du modèle.
pub const TASK_WRITTEN: &[&str] = &[
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

/// Balises de W qu'une tâche peut néanmoins porter en `Extras` : le
/// `PredecessorLink` abandonné (projet externe, type inconnu), capturé entier.
pub const TASK_CAPTURED_WRITTEN: &[&str] = &["PredecessorLink"];

/// Balises de `Calendars/Calendar` régénérées : champs scalaires lus et les
/// trois listes (`WeekDays`, `Exceptions`, `WorkWeeks`). Les enfants
/// inconnus de `WeekDay`, `Exception`, `WorkWeek` ou `TimePeriod` ne sont
/// pas capturés (aucun observé dans le corpus).
pub const CALENDAR_WRITTEN: &[&str] = &[
    "UID",
    "GUID",
    "Name",
    "IsBaseCalendar",
    "BaseCalendarUID",
    "WeekDays",
    "Exceptions",
    "WorkWeeks",
];

/// Balises d'un `PredecessorLink` gardé, régénérées. `Link` n'a pas
/// d'`Extras` (ADR-0010 amendé) : un enfant hors de cette liste d'un lien
/// gardé est perdu, compté et signalé (`mspdi.unsupported`,
/// `predecessor_link_child`).
pub const LINK_WRITTEN: &[&str] = &["PredecessorUID", "Type", "CrossProject", "LinkLag", "LagFormat"];

/// Balises régénérées sans être lues : valeurs dérivées que l'écrivain
/// recalcule (hiérarchie). Toute autre balise de W est lue par l'import,
/// sinon sa valeur d'origine serait perdue en silence.
pub const DERIVED_WRITTEN: &[&str] = &["OutlineNumber"];

/// Vrai si l'enfant `name` (nom local) de `Project` n'est pas régénéré.
#[must_use]
pub fn project_captures(name: &str) -> bool {
    !PROJECT_WRITTEN.contains(&name)
}

/// Vrai si l'enfant `name` (nom local) d'une tâche n'est pas régénéré.
#[must_use]
pub fn task_captures(name: &str) -> bool {
    !TASK_WRITTEN.contains(&name)
}

/// Vrai si l'enfant `name` (nom local) d'un calendrier n'est pas régénéré.
#[must_use]
pub fn calendar_captures(name: &str) -> bool {
    !CALENDAR_WRITTEN.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::{
        CALENDAR_WRITTEN, DERIVED_WRITTEN, LINK_WRITTEN, PROJECT_WRITTEN, TASK_CAPTURED_WRITTEN, TASK_WRITTEN,
    };
    use crate::xml::{CALENDAR_FIELDS, LINK_FIELDS, PROJECT_FIELDS, TASK_FIELDS};

    /// Conteneurs régénérés (parcourus par le lecteur, pas des champs).
    const CONTAINERS: &[&str] = &[
        "Calendars",
        "Tasks",
        "PredecessorLink",
        "WeekDays",
        "Exceptions",
        "WorkWeeks",
    ];

    fn no_duplicates(set: &[&str]) {
        let mut sorted = set.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), set.len(), "doublon dans {set:?}");
    }

    /// Toute balise de W est lue par l'import (liste blanche), ou est un
    /// conteneur parcouru, ou une valeur dérivée régénérée : aucune valeur
    /// d'origine n'est perdue en silence par la régénération.
    #[test]
    fn r128_every_written_tag_is_read_or_derived() {
        for (set, read) in [
            (PROJECT_WRITTEN, PROJECT_FIELDS),
            (TASK_WRITTEN, TASK_FIELDS),
            (CALENDAR_WRITTEN, CALENDAR_FIELDS),
            (LINK_WRITTEN, LINK_FIELDS),
        ] {
            no_duplicates(set);
            for tag in set {
                assert!(
                    read.contains(tag) || CONTAINERS.contains(tag) || DERIVED_WRITTEN.contains(tag),
                    "{tag} régénéré sans être lu"
                );
            }
        }
    }

    /// Champs lus mais non régénérés : capturés (lus et capturés à la fois).
    /// Seuls `LevelingDelay` (tâche) le sont aujourd'hui ; un ajout à la
    /// liste blanche du lecteur doit être décidé ici aussi.
    #[test]
    fn r128_read_but_not_written_fields_are_captured() {
        let missing = |read: &[&'static str], set: &[&str]| -> Vec<&'static str> {
            read.iter().copied().filter(|f| !set.contains(f)).collect()
        };
        assert!(missing(PROJECT_FIELDS, PROJECT_WRITTEN).is_empty());
        assert_eq!(missing(TASK_FIELDS, TASK_WRITTEN), ["LevelingDelay"]);
        assert!(missing(CALENDAR_FIELDS, CALENDAR_WRITTEN).is_empty());
        assert!(missing(LINK_FIELDS, LINK_WRITTEN).is_empty());
        assert!(super::task_captures("LevelingDelay"));
        assert!(super::task_captures("LevelingDelayFormat"));
        assert!(super::project_captures("Resources"));
        assert!(super::project_captures("Assignments"));
        assert!(!super::calendar_captures("Exceptions"));
        assert!(TASK_CAPTURED_WRITTEN.iter().all(|k| TASK_WRITTEN.contains(k)));
    }
}
