# Journal des modifications

Toutes les modifications notables de planrs sont consignées ici, au format [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) ; les numéros suivent [SemVer](https://semver.org/lang/fr/).

Une version publiée a une section `## [X.Y.Z] - AAAA-MM-JJ` en tête (sous une section « Non publié » vide) : `cargo xtask release-check vX.Y.Z` exige tag = version du workspace = première section (C-REL-01), et `cargo xtask release-notes vX.Y.Z` en tire les notes de la release. Procédure : `docs/features/release.md`. Parité mesurée par domaine : `docs/scorecard.md`.

## [Non publié]

## [1.0.0-rc.1] - 2026-10-26

Jalon J8 (ADR-0016, cahier `docs/plans/j8.md`) : usage quotidien, mise en page et modèles, export PDF, panneau d'opérations IA, traductions, version web, releases multiplateformes. Première pré-version publique prévue ; aucune règle J8 n'est « vérifiée contre MS Project » (`docs/scorecard.md`).

### Ajouté
- **Usage quotidien** : filtres (Toutes, Critiques, En retard, Inachevées, Jalons, Récapitulatives, Utilisant la ressource, Plage de dates, filtre personnalisé), récapitulatives liées affichées, F3 ; tri avec « conserver la structure », Maj+F3 ; repli par triangles, « Afficher jusqu'au niveau n », Alt+Maj+− et + ; recherche Ctrl/Cmd+F (suivant, précédent, correspondances repliées dépliées). Un filtre, un tri ou un repli n'écrit rien dans le document.
- Couper, copier, coller des lignes (UID neufs, liens internes recréés, affectations par nom de ressource, compte rendu des liens non recopiés) ; collage depuis un tableur (en-tête reconnu ou colonne de noms) ; Fichier → Importer CSV… avec aperçu (séparateur, encodage, langue).
- Menu Tâche et barre d'outils : marquer 0, 25, 50, 75, 100 % (une récapitulative vaut ses feuilles), bascule manuel ↔ automatique (dates figées, saisie des dates d'une tâche manuelle), bascule actif ↔ inactif ; colonne « Mode » ; surbrillance du chemin critique en option. Menu Projet dans la barre.
- **Mise en page** : six modèles (Classique, Sobre — défaut —, Contraste élevé, Présentation, Planning travaux, Impression N&B) en clair, sombre et impression, contrastes WCAG 2.2 vérifiés ; galerie avec aperçu réel, une annulation par application ; modèles personnels. Mise en forme directe : clic droit sur une barre ⇒ volet du type de barre (couleur, épaisseur, extrémités, textes), aperçu en direct, avertissement de contraste.
- Retours sur trois plannings DCE réels : fond clair et quadrillage vertical seul, durée après le nom en option (« Pose (4 sem.) »), durée des récapitulatives et date des jalons, étiquettes au-dessus des liens, mode « graphique seul », couleurs de phase, cartouche d'impression, mois relatifs M1…Mn.
- Style du document écrit dans le MSPDI (`<?planrs-style?>`) seulement s'il a été choisi ; fichier sans style relu et écrit octet pour octet comme avant.
- **Export PDF** (A4, A3, Letter, Tabloid ; échelle ou N pages en largeur et en hauteur ; en-tête et pied par zones avec jetons ; légende ; aucune ligne coupée ; PDF déterministe), mode impression en un clic (colonnes d'impression propres, aperçu page par page) ; `export_pdf` dans la CLI et le serveur MCP.
- **Panneau d'opérations IA** : volet latéral, table avant/après de chaque écriture proposée par un agent, Accepter / Refuser, historique de session ; onglet « Pourquoi cette date ? » (pilotes cliquables).
- Relais entre `planrs mcp` et l'application sous Windows (TCP en boucle locale seulement).
- MCP et CLI : `list_tasks` (nouveaux critères : inachevée, jalon, active, ressource, nom), `update_task { mode }`, `list_styles`, `get_style`, `apply_style`, `set_bar_style`, `reset_style`, `render_gantt { style }`.
- Traductions FR et EN de toute l'interface (contrôle `cargo xtask text-ids`, pseudo-langue pour les débordements).
- **Version web** (même code compilé en WebAssembly, trunk) : ouverture, enregistrement et exports par le navigateur, fichiers déposés, raccourcis retenus quand le navigateur le permet.
- `planrs --version` et `planrs-desktop --version` impriment `planrs <version>` sans ouvrir de fenêtre.
- Pipeline de release (`.github/workflows/release.yml`) : DMG macOS universel signé ad hoc (CLI dans `cli/planrs`), zip Windows, archive Linux construite pour glibc ≥ 2.36, paquet web ; empreintes `SHA256SUMS` et attestations de provenance ; contrôles avant et après publication ; tag d'essai `vX.Y.Z-test.N` en brouillon.
- `cargo xtask release-check`, `cargo xtask release-notes`, `cargo xtask icon` (icône de l'application, à valider), `cargo xtask fonts`, `cargo xtask text-ids`.
- Guide d'installation `docs/user/install.md`, guide de traduction `docs/user/translating.md`.

### Modifié
- Modèle par défaut des documents sans style : Sobre (Classique reste choisissable et identique à J7). La CLI et le serveur MCP gardent Classique pour un document sans style.
- `desktop-build.yml` réduit aux préversions (tags `ui-*`, lancement manuel) : il construit les mêmes artefacts que la release, sans vérification ni publication.

### Limites connues
- Binaires non notarisés (macOS) et non signés (Windows) ; pas de logo dans le PDF ; noms en écriture CJK affichés en boîtes vides sur le web.

## [0.1.0] - jalons J1 à J7 (version interne, jamais publiée)

Rattrapage des jalons J1 (2026-10-17) à J7 (2026-10-25), tiré de `docs/scorecard.md` ; détail et écarts dans les ADR-0006 à ADR-0015 de `docs/decisions.md`. Aucun binaire n'a été publié sous ce numéro.

### Ajouté
- **J1 — modèle et calendriers** : modèle V1 (projet, tâches, liens, ressources, affectations, 11 lignes de base) sérialisé en JSON versionné ; calendriers ouvrés (semaine type, exceptions, semaines datées, héritage), conversions de durées en minutes entières et point fixe.
- **J2 — moteur et lecture MSPDI** : chemin critique (passes avant et arrière, marges totale et libre, plusieurs chemins critiques), récapitulatives, tâches manuelles et inactives ; lecteur MSPDI borné ; corpus public de 5 fichiers MS Project comme oracle (plancher de parité en CI).
- **J3 — contraintes, planification à rebours, écriture** : les huit types de contraintes, échéances, planification depuis la fin ; écriture MSPDI avec aller-retour fidèle (éléments non modélisés gardés) et validation XSD ; import et export CSV (FR et EN).
- **J4 — automatisation** : session avec lots atomiques, annulation et rétablissement, `dry_run` ; registre de commandes unique servi par la CLI `planrs` et le serveur MCP ; Gantt en SVG ; édition dans l'interface egui.
- **J5 — ressources** : affectations, types de tâches, pilotage par l'effort, surutilisation et utilisation par période.
- **J6 — suivi** : avancement, date d'état, lignes de base et écarts, `explain_date` et `what_if`.
- **J7 — application** : édition complète dans l'interface (navigation, colonnes, gestes sur les barres, vues des ressources, fenêtres des calendriers), canal de contrôle entre `planrs mcp` et l'application ouverte (toute écriture validée par un bandeau), reprise après plantage.

### Mesuré
- Parité avec MS Project sur le corpus public : 156 / 159 cas identiques, plus 86 / 86 cas de ressources (`docs/scorecard.md`).
