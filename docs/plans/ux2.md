# Cahier des charges « seconde passe UX » — planrs (avant v1.0.0)

Référence : ADR-0019 (`docs/decisions.md`). Branche d'intégration `ux2-dev` (depuis `main` 7ceedd1, après le ruban, le correctif de l'éditeur et la première passe UX d'ADR-0018). Toute convention ci-dessous est au statut `hypothèse` sauf ce que sa source dit. Chaque lot : un agent, ≤ ~1 500 lignes (code + tests, références d'images non comptées), un worktree et un `CARGO_TARGET_DIR` propres, un commit `type(scope): …` par sous-étape testée, fiche `H-UI-xx` / `R-xxx` et tests **dans le même commit**, messages terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

Checks de chaque lot : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p <crates du lot>`, `cargo xtask panics`, `cargo xtask layers`, `cargo xtask rules`, `cargo xtask text-ids`, `cargo xtask icons --check`, `cargo deny check`, `cargo check -p planrs-core --target wasm32-unknown-unknown`, `cargo check -p planrs-ui-egui --target wasm32-unknown-unknown`. Machine à mémoire partagée (serveurdvp) : `export PATH=$HOME/.cargo/bin:$PATH`, `ramwait 2.5` puis `memlimit 3 cargo …` (`memlimit 5` pour `cargo test --workspace`), une seule commande cargo lourde à la fois.

Clean-room (règle 1) : conventions d'interface tirées des seules pages d'aide Microsoft (`support.microsoft.com`, `learn.microsoft.com` : Project d'abord, Excel quand Project n'a pas d'équivalent, Office pour Mac pour les gestes macOS), relues par l'agent qui écrit la fiche (titre exact, phrase citée) ; jamais un autre clone (MPXJ, ProjectLibre, GanttProject…) ni sa documentation. Le format MSPDI des champs personnalisés vient du seul schéma XML de Microsoft (S91) et de l'énumération `PjField` (S90), plus des fichiers du corpus. Une affirmation sur MS Project sans page relue reste une hypothèse.

Règle 10 : toute position ou hauteur (`f32`) passe par un test de finitude et est bornée avant usage (séparateurs, notifications, sélecteur de champ) ; aucune indexation pouvant paniquer dans les catalogues de champs ; un champ personnalisé illisible est un diagnostic, jamais une panique.

Règle 3 : aucune valeur calculée par l'interface. Chaque nouvelle colonne lit le modèle ou le `Schedule` du moteur, formatés par `planrs-core` / `format.rs` ; chaque écriture passe par une lecture de saisie puis le registre.

Règles du ruban et d'ADR-0018 gardées : toute commande nouvelle a son jumeau au menu (`h_ui_70_every_item_has_menu_twin`), une action, un prédicat (`RibbonCmd::action`, `RibbonCmd::enabled`) ; aucune fonction dupliquée ; **un geste = une entrée d'annulation** ; un geste sans effet = aucune entrée et aucun message d'erreur.

## 0. Audit dédoublonné (2026-10-28)

37 constats de deux passes (MSG, PASTE, LINK, QUIT, PANE, TL, DET, COL, KEY, MOUSE, OK), relus dans le code de `ux2-dev` ; 24 écarts retenus, classés par gravité.

| # | Écart (constats) | Gravité | Décision (ADR-0019) | Lot |
|---|---|---|---|---|
| 1 | Tampon de lignes collé sur un bloc de cellules : tâche insérée (« Fouilles des fondations » ligne 12) — **capture du propriétaire** (PASTE-01 des deux passes) | bloquant | D2, H-UI-106, R-261 | L2 |
| 2 | Lier refuse tout le lot dès qu'une paire est déjà liée ; 11→12 jamais créé (LINK-01, PASTE-04 partie Lier) — **2e message de la capture** | bloquant | D3, H-UI-107, R-260 | L2 |
| 3 | Cmd+Q sans effet sur le DMG (QUIT-01 passe 1, QUIT-03) | bloquant | D4, H-UI-108 | L3 |
| 4 | Tous les messages dans un bandeau pleine largeur, sans délai, qui pousse la vue ; « Fermer » efface tout (MSG-01, MSG-02 passe 2) | bloquant | D1, H-UI-104 | L1 |
| 5 | Pas de volet de détails (« Formulaire tâche ») ; aucun successeur nulle part (PANE-03, DET-01) | bloquant | D5, D6, H-UI-112, H-UI-113 | L4 (cadre), L5 |
| 6 | Ni « Insérer une colonne », ni « Ajouter une nouvelle colonne », ni recherche ; colonne cochée ajoutée en fin (COL-01 des deux passes) | bloquant | D7, H-UI-114 | L6 |
| 7 | 14 colonnes figées, 6 modifiables ; Format → Colonnes mêle colonnes et bascules d'affichage (COL-02 des deux passes) | bloquant | D7, H-UI-117 | L6 (liste), L7 (valeurs) |
| 8 | Aucun champ personnalisé : `ExtendedAttribute` gardés en `Extras` opaques (COL-03 des deux passes) | bloquant | D8, H-UI-118, R-263 à R-266 | L8, L9 |
| 9 | Refus en jargon interne : « utiliser link.update », « (opération 0) », « erreur(s) », code `link.cycle` ; aucun détail repliable (MSG-02 passe 1, MSG-01 passe 2) | gênant | D1, H-UI-105 | L1 |
| 10 | Texte de cellules collé sur des lignes entières ⇒ insertion de tâches (PASTE-02 passe 1) | gênant | D2, R-261 | L2 |
| 11 | Colonne de texte sans tabulation sur la ligne vide ⇒ tâches nommées « 7;6 » (PASTE-03) | gênant | D2, R-261 | L2 |
| 12 | Prédécesseur répété (« 6;6 », « 6;6FS ») ⇒ deux `link.add`, refus (PASTE-04 partie retenue) | gênant | D3, R-260 | L2 |
| 13 | Cmd+Q muet sous une fenêtre modale (Trier, Filtre, Collage spécial, Informations…) ; pendant une saisie, sortie par le raccourci intégré d'egui (`quit_shortcuts`) hors du `KEYMAP` (QUIT-01 et QUIT-02 passe 2) | gênant | D4, H-UI-108 | L3 |
| 14 | Ni Cmd+W, ni Cmd+H, ni Cmd+M ; « Quitter » du Dock sans garde (QUIT-02 passe 1) | gênant | D4, H-UI-109 | L3 (Dock : reporté) |
| 15 | Chronologie fermable seulement par Affichage → Chronologie ; clic droit dans le vide sans menu, pas de × (PANE-01, TL-01) | gênant | D5, H-UI-110 | L4 |
| 16 | Hauteur de la chronologie figée à 72 px (`TIMELINE_H`), aucun séparateur (PANE-02, TL-02) | gênant | D5, H-UI-111 | L4 |
| 17 | Glisser un en-tête ne déplace pas la colonne ; pas de « Paramètres de champ » (COL-04, KEY-01 Alt+F3) | gênant | D7, H-UI-115, H-UI-116 | L6 |
| 18 | Compte rendu de collage interne (« un bout hors de la copie ») et en Avertissement rose (MSG-03, MSG-02 passe 2) | mineur | D1, H-UI-104 | L0 (textes), L2 |
| 19 | Texte tabulé débordant sur une cellule : colonnes sautées non dites, refus en code (PASTE-02 passe 2) | mineur | D1, D2, R-257 (compte rendu) | L2 |
| 20 | Source non répétée sur une destination multiple (PASTE-05) | mineur | D2, R-262 | L2 |
| 21 | Volet des diagnostics « aucune erreur / aucun avertissement » en doublon des pastilles (PANE-04) | mineur | D5, H-UI-119 | L10 |
| 22 | Raccourcis absents : Ctrl/Cmd+P, Ctrl+W / Ctrl+F4, Ctrl+Maj+F5, Alt+F3, Ctrl+R (KEY-01, KEY-03 partie Ctrl+R) | mineur | H-UI-109, H-UI-120, R-258 amendée | L0 (touches), L2, L3, L6 |
| 23 | Glisser un lien sur une paire déjà liée : refus du registre au bandeau (R-243, même cause que n° 2) | mineur | D3, R-260 | L2 |
| 24 | Ctrl+− du pavé (KEY-02), double-clic de cellule (MOUSE-01), tracé de barre au glisser (MOUSE-02), poignée de recopie (KEY-03 partie) | mineur | gardés ou reportés, à signaler (ADR-0019) | — |
| — | Conformes, à garder (OK-01) : recoller un bloc sur lui-même, « 9 » sur 9, « 4 jours » sur 4 jours, « 9;9 » sur 9 ⇒ aucun lot, aucun message | info | non-régression | L2, L11 |

Écartés à la relecture : « deux cellules d'un même lot peuvent produire deux `link.add` de la même paire » (PASTE-04 : une cellule Prédécesseurs ne porte que les liens de sa tâche, deux cellules du lot ne visent jamais la même paire) ; « l'ordre des colonnes est déjà un `Vec` dans `UiState.columns` » (COL-04 : une entrée par variante, dans l'ordre de `Column::ALL`, `state.rs` ; corrigé par `UiState` version 2 en L0) ; « le menu « planrs » de la barre macOS est le menu par défaut de winit revenu » (QUIT-03 : `with_default_menu(false)` est inchangé depuis J7, c'est le nom du processus ; cause réelle non reproduite hors du Mac, d'où D4 qui ne dépend pas de la livraison de la touche).

## 1. Graphe des lots

```
L0 contrat ─┬─ L1 messages ───────────────┬─ L2 collage et liens ─┬─ L4 volets et séparateurs ─── L5 volet de détails ─┬─ L9 champs personnalisés (interface) ── L10 retours visuels et captures (seul) ── L11 vérification
            └─ L8 champs perso (cœur, MSPDI) ┴─ L3 Cmd+Q et fenêtres ┴─ L6 insertion de colonnes ─── L7 valeurs des champs ──┘
```

Vagues (au plus 2 agents, fichiers disjoints) : W1 {L0} → W2 {L1, L8} → W3 {L2, L3} → W4 {L4, L6} → W5 {L5, L7} → W6 {L9} → W7 {L10} → avant/après montré au propriétaire (session principale) → W8 {L11}. Après W3 : **DMG intermédiaire** construit par `desktop-build.yml` sur la tête de `ux2-dev` pour que le propriétaire essaie Cmd+Q, Cmd+H, Cmd+M et le collage sur son Mac (point 2 d'ADR-0019) ; un échec rouvre L3 sans bloquer W4.

Fichiers par lot (sous `crates/planrs-ui-egui/` sauf mention ; un fichier hors liste demande l'accord de la session principale) :
- **L0** : `src/{action.rs, app.rs (bras du réducteur, appel de `App::global_keys` en tête de `read_keys`, messages déplacés dans `notify.rs`), notify.rs (nouveau), quit.rs (nouveau, squelette), panes.rs (nouveau, pur), field.rs (nouveau : catalogue réduit aux 14 colonnes), view/details.rs (nouveau, squelette), keymap.rs, state.rs, column.rs (variante `Custom`, `ColumnState` v2), ribbon/spec.rs (variantes de `RibbonCmd` seulement, pas la table `RIBBON`), view/mod.rs (`ViewTransient`), i18n.rs, lib.rs}`, `locales/{fr,en}.ftl`, `crates/planrs-core/src/model/custom.rs` (nouveau : types seuls) et `crates/planrs-core/src/model/mod.rs` (export), `tests/ui/main.rs` (modules), `tests/perf.rs` (sections vides), `docs/{features/ui.md, scheduling-rules.md}`. Seul lot qui touche `action.rs`, `app.rs`, `keymap.rs`, `state.rs`, `i18n.rs`, `locales/*.ftl`.
- **L1** : `src/{notify.rs, error_text.rs (nouveau), view/chrome.rs (bandeau, zone des notifications, historique dans la barre d'état)}`, `tests/ui/messages.rs`.
- **L2** : `src/{clipboard.rs, cells.rs, edit.rs, structure.rs}`, `crates/planrs-automation/src/gesture.rs`, `tests/rules/r243_link_drag.rs` (cas ajouté), `tests/ui/{paste_destination.rs, cells.rs (ajouts)}`, `tests/perf.rs` (section L2), `docs/scheduling-rules.md` (R-260 à R-262, amendements R-243, R-244, R-256, R-257, R-258).
- **L3** : `apps/planrs/{src/main.rs, Cargo.toml}`, `Cargo.lock`, `deny.toml` (seulement si `muda` l'exige), `src/quit.rs`, `tests/ui/quit.rs`, `docs/features/ui.md` (§ 5, limite macOS).
- **L4** : `src/{panes.rs, view/main_view.rs, view/timeline.rs}`, `tests/ui/panes.rs`, `tests/perf.rs` (section L4).
- **L5** : `src/{view/details.rs, task_form.rs (nouveau, brouillon sorti de `dialog/task_info.rs`), dialog/task_info.rs}`, `tests/ui/{details.rs, task_info.rs (adapté)}`, `tests/perf.rs` (section L5).
- **L6** : `src/{column.rs, field.rs, view/table.rs, view/ribbon.rs (menu des colonnes), dialog/field_picker.rs (nouveau), dialog/field_settings.rs (nouveau), dialog/mod.rs}`, `tests/ui/{columns_insert.rs, columns.rs (adapté), rows_columns.rs (adapté)}`, `tests/perf.rs` (section L6).
- **L7** : `src/{field.rs, rows.rs, edit.rs, format.rs}`, `tests/ui/fields.rs`, `tests/perf.rs` (section L7).
- **L8** : `crates/planrs-core/src/model/{custom.rs, task.rs, mod.rs}`, `crates/planrs-automation/src/command/{custom.rs (nouveau), task.rs, mod.rs, params.rs}` (et la liste des outils MCP si elle est déclarée à part), `crates/planrs-mspdi/src/{custom.rs (nouveau), task.rs, project.rs, extras.rs, normalize.rs, write/*}`, `tests/rules/{r263_*.rs, r264_*.rs, r265_*.rs, r266_*.rs}`, `docs/scheduling-rules.md` (R-263 à R-266), `docs/features/mcp.md` si les outils y sont listés.
- **L9** : `src/{field.rs (champs personnalisés), rows.rs, edit.rs, dialog/custom_fields.rs (nouveau), dialog/mod.rs}`, `tests/ui/custom_fields.rs`.
- **L10** : `src/{theme.rs, view/chrome.rs (dessin des notifications, jumeaux de menus), view/table.rs (colonne « Ajouter une nouvelle colonne » affichée), view/diagnostics.rs, ribbon/spec.rs (table `RIBBON`), ribbon/icon_data.rs (régénéré)}`, `assets/icons/lucide/` (SVG ajoutés), `examples/snapshot.rs`, `tests/snapshots/*`, `tests/ui/{snapshots.rs, ribbon.rs}`, `docs/user/*`.
- **L11** : `tests/ui/ux2_e2e.rs`, `tests/perf.rs` (corrections), `.github/workflows/*` (corrections seulement), `CHANGELOG.md`, `docs/{scorecard.md, decisions.md (écarts), features/*.md}`.
- Fichiers partagés en ajout seulement : `tests/ui/main.rs` (modules posés en L0), `tests/perf.rs` (sections `// --- UX2 Lx ---` posées en L0), `docs/features/ui.md` (§ 1.35 « Volets, colonnes, champs et messages (ADR-0019) » posé en L0, rempli par chaque lot), `docs/scheduling-rules.md` (numéros réservés en L0).

Chemin critique : L0 → L2 → L4 → L5 → L9 → L10 → L11. Coupes, dans l'ordre (si le temps manque avant `v1.0.0`) : historique des messages (L1), F6 (L4), déplacement de colonne au glisser (L6), forme « Prédécesseurs et successeurs » du volet (L5), dates et indicateurs personnalisés (L8 et L9 gardent Texte et Nombre). Jamais coupés : collage selon la destination, non-changement (Lier, « 6;6 »), Cmd+Q gardé, notifications qui s'effacent et refus en langage utilisateur, chronologie fermable et redimensionnable, volet de détails avec prédécesseurs, successeurs et ressources, insertion de colonne avec recherche, Texte1-30 et Nombre1-20 en MSPDI, une annulation par geste, fluidité, captures régénérées une fois.

## 2. Index

Repère des exemples : projet « P » de `docs/plans/ux.md` § 2 (1 « Gros œuvre » ⊃ 2 « Fondations » 5j, 3 « Murs » 10j (2→3), 4 « Livraison » jalon (3→4) ; 5 « Second œuvre » ⊃ 6 « Plâtrerie » 4j (3→6), 7 « Peinture » 3j (6→7)), fenêtre 1440×900 ; « exemple embarqué » : la maison de la capture (49 tâches ; 9→10→11 liées, 11→12 non).

### 2.1 Sources (table de `docs/scheduling-rules.md`, relues en L0 : titre exact et phrase citée)

| Réf. | Page | Utilisée pour |
|---|---|---|
| S61 | (relue de nouveau) « Keyboard shortcuts for Project » | Phrases à relire : F6 (passer d'un volet à l'autre, si présente), « Scroll to task » (Ctrl+Maj+F5), Maj+F5 et Ctrl+F4 si présentes, « Delete row that has a selected cell » (H-UI-111, H-UI-120) |
| S81 | (ruban, relue) « Create a timeline in Project » | Masquer la chronologie, case View → Timeline (H-UI-110) |
| S89 | (UX, relue) « Move or copy cells, rows, and columns » | Collage à partir de la cellule active, refus d'une forme incompatible (H-UI-106, R-261, R-262) |
| S90 | learn.microsoft.com, « PjField enumeration (Project) » | FieldID de Texte1-30, Nombre1-20, Date1-10, Indicateur1-20 de tâche (R-265, table figée par test) |
| S91 | learn.microsoft.com, schéma XML de Project : « ExtendedAttributes » (projet) et « ExtendedAttribute » (tâche) | `FieldID`, `FieldName`, `Alias`, `Value`, éléments de formule et de table de choix (R-265, R-266) |
| S92 | support.microsoft.com, page Project sur l'ajout et le retrait de colonnes (« Add, change, or remove columns in Project », titre à relire) | Insert Column, Add New Column, Hide Column, Field Settings, déplacer une colonne (H-UI-114 à H-UI-116) |
| S93 | support.microsoft.com, page Project sur l'affichage fractionné (« Split a view » / « Display two views at the same time », titre à relire) | View → Details, Task Form, formes Resources & Predecessors, etc. (H-UI-111, H-UI-112) |
| S94 | support.microsoft.com, page Project sur les champs personnalisés (« Create a custom field », titre à relire) | Renommer un champ personnalisé (alias), types disponibles (H-UI-118, R-263) |
| S95 | support.microsoft.com, « Keyboard shortcuts in Excel » (onglet macOS) ou la page Office pour Mac équivalente | Cmd+Q, Cmd+W, Cmd+H, Cmd+M, Cmd+P (H-UI-108, H-UI-109) |

Une page introuvable ou qui ne dit pas ce qui est cité ⇒ la convention passe en `hypothèse` pure, consigné dans ADR-0019. Notifications discrètes (D1) : hypothèse (comportement d'Office, aucune page relue).

### 2.2 Conventions d'interface (table de `ui.md` § 1.11 ; une ligne par convention, testée par kittest `h_ui_NN_*`)

| Id | Convention | Source | Exemple | Lot |
|---|---|---|---|---|
| H-UI-104 | Messages : `Info` et `Warning` ⇒ notification en bas à droite au-dessus de la barre d'état, en surimpression (`egui::Area`, aucune vue ne bouge), largeur ≤ 420 px, durée 5 s (`Info`), 8 s (`Warning`), 10 s (refus d'une saisie, `Error` non bloquante), survol = pause, × la ferme, 3 au plus (la plus ancienne part) ; temps lu sur `ctx.input(time)` (déterministe sous kittest). `Blocking` (confirmation de sortie, reprise, ouverture ou enregistrement échoués, état illisible) ⇒ bandeau en haut, jusqu'à décision ; « Fermer » ne ferme que le message dont il est le bouton. Historique : 20 derniers messages, bouton de la barre d'état (cloche et compte des non lus). Compte rendu d'un collage réussi : toujours `Info`. **Amende** H-UI des bandeaux de J4 et `MAX_MESSAGES` | hypothèse (Office) | Collage avec 2 liens non repris ⇒ notification « 1 tâche collée. 2 liens vers des tâches non copiées n'ont pas été repris. », partie après 300 images à 60 Hz, ruban et table à la même place avant et après | L0, L1 |
| H-UI-105 | Refus du registre : phrase par code et sujet en **ID** de tâche (`error_text::user_text(&CommandError, &Project, lang)`), jamais d'UID, de nom de commande, de numéro d'opération ni de code dans la phrase ; « Détails » repliable sous la phrase : code, opération, champ, texte du moteur ; pluriels Fluent ; un code sans phrase dédiée ⇒ phrase générique « Cette modification n'est pas possible. » + détails | hypothèse | Lier 9-10 déjà liées (sans R-260) ⇒ « La tâche 10 a déjà la tâche 9 pour prédécesseur. » ; cycle ⇒ « Ce lien créerait une boucle entre les tâches 9, 10 et 11. » ; Détails ⇒ « link.cycle, opération 2 » | L1 |
| H-UI-106 | Coller selon la destination (R-261) : bloc de cellules ⇒ collage de cellules, jamais d'insertion, tampon de lignes ⇒ refus en notification (« Vous avez copié des lignes entières : sélectionnez une ligne entière (clic sur son N°) pour les coller. ») ; lignes entières ⇒ tampon inséré avant, texte à en-tête importé, autre texte en cellules depuis la cellule active ; ligne vide ⇒ tampon ou texte à en-tête en tâches à la fin, sinon cellules dans la colonne active (tâches ajoutées en fin, R-257). Après un collage de lignes, les lignes collées restent sélectionnées en lignes entières (un second collage insère encore) ; après un collage de cellules, le bloc écrit est sélectionné. **Amende H-UI-95** | S89 ; Excel ; reste : hypothèse | Ligne 11 copiée par son N°, bloc Prédécesseurs 9-11, Cmd+V ⇒ 49 tâches, aucune entrée d'annulation, notification de refus | L2 |
| H-UI-107 | Non-changement (R-260) : un geste dont le lot planifié est vide (valeur identique, liens déjà présents) n'écrit rien, n'ajoute aucune entrée d'annulation et n'affiche aucun message, sauf le glisser d'un lien sur une paire liée (notification `Info` « Ces tâches sont déjà liées. ») | hypothèse (MS Project : Lier sur des tâches liées ne fait rien) | Tâches 9-12 sélectionnées, Lier ⇒ 11→12 créé seul, une entrée ; Lier de nouveau ⇒ rien | L2 |
| H-UI-108 | Quitter : Ctrl/Cmd+Q dans **tous** les états (table, Gantt, chronologie, volet de détails, fenêtre modale ou non, saisie ouverte) ⇒ saisie de cellule ou de formulaire validée d'abord (refus ⇒ on reste, message), fenêtre modale annulée comme par Échap, puis `App::request_quit` (garde des modifications). macOS : menu applicatif natif (D4), « Quitter planrs ⌘Q » élément personnalisé qui pousse la même demande ; `egui::Options::quit_shortcuts` vide. Journal des touches si `PLANRS_LOG_KEYS=1` (desktop, `log::info!`, cible `planrs::keys`) | S95 ; hypothèse | Document modifié, fenêtre « Trier » ouverte, Cmd+Q ⇒ fenêtre fermée, bandeau « Enregistrer les modifications ? » ; document intact ⇒ `ViewportCommand::Close` | L0 (touche), L3 |
| H-UI-109 | Fenêtres : Ctrl/Cmd+W et Ctrl+F4 (Windows, Linux) ⇒ Fermer le document (gardé comme Quitter, fenêtre gardée, écran d'accueil) ; Cmd+H, ⌥⌘H, Cmd+M (macOS, menu natif : masquer, masquer les autres, réduire) ; Ctrl/Cmd+P ⇒ mode impression (`DialogKind::PageSetup { print_mode: true }`) ; web : W et P laissés au navigateur (`WEB_NEVER` gardé) | S95 ; S61 (Ctrl+F4 si présent) ; hypothèse | Document modifié, Cmd+W ⇒ bandeau de confirmation ; Enregistrer ⇒ écran d'accueil | L0 (touches), L3 |
| H-UI-110 | Chronologie fermable : clic droit **n'importe où** dans la frise ⇒ menu avec « Masquer la chronologie » (et « Retirer de la chronologie » sur un élément, H-UI-81 gardée) ; × de 16 px au coin haut droit, visible au survol de la frise et au focus clavier ; les trois chemins poussent `TimelineOp::SetVisible(false)` (même action que la case Affichage → Chronologie) ; aucune entrée d'annulation | S81 ; × : hypothèse (demande du propriétaire) | Clic droit dans le vide de la frise ⇒ menu ; « Masquer » ⇒ `timeline_visible = false`, la table remonte de la hauteur de la frise | L4 |
| H-UI-111 | Séparateurs : entre chronologie et vue, entre vue et volet de détails ; zone de saisie de ±4 px, curseur `ResizeVertical`, glisser ⇒ hauteur bornée (chronologie 48 px à 40 % de la zone, détails 120 px à 60 %, la vue garde ≥ 120 px ; non fini ⇒ ignoré), double-clic ⇒ hauteur par défaut (72 et 220 px), hauteurs dans `UiState` (`timeline_height`, `details.height`), écrites au relâcher ; F6 / Maj+F6 : clavier au volet suivant / précédent (table, chronologie, détails) | S93 ; F6 : S61 si présent, sinon hypothèse (Office) | Glisser le séparateur sous la frise de 80 px ⇒ frise 152 px, gardée après relance | L4 |
| H-UI-112 | Volet de détails (Formulaire tâche) : Affichage → Détails (case, groupe Fractionner) ; suit la tâche de la cellule active (ligne vide ou rien ⇒ « Sélectionnez une tâche ») ; haut : Nom, Durée, Pilotée par l'effort, Planifiée manuellement, Début, Fin, Type de tâche, % achevé, Contrainte (type, date) ; bas : deux grilles selon la forme (clic droit dans le volet : Ressources et prédécesseurs (défaut), Ressources et successeurs, Prédécesseurs et successeurs ; `UiState.details.form`) ; prédécesseurs et successeurs : ID, Nom (lu), Type, Décalage ; ressources : Nom, Unités, Travail ; récapitulative : champs non modifiables grisés avec raison (H-UI-73) | S93 ; reste : hypothèse | Sélection (11, Durée) ⇒ volet sur « Fouilles des fondations », prédécesseur 10 FS 0 j, successeur 13 FS 0 j (forme Prédécesseurs et successeurs) | L4 (cadre), L5 |
| H-UI-113 | Édition dans le volet : brouillon (`task_form::TaskFormDraft`, partagé avec « Informations sur la tâche ») ; OK (ou Entrée hors grille) ⇒ **un** lot (`TaskFormDraft::to_batch`, R-260 appliquée), libellé « Formulaire tâche » ; Annuler (ou Échap) ⇒ brouillon relu ; changer de cellule active avec un brouillon modifié ⇒ OK implicite (refus ⇒ sélection gardée, message, H-UI-86) ; lignes vides des grilles = ajout ; ID vidé = retrait ; ID inconnu ⇒ refus nommé (« Il n'y a pas de tâche 99. ») ; document modifié ailleurs (annuler, agent) ⇒ brouillon non modifié relu, modifié gardé avec avertissement | hypothèse | Successeur 14 ajouté à 11 puis OK ⇒ lien 11→14, une entrée ; Ctrl+Z ⇒ lien retiré | L5 |
| H-UI-114 | Insérer une colonne : clic droit sur un en-tête ⇒ « Insérer une colonne » (à gauche de la colonne cliquée), « Masquer la colonne », « Paramètres de champ… », Ajuster, Trier (H-UI-99 gardée) ; colonne « Ajouter une nouvelle colonne » en bout de table (titre grisé, largeur 120 px, aucune cellule modifiable), clic ⇒ sélecteur, colonne ajoutée à sa place ; sélecteur : zone de recherche focalisée (sous-chaîne sans accents ni casse, titre et alias), liste groupée par catégorie, ↑/↓, Entrée insère, Échap ferme ; champ déjà affiché marqué ✓ (l'insérer le déplace) ; champ indisponible grisé avec raison ; une colonne au plus une fois ; 64 colonnes au plus ; aucune entrée d'annulation (vue, pas document). **Remplace** les cases de colonnes de H-UI-99 | S92 ; hypothèse | Clic droit sur « Durée » → Insérer une colonne → « trav » → Entrée ⇒ « Travail » entre Nom et Durée | L6 |
| H-UI-115 | Paramètres de champ (Alt+F3, menu d'en-tête) : champ (changer de champ garde la place), titre (vide = titre par défaut, 64 caractères), alignement du titre et des données (gauche, centre, droite), largeur (`[MIN_WIDTH, MAX_WIDTH]`), « Ajuster » ; gardés dans `ColumnState` v2 | S92 ; Alt+F3 : S61 si présent | Titre « Durée prévue » sur Durée ⇒ en-tête renommé, valeur inchangée | L6 |
| H-UI-116 | Déplacer une colonne : appui sur l'en-tête d'une colonne **déjà sélectionnée** en colonne entière, glisser au-delà du seuil ⇒ trait d'insertion vertical de 2 px, lâcher ⇒ colonne déplacée (`UiState.columns`), Échap annule ; ID toujours en premier (refus, curseur interdit) ; sinon l'appui sélectionne (H-UI-90 gardée) | S92 ; même geste que H-UI-94 | « Fin » sélectionnée, glissée avant « Durée » ⇒ ordre Nom, Fin, Durée, Début | L6 |
| H-UI-117 | Catalogue des champs de tâche (`field::CATALOG`, catégories Général, Planification, Avancement, Ligne de base, Écarts, Marges, Ressources, Coûts, Personnalisés) : environ 80 champs ; modifiables : ceux dont le registre a la commande (Nom, Durée, Début, Fin, Prédécesseurs, Successeurs, % achevé, Mode, Actif, Priorité, Type et Date de contrainte, Échéance, Travail, Type de tâche, Pilotée par l'effort, Jalon, Notes (une ligne), Début et Fin réels, Durée réelle et restante, % travail achevé) ; en lecture seule : ID unique, WBS, Niveau, Récapitulative, Critique, Marges totale et libre, Début et Fin au plus tôt et au plus tard, lignes de base 0 à 10 (Début, Fin, Durée, Travail), écarts (Début, Fin, Durée, Travail), Noms des ressources, Créé le ; Coûts (Coût, Coût fixe, Coût réel, Coût restant, Coût prévu…) listés **grisés** (« Coûts : non calculés dans cette version ») ; un champ à choix (Mode, Type de contrainte, Type de tâche, Oui/Non) se saisit au clavier (préfixe reconnu) ou par la liste Alt+↓ | S92 (noms) ; reste : hypothèse | Colonne « Marge libre » : valeur du moteur ; saisie refusée « Ce champ est calculé. » | L6 (liste), L7 (valeurs) |
| H-UI-118 | Champs personnalisés : Texte1-30, Nombre1-20, Date1-10, Indicateur1-20 dans la catégorie Personnalisés du sélecteur, titre = alias s'il existe (« Lot (Texte1) ») ; saisie : texte ≤ 255 caractères, nombre lu dans la langue (point fixe, R-263), date comme Début, indicateur Oui/Non ; vide efface ; Projet → Champs personnalisés… : liste par type, renommer (alias ≤ 50 caractères) en un lot ; champ à formule ou à table de choix (relu, R-265) : absent du sélecteur | S94 ; hypothèse | Texte1 renommé « Lot », colonne insérée, « Gros œuvre » saisi sur 2 ⇒ enregistré puis relu à l'identique | L9 |
| H-UI-119 | Volet des diagnostics : affiché seulement si le document a au moins un diagnostic ; clic sur une pastille de la barre d'état ⇒ l'ouvre filtré sur sa gravité ; case Affichage → Diagnostics gardée (cochée mais rien à montrer ⇒ volet absent) | hypothèse (MS Project n'a pas ce volet) | Exemple embarqué sans diagnostic, case cochée ⇒ aucun volet | L10 |
| H-UI-120 | Raccourcis complémentaires : Ctrl+Maj+F5 ⇒ Défiler jusqu'à la tâche ; Ctrl/Cmd+R ⇒ Remplir à droite (R-258 amendée) ; Maj+F5 ⇒ Rechercher le suivant si S61 le dit, sinon absent ; Ctrl+− reste le zoom arrière (egui ne distingue pas le pavé numérique, écart signalé) | S61 ; S87 (Ctrl+R) | Bloc (6, Durée)-(6, Début), Ctrl+R ⇒ refus nommé (Début ne lit pas « 4 jours »), rien écrit ; (7, Nom), Ctrl+Maj+F5 ⇒ Gantt défilé jusqu'à la barre de 7 | L0 (touches), L2 |

Fiches amendées en L0 : H-UI-95 (« destination : H-UI-106 »), H-UI-99 (« cases de colonnes : remplacées par H-UI-114 »), H-UI-81 (« Masquer : H-UI-110 »), H-UI-19 (rappel : double-clic sur le N° ⇒ Informations, sur une cellule ⇒ saisie), § 1.33 (hauteur de la frise : H-UI-111), § 5 (`MAX_MESSAGES`, limite Dock), ADR-0018 « Reporté » (réordonner les colonnes : fait par H-UI-116 ; Ctrl+R : fait par H-UI-120).

### 2.3 Règles (`docs/scheduling-rules.md` ; numéros réservés en L0, fiches écrites par le lot qui les implémente, avec leurs tests, règle 7)

- **R-260 — Non-changement des lots de l'interface** (L2). Tout lot planifié par l'interface (`edit::commit_edit`, `cells::*_plan`, `structure::plan(Link)`, `TaskFormDraft::to_batch`, `gesture::link_drag`) : (a) liste de prédécesseurs ou de successeurs saisie dédoublonnée par ID (la dernière occurrence gagne pour type et décalage : « 6;6SS » ⇒ un lien 6 SS) ; (b) Lier (`StructureOp::Link`) ne produit `link.add` que pour les paires de la chaîne sans lien **dans un sens ou dans l'autre** ; tout lié ⇒ `None` (aucun lot) ; (c) `gesture::link_drag` sur une paire déjà liée ⇒ `GestureError::AlreadyLinked` (aucun lot ; l'interface en fait une notification `Info`) ; (d) une cellule ou un champ dont la valeur relue égale la valeur courante ⇒ aucune opération. Le registre garde `link.duplicate` (R-156 inchangée, contrat MCP). Exemples : chaîne 9, 10, 11, 12 avec 9→10 et 10→11 ⇒ un seul `link.add` 11→12 ; chaîne 9, 10, 11 liée ⇒ `None` ; « 6;6 » sur une tâche sans lien ⇒ un `link.add` ; « 9 » sur une tâche qui a 9 FS 0 ⇒ lot vide.
- **R-261 — Collage selon la destination** (L2 ; amende R-244, R-256, R-257). Routage pur `clipboard::paste_route(dest: PasteDest, slot: Option<ClipKind>, text) -> PasteRoute` : `PasteDest::Cells` ⇒ `Cells` si texte ou tampon de cellules, `Refuse(RowsOnCells)` si tampon de lignes ; `PasteDest::Rows` ⇒ `InsertRows` (tampon de lignes), `ImportRows` (texte à en-tête, R-256 a), `Cells` sinon (depuis la cellule active) ; `PasteDest::Blank` ⇒ `InsertRows` ou `ImportRows` comme ci-dessus, `Cells` sinon (colonne active, tâches en fin). Exemples : tampon de la ligne 11, destination bloc Prédécesseurs 9-11 ⇒ refus, rien écrit ; « 7;6\n9\n10 » sur la ligne vide colonne Prédécesseurs ⇒ trois tâches sans nom en fin avec ces prédécesseurs, une entrée ; « 5j » sur des lignes entières 6-7 ⇒ Durée de la cellule active seulement (bloc d'une cellule).
- **R-262 — Source répétée sur une destination multiple** (L2 ; amende R-257). Grille source `h × l`, bloc destination `H × L` avec `H % h == 0` et `L % l == 0` ⇒ source répétée pour couvrir le bloc ; sinon collée une fois depuis le coin (R-257) ; bornes de `MAX_BLOCK_CELLS`. R-258 amendée : Ctrl/Cmd+R recopie vers la droite la première colonne du bloc (bloc d'une colonne : celle de gauche), mêmes cellules sautées et comptes. Exemples : « 2j\t3j » sur Durée-Début 2-5 ⇒ refus de lecture nommé (Début ne lit pas « 3j ») ; « 2j\n3j » sur Durée 2, 3, 6, 7 (4 lignes affichées) ⇒ 2 j, 3 j, 2 j, 3 j ; 1×2 sur 3×2 ⇒ répétée trois fois.
- **R-263 — Modèle des champs personnalisés** (L8). `planrs_core::model::custom` : `CustomFieldId { kind: CustomKind::{Text, Number, Date, Flag}, index: u8 }` (Texte 1-30, Nombre 1-20, Date 1-10, Indicateur 1-20 ; hors bornes ⇒ `None` à la construction) ; `CustomValue::{Text(String) ≤ 255 caractères, Number(i64) en 1/10 000 (point fixe, règle 5 ; |x| ≤ 10^14), Date(Moment), Flag(bool)}` ; `Project.custom_fields: BTreeMap<CustomFieldId, CustomFieldDef { alias: Option<String> ≤ 50 }>` ; `Task.custom: BTreeMap<CustomFieldId, CustomValue>` (absent = vide ; `Flag(false)` et texte vide ne sont pas stockés) ; `Project::validate` : valeur du mauvais type ⇒ erreur. Aucun calcul n'en dépend. Exemple : Nombre1 « 12,5 » ⇒ `Number(125_000)`.
- **R-264 — Commandes des champs personnalisés** (L8). `task.update { custom: { "text1": "Lot A", "number1": "12.5", "flag1": true, "date1": null } }` (clé en minuscules `text1`…`flag20`, `null` efface ; texte relu par `planrs_core::text` : nombre au point ou à la virgule selon la langue, date comme `start`) ; `custom_field.rename { field: "text1", alias: "Lot" | null }` ; erreurs `custom.unknown_field`, `custom.value_type`, `custom.too_long`, `param.invalid` ; une entrée d'annulation par lot ; MCP : mêmes opérations, aucune date calculée. Exemple : `custom_field.rename text1 "Lot"` puis `task.update 2 { text1: "GO" }` ⇒ deux entrées.
- **R-265 — Lecture MSPDI des `ExtendedAttribute`** (L8). Définitions `Project/ExtendedAttributes/ExtendedAttribute` : `FieldID` d'un champ de R-263 (table figée depuis S90, testée) **sans** `Formula` ni table de choix (`ValuelistUID`, `LookupTable`, selon S91) ⇒ `CustomFieldDef { alias: Alias }` ; valeurs `Task/ExtendedAttribute { FieldID, Value }` lues selon le type (nombre décimal ⇒ point fixe arrondi au 1/10 000 le plus proche, demi vers le haut ; date ISO ; indicateur `0`/`1`) ; tout le reste (autre FieldID, formule, table de choix, ressource, valeur illisible) reste en `Extras` opaques **à l'identique** (R-087), une valeur illisible d'un champ modélisé ⇒ `Extras` + diagnostic `mspdi.custom_unreadable` (info). Exemple : Text1 alias « Lot » et deux valeurs ⇒ modèle ; Cost1 et Text2 à formule ⇒ `Extras` inchangés.
- **R-266 — Écriture MSPDI et aller-retour** (L8). Définition écrite pour tout champ qui a un alias ou au moins une valeur (`FieldID`, `FieldName` = nom canonique anglais de S90, `Alias` si présent), dans l'ordre des `FieldID`, avant les définitions gardées en `Extras` ; valeurs des tâches dans l'ordre des `FieldID` à la place que le schéma donne à `ExtendedAttribute` (S91) ; nombre écrit sans zéro final (« 12.5 ») ; `normalize` étendu (texte tronqué à 255, alias à 50, caractères interdits) ; `read(write(p)).project == normalize(p)` (P62). Exemple : Text1 « Lot », tâche 2 « GO » ⇒ relu identique ; Text2 à formule relu puis réécrit octet pour octet.

### 2.4 Propriétés

| Id | Énoncé | Lot |
|---|---|---|
| P61 | Pour tout projet généré (≤ 300 tâches, liens, récapitulatives) et toute saisie de Prédécesseurs, toute chaîne de Lier et tout bloc collé ou rempli : le lot ne contient aucun `link.add` d'une paire déjà liée ni deux fois la même paire ; appliquer le lot puis replanifier le même geste ⇒ lot vide (idempotence) ; aucune panique | L2 |
| P62 | Pour tout projet généré avec champs personnalisés (≤ 30 champs, alias, valeurs de chaque type aux bornes) et `Extras` d'`ExtendedAttribute` inconnus : `read(write(p)).project == normalize(p)`, `normalize` idempotente, `Extras` inconnus réécrits à l'identique ; deux écritures ⇒ mêmes octets | L8 |
| P63 | Pour toute suite (≤ 64) d'insertions, retraits, déplacements, paramètres de colonnes et de glissers de séparateurs : `UiState.columns` sans doublon, ID en premier, ≤ 64 colonnes, largeurs et hauteurs finies dans leurs bornes ; sérialisé puis relu ⇒ égal ; un état version 1 relu ⇒ mêmes colonnes visibles dans le même ordre | L0 (état), L4, L6 |

### 2.5 Budgets de performance (release, `tests/perf.rs`, job `perf` ; réseau généré de 5 000 tâches, 1920×1080, ruban déplié)

| Scénario | Mesure | Budget | Lot |
|---|---|---|---|
| Scénarios J7, J8, ruban et UX existants | inchangés | inchangés | L1 à L11 |
| `splitter_drag_5000` : glisser le séparateur de la frise puis celui des détails, 60 images chacun | durée d'image | médiane < 16 ms, p95 < 32 ms | L4 |
| `details_follow_5000` : volet ouvert, ↓ maintenue 120 images (la tâche suivie change à chaque image) | durée d'image | médiane < 16 ms, p95 < 32 ms | L5 |
| `field_picker_5000` : ouvrir le sélecteur, taper 6 caractères | durée d'image | < 16 ms | L6 |
| `wide_table_5000` : 30 colonnes affichées (dont lignes de base, écarts, marges), défilement vertical 120 images puis horizontal 60 | durée d'image | médiane < 16 ms, p95 < 32 ms | L7 |
| `custom_mspdi_5000` : lecture puis écriture MSPDI, 10 champs personnalisés par tâche | durée | < 1,5 × la lecture/écriture du même fichier sans champs | L8 |
| Web | `.wasm` gzip -9 | ≤ 4 Mio (C-REL-05), hausse ≤ 80 Kio par rapport à `ux2-dev` au départ | L11 |

---

## L0 — Contrat (séquentiel, en premier)

- **Objectif** : figer les types (volets, champs, colonnes v2, messages, quitter), actions, raccourcis, textes et fiches ; squelettes qui compilent (refus « non disponible », jamais `todo!`) ; **aucun changement visible** (captures intactes) hormis les touches nouvelles qui répondent « Ce geste n'est pas encore disponible ».
- **Sources** : relire S61 (F6, Ctrl+Maj+F5, Maj+F5, Ctrl+F4, Alt+F3), S81, S89, S90 à S95 ; les écrire dans `docs/scheduling-rules.md` (table des sources) ; écart ⇒ `hypothèse` pure, noté dans ADR-0019.
- **Types et API exactes (contrat)** :

```rust
// planrs_core::model::custom (nouveau, types seuls ; champs de Task et Project ajoutés en L8)
pub enum CustomKind { Text, Number, Date, Flag }
pub struct CustomFieldId { kind: CustomKind, index: u8 }        // Copy, Ord, Serialize ("text1")
impl CustomFieldId { pub const fn new(kind: CustomKind, index: u8) -> Option<Self>; pub const fn max(kind: CustomKind) -> u8; pub fn all() -> impl Iterator<Item = Self>; }
pub enum CustomValue { Text(String), Number(i64) /* 1/10 000 */, Date(Moment), Flag(bool) }
pub struct CustomFieldDef { pub alias: Option<String> }

// planrs_ui_egui::notify (nouveau ; UserMessage et la pile déplacés depuis app.rs, comportement actuel en L0)
pub enum MessageKind { Info, Warning, Refusal, Blocking }
pub struct UserMessage { pub kind: MessageKind, pub severity: Severity, pub title: Option<TextId>, pub text: String,
  pub detail: Option<String> /* repliable */, pub posted_at: f64 /* ctx.input(time) */, pub id: u64 }
pub struct MessageStore { /* notifications (≤ 3), bandeau, historique (≤ 20) */ }
impl MessageStore { pub fn post(&mut self, m: UserMessage); pub fn expire(&mut self, now: f64, hovered: Option<u64>);
  pub fn dismiss(&mut self, id: u64); pub fn banner(&self) -> &[UserMessage]; pub fn toasts(&self) -> &[UserMessage];
  pub fn history(&self) -> &[UserMessage]; }
pub const fn lifetime(kind: MessageKind) -> Option<f64>;          // 5, 8, 10 s ; Blocking : None

// planrs_ui_egui::panes (nouveau, pur, implémenté en L0)
pub struct PaneHeights { pub timeline: f32, pub view: f32, pub details: f32 }
pub fn layout(total: f32, timeline: Option<f32>, details: Option<f32>) -> PaneHeights; // bornes de H-UI-111, non fini ⇒ défauts
pub fn drag_split(current: f32, delta: f32, min: f32, max: f32) -> f32;              // borné, non fini ⇒ current
pub enum PaneFocus { Table, Timeline, Details }  pub fn next(f: PaneFocus, visible: (bool, bool), back: bool) -> PaneFocus;

// planrs_ui_egui::state (UiState version 2 ; version 1 relue : colonnes visibles dans l'ordre de Column::ALL)
pub struct ColumnState { pub column: Column, pub width: u16, pub title: Option<String>, pub align: Option<CellAlign>, pub header_align: Option<CellAlign> }
// UiState.columns : colonnes AFFICHÉES, dans l'ordre d'affichage ; `visible` disparaît (masquer = retirer)
pub timeline_height: u16,                                        // 72 par défaut
pub details: DetailsPrefs { visible: bool /* faux */, height: u16 /* 220 */, form: DetailsForm }
pub enum DetailsForm { #[default] ResourcesPredecessors, ResourcesSuccessors, PredecessorsSuccessors }

// planrs_ui_egui::column
Column::Custom(CustomFieldId)    // + variantes des champs de H-UI-117, posées en L0 (textes compris), servies en L6 et L7
// planrs_ui_egui::field (nouveau)
pub enum FieldCategory { General, Scheduling, Progress, Baseline, Variance, Slack, Resources, Costs, Custom }
pub struct FieldDesc { pub column: Column, pub title: TextId, pub category: FieldCategory, pub align: CellAlign,
  pub availability: Availability /* Ready | ReadOnly | Unavailable(TextId) */ }
pub fn catalog() -> &'static [FieldDesc];                          // L0 : les 14 colonnes existantes
pub fn search(query: &str, lang: TextLanguage, aliases: &BTreeMap<CustomFieldId, String>) -> Vec<Column>; // L0 : sous-chaîne, sans accents

// planrs_ui_egui::action (variantes ajoutées en fin d'énumération)
Action::Pane(PaneOp), Action::Column(ColumnOp), Action::Details(DetailsOp), Action::Message(MessageOp), Action::CloseDocumentGuarded
pub enum PaneOp { SetTimelineHeight(u16), ShowDetails(bool), SetDetailsHeight(u16), SetDetailsForm(DetailsForm), FocusNext { back: bool } }
pub enum ColumnOp { Insert { at: usize, column: Column }, Hide(usize), Move { from: usize, to: usize }, Settings { at: usize, settings: ColumnState }, OpenPicker { at: usize }, OpenSettings(usize) }
pub enum DetailsOp { Ok, Cancel }
pub enum MessageOp { Dismiss(u64), ToggleHistory }          // `DismissMessage` gardé = Dismiss du bandeau
// RibbonCmd (variantes seulement) : DetailsPane, InsertColumn, FieldSettings, CustomFields

// planrs_ui_egui::quit (nouveau ; L0 : Ctrl/Cmd+Q hors saisie et hors fenêtre, comme aujourd'hui)
impl App { pub(crate) fn global_keys(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) -> bool; // vrai : touche prise
  pub fn shell_quit(&mut self);                                   // menu natif macOS (L3)
}

// planrs_ui_egui::view::ViewTransient (champs ajoutés)
pub split_drag: Option<SplitDrag>, pub column_drag: Option<ColumnDrag>, pub picker: Option<FieldPicker>, pub details: Option<DetailsTransient>

// planrs_ui_egui::view::details (squelette : « Sélectionnez une tâche »)
pub fn show(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, rect: egui::Rect);

// planrs_ui_egui::keymap (UiBinding ajoutés ; KeyContext::Global pour Quit, CloseDocument, Print)
UiBinding::{CloseDocument, PrintMode, ScrollToTask, FillRight, FieldSettings, FocusNextPane, FocusPreviousPane}
// COMMAND+W, CTRL+F4 (hors macOS) ⇒ CloseDocument ; COMMAND+P ⇒ PrintMode ; COMMAND_SHIFT+F5 ⇒ ScrollToTask ;
// COMMAND+R ⇒ FillRight (Table) ; ALT+F3 ⇒ FieldSettings (Table) ; F6 / SHIFT+F6 ⇒ FocusNext/PreviousPane ; web : W, P, R retenus seulement si non pris par le navigateur (R : laissé, rechargement)
```

- **Réducteur (`app.rs`, posé en L0, rien d'autre à y toucher ensuite)** : `read_keys` commence par `self.global_keys(ctx, actions)` (quit.rs) ; `push_message` / `push_titled` délèguent à `MessageStore::post` avec `MessageKind` déduit de la gravité et du titre (L0 : tous au bandeau, affichage inchangé) ; `report_command_error` appelle `error_text::user_message(error, project, lang)` (L0 : phrase actuelle en `text`, détail vide) ; `Pane(op)` ⇒ `App::pane_op` (`panes.rs`, `impl App` ; L0 : hauteurs et visibilité écrites dans `UiState`, sans dessin) ; `Column(op)` ⇒ `App::column_op` (`column.rs` ; L0 : `Insert` et `Hide` servis sur `UiState.columns`, le reste « non disponible ») ; `Details(op)` ⇒ `App::details_op` (`view/details.rs` ; L0 : rien) ; `Message(op)` ⇒ `MessageStore` ; `CloseDocumentGuarded` ⇒ `request(Pending::Close…)` existant ; `egui::Options::quit_shortcuts` **gardé** en L0 (coupé en L3).
- **Textes** : toutes les clés FR et EN (notifications, détails, phrases de refus par code de `planrs-automation::error::codes` utilisés par l'interface, compte rendu du collage reformulé « n liens vers des tâches non copiées n'ont pas été repris » sans « un bout », refus `paste-rows-on-cells`, menus des volets et des colonnes, titres des ~80 champs et catégories, raison « Coûts : non calculés dans cette version », Formulaire tâche, champs personnalisés, Fermer le document), sections `# --- UX2 Lx ---` ; chaque `TextId` nouveau référencé par un squelette ou par `field::catalog` (`cargo xtask text-ids` vert dès L0).
- **Docs** : `ui.md` § 1.11 (H-UI-104 à H-UI-120), § 1.35 posé vide, amendements listés au § 2.2 ; `scheduling-rules.md` : S90 à S95, relecture de S61, S81, S89, numéros R-260 à R-266 réservés (fiches par L2 et L8), P61 à P63.
- **Tests exigés** : `p63_column_state_invariants` (proptest, 256 cas, opérations d'état seulement), `ui_state_v1_columns_migrate` (état v1 de `main` relu : mêmes colonnes visibles, même ordre, largeurs gardées), `panes_layout_bounds` (unitaire : non fini, bornes, vue ≥ 120 px), `panes_focus_cycle`, `notify_lifetimes` (unitaire : 5, 8, 10 s, `Blocking` jamais), `notify_store_bounds` (3 notifications, 20 d'historique, `dismiss` ne retire que son message), `field_catalog_unique` (chaque `Column` au plus une fois, chaque `TextId` défini FR et EN), `field_search_accents` (« duree » trouve « Durée »), `custom_field_id_bounds` (core : Texte 1-30, Nombre 1-20, Date 1-10, Indicateur 1-20, 0 et max+1 ⇒ `None`), `keymap_ux2_bindings_resolve` (chaque entrée résolue, `web_interceptable` conforme), tous les tests existants verts et **inchangés** (sauf `UiState` : liste des tests adaptés au commit).
- **Critère de fin** : checks verts ; captures comparées sous lavapipe identiques ; aucun comportement visible changé hors touches nouvelles (« non disponible »).
- **Dépendances** : aucune. **Taille** : ~1 400 lignes.

---

## L1 — Messages (notifications, bandeau des décisions, refus en langage utilisateur)

- **Objectif** : plus de bandeau rose pour un compte rendu ; des refus lisibles ; rien qui pousse la vue.
- **Fichiers** : § 1, L1.
- **Comportement** :
  - `notify.rs` : `MessageStore` complet (H-UI-104) ; `kind` déduit à la publication : titres de confirmation, de reprise, d'ouverture et d'enregistrement ⇒ `Blocking` ; `Severity::Error` d'un refus de commande ⇒ `Refusal` ; reste ⇒ `Info` / `Warning` ; même texte déjà affiché ⇒ minuterie relancée, pas de doublon ; `expire` appelé une fois par image (repeint demandé tant qu'une notification vit, `request_repaint_after`).
  - `error_text.rs` : `user_message(&CommandError, &Project, lang) -> (String, String /* détail */)` : table par code (`link.duplicate`, `link.cycle`, `link.self`, `link.summary_descendant`, `task.not_found`, `batch.would_create_errors` résumé par son premier diagnostic traduit, `text.*`, `param.invalid`, `custom.*`, `limit.exceeded`) ; UID du sujet ⇒ ID par la position (`project.tasks`), tâche disparue ⇒ « une tâche supprimée » ; détail = code, « opération n », champ, `error.message(lang)` ; phrase générique sinon.
  - `view/chrome.rs` : bandeau (`Blocking` seulement) avec un « Fermer » par message ; zone des notifications (`egui::Area` ancrée en bas à droite au-dessus de la barre d'état, ordre du plus récent en bas, × et « Détails » repliable par notification, survol ⇒ pause) ; bouton d'historique dans la barre d'état (compte des non lus, fenêtre surgissante des 20 derniers avec leur détail) ; dessin minimal (couleurs du thème existantes : la mise en forme finale est en L10).
- **Règles** : H-UI-104, H-UI-105.
- **Tests exigés** (`tests/ui/messages.rs`) : `h_ui_104_report_is_toast_not_banner` (collage de lignes avec liens non repris : aucune `Panel::top` des messages, ruban et rectangle de la table inchangés au pixel près), `h_ui_104_toast_expires` (300 images à 60 Hz ⇒ partie ; survol ⇒ gardée), `h_ui_104_three_toasts_max`, `h_ui_104_banner_only_blocking` (fermeture d'un document modifié ⇒ bandeau ; « Fermer » d'un message ne retire pas l'autre), `h_ui_104_history_keeps_20`, `h_ui_105_duplicate_link_sentence` (« La tâche 10 a déjà la tâche 9 pour prédécesseur. », sans « link.update » ni « opération »), `h_ui_105_cycle_sentence_ids` (IDs 9, 10, 11, pas d'UID), `h_ui_105_details_fold` (détail masqué par défaut, contient `link.cycle` et l'opération), `h_ui_105_plural_no_parenthesis` (aucun « (s) » dans les textes FR et EN : test sur les `.ftl`), `h_ui_105_every_ui_code_has_sentence` (chaque code produit par un lot de l'interface a sa phrase).
- **Critère de fin** : tests verts ; captures inchangées (aucune capture n'affiche de message) ; tests J4 à UX qui lisaient `messages` adaptés (liste au commit).
- **Dépendances** : L0. **Parallèle** : L8. **Taille** : ~1 100 lignes.

---

## L2 — Collage selon la destination et non-changement

- **Objectif** : les deux messages de la capture ne peuvent plus se produire ; coller est prévisible.
- **Fichiers** : § 1, L2.
- **Comportement** :
  - `clipboard.rs` : `paste_route` pur (R-261) appelé par `paste_clipboard_text` et `paste_internal` ; destination lue de `self.selected.kind()` et de `self.selection` (ligne vide) ; refus `RowsOnCells` ⇒ notification, rien écrit ; `apply_paste` : lignes collées en lignes entières (gardé), cellules : bloc écrit sélectionné ; `report_paste` en `Info`, textes reformulés (L0).
  - `cells.rs` : R-262 (répétition) dans `paste_plan` ; `fill_right_plan` (Ctrl/Cmd+R, R-258 amendée) ; compte rendu des colonnes au-delà de la dernière visible (« 2 valeurs hors de la table n'ont pas été collées ») ; refus de lecture nommé (ID, titre de colonne, texte) par H-UI-105.
  - `edit.rs` : `resolve` dédoublonne par ID (R-260 a) ; `replace_links` inchangé sinon (OK-01).
  - `structure.rs` : `plan(StructureOp::Link)` filtre les paires liées dans un sens ou l'autre ; tout lié ⇒ `None` (aucune entrée, aucun message).
  - `planrs-automation/src/gesture.rs` : `link_drag` ⇒ `GestureError::AlreadyLinked` sur une paire liée (R-243 amendée) ; l'interface (`view/main_view.rs` lit déjà le refus de geste) l'affiche en `Info` par le texte posé en L0, sans toucher `main_view.rs` (texte choisi par `GestureError::text_id`, déjà lu).
- **Règles** : R-260, R-261, R-262, amendements R-243, R-244, R-256, R-257, R-258 ; H-UI-106, H-UI-107, H-UI-120 (Ctrl+R) ; P61.
- **Tests exigés** (`tests/ui/paste_destination.rs`, unitaires, `tests/rules`) : `r260_link_skips_existing` (9-12 ⇒ 11→12 seul, une entrée), `r260_link_all_linked_noop` (aucune entrée, aucun message), `r260_link_reverse_pair_skipped`, `r260_duplicate_ids_merged` (« 6;6 », « 6;6SS »), `r260_same_value_no_entry`, `r243_already_linked_gesture` (règle), `r261_rows_clip_on_cells_refused` (geste de la capture, à la souris : 49 tâches, aucune entrée, notification), `r261_cell_text_on_rows_pastes_cells`, `r261_single_column_on_blank_adds_tasks` (« 7;6\n9\n10 » ⇒ trois tâches sans nom en fin, une entrée), `r261_header_text_on_rows_imports`, `r261_second_paste_same_route` (Cmd+V, Cmd+V : même type de collage), `r262_tiles_multiple`, `r262_non_multiple_once`, `r258_fill_right`, `p61_ui_batches_idempotent` (proptest, 256 cas), `ok01_identical_paste_noop` (non-régression OK-01), tests `h_ui_39_*`, `h_ui_95_*`, `r256_*`, `r257_*` existants verts (adaptés seulement pour la destination : liste au commit), `paste_cells_1000` et `paste_1000` tenus.
- **Critère de fin** : tests verts ; captures inchangées ; chaque geste d'édition = une entrée, chaque non-changement = zéro.
- **Dépendances** : L0, L1 (refus en notification). **Parallèle** : L3. **Taille** : ~1 200 lignes.

---

## L3 — Cmd+Q et gestes de fenêtre (macOS, Windows, Linux)

- **Objectif** : Cmd+Q passe toujours par la garde, comme au J7 ; gestes standard de fenêtre.
- **Fichiers** : § 1, L3.
- **Comportement** :
  - `apps/planrs` : dépendance `muda` (dernière version publiée depuis au moins deux semaines, `[target.'cfg(target_os = "macos")'.dependencies]`, `default-features = false` si cela suffit) ; `event_loop_hook` garde `with_default_menu(false)` ; à la création de l'application (fil principal) : menu « planrs » (À propos de planrs, séparateur, Masquer planrs ⌘H, Masquer les autres ⌥⌘H, Tout afficher, séparateur, Quitter planrs ⌘Q **élément personnalisé**) et menu Fenêtre (Réduire ⌘M, Zoom) par les éléments prédéfinis de `muda` sauf Quitter ; chaque image, `MenuEvent::receiver().try_recv()` ⇒ `App::shell_quit()` pour l'identifiant de Quitter ; libellés FR ou EN selon la langue au démarrage ; `cargo deny check` vert (licences de `muda` et de ses dépendances macOS) ; échec de création du menu ⇒ journal, application sans menu (comportement actuel), jamais de panique.
  - `quit.rs` : `global_keys` lit Ctrl/Cmd+Q, Ctrl/Cmd+W, Ctrl+F4, Ctrl/Cmd+P dans tous les états : saisie de cellule ouverte ⇒ validée (chemin H-UI-86 ; refus ⇒ touche consommée, message, rien d'autre) ; formulaire de détails modifié ⇒ OK implicite (H-UI-113, après L5 : appel prévu, sans effet avant) ; fenêtre modale (`self.view.dialog`, `view_dialog_open`) ⇒ fermée comme par Annuler ; puis `request_quit` / `CloseDocumentGuarded` / mode impression ; `egui::Options::quit_shortcuts` vidé à la création de `App` (ici, par `App::install_quit_policy(ctx)` appelé depuis `global_keys` à la première image) ; journal `PLANRS_LOG_KEYS=1` (touches et modificateurs reçus, cible `planrs::keys`, desktop seulement).
- **Règles** : H-UI-108, H-UI-109.
- **Tests exigés** (`tests/ui/quit.rs`, OS Mac et autre simulés par `ctx.os`) : `h_ui_108_cmd_q_dirty_confirms` (vue Gantt, table, chronologie), `h_ui_108_cmd_q_clean_closes`, `h_ui_108_cmd_q_in_modal` (Trier, Filtre, Collage spécial, Informations ⇒ fenêtre fermée puis bandeau), `h_ui_108_cmd_q_while_editing` (saisie validée puis bandeau ; saisie refusée ⇒ rien d'autre), `h_ui_108_no_builtin_quit_shortcut` (`quit_shortcuts` vide, aucun `ViewportCommand::Close` sans garde), `h_ui_108_shell_quit_guarded` (`shell_quit` ⇒ même chemin), `h_ui_109_cmd_w_closes_document_guarded`, `h_ui_109_ctrl_f4_windows`, `h_ui_109_cmd_p_print_mode`, `h_ui_109_web_leaves_w_p` ; dans `apps/planrs` : `macos_menu_spec` (pur : libellés, raccourcis, Quitter personnalisé ; compilé partout).
- **Critère de fin** : tests verts ; `desktop-build.yml` vert sur macOS (compilation et lien avec `muda`) ; DMG intermédiaire fourni au propriétaire (hors lot : session principale) ; limite du Dock réécrite au § 5 de `ui.md`.
- **Dépendances** : L0. **Parallèle** : L2. **Taille** : ~700 lignes.

---

## L4 — Volets et séparateurs, chronologie fermable

- **Objectif** : chronologie, vue et volet de détails comme des volets de MS Project : fermables, redimensionnables, mémorisés.
- **Fichiers** : § 1, L4.
- **Comportement** :
  - `view/main_view.rs` : rectangle de la vue découpé par `panes::layout` (chronologie si visible et assez de place, vue, détails si `details.visible`) ; deux séparateurs (H-UI-111 : zone de ±4 px, curseur, glisser par `view.split_drag`, `PaneOp::Set*Height` au relâcher, double-clic ⇒ défaut) ; volet du bas = `view::details::show` (contenu en L5) avec un en-tête de 22 px « Formulaire tâche » et un × (⇒ `PaneOp::ShowDetails(false)`) ; F6 / Maj+F6 ⇒ `PaneOp::FocusNext` (focus clavier rendu au volet ; table : H-UI-101 inchangée).
  - `view/timeline.rs` : hauteur lue de `UiState.timeline_height` (`TIMELINE_H` devient la valeur par défaut) ; clic droit n'importe où ⇒ menu avec « Masquer la chronologie » (et « Retirer de la chronologie » sur un élément) ; × au survol (H-UI-110) ; disposition interne des éléments recalculée pour la hauteur (lignes de la frise ajoutées quand la hauteur le permet, H-UI-78 gardée à 72 px).
  - `panes.rs` : `App::pane_op` complet (bornes à l'écriture, état enregistré comme les autres préférences).
- **Règles** : H-UI-110, H-UI-111 ; P63 (séparateurs).
- **Tests exigés** (`tests/ui/panes.rs`) : `h_ui_110_right_click_blank_hides` (TL-01), `h_ui_110_close_button_on_hover`, `h_ui_110_three_paths_same_action` (clic droit, ×, case du ruban), `h_ui_111_timeline_splitter_drag` (TL-02 : 72 ⇒ 152, curseur `ResizeVertical`, gardé après relance par `UiState`), `h_ui_111_splitter_bounds` (glisser hors fenêtre, valeurs non finies), `h_ui_111_double_click_resets`, `h_ui_111_details_splitter`, `h_ui_111_f6_cycles_panes`, `h_ui_81_remove_item_still_works` (non-régression), `p63_split_sequences` (proptest), `splitter_drag_5000` (perf).
- **Critère de fin** : tests verts ; captures inchangées (hauteur par défaut identique, × invisible sans survol, volet de détails fermé par défaut).
- **Dépendances** : L0, L1 (messages hors de la zone des volets). **Parallèle** : L6. **Taille** : ~900 lignes.

---

## L5 — Volet de détails (Formulaire tâche)

- **Objectif** : le « Formulaire tâche » de MS Project sous la vue, avec prédécesseurs, successeurs et ressources éditables.
- **Fichiers** : § 1, L5.
- **Comportement** :
  - `task_form.rs` : `TaskFormDraft` = `TaskInfoDraft` (`from_session`, `to_batch`, fonctions pures) sorti de `dialog/task_info.rs` sans changement de comportement, plus `successors: Vec<LinkRow>` et le nom lu de chaque ligne de lien ; `to_batch` : prédécesseurs par le chemin de `edit::replace_links` (R-260 compris), successeurs par `link.add` / `link.update` / `link.remove` portés par chaque successeur, ressources par `assignment.add` / `assignment.update` / `assignment.remove` (registre J5), champs du haut par `task.update` et `task.set_constraint` ; un lot, libellé « Formulaire tâche » ; aucune lecture dupliquée (`planrs_core::text`).
  - `dialog/task_info.rs` : lit `TaskFormDraft` ; onglet Prédécesseurs inchangé ; comportement et tests de « Informations » inchangés.
  - `view/details.rs` : `show` (H-UI-112) : champs du haut sur deux colonnes, deux grilles selon `details.form` (grille commune de J7, `grid.rs`, lignes vides d'ajout, Alt+↓ pour Type), OK / Annuler, menu contextuel des formes ; `App::details_op` ; suivi de la cellule active à chaque image (brouillon reconstruit seulement quand la tâche ou la génération du document change) ; changement de tâche avec brouillon modifié ⇒ OK implicite (H-UI-113) ; récapitulative et ligne vide gérées ; aucune `f32` non finie.
- **Règles** : H-UI-112, H-UI-113.
- **Tests exigés** (`tests/ui/details.rs`) : `h_ui_112_follows_active_cell` (DET-01), `h_ui_112_shows_successors`, `h_ui_112_forms_switch` (trois formes, gardée dans `UiState`), `h_ui_112_summary_greyed`, `h_ui_112_blank_row_placeholder`, `h_ui_113_ok_one_undo` (nom, durée et successeur changés ⇒ une entrée ; Ctrl+Z ⇒ tout revient), `h_ui_113_cancel_restores`, `h_ui_113_selection_change_commits`, `h_ui_113_refusal_keeps_selection` (ID 99 ⇒ « Il n'y a pas de tâche 99. »), `h_ui_113_identical_noop` (OK sans changement ⇒ aucune entrée), `h_ui_113_resources_units_work`, `h_ui_113_external_change_reloads`, `task_info_*` existants verts, `details_follow_5000` (perf).
- **Critère de fin** : tests verts ; captures inchangées (volet fermé par défaut) ; « Informations sur la tâche » inchangée.
- **Dépendances** : L4 (cadre du volet), L2 (R-260 dans `replace_links`). **Parallèle** : L7. **Taille** : ~1 400 lignes.

---

## L6 — Insertion de colonnes et liste des champs

- **Objectif** : ajouter, masquer, déplacer et régler une colonne comme dans MS Project.
- **Fichiers** : § 1, L6.
- **Comportement** :
  - `field.rs` : catalogue complet de H-UI-117 (titres, catégories, alignements, disponibilité ; valeurs servies en L7 : jusque-là, colonne nouvelle affichée vide et en lecture seule) ; `search` final.
  - `column.rs` : `App::column_op` complet (insertion à gauche, masquer, déplacer, paramètres ; ID en premier ; 64 au plus ; champ déjà affiché ⇒ déplacé).
  - `view/table.rs` : menu d'en-tête de H-UI-114 (Insérer une colonne, Masquer la colonne, Paramètres de champ…, Ajuster, Trier) à la place des cases ; colonne « Ajouter une nouvelle colonne » derrière une constante `SHOW_ADD_COLUMN` (fausse jusqu'à L10, testée vraie par les tests du lot via `FrameCtx`) ; glisser d'en-tête d'une colonne sélectionnée ⇒ déplacement (H-UI-116, `view.column_drag`, trait d'insertion, Échap) ; titre et alignements de `ColumnState`.
  - `dialog/field_picker.rs` : sélecteur surgissant (recherche focalisée, catégories, ↑/↓/Entrée/Échap, raison des grisés) ; `dialog/field_settings.rs` : fenêtre Paramètres de champ (H-UI-115) ; `dialog/mod.rs` : enregistrement.
  - `view/ribbon.rs` : menu Format → Colonnes = « Insérer une colonne… », « Paramètres de champ… », puis les bascules d'affichage dans un sous-menu « Affichage du tableau » séparé (COL-02 passe 2).
- **Règles** : H-UI-114 à H-UI-117 (liste) ; P63 (colonnes).
- **Tests exigés** (`tests/ui/columns_insert.rs`) : `h_ui_114_insert_left_of_clicked` (COL-01 : % achevé inséré à gauche de Durée, pas en fin), `h_ui_114_add_new_column_slot`, `h_ui_114_picker_search` (« trav » ⇒ Travail, Travail réel… ; « duree » sans accent), `h_ui_114_picker_keyboard`, `h_ui_114_existing_field_moves`, `h_ui_114_costs_greyed_reason`, `h_ui_114_max_64`, `h_ui_115_field_settings_title_align` (Alt+F3 et menu), `h_ui_116_drag_selected_header_moves`, `h_ui_116_id_stays_first`, `h_ui_116_unselected_header_selects` (H-UI-90 gardée), `h_ui_117_catalog_complete` (chaque champ de la liste de H-UI-117 présent, titre FR et EN), `h_ui_99_*` existants verts (cases de colonnes retirées : liste au commit), `field_picker_5000` (perf).
- **Critère de fin** : tests verts ; captures inchangées (`SHOW_ADD_COLUMN` faux, menus fermés).
- **Dépendances** : L0. **Parallèle** : L4. **Taille** : ~1 400 lignes.

---

## L7 — Valeurs et saisie des nouveaux champs

- **Objectif** : chaque champ du catalogue affiche la valeur du modèle ou du moteur et se saisit quand le registre le permet.
- **Fichiers** : § 1, L7.
- **Comportement** :
  - `rows.rs` : textes du `RowModel` calculés **seulement pour les colonnes affichées** (cache par génération et liste de colonnes) ; valeurs lues du modèle (priorité, WBS, notes sur une ligne, contrainte, échéance, travail, type, actif, créé le) ou du `Schedule` / `planrs-core` (marges, dates au plus tôt et au plus tard, écarts R-224 et R-225, lignes de base 0 à 10, réels et restants) ; aucune arithmétique de date ou de durée dans l'interface (règle 3).
  - `format.rs` : formats des nouveaux types (oui/non, priorité, type de contrainte, type de tâche, travail en heures, pour-cent du travail).
  - `edit.rs` : `writable` et `commit_edit` étendus aux champs modifiables de H-UI-117 (lecture par `planrs_core::text`, commandes existantes : `task.update`, `task.set_constraint`, `progress.*`, liens inverses pour Successeurs par le chemin de R-260) ; champ à choix : préfixe reconnu ou liste Alt+↓ ; champ calculé ⇒ « Ce champ est calculé. » ; R-259 (effacer) étendue aux nouveaux champs (valeur par défaut du registre ou non effaçable).
  - `field.rs` : disponibilité `ReadOnly` / `Ready` finale.
- **Règles** : H-UI-117 (valeurs) ; R-259 amendée (nouveaux champs, fiche mise à jour dans le même commit).
- **Tests exigés** (`tests/ui/fields.rs`) : `h_ui_117_values_from_engine` (marge libre, au plus tard, écart de fin égaux au `Schedule`, sur « P » et l'exemple embarqué), `h_ui_117_baseline_n_columns` (ligne de base 3 posée ⇒ colonnes Début3, Fin3), `h_ui_117_editable_fields_one_undo` (priorité, contrainte, échéance, travail, actif, successeurs : une entrée chacun, Ctrl+Z), `h_ui_117_computed_refused`, `h_ui_117_choice_prefix` (« man » ⇒ Manuelle), `r259_clear_new_fields`, `h_ui_117_copy_paste_new_columns` (R-257 sur Priorité 2-4), `wide_table_5000` (perf).
- **Critère de fin** : tests verts ; captures inchangées ; budgets J7, J8 et UX tenus (le cache des colonnes affichées n'alourdit pas `main_5000`).
- **Dépendances** : L6 (catalogue), L2 (`edit.rs`). **Parallèle** : L5. **Taille** : ~1 300 lignes.

---

## L8 — Champs personnalisés : cœur, registre, MSPDI

- **Objectif** : Texte1-30, Nombre1-20, Date1-10, Indicateur1-20 dans le modèle, le registre (MCP compris) et le MSPDI, à l'aller-retour exact.
- **Fichiers** : § 1, L8.
- **Comportement** : R-263 à R-266 tels qu'écrits au § 2.3 ; `Project.custom_fields` et `Task.custom` (sérialisation JSON des sessions et du journal de reprise compatibles : champ absent = vide) ; commandes et erreurs de R-264 (undo par instantanés, `dry_run`) ; MSPDI : table `FieldID` ⇄ `CustomFieldId` figée depuis S90 (test `r265_field_id_table` qui cite la page), lecture et écriture de R-265 et R-266, `normalize` étendu, `Extras` des champs modélisés retirés à la lecture (plus de double écriture) ; `extras_invalidation` : rien à jeter (aucune valeur calculée) ; corpus : chaque fichier à `ExtendedAttribute` relu et réécrit à l'identique en dehors des champs modélisés (rapport dans le commit).
- **Règles** : R-263 à R-266, P62 ; R-087, R-123, R-161 relues (amendées si besoin, dans le même commit).
- **Tests exigés** (`tests/rules`) : `r263_value_bounds` (texte 255, nombre 10^14, point fixe), `r263_validate_type_mismatch`, `r264_update_and_clear`, `r264_rename_alias`, `r264_errors` (champ inconnu, type, trop long), `r264_undo_one_entry`, `r264_mcp_same_ops`, `r265_field_id_table`, `r265_read_alias_values`, `r265_formula_field_stays_extras` (octet pour octet), `r265_unreadable_value_diagnostic`, `r266_write_order_and_format`, `r266_roundtrip_corpus` (fichiers du corpus à `ExtendedAttribute`), `p62_custom_roundtrip` (proptest, 256 cas), `custom_mspdi_5000` (perf, dans `planrs-mspdi`), wasm de `planrs-core` vert.
- **Critère de fin** : tests verts ; corpus et rejeux MCP inchangés hors champs modélisés ; `cargo xtask rules` vert.
- **Dépendances** : L0 (types). **Parallèle** : L1. **Taille** : ~1 500 lignes.

---

## L9 — Champs personnalisés : colonnes, saisie, renommage

- **Objectif** : un utilisateur de MS Project retrouve ses Texte1, Nombre1… dans le sélecteur, les renomme et les saisit.
- **Fichiers** : § 1, L9.
- **Comportement** : `field.rs` : entrées `Column::Custom(id)` de la catégorie Personnalisés, titre = alias (« Lot (Texte1) ») ; champs à formule absents ; `rows.rs` : valeurs formatées (nombre dans la langue, date comme Début, Oui/Non) ; `edit.rs` : `writable` et `commit_edit` pour `Custom` ⇒ `task.update { custom }` (R-264), vide ⇒ efface, R-257 à R-260 s'appliquent ; `dialog/custom_fields.rs` : Projet → Champs personnalisés… (liste par type, alias éditables, OK ⇒ un lot de `custom_field.rename`) ; titre de colonne suit l'alias à chaque image (sauf titre de Paramètres de champ).
- **Règles** : H-UI-118.
- **Tests exigés** (`tests/ui/custom_fields.rs`) : `h_ui_118_picker_lists_custom` (Texte1-30, Nombre1-20, Date1-10, Indicateur1-20), `h_ui_118_alias_title`, `h_ui_118_edit_text_number_date_flag` (une entrée chacun, « 12,5 » en FR, « 12.5 » en EN), `h_ui_118_rename_one_undo`, `h_ui_118_save_reload_identical` (enregistrer, relire, mêmes valeurs et alias), `h_ui_118_formula_field_hidden`, `h_ui_118_paste_fill_custom` (Ctrl+D sur Texte1).
- **Critère de fin** : tests verts ; captures inchangées.
- **Dépendances** : L6, L7, L8. **Parallèle** : aucun (fichiers de L7). **Taille** : ~900 lignes.

---

## L10 — Retours visuels et captures (seul)

- **Objectif** : rendre visibles les nouveautés dans le ruban et les menus, finir le dessin des notifications, **une seule** régénération des captures.
- **Fichiers** : § 1, L10.
- **Comportement** : `theme.rs` : jetons des notifications (fond, liseré par sorte, ombre ; texte ≥ 4,5:1 en clair et en sombre, testés) ; `view/chrome.rs` : dessin final des notifications et du bandeau, jumeaux de menus (Affichage → Détails, Format → Insérer une colonne…, Paramètres de champ…, Projet → Champs personnalisés…, Fichier → Fermer avec son raccourci) ; `view/table.rs` : `SHOW_ADD_COLUMN` vrai ; `view/diagnostics.rs` : H-UI-119 ; `ribbon/spec.rs` (table `RIBBON`) : Affichage → Fractionner : case Détails à côté de Chronologie ; Format → Colonnes : Insérer une colonne, Paramètres de champ ; Projet → Propriétés : Champs personnalisés ; icônes Lucide vendorisées (`cargo xtask icons`, SVG copiés de `lucide-static` à la version d'ADR-0017 D6) ; H-UI-75 tenue de 800 à 3 840 px ; exemple `snapshot` : options `--details`, `--columns <liste>`, `--toast` ; `docs/user/*` (« Volets, colonnes et champs personnalisés » pour l'utilisateur de MS Project) ; `ui.md` § 1.35 complété.
- **Captures** (lavapipe, une fois) : toutes les références régénérées ; nouvelles : `details_pane_fr_light`, `details_pane_fr_dark`, `field_picker_fr_light`, `toast_fr_light`, `toast_fr_dark`, `timeline_tall_fr_light`, `custom_columns_fr_light` ; avant/après dans `/mnt/backup/tmp/planrs-ux2-avant-apres/` sur serveurdvp, pour le propriétaire.
- **Tests exigés** : `h_ui_104_toast_contrast` (chaque thème et modèle), `h_ui_119_no_empty_diagnostics_pane`, `h_ui_119_pill_opens_filtered`, `h_ui_70_every_item_has_menu_twin` et `h_ui_70_ribbon_matches_spec` mis à jour, `h_ui_75_no_overflow_widths`, suite `ui` complète captures comparées (`PLANRS_UI_SNAPSHOTS=1`).
- **Critère de fin** : tests et captures verts en CI (job `ui`) ; budgets tenus ; avant/après prêt.
- **Dépendances** : L1 à L9. **Parallèle** : aucun. **Taille** : ~800 lignes (références d'images non comptées).

---

## L11 — Vérification finale

- **Objectif** : preuve que la seconde passe tient les règles du dépôt et les gestes de l'audit avant `v1.0.0`.
- **Fichiers** : § 1, L11.
- **Vérifications** :
  1. `cargo xtask rules` vert ; H-UI-104 à H-UI-120, R-260 à R-266, P61 à P63 ont chacune au moins un test à leur nom ; S90 à S95 citées (et S61, S81, S89 relues).
  2. Scénario kittest de bout en bout `e2e_ux2` sur l'exemple embarqué, **à la souris** sauf les raccourcis cités, gestes de l'audit dans l'ordre du § 0 : n° 1 (N° 11, Cmd+C, glisser Prédécesseurs 9-11, Cmd+V ⇒ refus, 49 tâches) ; n° 2 (9-12, Lier ⇒ 11→12 seul ; Lier de nouveau ⇒ rien) ; n° 4 et 18 (collage de lignes ⇒ notification partie après 10 s, vue immobile) ; n° 9 (refus de cycle ⇒ phrase en ID, détails repliés) ; n° 10, 11, 12, 20 ; n° 3, 13, 14 (Cmd+Q dans la vue, sous « Trier », pendant une saisie ; Cmd+W) ; n° 15, 16 (clic droit dans la frise ⇒ Masquer ; case ⇒ réafficher ; glisser le séparateur) ; n° 5 (Affichage → Détails, successeur ajouté, OK, Ctrl+Z) ; n° 6, 7, 17 (Insérer « Travail » à gauche de Durée, « Ajouter une nouvelle colonne » ⇒ Marge libre, Paramètres de champ, déplacer Fin) ; n° 8 (Texte1 renommé « Lot », saisi) ; n° 21 ; OK-01 ; puis annuler tout ⇒ projet de l'ouverture, rétablir tout, enregistrer et relire ⇒ mêmes tâches, liens et champs personnalisés ; chaque geste d'édition = une entrée, chaque non-changement = zéro ; relance ⇒ volets, hauteurs et colonnes restaurés.
  3. Parité : `h_ui_70_every_item_has_menu_twin` et `h_ui_100_entries_match_ribbon_actions` verts ; chaque raccourci nouveau atteignable par un menu.
  4. Fluidité (§ 2.5) en release, Sobre et Classique, rapport JSON en artefact du job `perf`.
  5. Web : `.wasm` gzip -9 ≤ 4 Mio et hausse ≤ 80 Kio (mesuré par `desktop-build.yml`, chiffre consigné dans ADR-0019) ; W, P, R laissés au navigateur ; notifications et volets sous Chromium sans tête (C-REL-06).
  6. macOS : `desktop-build.yml` vert avec `muda` ; essai du propriétaire (Cmd+Q, Cmd+W, Cmd+H, Cmd+M, collage de la capture) consigné.
  7. Corpus et rejeux inchangés hors champs personnalisés ; nombre de tests ≥ départ de `ux2-dev` + tests du cahier ; `cargo deny check` vert.
  8. Docs : `ui.md` (§ 1.11, § 1.35, § 5), `docs/user/*`, `scorecard.md` (Interface et MSPDI : implémenté, testé ; rien de « vérifié contre MS Project »), `CHANGELOG.md`, ADR-0019 complété des écarts par lot ; squelettes et textes « non disponible » de L0 retirés s'ils ne sont plus produits.
- **Hors lot (session principale et propriétaire)** : avant/après montré ; fusion `ux2-dev` → `main` après une CI **existante et verte sur le SHA de tête** ; essai sur Mac avant `v1.0.0`.
- **Critère de fin** : tous les checks de `CLAUDE.md` verts en local et en CI ; aucune convention du cahier sans test.
- **Dépendances** : tous les lots. **Taille** : ~700 lignes.

## 3. Points laissés ouverts

1. **Lignes copiées collées sur des cellules** (D2) : refus expliqué (Excel) plutôt qu'insertion (MS Project) ; à confirmer à l'essai.
2. **Cmd+Q** (D4) : la cause de la régression du DMG n'est pas établie ; le menu natif la contourne ; journal `PLANRS_LOG_KEYS=1` à lire si l'essai échoue encore. Quitter du Dock reste sans garde (journal de reprise).
3. **Formulaire tâche** (D6) : écriture à OK ou au changement de tâche ; l'alternative « écriture champ par champ » reste possible si le propriétaire la préfère (une annulation par champ).
4. **Coûts** : listés grisés tant que le moteur ne calcule aucun coût ; un jalon « coûts » est à décider après v1.0.0.
5. **Reportés après v1.0.0** (ADR-0019) : formules et tables de choix ; champs personnalisés des ressources ; Formulaire ressource ; poignée de recopie ; tracé de tâche au glisser dans le Gantt ; Ctrl+− du pavé pour supprimer une ligne.
