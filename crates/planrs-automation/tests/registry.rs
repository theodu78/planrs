//! Registre des commandes (ADR-0012 D7, lot L3) : table triée, noms MCP
//! conformes, outils du brief présents, réservés non listés, schémas de
//! référence (`tests/golden/schemas/<id>.json`, régénérés par
//! `PLANRS_BLESS=1`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use planrs_automation::registry::{Availability, Kind, by_mcp_name, listed, lookup};
use planrs_automation::text::Language;
use planrs_automation::{Batch, Command, REGISTRY};
use serde_json::{Map, Value};

/// Motif des noms d'outils de l'API Claude : `^[a-zA-Z0-9_-]{1,64}$`.
fn is_claude_tool_name(n: &str) -> bool {
    (1..=64).contains(&n.len())
        && n.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[test]
fn registry_sorted_unique_ids_and_mcp_names() {
    let ids: Vec<&str> = REGISTRY.iter().map(|s| s.id).collect();
    for pair in ids.windows(2) {
        assert!(pair[0] < pair[1], "REGISTRY non trié ou en double : {pair:?}");
    }
    let names: BTreeSet<&str> = REGISTRY.iter().map(|s| s.mcp_name).collect();
    assert_eq!(names.len(), REGISTRY.len(), "noms MCP en double");
    for spec in REGISTRY {
        assert_eq!(lookup(spec.id).unwrap().mcp_name, spec.mcp_name);
        assert_eq!(by_mcp_name(spec.mcp_name).unwrap().id, spec.id);
        // Id à point : `famille.action`, minuscules ASCII et `_`.
        let (family, action) = spec.id.split_once('.').unwrap();
        for part in [family, action] {
            assert!(
                !part.is_empty() && part.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                "id mal formé : {}",
                spec.id
            );
        }
        assert_eq!(spec.since_format, 1);
        assert!(!spec.summary_fr.is_empty() && !spec.summary_en.is_empty());
        assert_eq!(spec.summary(Language::Fr), spec.summary_fr);
        assert_eq!(spec.summary(Language::En), spec.summary_en);
        // Seules les écritures acceptent `dry_run` et s'annulent.
        let write = spec.kind == Kind::Write;
        assert_eq!(spec.dry_run, write, "{}", spec.id);
        assert_eq!(spec.undoable, write, "{}", spec.id);
    }
    assert!(lookup("task.nope").is_none());
    assert!(by_mcp_name("nope").is_none());
}

#[test]
fn registry_mcp_names_match_claude_pattern() {
    for spec in REGISTRY {
        assert!(
            is_claude_tool_name(spec.mcp_name),
            "nom MCP invalide : {}",
            spec.mcp_name
        );
    }
    assert!(!is_claude_tool_name("add.task"));
    assert!(!is_claude_tool_name(""));
}

/// Outils V1 du brief (§ 6), listés ou réservés.
const BRIEF_TOOLS: &[&str] = &[
    "open_project",
    "project_summary",
    "list_tasks",
    "get_task",
    "critical_path",
    "resource_usage",
    "baseline_variance",
    "render_gantt",
    "add_task",
    "update_task",
    "delete_task",
    "move_task",
    "add_link",
    "remove_link",
    "set_constraint",
    "assign_resource",
    "update_assignment",
    "add_resource",
    "update_progress",
    "set_baseline",
    "edit_calendar",
    "what_if",
    "explain_date",
    "undo",
    "redo",
    "history",
    "save_project",
    "export_mspdi",
];

#[test]
fn registry_brief_tools_present() {
    for name in BRIEF_TOOLS {
        assert!(by_mcp_name(name).is_some(), "outil du brief absent : {name}");
    }
    // Écarts au brief assumés (ADR-0012 D8) : outils J4 en plus.
    for name in ["new_project", "apply_batch", "import_csv"] {
        assert!(by_mcp_name(name).unwrap().is_listed());
    }
}

#[test]
fn registry_reserved_not_listed() {
    let reserved: BTreeSet<&str> = REGISTRY.iter().filter(|s| !s.is_listed()).map(|s| s.id).collect();
    let expected: BTreeSet<&str> = ["baseline.set", "progress.reschedule", "progress.update"]
        .into_iter()
        .collect();
    // Contrat J6 (lot L0) : six entrées réservées, `progress.reschedule`
    // créée ; `analysis.explain_date` activée par le lot L7,
    // `analysis.what_if` et `analysis.baseline_variance` par le lot L8.
    assert_eq!(reserved, expected);
    assert_eq!(expected.len(), 3);
    for spec in REGISTRY.iter().filter(|s| !s.is_listed()) {
        assert_eq!(spec.available, Availability::Milestone(6));
    }
    assert!(listed().all(|s| s.available == Availability::Now));
    assert_eq!(listed().count(), REGISTRY.len() - expected.len());
}

/// Lot L8 (ADR-0013 D9) : les huit outils J5 sont listés, 38 outils en
/// tout ; `delete_resource` est la seule écriture J5 destructive.
#[test]
fn registry_j5_tools_listed() {
    for name in [
        "add_resource",
        "update_resource",
        "delete_resource",
        "list_resources",
        "assign_resource",
        "update_assignment",
        "unassign_resource",
        "resource_usage",
    ] {
        let spec = by_mcp_name(name).unwrap();
        assert!(spec.is_listed(), "{name}");
        assert_eq!(spec.available, Availability::Now, "{name}");
    }
    // 38 outils en J5, 39 avec `explain_date` (J6, lot L7), 41 avec
    // `what_if` et `baseline_variance` (lot L8).
    assert_eq!(listed().count(), 41);
    assert_eq!(by_mcp_name("list_resources").unwrap().kind, Kind::Read);
    assert_eq!(by_mcp_name("resource_usage").unwrap().kind, Kind::Analysis);
}

/// Chaque commande d'écriture d'un lot a son entrée, de même id, listée et
/// annulable.
#[test]
fn registry_write_commands_match_batch_ops() {
    let json = r#"{"format_version":1,"ops":[
        {"op":"task.add","name":"A"},
        {"op":"task.update","uid":1},
        {"op":"task.delete","uid":1},
        {"op":"task.move","uid":1,"to":"end"},
        {"op":"task.indent","uid":1},
        {"op":"task.outdent","uid":1},
        {"op":"link.add","pred":1,"succ":2},
        {"op":"link.update","pred":1,"succ":2},
        {"op":"link.remove","pred":1,"succ":2},
        {"op":"task.set_constraint","uid":1,"kind":"asap"},
        {"op":"task.set_deadline","uid":1,"deadline":null},
        {"op":"project.set_info"},
        {"op":"project.set_anchor","finish":"2026-12-18T17:00"},
        {"op":"project.set_status_date"},
        {"op":"calendar.set_exception","cal":1,"from":"2026-11-11","to":"2026-11-11"},
        {"op":"calendar.remove_exception","cal":"project","from":"2026-11-11"},
        {"op":"resource.add","name":"R","as":"$r"},
        {"op":"resource.update","uid":"$r","name":"S"},
        {"op":"resource.delete","uid":1},
        {"op":"assignment.add","task":1,"resource":"$r"},
        {"op":"assignment.update","task":1,"resource":1},
        {"op":"assignment.remove","task":1,"resource":1}
    ]}"#;
    let batch = Batch::from_json_str(json).unwrap();
    assert_eq!(batch.ops.len(), 22);
    for op in &batch.ops {
        let spec = lookup(op.id()).unwrap();
        assert!(spec.is_listed() && spec.undoable, "{}", op.id());
        let v = serde_json::to_value(op).unwrap();
        assert_eq!(v["op"], op.id());
    }
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/schemas")
}

/// Vrai si `v` contient une clé `key` à un niveau quelconque.
fn contains_key(v: &Value, key: &str) -> bool {
    match v {
        Value::Object(m) => m.contains_key(key) || m.values().any(|x| contains_key(x, key)),
        Value::Array(a) => a.iter().any(|x| contains_key(x, key)),
        _ => false,
    }
}

/// Un fichier par entrée du registre, listée ou réservée (les entrées
/// réservées J6 portent le schéma de leur contrat, lot L0). Chaque schéma : JSON Schema 2020-12,
/// objet au sommet sans combinateur (exigence des outils de l'API Claude),
/// sans `$ref` (sous-schémas en ligne), convertible en `JsonObject` de rmcp
/// (`serde_json::Map<String, Value>`, `rmcp::model::JsonObject`).
#[test]
fn schemas_golden() {
    let dir = golden_dir();
    let bless = std::env::var_os("PLANRS_BLESS").is_some();
    if bless {
        std::fs::create_dir_all(&dir).unwrap();
    }
    let mut expected_files = BTreeSet::new();
    let mut stale = Vec::new();
    for spec in REGISTRY {
        let schema = serde_json::to_value((spec.schema)()).unwrap();
        let object: Map<String, Value> = serde_json::from_value(schema.clone())
            .unwrap_or_else(|_| panic!("{} : schéma non objet", spec.id));
        assert_eq!(
            object.get("$schema").and_then(Value::as_str),
            Some("https://json-schema.org/draft/2020-12/schema"),
            "{}",
            spec.id
        );
        assert_eq!(
            object.get("type").and_then(Value::as_str),
            Some("object"),
            "{}",
            spec.id
        );
        for combinator in ["oneOf", "anyOf", "allOf"] {
            assert!(
                !object.contains_key(combinator),
                "{} : {combinator} au sommet",
                spec.id
            );
        }
        assert!(!contains_key(&schema, "$ref"), "{} : $ref restant", spec.id);

        let file = format!("{}.json", spec.id);
        let path = dir.join(&file);
        expected_files.insert(file);
        let mut text = serde_json::to_string_pretty(&schema).unwrap();
        text.push('\n');
        if bless {
            std::fs::write(&path, &text).unwrap();
        } else if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            stale.push(spec.id);
        }
    }
    assert!(
        stale.is_empty(),
        "schémas différents de la référence (PLANRS_BLESS=1 pour régénérer) : {stale:?}"
    );
    let present: BTreeSet<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        present, expected_files,
        "fichiers de référence en trop ou manquants"
    );
}

/// Le schéma d'un lot décrit chaque commande par sa constante `op`.
#[test]
fn schemas_batch_lists_every_op() {
    let schema = serde_json::to_value((lookup("batch.apply").unwrap().schema)()).unwrap();
    let text = schema.to_string();
    let json = r#"{"format_version":1,"ops":[{"op":"task.add","name":"A"}]}"#;
    let op: Command = Batch::from_json_str(json).unwrap().ops.remove(0);
    assert!(text.contains(&format!("\"{}\"", op.id())));
    for spec in listed().filter(|s| s.kind == Kind::Write && s.id != "batch.apply") {
        assert!(
            text.contains(&format!("\"const\":\"{}\"", spec.id)),
            "{} absent du lot",
            spec.id
        );
    }
}
