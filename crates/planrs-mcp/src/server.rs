//! `PlanrsServer` et son `ServerHandler` (lot L8).
//!
//! Chaque appel d'outil passe par [`PlanrsServer::dispatch`] : nom MCP →
//! entrée du registre → garde ADR-0002 → `Workspace::call`. Une erreur
//! métier est un résultat d'outil en erreur structurée
//! (`CallToolResult::structured_error`), jamais une erreur JSON-RPC
//! (C-MCP-05) ; seul un nom inconnu est refusé en `invalid_params`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use planrs_automation::error::codes;
use planrs_automation::registry::by_mcp_name;
use planrs_automation::text::{Language, TextContext, format_date};
use planrs_automation::{ToolError, Workspace, view};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, JsonObject,
    ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::{MaybeSendFuture, RequestContext};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};
use serde_json::{Value, json};

use crate::journal_file::JournalFile;
use crate::{McpServeError, ServeOptions, tools};

/// Serveur MCP : un espace de travail partagé, un journal facultatif.
#[derive(Clone, Debug)]
pub struct PlanrsServer {
    ws: Arc<Mutex<Workspace>>,
    journal: Option<Arc<Mutex<JournalFile>>>,
    lang: Language,
    /// Dossier des rendez-vous du canal de contrôle (garde ADR-0002) ;
    /// `None` : garde inactive (`--headless`, ou aucun dossier d'état).
    control_dir: Option<PathBuf>,
}

/// Texte selon la langue.
const fn tr(lang: Language, fr: &'static str, en: &'static str) -> &'static str {
    match lang {
        Language::Fr => fr,
        Language::En => en,
    }
}

impl PlanrsServer {
    /// Serveur sans projet ouvert : espace de travail, journal (ouvert ici),
    /// garde sur le dossier d'état de la plateforme sauf
    /// `ignore_app_guard`. Le projet `opts.open` n'est pas ouvert ici
    /// (voir [`Self::open_initial`]).
    ///
    /// # Errors
    ///
    /// [`McpServeError::Io`] si le journal ne s'ouvre pas.
    pub fn new(opts: &ServeOptions) -> Result<Self, McpServeError> {
        let journal = match &opts.journal {
            None => None,
            Some(p) => Some(Arc::new(Mutex::new(
                JournalFile::open(p).map_err(McpServeError::Io)?,
            ))),
        };
        let control_dir = if opts.ignore_app_guard {
            None
        } else {
            planrs_automation::control::state_dir()
        };
        Ok(Self {
            ws: Arc::new(Mutex::new(Workspace::new(opts.now, opts.lang))),
            journal,
            lang: opts.lang,
            control_dir,
        })
    }

    /// Remplace le dossier des rendez-vous de la garde (`None` : garde
    /// inactive). Tests et installations portables.
    #[must_use]
    pub fn with_control_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.control_dir = dir;
        self
    }

    /// Ouvre `path` au démarrage (`--open`), garde comprise. Hors journal.
    ///
    /// # Errors
    ///
    /// [`McpServeError::Io`] avec le code et le message de l'outil
    /// (`file.open_in_app`, `file.read`, MSPDI illisible…).
    pub fn open_initial(&self, path: &Path) -> Result<(), McpServeError> {
        let params = json!({ "path": path.to_string_lossy() });
        let res = self.with_workspace(|ws| {
            self.guard("project.open", &params, ws)?;
            ws.call("project.open", params.clone())
        });
        res.map(drop)
            .map_err(|e| McpServeError::Io(format!("{} : {}", e.code, e.message)))
    }

    /// Exécute un appel d'outil (cœur de `call_tool`, synchrone).
    ///
    /// # Errors
    ///
    /// `invalid_params` si `name` n'est pas un outil du registre ; toute
    /// autre erreur est un résultat en erreur structurée.
    pub fn dispatch(&self, name: &str, arguments: Option<JsonObject>) -> Result<CallToolResult, McpError> {
        let Some(spec) = by_mcp_name(name) else {
            return Err(McpError::invalid_params(format!("unknown tool: {name}"), None));
        };
        let params = arguments.map_or(Value::Null, Value::Object);
        let lang = self.lang;
        let Ok(mut ws) = self.ws.lock() else {
            return Ok(error_result(&poisoned(lang)));
        };
        // Verrou du journal pris sous celui de l'espace : l'ordre des `seq`
        // est l'ordre d'exécution.
        let mut journal = self.journal.as_ref().and_then(|j| {
            if let Ok(g) = j.lock() {
                Some(g)
            } else {
                eprintln!("planrs mcp : journal inutilisable (verrou empoisonné), appel non journalisé");
                None
            }
        });
        let seq = journal.as_mut().map(|j| j.begin(name, &params));
        let res = self
            .guard(spec.id, &params, &ws)
            .and_then(|()| ws.call(spec.id, params));
        let result = match res {
            Ok(value) if spec.id == "render.gantt" => render_result(value, &ws, lang),
            Ok(value) => CallToolResult::structured(value),
            Err(e) => error_result(&e),
        };
        if let (Some(j), Some(seq)) = (journal.as_mut(), seq) {
            let logged = match (&result.structured_content, result.is_error) {
                (Some(v), Some(true)) => json!({ "error": v }),
                (Some(v), _) => json!({ "ok": v }),
                (None, _) => json!({ "ok": null }),
            };
            j.finish(seq, name, logged);
        }
        Ok(result)
    }

    /// Exécute `f` sur l'espace de travail ; verrou empoisonné :
    /// `session.poisoned`.
    fn with_workspace<T>(
        &self,
        f: impl FnOnce(&mut Workspace) -> Result<T, ToolError>,
    ) -> Result<T, ToolError> {
        match self.ws.lock() {
            Ok(mut ws) => f(&mut ws),
            Err(_) => Err(poisoned(self.lang)),
        }
    }

    /// Garde ADR-0002 : refuse d'ouvrir ou d'écrire un fichier tenu par une
    /// application vivante (`file.open_in_app`).
    fn guard(&self, id: &str, params: &Value, ws: &Workspace) -> Result<(), ToolError> {
        let Some(dir) = &self.control_dir else {
            return Ok(());
        };
        let given = params.get("path").and_then(Value::as_str).map(PathBuf::from);
        let target = match id {
            "project.open" | "project.export_mspdi" => given,
            "project.save" => given.or_else(|| {
                ws.session()
                    .and_then(|s| s.origin().path.clone())
                    .map(PathBuf::from)
            }),
            _ => None,
        };
        let Some(target) = target else {
            return Ok(());
        };
        if planrs_automation::control::find_live_for_in(dir, &target).is_some() {
            return Err(ToolError::simple(
                codes::FILE_OPEN_IN_APP,
                tr(
                    self.lang,
                    "Le fichier est ouvert dans planrs ; fermez-le ou relancez avec --headless",
                    "The file is open in planrs; close it or restart with --headless",
                )
                .to_owned(),
            )
            .with_field("path"));
        }
        Ok(())
    }

    /// Consignes du serveur pour l'agent.
    const fn instructions(&self) -> &'static str {
        tr(
            self.lang,
            "planrs : moteur de planification compatible MS Project. Règles : les tâches sont désignées par \
             leur UID stable (jamais par leur ID de ligne) ; toute date, durée ou marge vient du moteur, ne \
             jamais en calculer ni en estimer une soi-même ; les outils d'écriture acceptent dry_run:true pour \
             voir le diff du planning sans rien modifier ; dans apply_batch, nommer une tâche créée par \
             \"as\":\"$x\" et la désigner par \"$x\" dans les opérations suivantes. Commencer par new_project, \
             open_project ou import_csv.",
            "planrs: an MS Project-compatible scheduling engine. Rules: tasks are designated by their stable \
             UID (never by their row ID); every date, duration and slack comes from the engine, never compute \
             or estimate one yourself; write tools accept dry_run:true to preview the schedule diff without \
             changing anything; in apply_batch, name a created task with \"as\":\"$x\" and refer to it as \
             \"$x\" in later operations. Start with new_project, open_project or import_csv.",
        )
    }
}

/// Erreur `session.poisoned` (verrou empoisonné par une panique).
fn poisoned(lang: Language) -> ToolError {
    ToolError::simple(
        codes::SESSION_POISONED,
        tr(
            lang,
            "session inutilisable après une erreur interne : relancer le serveur",
            "session unusable after an internal error: restart the server",
        )
        .to_owned(),
    )
}

/// Résultat en erreur structurée.
fn error_result(e: &ToolError) -> CallToolResult {
    let value = serde_json::to_value(e).unwrap_or_else(|_| json!({ "code": e.code, "message": e.message }));
    CallToolResult::structured_error(value)
}

/// Résultat de `render_gantt` : résumé texte puis SVG en contenu texte,
/// rapport du rendu en contenu structuré (sans le SVG).
fn render_result(value: Value, ws: &Workspace, lang: Language) -> CallToolResult {
    let Value::Object(mut report) = value else {
        return CallToolResult::structured(value);
    };
    let svg = match report.remove("svg") {
        Some(Value::String(s)) => s,
        _ => String::new(),
    };
    let mut content = Vec::with_capacity(2);
    if let Some(s) = ws.session() {
        let summary = view::summary(s);
        let finish = format_date(
            s.schedule().project_finish,
            &TextContext::for_project(s.project(), lang),
        );
        let hidden = report.get("hidden").and_then(Value::as_u64).unwrap_or(0);
        let mut text = match lang {
            Language::Fr => format!(
                "{} tâches, {} critiques, fin {finish}",
                summary.tasks, summary.critical
            ),
            Language::En => format!(
                "{} tasks, {} critical, finish {finish}",
                summary.tasks, summary.critical
            ),
        };
        if hidden > 0 {
            let more = match lang {
                Language::Fr => format!(" ({hidden} lignes non affichées)"),
                Language::En => format!(" ({hidden} rows not shown)"),
            };
            text.push_str(&more);
        }
        content.push(ContentBlock::text(text));
    }
    content.push(ContentBlock::text(svg));
    let mut result = CallToolResult::success(content);
    result.structured_content = Some(Value::Object(report));
    result
}

impl ServerHandler for PlanrsServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(self.instructions())
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + MaybeSendFuture + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(tools::listed(self.lang))))
    }

    // Appel synchrone : `Workspace::call` ne fait aucune attente.
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, McpError>> + MaybeSendFuture + '_ {
        std::future::ready(self.dispatch(&request.name, request.arguments).map(Into::into))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        by_mcp_name(name)
            .filter(|s| s.is_listed())
            .map(|s| tools::tool_for(s, self.lang))
    }
}
