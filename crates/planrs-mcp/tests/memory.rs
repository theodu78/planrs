//! Serveur MCP en mémoire : client rmcp sur `tokio::io::duplex` (lot L8,
//! C-MCP-04 à C-MCP-07).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::missing_panics_doc,
    reason = "tests : un échec doit s'arrêter net"
)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use planrs_automation::REGISTRY;
use planrs_automation::journal::parse_line;
use planrs_automation::text::Language;
use planrs_core::Moment;
use planrs_mcp::{McpServeError, ServeOptions, serve_on};
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::RunningService;
use rmcp::{RoleClient, ServiceExt};
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;
use tokio::task::JoinHandle;

fn now() -> Moment {
    "2026-10-19T08:00".parse().unwrap()
}

fn opts() -> ServeOptions {
    // Garde ADR-0002 testée à part (guard.rs) : ici, aucun rendez-vous lu.
    let mut o = ServeOptions::new(now(), Language::En);
    o.ignore_app_guard = true;
    o
}

/// Dossier temporaire propre à un test (retiré à la fin du test).
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("planrs-mcp-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

type Client = RunningService<RoleClient, ()>;

/// Serveur sur un `duplex`, client rmcp connecté.
async fn connect(opts: ServeOptions) -> (Client, JoinHandle<Result<(), McpServeError>>) {
    let (client_io, server_io) = tokio::io::duplex(1 << 20);
    let (r, w) = tokio::io::split(server_io);
    let server = tokio::spawn(serve_on(r, w, opts));
    let client = ().serve(client_io).await.unwrap();
    (client, server)
}

async fn call(client: &Client, name: &'static str, args: Value) -> CallToolResult {
    let Value::Object(args) = args else {
        panic!("arguments : objet attendu")
    };
    client
        .call_tool(CallToolRequestParams::new(name).with_arguments(args))
        .await
        .unwrap()
}

/// Appel réussi : contenu structuré.
async fn ok(client: &Client, name: &'static str, args: Value) -> Value {
    let r = call(client, name, args).await;
    assert_eq!(r.is_error, Some(false), "{name} : {:?}", r.structured_content);
    r.structured_content.unwrap()
}

/// Appel refusé : erreur structurée.
async fn refused(client: &Client, name: &'static str, args: Value) -> Value {
    let r = call(client, name, args).await;
    assert_eq!(r.is_error, Some(true), "{name} : {:?}", r.structured_content);
    r.structured_content.unwrap()
}

async fn new_project(client: &Client) {
    ok(
        client,
        "new_project",
        json!({ "name": "Maison", "start": "2026-11-02T08:00" }),
    )
    .await;
}

#[tokio::test]
async fn mcp_list_tools_from_registry() {
    let (client, _server) = connect(opts()).await;
    let tools = client.list_all_tools().await.unwrap();
    let listed: BTreeSet<String> = tools.iter().map(|t| t.name.to_string()).collect();
    let expected: BTreeSet<String> = REGISTRY
        .iter()
        .filter(|s| s.is_listed())
        .map(|s| s.mcp_name.to_owned())
        .collect();
    assert_eq!(listed, expected);
    assert_eq!(tools.len(), expected.len());
    for t in &tools {
        // C-MCP-07.
        assert!(
            (1..=64).contains(&t.name.len())
                && t.name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
            "{}",
            t.name
        );
        let spec = planrs_automation::registry::by_mcp_name(&t.name).unwrap();
        assert_eq!(t.input_schema.get("type"), Some(&json!("object")));
        let props = t.input_schema.get("properties").and_then(Value::as_object);
        let write = spec.kind == planrs_automation::Kind::Write;
        assert_eq!(
            props.is_some_and(|p| p.contains_key("dry_run")),
            write,
            "{}",
            t.name
        );
        let read_only = t.annotations.as_ref().and_then(|a| a.read_only_hint);
        let reads = matches!(
            spec.kind,
            planrs_automation::Kind::Read | planrs_automation::Kind::Analysis
        );
        assert_eq!(read_only, Some(reads), "{}", t.name);
        assert!(t.description.as_deref().is_some_and(|d| !d.is_empty()));
    }
    let info = client.peer_info().unwrap();
    assert!(info.capabilities.tools.is_some());
    assert!(
        info.instructions
            .as_deref()
            .is_some_and(|i| i.contains("dry_run"))
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn mcp_new_project_add_task_roundtrip() {
    let (client, _server) = connect(opts()).await;
    new_project(&client).await;
    let out = ok(
        &client,
        "add_task",
        json!({ "name": "Fondations", "duration": { "text": "5d" } }),
    )
    .await;
    assert_eq!(out["committed"], json!(true));
    assert_eq!(out["history_len"], json!(1));
    let uid = out["created_uids"][0].as_u64().unwrap();
    let created = &out["created"][0];
    assert_eq!(created["uid"].as_u64(), Some(uid));
    assert_eq!(created["name"], json!("Fondations"));
    // Dates du moteur (calendrier Standard : 08:00-12:00, 13:00-17:00).
    assert_eq!(created["start"], json!("2026-11-02T08:00"));
    assert_eq!(created["finish"], json!("2026-11-06T17:00"));
    assert_eq!(created["duration"]["minutes"], json!(2400));

    let got = ok(&client, "get_task", json!({ "uid": uid })).await;
    assert_eq!(&got, created);
    let summary = ok(&client, "project_summary", json!({})).await;
    assert_eq!(summary["tasks"], json!(1));
    assert_eq!(summary["finish"], json!("2026-11-06T17:00"));
    assert_eq!(summary["history_len"], json!(1));
    // Le texte du résultat est le même JSON (clients sans structuredContent).
    let r = call(&client, "project_summary", json!({})).await;
    let text = r.content[0].as_text().unwrap().text.clone();
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), summary);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn mcp_dry_run_no_history() {
    let (client, _server) = connect(opts()).await;
    new_project(&client).await;
    let out = ok(
        &client,
        "add_task",
        json!({ "name": "A", "duration": { "text": "2d" }, "dry_run": true }),
    )
    .await;
    assert_eq!(out["committed"], json!(false));
    assert_eq!(out["history_len"], json!(0));
    assert_eq!(out["diff"]["finish_after"], json!("2026-11-03T17:00"));
    let batch = ok(
        &client,
        "apply_batch",
        json!({ "ops": [
            { "op": "task.add", "name": "B", "as": "$b" },
            { "op": "task.add", "name": "C", "as": "$c" },
            { "op": "link.add", "pred": "$b", "succ": "$c" }
        ], "dry_run": true }),
    )
    .await;
    assert_eq!(batch["committed"], json!(false));
    // C-MCP-04 : aucune entrée d'historique, aucune tâche.
    let history = ok(&client, "history", json!({})).await;
    assert_eq!(ok(&client, "project_summary", json!({})).await["tasks"], json!(0));
    assert_eq!(
        ok(&client, "project_summary", json!({})).await["history_len"],
        json!(0)
    );
    assert_eq!(history["entries"], json!([]));
    // Puis un vrai lot : une seule entrée pour trois commandes.
    let batch = ok(
        &client,
        "apply_batch",
        json!({ "ops": [
            { "op": "task.add", "name": "B", "as": "$b" },
            { "op": "task.add", "name": "C", "as": "$c" },
            { "op": "link.add", "pred": "$b", "succ": "$c" }
        ] }),
    )
    .await;
    assert_eq!(batch["history_len"], json!(1));
    assert_eq!(batch["labels"].as_object().unwrap().len(), 2);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn mcp_error_is_structured_not_rpc() {
    let (client, _server) = connect(opts()).await;
    // Aucun projet : erreur métier, pas JSON-RPC (C-MCP-05).
    let e = refused(&client, "add_task", json!({ "name": "A" })).await;
    assert_eq!(e["code"], json!("session.none"));
    assert!(e["message"].as_str().is_some_and(|m| m.contains("new_project")));
    new_project(&client).await;
    let e = refused(&client, "delete_task", json!({ "uid": 999 })).await;
    assert_eq!(e["code"], json!("task.not_found"));
    assert_eq!(e["op_index"], json!(0));
    let e = refused(&client, "add_task", json!({ "nom": "A" })).await;
    assert_eq!(e["code"], json!("param.invalid"));
    // Cycle : refus R-160 avec diagnostics.
    let out = ok(
        &client,
        "apply_batch",
        json!({ "ops": [
            { "op": "task.add", "name": "A", "as": "$a" },
            { "op": "task.add", "name": "B", "as": "$b" },
            { "op": "link.add", "pred": "$a", "succ": "$b" }
        ] }),
    )
    .await;
    let a = out["labels"]["$a"].clone();
    let b = out["labels"]["$b"].clone();
    let e = refused(&client, "add_link", json!({ "pred": b, "succ": a })).await;
    assert_eq!(e["code"], json!("batch.would_create_errors"));
    assert_ne!(e["diagnostics"].as_array().map(Vec::len), Some(0));
    // Commande réservée (J5-J6) appelée par son nom : erreur structurée.
    let e = refused(&client, "add_resource", json!({})).await;
    assert_eq!(e["code"], json!("command.not_yet_available"));
    // Seul un nom inconnu est une erreur JSON-RPC.
    let rpc = client.call_tool(CallToolRequestParams::new("no_such_tool")).await;
    assert!(rpc.is_err());
    // La session reste utilisable.
    assert_eq!(ok(&client, "project_summary", json!({})).await["tasks"], json!(2));
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn mcp_undo_redo() {
    let (client, _server) = connect(opts()).await;
    new_project(&client).await;
    ok(
        &client,
        "add_task",
        json!({ "name": "A", "duration": { "text": "1d" } }),
    )
    .await;
    ok(
        &client,
        "add_task",
        json!({ "name": "B", "duration": { "text": "1d" } }),
    )
    .await;
    let tasks = |v: Value| v["tasks"].clone();
    assert_eq!(tasks(ok(&client, "project_summary", json!({})).await), json!(2));
    let u = ok(&client, "undo", json!({})).await;
    assert_eq!(u["history_len"], json!(1));
    assert_eq!(tasks(ok(&client, "project_summary", json!({})).await), json!(1));
    ok(&client, "undo", json!({})).await;
    assert_eq!(tasks(ok(&client, "project_summary", json!({})).await), json!(0));
    let e = refused(&client, "undo", json!({})).await;
    assert_eq!(e["code"], json!("history.empty"));
    ok(&client, "redo", json!({})).await;
    ok(&client, "redo", json!({})).await;
    let s = ok(&client, "project_summary", json!({})).await;
    assert_eq!(tasks(s.clone()), json!(2));
    assert_eq!(s["finish"], json!("2026-11-02T17:00"));
    let e = refused(&client, "redo", json!({})).await;
    assert_eq!(e["code"], json!("history.empty"));
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn mcp_list_tasks_paginated() {
    let (client, _server) = connect(opts()).await;
    new_project(&client).await;
    let ops: Vec<Value> = (1..=60)
        .map(|i| json!({ "op": "task.add", "name": format!("T{i}"), "duration": { "text": "1d" } }))
        .collect();
    ok(&client, "apply_batch", json!({ "ops": ops })).await;
    let p1 = ok(&client, "list_tasks", json!({})).await;
    assert_eq!(p1["tasks"].as_array().unwrap().len(), 50);
    assert_eq!(p1["total"], json!(60));
    let cursor = p1["next_cursor"].clone();
    assert!(cursor.is_u64());
    let p2 = ok(&client, "list_tasks", json!({ "cursor": cursor })).await;
    let page2 = p2["tasks"].as_array().unwrap();
    assert_eq!(page2.len(), 10);
    assert_eq!(page2[0]["name"], json!("T51"));
    assert_eq!(p2["next_cursor"], Value::Null);
    let e = refused(&client, "list_tasks", json!({ "limit": 501 })).await;
    assert_eq!(e["code"], json!("limit.exceeded"));
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn mcp_render_gantt_bounded() {
    let (client, _server) = connect(opts()).await;
    new_project(&client).await;
    let ops: Vec<Value> = (1..=10)
        .map(|i| json!({ "op": "task.add", "name": format!("T{i}"), "duration": { "text": "1d" } }))
        .collect();
    ok(&client, "apply_batch", json!({ "ops": ops })).await;
    let r = call(&client, "render_gantt", json!({ "max_rows": 4 })).await;
    assert_eq!(r.is_error, Some(false));
    let report = r.structured_content.clone().unwrap();
    assert_eq!(report["shown"], json!(4));
    assert_eq!(report["hidden"], json!(6));
    assert!(report.get("svg").is_none());
    let summary = &r.content[0].as_text().unwrap().text;
    assert_eq!(
        summary,
        "10 tasks, 10 critical, finish 11/02/2026 17:00 (6 rows not shown)"
    );
    let svg = &r.content[1].as_text().unwrap().text;
    assert!(
        svg.starts_with("<svg") || svg.starts_with("<?xml"),
        "{}",
        svg.chars().take(40).collect::<String>()
    );
    assert_eq!(report["bytes"].as_u64(), Some(svg.len() as u64));
    // Troisième bloc : le rapport structuré en JSON (spécification MCP).
    assert_eq!(r.content.len(), 3);
    let text: Value = serde_json::from_str(&r.content[2].as_text().unwrap().text).unwrap();
    assert_eq!(text, report);
    let e = refused(&client, "render_gantt", json!({ "max_rows": 201 })).await;
    assert_eq!(e["code"], json!("limit.exceeded"));
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn mcp_journal_written_before_apply() {
    let dir = TempDir::new("journal");
    let path = dir.0.join("run.jsonl");
    let mut o = opts();
    o.journal = Some(path.clone());
    let (client, _server) = connect(o).await;
    new_project(&client).await;
    ok(&client, "add_task", json!({ "name": "A" })).await;
    refused(&client, "delete_task", json!({ "uid": 999 })).await;
    client.cancel().await.unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let entries: Vec<_> = text.lines().map(|l| parse_line(l).unwrap()).collect();
    assert_eq!(entries.len(), 6);
    for (i, pair) in entries.chunks(2).enumerate() {
        let (req, res) = (&pair[0], &pair[1]);
        let seq = u64::try_from(i).unwrap() + 1;
        // La requête précède le résultat, même `seq`, sans résultat.
        assert_eq!((req.seq, res.seq), (seq, seq));
        assert_eq!(req.result, None);
        assert_ne!(req.params, Value::Null);
        assert_eq!(res.params, Value::Null);
        assert_eq!(req.tool, res.tool);
    }
    assert_eq!(entries[0].tool, "new_project");
    assert_eq!(entries[2].params, json!({ "name": "A" }));
    let added = entries[3].result.as_ref().unwrap();
    assert_eq!(added["ok"]["committed"], json!(true));
    let refused = entries[5].result.as_ref().unwrap();
    assert_eq!(refused["error"]["code"], json!("task.not_found"));
}

/// Un journal qu'on ne peut pas écrire n'empêche pas le travail.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn mcp_journal_write_failure_does_not_block() {
    let mut o = opts();
    o.journal = Some(PathBuf::from("/dev/full"));
    let (client, _server) = connect(o).await;
    new_project(&client).await;
    let out = ok(&client, "add_task", json!({ "name": "A" })).await;
    assert_eq!(out["committed"], json!(true));
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn mcp_journal_unopenable_fails_at_start() {
    let dir = TempDir::new("nojournal");
    let mut o = opts();
    o.journal = Some(dir.0.join("absent").join("run.jsonl"));
    let (_c, s) = tokio::io::duplex(64);
    let (r, w) = tokio::io::split(s);
    let res = serve_on(r, w, o).await;
    assert!(matches!(res, Err(McpServeError::Io(_))), "{res:?}");
}

/// C-MCP-06 : une ligne de plus de 8 Mio ferme la session proprement.
#[tokio::test]
async fn mcp_line_too_long_closes() {
    let (client_io, server_io) = tokio::io::duplex(1 << 16);
    let (r, w) = tokio::io::split(server_io);
    let server = tokio::spawn(serve_on(r, w, opts()));
    let init = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-06-18", "capabilities": {},
                    "clientInfo": { "name": "test", "version": "0" } }
    });
    let (mut read_half, mut write_half) = tokio::io::split(client_io);
    // Les réponses du serveur sont lues pour ne pas remplir le tube.
    let drain = async {
        let mut sink = Vec::new();
        let _ = tokio::io::AsyncReadExt::read_to_end(&mut read_half, &mut sink).await;
    };
    let feed = async {
        write_half
            .write_all(format!("{init}\n").as_bytes())
            .await
            .unwrap();
        write_half
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .await
            .unwrap();
        // 8 Mio + 1 octets sans fin de ligne.
        let chunk = vec![b'x'; 1 << 16];
        let total = 8 * 1024 * 1024 + 1;
        let mut sent = 0usize;
        while sent < total {
            let n = chunk.len().min(total - sent);
            if write_half.write_all(&chunk[..n]).await.is_err() {
                break;
            }
            sent += n;
        }
    };
    let finished = async { server.await.unwrap() };
    let ((), (), res) = tokio::join!(
        async {
            tokio::select! {
                () = drain => {}
                () = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
            }
        },
        feed,
        finished
    );
    assert_eq!(res, Err(McpServeError::LineTooLong));
}

/// Entrée fermée avant l'initialisation : fin normale.
#[tokio::test]
async fn mcp_eof_before_initialize_is_clean() {
    let (client_io, server_io) = tokio::io::duplex(64);
    let (r, w) = tokio::io::split(server_io);
    drop(client_io);
    assert_eq!(serve_on(r, w, opts()).await, Ok(()));
}
