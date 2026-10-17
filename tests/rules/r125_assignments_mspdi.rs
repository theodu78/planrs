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
