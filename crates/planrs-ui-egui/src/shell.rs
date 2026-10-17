//! Enveloppe eframe : seul type qui implémente `eframe::App`.
//!
//! Lit l'état d'interface au démarrage et l'écrit à la sauvegarde, en JSON
//! (clé [`STATE_KEY`]). La mémoire interne d'egui n'est jamais persistée :
//! notre état est la seule source. Le stockage n'existe que si l'application
//! active la feature `persistence` d'eframe (desktop et web).

use crate::app::{App, Env};
use crate::state::{STATE_KEY, UiState};

/// Application eframe de planrs, commune au desktop et au web.
pub struct PlanrsApp {
    /// Application pure.
    app: App,
}

impl PlanrsApp {
    /// Crée l'application : état relu depuis le stockage d'eframe s'il
    /// existe (lecture tolérante, avertissement affiché une fois).
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>, env: Env) -> Self {
        let stored = cc.storage.and_then(|storage| storage.get_string(STATE_KEY));
        let (state, warning) = stored.map_or_else(
            || (UiState::default(), None),
            |json| UiState::from_json_lenient(&json),
        );
        let mut app = App::new(state, env);
        if let Some(warning) = warning {
            app.report_state_load(&warning);
        }
        Self { app }
    }

    /// Application pure enveloppée.
    #[must_use]
    pub const fn app(&self) -> &App {
        &self.app
    }
}

impl eframe::App for PlanrsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::ui(ui, &mut self.app);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        match self.app.state.to_json() {
            Ok(json) => storage.set_string(STATE_KEY, json),
            Err(error) => log::warn!("état d'interface non sauvegardé : {error}"),
        }
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }
}
