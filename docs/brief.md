# Cahier des charges — Clone open source de MS Project en Rust, pilotable par IA

> Brief d'origine du propriétaire (2026-10-17), conservé tel quel. Les règles de la section 2 sont reprises dans `CLAUDE.md` ; les décisions qui le précisent ou s'en écartent sont dans `docs/decisions.md`.

## 1. Mission

Tu construis un logiciel de planification de projet open source, équivalent fonctionnel de Microsoft Project Desktop pour l'essentiel de l'usage quotidien. Le produit a trois propriétés qui le distinguent :

1. **Un moteur de planification déterministe en Rust**, distribué comme un seul binaire sans dépendance (pas de JVM, pas de runtime externe).
2. **Une IA peut le piloter nativement** via un serveur MCP : un agent modifie le planning par des opérations métier, et le moteur recalcule tout instantanément.
3. **Une application avec un diagramme de Gantt éditable**, pour les humains, en natif (macOS, Windows, Linux) et dans le navigateur à partir du même code Rust.

Nom de travail du projet : `planrs` (provisoire, à remplacer partout via une constante unique).

## 2. Règles non négociables

1. **Clean-room strict.** Tu implémentes à partir des spécifications publiques (schéma MSPDI XML de Microsoft, documentation d'aide de MS Project) et du comportement observé. Tu ne lis, ne traduis et ne t'inspires jamais du code source de MPXJ (LGPL), ProjectLibre (CPAL), GanttProject (GPL) ou de tout autre logiciel sous licence incompatible. En cas de doute sur une règle de calcul, tu l'écris comme hypothèse (voir règle 4), tu ne vas pas chercher la réponse dans ces codes.
2. **Licence** : `MIT OR Apache-2.0` pour tout le code. Chaque dépendance doit avoir une licence compatible ; vérifie-la avec `cargo deny`.
3. **Le LLM ne calcule jamais une date.** Toute date, durée ou marge provient du moteur. Le serveur MCP expose des opérations ; il ne renvoie jamais de résultat estimé.
4. **Chaque règle de calcul est tracée.** Toute règle reproduisant un comportement de MS Project est documentée dans `docs/scheduling-rules.md` avec un identifiant (`R-001`, `R-002`…), sa formulation, sa source (page de documentation Microsoft, ou « hypothèse ») et son statut (`hypothèse` / `vérifié contre MS Project`). Chaque règle a au moins un test qui porte son identifiant.
5. **Déterminisme.** Mêmes entrées, mêmes sorties, sur toutes les plateformes. Pas de flottants pour les durées : tout est stocké en minutes entières (`i64`). Pas d'itération sur des `HashMap` dans le calcul (utiliser `BTreeMap` ou des index stables).
6. **Le cœur ne dépend d'aucune interface.** La crate du moteur ne connaît ni MCP, ni Tauri, ni le système de fichiers.
7. **Pas de fonctionnalité sans test.** Un commit qui ajoute une règle de calcul ajoute ses tests dans le même commit.
8. **Rust uniquement.** Aucun JavaScript ni TypeScript. Pas de Tauri, d'Electron ni de webview : l'application desktop est en egui/eframe, et la version web est le même code Rust compilé en WebAssembly (trunk + wasm-bindgen). N'ajoute jamais de framework d'interface web comme dépendance, étape de build ou cible de packaging.
9. **Couches strictes.** Aucune crate en dessous de `planrs-ui-egui` ne peut dépendre d'egui, eframe, winit ou rfd.
10. **Ne jamais planter.** Cette règle passe avant les nouvelles fonctionnalités : un utilisateur confie son planning au logiciel, et un plantage lui fait perdre son travail. Pas de `unwrap()` ni de `panic!` hors des tests ; toute erreur remonte proprement et s'affiche.

## 3. Architecture

Workspace Cargo :

```
planrs/
├── crates/
│   ├── planrs-core/      # Modèle de données + moteur de planification (aucune I/O)
│   ├── planrs-mspdi/     # Import/export MSPDI XML (quick-xml + serde)
│   ├── planrs-render/    # Rendu Gantt en SVG (pour l'IA, les tests et l'export)
│   ├── planrs-automation/ # Session « headless » : projet ouvert + historique + exécution de commandes, sans interface
│   ├── planrs-mcp/       # Serveur MCP (SDK officiel `rmcp`), transport stdio et HTTP, au-dessus de planrs-automation
│   └── planrs-ui-egui/   # Interface egui : table, Gantt, vues, état d'interface sérialisable, canal de contrôle
├── apps/
│   ├── planrs/           # Application desktop (eframe, rendu wgpu)
│   ├── planrs-web/       # Même interface compilée en WASM (trunk)
│   └── planrs-cli/       # Binaire en ligne de commande (clap)
├── tests/
│   ├── rules/            # Un test par règle R-xxx
│   ├── fixtures/         # Plannings MSPDI de test
│   └── oracle/           # Vide pour l'instant : accueillera les résultats MS Project
└── docs/
    ├── scheduling-rules.md
    ├── decisions.md      # Journal des décisions d'architecture (ADR courts)
    └── scorecard.md      # Parité fonctionnelle mesurée, mise à jour à chaque jalon
```

Choix techniques :
- **Dates** : `chrono`, avec des fuseaux ignorés dans le moteur (tout en heure locale du projet, comme MS Project).
- **Undo/redo** : chaque modification est une `Command` sérialisable appliquée au modèle ; l'historique est une pile de commandes. Le même mécanisme sert à l'aperçu des modifications proposées par l'IA (application sur une copie, calcul du diff, puis validation ou abandon).
- **Recalcul** : complet à chaque modification pour la V1. L'objectif de performance est inférieur à 50 ms pour 5 000 tâches et 10 000 liens. Le recalcul incrémental n'est envisagé que si ce budget n'est pas tenu.
- **Interface** : egui/eframe sur wgpu. La table et le Gantt sont dessinés à la main et virtualisés (seules les lignes visibles sont dessinées) pour rester fluides à 5 000 lignes.
- **Un seul point d'entrée pour toutes les actions.** Chaque action (menu, raccourci, CLI, MCP, canal de contrôle) passe par le même registre de commandes de `planrs-automation`, identifiées par un id stable (`task.add`, `link.add`…). Une commande ajoutée au registre est disponible partout.
- **Interface fine et pilotée par les données.** L'état de l'interface (sélection, zoom, vue active, colonnes, filtres) vit dans une structure sérialisable (serde). Un canal de contrôle authentifié (JSON en stdio ou en TCP local, activé par `--control`) permet de lire cet état, d'exécuter des commandes et de prendre une capture d'écran : un agent peut ainsi piloter l'application ouverte.

## 4. Modèle de données (V1)

- **Projet** : nom, date de début (ou de fin, en planification à rebours), date d'état, calendrier par défaut, options (heures par jour, heures par semaine, jours par mois, type de tâche par défaut, pilotage par l'effort par défaut).
- **Calendrier** : semaine type avec plages horaires par jour, exceptions datées (jours fériés, journées modifiées), calendrier de base éventuel dont il hérite.
- **Tâche** : identifiant unique stable, numéro d'ordre, nom, niveau hiérarchique (WBS), durée, travail, type (unités fixes / durée fixe / travail fixe), pilotage par l'effort (booléen), contrainte (type + date), échéance, calendrier propre éventuel, jalon (booléen), pourcentage d'avancement, début et fin réels, notes.
- **Lien** : prédécesseur, successeur, type (FS, SS, FF, SF), décalage (en minutes ouvrées, en minutes écoulées ou en pourcentage de la durée du prédécesseur).
- **Ressource** : nom, type (travail / matériel / coût), capacité maximale, calendrier propre, coût horaire standard.
- **Affectation** : tâche, ressource, unités, travail.
- **Ligne de base** : une ligne de base (copie des dates, durées et travail au moment de l'enregistrement).

Valeurs calculées : début et fin au plus tôt, début et fin au plus tard, marge totale, marge libre, critique (booléen), dates et durées des tâches récapitulatives, surutilisation des ressources.

## 5. Moteur de planification (V1)

Implémente dans cet ordre. Chaque point devient une ou plusieurs règles `R-xxx`.

1. **Calendriers ouvrés** : convertir une durée ouvrée en date de fin et inversement. Valeurs par défaut supposées (statut `hypothèse`) : 8 h par jour, 40 h par semaine, 20 jours par mois ; calendrier standard du lundi au vendredi, 8 h–12 h et 13 h–17 h. Attention : une durée saisie en « jours » se convertit avec le paramètre heures par jour, indépendamment des plages réelles du calendrier.
2. **Chemin critique** : passage avant, passage arrière, marges totale et libre, tâches critiques (marge totale ≤ 0).
3. **Liens et décalages** : les quatre types, décalages positifs et négatifs, en temps ouvré, en temps écoulé et en pourcentage.
4. **Tâches récapitulatives** : agrégation des dates, durées et avancement des sous-tâches ; les liens posés sur une récapitulative s'appliquent à ses sous-tâches.
5. **Contraintes** : le plus tôt possible, le plus tard possible, doit commencer le, doit finir le, ne pas commencer avant / après le, ne pas finir avant / après le. Gérer les conflits entre contraintes et liens (avec signalement explicite, sans plantage).
6. **Ressources et affectations** : équation travail = durée × unités ; comportement de chacun des trois types de tâche quand on modifie la durée, le travail ou les unités ; pilotage par l'effort quand on ajoute ou retire une ressource ; calendrier de la ressource pris en compte pour les tâches affectées.
7. **Surutilisation** : détecter et signaler les ressources surutilisées par période. Le nivellement automatique est hors périmètre de la V1.
8. **Avancement** : saisie du pourcentage, du début et de la fin réels ; replanification du travail restant après la date d'état.
9. **Ligne de base** : enregistrement et écarts (début, fin, durée, travail).
10. **Planification à rebours** depuis une date de fin de projet.

Pour chaque règle où tu n'es pas certain du comportement exact de MS Project, choisis le comportement le plus probable, marque-le `hypothèse` dans `scheduling-rules.md` et continue. Ne reste jamais bloqué sur une incertitude.

## 6. Serveur MCP

Principes :
- Des **opérations métier atomiques**, pas un accès brut au modèle.
- Chaque opération d'écriture accepte un paramètre `dry_run`. En mode `dry_run`, elle renvoie le diff (tâches dont les dates changent, avant et après, et effet sur la fin du projet et le chemin critique) sans rien appliquer.
- Chaque réponse est concise et structurée (JSON), avec les identifiants de tâche stables.
- Toute écriture est annulable via `undo`.

Outils V1 :
- Lecture : `open_project`, `project_summary`, `list_tasks` (avec filtres : critiques, en retard, par ressource, par plage de dates), `get_task`, `critical_path`, `resource_usage`, `baseline_variance`, `render_gantt` (renvoie un SVG, avec options de plage et de filtre).
- Écriture : `add_task`, `update_task`, `delete_task`, `move_task` (hiérarchie et ordre), `add_link`, `remove_link`, `set_constraint`, `assign_resource`, `update_assignment`, `add_resource`, `update_progress`, `set_baseline`, `edit_calendar`.
- Analyse : `what_if` (applique une liste d'opérations sur une copie et renvoie l'impact), `explain_date` (explique pourquoi une tâche commence à cette date : prédécesseur pilote, contrainte, calendrier).
- Historique : `undo`, `redo`, `history`.
- Fichiers : `save_project`, `export_mspdi`.

`explain_date` est prioritaire : c'est ce qui permet à un humain de faire confiance aux modifications de l'IA.

## 7. Ligne de commande

Le binaire `planrs` expose les mêmes opérations que le MCP, pour les scripts et les harnais d'agents :

```
planrs summary plan.xml
planrs tasks plan.xml --critical
planrs explain plan.xml --task 42
planrs apply plan.xml ops.json --dry-run
planrs render plan.xml -o gantt.svg
planrs mcp            # lance le serveur MCP en stdio
planrs mcp --http :8080
```

## 8. Application desktop

- **Vue principale** : table des tâches à gauche (colonnes configurables), Gantt à droite, avec échelle de temps zoomable (jour, semaine, mois, trimestre).
- **Édition dans la table** : saisie des durées au format MS Project (`5j`, `3h`, `2s`, `5ej` pour les jours écoulés), des prédécesseurs (`3FS+2j`, `5SS`), indentation et désindentation.
- **Édition dans le Gantt** : déplacer et redimensionner une barre, tracer un lien par glisser-déposer entre deux barres.
- **Affichage** : chemin critique en rouge, jalons en losange, récapitulatives, ligne de base en barre fine, ligne de la date d'état.
- **Vues secondaires** : feuille des ressources, utilisation des ressources (avec surutilisations en évidence), éditeur de calendrier.
- **Raccourcis** : reprendre ceux de MS Project pour les actions courantes (indenter, lier, annuler).
- **Panneau d'opérations IA** : quand un agent propose une modification via MCP sur le projet ouvert, l'application affiche le diff et demande validation avant de l'appliquer.
- **Thème clair et sombre**, interface en français et en anglais (fichiers de traduction séparés dès le départ).
- **Vérification visuelle par l'agent.** Un exemple `cargo run -p planrs-ui-egui --example snapshot -- <fichier.xml> <vue>` rend l'interface hors écran en PNG, sans ouvrir de fenêtre. Tu l'utilises pour vérifier chaque modification d'interface avant de la déclarer terminée.
- **Version web** : mêmes fonctionnalités que le desktop, avec ouverture et enregistrement par téléversement et téléchargement de fichiers. Pas de compte ni de serveur en V1.

## 9. Formats

- **Natif** : MSPDI XML. C'est le format d'enregistrement de la V1, ce qui garantit l'échange avec MS Project.
- **Import et export** : MSPDI XML (aller-retour sans perte sur tous les champs du modèle V1), CSV des tâches.
- **Export** : SVG et PDF du Gantt.
- **Hors périmètre V1** : lecture et écriture des fichiers `.mpp` binaires.

## 10. Stratégie de tests (sans accès à MS Project pour l'instant)

1. **Tests de règles** : au moins un test par règle `R-xxx`, nommé avec son identifiant, avec des valeurs calculées à la main et commentées.
2. **Tests de propriétés** (`proptest`) sur des plannings générés aléatoirement :
   - aucune tâche ne démarre avant la fin de son prédécesseur FS plus le décalage (hors contrainte qui l'impose, signalée) ;
   - les tâches critiques forment au moins une chaîne continue du début à la fin du projet ;
   - marge totale ≥ marge libre pour toute tâche ;
   - travail = durée × unités, à l'arrondi à la minute près ;
   - un aller-retour MSPDI ne modifie aucune valeur ;
   - `undo` après n'importe quelle opération restaure exactement l'état précédent.
3. **Fichiers de référence** (« golden files ») : plannings de test dans `tests/fixtures`, avec les résultats attendus figés ; toute variation fait échouer le test et doit être justifiée.
4. **Préparation de l'oracle** : le dossier `tests/oracle/` et un harnais de comparaison existent dès le départ. Format attendu : un fichier MSPDI d'entrée et le fichier MSPDI recalculé par MS Project. Le harnais compare les dates champ par champ et produit un rapport. Rédige aussi le script VBA (`tools/oracle/generate.bas`) qui, exécuté dans MS Project Desktop, chargera chaque fichier d'entrée, recalculera et exportera le résultat. Il sera utilisé quand une licence sera disponible ; à ce moment, les règles vérifiées passeront du statut `hypothèse` à `vérifié`.
5. **Corpus de fichiers publics** : un fichier enregistré par MS Project contient les dates que MS Project a calculées. Le harnais `tests/corpus/` prend des fichiers MSPDI réels (supports de formation, modèles, exemples publics convertis), efface les valeurs calculées, laisse le moteur recalculer et compare. Les tâches en planification manuelle sont exclues de la comparaison. Les fichiers eux-mêmes ne sont **pas** commités dans le dépôt (droits d'auteur) : le dossier est dans `.gitignore`, et seul un manifeste (source, date, empreinte SHA-256) est versionné. La conversion des `.mpp` en MSPDI peut utiliser MPXJ **comme outil externe uniquement** ; son code n'est jamais lu ni repris.
6. **Plancher de parité** : un fichier `crates/planrs-core/src/parity.rs` définit un `FLOOR`, le pourcentage minimal de cas du corpus et de l'oracle reproduits à l'identique. La CI échoue si la parité mesurée passe sous ce plancher. Quand la parité monte, tu relèves le plancher ; tu ne le baisses jamais.
7. **Captures de référence** de l'interface : quelques vues clés rendues par l'exemple `snapshot` et comparées à des PNG de référence, pour détecter les régressions visuelles.
8. **Performance** : un benchmark (`criterion`) sur un planning de 5 000 tâches, exécuté en CI, avec le budget de 50 ms.

## 11. Jalons

Livre dans cet ordre. Chaque jalon se termine par une mise à jour de `scorecard.md` et un tag Git.

| Jalon | Contenu | Critère de fin |
|---|---|---|
| J1 | Workspace, CI (tests, clippy, fmt, cargo deny), modèle de données, calendriers | Conversions durée ↔ dates testées |
| J2 | Chemin critique, liens, décalages, récapitulatives | Propriétés CPM vertes sur 10 000 plannings aléatoires |
| J3 | Contraintes, planification à rebours, import/export MSPDI | Aller-retour MSPDI sans perte |
| J4 | Session headless et registre de commandes, CLI, rendu SVG, serveur MCP (lecture + écriture + `dry_run` + undo), harnais du corpus public et plancher de parité | Un agent Claude construit un planning de 30 tâches via MCP |
| J5 | Ressources, affectations, types de tâches, pilotage par l'effort, surutilisation | Toutes les règles ressources documentées et testées |
| J6 | Avancement, date d'état, ligne de base, `explain_date`, `what_if` | Scénario complet de suivi testé de bout en bout |
| J7 | Application egui : table, Gantt, édition, vues ressources, calendriers, exemple `snapshot`, canal de contrôle | Planning de 5 000 tâches fluide à l'écran, vérifié par captures |
| J8 | Panneau d'opérations IA, export PDF, traductions, version web WASM, binaires multiplateformes | Builds macOS, Windows, Linux et web publiés en release |

## 12. Hors périmètre de la V1

Fichiers `.mpp` binaires ; nivellement automatique des ressources ; multi-utilisateurs et édition simultanée ; version hébergée en ligne ; sous-projets et projets maîtres ; champs personnalisés et formules ; rapports avancés ; valeur acquise ; spécificités métier (BTP ou autres).

L'architecture ne doit cependant pas bloquer la version hébergée : `planrs-core` doit compiler en WASM dès J1, et `planrs-ui-egui` dès J7 (vérifié en CI).

## 13. Méthode de travail

- Avant chaque jalon, écris un plan court dans `docs/decisions.md` et signale les choix structurants.
- Quand une règle de MS Project est incertaine, consigne-la comme hypothèse et avance.
- Préfère des commits petits et testés à de grosses fonctionnalités non vérifiées.
- Ne déclare jamais une fonctionnalité terminée si ses tests ne passent pas ; `scorecard.md` distingue « implémenté », « testé » et « vérifié contre MS Project ».
