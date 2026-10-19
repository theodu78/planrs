//! `planrs explain <plan> --task N` : explication d'une date (J6). Cachée
//! de l'aide ; répond « disponible en J6 » avec le code d'usage (2), sans
//! lire le fichier (ADR-0012, écarts au brief § 7).

use std::process::ExitCode;

use clap::Args;

use super::{Ctx, warn};
use crate::exit;

/// Arguments de `planrs explain`.
#[derive(Debug, Args)]
pub struct ExplainArgs {
    /// Fichier MSPDI (`.xml`).
    pub plan: Option<String>,
    /// ID de la tâche (converti en UID par `Project::task_by_id` en J6).
    #[arg(long, value_name = "N")]
    pub task: Option<u32>,
}

/// Exécute la sous-commande.
pub fn run(ctx: &Ctx, args: &ExplainArgs) -> ExitCode {
    let _ = (&args.plan, args.task);
    warn(ctx.tr(
        "explain : disponible en J6 (explication des dates)",
        "explain: available in milestone J6 (date explanation)",
    ));
    ExitCode::from(exit::USAGE)
}
