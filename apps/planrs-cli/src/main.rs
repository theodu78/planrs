//! Binaire `planrs` en ligne de commande.
//!
//! Il expose les mêmes opérations que le serveur MCP (`summary`, `tasks`,
//! `apply`, `render`, `import-csv`, `mcp`, `explain`), toutes via le
//! `Workspace` de `planrs-automation` : mêmes JSON que le MCP (ADR-0012 D1).
//! Codes de sortie (C-MCP-08) dans [`exit`] ; « maintenant » et langue dans
//! [`now`] (seule lecture de l'horloge). Détail : `docs/features/cli.md`.

mod cmd;
mod exit;
mod now;

use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use planrs_automation::text::Language;
use planrs_core::PRODUCT_NAME;

/// Planification de projet compatible MS Project, pilotable par IA.
#[derive(Debug, Parser)]
#[command(name = PRODUCT_NAME, version, arg_required_else_help = true)]
struct Cli {
    /// « Maintenant » (`AAAA-MM-JJTHH:MM`) ; défaut : `PLANRS_NOW`, sinon
    /// l'horloge locale.
    #[arg(long, global = true, value_name = "AAAA-MM-JJTHH:MM")]
    now: Option<String>,
    /// Langue des messages et des textes (défaut : `LANG`, sinon `fr`).
    #[arg(long, global = true, value_enum)]
    lang: Option<LangArg>,
    #[command(subcommand)]
    command: Command,
}

/// Langue (`--lang`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum LangArg {
    /// Français.
    Fr,
    /// Anglais.
    En,
}

/// Sous-commandes.
#[derive(Debug, Subcommand)]
enum Command {
    /// Résumé du projet.
    Summary(cmd::summary::SummaryArgs),
    /// Liste des tâches.
    Tasks(cmd::tasks::TasksArgs),
    /// Applique un lot de commandes.
    Apply(cmd::apply::ApplyArgs),
    /// Gantt en SVG.
    Render(cmd::render::RenderArgs),
    /// Import d'un CSV de tâches.
    ImportCsv(cmd::import_csv::ImportCsvArgs),
    /// Serveur MCP (stdio).
    Mcp(cmd::mcp::McpArgs),
    /// Explication d'une date (J6).
    #[command(hide = true)]
    Explain(cmd::explain::ExplainArgs),
}

fn main() -> ExitCode {
    // Erreur d'usage : clap affiche l'aide et sort avec le code 2 ;
    // `--help` et `--version` sortent avec 0.
    let cli = Cli::parse();
    let now = match now::resolve_now(cli.now.as_deref()) {
        Ok(m) => m,
        Err(message) => {
            cmd::warn(&message);
            return ExitCode::from(exit::USAGE);
        }
    };
    let ctx = cmd::Ctx {
        now,
        lang: match cli.lang {
            Some(LangArg::Fr) => Language::Fr,
            Some(LangArg::En) => Language::En,
            None => now::env_language(),
        },
        lang_explicit: cli.lang.is_some(),
    };
    match cli.command {
        Command::Summary(a) => cmd::summary::run(&ctx, &a),
        Command::Tasks(a) => cmd::tasks::run(&ctx, &a),
        Command::Apply(a) => cmd::apply::run(&ctx, &a),
        Command::Render(a) => cmd::render::run(&ctx, &a),
        Command::ImportCsv(a) => cmd::import_csv::run(&ctx, &a),
        Command::Mcp(a) => cmd::mcp::run(&ctx, a),
        Command::Explain(a) => cmd::explain::run(&ctx, &a),
    }
}
