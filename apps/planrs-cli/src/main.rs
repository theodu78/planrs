//! Binaire `planrs` en ligne de commande.
//!
//! Il exposera les mêmes opérations que le serveur MCP (`summary`, `tasks`,
//! `explain`, `apply`, `render`, `mcp`), toutes via le registre de commandes
//! de `planrs-automation`. État J1 : seule l'option `--version` existe.

use clap::Parser;
use planrs_core::PRODUCT_NAME;

/// Planification de projet compatible MS Project, pilotable par IA.
#[derive(Debug, Parser)]
#[command(name = PRODUCT_NAME, version, arg_required_else_help = true)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
