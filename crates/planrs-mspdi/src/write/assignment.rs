//! Affectations à l'écriture (R-125).

use planrs_core::{Assignment, Subject};

use super::emit::Emitter;
use super::resource::units_text;
use super::{OrderProfile, baseline, entity, plan, unscoped};
use crate::codes;
use crate::scalar::format_iso_duration;
use crate::schema::Parent;

/// `ResourceUID` d'une affectation sans ressource (R-125, R-079).
const NO_RESOURCE: &str = "-65535";

/// Écrit une affectation. `Start`/`Finish` (calculés) ne sont pas écrits.
pub(crate) fn write(em: &mut Emitter, a: &Assignment, profile: OrderProfile) {
    em.subject = Some(Subject::Assignment(a.uid()));
    let numbers: Vec<u8> = a.baselines.keys().map(|n| n.get()).collect();
    let planned = plan(em, "Assignment", Parent::Assignment, &a.extras, &numbers, profile);
    let top = unscoped(&planned);
    entity(
        em,
        "Assignment",
        Parent::Assignment,
        profile,
        &top,
        |em, name| match name {
            "UID" => em.leaf(name, &a.uid().get().to_string()),
            "GUID" => em.opt_leaf(name, a.guid.map(|g| g.to_string())),
            "TaskUID" => em.leaf(name, &a.task.get().to_string()),
            "ResourceUID" => em.leaf(
                name,
                &a.resource
                    .map_or_else(|| NO_RESOURCE.to_owned(), |r| r.get().to_string()),
            ),
            "ActualWork" => em.leaf(name, &format_iso_duration(a.actual_work.minutes().get())),
            // R-125 : retard en dixièmes de minute (O20).
            "Delay" => em.leaf(name, &a.delay.get().saturating_mul(10).to_string()),
            "RemainingWork" => em.leaf(name, &format_iso_duration(a.remaining_work.minutes().get())),
            "Units" => em.leaf(name, &units_text(a.units)),
            "Work" => em.leaf(name, &format_iso_duration(a.work.minutes().get())),
            "WorkContour" => em.leaf(name, &codes::work_contour_code(a.contour).to_string()),
            "Baseline" => baseline::assignment(em, &a.baselines, &planned, profile),
            _ => {}
        },
    );
    em.subject = None;
}
