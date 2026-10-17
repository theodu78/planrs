//! R-126 — Lignes de base MSPDI (lecture ; écriture : commit d'écriture des
//! lignes de base).

use planrs_core::{
    AssignmentUid, BaselineNumber, DurationUnit, Minutes, Money, ResourceUid, TaskUid, TimeBasis,
};
use planrs_tests::moment;

use crate::r088_resources_assignments_baselines_import as r088;

fn zero() -> BaselineNumber {
    BaselineNumber::new(0).unwrap()
}

#[test]
fn r126_task_baseline() {
    let doc = r088::fixture();
    let t = doc.project.tasks.get(TaskUid::new(1)).unwrap();
    let b = t.baselines[&zero()];
    assert_eq!(b.start, Some(moment("2026-10-19T08:00")));
    assert_eq!(b.finish, Some(moment("2026-10-23T17:00")));
    // `PT40H0M0S`, `DurationFormat` 39 (jour estimé) mais
    // `EstimatedDuration 0` l'emporte.
    let d = b.duration.unwrap();
    assert_eq!(
        (d.minutes(), d.unit(), d.basis(), d.is_estimated()),
        (Minutes::new(2_400), DurationUnit::Day, TimeBasis::Working, false)
    );
    assert_eq!(b.work.unwrap().minutes(), Minutes::new(2_400));
    // `Cost` en centièmes (O18).
    assert_eq!(b.cost, Some(Money::new(83_175).unwrap()));
    assert_eq!(b.fixed_cost, Some(Money::new(1_250).unwrap()));
    // `TimephasedData` de la ligne de base : `Extras` sous sa portée ;
    // `BCWS` (X) : ni lu ni conservé.
    let keys: Vec<&str> = t.extras.0.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, ["Baseline#0/TimephasedData"]);
}

#[test]
fn r126_resource_and_assignment_baselines() {
    let doc = r088::fixture();
    let r = doc.project.resources.get(ResourceUid::new(1)).unwrap();
    let b = r.baselines[&zero()];
    assert_eq!(b.work.unwrap().minutes(), Minutes::new(12_000));
    assert_eq!(b.cost, Some(Money::new(83_175).unwrap()));
    let a = doc.project.assignments.get(AssignmentUid::new(1)).unwrap();
    let b = a.baselines[&zero()];
    assert_eq!(
        (b.start, b.finish),
        (Some(moment("2026-10-19T08:00")), Some(moment("2026-10-20T17:00")))
    );
    assert_eq!(b.work.unwrap().minutes(), Minutes::new(960));
    assert_eq!(b.cost, Some(Money::new(6_654).unwrap()));
}
