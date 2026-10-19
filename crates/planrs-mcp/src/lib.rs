//! Serveur MCP de planrs (SDK officiel `rmcp`, transport stdio).
//!
//! Il expose les commandes de `planrs-automation` à un agent IA. Il ne
//! calcule jamais une date lui-même et ne renvoie jamais de résultat estimé :
//! tout vient du moteur (règle 3). Les outils sont générés depuis
//! `planrs_automation::REGISTRY`, au-dessus de `Workspace` (ADR-0012 D7,
//! D8). Un fichier tenu par une application vivante est refusé
//! (`file.open_in_app`, ADR-0002).
//!
//! Transport stdio (tokio à fil unique), lignes d'entrée bornées
//! (C-MCP-06), journal de rejeu `--journal` facultatif. Détail :
//! `docs/features/mcp.md` (L18) et ADR-0012.

mod bounded;
mod journal_file;
mod server;
mod tools;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use planrs_automation::text::Language;
use planrs_core::Moment;
use rmcp::service::ServerInitializeError;
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
    /// Squelette L0 ; plus renvoyé depuis L8 (gardé pour ne pas casser
    /// les `match` des appelants).
    #[error("serveur MCP pas encore disponible (J4, L8)")]
    NotYetAvailable,
}

/// Sert sur l'entrée et la sortie standard (runtime tokio à fil unique).
///
/// # Errors
///
/// [`McpServeError::Io`] (runtime, journal, fichier `--open`),
/// [`McpServeError::LineTooLong`] si la session s'est close sur une ligne
/// trop longue (C-MCP-06 : la CLI sort avec le code 3).
pub fn serve_stdio(opts: ServeOptions) -> Result<(), McpServeError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| McpServeError::Io(format!("runtime tokio : {e}")))?;
    let res = rt.block_on(serve_on(tokio::io::stdin(), tokio::io::stdout(), opts));
    // La lecture de stdin occupe un fil bloquant qui ne rend pas la main :
    // ne pas l'attendre.
    rt.shutdown_background();
    res
}

/// Sert sur un couple lecteur/écrivain (tests : `tokio::io::duplex`).
///
/// # Errors
///
/// Celles de [`PlanrsServer::new`], [`PlanrsServer::open_initial`] et
/// [`serve_with`].
pub async fn serve_on<R, W>(r: R, w: W, opts: ServeOptions) -> Result<(), McpServeError>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    let server = PlanrsServer::new(&opts)?;
    if let Some(path) = &opts.open {
        server.open_initial(path)?;
    }
    serve_with(server, r, w, opts.max_line_bytes).await
}

/// Sert `server` sur un couple lecteur/écrivain, lignes d'entrée bornées à
/// `max_line_bytes` octets. Rend la main quand le client ferme la
/// connexion.
///
/// # Errors
///
/// [`McpServeError::LineTooLong`] (C-MCP-06), [`McpServeError::Io`]
/// (transport, tâche du service).
pub async fn serve_with<R, W>(
    server: PlanrsServer,
    r: R,
    w: W,
    max_line_bytes: usize,
) -> Result<(), McpServeError>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    let tripped = Arc::new(AtomicBool::new(false));
    let reader = bounded::BoundedLines::new(r, max_line_bytes, Arc::clone(&tripped));
    let res = match rmcp::serve_server(server, (reader, w)).await {
        Ok(running) => match running.waiting().await {
            Ok(_) => Ok(()),
            Err(e) => Err(McpServeError::Io(format!("service MCP : {e}"))),
        },
        // Entrée fermée avant l'initialisation : fin normale.
        Err(ServerInitializeError::ConnectionClosed(_)) => Ok(()),
        Err(e) => Err(McpServeError::Io(format!("initialisation MCP : {e}"))),
    };
    if tripped.load(Ordering::SeqCst) {
        return Err(McpServeError::LineTooLong);
    }
    res
}
