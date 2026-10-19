//! Affectations à l'écriture (R-125, R-199).

use planrs_core::{Assignment, Subject, Work};

use super::computed::Values;
use super::emit::Emitter;
use super::resource::units_text;
use super::{OrderProfile, WriteCode, baseline, entity, leveling, plan, timephased, unscoped};
use crate::codes;
use crate::scalar::format_iso_duration;
use crate::schema::Parent;

/// `ResourceUID` d'une affectation sans ressource (R-125, R-079).
const NO_RESOURCE: &str = "-65535";

/// Travail restant écrit d'une affectation (R-199, comme R-127 pour une
/// tâche) : sans travail réel, celui que MS Project écrit toujours, reste =
/// travail − réel = travail (le modèle n'est tenu à jour par aucune
/// commande : 0 pour une affectation créée, périmé après un rééquilibrage) ;
/// avec un travail réel saisi (avancement, J6), la valeur saisie telle
/// quelle.
pub(crate) fn remaining_work(a: &Assignment) -> Work {
    if a.actual_work == Work::ZERO {
        a.work
    } else {
        a.remaining_work
    }
}

/// Écrit une affectation ; `values` : `Start`/`Finish` à écrire (R-198,
/// R-199, R-122), absents sinon ; `TimephasedData` propres jetées si le
/// placement du moteur a changé (R-199, `mspdi.write.timephased_dropped`).
pub(crate) fn write(em: &mut Emitter, a: &Assignment, values: Option<&Values>, profile: OrderProfile) {
    let computed = values.and_then(|v| v.assignments.get(&a.uid()));
    em.subject = Some(Subject::Assignment(a.uid()));
    let numbers: Vec<u8> = a.baselines.keys().map(|n| n.get()).collect();
    let planned = plan(em, "Assignment", Parent::Assignment, &a.extras, &numbers, profile);
    let mut top = unscoped(&planned);
    leveling::drop_legacy(&mut top, a.leveling_delay);
    if computed.is_some_and(|v| v.drop_timephased) {
        let before = top.len();
        top.retain(|p| !timephased::is_scheduled_work_key(&p.key, p.payload));
        let count = u32::try_from(before.saturating_sub(top.len())).unwrap_or(u32::MAX);
        if count > 0 {
            em.report(
                WriteCode::TimephasedDropped { count },
                "Assignment/TimephasedData",
                None,
            );
        }
    }
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
            "RemainingWork" => em.leaf(name, &format_iso_duration(remaining_work(a).minutes().get())),
            "Units" => em.leaf(name, &units_text(a.units)),
            "Work" => em.leaf(name, &format_iso_duration(a.work.minutes().get())),
            "WorkContour" => em.leaf(name, &codes::work_contour_code(a.contour).to_string()),
            "Baseline" => baseline::assignment(em, &a.baselines, &planned, profile),
            "Start" => em.opt_date(name, computed.and_then(|v| v.start)),
            "Finish" => em.opt_date(name, computed.and_then(|v| v.finish)),
            "LevelingDelay" | "LevelingDelayFormat" => leveling::write(em, name, a.leveling_delay),
            _ => {}
        },
    );
    em.subject = None;
}
