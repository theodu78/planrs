# Cahier des charges « ruban et chronologie » — planrs (avant v1.0.0)

Référence : ADR-0017 (`docs/decisions.md`). Branche d'intégration `ribbon-dev` (depuis `main` au tag `j8`). Toute convention ci-dessous est au statut `hypothèse` sauf ce que sa source dit. Chaque lot : un agent, ≤ ~1 500 lignes (code + tests, références d'images non comptées), un worktree et un `CARGO_TARGET_DIR` propres, un commit `type(scope): …` par sous-étape testée, fiche `H-UI-xx` / `H-R16` et tests **dans le même commit**, messages terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

Checks de chaque lot : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p <crates du lot>`, `cargo xtask panics`, `cargo xtask layers`, `cargo xtask rules`, `cargo xtask text-ids`, `cargo xtask icons --check` (dès L0), `cargo deny check`, `cargo check -p planrs-core --target wasm32-unknown-unknown`, `cargo check -p planrs-ui-egui --target wasm32-unknown-unknown`. Machine à mémoire partagée (serveurdvp) : `ramwait 2.5` puis `memlimit 3 cargo …` (`memlimit 5` pour `cargo test --workspace`), `export PATH=$HOME/.cargo/bin:$PATH`, une seule commande cargo lourde à la fois.

Clean-room (règle 1) : conventions du ruban et de la chronologie tirées des seules pages d'aide Microsoft (`support.microsoft.com`, `learn.microsoft.com`), relues par l'agent qui écrit la fiche (titre exact, phrase citée) ; **jamais d'icône, de capture ni de maquette Microsoft copiée** ; jamais un autre clone (MPXJ, ProjectLibre, GanttProject…) ni sa documentation. Une affirmation sur MS Project sans page relue reste une hypothèse.

Règle 10 : aucune conversion `f32` → entier sans test de finitude et bornage ; la largeur du ruban et de la frise vient d'egui (`f32`) et passe par une fonction de bornage testée avant tout calcul entier ; tout indice de ligne passe par `RowView` ; plafonds écrits (200 tâches sur la frise, 3 rangées, 64 éléments par groupe de ruban, 9 niveaux de plan) et dépassement = message, jamais de panique.

Règle 3 : ni l'interface ni le rendu ne calculent une date ; les dates de la frise (extrémités, jalons, date d'état) et leurs textes viennent du moteur par `planrs-automation`. La seule conversion permise reste date → pixel (`TimeAxis::x`, `timeline` en 1/16 px) ; le rectangle de la partie visible du Gantt est une projection pixel → pixel (rapport des échelles), sans date.

Règle 9 : `planrs-render` reste sans egui ; la géométrie de la frise y vit, egui ne fait que la peindre.

## 1. Graphe des lots

```
L0 contrat (types, icônes, squelettes, textes, fiches) ─┬─ L1 chronologie : géométrie, scène, SVG, PDF (render, automation) ─┬─ L4 chronologie à l'écran et à l'impression (interface)
                                                        ├─ L2 ruban : moteur, contenu, menus jumeaux (interface) ──────────┤
                                                        └─ L3 commandes nouvelles (structure, notes, défilement, ajout à la frise)
                                                                                                                          └─ L5 bascule visible et captures (seul) ── L6 vérification finale
```

Vagues (au plus 2 agents, fichiers disjoints) : W1 {L0} → W2 {L1, L2} → W3 {L3, L4} → W4 {L5} → avant/après montré au propriétaire (session principale) → W5 {L6}.

Fichiers par lot (un fichier hors liste demande l'accord de la session principale) :
- **L0** : tout ce que liste son paragraphe ; seul lot qui touche `action.rs`, `app.rs` (bras du réducteur, commutateur `Chrome`), `state.rs`, `keymap.rs`, `i18n.rs`, `locales/{fr,en}.ftl`, `planrs-render/src/style.rs`.
- **L1** : `planrs-render/src/{timeline.rs, paginate.rs, scene.rs, svg.rs, lib.rs}`, `planrs-render/tests/timeline*.rs`, `crates/planrs-pdf/src/*` (dessin de la frise), `planrs-automation/src/{timeline.rs, pdf.rs, render_text.rs}`, `tests/pdf_captures/main.rs` (cas frise), `docs/features/render.md` (H-R16).
- **L2** : `planrs-ui-egui/src/{ribbon/mod.rs, ribbon/spec.rs, view/ribbon.rs, view/chrome.rs}`, `tests/ui/ribbon.rs`.
- **L3** : `planrs-ui-egui/src/{structure.rs, timeline_ops.rs, nav.rs, dialog/task_info.rs}`, `tests/ui/ribbon_commands.rs`.
- **L4** : `planrs-ui-egui/src/{view/timeline.rs, view/main_view.rs, view/mod.rs, view/scene_paint.rs, dialog/page_setup.rs}`, `tests/ui/timeline.rs`, `tests/perf.rs`.
- **L5** : `planrs-ui-egui/src/{app.rs (défaut du commutateur), view/chrome.rs (retrait de la barre d'outils), i18n.rs, locales/*.ftl (textes retirés)}`, `examples/snapshot.rs`, `tests/snapshots/*`, `tests/ui/{snapshots.rs, main.rs}`, `docs/features/ui.md`, `docs/user/*`.
- **L6** : `tests/ui/ribbon_e2e.rs`, `tests/perf.rs` (corrections), `.github/workflows/*` (corrections seulement), `CHANGELOG.md`, `docs/{scorecard.md, decisions.md (écarts), features/*.md}`.
- Fichiers partagés en ajout seulement : `tests/ui/main.rs` (modules posés en L0), `docs/features/ui.md` (sections posées en L0, remplies par chaque lot).

Chemin critique : L0 → L1 → L4 → L5 → L6. Coupes, dans l'ordre (si le temps manque avant `v1.0.0`) : réduction adaptative à deux paliers (L2 : garder seulement le repli de groupe en bouton à menu), rectangle de la partie visible (L4), surimpression du ruban replié (L2 : un clic sur un onglet déplie simplement), impression de la frise en SVG (L1 : PDF et aperçu gardés). Jamais coupés : parité ruban ↔ menu, Ctrl+F1, icônes, frise à l'écran, frise au PDF, captures, fluidité.

## 2. Index

Repère des exemples (celui de J8) : calendrier Standard « S » (lun-ven 08:00-12:00, 13:00-17:00), début du projet lun 19/10/2026 08:00. Projet « P » : 1 « Gros œuvre » (récapitulative) ⊃ 2 « Fondations » 5j, 3 « Murs » 10j (2→3 FS), 4 « Livraison » jalon (3→4 FS) ; 5 « Second œuvre » ⊃ 6 « Plâtrerie » 4j (3→6 FS), 7 « Peinture » 3j (6→7 FS). Dates du moteur : 2 du 19/10 au 23/10, 3 du 26/10 au 06/11, 4 le ven 06/11 17:00, 6 du 09/11 au 12/11, 7 du 13/11 au mar 17/11 17:00 (fin du projet). Fenêtre 1440×900, densité Normale, 1 px = 16 unités de scène.

### 2.1 Sources (table de `docs/scheduling-rules.md`, relues en L0 : titre exact et phrase citée)

| Réf. | Page | Utilisée pour |
|---|---|---|
| S80 | support.microsoft.com, « Learn the Project 2010 Ribbon » | Cinq onglets Task, Resource, Project, View, Format ; groupes Schedule (pourcentage, lier, inactiver, Indent) et Tasks (Auto Schedule) de Task ; View de Resource (Team Planner) ; Schedule de Project (Set Baseline) ; Split View (case Timeline, Details) et Task Views de View ; Gantt Chart Style de Format (H-UI-69, H-UI-70) |
| S81 | support.microsoft.com, « Create a timeline in Project » | View → Timeline ; « It appears above the current view » ; « The timeline is on by default most of the time » ; clic droit → Add to Timeline (H-UI-78, H-UI-79) |
| S82 | support.microsoft.com, « Print the timeline in Project » | La chronologie s'imprime (H-UI-82 ; la position sur la page est une hypothèse) |
| S83 | support.microsoft.com, « Customize the ribbon in Office » (ou « Show or hide the ribbon in Office », celle qui le dit) | Repli par double-clic sur un onglet, Ctrl+F1 (H-UI-72) |
| S84 | support.microsoft.com, « Add new tasks » et « Add notes to a task or to the project schedule » | Bouton Task du groupe Insert ; Notes et Information du groupe Properties (H-UI-70, H-UI-83, H-UI-84) |
| S61 | (J7) « Keyboard shortcuts for Project » | Raccourcis affichés dans les info-bulles (Ctrl+F2, Ctrl+Maj+F2, Maj+F2, Inser, F3…) |

Libellés français des onglets et groupes : choisis d'après la demande du propriétaire, **non relus** sur l'interface française de Project (hypothèse). Une page introuvable ou qui ne dit pas ce qui est cité ⇒ la convention passe en `hypothèse` pure, consigné dans ADR-0017.

### 2.2 Contenu exact du ruban (H-UI-70)

Grands boutons en gras ; « ▾ » liste déroulante (menu) ; « [ ] » bascule ou case. Colonne « Jumeau » : entrée de la barre de menus qui pousse la même action (créée en L2 si elle manque, marquée « + »). Icône : nom du fichier Lucide 1.55.0.

| Onglet | Groupe | Élément | Action poussée | Icône | Jumeau au menu |
|---|---|---|---|---|---|
| (accès rapide) | — | Enregistrer · Annuler · Rétablir | `Action::{Save, Undo, Redo}` | `save`, `undo-2`, `redo-2` | Fichier, Édition |
| Tâche | Affichage | **Diagramme de Gantt ▾** : Gantt, Gantt de suivi, Feuille des ressources, Utilisation des ressources | `SetView` + `SetGanttStyle` | `chart-gantt` | Affichage |
| Tâche | Presse-papiers | **Coller** · Couper · Copier · Collage spécial… | `Clipboard(Paste, Cut, Copy, PasteSpecial)` | `clipboard-paste`, `scissors`, `copy`, `clipboard-list` | Édition |
| Tâche | Mise en forme | **Mettre en forme les barres** · Modèles… | `Format(OpenBars)`, `ShowDialog(Gallery)` | `paintbrush`, `swatch-book` | Format |
| Tâche | Planification | 0 % · 25 % · 50 % · 75 % · 100 % (rangée de cinq, texte seul) · Lier · Délier · Indenter · Désindenter · Désactiver [ ] | `TaskCmd::MarkPercent`, `Structure(Link, Unlink, Indent, Outdent)`, `TaskCmd::ToggleActive` | —, `link`, `unlink`, `indent-increase`, `indent-decrease`, `circle-off` | Tâche, Édition |
| Tâche | Tâches | **Planifier manuellement [ ]** | `TaskCmd::ToggleMode` | `pin` | Tâche |
| Tâche | Insérer | **Tâche** · Récapitulative · Jalon | `Structure(InsertBefore, InsertSummary, InsertMilestone)` | `list-plus`, `list-tree`, `diamond` | Édition (+ Récapitulative, + Jalon) |
| Tâche | Propriétés | **Informations** · Notes · Ajouter à la chronologie [ ] | `ShowTaskInformation`, `ShowTaskNotes`, `Timeline(ToggleSelection)` | `info`, `notebook-pen`, `timeline` | Tâche (+ Notes, + chronologie) |
| Tâche | Édition | **Rechercher** · Défiler jusqu'à la tâche | `TaskView(OpenSearch)`, `ScrollToTask` | `search`, `locate` | Édition (+ Défiler) |
| Ressource | Affichage | **Feuille des ressources** · **Utilisation des ressources** | `SetView` | `users`, `chart-column` | Affichage |
| Ressource | Affectations | **Affecter les ressources** | `ShowDialog(AssignResources)` | `user-plus` | Édition |
| Projet | Propriétés | **Informations sur le projet** · **Modifier le temps de travail** | `ShowDialog(ProjectInformation, WorkingTime)` | `file-text`, `calendar-cog` | Projet |
| Projet | Planification | **Définir la référence** | `ShowDialog(Baseline)` | `calendar-check` | Projet |
| Projet | État | **Mettre à jour le projet** | `ShowDialog(Reschedule)` | `refresh-cw` | Projet |
| Projet | Impression | **Mode impression** · Mise en page… · Exporter en PDF… | `ShowDialog(PageSetup)`, `Export(Pdf)` | `printer`, `file-cog`, `file-down` | Fichier |
| Affichage | Affichages des tâches | **Diagramme de Gantt** · **Gantt de suivi** | `SetView` + `SetGanttStyle` | `chart-gantt`, `chart-no-axes-gantt` | Affichage |
| Affichage | Affichages des ressources | Feuille des ressources · Utilisation des ressources | `SetView` | `users`, `chart-column` | Affichage |
| Affichage | Données | Trier ▾ (Trier…, Par ID) · Mode plan ▾ (niveaux 1 à 9, Tous, Masquer les sous-tâches, Afficher les sous-tâches) · Filtre ▾ (préréglages, Filtre personnalisé…, Effacer le filtre) | `TaskView(OpenSortDialog, ResetSort, SetOutlineLevel, HideSubtasks, ShowSubtasks, SetFilter, OpenFilterDialog, ClearFilter)` | `arrow-up-down`, `chevrons-down-up`, `filter` | Affichage |
| Affichage | Zoom | Échelle de temps ▾ (Jour, Semaine, Mois, Trimestre) · Zoom avant · Zoom arrière · **Projet entier** | `SetZoom`, `ZoomBy(±1)`, `FitZoom` | `calendar-range`, `zoom-in`, `zoom-out`, `maximize` | Affichage |
| Affichage | Fractionner l'affichage | Chronologie [ ] · Volet Opérations IA [ ] · Diagnostics [ ] | `Timeline(SetVisible)`, `SidePanel(Toggle)`, `ToggleDiagnostics` | (cases) | Affichage (+ Chronologie) |
| Affichage | Interface | Thème ▾ · Langue ▾ · Réduire le ruban [ ] | `SetTheme`, `SetLanguage`, `Ribbon(ToggleCollapsed)` | `sun-moon`, `languages`, `chevrons-up-down` | Affichage (+ Réduire le ruban, Ctrl/Cmd+F1) |
| Format | Colonnes | **Colonnes ▾** (une case par colonne de `Column::ALL`, ID et Nom grisées) | `ToggleColumn` | `columns-3` | Format (+ Colonnes, aujourd'hui seulement au clic droit de l'en-tête) |
| Format | Styles des barres | **Mettre en forme les barres** · Tâches critiques [ ] · Mois relatifs [ ] | `Format(OpenBars)`, `SetHighlightCritical`, `Format(SetRelativeMonths)` | `paintbrush` | Format, Affichage |
| Format | Styles du diagramme | **Modèles de mise en forme…** | `ShowDialog(Gallery)` | `swatch-book` | Format |
| Format | Afficher/Masquer | Graphique seul [ ] | `SetChartOnly` | `panel-left-close` | Affichage |

Textes anglais (mêmes clés) : Task, Resource, Project, View, Format ; View, Clipboard, Formatting, Schedule, Tasks, Insert, Properties, Editing ; Assignments ; Properties, Schedule, Status, Print ; Task Views, Resource Views, Data, Zoom, Split View, Interface ; Columns, Bar Styles, Gantt Chart Style, Show/Hide. Libellés des éléments : ceux des entrées de menu existantes quand elles existent (une seule clé `.ftl`, pas de doublon), sinon clé `ribbon-*`.

### 2.3 Conventions d'interface (table de `ui.md` § 1.11 ; une ligne par convention, testée par kittest `h_ui_NN_*`)

| Id | Convention | Source | Exemple | Lot |
|---|---|---|---|---|
| H-UI-69 | Ruban de cinq onglets **Tâche, Ressource, Projet, Affichage, Format** (EN Task, Resource, Project, View, Format), sous la barre de menus, à la place des deux rangées d'outils ; onglet Tâche au premier lancement, puis le dernier choisi (`UiState.ribbon.tab`) ; ruban présent aussi sur l'accueil (éléments grisés sans document) | S80 ; mémorisation : hypothèse | Premier lancement ⇒ onglet Tâche, groupe Planification visible ; choisir Format, relancer ⇒ Format | L2 |
| H-UI-70 | Contenu exact du § 2.2 ; chaque élément pousse l'action de son jumeau de la barre de menus, avec le même prédicat d'activation (document, sélection, H-UI-32) ; aucune commande n'est atteignable par le ruban seul | S80, S84 (groupes nommés) ; reste : hypothèse | Tâche → Lier sur {2, 3} ⇒ même lot que Édition → Lier ; sans sélection ⇒ grisé dans les deux | L2 |
| H-UI-71 | Gabarit : bande d'onglets 26 px (barre d'accès rapide à gauche), corps 92 px ; grand bouton = icône 32 px + libellé sur au plus 2 lignes dessous, largeur ≥ 52 px ; petit bouton = icône 16 px + libellé à droite, hauteur 22 px, trois par colonne ; liste = libellé + « ▾ » ; nom du groupe centré sous le groupe (texte atténué), séparateur vertical entre groupes | hypothèse | Groupe Insérer : « Tâche » grand, « Récapitulative » et « Jalon » empilés à sa droite | L2 |
| H-UI-72 | Repli : double-clic sur un onglet ou Ctrl+F1 (Cmd+F1 sur macOS) ⇒ seule la bande d'onglets reste (la vue gagne 92 px) ; replié, un clic sur un onglet ouvre le corps en surimpression au-dessus de la vue, refermé par une action du ruban, Échap ou un clic ailleurs ; état gardé (`UiState.ribbon.collapsed`) ; Affichage → Réduire le ruban fait de même | S83 ; surimpression : hypothèse | Ctrl+F1 ⇒ replié ; clic sur « Projet » ⇒ corps en surimpression, clic sur « Définir la référence » ⇒ fenêtre ouverte et surimpression refermée | L0 (touche), L2 |
| H-UI-73 | Info-bulle de chaque élément : libellé appuyé, une phrase d'aide, raccourci de `KEYMAP` du système (« Lier les tâches (Ctrl+F2) », « ⌘F2 » sur macOS) ; élément grisé : la raison (« Sélectionnez au moins une tâche », « Retirez le tri ou le filtre » de H-UI-32) | S61 (raccourcis) ; reste : hypothèse | Survol de « Indenter » ⇒ « Indenter (Alt+Maj+→) » | L2 |
| H-UI-74 | Bascules et cases reflètent l'état : Désactiver enfoncé si la tâche active est inactive, Planifier manuellement si elle est manuelle, Ajouter à la chronologie si elle est sur la frise ; cases d'affichage selon `UiState` et la vue ; sélection mixte : état de la tâche active | hypothèse | 6 manuelle sélectionnée ⇒ « Planifier manuellement » enfoncé | L2 |
| H-UI-75 | Largeur insuffisante : groupes réduits de droite à gauche, dans un ordre fixe (1. petits boutons sans libellé, info-bulle gardée ; 2. groupe replié en un bouton à menu portant son nom et son icône principale) ; jamais de débordement, de chevauchement ni de retour à la ligne de 800 à 3 840 px ; mêmes actions dans le menu du groupe replié | hypothèse (comportement d'Office non relu) | Onglet Tâche à 800 px ⇒ Édition et Propriétés repliés en boutons ; « Propriétés ▾ » → Informations ⇒ fenêtre ouverte | L2 |
| H-UI-76 | Barre d'accès rapide à gauche des onglets : Enregistrer, Annuler, Rétablir, icônes seules (nom accessible et info-bulle traduits), mêmes prédicats que le menu Édition | hypothèse | Aucun lot annulable ⇒ Annuler grisé | L2 |
| H-UI-77 | Icônes Lucide 1.55.0 (ISC, MIT pour les dérivées de Feather), vectorielles, à la couleur du texte du thème (atténuée si grisé), nettes à 1×, 1,5×, 2× et 3× ; aucune icône, image ou capture Microsoft ; tout élément a un libellé (grands et petits boutons) ou au moins une info-bulle et un nom accessible (accès rapide, palier 1 de H-UI-75) | règle 1 ; licences : `assets/icons/lucide/LICENSE` | Thème sombre ⇒ icônes claires ; « Coller » grisé ⇒ icône atténuée | L0, L2 |
| H-UI-78 | Chronologie : frise de 72 px au-dessus de la vue Gantt (table et Gantt, toute la largeur), Affichage → Chronologie (case, **cochée par défaut**) ; « Début » et date courte au bord gauche, « Fin » et date courte au bord droit (dates du moteur) ; graduations des mois (trimestres au-delà de 24 mois, années au-delà de 8 ans) ; date d'état en trait tireté étiqueté « État JJ/MM » ; absente des vues des ressources | S81 (au-dessus de la vue, affichée par défaut) ; contenu : hypothèse | P ⇒ « Début lun 19/10/26 » à gauche, « Fin mar 17/11/26 » à droite, graduations « oct. », « nov. » | L4 |
| H-UI-79 | Ajout et retrait : Tâche → Propriétés « Ajouter à la chronologie » et menu contextuel de la table et du Gantt (« Ajouter à la chronologie » / « Retirer de la chronologie ») ; bascule sur la sélection : si une tâche sélectionnée n'y est pas, toutes y vont, sinon toutes en sortent ; un seul `style.apply { patch: { timeline } }`, une entrée d'annulation ; liste triée par UID, ≤ 200 (au-delà : message, rien écrit) ; UID d'une tâche supprimée ignoré à l'affichage | S81 (clic droit « Add to Timeline ») ; bascule et limite : hypothèse | Sélection {3, 4}, Ajouter ⇒ barre « Murs » et jalon « Livraison » ; Ctrl+Z ⇒ frise vide | L3, L4 |
| H-UI-80 | Disposition : tâche ou récapitulative = barre de son début à sa fin planifiés, nom dedans (« … » si tronqué ; à droite de la barre si elle fait moins de 24 px et qu'il y a la place) ; rangées attribuées au premier ajustement par (début, ID), **3 rangées au plus**, le reste compté « +n » au bord droit ; jalon = losange de 10 px sur l'axe, date courte et nom au-dessus, au-dessous en cas de collision avec le précédent ; ordre de dessin : barres, axe, jalons, textes | hypothèse ; géométrie H-R16 | P avec {2, 3, 4, 6} : 2, 3 et 6 sur la rangée 1 (aucun chevauchement), 4 en losange sur l'axe au 06/11 | L1, L4 |
| H-UI-81 | Rectangle de la partie visible du Gantt dessiné sur la frise (projection pixel → pixel du défilement et de l'échelle, aucune date calculée), suivi image par image ; clic sur une barre ou un jalon ⇒ sélection de la tâche, défilement de la table jusqu'à sa ligne et du Gantt jusqu'à sa barre (comme H-UI-84) ; clic ailleurs : rien | hypothèse | Défiler le Gantt d'une page à droite ⇒ rectangle décalé d'autant ; clic sur « Murs » ⇒ 3 sélectionnée et visible | L4 |
| H-UI-82 | Impression : case « Chronologie » de Mise en page (`PageStyle.timeline`, enregistrée avec le projet comme le reste de la mise en page) ; Mode impression la coche d'office quand la frise est affichée à l'écran ; frise en tête de la **première page** de l'aperçu, du PDF et du SVG, sous l'en-tête et le cartouche, sur toute la largeur imprimable, même géométrie (H-R16) sans le rectangle visible ; les pages suivantes n'ont pas de frise | S82 (la chronologie s'imprime) ; position : hypothèse | Mode impression sur P avec frise affichée ⇒ page 1 : en-tête, frise, puis table et Gantt ; page 2 : pas de frise | L1, L4 |
| H-UI-83 | Insérer : Tâche = Inser existant ; **Jalon** = `task.add { name: « Nouveau jalon », milestone: true }` avant la première tâche sélectionnée (fin du projet sans sélection) ; **Récapitulative** = `task.add { name: « Nouvelle tâche récapitulative » }` avant la première sélectionnée, au niveau de celle-ci, puis `task.indent` des tâches sélectionnées, un seul lot, une annulation ; sans sélection : récapitulative vide en fin ; grisés sous tri ou filtre (H-UI-32) | S84 (groupe Insert) ; effet : hypothèse | Sélection {6, 7}, Récapitulative ⇒ 6 « Nouvelle tâche récapitulative » (niveau 2, sous 5) ⊃ 7 « Plâtrerie », 8 « Peinture » ; Ctrl+Z ⇒ P | L3 |
| H-UI-84 | Défiler jusqu'à la tâche : le Gantt défile pour placer le début de la barre de la tâche active à 1/8 de la largeur visible (`TimeAxis::x`, date → pixel), la table jusqu'à sa ligne ; jalon : son losange ; sans tâche active : grisé. Notes : « Informations sur la tâche » ouverte à l'onglet Notes | S84 (Notes au groupe Properties) ; reste : hypothèse | 7 active, zoom Jour, Gantt au début du projet ⇒ barre de 7 visible près du bord gauche | L3 |

Convention du rendu (`render.md` § H-R) :
- **H-R16 — Géométrie de la chronologie.** `timeline::layout(&TimelineInput, width16) -> TimelineLayout`, entiers en 1/16 px : axe linéaire du début à la fin du projet sur `width16` moins les marges des étiquettes d'extrémité (largeurs d'Inter, H-R11), abscisse d'un instant = arrondi inférieur de `(t − début) × largeur / (fin − début)` en `i128` puis borné ; projet de durée nulle ⇒ un jour ; hauteur fixe 72 px (16 × 72) : bande des étiquettes d'extrémité, trois rangées de barres de 14 px espacées de 2 px, axe, bande des jalons ; textes tronqués avec « … » à leur boîte (H-R11), jamais rognés ; sortie : listes de rectangles, losanges, segments et textes positionnés, dans l'ordre de dessin de H-UI-80 ; aucune dépendance à egui. Exemple : P sur 1 000 px utiles (du lun 19/10 08:00 au mar 17/11 17:00, 29,375 jours calendaires), 3 du lun 26/10 08:00 au ven 06/11 17:00 ⇒ barre de x ≈ 238 à 626 px de l'origine de l'axe, rangée 1 (L1 écrit dans la fiche les valeurs exactes en 1/16 px, marges comprises).

### 2.4 Propriété

| Id | Énoncé | Lot |
|---|---|---|
| P58 | Pour tout projet généré, toute liste ≤ 200 UID (absents, doublons, jalons, récapitulatives, tâches de durée nulle ou hors plage compris) et toute largeur de 0 à 100 000 px : `layout` ne panique pas, toute forme est dans la bande [0, largeur] × [0, 72 px], deux barres d'une même rangée ne se chevauchent pas, deux textes ne se chevauchent pas, éléments dessinés + compteur « +n » = éléments valides, deux appels ⇒ même résultat | L1 |

### 2.5 Budgets de performance (release, `tests/perf.rs`, job `perf` ; réseau généré de 5 000 tâches, 1920×1080)

| Scénario | Mesure | Budget | Lot |
|---|---|---|---|
| Scénarios J7 et J8 existants, ruban déplié et frise affichée (50 tâches de la frise) | inchangés | inchangés | L4, L6 |
| `timeline_5000` : 200 tâches sur la frise, largeur de fenêtre changée à chaque image (60 images), puis 60 images de défilement du Gantt | durée de `layout` ; durée d'image | `layout` médiane < 2 ms ; image médiane < 16 ms, p95 < 32 ms | L4 |
| `ribbon_tabs` : changement d'onglet à chaque image (5 onglets × 12) et survol d'éléments | durée d'image | médiane < 16 ms, p95 < 32 ms | L2 |
| Web | `.wasm` gzip -9 | ≤ 4 Mio (C-REL-05), hausse ≤ 150 Kio par rapport à `j8` | L6 |

---

## L0 — Contrat (séquentiel, en premier)

- **Objectif** : figer types, actions, textes, fiches et icônes ; squelettes qui compilent (vides ou message « non disponible », jamais `todo!`) ; **aucun changement visible** (captures intactes : commutateur `Chrome::Toolbar` par défaut).
- **Sources** : relire S80 à S84 (titre exact, phrase citée) et les écrire dans `docs/scheduling-rules.md` (table des sources) ; une page qui ne dit pas ce que le § 2.1 lui prête ⇒ convention en `hypothèse` pure, écart noté dans ADR-0017.
- **Icônes (premier commit)** :
  - `assets/icons/lucide/*.svg` : la liste du § 2.2 (environ 50 fichiers, copiés tels quels du paquet npm `lucide-static@1.55.0`, téléchargé hors du dépôt dans un dossier vide, jamais exécuté) ; `assets/icons/lucide/LICENSE` (texte ISC et texte MIT de Feather, tels quels) ; `NOTICE` complété (Lucide, version, licences).
  - `xtask/src/icons.rs` : `cargo xtask icons` lit chaque SVG avec `roxmltree` (0.21, au lock), accepte seulement `path`, `line`, `polyline`, `polygon`, `rect` (coins arrondis compris), `circle`, `ellipse`, sans `transform`, sur la grille 24 ; aplatit par `kurbo` (0.13, au lock ; tolérance 0,02 unité de grille) ; écrit `crates/planrs-ui-egui/src/ribbon/icon_data.rs` : `enum Icon` (un variant par fichier, nom en CamelCase), pour chaque icône des polylignes `&[&[(i16, i16)]]` en 1/64 d'unité de grille avec un drapeau « fermée », et les points isolés (sous-chemins de longueur < 0,1, ex. `h.01`) en disques ; fichier trié, déterministe ; refus explicite de tout élément ou attribut non pris en charge (`fill` autre que `none` ou `currentColor`, `transform`…). `cargo xtask icons --check` : échoue si le fichier généré diffère (test de fraîcheur, comme `cargo xtask fonts`), branché dans la CI (`checks`).
  - `crates/planrs-ui-egui/src/ribbon/icons.rs` : `pub fn paint_icon(painter: &egui::Painter, rect: egui::Rect, icon: Icon, color: egui::Color32)` : échelle = côté du rectangle / 24, trait = 2 × échelle, polylignes par `PathShape::line` / `closed_line`, disques de rayon trait/2 aux extrémités des polylignes ouvertes et aux points isolés (bouts ronds de Lucide) ; rectangle non fini ou vide ⇒ rien. Tests : `h_ui_77_icon_data_fresh` (appelle le générateur sur les SVG et compare), `h_ui_77_every_icon_has_shapes`, `h_ui_77_icon_inside_rect` (toute forme dans le rectangle à 16, 24 et 32 px), `h_ui_77_non_finite_rect_draws_nothing`.
  - Mesure consignée dans ADR-0017 : taille de `icon_data.rs`, octets ajoutés au `.wasm` gzip -9 (`trunk build --release` sous `memlimit 3`, ou relevé du job CI `Web` si le lien LTO ne tient pas en mémoire, comme en J8).
- **Types et API exactes (contrat)** :

```rust
// planrs_render::style (champs ajoutés, serde : défaut, omis s'ils sont vides)
pub struct TimelineSpec { pub tasks: Vec<u32> /* UID triés, sans doublon, ≤ MAX_TIMELINE_TASKS */ }
pub const MAX_TIMELINE_TASKS: usize = 200;
// PlanStyle.timeline: TimelineSpec ; StylePatch.timeline: Option<TimelineSpec> (remplacée en entier, comme `phases`)
// PageStyle.timeline: bool (#[serde(default, skip_serializing_if = "is_false")])

// planrs_render::timeline (squelette ; implémenté en L1, H-R16)
pub struct TimelineInput { pub start: Moment, pub finish: Moment, pub status: Option<Moment>,
  pub start_label: String, pub finish_label: String, pub status_label: Option<String>,
  pub ticks: Vec<(Moment, String)>, pub items: Vec<TimelineItem> }
pub struct TimelineItem { pub uid: TaskUid, pub id: u32, pub kind: TimelineKind, pub start: Moment, pub finish: Moment,
  pub name: String, pub date_label: String }
pub enum TimelineKind { Bar, Milestone }
pub struct TimelineLayout { pub height16: i64, pub shapes: Vec<TimelineShape>, pub hidden: u32 }
pub enum TimelineShape { Band { rect16: [i64; 4] }, Bar { uid: TaskUid, row: u8, rect16: [i64; 4] },
  Diamond { uid: TaskUid, center16: (i64, i64), half16: i64 }, Axis { y16: i64, x16: (i64, i64) },
  Tick { x16: i64 }, StatusLine { x16: i64 }, Text { x16: i64, y16: i64, text: String, role: TimelineText, uid: Option<TaskUid> } }
pub enum TimelineText { Bound, Tick, Status, BarName, MilestoneName, MilestoneDate, Overflow }
pub const TIMELINE_HEIGHT16: i64 = 72 * 16;
pub fn layout(input: &TimelineInput, width16: i64) -> TimelineLayout;   // L0 : bande vide

// planrs_automation::timeline (squelette ; L1)
pub fn input(s: &Session, spec: &TimelineSpec, lang: Language) -> TimelineInput; // dates et textes du moteur (règle 3)

// planrs_ui_egui::ribbon (L0 : types et table vide ; L2 : moteur et contenu)
pub enum RibbonTab { Task, Resource, Project, View, Format }   // serde, défaut Task
pub struct RibbonPrefs { pub tab: RibbonTab, pub collapsed: bool } // UiState.ribbon
pub enum RibbonOp { Select(RibbonTab), ToggleCollapsed, OpenOverlay(RibbonTab), CloseOverlay }
pub enum Chrome { Toolbar, Ribbon }  // non persistant ; App::set_chrome, App::chrome ; défaut Toolbar jusqu'à L5
pub struct GroupSpec { pub label: TextId, pub icon: Icon, pub items: &'static [ItemSpec], pub shrink_rank: u8 }
pub struct ItemSpec { pub size: ItemSize, pub icon: Option<Icon>, pub label: TextId, pub tip: TextId,
  pub binding: Option<Binding>, pub cmd: RibbonCmd }
pub enum ItemSize { Large, Small, PercentRow, Dropdown, Check }
pub enum RibbonCmd { /* une variante par ligne du § 2.2 ; RibbonCmd::action(&FrameCtx) -> Option<Action>
  et RibbonCmd::enabled(&FrameCtx) -> Enabled { yes, reason: Option<TextId> } partagés avec les menus */ }
pub const RIBBON: &[(RibbonTab, &[GroupSpec])];

// planrs_ui_egui::action (variantes ajoutées en fin d'énumération)
Action::Ribbon(RibbonOp), Action::Timeline(TimelineOp), Action::ScrollToTask, Action::ShowTaskNotes
pub enum TimelineOp { SetVisible(bool), ToggleSelection, Remove(TaskUid), Activate(TaskUid) }
StructureOp::InsertMilestone, StructureOp::InsertSummary   // `structure::plan` : None + message jusqu'à L3
UiBinding::ToggleRibbon  // KEYMAP : (Global, COMMAND+F1) ; web_interceptable : oui
// UiState (version 1 gardée) : ribbon: RibbonPrefs, timeline_visible: bool (défaut vrai ; sans effet tant que Chrome::Toolbar)
// view::ViewTransient : timeline: TimelineView (cache de la disposition, clé : génération, style, largeur, langue)
```

- **Réducteur (`app.rs`, posé en L0, rien d'autre à y toucher avant L5)** : `Ribbon(op)` ⇒ `crate::ribbon::reduce(&mut state.ribbon, &mut view.ribbon_overlay, op)` ; `Timeline(SetVisible)` ⇒ `state.timeline_visible` ; `Timeline(ToggleSelection | Remove)` ⇒ `crate::timeline_ops::batch(session, selection, op)` puis `Action::Exec` (squelette : `None` + message « non disponible ») ; `Timeline(Activate(uid))` ⇒ `Select` + `ScrollToTask` ; `ScrollToTask` ⇒ `crate::nav::scroll_to_task(…)` (squelette : rien) ; `ShowTaskNotes` ⇒ `crate::dialog::task_info::open_on(TaskInfoTab::Notes, …)` (squelette : onglet Général). `App::show` appelle `view::ribbon::show` au lieu de `view::chrome::toolbar` si `Chrome::Ribbon`, et `view::main_view` lit `fc.chrome` pour la frise.
- **Textes** : toutes les clés FR et EN du ruban (onglets, groupes, éléments, info-bulles, raisons de grisé), de la frise (« Début », « Fin », « État », « +{n} », Ajouter/Retirer de la chronologie, case Chronologie de la mise en page, message de limite) et des menus jumeaux nouveaux, sections `# --- RUBAN Lxx ---` ; chaque `TextId` nouveau est référencé par `RIBBON` ou par un squelette (`cargo xtask text-ids` vert dès L0).
- **Docs** : `docs/features/ui.md` § 1.11 (H-UI-69 à H-UI-84) et sections vides § 1.32 « Ruban » et § 1.33 « Chronologie » ; `render.md` H-R16 ; `scheduling-rules.md` S80 à S84 ; propriété P58 ; `docs/rename.md` si un identifiant nouveau porte le nom du produit.
- **Fixtures** : `crates/planrs-ui-egui/tests/fixtures/ui_state_v1.json` **régénéré une fois** (`ribbon`, `timeline_visible`) ; schémas de référence d'`apply_style` / `style.apply` régénérés (`StylePatch.timeline`) ; `project_v1.json` inchangé ; un MSPDI sans style relu et écrit : octets identiques.
- **Tests exigés** : `h_ui_77_*` (ci-dessus) ; `style_timeline_roundtrip` (P56 étendue : style avec `timeline` et `page.timeline` sérialisé puis relu = identité, MSPDI stylé lu puis écrit = mêmes octets) ; `style_timeline_bounded` (201 UID ou doublons ⇒ style borné ou diagnostic, jamais de panique) ; `keymap_ctrl_f1_toggles_ribbon` (liaison résolue, `web_interceptable`) ; `ui_state_v1_fixture` (relecture tolérante, champs nouveaux) ; `ribbon_spec_text_ids_exist` ; tous les tests J8 inchangés et verts.
- **Critère de fin** : tous les checks verts ; captures comparées sous lavapipe identiques ; aucun comportement visible changé.
- **Dépendances** : aucune. **Taille** : ~1 000 lignes hors SVG et fichier généré.

---

## L1 — Chronologie : géométrie, scène, SVG et PDF (render, automation)

- **Objectif** : la frise calculée une fois, sans egui, et imprimée.
- **Fichiers** : § 1, L1.
- **Comportement** :
  - `planrs_render::timeline::layout` selon H-R16 et H-UI-80 : axe, extrémités, graduations (mois ; trimestres au-delà de 24 mois ; années au-delà de 8 ans, choisi sur la durée du projet en jours entiers), date d'état si dans la plage, barres en 3 rangées au plus (premier ajustement par (début, ID), écart minimal 4 px entre barres d'une rangée), compteur « +n », jalons et étiquettes alternées, troncature « … » par H-R11 ; couleurs prises dans `PlanStyle` (jetons existants : barre de tâche, jalon, texte, texte atténué, date d'état ; aucun jeton nouveau).
  - `planrs_automation::timeline::input` : extrémités `project_start` / `project_finish` du planning, date d'état effective, graduations et étiquettes formatées par `SessionTexts` dans la langue demandée (dates courtes de H-UI-63 « JJ/MM/AA », jour de la semaine aux extrémités), éléments = UID de `spec.tasks` présents, non exclus du planning (une tâche inactive reste, grisée par le style `Inactive`), jalon si le moteur la dit jalon.
  - Pagination et scène : `PaginateOptions` / `PageStyle.timeline` ⇒ `Pages.timeline16: (i64, i64)` (haut et hauteur dans la scène, 0 sinon) ; la frise occupe le haut de la **première page seulement**, sous l'en-tête et le cartouche, sur la largeur imprimable ; les lignes de la page 1 diminuent d'autant (P54 tient toujours).
  - SVG (`svg.rs`) et PDF (`planrs-pdf`) dessinent `TimelineLayout` avec les mêmes primitives que la scène (rectangles, polygones, segments, glyphes placés un par un, H-R13) ; PDF toujours déterministe (P57).
  - `planrs_automation::pdf::export` et `export_rows` passent `page.timeline` ; `project.export_pdf` (CLI, MCP) imprime la frise si le style du document la demande.
- **Règles** : H-R16, H-UI-80, H-UI-82 (partie sortie), P58, P54, P57.
- **Tests exigés** : `h_r16_axis_maps_bounds` (début ⇒ marge gauche, fin ⇒ bord droit moins marge), `h_r16_example_p_wall_bar` (exemple de H-R16, ± 1/16 px), `h_r16_zero_length_project`, `h_r16_ticks_months_quarters_years`, `h_ui_80_rows_first_fit`, `h_ui_80_overflow_counter`, `h_ui_80_milestone_label_alternates`, `h_ui_80_text_truncated_not_clipped`, `p58_timeline_layout_props` (proptest, 256 cas), `h_ui_82_pdf_first_page_only` (frise sur la page 1, absente de la page 2 ; textes « Début », « Fin » extraits du PDF), `h_ui_82_svg_has_timeline`, `p54_with_timeline`, `p57_with_timeline`, `timeline_input_dates_from_engine` (automation : dates égales au `Schedule`, aucun calcul), `timeline_input_skips_deleted_uid`.
- **Critère de fin** : tests verts ; PDF de `tracking_j6.xml` avec frise relu par `pdftoppm` (artefact local, pas une référence) ; `pdf_5000` dans son budget.
- **Dépendances** : L0. **Parallèle** : L2 (fichiers disjoints). **Taille** : ~1 200 lignes.

---

## L2 — Ruban : moteur, contenu, menus jumeaux (interface)

- **Objectif** : le ruban complet derrière `Chrome::Ribbon`, chaque élément jumeau d'une entrée de menu.
- **Fichiers** : § 1, L2.
- **Comportement** :
  - `ribbon/spec.rs` : table `RIBBON` exactement selon le § 2.2 (ordre des onglets, des groupes et des éléments, tailles, icônes, `shrink_rank` de H-UI-75) ; `RibbonCmd::action` et `RibbonCmd::enabled` sont **les seules** sources de l'action et de l'activation ; les menus de `view/chrome.rs` qui ont un jumeau les appellent aussi (factorisation des prédicats aujourd'hui écrits dans `task_menu`, `edit_menu`, `format_menu`, `view_menu`).
  - `view/ribbon.rs` : `pub fn show(ui, fc, prefs, overlay)` : `egui::Panel::top("planrs_ribbon")` ; barre d'accès rapide (H-UI-76) ; bande d'onglets (`selectable_label` par onglet, `widget_info` nommé, double-clic ⇒ `RibbonOp::ToggleCollapsed`) ; corps (H-UI-71) ; grands et petits boutons dessinés à la main (`ui.allocate_exact_size`, `paint_icon`, galley du libellé ; état enfoncé, survol, focus clavier, grisé) avec `widget_info` (rôle bouton, case ou liste ; nom = libellé) pour l'accessibilité et `get_by_label` ; listes par `egui::containers::menu` (mêmes entrées que le menu) ; info-bulles (H-UI-73 : `on_hover_ui` avec libellé appuyé, aide, raccourci par `shortcut_for_os` ; grisé : `on_disabled_hover_ui` avec la raison) ; bascules (H-UI-74) ; réduction (H-UI-75 : largeurs mesurées une fois par langue et par largeur arrondie au pixel, cache dans la mémoire d'egui, réduction par paliers selon `shrink_rank`) ; repli et surimpression (H-UI-72 : `egui::Area` ancrée sous la bande d'onglets, ordre `Foreground`, fermée par Échap, clic extérieur ou action).
  - `view/chrome.rs` : jumeaux manquants ajoutés à la barre de menus (marqués « + » au § 2.2 : Édition → Défiler jusqu'à la tâche, Insérer une récapitulative, Insérer un jalon ; Tâche → Notes, Ajouter à la chronologie ; Affichage → Chronologie, Réduire le ruban (Ctrl/Cmd+F1) ; Format → Colonnes) ; `toolbar` et `task_tools` **inchangés** (retirés en L5).
- **Règles** : H-UI-69 à H-UI-77.
- **Tests exigés** (`tests/ui/ribbon.rs`, harnais en `Chrome::Ribbon`) : `h_ui_69_five_tabs_in_order` (FR et EN), `h_ui_69_tab_remembered`, `h_ui_69_ribbon_on_welcome_disabled`, `h_ui_70_ribbon_matches_spec` (libellés de chaque onglet dans l'ordre du § 2.2), `h_ui_70_every_item_has_menu_twin` (chaque `RibbonCmd` a une entrée de menu qui pousse la même `Action`), `h_ui_70_link_same_batch_as_menu` (P : Lier {2, 3} par le ruban et par le menu ⇒ même lot), `h_ui_70_disabled_without_selection`, `h_ui_71_large_small_geometry` (rectangles : grand ≥ 52 × 76 px, petit 22 px de haut, trois par colonne), `h_ui_72_double_click_collapses`, `h_ui_72_ctrl_f1_toggles`, `h_ui_72_collapsed_click_opens_overlay` (la table ne bouge pas : même rectangle avant et après), `h_ui_72_overlay_closes_on_action_and_escape`, `h_ui_72_collapsed_persisted`, `h_ui_73_tooltip_shows_shortcut` (Indenter ⇒ « Alt+Maj+→ » ; macOS simulé ⇒ glyphes de `format_shortcut`), `h_ui_73_disabled_reason` (tri actif ⇒ « Retirez le tri ou le filtre »), `h_ui_74_toggle_reflects_active_task`, `h_ui_75_no_overflow_widths` (800, 1 024, 1 440, 1 920, 3 840 px, FR et EN, chaque onglet : aucun élément hors du panneau, aucun chevauchement), `h_ui_75_collapsed_group_menu_runs_action`, `h_ui_76_quick_access_undo_disabled`, `h_ui_77_disabled_icon_dimmed`, `ribbon_percent_row_marks_50` (R-252 par le ruban), `ribbon_tabs` (perf, § 2.5).
- **Critère de fin** : tests verts ; captures existantes intactes (`Chrome::Toolbar` par défaut) ; capture locale `snapshot … --chrome ribbon` relue par l'agent (artefact, pas une référence).
- **Dépendances** : L0. **Parallèle** : L1. **Taille** : ~1 450 lignes.

---

## L3 — Commandes nouvelles du ruban (structure, notes, défilement, ajout à la frise)

- **Objectif** : servir les quatre actions et deux opérations de structure du contrat, par des lots de commandes existantes.
- **Fichiers** : § 1, L3.
- **Comportement** :
  - `structure::plan` : `InsertMilestone` et `InsertSummary` selon H-UI-83 (placement par `Placement::Before`, étiquette `$s` de R-150 pour indenter sous la tâche créée, un seul `Batch`) ; refus sous tri ou filtre (H-UI-32, comme `InsertBefore` aujourd'hui s'il l'est, sinon comme Abaisser) ; sélection non contiguë pour Récapitulative : refusée avec message (« Sélectionnez des tâches voisines »), rien écrit.
  - `timeline_ops::batch(session, selection, op) -> Result<Option<Batch>, TimelineOpError>` : liste actuelle du style effectif ; `ToggleSelection` et `Remove` selon H-UI-79 ; liste triée, sans doublon, ≤ 200 (sinon `TimelineOpError::TooMany`, message) ; un `style.apply { patch: { timeline } }`, et `template` = modèle par défaut des préférences si le document n'a pas de style (comme la couleur de phase, J8 L15) ; rien à faire ⇒ `None`.
  - `nav::scroll_to_task` (H-UI-84) : ligne de la tâche par `RowView::display_of` (masquée ⇒ message « tâche masquée par la vue »), abscisse par `TimeAxis::x(start)` du planning, défilement horizontal borné ; jalon : son instant.
  - `dialog/task_info.rs` : `open_on(TaskInfoTab::Notes, …)` ouvre la fenêtre existante sur l'onglet Notes.
- **Règles** : H-UI-79 (lot et bascule), H-UI-83, H-UI-84.
- **Tests exigés** (`tests/ui/ribbon_commands.rs`) : `h_ui_83_insert_milestone` (jalon avant 6, durée 0, une annulation), `h_ui_83_insert_summary_one_undo` (exemple de H-UI-83 ; Ctrl+Z ⇒ P), `h_ui_83_insert_summary_no_selection_at_end`, `h_ui_83_insert_summary_non_contiguous_refused`, `h_ui_83_locked_under_sort`, `h_ui_79_add_selection_one_undo` (exemple de H-UI-79 : `PlanStyle.timeline.tasks == [3, 4]`), `h_ui_79_toggle_removes_when_all_present`, `h_ui_79_limit_200`, `h_ui_79_unstyled_doc_gets_default_template`, `h_ui_84_scroll_to_task` (exemple de H-UI-84 : abscisse de la barre de 7 dans [largeur/8 − 1, largeur/8 + 1]), `h_ui_84_hidden_task_message`, `h_ui_84_notes_tab`.
- **Critère de fin** : tests verts ; aucune capture touchée.
- **Dépendances** : L0. **Parallèle** : L4 (fichiers disjoints). **Taille** : ~800 lignes.

---

## L4 — Chronologie à l'écran et à l'impression (interface)

- **Objectif** : la frise dans la vue Gantt, cliquable, et dans la mise en page.
- **Fichiers** : § 1, L4.
- **Comportement** :
  - `view/timeline.rs` : `pub fn show(ui, fc, view: &mut TimelineView)` en tête de la vue Gantt (`main_view.rs`), si `Chrome::Ribbon` et `UiState.timeline_visible` et vue Gantt ou Gantt de suivi ; disposition par `planrs_automation::timeline::input` + `planrs_render::timeline::layout`, **mise en cache** (clé : génération du document, empreinte du style effectif, largeur arrondie au pixel, langue) ; peinte au `Painter` (formes et galleys, comme le Gantt stylé de L11 ; police du planning du style) ; rectangle de la partie visible (H-UI-81 : abscisses du Gantt projetées par le rapport `largeur_frise / largeur_totale_gantt`, sans date) ; clic ⇒ `Timeline(Activate(uid))` ; clic droit ⇒ menu « Retirer de la chronologie » (`Timeline(Remove(uid))`) ; `widget_info` par élément (nom de la tâche) ; séparateur bas de 1 px.
  - `view/main_view.rs` : place la frise au-dessus de la table et du Gantt (la frise ne défile pas avec eux) ; menu contextuel de la table et du Gantt : entrée « Ajouter à la chronologie » / « Retirer de la chronologie » (libellé selon H-UI-74) ⇒ `Timeline(ToggleSelection)`.
  - `dialog/page_setup.rs` : case « Chronologie » (H-UI-82) liée à `page.timeline` du brouillon ; en mode impression, cochée d'office si `timeline_visible` ; aperçu, « Exporter en PDF… » et « Exporter en SVG… » par le chemin de L1 ; `view/scene_paint.rs` peint la frise de la page 1 de l'aperçu depuis `TimelineLayout`.
  - `tests/perf.rs` : scénario `timeline_5000` (§ 2.5) ; scénarios existants relancés avec `Chrome::Ribbon` et 50 tâches sur la frise.
- **Règles** : H-UI-78, H-UI-79 (menu contextuel), H-UI-80 (écran), H-UI-81, H-UI-82 (interface).
- **Tests exigés** (`tests/ui/timeline.rs`) : `h_ui_78_timeline_shows_project_bounds` (P : « Début lun 19/10/26 », « Fin mar 17/11/26 » présents), `h_ui_78_toggle_hides` (case décochée ⇒ table remontée de 72 px), `h_ui_78_absent_from_resource_views`, `h_ui_78_status_date_line`, `h_ui_79_context_menu_entry` (clic droit sur 3 ⇒ « Ajouter à la chronologie » ⇒ barre « Murs » sur la frise), `h_ui_80_screen_matches_layout` (rectangles peints = `TimelineLayout` à 1/16 px près, comme P55), `h_ui_81_click_selects_and_scrolls`, `h_ui_81_visible_window_tracks_scroll`, `h_ui_81_right_click_removes`, `h_ui_82_print_mode_includes_timeline`, `h_ui_82_unchecked_no_timeline`, `h_ui_82_saved_with_project` (« Enregistrer avec le projet » ⇒ `page.timeline` dans le style, une annulation), `timeline_cache_reused` (aucune nouvelle disposition sur 10 images immobiles), `timeline_5000` (perf).
- **Critère de fin** : tests verts ; budgets du § 2.5 tenus en release sur serveurdvp (Sobre et Classique) ; captures existantes intactes (`Chrome::Toolbar` par défaut).
- **Dépendances** : L0, L1. **Parallèle** : L3. **Taille** : ~1 300 lignes.

---

## L5 — Bascule visible et captures (seul)

- **Objectif** : rendre le ruban et la frise visibles par défaut, retirer l'ancienne barre d'outils, **une seule** régénération des captures.
- **Fichiers** : § 1, L5.
- **Comportement** : `Chrome::Ribbon` devient le seul habillage (variante `Toolbar`, `view::chrome::toolbar`, `task_tools` et `App::set_chrome` retirés ; les tests de L2 et L4 perdent leur `set_chrome`) ; textes de la barre d'outils devenus inutiles retirés des `.ftl` et de `text_ids!` (`cargo xtask text-ids`) ; tests J7 et J8 qui cliquaient la barre d'outils réécrits sur le ruban (même assertion, autre chemin, liste dans le commit) ; exemple `snapshot` : options `--tab <onglet>`, `--ribbon-collapsed`, vue `timeline` (P avec {2, 3, 4, 6, 7} sur la frise) et `print-timeline` ; `docs/features/ui.md` § 1.32, § 1.33 et limites ; `docs/user/` (guide : « Où est… » pour l'utilisateur de MS Project, cinq onglets).
- **Captures** (lavapipe, une fois) : les 46 références régénérées (ruban à la place des deux rangées d'outils ; frise vide en tête des vues Gantt) ; nouvelles : `ribbon_{task,resource,project,view,format}_fr_light`, `ribbon_task_en_dark`, `ribbon_collapsed_fr_light`, `ribbon_overlay_fr_light`, `ribbon_800px_fr_light`, `ribbon_hidpi_fr_light` (`pixels_per_point` 2), `timeline_fr_light`, `timeline_en_dark`, `print_timeline_fr_light`. Avant/après (anciennes et nouvelles références côte à côte) dans `/mnt/backup/tmp/planrs-ribbon-avant-apres/` sur serveurdvp, pour le propriétaire.
- **Règles** : toutes celles du cahier, à l'écran par défaut.
- **Tests exigés** : suite `ui` complète verte captures comparées (`PLANRS_UI_SNAPSHOTS=1`) ; `h_ui_69_default_chrome_is_ribbon` ; `h_ui_78_default_visible` ; `relay_j7.jsonl` et `relay_j8.jsonl` régénérés si `app.ui_state` change (`visible_rows`), écart expliqué dans le commit.
- **Critère de fin** : tests et captures verts en CI (job `ui`) ; budgets de fluidité tenus ; avant/après prêt.
- **Dépendances** : L2, L3, L4. **Parallèle** : aucun. **Taille** : ~700 lignes (références d'images non comptées).

---

## L6 — Vérification finale

- **Objectif** : preuve que le ruban et la frise tiennent les règles du dépôt avant `v1.0.0`.
- **Fichiers** : § 1, L6.
- **Vérifications** :
  1. `cargo xtask rules` vert ; H-UI-69 à H-UI-84, H-R16 et P58 ont chacune au moins un test à leur nom ; S80 à S84 relues et citées.
  2. Scénario kittest de bout en bout `e2e_ribbon` sur P, **sans menu ni raccourci** sauf Ctrl+F1 : onglet Tâche ; sélectionner 2 et 3, Lier (déjà liés : lot vide, message) ; 50 % sur 1 ; Planifier manuellement sur 6 puis à nouveau ; Insérer → Jalon avant 5 ; Insérer → Récapitulative sur {6, 7} ; Propriétés → Ajouter à la chronologie sur {3, 4} ; Affichage → Mode plan → Niveau 1 puis Tous ; Filtre « Critiques » puis Effacer ; Zoom → Projet entier ; clic sur « Murs » dans la frise ⇒ 3 active ; Ctrl+F1, clic sur Projet en surimpression, Définir la référence, fermer ; Projet → Mode impression : frise cochée, aperçu page 1 avec frise, Exporter en PDF (service en mémoire) ; Format → Modèles de mise en forme, appliquer Présentation ; annuler tout ⇒ projet égal à l'ouverture ; rétablir tout ; enregistrer et relire ⇒ mêmes tâches, même style (frise comprise).
  3. Parité clavier : `h_ui_70_every_item_has_menu_twin` vert ; parcours de la barre de menus au clavier (F10/Alt d'egui non garanti : vérifier au moins que chaque jumeau est atteignable par les menus et les raccourcis listés).
  4. Fluidité (§ 2.5) en release, Sobre et Classique, rapport JSON en artefact du job `perf`.
  5. Web : `trunk build --release` (CI de release ou relevé local), `.wasm` gzip -9 ≤ 4 Mio et hausse ≤ 150 Kio par rapport à `j8` (chiffre consigné dans ADR-0017) ; démarrage sous Chromium sans tête (C-REL-06) ; Ctrl+F1 retenu au navigateur (`web_intercepts`).
  6. Licences : `cargo deny check` vert ; `NOTICE` et `assets/icons/lucide/LICENSE` présents dans les archives desktop et web (`release.yml`, `cargo xtask release-check`) ; aucune image Microsoft dans le dépôt (relecture des fichiers ajoutés depuis `j8`).
  7. Corpus et rejeux inchangés (parité, `FLOOR`, octets des MSPDI sans style) ; nombre de tests ≥ celui de `j8` + tests du cahier.
  8. Docs : `ui.md`, `render.md`, `docs/user/*`, `scorecard.md` (lignes Interface et Rendu : implémenté, testé ; rien de « vérifié contre MS Project »), `CHANGELOG.md` (section non publiée), ADR-0017 complété des écarts par lot.
- **Hors lot (session principale et propriétaire)** : avant/après montré au propriétaire ; fusion `ribbon-dev` → `main` après une CI **existante et verte sur le SHA de tête** ; essai par le propriétaire sur son Mac (ruban, Cmd+F1, frise, impression) avant `v1.0.0-rc.1`.
- **Critère de fin** : tous les checks de `CLAUDE.md` verts en local et en CI ; aucune convention du cahier sans test.
- **Dépendances** : tous les lots. **Taille** : ~600 lignes.

## 3. Points laissés ouverts

1. **Libellés français** des onglets et groupes : choisis (Tâche, Ressource, Projet, Affichage, Format ; Presse-papiers, Planification, Insérer, Propriétés, Édition…), non relus sur l'interface française de Project ; le propriétaire peut les corriger dans les `.ftl` sans toucher au code.
2. **Hauteur** : ruban déplié + frise ≈ 216 px au-dessus de la table ; sur un écran de portable (768 px utiles), conseiller le repli (Ctrl/Cmd+F1). Repli automatique sous une hauteur de fenêtre : non prévu (à décider au vu des captures).
3. **Cmd+F1 sur macOS** : Office pour Mac utilise d'autres raccourcis pour le ruban ; Cmd+F1 peut être pris par le système (luminosité, recopie vidéo) selon les réglages du clavier : le double-clic et le menu Affichage restent les chemins garantis.
4. **Style écrit pour la seule frise** : ajouter une tâche à la frise d'un document sans style lui donne un style (modèle par défaut des préférences), donc une instruction `<?planrs-style?>` à l'enregistrement ; avec « ne jamais écrire le style », la frise n'est pas enregistrée (dit dans l'info-bulle de « Ajouter à la chronologie »).
5. **Reportés après v1.0.0** (ADR-0017) : groupe Police, onglet « Format de la chronologie » (légendes en encadré, plusieurs barres, plages de dates), glisser le rectangle visible, copier la frise, case de la fenêtre Informations, personnalisation du ruban et de l'accès rapide, info-touches Alt, coulisses Fichier, Ajouter des ressources, nivellement, Planificateur d'équipe, vues Réseau, Calendrier et Détails, Respecter les liens, Marquer comme prévu.
