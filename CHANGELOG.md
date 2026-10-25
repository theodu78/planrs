# Journal des modifications

Toutes les modifications notables de planrs sont consignées ici, au format [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) ; les numéros suivent [SemVer](https://semver.org/lang/fr/).

Une version publiée a une section `## [X.Y.Z] - AAAA-MM-JJ` en tête (sous une section « Non publié » vide) : `cargo xtask release-check vX.Y.Z` exige tag = version du workspace = première section (C-REL-01), et `cargo xtask release-notes vX.Y.Z` en tire les notes de la release. Procédure : `docs/features/release.md`. Parité mesurée par domaine : `docs/scorecard.md`.

## [Non publié]

Jalon J8 (ADR-0016, cahier `docs/plans/j8.md`), en cours.

### Ajouté
- `planrs --version` et `planrs-desktop --version` impriment `planrs <version>` sans ouvrir de fenêtre.
- Pipeline de release (`.github/workflows/release.yml`) : DMG macOS universel signé ad hoc (CLI dans `cli/planrs`), zip Windows, archive Linux construite pour glibc ≥ 2.36, paquet web ; empreintes `SHA256SUMS` et attestations de provenance ; contrôles avant et après publication.
- `cargo xtask release-check`, `cargo xtask release-notes`, `cargo xtask icon` (icône de l'application, à valider).
- Guide d'installation `docs/user/install.md`.

### Modifié
- `desktop-build.yml` réduit aux préversions (tags `ui-*`, lancement manuel) : il construit les mêmes artefacts que la release, sans vérification ni publication.

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
