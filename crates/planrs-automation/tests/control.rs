//! Canal de contrôle (lot L10, ADR-0002, ADR-0012 D9) : trames, bornes de
//! ligne, rendez-vous 0600, nettoyage des rendez-vous périmés, jeton,
//! correspondance par chemin canonique, méthodes réservées à J7.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]

use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use planrs_automation::control::{
    Frame, LineError, MAX_LINE_BYTES, Method, Reply, codes, read_line_bounded, to_line,
};
use serde_json::{Value, json};

/// Dossier temporaire propre au test, supprimé à la fin.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        // Chemin court : une adresse de socket Unix tient en ~100 octets.
        let dir = std::env::temp_dir().join(format!("pc-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn control_frame_roundtrip() {
    for (i, method) in Method::ALL.into_iter().enumerate() {
        let frame = Frame {
            v: 1,
            id: u64::try_from(i).unwrap() + 40,
            method,
            params: json!({ "token": "x", "n": [1, 2] }),
        };
        let line = to_line(&frame).unwrap();
        assert_eq!(line.last(), Some(&b'\n'));
        let text = std::str::from_utf8(&line).unwrap();
        assert!(
            text.contains(&format!("\"method\":\"{}\"", method.wire_name())),
            "{text}"
        );
        let mut reader = Cursor::new(line.clone());
        let read = read_line_bounded(&mut reader, MAX_LINE_BYTES).unwrap().unwrap();
        let back: Frame = serde_json::from_slice(&read).unwrap();
        assert_eq!(back, frame);
    }
    assert_eq!(
        Method::ALL.map(Method::wire_name),
        [
            "hello",
            "query",
            "execute",
            "undo",
            "redo",
            "history",
            "ui_state",
            "screenshot"
        ]
    );

    // Forme exacte sur le fil ; `params` absent vaut `null`.
    let frame: Frame = serde_json::from_str(r#"{"v":1,"id":3,"method":"hello"}"#).unwrap();
    assert_eq!(frame.params, Value::Null);
    assert_eq!(frame.method, Method::Hello);
    // Champ inconnu ou méthode inconnue : refus.
    assert!(serde_json::from_str::<Frame>(r#"{"v":1,"id":3,"method":"hello","x":1}"#).is_err());
    assert!(serde_json::from_str::<Frame>(r#"{"v":1,"id":3,"method":"open"}"#).is_err());

    let replies = [
        Reply::Ok {
            id: 1,
            result: json!({ "a": 1 }),
        },
        Reply::error(2, codes::BAD_TOKEN, "jeton refusé"),
        Reply::PendingApproval { id: 3 },
    ];
    let expected = [
        r#"{"ok":{"id":1,"result":{"a":1}}}"#,
        r#"{"err":{"id":2,"error":{"code":"control.bad_token","message":"jeton refusé"}}}"#,
        r#"{"pending_approval":{"id":3}}"#,
    ];
    for ((reply, wire), id) in replies.iter().zip(expected).zip(1..) {
        assert_eq!(serde_json::to_string(reply).unwrap(), wire);
        assert_eq!(serde_json::from_str::<Reply>(wire).unwrap(), *reply);
        assert_eq!(reply.id(), id);
    }
}

#[test]
fn control_line_too_long_refused() {
    // Lecteur borné : exactement la borne passe, un octet de plus non.
    let max = 16;
    let mut ok = Cursor::new(b"0123456789abcdef\nsuite\r\nfin".to_vec());
    assert_eq!(
        read_line_bounded(&mut ok, max).unwrap().unwrap(),
        b"0123456789abcdef"
    );
    assert_eq!(read_line_bounded(&mut ok, max).unwrap().unwrap(), b"suite");
    assert_eq!(read_line_bounded(&mut ok, max).unwrap().unwrap(), b"fin");
    assert!(read_line_bounded(&mut ok, max).unwrap().is_none());

    let mut long = Cursor::new(b"0123456789abcdefg\n".to_vec());
    assert!(matches!(
        read_line_bounded(&mut long, max),
        Err(LineError::TooLong)
    ));
    // Sans fin de ligne, la borne s'applique aussi (petit tampon : la ligne
    // arrive en plusieurs morceaux).
    let mut chunked = BufReader::with_capacity(3, Cursor::new(vec![b'a'; 40]));
    assert!(matches!(
        read_line_bounded(&mut chunked, max),
        Err(LineError::TooLong)
    ));
    let mut chunked = BufReader::with_capacity(3, Cursor::new(vec![b'a'; 16]));
    assert_eq!(read_line_bounded(&mut chunked, max).unwrap().unwrap().len(), 16);
    assert_eq!(MAX_LINE_BYTES, 8 * 1024 * 1024);

    #[cfg(unix)]
    unix::line_too_long_over_socket();
}

#[cfg(unix)]
#[test]
fn control_rendezvous_mode_0600() {
    unix::rendezvous_mode_0600();
}

#[cfg(unix)]
#[test]
fn control_stale_rendezvous_removed() {
    unix::stale_rendezvous_removed();
}

#[cfg(unix)]
#[test]
fn control_hello_wrong_token_refused() {
    unix::hello_wrong_token_refused();
}

#[cfg(unix)]
#[test]
fn control_find_live_matches_canonical_path() {
    unix::find_live_matches_canonical_path();
}

#[cfg(unix)]
#[test]
fn control_unsupported_methods_in_j4() {
    unix::unsupported_methods_in_j4();
}

/// Hors Unix (Windows, J4) : pas d'écouteur ; un rendez-vous est vivant
/// s'il a été écrit il y a moins de 24 h (hypothèse documentée).
#[cfg(not(unix))]
#[test]
fn control_find_live_by_age_without_listener() {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime};

    use planrs_automation::control::{
        ControlError, Rendezvous, canonical_project, find_live_for_in, new_token, publish_in,
        spawn_hello_listener_in,
    };

    let tmp = TempDir::new("age");
    let dir = tmp.path().join("control");
    let file = tmp.path().join("plan.xml");
    std::fs::write(&file, b"<Project/>").unwrap();
    assert_eq!(
        spawn_hello_listener_in(&dir, new_token().unwrap(), Arc::new(Mutex::new(None))).unwrap_err(),
        ControlError::Unsupported
    );
    let r = Rendezvous {
        v: 1,
        pid: std::process::id(),
        endpoint: String::new(),
        token: new_token().unwrap(),
        project: canonical_project(&file),
    };
    let guard = publish_in(&dir, &r).unwrap();
    assert_eq!(find_live_for_in(&dir, &file), Some(r));
    let old = SystemTime::now() - Duration::from_hours(25);
    std::fs::File::options()
        .write(true)
        .open(guard.path())
        .unwrap()
        .set_modified(old)
        .unwrap();
    assert!(find_live_for_in(&dir, &file).is_none());
    assert!(!guard.path().exists());
}

#[cfg(unix)]
mod unix {
    use std::io::{BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixStream;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use planrs_automation::control::{
        ControlError, ListenerHandle, Rendezvous, WireError, canonical_project, find_live_for_in, new_token,
        probe, publish_in, spawn_hello_listener_in,
    };

    use super::*;

    fn listener(dir: &Path) -> (ListenerHandle, String) {
        let token = new_token().unwrap();
        let handle = spawn_hello_listener_in(dir, token.clone(), Arc::new(Mutex::new(None))).unwrap();
        (handle, token)
    }

    /// Connexion de test : envoie des trames, lit les réponses.
    struct Client {
        stream: UnixStream,
        reader: BufReader<UnixStream>,
    }

    impl Client {
        fn connect(endpoint: &str) -> Self {
            let stream = UnixStream::connect(endpoint).unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
            let reader = BufReader::new(stream.try_clone().unwrap());
            Self { stream, reader }
        }

        fn send_raw(&mut self, bytes: &[u8]) {
            self.stream.write_all(bytes).unwrap();
        }

        fn call(&mut self, id: u64, method: Method, params: Value) -> Reply {
            let frame = Frame {
                v: 1,
                id,
                method,
                params,
            };
            self.send_raw(&to_line(&frame).unwrap());
            self.reply()
        }

        fn reply(&mut self) -> Reply {
            let line = read_line_bounded(&mut self.reader, MAX_LINE_BYTES)
                .unwrap()
                .expect("réponse attendue");
            serde_json::from_slice(&line).unwrap()
        }

        /// La connexion a été fermée par l'application.
        fn is_closed(&mut self) -> bool {
            matches!(read_line_bounded(&mut self.reader, MAX_LINE_BYTES), Ok(None))
        }

        fn hello(&mut self, token: &str) -> Reply {
            self.call(1, Method::Hello, json!({ "token": token }))
        }
    }

    fn err_code(reply: &Reply) -> &str {
        match reply {
            Reply::Err {
                error: WireError { code, .. },
                ..
            } => code,
            other => panic!("erreur attendue : {other:?}"),
        }
    }

    fn project_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"<Project/>").unwrap();
        path
    }

    fn json_files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| Path::new(n).extension().is_some_and(|e| e == "json"))
            .collect();
        names.sort();
        names
    }

    pub(super) fn line_too_long_over_socket() {
        let tmp = TempDir::new("long");
        let (handle, token) = listener(tmp.path());
        let mut client = Client::connect(handle.endpoint());
        assert!(matches!(client.hello(&token), Reply::Ok { .. }));
        // Écriture dans un fil : l'application coupe avant la fin.
        let mut writer = client.stream.try_clone().unwrap();
        let sender = std::thread::spawn(move || {
            let mut line = vec![b' '; MAX_LINE_BYTES + 1];
            line.push(b'\n');
            let _ = writer.write_all(&line);
        });
        let reply = client.reply();
        assert_eq!(err_code(&reply), codes::LINE_TOO_LONG);
        assert!(client.is_closed());
        sender.join().unwrap();
    }

    pub(super) fn rendezvous_mode_0600() {
        let tmp = TempDir::new("mode");
        // Dossier existant trop ouvert : resserré à 0700.
        let dir = tmp.path().join("state").join("control");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let r = Rendezvous {
            v: 1,
            pid: std::process::id(),
            endpoint: "/nulle/part.sock".to_owned(),
            token: new_token().unwrap(),
            project: Some("/p/a.xml".to_owned()),
        };
        let guard = publish_in(&dir, &r).unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(guard.path()), 0o600);
        assert_eq!(mode(&dir), 0o700);
        let back: Rendezvous = serde_json::from_slice(&std::fs::read(guard.path()).unwrap()).unwrap();
        assert_eq!(back, r);
        // Aucun fichier temporaire ne reste.
        let names: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert_eq!(names.len(), 1);
        // Le jeton n'apparaît pas dans la trace.
        assert!(!format!("{r:?}").contains(&r.token));

        // Dossier créé de zéro, parents compris, en 0700.
        let fresh = tmp.path().join("neuf").join("control");
        let guard2 = publish_in(&fresh, &r).unwrap();
        assert_eq!(mode(&fresh), 0o700);
        assert_eq!(mode(guard2.path()), 0o600);
        assert_ne!(guard.path(), guard2.path());

        // Suppression à la destruction.
        let path = guard.path().to_path_buf();
        drop(guard);
        assert!(!path.exists());

        // Refus : jeton mal formé, version inconnue.
        let bad = Rendezvous {
            token: "abc".to_owned(),
            ..r.clone()
        };
        assert_eq!(publish_in(&dir, &bad).unwrap_err(), ControlError::InvalidToken);
        let mut v2 = r;
        v2.v = 2;
        assert!(matches!(publish_in(&dir, &v2), Err(ControlError::Invalid(_))));
        assert_eq!(json_files(&dir), Vec::<String>::new());
    }

    pub(super) fn stale_rendezvous_removed() {
        let tmp = TempDir::new("stale");
        let dir = tmp.path().join("control");
        let file = project_file(tmp.path(), "plan.xml");
        let project = canonical_project(&file);
        assert!(project.is_some());

        // 1. Socket absente (application morte).
        let dead = Rendezvous {
            v: 1,
            pid: 999_999,
            endpoint: dir.join("mort.sock").to_str().unwrap().to_owned(),
            token: new_token().unwrap(),
            project: project.clone(),
        };
        std::mem::forget(publish_in(&dir, &dead).unwrap());
        assert_eq!(json_files(&dir).len(), 1);
        assert!(find_live_for_in(&dir, &file).is_none());
        assert_eq!(json_files(&dir), Vec::<String>::new());

        // 2. Écouteur vivant mais jeton du rendez-vous faux.
        let (handle, _token) = listener(&dir);
        let mut wrong = handle.rendezvous(project.clone());
        wrong.token = new_token().unwrap();
        std::mem::forget(publish_in(&dir, &wrong).unwrap());
        assert!(find_live_for_in(&dir, &file).is_none());
        assert_eq!(json_files(&dir), Vec::<String>::new());

        // 3. Écouteur arrêté après publication : le rendez-vous périme.
        let guard = publish_in(&dir, &handle.rendezvous(project.clone())).unwrap();
        assert!(find_live_for_in(&dir, &file).is_some());
        let endpoint = handle.endpoint().to_owned();
        drop(handle);
        assert!(!Path::new(&endpoint).exists(), "socket supprimée");
        std::mem::forget(guard);
        assert!(find_live_for_in(&dir, &file).is_none());
        assert_eq!(json_files(&dir), Vec::<String>::new());

        // 4. Fichier v1 illisible : supprimé ; autre version : conservé.
        std::fs::write(dir.join("1-0.json"), b"{pas du json").unwrap();
        std::fs::write(dir.join("2-0.json"), br#"{"v":1,"pid":"x"}"#).unwrap();
        std::fs::write(dir.join("3-0.json"), br#"{"v":2,"futur":true}"#).unwrap();
        std::fs::write(dir.join("4-0.tmp"), b"en cours").unwrap();
        assert!(find_live_for_in(&dir, &file).is_none());
        assert_eq!(json_files(&dir), ["3-0.json"]);
        assert!(dir.join("4-0.tmp").exists());

        // 5. Rendez-vous d'un autre projet dont la socket a disparu :
        // nettoyé au passage.
        let mut other = dead;
        other.project = Some("/ailleurs/autre.xml".to_owned());
        std::mem::forget(publish_in(&dir, &other).unwrap());
        assert!(find_live_for_in(&dir, &file).is_none());
        assert_eq!(json_files(&dir), ["3-0.json"]);
    }

    pub(super) fn hello_wrong_token_refused() {
        let tmp = TempDir::new("tok");
        let (handle, token) = listener(tmp.path());

        // Jeton faux : refus puis fermeture.
        let mut client = Client::connect(handle.endpoint());
        let reply = client.hello(&new_token().unwrap());
        assert_eq!(err_code(&reply), codes::BAD_TOKEN);
        assert_eq!(reply.id(), 1);
        assert!(client.is_closed());

        // Jeton mal formé ou absent.
        for params in [json!({ "token": "zz" }), json!({}), Value::Null] {
            let mut client = Client::connect(handle.endpoint());
            let reply = client.call(5, Method::Hello, params);
            assert_eq!(err_code(&reply), codes::BAD_TOKEN);
            assert!(client.is_closed());
        }

        // Requête avant `hello` : refus puis fermeture.
        let mut client = Client::connect(handle.endpoint());
        let reply = client.call(2, Method::Query, Value::Null);
        assert_eq!(err_code(&reply), codes::UNAUTHENTICATED);
        assert!(client.is_closed());

        // Sonde avec un jeton faux.
        let mut r = handle.rendezvous(None);
        r.token = new_token().unwrap();
        match probe(&r) {
            Err(ControlError::Refused(e)) => assert_eq!(e.code, codes::BAD_TOKEN),
            other => panic!("refus attendu : {other:?}"),
        }

        // Le bon jeton passe, en majuscules aussi.
        let mut client = Client::connect(handle.endpoint());
        assert!(matches!(client.hello(&token.to_uppercase()), Reply::Ok { .. }));
        let ok = probe(&handle.rendezvous(None)).unwrap();
        assert_eq!(ok["app"], "planrs");
        assert_eq!(ok["pid"], std::process::id());

        // Écouteur refusé pour un jeton mal formé.
        assert_eq!(
            spawn_hello_listener_in(tmp.path(), "abc".to_owned(), Arc::new(Mutex::new(None))).unwrap_err(),
            ControlError::InvalidToken
        );
    }

    pub(super) fn find_live_matches_canonical_path() {
        let tmp = TempDir::new("find");
        let dir = tmp.path().join("control");
        std::fs::create_dir_all(tmp.path().join("sub")).unwrap();
        let file = project_file(tmp.path(), "plan.xml");
        let other = project_file(tmp.path(), "autre.xml");
        let (handle, _token) = listener(&dir);
        let r = handle.rendezvous(canonical_project(&file));
        let _guard = publish_in(&dir, &r).unwrap();

        // Chemin non canonique (`sub/..`, lien symbolique) : même fichier.
        let detour = tmp.path().join("sub").join("..").join("plan.xml");
        assert_eq!(find_live_for_in(&dir, &detour), Some(r.clone()));
        let link = tmp.path().join("lien.xml");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert_eq!(find_live_for_in(&dir, &link), Some(r.clone()));
        assert_eq!(find_live_for_in(&dir, &file), Some(r));
        // Autre fichier, fichier absent, dossier absent : rien.
        assert!(find_live_for_in(&dir, &other).is_none());
        assert!(find_live_for_in(&dir, &tmp.path().join("absent.xml")).is_none());
        assert!(find_live_for_in(&tmp.path().join("rien"), &file).is_none());
        // Le rendez-vous vivant est conservé.
        assert_eq!(json_files(&dir).len(), 1);
    }

    pub(super) fn unsupported_methods_in_j4() {
        let tmp = TempDir::new("meth");
        let (handle, token) = listener(tmp.path());
        let mut client = Client::connect(handle.endpoint());
        match client.hello(&token) {
            Reply::Ok { id, result } => {
                assert_eq!(id, 1);
                assert_eq!(result["v"], 1);
                assert_eq!(result["methods"], json!(["hello", "query"]));
            }
            other => panic!("hello attendu : {other:?}"),
        }
        // Méthodes réservées : refus, la connexion reste ouverte.
        let reserved = [
            Method::Execute,
            Method::Undo,
            Method::Redo,
            Method::History,
            Method::UiState,
            Method::Screenshot,
        ];
        for (method, id) in reserved.into_iter().zip(10..) {
            assert!(!method.available_in_j4());
            let reply = client.call(id, method, json!({}));
            assert_eq!(err_code(&reply), codes::UNSUPPORTED, "{method:?}");
            assert_eq!(reply.id(), id);
        }
        // `query` : résumé (aucun document ouvert ⇒ null).
        for params in [Value::Null, json!({}), json!({ "command": "project.summary" })] {
            assert_eq!(
                client.call(20, Method::Query, params),
                Reply::Ok {
                    id: 20,
                    result: Value::Null
                }
            );
        }
        let reply = client.call(21, Method::Query, json!({ "command": "task.list" }));
        assert_eq!(err_code(&reply), codes::UNSUPPORTED);
        let reply = client.call(22, Method::Query, json!([1]));
        assert_eq!(err_code(&reply), codes::INVALID_FRAME);
        // Version inconnue, trame illisible : refus, identifiant repris.
        client.send_raw(b"{\"v\":2,\"id\":23,\"method\":\"query\",\"params\":null}\n");
        let reply = client.reply();
        assert_eq!(err_code(&reply), codes::UNSUPPORTED_VERSION);
        assert_eq!(reply.id(), 23);
        client.send_raw(b"{\"v\":1,\"id\":24,\"method\":\"open\"}\n");
        let reply = client.reply();
        assert_eq!(err_code(&reply), codes::INVALID_FRAME);
        assert_eq!(reply.id(), 24);
        client.send_raw(b"pas du json\n");
        assert_eq!(err_code(&client.reply()), codes::INVALID_FRAME);
        // Toujours ouverte après ces refus.
        assert!(matches!(
            client.call(25, Method::Query, Value::Null),
            Reply::Ok { id: 25, .. }
        ));
    }
}
