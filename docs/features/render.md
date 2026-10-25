# Rendu SVG du Gantt (`planrs-render`)

État au 2026-10-18 (lot L7 de J4, ADR-0012 décision 10). `planrs_render::svg(project, schedule, &GanttRequest) -> (String, RenderReport)` produit un diagramme de Gantt en SVG pour l'IA (MCP), la CLI, les tests et l'export. Infaillible : toute borne atteinte est signalée dans le rapport, jamais par une panique ni une erreur. PNG reporté en J7.

Le crate ne dépend que de `planrs-core` (vérifié par `cargo xtask layers`) : ni egui, ni `planrs-automation`, ni chrono. Les dates des barres viennent du planning du moteur (règle 3) ; le rendu n'en calcule aucune.

## 1. Modules

| Module | Rôle |
|---|---|
| `request.rs` | `GanttRequest` (plage, échelle, filtre, langue, thème, bornes), `RangeSpec`, `Zoom`, `RowFilter`, `Lang`, `Theme`, `RenderLimits`, `RenderReport` (contrat L0). |
| `axis.rs` | Plage affichée (`resolve_range`), échelle (`resolve_scale`), projection entière instant → abscisse en 1/16 px (`Axis`), écriture à quatre décimales (`push_fixed16`), cellules et étiquettes des deux niveaux de l'en-tête (`tier_cells`). |
| `scene.rs` | Lignes retenues par le filtre (nature, critique, abscisses de barre), liens entre lignes retenues, tracé orthogonal d'un lien (`link_path`) et pointe de flèche, en entiers. |
| `svg.rs` | Palette, feuille de style, fragments par ligne, assemblage du document, coupe à `max_bytes`, échappement XML. |

## 2. Hypothèses d'affichage (H-R)

Ce sont des choix d'affichage, pas des règles de calcul : pas de `R-xxx`. Ils reprennent ceux de l'interface (H-UI, `docs/features/ui.md`) quand ils existent.

- **H-R1, géométrie.** Ligne 22 px, en-tête 44 px (deux niveaux de 22 px), colonne ID 40 px, colonne Nom 240 px, Gantt à partir de 280 px. Police générique `sans-serif` 12 px déclarée dans le style, jamais mesurée.
- **H-R2, axe entier.** Seizièmes de pixel par jour : Jour 576 (36 px), Semaine 192 (12 px), Mois 56 (3,5 px), Trimestre 19 (1,1875 px, arrondi de 1,2). Abscisse `x16 = 280 × 16 + round_div((minutes − minuit du premier jour) × px16, 1 440)` en `i64` (`round_div` du cœur, demi loin de zéro). Écriture `x16/16` à quatre décimales fixes (`{}.{:04}`, reste × 625), signe devant ; toutes les coordonnées (ordonnées comprises) passent par cette écriture. Aucun flottant.
- **H-R3, plage.** `Auto` : jour de début du projet − 7, jour de fin + 21, étendue jusqu'à la date d'état effective si elle est à moins de 90 jours et que l'extension tient dans `max_days`, début reculé au premier jour de semaine du projet (même règle que la plage par défaut de l'interface). `Span { from, to }` : jours inclus, bornes remises dans l'ordre. Coupe à `max_days` jours (`truncated_days`), au moins un jour.
- **H-R4, échelle ajustée.** `Auto { width_px }` : Semaine si `jours × 12 ≤ largeur`, Trimestre si `jours × 7 > 3 × largeur` (soit `jours × 3,5 > 1,5 × largeur`), Mois sinon ; largeur nulle : Mois (comme « Ajuster »).
- **H-R5, en-tête.** Deux niveaux comme l'interface : Jour = semaine / jour, Semaine = mois / semaine, Mois = trimestre / mois, Trimestre = année / trimestre. Semaines alignées sur le premier jour de semaine du projet, autres unités civiles. Étiquettes (FR / EN) : « L 2 » / « 2 » ; « 2 nov. » / « Nov 2 », puis « 2 » ; « novembre 2026 », « novembre », « nov. 26 » (« Nov '26 »), « nov. », « N » ; « T4 2026 » / « Q4 2026 », puis « T4 » ; « 2026 ». Une seule variante par niveau : la plus longue dont toutes les formes de 2026 tiennent dans la plus petite cellule de l'unité, largeur estimée `caractères × 125 + 64` en 1/16 px (≈ 0,6 × 13 px par caractère + 4 px). Une cellule rognée au bord de la plage n'a pas d'étiquette si celle-ci n'y tient pas. Écart à l'interface : pas de variante « numéro de semaine ».
- **H-R6, lignes.** Ordre des ID, aucun tri. Nature d'une ligne comme l'interface : ligne vide (ID seul, en atténué), récapitulative (nom en gras, barre à crochets), jalon (case cochée ou feuille de durée nulle : losange de demi-diagonale 6 px), exclue du calcul (contour atténué de la barre d'affichage R-062, losange si elle est de largeur ≤ 1 px), feuille (barre de 10 px de haut, coins de 2 px, au moins 1 px de large). Chemin critique en rouge (récapitulative critique en rouge sombre). Retrait du nom : 12 px par niveau au-delà du premier, plafonné à 120 px. Noms coupés à `name_chars` caractères suivis de « … ». Barres débordant de la plage rognées à 64 px au-delà du bord, le reste masqué par une découpe.
- **H-R7, liens.** Tracé orthogonal identique à `link_path` de l'interface (FS direct par le dessus ou replié par l'intercolonne, SS, FF, SF direct ou replié), segment de sortie 6 px, flèche de 4 px. Un lien n'est dessiné que si ses deux extrémités sont des lignes montrées ; il est critique si les deux le sont. Couches : liens sous les barres.
- **H-R8, date d'état.** Ligne verticale en tirets (6 / 4, 1,5 px) à la date d'état effective (date d'état, sinon date courante), seulement si elle est dans la plage.
- **H-R9, couleurs.** Constantes `Rgb` reprises de `Palette::LIGHT` et `Palette::DARK` de l'interface (dette de duplication, ADR-0012 D10), une classe CSS par rôle.
- **H-R10, échappement.** Tous les textes (noms, titre du document) : `& < > " '` en entités prédéfinies, tabulation et fins de ligne en espace, caractères interdits par XML 1.0 (contrôles C0 hors tabulation et fins de ligne, U+FFFE, U+FFFF) en U+FFFD. Aucune section CDATA, aucune référence numérique.
- **H-R11, largeurs de texte (J8, ADR-0016 D9).** Largeurs tirées de la table d'Inter (`metrics_inter.rs`, écrite par `cargo xtask fonts` depuis `assets/fonts/`, fraîcheur vérifiée par `metrics_inter_fresh`) : chaque caractère prend l'avance de sa graisse (Regular ou SemiBold, cadratin 2 048), convertie en 1/16 px par arrondi inférieur `avance × taille16 / 2 048` (`i64`), **caractère par caractère**, sans crénage ni façonnage ; caractère absent ⇒ un cadratin. Exemple : « Murs » en 12 px (taille16 = 192) = 173 + 113 + 72 + 101 = 459 (28,69 px). Remplace l'estimation de H-R1 et de H-R5 dans les lots qui l'adoptent (L4) ; tests `h_r11_*` (`metrics.rs`). Statut : hypothèse (choix de conception).
- **H-R12, style dans le fichier (J8).** Instruction `<?planrs-style {"v":1,"base":"sobre","over":{…}}?>` placée juste après la déclaration XML ; JSON compact, clés triées, `>` écrit `\u003e` ; à la lecture : première instruction de cible `planrs-style` avant l'élément racine, ≤ 64 Kio, sinon ignorée avec diagnostic `style.ignored` ; version inconnue ⇒ ignorée. Contrat posé en L0 (`planrs_render::parse_doc_style`) ; écriture et lecture MSPDI en L6.
- **H-R13, PDF déterministe (J8).** Glyphes placés un par un aux avances de H-R11, date de création et identifiant du document fournis par l'appelant (identifiant = 16 premiers octets d'un hachage stable du contenu des pages), aucune horloge ni aléa ; deux appels ⇒ mêmes octets. Essai L0 : `planrs_pdf::smoke_page` (krilla 0.8.2), test `pdf_smoke_one_page` ; export complet en L7.

## 3. Bornes et rapport

- **Lignes** : au plus `max_rows` (400) lignes du filtre ; au-delà, une dernière ligne « … N tâches non affichées » / « … N tasks not shown » (singulier pour une tâche). Les lignes écartées par le filtre (`critical_only`, `uids`, `max_outline`) ne sont pas annoncées : le filtre n'est pas une borne.
- **Octets** : si le document dépasse `max_bytes` (4 Mio), il s'arrête à la dernière ligne entière qui tient, suivie de la ligne de troncature ; la partie fixe est majorée (hauteur et compteur maximaux) avant de choisir les lignes, d'où `bytes ≤ max_bytes`. Si même l'en-tête ne tient pas, un document vide `<svg … width="0" height="0"/>` (62 octets) est rendu, qui peut dépasser une borne inférieure à sa taille.
- **Jours** : au plus `max_days` (7 305).
- **Rapport** : `shown` lignes écrites, `hidden = tâches du projet − shown` (filtre et bornes), `truncated_days`, `truncated_bytes`, `bytes` = taille de la sortie.

## 4. Tests

`crates/planrs-render/tests/golden.rs` : références octet à octet `tests/golden/*.svg` (régénérées par `PLANRS_BLESS=1`) — maison de démonstration de l'interface en français clair et anglais sombre, un lien de chaque type, jalon + tâche exclue + ligne vide + date d'état ; mêmes octets pour mêmes entrées à toutes les échelles ; filtre critique, par UID et par niveau ; exemple chiffré de l'axe (mer 04/11/2026 08:00 à l'échelle Semaine depuis lun 02/11 → x16 = 448 → `308.0000` avec la table). `tests/hostile.rs` : 256 cas de noms et titres hostiles (contrôles, `]]>`, balises, entités, 1 Mio) relus par quick-xml, texte visible conforme ; 5 000 tâches (400 montrées, 4 600 cachées, coupe à l'octet, borne absurde) ; plage de 166 ans coupée à 7 305 jours et bornes extrêmes sans panique. Tests unitaires de l'axe, des étiquettes, des tracés et de l'échappement dans `src/`.

## 5. Style, géométrie, pagination, PDF (J8, ADR-0016)

Sections posées par le lot L0 (contrat) ; chaque lot y ajoute ses lignes.

### 5.1 Style

`planrs_render::style` (contrat L0) : `PlanStyle` (sérialisé, `deny_unknown_fields`, aucun flottant, bornes vérifiées par `validate`), six modèles intégrés `templates/*.json` (Classique, Sobre, Contraste élevé, Présentation, Chantier, Impression N&B), chacun en variantes `light`, `dark` et `print` ; `template`, `resolve` (modèle + `StylePatch`), `parse_doc_style` (≤ 64 Kio) ; `contrast_ratio` (WCAG 2.2, entiers : luminance en millionièmes par une table sRGB de 256 entrées, rapport en millièmes).

### 5.2 Géométrie

`planrs_render::geometry` (contrat L0, implémentation L4) : axe entier unique, rectangles de barre, tracé des liens et placement des textes en 1/16 px (`i64`), lus par le SVG, le PDF et l'écran (H-UI-42, P55). Mesure des textes : `TextMeasure`, `InterMetrics` (H-R11).

### 5.3 Pagination

`planrs_render::paginate` (contrat L0, implémentation L7) : bandes de pages sans ligne coupée, en-têtes et échelle répétés, plafond `MAX_PAGES` = 1 000.

### 5.4 PDF

Crate `planrs-pdf` (couche du rendu, sans egui ni `planrs-automation`) : krilla 0.8.2 sans fonctions par défaut ; essai L0 (`smoke_page`) ; `pdf(pages, fonts, meta)` en L7 (H-R13).

## 6. Dettes

- Projection temps → abscisse, cellules d'en-tête et tracé des liens dupliqués avec `planrs-ui-egui` (`timescale.rs`, `view/gantt.rs`), en entiers ici et en `f32` là-bas ; le déplacement de `TimeAxis` vers `render` est prévu après le critère de fin J4 (ADR-0012 D10).
- Palette dupliquée (H-R9).
- Date civile d'un `Day` lue sur son écriture ISO (`AAAA-MM-JJ`) pour ne pas dépendre de chrono ; un accesseur `(année, mois, jour)` dans le cœur la remplacerait.
- Jours chômés non grisés (comme l'interface).
