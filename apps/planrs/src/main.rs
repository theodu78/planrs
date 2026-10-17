//! Application desktop de planrs (macOS, Windows, Linux).
//!
//! Ouvre une fenêtre eframe titrée avec le nom du produit et y affiche
//! l'interface de `planrs-ui-egui`. Une erreur d'initialisation graphique est
//! affichée sur la sortie d'erreur et rend un code de sortie non nul : jamais
//! de panique (règle 10).

use std::process::ExitCode;

use planrs_core::PRODUCT_NAME;
use planrs_ui_egui::PlanrsApp;

fn main() -> ExitCode {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_title(PRODUCT_NAME),
        ..Default::default()
    };
    let result = eframe::run_native(
        PRODUCT_NAME,
        options,
        Box::new(|cc| Ok(Box::new(PlanrsApp::new(cc)))),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{PRODUCT_NAME} : impossible de démarrer l'interface : {error}");
            ExitCode::FAILURE
        }
    }
}
