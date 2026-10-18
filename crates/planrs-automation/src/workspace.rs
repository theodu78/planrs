//! Couche fine au-dessus de [`Session`] pour la CLI, le MCP et le canal :
//! session facultative, « maintenant », langue, racine des chemins
//! (ADR-0012 D1, lot L6).

use std::path::PathBuf;

use planrs_core::{Diagnostic, Moment, Subject};
use serde::Serialize;

use crate::error::codes;
use crate::session::Session;
use crate::text::Language;

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

    /// Routeur unique : commande `id` du registre, paramètres JSON.
    /// Squelette L0 : `command.not_yet_available`.
    ///
    /// # Errors
    ///
    /// [`ToolError`] pour tout refus.
    pub fn call(&mut self, id: &str, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let _ = (id, params, self.now, self.lang, &self.root);
        Err(ToolError {
            code: codes::COMMAND_NOT_YET_AVAILABLE,
            message: codes::COMMAND_NOT_YET_AVAILABLE.to_owned(),
            op_index: None,
            field: None,
            subject: None,
            diagnostics: Vec::new(),
        })
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
}
