//! Rendez-vous : fichier 0600 dans un dossier 0700 qui annonce une
//! application vivante (lot L10).
//!
//! Un fichier `<pid>-<n>.json` par document ouvert, écrit dans un fichier
//! temporaire `.tmp` puis renommé : un lecteur ne voit jamais un rendez-vous
//! à moitié écrit. Seuls les fichiers `.json` de version 1 sont lus ; un
//! fichier d'une autre version n'est jamais supprimé.

use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ControlError, PROTOCOL_VERSION, ensure_dir, next_stem};

/// Variable d'environnement qui remplace le dossier d'état (chemin absolu ;
/// tests d'intégration, installations portables).
pub const STATE_DIR_ENV: &str = "PLANRS_CONTROL_DIR";

/// Taille maximale d'un fichier de rendez-vous lu (64 Kio).
const MAX_RENDEZVOUS_BYTES: u64 = 64 * 1024;

/// Délai de connexion et de réponse d'une sonde `hello`.
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Contenu du fichier de rendez-vous.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rendezvous {
    /// Version (1).
    pub v: u32,
    /// Processus de l'application.
    pub pid: u32,
    /// Adresse de l'écouteur (socket Unix).
    pub endpoint: String,
    /// Jeton (64 chiffres hexadécimaux).
    pub token: String,
    /// Chemin canonique du projet ouvert.
    pub project: Option<String>,
}

impl fmt::Debug for Rendezvous {
    // Le jeton n'apparaît jamais dans une trace.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rendezvous")
            .field("v", &self.v)
            .field("pid", &self.pid)
            .field("endpoint", &self.endpoint)
            .field("token", &"…")
            .field("project", &self.project)
            .finish()
    }
}

/// Fichier publié, supprimé à la destruction.
#[derive(Debug)]
pub struct RendezvousGuard {
    path: PathBuf,
}

impl RendezvousGuard {
    /// Chemin du fichier publié.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for RendezvousGuard {
    fn drop(&mut self) {
        // Échec ignoré : un rendez-vous périmé est nettoyé par
        // `find_live_for`.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Dossier des rendez-vous de la plateforme :
/// - [`STATE_DIR_ENV`] s'il est absolu ;
/// - Linux et autres Unix : `$XDG_STATE_HOME/planrs/control`, à défaut
///   `~/.local/state/planrs/control` ;
/// - macOS : `~/Library/Application Support/planrs/control` ;
/// - Windows : `%LOCALAPPDATA%\planrs\control`.
///
/// `None` si aucune base n'est connue (variables absentes ou relatives).
#[must_use]
pub fn state_dir() -> Option<PathBuf> {
    if let Some(dir) = absolute_env(STATE_DIR_ENV) {
        return Some(dir);
    }
    platform_base().map(|base| base.join("planrs").join("control"))
}

fn absolute_env(name: &str) -> Option<PathBuf> {
    let value = std::env::var_os(name)?;
    let path = PathBuf::from(value);
    path.is_absolute().then_some(path)
}

#[cfg(target_os = "macos")]
fn platform_base() -> Option<PathBuf> {
    absolute_env("HOME").map(|home| home.join("Library").join("Application Support"))
}

#[cfg(windows)]
fn platform_base() -> Option<PathBuf> {
    absolute_env("LOCALAPPDATA")
}

#[cfg(all(not(target_os = "macos"), not(windows)))]
fn platform_base() -> Option<PathBuf> {
    // Spécification XDG : une valeur relative est ignorée.
    absolute_env("XDG_STATE_HOME")
        .or_else(|| absolute_env("HOME").map(|home| home.join(".local").join("state")))
}

/// Chemin canonique d'un projet, sous la forme écrite dans
/// [`Rendezvous::project`]. `None` si le fichier n'existe pas ou si son
/// chemin n'est pas en UTF-8.
#[must_use]
pub fn canonical_project(path: &Path) -> Option<String> {
    let canonical = std::fs::canonicalize(path).ok()?;
    canonical.into_os_string().into_string().ok()
}

/// Publie un rendez-vous dans [`state_dir`].
///
/// # Errors
///
/// [`ControlError::NoStateDir`], ou celles de [`publish_in`].
pub fn publish(r: &Rendezvous) -> Result<RendezvousGuard, ControlError> {
    let dir = state_dir().ok_or(ControlError::NoStateDir)?;
    publish_in(&dir, r)
}

/// Publie un rendez-vous dans `dir` (créé en 0700) : fichier 0600, écrit
/// puis renommé.
///
/// # Errors
///
/// [`ControlError::Invalid`] (version autre que 1),
/// [`ControlError::InvalidToken`], [`ControlError::Io`].
pub fn publish_in(dir: &Path, r: &Rendezvous) -> Result<RendezvousGuard, ControlError> {
    if r.v != PROTOCOL_VERSION {
        return Err(ControlError::Invalid(format!("version {}", r.v)));
    }
    if super::decode_token(&r.token).is_none() {
        return Err(ControlError::InvalidToken);
    }
    ensure_dir(dir)?;
    let bytes = serde_json::to_vec(r).map_err(|e| ControlError::Io(e.to_string()))?;
    let stem = next_stem();
    let tmp = dir.join(format!("{stem}.tmp"));
    let path = dir.join(format!("{stem}.json"));
    // Reste éventuel d'un processus mort au même pid.
    let _ = std::fs::remove_file(&tmp);
    write_private(&tmp, &bytes).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })?;
    if let Err(e) = std::fs::rename(&tmp, &path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(ControlError::io("publication du rendez-vous", &e));
    }
    Ok(RendezvousGuard { path })
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), ControlError> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|e| ControlError::io("écriture du rendez-vous", &e))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| ControlError::io("écriture du rendez-vous", &e))
}

/// Rendez-vous vivant d'une application qui tient `path`, cherché dans
/// [`state_dir`].
#[must_use]
pub fn find_live_for(path: &Path) -> Option<Rendezvous> {
    find_live_for_in(&state_dir()?, path)
}

/// Rendez-vous vivant d'une application qui tient `path`, cherché dans
/// `dir`.
///
/// Les fichiers sont lus dans l'ordre de leur nom. Un rendez-vous qui
/// désigne `path` (comparé par chemin canonique) est sondé par un `hello`
/// avec son jeton ; un échec le supprime (application morte ou rendez-vous
/// périmé). Sont aussi supprimés : un fichier de version 1 illisible, et
/// sous Unix un rendez-vous dont la socket n'existe plus.
///
/// Windows (J4, hypothèse) : pas d'écouteur ; un rendez-vous est tenu pour
/// vivant s'il a été écrit il y a moins de 24 h. À remplacer en J7 par une
/// sonde sur le named pipe.
#[must_use]
pub fn find_live_for_in(dir: &Path, path: &Path) -> Option<Rendezvous> {
    let target = canonical_project(path)?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    for file in files {
        let Some(r) = read_rendezvous(&file) else {
            continue;
        };
        if r.project.as_deref() != Some(target.as_str()) {
            if endpoint_gone(&r) {
                let _ = std::fs::remove_file(&file);
            }
            continue;
        }
        if is_live(&r, &file) {
            return Some(r);
        }
        let _ = std::fs::remove_file(&file);
    }
    None
}

/// Lit un rendez-vous de version 1 ; supprime un fichier v1 illisible.
fn read_rendezvous(file: &Path) -> Option<Rendezvous> {
    let meta = std::fs::metadata(file).ok()?;
    if !meta.is_file() || meta.len() > MAX_RENDEZVOUS_BYTES {
        return None;
    }
    let bytes = std::fs::read(file).ok()?;
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        let _ = std::fs::remove_file(file);
        return None;
    };
    if value.get("v").and_then(Value::as_u64) != Some(u64::from(PROTOCOL_VERSION)) {
        // Autre version (ou pas de version) : appartient à une autre
        // application, ne pas y toucher.
        return None;
    }
    if let Ok(r) = serde_json::from_value::<Rendezvous>(value) {
        Some(r)
    } else {
        let _ = std::fs::remove_file(file);
        None
    }
}

#[cfg(unix)]
fn endpoint_gone(r: &Rendezvous) -> bool {
    !Path::new(&r.endpoint).exists()
}

#[cfg(not(unix))]
fn endpoint_gone(_r: &Rendezvous) -> bool {
    false
}

#[cfg(unix)]
fn is_live(r: &Rendezvous, _file: &Path) -> bool {
    probe(r).is_ok()
}

#[cfg(not(unix))]
fn is_live(_r: &Rendezvous, file: &Path) -> bool {
    const MAX_AGE: Duration = Duration::from_hours(24);
    std::fs::metadata(file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|written| written.elapsed().ok())
        .is_some_and(|age| age < MAX_AGE)
}

/// Sonde une application : `hello` avec le jeton du rendez-vous, rend le
/// résultat de l'application (`app`, `version`, `pid`…).
///
/// # Errors
///
/// [`ControlError::Io`] (connexion, délai de 2 s, réponse illisible),
/// [`ControlError::Refused`] (jeton refusé), [`ControlError::Unsupported`]
/// hors Unix.
#[cfg(unix)]
pub fn probe(r: &Rendezvous) -> Result<Value, ControlError> {
    use std::io::BufReader;
    use std::os::unix::net::UnixStream;

    use super::{Frame, Method, Reply, read_line_bounded, to_line};

    const PROBE_ID: u64 = 1;
    let stream =
        UnixStream::connect(&r.endpoint).map_err(|e| ControlError::io("connexion à l'application", &e))?;
    stream
        .set_read_timeout(Some(PROBE_TIMEOUT))
        .and_then(|()| stream.set_write_timeout(Some(PROBE_TIMEOUT)))
        .map_err(|e| ControlError::io("délai du canal", &e))?;
    let hello = Frame {
        v: PROTOCOL_VERSION,
        id: PROBE_ID,
        method: Method::Hello,
        params: serde_json::json!({ "token": r.token }),
    };
    let line = to_line(&hello).map_err(|e| ControlError::Io(e.to_string()))?;
    (&stream)
        .write_all(&line)
        .map_err(|e| ControlError::io("envoi du hello", &e))?;
    let mut reader = BufReader::new(&stream);
    let reply = match read_line_bounded(&mut reader, super::MAX_LINE_BYTES) {
        Ok(Some(line)) => line,
        Ok(None) => return Err(ControlError::Io("connexion fermée".to_owned())),
        Err(super::LineError::TooLong) => {
            return Err(ControlError::Io("réponse trop longue".to_owned()));
        }
        Err(super::LineError::Io(e)) => return Err(ControlError::io("réponse", &e)),
    };
    match serde_json::from_slice::<Reply>(&reply) {
        Ok(Reply::Ok { id, result }) if id == PROBE_ID => Ok(result),
        Ok(Reply::Err { error, .. }) => Err(ControlError::Refused(error)),
        Ok(_) => Err(ControlError::Io("réponse inattendue".to_owned())),
        Err(e) => Err(ControlError::Io(format!("réponse illisible : {e}"))),
    }
}

/// Sonde une application (hors Unix : pas d'écouteur en J4).
///
/// # Errors
///
/// Toujours [`ControlError::Unsupported`].
#[cfg(not(unix))]
pub fn probe(r: &Rendezvous) -> Result<Value, ControlError> {
    let _ = (r, PROBE_TIMEOUT);
    Err(ControlError::Unsupported)
}
