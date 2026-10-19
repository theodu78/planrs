//! Ouverture et enregistrement de fichiers sans runtime asynchrone.
//!
//! Le réducteur d'`App` appelle [`FileService::pick_and_open`] ; le résultat
//! revient par un canal `std::sync::mpsc`, relevé (`try_recv`) au début de
//! l'image suivante. Natif : `rfd::AsyncFileDialog` créé sur le fil
//! d'interface et attendu par `pollster` dans un fil dédié, qui lit le
//! fichier (`std::fs::read`) et appelle [`document::load`] ; jamais de
//! `rfd::FileDialog` synchrone (sa boucle modale imbriquée fait avorter winit
//! sur macOS, ADR-0011). `FileHandle::read` de rfd n'est jamais utilisé (il
//! fait `unwrap` en interne). Web : `rfd::AsyncFileDialog` (téléversement par
//! un `<input type=file>` généré par rfd) lancé par
//! `wasm_bindgen_futures::spawn_local`, puis lecture par `Blob::arrayBuffer`
//! (promesse rejetée → erreur traduite). Tout chemin qui se termine envoie un
//! [`OpenEvent`]. Le fil dédié n'existe qu'en natif (`std::thread` panique en
//! wasm32).
//!
//! Enregistrement ([`FileService::save`], lot L13) : les octets MSPDI sont
//! produits par `Session::save_bytes` sur le fil d'interface, puis écrits
//! hors de lui. Desktop : chemin connu → `planrs_automation::files::write_atomic`
//! dans le fil `planrs-save` ; sinon `rfd::AsyncFileDialog::save_file` créé
//! sur le fil d'interface et attendu dans ce même fil, qui écrit ensuite.
//! Web : `save_file` (qui rend aussitôt un descripteur) puis
//! `FileHandle::write` sur ce seul descripteur (téléchargement). Tout chemin
//! qui se termine envoie un [`SaveEvent`] ; l'application ne marque le
//! document enregistré qu'à la réception de [`SaveEvent::Saved`].

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

/// Destination d'un enregistrement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveTarget {
    /// Chemin connu (desktop, « Enregistrer » d'un document déjà lié à un
    /// fichier) : écriture directe, sans boîte de dialogue.
    Path(String),
    /// Boîte « Enregistrer sous » (desktop) ou téléchargement (web).
    Dialog {
        /// Nom de fichier proposé (déjà assaini, avec `.xml`).
        suggested_name: String,
        /// Titre de la boîte, traduit.
        title: String,
        /// Libellé du filtre MSPDI, traduit.
        filter: String,
    },
}

/// Demande d'enregistrement : octets à écrire et destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRequest {
    /// Octets MSPDI (`Session::save_bytes`), écrits tels quels.
    pub bytes: Vec<u8>,
    /// Destination.
    pub target: SaveTarget,
    /// Langue des messages d'erreur d'écriture.
    pub language: crate::i18n::Language,
}

/// Résultat d'une demande d'enregistrement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveEvent {
    /// L'utilisateur a refermé la boîte sans choisir : rien n'est écrit.
    Cancelled,
    /// Octets écrits (desktop) ou remis au navigateur (web).
    Saved {
        /// Nom affiché du fichier écrit.
        name: String,
        /// Chemin UTF-8 écrit (desktop) ; `None` sur le web ou pour un
        /// chemin non UTF-8 (le prochain « Enregistrer » redemandera).
        path: Option<String>,
    },
    /// Écriture impossible : détail déjà traduit (cause système comprise).
    Failed(String),
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

    /// Écrit `request.bytes` vers `request.target` et envoie le résultat par
    /// `tx`, puis demande une nouvelle image à `ctx`. Par défaut, rien
    /// n'est écrit et la réponse est [`SaveEvent::Cancelled`], comme un
    /// utilisateur qui referme la boîte : le document reste modifié.
    fn save(&mut self, ctx: &egui::Context, request: SaveRequest, tx: Sender<SaveEvent>) {
        drop(request);
        deliver(&tx, ctx, SaveEvent::Cancelled);
    }
}

/// Sélecteur réel : rfd asynchrone (natif : attendu dans un fil dédié ; web :
/// sur l'exécuteur du navigateur).
#[derive(Clone, Copy, Debug, Default)]
pub struct RfdFileService;

/// Sélecteur inerte (tests) : n'ouvre rien et répond aussitôt
/// [`OpenEvent::Cancelled`], comme un utilisateur qui referme la boîte.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFileService;

/// Envoie un événement ; une application déjà détruite (ou une demande
/// abandonnée) est ignorée.
fn deliver<T>(tx: &Sender<T>, ctx: &egui::Context, event: T) {
    if tx.send(event).is_err() {
        log::warn!("fichiers : l'application ne reçoit plus les résultats");
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
        // Le fil d'attente est créé avant le sélecteur : s'il ne peut pas
        // l'être, l'erreur s'affiche et aucune boîte ne reste ouverte pour
        // rien. Il reçoit le futur du sélecteur par un canal.
        let (dialog_tx, dialog_rx) = std::sync::mpsc::channel::<PickFuture>();
        let thread_ctx = ctx.clone();
        let thread_tx = tx.clone();
        let spawned = std::thread::Builder::new()
            .name("planrs-open".to_owned())
            .spawn(move || {
                // Canal fermé sans futur : demande abandonnée, rien à envoyer.
                let Ok(picked) = dialog_rx.recv() else {
                    return;
                };
                let event = match pollster::block_on(picked) {
                    None => OpenEvent::Cancelled,
                    Some(handle) => open_path(handle.path(), now),
                };
                deliver(&thread_tx, &thread_ctx, event);
            });
        if let Err(error) = spawned {
            let origin = DocOrigin {
                name: String::new(),
                path: None,
                demo: false,
            };
            let result = Err(OpenError::Read(error.to_string()));
            deliver(&tx, ctx, OpenEvent::Opened { origin, result });
            return;
        }
        // Créé ici, sur le fil d'interface : sur macOS, rfd attache le
        // sélecteur à la fenêtre comme feuille
        // (`beginSheetModalForWindow:completionHandler:`), qui rend la main
        // aussitôt ; aucune boucle modale ne tourne pendant le traitement de
        // l'événement en cours. La fenêtre principale est celle de
        // l'application (`mainWindow`) : eframe n'en ouvre qu'une.
        let picked = rfd::AsyncFileDialog::new()
            .set_title(title)
            .add_filter(filter, &["xml"])
            .pick_file();
        if dialog_tx.send(Box::pin(picked)).is_err() {
            // Le fil s'est arrêté avant de recevoir le futur : la demande
            // n'aboutira pas ; `App` la relance au prochain « Ouvrir ».
            log::error!("ouverture : le fil d'attente du sélecteur s'est arrêté");
            deliver(&tx, ctx, OpenEvent::Cancelled);
        }
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

    #[cfg(not(target_arch = "wasm32"))]
    fn save(&mut self, ctx: &egui::Context, request: SaveRequest, tx: Sender<SaveEvent>) {
        let SaveRequest {
            bytes,
            target,
            language,
        } = request;
        // Même schéma que l'ouverture : le fil est créé d'abord, le futur
        // du sélecteur (s'il y en a un) lui est envoyé ensuite.
        let (dialog_tx, dialog_rx) = std::sync::mpsc::channel::<SaveFuture>();
        let thread_ctx = ctx.clone();
        let thread_tx = tx.clone();
        let known = match &target {
            SaveTarget::Path(path) => Some(std::path::PathBuf::from(path)),
            SaveTarget::Dialog { .. } => None,
        };
        let spawned = std::thread::Builder::new()
            .name("planrs-save".to_owned())
            .spawn(move || {
                let path = if let Some(path) = known {
                    path
                } else {
                    // Canal fermé sans futur : demande abandonnée.
                    let Ok(picked) = dialog_rx.recv() else {
                        return;
                    };
                    let Some(handle) = pollster::block_on(picked) else {
                        deliver(&thread_tx, &thread_ctx, SaveEvent::Cancelled);
                        return;
                    };
                    handle.path().to_path_buf()
                };
                let event = write_path(&path, &bytes, language);
                deliver(&thread_tx, &thread_ctx, event);
            });
        if let Err(error) = spawned {
            deliver(&tx, ctx, SaveEvent::Failed(error.to_string()));
            return;
        }
        let SaveTarget::Dialog {
            suggested_name,
            title,
            filter,
        } = target
        else {
            return;
        };
        // Créé sur le fil d'interface (feuille attachée à la fenêtre sur
        // macOS, ADR-0011), attendu dans `planrs-save`.
        let picked = rfd::AsyncFileDialog::new()
            .set_title(title)
            .set_file_name(suggested_name)
            .add_filter(filter, &["xml"])
            .save_file();
        if dialog_tx.send(Box::pin(picked)).is_err() {
            log::error!("enregistrement : le fil d'attente du sélecteur s'est arrêté");
            deliver(&tx, ctx, SaveEvent::Cancelled);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn save(&mut self, ctx: &egui::Context, request: SaveRequest, tx: Sender<SaveEvent>) {
        let ctx = ctx.clone();
        let SaveRequest { bytes, target, .. } = request;
        // Pas de chemin sur le web : un chemin connu (impossible ici) est
        // traité comme un téléchargement sous son nom.
        let (name, title, filter) = match target {
            SaveTarget::Path(path) => (path, String::new(), String::new()),
            SaveTarget::Dialog {
                suggested_name,
                title,
                filter,
            } => (suggested_name, title, filter),
        };
        let dialog = rfd::AsyncFileDialog::new()
            .set_title(title)
            .set_file_name(name)
            .add_filter(filter, &["xml"]);
        wasm_bindgen_futures::spawn_local(async move {
            // `save_file` rend aussitôt un descripteur inscriptible ;
            // `write` n'est appelé que sur lui (tout autre descripteur fait
            // paniquer rfd). Limite connue : rfd ne distingue pas « Annuler »
            // du téléchargement dans sa fenêtre (la promesse rejetée est
            // avalée) ; l'un et l'autre rendent `Ok`, et le document est
            // alors marqué enregistré.
            let event = match dialog.save_file().await {
                None => SaveEvent::Cancelled,
                Some(handle) => match handle.write(&bytes).await {
                    Ok(()) => SaveEvent::Saved {
                        name: handle.file_name(),
                        path: None,
                    },
                    Err(error) => SaveEvent::Failed(error.to_string()),
                },
            };
            deliver(&tx, &ctx, event);
        });
    }
}

/// Futur d'une boîte « Enregistrer sous » native, envoyé au fil
/// `planrs-save`.
#[cfg(not(target_arch = "wasm32"))]
type SaveFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Option<rfd::FileHandle>> + Send>>;

/// Écrit atomiquement (`files::write_atomic`) et rend l'événement
/// correspondant.
#[cfg(not(target_arch = "wasm32"))]
fn write_path(path: &std::path::Path, bytes: &[u8], language: crate::i18n::Language) -> SaveEvent {
    let lang = match language {
        crate::i18n::Language::Fr => planrs_automation::text::Language::Fr,
        crate::i18n::Language::En => planrs_automation::text::Language::En,
    };
    match planrs_automation::files::write_atomic(path, bytes) {
        Ok(()) => SaveEvent::Saved {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.to_str().map(str::to_owned),
        },
        Err(error) => SaveEvent::Failed(error.message(lang)),
    }
}

/// Nom de fichier proposé à l'enregistrement : `name` sans les caractères
/// refusés par Windows, macOS ou Linux (`<>:"/\|?*` et contrôles, remplacés
/// par `_`), espaces de bord retirés, suffixé de `.xml` s'il n'y est pas ;
/// « planrs.xml » si rien ne reste.
#[must_use]
pub fn suggested_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches('.');
    let stem = if cleaned.is_empty() {
        planrs_core::PRODUCT_NAME
    } else {
        cleaned
    };
    if stem.to_ascii_lowercase().ends_with(".xml") {
        stem.to_owned()
    } else {
        format!("{stem}.xml")
    }
}

/// Futur d'un sélecteur natif, envoyé au fil d'attente.
#[cfg(not(target_arch = "wasm32"))]
type PickFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Option<rfd::FileHandle>> + Send>>;

/// Lit et calcule un fichier choisi sur le desktop. Chemin lu par
/// `std::fs::read`, jamais par `FileHandle::read`.
#[cfg(not(target_arch = "wasm32"))]
fn open_path(path: &std::path::Path, now: Moment) -> OpenEvent {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let origin = DocOrigin {
        name,
        path: path.to_str().map(str::to_owned),
        demo: false,
    };
    let result = std::fs::read(path)
        .map_err(|error| OpenError::Read(error.to_string()))
        .and_then(|bytes| document::load(&bytes, now));
    OpenEvent::Opened { origin, result }
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

#[cfg(test)]
mod tests {
    use super::suggested_file_name;

    #[test]
    fn suggested_file_name_is_safe_and_xml() {
        assert_eq!(suggested_file_name("Projet1"), "Projet1.xml");
        assert_eq!(suggested_file_name("maison.XML"), "maison.XML");
        assert_eq!(
            suggested_file_name("Exemple : construction d'une maison"),
            "Exemple _ construction d'une maison.xml"
        );
        assert_eq!(suggested_file_name("a/b\\c*?\u{7}"), "a_b_c___.xml");
        assert_eq!(
            suggested_file_name("  ...  "),
            format!("{}.xml", planrs_core::PRODUCT_NAME)
        );
    }
}
