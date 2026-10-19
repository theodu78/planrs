//! Application desktop de planrs (macOS, Windows, Linux).
//!
//! Ouvre une fenêtre eframe titrée avec le nom du produit et y affiche
//! l'interface de `planrs-ui-egui`. C'est ici seulement que l'horloge et la
//! langue du système sont lues, puis injectées (`Env`). Une erreur
//! d'initialisation graphique est affichée sur la sortie d'erreur et rend un
//! code de sortie non nul : jamais de panique (règle 10).
//!
//! Canal de contrôle (ADR-0002, ADR-0012 D9) : après l'ouverture d'un fichier
//! par chemin, l'application lance l'écouteur `hello`/`query` (une fois) et
//! publie son rendez-vous, pour que `planrs mcp` refuse d'écrire le même
//! fichier (`file.open_in_app`). Le résumé servi par `query` est tenu à jour
//! à chaque édition.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};

use planrs_automation::control::{
    ControlError, ListenerHandle, Rendezvous, RendezvousGuard, canonical_project, new_token, publish_in,
    spawn_hello_listener_in, state_dir,
};
use planrs_automation::view::{SummaryView, summary};
use planrs_core::{Moment, PRODUCT_NAME};
use planrs_ui_egui::{DocumentObserver, Env, Language, OpenDocument, PlanrsApp, RfdFileService};

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

/// Rendez-vous du canal de contrôle, suivi du document ouvert.
#[derive(Default)]
struct ControlRendezvous {
    /// Dossier des rendez-vous ; `None` : celui de la plateforme
    /// (`state_dir`, lu au premier besoin). Imposé par les tests.
    dir: Option<PathBuf>,
    /// Résumé servi par `query` (`None` : aucun document).
    summary: Arc<Mutex<Option<SummaryView>>>,
    /// Rendez-vous publié, supprimé à la destruction (avant l'arrêt de
    /// l'écouteur : ordre de déclaration).
    guard: Option<RendezvousGuard>,
    /// Écouteur, lancé à la première ouverture d'un fichier par chemin.
    listener: Option<ListenerHandle>,
    /// L'écouteur n'a pas pu démarrer : pas de nouvel essai (signalé une
    /// fois).
    listener_failed: bool,
    /// Chemin (tel qu'ouvert) du document dont le rendez-vous est publié.
    path: Option<String>,
}

impl ControlRendezvous {
    /// Dossier des rendez-vous (créé au besoin, en 0700, par le canal).
    fn dir(&mut self) -> Result<PathBuf, ControlError> {
        if self.dir.is_none() {
            self.dir = state_dir();
        }
        self.dir.clone().ok_or(ControlError::NoStateDir)
    }

    /// Rendez-vous de l'écouteur pour `project`, l'écouteur étant lancé au
    /// premier besoin (un échec est signalé une fois, sans nouvel essai).
    fn rendezvous(&mut self, project: String) -> Option<Rendezvous> {
        if self.listener.is_none() && !self.listener_failed {
            let summary = Arc::clone(&self.summary);
            let started = self
                .dir()
                .and_then(|dir| new_token().and_then(|token| spawn_hello_listener_in(&dir, token, summary)));
            match started {
                Ok(listener) => self.listener = Some(listener),
                Err(error) => {
                    self.listener_failed = true;
                    eprintln!("{PRODUCT_NAME} : canal de contrôle indisponible ({error})");
                }
            }
        }
        self.listener
            .as_ref()
            .map(|listener| listener.rendezvous(Some(project)))
    }

    /// Publie le rendez-vous de `path` (le précédent est retiré).
    fn publish_for(&mut self, path: Option<&str>) {
        self.guard = None;
        self.path = path.map(str::to_owned);
        let Some(project) = path.and_then(|path| canonical_project(Path::new(path))) else {
            return;
        };
        let Some(rendezvous) = self.rendezvous(project) else {
            return;
        };
        match self.dir().and_then(|dir| publish_in(&dir, &rendezvous)) {
            Ok(guard) => self.guard = Some(guard),
            Err(error) => eprintln!("{PRODUCT_NAME} : rendez-vous non publié ({error})"),
        }
    }
}

impl DocumentObserver for ControlRendezvous {
    fn document_changed(&mut self, document: Option<&OpenDocument>) {
        let current = document.map(|document| summary(&document.session));
        *self.summary.lock().unwrap_or_else(PoisonError::into_inner) = current;
        let path = document.and_then(|document| document.origin.path.as_deref());
        if path != self.path.as_deref() {
            self.publish_for(path);
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
            let mut app = PlanrsApp::new(cc, env);
            app.app_mut().set_observer(Box::new(ControlRendezvous::default()));
            Ok(Box::new(app))
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use std::path::Path;

    use planrs_automation::control::find_live_for_in;
    use planrs_ui_egui::document;
    use planrs_ui_egui::{DocOrigin, DocumentObserver, OpenDocument};

    use super::ControlRendezvous;

    /// Document de l'exemple, ouvert depuis `path`.
    fn demo_at(path: &Path) -> OpenDocument {
        let loaded = document::load(planrs_ui_egui::DEMO_MSPDI, "2027-01-15T08:00".parse().unwrap()).unwrap();
        let origin = DocOrigin {
            name: "maison.xml".to_owned(),
            path: Some(path.to_str().unwrap().to_owned()),
            demo: false,
        };
        OpenDocument::new(loaded, origin, 1)
    }

    /// Le rendez-vous suit le chemin du document : publié à l'ouverture
    /// (résumé tenu), déplacé avec le chemin, retiré à la fermeture.
    #[cfg(unix)]
    #[test]
    fn desktop_rendezvous_follows_document_path() {
        let root = std::env::temp_dir().join(format!("planrs-l11-{}", std::process::id()));
        let dir = root.join("control");
        std::fs::create_dir_all(&root).unwrap();
        let first = root.join("a.xml");
        let second = root.join("b.xml");
        for file in [&first, &second] {
            std::fs::write(file, planrs_ui_egui::DEMO_MSPDI).unwrap();
        }
        let mut observer = ControlRendezvous {
            dir: Some(dir.clone()),
            ..ControlRendezvous::default()
        };
        observer.document_changed(Some(&demo_at(&first)));
        assert!(find_live_for_in(&dir, &first).is_some());
        assert!(observer.summary.lock().unwrap().is_some());
        observer.document_changed(Some(&demo_at(&second)));
        assert!(find_live_for_in(&dir, &first).is_none());
        assert!(find_live_for_in(&dir, &second).is_some());
        observer.document_changed(None);
        assert!(find_live_for_in(&dir, &second).is_none());
        assert!(observer.summary.lock().unwrap().is_none());
        drop(observer);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
