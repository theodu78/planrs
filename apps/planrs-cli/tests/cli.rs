//! Tests du binaire `planrs` (lot L9) : lancé par `CARGO_BIN_EXE_planrs`,
//! environnement maîtrisé (`PLANRS_NOW`, `LANG`), fichiers dans un dossier
//! temporaire propre à chaque test. Codes de sortie : C-MCP-08.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use planrs_automation::Workspace;
use planrs_automation::text::Language;
use planrs_core::Moment;
use serde_json::{Value, json};

const NOW: &str = "2026-10-19T08:00";

/// Dossier temporaire propre à un test (retiré à la fin du test).
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("planrs-cli-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn house_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/planrs-ui-egui/assets/demo/house.xml")
}

/// Copie de `house.xml` dans `dir`.
fn house_copy(dir: &TempDir) -> PathBuf {
    let p = dir.join("maison.xml");
    std::fs::copy(house_path(), &p).unwrap();
    p
}

/// Commande `planrs` à environnement maîtrisé (français, sans
/// `PLANRS_NOW`).
fn planrs() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_planrs"));
    c.env_remove("PLANRS_NOW").env("LANG", "fr_FR.UTF-8");
    c
}

fn run(args: &[&str]) -> Output {
    planrs().args(args).output().unwrap()
}

fn code(o: &Output) -> i32 {
    o.status.code().expect("code de sortie")
}

fn stdout_json(o: &Output) -> Value {
    serde_json::from_slice(&o.stdout)
        .unwrap_or_else(|e| panic!("JSON attendu ({e}) : {}", String::from_utf8_lossy(&o.stdout)))
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn now() -> Moment {
    NOW.parse().unwrap()
}

/// Espace en mémoire (ce que sert le MCP) avec `path` ouvert.
fn workspace_with(path: &Path, lang: Language) -> Workspace {
    let mut ws = Workspace::new(now(), lang);
    ws.call("project.open", json!({ "path": s(path) })).unwrap();
    ws
}

const ADD_TASK: &str = r#"{"format_version":1,"lang":"fr","ops":[{"op":"task.add","name":"Peinture des volets","duration":{"text":"3j"}}]}"#;

#[test]
fn cli_summary_house_json_equals_mcp_view() {
    let house = house_path();
    let o = run(&["--now", NOW, "summary", s(&house), "--json"]);
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
    let cli = stdout_json(&o);
    let mut ws = workspace_with(&house, Language::Fr);
    let view = ws.call("project.summary", json!({})).unwrap();
    assert_eq!(cli, view);
    assert_eq!(cli["tasks"], json!(49));
    // Sortie texte : mêmes dates, issues du moteur.
    let o = run(&["--now", NOW, "summary", s(&house)]);
    assert_eq!(code(&o), 0);
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("Construction d'une maison"), "{text}");
    assert!(text.contains(view["finish"].as_str().unwrap()), "{text}");
    // `tasks --json` : la vue `ListView` (toutes les pages réunies).
    let o = run(&["--now", NOW, "tasks", s(&house), "--json", "--critical"]);
    assert_eq!(code(&o), 0);
    let list = stdout_json(&o);
    let mcp = ws
        .call("task.list", json!({ "critical": true, "limit": 500 }))
        .unwrap();
    assert_eq!(list["tasks"], mcp["tasks"]);
    assert_eq!(list["total"], mcp["total"]);
    // `--limit` : une page, telle que le MCP la rend.
    let o = run(&["--now", NOW, "tasks", s(&house), "--json", "--limit", "5"]);
    assert_eq!(code(&o), 0);
    let page = ws.call("task.list", json!({ "limit": 5 })).unwrap();
    assert_eq!(stdout_json(&o), page);
    // Texte : une ligne par tâche, plus en-tête et pied.
    let o = run(&["--now", NOW, "tasks", s(&house)]);
    assert_eq!(code(&o), 0);
    let text = String::from_utf8(o.stdout).unwrap();
    assert_eq!(text.lines().count(), 49 + 2, "{text}");
}

#[test]
fn cli_apply_dry_run_leaves_file() {
    let dir = TempDir::new("dry");
    let plan = house_copy(&dir);
    let ops = dir.join("ops.json");
    std::fs::write(&ops, ADD_TASK).unwrap();
    let before = std::fs::read(&plan).unwrap();
    let out = dir.join("sortie.xml");
    let o = run(&[
        "--now",
        NOW,
        "apply",
        s(&plan),
        s(&ops),
        "--dry-run",
        "-o",
        s(&out),
        "--json",
    ]);
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
    let v = stdout_json(&o);
    assert_eq!(v["committed"], json!(false));
    assert_eq!(v["created_uids"].as_array().unwrap().len(), 1);
    assert_eq!(std::fs::read(&plan).unwrap(), before);
    assert!(!out.exists(), "--dry-run n'écrit rien, même avec -o");
}

#[test]
fn cli_apply_writes_atomic() {
    let dir = TempDir::new("write");
    let plan = house_copy(&dir);
    let ops = dir.join("ops.json");
    std::fs::write(&ops, ADD_TASK).unwrap();
    let original = std::fs::read(&plan).unwrap();

    // `-o` : le plan reste intact, la sortie porte la tâche.
    let out = dir.join("sortie.xml");
    let o = run(&["--now", NOW, "apply", s(&plan), s(&ops), "-o", s(&out), "--json"]);
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
    let outcome = stdout_json(&o);
    assert_eq!(outcome["committed"], json!(true));
    assert_eq!(std::fs::read(&plan).unwrap(), original);
    let mut ws = workspace_with(&out, Language::Fr);
    let summary = ws.call("project.summary", json!({})).unwrap();
    assert_eq!(summary["tasks"], json!(50));

    // Sans `-o` : le plan est remplacé, rien d'autre ne reste dans le
    // dossier (pas de temporaire).
    let o = run(&["--now", NOW, "apply", s(&plan), s(&ops)]);
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("Lot appliqué"), "{text}");
    assert!(text.contains(s(&plan)), "{text}");
    assert_ne!(std::fs::read(&plan).unwrap(), original);
    let mut ws = workspace_with(&plan, Language::Fr);
    let list = ws.call("task.list", json!({ "limit": 500 })).unwrap();
    let names: Vec<&str> = list["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Peinture des volets"), "{names:?}");
    let mut entries: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(entries, ["maison.xml", "ops.json", "sortie.xml"]);
}

#[test]
fn cli_apply_refused_exit_1() {
    let dir = TempDir::new("refused");
    let plan = house_copy(&dir);
    let before = std::fs::read(&plan).unwrap();
    let ops = dir.join("ops.json");
    std::fs::write(
        &ops,
        r#"{"format_version":1,"ops":[{"op":"task.update","uid":99999,"name":"X"}]}"#,
    )
    .unwrap();
    let o = run(&["--now", NOW, "apply", s(&plan), s(&ops), "--json"]);
    assert_eq!(code(&o), 1);
    let v = stdout_json(&o);
    assert_eq!(v["error"]["code"], json!("task.not_found"));
    assert_eq!(v["error"]["op_index"], json!(0));
    assert!(String::from_utf8_lossy(&o.stderr).contains("task.not_found"));
    assert_eq!(std::fs::read(&plan).unwrap(), before);
    // Lot JSON mal formé : refus métier (`param.invalid`), même code.
    std::fs::write(&ops, "{ pas du JSON").unwrap();
    let o = run(&["--now", NOW, "apply", s(&plan), s(&ops)]);
    assert_eq!(code(&o), 1);
    assert!(String::from_utf8_lossy(&o.stderr).contains("param.invalid"));
    assert_eq!(std::fs::read(&plan).unwrap(), before);
}

#[test]
fn cli_missing_file_exit_3() {
    let dir = TempDir::new("missing");
    let absent = dir.join("absent.xml");
    for args in [
        vec!["summary", s(&absent)],
        vec!["tasks", s(&absent), "--json"],
        vec!["render", s(&absent), "-o", "x.svg"],
    ] {
        let o = planrs().arg("--now").arg(NOW).args(&args).output().unwrap();
        assert_eq!(code(&o), 3, "{args:?}");
        assert!(
            String::from_utf8_lossy(&o.stderr).contains("file.read"),
            "{args:?}"
        );
    }
    // MSPDI illisible.
    let bad = dir.join("mauvais.xml");
    std::fs::write(&bad, "<Project><Tasks>").unwrap();
    let o = run(&["--now", NOW, "summary", s(&bad)]);
    assert_eq!(code(&o), 3);
    assert!(String::from_utf8_lossy(&o.stderr).contains("file.invalid"));
    // Lot absent.
    let plan = house_copy(&dir);
    let o = run(&["--now", NOW, "apply", s(&plan), s(&dir.join("ops.json"))]);
    assert_eq!(code(&o), 3);
    // Écriture impossible (dossier inexistant).
    let o = run(&[
        "--now",
        NOW,
        "render",
        s(&plan),
        "-o",
        s(&dir.join("nul/part/x.svg")),
    ]);
    assert_eq!(code(&o), 3);
    assert!(String::from_utf8_lossy(&o.stderr).contains("file.write"));
}

#[test]
#[allow(clippy::many_single_char_names, reason = "tests : a.svg, b.svg, c.svg")]
fn cli_render_svg_bytes_stable() {
    let dir = TempDir::new("render");
    let house = house_path();
    let a = dir.join("a.svg");
    let b = dir.join("b.svg");
    for out in [&a, &b] {
        let o = run(&["--now", NOW, "render", s(&house), "-o", s(out), "--zoom", "week"]);
        assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
    }
    let bytes = std::fs::read(&a).unwrap();
    assert_eq!(bytes, std::fs::read(&b).unwrap());
    assert!(
        bytes.starts_with(b"<svg"),
        "{}",
        String::from_utf8_lossy(&bytes[..64])
    );
    // Même rendu que `render.gantt` du MCP.
    let mut ws = workspace_with(&house, Language::Fr);
    let v = ws.call("render.gantt", json!({ "zoom": "week" })).unwrap();
    assert_eq!(v["svg"].as_str().unwrap().as_bytes(), bytes.as_slice());
    // Filtre et plage.
    let c = dir.join("c.svg");
    let o = run(&[
        "--now",
        NOW,
        "render",
        s(&house),
        "-o",
        s(&c),
        "--critical",
        "--from",
        "2026-11-02",
        "--to",
        "2027-01-29",
    ]);
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
    let v = ws
        .call(
            "render.gantt",
            json!({ "critical_only": true, "from": "2026-11-02", "to": "2027-01-29" }),
        )
        .unwrap();
    assert_eq!(v["svg"].as_str().unwrap().as_bytes(), std::fs::read(&c).unwrap());
    // Jour mal formé : usage.
    let o = run(&["render", s(&house), "-o", s(&c), "--from", "02/11/2026"]);
    assert_eq!(code(&o), 2);
}

/// Début du projet écrit par `import-csv` avec cet environnement.
fn imported_start(dir: &TempDir, env_now: Option<&str>, flag: Option<&str>) -> (i32, Option<String>) {
    let csv = dir.join("taches.csv");
    std::fs::write(&csv, "Nom;Durée\nFondations;5j\n").unwrap();
    let out = dir.join("taches.xml");
    let _ = std::fs::remove_file(&out);
    let mut c = planrs();
    if let Some(e) = env_now {
        c.env("PLANRS_NOW", e);
    }
    if let Some(f) = flag {
        c.args(["--now", f]);
    }
    let o = c.args(["import-csv", s(&csv), "-o", s(&out)]).output().unwrap();
    if code(&o) != 0 {
        return (code(&o), None);
    }
    let mut ws = workspace_with(&out, Language::Fr);
    let v = ws.call("project.summary", json!({})).unwrap();
    (0, Some(v["start"].as_str().unwrap().to_owned()))
}

#[test]
fn cli_now_env_and_flag_precedence() {
    let dir = TempDir::new("now");
    // Variable seule.
    let got = imported_start(&dir, Some("2026-11-02T08:00"), None);
    assert_eq!(got, (0, Some("2026-11-02T08:00".to_owned())));
    // L'option l'emporte sur la variable.
    let got = imported_start(&dir, Some("2026-11-02T08:00"), Some("2026-12-01T08:00"));
    assert_eq!(got, (0, Some("2026-12-01T08:00".to_owned())));
    // Option seule.
    let got = imported_start(&dir, None, Some("2027-02-01T08:00"));
    assert_eq!(got, (0, Some("2027-02-01T08:00".to_owned())));
    // Horloge locale à défaut : une date est écrite.
    let (c, start) = imported_start(&dir, None, None);
    assert_eq!(c, 0);
    assert!(start.is_some());
    // Valeurs invalides : usage (2).
    assert_eq!(imported_start(&dir, Some("demain"), None).0, 2);
    assert_eq!(imported_start(&dir, None, Some("2026-02-30T08:00")).0, 2);
    // Une variable invalide est ignorée si l'option est donnée.
    let got = imported_start(&dir, Some("demain"), Some("2026-11-02T08:00"));
    assert_eq!(got.0, 0);
}

#[test]
fn cli_explain_hidden_exit_2() {
    let house = house_path();
    let o = run(&["explain", s(&house), "--task", "3"]);
    assert_eq!(code(&o), 2);
    assert!(String::from_utf8_lossy(&o.stderr).contains("J6"));
    let o = run(&["--help"]);
    assert_eq!(code(&o), 0);
    let help = String::from_utf8(o.stdout).unwrap();
    assert!(!help.contains("explain"), "{help}");
    assert!(help.contains("summary") && help.contains("import-csv"), "{help}");
    // Usage : sous-commande inconnue, argument manquant.
    assert_eq!(code(&run(&["inconnue"])), 2);
    assert_eq!(code(&run(&["summary"])), 2);
    assert_eq!(code(&run(&["mcp", "--http", ":8080"])), 2);
}

#[test]
fn cli_import_csv_roundtrip() {
    let dir = TempDir::new("csv");
    let csv = dir.join("taches.csv");
    std::fs::write(
        &csv,
        "Nom;Durée;Prédécesseurs\nFondations;5j;\nMurs;10j;1\nToit;3j;2\n",
    )
    .unwrap();
    let out = dir.join("taches.xml");
    let o = run(&["--now", NOW, "import-csv", s(&csv), "-o", s(&out)]);
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("3 tâche(s), 2 lien(s)"), "{text}");
    // Relu : mêmes tâches que l'import en mémoire (ce que fait le MCP).
    let mut ws = workspace_with(&out, Language::Fr);
    let reread = ws.call("task.list", json!({})).unwrap();
    let mut direct = Workspace::new(now(), Language::Fr);
    direct
        .call("project.import_csv", json!({ "path": s(&csv) }))
        .unwrap();
    let expected = direct.call("task.list", json!({})).unwrap();
    assert_eq!(reread["tasks"], expected["tasks"]);
    assert_eq!(reread["total"], json!(3));
    // CSV vide : erreur de fichier (3).
    let empty = dir.join("vide.csv");
    std::fs::write(&empty, "").unwrap();
    let o = run(&["--now", NOW, "import-csv", s(&empty), "-o", s(&out)]);
    assert_eq!(code(&o), 3, "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn cli_lang_flag_and_env() {
    let house = house_path();
    let o = run(&["--now", NOW, "--lang", "en", "summary", s(&house)]);
    assert_eq!(code(&o), 0);
    assert!(String::from_utf8(o.stdout).unwrap().contains("Finish"));
    let o = planrs()
        .env("LANG", "en_US.UTF-8")
        .args(["--now", NOW, "summary", s(&house)])
        .output()
        .unwrap();
    assert!(String::from_utf8(o.stdout).unwrap().contains("Finish"));
    let o = run(&["--now", NOW, "summary", s(&house)]);
    assert!(String::from_utf8(o.stdout).unwrap().contains("Fin"));
}
