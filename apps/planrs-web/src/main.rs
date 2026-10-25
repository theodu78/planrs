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
//! Le même observateur tient un drapeau « modifié » ([`UnsavedFlag`]) : tant
//! qu'il est levé, l'écouteur `beforeunload` demande confirmation avant de
//! fermer ou de recharger l'onglet (relecture finale J4, F3).
//!
//! Au démarrage, `<html>` reçoit `data-planrs-started="1"` : la
//! vérification de la release y lit la preuve que le wasm a tourné
//! (C-REL-06, `docs/features/release.md`).
//!
//! Compilée pour une cible native (ce que fait `cargo check --workspace`), le
//! binaire se contente d'indiquer qu'il faut passer par trunk.

use std::cell::Cell;
use std::rc::Rc;

use planrs_ui_egui::{DocumentObserver, OpenDocument};

/// Drapeau partagé « le document ouvert est modifié », lu par l'écouteur
/// `beforeunload` et tenu par l'observateur du document.
#[cfg_attr(
    not(any(target_arch = "wasm32", test)),
    allow(dead_code, reason = "utilisé par la seule cible wasm32 (et les tests)")
)]
#[derive(Clone, Debug, Default)]
pub struct UnsavedFlag(Rc<Cell<bool>>);

#[cfg_attr(
    not(any(target_arch = "wasm32", test)),
    allow(dead_code, reason = "utilisé par la seule cible wasm32 (et les tests)")
)]
impl UnsavedFlag {
    /// Le document courant a des modifications non enregistrées.
    #[must_use]
    pub fn is_set(&self) -> bool {
        self.0.get()
    }
}

impl DocumentObserver for UnsavedFlag {
    fn document_changed(&mut self, document: Option<&OpenDocument>) {
        self.0.set(document.is_some_and(OpenDocument::is_dirty));
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::wasm_bindgen::closure::Closure;
    use eframe::wasm_bindgen::{JsCast as _, JsValue};
    use planrs_core::Moment;
    use planrs_ui_egui::app::window_title_for;
    use planrs_ui_egui::{DocumentObserver, Env, Language, OpenDocument, PlanrsApp, RfdFileService};

    use super::UnsavedFlag;

    /// Identifiant du canevas déclaré dans `index.html`.
    const CANVAS_ID: &str = "planrs_canvas";

    /// Attribut posé sur `<html>` au démarrage du wasm (C-REL-06).
    const STARTED_ATTR: &str = "data-planrs-started";

    /// Observateur du document : titre de l'onglet et drapeau « modifié »
    /// tenus à jour (ouverture, édition, enregistrement, fermeture).
    pub struct TabObserver {
        /// Document HTML de la page.
        document: web_sys::Document,
        /// Drapeau lu par l'écouteur `beforeunload`.
        unsaved: UnsavedFlag,
    }

    impl DocumentObserver for TabObserver {
        fn document_changed(&mut self, document: Option<&OpenDocument>) {
            self.document.set_title(&window_title_for(document));
            self.unsaved.document_changed(document);
        }
    }

    /// Écouteur `beforeunload` : fermer ou recharger l'onglet d'un document
    /// modifié demande confirmation (boîte du navigateur). L'écouteur vit
    /// autant que la page (`forget`).
    fn guard_unload(window: &web_sys::Window, unsaved: UnsavedFlag) -> Result<(), JsValue> {
        let listener = Closure::<dyn FnMut(web_sys::BeforeUnloadEvent)>::new(
            move |event: web_sys::BeforeUnloadEvent| {
                if unsaved.is_set() {
                    event.prevent_default();
                    // Navigateurs plus anciens : une valeur non vide.
                    event.set_return_value(planrs_core::PRODUCT_NAME);
                }
            },
        );
        window.add_event_listener_with_callback("beforeunload", listener.as_ref().unchecked_ref())?;
        listener.forget();
        Ok(())
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
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("window introuvable"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("document introuvable"))?;
        document.set_title(planrs_core::PRODUCT_NAME);
        // C-REL-06 (J8, lot L1) : preuve que le wasm a tourné, lue dans le
        // DOM par la vérification de la release (`chromium --headless`),
        // posée avant l'initialisation graphique (absente en CI).
        if let Some(root) = document.document_element() {
            root.set_attribute(STARTED_ATTR, "1")?;
        }
        let unsaved = UnsavedFlag::default();
        guard_unload(&window, unsaved.clone())?;
        let tab = TabObserver {
            document: document.clone(),
            unsaved,
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use planrs_automation::Batch;
    use planrs_ui_egui::{App, EditOrigin, Env};

    use super::UnsavedFlag;

    /// Relecture finale J4, F3 : le drapeau lu par `beforeunload` suit le
    /// document (levé par une édition, baissé par l'annulation et la
    /// fermeture).
    #[test]
    fn web_unsaved_flag_follows_document() {
        let flag = UnsavedFlag::default();
        let mut app = App::new(
            planrs_ui_egui::UiState::default(),
            Env::fixed("2027-01-15T08:00".parse().unwrap()),
        );
        app.set_observer(Box::new(flag.clone()));
        app.open_demo();
        assert!(!flag.is_set());
        let batch =
            Batch::from_json_str(r#"{"format_version":1,"ops":[{"op":"task.update","uid":3,"name":"X"}]}"#)
                .unwrap();
        app.execute(&batch, EditOrigin::User).unwrap();
        assert!(flag.is_set());
        app.undo().unwrap();
        assert!(!flag.is_set());
        app.execute(&batch, EditOrigin::User).unwrap();
        app.open_demo();
        assert!(!flag.is_set());
    }
}
