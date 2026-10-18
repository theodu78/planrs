//! Serveur MCP de planrs (SDK officiel `rmcp`, transport stdio).
//!
//! Il expose les commandes de `planrs-automation` à un agent IA. Il ne
//! calcule jamais une date lui-même et ne renvoie jamais de résultat estimé :
//! tout vient du moteur (règle 3). Les outils sont générés depuis
//! `planrs_automation::REGISTRY`, au-dessus de `Workspace` (ADR-0012 D7,
//! D8). Un fichier tenu par une application vivante est refusé
//! (`file.open_in_app`, ADR-0002).
//!
//! État : contrat J4 (L0) ; le serveur est écrit par le lot L8.

mod bounded;
mod journal_file;
mod server;
mod tools;

use std::path::PathBuf;

use planrs_automation::text::Language;
use planrs_core::Moment;
use tokio::io::{AsyncRead, AsyncWrite};

pub use server::PlanrsServer;

/// Options du serveur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServeOptions {
    /// « Maintenant », injecté.
    pub now: Moment,
    /// Langue par défaut des textes (`en` pour le MCP, ADR-0012).
    pub lang: Language,
    /// Journal de rejeu (`--journal`).
    pub journal: Option<PathBuf>,
    /// Projet ouvert au démarrage (`--open`).
    pub open: Option<PathBuf>,
    /// Longueur maximale d'une ligne d'entrée (8 Mio).
    pub max_line_bytes: usize,
    /// Ignore la garde ADR-0002 (`--headless`).
    pub ignore_app_guard: bool,
}

impl ServeOptions {
    /// Options par défaut à l'instant `now`.
    #[must_use]
    pub const fn new(now: Moment, lang: Language) -> Self {
        Self {
            now,
            lang,
            journal: None,
            open: None,
            max_line_bytes: 8 * 1024 * 1024,
            ignore_app_guard: false,
        }
    }
}

/// Échec du serveur.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum McpServeError {
    /// Entrée/sortie (runtime, journal, fichier `--open`).
    #[error("entrée/sortie : {0}")]
    Io(String),
    /// Transport fermé sur une ligne trop longue (C-MCP-06).
    #[error("ligne d'entrée trop longue")]
    LineTooLong,
    /// Squelette L0.
    #[error("serveur MCP pas encore disponible (J4, L8)")]
    NotYetAvailable,
}

/// Sert sur l'entrée et la sortie standard (runtime tokio à fil unique).
/// Squelette L0.
///
/// # Errors
///
/// [`McpServeError`] ; en L0, toujours [`McpServeError::NotYetAvailable`].
pub fn serve_stdio(opts: ServeOptions) -> Result<(), McpServeError> {
    drop(opts);
    Err(McpServeError::NotYetAvailable)
}

/// Sert sur un couple lecteur/écrivain (tests : `tokio::io::duplex`).
/// Squelette L0.
///
/// # Errors
///
/// [`McpServeError`] ; en L0, toujours [`McpServeError::NotYetAvailable`].
#[allow(
    clippy::unused_async,
    reason = "contrat J4 (L0) : signature asynchrone figée, corps écrit en L8"
)]
pub async fn serve_on<R, W>(r: R, w: W, opts: ServeOptions) -> Result<(), McpServeError>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    drop((r, w, opts));
    Err(McpServeError::NotYetAvailable)
}
