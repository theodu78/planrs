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
    clippy::arithmetic_side_effects,
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

/// Budget du rejeu complet en debug (vérification 5). Temps réel mesuré
/// pendant que les autres rejeux tournent en parallèle sur les runners de la
/// CI (2 à 4 cœurs) : 10 s, qui attrape une explosion de complexité sans
/// dépendre de la charge (0,2 à 0,5 s par rejeu en local, 2,3 s observé sur
/// la CI Linux le 2026-10-19 avec huit rejeux simultanés).
const BUDGET: Duration = Duration::from_secs(10);

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

/// Scénario J5 (lot L9, `docs/plans/j5.md`) : ressources, affectations,
/// types de tâche, pilotage par l'effort, surutilisation, rejoué en mémoire ;
/// MSPDI exporté égal à l'attendu par `normalize`.
///
/// Vérifications du cahier, sur les résultats attendus : `dry_run` sans
/// effet et même diff que l'appel validé ; surutilisation signalée par
/// `diff.overallocated` à l'affectation à 150 %, visible par semaine dans
/// `resource_usage`, résolue par `update_assignment` ; `delete_resource`
/// sans `cascade` refusé ; réouverture du MSPDI exporté : mêmes tâches
/// (durées comprises), mêmes ressources, même résumé (hors chemin, état
/// modifié, historique).
#[tokio::test]
async fn replay_resources_j5_memory() {
    let s = replay_memory("resources_j5", Some("resources_j5.xml")).await;
    if replay::blessing() {
        return;
    }
    let at = |tool: &str, params: &Value| index_of(&s, tool, params);
    let ok = |i: usize| ok_at(&s, i);
    assert!(
        s.calls
            .iter()
            .filter(|c| c.tool != "delete_resource")
            .all(|c| error_code(c.expected.as_ref().unwrap()).is_none())
    );
    // Quatre ressources, un calendrier propre pour chaque ressource de travail.
    let added: Vec<&Value> = (0..s.calls.len())
        .filter(|&i| s.calls[i].tool == "add_resource")
        .map(|i| &ok(i)["side_effects"])
        .collect();
    assert_eq!(
        added,
        [
            &json!([{ "calendar_created": 4 }]),
            &json!([{ "calendar_created": 5 }]),
            &json!([]),
            &json!([])
        ]
    );
    // Électricien ajouté à une tâche pilotée par l'effort : dry_run puis
    // validé, même diff ; 48 h gardées, 24 h chacun (3 j par affectation) ;
    // durée de la tâche = union des parts, 5 j du lun 23 au ven 27/11 (la
    // part de l'Électricien décalée par son congé, R-173, S14).
    let dry = at(
        "assign_resource",
        &json!({ "task": 4, "resource": 2, "dry_run": true }),
    );
    assert_eq!(ok(dry)["committed"], json!(false));
    assert_eq!(ok(dry)["diff"], ok(dry + 1)["diff"]);
    assert_eq!(ok(dry + 1)["committed"], json!(true));
    let reseaux = ok(at("get_task", &json!({ "uid": 4 })));
    assert_eq!(
        (&reseaux["work"]["text"], &reseaux["duration"]["text"]),
        (&json!("48h"), &json!("5j"))
    );
    // Durée fixe : unités à 50 % ⇒ travail 16 h ; travail fixe : 8 j ⇒ 50 %.
    let charpente = ok(at("get_task", &json!({ "uid": 3 })));
    assert_eq!(charpente["work"]["text"], json!("16h"));
    let enduits = ok(at("get_task", &json!({ "uid": 6 })));
    assert_eq!(
        (
            &enduits["work"]["text"],
            &enduits["resources"][0]["units"]["text"]
        ),
        (&json!("32h"), &json!("50%"))
    );
    // Béton (matériel, UID 3) quantité 12 sur Dalle (UID 7) : la quantité
    // rendue est celle saisie (portée par le travail, 12 h, O30), relisible
    // comme `quantity`, dans les vues des tâches et des ressources.
    let last = |tool: &str| s.calls.iter().rposition(|c| c.tool == tool).unwrap();
    let listed = ok(last("list_resources"));
    let beton = listed["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["uid"] == json!(3))
        .unwrap();
    let twelve = json!({ "text": "12", "value": 120_000 });
    assert_eq!(beton["assignments"][0]["units"], twelve, "{beton}");
    assert_eq!(beton["assignments"][0]["effective_units"], twelve, "{beton}");
    let tasks = ok(last("list_tasks"));
    let dalle = tasks["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["uid"] == json!(7))
        .unwrap();
    let on_dalle = dalle["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["uid"] == json!(3))
        .unwrap();
    assert_eq!(on_dalle["units"], twelve, "{dalle}");
    // Surutilisation du Maçon (UID 1).
    let over = at(
        "assign_resource",
        &json!({ "task": 7, "resource": 1, "units": { "text": "150%" } }),
    );
    assert_eq!(
        ok(over)["diff"]["overallocated"],
        json!({ "new": [1], "resolved": [] })
    );
    let usage = ok(over + 1);
    assert_eq!(s.calls[over + 1].tool, "resource_usage");
    assert_eq!(
        usage["rows"][0]["overallocated"],
        json!([false, true, false, false])
    );
    let fix = at(
        "update_assignment",
        &json!({ "task": 7, "resource": 1, "units": { "text": "100%" } }),
    );
    assert_eq!(
        ok(fix)["diff"]["overallocated"],
        json!({ "new": [], "resolved": [1] })
    );
    let refused = s.calls.iter().find(|c| c.tool == "delete_resource").unwrap();
    assert_eq!(
        error_code(refused.expected.as_ref().unwrap()),
        Some("resource.has_assignments")
    );
    assert_reopened(&s);
}

/// Index du premier appel `tool` de paramètres `params`.
fn index_of(s: &Scenario, tool: &str, params: &Value) -> usize {
    s.calls
        .iter()
        .position(|c| c.tool == tool && &c.params == params)
        .unwrap_or_else(|| panic!("{tool} {params} absent du scénario"))
}

/// Résultat `ok` attendu de l'appel `i`.
fn ok_at(s: &Scenario, i: usize) -> &Value {
    let r = s.calls[i].expected.as_ref().unwrap();
    r.get("ok").unwrap_or_else(|| panic!("appel {} : {r}", i + 1))
}

/// `resources_j5` : la réouverture du MSPDI exporté redonne le projet.
fn assert_reopened(s: &Scenario) {
    let at = |tool: &str, params: &Value| index_of(s, tool, params);
    let ok = |i: usize| ok_at(s, i);
    // Réouverture : mêmes tâches (dates, durées, travail, marges), mêmes
    // ressources. La durée d'une feuille placée par plusieurs parts est la
    // même en session et relue : l'union des parts (Réseaux, UID 4 : 5 j,
    // du lun 23 au ven 27/11), reportée dans le modèle après chaque recalcul
    // (R-173) et écrite par l'écrivain MSPDI (R-199).
    let opened = at("open_project", &json!({ "path": "$TMP/resources_j5.xml" }));
    let before = opened - 4;
    assert_eq!(s.calls[before].tool, "list_tasks");
    assert_eq!(s.calls[opened + 1].tool, "list_tasks");
    let reseaux = ok(before)["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["uid"] == json!(4))
        .unwrap();
    assert_eq!(reseaux["duration"]["minutes"], json!(2400), "{reseaux}");
    assert_eq!(ok(opened + 1), ok(before));
    assert_eq!(s.calls[opened + 2].tool, "list_resources");
    assert_eq!(ok(opened + 2), ok(before + 1));
    let strip = |v: &Value| {
        let mut v = v.clone();
        for k in ["dirty", "path", "history_len"] {
            v.as_object_mut().unwrap().remove(k);
        }
        v
    };
    assert_eq!(strip(ok(opened + 3)), strip(ok(before + 2)));
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

/// Essai réel n° 2 avec Claude du 2026-10-19 (`docs/features/mcp.md` § 6,
/// ressources J5) : journal de `claude -p` versionné, chemins ramenés à
/// `$TMP`, rejoué tel quel. Seize appels, aucune erreur ; trois
/// `apply_batch` en `dry_run` comparés avant d'appliquer ; six artisans,
/// surutilisation détectée (Maçon, Carreleur) puis résolue par des liens,
/// aucune surcharge à la fin ; fin du projet mer 27/01/2027 17:00.
#[tokio::test]
async fn replay_claude_run_2() {
    let s = replay_memory("claude_run_2", Some("renovation-maison.xml")).await;
    if replay::blessing() {
        return;
    }
    let tools: Vec<&str> = s.calls.iter().map(|c| c.tool.as_str()).collect();
    assert_eq!(
        tools,
        [
            "new_project",
            "apply_batch",
            "apply_batch",
            "list_resources",
            "resource_usage",
            "list_tasks",
            "critical_path",
            "apply_batch",
            "apply_batch",
            "apply_batch",
            "apply_batch",
            "critical_path",
            "resource_usage",
            "list_resources",
            "save_project",
            "render_gantt"
        ]
    );
    assert!(
        s.calls
            .iter()
            .all(|c| error_code(c.expected.as_ref().unwrap()).is_none())
    );
    let ok = |i: usize| ok_at(&s, i);
    // Surutilisation détectée : Maçon (UID 1) et Carreleur (UID 5).
    let over: Vec<&Value> = ok(3)["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| &r["uid"])
        .collect();
    assert_eq!(over, [&json!(1), &json!(5)]);
    // Trois essais en `dry_run`, rien d'appliqué ; le premier fait
    // apparaître une surcharge du Plaquiste (UID 4).
    for i in 7..=9 {
        assert_eq!(s.calls[i].params["dry_run"], json!(true));
        assert_eq!(ok(i)["committed"], json!(false));
    }
    assert_eq!(
        ok(7)["diff"]["overallocated"],
        json!({ "new": [4], "resolved": [1, 5] })
    );
    // Lot validé : surcharges résolues, aucune nouvelle.
    assert_eq!(ok(10)["committed"], json!(true));
    assert_eq!(
        ok(10)["diff"]["overallocated"],
        json!({ "new": [], "resolved": [1, 5] })
    );
    // Fin du projet et chemin critique après la résolution.
    assert_eq!(ok(11)["project_finish"], json!("2027-01-27T17:00"));
    assert_eq!(ok(11)["total"], json!(22));
    // Six artisans, aucune surcharge restante.
    let all = ok(13)["resources"].as_array().unwrap();
    assert_eq!(all.len(), 6);
    assert!(all.iter().all(|r| r["overallocated"] == json!(false)));
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
