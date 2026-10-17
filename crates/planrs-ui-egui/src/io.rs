//! Ouverture de fichiers sans runtime asynchrone.
//!
//! Le réducteur d'`App` appelle [`FileService::pick_and_open`] ; le résultat
//! revient par un canal `std::sync::mpsc`, relevé (`try_recv`) au début de
//! l'image suivante. Natif : `rfd::FileDialog` synchrone sur le fil
//! d'interface (motif sûr sur macOS ; l'image est gelée pendant la boîte
//! modale), puis `std::fs::read` et [`document::load`]. Web :
//! `rfd::AsyncFileDialog` (téléversement par un `<input type=file>` généré
//! par rfd) lancé par `wasm_bindgen_futures::spawn_local`. Jamais de
//! `std::thread::spawn` (panique en wasm32).

use std::sync::mpsc::Sender;

use planrs_core::Moment;

use crate::app::DocOrigin;
#[cfg(not(target_arch = "wasm32"))]
use crate::document::OpenError;
use crate::document::{self, LoadedDocument};

/// Résultat d'une demande d'ouverture.
#[derive(Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "un seul événement par ouverture, consommé aussitôt : une boîte n'apporterait rien"
)]
pub enum OpenEvent {
    /// L'utilisateur a refermé le sélecteur sans choisir.
    Cancelled,
    /// Un fichier a été lu : document chargé ou erreur.
    Opened {
        /// Provenance du fichier.
        origin: DocOrigin,
        /// Document chargé, ou raison de l'échec.
        result: Result<LoadedDocument, document::OpenError>,
    },
}

/// Accès aux fichiers, injecté par l'application (`Env::files`).
pub trait FileService {
    /// Ouvre le sélecteur de fichiers et envoie le résultat par `tx`, puis
    /// demande une nouvelle image à `ctx`. `title` et `filter` sont déjà
    /// traduits ; `now` est la date courante injectée pour le lecteur MSPDI.
    fn pick_and_open(
        &mut self,
        ctx: &egui::Context,
        now: Moment,
        title: String,
        filter: String,
        tx: Sender<OpenEvent>,
    );
}

/// Sélecteur réel : rfd (natif synchrone, web asynchrone).
#[derive(Clone, Copy, Debug, Default)]
pub struct RfdFileService;

/// Sélecteur inerte (tests) : n'ouvre rien et répond aussitôt
/// [`OpenEvent::Cancelled`], comme un utilisateur qui referme la boîte.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFileService;

/// Envoie un événement ; une application déjà détruite est ignorée.
fn deliver(tx: &Sender<OpenEvent>, ctx: &egui::Context, event: OpenEvent) {
    if tx.send(event).is_err() {
        log::warn!("ouverture : l'application ne reçoit plus les résultats");
    }
    ctx.request_repaint();
}

impl FileService for RfdFileService {
    #[cfg(not(target_arch = "wasm32"))]
    fn pick_and_open(
        &mut self,
        ctx: &egui::Context,
        now: Moment,
        title: String,
        filter: String,
        tx: Sender<OpenEvent>,
    ) {
        let picked = rfd::FileDialog::new()
            .set_title(title)
            .add_filter(filter, &["xml"])
            .pick_file();
        let event = match picked {
            None => OpenEvent::Cancelled,
            Some(path) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let origin = DocOrigin {
                    name,
                    path: path.to_str().map(str::to_owned),
                    demo: false,
                };
                let result = std::fs::read(&path)
                    .map_err(|error| OpenError::Read(error.to_string()))
                    .and_then(|bytes| document::load(&bytes, now));
                OpenEvent::Opened { origin, result }
            }
        };
        deliver(&tx, ctx, event);
    }

    #[cfg(target_arch = "wasm32")]
    fn pick_and_open(
        &mut self,
        ctx: &egui::Context,
        now: Moment,
        title: String,
        filter: String,
        tx: Sender<OpenEvent>,
    ) {
        let ctx = ctx.clone();
        // Lancé dans la même image que le clic : l'activation transitoire du
        // navigateur est encore valide pour ouvrir le sélecteur.
        let dialog = rfd::AsyncFileDialog::new()
            .set_title(title)
            .add_filter(filter, &["xml"]);
        wasm_bindgen_futures::spawn_local(async move {
            let event = match dialog.pick_file().await {
                None => OpenEvent::Cancelled,
                Some(handle) => {
                    let origin = DocOrigin {
                        name: handle.file_name(),
                        path: None,
                        demo: false,
                    };
                    // `read` rend un vecteur vide en cas d'échec : le lecteur
                    // MSPDI le refuse proprement (« document vide »).
                    let bytes = handle.read().await;
                    OpenEvent::Opened {
                        origin,
                        result: document::load(&bytes, now),
                    }
                }
            };
            deliver(&tx, &ctx, event);
        });
    }
}

impl FileService for NoFileService {
    fn pick_and_open(
        &mut self,
        ctx: &egui::Context,
        _now: Moment,
        _title: String,
        _filter: String,
        tx: Sender<OpenEvent>,
    ) {
        deliver(&tx, ctx, OpenEvent::Cancelled);
    }
}
