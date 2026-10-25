//! Rendu du diagramme de Gantt en SVG.
//!
//! Sert à l'IA (via MCP), aux tests et à l'export. Ne dépend que de
//! `planrs-core` : aucune interface graphique ici. Déterministe : coordonnées
//! entières en 1/16 px, aucune police système, sortie bornée et rapport de
//! troncature (ADR-0012 D10). PNG reporté en J7.
//!
//! Hypothèses d'affichage H-R : `docs/features/render.md`.

mod axis;
mod metrics;
mod metrics_inter;
mod request;
mod scene;
mod svg;

pub use metrics::{FontRef, FontWeight, InterMetrics, TextMeasure};
pub use request::{GanttRequest, Lang, RangeSpec, RenderLimits, RenderReport, RowFilter, Theme, Zoom};
pub use svg::svg;
