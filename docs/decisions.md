# Décisions d'architecture (ADR courts)

Une entrée par décision, datée, avec le contexte, les options écartées et les conséquences. Les décisions qui précisent ou s'écartent du brief (`docs/brief.md`) sont marquées **écart au brief**.

## ADR-0001 — 2026-10-17 — Modèle PhotoCraft : Rust pur, egui, registre de commandes unique, MCP

**Contexte.** Le brief reprend la recette de PhotoCraft (éditeur d'images clean-room en Rust pur, ~65 k étoiles) : egui/eframe sur wgpu, un registre de commandes typées appelé de la même façon par l'interface, la CLI, un canal de contrôle JSON et un serveur MCP, un corpus de fichiers réels comme oracle, un plancher de parité. Le propriétaire veut la même recette pour MS Project.

**Décision.** Rust uniquement, egui/eframe, un seul registre de commandes dans `planrs-automation`. Pas de Tauri, pas de web UI.

**Écarté.** Toolkit web via Tauri (meilleure grille éditable, mais viole le principe « Rust pur » qui fait l'identité du projet) ; iced ou Slint (pas meilleurs pour une grille, licence Slint à surveiller).

**Conséquences.** Les faiblesses connues d'egui s'assument : saisie IME, accessibilité, édition de cellules. Leçon PhotoCraft (214 raccourcis cassés non vus par 1 700 tests unitaires) : l'interface aura des **tests d'interaction** avec `egui_kittest` (simulation clavier/souris + captures avec seuil de tolérance), qui remplace l'exemple `snapshot` maison du brief.

## ADR-0002 — 2026-10-17 — `planrs mcp` se relie à l'application ouverte, sinon travaille seul

**Contexte.** L'application desktop et le serveur MCP peuvent tenir chacun une copie du projet en mémoire ; un agent modifierait la sienne sans que l'utilisateur le voie, et le premier enregistrement écraserait l'autre.

**Décision.** Une seule configuration côté agent : `planrs mcp`. Au démarrage, le serveur MCP cherche une application ouverte (fichier de rendez-vous avec jeton dans le répertoire d'état utilisateur, socket Unix / named pipe). S'il la trouve, il relaie les commandes par le canal de contrôle : l'application est l'unique source de vérité, elle affiche le diff et demande validation. Sinon, il ouvre le projet en headless via `planrs-automation`.

**Écarté.** Serveur MCP embarqué dans l'application (la configuration MCP de l'agent changerait selon que l'app est ouverte) ; verrou fichier avec rechargement (perte de l'undo, rupture d'UX).

**Conséquences.** Le canal de contrôle est une pièce de J4, pas de J7 : son protocole (JSON par lignes, jeton, id de commande) est défini avec le registre. Transport : socket Unix / named pipe avec jeton dans un fichier en 0600 ; TCP local seulement si demandé explicitement, lié à 127.0.0.1.

## ADR-0003 — 2026-10-17 — Cible : utilisateurs occasionnels de MS Project et agents IA

**Contexte.** Le propriétaire vise ceux qui utilisent MS Project de temps en temps et ne veulent pas payer la licence, et les agents IA.

**Décision.** Le produit premier est le moteur + MCP + `explain_date` ; l'interface (J7-J8) démarre seulement après une utilisation réelle de J4-J6 par des agents sur un projet non trivial. **Écart au brief** : l'import CSV des tâches remonte en J3 (un occasionnel part d'un tableau), et le chemin « demandez un export XML depuis MS Project » est documenté pour l'utilisateur dès J3. La lecture `.mpp` reste hors V1 (format binaire non documenté ; le refaire en clean-room est un projet à part).

## ADR-0004 — 2026-10-17 — Pas de licence MS Project : le corpus public est l'oracle

**Contexte.** Le propriétaire ne prévoit pas de licence MS Project. Les fichiers MSPDI publics (modèles, supports de formation) contiennent les dates calculées par MS Project.

**Décision.** Le harnais `tests/corpus/` est monté dès **J2** (écart au brief : J4), car il ne demande que le CPM. Le plancher de parité `FLOOR` est défini sur un **manifeste figé** (liste de fichiers avec SHA-256, versionnée) et en nombre de cas, pas en pourcentage d'un corpus mouvant : ajouter un fichier difficile ne fait pas baisser la parité mesurée, il ouvre une nouvelle ligne. `tests/oracle/` et `tools/oracle/generate.bas` existent comme coquilles, pour le jour où une licence serait disponible.

## ADR-0005 — 2026-10-17 — Précisions au modèle et au moteur (écarts assumés au brief)

- **Point fixe.** Durées et travail en minutes `i64`. Unités d'affectation et pourcentages en 1/10 000 (`i64`, 10 000 = 100 %). Coûts en centièmes d'unité monétaire (`i64`). Toute division est suivie d'une règle d'arrondi documentée comme règle `R-xxx` (MS Project arrondit à la minute, hypothèse à vérifier). Aucun flottant dans `planrs-core` ; les flottants n'apparaissent qu'à l'import/export MSPDI, avec conversion explicite.
- **Horloge injectée.** `planrs-core` ne lit jamais l'heure système ; la date d'état par défaut est passée par l'appelant.
- **Diagnostics.** Le recalcul produit une liste de diagnostics typés (conflit contrainte/lien, lien circulaire, surutilisation, échéance dépassée), exposée par le MCP et affichée par l'interface. Rien n'est silencieux, rien ne plante.
- **Avancement.** Trois mesures distinctes comme MS Project : `% achevé` (durée), `% travail achevé`, `% physique achevé` ; durée réelle / durée restante ; travail réel / travail restant. La règle « travail = durée × unités » porte sur le travail restant une fois l'avancement saisi.
- **Mode de planification.** Les tâches en mode manuel, inactives et récurrentes sont **stockées** pour l'aller-retour MSPDI sans perte, mais pas recalculées en V1.
- **Fractionnement (splits).** Non modélisé avant J6 ; décision prise à J6 : soit segments dans le modèle, soit replanification sans fractionnement, documentée comme écart.
- **Identifiants.** Reprendre ceux de MSPDI : `UID` (stable), `ID` (ordre d'affichage), `GUID`. Pas de quatrième identifiant.
- **Calendriers.** Combinaison calendrier de tâche × calendrier de ressource prévue dans la structure dès J1 (règles en J5) ; calendrier 24 h et durées écoulées dès J1.
- **Undo/redo.** Historique par **instantanés** du modèle (clone de 5 000 tâches = microsecondes) ; la `Command` reste sérialisée pour le journal, le `dry_run` et le MCP. Journal de commandes sur disque (write-ahead) pour récupérer le travail après un plantage (règle 10).
- **Format natif.** MSPDI ne porte ni l'état des vues ni l'historique : fichier compagnon `<nom>.planrs.json` à côté du `.xml`, optionnel, jamais requis pour ouvrir.
- **`render_gantt`.** Renvoie le SVG et, à la demande, un PNG (resvg) en contenu image MCP : un LLM lit mal un SVG de 5 000 lignes.
- **`dry_run` / `what_if`.** Réponse résumée (fin du projet, chemin critique, N tâches les plus décalées, total) avec le détail à la demande.
- **Propriété « chaîne critique continue ».** Fausse en général (contraintes, échéances, marge négative, plusieurs fins). Reformulée : sans contrainte ni échéance, il existe un chemin de marge totale 0 de la première à la dernière tâche.
- **Captures d'interface.** `egui_kittest` avec seuil de tolérance (`kittest.toml`) ; CI Linux avec adaptateur wgpu logiciel (lavapipe).
- **`rmcp`** épinglé en 3.x (API mouvante) ; `svg2pdf` pour l'export PDF.
