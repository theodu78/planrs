//! Rendu du diagramme de Gantt en SVG.
//!
//! Sert à l'IA (via MCP), aux tests et à l'export. Ne dépend que de
//! `planrs-core` (et de serde pour le style) : aucune interface graphique ici. Déterministe : coordonnées
//! entières en 1/16 px, aucune police système, sortie bornée et rapport de
//! troncature (ADR-0012 D10). PNG reporté en J7.
//!
//! Hypothèses d'affichage H-R : `docs/features/render.md`. J8 (ADR-0016) :
//! style et modèles (`style`), géométrie partagée (`geometry`), pagination
//! (`paginate`), largeurs d'Inter (H-R11).

mod axis;
pub mod geometry;
mod metrics;
mod metrics_inter;
pub mod paginate;
mod request;
mod scene;
pub mod style;
mod svg;

pub use metrics::{FontRef, FontWeight, InterMetrics, TextMeasure};
pub use request::{GanttRequest, Lang, RangeSpec, RenderLimits, RenderReport, RowFilter, Theme, Zoom};
pub use style::{PlanStyle, Rgb, TemplateId, contrast_ratio, parse_doc_style, resolve, template};
pub use svg::svg;
