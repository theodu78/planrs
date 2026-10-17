# Renommage de `planrs`

Le nom est provisoire. Endroits où il apparaît (à tenir à jour) :
- **nom affiché** : `planrs_core::PRODUCT_NAME` (`crates/planrs-core/src/lib.rs`), seul endroit du code qui le définit ; CLI, fenêtre desktop et titre de la page web le lisent
- noms des crates `crates/planrs-*`, `apps/*` et `tests/` (`planrs-tests`) : `[package] name`, `[workspace.dependencies]` du `Cargo.toml` racine, chemins des dossiers, identifiants Rust `planrs_*`
- identifiants Rust en CamelCase `Planrs*` (type `PlanrsApp` de `crates/planrs-ui-egui/src/lib.rs`, utilisé par `apps/planrs` et `apps/planrs-web`)
- champs `description` de tous les `Cargo.toml` et commentaires de doc `//!` des crates
- binaires : `planrs` (CLI, `apps/planrs-cli/Cargo.toml`) et `planrs-desktop` (`apps/planrs/Cargo.toml`)
- `repository` du `Cargo.toml` racine
- `xtask/src/layers.rs` : liste `GUI_ALLOWED` des crates autorisées à dépendre d'egui ; `xtask/src/panics.rs` : liste `EXEMPT_PACKAGES` (`planrs-tests`)
- `deny.toml` (commentaire d'en-tête, crates internes `planrs-tests` et `planrs-web` dans `[licenses.private]`)
- `.gitignore` (`/apps/planrs-web/dist/`)
- `tests/corpus/manifest.toml` (commentaire `planrs_core::parity::CORPUS_FLOOR`)
- variable d'environnement `PLANRS_CORPUS` : harnais du corpus (`tests/corpus/main.rs`, `tests/corpus/verdict.rs`), job CI `corpus` (`.github/workflows/ci.yml`), `tests/corpus/README.md`
- `apps/planrs-web/index.html` (titre initial, `data-bin`, id du canevas `planrs_canvas`) et `apps/planrs-web/src/main.rs` (`CANVAS_ID`)
- `tools/oracle/generate.bas` : nom du module `PlanrsOracle`, en-tête, titres des boîtes de dialogue et chemin d'exemple `C:\planrs\...`
- `LICENSE-MIT` (« The planrs contributors »), `README.md`, `CLAUDE.md`, `docs/`, READMEs de `tests/`
- fichier compagnon `<nom>.planrs.json`, répertoire d'état utilisateur, fichier de rendez-vous du canal de contrôle
- clé de persistance de l'état d'interface `planrs.ui_state` (`STATE_KEY` dans `crates/planrs-ui-egui/src/state.rs`, stockage eframe : fichier sur le desktop, localStorage sur le web)
- `.github/workflows/*`
