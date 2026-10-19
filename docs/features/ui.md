# Interface egui (`planrs-ui-egui`) — fondations

État au 2026-10-18 (branche `ui-foundations`, tête 1d3eb09, ADR-0011, après la passe de corrections de la relecture). L'interface est **en lecture seule** : elle ouvre un fichier MSPDI, laisse le moteur calculer le planning et l'affiche (table des tâches et Gantt synchronisés, zoom, thème clair/sombre, français/anglais), sur le desktop et sur le web avec le même code. Aucune édition tant que le registre de commandes (J4) n'existe pas. **Mise à jour J4, lot L11 (2026-10-19)** : le document est une session d'édition de `planrs_automation` ; annuler, rétablir et les raccourcis sont câblés (§ 1.12). La saisie dans les cellules vient avec L12. Ce document décrit le code tel qu'il est ; les noms de types et de fonctions viennent tous de `crates/planrs-ui-egui/src/`.

## 1. Architecture

### 1.1 Crates et couches

| Crate | Rôle |
|---|---|
| `crates/planrs-ui-egui` | Toute l'interface : état, vues, ouverture, i18n, thème. Dépend de `planrs-core`, `planrs-mspdi`, egui, eframe, rfd, fluent-bundle, unic-langid, sys-locale (en wasm32 : wasm-bindgen-futures, js-sys, rfd avec la feature `file-handle-inner`). Seule crate de bibliothèque autorisée à dépendre d'egui/eframe/rfd (règle 9, `cargo xtask layers`). |
| `apps/planrs` (binaire `planrs-desktop`) | Fenêtre eframe native (1440×900, minimum 800×500) ; lit l'horloge locale (`chrono::Local`) et la langue du système, puis les injecte (`Env`). Feature `persistence` d'eframe. |
| `apps/planrs-web` (binaire `planrs-web`) | Même interface en `wasm32-unknown-unknown`, servie par trunk ; « maintenant » par `js_sys::Date`, langue par `navigator.language` (feature `js` de sys-locale). Compilée pour une cible native, le binaire indique seulement qu'il faut passer par trunk. |

Point d'entrée unique : `planrs_ui_egui::ui(ui: &mut egui::Ui, app: &mut App)`, appelé tel quel par le desktop et le web (via `PlanrsApp`), par les tests kittest (`Harness::build_ui_state`), par le test de fluidité et par l'exemple `snapshot`. `App` ne dépend pas d'eframe ; `PlanrsApp` (`shell.rs`) est le seul type qui implémente `eframe::App`.

### 1.2 Modules

| Module | Responsabilité |
|---|---|
| `lib.rs` | Réexports, `DEMO_MSPDI` (exemple embarqué `assets/demo/house.xml`, 49 tâches, écrit à la main), fonction `ui`. |
| `app.rs` | `App` (application pure), `Env`, `OpenDocument`, `DocOrigin`, `UserMessage`, `MAX_MESSAGES` ; réducteur des `Action` ; relève des ouvertures asynchrones ; cache des diagnostics ; assemblage d'une image (`App::show`). |
| `action.rs` | `Action` : intentions de l'utilisateur (ouvrir, fermer, sélectionner, langue, thème, zoom, ajustement, pas de zoom ancré, largeur de table, défilements, diagnostics, bandeau). |
| `shell.rs` | `PlanrsApp` : enveloppe eframe, lecture et écriture de `UiState` dans `eframe::Storage` (clé `STATE_KEY = "planrs.ui_state"`), mémoire egui jamais persistée (`persist_egui_memory` → `false`), filtrage des entrées brutes (`sanitize_raw_input`, appelé par `raw_input_hook`). |
| `state.rs` | `UiState` (préférences persistées), `LanguagePref`, `ThemePref`, `StateLoadError`, bornes de largeur de table. |
| `column.rs` | `Column` (8 colonnes dans l'ordre d'affichage de `Column::ALL`), `ColumnState`, `CellAlign`. |
| `document.rs` | `document::load(bytes, now) -> Result<LoadedDocument, OpenError>` : lecture `planrs_mspdi::read`, calcul `planrs_core::schedule`, plage par défaut (`range`, `range_truncated`). Pur, sans egui. |
| `io.rs` | `FileService` (injecté), `RfdFileService` (rfd natif synchrone / web asynchrone), `NoFileService` (tests), `OpenEvent`. |
| `rows.rs` | `RowModel`, `Row`, `RowKind`, `LinkView` : lignes, textes pré-formatés et liens triés, construits une fois par ouverture ou changement de langue. |
| `format.rs` | Formateurs d'affichage purs (durée, prédécesseurs, date, marge) en arithmétique entière ; `FormatLabels`, `UnitLabels`, `PluralRule`. |
| `timescale.rs` | `TimeScaleZoom`, `TierUnit`, `ViewSpan` (fin exclusive), `DefaultRange`, `default_range`, `fit_zoom`, `TimeAxis` (temps → pixels), `tier_cells`, `zoom_anchor`. |
| `geom.rs` | Conversions entier ↔ `f32` regroupées (`index_to_f32`, `f32_to_index`, `i64_to_f32`), `visible_rows`, `row_at`, `MainLayout`, `main_layout`, `clamp_table_width`. |
| `theme.rs` | `Palette::{LIGHT, DARK}`, `Palette::for_theme(egui::Theme)`. |
| `selection.rs` | `Selection` (UID sélectionnés, ancre, tâche active, ordre des clics), `SelectMode` (Ctrl/Cmd+clic, Maj+clic) — lot L14. |
| `structure.rs` | `StructureOp`, `plan` (sélection → un lot du registre), `PendingDelete`, `App::run_structure`, `App::answer_delete`, fenêtre de confirmation `confirm_modal` — lot L14. |
| `i18n.rs` | `Language`, `TextId` (généré par la macro `text_ids!`), `Texts` (avec `FormatLabels` construits une fois), `Arg`, chargement des `locales/{fr,en}.ftl`. |
| `view/mod.rs` | `FrameCtx`, `FrameStats`, `Metrics`, `ViewTransient`, `DiagnosticsView`, constantes `ROW_H = 22`, `HEADER_H = 44`, `OVERSCAN = 2`. |
| `view/chrome.rs` | Barre de menus (Fichier ; Affichage : zoom, thème, langue, case « Diagnostics »), barre d'outils, écran d'accueil, barre d'état, bandeau de messages. |
| `view/main_view.rs` | Vue principale : en-têtes, `ScrollArea` verticale unique, séparateur, barre horizontale du Gantt, gestes (molette, Ctrl+molette, glisser). |
| `view/table.rs` | En-tête et corps virtualisé de la table, dessinés au `Painter`. |
| `view/timescale.rs` | En-tête à deux niveaux de l'échelle de temps. |
| `view/gantt.rs` | Corps du Gantt : grille, sélection, date d'état, liens (`link_path`, fonction pure), barres. |
| `view/diagnostics.rs` | Panneau repliable des diagnostics d'import et de calcul. |

### 1.3 Flux d'une image

```text
eframe (desktop/web)            kittest / perf / snapshot
  PlanrsApp::raw_input_hook       (sanitize_raw_input : événements non finis retirés)
  PlanrsApp::ui(ui, frame)        Harness::build_ui_state ou Context::run_ui
            \                          /
             planrs_ui_egui::ui(ui, &mut App)
                         |
                   App::show(ui)
   1. poll_io()        relève les OpenEvent du canal mpsc (ouverture finie)
   2. apply_theme()    ctx.set_theme seulement si ThemePref a changé
   3. apply_language() Texts, RowModel et DiagnosticsView reconstruits si la
                       langue résolue change
   4. Palette::for_theme(ctx.theme())   (thème résolu, jamais la préférence)
   5. FrameCtx { state, doc, rows, texts, palette, metrics, diagnostics,
                 selection, actions: &mut Vec<Action>, stats: &mut FrameStats }
         |-- view::chrome::menu_bar      (Panel::top)
         |-- view::chrome::toolbar       (Panel::top, horizontal_wrapped)
         |-- view::chrome::status_bar    (Panel::bottom + bandeau de messages)
         |-- view::diagnostics::panel    (si UiState.diagnostics_open)
         `-- CentralPanel
               |-- document ouvert : view::main_view::show(ui, fc, &mut ViewTransient)
               |      geom::main_layout -> MainLayout (Gantt arrêté à Metrics::vbar_w du bord)
               |      relevé last_gantt_width / last_gantt_left dans ViewTransient
               |      TimeAxis::new(range, zoom, gantt_clip.left() - gantt_scroll_x)
               |      table::paint_header, timescale::paint_header
               |      ScrollArea::vertical().show_viewport  (ScrollStyle::solid)
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

`sanitize_raw_input` retire les événements de pointeur, de molette, de zoom, de rotation et de toucher qui portent une composante non finie (NaN, infini) : egui échouerait sur une assertion avant tout code planrs.

### 1.4 Ouverture d'un fichier

- **Exemple embarqué** : `Action::OpenDemo` → `App::open_demo` → `App::open_bytes(DEMO_MSPDI, origin)`, synchrone.
- **Desktop** : `Action::OpenDialog` → `RfdFileService::pick_and_open` : un fil `planrs-open` est d'abord créé (échec → erreur d'ouverture affichée, aucune boîte ouverte), puis `rfd::AsyncFileDialog` est créé **sur le fil d'interface** (filtre `.xml`, titre traduit « Ouvrir un fichier MSPDI ») et son futur est envoyé au fil, qui l'attend par `pollster::block_on`, lit le fichier par `std::fs::read` (jamais `FileHandle::read`) et appelle `document::load`. Sur macOS, rfd attache le sélecteur à la fenêtre principale comme **feuille** (`beginSheetModalForWindow:completionHandler:`), qui rend la main aussitôt : l'interface continue de se dessiner et aucune boucle modale ne tourne dans le gestionnaire d'événement de winit. Jamais de `rfd::FileDialog` synchrone : sa boucle modale imbriquée fait avorter winit 0.30 sur macOS (ADR-0011, relecture sur Mac). Le résultat (`OpenEvent::Opened` ou `Cancelled`) part dans un `std::sync::mpsc`, `request_repaint` réveille l'interface, et il est relevé au début de l'image suivante.
- **Web** : même action → `rfd::AsyncFileDialog` (téléversement par un `<input type=file>` généré par rfd) lancé par `wasm_bindgen_futures::spawn_local` ; lecture par `Blob::arrayBuffer` (`read_web_file`, sur le `web_sys::File` rendu par `FileHandle::inner`, feature `file-handle-inner` de rfd) : une promesse rejetée devient `OpenError::Read` avec le message JavaScript, un contenu vide est refusé par le lecteur MSPDI. Tout chemin qui se termine envoie un `OpenEvent`. Aucun fichier ne quitte le navigateur (note de l'accueil).
- **Relance de « Ouvrir »** : `App::is_opening` n'est qu'un indicateur. Une nouvelle demande pendant une ouverture en cours abandonne la précédente (nouveau canal ; son résultat éventuel, envoyé dans l'ancien canal fermé, est ignoré) et relance le sélecteur : un sélecteur web qui ne répond jamais (pas d'événement `cancel`) ne bloque pas « Ouvrir ». Sur macOS, la feuille bloque les clics sur la fenêtre : pas de second « Ouvrir » possible. Sur Windows et Linux, le sélecteur n'est pas rattaché à la fenêtre (pas de `set_parent`, `App` ignore eframe) : un second « Ouvrir » ouvre un second sélecteur et le résultat du premier est ignoré (non testé sur ces plateformes).
- Un échec devient un `UserMessage` d'erreur traduit (bandeau refermable, jamais de modale) ; le document précédent reste affiché. Une ouverture réussie retire les erreurs d'ouverture précédentes et l'avertissement de troncature.
- `document::load` ouvre une `Session` (`Session::open_bytes`) qui calcule le planning (règle 3 : aucune date calculée par l'interface) ; il est recalculé par le moteur à chaque lot de commandes (§ 1.12). `App::install` attribue un numéro d'ouverture (`OpenDocument::opening`, la génération de la session repartant de 0), reconstruit le `RowModel` et la `DiagnosticsView`, efface la sélection, remet la vue à zéro et demande l'ajustement du zoom ; si `LoadedDocument::range_truncated`, il ajoute l'avertissement traduit `range-truncated` (« Plage d'affichage limitée à 20 ans »).

### 1.5 Environnement injecté (`Env`)

`Env { now: Box<dyn Fn() -> Moment>, system_language: Option<Language>, files: Box<dyn FileService> }` est la seule porte vers l'horloge, la langue du système et les fichiers. `now` sert de `ReadOptions::current_date` à chaque ouverture. `Env::fixed(now)` (tests, perf, exemple) : date fixe, aucune langue système (repli français), `NoFileService` qui répond `Cancelled`. Le `clippy.toml` de la crate interdit `chrono::Utc::now`, `chrono::Local::now`, `SystemTime`, `Instant` et `std::thread::spawn` (déterminisme, et ces appels paniquent en wasm32).

### 1.6 État persistant (`UiState`) et compatibilité

`UiState` (serde, `#[serde(default)]`, `UiState: Eq`, **aucun `f32`**) : `version`, `language`, `theme`, `zoom` (écrasé par l'ajustement à chaque ouverture), `columns` (une entrée par colonne dans l'ordre de `Column::ALL` : visibilité, largeur en points entiers), `table_width` (points entiers), `diagnostics_open`.

- JSON compact déterministe (`to_json`), JSON de référence `tests/fixtures/ui_state_v1.json` vérifié par `ui_state_default_json_reference`.
- Lecture tolérante `UiState::from_json_lenient`, **champ par champ** : champ absent → défaut ; champ inconnu → ignoré ; champ illisible → sa valeur par défaut, les autres conservés, `StateLoadError::Fields` (noms des champs) ; colonne illisible → ignorée ; JSON illisible → état par défaut + `StateLoadError::Json` ; autre chose qu'un objet → `StateLoadError::NotObject` ; `version` > `UI_STATE_VERSION` (1) → état par défaut + `StateLoadError::TooNew` (lu avant le reste). L'avertissement est affiché une fois (`App::report_state_load`), jamais bloquant.
- `normalized()` borne les largeurs, complète et dédoublonne les colonnes, force ID et Nom visibles.
- Règle de version : ajouter un champ avec défaut ne change pas la version ; retirer, renommer ou changer le sens d'un champ l'incrémente.
- Hors de l'état persistant : document ouvert, sélection, `ViewTransient` (défilements, ajustement en attente, accumulateur de zoom, largeur et bord du Gantt de l'image précédente), messages, caches.

### 1.7 Modèle de lignes (`RowModel`) et diagnostics (`DiagnosticsView`)

`RowModel::build(doc, labels)` est appelé à l'ouverture et au changement de langue, jamais pendant une image :
- une `Row` par tâche, **dans l'ordre des ID** (aucun tri) : UID, ID, niveau, `RowKind` (`Null`, `Leaf`, `Summary`, `Milestone`, `Excluded`), criticité, barre `(début, fin)` prise dans `TaskSchedule.start/finish` (ou la barre d'affichage d'une tâche exclue) ;
- les textes des 8 colonnes, pré-formatés ligne par ligne (`RowModel::text(i, column)`) ;
- les liens (`LinkView { from, to, kind, critical }`, critique si ses deux extrémités le sont) triés par `min(from, to)` ; `links_touching(range)` rend les liens dont l'intervalle de lignes coupe la plage visible (un lien long qui traverse la fenêtre est gardé), par dichotomie puis balayage borné ;
- `index_of(uid)` en O(log n) (`BTreeMap`).

`DiagnosticsView::build(doc)` (même moments, cache d'`App`, `App::diagnostics_view`) : diagnostics d'import puis de calcul, mis en texte (`Display` du cœur, en français), triés par gravité de façon stable, avec les compteurs `errors` et `warnings` ; « → » (U+2192, absent des polices d'egui) y est remplacé par « -> ». Aucune image ne refait ce travail proportionnel au nombre de diagnostics.

### 1.8 Échelle de temps (`TimeAxis`) : aucun pixel → date

- Zoom discret `TimeScaleZoom::{Day, Week, Month, Quarter}` : 36 ; 12 ; 3,5 ; 1,2 px par jour ; paires d'en-têtes semaine/jour, mois/semaine, trimestre/mois, année/trimestre. Étiquettes choisies par largeur estimée, de la plus longue à la plus courte ; semaine : « 2 nov. » / « Nov 2 », puis numéro ISO préfixé (`tier-week` : « S45 » / « W45 »), puis le jour seul.
- `default_range` (rend `DefaultRange { span: ViewSpan, truncated }`) : début du projet − 7 jours, fin + 21 jours ; étendue à la date d'état effective **seulement** si celle-ci est à `STATUS_REACH_DAYS` (90) jours ou moins de cette plage et que l'extension reste sous `MAX_RANGE_DAYS` (7 305 jours, 20 ans) ; sinon la date d'état est hors champ (sa ligne n'est pas dessinée), ce qui couvre une horloge en panne repliée sur `Moment::MIN`. Début reculé au premier jour de semaine du projet ; un projet de plus de 20 ans est tronqué et `truncated` vaut vrai (message traduit).
- `fit_zoom(range, largeur)` : `Week` si toute la plage tient dans la largeur du Gantt ; sinon `Month`, sauf si la plage au zoom Mois dépasse `QUARTER_FIT_FACTOR` (1,5) fois la largeur, auquel cas `Quarter` ; largeur nulle ou non finie → `Month`. Appliqué à chaque ouverture dès que la largeur est connue (défilement seulement borné), et par le bouton « Ajuster » (`Action::FitZoom`, ancré au centre).
- `TimeAxis::x(Moment) -> f32` est le **seul** sens de conversion : écart entier en minutes (`i64`) à l'origine, puis une seule conversion en `f32`. Il n'existe aucune méthode inverse publique. Les gestes ne produisent que des indices de ligne (`geom::row_at`) ou des deltas en pixels (`ScrollGanttX`, `ZoomStep { anchor_x }`, `zoom_anchor`). Seule exception, privée et réservée au dessin : `TimeAxis::first_day_at_or_before` cherche par dichotomie le premier jour visible pour borner l'en-tête et la grille.
- **Ancrage du zoom.** `App::zoom_to(zoom, ancre)` garde à la même place le point du contenu situé à `ancre` pixels du bord gauche du Gantt (ancre bornée à la largeur du Gantt, non finie → 0), par `zoom_anchor`, puis borne le défilement. Ctrl/Cmd+molette (ou pincement, `zoom_delta`) au-dessus du Gantt : un pas par facteur 1,25 cumulé, `ZoomStep.anchor_x` = abscisse du pointeur **relative au bord gauche du Gantt** (le point sous le pointeur ne bouge pas). Boutons et menu de zoom (`SetZoom`) et « Ajuster » : ancrés au centre du Gantt visible (`last_gantt_width / 2`), sans saut dans le temps.
- Molette horizontale (ou Maj+molette) et glisser sur le Gantt : défilement horizontal, borné à `[0, largeur du contenu − largeur visible]`.

### 1.9 Palette et thème

`ThemePref::{System, Light, Dark}` ; `ctx.set_theme` n'est appelé qu'au changement. La palette (`Palette::LIGHT` / `Palette::DARK`, une seule pour la table et le Gantt) est choisie sur le thème **résolu** (`ctx.theme()`). Couleurs propres (hypothèses H-UI) : barres bleues, barres critiques rouges, récapitulatives et jalons sombres (clairs en thème sombre), récapitulative critique en rouge sombre (`summary_critical`), date d'état en tirets violets. `non_working` est défini (contraste testé) mais pas encore dessiné (jours chômés, § 5). Tests de contraste et de distinction dans `theme.rs`, dont le bandeau (`palette_message_banner_contrast` : `text_strong` sur `message_error_bg` / `message_warning_bg` ≥ 4,5:1 dans les deux thèmes).

### 1.10 Langues et garde « aucune chaîne en dur »

- `locales/fr.ftl` et `locales/en.ftl`, embarqués par `include_str!` ; identifiants `TextId` générés par `text_ids!` (une clé libre ne compile pas). `Texts::get` / `get_args` ; repli anglais → français → clé brute (journalisée une fois), jamais de panique ; `set_use_isolating(false)`. `Texts::format_labels()` rend des `FormatLabels` construits une fois par langue (plus de recherche fluent par image).
- Règle de pluriel par langue, lue dans la clé `fmt-plural-rule` (FR `below-two` : singulier si |v| < 2 ; EN `exactly-one` : singulier si |v| = 1 ; repli `exactly-one`) → `FormatLabels::plural: PluralRule`.
- `LanguagePref::{System, Fr, En}` résolue par `Language::resolve` (système absent → français).
- Gardes automatiques (`i18n.rs`) : clés FR = clés EN = `TextId::ALL`, mêmes variables, aucune marque d'isolation U+2068/U+2069, aucune espace fine insécable U+202F (absente des polices par défaut), typographie française, tous les messages formatables.
- « Aucune chaîne en dur » : règle de relecture, **pas un test automatique**. Indices indirects : les tests kittest cherchent les widgets par des libellés construits par `Texts`, et le contrôle d'accessibilité de kittest échoue sur un widget sans nom ; mais rien ne détecte un littéral visible écrit dans une vue.
- Noms de mois et de jours dans les `.ftl` (pas de `chrono/unstable-locales`). Les messages de diagnostic du cœur et les erreurs d'ouverture restent en français (`docs/ui-needs.md` N5, N9).

### 1.11 Formats d'affichage et hypothèses H-UI

Règles d'affichage, pas de calcul : elles n'ont pas d'identifiant `R-xxx` et ne sont vérifiées contre rien. Formateurs purs de `format.rs`, à migrer vers le cœur en J4 (`docs/ui-needs.md` N1, N2).

| Donnée | Français | Anglais |
|---|---|---|
| Durée (forme longue, unité du modèle, décimales par `ConversionFactors::to_unit_value`) | « 5 jours », « 1 jour? » (estimée), « 5 jours écoulés » | « 5 days », « 1 day? », « 5 edays » |
| Prédécesseurs (FS omis sans décalage, séparateur) | « 3;5DD+2 jours », « 17FF » | « 3,5SS+2 days » |
| Date | « lun 02/11/26 » | « Mon 11/2/26 » |
| Marge totale | jours ouvrés signés | idem |
| Récapitulative | durée dans l'unité par défaut du projet, temps ouvré | idem |
| Tâche exclue | colonnes calculées vides, sauf dates d'affichage d'une inactive | idem |

Identifiants cités dans le code (définis ici) :

| Id | Hypothèse | Où |
|---|---|---|
| H-UI-04 | Durées en forme longue avec l'unité du modèle ; suffixe « ? » d'une durée estimée identique en FR et en EN | `locales/*.ftl`, `format.rs` |
| H-UI-08 | Durée d'une récapitulative dans l'unité par défaut du projet, en temps ouvré | `rows.rs` |
| H-UI-09 | Marge totale en jours ouvrés signés | `format.rs` (`format_slack`) |
| H-UI-10 | Tâche exclue : colonnes calculées vides, sauf dates d'affichage d'une inactive ; ligne vide : seul l'ID | `rows.rs` |
| H-UI-12 | ID et Nom toujours visibles | `column.rs` |
| H-G1 | Feuille de durée nulle dessinée en jalon | `rows.rs` |

Autres hypothèses H-UI non numérotées (commentaires du code) : règle de pluriel ci-dessus, plage par défaut et seuil de 90 jours, paliers de `fit_zoom`, numéro de semaine ISO quel que soit le premier jour de semaine du projet, tracé orthogonal des liens, lien critique si ses deux extrémités le sont, couleurs.

### 1.12 Édition : session, annuler/rétablir, raccourcis (J4, lot L11)

- **Document.** `OpenDocument { session, import_diagnostics, range, origin, opening }` : `project()`, `schedule()` et `generation()` lisent la session ; `cache_key() = (opening, generation)` ; `set_origin` change l'origine ici et dans la session (nom, chemin ; le drapeau `demo` reste côté interface). La session ne doit pas être modifiée directement : tout passe par `App`.
- **Édition.** `Action::Exec(Batch)` → `App::execute(&Batch, EditOrigin::User)` → `Session::apply(Commit)` ; `Action::Undo`/`Redo` → `App::undo`/`redo`. Après un commit, une annulation ou un rétablissement, `App::refresh` reconstruit `RowModel` et `DiagnosticsView`, prend pour plage l'union de l'ancienne et de la nouvelle plage par défaut (la nouvelle seule au-delà de 20 ans), garde la sélection par UID (retirée si la tâche n'existe plus) et les défilements (bornés). Un lot vide ne rafraîchit rien.
- **Refus.** Un `CommandError` venant de l'utilisateur s'affiche dans le bandeau sous le titre « Modification refusée » (`command-error-title`) ; texte = clé `error-<code>` (points changés en tirets, `task.not_found` → `error-task-not_found`, `i18n::error_text_id`), argument `$detail` = `CommandError::message` dans la langue de l'interface. Une clé par code stable du moteur (`error::codes` et `text.*`), vérifiée par `ui_ftl_keys_for_all_error_codes` qui relève les codes dans les sources du moteur. `EditOrigin::Agent` (canal, J7) : refus rendu à l'appelant, rien n'est affiché. « Rien à annuler » : aucun message. Une édition réussie retire les refus affichés. Chaque message du bandeau porte son titre (`UserMessage::title`), affiché une fois par suite de messages de même titre.
- **Raccourcis.** `keymap::KEYMAP: &[(KeyContext, Shortcut, Binding)]`, `Binding::Command(id du registre)` ou `Binding::Ui(UiBinding)` : Ctrl/Cmd+Maj+Z et Ctrl/Cmd+Y → `history.redo`, Ctrl/Cmd+Z → `history.undo` (contexte `Document`), Ctrl/Cmd+O → ouvrir (contexte `Global`). Lus avant le dessin (touche consommée), jamais pendant la saisie dans un champ de texte (`Context::text_edit_focused`). egui ignorant Maj et Alt en trop, la combinaison la plus précise d'une touche vient d'abord (test). Chaque id existe dans `REGISTRY` et est listé (`ui_keymap_commands_exist_in_registry`) ; un test par entrée vérifie son effet.
- **Menu Édition.** Annuler, Rétablir (grisés sans lot à annuler ou à rétablir), raccourci affiché par `Context::format_shortcut`.
- **Titre de la fenêtre.** `App::window_title()` : « *nom — planrs » si le document est modifié (`Session::is_dirty`), envoyé par `ViewportCommand::Title` au changement seulement.
- **Langue de la session.** `document::load` ouvre la session en français ; l'interface ne s'en sert pas (messages d'erreur dans sa propre langue). Un lot de texte saisi (L12) doit porter `lang` (« 3d » est refusé par une session française sans `"lang":"en"`).
- **Canal de contrôle (desktop).** `App::set_observer(Box<dyn DocumentObserver>)` : l'observateur est appelé après chaque ouverture, fermeture, commit, annulation ou rétablissement. `apps/planrs` (`ControlRendezvous`) y tient le résumé servi par `query` (`view::summary`) et, quand le chemin du document change, lance l'écouteur `hello`/`query` (une fois, jeton `new_token`) puis publie le rendez-vous (`canonical_project`) ; l'ancien est retiré. Échecs signalés sur la sortie d'erreur, sans effet sur l'interface. Voir `docs/features/control.md`.

### 1.13 Édition des cellules et ligne vide de fin (J4, lot L12)

- **Colonnes éditables.** Nom, Durée, Début, Fin, Prédécesseurs (`edit::editable`) ; ID, marge et « Critique » sont calculées.
- **Cellule courante.** La ligne courante est la tâche sélectionnée ou, sans sélection, la ligne vide de fin ; la colonne courante est posée par un clic sur la cellule et par Tab. État transitoire `ViewTransient::edit` (`edit::TableEdit` : colonne, ligne vide courante, saisie ouverte), jamais sérialisé, remis à zéro à l'ouverture et à la fermeture.
- **Ouverture.** F2 ou double-clic : saisie avec le texte affiché, curseur en fin ; frappe d'un caractère (hors champ de texte, menu ouvert, Ctrl/Cmd) : le caractère remplace la valeur. La zone de saisie (`TextEdit`, libellé `cell-edit-label`) est posée sur la cellule ; les raccourcis de `KEYMAP` sont inactifs pendant la saisie.
- **Fin de saisie.** Entrée (ou flèche bas) valide et descend d'une ligne (de la dernière tâche vers la ligne vide ; sur la ligne vide, la ligne vide reste courante pour saisir des tâches à la suite), flèche haut valide et monte, Tab / Maj+Tab valident et passent à la colonne éditable suivante / précédente, Échap abandonne, un défilement vertical ou la perte du focus valident sur place. Valider sans avoir rien changé depuis F2 ou le double-clic n'envoie rien (ni historique, ni contrainte posée par R-158 sur une date relue).
- **Lecture → lot (`edit::commit_edit`, pure).** Durée → `task.update duration {text}` (lue d'abord par `parse_duration` pour l'erreur sous la cellule) ; Début/Fin → `parse_date` (rôle Start/Finish) → `task.update start|finish`, R-158 appliquée par le registre ; Prédécesseurs → `parse_predecessors`, ID → UID côté interface (R-150 ; ID absent : `edit-error-unknown-id`), puis `link.remove` des anciens absents, `link.update` (type ou décalage changé), `link.add` (un lien pendant n'est pas touché) ; Nom → `task.update name`. Ligne vide : `task.add at End` portant la valeur saisie (nom vide pour une autre colonne que Nom ; prédécesseurs : `task.add as "$new"` puis `link.add` vers l'étiquette, un seul lot), rien si la saisie est vide. Le lot porte `lang` = langue de l'interface (la session est ouverte en français).
- **Refus.** Texte illisible : la saisie reste ouverte, message traduit sous la cellule (clé `error-text-<code>`, détail `CommandError::message`), effacé à la frappe suivante. Refus du registre (cycle, doublon, R-160) : bandeau « Modification refusée », la saisie est déjà fermée.
- **Texte initial et relecture (`ui_parse_display_roundtrip`).** Le texte initial est l'affichage s'il se relit à la même valeur, sinon l'écriture du moteur : « 5 jours écoulés » n'est pas une unité de R-140 (→ « 5ej ») ; une date de 2084 à 2149 affichée avec une année sur deux chiffres se relirait en 1984-2049 (→ « JJ/MM/AAAA hh:mm ») ; prédécesseurs non relisibles → écriture R-142. L'heure n'étant pas affichée, une date validée prend l'heure par défaut du rôle (R-033). Propriété vérifiée sur 256 cas, en français et en anglais.
- **Ligne vide de fin.** Une ligne de plus que de tâches dans la zone de défilement (table seulement, sans barre ni texte, libellé `blank-row-label`), visible au bas de la table ; elle ajoute une ligne de contenu, d'où les captures régénérées (curseur de la barre verticale, capture défilée).

### 1.14 Fichiers : Nouveau, Enregistrer, Enregistrer sous, document modifié (J4, lot L13)

- **Menu Fichier.** Nouveau (Ctrl/Cmd+N, `project.new`, contexte `Global`), Ouvrir l'exemple, Ouvrir (Ctrl/Cmd+O), Enregistrer (Ctrl/Cmd+S, `project.save`), Enregistrer sous… (Ctrl/Cmd+Maj+S, `UiBinding::SaveAs`), Fermer ; raccourcis affichés depuis `KEYMAP`. Barre d'outils inchangée (captures identiques).
- **Nouveau.** `App::new_project` → `Session::new_project` : nom traduit `new-project-name` (« Projet1 » / « Project1 »), planifié depuis le lundi **strictement** suivant « maintenant », 08:00 (`app::next_monday_morning`, hypothèse : MS Project propose la date du jour), langue de la session = langue de l'interface. Document non modifié, sans chemin.
- **Enregistrer.** `App::start_save` : octets de `Session::save_bytes(WriteOptions::default())` produits sur le fil d'interface, remis à `FileService::save(SaveRequest { bytes, target, language })`. Cible `SaveTarget::Path` si le document a un chemin et que ce n'est pas « Enregistrer sous », sinon `SaveTarget::Dialog` (nom proposé `io::suggested_file_name` : caractères interdits remplacés par `_`, suffixe `.xml`). Desktop (`RfdFileService`) : fil `planrs-save`, `files::write_atomic` (chemin connu) ou `AsyncFileDialog::save_file` créé sur le fil d'interface et attendu dans ce fil ; web : `save_file` puis `FileHandle::write` sur ce seul descripteur (téléchargement). Réponse `SaveEvent::{Cancelled, Saved { name, path }, Failed(détail)}` relevée en début d'image (`poll_save`). Sur `Saved` : nouvelle origine (nom, chemin, `demo` retiré ; l'observateur déplace le rendez-vous), puis `mark_saved` **seulement si** `cache_key` n'a pas changé depuis la production des octets (une édition faite pendant l'écriture laisse le document modifié). `Failed` : bandeau « Impossible d'enregistrer le fichier ». Valeurs ajustées par l'écrivain MSPDI (diagnostics d'écriture) : message d'information. Un seul enregistrement à la fois sur le desktop (deux écritures du même fichier pourraient finir dans le désordre) ; sur le web, une demande restée sans réponse est abandonnée.
- **Document modifié.** Fermer, ouvrir l'exemple, ouvrir un fichier, Nouveau : si `is_dirty`, bandeau non bloquant « Enregistrer les modifications de « nom » ? » (Enregistrer / Ne pas enregistrer / Annuler, `Action::Confirm(ConfirmChoice)`) ; « Enregistrer » poursuit l'opération seulement après un enregistrement réussi. Un document arrivé du sélecteur alors que le document courant a changé depuis la demande redemande confirmation. Desktop : `close_requested` → `ViewportCommand::CancelClose` puis bandeau ; « Ne pas enregistrer » envoie `ViewportCommand::Close` ; une fermeture pendant un enregistrement est annulée et reprise après son succès.

### 1.15 Structure : insertion, suppression, hiérarchie, liens (J4, lot L14)

- **Sélection.** `App::selection` reste la tâche active ; `App::selected()` (`Selection`) porte l'ensemble, l'ancre et l'ordre des clics. Clic simple : une tâche ; Ctrl/Cmd+clic : ajoute ou retire ; Maj+clic : plage de lignes depuis l'ancre (remplace la sélection). La table et le Gantt ne poussent que `Action::Select` ; `main_view` le réécrit en `Action::SelectWith { uid, mode }` d'après les modificateurs de l'image. `App` resynchronise l'ensemble sur la tâche active quand elles divergent (clic simple, tâche supprimée, nouveau document, affectation directe de `App::selection`). Table et Gantt surlignent toutes les tâches sélectionnées (`FrameCtx::selected`).
- **Commandes** (`Action::Structure(StructureOp)`, contexte `Document`, jamais pendant une saisie) : Inser → `task.add` (nom vide) `at: Before(première tâche sélectionnée dans l'ordre des ID)`, `End` sans sélection, la tâche créée devient la sélection ; Suppr → `task.delete` de chaque tâche sélectionnée sans ancêtre sélectionné, `cascade:true` pour une récapitulative ; Alt+Maj+→ / ← → `task.indent` / `task.outdent` dans l'ordre des ID, tâches dont un ancêtre est sélectionné exclues (le bloc ne bouge que d'un niveau) ; Ctrl+F2 → `link.add` FS en chaîne dans l'ordre des clics (ordre des ID pour une plage) ; Ctrl+Maj+F2 → `link.remove` de chaque lien dont les deux tâches sont sélectionnées. **Un seul lot** par commande (une entrée d'historique) : un doublon, un cycle ou une hiérarchie refusée font refuser tout le lot (R-153, R-156, R-160), refus traduit dans le bandeau. Rien à faire (sélection vide, un seul maillon, aucun lien) : aucun lot.
- **Confirmation.** Un lot de suppression contenant une récapitulative n'est pas appliqué : `App::pending_delete()` le garde avec le nombre de tâches supprimées (sous-tâches comprises) et la clé de cache du document ; une fenêtre modale (`confirm-delete-*`) demande « Supprimer » ou « Annuler » (Échap ou clic hors de la fenêtre = Annuler) → `Action::ConfirmDelete(bool)`. Une demande devenue caduque (document modifié, annulé ou changé entre-temps) est abandonnée sans effet. Les commandes de structure sont ignorées tant qu'une confirmation est ouverte.
- **Menu contextuel.** Clic droit sur une ligne de la table ou du Gantt : la ligne est sélectionnée si elle ne l'est pas (sinon la sélection multiple est gardée), puis un menu au pointeur propose les six commandes avec leur raccourci (`keymap::shortcut_for`) ; entrées autres qu'« Insérer une tâche » grisées sans sélection.
- **Hiérarchie lue côté interface** (ordre et niveaux seulement, aucun calcul de planning) : récapitulative = la tâche suivante est plus profonde (R-052) ; bloc = tâches suivantes plus profondes, lignes vides comprises (même parcours que `task.delete`).
- **Hypothèses (H-UI, non numérotées).** Insertion d'une seule tâche quelle que soit la taille de la sélection (MS Project en insère autant que de lignes sélectionnées) ; nom vide ; Ctrl+Maj+F2 sur une seule tâche ne retire rien (MS Project retire alors tous ses liens) ; Maj+Inser et Maj+Suppr déclenchent aussi Inser et Suppr (egui ignore Maj en trop).

## 2. Rendu

- **Virtualisation.** Une seule `ScrollArea::vertical().show_viewport` englobe la table et le Gantt : les deux corps reçoivent la même plage `geom::visible_rows(viewport.top, viewport.bottom, ROW_H, len, OVERSCAN)` dans la même image (pas d'image de retard). Seules ces lignes (fenêtre + 2 de part et d'autre) sont parcourues ; aucun travail proportionnel au nombre de tâches pendant une image (textes pré-formatés, liens par `links_touching`, barres hors de la découpe horizontale non dessinées). Hauteur de ligne fixe 22 points, en-tête commun de 44 points à deux niveaux.
- **Barre verticale.** `ScrollStyle::solid()`, toujours visible (`ScrollBarVisibility::AlwaysVisible`) ; elle occupe `Metrics::vbar_w` à droite du corps, que `main_layout` retire au Gantt ; les coins au-dessus et au-dessous prennent le fond de l'en-tête.
- **Table.** Dessin direct au `Painter` : fond, zébrure, sélection, trait de ligne, texte de chaque cellule visible par une galley du cache d'egui (`LayoutJob` tronqué par « … », couleur `Color32::PLACEHOLDER` fixée au dessin), nom indenté par niveau (retrait plafonné), récapitulative en texte appuyé et faux gras. Texte en couleur normale : comme MS Project, la table n'est pas colorée par la criticité ; **seule la colonne « Critique » d'une tâche critique est en rouge** ; lignes vides et tâches exclues en texte atténué. Chaque ligne est une zone cliquable (`Action::Select`) qui déclare un `widget_info` `Role::Row` avec le libellé traduit `row-label` (accessibilité, `get_by_label` de kittest). `egui_table` n'est pas utilisé (egui 0.36, incompatible).
- **Gantt, ordre des couches** (`gantt::paint_body`) : fond → grille verticale aux frontières des deux niveaux → bande de sélection → ligne de la date d'état (`effective_status_date`, tirets) → liens → barres. Barres : feuille (rectangle arrondi, contour), récapitulative (barre à crochets, rouge sombre si critique), jalon (losange), tâche exclue (contour gris), critique en rouge.
- **Liens.** `link_path(kind, p, s, metrics)` (pure, testée) : polyligne orthogonale d'au plus 6 points et pointe de flèche, cas direct et cas replié pour FS et SF, SS et FF par le côté correspondant ; un jalon se raccorde aux pointes de son losange. Culling par intersection d'intervalles de lignes puis par la boîte englobante du tracé.
- **Séparateur et barre horizontale.** Séparateur glissable (`SetTableWidth`, bornes `TABLE_WIDTH_MIN..=TABLE_WIDTH_MAX` et 200 points minimum pour le Gantt) ; barre horizontale minimale du Gantt (curseur proportionnel, glisser, clic sur la piste = une page).
- **Bandeau de messages** (sous la barre d'outils) : texte fort (`text_strong`) sur fond teinté (`message_error_bg` ou `message_warning_bg`), liseré gauche coloré selon la gravité, titre traduit `open-error-title` s'il y a une erreur, bouton `message-dismiss` (« Fermer le message » / « Dismiss ») qui vide la liste. `App` garde au plus `MAX_MESSAGES` (3) messages, les plus récents, sans doublon ; le bandeau borne aussi à l'affichage.
- **Barre d'état** : nom du document, nombre de tâches, plage du projet, compteurs d'erreurs et d'avertissements (lus dans `DiagnosticsView`, cliquables : ouvrent ou referment le panneau).
- **Panneau des diagnostics** (`Panel::bottom` redimensionnable), ouvert ou refermé par les compteurs de la barre d'état (que rien ne signale comme cliquables) ou par la case « Diagnostics » du menu Affichage, toujours active (un panneau resté ouvert après « Fermer » se referme) : une ligne par diagnostic, colorée par gravité, tronquée (texte entier au survol), virtualisée par `ScrollArea::show_rows` ; sans document : « Aucun document ouvert » ; sans diagnostic : les deux compteurs à zéro.
- **Relevé.** `App::last_frame: FrameStats` : `table_rows` (écrit par `table::paint_body` seulement), `gantt_rows` (écrit par `gantt::paint_body` seulement), `links_painted`, `shapes` (formes émises par la vue principale, comptées à la main par les vues). Les deux plages venant de deux chemins de dessin distincts, les tests d'alignement peuvent échouer.

## 3. Tests

`cargo test -p planrs-ui-egui` (2026-10-18, tête 1d3eb09) : **111** tests unitaires, **29** tests kittest (22 de comportement + 7 captures), **1** test de fluidité exécuté + **1** ignoré (mesure release).

| Niveau | Où | Quoi |
|---|---|---|
| Unitaires purs (111) | `#[cfg(test)]` des modules `app`, `column`, `document`, `format`, `geom`, `i18n`, `rows`, `shell`, `state`, `theme`, `timescale`, `view/chrome`, `view/gantt`, `view/main_view`, `view/table` | Formateurs (dont pluriels), conversions et lignes visibles, `main_layout` (barre verticale réservée), axe, plage (date d'état lointaine, troncature), `fit_zoom` à trois paliers, en-têtes FR et EN (mois, trimestres, semaines), `zoom_anchor`, tracé des liens, `UiState` (JSON de référence, lecture champ par champ, bornes, aller-retour), parité des `.ftl` et `FormatLabels`, palette et contraste du bandeau, `RowModel` sur l'exemple (jours fériés figés dans `rows_demo_fr`), messages bornés et dédoublonnés, cache des diagnostics, couleurs de la table, filtrage des entrées non finies, synchronisation des panneaux (`panes_share_row_range`, `RowModel` réel) et gestes de molette sur un `Context` nu. |
| kittest sans rendu (23) | `tests/ui/main.rs` (cible `ui`) | Accueil → exemple, `ui_settles`, langue (libellés et nom de l'exemple), boutons de zoom, bornes du zoom et du défilement horizontal (borne haute comprise), thème, ouverture annulée, bandeau d'erreur refermable, fermeture, panneau des diagnostics, case « Diagnostics » du menu Affichage (avec et sans document), sélection au clic, alignement table/Gantt après molette, `ScrollToRow`, ancrage sous le pointeur (Ctrl+molette et `ZoomStep`), bouton de zoom qui garde le centre, `FitZoom`, `OpenEvent` en succès et en erreur, relance de « Ouvrir » pendant une ouverture, diagnostic de lien pendant affiché (« -> »). Exécutés partout, sans GPU. |
| Captures de référence (7) | `tests/ui/snapshots.rs`, `tests/ui/common.rs`, références `tests/snapshots/*.png`, seuils `kittest.toml` | 7 vues : accueil, principale clair / sombre EN, semaine, jour, sélection défilée, diagnostics. Pixels comparés seulement si `PLANRS_UI_SNAPSHOTS=1`, par un adaptateur wgpu logiciel exclusivement (lavapipe) ; variable absente ou `0` : scénario et assertions exécutés, capture signalée non comparée ; autre valeur : échec. Références régénérées après les corrections (couleurs de la table, barre verticale, étiquettes de semaine). |
| kittest session (L11, 12) | `tests/ui/session.rs` (cible `ui`) | `ui_undo_redo_restores_rows` (lignes identiques après annuler, « * » du titre), `ui_refresh_keeps_selection_by_uid` (insertion en tête, suppression de la sélection, défilement gardé), `ui_keymap_commands_exist_in_registry`, `ui_keymap_each_shortcut_dispatches_{0..3}` + `_covers_all` (un test par entrée de `KEYMAP`), `ui_command_error_banner_translated` (FR, EN, agent, sans document), `ui_ftl_keys_for_all_error_codes`, `ui_observer_sees_open_edit_undo_close`, `ui_edit_menu_undo_redo`. Captures régénérées pour le menu Édition (seule la barre de menus change). Desktop : `desktop_rendezvous_follows_document_path` (`apps/planrs`, Unix : publication, déplacement, retrait du rendez-vous dans un dossier imposé). |
| kittest fichiers (L13, 7) | `tests/ui/files.rs` (cible `ui`, `FileService` simulé) | `ui_save_bytes_reread_equal` (octets écrits = `save_bytes`, relus puis réécrits identiques), `ui_dirty_star_and_clear_after_save` (menu, Ctrl+S sur chemin connu, Enregistrer sous, échec, annulation), `ui_save_during_edit_stays_dirty`, `ui_close_dirty_asks_confirmation` (Annuler, Ne pas enregistrer, Enregistrer, enregistrement annulé, exemple et Nouveau, fermeture de fenêtre `CancelClose`/`Close`), `ui_close_clean_window_not_cancelled`, `ui_new_project_empty_with_blank_row`, `ui_new_project_name_translated` ; `ui_keymap_each_shortcut_dispatches_{4..6}` dans `session.rs`. |
| kittest structure (L14, 7 + 6 raccourcis) | `tests/ui/structure.rs` (cible `ui`) | `ui_insert_before`, `ui_delete_summary_confirms_cascade` (annuler puis confirmer, feuille sans confirmation), `ui_delete_confirmation_stale_is_dropped`, `ui_indent_outdent_shortcuts` (plage Maj+clic, bloc + enfant, refus traduit), `ui_link_selection_fs_chain` (ordre des clics, doublon refusé en entier, délier, plage dans l'ordre des ID, cycle refusé), `ui_structure_undo_exact` (projet et lignes identiques après chaque annulation), `ui_context_menu_runs_structure` ; `ui_keymap_each_shortcut_dispatches_{4..9}` délèguent à `structure::check_keymap_entry`. Unitaires : `selection::tests`, `structure::tests`. |
| Fluidité | `tests/perf.rs` | Voir ci-dessous. |

**CI GitHub.** Run du commit 1d3eb09 (39075623215) entièrement vert : job `checks` (fmt, clippy, tests, deny, xtask, wasm), tests macOS et Windows, MSRV, job `ui` (captures comparées, `ubuntu-24.04`, lavapipe, références régénérées), job `perf` avec l'étape « Fluidité de l'interface (release) », corpus. Au run précédent (1bd203d), `ui` et `perf` étaient verts, `checks` rouge sur clippy (lints de la toolchain stable, corrigés dans 1d3eb09).

**Fluidité** (`crates/planrs-ui-egui/tests/perf.rs`) :
- réseau MSPDI généré en mémoire, déterministe sans aléa : 250 récapitulatives de 19 feuilles (5 000 tâches), 10 000 liens entre feuilles proches (FS depuis la feuille `k − 19` en 19 voies parallèles, SS depuis `k − 1`, FF depuis `k − 2` une feuille sur neuf), durées de 4 à 16 heures, un jalon toutes les 97 feuilles ;
- `shapes_do_not_grow_with_task_count` (toujours exécuté) : à fenêtre 1920×1080 et zoom Semaine égaux, formes à 5 000 tâches (début, milieu, fin) ≤ 2 × formes à 100 tâches, et lignes peintes ≤ `ceil(1080 / 22) + 2 × OVERSCAN + 1`. Mesuré le 2026-10-18 : 775 formes à 100 tâches ; 444 / 460 / 441 à 5 000 tâches ;
- `frame_budget_5000_tasks` (ignoré, release) : `egui::Context` nu (sans AccessKit), `run_ui` + `tessellate` chronométrés, 3 zooms (Jour, Semaine, Trimestre) × 3 positions (`ScrollToRow` début, milieu, fin, Gantt défilé jusqu'aux barres visibles) × 30 images, chacune avec un cran de molette de 37 points ; échec si une médiane ≥ 16 ms ou un p95 ≥ 32 ms. Le GPU n'est pas mesuré.

Mesure locale du 2026-10-18 après les corrections (release, conteneur Linux partagé, 1920×1080) :

| Zoom | Médiane (3 positions × 30 images) | p95 | Pire médiane d'une position | Pire p95 d'une position |
|---|---|---|---|---|
| Jour | 0,49 ms | 0,55 ms | 0,50 ms | 0,57 ms |
| Semaine | 0,46 ms | 0,51 ms | 0,46 ms | 0,53 ms |
| Trimestre | 0,42 ms | 0,58 ms | 0,52 ms | 0,63 ms |

Ouverture du réseau (lecture MSPDI, calcul, `RowModel`) : environ 100 ms. Environ 770 à 940 formes et 110 liens dessinés par image.

## 4. Livraison

- **Web** : `apps/planrs-web`, `Trunk.toml` (`public_url = "./"`, `filehash = true`), `index.html` minimal (canevas `planrs_canvas`, aucun JS écrit à la main ; `data-wasm-opt="z"`). `trunk build --release` produit `dist/`, à servir en HTTP statique (pas en `file://`).
- **Workflow** `.github/workflows/desktop-build.yml`, déclenché par `workflow_dispatch` (visible une fois le fichier sur la branche par défaut) et par les tags `ui-*` : binaire macOS universel (`lipo` aarch64 + x86_64, non signé), binaire Linux x86_64, `dist/` web ; chaque artefact est un `.tar.gz` avec un `README.txt` (le bit exécutable survit ; le README macOS explique le premier lancement d'un binaire non signé, celui de Linux les prérequis). Aucune signature ni notarisation. Pas encore exécuté.
- **CI** (`.github/workflows/ci.yml`) : `planrs-ui-egui` et `planrs-web` compilés en wasm32 par le job `checks` ; job `ui` pour les captures ; étape « Fluidité de l'interface (release) » dans le job `perf`.

## 5. Limites connues et reportés

Reportés (ADR-0011), non implémentés :
- édition dans le Gantt (glisser une barre, tracer un lien), structure (insertion, suppression, indentation : lot L14), glisser-déposer : J4 coupable ou J7 ;
- fichiers récents ; redimensionnement des colonnes ; défilement horizontal de la table ; filtres ; tri ;
- panneau de diagnostics détaillé (version minimale seulement : une ligne par diagnostic, en français) ;
- **jours chômés** non grisés dans le Gantt (`Palette::non_working` existe mais n'est pas dessinée ; `Schedule` n'expose pas les calendriers compilés, `docs/ui-needs.md` N3) ;
- profil `wasm-release` (LTO) ; ressources, utilisation, calendriers, panneau IA, enregistrement.

Constatés à la relecture et non corrigés :
- **polices non latines** : les noms en chinois, japonais, coréen, arabe, hébreu ou en emoji s'affichent en boîtes vides (polices par défaut d'egui) ; sur le desktop, la feature `system_fonts` d'eframe (active par défaut, native seulement) pourrait fournir un repli par les polices du système, non vérifié ; rien sur le web ni sous kittest. Décision à prendre (police de repli embarquée : licence, taille ; chargement paresseux sur le web) ;
- **erreurs d'ouverture non traduites** : le détail est le `Display` de `MspdiError` (français) ou de `std::io::Error` (langue du système), figé dans le message (un changement de langue retraduit le titre, pas le détail) ; le nom du fichier fautif n'apparaît pas dans le message (`docs/ui-needs.md` N9, N10) ;
- ~~**contraintes ignorées sans diagnostic**~~ (clos en J3 : le moteur honore les contraintes de date et émet des diagnostics ; la tâche 41 de l'exemple démarre le 26/04, `docs/ui-needs.md` N4) ;
- « → » des diagnostics du cœur remplacé par « -> » à l'affichage (glyphe absent des polices d'egui) ;
- barre d'outils sur deux lignes sous environ 900 px de large (`horizontal_wrapped`) ;
- `ScrollArea` d'egui 0.37 estompe le contenu près du bas de la zone (fondu de défilement) : la dernière ligne visible de la table et du Gantt apparaît pâlie ;
- étiquettes de l'en-tête de l'échelle de temps tronquées au bord gauche (« octobre 2… ») au lieu d'être recalées comme dans MS Project ;
- navigateur sans événement `cancel` sur le sélecteur de fichier (Safari < 16.4) : l'annulation ne répond jamais ; relancer « Ouvrir » abandonne la demande précédente ;
- web : le sélecteur est créé dans un futur lancé en fin d'image, donc hors du gestionnaire du clic ; il repose sur l'activation transitoire du navigateur (environ 5 s dans Chromium), à vérifier sur Safari ;
- clé `open-in-progress` (`TextId::OpenInProgress`) encore définie mais plus affichée depuis la relance de « Ouvrir » ;
- `App` ne s'ouvre que depuis des octets MSPDI (`open_bytes`, `open_demo`) : pas d'injection directe d'un `Project` (`docs/ui-needs.md` N8) ;
- la mesure de fluidité ne couvre que le CPU (construction + tessellation), pas le GPU ni le navigateur (facteur 2 à 3 attendu en wasm, non mesuré) ;
- macOS : binaire non signé (premier lancement par clic droit → Ouvrir ou `xattr -d com.apple.quarantine`, voir le README de l'artefact) ;
- sélecteur de fichiers réel jamais exercé par un test automatique (les tests injectent `NoFileService`, la CI n'a pas d'écran) : vérification manuelle sur chaque plateforme ; Windows et Linux non vérifiés, sélecteur non rattaché à la fenêtre sur ces deux plateformes ;
- Linux : X11 exige `libxkbcommon-x11` (paquet `libxkbcommon-x11-0`) ; « Ouvrir » exige un portail XDG (`xdg-desktop-portal`) ;
- aucune règle d'affichage n'est vérifiée contre MS Project (hypothèses H-UI, § 1.11).
- **web, enregistrement** : rfd ne distingue pas « Annuler » du téléchargement dans sa fenêtre (la promesse rejetée est avalée par `FileHandle::write`) ; les deux rendent `Ok` et le document est marqué enregistré. Pas de garde `beforeunload` : fermer l'onglet d'un document modifié ne demande rien (lot L13) ;
- **ligne vide de fin** : apportée par L12 ; le test `ui_new_project_empty_with_blank_row` vérifie le document vide et la vue principale, pas la ligne vide (lot L13).

## 6. Comment lancer

```sh
# Desktop (binaire planrs-desktop)
cargo run --release -p planrs

# Web : depuis apps/planrs-web, trunk 0.21 et la cible wasm32-unknown-unknown
cd apps/planrs-web && trunk serve            # développement, http://127.0.0.1:8080
cd apps/planrs-web && trunk build --release  # dist/ statique, à servir en HTTP :
python3 -m http.server -d apps/planrs-web/dist 8080

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

# Binaires desktop et dist/ web par la CI : pousser un tag ui-* (workflow desktop-build.yml)
git tag ui-<suffixe> && git push origin ui-<suffixe>
```

L'exemple `snapshot` utilise le même harnais que les captures (1280×720, 1 px par point, OS fixé, « maintenant » fixe au 2027-01-15) et préfère un adaptateur wgpu logiciel ; à défaut, il prend le premier adaptateur disponible et le signale (pixels alors différents de la CI). Langue et thème : français clair, sauf `main-dark` (français sombre) et `main-en` (anglais clair) ; `main-day` est défilée deux jours avant la date d'état ; `welcome` ignore le fichier.
