//! Cumuls du travail et surutilisation des ressources (ADR-0013 D6 ; règles
//! R-184 à R-187). Squelette du contrat J5 (lot L0), rempli par le lot L5.
//!
//! Contenu prévu :
//! - travail des tâches ([`super::TaskSchedule::work`]) et des ressources
//!   ([`super::ResourceLoad::work`]) : sommes des affectations de travail
//!   (R-184), ressources matérielles et de coût à part (R-185) ;
//! - surutilisation exacte : débit de chaque affectation = travail ÷ temps
//!   ouvré de son intervalle (unités effectives), balayage des intervalles
//!   travaillés réels, énumération bornée aux grappes d'affectations qui se
//!   chevauchent, capacité = `max_units` (R-186, R-187) ;
//! - sortie bornée à [`super::MAX_OVERLOADS`] intervalles par ressource,
//!   diagnostic `resource.overallocated` (un par ressource).
//!
//! L'utilisation par période (R-188) se calcule à la demande :
//! [`crate::resource::usage`].
