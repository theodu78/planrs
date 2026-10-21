//! Octets MSPDI → session d'édition prête à afficher (lecture + calcul),
//! sans egui.
//!
//! Le document est une [`Session`] de `planrs_automation` (ADR-0012 D11) :
//! toute édition passe par un lot de commandes du registre, recalculé par
//! le moteur. Toute date affichée vient de `planrs_core::schedule`
//! (règle 3).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use planrs_automation::text::Language as TextLanguage;
use planrs_automation::{DocOrigin as SessionOrigin, Session};
use planrs_core::{CalendarSet, CompiledCalendar, Limits, Moment, Project};
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

/// Calendriers compilés d'un document (N3, J7 lot L7) : compilés à la
/// première demande, **une fois par génération** de la session, avec le
/// budget du moteur (`Limits::compiled_segments`, R-172 amendée) ; partagés
/// par `Arc` (aucune copie par image).
///
/// Le cache vit dans l'`OpenDocument` et se lit par `&self` (le contexte
/// d'image ne prête le document qu'en lecture) : verrou court, jamais tenu
/// pendant le dessin ; un verrou empoisonné est repris tel quel (règle 10).
#[derive(Debug)]
pub struct CalendarCache {
    /// Ensemble compilé et génération qu'il reflète.
    slot: Mutex<Option<(u64, Arc<CalendarSet>)>>,
    /// Nombre de compilations faites (tests : une par génération).
    compiles: AtomicU64,
    /// Budget de compilation, en segments gardés.
    limit: usize,
}

impl CalendarCache {
    /// Cache vide au budget du moteur.
    #[must_use]
    pub fn new() -> Self {
        Self::with_limit(Limits::default().compiled_segments)
    }

    /// Cache vide au budget `limit` (tests du refus).
    #[must_use]
    pub const fn with_limit(limit: usize) -> Self {
        Self {
            slot: Mutex::new(None),
            compiles: AtomicU64::new(0),
            limit,
        }
    }

    /// Calendriers de `project` à la génération `generation` : ceux du
    /// cache s'il reflète cette génération, sinon compilés maintenant (une
    /// fois) et gardés. Ne panique jamais : un calendrier en échec ou hors
    /// budget est dans l'ensemble comme une erreur (`CalendarSet::get`).
    pub fn get(&self, project: &Project, generation: u64) -> Arc<CalendarSet> {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((cached, set)) = slot.as_ref()
            && *cached == generation
        {
            return Arc::clone(set);
        }
        let (set, _) = CalendarSet::compile_with(project, self.limit);
        self.compiles.fetch_add(1, Ordering::Relaxed);
        if let Err(error) = set.get(project.calendar) {
            // Une fois par génération (le résultat est gardé) ; le moteur, de
            // même budget, a déjà émis son diagnostic (panneau des
            // diagnostics).
            log::warn!("jours chômés non grisés : calendrier du projet indisponible ({error})");
        }
        let set = Arc::new(set);
        *slot = Some((generation, Arc::clone(&set)));
        set
    }

    /// Nombre de compilations faites depuis la création du cache.
    #[must_use]
    pub fn compiles(&self) -> u64 {
        self.compiles.load(Ordering::Relaxed)
    }
}

impl Default for CalendarCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CalendarCache {
    /// Copie vide, au même budget : la copie recompilera à sa première
    /// demande (un document cloné peut ensuite diverger).
    fn clone(&self) -> Self {
        Self::with_limit(self.limit)
    }
}

/// Calendrier du projet compilé, si sa compilation a réussi (repli : aucun
/// grisé).
#[must_use]
pub fn project_calendar<'a>(set: &'a CalendarSet, project: &Project) -> Option<&'a CompiledCalendar> {
    set.get(project.calendar).ok()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "tests : un échec doit s'arrêter net et lisiblement"
    )]

    use super::{CalendarCache, OpenError, load, project_calendar};

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

    /// N3 : une compilation par génération ; le refus du budget laisse
    /// l'ensemble sans calendrier du projet, sans panique.
    #[test]
    fn n3_calendar_cache_once_per_generation() {
        let doc = load(crate::DEMO_MSPDI, now()).unwrap();
        let project = doc.session.project();
        let cache = CalendarCache::new();
        let first = cache.get(project, 0);
        let again = cache.get(project, 0);
        assert!(std::sync::Arc::ptr_eq(&first, &again));
        assert_eq!(cache.compiles(), 1);
        assert!(project_calendar(&first, project).is_some());
        let next = cache.get(project, 1);
        assert_eq!(cache.compiles(), 2);
        assert!(!std::sync::Arc::ptr_eq(&first, &next));
        // Le clone repart vide.
        assert_eq!(cache.clone().compiles(), 0);
        let tiny = CalendarCache::with_limit(1);
        let set = tiny.get(project, 0);
        assert!(project_calendar(&set, project).is_none());
        tiny.get(project, 0);
        assert_eq!(tiny.compiles(), 1);
    }
}
