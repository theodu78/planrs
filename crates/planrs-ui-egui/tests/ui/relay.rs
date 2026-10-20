//! Relais côté application, bandeau de validation (J7, lot L3, ADR-0015
//! D9 à D11) : contrats C-CTL-02 à C-CTL-05, C-CTL-07, C-CTL-09, bandeau
//! H-UI-27 au clavier, entrée d'historique « agent », `app.ui_state`,
//! `app.ui`, propriété P47 (scénario de 30 appels rejoué par le relais en
//! mémoire et en local). C-CTL-02 et C-CTL-04 passent aussi par la socket
//! réelle (écouteur et client du canal) sous Unix.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::Duration;

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use planrs_automation::control::{CallState, RelayCall, RelayKind, Reply, WireError};
use planrs_automation::error::codes;
use planrs_automation::text::Language as ToolLanguage;
use planrs_automation::{EditSource, Workspace};
use planrs_ui_egui::i18n::{Arg, TextId};
use planrs_ui_egui::{Action, App, LanguagePref, ThemePref, TimeScaleZoom, ViewKind};
use serde_json::{Value, json};

use crate::{fr, harness, now, open_demo};

/// Identifiant d'appel unique.
fn next_id() -> u64 {
    static N: AtomicU64 = AtomicU64::new(100);
    N.fetch_add(1, Ordering::Relaxed)
}

/// Exemple ouvert, file des appels branchée.
fn relay_harness() -> (Harness<'static, App>, SyncSender<RelayCall>) {
    let mut harness = harness();
    let (tx, rx) = sync_channel(64);
    harness.state_mut().control_inbox(rx);
    open_demo(&mut harness);
    (harness, tx)
}

/// Appel relayé en file.
struct Sent {
    id: u64,
    state: Arc<CallState>,
    replies: Receiver<Reply>,
}

impl Sent {
    /// Réponses arrivées.
    fn drain(&self) -> Vec<Reply> {
        self.replies.try_iter().collect()
    }
}

fn push(tx: &SyncSender<RelayCall>, kind: RelayKind) -> Sent {
    let id = next_id();
    let (reply, replies) = sync_channel(4);
    let state = Arc::new(CallState::new());
    tx.send(RelayCall {
        id,
        kind,
        state: Arc::clone(&state),
        reply,
    })
    .unwrap();
    Sent { id, state, replies }
}

fn execute(tx: &SyncSender<RelayCall>, tool: &str, params: Value) -> Sent {
    push(
        tx,
        RelayKind::Execute {
            tool: tool.to_owned(),
            params,
        },
    )
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

fn ok_value(reply: Reply) -> Value {
    match reply {
        Reply::Ok { result, .. } => result,
        other => panic!("succès attendu : {other:?}"),
    }
}

/// Résultat final d'un appel : écriture acceptée (`approve`) ou refusée.
fn finish(harness: &mut Harness<'static, App>, sent: &Sent, approve: bool) -> Reply {
    harness.run();
    let mut replies = sent.drain();
    if replies.first() == Some(&Reply::PendingApproval { id: sent.id }) {
        replies.remove(0);
        harness.state_mut().dispatch(if approve {
            Action::Approve(sent.id)
        } else {
            Action::Reject(sent.id)
        });
        harness.run();
        replies.extend(sent.drain());
    }
    assert_eq!(replies.len(), 1, "{replies:?}");
    replies.remove(0)
}

/// Projet du document en JSON.
fn project_json(app: &App) -> Value {
    serde_json::to_value(app.document.as_ref().unwrap().project()).unwrap()
}

fn generation(app: &App) -> u64 {
    app.document.as_ref().unwrap().generation()
}

fn pending_ids(app: &App) -> Vec<u64> {
    app.relay().unwrap().pending().iter().map(|p| p.id).collect()
}

fn rename(uid: u32, name: &str) -> Value {
    json!({ "uid": uid, "name": name })
}

/// C-CTL-02 (mémoire) : `pending_approval` aussitôt, `ok` après Accepter ;
/// une entrée d'historique d'origine `Agent`.
#[test]
fn c_ctl_02_pending_then_ok() {
    let (mut harness, tx) = relay_harness();
    let sent = execute(&tx, "task.update", rename(3, "Plans validés"));
    harness.run();
    assert_eq!(sent.drain(), [Reply::PendingApproval { id: sent.id }]);
    assert_eq!(pending_ids(harness.state()), [sent.id]);
    assert_eq!(generation(harness.state()), 0, "rien d'appliqué avant Accepter");
    harness.state_mut().dispatch(Action::Approve(sent.id));
    harness.run();
    let result = ok_value(sent.drain().remove(0));
    assert_eq!(result["committed"], true);
    let app = harness.state();
    assert_eq!(generation(app), 1);
    let history = app.document.as_ref().unwrap().session.history();
    assert_eq!(history.entries.len(), 1);
    assert_eq!(history.entries[0].source, EditSource::Agent);
    assert_eq!(app.relay().unwrap().pending().len(), 0);
}

/// C-CTL-03 : Refuser ⇒ `control.rejected` ; projet, historique,
/// génération et drapeau modifié intacts.
#[test]
fn c_ctl_03_reject_leaves_state() {
    let (mut harness, tx) = relay_harness();
    let before = project_json(harness.state());
    let sent = execute(&tx, "task.update", rename(3, "Refusé"));
    let reply = finish(&mut harness, &sent, false);
    assert_eq!(err_code(&reply), codes::CONTROL_REJECTED);
    let app = harness.state();
    assert_eq!(project_json(app), before);
    assert_eq!(generation(app), 0);
    assert!(!app.document.as_ref().unwrap().is_dirty());
    assert_eq!(app.document.as_ref().unwrap().session.history().entries.len(), 0);
    assert!(!sent.state.claim(), "appel déjà pris");
}

/// C-CTL-04 (application) : appel expiré ⇒ bandeau retiré, et un Accepter
/// tardif n'applique rien ; `Cancel` retire aussi le bandeau.
#[test]
fn c_ctl_04_timeout_then_accept_does_nothing() {
    let (mut harness, tx) = relay_harness();
    let sent = execute(&tx, "task.update", rename(3, "Trop tard"));
    harness.run();
    assert_eq!(pending_ids(harness.state()), [sent.id]);
    // Délai de l'écouteur : expiré, puis `Cancel`.
    assert!(sent.state.expire());
    harness.state_mut().dispatch(Action::Approve(sent.id));
    harness.run();
    assert_eq!(pending_ids(harness.state()).len(), 0);
    assert_eq!(generation(harness.state()), 0);
    assert_eq!(sent.drain(), [Reply::PendingApproval { id: sent.id }]);
    // `Cancel(id)` seul suffit à retirer le bandeau.
    let second = execute(&tx, "task.update", rename(3, "Annulé"));
    harness.run();
    assert_eq!(pending_ids(harness.state()), [second.id]);
    let _cancel = push(&tx, RelayKind::Cancel(second.id));
    harness.run();
    assert_eq!(pending_ids(harness.state()).len(), 0);
    assert_eq!(generation(harness.state()), 0);
}

/// C-CTL-05 : ouvertures relayées refusées `control.not_relayable`, avec
/// ou sans document ; id inconnu `param.invalid` ; sans document,
/// `session.none`.
#[test]
fn c_ctl_05_open_not_relayable() {
    let mut harness = harness();
    let (tx, rx) = sync_channel(8);
    harness.state_mut().control_inbox(rx);
    for open in [true, false] {
        let calls = [
            ("project.open", json!({ "path": "/tmp/x.xml" })),
            ("project.new", json!({ "name": "P", "start": "2027-01-18T08:00" })),
            ("project.import_csv", json!({ "path": "/tmp/x.csv" })),
        ];
        for (tool, params) in calls {
            let sent = execute(&tx, tool, params);
            let reply = finish(&mut harness, &sent, true);
            assert_eq!(err_code(&reply), codes::CONTROL_NOT_RELAYABLE, "{tool}");
        }
        let sent = execute(&tx, "task.nope", json!({}));
        assert_eq!(err_code(&finish(&mut harness, &sent, true)), codes::PARAM_INVALID);
        let sent = execute(&tx, "project.summary", json!({}));
        let reply = finish(&mut harness, &sent, true);
        if open {
            assert_eq!(err_code(&reply), codes::SESSION_NONE);
            open_demo(&mut harness);
        } else {
            assert!(ok_value(reply)["tasks"].is_number());
        }
    }
}

/// C-CTL-07 : génération changée entre l'aperçu et Accepter ⇒ nouvel
/// aperçu, rien d'appliqué ; le second Accepter applique.
#[test]
fn c_ctl_07_stale_generation_new_preview() {
    let (mut harness, tx) = relay_harness();
    let sent = execute(
        &tx,
        "task.update",
        json!({ "uid": 3, "duration": { "text": "20j" } }),
    );
    harness.run();
    let first = harness.state().relay().unwrap().pending()[0]
        .preview
        .clone()
        .unwrap();
    assert!(first.changed_tasks > 0);
    // L'utilisateur modifie le projet entre-temps.
    let batch = planrs_automation::Batch::from_json_str(
        r#"{"format_version":1,"ops":[{"op":"task.update","uid":2,"duration":{"text":"1j"}}]}"#,
    )
    .unwrap();
    harness.state_mut().dispatch(Action::Exec(batch));
    harness.run();
    assert_eq!(generation(harness.state()), 1);
    let after_user = project_json(harness.state());
    harness.state_mut().dispatch(Action::Approve(sent.id));
    harness.run();
    let app = harness.state();
    assert_eq!(project_json(app), after_user, "aperçu périmé : rien d'appliqué");
    let pending = &app.relay().unwrap().pending()[0];
    assert_eq!((pending.id, pending.generation), (sent.id, 1));
    assert_ne!(pending.preview.as_ref().unwrap(), &first, "nouvel aperçu");
    assert_eq!(sent.drain(), [Reply::PendingApproval { id: sent.id }]);
    harness.state_mut().dispatch(Action::Approve(sent.id));
    harness.run();
    assert_eq!(ok_value(sent.drain().remove(0))["committed"], true);
    assert_eq!(generation(harness.state()), 2);
}

/// C-CTL-09 : une lecture relayée rend le même JSON que `Workspace::call`
/// local sur la même session.
#[test]
fn c_ctl_09_read_same_as_local() {
    let (mut harness, tx) = relay_harness();
    let mut ws = Workspace::new(now(), ToolLanguage::Fr);
    ws.set_session(harness.state().document.as_ref().unwrap().session.clone());
    let reads = [
        ("project.summary", json!({})),
        ("task.list", json!({})),
        ("task.get", json!({ "uid": 3 })),
        ("task.get", json!({ "uid": 999 })),
        ("resource.list", json!({})),
        ("analysis.critical_path", json!({})),
        ("analysis.explain_date", json!({ "task": 5 })),
        ("history.list", json!({})),
        (
            "task.update",
            json!({ "uid": 3, "name": "Simulé", "dry_run": true }),
        ),
    ];
    for (tool, params) in reads {
        let sent = execute(&tx, tool, params.clone());
        let relayed = finish(&mut harness, &sent, true);
        let local = ws.call(tool, params);
        match (relayed, local) {
            (Reply::Ok { result, .. }, Ok(local)) => assert_eq!(result, local, "{tool}"),
            (Reply::Err { error, .. }, Err(local)) => {
                let wire = serde_json::to_value(&error).unwrap();
                assert_eq!(wire, serde_json::to_value(&local).unwrap(), "{tool}");
            }
            (relayed, local) => panic!("{tool} : {relayed:?} / {local:?}"),
        }
    }
    assert_eq!(generation(harness.state()), 0, "le dry_run ne change rien");
}

/// H-UI-27 : bandeau avec le focus clavier ; Entrée accepte, Échap
/// refuse ; un seul à la fois, les autres en file.
#[test]
fn h_ui_27_banner_keyboard() {
    let (mut harness, tx) = relay_harness();
    let first = execute(&tx, "task.update", rename(3, "Par Entrée"));
    let second = execute(&tx, "task.update", rename(4, "Par Échap"));
    harness.run();
    let texts = harness.state().texts();
    let accept = texts.get(TextId::ApprovalAccept);
    let reject = texts.get(TextId::ApprovalReject);
    let queued = texts.get_args(TextId::ApprovalQueued, &[("count", Arg::Count(1))]);
    let title = texts.get_args(TextId::ApprovalTitle, &[("name", Arg::Str("task.update".into()))]);
    harness.get_by_label(&queued);
    assert!(harness.query_by_label_contains(&title).is_some());
    assert!(harness.get_by_label(&accept).is_focused());
    harness.key_press_modifiers(Modifiers::NONE, Key::Enter);
    harness.run();
    assert_eq!(first.drain().len(), 2, "pending puis ok");
    assert_eq!(generation(harness.state()), 1);
    // Bandeau suivant, focus redonné à Accepter ; Échap refuse.
    assert!(harness.query_by_label(&queued).is_none());
    assert!(harness.get_by_label(&accept).is_focused());
    harness.key_press_modifiers(Modifiers::NONE, Key::Escape);
    harness.run();
    let replies = second.drain();
    assert_eq!(err_code(replies.last().unwrap()), codes::CONTROL_REJECTED);
    assert_eq!(generation(harness.state()), 1);
    assert!(harness.query_by_label(&reject).is_none(), "bandeau retiré");
    assert_eq!(pending_ids(harness.state()).len(), 0);
}

/// Un lot accepté de l'agent = une seule entrée d'annulation, marquée
/// « agent » dans le menu Édition ; une annulation le défait en entier.
#[test]
fn relay_agent_entry_single_undo() {
    let (mut harness, tx) = relay_harness();
    let before = project_json(harness.state());
    let batch = json!({ "ops": [
        { "op": "task.update", "uid": 3, "name": "A" },
        { "op": "task.update", "uid": 4, "duration": { "text": "3j" } },
    ] });
    let sent = execute(&tx, "batch.apply", batch);
    assert_eq!(ok_value(finish(&mut harness, &sent, true))["committed"], true);
    let history = harness.state().document.as_ref().unwrap().session.history();
    assert_eq!(history.entries.len(), 1);
    assert_eq!(
        (history.entries[0].ops, history.entries[0].source),
        (2, EditSource::Agent)
    );
    // Menu Édition : « Annuler : agent — 2 opérations ».
    let texts = harness.state().texts();
    let ops = texts.get_args(TextId::MenuUndoAgentOps, &[("count", Arg::Count(2))]);
    let label = texts.get_args(TextId::MenuUndoAgent, &[("detail", Arg::Str(ops.into()))]);
    crate::click(&mut harness, &fr(TextId::MenuEdit));
    assert!(harness.query_by_label_contains(&label).is_some(), "{label}");
    harness.key_press_modifiers(Modifiers::NONE, Key::Escape);
    harness.run();
    harness.state_mut().dispatch(Action::Undo);
    harness.run();
    assert_eq!(project_json(harness.state()), before);
}

/// `app.ui_state` et `app.ui` : état d'interface servi aussitôt ; actions
/// d'interface appliquées sans historique ; UID inconnu refusé.
#[test]
fn relay_ui_state_and_ui_actions() {
    let (mut harness, tx) = relay_harness();
    let sent = push(&tx, RelayKind::UiState);
    let state = ok_value(finish(&mut harness, &sent, true));
    assert_eq!(state["view"], "gantt");
    assert_eq!(state["dirty"], false);
    assert_eq!(state["generation"], 0);
    assert_eq!(state["opening"], 1);
    assert_eq!(state["visible_rows"][0], 0);
    assert!(state["ui_state"]["version"].is_number());
    let params = json!({ "select": 5, "zoom": "month", "view": "resource_sheet", "theme": "dark" });
    let sent = execute(&tx, "app.ui", params);
    assert_eq!(ok_value(finish(&mut harness, &sent, true))["applied"], 4);
    harness.run();
    let app = harness.state();
    assert_eq!(app.selection.map(planrs_core::TaskUid::get), Some(5));
    assert_eq!(app.state.zoom, TimeScaleZoom::Month);
    assert_eq!(app.state.view, ViewKind::ResourceSheet);
    assert_eq!(app.state.theme, ThemePref::Dark);
    assert_eq!(generation(app), 0);
    let sent = execute(&tx, "app.ui_state", json!({}));
    let state = ok_value(finish(&mut harness, &sent, true));
    assert_eq!(
        (state["active"].clone(), state["selection"].clone()),
        (json!(5), json!([5]))
    );
    for (params, code) in [
        (json!({ "select": 999 }), codes::TASK_NOT_FOUND),
        (json!({}), codes::PARAM_INVALID),
        (json!({ "nope": 1 }), codes::PARAM_INVALID),
    ] {
        let sent = execute(&tx, "app.ui", params);
        assert_eq!(err_code(&finish(&mut harness, &sent, true)), code);
    }
    let sent = execute(&tx, "app.ui", json!({ "language": "en" }));
    finish(&mut harness, &sent, true);
    harness.run();
    assert_eq!(harness.state().state.language, LanguagePref::En);
    let sent = push(&tx, RelayKind::Screenshot { scale_down: 0 });
    let reply = finish(&mut harness, &sent, true);
    assert_eq!(err_code(&reply), codes::CONTROL_SCREENSHOT_UNAVAILABLE);
}

/// Un appel expiré dans la file (avant d'être servi) est ignoré.
#[test]
fn relay_expired_in_queue_is_skipped() {
    let (mut harness, tx) = relay_harness();
    let sent = execute(&tx, "task.update", rename(3, "Jamais"));
    assert!(sent.state.expire());
    harness.run();
    assert_eq!(sent.drain().len(), 0);
    assert_eq!(pending_ids(harness.state()).len(), 0);
    // Un refus du `dry_run` part aussitôt, sans bandeau.
    let sent = execute(&tx, "task.update", rename(999, "x"));
    let reply = finish(&mut harness, &sent, true);
    assert_eq!(err_code(&reply), codes::TASK_NOT_FOUND);
    // Document remplacé : l'écriture en attente est refusée.
    let sent = execute(&tx, "task.update", rename(3, "Ailleurs"));
    harness.run();
    harness.state_mut().open_demo();
    harness.run();
    let replies = sent.drain();
    assert_eq!(err_code(replies.last().unwrap()), codes::CONTROL_REJECTED);
}

/// P47 (mémoire) : un scénario de 30 appels rejoué par le relais
/// (écritures acceptées) et en local donne le même projet, le même nombre
/// d'entrées d'historique et les mêmes résultats.
#[test]
fn p47_relay_replays_like_local() {
    let (mut harness, tx) = relay_harness();
    let mut ws = Workspace::new(now(), ToolLanguage::Fr);
    ws.set_session(harness.state().document.as_ref().unwrap().session.clone());
    let mut calls: Vec<(&str, Value)> = Vec::new();
    for i in 0..6_u32 {
        let uid = 2 + i;
        calls.push(("task.update", json!({ "uid": uid, "name": format!("Tâche {i}") })));
        calls.push(("task.get", json!({ "uid": uid })));
        calls.push((
            "task.update",
            json!({ "uid": uid, "duration": { "text": format!("{}j", i + 2) } }),
        ));
        calls.push(("project.summary", json!({})));
        calls.push(if i % 2 == 0 {
            ("history.undo", json!({}))
        } else {
            ("analysis.critical_path", json!({}))
        });
    }
    assert_eq!(calls.len(), 30);
    for (tool, params) in calls {
        let sent = execute(&tx, tool, params.clone());
        let relayed = finish(&mut harness, &sent, true);
        let local = ws.call(tool, params);
        match (relayed, local) {
            (Reply::Ok { result, .. }, Ok(local)) => assert_eq!(result, local, "{tool}"),
            (Reply::Err { error, .. }, Err(local)) => assert_eq!(error.code, local.code, "{tool}"),
            (relayed, local) => panic!("{tool} : {relayed:?} / {local:?}"),
        }
    }
    let app = harness.state();
    let session = &app.document.as_ref().unwrap().session;
    let local = ws.session().unwrap();
    assert_eq!(project_json(app), serde_json::to_value(local.project()).unwrap());
    assert_eq!(session.history().entries.len(), local.history().entries.len());
    assert_eq!(session.history().redo.len(), local.history().redo.len());
    assert!(
        session
            .history()
            .entries
            .iter()
            .all(|e| e.source == EditSource::Agent)
    );
}

/// C-CTL-02 et C-CTL-04 de bout en bout : écouteur réel, client du canal
/// dans un fil, application pilotée image par image.
#[cfg(unix)]
#[allow(
    clippy::disallowed_methods,
    reason = "test natif : l'agent tourne dans son fil, comme `planrs mcp`"
)]
mod socket {
    use std::thread;

    use planrs_automation::control::{
        ListenerOptions, RelayClient, RelayError, new_token, spawn_listener_with,
    };

    use super::*;

    /// Dossier temporaire court (chemin de socket).
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let n = next_id();
            let dir = std::env::temp_dir().join(format!("pu-{tag}-{}-{n}", std::process::id()));
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

    /// Application branchée sur un écouteur réel de délai `timeout`.
    fn wired(
        tag: &str,
        timeout: Duration,
    ) -> (
        Harness<'static, App>,
        planrs_automation::control::ListenerHandle,
        TempDir,
    ) {
        let tmp = TempDir::new(tag);
        let mut harness = harness();
        let (tx, rx) = sync_channel(64);
        harness.state_mut().control_inbox(rx);
        open_demo(&mut harness);
        let options = ListenerOptions {
            approval_timeout: timeout,
            reply_grace: Duration::from_millis(200),
        };
        let handle = spawn_listener_with(&tmp.0, new_token().unwrap(), tx, Arc::new(|| {}), options).unwrap();
        (harness, handle, tmp)
    }

    /// Fait tourner l'interface jusqu'à `done` (~10 s au plus).
    fn pump(harness: &mut Harness<'static, App>, mut done: impl FnMut(&App) -> bool) {
        // 2 000 images de 5 ms au plus (~10 s).
        for _ in 0..2_000 {
            if done(harness.state()) {
                return;
            }
            harness.step();
            thread::sleep(Duration::from_millis(5));
        }
        panic!("délai dépassé");
    }

    #[test]
    fn c_ctl_02_pending_then_ok_over_socket() {
        let (mut harness, handle, _tmp) = wired("ok", Duration::from_secs(30));
        let rendezvous = handle.rendezvous(None);
        let agent = thread::spawn(move || {
            let mut client = RelayClient::connect(&rendezvous, Duration::from_secs(5)).unwrap();
            let write = client.call("task.update", rename(3, "Par la socket"), Duration::from_secs(30));
            let read = client.call("history.list", json!({}), Duration::from_secs(30));
            (write, read)
        });
        pump(&mut harness, |app| {
            app.relay().is_some_and(|relay| !relay.pending().is_empty())
        });
        let id = pending_ids(harness.state())[0];
        harness.state_mut().dispatch(Action::Approve(id));
        pump(&mut harness, |_| agent.is_finished());
        let (write, read) = agent.join().unwrap();
        assert_eq!(write.unwrap()["committed"], true);
        assert_eq!(read.unwrap()["entries"][0]["source"], "agent");
        assert_eq!(generation(harness.state()), 1);
    }

    #[test]
    fn c_ctl_04_timeout_over_socket() {
        let (mut harness, handle, _tmp) = wired("late", Duration::from_millis(400));
        let rendezvous = handle.rendezvous(None);
        let agent = thread::spawn(move || {
            let mut client = RelayClient::connect(&rendezvous, Duration::from_secs(5)).unwrap();
            client.call("task.update", rename(3, "Trop tard"), Duration::from_secs(30))
        });
        pump(&mut harness, |app| {
            app.relay().is_some_and(|relay| !relay.pending().is_empty())
        });
        let id = pending_ids(harness.state())[0];
        pump(&mut harness, |_| agent.is_finished());
        match agent.join().unwrap() {
            Err(RelayError::App(error)) => assert_eq!(error["code"], codes::CONTROL_APPROVAL_TIMEOUT),
            other => panic!("délai attendu : {other:?}"),
        }
        // Bandeau retiré ; Accepter ensuite n'applique rien.
        pump(&mut harness, |app| {
            app.relay().is_some_and(|relay| relay.pending().is_empty())
        });
        harness.state_mut().dispatch(Action::Approve(id));
        harness.run();
        assert_eq!(generation(harness.state()), 0);
    }
}
