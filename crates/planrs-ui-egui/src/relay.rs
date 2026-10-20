//! Relais des appels d'un agent côté application (J7, ADR-0015 D9 à D11 ;
//! lot L3) : file des `planrs_automation::control::RelayCall` vidée par le
//! réducteur en début d'image, exécution par
//! `planrs_automation::call_with` sur la session du document, validation de
//! toute écriture par le bandeau (H-UI-27), `app.ui_state` et `app.ui`.
//!
//! Contrat J7 (lot L0) : module vide ; le web n'a pas de canal.
