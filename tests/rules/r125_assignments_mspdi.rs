//! R-125 — Affectations MSPDI : unités, travail, retard en dixièmes,
//! profil, aucune ressource (lecture ; écriture : commit d'écriture des
//! affectations).

use planrs_core::{AssignmentUid, Minutes, Units, WorkContour};

use crate::r088_resources_assignments_baselines_import as r088;

#[test]
fn r125_assignment_values() {
    let doc = r088::fixture();
    let a = |uid: u32| doc.project.assignments.get(AssignmentUid::new(uid)).unwrap();
    assert_eq!(a(1).units, Units::new(5_000).unwrap());
    assert_eq!(a(1).delay, Minutes::new(420));
    assert_eq!(a(1).contour, WorkContour::Flat);
    assert_eq!(a(2).resource, None);
    assert_eq!(a(2).contour, WorkContour::Contoured);
    assert_eq!(a(4).units, Units::new(25_000).unwrap());
}

#[test]
fn r125_assignments_are_written() {
    use planrs_core::{Assignment, Minutes, Resource, ResourceUid, Task, TaskUid, Units, Work};
    use planrs_tests::mspdi::{compact, new_project, reread, text, written};
    let mut proj = new_project("2026-10-19T08:00");
    proj.tasks
        .push(Task::new(TaskUid::new(1), "A".to_owned(), &proj.options))
        .unwrap();
    proj.resources
        .push(Resource::new(ResourceUid::new(1), "R".to_owned()))
        .unwrap();
    let mut a = Assignment::new(AssignmentUid::new(1), TaskUid::new(1), Some(ResourceUid::new(1)));
    a.units = Units::new(5_000).unwrap();
    a.delay = Minutes::new(420);
    a.work = Work::new(Minutes::new(960)).unwrap();
    a.contour = WorkContour::Bell;
    proj.assignments.push(a).unwrap();
    proj.assignments
        .push(Assignment::new(AssignmentUid::new(2), TaskUid::new(1), None))
        .unwrap();
    let out = written(&proj);
    let xml = compact(&text(&out));
    assert!(xml.contains("<Units>0.5</Units>"));
    assert!(xml.contains("<Delay>4200</Delay>"));
    assert!(xml.contains("<ResourceUID>-65535</ResourceUID>"));
    assert!(xml.contains("<WorkContour>6</WorkContour>"));
    assert!(xml.contains("<Work>PT16H0M0S</Work>"));
    let back = reread(&out.xml).project;
    assert_eq!(back.assignments, proj.assignments);
}
