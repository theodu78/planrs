//! Lot L6 (ADR-0012) : fichiers (lecture bornée, écriture atomique,
//! confinement), journal, vues JSON et `Workspace`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use planrs_automation::error::codes;
use planrs_automation::files::{FileError, confine, read_path, write_atomic};
use planrs_automation::journal::{Entry, parse_line, to_line};
use serde_json::json;

/// Dossier temporaire propre à un test (retiré à la fin du test).
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("planrs-ws-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn files_write_atomic_replaces() {
    let dir = TempDir::new("atomic");
    let p = dir.path().join("plan.xml");
    write_atomic(&p, b"ancien").unwrap();
    write_atomic(&p, b"nouveau contenu").unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b"nouveau contenu");
    // Aucun temporaire laissé dans le dossier.
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![std::ffi::OsString::from("plan.xml")]);
    // Dossier absent : erreur propre, rien de créé.
    let missing = dir.path().join("absent").join("plan.xml");
    let err = write_atomic(&missing, b"x").unwrap_err();
    assert_eq!(err.code(), codes::FILE_WRITE);
    assert!(!missing.exists());
    // Lecture bornée.
    assert_eq!(read_path(&p, 1_000).unwrap(), b"nouveau contenu");
    let err = read_path(&p, 3).unwrap_err();
    assert!(matches!(err, FileError::TooLarge { max: 3, .. }));
    assert_eq!(err.code(), codes::FILE_TOO_LARGE);
    assert_eq!(read_path(&missing, 10).unwrap_err().code(), codes::FILE_READ);
    assert_eq!(read_path(dir.path(), 10).unwrap_err().code(), codes::FILE_READ);
}

#[test]
fn files_confine_outside_root_refused() {
    let base = TempDir::new("confine");
    let root = base.path().join("root");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("sub/a.xml"), b"a").unwrap();
    std::fs::write(base.path().join("secret.xml"), b"s").unwrap();
    let canon_root = std::fs::canonicalize(&root).unwrap();

    // Relatif, absolu, fichier à créer : dans la racine.
    assert_eq!(
        confine(&root, std::path::Path::new("sub/a.xml")).unwrap(),
        canon_root.join("sub/a.xml")
    );
    assert_eq!(
        confine(&root, &root.join("sub/a.xml")).unwrap(),
        canon_root.join("sub/a.xml")
    );
    assert_eq!(
        confine(&root, std::path::Path::new("new.xml")).unwrap(),
        canon_root.join("new.xml")
    );
    // Hors de la racine : « .. », absolu ailleurs, dossier absent, nom « .. ».
    for p in [
        PathBuf::from("../secret.xml"),
        PathBuf::from("sub/../../secret.xml"),
        base.path().join("secret.xml"),
        PathBuf::from("/etc/passwd"),
        PathBuf::from("absent/new.xml"),
        PathBuf::from(".."),
    ] {
        let err = confine(&root, &p).unwrap_err();
        assert_eq!(err.code(), codes::FILE_OUTSIDE_ROOT, "{}", p.display());
    }
    // Lien symbolique qui sort de la racine.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(base.path().join("secret.xml"), root.join("lien.xml")).unwrap();
        assert_eq!(
            confine(&root, std::path::Path::new("lien.xml"))
                .unwrap_err()
                .code(),
            codes::FILE_OUTSIDE_ROOT
        );
    }
}

#[test]
fn journal_line_roundtrip() {
    let entries = [
        Entry {
            v: 1,
            seq: 0,
            tool: "add_task".to_owned(),
            params: json!({ "name": "Fondations\n« é »", "duration": { "text": "5j" } }),
            result: None,
        },
        Entry {
            v: 1,
            seq: 42,
            tool: "task.add".to_owned(),
            params: json!({}),
            result: Some(json!({ "committed": true, "created": [7] })),
        },
    ];
    for e in &entries {
        let line = to_line(e);
        assert!(!line.contains('\n'), "une entrée par ligne : {line}");
        assert_eq!(parse_line(&line).unwrap(), *e);
    }
    // `result` absent : non écrit.
    assert!(!to_line(&entries[0]).contains("result"));
    assert!(parse_line("{").is_err());
    assert!(parse_line(r#"{"v":1,"seq":1,"tool":"x","params":{},"extra":1}"#).is_err());
}
