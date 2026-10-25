//! Application desktop de planrs (macOS, Windows, Linux).
//!
//! Ouvre une fenêtre eframe titrée avec le nom du produit et y affiche
//! l'interface de `planrs-ui-egui`. C'est ici seulement que l'horloge et la
//! langue du système sont lues, puis injectées (`Env`). Une erreur
//! d'initialisation graphique est affichée sur la sortie d'erreur et rend un
//! code de sortie non nul : jamais de panique (règle 10).
//!
//! Canal de contrôle (ADR-0002, ADR-0015 D9) : après l'ouverture d'un fichier
//! par chemin, l'application lance l'écouteur du relais (une fois) et publie
//! son rendez-vous : `planrs mcp` s'y rattache et relaie ses appels, que
//! l'application exécute sur la session du document (toute écriture après
//! validation par le bandeau) ; en `--headless`, il refuse d'écrire le même
//! fichier (`file.open_in_app`). Les appels arrivent dans une file bornée
//! vidée par l'interface en début d'image ; l'écouteur la réveille par
//! `request_repaint`.
//!
//! Ligne de commande (J7, lot L14) : `planrs-desktop [plan.xml]` ouvre le
//! fichier au démarrage ([`startup_path`], chemin rendu absolu pour les
//! fichiers récents) ; un échec s'affiche dans le bandeau de l'interface,
//! nom du fichier compris. La capture relayée (`app.screenshot`) est
//! permise ici seulement ([`App::enable_screenshots`]).
//!
//! Polices : eframe est pris avec ses features par défaut, dont
//! `system_fonts` ; `NativeOptions::system_font_fallback` reste vrai
//! ([`native_options`]) : un caractère absent des polices d'egui (CJK,
//! arabe…) est cherché dans les polices du système (fontconfig, CoreText,
//! DirectWrite), sans police embarquée (ADR-0015, décision J8).
//!
//! Reprise après plantage (J7, lot L15) : au démarrage, les journaux
//! orphelins du dossier de reprise (`recovery_dir`) qui portent des
//! modifications sont proposés par un bandeau (Restaurer, Ignorer, Plus
//! tard), les autres supprimés ; puis chaque document ouvert tient son
//! journal ([`Recorder`]), supprimé à sa fermeture propre. Un plantage, ou
//! « Quitter » du Dock sur macOS (non interceptable, voir plus bas), laisse
//! le journal : rien n'est perdu au démarrage suivant.
//!
//! macOS : le menu applicatif par défaut de winit est retiré
//! ([`event_loop_hook`]). Son entrée « Quitter » (Cmd+Q) appelle
//! `terminate:`, qui ferme la fenêtre sans `CloseRequested` : le garde des
//! modifications non enregistrées ne la voyait pas. Sans ce menu, Cmd+Q
//! arrive comme une touche et passe par le `KEYMAP` (confirmation). « Quitter »
//! du Dock et la fermeture de session appellent toujours `terminate:` : non
//! interceptables sans délégué Objective-C (limite, docs/features/ui.md § 5).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

use planrs_automation::control::{
    ControlError, ListenerHandle, RELAY_QUEUE, RelayCall, Rendezvous, RendezvousGuard, Waker,
    canonical_project, new_token, publish_in, spawn_listener_in, state_dir,
};
use planrs_automation::recovery::{Recorder, orphans, recovery_dir};
use planrs_core::{Moment, PRODUCT_NAME};
use planrs_ui_egui::relay::ScreenshotConfig;
use planrs_ui_egui::{App, DocumentObserver, Env, Language, OpenDocument, PlanrsApp, RfdFileService};

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
struct ControlRendezvous {
    /// Dossier des rendez-vous ; `None` : celui de la plateforme
    /// (`state_dir`, lu au premier besoin). Imposé par les tests.
    dir: Option<PathBuf>,
    /// File des appels relayés, vidée par l'application.
    inbox: SyncSender<RelayCall>,
    /// Réveil de l'interface à chaque appel.
    wake: Waker,
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
    /// Numéro d'ouverture du dernier document vu : un document neuf change
    /// le jeton de l'écouteur (relecture finale J7, relais F2).
    opening: Option<u64>,
}

impl ControlRendezvous {
    /// Rendez-vous sans écouteur, et la file que l'application vide.
    fn new(wake: Waker) -> (Self, Receiver<RelayCall>) {
        let (inbox, rx) = sync_channel(RELAY_QUEUE);
        let control = Self {
            dir: None,
            inbox,
            wake,
            guard: None,
            listener: None,
            listener_failed: false,
            path: None,
            opening: None,
        };
        (control, rx)
    }

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
            let inbox = self.inbox.clone();
            let wake = Arc::clone(&self.wake);
            let started = self
                .dir()
                .and_then(|dir| new_token().and_then(|token| spawn_listener_in(&dir, token, inbox, wake)));
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
    /// Rendez-vous du chemin du document. Un document **neuf** (autre
    /// numéro d'ouverture : autre fichier, même fichier rouvert, « Nouveau »)
    /// change le jeton de l'écouteur avant la publication : un agent
    /// rattaché au document précédent est refusé
    /// (`control.document_changed`) au lieu d'agir sur celui-ci par les
    /// mêmes UID ; « Enregistrer sous » (même ouverture) garde le jeton.
    fn document_changed(&mut self, document: Option<&OpenDocument>) {
        let path = document.and_then(|document| document.origin.path.as_deref());
        let opening = document.map(|document| document.opening);
        let mut rotated = false;
        if opening.is_some() && opening != self.opening {
            if self.opening.is_some()
                && let Some(listener) = &mut self.listener
            {
                match listener.rotate_token() {
                    Ok(()) => rotated = true,
                    Err(error) => eprintln!("{PRODUCT_NAME} : jeton du canal non renouvelé ({error})"),
                }
            }
            self.opening = opening;
        }
        if rotated || path != self.path.as_deref() {
            self.publish_for(path);
        }
    }
}

/// Réglage de la boucle d'événements : sur macOS, pas de menu applicatif
/// par défaut (voir l'en-tête du module) ; rien ailleurs.
#[cfg_attr(
    target_os = "macos",
    allow(
        clippy::unnecessary_wraps,
        reason = "`None` hors macOS : même signature sur toutes les plateformes"
    )
)]
fn event_loop_hook() -> Option<eframe::EventLoopBuilderHook> {
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::EventLoopBuilderExtMacOS as _;
        Some(Box::new(|builder| {
            builder.with_default_menu(false);
        }))
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Reprise après plantage : orphelins de `root` proposés (ceux sans
/// modification supprimés), puis journal de chaque document dans `root`.
fn start_recovery(app: &mut App, root: PathBuf) {
    let (offered, stale): (Vec<_>, Vec<_>) = orphans(&root)
        .into_iter()
        .partition(planrs_automation::recovery::Orphan::worth_restoring);
    for orphan in stale {
        if let Err(error) = orphan.discard() {
            eprintln!("{PRODUCT_NAME} : journal de reprise non supprimé ({error})");
        }
    }
    app.offer_recovery(offered);
    app.set_recovery(Box::new(Recorder::in_dir(root)));
}

/// Usage affiché pour un argument refusé.
const USAGE: &str = "usage : planrs-desktop [fichier.xml]";

/// Fichier à ouvrir au démarrage : premier argument, rendu absolu (les
/// fichiers récents restent valables quel que soit le dossier de
/// lancement). Aucun argument : `None`.
///
/// # Errors
///
/// Option inconnue, chemin non UTF-8 ou plus d'un argument : message à
/// afficher (l'application démarre quand même, sans fichier).
fn startup_path(mut args: impl Iterator<Item = std::ffi::OsString>) -> Result<Option<String>, String> {
    let Some(first) = args.next() else {
        return Ok(None);
    };
    if args.next().is_some() {
        return Err(format!("un seul fichier attendu\n{USAGE}"));
    }
    let path = PathBuf::from(&first);
    if first.to_str().is_some_and(|text| text.starts_with('-')) {
        return Err(format!("option inconnue : {}\n{USAGE}", path.display()));
    }
    let absolute = std::path::absolute(&path).unwrap_or(path);
    absolute
        .into_os_string()
        .into_string()
        .map(Some)
        .map_err(|raw| format!("chemin non UTF-8 refusé : {}", PathBuf::from(raw).display()))
}

/// Options de la fenêtre : titre, tailles, boucle d'événements ; repli des
/// polices du système gardé (`system_font_fallback`, défaut d'eframe).
fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(PRODUCT_NAME)
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([800.0, 500.0]),
        event_loop_builder: event_loop_hook(),
        ..Default::default()
    }
}

fn main() -> ExitCode {
    let startup = startup_path(std::env::args_os().skip(1)).unwrap_or_else(|error| {
        eprintln!("{PRODUCT_NAME} : {error}");
        None
    });
    let options = native_options();
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
            let ctx = cc.egui_ctx.clone();
            let (control, inbox) = ControlRendezvous::new(Arc::new(move || ctx.request_repaint()));
            app.app_mut().control_inbox(inbox);
            app.app_mut().enable_screenshots(ScreenshotConfig::default());
            match recovery_dir() {
                Some(root) => start_recovery(app.app_mut(), root),
                None => {
                    eprintln!("{PRODUCT_NAME} : reprise après plantage indisponible (aucun dossier d'état)");
                }
            }
            app.app_mut().set_observer(Box::new(control));
            // Après l'observateur : le rendez-vous suit le fichier ouvert.
            if let Some(path) = &startup {
                app.app_mut().open_path(path);
            }
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

    use super::{ControlRendezvous, startup_path};

    /// macOS : le menu par défaut de winit (« Quitter » = `terminate:`, sans
    /// `CloseRequested`) est retiré ; ailleurs, aucun réglage.
    #[test]
    fn desktop_event_loop_hook_only_on_macos() {
        assert_eq!(super::event_loop_hook().is_some(), cfg!(target_os = "macos"));
    }

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

    /// Relecture finale J7, relais F2 : un document neuf (autre ouverture)
    /// change le jeton publié ; « Enregistrer sous » (même ouverture, autre
    /// chemin) le garde.
    #[cfg(unix)]
    #[test]
    fn desktop_new_document_rotates_token() {
        let root = std::env::temp_dir().join(format!("planrs-rot-{}", std::process::id()));
        let dir = root.join("control");
        std::fs::create_dir_all(&root).unwrap();
        let first = root.join("a.xml");
        let second = root.join("b.xml");
        for file in [&first, &second] {
            std::fs::write(file, planrs_ui_egui::DEMO_MSPDI).unwrap();
        }
        let (mut observer, _inbox) = ControlRendezvous::new(std::sync::Arc::new(|| {}));
        observer.dir = Some(dir.clone());
        let doc = demo_at(&first);
        observer.document_changed(Some(&doc));
        let token = find_live_for_in(&dir, &first).unwrap().token;
        // Même fichier, autre ouverture : nouveau jeton.
        let mut reopened = demo_at(&first);
        reopened.opening = 2;
        observer.document_changed(Some(&reopened));
        let rotated = find_live_for_in(&dir, &first).unwrap().token;
        assert_ne!(rotated, token);
        // « Enregistrer sous » : même ouverture, autre chemin, même jeton.
        let mut saved_as = reopened.clone();
        saved_as.set_origin(planrs_ui_egui::DocOrigin {
            name: "b.xml".to_owned(),
            path: Some(second.to_str().unwrap().to_owned()),
            demo: false,
        });
        observer.document_changed(Some(&saved_as));
        assert_eq!(find_live_for_in(&dir, &second).unwrap().token, rotated);
        assert!(find_live_for_in(&dir, &first).is_none());
        drop(observer);
        std::fs::remove_dir_all(&root).unwrap();
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
        let (mut observer, _inbox) = ControlRendezvous::new(std::sync::Arc::new(|| {}));
        observer.dir = Some(dir.clone());
        observer.document_changed(Some(&demo_at(&first)));
        assert!(find_live_for_in(&dir, &first).is_some());
        observer.document_changed(Some(&demo_at(&second)));
        assert!(find_live_for_in(&dir, &first).is_none());
        assert!(find_live_for_in(&dir, &second).is_some());
        observer.document_changed(None);
        assert!(find_live_for_in(&dir, &second).is_none());
        drop(observer);
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Arguments : aucun, un chemin (rendu absolu), une option ou deux
    /// fichiers refusés ; le fichier donné s'ouvre (et entre aux fichiers
    /// récents), un fichier illisible s'affiche dans le bandeau avec son nom.
    #[test]
    fn desktop_open_from_argument() {
        use std::ffi::OsString;

        use planrs_ui_egui::{App, Env, UiState};

        assert_eq!(startup_path(std::iter::empty()), Ok(None));
        let relative = startup_path([OsString::from("plan.xml")].into_iter())
            .unwrap()
            .unwrap();
        assert!(Path::new(&relative).is_absolute(), "{relative}");
        assert!(relative.ends_with("plan.xml"));
        assert!(startup_path([OsString::from("--aide")].into_iter()).is_err());
        assert!(startup_path([OsString::from("a.xml"), OsString::from("b.xml")].into_iter()).is_err());

        let root = std::env::temp_dir().join(format!("planrs-l14-arg-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let good = root.join("maison.xml");
        std::fs::write(&good, planrs_ui_egui::DEMO_MSPDI).unwrap();
        let bad = root.join("tronqué.xml");
        std::fs::write(&bad, b"").unwrap();
        let now = "2027-01-15T08:00".parse().unwrap();
        let mut app = App::new(UiState::default(), Env::fixed(now));
        let path = startup_path([good.clone().into_os_string()].into_iter())
            .unwrap()
            .unwrap();
        app.open_path(&path);
        let doc = app.document.as_ref().unwrap();
        assert_eq!(doc.origin.name, "maison.xml");
        assert_eq!(doc.origin.path.as_deref(), Some(path.as_str()));
        assert_eq!(
            app.state.recent.first().map(|r| r.path.as_str()),
            Some(path.as_str())
        );
        assert!(app.messages().is_empty(), "{:?}", app.messages());
        app.open_path(bad.to_str().unwrap());
        assert_eq!(
            app.document.as_ref().unwrap().opening,
            1,
            "le document reste affiché"
        );
        let message = &app.messages().last().unwrap().text;
        assert!(message.contains("tronqué.xml"), "{message}");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Démarrage : un orphelin modifié est proposé, un orphelin sans
    /// modification supprimé ; le document ouvert ensuite tient son journal
    /// (verrou tenu : pas un orphelin), supprimé à sa fermeture.
    #[test]
    fn desktop_start_recovery_offers_and_cleans() {
        use planrs_automation::recovery::{Journal, orphans};
        use planrs_ui_egui::{Action, App, Env, UiState};

        let root = std::env::temp_dir().join(format!("planrs-l15-start-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let now = "2027-01-15T08:00".parse().unwrap();
        let mut session = document::load(planrs_ui_egui::DEMO_MSPDI, now).unwrap().session;
        // Sans modification : supprimé ; base non enregistrée : proposé.
        drop(Journal::create(&root, &session).unwrap());
        session.mark_unsaved();
        drop(Journal::create(&root, &session).unwrap());
        assert_eq!(orphans(&root).len(), 2);

        let mut app = App::new(UiState::default(), Env::fixed(now));
        super::start_recovery(&mut app, root.clone());
        assert_eq!(app.recovery_offers().len(), 1);
        assert!(app.recovery_offers().iter().all(|o| !o.saved));
        assert_eq!(orphans(&root).len(), 1, "orphelin sans modification supprimé");
        app.open_demo();
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            2,
            "journal du document"
        );
        assert_eq!(orphans(&root).len(), 1, "journal vivant vu orphelin");
        app.dispatch(Action::CloseDocument);
        let ctx = eframe::egui::Context::default();
        let mut output = ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
            planrs_ui_egui::ui(ui, &mut app);
        });
        output.textures_delta.clear();
        assert!(app.document.is_none());
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            1,
            "journal supprimé à la fermeture"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Polices : le repli sur les polices du système reste actif (feature
    /// `system_fonts` d'eframe, option par défaut).
    #[test]
    fn desktop_keeps_system_font_fallback() {
        assert!(super::native_options().system_font_fallback);
    }
}
