//! `planrs mcp [--journal F] [--open plan.xml] [--headless] [--http ADDR]` :
//! serveur MCP sur l'entrée et la sortie standard (`planrs_mcp::serve_stdio`).
//!
//! Langue par défaut des textes : anglais (ADR-0012, arbitrage 6), sauf
//! `--lang` explicite. `--http` est le lot L17 (coupable) : refusé ici en
//! usage (code 2) tant qu'il n'est pas livré.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;
use planrs_automation::text::Language;
use planrs_mcp::{McpServeError, ServeOptions};

use super::{Ctx, warn};
use crate::exit;

/// Arguments de `planrs mcp`.
#[derive(Debug, Args)]
pub struct McpArgs {
    /// Journal de rejeu (JSONL), écrit avant chaque appel.
    #[arg(long, value_name = "FICHIER")]
    pub journal: Option<PathBuf>,
    /// Projet ouvert au démarrage.
    #[arg(long, value_name = "PLAN")]
    pub open: Option<PathBuf>,
    /// Ignore la garde « fichier ouvert dans l'application » (ADR-0002).
    #[arg(long)]
    pub headless: bool,
    /// Transport HTTP (lot L17, pas encore disponible).
    #[arg(long, value_name = "ADRESSE")]
    pub http: Option<String>,
}

/// Exécute la sous-commande.
pub fn run(ctx: &Ctx, args: McpArgs) -> ExitCode {
    if args.http.is_some() {
        warn(ctx.tr(
            "mcp --http : pas encore disponible (stdio seulement)",
            "mcp --http: not available yet (stdio only)",
        ));
        return ExitCode::from(exit::USAGE);
    }
    let lang = if ctx.lang_explicit { ctx.lang } else { Language::En };
    let mut opts = ServeOptions::new(ctx.now, lang);
    opts.journal = args.journal;
    opts.open = args.open;
    opts.ignore_app_guard = args.headless;
    match planrs_mcp::serve_stdio(opts) {
        Ok(()) => ExitCode::from(exit::SUCCESS),
        Err(e @ McpServeError::NotYetAvailable) => {
            warn(&format!("mcp : {e}"));
            ExitCode::from(exit::USAGE)
        }
        // Entrée/sortie, ligne trop longue (C-MCP-06 : code 3) et toute
        // erreur ajoutée par le serveur : erreur de fichier ou de flux.
        Err(e) => {
            warn(&format!("mcp : {e}"));
            ExitCode::from(exit::FILE)
        }
    }
}
