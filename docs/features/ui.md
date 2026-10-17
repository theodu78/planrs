# Interface egui (`planrs-ui-egui`) — fondations

État au 2026-10-17 (branche `ui-foundations`, ADR-0011). L'interface est **en lecture seule** : elle ouvre un fichier MSPDI, laisse le moteur calculer le planning et l'affiche (table des tâches et Gantt synchronisés, zoom, thème clair/sombre, français/anglais), sur le desktop et sur le web avec le même code. Aucune édition tant que le registre de commandes (J4) n'existe pas. Ce document décrit le code tel qu'il est ; les noms de types et de fonctions viennent tous de `crates/planrs-ui-egui/src/`.

## 1. Architecture

### 1.1 Crates et couches

| Crate | Rôle |
|---|---|
| `crates/planrs-ui-egui` | Toute l'interface : état, vues, ouverture, i18n, thème. Dépend de `planrs-core`, `planrs-mspdi`, egui, eframe, rfd, fluent-bundle, unic-langid, sys-locale. Seule crate de bibliothèque autorisée à dépendre d'egui/eframe/rfd (règle 9, `cargo xtask layers`). |
| `apps/planrs` (binaire `planrs-desktop`) | Fenêtre eframe native ; lit l'horloge locale (`chrono::Local`) et la langue du système, puis les injecte (`Env`). Feature `persistence` d'eframe. |
| `apps/planrs-web` (binaire `planrs-web`) | Même interface en `wasm32-unknown-unknown`, servie par trunk ; « maintenant » par `js_sys::Date`, langue par `navigator.language` (feature `js` de sys-locale). Compilée pour une cible native, le binaire indique seulement qu'il faut passer par trunk. |

Point d'entrée unique : `planrs_ui_egui::ui(ui: &mut egui::Ui, app: &mut App)`, appelé tel quel par le desktop et le web (via `PlanrsApp`), par les tests kittest (`Harness::build_ui_state`), par le test de fluidité et par l'exemple `snapshot`. `App` ne dépend pas d'eframe ; `PlanrsApp` (`shell.rs`) est le seul type qui implémente `eframe::App`.

### 1.2 Modules

| Module | Responsabilité |
|---|---|
| `lib.rs` | Réexports, `DEMO_MSPDI` (exemple embarqué `assets/demo/house.xml`, 49 tâches, écrit à la main), fonction `ui`. |
| `app.rs` | `App` (application pure), `Env`, `OpenDocument`, `DocOrigin`, `UserMessage` ; réducteur des `Action` ; relève des ouvertures asynchrones ; assemblage d'une image (`App::show`). |
| `action.rs` | `Action` : intentions de l'utilisateur (ouvrir, fermer, sélectionner, langue, thème, zoom, pas de zoom ancré, largeur de table, défilements, diagnostics, bandeau). |
| `shell.rs` | `PlanrsApp` : enveloppe eframe, lecture et écriture de `UiState` dans `eframe::Storage` (clé `STATE_KEY = "planrs.ui_state"`), mémoire egui jamais persistée (`persist_egui_memory` → `false`). |
| `state.rs` | `UiState` (préférences persistées), `LanguagePref`, `ThemePref`, `StateLoadError`, bornes de largeur de table. |
| `column.rs` | `Column` (8 colonnes dans l'ordre d'affichage de `Column::ALL`), `ColumnState`, `CellAlign`. |
| `document.rs` | `document::load(bytes, now) -> Result<LoadedDocument, OpenError>` : lecture `planrs_mspdi::read`, calcul `planrs_core::schedule`, plage par défaut. Pur, sans egui. |
| `io.rs` | `FileService` (injecté), `RfdFileService` (rfd natif synchrone / web asynchrone), `NoFileService` (tests), `OpenEvent`. |
| `rows.rs` | `RowModel`, `Row`, `RowKind`, `LinkView` : lignes, textes pré-formatés et liens triés, construits une fois par ouverture ou changement de langue. |
| `format.rs` | Formateurs d'affichage purs (durée, prédécesseurs, date, marge) en arithmétique entière ; `FormatLabels`, `UnitLabels`. |
| `timescale.rs` | `TimeScaleZoom`, `TierUnit`, `DayRange` (fin exclusive), `default_range`, `fit_zoom`, `TimeAxis` (temps → pixels), `tier_cells`, `zoom_anchor`. |
| `geom.rs` | Conversions entier ↔ `f32` regroupées (`index_to_f32`, `f32_to_index`, `i64_to_f32`), `visible_rows`, `row_at`, `MainLayout`, `main_layout`, `clamp_table_width`. |
| `theme.rs` | `Palette::{LIGHT, DARK}`, `Palette::for_theme(egui::Theme)`. |
| `i18n.rs` | `Language`, `TextId` (généré par la macro `text_ids!`), `Texts`, `Arg`, chargement des `locales/{fr,en}.ftl`. |
| `view/mod.rs` | `FrameCtx`, `FrameStats`, `Metrics`, `ViewTransient`, constantes `ROW_H = 22`, `HEADER_H = 44`, `OVERSCAN = 2`. |
| `view/chrome.rs` | Barre de menus (Fichier, Affichage), barre d'outils, écran d'accueil, barre d'état, bandeau de messages. |
| `view/main_view.rs` | Vue principale : en-têtes, `ScrollArea` verticale unique, séparateur, barre horizontale du Gantt, gestes (molette, Ctrl+molette, glisser). |
| `view/table.rs` | En-tête et corps virtualisé de la table, dessinés au `Painter`. |
| `view/timescale.rs` | En-tête à deux niveaux de l'échelle de temps. |
| `view/gantt.rs` | Corps du Gantt : grille, sélection, date d'état, liens (`link_path`, fonction pure), barres. |
| `view/diagnostics.rs` | Panneau repliable des diagnostics d'import et de calcul. |

### 1.3 Flux d'une image

```text
eframe (desktop/web)            kittest / perf / snapshot
  PlanrsApp::ui(ui, frame)        Harness::build_ui_state ou Context::run_ui
            \                          /
             planrs_ui_egui::ui(ui, &mut App)
                         |
                   App::show(ui)
   1. poll_io()        relève les OpenEvent du canal mpsc (ouverture finie)
   2. apply_theme()    ctx.set_theme seulement si ThemePref a changé
   3. apply_language() Texts + RowModel reconstruits si la langue résolue change
   4. Palette::for_theme(ctx.theme())   (thème résolu, jamais la préférence)
   5. FrameCtx { state, doc, rows, texts, palette, metrics, selection,
                 actions: &mut Vec<Action>, stats: &mut FrameStats }
         |-- view::chrome::menu_bar      (Panel::top)
         |-- view::chrome::toolbar       (Panel::top)
         |-- view::chrome::status_bar    (Panel::bottom + bandeau de messages)
         |-- view::diagnostics::panel    (si UiState.diagnostics_open)
         `-- CentralPanel
               |-- document ouvert : view::main_view::show(ui, fc, &mut ViewTransient)
               |      geom::main_layout -> MainLayout
               |      TimeAxis::new(range, zoom, gantt_left - gantt_scroll_x)
               |      table::paint_header, timescale::paint_header
               |      ScrollArea::vertical().show_viewport
               |          geom::visible_rows(viewport, ROW_H, len, OVERSCAN)
               |          table::paint_body  + gantt::paint_body   (même plage)
               |      molette X, Ctrl+molette, séparateur, barre horizontale
               `-- sinon : view::chrome::welcome
   6. last_frame = FrameStats (lignes table/Gantt, liens, formes)
   7. réducteur : App::apply(action) pour chaque Action de l'image
   8. fit_if_needed() (zoom ajusté après une ouverture), request_repaint si
      une action ou un ajustement a eu lieu
```

Les vues ne modifient jamais `App` : elles lisent `FrameCtx` et poussent des `Action`. L'effet d'une action est donc visible à l'image suivante (une image de plus est demandée). `App::dispatch` met une action en file depuis l'extérieur (tests, exemple).

### 1.4 Ouverture d'un fichier

- **Exemple embarqué** : `Action::OpenDemo` → `App::open_demo` → `App::open_bytes(DEMO_MSPDI, origin)`, synchrone.
- **Desktop** : `Action::OpenDialog` → `RfdFileService::pick_and_open` : `rfd::FileDialog` **synchrone** sur le fil d'interface (filtre `.xml`, titre traduit), puis `std::fs::read` (pas `FileHandle::read`, qui perd l'erreur), puis `document::load`. Le résultat (`OpenEvent::Opened` ou `Cancelled`) part dans un `std::sync::mpsc` et est relevé au début de l'image suivante.
- **Web** : même action → `rfd::AsyncFileDialog` (téléversement par un `<input type=file>` généré par rfd) lancé par `wasm_bindgen_futures::spawn_local` dans l'image du clic ; `handle.read().await` puis `document::load` ; même canal. Aucun fichier ne quitte le navigateur (note de l'accueil).
- Une seule ouverture à la fois (`App::is_opening`) ; une seconde demande affiche un message d'information.
- Un échec devient un `UserMessage` d'erreur traduit (bandeau refermable, jamais de modale) ; le document précédent reste affiché. Une ouverture réussie retire les erreurs d'ouverture précédentes.
- `document::load` calcule le planning **une fois**, à l'ouverture (règle 3 : aucune date calculée par l'interface). `App::install` attribue une génération (`OpenDocument::generation`), reconstruit le `RowModel`, efface la sélection et demande l'ajustement du zoom.

### 1.5 Environnement injecté (`Env`)

`Env { now: Box<dyn Fn() -> Moment>, system_language: Option<Language>, files: Box<dyn FileService> }` est la seule porte vers l'horloge, la langue du système et les fichiers. `now` sert de `ReadOptions::current_date` à chaque ouverture. `Env::fixed(now)` (tests, perf, exemple) : date fixe, aucune langue système (repli français), `NoFileService` qui répond `Cancelled`. Le `clippy.toml` de la crate interdit `chrono::Utc::now`, `chrono::Local::now`, `SystemTime`, `Instant` et `std::thread::spawn` (déterminisme, et ces appels paniquent en wasm32).

### 1.6 État persistant (`UiState`) et compatibilité

`UiState` (serde, `#[serde(default)]`, `UiState: Eq`, **aucun `f32`**) : `version`, `language`, `theme`, `zoom` (écrasé par l'ajustement à chaque ouverture), `columns` (une entrée par colonne dans l'ordre de `Column::ALL` : visibilité, largeur en points entiers), `table_width` (points entiers), `diagnostics_open`.

- JSON compact déterministe (`to_json`), JSON de référence `tests/fixtures/ui_state_v1.json` vérifié par `ui_state_default_json_reference`.
- Lecture tolérante `UiState::from_json_lenient` : champ absent → défaut ; champ inconnu → ignoré ; JSON illisible ou autre chose qu'un objet → état par défaut + `StateLoadError::Json` ; `version` > `UI_STATE_VERSION` (1) → état par défaut + `StateLoadError::TooNew`. L'avertissement est affiché une fois (`App::report_state_load`), jamais bloquant.
- `normalized()` borne les largeurs, complète et dédoublonne les colonnes, force ID et Nom visibles.
- Règle de version : ajouter un champ avec défaut ne change pas la version ; retirer, renommer ou changer le sens d'un champ l'incrémente.
- Hors de l'état persistant : document ouvert, sélection, `ViewTransient` (défilements, ajustement en attente, accumulateur de zoom), caches.

### 1.7 Modèle de lignes (`RowModel`)

Construit par `RowModel::build(doc, labels)` à l'ouverture et au changement de langue, jamais pendant une image :
- une `Row` par tâche, **dans l'ordre des ID** (aucun tri) : UID, ID, niveau, `RowKind` (`Null`, `Leaf`, `Summary`, `Milestone`, `Excluded`), criticité, barre `(début, fin)` prise dans `TaskSchedule.start/finish` (ou la barre d'affichage d'une tâche exclue) ;
- les textes des 8 colonnes, pré-formatés ligne par ligne (`RowModel::text(i, column)`) ;
- les liens (`LinkView { from, to, kind, critical }`) triés par `min(from, to)`, avec `max_span` ; `links_touching(range)` rend les liens dont l'intervalle de lignes coupe la plage visible (un lien long qui traverse la fenêtre est gardé), par dichotomie puis balayage borné ;
- `index_of(uid)` en O(log n) (`BTreeMap`).

### 1.8 Échelle de temps (`TimeAxis`) : aucun pixel → date

- Zoom discret `TimeScaleZoom::{Day, Week, Month, Quarter}` : 36 ; 12 ; 3,5 ; 1,2 px par jour ; paires d'en-têtes semaine/jour, mois/semaine, trimestre/mois, année/trimestre.
- `default_range` : début du projet − 7 jours reculé au premier jour de semaine du projet, fin + 21 jours, étendue à la date d'état effective, plafonnée à `MAX_RANGE_DAYS` (20 ans).
- `fit_zoom` : `Week` si la plage tient dans la largeur du Gantt, sinon `Month` ; appliqué à chaque ouverture dès que la largeur est connue, et par le bouton « Ajuster » (`Action::FitZoom`).
- `TimeAxis::x(Moment) -> f32` est le **seul** sens de conversion : écart entier en minutes (`i64`) à l'origine, puis une seule conversion en `f32`. Il n'existe aucune méthode inverse. Les gestes ne produisent que des indices de ligne (`geom::row_at`) ou des deltas en pixels (`ScrollGanttX`, `ZoomStep { anchor_x }`, `zoom_anchor`).
- Ctrl/Cmd+molette (ou pincement, `zoom_delta`) au-dessus du Gantt : un pas de zoom par facteur 1,25 cumulé, ancré sous le pointeur. Molette horizontale (ou Maj+molette) et glisser sur le Gantt : défilement horizontal. Le défilement horizontal est borné à `[0, largeur du contenu − largeur visible]`.

### 1.9 Palette et thème

`ThemePref::{System, Light, Dark}` ; `ctx.set_theme` n'est appelé qu'au changement. La palette (`Palette::LIGHT` / `Palette::DARK`, une seule pour la table et le Gantt) est choisie sur le thème **résolu** (`ctx.theme()`). Couleurs propres (hypothèses H-UI) : barres bleues, chemin critique rouge, récapitulatives et jalons sombres (clairs en thème sombre), date d'état en tirets violets. Tests de contraste et de distinction dans `theme.rs`.

### 1.10 Langues et garde « aucune chaîne en dur »

- `locales/fr.ftl` et `locales/en.ftl`, embarqués par `include_str!` ; identifiants `TextId` générés par `text_ids!` (une clé libre ne compile pas). `Texts::get` / `get_args` ; repli anglais → français → clé brute (journalisée une fois), jamais de panique ; `set_use_isolating(false)`.
- `LanguagePref::{System, Fr, En}` résolue par `Language::resolve` (système absent → français).
- Gardes automatiques (`i18n.rs`) : clés FR = clés EN = `TextId::ALL`, mêmes variables, aucune marque d'isolation U+2068/U+2069, aucune espace fine insécable U+202F (absente des polices par défaut), typographie française, tous les messages formatables.
- « Aucune chaîne en dur » : règle de relecture, **pas un test automatique**. Indices indirects : les tests kittest cherchent les widgets par des libellés construits par `Texts`, et le contrôle d'accessibilité de kittest échoue sur un widget sans nom ; mais rien ne détecte un littéral visible écrit dans une vue.
- Noms de mois et de jours dans les `.ftl` (pas de `chrono/unstable-locales`). Les messages de diagnostic du cœur restent en français (`Display` des diagnostics, `docs/ui-needs.md` N5).

### 1.11 Formats d'affichage (hypothèses H-UI)

Règles d'affichage, pas de calcul : elles n'ont pas d'identifiant `R-xxx` et ne sont vérifiées contre rien. Formateurs purs de `format.rs`, à migrer vers le cœur en J4 (`docs/ui-needs.md` N1, N2).

| Donnée | Français | Anglais |
|---|---|---|
| Durée (forme longue, unité du modèle, décimales par `ConversionFactors::to_unit_value`) | « 5 jours », « 1 jour? » (estimée), « 5 jours écoulés » | « 5 days », « 1 day? », « 5 edays » |
| Prédécesseurs (FS omis sans décalage, séparateur) | « 3;5DD+2 jours », « 17FF » | « 3,5SS+2 days » |
| Date | « lun 02/11/26 » | « Mon 11/2/26 » |
| Marge totale | jours ouvrés signés | idem |
| Récapitulative | durée dans l'unité par défaut du projet, temps ouvré (H-UI-08) | idem |
| Tâche exclue | colonnes calculées vides, sauf dates d'affichage d'une inactive (H-UI-10) | idem |

## 2. Rendu

- **Virtualisation.** Une seule `ScrollArea::vertical().show_viewport` englobe la table et le Gantt : les deux corps reçoivent la même plage `geom::visible_rows(viewport.top, viewport.bottom, ROW_H, len, OVERSCAN)` dans la même image (pas d'image de retard). Seules ces lignes (fenêtre + 2 de part et d'autre) sont parcourues ; aucun travail proportionnel au nombre de tâches pendant une image (textes pré-formatés, liens par `links_touching`, barres hors de la découpe horizontale non dessinées). Hauteur de ligne fixe 22 points, en-tête commun de 44 points à deux niveaux.
- **Table.** Dessin direct au `Painter` : fond, zébrure, sélection, trait de ligne, texte de chaque cellule visible par une galley du cache d'egui (`LayoutJob` tronqué par « … », couleur `Color32::PLACEHOLDER` fixée au dessin), nom indenté par niveau, récapitulative en faux gras, ligne critique en rouge. Chaque ligne est une zone cliquable (`Action::Select`) qui déclare un `widget_info` `Role::Row` avec le libellé traduit `row-label` (accessibilité, `get_by_label` de kittest). `egui_table` n'est pas utilisé (egui 0.36, incompatible).
- **Gantt, ordre des couches** (`gantt::paint_body`) : fond → grille verticale aux frontières des deux niveaux → bande de sélection → ligne de la date d'état (`effective_status_date`, tirets) → liens → barres. Barres : feuille (rectangle arrondi, contour), récapitulative (barre à crochets), jalon (losange), tâche exclue (contour gris), critique en rouge.
- **Liens.** `link_path(kind, p, s, metrics)` (pure, testée) : polyligne orthogonale d'au plus 6 points et pointe de flèche, cas direct et cas replié pour FS et SF, SS et FF par le côté correspondant ; un jalon se raccorde aux pointes de son losange. Culling par intersection d'intervalles de lignes puis par la boîte englobante du tracé.
- **Séparateur et barre horizontale.** Séparateur glissable (`SetTableWidth`, bornes `TABLE_WIDTH_MIN..=TABLE_WIDTH_MAX` et 200 points minimum pour le Gantt) ; barre horizontale minimale du Gantt (curseur proportionnel, glisser, clic sur la piste = une page).
- **Relevé.** `App::last_frame: FrameStats` : `table_rows`, `gantt_rows`, `links_painted`, `shapes` (formes émises par la vue principale, comptées à la main par les vues).

## 3. Tests

| Niveau | Où | Quoi |
|---|---|---|
| Unitaires purs | `#[cfg(test)]` des modules `app`, `column`, `document`, `format`, `geom`, `i18n`, `rows`, `state`, `theme`, `timescale`, `view/gantt`, `view/main_view` | Formateurs, conversions et lignes visibles, `main_layout`, axe et plage, ajustement, en-têtes, `zoom_anchor`, tracé des liens, `UiState` (JSON de référence, tolérance, bornes, aller-retour), parité des `.ftl`, palette, `RowModel` sur l'exemple, synchronisation des panneaux et gestes de molette sur un `Context` nu. |
| kittest sans rendu | `tests/ui/main.rs` (cible `ui`) | Accueil → exemple, `ui_settles`, langue (libellés et nom de l'exemple), zoom, bornes du zoom et du défilement, thème, ouverture annulée, bandeau d'erreur refermable, fermeture, panneau des diagnostics, sélection au clic, alignement table/Gantt après molette. Exécutés partout, sans GPU. |
| Captures de référence | `tests/ui/snapshots.rs`, `tests/ui/common.rs`, références `tests/snapshots/*.png`, seuils `kittest.toml` | 7 vues (accueil, principale clair/sombre/EN, semaine, jour, sélection défilée, diagnostics). Pixels comparés seulement si `PLANRS_UI_SNAPSHOTS=1`, par un adaptateur wgpu logiciel exclusivement (lavapipe) ; variable absente ou `0` : scénario et assertions exécutés, capture signalée non comparée ; autre valeur : échec. Job CI `ui` (voir `.github/workflows/ci.yml`). |
| Fluidité | `tests/perf.rs` | Voir ci-dessous. |

**Fluidité** (`crates/planrs-ui-egui/tests/perf.rs`) :
- réseau MSPDI généré en mémoire, déterministe sans aléa : 250 récapitulatives de 19 feuilles (5 000 tâches), 10 000 liens entre feuilles proches (FS depuis la feuille `k − 19` en 19 voies parallèles, SS depuis `k − 1`, FF depuis `k − 2` une feuille sur neuf), durées de 4 à 16 heures, un jalon toutes les 97 feuilles ;
- `shapes_do_not_grow_with_task_count` (toujours exécuté, debug) : à fenêtre 1920×1080 et zoom Semaine égaux, formes à 5 000 tâches (début, milieu, fin) ≤ 2 × formes à 100 tâches, et lignes peintes ≤ `ceil(1080 / 22) + 2 × OVERSCAN + 1`. Mesuré le 2026-10-17 : 773 formes à 100 tâches ; 809 / 558 / 370 à 5 000 tâches ;
- `frame_budget_5000_tasks` (ignoré, release) : `egui::Context` nu (sans AccessKit), `run_ui` + `tessellate` chronométrés, 3 zooms (Jour, Semaine, Trimestre) × 3 positions (`ScrollToRow` début, milieu, fin, Gantt défilé jusqu'aux barres visibles) × 30 images, chacune avec un cran de molette de 37 points ; échec si une médiane ≥ 16 ms ou un p95 ≥ 32 ms. Le GPU n'est pas mesuré.

Mesure locale du 2026-10-17 (release, conteneur Linux partagé, 1920×1080) :

| Zoom | Médiane (3 positions × 30 images) | p95 | Pire médiane d'une position | Pire p95 d'une position |
|---|---|---|---|---|
| Jour | 0,77 ms | 0,91 ms | 0,82 ms | 0,92 ms |
| Semaine | 0,57 ms | 0,70 ms | 0,59 ms | 0,70 ms |
| Trimestre | 0,52 ms | 0,63 ms | 0,52 ms | 0,62 ms |

Ouverture du réseau (lecture MSPDI, calcul, `RowModel`) : 80 à 118 ms. Environ 840 formes et 110 liens dessinés par image.

## 4. Livraison

- **Web** : `apps/planrs-web`, `Trunk.toml` (`public_url = "./"`, `filehash = true`), `index.html` minimal (canevas `planrs_canvas`, aucun JS écrit à la main ; `data-wasm-opt="z"`). `trunk build --release` produit `dist/`, à servir en HTTP statique.
- **Workflow** `.github/workflows/desktop-build.yml`, déclenché par `workflow_dispatch` (visible une fois le fichier sur la branche par défaut) et par les tags `ui-*` : binaire macOS universel (`lipo` aarch64 + x86_64, non signé), binaire Linux x86_64, `dist/` web ; chaque artefact est un `.tar.gz` avec un `README.txt` (le bit exécutable survit). Aucune signature ni notarisation.
- **CI** (`.github/workflows/ci.yml`) : `planrs-ui-egui` et `planrs-web` compilés en wasm32 par le job `checks` ; job `ui` pour les captures. Le test de fluidité n'est pas encore lancé par le job `perf` (voir § 5).

## 5. Limites connues et reportés

Reportés par l'ADR-0011 (décision 14 et conséquences), non implémentés :
- toute édition (table, Gantt, liens), annulation, raccourcis MS Project : après le registre de commandes (J4) ;
- fichiers récents ; redimensionnement des colonnes ; défilement horizontal de la table ; filtres ; tri ;
- panneau de diagnostics détaillé (version minimale seulement : une ligne par diagnostic, en français) ;
- **jours chômés** non grisés dans le Gantt (`Palette::non_working` existe mais n'est pas dessinée ; `Schedule` n'expose pas les calendriers compilés, `docs/ui-needs.md` N3) ;
- profil `wasm-release` (LTO) ; ressources, utilisation, calendriers, panneau IA, enregistrement.

Constatés en écrivant ce document :
- `Palette::summary_critical` est définie et testée mais pas utilisée : une récapitulative critique est dessinée avec la couleur `summary` (noire en clair), pas en rouge foncé ;
- `ScrollArea` d'egui 0.37 estompe par défaut le contenu près des bords haut et bas (`spacing.scroll.fade`) : la dernière ligne visible de la table et du Gantt apparaît pâlie ;
- le panneau des diagnostics trie et copie ses entrées à chaque image où il est ouvert (coût proportionnel au nombre de diagnostics, plafonné à 10 000 par le lecteur MSPDI) ;
- `App` ne s'ouvre que depuis des octets MSPDI (`open_bytes`, `open_demo`) : pas d'injection directe d'un `Project` (le test de fluidité génère donc un MSPDI) ;
- le test de fluidité n'est pas dans le job CI `perf` ; la mesure ne couvre que le CPU (construction + tessellation), pas le GPU ni le navigateur (facteur 2 à 3 attendu en wasm, non mesuré) ;
- aucune règle d'affichage n'est vérifiée contre MS Project (hypothèses H-UI de l'ADR-0011).

## 6. Comment lancer

```sh
# Desktop (binaire planrs-desktop)
cargo run -p planrs --release

# Web : depuis apps/planrs-web, trunk 0.21 et la cible wasm32-unknown-unknown
cd apps/planrs-web && trunk serve            # développement, http://127.0.0.1:8080
cd apps/planrs-web && trunk build --release  # dist/ statique

# Rendu hors écran d'une vue en PNG (sans fenêtre)
cargo run -p planrs-ui-egui --example snapshot -- crates/planrs-ui-egui/assets/demo/house.xml main sortie.png
#   vues : welcome | main | main-week | main-day | main-dark | main-en
#   sortie par défaut : planrs-snapshot-<vue>.png ; erreurs : message et code de sortie non nul

# Tests de l'interface (sans GPU)
cargo test -p planrs-ui-egui

# Captures comparées (Linux, lavapipe : paquet mesa-vulkan-drivers)
PLANRS_UI_SNAPSHOTS=1 WGPU_BACKEND=vulkan cargo test -p planrs-ui-egui --test ui
# Mise à jour des références (la CI fait foi en cas d'écart de Mesa)
PLANRS_UI_SNAPSHOTS=1 UPDATE_SNAPSHOTS=1 WGPU_BACKEND=vulkan cargo test -p planrs-ui-egui --test ui

# Fluidité (release)
cargo test -p planrs-ui-egui --release --test perf -- --ignored --nocapture
```

L'exemple `snapshot` utilise le même harnais que les captures (1280×720, 1 px par point, OS fixé, « maintenant » fixe au 2027-01-15) et préfère un adaptateur wgpu logiciel ; à défaut, il prend le premier adaptateur disponible et le signale (pixels alors différents de la CI). Langue et thème : français clair, sauf `main-dark` (français sombre) et `main-en` (anglais clair) ; `main-day` est défilée deux jours avant la date d'état ; `welcome` ignore le fichier.
