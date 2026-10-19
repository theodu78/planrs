//! Octets MSPDI → session d'édition prête à afficher (lecture + calcul),
//! sans egui.
//!
//! Le document est une [`Session`] de `planrs_automation` (ADR-0012 D11) :
//! toute édition passe par un lot de commandes du registre, recalculé par
//! le moteur. Toute date affichée vient de `planrs_core::schedule`
//! (règle 3).

use planrs_automation::text::Language as TextLanguage;
use planrs_automation::{DocOrigin as SessionOrigin, Session};
use planrs_core::Moment;
use planrs_mspdi::{ImportDiagnostic, MspdiError};

use crate::timescale::{ViewSpan, default_range};

/// Résultat de [`load`] : devient un `OpenDocument` quand l'application lui
/// attribue une origine et un numéro d'ouverture.
#[derive(Clone, Debug)]
pub struct LoadedDocument {
    /// Session d'édition : modèle lu, planning calculé à l'ouverture,
    /// historique vide.
    pub session: Session,
    /// Diagnostics d'import (ceux du calcul sont dans `schedule`).
    pub import_diagnostics: Vec<ImportDiagnostic>,
    /// Plage de jours affichée par l'échelle de temps.
    pub range: ViewSpan,
    /// La plage a été tronquée à 20 ans (projet trop long) : l'application
    /// le signale par un message traduit.
    pub range_truncated: bool,
}

/// Échec d'ouverture d'un document.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// Lecture des octets impossible (fichier, téléversement).
    #[error("lecture impossible : {0}")]
    Read(String),
    /// Document MSPDI structurellement inexploitable.
    #[error(transparent)]
    Mspdi(#[from] MspdiError),
}

/// Lit un document MSPDI et ouvre sa session (planning calculé).
///
/// `now` est la date courante injectée (`ReadOptions::current_date`) : aucune
/// horloge n'est lue ici. L'origine de la session est posée par
/// l'application à l'installation ; sa langue (français) ne sert pas à
/// l'interface, qui passe toujours la sienne (messages d'erreur, lecture des
/// saisies).
///
/// # Errors
///
/// [`OpenError::Mspdi`] si le lecteur MSPDI refuse le document ; tout le
/// reste devient un diagnostic.
pub fn load(bytes: &[u8], now: Moment) -> Result<LoadedDocument, OpenError> {
    let origin = SessionOrigin {
        name: String::new(),
        path: None,
    };
    let (session, import_diagnostics) =
        Session::open_bytes(bytes, origin, now, TextLanguage::Fr).map_err(|error| match error {
            planrs_automation::OpenError::Mspdi(error) => OpenError::Mspdi(error),
        })?;
    let range = default_range(session.project(), session.schedule());
    Ok(LoadedDocument {
        session,
        import_diagnostics,
        range: range.span,
        range_truncated: range.truncated,
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "tests : un échec doit s'arrêter net et lisiblement"
    )]

    use super::{OpenError, load};

    fn now() -> planrs_core::Moment {
        "2026-10-17T08:00".parse().unwrap()
    }

    #[test]
    fn document_load_demo() {
        let doc = load(crate::DEMO_MSPDI, now()).unwrap();
        let (project, schedule) = (doc.session.project(), doc.session.schedule());
        assert_eq!(project.tasks.len(), 49);
        assert!(doc.import_diagnostics.is_empty(), "{:?}", doc.import_diagnostics);
        assert!(schedule.diagnostics.is_empty(), "{:?}", schedule.diagnostics);
        assert_eq!(schedule.project_finish, "2027-05-17T17:00".parse().unwrap());
        let links: usize = project.tasks.iter().map(|t| t.predecessors.len()).sum();
        assert_eq!(links, 46);
        assert!(doc.range.start <= schedule.project_start.day());
        assert!(doc.range.end > schedule.project_finish.day());
        assert!(!doc.range_truncated);
        assert!(!doc.session.is_dirty());
        assert_eq!(doc.session.generation(), 0);
    }

    #[test]
    fn document_load_garbage_is_error() {
        let Err(err) = load(b"pas du xml", now()) else {
            panic!("un document illisible doit être refusé");
        };
        assert!(matches!(err, OpenError::Mspdi(_)));
        assert_ne!(err.to_string(), "");
        let read = OpenError::Read("accès refusé".to_owned());
        assert_eq!(read.to_string(), "lecture impossible : accès refusé");
    }
}
