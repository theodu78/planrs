//! Écouteur minimal de l'application (lot L10) : `hello` et `query`
//! (`project.summary`) sur une socket Unix, dans un fil dédié.
//!
//! Chaque connexion commence par un `hello` porteur du jeton ; toute autre
//! requête avant, ou un jeton faux, est refusée et la connexion fermée. En
//! J4, `execute`, `undo`, `redo`, `history`, `ui_state` et `screenshot`
//! répondent `control.unsupported` (relais des commandes en J7).
//!
//! Windows : pas d'écouteur en J4 ([`ControlError::Unsupported`]).

use std::fmt;
use std::sync::{Arc, Mutex};

use super::{ControlError, Rendezvous, decode_token, state_dir};
use crate::view::SummaryView;

/// Écouteur en cours ; arrêté à sa destruction (socket supprimée).
pub struct ListenerHandle {
    endpoint: String,
    token: String,
    #[cfg(unix)]
    running: unix::Running,
}

impl fmt::Debug for ListenerHandle {
    // Le jeton n'apparaît jamais dans une trace.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListenerHandle")
            .field("endpoint", &self.endpoint)
            .field("token", &"…")
            .finish_non_exhaustive()
    }
}

impl ListenerHandle {
    /// Adresse de l'écouteur (chemin de la socket).
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Rendez-vous à publier pour cet écouteur (`project` : chemin
    /// canonique, voir [`super::canonical_project`]).
    #[must_use]
    pub fn rendezvous(&self, project: Option<String>) -> Rendezvous {
        Rendezvous {
            v: super::PROTOCOL_VERSION,
            pid: std::process::id(),
            endpoint: self.endpoint.clone(),
            token: self.token.clone(),
            project,
        }
    }
}

#[cfg(unix)]
impl Drop for ListenerHandle {
    fn drop(&mut self) {
        self.running.stop();
    }
}

/// Lance l'écouteur `hello`/`query` dans un fil dédié, socket dans
/// [`state_dir`].
///
/// `summary` est le résumé du document ouvert, tenu à jour par
/// l'application (`None` : aucun document).
///
/// # Errors
///
/// [`ControlError::NoStateDir`], ou celles de [`spawn_hello_listener_in`].
pub fn spawn_hello_listener(
    token: String,
    summary: Arc<Mutex<Option<SummaryView>>>,
) -> Result<ListenerHandle, ControlError> {
    let dir = state_dir().ok_or(ControlError::NoStateDir)?;
    spawn_hello_listener_in(&dir, token, summary)
}

/// Lance l'écouteur, socket `<pid>-<n>.sock` (0600) dans `dir` (0700).
///
/// # Errors
///
/// [`ControlError::InvalidToken`], [`ControlError::Invalid`] (chemin non
/// UTF-8), [`ControlError::Io`] (dossier, socket, fil) ;
/// [`ControlError::Unsupported`] hors Unix.
pub fn spawn_hello_listener_in(
    dir: &std::path::Path,
    token: String,
    summary: Arc<Mutex<Option<SummaryView>>>,
) -> Result<ListenerHandle, ControlError> {
    let expected = decode_token(&token).ok_or(ControlError::InvalidToken)?;
    #[cfg(unix)]
    {
        let (endpoint, running) = unix::spawn(dir, expected, summary)?;
        Ok(ListenerHandle {
            endpoint,
            token,
            running,
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (dir, expected);
        drop((token, summary));
        Err(ControlError::Unsupported)
    }
}

/// Écouteur du relais J7 (ADR-0015 D9) : chaque requête authentifiée
/// devient un [`super::RelayCall`] poussé dans `inbox`, puis `wake` réveille
/// l'interface. Remplace [`spawn_hello_listener_in`] au lot L3.
///
/// # Errors
///
/// [`ControlError::InvalidToken`] ; squelette J7 L0 :
/// [`ControlError::Unsupported`].
#[allow(
    clippy::needless_pass_by_value,
    reason = "contrat J7 : l'écouteur du lot L3 garde le jeton, la file et le rappel"
)]
pub fn spawn_listener_in(
    dir: &std::path::Path,
    token: String,
    inbox: std::sync::mpsc::SyncSender<super::RelayCall>,
    wake: super::Waker,
) -> Result<ListenerHandle, ControlError> {
    let _ = decode_token(&token).ok_or(ControlError::InvalidToken)?;
    drop((dir, inbox, wake));
    Err(ControlError::Unsupported)
}

#[cfg(unix)]
mod unix {
    use std::io::{BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use serde_json::{Value, json};

    use super::super::{
        ControlError, Frame, LineError, MAX_LINE_BYTES, Method, PROTOCOL_VERSION, Reply, codes, ensure_dir,
        next_stem, read_line_bounded, to_line, token_matches,
    };
    use crate::view::SummaryView;

    /// Connexions servies en même temps ; au-delà, refusées.
    const MAX_CONNECTIONS: usize = 8;
    /// Connexion sans requête fermée après ce délai.
    const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
    /// Délai d'écriture d'une réponse.
    const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
    /// Lecture unique servie en J4.
    const SUMMARY_COMMAND: &str = "project.summary";

    struct Shared {
        stop: AtomicBool,
        token: [u8; 32],
        summary: Arc<Mutex<Option<SummaryView>>>,
        active: AtomicUsize,
    }

    pub(super) struct Running {
        shared: Arc<Shared>,
        socket: PathBuf,
        thread: Option<JoinHandle<()>>,
    }

    impl Running {
        pub(super) fn stop(&mut self) {
            self.shared.stop.store(true, Ordering::SeqCst);
            // Réveille `accept` ; l'échec signifie que le fil est déjà
            // sorti.
            let _ = UnixStream::connect(&self.socket);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
            let _ = std::fs::remove_file(&self.socket);
        }
    }

    pub(super) fn spawn(
        dir: &Path,
        token: [u8; 32],
        summary: Arc<Mutex<Option<SummaryView>>>,
    ) -> Result<(String, Running), ControlError> {
        use std::os::unix::fs::PermissionsExt;

        ensure_dir(dir)?;
        let socket = dir.join(format!("{}.sock", next_stem()));
        let endpoint = socket
            .to_str()
            .ok_or_else(|| ControlError::Invalid("chemin de socket non UTF-8".to_owned()))?
            .to_owned();
        // Reste éventuel d'un processus mort au même pid.
        let _ = std::fs::remove_file(&socket);
        let listener =
            UnixListener::bind(&socket).map_err(|e| ControlError::io("ouverture de la socket", &e))?;
        let cleanup = |e: ControlError| {
            let _ = std::fs::remove_file(&socket);
            e
        };
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| cleanup(ControlError::io("droits de la socket", &e)))?;
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            token,
            summary,
            active: AtomicUsize::new(0),
        });
        let worker = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("planrs-control".to_owned())
            .spawn(move || accept_loop(&listener, &worker))
            .map_err(|e| cleanup(ControlError::io("fil de l'écouteur", &e)))?;
        Ok((
            endpoint,
            Running {
                shared,
                socket,
                thread: Some(thread),
            },
        ))
    }

    /// Décrémente le compteur de connexions à la fin du fil.
    struct ActiveGuard(Arc<Shared>);

    impl Drop for ActiveGuard {
        fn drop(&mut self) {
            self.0.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    fn accept_loop(listener: &UnixListener, shared: &Arc<Shared>) {
        for conn in listener.incoming() {
            if shared.stop.load(Ordering::SeqCst) {
                break;
            }
            let Ok(stream) = conn else {
                // Descripteurs épuisés… : on laisse respirer le système.
                thread::sleep(Duration::from_millis(20));
                continue;
            };
            if shared.active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                shared.active.fetch_sub(1, Ordering::SeqCst);
                continue;
            }
            let guard = ActiveGuard(Arc::clone(shared));
            // Échec du fil : la fermeture de `guard` rend la place.
            let _ = thread::Builder::new()
                .name("planrs-control-conn".to_owned())
                .spawn(move || {
                    serve(&stream, &guard.0);
                    drop(guard);
                });
        }
    }

    fn serve(stream: &UnixStream, shared: &Shared) {
        if stream.set_read_timeout(Some(IDLE_TIMEOUT)).is_err()
            || stream.set_write_timeout(Some(WRITE_TIMEOUT)).is_err()
        {
            return;
        }
        let mut reader = BufReader::new(stream);
        let mut authenticated = false;
        loop {
            let line = match read_line_bounded(&mut reader, MAX_LINE_BYTES) {
                Ok(Some(line)) => line,
                Ok(None) | Err(LineError::Io(_)) => return,
                Err(LineError::TooLong) => {
                    let reply = Reply::error(
                        0,
                        codes::LINE_TOO_LONG,
                        format!("ligne de plus de {MAX_LINE_BYTES} octets"),
                    );
                    let _ = send(stream, &reply);
                    return;
                }
            };
            if shared.stop.load(Ordering::SeqCst) {
                return;
            }
            let (reply, keep_open) = respond(&line, &mut authenticated, shared);
            if send(stream, &reply).is_err() || !keep_open {
                return;
            }
        }
    }

    fn send(mut stream: &UnixStream, reply: &Reply) -> std::io::Result<()> {
        let line = to_line(reply).map_err(std::io::Error::other)?;
        stream.write_all(&line)
    }

    /// Réponse à une ligne, et si la connexion reste ouverte.
    fn respond(line: &[u8], authenticated: &mut bool, shared: &Shared) -> (Reply, bool) {
        let frame = match serde_json::from_slice::<Frame>(line) {
            Ok(frame) => frame,
            Err(e) => {
                // L'identifiant est repris s'il se lit encore.
                let id = serde_json::from_slice::<Value>(line)
                    .ok()
                    .and_then(|v| v.get("id").and_then(Value::as_u64))
                    .unwrap_or(0);
                let reply = Reply::error(id, codes::INVALID_FRAME, format!("trame illisible : {e}"));
                return (reply, *authenticated);
            }
        };
        let id = frame.id;
        if frame.v != PROTOCOL_VERSION {
            let reply = Reply::error(
                id,
                codes::UNSUPPORTED_VERSION,
                format!(
                    "version {} du protocole non prise en charge (attendue : {PROTOCOL_VERSION})",
                    frame.v
                ),
            );
            return (reply, *authenticated);
        }
        match frame.method {
            Method::Hello => {
                let given = frame.params.get("token").and_then(Value::as_str).unwrap_or("");
                if token_matches(&shared.token, given) {
                    *authenticated = true;
                    (
                        Reply::Ok {
                            id,
                            result: hello_result(),
                        },
                        true,
                    )
                } else {
                    *authenticated = false;
                    (Reply::error(id, codes::BAD_TOKEN, "jeton refusé"), false)
                }
            }
            _ if !*authenticated => (
                Reply::error(id, codes::UNAUTHENTICATED, "hello avec jeton attendu en premier"),
                false,
            ),
            Method::Query => (query(id, &frame.params, shared), true),
            other => (
                Reply::error(
                    id,
                    codes::UNSUPPORTED,
                    format!(
                        "méthode « {} » non prise en charge en J4 (relais en J7)",
                        other.wire_name()
                    ),
                ),
                true,
            ),
        }
    }

    fn hello_result() -> Value {
        let methods: Vec<&str> = Method::ALL
            .iter()
            .filter(|m| m.available_in_j4())
            .map(|m| m.wire_name())
            .collect();
        json!({
            "v": PROTOCOL_VERSION,
            "app": "planrs",
            "version": env!("CARGO_PKG_VERSION"),
            "pid": std::process::id(),
            "methods": methods,
        })
    }

    /// `query` : `params` nul, `{}` ou `{"command":"project.summary"}`.
    fn query(id: u64, params: &Value, shared: &Shared) -> Reply {
        let command = match params {
            Value::Null => None,
            Value::Object(map) => match map.get("command") {
                None => None,
                Some(Value::String(c)) => Some(c.as_str()),
                Some(_) => {
                    return Reply::error(id, codes::INVALID_FRAME, "« command » doit être une chaîne");
                }
            },
            _ => return Reply::error(id, codes::INVALID_FRAME, "« params » doit être un objet"),
        };
        if let Some(c) = command.filter(|c| *c != SUMMARY_COMMAND) {
            return Reply::error(
                id,
                codes::UNSUPPORTED,
                format!("lecture « {c} » non prise en charge en J4 (seule : {SUMMARY_COMMAND})"),
            );
        }
        let summary = shared
            .summary
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        match serde_json::to_value(summary) {
            Ok(result) => Reply::Ok { id, result },
            Err(e) => Reply::error(id, codes::INTERNAL, format!("résumé illisible : {e}")),
        }
    }
}
