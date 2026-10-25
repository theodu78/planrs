//! Écouteurs de l'application sur une socket Unix, dans un fil dédié :
//!
//! - [`spawn_listener_in`] (J7, lot L3, ADR-0015 D9) : relais. Chaque
//!   requête authentifiée (`query`, `execute`, `ui_state`, `screenshot`)
//!   devient un [`RelayCall`] poussé dans la file de l'application, qui
//!   répond par son canal ; l'écouteur ne touche jamais la session ;
//! - [`spawn_hello_listener_in`] (J4, lot L10) : `hello` et `query`
//!   (`project.summary`) seulement, sans relais (garde du fichier ; gardé
//!   pour les tests des surfaces qui simulent une application).
//!
//! Chaque connexion commence par un `hello` porteur du jeton ; toute autre
//! requête avant, ou un jeton faux, est refusée et la connexion fermée
//! (C-CTL-01).
//!
//! Windows : pas d'écouteur ([`ControlError::Unsupported`] ; règle des
//! 24 h, ADR-0015).

use std::fmt;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{ControlError, RelayCall, Rendezvous, Waker, decode_token, state_dir};
use crate::view::SummaryView;

/// Capacité conseillée de la file des appels relayés (64) : au-delà,
/// l'écouteur répond `control.busy`.
pub const RELAY_QUEUE: usize = 64;

/// Délai d'un appel relayé, validation comprise (120 s, C-CTL-04).
pub const APPROVAL_TIMEOUT: Duration = Duration::from_secs(120);

/// Réglages de l'écouteur du relais.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListenerOptions {
    /// Délai d'un appel depuis sa mise en file (validation comprise) ; à
    /// l'expiration, `control.approval_timeout` et l'appel est retiré.
    pub approval_timeout: Duration,
    /// Attente de la réponse d'un appel que l'application a pris juste
    /// avant l'expiration.
    pub reply_grace: Duration,
}

impl Default for ListenerOptions {
    fn default() -> Self {
        Self {
            approval_timeout: APPROVAL_TIMEOUT,
            reply_grace: Duration::from_secs(10),
        }
    }
}

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

impl ListenerHandle {
    /// Remplace le jeton de l'écouteur (relecture finale J7, relais F2 :
    /// un document neuf dans l'application). Les connexions déjà
    /// authentifiées se terminent normalement ; tout `hello` suivant doit
    /// porter le nouveau jeton, que [`ListenerHandle::rendezvous`] publie :
    /// un agent rattaché au document précédent est refusé
    /// (`control.bad_token`) au lieu d'agir en silence sur le nouveau.
    ///
    /// # Errors
    ///
    /// [`ControlError::Io`] si le générateur du système est indisponible
    /// (le jeton précédent reste) ; [`ControlError::Unsupported`] hors
    /// Unix.
    pub fn rotate_token(&mut self) -> Result<(), ControlError> {
        let token = super::new_token()?;
        let bytes = decode_token(&token).ok_or(ControlError::InvalidToken)?;
        #[cfg(unix)]
        {
            self.running.set_token(bytes);
            self.token = token;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = bytes;
            Err(ControlError::Unsupported)
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
        let (endpoint, running) = unix::spawn(dir, expected, unix::Service::Hello(summary))?;
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

/// Écouteur du relais J7 (ADR-0015 D9, lot L3), réglages par défaut
/// ([`ListenerOptions`]) : voir [`spawn_listener_with`].
///
/// # Errors
///
/// Celles de [`spawn_listener_with`].
pub fn spawn_listener_in(
    dir: &std::path::Path,
    token: String,
    inbox: SyncSender<RelayCall>,
    wake: Waker,
) -> Result<ListenerHandle, ControlError> {
    spawn_listener_with(dir, token, inbox, wake, ListenerOptions::default())
}

/// Lance l'écouteur du relais, socket `<pid>-<n>.sock` (0600) dans `dir`
/// (0700) : chaque requête authentifiée devient un [`RelayCall`] poussé
/// dans `inbox` (file bornée de l'application ; pleine : `control.busy`,
/// connexion maintenue), puis `wake` réveille l'interface. Les réponses de
/// l'application (`pending_approval`, puis `ok` ou `err`) partent sur la
/// même connexion. Sans réponse finale en `options.approval_timeout`,
/// l'appel expire ([`super::CallState::expire`]), un `Cancel(id)` suit
/// dans la file et l'agent reçoit `control.approval_timeout`.
///
/// # Errors
///
/// [`ControlError::InvalidToken`], [`ControlError::Invalid`] (chemin non
/// UTF-8), [`ControlError::Io`] (dossier, socket, fil) ;
/// [`ControlError::Unsupported`] hors Unix.
pub fn spawn_listener_with(
    dir: &std::path::Path,
    token: String,
    inbox: SyncSender<RelayCall>,
    wake: Waker,
    options: ListenerOptions,
) -> Result<ListenerHandle, ControlError> {
    let expected = decode_token(&token).ok_or(ControlError::InvalidToken)?;
    #[cfg(unix)]
    {
        let service = unix::Service::Relay { inbox, wake, options };
        let (endpoint, running) = unix::spawn(dir, expected, service)?;
        Ok(ListenerHandle {
            endpoint,
            token,
            running,
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (dir, expected, options);
        drop((token, inbox, wake));
        Err(ControlError::Unsupported)
    }
}

#[cfg(unix)]
mod unix {
    use std::io::{BufRead, BufReader, ErrorKind, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc::{RecvTimeoutError, SyncSender, TrySendError, sync_channel};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use serde_json::{Map, Value, json};

    use super::super::{
        CallState, ControlError, Frame, LineError, MAX_LINE_BYTES, Method, PROTOCOL_VERSION, RelayCall,
        RelayKind, Reply, Waker, codes, ensure_dir, next_stem, read_line_bounded, to_line, token_matches,
    };
    use super::ListenerOptions;
    use crate::error::codes as tool_codes;
    use crate::registry::{Kind, lookup};
    use crate::view::SummaryView;

    /// Connexions servies en même temps ; au-delà, refusées par
    /// `control.busy` (le client ne se détache pas).
    const MAX_CONNECTIONS: usize = 8;
    /// Connexion authentifiée sans requête fermée après ce délai.
    const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
    /// Connexion fermée si son `hello` n'arrive pas dans ce délai (une place
    /// sur huit n'est pas tenue 30 s par une connexion muette).
    const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
    /// Pas de la surveillance de la connexion d'un appel en attente : un
    /// client parti (délai écoulé, processus mort) est vu en au plus ce
    /// délai, et son appel expiré.
    const HANGUP_POLL: Duration = Duration::from_millis(100);
    /// Délai d'écriture d'une réponse.
    const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
    /// Lecture servie par défaut (`query` sans `command`).
    const SUMMARY_COMMAND: &str = "project.summary";

    /// Ce que sert l'écouteur.
    pub(super) enum Service {
        /// J4 : `hello` et le résumé tenu par l'application.
        Hello(Arc<Mutex<Option<SummaryView>>>),
        /// J7 : relais vers la file de l'application.
        Relay {
            /// File de l'application.
            inbox: SyncSender<RelayCall>,
            /// Réveil de l'interface.
            wake: Waker,
            /// Délais.
            options: ListenerOptions,
        },
    }

    struct Shared {
        stop: AtomicBool,
        /// Jeton attendu, remplacé par `ListenerHandle::rotate_token`.
        token: Mutex<[u8; 32]>,
        service: Service,
        active: AtomicUsize,
    }

    pub(super) struct Running {
        shared: Arc<Shared>,
        socket: PathBuf,
        thread: Option<JoinHandle<()>>,
    }

    impl Running {
        pub(super) fn set_token(&self, token: [u8; 32]) {
            *self.shared.token.lock().unwrap_or_else(PoisonError::into_inner) = token;
        }

        pub(super) fn stop(&mut self) {
            self.shared.stop.store(true, Ordering::SeqCst);
            // Réveille `accept`. Socket injoignable (fichier supprimé,
            // dossier retiré) : le fil n'est pas attendu, pour ne jamais
            // bloquer la fermeture ; il reste suspendu sur `accept`.
            let woken = UnixStream::connect(&self.socket).is_ok();
            if let Some(thread) = self.thread.take()
                && (woken || thread.is_finished())
            {
                let _ = thread.join();
            }
            let _ = std::fs::remove_file(&self.socket);
        }
    }

    pub(super) fn spawn(
        dir: &Path,
        token: [u8; 32],
        service: Service,
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
            token: Mutex::new(token),
            service,
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
                // Refus dit au client (relecture finale J7, relais F4) :
                // une fermeture muette se lisait « application partie » et
                // détachait l'agent d'une application vivante.
                let busy = Reply::error(
                    0,
                    codes::BUSY,
                    format!("{MAX_CONNECTIONS} connexions déjà servies : réessayer"),
                );
                if stream.set_write_timeout(Some(WRITE_TIMEOUT)).is_ok() {
                    let _ = send(&stream, &busy);
                }
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

    /// Suite d'une requête lue.
    enum Step {
        /// Réponse immédiate, et si la connexion reste ouverte.
        Reply(Reply, bool),
        /// Appel à relayer à l'application, avec le délai de validation
        /// demandé par le client (`approval_timeout_ms`), s'il y en a un.
        Relay(u64, RelayKind, Option<Duration>),
    }

    fn serve(stream: &UnixStream, shared: &Shared) {
        if stream.set_read_timeout(Some(HELLO_TIMEOUT)).is_err()
            || stream.set_write_timeout(Some(WRITE_TIMEOUT)).is_err()
        {
            return;
        }
        let mut reader = BufReader::new(stream);
        let mut authenticated = false;
        loop {
            // `hello` accepté : délai d'une connexion inactive.
            let timeout = if authenticated {
                IDLE_TIMEOUT
            } else {
                HELLO_TIMEOUT
            };
            if stream.set_read_timeout(Some(timeout)).is_err() {
                return;
            }
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
            match respond(&line, &mut authenticated, shared) {
                Step::Reply(reply, keep_open) => {
                    if send(stream, &reply).is_err() || !keep_open {
                        return;
                    }
                }
                Step::Relay(id, kind, asked) => {
                    if relay(&mut reader, shared, id, kind, asked).is_err() {
                        return;
                    }
                }
            }
        }
    }

    fn send(mut stream: &UnixStream, reply: &Reply) -> std::io::Result<()> {
        let line = to_line(reply).map_err(std::io::Error::other)?;
        stream.write_all(&line)
    }

    /// Réponse à une ligne, ou appel à relayer.
    fn respond(line: &[u8], authenticated: &mut bool, shared: &Shared) -> Step {
        let frame = match serde_json::from_slice::<Frame>(line) {
            Ok(frame) => frame,
            Err(e) => {
                // L'identifiant est repris s'il se lit encore.
                let id = serde_json::from_slice::<Value>(line)
                    .ok()
                    .and_then(|v| v.get("id").and_then(Value::as_u64))
                    .unwrap_or(0);
                let reply = Reply::error(id, codes::INVALID_FRAME, format!("trame illisible : {e}"));
                return Step::Reply(reply, *authenticated);
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
            return Step::Reply(reply, *authenticated);
        }
        if frame.method == Method::Hello {
            let given = frame.params.get("token").and_then(Value::as_str).unwrap_or("");
            let expected = *shared.token.lock().unwrap_or_else(PoisonError::into_inner);
            return if token_matches(&expected, given) {
                *authenticated = true;
                let result = hello_result(&shared.service);
                Step::Reply(Reply::Ok { id, result }, true)
            } else {
                *authenticated = false;
                Step::Reply(Reply::error(id, codes::BAD_TOKEN, "jeton refusé"), false)
            };
        }
        if !*authenticated {
            return Step::Reply(
                Reply::error(id, codes::UNAUTHENTICATED, "hello avec jeton attendu en premier"),
                false,
            );
        }
        match &shared.service {
            Service::Hello(summary) => match frame.method {
                Method::Query => Step::Reply(query_summary(id, &frame.params, summary), true),
                other => Step::Reply(
                    Reply::error(
                        id,
                        codes::UNSUPPORTED,
                        format!(
                            "méthode « {} » non prise en charge par cet écouteur",
                            other.wire_name()
                        ),
                    ),
                    true,
                ),
            },
            Service::Relay { .. } => match relay_kind(frame.method, frame.params) {
                Ok((kind, asked)) => Step::Relay(id, kind, asked),
                Err((code, message)) => Step::Reply(Reply::error(id, code, message), true),
            },
        }
    }

    fn hello_result(service: &Service) -> Value {
        let relay = matches!(service, Service::Relay { .. });
        let methods: Vec<&str> = Method::ALL
            .iter()
            .filter(|m| relay || m.available_in_j4())
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

    /// Paramètres en objet (`null` : objet vide) ; sinon refus.
    fn object(params: Value) -> Result<Map<String, Value>, (&'static str, String)> {
        match params {
            Value::Null => Ok(Map::new()),
            Value::Object(map) => Ok(map),
            _ => Err((codes::INVALID_FRAME, "« params » doit être un objet".to_owned())),
        }
    }

    /// Refuse les clés restantes.
    fn no_more(map: &Map<String, Value>) -> Result<(), (&'static str, String)> {
        match map.keys().next() {
            None => Ok(()),
            Some(k) => Err((codes::INVALID_FRAME, format!("champ inconnu « {k} »"))),
        }
    }

    /// Délai de validation demandé par le client (`approval_timeout_ms`,
    /// entier positif), retiré de `map`.
    fn asked_timeout(map: &mut Map<String, Value>) -> Result<Option<Duration>, (&'static str, String)> {
        match map.remove("approval_timeout_ms") {
            None | Some(Value::Null) => Ok(None),
            Some(v) => v
                .as_u64()
                .filter(|ms| *ms > 0)
                .map(|ms| Some(Duration::from_millis(ms)))
                .ok_or((
                    codes::INVALID_FRAME,
                    "« approval_timeout_ms » doit être un entier positif".to_owned(),
                )),
        }
    }

    /// Appel relayé d'une requête authentifiée, et le délai de validation
    /// demandé (`execute` seulement).
    fn relay_kind(
        method: Method,
        params: Value,
    ) -> Result<(RelayKind, Option<Duration>), (&'static str, String)> {
        let mut map = object(params)?;
        match method {
            // Traité avant.
            Method::Hello => Err((codes::INVALID_FRAME, "hello déjà traité".to_owned())),
            Method::Query => {
                let tool = match map.remove("command") {
                    None | Some(Value::Null) => SUMMARY_COMMAND.to_owned(),
                    Some(Value::String(c)) => c,
                    Some(_) => {
                        return Err((
                            codes::INVALID_FRAME,
                            "« command » doit être une chaîne".to_owned(),
                        ));
                    }
                };
                let params = map.remove("params").unwrap_or_else(|| Value::Object(Map::new()));
                no_more(&map)?;
                let readable = lookup(&tool).is_some_and(|s| matches!(s.kind, Kind::Read | Kind::Analysis));
                if !readable {
                    return Err((
                        codes::UNSUPPORTED,
                        format!("« {tool} » n'est pas une lecture : passer par execute"),
                    ));
                }
                Ok((RelayKind::Execute { tool, params }, None))
            }
            Method::Execute => {
                let Some(Value::String(tool)) = map.remove("tool") else {
                    return Err((codes::INVALID_FRAME, "« tool » (chaîne) attendu".to_owned()));
                };
                let params = map.remove("params").unwrap_or_else(|| Value::Object(Map::new()));
                let asked = asked_timeout(&mut map)?;
                no_more(&map)?;
                Ok((RelayKind::Execute { tool, params }, asked))
            }
            Method::UiState => {
                no_more(&map)?;
                Ok((RelayKind::UiState, None))
            }
            Method::Screenshot => {
                let scale_down = match map.remove("scale_down") {
                    None | Some(Value::Null) => 0,
                    Some(v) => v.as_u64().map(|n| u8::try_from(n).unwrap_or(u8::MAX)).ok_or((
                        codes::INVALID_FRAME,
                        "« scale_down » doit être un entier".to_owned(),
                    ))?,
                };
                no_more(&map)?;
                Ok((RelayKind::Screenshot { scale_down }, None))
            }
        }
    }

    /// Relaie un appel à l'application et transmet ses réponses ; rend
    /// l'erreur d'écriture sur la connexion (connexion à fermer). Délai :
    /// celui de l'écouteur, ou celui demandé par le client s'il est plus
    /// court (relecture finale J7, relais F1 : sinon l'agent apprenait
    /// « rien n'a été appliqué » alors que le bandeau restait acceptable).
    /// Pendant l'attente, la connexion est surveillée : client parti ⇒
    /// appel expiré, bandeau retiré.
    fn relay(
        reader: &mut BufReader<&UnixStream>,
        shared: &Shared,
        id: u64,
        kind: RelayKind,
        asked: Option<Duration>,
    ) -> std::io::Result<()> {
        let stream = *reader.get_ref();
        let Service::Relay { inbox, wake, options } = &shared.service else {
            return send(stream, &Reply::error(id, codes::UNSUPPORTED, "relais non servi"));
        };
        let (tx, rx) = sync_channel::<Reply>(4);
        let state = Arc::new(CallState::new());
        let call = RelayCall {
            id,
            kind,
            state: Arc::clone(&state),
            reply: tx,
        };
        match inbox.try_send(call) {
            Ok(()) => wake(),
            Err(TrySendError::Full(_)) => {
                return send(
                    stream,
                    &Reply::error(
                        id,
                        codes::BUSY,
                        "file des appels de l'application pleine : réessayer",
                    ),
                );
            }
            Err(TrySendError::Disconnected(_)) => {
                return send(
                    stream,
                    &Reply::error(
                        id,
                        tool_codes::CONTROL_APP_GONE,
                        "application en cours de fermeture",
                    ),
                );
            }
        }
        let timeout = asked.map_or(options.approval_timeout, |asked| {
            asked.min(options.approval_timeout)
        });
        let start = Instant::now();
        let mut pending = false;
        loop {
            let left = timeout.saturating_sub(start.elapsed());
            match rx.recv_timeout(left.min(HANGUP_POLL)) {
                Ok(reply @ Reply::PendingApproval { .. }) => {
                    pending = true;
                    send(stream, &reply)?;
                }
                Ok(reply) => return send(stream, &reply),
                Err(RecvTimeoutError::Disconnected) => {
                    return send(
                        stream,
                        &Reply::error(id, codes::INTERNAL, "appel abandonné par l'application"),
                    );
                }
                Err(RecvTimeoutError::Timeout) if !left.is_zero() && left > HANGUP_POLL => {
                    if hung_up(reader) {
                        // Client parti : plus personne à qui répondre.
                        if state.expire() {
                            cancel(inbox, wake, id, &state);
                        }
                        return Err(std::io::Error::from(ErrorKind::ConnectionAborted));
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if state.expire() {
                        cancel(inbox, wake, id, &state);
                        return send(stream, &expired(id, pending));
                    }
                    // Pris juste avant l'expiration : la réponse suit.
                    return match rx.recv_timeout(options.reply_grace) {
                        Ok(Reply::PendingApproval { .. }) | Err(_) => send(stream, &expired(id, pending)),
                        Ok(reply) => send(stream, &reply),
                    };
                }
            }
        }
    }

    /// Vrai si le client a fermé la connexion (fin de flux), lu sans
    /// attendre ; un octet reçu reste dans le tampon du lecteur.
    fn hung_up(reader: &mut BufReader<&UnixStream>) -> bool {
        let stream = *reader.get_ref();
        if stream.set_nonblocking(true).is_err() {
            return false;
        }
        let gone = match reader.fill_buf() {
            Ok(buf) => buf.is_empty(),
            Err(e) => !matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted),
        };
        let _ = stream.set_nonblocking(false);
        gone
    }

    /// Réponse d'un appel expiré : validation attendue (`approval_timeout`)
    /// ou application muette.
    fn expired(id: u64, pending: bool) -> Reply {
        if pending {
            Reply::error(
                id,
                tool_codes::CONTROL_APPROVAL_TIMEOUT,
                "aucune réponse de l'utilisateur dans le délai : rien n'a été appliqué",
            )
        } else {
            Reply::error(id, tool_codes::CONTROL_APP_GONE, "l'application ne répond pas")
        }
    }

    /// Signale à l'application l'abandon de l'appel `id` (bandeau retiré) ;
    /// le `state` de l'appel abandonné l'accompagne : l'application ne
    /// retire que cet appel-là, pas celui d'un autre agent au même `id`
    /// (relecture finale J7, relais F3). File pleine ou fermée :
    /// l'application voit l'appel expiré à la prochaine image
    /// ([`CallState::is_pending`]).
    fn cancel(inbox: &SyncSender<RelayCall>, wake: &Waker, id: u64, state: &Arc<CallState>) {
        let (reply, _) = sync_channel(1);
        let call = RelayCall {
            id,
            kind: RelayKind::Cancel(id),
            state: Arc::clone(state),
            reply,
        };
        if inbox.try_send(call).is_ok() {
            wake();
        }
    }

    /// `query` du J4 : `params` nul, `{}` ou `{"command":"project.summary"}`.
    fn query_summary(id: u64, params: &Value, summary: &Mutex<Option<SummaryView>>) -> Reply {
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
                format!("lecture « {c} » non prise en charge par cet écouteur (seule : {SUMMARY_COMMAND})"),
            );
        }
        let summary = summary.lock().unwrap_or_else(PoisonError::into_inner).clone();
        match serde_json::to_value(summary) {
            Ok(result) => Reply::Ok { id, result },
            Err(e) => Reply::error(id, codes::INTERNAL, format!("résumé illisible : {e}")),
        }
    }
}
