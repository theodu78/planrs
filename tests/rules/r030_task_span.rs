//! R-030 — Placement d'une durée > 0. Passe avant : début = instant ouvré
//! suivant la date au plus tôt, fin = début + durée (R-021). Passe arrière :
//! fin = instant ouvré précédant la date au plus tard, début = fin − durée
//! (R-022). Calendrier Standard.

use planrs_core::calendar::CompiledCalendar;
use planrs_core::{Moment, TimeScale};
use planrs_tests::{compile_one, mins, moment, standard};

fn cal() -> CompiledCalendar {
    compile_one(standard())
}

fn forward(c: &CompiledCalendar, from: &str, d: i64) -> (Moment, Moment) {
    let s = TimeScale::Working(c).forward_span(moment(from), mins(d)).unwrap();
    (s.start, s.finish)
}

fn backward(c: &CompiledCalendar, to: &str, d: i64) -> (Moment, Moment) {
    let s = TimeScale::Working(c).backward_span(moment(to), mins(d)).unwrap();
    (s.start, s.finish)
}

#[test]
fn r030_forward_span_aligns_start() {
    let c = cal();
    // Lun 12:00 (pause), 240 : début 13:00, fin 17:00.
    assert_eq!(
        forward(&c, "2026-10-19T12:00", 240),
        (moment("2026-10-19T13:00"), moment("2026-10-19T17:00"))
    );
    // Ven 23/10 17:00, 480 : début lun 26/10 08:00, fin 17:00.
    assert_eq!(
        forward(&c, "2026-10-23T17:00", 480),
        (moment("2026-10-26T08:00"), moment("2026-10-26T17:00"))
    );
    // Sam 24/10 10:00, 480 : début lun 08:00, fin 17:00.
    assert_eq!(
        forward(&c, "2026-10-24T10:00", 480),
        (moment("2026-10-26T08:00"), moment("2026-10-26T17:00"))
    );
}

#[test]
fn r030_backward_span_aligns_finish() {
    let c = cal();
    // Au plus tard lun 26/10 08:00, 480 : fin ven 23/10 17:00, début ven 08:00.
    assert_eq!(
        backward(&c, "2026-10-26T08:00", 480),
        (moment("2026-10-23T08:00"), moment("2026-10-23T17:00"))
    );
    // Au plus tard lun 13:00, 240 : fin lun 12:00, début lun 08:00.
    assert_eq!(
        backward(&c, "2026-10-19T13:00", 240),
        (moment("2026-10-19T08:00"), moment("2026-10-19T12:00"))
    );
}
