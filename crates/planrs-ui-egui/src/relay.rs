//! Relais des appels d'un agent côté application (J7, ADR-0015 D9 à D11 ;
//! lot L3) : file des [`RelayCall`] vidée par le réducteur en début
//! d'image, exécution par [`call_with`] sur la session du document,
//! validation de toute écriture par le bandeau (H-UI-27), `app.ui_state`
//! et `app.ui`.
//!
//! Déroulé d'une écriture (C-CTL-02 à C-CTL-04, C-CTL-07) :
//! 1. `dry_run` du même appel ([`call_with`]) : refus ⇒ erreur rendue
//!    aussitôt ; sinon `pending_approval`, et le résumé du diff
//!    ([`ApprovalSummary`]) s'affiche dans le bandeau ;
//! 2. Accepter : appel expiré ([`CallState::claim`] faux) ⇒ rien ; document
//!    remplacé ⇒ `control.rejected` ; génération changée depuis l'aperçu ⇒
//!    nouvel aperçu, rien d'appliqué ; sinon l'appel est exécuté avec
//!    l'origine `Agent` et le « maintenant » de la demande, puis `ok` ;
//! 3. Refuser ⇒ `control.rejected` ; projet, historique, génération et
//!    drapeau modifié intacts ;
//! 4. délai de l'écouteur ⇒ `Cancel(id)` (ou appel vu expiré) : bandeau
//!    retiré.
//!
//! Lectures (genres `Read`, `Analysis`, `history.list`) servies aussitôt,
//! même JSON que `Workspace::call` (C-CTL-09). `project.open`,
//! `project.new`, `project.import_csv` : `control.not_relayable`
//! (C-CTL-05). Le web n'a pas de canal : ce module n'y est pas compilé.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender};

use planrs_automation::command::params::{ThemeJson, UiActionParams, UiLanguageJson, ViewJson, ZoomJson};
use planrs_automation::control::{CallState, RelayCall, RelayKind, Reply};
use planrs_automation::error::codes;
use planrs_automation::registry::{Kind, lookup};
use planrs_automation::workspace::relayable;
use planrs_automation::{CallEnv, EditSource, Patch, ToolError, call_with};
use planrs_core::{Moment, TaskUid};
use serde_json::{Map, Value, json};

use crate::action::{Action, UiOp};
use crate::app::{App, DocOrigin};
use crate::geom;
use crate::state::{LanguagePref, ThemePref, ViewKind};
use crate::timescale::TimeScaleZoom;
use crate::view::{self, ApprovalBanner, ApprovalSummary};

/// Écritures qui demandent une validation sans être du genre `Write`.
const APPROVED_TOOLS: [&str; 4] = [
    "history.redo",
    "history.undo",
    "project.export_mspdi",
    "project.save",
];

/// Écriture relayée en attente de la réponse de l'utilisateur.
#[derive(Debug)]
pub struct PendingApproval {
    /// Identifiant de la requête de l'agent.
    pub id: u64,
    /// Outil demandé.
    pub tool: String,
    /// Paramètres de l'appel.
    pub params: Value,
    /// Résumé du `dry_run` (`None` : appel sans aperçu).
    pub preview: Option<ApprovalSummary>,
    /// Génération de la session à l'aperçu.
    pub generation: u64,
    /// Numéro d'ouverture du document visé.
    pub opening: u64,
    /// « Maintenant » de la demande, repris à l'acceptation (règle 5).
    pub now: Moment,
    /// État partagé avec l'écouteur.
    state: Arc<CallState>,
    /// Réponse finale.
    reply: SyncSender<Reply>,
}

impl PendingApproval {
    /// Chemin visé par un enregistrement ou un export.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.params.get("path").and_then(Value::as_str)
    }
}

/// Relais de l'application : file des appels et écritures en attente.
#[derive(Debug)]
pub struct Relay {
    /// File remplie par l'écouteur.
    inbox: Receiver<RelayCall>,
    /// Écritures en attente, la première au bandeau.
    pending: VecDeque<PendingApproval>,
    /// Bandeau qui a déjà reçu le focus clavier.
    focused: Option<u64>,
    /// Dossier des rendez-vous (garde ADR-0002 d'un enregistrement relayé) ;
    /// `None` : celui de la plateforme.
    control_dir: Option<PathBuf>,
}

impl Relay {
    /// Écritures en attente, la première au bandeau.
    #[must_use]
    pub const fn pending(&self) -> &VecDeque<PendingApproval> {
        &self.pending
    }

    /// Retire l'écriture `id` de la file.
    fn take(&mut self, id: u64) -> Option<PendingApproval> {
        let pos = self.pending.iter().position(|p| p.id == id)?;
        self.pending.remove(pos)
    }
}

/// Réponse d'erreur d'outil.
fn tool_reply(id: u64, error: &ToolError) -> Reply {
    Reply::tool_error(id, error)
}

/// Envoie une réponse ; un écouteur parti n'est pas une erreur de
/// l'application.
fn send(reply: &SyncSender<Reply>, value: Reply) {
    if reply.try_send(value).is_err() {
        log::debug!("réponse du relais non remise : écouteur parti");
    }
}

/// Résumé d'un `dry_run` (JSON d'`ApplyOutcome`), sans calcul.
#[must_use]
pub fn summary_of(outcome: &Value) -> ApprovalSummary {
    let at = |path: &str| outcome.pointer(path);
    let len = |path: &str| at(path).and_then(Value::as_array).map_or(0, Vec::len);
    ApprovalSummary {
        changed_tasks: at("/diff/changed_total")
            .and_then(Value::as_u64)
            .map_or(0, |n| u32::try_from(n).unwrap_or(u32::MAX)),
        finish_delta: at("/diff/finish_delta/minutes").and_then(Value::as_i64),
        finish_delta_text: at("/diff/finish_delta/text")
            .and_then(Value::as_str)
            .map(str::to_owned),
        critical_changed: len("/diff/critical_entered") > 0 || len("/diff/critical_left") > 0,
        warnings: u32::try_from(len("/warnings")).unwrap_or(u32::MAX),
    }
}

/// Erreur d'outil simple.
fn error(code: &'static str, message: &str) -> ToolError {
    ToolError::simple(code, message.to_owned())
}

/// Vrai si l'appel demande une validation : écriture (hors `dry_run`),
/// enregistrement, export, annuler, rétablir.
fn needs_approval(tool: &str, params: &Value) -> bool {
    let write = lookup(tool).is_some_and(|spec| spec.kind == Kind::Write) || APPROVED_TOOLS.contains(&tool);
    write && params.get("dry_run").and_then(Value::as_bool) != Some(true)
}

/// Paramètres d'un `dry_run` (objet, `dry_run: true`), si l'appel en a un.
fn dry_params(tool: &str, params: &Value) -> Option<Value> {
    let batch = tool == "batch.apply" || lookup(tool).is_some_and(|spec| spec.kind == Kind::Write);
    if !batch {
        return None;
    }
    let mut map = match params {
        Value::Null => Map::new(),
        Value::Object(map) => map.clone(),
        // Mal formé : le `dry_run` le refusera tel quel.
        other => return Some(other.clone()),
    };
    map.insert("dry_run".to_owned(), Value::Bool(true));
    Some(Value::Object(map))
}

/// Actions d'interface d'`app.ui`, dans l'ordre du schéma.
///
/// # Errors
///
/// `param.invalid` (paramètres mal formés, aucun champ).
pub fn ui_ops(params: Value) -> Result<Vec<Action>, ToolError> {
    let p: UiActionParams = serde_json::from_value(params).map_err(|e| {
        ToolError::simple(
            codes::PARAM_INVALID,
            format!("paramètres de app.ui invalides : {e}"),
        )
    })?;
    let mut ops = Vec::new();
    match p.select {
        Patch::Keep => {}
        Patch::Set(uid) => ops.push(Action::Ui(UiOp::Select(Some(uid)))),
        Patch::Clear => ops.push(Action::Ui(UiOp::Select(None))),
    }
    if let Some(uid) = p.scroll_to {
        ops.push(Action::Ui(UiOp::ScrollTo(uid)));
    }
    if let Some(zoom) = p.zoom {
        ops.push(match zoom {
            ZoomJson::Auto => Action::FitZoom,
            ZoomJson::Day => Action::Ui(UiOp::Zoom(TimeScaleZoom::Day)),
            ZoomJson::Week => Action::Ui(UiOp::Zoom(TimeScaleZoom::Week)),
            ZoomJson::Month => Action::Ui(UiOp::Zoom(TimeScaleZoom::Month)),
            ZoomJson::Quarter => Action::Ui(UiOp::Zoom(TimeScaleZoom::Quarter)),
        });
    }
    if let Some(v) = p.view {
        ops.push(Action::Ui(UiOp::View(match v {
            ViewJson::Gantt => ViewKind::Gantt,
            ViewJson::ResourceSheet => ViewKind::ResourceSheet,
            ViewJson::ResourceUsage => ViewKind::ResourceUsage,
        })));
    }
    if let Some(t) = p.theme {
        ops.push(Action::Ui(UiOp::Theme(match t {
            ThemeJson::System => ThemePref::System,
            ThemeJson::Light => ThemePref::Light,
            ThemeJson::Dark => ThemePref::Dark,
        })));
    }
    if let Some(l) = p.language {
        ops.push(Action::Ui(UiOp::Language(match l {
            UiLanguageJson::System => LanguagePref::System,
            UiLanguageJson::Fr => LanguagePref::Fr,
            UiLanguageJson::En => LanguagePref::En,
        })));
    }
    if ops.is_empty() {
        return Err(ToolError::simple(
            codes::PARAM_INVALID,
            "app.ui : au moins un champ attendu".to_owned(),
        ));
    }
    Ok(ops)
}

impl App {
    /// Branche la file des appels relayés (desktop : écouteur du canal).
    pub fn control_inbox(&mut self, inbox: Receiver<RelayCall>) {
        self.relay = Some(Relay {
            inbox,
            pending: VecDeque::new(),
            focused: None,
            control_dir: None,
        });
    }

    /// Dossier des rendez-vous lu par la garde d'un enregistrement relayé
    /// (tests ; défaut : celui de la plateforme).
    pub fn set_control_dir(&mut self, dir: PathBuf) {
        if let Some(relay) = &mut self.relay {
            relay.control_dir = Some(dir);
        }
    }

    /// Relais branché, avec ses écritures en attente.
    #[must_use]
    pub const fn relay(&self) -> Option<&Relay> {
        self.relay.as_ref()
    }

    /// Environnement d'un appel relayé (« maintenant » lu maintenant).
    fn call_env(&self, now: Moment, source: EditSource) -> CallEnv {
        CallEnv {
            source,
            ..CallEnv::new(now, self.text_language())
        }
    }

    /// Vide la file des appels relayés (début d'image) : sert les
    /// lectures, met les écritures en attente, retire les appels expirés
    /// et ceux d'un document remplacé.
    pub(crate) fn drain_relay(&mut self) {
        let Some(relay) = &self.relay else {
            return;
        };
        let calls: Vec<RelayCall> = relay.inbox.try_iter().collect();
        for call in calls {
            self.serve_call(call);
        }
        let opening = self.document.as_ref().map(|document| document.opening);
        let Some(relay) = &mut self.relay else {
            return;
        };
        let mut kept = VecDeque::new();
        for pending in relay.pending.drain(..) {
            if !pending.state.is_pending() {
                continue;
            }
            if Some(pending.opening) == opening {
                kept.push_back(pending);
            } else if pending.state.claim() {
                let e = error(
                    codes::CONTROL_REJECTED,
                    "document fermé ou remplacé : rien n'a été appliqué",
                );
                send(&pending.reply, tool_reply(pending.id, &e));
            }
        }
        relay.pending = kept;
    }

    /// Sert un appel relayé.
    fn serve_call(&mut self, call: RelayCall) {
        let RelayCall {
            id,
            kind,
            state,
            reply,
        } = call;
        if let RelayKind::Cancel(target) = kind {
            if let Some(relay) = &mut self.relay {
                drop(relay.take(target));
            }
            return;
        }
        if !state.is_pending() {
            return;
        }
        match kind {
            RelayKind::UiState => self.reply_now(&state, &reply, id, Ok(self.ui_state_json())),
            RelayKind::Screenshot { .. } => self.reply_screenshot(&state, &reply, id),
            RelayKind::Execute { tool, params } => self.serve_execute(id, tool, params, state, reply),
            RelayKind::Cancel(_) => {}
        }
    }

    /// Réponse immédiate d'un appel pris maintenant.
    #[allow(clippy::unused_self, reason = "méthode du relais, comme les autres réponses")]
    fn reply_now(
        &self,
        state: &CallState,
        reply: &SyncSender<Reply>,
        id: u64,
        result: Result<Value, ToolError>,
    ) {
        if !state.claim() {
            return;
        }
        send(
            reply,
            match result {
                Ok(result) => Reply::Ok { id, result },
                Err(e) => tool_reply(id, &e),
            },
        );
    }

    /// Capture de la fenêtre : lot L14 (jusque-là indisponible).
    fn reply_screenshot(&self, state: &CallState, reply: &SyncSender<Reply>, id: u64) {
        let e = error(
            codes::CONTROL_SCREENSHOT_UNAVAILABLE,
            "capture de la fenêtre non disponible dans cette version",
        );
        self.reply_now(state, reply, id, Err(e));
    }

    /// Sert `execute { tool, params }`.
    fn serve_execute(
        &mut self,
        id: u64,
        tool: String,
        params: Value,
        state: Arc<CallState>,
        reply: SyncSender<Reply>,
    ) {
        match tool.as_str() {
            "app.ui_state" => return self.reply_now(&state, &reply, id, Ok(self.ui_state_json())),
            "app.screenshot" => return self.reply_screenshot(&state, &reply, id),
            "app.ui" => {
                let result = self.queue_ui(params);
                return self.reply_now(&state, &reply, id, result);
            }
            _ => {}
        }
        let now = (self.env.now)();
        let env = self.call_env(now, EditSource::User);
        if let Err(e) = relayable(&tool, env.lang) {
            // Refus du routeur lui-même (id inconnu, ouverture), avec ou
            // sans document : même erreur que `call_with`.
            return self.reply_now(&state, &reply, id, Err(e));
        }
        let Some(document) = &mut self.document else {
            let e = error(codes::SESSION_NONE, "aucun projet ouvert dans l'application");
            return self.reply_now(&state, &reply, id, Err(e));
        };
        if !needs_approval(&tool, &params) {
            if !state.claim() {
                return;
            }
            let result = call_with(&mut document.session, &env, &tool, params);
            send(
                &reply,
                match result {
                    Ok(result) => Reply::Ok { id, result },
                    Err(e) => tool_reply(id, &e),
                },
            );
            return;
        }
        if let Err(e) = self.guard_save_path(&tool, &params) {
            return self.reply_now(&state, &reply, id, Err(e));
        }
        let Some(document) = &mut self.document else {
            return;
        };
        let preview = match dry_params(&tool, &params) {
            None => None,
            Some(dry) => match call_with(&mut document.session, &env, &tool, dry) {
                Ok(outcome) => Some(summary_of(&outcome)),
                Err(e) => return self.reply_now(&state, &reply, id, Err(e)),
            },
        };
        let pending = PendingApproval {
            id,
            tool,
            params,
            preview,
            generation: document.generation(),
            opening: document.opening,
            now,
            state,
            reply,
        };
        send(&pending.reply, Reply::PendingApproval { id });
        if let Some(relay) = &mut self.relay {
            relay.pending.push_back(pending);
        }
    }

    /// Garde ADR-0002 d'un enregistrement relayé vers un chemin choisi par
    /// l'agent : refus si une autre application tient ce fichier.
    fn guard_save_path(&self, tool: &str, params: &Value) -> Result<(), ToolError> {
        if !matches!(tool, "project.save" | "project.export_mspdi") {
            return Ok(());
        }
        let Some(path) = params.get("path").and_then(Value::as_str) else {
            return Ok(());
        };
        let path = std::path::Path::new(path);
        let dir = self
            .relay
            .as_ref()
            .and_then(|relay| relay.control_dir.clone())
            .or_else(planrs_automation::control::state_dir);
        let held = dir
            .and_then(|dir| planrs_automation::control::find_live_for_in(&dir, path))
            .is_some_and(|r| r.pid != std::process::id());
        if held {
            return Err(ToolError::simple(
                codes::FILE_OPEN_IN_APP,
                format!("fichier tenu par une autre application : {}", path.display()),
            )
            .with_field("path"));
        }
        Ok(())
    }

    /// `app.ui` : actions mises en file (appliquées en fin d'image).
    fn queue_ui(&mut self, params: Value) -> Result<Value, ToolError> {
        let ops = ui_ops(params)?;
        for op in &ops {
            let uid = match op {
                Action::Ui(UiOp::Select(Some(uid)) | UiOp::ScrollTo(uid)) => Some(*uid),
                _ => None,
            };
            if let Some(uid) = uid {
                let known = self
                    .rows
                    .as_ref()
                    .is_some_and(|rows| rows.index_of(TaskUid::new(uid)).is_some());
                if !known {
                    return Err(ToolError::simple(
                        codes::TASK_NOT_FOUND,
                        format!("tâche {uid} absente de la vue"),
                    ));
                }
            }
        }
        let applied = ops.len();
        self.actions.extend(ops);
        Ok(json!({ "applied": applied }))
    }

    /// `app.ui_state` : préférences, sélection, vue, lignes visibles,
    /// document modifié, génération, numéro d'ouverture.
    #[must_use]
    pub fn ui_state_json(&self) -> Value {
        let ui_state = self
            .state
            .to_json()
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .unwrap_or(Value::Null);
        let mut selection: Vec<u32> = self.selected.click_order().iter().map(|uid| uid.get()).collect();
        selection.sort_unstable();
        let visible = self.rows.as_ref().map(|rows| {
            let top = self.view.last_scroll_y;
            let range = geom::visible_rows(top, top + self.view.last_body_height, view::ROW_H, rows.len(), 0);
            json!([range.start, range.end.saturating_sub(1).max(range.start)])
        });
        json!({
            "ui_state": ui_state,
            "selection": selection,
            "active": self.selection.map(TaskUid::get),
            "view": self.state.view,
            "visible_rows": visible,
            "dirty": self.document.as_ref().is_some_and(crate::app::OpenDocument::is_dirty),
            "generation": self.document.as_ref().map(crate::app::OpenDocument::generation),
            "opening": self.document.as_ref().map(|document| document.opening),
        })
    }

    /// Accepter (C-CTL-02, C-CTL-07) : voir l'en-tête du module.
    pub(crate) fn approve(&mut self, id: u64) {
        let Some(relay) = &mut self.relay else {
            return;
        };
        let Some(mut pending) = relay.take(id) else {
            return;
        };
        if !pending.state.is_pending() {
            return;
        }
        let lang = self.text_language();
        let Some(document) = &mut self.document else {
            return self.reject_pending(&pending, "aucun document : rien n'a été appliqué");
        };
        if document.opening != pending.opening {
            return self.reject_pending(&pending, "document fermé ou remplacé : rien n'a été appliqué");
        }
        if document.generation() != pending.generation {
            // Aperçu périmé : nouvel aperçu, rien d'appliqué.
            let env = CallEnv::new(pending.now, lang);
            if let Some(dry) = dry_params(&pending.tool, &pending.params) {
                match call_with(&mut document.session, &env, &pending.tool, dry) {
                    Ok(outcome) => pending.preview = Some(summary_of(&outcome)),
                    Err(e) => {
                        if pending.state.claim() {
                            send(&pending.reply, tool_reply(pending.id, &e));
                        }
                        return;
                    }
                }
            }
            pending.generation = document.generation();
            if let Some(relay) = &mut self.relay {
                relay.pending.push_front(pending);
                relay.focused = None;
            }
            return;
        }
        if !pending.state.claim() {
            return;
        }
        let env = CallEnv {
            source: EditSource::Agent,
            ..CallEnv::new(pending.now, lang)
        };
        let before = document.generation();
        let result = call_with(&mut document.session, &env, &pending.tool, pending.params.clone());
        let changed = document.generation() != before;
        let origin = document.session.origin().clone();
        if origin.path != document.origin.path {
            document.origin = DocOrigin {
                name: origin.name,
                path: origin.path,
                demo: false,
            };
        }
        send(
            &pending.reply,
            match result {
                Ok(result) => Reply::Ok { id, result },
                Err(e) => tool_reply(id, &e),
            },
        );
        if changed {
            self.refresh();
        } else {
            self.notify_observer();
        }
        self.after_banner();
    }

    /// Refuser (C-CTL-03) : rien n'est appliqué.
    pub(crate) fn reject(&mut self, id: u64) {
        let Some(relay) = &mut self.relay else {
            return;
        };
        let Some(pending) = relay.take(id) else {
            return;
        };
        self.reject_pending(&pending, "refusé par l'utilisateur : rien n'a été appliqué");
    }

    /// Répond `control.rejected` à une écriture en attente.
    fn reject_pending(&mut self, pending: &PendingApproval, message: &str) {
        if pending.state.claim() {
            send(
                &pending.reply,
                tool_reply(pending.id, &error(codes::CONTROL_REJECTED, message)),
            );
        }
        self.after_banner();
    }

    /// Plus aucun bandeau : le focus clavier revient à la table.
    fn after_banner(&mut self) {
        let empty = self.relay.as_ref().is_none_or(|relay| relay.pending.is_empty());
        if empty && self.document.is_some() {
            self.view.focus_table_pending = true;
        }
    }

    /// Bandeau à dessiner pour cette image (le focus est demandé une fois
    /// par bandeau).
    pub(crate) fn approval_banner(&mut self) -> Option<(u64, bool)> {
        let relay = self.relay.as_mut()?;
        let id = relay.pending.front()?.id;
        let take_focus = relay.focused != Some(id);
        relay.focused = Some(id);
        Some((id, take_focus))
    }
}

/// Bandeau de l'écriture `id` en tête de `relay`.
#[must_use]
pub fn banner_for(relay: &Relay, take_focus: bool) -> Option<ApprovalBanner<'_>> {
    let front = relay.pending.front()?;
    Some(ApprovalBanner {
        id: front.id,
        tool: &front.tool,
        summary: front.preview.as_ref(),
        path: front.path(),
        queued: relay.pending.len().saturating_sub(1),
        take_focus,
    })
}
