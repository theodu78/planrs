//! Données de la chronologie (ADR-0017 D7, H-R16) : extrémités, date
//! d'état, graduations et tâches de la frise, avec leurs textes, tirés du
//! planning du moteur (règle 3 : aucune date calculée ici ni dans
//! l'interface). La géométrie est celle de `planrs_render::timeline`.
//!
//! Lot L0 (contrat) : squelette, extrémités du planning seules (sans texte,
//! graduation ni tâche) ; textes, date d'état et éléments en L1.

use planrs_render::style::TimelineSpec;
use planrs_render::timeline::TimelineInput;

use crate::Session;
use crate::text::Language;

/// Données de la frise de la session `s` pour la liste `spec`, textes en
/// `lang`. Un UID absent du projet est ignoré.
#[must_use]
pub fn input(s: &Session, spec: &TimelineSpec, lang: Language) -> TimelineInput {
    let _ = (spec, lang);
    let schedule = s.schedule();
    TimelineInput {
        start: schedule.project_start,
        finish: schedule.project_finish,
        status: None,
        start_label: String::new(),
        finish_label: String::new(),
        status_label: None,
        ticks: Vec::new(),
        items: Vec::new(),
    }
}
