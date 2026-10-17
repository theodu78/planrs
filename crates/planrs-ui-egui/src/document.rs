//! Octets MSPDI → document prêt à afficher (lecture + calcul), sans egui.
//!
//! Le planning est calculé une seule fois, à l'ouverture : les fondations sont
//! en lecture seule. Toute date affichée vient de `planrs_core::schedule`
//! (règle 3).

use planrs_core::{Moment, Project, Schedule};
use planrs_mspdi::{ImportDiagnostic, MspdiError, ReadOptions};

use crate::timescale::{DayRange, default_range};

/// Résultat de [`load`] : devient un `OpenDocument` quand l'application lui
/// attribue une origine et une génération.
#[derive(Clone, Debug)]
pub struct LoadedDocument {
    /// Modèle lu (entrées seules).
    pub project: Project,
    /// Planning calculé à l'ouverture.
    pub schedule: Schedule,
    /// Diagnostics d'import (ceux du calcul sont dans `schedule`).
    pub import_diagnostics: Vec<ImportDiagnostic>,
    /// Plage de jours affichée par l'échelle de temps.
    pub range: DayRange,
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

/// Lit un document MSPDI et calcule son planning.
///
/// `now` est la date courante injectée (`ReadOptions::current_date`) : aucune
/// horloge n'est lue ici.
///
/// # Errors
///
/// [`OpenError::Mspdi`] si le lecteur MSPDI refuse le document ; tout le
/// reste devient un diagnostic.
pub fn load(bytes: &[u8], now: Moment) -> Result<LoadedDocument, OpenError> {
    let document = planrs_mspdi::read(bytes, &ReadOptions::new(now))?;
    let schedule = planrs_core::schedule(&document.project);
    let range = default_range(&document.project, &schedule);
    Ok(LoadedDocument {
        project: document.project,
        schedule,
        import_diagnostics: document.diagnostics,
        range,
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
        assert_eq!(doc.project.tasks.len(), 49);
        assert!(doc.import_diagnostics.is_empty(), "{:?}", doc.import_diagnostics);
        assert!(
            doc.schedule.diagnostics.is_empty(),
            "{:?}",
            doc.schedule.diagnostics
        );
        assert_eq!(doc.schedule.project_finish, "2027-05-17T17:00".parse().unwrap());
        let links: usize = doc.project.tasks.iter().map(|t| t.predecessors.len()).sum();
        assert_eq!(links, 46);
        assert!(doc.range.start <= doc.schedule.project_start.day());
        assert!(doc.range.end > doc.schedule.project_finish.day());
    }

    #[test]
    fn document_load_garbage_is_error() {
        let Err(err) = load(b"pas du xml", now()) else {
            panic!("un document illisible doit être refusé");
        };
        assert!(matches!(err, OpenError::Mspdi(_)));
        assert!(!err.to_string().is_empty());
        let read = OpenError::Read("accès refusé".to_owned());
        assert_eq!(read.to_string(), "lecture impossible : accès refusé");
    }
}
