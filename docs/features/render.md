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

## 3. Bornes et rapport

- **Lignes** : au plus `max_rows` (400) lignes du filtre ; au-delà, une dernière ligne « … N tâches non affichées » / « … N tasks not shown » (singulier pour une tâche). Les lignes écartées par le filtre (`critical_only`, `uids`, `max_outline`) ne sont pas annoncées : le filtre n'est pas une borne.
- **Octets** : si le document dépasse `max_bytes` (4 Mio), il s'arrête à la dernière ligne entière qui tient, suivie de la ligne de troncature ; la partie fixe est majorée (hauteur et compteur maximaux) avant de choisir les lignes, d'où `bytes ≤ max_bytes`. Si même l'en-tête ne tient pas, un document vide `<svg … width="0" height="0"/>` (62 octets) est rendu, qui peut dépasser une borne inférieure à sa taille.
- **Jours** : au plus `max_days` (7 305).
- **Rapport** : `shown` lignes écrites, `hidden = tâches du projet − shown` (filtre et bornes), `truncated_days`, `truncated_bytes`, `bytes` = taille de la sortie.

## 4. Tests

`crates/planrs-render/tests/golden.rs` : références octet à octet `tests/golden/*.svg` (régénérées par `PLANRS_BLESS=1`) — maison de démonstration de l'interface en français clair et anglais sombre, un lien de chaque type, jalon + tâche exclue + ligne vide + date d'état ; mêmes octets pour mêmes entrées à toutes les échelles ; filtre critique, par UID et par niveau ; exemple chiffré de l'axe (mer 04/11/2026 08:00 à l'échelle Semaine depuis lun 02/11 → x16 = 448 → `308.0000` avec la table). `tests/hostile.rs` : 256 cas de noms et titres hostiles (contrôles, `]]>`, balises, entités, 1 Mio) relus par quick-xml, texte visible conforme ; 5 000 tâches (400 montrées, 4 600 cachées, coupe à l'octet, borne absurde) ; plage de 166 ans coupée à 7 305 jours et bornes extrêmes sans panique. Tests unitaires de l'axe, des étiquettes, des tracés et de l'échappement dans `src/`.

## 5. Dettes

- Projection temps → abscisse, cellules d'en-tête et tracé des liens dupliqués avec `planrs-ui-egui` (`timescale.rs`, `view/gantt.rs`), en entiers ici et en `f32` là-bas ; le déplacement de `TimeAxis` vers `render` est prévu après le critère de fin J4 (ADR-0012 D10).
- Palette dupliquée (H-R9).
- Date civile d'un `Day` lue sur son écriture ISO (`AAAA-MM-JJ`) pour ne pas dépendre de chrono ; un accesseur `(année, mois, jour)` dans le cœur la remplacerait.
- Jours chômés non grisés (comme l'interface).
