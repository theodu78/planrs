//! Écriture SVG (lot L7).

use planrs_core::{Project, Schedule};

use crate::request::{GanttRequest, RenderReport};

/// SVG du Gantt et rapport. Infaillible : toute borne atteinte est signalée
/// dans le rapport. Squelette L0 : document SVG vide.
#[must_use]
pub fn svg(project: &Project, schedule: &Schedule, req: &GanttRequest) -> (String, RenderReport) {
    let _ = (project, schedule, req);
    let out = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#);
    let report = RenderReport {
        bytes: out.len(),
        ..RenderReport::default()
    };
    (out, report)
}
