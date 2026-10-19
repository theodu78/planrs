//! Rejeu du scénario `build_30` en mémoire (lot L18, critère de fin J4) :
//! « un agent construit un planning de 30 tâches via MCP », sans LLM.
//!
//! Le scénario (`tests/mcp/scenarios/build_30.jsonl`) est rejoué sur
//! `serve_on` + `tokio::io::duplex`, `--now 2026-10-19T08:00`, langue `fr`.
//! Vérifications du cahier (`docs/plans/j4.md`, L18) : 1 (résultats égaux
//! aux attendus), 2 (MSPDI enregistré), 3 (undo jusqu'au projet vide),
//! 5 (budget < 2 s). La 4 (vrai binaire en stdio) est dans
//! `apps/planrs-cli/tests/mcp_replay.rs`, la 6 (HTTP) dans
//! `tests/replay_http.rs`. `PLANRS_BLESS=1` régénère les attendus (à relire).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::missing_panics_doc,
    reason = "tests : un échec doit s'arrêter net"
)]

#[path = "common/replay.rs"]
mod replay;

use std::path::Path;
use std::time::{Duration, Instant};

use planrs_automation::text::Language;
use planrs_core::{Moment, Severity};
use planrs_mcp::{McpServeError, ServeOptions, serve_on};
use replay::{NOW, Scenario, TempDir, logged, normalized, repo_root, substitute};
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::{RoleClient, ServiceExt};
use serde_json::{Value, json};
use tokio::task::JoinHandle;

const SCENARIO: &str = "build_30";

/// Budget du rejeu complet en debug (vérification 5).
const BUDGET: Duration = Duration::from_secs(2);

fn root() -> std::path::PathBuf {
    repo_root(env!("CARGO_MANIFEST_DIR"))
}

fn now() -> Moment {
    NOW.parse().unwrap()
}

fn opts(journal: Option<&Path>) -> ServeOptions {
    let mut o = ServeOptions::new(now(), Language::Fr);
    o.ignore_app_guard = true;
    o.journal = journal.map(Path::to_path_buf);
    o
}

type Client = RunningService<RoleClient, ()>;

async fn connect(o: ServeOptions) -> (Client, JoinHandle<Result<(), McpServeError>>) {
    let (client_io, server_io) = tokio::io::duplex(1 << 22);
    let (r, w) = tokio::io::split(server_io);
    let server = tokio::spawn(serve_on(r, w, o));
    let client = ().serve(client_io).await.unwrap();
    (client, server)
}

/// Appel d'outil ; résultat sous la forme du journal.
async fn call(client: &Client, tool: &str, params: &Value) -> Value {
    let mut req = CallToolRequestParams::new(tool.to_owned());
    if let Value::Object(args) = params {
        req = req.with_arguments(args.clone());
    }
    let r = client.call_tool(req).await.unwrap();
    logged(r.is_error, r.structured_content)
}

/// Rejoue tous les appels ; résultats ramenés au jeton `$TMP`.
async fn run(client: &Client, scenario: &Scenario, tmp: &Path) -> Vec<Value> {
    let mut got = Vec::with_capacity(scenario.calls.len());
    for c in &scenario.calls {
        let r = call(client, &c.tool, &substitute(&c.params, tmp)).await;
        got.push(normalized(&r, tmp));
    }
    got
}

/// Rejeu complet avec journal ; rend le journal produit et la durée.
async fn replay_with_journal(scenario: &Scenario, tmp: &TempDir) -> (Vec<Value>, String, Duration) {
    let journal = tmp.path().join("run.jsonl");
    let (client, server) = connect(opts(Some(&journal))).await;
    let t0 = Instant::now();
    let got = run(&client, scenario, tmp.path()).await;
    let elapsed = t0.elapsed();
    client.cancel().await.unwrap();
    server.await.unwrap().unwrap();
    let produced = std::fs::read_to_string(&journal).unwrap();
    (got, produced, elapsed)
}

/// Rejeu en mémoire du scénario `name` : résultats égaux aux attendus,
/// journal produit égal au scénario, budget de durée ; si `saved` (nom du
/// fichier enregistré sous `$TMP`), MSPDI `normalize` égal à l'attendu
/// relu. `PLANRS_BLESS=1` régénère les attendus au lieu de comparer.
async fn replay_memory(name: &str, saved: Option<&str>) -> Scenario {
    let root = root();
    let scenario = Scenario::load(&root, name);
    let tmp = TempDir::new("memory");
    let (got, produced, elapsed) = replay_with_journal(&scenario, &tmp).await;
    let saved = saved.map(|f| tmp.path().join(f));
    if replay::blessing() {
        replay::bless(&root, &scenario, &produced, tmp.path(), saved.as_deref());
        return scenario;
    }
    replay::assert_results(&scenario, &got);
    replay::assert_journal(&scenario, &produced, tmp.path());
    assert!(
        elapsed < BUDGET,
        "rejeu de {name} en {elapsed:?}, budget {BUDGET:?}"
    );
    if let Some(saved) = saved {
        let read = |p: &Path| {
            let bytes = std::fs::read(p).unwrap_or_else(|e| panic!("{} : {e}", p.display()));
            planrs_mspdi::read(&bytes, &planrs_mspdi::ReadOptions::new(now()))
                .unwrap_or_else(|e| panic!("{} : {e}", p.display()))
                .project
        };
        let w = planrs_mspdi::write::WriteOptions::default();
        assert_eq!(
            planrs_mspdi::normalize(&read(&saved), &w),
            planrs_mspdi::normalize(&read(&replay::expected_xml_path(&root, name)), &w),
            "{name} : MSPDI enregistré différent de l'attendu"
        );
    }
    scenario
}

/// Vérifications 1 et 5, plus le journal produit égal au scénario.
#[tokio::test]
async fn replay_build_30_memory() {
    replay_memory(SCENARIO, Some("build_30.xml")).await;
}

/// Essai réel avec Claude du 2026-10-19 (`docs/features/mcp.md` § 6) :
/// journal de `claude -p` versionné, chemins ramenés à `$TMP`, rejoué tel
/// quel. Six appels, aucune erreur ; un lot atomique de 75 opérations.
#[tokio::test]
async fn replay_claude_run_1() {
    let scenario = replay_memory("claude_run_1", Some("renovation-maison.xml")).await;
    if replay::blessing() {
        return;
    }
    let tools: Vec<&str> = scenario.calls.iter().map(|c| c.tool.as_str()).collect();
    assert_eq!(
        tools,
        [
            "new_project",
            "apply_batch",
            "critical_path",
            "project_summary",
            "save_project",
            "render_gantt"
        ]
    );
    assert!(
        scenario
            .calls
            .iter()
            .all(|c| error_code(c.expected.as_ref().unwrap()).is_none())
    );
    assert_eq!(scenario.calls[1].params["ops"].as_array().unwrap().len(), 75);
    let summary = &scenario.calls[3].expected.as_ref().unwrap()["ok"];
    assert_eq!(
        (
            &summary["tasks"],
            &summary["summaries"],
            &summary["milestones"],
            &summary["links"],
            &summary["finish"],
            &summary["history_len"],
        ),
        (
            &json!(35),
            &json!(5),
            &json!(4),
            &json!(40),
            &json!("2027-03-10T17:00"),
            &json!(1)
        )
    );
}

/// Vérification 2 : MSPDI enregistré relu, 30 tâches, `normalize` égal à
/// l'attendu relu, aucun diagnostic d'erreur (import et planning).
#[tokio::test]
async fn replay_build_30_saved_mspdi_matches() {
    let root = root();
    let scenario = Scenario::load(&root, SCENARIO);
    let tmp = TempDir::new("saved");
    let (client, _server) = connect(opts(None)).await;
    run(&client, &scenario, tmp.path()).await;
    client.cancel().await.unwrap();

    let read = |p: &Path| {
        let bytes = std::fs::read(p).unwrap_or_else(|e| panic!("{} : {e}", p.display()));
        planrs_mspdi::read(&bytes, &planrs_mspdi::ReadOptions::new(now()))
            .unwrap_or_else(|e| panic!("{} : {e}", p.display()))
    };
    let saved = read(&tmp.path().join("build_30.xml"));
    let expected = read(&replay::expected_xml_path(&root, SCENARIO));
    let tasks = saved
        .project
        .tasks
        .iter()
        .filter(|t| !t.is_null && t.uid().get() != 0)
        .count();
    assert_eq!(tasks, 30);
    let errors: Vec<_> = saved
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "import : {errors:?}");
    let sched = planrs_core::schedule(&saved.project);
    let errors: Vec<_> = sched
        .diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "planning : {errors:?}");
    let w = planrs_mspdi::write::WriteOptions::default();
    assert_eq!(
        planrs_mspdi::normalize(&saved.project, &w),
        planrs_mspdi::normalize(&expected.project, &w)
    );
}

/// Code d'erreur d'un résultat journalisé.
fn error_code(r: &Value) -> Option<&str> {
    r.get("error").and_then(|e| e.get("code")).and_then(Value::as_str)
}

/// Vérification 3 (P30 de bout en bout) : `undo` répété jusqu'à
/// `history.empty` redonne exactement le projet vide du `new_project`
/// (MSPDI exporté identique octet pour octet).
#[tokio::test]
async fn replay_build_30_undo_to_empty() {
    let root = root();
    let scenario = Scenario::load(&root, SCENARIO);
    let tmp = TempDir::new("undo");

    // Référence : le projet juste après `new_project`.
    let first = &scenario.calls[0];
    assert_eq!(first.tool, "new_project");
    let (client, _server) = connect(opts(None)).await;
    let r = call(&client, "new_project", &first.params).await;
    assert!(r.get("ok").is_some(), "{r}");
    let empty = tmp.path().join("empty.xml");
    let r = call(
        &client,
        "export_mspdi",
        &json!({ "path": empty.display().to_string() }),
    )
    .await;
    assert!(r.get("ok").is_some(), "{r}");
    client.cancel().await.unwrap();

    let (client, _server) = connect(opts(None)).await;
    run(&client, &scenario, tmp.path()).await;
    let mut undone = 0_u32;
    loop {
        let r = call(&client, "undo", &json!({})).await;
        if r.get("ok").is_some() {
            undone += 1;
            assert!(undone <= 1_000, "undo sans fin");
            continue;
        }
        assert_eq!(error_code(&r), Some("history.empty"), "{r}");
        break;
    }
    // 30 tâches, 25 liens, 2 contraintes, 1 exception de calendrier.
    assert_eq!(undone, 58);
    let tasks = call(&client, "list_tasks", &json!({})).await;
    assert_eq!(tasks["ok"]["tasks"], json!([]), "{tasks}");
    let after = tmp.path().join("undone.xml");
    let r = call(
        &client,
        "export_mspdi",
        &json!({ "path": after.display().to_string() }),
    )
    .await;
    assert!(r.get("ok").is_some(), "{r}");
    client.cancel().await.unwrap();
    assert_eq!(
        std::fs::read_to_string(&after).unwrap(),
        std::fs::read_to_string(&empty).unwrap(),
        "projet après undo complet différent du projet vide"
    );
}
