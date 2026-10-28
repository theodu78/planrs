//! Refus du registre en langage d'utilisateur (seconde passe UX, ADR-0019
//! D1, H-UI-105).
//!
//! Contrat posé par le lot L0 : [`user_message`] est appelée par
//! `App::report_command_error` et rend la phrase affichée et le détail
//! technique repliable. En L0, la phrase est celle d'avant la passe
//! (message traduit par code, texte du moteur inséré) et le détail est
//! vide ; le lot L1 écrit une phrase par code, le sujet en ID de tâche, et
//! met le code, l'opération et le texte du moteur dans le détail.

use planrs_automation::CommandError;
use planrs_core::Project;

use crate::i18n::{Arg, TextId, Texts, error_text_id};

/// Phrase et détail d'un refus de commande : `(texte, détail)` ; détail
/// vide s'il n'y en a pas. `project` : document visé (sujets en ID, L1) ;
/// `engine` : langue des textes du moteur.
#[must_use]
pub fn user_message(
    error: &CommandError,
    project: Option<&Project>,
    texts: &Texts,
    engine: planrs_automation::text::Language,
) -> (String, String) {
    // L0 : le projet n'est pas encore lu (sujets en UID dans le texte du
    // moteur, comme avant la passe).
    let _ = project;
    let detail = error.message(engine);
    let text = texts.get_args(error_text_id(error.code), &[("detail", Arg::Str(detail.into()))]);
    (text, String::new())
}

/// Textes posés par le lot L0 pour les refus en langage d'utilisateur,
/// servis par le lot L1 (H-UI-105) : phrase générique, détail (code,
/// opération), tâche supprimée, et une phrase par code du registre que
/// l'interface produit.
pub const L1_TEXTS: [TextId; 14] = [
    TextId::RefusalGeneric,
    TextId::RefusalDetailCode,
    TextId::RefusalDetailOperation,
    TextId::RefusalDeletedTask,
    TextId::RefusalLinkDuplicate,
    TextId::RefusalLinkCycle,
    TextId::RefusalLinkSelf,
    TextId::RefusalLinkSummary,
    TextId::RefusalTaskNotFound,
    TextId::RefusalBatchErrors,
    TextId::RefusalParamInvalid,
    TextId::RefusalLimitExceeded,
    TextId::RefusalTextParse,
    TextId::RefusalCustomField,
];
