//! Interface egui de planrs : table des tâches, Gantt, état d'interface
//! sérialisable.
//!
//! C'est la seule crate (avec les applications) autorisée à dépendre d'egui,
//! eframe, winit ou rfd (règle 9). Elle est partagée par l'application
//! desktop (`apps/planrs`), la version web (`apps/planrs-web`) et les tests
//! kittest : tous appellent [`ui`] avec un `&mut egui::Ui` (API d'egui 0.37)
//! et une [`App`] indépendante d'eframe. Le document est une session
//! d'édition de `planrs_automation` : toute modification passe par un lot de
//! commandes du registre ([`Action::Exec`]) ; aucune date n'est calculée ici
//! (règle 3), tout vient de `planrs_core`.

pub mod action;
pub mod app;
pub mod column;
pub mod document;
pub mod format;
pub mod geom;
pub mod i18n;
pub mod io;
pub mod keymap;
pub mod rows;
pub mod shell;
pub mod state;
pub mod theme;
pub mod timescale;
pub mod view;

pub use action::Action;
pub use app::{App, DocOrigin, DocumentObserver, EditOrigin, Env, OpenDocument};
pub use i18n::Language;
pub use io::{FileService, NoFileService, RfdFileService};
pub use shell::PlanrsApp;
pub use state::{LanguagePref, ThemePref, UiState};
pub use timescale::TimeScaleZoom;

/// Projet de démonstration embarqué (construction d'une maison, MSPDI écrit à
/// la main) : bouton « Ouvrir l'exemple » et fixture des tests.
pub const DEMO_MSPDI: &[u8] = include_bytes!("../assets/demo/house.xml");

/// Dessine une image complète de l'application dans `ui` : la même fonction
/// sert au desktop, au web et aux tests (`Harness::build_ui_state`).
pub fn ui(ui: &mut egui::Ui, app: &mut App) {
    app.show(ui);
}
