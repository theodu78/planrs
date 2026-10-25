//! Presse-papiers de lignes (R-244, R-256 ; ADR-0016 D3).
//!
//! Tampon interne [`RowClip`] (tâches, liens internes, affectations par nom
//! de ressource) et texte tabulé des colonnes visibles pour le presse-papiers
//! du système ; au collage, une empreinte stable ([`fingerprint`], FNV-1a
//! 64) égale au dernier texte émis ⇒ collage complet, sinon collage externe
//! réduit ([`paste_text`]). Contrat posé par le lot L0, implémenté en L3 :
//! jusque-là [`ClipError::NotYetAvailable`] (sauf [`fingerprint`]).

use planrs_core::{Constraint, Diagnostic, Duration, Lag, LinkType, Moment, Priority, TaskUid, Units};

use crate::command::{Batch, Placement};
use crate::csv::CsvColumn;
use crate::session::Session;
use crate::text::{Language, TextContext};

/// Lignes au plus dans un texte collé (`paste.too_large`).
pub const MAX_PASTE_LINES: usize = 10_000;
/// Octets au plus dans un texte collé (`paste.too_large`).
pub const MAX_PASTE_BYTES: usize = 4 << 20;

/// Champs copiés d'une tâche (ni avancement, ni réels, ni ligne de base, ni
/// UID d'origine : R-244).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipFields {
    /// Nom.
    pub name: String,
    /// Durée.
    pub duration: Duration,
    /// Planification manuelle.
    pub manual: bool,
    /// Valeurs manuelles (début, fin, durée).
    pub manual_values: Option<(Option<Moment>, Option<Moment>, Option<Duration>)>,
    /// Jalon.
    pub milestone: bool,
    /// Contrainte.
    pub constraint: Constraint,
    /// Échéance.
    pub deadline: Option<Moment>,
    /// Notes.
    pub notes: String,
    /// Active.
    pub active: bool,
    /// Priorité.
    pub priority: Priority,
    /// Calendrier de la tâche, par nom.
    pub calendar: Option<String>,
}

/// Tâche du tampon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipTask {
    /// Niveau relatif au premier niveau du tampon (0 = même niveau).
    pub rel_level: u16,
    /// Champs copiés.
    pub fields: ClipFields,
}

/// Lien interne au tampon (indices dans `tasks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipLink {
    /// Prédécesseur.
    pub from: u32,
    /// Successeur.
    pub to: u32,
    /// Type.
    pub kind: LinkType,
    /// Décalage.
    pub lag: Lag,
}

/// Affectation du tampon, par nom de ressource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipAssignment {
    /// Indice de la tâche dans `tasks`.
    pub task: u32,
    /// Nom de la ressource.
    pub resource: String,
    /// Unités.
    pub units: Units,
}

/// Tampon interne de lignes copiées.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowClip {
    /// Tâches, dans l'ordre du projet.
    pub tasks: Vec<ClipTask>,
    /// Liens dont les deux bouts sont dans le tampon.
    pub links: Vec<ClipLink>,
    /// Affectations.
    pub assignments: Vec<ClipAssignment>,
}

/// Traitement des ressources au collage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResourcePaste {
    /// Par nom ; nom absent ⇒ affectation omise et comptée.
    #[default]
    Match,
    /// Par nom ; nom absent ⇒ ressource de travail créée.
    MatchOrCreate,
    /// Aucune affectation collée.
    Drop,
}

/// Options du collage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PasteOptions {
    /// Recréer les liens internes.
    pub links: bool,
    /// Ressources.
    pub resources: ResourcePaste,
}

impl Default for PasteOptions {
    fn default() -> Self {
        Self {
            links: true,
            resources: ResourcePaste::Match,
        }
    }
}

/// Compte rendu d'un collage.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PasteReport {
    /// Tâches créées.
    pub tasks: u32,
    /// Liens externes non recréés (`paste.links_dropped`).
    pub links_dropped: u32,
    /// Affectations omises.
    pub assignments_dropped: u32,
    /// Ressources créées.
    pub resources_created: u32,
    /// Diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Refus d'une copie ou d'un collage.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ClipError {
    /// Tâche inconnue.
    #[error("tâche inconnue : {0:?}")]
    UnknownTask(TaskUid),
    /// Texte non reconnu (`paste.unrecognized`).
    #[error("texte collé non reconnu")]
    Unrecognized,
    /// Texte trop grand (`paste.too_large`).
    #[error("texte collé trop grand")]
    TooLarge,
    /// Livré par le lot L3.
    #[error("presse-papiers pas encore disponible")]
    NotYetAvailable,
}

impl RowClip {
    /// Copie des tâches `uids` (descendants des récapitulatives inclus).
    /// Squelette L0.
    ///
    /// # Errors
    ///
    /// [`ClipError`].
    pub fn capture(s: &Session, uids: &[TaskUid]) -> Result<Self, ClipError> {
        let _ = (s, uids);
        Err(ClipError::NotYetAvailable)
    }

    /// Texte tabulé des colonnes `cols` (une ligne par tâche). Squelette
    /// L0 : texte vide.
    #[must_use]
    pub fn to_tsv(&self, cols: &[CsvColumn], ctx: &TextContext) -> String {
        let _ = (cols, ctx);
        String::new()
    }
}

/// Empreinte stable d'un texte du presse-papiers : FNV-1a 64 bits sur le
/// texte aux fins de ligne normalisées en `\n` (`\r\n` et `\r` seul lus
/// `\n`). Jamais `RandomState` : même texte, même empreinte partout.
#[must_use]
pub fn fingerprint(text: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    let mut feed = |b: u8| {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(PRIME);
    };
    let mut bytes = text.bytes().peekable();
    while let Some(b) = bytes.next() {
        if b == b'\r' {
            if bytes.peek() == Some(&b'\n') {
                bytes.next();
            }
            feed(b'\n');
        } else {
            feed(b);
        }
    }
    hash
}

/// R-244 : lot qui colle le tampon à `at`. Squelette L0.
///
/// # Errors
///
/// [`ClipError`].
pub fn paste_batch(
    s: &Session,
    c: &RowClip,
    at: &Placement,
    o: &PasteOptions,
) -> Result<(Batch, PasteReport), ClipError> {
    let _ = (s, c, at, o);
    Err(ClipError::NotYetAvailable)
}

/// R-256 : lot qui colle un texte externe à `at`. Squelette L0.
///
/// # Errors
///
/// [`ClipError`].
pub fn paste_text(
    s: &Session,
    text: &str,
    at: &Placement,
    lang: Language,
) -> Result<(Batch, PasteReport), ClipError> {
    let _ = (s, text, at, lang);
    Err(ClipError::NotYetAvailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vecteurs FNV-1a 64 publiés (chaîne vide, « a ») et normalisation
    /// des fins de ligne.
    #[test]
    fn fingerprint_fnv1a_vectors_and_newlines() {
        assert_eq!(fingerprint(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fingerprint("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fingerprint("A\r\nB"), fingerprint("A\nB"));
        assert_eq!(fingerprint("A\rB"), fingerprint("A\nB"));
        assert_ne!(fingerprint("A\tB"), fingerprint("A\nB"));
    }
}
