//! R-193 — Travail fixe toujours piloté par l'effort, récapitulative jamais
//! (S41 : « always effort-driven » ; S42 : « Summary tasks … can't be set
//! to effort-driven », « You can't remove effort-driven scheduling from
//! fixed-work tasks »). Unités nulles refusées en travail fixe.

use crate::r170_work_equals_duration_times_units::{DAY, part, task, uid, units, work};
use planrs_core::Minutes;
use planrs_core::TaskType::{FixedUnits, FixedWork};
use planrs_core::resource::{Edit, EditError, Recalculated, apply, edit::effective_effort_driven};

#[test]
fn r193_fw_effort_driven_refused() {
    let parts = [part(1, 10_000, 960)];
    let side = task(FixedWork, true, 2 * DAY, &parts);
    assert_eq!(
        apply(&side, &parts, &Edit::EffortDriven(false)),
        Err(EditError::FixedWorkEffortDriven)
    );
    assert_eq!(
        EditError::FixedWorkEffortDriven.code(),
        "edit.fixed_work_effort_driven"
    );
    // `true` est accepté et ne recalcule rien.
    let out = apply(&side, &parts, &Edit::EffortDriven(true)).unwrap();
    assert_eq!(out.recalculated, Recalculated::Nothing);
}

#[test]
fn r193_type_fixed_work_makes_effort_driven() {
    // Passage en travail fixe d'une tâche non pilotée : accepté, rien de
    // recalculé ; le pilotage effectif devient vrai (la commande l'écrit).
    let parts = [part(1, 10_000, 960)];
    let side = task(FixedUnits, false, 2 * DAY, &parts);
    let out = apply(&side, &parts, &Edit::Type(FixedWork)).unwrap();
    assert_eq!(out.recalculated, Recalculated::Nothing);
    assert!(effective_effort_driven(FixedWork, false));
    assert!(!effective_effort_driven(FixedUnits, false));
}

#[test]
fn r193_summary_refused() {
    let mut side = task(FixedUnits, false, 2 * DAY, &[]);
    side.summary = true;
    for edit in [
        Edit::Duration(Minutes::new(DAY)),
        Edit::Work(work(60)),
        Edit::Add {
            uid: uid(1),
            units: units(10_000),
        },
        Edit::Type(FixedWork),
        Edit::EffortDriven(true),
    ] {
        assert_eq!(apply(&side, &[], &edit), Err(EditError::SummaryNotEditable));
    }
}

#[test]
fn r193_zero_units_fw_refused() {
    let parts = [part(1, 10_000, 960)];
    let side = task(FixedWork, true, 2 * DAY, &parts);
    let zero = Edit::Units {
        uid: uid(1),
        units: units(0),
    };
    assert_eq!(apply(&side, &parts, &zero), Err(EditError::ZeroUnitsFixedWork));
    let add = Edit::Add {
        uid: uid(2),
        units: units(0),
    };
    assert_eq!(apply(&side, &parts, &add), Err(EditError::ZeroUnitsFixedWork));
    // Passage en travail fixe avec une affectation à unités nulles : refusé.
    let parts = [part(1, 0, 0)];
    let side = task(FixedUnits, false, 2 * DAY, &parts);
    assert_eq!(
        apply(&side, &parts, &Edit::Type(FixedWork)),
        Err(EditError::ZeroUnitsFixedWork)
    );
}
