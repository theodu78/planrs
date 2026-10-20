//! Couche fine au-dessus de [`Session`] pour la CLI, le MCP et le canal :
//! session facultative, « maintenant », langue, racine des chemins
//! (ADR-0012 D1, lot L6).
//!
//! [`Workspace::call`] est le routeur unique : un id du registre, des
//! paramètres JSON (forme du schéma de la commande, plus `dry_run` et
//! `detail` pour une écriture), une valeur JSON ([`crate::view`]) ou une
//! [`ToolError`]. Une écriture unitaire est un lot d'une commande ; un
//! appel d'écriture réussi hors `dry_run` crée une entrée d'historique
//! (C-MCP-04). Les fichiers passent par [`crate::files`] (absent en wasm :
//! les commandes de fichiers y sont refusées `file.read`/`file.write`).

use std::path::PathBuf;

use planrs_core::{Diagnostic, Moment, Subject, TaskUid};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::command::params::{
    ApplyBatchParams, ExplainParams, ExportMspdiParams, ImportCsvParams, NewProjectParams, NoParams,
    OpenProjectParams, RenderGanttParams, ResourceListParams, SaveProjectParams, TaskGetParams,
    TaskListParams, UsageParams, ZoomJson,
};
use crate::command::{AnchorSpec, Batch, Command, parse_params};
use crate::error::{CommandError, CommandErrorKind, HistoryError, codes};
use crate::outcome::ApplyOutcome;
use crate::registry::{Availability, Kind, lookup};
use crate::session::{ApplyMode, ApplyOptions, NewProject, Session};
use crate::text::Language;
use crate::view::{self, DiagView, MAX_VIEW_DIAGNOSTICS, ViewCtx};

/// Taille maximale d'un fichier MSPDI lu (256 Mio, borne du lecteur).
pub const MAX_MSPDI_BYTES: u64 = 256 * 1024 * 1024;
/// Taille maximale d'un fichier CSV lu (64 Mio, borne de l'import).
pub const MAX_CSV_BYTES: u64 = 64 * 1024 * 1024;
/// Lignes d'un rendu Gantt au plus (`render.gantt`).
pub const MAX_RENDER_ROWS: u32 = 200;
/// Largeur visée par l'échelle automatique d'un rendu.
const AUTO_WIDTH_PX: u32 = 1_200;

/// Espace de travail : au plus une session ouverte.
#[derive(Debug)]
pub struct Workspace {
    session: Option<Session>,
    now: Moment,
    lang: Language,
    root: Option<PathBuf>,
}

/// Erreur d'outil, sérialisée telle quelle (`structured_error` du MCP).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolError {
    /// Code stable.
    pub code: &'static str,
    /// Message (langue de l'espace de travail).
    pub message: String,
    /// Index de l'opération fautive.
    pub op_index: Option<usize>,
    /// Champ fautif.
    pub field: Option<String>,
    /// Élément concerné.
    pub subject: Option<Subject>,
    /// Diagnostics (refus R-160).
    pub diagnostics: Vec<Diagnostic>,
}

impl ToolError {
    /// Erreur d'outil d'une commande refusée, message dans `lang`.
    #[must_use]
    pub fn from_command(e: &CommandError, lang: Language) -> Self {
        let diagnostics = match &e.kind {
            CommandErrorKind::WouldCreateErrors(ds) => {
                ds.iter().take(MAX_VIEW_DIAGNOSTICS).cloned().collect()
            }
            _ => Vec::new(),
        };
        Self {
            code: e.code,
            message: e.message(lang),
            op_index: e.op_index,
            field: e.field.clone(),
            subject: e.subject,
            diagnostics,
        }
    }

    /// Erreur sans index, champ, sujet ni diagnostic.
    #[must_use]
    pub const fn simple(code: &'static str, message: String) -> Self {
        Self {
            code,
            message,
            op_index: None,
            field: None,
            subject: None,
            diagnostics: Vec::new(),
        }
    }

    /// Même erreur, sur le champ `field`.
    #[must_use]
    pub fn with_field(mut self, field: &str) -> Self {
        self.field = Some(field.to_owned());
        self
    }
}

/// Texte selon la langue.
const fn tr(lang: Language, fr: &'static str, en: &'static str) -> &'static str {
    match lang {
        Language::Fr => fr,
        Language::En => en,
    }
}

/// Résultat d'une ouverture (`project.open`, `project.import_csv`,
/// `project.new`).
#[derive(Serialize)]
struct OpenedView {
    summary: view::SummaryView,
    /// Diagnostics de lecture, au plus [`MAX_VIEW_DIAGNOSTICS`].
    diagnostics: Vec<DiagView>,
    /// Diagnostics de lecture en tout.
    diagnostics_total: usize,
}

/// Résultat d'un enregistrement (`project.save`, `project.export_mspdi`).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Serialize)]
struct SavedView {
    path: String,
    bytes: usize,
    /// Codes des diagnostics d'écriture (arrondis, troncatures), au plus
    /// [`MAX_VIEW_DIAGNOSTICS`].
    diagnostics: Vec<String>,
    dirty: bool,
}

/// Rapport d'un rendu (`render.gantt`).
#[derive(Serialize)]
struct RenderView {
    svg: String,
    shown: u32,
    hidden: u32,
    truncated_days: bool,
    truncated_bytes: bool,
    bytes: usize,
}

/// Sérialise une vue (les vues n'ont que des clés textuelles : l'échec
/// n'est pas attendu, il devient une erreur propre).
fn to_value<T: Serialize>(v: &T) -> Result<Value, ToolError> {
    serde_json::to_value(v)
        .map_err(|e| ToolError::simple(codes::PARAM_INVALID, format!("sérialisation impossible : {e}")))
}

/// Retire de `obj` le drapeau booléen `key` (absent : faux).
fn take_flag(obj: &mut Map<String, Value>, key: &str, lang: Language) -> Result<bool, ToolError> {
    match obj.remove(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(b),
        Some(_) => Err(ToolError::simple(
            codes::PARAM_INVALID,
            format!(
                "{} « {key} »",
                tr(lang, "booléen attendu pour", "boolean expected for")
            ),
        )
        .with_field(key)),
    }
}

/// Paramètres en objet JSON (`null` : objet vide).
fn object(params: Value, lang: Language) -> Result<Map<String, Value>, ToolError> {
    match params {
        Value::Object(m) => Ok(m),
        Value::Null => Ok(Map::new()),
        _ => Err(ToolError::simple(
            codes::PARAM_INVALID,
            tr(lang, "objet JSON attendu", "JSON object expected").to_owned(),
        )),
    }
}

impl Workspace {
    /// Espace vide.
    #[must_use]
    pub const fn new(now: Moment, lang: Language) -> Self {
        Self {
            session: None,
            now,
            lang,
            root: None,
        }
    }

    /// Confine les chemins à `root` (MCP HTTP).
    #[must_use]
    pub fn with_root(mut self, root: PathBuf) -> Self {
        self.root = Some(root);
        self
    }

    /// Langue des messages et des sessions créées ici.
    #[must_use]
    pub const fn lang(&self) -> Language {
        self.lang
    }

    /// « Maintenant » injecté.
    #[must_use]
    pub const fn now(&self) -> Moment {
        self.now
    }

    /// Session ouverte.
    #[must_use]
    pub const fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// Session ouverte, modifiable.
    pub const fn session_mut(&mut self) -> Option<&mut Session> {
        self.session.as_mut()
    }

    /// Remplace la session ouverte (une ouverture remplace la précédente,
    /// même modifiée : l'appelant enregistre avant s'il le faut).
    pub fn set_session(&mut self, session: Session) {
        self.session = Some(session);
    }

    /// Routeur unique : commande `id` du registre, paramètres JSON.
    ///
    /// # Errors
    ///
    /// [`ToolError`] pour tout refus : `param.invalid` (id inconnu,
    /// paramètres mal formés), `command.not_yet_available` (commande
    /// réservée), `session.none` (aucun projet ouvert), `file.*`, ou
    /// l'erreur de la commande (`task.not_found`,
    /// `batch.would_create_errors`…). L'état est intact après un refus.
    pub fn call(&mut self, id: &str, params: Value) -> Result<Value, ToolError> {
        let lang = self.lang;
        let cmd = |e: CommandError| ToolError::from_command(&e, lang);
        let Some(spec) = lookup(id) else {
            return Err(ToolError::simple(
                codes::PARAM_INVALID,
                format!("{} : {id}", tr(lang, "commande inconnue", "unknown command")),
            ));
        };
        if let Availability::Milestone(m) = spec.available {
            return Err(cmd(CommandError::not_yet(m)));
        }
        match id {
            "project.open" => self.open(&parse_params::<OpenProjectParams>(params).map_err(cmd)?),
            "project.new" => self.new_project(parse_params::<NewProjectParams>(params).map_err(cmd)?),
            "project.import_csv" => self.import_csv(&parse_params::<ImportCsvParams>(params).map_err(cmd)?),
            "project.save" => {
                let p = parse_params::<SaveProjectParams>(params).map_err(cmd)?;
                self.save(p.path.as_deref(), true)
            }
            "project.export_mspdi" => {
                let p = parse_params::<ExportMspdiParams>(params).map_err(cmd)?;
                self.save(Some(&p.path), false)
            }
            "project.summary" => {
                parse_params::<NoParams>(params).map_err(cmd)?;
                to_value(&view::summary(self.current()?))
            }
            "task.list" => {
                let p = parse_params::<TaskListParams>(params).map_err(cmd)?;
                to_value(&view::list(self.current()?, &p).map_err(cmd)?)
            }
            "task.get" => {
                let p = parse_params::<TaskGetParams>(params).map_err(cmd)?;
                let s = self.current()?;
                let uid = TaskUid::new(p.uid);
                let v = ViewCtx::new(s).task_detail(uid).ok_or_else(|| {
                    cmd(
                        CommandError::new(codes::TASK_NOT_FOUND, CommandErrorKind::UnknownTask(uid))
                            .with_field("uid"),
                    )
                })?;
                to_value(&v)
            }
            "resource.list" | "analysis.resource_usage" => self.resource_read(id, params),
            "analysis.explain_date" => {
                let p = parse_params::<ExplainParams>(params).map_err(cmd)?;
                to_value(&crate::explain::explain_date(self.current()?, &p).map_err(cmd)?)
            }
            "analysis.critical_path" => {
                parse_params::<NoParams>(params).map_err(cmd)?;
                to_value(&ViewCtx::new(self.current()?).critical_path())
            }
            "render.gantt" => self.render(&parse_params::<RenderGanttParams>(params).map_err(cmd)?),
            "history.list" => {
                parse_params::<NoParams>(params).map_err(cmd)?;
                to_value(&self.current()?.history())
            }
            "history.undo" | "history.redo" => {
                parse_params::<NoParams>(params).map_err(cmd)?;
                let s = self.current_mut()?;
                let outcome = if id == "history.undo" { s.undo() } else { s.redo() };
                let outcome = outcome.map_err(|e| history_error(&e, lang))?;
                to_value(&ViewCtx::new(s).outcome(&outcome))
            }
            "batch.apply" => {
                let mut obj = object(params, lang)?;
                let dry_run = take_flag(&mut obj, "dry_run", lang)?;
                let detail = take_flag(&mut obj, "detail", lang)?;
                check_batch_version(&obj).map_err(cmd)?;
                let p = parse_params::<ApplyBatchParams>(Value::Object(obj)).map_err(cmd)?;
                self.apply(&p.into_batch(), dry_run, detail)
            }
            _ if spec.kind == Kind::Write => {
                let mut obj = object(params, lang)?;
                let dry_run = take_flag(&mut obj, "dry_run", lang)?;
                let detail = take_flag(&mut obj, "detail", lang)?;
                if obj.contains_key("op") {
                    return Err(ToolError::simple(
                        codes::PARAM_INVALID,
                        tr(lang, "champ inconnu « op »", "unknown field \"op\"").to_owned(),
                    )
                    .with_field("op"));
                }
                obj.insert("op".to_owned(), Value::String(id.to_owned()));
                let op = parse_params::<Command>(Value::Object(obj)).map_err(cmd)?;
                let batch = Batch {
                    format_version: Batch::FORMAT_VERSION,
                    lang: None,
                    ops: vec![op],
                    label: None,
                };
                self.apply(&batch, dry_run, detail)
            }
            _ => Err(ToolError::simple(
                codes::PARAM_INVALID,
                format!("{} : {id}", tr(lang, "commande inconnue", "unknown command")),
            )),
        }
    }

    /// Lectures des ressources (J5) : `resource.list`,
    /// `analysis.resource_usage`.
    fn resource_read(&self, id: &str, params: Value) -> Result<Value, ToolError> {
        let lang = self.lang;
        let cmd = |e: CommandError| ToolError::from_command(&e, lang);
        if id == "resource.list" {
            let p = parse_params::<ResourceListParams>(params).map_err(cmd)?;
            to_value(&view::list_resources(self.current()?, &p).map_err(cmd)?)
        } else {
            let p = parse_params::<UsageParams>(params).map_err(cmd)?;
            to_value(&view::resource_usage(self.current()?, &p).map_err(cmd)?)
        }
    }

    /// Session ouverte, sinon `session.none`.
    fn current(&self) -> Result<&Session, ToolError> {
        let lang = self.lang;
        self.session.as_ref().ok_or_else(|| no_session(lang))
    }

    fn current_mut(&mut self) -> Result<&mut Session, ToolError> {
        let lang = self.lang;
        self.session.as_mut().ok_or_else(|| no_session(lang))
    }

    /// Applique un lot à la session ouverte.
    fn apply(&mut self, batch: &Batch, dry_run: bool, detail: bool) -> Result<Value, ToolError> {
        let lang = self.lang;
        let s = self.current_mut()?;
        let mode = if dry_run {
            ApplyMode::DryRun
        } else {
            ApplyMode::Commit
        };
        let outcome: ApplyOutcome = s
            .apply_with(batch, mode, ApplyOptions { detail })
            .map_err(|e| ToolError::from_command(&e, lang))?;
        to_value(&ViewCtx::new(s).outcome(&outcome))
    }

    /// Ouvre une session, renvoie son résumé et les diagnostics de lecture.
    fn opened(
        &mut self,
        session: Session,
        diagnostics: Vec<DiagView>,
        total: usize,
    ) -> Result<Value, ToolError> {
        let summary = view::summary(&session);
        self.session = Some(session);
        to_value(&OpenedView {
            summary,
            diagnostics,
            diagnostics_total: total,
        })
    }

    fn new_project(&mut self, p: NewProjectParams) -> Result<Value, ToolError> {
        let lang = self.lang;
        let anchor = match (p.start, p.finish) {
            (Some(m), None) => AnchorSpec::Start(m),
            (None, Some(m)) => AnchorSpec::Finish(m),
            _ => {
                return Err(ToolError::simple(
                    codes::PARAM_INVALID,
                    tr(
                        lang,
                        "indiquer exactement une date : « start » ou « finish »",
                        "give exactly one date: \"start\" or \"finish\"",
                    )
                    .to_owned(),
                )
                .with_field("start"));
            }
        };
        let init = NewProject {
            name: p.name,
            anchor,
            lang: Some(p.lang.unwrap_or(lang)),
        };
        let session = Session::new_project(init, self.now).map_err(|e| ToolError::from_command(&e, lang))?;
        self.opened(session, Vec::new(), 0)
    }

    fn render(&self, p: &RenderGanttParams) -> Result<Value, ToolError> {
        let lang = self.lang;
        let s = self.current()?;
        let max_rows = p.max_rows.unwrap_or(MAX_RENDER_ROWS);
        if max_rows == 0 {
            return Err(ToolError::simple(
                codes::PARAM_INVALID,
                tr(
                    lang,
                    "« max_rows » doit valoir au moins 1",
                    "\"max_rows\" must be at least 1",
                )
                .to_owned(),
            )
            .with_field("max_rows"));
        }
        if max_rows > MAX_RENDER_ROWS {
            let e = CommandError::new(
                codes::LIMIT_EXCEEDED,
                CommandErrorKind::Limit {
                    what: "max_rows",
                    max: u64::from(MAX_RENDER_ROWS),
                },
            )
            .with_field("max_rows");
            return Err(ToolError::from_command(&e, lang));
        }
        let sched = s.schedule();
        let range = match (p.from, p.to) {
            (None, None) => planrs_render::RangeSpec::Auto,
            (from, to) => {
                let from = from.unwrap_or_else(|| sched.project_start.day());
                let to = to.unwrap_or_else(|| sched.project_finish.day());
                if from > to {
                    return Err(ToolError::simple(
                        codes::PARAM_INVALID,
                        tr(lang, "« from » est après « to »", "\"from\" is after \"to\"").to_owned(),
                    )
                    .with_field("from"));
                }
                planrs_render::RangeSpec::Span { from, to }
            }
        };
        let zoom = match p.zoom.unwrap_or_default() {
            ZoomJson::Auto => planrs_render::Zoom::Auto {
                width_px: AUTO_WIDTH_PX,
            },
            ZoomJson::Day => planrs_render::Zoom::Day,
            ZoomJson::Week => planrs_render::Zoom::Week,
            ZoomJson::Month => planrs_render::Zoom::Month,
            ZoomJson::Quarter => planrs_render::Zoom::Quarter,
        };
        let req = planrs_render::GanttRequest {
            range,
            zoom,
            filter: planrs_render::RowFilter {
                critical_only: p.critical_only,
                uids: None,
                max_outline: None,
            },
            language: match lang {
                Language::Fr => planrs_render::Lang::Fr,
                Language::En => planrs_render::Lang::En,
            },
            theme: planrs_render::Theme::Light,
            limits: planrs_render::RenderLimits {
                max_rows,
                ..planrs_render::RenderLimits::default()
            },
        };
        let (svg, report) = planrs_render::svg(s.project(), sched, &req);
        to_value(&RenderView {
            svg,
            shown: report.shown,
            hidden: report.hidden,
            truncated_days: report.truncated_days,
            truncated_bytes: report.truncated_bytes,
            bytes: report.bytes,
        })
    }
}

/// Erreur `session.none`.
fn no_session(lang: Language) -> ToolError {
    ToolError::simple(
        codes::SESSION_NONE,
        tr(
            lang,
            "aucun projet ouvert : ouvrir (open_project), créer (new_project) ou importer (import_csv) d'abord",
            "no project is open: open (open_project), create (new_project) or import (import_csv) one first",
        )
        .to_owned(),
    )
}

/// Version d'un lot contrôlée avant ses opérations (ADR-0012 L3) : un lot
/// d'un format futur est refusé comme tel (`batch.unsupported_version`),
/// même si ses opérations sont inconnues ici. Absente : 1 ; non entière :
/// `param.invalid` par la lecture qui suit.
fn check_batch_version(obj: &Map<String, Value>) -> Result<(), CommandError> {
    match obj.get("format_version").and_then(Value::as_u64) {
        Some(v) if v != u64::from(Batch::FORMAT_VERSION) => Err(CommandError::new(
            codes::BATCH_UNSUPPORTED_VERSION,
            CommandErrorKind::UnsupportedVersion(u32::try_from(v).unwrap_or(u32::MAX)),
        )
        .with_field("format_version")),
        _ => Ok(()),
    }
}

/// Erreur d'annulation ou de rétablissement.
fn history_error(e: &HistoryError, lang: Language) -> ToolError {
    let message = match e {
        HistoryError::Empty => tr(lang, "rien à annuler ou à rétablir", "nothing to undo or redo").to_owned(),
        HistoryError::Inconsistent(m) => match lang {
            Language::Fr => format!("historique incohérent, session intacte : {m}"),
            Language::En => format!("inconsistent history, session unchanged: {m}"),
        },
    };
    ToolError::simple(e.code(), message)
}

/// Nom affiché d'un document : nom du fichier sans extension.
#[cfg(not(target_arch = "wasm32"))]
fn display_name(path: &std::path::Path) -> String {
    path.file_stem().map_or_else(
        || path.display().to_string(),
        |s| s.to_string_lossy().into_owned(),
    )
}

#[cfg(not(target_arch = "wasm32"))]
impl Workspace {
    /// Chemin effectif d'un paramètre `path`, tel que les outils de fichier
    /// le liront ou l'écriront : confiné à la racine s'il y en a une (un
    /// chemin relatif part alors de la racine), tel quel sinon. Sert à la
    /// garde ADR-0002 des surfaces, qui doit viser le même fichier.
    ///
    /// # Errors
    ///
    /// `param.invalid` (chemin vide) ; `file.outside_root` et les erreurs
    /// de confinement (champ `path`).
    pub fn resolve_path(&self, path: &str) -> Result<PathBuf, ToolError> {
        self.resolve(path, "path")
    }

    /// Chemin effectif : confiné à la racine s'il y en a une.
    fn resolve(&self, path: &str, field: &str) -> Result<PathBuf, ToolError> {
        let lang = self.lang;
        if path.is_empty() {
            return Err(ToolError::simple(
                codes::PARAM_INVALID,
                tr(lang, "chemin vide", "empty path").to_owned(),
            )
            .with_field(field));
        }
        let p = PathBuf::from(path);
        match &self.root {
            None => Ok(p),
            Some(root) => crate::files::confine(root, &p).map_err(|e| file_error(&e, lang).with_field(field)),
        }
    }

    fn open(&mut self, p: &OpenProjectParams) -> Result<Value, ToolError> {
        let lang = self.lang;
        let path = self.resolve(&p.path, "path")?;
        let bytes = crate::files::read_path(&path, MAX_MSPDI_BYTES)
            .map_err(|e| file_error(&e, lang).with_field("path"))?;
        let origin = crate::session::DocOrigin {
            name: display_name(&path),
            path: Some(path.display().to_string()),
        };
        let (session, diags) = Session::open_bytes(&bytes, origin, self.now, lang).map_err(|e| {
            ToolError::simple(
                e.code(),
                format!(
                    "{} : {e}",
                    tr(lang, "document MSPDI inexploitable", "unusable MSPDI document")
                ),
            )
            .with_field("path")
        })?;
        let views = diags
            .iter()
            .take(MAX_VIEW_DIAGNOSTICS)
            .map(DiagView::from_import)
            .collect();
        self.opened(session, views, diags.len())
    }

    fn import_csv(&mut self, p: &ImportCsvParams) -> Result<Value, ToolError> {
        let lang = self.lang;
        let path = self.resolve(&p.path, "path")?;
        let bytes = crate::files::read_path(&path, MAX_CSV_BYTES)
            .map_err(|e| file_error(&e, lang).with_field("path"))?;
        // Hypothèse : le projet importé est planifié depuis « maintenant »
        // (MS Project prend la date du jour pour un projet neuf).
        let mut options =
            crate::csv::CsvImportOptions::new(planrs_core::ScheduleAnchor::FromStart(self.now), self.now);
        options.locale = p.lang.map(|l| match l {
            Language::Fr => crate::csv::CsvLocale::Fr,
            Language::En => crate::csv::CsvLocale::En,
        });
        let imported = crate::csv::import(&bytes, &options).map_err(|e| {
            ToolError::simple(
                e.code(),
                format!("{} : {e}", tr(lang, "import CSV impossible", "CSV import failed")),
            )
            .with_field("path")
        })?;
        let origin = crate::session::DocOrigin {
            name: display_name(&path),
            path: None,
        };
        let session = Session::from_project(imported.project, origin, self.now, p.lang.unwrap_or(lang));
        let total = imported.diagnostics.len();
        let views = imported
            .diagnostics
            .iter()
            .take(MAX_VIEW_DIAGNOSTICS)
            .map(|d| DiagView {
                code: d.code.code().to_owned(),
                severity: d.code.severity(),
                subject: None,
                message: format!(
                    "{} {}{}",
                    tr(lang, "ligne", "line"),
                    d.line,
                    d.column.as_deref().map(|c| format!(" ({c})")).unwrap_or_default()
                ),
            })
            .collect();
        self.opened(session, views, total)
    }

    /// Enregistre (`mark = true` : marque la session enregistrée et suit le
    /// nouveau chemin) ou exporte une copie.
    fn save(&mut self, path: Option<&str>, mark: bool) -> Result<Value, ToolError> {
        let lang = self.lang;
        let target = if let Some(p) = path {
            self.resolve(p, "path")?
        } else {
            let Some(p) = self.current()?.origin().path.clone() else {
                return Err(ToolError::simple(
                    codes::PARAM_INVALID,
                    tr(
                        lang,
                        "projet sans fichier : indiquer « path »",
                        "the project has no file yet: give \"path\"",
                    )
                    .to_owned(),
                )
                .with_field("path"));
            };
            self.resolve(&p, "path")?
        };
        let s = self.current_mut()?;
        let written = s.save_bytes(&planrs_mspdi::write::WriteOptions::default());
        crate::files::write_atomic(&target, &written.xml)
            .map_err(|e| file_error(&e, lang).with_field("path"))?;
        if mark {
            s.set_origin(crate::session::DocOrigin {
                name: display_name(&target),
                path: Some(target.display().to_string()),
            });
            s.mark_saved();
        }
        to_value(&SavedView {
            path: target.display().to_string(),
            bytes: written.xml.len(),
            diagnostics: written
                .diagnostics
                .iter()
                .take(MAX_VIEW_DIAGNOSTICS)
                .map(|d| d.code.code().to_owned())
                .collect(),
            dirty: s.is_dirty(),
        })
    }
}

/// Erreur d'outil d'une erreur de fichier.
#[cfg(not(target_arch = "wasm32"))]
fn file_error(e: &crate::files::FileError, lang: Language) -> ToolError {
    ToolError::simple(e.code(), e.message(lang))
}

#[cfg(target_arch = "wasm32")]
impl Workspace {
    fn no_files(&self, code: &'static str) -> ToolError {
        ToolError::simple(
            code,
            tr(
                self.lang,
                "pas de système de fichiers dans cette version",
                "no file system in this build",
            )
            .to_owned(),
        )
    }

    fn open(&mut self, _: &OpenProjectParams) -> Result<Value, ToolError> {
        Err(self.no_files(codes::FILE_READ))
    }

    fn import_csv(&mut self, _: &ImportCsvParams) -> Result<Value, ToolError> {
        Err(self.no_files(codes::FILE_READ))
    }

    fn save(&mut self, _: Option<&str>, _: bool) -> Result<Value, ToolError> {
        Err(self.no_files(codes::FILE_WRITE))
    }
}
