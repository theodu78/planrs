//! Placement d'une feuille affectée, affectation par affectation (ADR-0013
//! D1 ; règles R-170, R-171, R-173 à R-177). Squelette du contrat J5 (lot
//! L0), rempli par le lot L4a.
//!
//! Contenu prévu :
//! - calendrier effectif de chaque affectation ([`super::EffectiveCalendar`],
//!   R-171) : intersection compilée du calendrier de la tâche et de celui
//!   de la ressource (`CompiledCalendar::intersect`, R-172), sinon l'un des
//!   deux, sinon le calendrier du projet ;
//! - `d_a` = `round_div(W × 10 000, U)` borné au domaine, sauf durée fixe ou
//!   unités nulles (`d_a` = durée de la tâche, R-170) ;
//! - `s_a` = ES + retard de nivellement de la tâche + `Delay` + retard de
//!   nivellement de l'affectation (R-177, R-178), `f_a` placé sur le
//!   calendrier effectif ; fin de la tâche = max `f_a`, durée calculée =
//!   mesure de l'union des intervalles travaillés (R-173) ;
//! - passe arrière et contraintes de fin : min sur les affectations (R-174) ;
//! - affectations neutres (R-176) et tâches sans affectation de travail
//!   (R-175).
//!
//! Le résultat est rangé dans [`super::Schedule::assignments`].
