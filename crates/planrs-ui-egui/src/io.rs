//! Ouverture de fichiers sans runtime asynchrone.
//!
//! Le réducteur d'`App` appelle [`FileService::pick_and_open`] ; le résultat
//! revient par un canal `std::sync::mpsc`, relevé (`try_recv`) au début de
//! l'image suivante. Natif : `rfd::FileDialog` synchrone sur le fil
//! d'interface (motif sûr sur macOS ; l'image est gelée pendant la boîte
//! modale), puis `std::fs::read` et [`document::load`] ; `FileHandle::read`
//! de rfd n'est jamais utilisé (il fait `unwrap` en interne). Web :
//! `rfd::AsyncFileDialog` (téléversement par un `<input type=file>` généré
//! par rfd) lancé par `wasm_bindgen_futures::spawn_local`, puis lecture par
//! `Blob::arrayBuffer` (promesse rejetée → erreur traduite). Tout chemin qui
//! se termine envoie un [`OpenEvent`]. Jamais de `std::thread::spawn`
//! (panique en wasm32).

use std::sync::mpsc::Sender;

use planrs_core::Moment;

use crate::app::DocOrigin;
use crate::document::{self, LoadedDocument, OpenError};

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
        // Chemin lu par `std::fs::read`, jamais par `FileHandle::read`.
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
        // Le réducteur s'exécute en fin d'image (dans `requestAnimationFrame`,
        // pas dans le gestionnaire du clic) et `pick_file` ne crée le
        // `<input type=file>` que dans le futur : le navigateur l'accepte grâce
        // à la fenêtre d'activation transitoire qui suit le clic (environ 5 s
        // dans Chromium). Hypothèse, à vérifier sur Safari.
        let dialog = rfd::AsyncFileDialog::new()
            .set_title(title)
            .add_filter(filter, &["xml"]);
        wasm_bindgen_futures::spawn_local(async move {
            // Annulation : rfd résout la promesse à `None` sur l'événement
            // `cancel` du sélecteur. Un navigateur qui n'émet pas `cancel`
            // (Safari < 16.4) ne résout jamais : aucun événement n'arrive,
            // et `App` relance une ouverture demandée pendant l'attente.
            let event = match dialog.pick_file().await {
                None => OpenEvent::Cancelled,
                Some(handle) => {
                    let origin = DocOrigin {
                        name: handle.file_name(),
                        path: None,
                        demo: false,
                    };
                    // Lecture par `Blob::arrayBuffer` : un échec (fichier
                    // supprimé, permission retirée) rejette la promesse et
                    // devient `OpenError::Read` ; un contenu vide est refusé
                    // par le lecteur MSPDI (`OpenError::Mspdi`). Jamais
                    // `FileHandle::read`, qui n'écoute que `onload` (promesse
                    // jamais résolue en cas d'échec).
                    let result = read_web_file(&handle)
                        .await
                        .and_then(|bytes| document::load(&bytes, now));
                    OpenEvent::Opened { origin, result }
                }
            };
            deliver(&tx, &ctx, event);
        });
    }
}

/// Lit le contenu d'un fichier choisi dans le navigateur
/// (`Blob::arrayBuffer`). Toute exception JavaScript devient
/// `OpenError::Read` avec son message.
#[cfg(target_arch = "wasm32")]
async fn read_web_file(handle: &rfd::FileHandle) -> Result<Vec<u8>, OpenError> {
    // `inner` rend le `web_sys::File` d'un descripteur obtenu par
    // `pick_file` (toujours lisible).
    let promise = handle.inner().array_buffer();
    match wasm_bindgen_futures::JsFuture::from(promise).await {
        Ok(buffer) => Ok(js_sys::Uint8Array::new(&buffer).to_vec()),
        Err(error) => Err(OpenError::Read(js_error_text(&error))),
    }
}

/// Message d'une exception JavaScript (`error.message`), sinon sa forme de
/// débogage.
#[cfg(target_arch = "wasm32")]
fn js_error_text(error: &wasm_bindgen_futures::wasm_bindgen::JsValue) -> String {
    js_sys::Reflect::get(error, &js_sys::JsString::from("message"))
        .ok()
        .and_then(|message| message.as_string())
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| format!("{error:?}"))
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
