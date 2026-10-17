//! Interface egui de planrs : table des tâches, Gantt, vues secondaires,
//! état d'interface sérialisable et canal de contrôle.
//!
//! C'est la seule crate (avec les applications) autorisée à dépendre d'egui,
//! eframe, winit ou rfd (règle 9). Elle est partagée par l'application
//! desktop (`apps/planrs`) et la version web (`apps/planrs-web`).
//!
//! État : squelette du jalon J1, une fenêtre vide.

use planrs_core::PRODUCT_NAME;

/// Application egui de planrs, commune au desktop et au web.
#[derive(Debug, Default)]
pub struct PlanrsApp;

impl PlanrsApp {
    /// Crée l'application à partir du contexte de création d'eframe.
    #[must_use]
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self
    }
}

impl eframe::App for PlanrsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(PRODUCT_NAME);
            ui.label("Pré-alpha : rien d'utilisable pour l'instant.");
        });
    }
}
