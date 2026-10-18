//! Erreurs des commandes, de l'historique et de l'ouverture (contrat J4,
//! ADR-0012).
//!
//! Chaque erreur porte un code stable (`task.not_found`…), repris tel quel
//! par le MCP, la CLI et l'interface (clé de traduction `.ftl`). Les messages
//! FR/EN sont rédigés par le lot L3.

use planrs_core::{Diagnostic, ModelError, Subject, TaskUid};
use planrs_mspdi::MspdiError;

use crate::text::{Language, TextError};

/// Codes d'erreur stables. Les codes `text.*` sont ceux de
/// [`TextError::code`].
pub mod codes {
    /// Tâche (UID) inconnue.
    pub const TASK_NOT_FOUND: &str = "task.not_found";
    /// Étiquette `$x` inconnue dans le lot (R-150).
    pub const BATCH_UNKNOWN_LABEL: &str = "batch.unknown_label";
    /// Étiquette `$x` redéfinie dans le lot (R-150).
    pub const BATCH_DUPLICATE_LABEL: &str = "batch.duplicate_label";
    /// Paramètre invalide.
    pub const PARAM_INVALID: &str = "param.invalid";
    /// Paramètres contradictoires (`text` et `minutes` d'une durée…).
    pub const PARAM_MISMATCH: &str = "param.mismatch";
    /// Champ calculé d'une récapitulative (R-157).
    pub const TASK_COMPUTED_FIELD: &str = "task.computed_field";
    /// Le lot ferait apparaître des erreurs (R-160).
    pub const BATCH_WOULD_CREATE_ERRORS: &str = "batch.would_create_errors";
    /// Lien déjà présent (R-156).
    pub const LINK_DUPLICATE: &str = "link.duplicate";
    /// Lien d'une tâche vers elle-même (R-156).
    pub const LINK_SELF: &str = "link.self";
    /// Lien entre une récapitulative et sa descendante (R-156, R-059).
    pub const LINK_SUMMARY_DESCENDANT: &str = "link.summary_descendant";
    /// Lien absent (R-156, `link.update`, `link.remove`).
    pub const LINK_NOT_FOUND: &str = "link.not_found";
    /// Récapitulative supprimée sans `cascade` (R-155).
    pub const TASK_HAS_CHILDREN: &str = "task.has_children";
    /// Indentation ou désindentation refusée (R-153).
    pub const TASK_OUTLINE_REFUSED: &str = "task.outline_refused";
    /// Déplacement dans son propre sous-arbre (R-154).
    pub const TASK_MOVE_INTO_SUBTREE: &str = "task.move_into_subtree";
    /// Borne dépassée.
    pub const LIMIT_EXCEEDED: &str = "limit.exceeded";
    /// `format_version` non pris en charge.
    pub const BATCH_UNSUPPORTED_VERSION: &str = "batch.unsupported_version";
    /// Commande réservée à un jalon ultérieur.
    pub const COMMAND_NOT_YET_AVAILABLE: &str = "command.not_yet_available";
    /// Dates d'une tâche manuelle (R-157, ADR-0005).
    pub const TASK_MANUAL_UNSUPPORTED: &str = "task.manual_unsupported";
    /// Exception de calendrier en chevauchement partiel (R-165).
    pub const CALENDAR_EXCEPTION_OVERLAP: &str = "calendar.exception_overlap";
    /// Aucune session ouverte (`Workspace`).
    pub const SESSION_NONE: &str = "session.none";
    /// Verrou de session empoisonné (MCP).
    pub const SESSION_POISONED: &str = "session.poisoned";
    /// Fichier illisible (lecture impossible).
    pub const FILE_READ: &str = "file.read";
    /// Écriture impossible.
    pub const FILE_WRITE: &str = "file.write";
    /// Fichier trop gros.
    pub const FILE_TOO_LARGE: &str = "file.too_large";
    /// Document MSPDI inexploitable.
    pub const FILE_INVALID: &str = "file.invalid";
    /// Chemin hors de la racine autorisée (MCP HTTP).
    pub const FILE_OUTSIDE_ROOT: &str = "file.outside_root";
    /// Fichier tenu par une application vivante (ADR-0002).
    pub const FILE_OPEN_IN_APP: &str = "file.open_in_app";
    /// Rien à annuler ou à rétablir.
    pub const HISTORY_EMPTY: &str = "history.empty";
    /// Entrée d'historique incohérente avec le projet : annulation refusée,
    /// session intacte (R-164 ; jamais attendu).
    pub const HISTORY_INCONSISTENT: &str = "history.inconsistent";
}

/// Commande refusée. L'état de la session est intact (atomicité du lot).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{code}")]
pub struct CommandError {
    /// Code stable ([`codes`]).
    pub code: &'static str,
    /// Index de l'opération fautive dans le lot.
    pub op_index: Option<usize>,
    /// Champ fautif (`duration`, `uid`…).
    pub field: Option<String>,
    /// Élément du projet concerné.
    pub subject: Option<Subject>,
    /// Détail.
    pub kind: CommandErrorKind,
}

/// Détail d'une [`CommandError`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandErrorKind {
    /// UID inconnu.
    UnknownTask(TaskUid),
    /// Étiquette inconnue.
    UnknownLabel(String),
    /// Étiquette redéfinie.
    DuplicateLabel(String),
    /// Paramètre invalide (explication).
    InvalidParam(String),
    /// Texte illisible.
    Text(TextError),
    /// Champ calculé d'une récapitulative.
    ComputedField,
    /// Erreurs nouvelles (R-160).
    WouldCreateErrors(Vec<Diagnostic>),
    /// Lien déjà présent.
    DuplicateLink,
    /// Lien d'une tâche vers elle-même.
    SelfLink,
    /// Lien récapitulative ↔ descendante.
    SummaryDescendantLink,
    /// Récapitulative et ses enfants.
    HasChildren {
        /// Nombre de descendants.
        count: usize,
    },
    /// Hiérarchie refusée.
    OutlineRefused,
    /// Déplacement dans son propre sous-arbre.
    MoveIntoOwnSubtree,
    /// Borne dépassée.
    Limit {
        /// Grandeur bornée (`ops`, `task_uid`…).
        what: &'static str,
        /// Maximum.
        max: u64,
    },
    /// Version de format non prise en charge.
    UnsupportedVersion(u32),
    /// Commande réservée.
    NotYetAvailable {
        /// Jalon de disponibilité.
        milestone: u8,
    },
    /// Tâche manuelle.
    ManualTaskUnsupported,
    /// Paramètres contradictoires.
    ParamMismatch,
}

impl CommandError {
    /// Erreur sans index, champ ni sujet.
    #[must_use]
    pub const fn new(code: &'static str, kind: CommandErrorKind) -> Self {
        Self {
            code,
            op_index: None,
            field: None,
            subject: None,
            kind,
        }
    }

    /// Commande pas encore disponible : réservée au jalon `milestone`, ou
    /// squelette du contrat J4 pas encore rempli.
    #[must_use]
    pub const fn not_yet(milestone: u8) -> Self {
        Self::new(
            codes::COMMAND_NOT_YET_AVAILABLE,
            CommandErrorKind::NotYetAvailable { milestone },
        )
    }

    /// Même erreur, à l'index d'opération `i`.
    #[must_use]
    pub fn with_op_index(mut self, i: usize) -> Self {
        self.op_index = Some(i);
        self
    }

    /// Message pour l'utilisateur. Squelette du contrat : le code seul,
    /// messages FR/EN rédigés par le lot L3.
    #[must_use]
    pub fn message(&self, lang: Language) -> String {
        let _ = lang;
        self.code.to_owned()
    }
}

/// Annulation ou rétablissement impossible.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum HistoryError {
    /// Rien à annuler ou à rétablir (`history.empty`).
    #[error("rien à annuler ou à rétablir")]
    Empty,
    /// Delta d'historique inapplicable (`history.inconsistent`) : la
    /// session reste intacte.
    #[error("historique incohérent : {0}")]
    Inconsistent(ModelError),
}

impl HistoryError {
    /// Code stable.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Empty => codes::HISTORY_EMPTY,
            Self::Inconsistent(_) => codes::HISTORY_INCONSISTENT,
        }
    }
}

/// Ouverture d'un document impossible.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    /// Document MSPDI inexploitable (`file.invalid`).
    #[error(transparent)]
    Mspdi(#[from] MspdiError),
}

impl OpenError {
    /// Code stable.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Mspdi(_) => codes::FILE_INVALID,
        }
    }
}
