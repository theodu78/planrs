//! Version web de planrs : même interface que le desktop, compilée en
//! `wasm32-unknown-unknown` et servie par trunk (`trunk serve` dans ce
//! dossier).
//!
//! Aucun JavaScript écrit à la main (règle 8) : trunk génère la colle
//! wasm-bindgen, `index.html` ne contient que le canevas. La CI vérifie la
//! compilation en `wasm32-unknown-unknown` (clippy, sans trunk) ; seul
//! `trunk build` attend le packaging web. « Maintenant » vient de la date du
//! navigateur et la langue de `navigator.language`, injectés par `Env`.
//!
//! L'onglet porte le titre de la fenêtre desktop (« *nom — planrs » si le
//! document est modifié) : eframe web ignore `ViewportCommand::Title`, un
//! observateur du document ([`web::TabObserver`]) pose `document.title`.
//!
//! Compilée pour une cible native (ce que fait `cargo check --workspace`), le
//! binaire se contente d'indiquer qu'il faut passer par trunk.

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::wasm_bindgen::{JsCast as _, JsValue};
    use planrs_core::Moment;
    use planrs_ui_egui::app::window_title_for;
    use planrs_ui_egui::{DocumentObserver, Env, Language, OpenDocument, PlanrsApp, RfdFileService};

    /// Identifiant du canevas déclaré dans `index.html`.
    const CANVAS_ID: &str = "planrs_canvas";

    /// Observateur du document : titre de l'onglet tenu à jour (ouverture,
    /// édition, enregistrement, fermeture).
    pub struct TabObserver {
        /// Document HTML de la page.
        document: web_sys::Document,
    }

    impl DocumentObserver for TabObserver {
        fn document_changed(&mut self, document: Option<&OpenDocument>) {
            self.document.set_title(&window_title_for(document));
        }
    }

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

    /// « Maintenant » d'après l'horloge locale du navigateur, tronqué à la
    /// minute ; repli sur un instant fixe (signalé) hors du domaine du cœur.
    fn now() -> Moment {
        let date = js_sys::Date::new_0();
        let naive = i32::try_from(date.get_full_year())
            .ok()
            .zip(date.get_month().checked_add(1))
            .and_then(|(year, month)| chrono::NaiveDate::from_ymd_opt(year, month, date.get_date()))
            .and_then(|day| day.and_hms_opt(date.get_hours(), date.get_minutes(), 0));
        match naive.map(Moment::from_naive_floor) {
            Some(Ok((moment, _truncated))) => moment,
            Some(Err(error)) => {
                log::warn!(
                    "date du navigateur inutilisable ({error}), repli sur {}",
                    Moment::MIN
                );
                Moment::MIN
            }
            None => {
                log::warn!("date du navigateur invalide, repli sur {}", Moment::MIN);
                Moment::MIN
            }
        }
    }

    async fn start() -> Result<(), JsValue> {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or_else(|| JsValue::from_str("document introuvable"))?;
        document.set_title(planrs_core::PRODUCT_NAME);
        let tab = TabObserver {
            document: document.clone(),
        };
        let canvas = document
            .get_element_by_id(CANVAS_ID)
            .ok_or_else(|| JsValue::from_str("canevas introuvable"))?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("l'élément n'est pas un canevas"))?;
        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    let env = Env {
                        now: Box::new(now),
                        system_language: Language::from_system(),
                        files: Box::new(RfdFileService),
                    };
                    let mut app = PlanrsApp::new(cc, env);
                    app.app_mut().set_observer(Box::new(tab));
                    Ok(Box::new(app))
                }),
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
