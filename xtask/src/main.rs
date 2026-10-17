//! Tâches de maintenance du dépôt (motif cargo-xtask), lancées par
//! `cargo xtask <tâche>` grâce à l'alias de `.cargo/config.toml`.
//!
//! Tâches :
//! - `layers` : vérifie la règle 9 (couches strictes). Aucune crate du
//!   workspace, hors exemptions, ne doit dépendre, directement ou
//!   transitivement, d'egui, eframe, winit ou rfd.
//! - `corpus fetch` : télécharge les fichiers du corpus public
//!   (`tests/corpus/manifest.toml`), contrôle taille et SHA-256.
//! - `panics` : complète clippy pour la règle 10 (ne jamais planter). Refuse
//!   hors du code de test les `allow`/`expect` qui lèvent un interdit de la
//!   règle 10 et la famille `assert!`, qu'aucune lint clippy ne refuse.

mod corpus;
mod layers;
mod panics;

use std::process::ExitCode;

const USAGE: &str = "usage : cargo xtask <tâche>

tâches :
  layers   vérifie les couches (règle 9)
  panics   vérifie les interdits de la règle 10 que clippy ne couvre pas
  corpus fetch
           télécharge le corpus public (tests/corpus/files, sha256 vérifié)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("layers") => layers::run(),
        Some("panics") => panics::run(),
        Some("corpus") => corpus::run(args.get(1..).unwrap_or_default()),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}
