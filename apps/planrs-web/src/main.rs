//! Version web de planrs : même interface que le desktop, compilée en
//! `wasm32-unknown-unknown` et servie par trunk (`trunk serve` dans ce
//! dossier).
//!
//! Aucun JavaScript écrit à la main (règle 8) : trunk génère la colle
//! wasm-bindgen, `index.html` ne contient que le canevas. La CI vérifie la
//! compilation en `wasm32-unknown-unknown` (clippy, sans trunk) ; seul
//! `trunk build` attend le packaging web. Seul le squelette existe.
//!
//! Compilée pour une cible native (ce que fait `cargo check --workspace`), le
//! binaire se contente d'indiquer qu'il faut passer par trunk.

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::wasm_bindgen::{JsCast as _, JsValue};
    use planrs_ui_egui::PlanrsApp;

    /// Identifiant du canevas déclaré dans `index.html`.
    const CANVAS_ID: &str = "planrs_canvas";

    /// Point d'entrée appelé par la colle wasm-bindgen au chargement.
    pub fn main() {
        // Un échec d'initialisation du journal n'empêche pas l'application de tourner.
        let _ = eframe::WebLogger::init(log::LevelFilter::Info);
        wasm_bindgen_futures::spawn_local(async {
            if let Err(error) = start().await {
                log::error!("{} : échec du démarrage : {error:?}", planrs_core::PRODUCT_NAME);
            }
        });
    }

    async fn start() -> Result<(), JsValue> {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or_else(|| JsValue::from_str("document introuvable"))?;
        document.set_title(planrs_core::PRODUCT_NAME);
        let canvas = document
            .get_element_by_id(CANVAS_ID)
            .ok_or_else(|| JsValue::from_str("canevas introuvable"))?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("l'élément n'est pas un canevas"))?;
        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(PlanrsApp::new(cc)))),
            )
            .await
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    web::main();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    eprintln!(
        "{} (web) se construit avec trunk pour la cible wasm32-unknown-unknown : `trunk serve` dans apps/planrs-web.",
        planrs_core::PRODUCT_NAME
    );
    std::process::ExitCode::FAILURE
}
