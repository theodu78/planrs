# Renommage de `planrs`

Le nom est provisoire. Endroits où il apparaît (à tenir à jour) :
- **nom affiché** : `planrs_core::PRODUCT_NAME` (`crates/planrs-core/src/lib.rs`), seul endroit du code qui le définit ; CLI, fenêtre desktop et titre de la page web le lisent
- noms des crates `crates/planrs-*`, `apps/*` et `tests/` (`planrs-tests`) : `[package] name`, `[workspace.dependencies]` du `Cargo.toml` racine, chemins des dossiers, identifiants Rust `planrs_*`
- binaires : `planrs` (CLI, `apps/planrs-cli/Cargo.toml`) et `planrs-desktop` (`apps/planrs/Cargo.toml`)
- `repository` du `Cargo.toml` racine
- `xtask/src/main.rs` : liste `GUI_ALLOWED` des crates autorisées à dépendre d'egui
- `apps/planrs-web/index.html` (titre initial, `data-bin`, id du canevas `planrs_canvas`) et `apps/planrs-web/src/main.rs` (`CANVAS_ID`)
- `tools/oracle/generate.bas` : nom du module `PlanrsOracle` et titres des boîtes de dialogue
- `LICENSE-MIT` (« The planrs contributors »), `README.md`, `CLAUDE.md`, `docs/`, READMEs de `tests/`
- fichier compagnon `<nom>.planrs.json`, répertoire d'état utilisateur, fichier de rendez-vous du canal de contrôle
- `.github/workflows/*`
