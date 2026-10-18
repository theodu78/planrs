//! Binaire `planrs` en ligne de commande.
//!
//! Il expose les mêmes opérations que le serveur MCP (`summary`, `tasks`,
//! `apply`, `render`, `import-csv`, `mcp`, `explain`), toutes via le
//! `Workspace` de `planrs-automation` : mêmes JSON que le MCP (ADR-0012 D1).
//! Codes de sortie (C-MCP-08) dans [`exit`].
//!
//! État : contrat J4 (L0) ; les sous-commandes sont écrites par le lot L9.

mod cmd;
mod exit;

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use planrs_core::PRODUCT_NAME;

/// Planification de projet compatible MS Project, pilotable par IA.
#[derive(Debug, Parser)]
#[command(name = PRODUCT_NAME, version, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Sous-commandes.
#[derive(Debug, Subcommand)]
enum Command {
    /// Résumé du projet.
    Summary,
    /// Liste des tâches.
    Tasks,
    /// Applique un lot de commandes.
    Apply,
    /// Gantt en SVG.
    Render,
    /// Import d'un CSV de tâches.
    ImportCsv,
    /// Serveur MCP (stdio).
    Mcp,
    /// Explication d'une date (J6).
    #[command(hide = true)]
    Explain,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Summary => cmd::summary::run(),
        Command::Tasks => cmd::tasks::run(),
        Command::Apply => cmd::apply::run(),
        Command::Render => cmd::render::run(),
        Command::ImportCsv => cmd::import_csv::run(),
        Command::Mcp => cmd::mcp::run(),
        Command::Explain => cmd::explain::run(),
    }
}
