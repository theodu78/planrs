//! Application desktop de planrs (macOS, Windows, Linux).
//!
//! Ouvre une fenêtre eframe titrée avec le nom du produit et y affiche
//! l'interface de `planrs-ui-egui`. C'est ici seulement que l'horloge et la
//! langue du système sont lues, puis injectées (`Env`). Une erreur
//! d'initialisation graphique est affichée sur la sortie d'erreur et rend un
//! code de sortie non nul : jamais de panique (règle 10).

use std::process::ExitCode;

use planrs_core::{Moment, PRODUCT_NAME};
use planrs_ui_egui::{Env, Language, PlanrsApp, RfdFileService};

/// « Maintenant » en heure locale, tronqué à la minute. Hors du domaine des
/// dates du cœur (1984-2149), repli sur un instant fixe, signalé.
fn now() -> Moment {
    match Moment::from_naive_floor(chrono::Local::now().naive_local()) {
        Ok((moment, _truncated)) => moment,
        Err(error) => {
            eprintln!(
                "{PRODUCT_NAME} : date système inutilisable ({error}), repli sur {}",
                Moment::MIN
            );
            Moment::MIN
        }
    }
}

fn main() -> ExitCode {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(PRODUCT_NAME)
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        PRODUCT_NAME,
        options,
        Box::new(|cc| {
            let env = Env {
                now: Box::new(now),
                system_language: Language::from_system(),
                files: Box::new(RfdFileService),
            };
            Ok(Box::new(PlanrsApp::new(cc, env)))
        }),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{PRODUCT_NAME} : impossible de démarrer l'interface : {error}");
            ExitCode::FAILURE
        }
    }
}
