//! Session « headless » de planrs : projet ouvert, historique et registre de
//! commandes.
//!
//! Toute action (menu, raccourci, CLI, MCP, canal de contrôle) passe par le
//! même registre de commandes, identifiées par un id stable (`task.add`,
//! `link.add`…). Cette crate ne dépend d'aucune interface graphique
//! (règle 9, vérifiée par `cargo xtask layers`).
//!
//! J4 (ADR-0012, contrat L0) : [`session::Session`] (lot de commandes,
//! `dry_run`, annuler/rétablir), [`command`] (format versionné), [`registry`],
//! vues JSON, journal et [`workspace::Workspace`] communs à la CLI, au MCP
//! et au canal de contrôle.
//!
//! Modules livrés en J3 (ADR-0010) :
//! - [`text`] : texte des durées, des décalages, des prédécesseurs et des
//!   dates à la manière de MS Project, en français et en anglais (R-140 à
//!   R-143) ;
//! - [`csv`] : import et export des tâches en CSV, depuis et vers MS Project
//!   ou un tableur (R-144 à R-146).
//!
//! Aucune entrée/sortie hors de `files` et `control` (absents en wasm) : du
//! texte ou des octets en entrée, une valeur du modèle en sortie.

pub mod command;
#[cfg(not(target_arch = "wasm32"))]
pub mod control;
pub mod csv;
mod delta;
pub mod diff;
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod files;
pub mod history;
pub mod journal;
pub mod json;
pub mod outcome;
pub mod registry;
pub mod session;
pub mod text;
mod uid;
pub mod view;
pub mod workspace;

pub use command::{Batch, Command, Patch, Placement, TaskRef};
pub use diff::{ScheduleDiff, TaskChange};
pub use error::{CommandError, CommandErrorKind, HistoryError, OpenError};
pub use history::HistoryView;
pub use outcome::{ApplyOutcome, SideEffect};
pub use registry::{Availability, CommandSpec, Kind, REGISTRY};
pub use session::{AnchorSpec, ApplyMode, DocOrigin, NewProject, Session, SessionLimits};
pub use workspace::{ToolError, Workspace};
