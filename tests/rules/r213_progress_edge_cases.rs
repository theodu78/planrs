//! R-213 — Cas limites de la saisie d'avancement : récapitulative et tâche
//! inactive refusées ; jalon (D = 0) à un % strictement entre 0 et 100 ⇒
//! 100 %, fin réelle = début réel, `MilestoneRounded`.

use crate::r201_percent_input::{MON, at, calendar, ctx, pct, task};
use planrs_core::progress::edit::{EditError, EditNote, Input, apply};
use planrs_core::{Minutes, Percent};

#[test]
fn r213_summary_refused() {
    let cal = calendar();
    let mut c = ctx(&cal, MON, "2026-10-23T17:00");
    c.summary = true;
    let e = apply(&task(2_400), &[], &[Input::Percent(pct(5_000))], &c);
    assert_eq!(e, Err(EditError::SummaryNotEditable));
    assert_eq!(
        EditError::SummaryNotEditable.code(),
        "progress.summary_not_editable"
    );
    c.summary = false;
    let mut t = task(2_400);
    t.active = false;
    assert_eq!(
        apply(&t, &[], &[Input::Percent(pct(5_000))], &c),
        Err(EditError::Inactive)
    );
    assert_eq!(EditError::Inactive.code(), "progress.inactive");
}

#[test]
fn r213_milestone_rounded() {
    // Jalon lun 08:00 à 30 % : 100 %, début et fin réels lun 08:00, D 0.
    let cal = calendar();
    let c = ctx(&cal, MON, MON);
    let e = apply(&task(0), &[], &[Input::Percent(pct(3_000))], &c).unwrap();
    assert_eq!(e.notes, [EditNote::MilestoneRounded]);
    assert_eq!(e.progress.actual_start, Some(at(MON)));
    assert_eq!(e.progress.actual_finish, Some(at(MON)));
    assert_eq!(e.progress.percent_complete, Percent::FULL);
    assert_eq!(e.duration, Minutes::ZERO);
    // 100 % : même résultat, sans remarque.
    let e = apply(&task(0), &[], &[Input::Percent(Percent::FULL)], &c).unwrap();
    assert_eq!(e.notes, []);
    assert_eq!(e.progress.actual_finish, Some(at(MON)));
}
