# Cahier des charges « passe UX » — planrs (avant v1.0.0)

Référence : ADR-0018 (`docs/decisions.md`). Branche d'intégration `ux-dev` (depuis `main` 519d5ee, après le ruban et le correctif de l'édition de cellule). Toute convention ci-dessous est au statut `hypothèse` sauf ce que sa source dit. Chaque lot : un agent, ≤ ~1 500 lignes (code + tests, références d'images non comptées), un worktree et un `CARGO_TARGET_DIR` propres, un commit `type(scope): …` par sous-étape testée, fiche `H-UI-xx` / `R-xxx` et tests **dans le même commit**, messages terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

Checks de chaque lot : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p <crates du lot>`, `cargo xtask panics`, `cargo xtask layers`, `cargo xtask rules`, `cargo xtask text-ids`, `cargo xtask icons --check`, `cargo deny check`, `cargo check -p planrs-core --target wasm32-unknown-unknown`, `cargo check -p planrs-ui-egui --target wasm32-unknown-unknown`. Machine à mémoire partagée (serveurdvp) : `ramwait 2.5` puis `memlimit 3 cargo …` (`memlimit 5` pour `cargo test --workspace`), `export PATH=$HOME/.cargo/bin:$PATH`, une seule commande cargo lourde à la fois.

Clean-room (règle 1) : conventions d'interface tirées des seules pages d'aide Microsoft (`support.microsoft.com`, `learn.microsoft.com` : Project d'abord, Excel quand Project n'a pas d'équivalent), relues par l'agent qui écrit la fiche (titre exact, phrase citée) ; jamais un autre clone (MPXJ, ProjectLibre, GanttProject…) ni sa documentation, ni le code d'un autre tableur. Une affirmation sur MS Project ou Excel sans page relue reste une hypothèse.

Règle 10 : toute position du pointeur (`f32`) passe par un test de finitude avant usage ; aucun indice de ligne hors de `RowView` / `VisibleRows` ; bloc borné à `MAX_BLOCK_CELLS` (200 000 cellules) et texte collé aux bornes de R-256 (4 Mio, 10 000 lignes) : dépassement = message, jamais de panique ; défilement automatique borné (vitesse plafonnée, défilement ramené dans `[0, max]`).

Règle 3 : aucune date calculée par l'interface ; chaque cellule écrite passe par la lecture de la saisie (`edit::commit_edit`) puis le registre ; copier écrit des textes formatés par le moteur (`RowModel`, `edit::initial_text`).

Règles du ruban gardées : toute commande nouvelle a son jumeau au menu (parité clavier, `h_ui_70_every_item_has_menu_twin`), une action, un prédicat (`RibbonCmd::action`, `RibbonCmd::enabled`) ; aucune fonction dupliquée ; un geste = une entrée d'annulation.

## 0. Audit dédoublonné (2026-10-28)

46 constats de deux passes (SEL/EDIT/ROW/COL/CTX/GANTT/VIS/FOCUS/DOC et U01 à U24), relus dans le code de `ux-dev` ; 22 écarts retenus, classés par gravité.

| # | Écart (constats) | Gravité | Décision (ADR-0018) | Lot |
|---|---|---|---|---|
| 1 | Cliquer-glisser ne sélectionne rien, ni cellules ni lignes (SEL-01, U01) — **bug signalé** | bloquant | D1, H-UI-88 | L1 |
| 2 | Sélection au relâcher : un clic qui bouge de plus de ~6 px est perdu (SEL-02, U05) | bloquant | D1, H-UI-88 | L1 |
| 3 | Modèle en lignes seules : pas de bloc, Maj+←/→ = ← → (SEL-03, U02) | bloquant | modèle, D2, H-UI-87, H-UI-92 | L0, L1 |
| 4 | Coller « 7j » sur une Durée crée une tâche ; copier une cellule copie la ligne (EDIT-01, U11, U12) | bloquant | D3, H-UI-95, R-257 | L2 |
| 5 | Suppr sur une cellule supprime la tâche ; saisie vide refusée (EDIT-03, U13, U14) | majeur | D5, H-UI-97, R-259 | L2 |
| 6 | Pas de Remplir vers le bas Ctrl+D (EDIT-02, U10) | majeur | D4, H-UI-96, R-258 | L2 |
| 7 | Pas de Ctrl/Cmd+A, Maj+Espace, Ctrl+Espace, Maj+Retour arrière (SEL-06, U06) | majeur | D2, H-UI-92 | L0, L1 |
| 8 | Glisser sur les ID ne sélectionne pas de lignes (SEL-07, U03) | majeur | D1, H-UI-89 | L1 |
| 9 | Pas de défilement automatique pendant un glisser (sélection, barre, lien) (SEL-04, U04) | majeur | D1, H-UI-91 | L1 |
| 10 | Sélection peu visible : 1,2:1 en clair, 1,5:1 en sombre (SEL-08, U09) | majeur | D11, H-UI-103 | L5 |
| 11 | Glisser une ligne par son ID ne la déplace pas ; H-UI-32 cite un geste absent (ROW-01, U16) | majeur | D6, H-UI-94 | L3 |
| 12 | Clic dans le Gantt : ↓ et Ctrl+C muets mais la frappe ouvre une saisie invisible (EDIT-04, U21) | moyen | D8, H-UI-101 | L1 |
| 13 | Clic gauche sur un en-tête de colonne sans effet ; focus non rendu (SEL-05, U07, FOCUS-01) | moyen | D1, H-UI-90, H-UI-101 | L1 |
| 14 | Menus contextuels sans Couper, Copier, Coller ; clic droit ne pose pas la colonne ; pas de menu d'échelle (CTX-01, U19) | moyen | D10, H-UI-100 | L4 |
| 15 | Ctrl+clic sur la seule ligne sélectionnée ⇒ ligne vide active (U08) | moyen | modèle, H-UI-87 | L0 |
| 16 | Double-clic sur la bordure d'en-tête n'ajuste pas (COL-01, U17) | moyen | D7, H-UI-99 | L3 |
| 17 | Menu d'en-tête sans Ajuster ni Trier (COL-03, U20) | mineur | D7, H-UI-99 | L3 |
| 18 | Entrée/Tab sortent du bloc ; pas de Ctrl+Entrée (EDIT-05, U15) | mineur | D2, D4, H-UI-93, H-UI-98 | L1, L2 |
| 19 | Glisser une barre d'une sélection multiple perd la sélection (GANTT-02) | mineur | D9, H-UI-102 | L1 |
| 20 | Aucun survol de ligne, aucun curseur sur l'ID (VIS-01, U23) | mineur | D11, H-UI-103 | L3 (curseurs), L5 (survol) |
| 21 | Poignée de bordure étroite (±3 pt) (COL-02, partie retenue) | mineur | D7, H-UI-99 | L3 |
| 22 | Glisser dans le vide du Gantt : défilement, jamais sélection (GANTT-01, U22) | mineur | D9 : **gardé**, à signaler | — |
| — | Conventions écrites en retard (H-UI-16, H-UI-32, R-256, ADR-0016 D3) (DOC-01) | doc | amendées en L0 | L0 |
| — | Réordonner les colonnes au glisser d'en-tête (COL-03, U18) | mineur | reporté après v1.0.0 | — |
| — | Conformes, à garder (U24) : Maj+clic, Ctrl+clic, Maj+↓, H-UI-86, focus après ruban, Inser, glisser de bordure, clic droit dans une multi-sélection | info | non-régression | L6 |

Écartés à la relecture : COL-02 « une entrée d'annulation par image du glisser » (largeur dans `UiState`, hors historique) ; FOCUS-01 « focus perdu » (`refocus_table_after_click` le rend à l'image suivante ; une image perdue au plus : corrigé quand même en L1 par `FocusPane(Table)` au clic d'en-tête). Hors cahier : « → » des diagnostics absent d'Ubuntu-Light dans le harnais (ADR-0018, point 6).

## 1. Graphe des lots

```
L0 contrat (modèle de sélection, types, actions, raccourcis, textes, fiches) ─┬─ L1 sélection souris et clavier (table, Gantt, défilement auto) ─┬─ L3 lignes et colonnes (déplacer, ajuster, menu d'en-tête, curseurs) ─┐
                                                                              └─ L2 édition sur sélection (cellules, Ctrl+D, effacer, presse-papiers) ┴─ L4 menus contextuels et jumeaux (table, Gantt, échelle, Édition) ─┴─ L5 retours visuels et captures (seul) ── L6 vérification finale
```

Vagues (au plus 2 agents, fichiers disjoints) : W1 {L0} → W2 {L1, L2} → W3 {L3, L4} → W4 {L5} → avant/après montré au propriétaire (session principale) → W5 {L6}.

Fichiers par lot (un fichier hors liste demande l'accord de la session principale) :
- **L0** : `planrs-ui-egui/src/{selection.rs, nav.rs, block.rs (nouveau), cells.rs (nouveau, squelettes), action.rs, app.rs (bras du réducteur), keymap.rs, grid.rs (Ctrl+Entrée dans `finish_keys`), view/mod.rs (`ViewTransient`), view/table.rs (une ligne : `Finish::FillTyped` ⇒ action), clipboard.rs (champ `ClipboardSlot.kind` seulement), theme.rs (jetons déclarés, inutilisés), i18n.rs, lib.rs}`, `locales/{fr,en}.ftl`, `tests/ui/main.rs` (modules), `tests/perf.rs` (sections vides), `docs/{features/ui.md, scheduling-rules.md}`. Seul lot qui touche `action.rs`, `app.rs`, `keymap.rs`, `i18n.rs`, `locales/*.ftl`.
- **L1** : `src/{selection.rs, block.rs, nav.rs, view/table.rs, view/main_view.rs}`, `tests/ui/{select_mouse.rs, select_keys.rs}`, `tests/perf.rs` (section L1).
- **L2** : `src/{cells.rs, clipboard.rs, edit.rs}`, `tests/ui/{cells.rs, clipboard.rs}`, `tests/perf.rs` (section L2), `docs/scheduling-rules.md` (fiches R-257 à R-259).
- **L3** : `src/{view/table.rs, structure.rs, column.rs}`, `tests/ui/rows_columns.rs`, `tests/perf.rs` (section L3).
- **L4** : `src/{view/main_view.rs, view/chrome.rs, ribbon/spec.rs (variantes de `RibbonCmd`, pas la table `RIBBON`)}`, `tests/ui/context_menus.rs`.
- **L5** : `src/{theme.rs, view/table.rs (dessin), view/gantt.rs (fond des lignes sélectionnées), ribbon/spec.rs (table `RIBBON`), ribbon/icon_data.rs (régénéré)}`, `assets/icons/lucide/` (deux SVG ajoutés), `examples/snapshot.rs`, `tests/snapshots/*`, `tests/ui/{snapshots.rs, ribbon.rs (attendus du groupe Édition)}`, `docs/user/*`.
- **L6** : `tests/ui/ux_e2e.rs`, `tests/perf.rs` (corrections), `.github/workflows/*` (corrections seulement), `CHANGELOG.md`, `docs/{scorecard.md, decisions.md (écarts), features/*.md}`.
- Fichiers partagés en ajout seulement : `tests/ui/main.rs` (modules posés en L0), `tests/perf.rs` (sections `// --- UX Lx ---` posées en L0), `docs/features/ui.md` (§ 1.34 posé en L0, rempli par chaque lot).

Chemin critique : L0 → L1 → L3 → L5 → L6. Coupes, dans l'ordre (si le temps manque avant `v1.0.0`) : menu de l'en-tête de l'échelle (L4), défilement automatique horizontal de la table (L1 : le vertical reste), Ctrl+Entrée (L2), survol des lignes (L5). Jamais coupés : appui-glisser (bug signalé), bloc et cellule active, Ctrl/Cmd+A, copier-coller de cellules, Ctrl+D, Suppr selon la cible, déplacement de lignes, défilement vertical automatique, focus unique, visibilité de la sélection, parité ruban ↔ menu, captures, fluidité.

## 2. Index

Repère des exemples (celui du ruban) : projet « P » : 1 « Gros œuvre » (récapitulative) ⊃ 2 « Fondations » 5j, 3 « Murs » 10j (2→3 FS), 4 « Livraison » jalon (3→4 FS) ; 5 « Second œuvre » ⊃ 6 « Plâtrerie » 4j (3→6 FS), 7 « Peinture » 3j (6→7 FS) ; colonnes visibles par défaut (ID, Nom, Durée, Début, Fin, Prédécesseurs, …) ; fenêtre 1440×900, densité Normale. « (3, Durée) » désigne la cellule de la ligne d'ID 3, colonne Durée.

### 2.1 Sources (table de `docs/scheduling-rules.md`, relues en L0 : titre exact et phrase citée)

| Réf. | Page | Utilisée pour |
|---|---|---|
| S61 | (J7, relue de nouveau) « Keyboard shortcuts for Project » | Phrases à relire et citer : « Select all rows and columns » (Ctrl+Maj+Espace), « Select a row » (Maj+Espace), « Select a column » (Ctrl+Espace), « Reduce a selection to a single field » (Maj+Retour arrière), « Extend the selection … one field » (Maj+←/→), « … to the start / end of the row » (Maj+Origine/Fin), « Fill down a column » (Ctrl+D), « Clear or reset the selected field » (Ctrl+Suppr), « Move within a selection … » (H-UI-87, H-UI-92, H-UI-93, H-UI-96, H-UI-97) |
| S85 | support.microsoft.com, « Move a task » (ou la page Project qui décrit le glisser par l'ID) | Pointer l'ID, glisser, trait d'insertion, l'ID unique est gardé (H-UI-94) |
| S86 | support.microsoft.com, « Select cell contents in Excel » | Cliquer-glisser, Maj+clic, en-tête de ligne ou de colonne, Ctrl+A, cellule active du bloc (H-UI-88 à H-UI-90) |
| S87 | support.microsoft.com, « Keyboard shortcuts in Excel » | Ctrl+D, Ctrl+Entrée « fill the selected cell range with the current entry », Suppr efface le contenu (H-UI-96 à H-UI-98) |
| S88 | support.microsoft.com, « Change the column width or row height in Excel » (et la page Project sur la largeur de colonne si elle existe) | Double-clic sur la bordure ⇒ ajustement au contenu (H-UI-99) |
| S89 | support.microsoft.com, « Move or copy cells and cell contents » | Coller un bloc à partir de la cellule active, une valeur dans un bloc (H-UI-95, R-257) |

Une page introuvable ou qui ne dit pas ce qui est cité ⇒ la convention passe en `hypothèse` pure, consigné dans ADR-0018. Le menu contextuel des tâches de Project et le groupe Editing (Fill, Clear) : S80 relue pour le groupe Editing ; le reste en hypothèse.

### 2.2 Conventions d'interface (table de `ui.md` § 1.11 ; une ligne par convention, testée par kittest `h_ui_NN_*`)

| Id | Convention | Source | Exemple | Lot |
|---|---|---|---|---|
| H-UI-87 | Sélection = bloc de cellules (`Cells`) ou lignes entières (`Rows`), avec ancre (ligne, colonne) et cellule active toujours dedans ; bloc = lignes **affichées** de l'ancre à l'active × colonnes **visibles** de l'ancre à l'active ; Ctrl/Cmd+clic ajoute ou retire une ligne (bloc réduit à la colonne cliquée), jamais la dernière ; commandes de tâche sur les lignes de la sélection, commandes de cellule sur le bloc. **Remplace H-UI-16** | S61 (« Extend the selection… ») ; S86 ; reste : hypothèse | Appui (2, Durée), Maj+appui (5, Début) ⇒ bloc 2-5 × Durée-Début, active (5, Début) ; Ctrl+clic sur 3 seule sélectionnée ⇒ rien ne change | L0 |
| H-UI-88 | Appui sur une cellule (sans saisie ouverte) : sélection immédiate (cellule seule, ancre posée) ; glisser : bloc de l'ancre à la cellule sous le pointeur, mis à jour à chaque image ; Maj+appui : étend depuis l'ancre ; Ctrl/Cmd+appui : H-UI-87, sans glisser (la multi-sélection n'est jamais remplacée par une plage ; relecture finale, n° 6) ; relâcher : rien de plus ; un mouvement sous le seuil de glisser d'egui (`max_click_dist`, 6 points), même s'il franchit une bordure (cellule, ID, titre de colonne), = même cellule, et au-delà chaque changement de cellule compte (relecture finale, n° 1) ; double-clic inchangé (H-UI-19) ; saisie ouverte : H-UI-86 (valider d'abord, pas de glisser) ; la ligne vide n'entre jamais dans un bloc de plus d'une ligne | S86 ; hypothèse | Appui (2, Durée), glisser jusqu'à (5, Début), relâcher ⇒ 8 cellules, active (5, Début) ; appui (6, Nom) et 8 px de bougé ⇒ (6, Nom) sélectionnée ; appui à 2 px de la bordure droite du Nom, 3 px de bougé vers la Durée ⇒ (6, Nom) seule | L1 |
| H-UI-89 | Colonne ID = en-tête de ligne : appui ⇒ ligne entière (`Rows`), glisser ⇒ plage de lignes entières, Maj/Ctrl comme H-UI-87 ; appui sur l'ID d'une ligne **déjà** sélectionnée en lignes entières puis glisser ⇒ déplacement (H-UI-94) ; double-clic sur l'ID : Informations (H-UI-19) | S86 (en-têtes de ligne) ; S85 ; hypothèse | Appui ID 2, glisser jusqu'à ID 6 ⇒ lignes 2 à 6 entières | L1 |
| H-UI-90 | En-tête de colonne : clic ⇒ colonne entière (lignes affichées, active en première ligne) ; glisser sur les en-têtes ou Maj+clic ⇒ plusieurs colonnes ; clic sur l'en-tête de l'ID ⇒ tout (H-UI-92) ; le clavier reste à la table | S61 (« Select a column ») ; S86 | Clic sur « Durée » ⇒ Durée de 1 à 7, active (1, Durée) | L1 |
| H-UI-91 | Défilement automatique pendant tout glisser de la vue (sélection, déplacement de lignes, barre, lien) : pointeur à moins d'une ligne du bord haut ou bas du corps, ou au-delà ⇒ défilement de `v = clamp(4 + 8·d / hauteur_ligne, 4, 40)` lignes par seconde (`d` : distance au seuil, `dt` de l'image), sélection ou cible suivant la ligne sous le pointeur borné au corps ; horizontal pour un bloc de la table à moins de 20 px du bord gauche/droit, ou au-delà (même formule, `d` rapporté à 20 px, en colonnes de 80 px) ; dans chaque axe, seulement dans le sens où le pointeur s'est éloigné du point d'appui d'au moins le seuil de glisser (relecture finale, n° 2) ; aucune `f32` non finie | S86 (comportement d'Excel) ; vitesses : hypothèse | Glisser de (3, Nom) au bord bas, maintenu 30 images à 60 Hz ⇒ défilement > 0 et bloc étendu aux lignes devenues visibles ; glisser vertical à 40 px du bord droit avec 1 px de dérive vers la droite ⇒ aucun défilement horizontal | L1 |
| H-UI-92 | Raccourcis : Maj+←/→ étend d'une colonne ; Maj+Origine/Fin jusqu'au premier/dernier champ ; Ctrl+Maj+Origine/Fin jusqu'à la première/dernière cellule ; Ctrl/Cmd+A et Ctrl+Maj+Espace : tout (lignes entières, active (première ligne, Nom)) ; Maj+Espace : lignes entières de la sélection ; Ctrl+Espace (Ctrl aussi sur macOS) : colonnes entières de la sélection ; Maj+Retour arrière : cellule active seule ; contexte Table ; retenus au navigateur sur le web ; Maj+Espace ne tape pas d'espace | S61 ; Ctrl+A : S86 ; macOS : hypothèse | (3, Durée), Maj+→ ⇒ bloc Durée-Début de la ligne 3 ; Ctrl+A ⇒ 7/7 lignes entières | L0 (touches), L1 |
| H-UI-93 | Bloc de plus d'une cellule, hors saisie et en fin de saisie : Entrée descend dans le bloc (en bas : haut de la colonne suivante du bloc, en fin : première cellule), Maj+Entrée l'inverse ; Tab va à droite (en bout : début de la ligne suivante du bloc), Maj+Tab l'inverse ; la sélection ne change pas ; cellule seule : H-UI-13, H-UI-14 | S61 (« Move within a selection down one field. ») | Bloc 2-3 × Durée-Début, active (2, Durée) : Entrée ⇒ (3, Durée), Entrée ⇒ (2, Début) | L1 |
| H-UI-94 | Déplacer des lignes : appui sur l'ID d'une ligne sélectionnée en lignes entières, glisser au-delà du seuil ⇒ curseur de déplacement, trait d'insertion de 2 px entre deux lignes ; lâcher ⇒ un lot de `task.move` (`Placement::Before` de la tâche sous le trait, `After` de la dernière tâche de premier niveau en fin) des tâches sélectionnées sans ancêtre sélectionné, dans l'ordre ; UID et avancement gardés ; refus (message, rien écrit) : lâcher dans son propre bloc, lignes non sœurs voisines (`siblings_adjacent`), tri ou filtre (H-UI-32 : curseur interdit, raison au survol) ; Échap annule ; une entrée d'annulation. **Amende H-UI-32** (le geste existe désormais) | S85 ; reste : hypothèse | Lignes 6-7 entières, glisser l'ID 6 au-dessus de 2 ⇒ 6 et 7 sous 1, avant 2 ; Ctrl+Z ⇒ P | L3 |
| H-UI-95 | Presse-papiers selon le type : lignes entières ⇒ H-UI-39 (tampon `RowClip`, insertion) ; bloc ⇒ copier écrit le texte tabulé des cellules (texte de saisie de `edit::initial_text` pour une colonne modifiable, texte affiché sinon), couper = copier puis effacer (R-259) ; coller (R-257) : tampon de lignes ⇒ lignes ; texte à en-tête reconnu ⇒ R-256 a ; sinon cellules à partir du coin haut-gauche du bloc ; un clic dans le Gantt ne coupe plus le presse-papiers (H-UI-101). **Amende H-UI-39** | S89 ; hypothèse | (3, Durée) active, coller « 7j » ⇒ 3 dure 7 j, toujours 7 tâches, une annulation ; copier (3, Durée) ⇒ « 10j » ; lignes 6-7 entières copiées puis collées sur (2, Nom) ⇒ deux tâches insérées avant 2 | L2 |
| H-UI-96 | Ctrl/Cmd+D : remplir vers le bas (R-258) ; menu Édition et ruban (Édition → Remplir ▾ → Vers le bas) ; web : retenu au navigateur | S61 (« Fill down a column. ») ; S87 | Bloc (2, Durée)-(3, Durée), Ctrl+D ⇒ 3 dure 5 j | L0 (touche), L2 |
| H-UI-97 | Suppr : lignes entières, ou bloc d'**une** cellule dans l'ID ou le Nom ⇒ supprimer les tâches (H-UI de J4 ; message « n tâche(s) supprimée(s) — Ctrl/Cmd+Z annule ») ; tout autre bloc (plusieurs cellules, colonne entière du Nom comprise, quelle que soit la cellule active) ⇒ effacer son contenu (R-259 ; relecture finale, n° 3) ; Ctrl+Suppr : effacer toujours ; Cmd+Retour arrière (macOS) : supprimer les tâches ; Retour arrière : saisie vide de la cellule active, dont la validation suit R-259 | S61 (« Clear or reset the selected field. ») ; S87 (Suppr d'Excel) ; partage : hypothèse | (3, Prédécesseurs), Suppr ⇒ lien 2→3 retiré, 3 reste ; (3, Nom), Suppr ⇒ 3 supprimée, message ; clic sur le titre « Nom » puis Suppr ⇒ noms effacés, aucune tâche supprimée | L0 (touches), L2 |
| H-UI-98 | Saisie ouverte dans un bloc de plus d'une cellule : Ctrl/Cmd+Entrée valide le texte dans **toutes** les cellules modifiables du bloc (même lecture que la saisie), un lot ; refus d'une cellule ⇒ saisie gardée, message (H-UI-86) | S87 ; hypothèse | Bloc Durée 2-3, frappe « 2 », Ctrl+Entrée ⇒ 2 et 3 durent 2 j, une annulation | L0 (touche), L2 |
| H-UI-99 | Colonnes : poignée de ±4 px au bord droit de l'en-tête (curseur de redimensionnement) ; double-clic ⇒ largeur ajustée au plus large des textes des lignes affichées et du titre (marges et retrait de niveau du Nom compris), bornée `[MIN_WIDTH, MAX_WIDTH]` ; menu d'en-tête : Masquer la colonne, Ajuster la largeur, Trier par ordre croissant, Trier par ordre décroissant (`TaskViewOp::SetSort`, clés de `SortKey` seulement, grisé sinon), puis les cases des colonnes | S88 ; menu : hypothèse | Double-clic sur la bordure de « Nom » ⇒ largeur = « Second œuvre » + retrait + marges | L3 |
| H-UI-100 | Menus contextuels jumeaux du ruban (`RibbonCmd` : même action, même prédicat, même raccourci affiché) : table et Gantt = [Mettre en forme les barres (sur une barre)] · Couper, Copier, Coller, Collage spécial… · Remplir vers le bas, Effacer le contenu · Insérer une tâche, Supprimer la tâche · Indenter, Désindenter · Lier, Délier · Désactiver [ ], Planifier manuellement [ ] · Informations, Notes, Affecter les ressources…, Défiler jusqu'à la tâche, Ajouter à la chronologie ; clic droit hors de la sélection ⇒ la cellule cliquée devient la sélection, dans la sélection ⇒ gardée ; en-tête de l'échelle : Échelle de temps ▾, Zoom avant, Zoom arrière, Projet entier ; menu Édition : Remplir vers le bas (Ctrl+D), Effacer le contenu (Ctrl+Suppr), Tout sélectionner (Ctrl+A) | S81 (clic droit « Add to Timeline ») ; S80 (Editing) ; reste : hypothèse | Clic droit sur (6, Durée) hors sélection ⇒ (6, Durée) sélectionnée, menu avec « Copier (Ctrl+C) » | L4 |
| H-UI-101 | Clavier unique : un clic dans le Gantt (barre, vide, lien), sur un en-tête ou une poignée laisse le clavier à la table ; flèches, frappe, presse-papiers, Ctrl/Cmd+A agissent sur la cellule active quel que soit le volet cliqué. **Remplace** « Clic dans le Gantt : le clavier quitte la table » (J7 L1) | hypothèse (vue Gantt de Project : une seule sélection) | Clic sur la barre de 6, ↓ ⇒ 7 active ; Ctrl+C ⇒ cellule active copiée | L1 |
| H-UI-102 | Glisser une barre d'une tâche déjà sélectionnée garde la sélection (seule la barre glissée bouge) ; d'une tâche non sélectionnée : la sélectionne seule ; glisser dans le vide : défilement horizontal (H-UI-20 inchangée) | hypothèse | Sélection {2, 6}, glisser la barre de 6 ⇒ 6 déplacée, {2, 6} gardée | L1 |
| H-UI-103 | Retours visuels : bloc teinté (`selection`), cellule active non teintée dans son cadre de 2 px, bordure de 2 px du bloc (`selection_border`, ≥ 3:1 sur le fond et la bande), ID des lignes sélectionnées en en-tête appuyé (`header_selected`), lignes entières teintées sur toute la largeur et dans le Gantt ; teinte ≥ 1,5:1 sur le fond et ≥ 1,3:1 sur le zébrage et les bandes, texte ≥ 4,5:1 dessus, en clair et en sombre ; survol discret de la ligne (`hover_row`, jamais pendant un glisser) ; curseurs : déplacement sur l'ID d'une ligne sélectionnée en lignes entières, redimensionnement sur les poignées, interdit pendant un déplacement refusé ; éléments grisés : raison en info-bulle (H-UI-73) | WCAG 1.4.11 ; reste : hypothèse | Bloc 2-5 × Durée-Début en clair ⇒ teinte, bordure, (5, Début) blanche dans son cadre, ID 2 à 5 appuyés | L3 (curseurs), L5 |

Fiches amendées en L0 : H-UI-16 (« remplacée par H-UI-87 et H-UI-92 »), H-UI-32 (« glisser-déplacer de lignes : H-UI-94 »), H-UI-39 (« bloc de cellules : H-UI-95 »), § 1.12 (focus du Gantt : H-UI-101), R-256 (« appelée pour une sélection de lignes entières ou un texte à en-tête ; bloc de cellules : R-257 »), ADR-0016 D3 (« collage cellule par cellule : fait par ADR-0018 D3 »).

### 2.3 Règles (`docs/scheduling-rules.md`, section « Usage quotidien » ; numéros réservés en L0, fiches écrites par L2 avec leurs tests, règle 7)

- **R-257 — Coller dans des cellules.** `cells::paste_plan(block, texte, session, rows, langue)` : bornes de R-256 ; lignes `\n` ou `\r\n` (ligne vide finale ignorée), cellules par tabulation ; grille `h × l` ; texte d'une seule cellule ⇒ recopiée dans tout le bloc ; sinon posée à partir du coin haut-gauche du bloc, sur les lignes **affichées** suivantes et les colonnes **visibles** suivantes ; colonne calculée ou cellule non modifiable (durée, début, fin d'une récapitulative) ⇒ sautée et comptée ; colonnes au-delà de la dernière visible ⇒ ignorées et comptées ; lignes au-delà de la dernière tâche ⇒ tâches ajoutées en fin comme une saisie sur la ligne vide (`edit::commit_edit` de la ligne vide), refusé sous tri ou filtre (`cells.past_end`) ; chaque cellule ⇒ `edit::commit_edit(EditState::typed(uid, colonne, texte))`, opérations concaténées dans **un** lot (`Batch.ops`), libellé « Coller » ; une lecture refusée ⇒ tout le collage refusé (`cells.parse`, ID et colonne de la première cellule fautive), rien écrit ; refus du registre ⇒ bandeau, rien écrit. Exemple (P) : (3, Durée) active, « 7j » ⇒ 3 dure 7 j ; bloc Durée 6-7, « 2j » ⇒ 6 et 7 durent 2 j ; (2, Durée), « 3j\n4j » ⇒ 2 = 3 j, 3 = 4 j ; (2, Durée), « abc » ⇒ `cells.parse` « 2, Durée », rien écrit.
- **R-258 — Remplir vers le bas.** `cells::fill_down_plan` : pour chaque colonne du bloc, texte de saisie (`edit::initial_text`) de la première ligne du bloc recopié dans les lignes suivantes du bloc ; bloc d'une seule ligne ⇒ source = ligne affichée au-dessus (aucune au-dessus ⇒ rien, message) ; cellules non modifiables sautées et comptées ; un lot ; Ctrl+Entrée (`cells::fill_with`) : même chemin avec le texte saisi pour toutes les cellules du bloc. Exemple : bloc Durée 2-3, Ctrl+D ⇒ 3 dure 5 j ; (7, Durée) seule ⇒ 7 dure 4 j (copiée de 6) ; bloc Durée 4-6 ⇒ 5 (récapitulative) sautée, 6 reçoit le texte de 4, « 1 cellule non modifiable » au bandeau.
- **R-259 — Effacer le contenu.** `cells::clear_plan` et validation d'une saisie vide : Nom ⇒ nom vide si le registre l'accepte (sinon non effaçable) ; Prédécesseurs ⇒ liens retirés (chemin existant de la saisie vide) ; % achevé ⇒ 0 ; Début ou Fin d'une tâche automatique ⇒ contrainte par défaut du sens de planification (`task.set_constraint`), date d'une tâche manuelle ⇒ non effaçable ; Durée ⇒ non effaçable ; calculées ⇒ non effaçables ; non effaçables comptées au bandeau (« n cellules ne peuvent pas être vides ») ; un lot ; rien à effacer ⇒ aucun lot. Exemple : bloc Prédécesseurs 3 et 6, Ctrl+Suppr ⇒ 2→3 et 3→6 retirés, une annulation ; (3, Durée), Suppr ⇒ rien, message.

### 2.4 Propriétés

| Id | Énoncé | Lot |
|---|---|---|
| P59 | Pour tout projet généré (≤ 300 tâches, récapitulatives, jalons, manuelles), toute vue (filtre, tri, repli), tout bloc valide et tout texte (grille ≤ 40 × 16 de jetons tirés parmi durées, dates, nombres, vides, texte libre, tabulations et fins de ligne mêlées) : `paste_plan`, `fill_down_plan`, `clear_plan`, `fill_with` ne paniquent pas ; `Ok` ⇒ chaque opération vise une tâche d'une ligne du bloc (ou un `task.add` en fin pour R-257) et un champ d'une colonne du bloc ; deux appels ⇒ même lot ; `Err` ⇒ aucun lot | L2 |
| P60 | Pour toute suite (≤ 64) d'appuis, de glissers, de raccourcis de sélection et de navigation, sur une vue générée (filtre, tri, repli, colonnes masquées) : aucune panique ; cellule active dans la sélection ; lignes ⊆ lignes affichées ; colonnes = plage contiguë de colonnes visibles ; ligne vide seulement en sélection d'une cellule ; `Rows` ⇒ toutes les colonnes visibles | L0 |

### 2.5 Budgets de performance (release, `tests/perf.rs`, job `perf` ; réseau généré de 5 000 tâches, 1920×1080, ruban déplié)

| Scénario | Mesure | Budget | Lot |
|---|---|---|---|
| Scénarios J7, J8 et ruban existants | inchangés | inchangés | L1 à L6 |
| `select_drag_5000` : appui ligne 10, glisser jusqu'au bord bas et 120 images de défilement automatique, puis 60 images de glisser horizontal | durée d'image | médiane < 16 ms, p95 < 32 ms | L1 |
| `select_all_copy_5000` : Ctrl+A puis copie d'un bloc 5 000 × 3 | image de Ctrl+A ; durée de la copie | < 16 ms ; < 200 ms | L1, L2 |
| `fill_down_1000` : Ctrl+D sur Durée de 1 000 lignes ; `paste_cells_1000` : collage d'un texte 1 000 × 2 | durée du geste (lot appliqué) | objectif 500 ms, porte bloquante 1 s (comme `paste_1000`, ADR-0016 F4) | L2 |
| `row_drag_5000` : glisser l'ID d'une ligne sur 60 images avec trait d'insertion ; `best_fit_5000` : double-clic sur la bordure de Nom | durée d'image ; durée de l'ajustement | médiane < 16 ms, p95 < 32 ms ; < 50 ms | L3 |
| Web | `.wasm` gzip -9 | ≤ 4 Mio (C-REL-05), hausse ≤ 60 Kio par rapport à `ux-dev` au départ | L6 |

---

## L0 — Contrat (séquentiel, en premier)

- **Objectif** : figer le modèle de sélection (implémenté et testé, pur), les types, actions, raccourcis, textes et fiches ; squelettes qui compilent (refus « non disponible », jamais `todo!`) ; **aucun changement visible** (captures intactes ; un clic sélectionne et se dessine comme aujourd'hui).
- **Sources** : relire S61 (phrases du § 2.1) et S85 à S89 ; les écrire dans `docs/scheduling-rules.md` (table des sources, titre exact, phrase citée) ; écart ⇒ convention en `hypothèse` pure, noté dans ADR-0018.
- **Types et API exactes (contrat)** :

```rust
// planrs_ui_egui::selection (implémenté en L0)
pub enum SelectionKind { #[default] Cells, Rows }
// Selection gagne `kind: SelectionKind` ; API ajoutée :
impl Selection {
  pub const fn kind(&self) -> SelectionKind;
  pub fn all(rows: VisibleRows<'_>) -> Self;                       // Rows, ancre et active : première ligne affichée
  pub fn range(anchor: TaskUid, to: TaskUid, kind: SelectionKind, rows: VisibleRows<'_>) -> Self;
  pub fn with_kind(self, kind: SelectionKind) -> Self;
}
// toggle de la seule tâche sélectionnée : sans effet (U08) ; `single` ⇒ Cells

// planrs_ui_egui::nav
pub struct CellCursor<U, C> { pub row: RowRef<U>, pub column: C, pub anchor: Option<U>,
  pub anchor_column: Option<C> /* ancre de colonnes du bloc ; None : colonne active seule */ }
pub fn autoscroll_speed(pointer: f32, top: f32, bottom: f32, extent: f32) -> f32; // H-UI-91, lignes/s signées, 0 si non fini
pub enum InBlockMove { Down, Up, Right, Left }                                     // H-UI-93

// planrs_ui_egui::block (nouveau, pur, implémenté en L0)
pub const MAX_BLOCK_CELLS: usize = 200_000;
pub struct Block { pub kind: SelectionKind, pub rows: Vec<DisplayRow> /* ordre d'affichage */,
  pub columns: Vec<Column> /* visibles, contiguës */, pub active: (RowRef<TaskUid>, Column) }
pub fn resolve(sel: &Selection, cursor: &TableCursor, rows: VisibleRows<'_>, visible: &[Column]) -> Block;
impl Block { pub fn contains(&self, d: DisplayRow, c: Column) -> bool; pub fn cells(&self) -> usize;
  pub fn is_single(&self) -> bool; pub fn step(&self, from: (DisplayRow, Column), mv: InBlockMove) -> (DisplayRow, Column); }

// planrs_ui_egui::action (variantes ajoutées en fin d'énumération)
Action::SelectOp(SelectOp), Action::Cells(CellOp), Action::MoveRows { before: Option<TaskUid> }, Action::FitColumn(Column)
pub enum SelectOp {
  Press { row: RowRef<TaskUid>, column: Column, target: PressTarget, mode: PressMode },
  DragTo { row: RowRef<TaskUid>, column: Column },
  All, EntireRows, EntireColumns, Collapse,
}
pub enum PressTarget { Cell, RowHeader, ColumnHeader }
pub enum PressMode { Replace, Extend /* Maj */, Toggle /* Ctrl/Cmd */ }
pub enum CellOp { FillDown, Clear, DeleteKey, FillTyped(String) }
// `Action::Select` et `SelectWith` gardés (Gantt, chronologie, agents) : ils posent une sélection `Cells` d'une cellule

// planrs_ui_egui::view::ViewTransient (champs ajoutés)
pub select_drag: Option<SelectDrag>, pub row_drag: Option<RowDrag>
pub struct SelectDrag { pub target: PressTarget, pub last: (RowRef<TaskUid>, Column) }
pub struct RowDrag { pub uids: Vec<TaskUid>, pub insert_before: Option<DisplayRow> /* None : fin */, pub refusal: Option<TextId> }

// planrs_ui_egui::cells (nouveau ; squelettes ⇒ Err(CellError::NotAvailable), retiré en L2)
pub struct CellPlan { pub batch: Batch, pub written: usize, pub skipped: usize }
pub enum CellError { NotAvailable, TooLarge, Parse { id: u32, column: Column, detail: String }, PastEnd, NothingAbove }
pub fn copy_text(block: &Block, doc: &OpenDocument, rows: VisibleRows<'_>, lang: TextLanguage) -> String;
pub fn paste_plan(block: &Block, text: &str, s: &Session, rows: VisibleRows<'_>, lang: TextLanguage) -> Result<CellPlan, CellError>;
pub fn fill_down_plan(block: &Block, s: &Session, rows: VisibleRows<'_>, lang: TextLanguage) -> Result<Option<CellPlan>, CellError>;
pub fn fill_with(block: &Block, text: &str, s: &Session, rows: VisibleRows<'_>, lang: TextLanguage) -> Result<CellPlan, CellError>;
pub fn clear_plan(block: &Block, s: &Session, rows: VisibleRows<'_>, lang: TextLanguage) -> Result<Option<CellPlan>, CellError>;
pub enum DeleteTarget { Tasks, Contents }  pub fn delete_target(block: &Block) -> DeleteTarget; // H-UI-97, implémenté en L0

// planrs_ui_egui::clipboard
pub enum ClipKind { #[default] Rows, Cells }  // ClipboardSlot.kind

// planrs_ui_egui::grid
Finish::FillTyped   // Ctrl/Cmd+Entrée dans `finish_keys` ; table.rs le traduit en Action::Cells(CellOp::FillTyped(texte))

// planrs_ui_egui::keymap (UiBinding ajoutés ; entrées avant les moins précises de la même touche)
UiBinding::{SelectAll, SelectRows, SelectColumns, CollapseSelection, FillDown, ClearCells}
// Table : COMMAND+A, COMMAND_SHIFT+Space ⇒ SelectAll ; SHIFT+Space ⇒ SelectRows ; CTRL+Space ⇒ SelectColumns ;
//   SHIFT+Backspace ⇒ CollapseSelection ; COMMAND+D ⇒ FillDown ; COMMAND+Delete ⇒ ClearCells ;
//   nav(SHIFT, ←/→, Left/Right, true), nav(SHIFT, Home/End, RowStart/RowEnd, true), nav(COMMAND_SHIFT, Home/End, First/Last, true),
//   nav(COMMAND_SHIFT, ←/→, RowStart/RowEnd, true)
// `entry_action` : (Command("task.delete"), Delete sans modificateur) ⇒ Action::Cells(CellOp::DeleteKey) (le raccourci « Suppr » reste affiché pour Supprimer)
// theme::Palette : jetons `selection_border`, `header_selected`, `hover_row` (valeurs posées, non dessinés avant L5)
```

- **Réducteur (`app.rs`, posé en L0, rien d'autre à y toucher ensuite)** : `SelectOp(op)` ⇒ `App::select_op(op)` (`selection.rs`, `impl App` ; L0 : `Press` et `All` servis par le modèle, les autres sans effet) ; `Nav` étendu en colonnes ⇒ `App::navigate` (L0 : comportement actuel) ; `Cells(op)` ⇒ `App::cells_op(op, ctx)` (`cells.rs` ; L0 : `DeleteKey` ⇒ `Structure(Delete)` comme aujourd'hui, le reste message « non disponible ») ; `MoveRows` ⇒ `App::move_rows` (`structure.rs`, L0 : message) ; `FitColumn` ⇒ `App::fit_column` (`column.rs`, L0 : rien). Chaque appel `impl App` vit dans le fichier du lot qui le remplit.
- **Textes** : toutes les clés FR et EN (menus Édition et contextuels nouveaux, Remplir ▾, Effacer ▾, Tout sélectionner, Ajuster la largeur, Trier croissant / décroissant, messages `cells-*`, `rows-move-*`, « n tâche(s) supprimée(s) — Ctrl/Cmd+Z annule », info-bulles), sections `# --- UX Lx ---` ; chaque `TextId` nouveau est référencé par un squelette (`cargo xtask text-ids` vert dès L0).
- **Docs** : `ui.md` § 1.11 (H-UI-87 à H-UI-103), § 1.34 « Sélection, gestes et édition sur sélection (ADR-0018) » posé vide, amendements du § 2.2 ; `scheduling-rules.md` S85 à S89, relecture de S61, numéros R-257 à R-259 réservés dans l'en-tête de la section « Usage quotidien » (fiches écrites par L2 avec leurs tests : règle 7, `cargo xtask rules`), amendement de R-256 ; P59 et P60.
- **Tests exigés** : `h_ui_87_block_resolution` (exemple de H-UI-87), `h_ui_87_toggle_last_row_noop` (U08), `h_ui_87_rows_kind_all_columns`, `p60_selection_invariants` (proptest, 256 cas), `block_step_in_block` (H-UI-93, unitaire), `nav_autoscroll_speed_bounds` (non fini, seuils, plafond 40), `h_ui_97_delete_target` (unitaire : Rows, ID, Nom d'une cellule ⇒ Tasks ; Durée, bloc de plusieurs cellules ⇒ Contents), `keymap_ux_bindings_resolve` (chaque nouvelle entrée résolue, avant les moins précises, `web_interceptable`), `keymap_shift_arrow_extends` (Maj+→ ne résout plus `Right` non étendu), tous les tests existants verts et **inchangés** (Suppr supprime toujours en L0).
- **Critère de fin** : checks verts ; captures comparées sous lavapipe identiques ; aucun comportement visible changé.
- **Dépendances** : aucune. **Taille** : ~1 200 lignes.

---

## L1 — Sélection à la souris et au clavier (table, Gantt, défilement automatique)

- **Objectif** : le bug signalé corrigé ; tous les gestes de sélection de H-UI-87 à H-UI-93, H-UI-101, H-UI-102.
- **Fichiers** : § 1, L1.
- **Comportement** :
  - `view/table.rs` : `row_widget` en `Sense::click_and_drag()` ; appui détecté à l'image de l'appui (`contains_pointer` et `pointer.primary_pressed()`, position d'appui `press_origin`) ⇒ `SelectOp::Press { target: Cell | RowHeader (colonne ID), mode }` + `FocusPane(Table)` ; glisser ⇒ `SelectOp::DragTo` quand la cellule sous le pointeur change (pointeur hors du corps : ligne et colonne bornées) ; `view.select_drag` tenu de l'appui au relâcher ; double-clic inchangé (H-UI-19) ; saisie ouverte : chemin H-UI-86 inchangé (clic différé, aucun glisser) ; en-tête : clic ⇒ `Press { target: ColumnHeader }`, glisser sur les en-têtes ⇒ `DragTo`, en-tête de l'ID ⇒ `SelectOp::All` ; poignées et en-têtes poussent `FocusPane(Table)` ; Maj+Espace : l'événement `Text(" ")` retiré avant `take_typed`.
  - `selection.rs` (`impl App`) : `select_op` sert tous les `SelectOp` sur `selected`, `selection` et `view.cursor` (ancre de colonnes), puis `sync_selection` ; `Press` avec `Replace` pose aussi `anchor_column`.
  - `nav.rs` : `navigate` étendu en colonnes (H-UI-92) ; bloc de plus d'une cellule ⇒ Entrée, Tab et leurs Maj par `Block::step` (H-UI-93), y compris la fin de saisie (`Action::Nav` déjà poussé par `edit_cells`) ; défilement automatique (H-UI-91) : `App::autoscroll(dt)` appelé par la vue tant qu'un glisser de la vue est en cours (`select_drag`, `row_drag`, glisser de barre ou de lien), écrit `pending_scroll_y` / `table_scroll_x`, borné.
  - `view/main_view.rs` : clic et appui dans le Gantt ⇒ sélection (`Select`/`SelectWith`, ligne ; depuis la relecture finale n° 4 : `SelectOp::Press { Cell }` dans la colonne active, un clic simple réduit toujours la sélection) et `FocusPane(Table)` (H-UI-101) ; `bar_gesture` ne pousse `Select` que si la tâche n'est pas sélectionnée (H-UI-102) ; défilement automatique pendant un glisser de barre ou de lien (H-UI-91) ; fond : le glisser horizontal du Gantt reste (H-UI-20) ; repeint demandé (`request_repaint`) pendant le défilement automatique.
- **Règles** : H-UI-87 à H-UI-93, H-UI-101, H-UI-102, P60 (complétée par les gestes).
- **Tests exigés** (`tests/ui/select_mouse.rs`, `tests/ui/select_keys.rs`, sur « P » sauf mention) : `h_ui_88_drag_selects_block` (U01 : 8 cellules, active (5, Début)), `h_ui_88_press_selects_before_release`, `h_ui_88_jitter_click_selects` (U05 : 8 px de bougé), `h_ui_88_shift_press_extends`, `h_ui_88_ctrl_press_toggles_row`, `h_ui_88_editing_click_commits_first` (H-UI-86 gardée), `h_ui_89_id_drag_selects_rows` (U03), `h_ui_90_header_click_selects_column` (U07), `h_ui_90_header_drag_columns`, `h_ui_90_corner_selects_all`, `h_ui_91_autoscroll_down` (U04 : 200 tâches générées, 30 images au bord ⇒ défilement > 0, bloc étendu), `h_ui_91_autoscroll_bar_link_drag`, `h_ui_92_shift_right_extends` (U02), `h_ui_92_cmd_a_selects_all` (U06 : 49/49 sur l'exemple embarqué), `h_ui_92_shift_space_rows_no_typing`, `h_ui_92_ctrl_space_columns`, `h_ui_92_shift_backspace_collapses`, `h_ui_92_web_intercepts`, `h_ui_93_enter_tab_cycle_in_block`, `h_ui_101_gantt_click_keyboard_to_table` (U21 : ↓ après clic dans le Gantt ⇒ ligne suivante ; frappe ⇒ saisie visible), `h_ui_101_header_click_keeps_keys`, `h_ui_102_bar_drag_keeps_multi_selection`, `u24_non_regression` (Maj+clic, Ctrl+clic, Maj+↓, clic droit dans une multi-sélection, Inser), `select_drag_5000` (perf).
- **Critère de fin** : tests verts ; captures inchangées (bloc dessiné comme les lignes sélectionnées d'aujourd'hui) ; tests J7, J8 et ruban verts.
- **Dépendances** : L0. **Parallèle** : L2 (fichiers disjoints). **Taille** : ~1 400 lignes.

---

## L2 — Édition sur sélection (cellules, Ctrl+D, effacer, presse-papiers)

- **Objectif** : copier, couper, coller, remplir et effacer des cellules ; Suppr selon la cible ; saisie vide acceptée.
- **Fichiers** : § 1, L2.
- **Comportement** :
  - `cells.rs` : R-257, R-258, R-259 et `fill_with` (Ctrl+Entrée) selon le § 2.3, sur `block::resolve` ; chaque cellule par `edit::commit_edit` (aucune lecture dupliquée), opérations concaténées dans un lot ; `App::cells_op` : `FillDown`, `Clear`, `FillTyped` (refus ⇒ saisie gardée et message sous la cellule, H-UI-86), `DeleteKey` ⇒ `delete_target` : `Tasks` ⇒ `Structure(Delete)` (confirmation des récapitulatives inchangée) puis message « n tâche(s) supprimée(s) — Ctrl/Cmd+Z annule », `Contents` ⇒ `Clear` ; compte rendu au bandeau quand des cellules sont sautées.
  - `clipboard.rs` : routage de H-UI-95 (`ClipKind`) ; `take_clipboard_events` accepte Copier, Couper et Coller quand le clavier est à la table au sens de H-UI-101 (plus de cas « Gantt ») ; menu Édition (Couper, Copier, Coller) et ruban suivent le même routage ; couper un bloc = copier puis `clear_plan` (deux opérations, une entrée d'annulation pour l'effacement) ; R-244 et R-256 inchangés pour les lignes entières.
  - `edit.rs` : validation d'un texte vide ⇒ `clear_plan` de la cellule (R-259) au lieu de `Text(Empty)` (U14) ; prédicat unique `writable(row, column)` (colonne modifiable et champ modifiable pour ce type de ligne), utilisé par la saisie et par `cells.rs`.
- **Règles** : R-257, R-258, R-259, H-UI-95 à H-UI-98, P59.
- **Tests exigés** (`tests/ui/cells.rs`, unitaires de `cells.rs`) : `r257_single_value_fills_block`, `r257_grid_from_corner`, `r257_parse_error_atomic`, `r257_skips_computed_counts`, `r257_past_end_adds_tasks`, `r257_past_end_refused_under_filter`, `r258_fill_down_block`, `r258_single_row_copies_above`, `r258_summary_target_skipped`, `r259_clear_predecessors`, `r259_clear_start_removes_constraint`, `r259_duration_not_clearable`, `p59_cell_plans_props` (proptest, 256 cas), `h_ui_95_paste_value_into_cell` (U11 : (3, Durée) « 7j » ⇒ 7 j, nombre de tâches inchangé), `h_ui_95_copy_cell_text` (U12 : « 10j »), `h_ui_95_rows_clip_still_inserts` (H-UI-39 gardée), `h_ui_95_header_text_imports_rows`, `h_ui_95_cut_block_one_undo`, `h_ui_96_ctrl_d_one_undo` (U10), `h_ui_97_delete_on_duration_clears` (U13), `h_ui_97_delete_on_name_deletes_with_message`, `h_ui_97_backspace_enter_clears` (U14), `h_ui_98_ctrl_enter_fills_block` (U15), `h_ui_101_copy_after_gantt_click`, tests `h_ui_39_*` et `r256_*` existants verts (adaptés seulement si un geste passe désormais par un bloc : liste dans le commit), `fill_down_1000`, `paste_cells_1000`, `select_all_copy_5000` (perf).
- **Critère de fin** : tests verts ; captures inchangées ; chaque geste d'édition = une entrée d'annulation (vérifié par `history` dans chaque test).
- **Dépendances** : L0. **Parallèle** : L1. **Taille** : ~1 300 lignes.

---

## L3 — Lignes et colonnes (déplacer, ajuster, menu d'en-tête, curseurs)

- **Objectif** : glisser-déplacer de lignes par l'ID, ajustement de colonne, menu d'en-tête complet, curseurs.
- **Fichiers** : § 1, L3.
- **Comportement** :
  - `view/table.rs` : appui sur l'ID d'une ligne sélectionnée en lignes entières (H-UI-94) ⇒ `view.row_drag` au-delà du seuil (avant : sélection de L1) ; trait d'insertion de 2 px (`Palette::cursor`) entre deux lignes, recalculé à chaque image ; refus calculé pendant le glisser (curseur interdit, raison en info-bulle) ; lâcher ⇒ `Action::MoveRows { before }` ; Échap ⇒ annulé ; défilement automatique par L1 (`row_drag` déjà lu) ; poignée de ±4 px, double-clic ⇒ `Action::FitColumn` ; menu d'en-tête de H-UI-99 ; curseurs : `Move` sur l'ID d'une ligne sélectionnée en lignes entières, `ResizeColumn` sur les poignées, `NotAllowed` pendant un déplacement refusé.
  - `structure.rs` : `App::move_rows(before)` et `plan_move_to(project, selection, before) -> Result<Batch, StructureRefusal>` : tâches sélectionnées sans ancêtre sélectionné, sœurs voisines, `task.move` chacune vers `Placement::Before(before)` (ou `After` de la dernière tâche de premier niveau), un lot ; refus `InsideOwnBlock`, `NotAdjacent`, `ViewReorderLocked` (H-UI-32).
  - `column.rs` : `fit_width(texts: impl Iterator<Item = f32>, title: f32, pad: f32, indent: f32) -> Option<u16>` (pur, finitude, bornes) ; `sort_key(Column) -> Option<SortKey>` ; `App::fit_column` mesure les galleys des lignes affichées (texte du `RowModel`, retrait du Nom) et pousse la largeur par le chemin de `SetColumnWidth`.
- **Règles** : H-UI-94, H-UI-99, H-UI-103 (curseurs), H-UI-32 amendée.
- **Tests exigés** (`tests/ui/rows_columns.rs`) : `h_ui_94_drag_rows_moves_one_undo` (exemple de H-UI-94 ; UID gardés ; Ctrl+Z ⇒ P), `h_ui_94_insert_indicator_position`, `h_ui_94_drop_inside_own_block_refused`, `h_ui_94_non_adjacent_refused`, `h_ui_94_locked_under_sort` (curseur interdit, rien écrit), `h_ui_94_escape_cancels`, `h_ui_94_press_unselected_id_selects` (pas de déplacement), `h_ui_99_double_click_fits` (U17 : largeur ≠ 80, = mesure attendue ± 1), `h_ui_99_grip_width`, `h_ui_99_header_menu_sort_and_fit` (U20 : Trier croissant sur Durée ⇒ `SortSpec { Duration, descending: false }`), `h_ui_99_unsortable_greyed`, `h_ui_103_id_move_cursor`, `column_fit_width_bounds` (unitaire), `row_drag_5000`, `best_fit_5000` (perf).
- **Critère de fin** : tests verts ; captures inchangées.
- **Dépendances** : L1 (table.rs, sélection de lignes). **Parallèle** : L4 (fichiers disjoints). **Taille** : ~1 000 lignes.

---

## L4 — Menus contextuels et jumeaux (table, Gantt, échelle, menu Édition)

- **Objectif** : des menus contextuels cohérents avec le ruban et le menu Édition.
- **Fichiers** : § 1, L4.
- **Comportement** :
  - `ribbon/spec.rs` : variantes `RibbonCmd::{FillDown, ClearContents, SelectAll}` (action, prédicat, libellé, raccourci) ; la table `RIBBON` n'est **pas** touchée (L5) ; liste `CONTEXT_MENU: &[RibbonCmd]` dans l'ordre de H-UI-100 (séparateurs compris).
  - `view/main_view.rs` : `context_menu` reconstruit sur `CONTEXT_MENU` (`RibbonCmd::action`, `RibbonCmd::enabled`, raison grisée, raccourci de `KEYMAP`, cases de H-UI-74) ; entrée « Mettre en forme les barres » gardée sur une barre ; clic droit hors de la sélection ⇒ `SelectOp::Press { mode: Replace }` sur la cellule cliquée (table) ou la ligne (Gantt), dans la sélection ⇒ gardée ; `CONTEXT_ENTRIES` retiré si plus lu (le menu Édition le lit : il passe aussi par `RibbonCmd`) ; menu de l'en-tête de l'échelle (clic droit sur `layout.gantt_header`) : Échelle de temps ▾, Zoom avant, Zoom arrière, Projet entier, mêmes `RibbonCmd` que le groupe Zoom.
  - `view/chrome.rs` : menu Édition : Remplir vers le bas, Effacer le contenu, Tout sélectionner (jumeaux des `RibbonCmd` nouveaux).
- **Règles** : H-UI-100, H-UI-70 (parité gardée).
- **Tests exigés** (`tests/ui/context_menus.rs`) : `h_ui_100_table_menu_entries` (U19 : libellés dans l'ordre, FR et EN), `h_ui_100_gantt_menu_same_entries`, `h_ui_100_right_click_outside_selects_cell` (colonne posée), `h_ui_100_right_click_inside_keeps_block`, `h_ui_100_entries_match_ribbon_actions` (chaque entrée pousse l'action de son `RibbonCmd`), `h_ui_100_copy_from_menu_copies_block`, `h_ui_100_timescale_menu_zoom`, `h_ui_100_edit_menu_twins` (Remplir, Effacer, Tout sélectionner : action et raccourci affiché), `h_ui_70_every_item_has_menu_twin` vert.
- **Critère de fin** : tests verts ; captures inchangées (menus fermés dans les captures).
- **Dépendances** : L1 (main_view.rs), L2 (actions servies). **Parallèle** : L3. **Taille** : ~800 lignes.

---

## L5 — Retours visuels et captures (seul)

- **Objectif** : la sélection bien visible, le survol, les nouveaux éléments du ruban, **une seule** régénération des captures.
- **Fichiers** : § 1, L5.
- **Comportement** : `theme.rs` : valeurs de `selection`, `selection_border`, `header_selected`, `hover_row` en clair et en sombre (seuils de H-UI-103 tenus aussi par les bandes de phase et le zébrage de chaque modèle, H-UI-53) ; `view/table.rs` : dessin du bloc (teinte des cellules hors cellule active, bordure du bloc, ID appuyés), lignes entières teintées, survol de ligne (hors glisser) ; `view/gantt.rs` : fond des lignes sélectionnées selon le type (lignes entières et lignes d'un bloc : même teinte que l'ID appuyé, discrète) ; `ribbon/spec.rs` : groupe Édition de Tâche = Rechercher, Défiler jusqu'à la tâche, Remplir ▾ (Vers le bas), Effacer ▾ (Contenu), icônes Lucide déjà vendorisées (`arrow-down-to-line`, `eraser` ajoutées par `cargo xtask icons` si absentes : SVG copiés de `lucide-static@1.55.0`, comme ADR-0017 D6) ; H-UI-75 tenue de 800 à 3 840 px ; exemple `snapshot` : option `--block <id1>:<col1>-<id2>:<col2>` ; `docs/user/*` (« Sélectionner, copier, remplir » pour l'utilisateur d'Excel et de Project) ; `ui.md` § 1.34 complété.
- **Captures** (lavapipe, une fois) : toutes les références régénérées ; nouvelles : `select_block_fr_light`, `select_block_fr_dark`, `select_rows_fr_light`, `row_drag_indicator_fr_light`, `ribbon_task_editing_fr_light` ; avant/après dans `/mnt/backup/tmp/planrs-ux-avant-apres/` sur serveurdvp, pour le propriétaire.
- **Tests exigés** : `h_ui_103_selection_contrast` (chaque thème et modèle : teinte ≥ 1,5:1 sur le fond, ≥ 1,3:1 sur zébrage et bandes, bordure ≥ 3:1, texte ≥ 4,5:1 ; U09), `h_ui_103_active_cell_unfilled`, `h_ui_103_hover_not_during_drag`, `h_ui_75_no_overflow_widths` vert avec le groupe Édition élargi, `h_ui_70_ribbon_matches_spec` mis à jour, suite `ui` complète captures comparées (`PLANRS_UI_SNAPSHOTS=1`).
- **Critère de fin** : tests et captures verts en CI (job `ui`) ; budgets de fluidité tenus ; avant/après prêt.
- **Dépendances** : L3, L4. **Parallèle** : aucun. **Taille** : ~700 lignes (références d'images non comptées).

---

## L6 — Vérification finale

- **Objectif** : preuve que la passe UX tient les règles du dépôt et les gestes de l'audit avant `v1.0.0`.
- **Fichiers** : § 1, L6.
- **Vérifications** :
  1. `cargo xtask rules` vert ; H-UI-87 à H-UI-103, R-257 à R-259, P59 et P60 ont chacune au moins un test à leur nom ; S61 (relecture) et S85 à S89 citées.
  2. Scénario kittest de bout en bout `e2e_ux` sur « P » puis sur l'exemple embarqué, **à la souris** sauf les raccourcis cités, reprenant les gestes de l'audit dans l'ordre du § 0 : U01 (glisser (2, Durée) → (5, Début)), U05 (clic tremblé), U02 (Maj+→), U03 (glisser les ID 2 → 6), U04 (glisser au bord, défilement), U06 (Cmd/Ctrl+A), U07 (clic sur « Durée »), U08 (Ctrl+clic sur la seule ligne), U12 puis U11 (copier (3, Durée), coller sur (6, Durée) ; coller « 7j »), U10 (Ctrl+D), U13 et U14 (Suppr sur Durée, Retour arrière + Entrée sur Prédécesseurs), U15 (Ctrl+Entrée), U16 (glisser l'ID 6-7 au-dessus de 2), U17 (double-clic de bordure), U20 (Trier croissant depuis l'en-tête, puis Maj+F3), U19 (clic droit hors sélection ⇒ Copier), U21 (clic dans le Gantt puis ↓ et frappe), GANTT-02 (glisser une barre d'une multi-sélection), U24 (non-régression) ; puis annuler tout ⇒ projet égal à l'ouverture, rétablir tout, enregistrer et relire ⇒ mêmes tâches ; chaque geste d'édition = exactement une entrée d'annulation.
  3. Parité : `h_ui_70_every_item_has_menu_twin` et `h_ui_100_entries_match_ribbon_actions` verts ; chaque raccourci nouveau atteignable aussi par un menu.
  4. Fluidité (§ 2.5) en release, Sobre et Classique, rapport JSON en artefact du job `perf`.
  5. Web : `.wasm` gzip -9 ≤ 4 Mio et hausse ≤ 60 Kio (chiffre consigné dans ADR-0018) ; démarrage sous Chromium sans tête (C-REL-06) ; Ctrl+A, Ctrl+D, Ctrl+Espace, Maj+Espace retenus au navigateur (`web_intercepts`) ; Ctrl/Cmd+C, X, V toujours laissés au navigateur (événements du presse-papiers).
  6. Corpus et rejeux inchangés ; nombre de tests ≥ celui du départ de `ux-dev` + tests du cahier ; `cargo deny check` vert.
  7. Docs : `ui.md` (§ 1.11, § 1.34, limites), `docs/user/*`, `scorecard.md` (ligne Interface : implémenté, testé ; rien de « vérifié contre MS Project »), `CHANGELOG.md` (section non publiée), ADR-0018 complété des écarts par lot.
- **Hors lot (session principale et propriétaire)** : avant/après montré au propriétaire ; fusion `ux-dev` → `main` après une CI **existante et verte sur le SHA de tête** ; essai par le propriétaire sur son Mac (glisser au pavé tactile, Cmd+A, Cmd+D, Suppr, déplacement de lignes) avant `v1.0.0`.
- **Critère de fin** : tous les checks de `CLAUDE.md` verts en local et en CI ; aucune convention du cahier sans test.
- **Dépendances** : tous les lots. **Taille** : ~600 lignes.

## 3. Points laissés ouverts

1. **Suppr** (D5) : partage ID/Nom ⇒ supprimer, autres colonnes ⇒ effacer ; hypothèse à faire valider par le propriétaire (alternative : toujours effacer comme Excel, la suppression passant par le menu, Ctrl+- ou Cmd+Retour arrière).
2. **Collage d'une colonne de texte** (D3) : écrase les cellules dessous au lieu de créer des tâches, sauf sur la ligne vide ou avec des lignes entières sélectionnées ; à confirmer à l'essai.
3. **Ctrl+Espace sur macOS** : réservé par défaut au changement de source de saisie ; le clic sur l'en-tête reste le chemin garanti.
4. **Gantt** : glisser dans le vide garde le défilement (D9) ; sélection au glisser vertical et tracé de barres : après v1.0.0 si le propriétaire le demande.
5. **Reportés après v1.0.0** (ADR-0018) : réordonner les colonnes au glisser d'en-tête, plusieurs blocs disjoints, poignée de recopie et Ctrl+R, indicateur « supprimer la tâche / effacer seulement », Collage spécial de cellules, gestes de bloc dans la feuille des ressources.
