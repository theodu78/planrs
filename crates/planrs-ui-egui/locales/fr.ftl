# Traductions françaises de l'interface (langue de repli).
# Chaque clé correspond à une variante de `TextId` (src/i18n.rs) ; un test
# vérifie que fr.ftl et en.ftl déclarent exactement les mêmes clés.
# Typographie : espace insécable U+00A0 avant « : ; ? ! », jamais U+202F
# (absente des polices d'egui) ; points de suspension U+2026 « … » (présents
# dans les polices d'egui, qui s'en servent pour tronquer).

## Écran d'accueil

welcome-title = Bienvenue dans { $product }
welcome-subtitle = Clone libre de MS Project — ouvrir, modifier, enregistrer
welcome-open-demo = Ouvrir l'exemple : construction d'une maison
welcome-open-file = Ouvrir un fichier MSPDI (.xml)…
welcome-new-project = Nouveau projet
welcome-web-note = Le fichier reste dans votre navigateur ; rien n'est envoyé.
welcome-language = Langue
welcome-theme = Thème

## Menus

menu-file = Fichier
menu-open = Ouvrir un fichier MSPDI (.xml)…
menu-open-demo = Ouvrir l'exemple
menu-close = Fermer
menu-view = Affichage
menu-zoom = Zoom
menu-theme = Thème
menu-language = Langue
menu-diagnostics = Diagnostics

## Barre d'outils et zoom

toolbar-open = Ouvrir…
toolbar-fit = Ajuster
zoom-day = Jour
zoom-week = Semaine
zoom-month = Mois
zoom-quarter = Trimestre

## Thème et langue (noms de langue dans leur propre langue)

theme-system = Système
theme-light = Clair
theme-dark = Sombre
lang-system = Système
lang-fr = Français
lang-en = English

## Colonnes de la table

col-id = N°
col-name = Nom de la tâche
col-duration = Durée
col-start = Début
col-finish = Fin
col-predecessors = Prédécesseurs
col-total-slack = Marge totale
col-critical = Critique
col-percent-complete = % achevé
col-resource-names = Noms ressources
col-baseline-start = Début de référence
col-baseline-finish = Fin de référence
col-finish-variance = Variation de fin
yes = Oui
no = Non

## Unités de durée (forme longue, hypothèse H-UI-04 : « 5 jours », « 1 jour? »)

unit-minute-one = minute
unit-minute-other = minutes
unit-hour-one = heure
unit-hour-other = heures
unit-day-one = jour
unit-day-other = jours
unit-week-one = semaine
unit-week-other = semaines
unit-month-one = mois
unit-month-other = mois
unit-minute-elapsed-one = minute écoulée
unit-minute-elapsed-other = minutes écoulées
unit-hour-elapsed-one = heure écoulée
unit-hour-elapsed-other = heures écoulées
unit-day-elapsed-one = jour écoulé
unit-day-elapsed-other = jours écoulés
unit-week-elapsed-one = semaine écoulée
unit-week-elapsed-other = semaines écoulées
unit-month-elapsed-one = mois écoulé
unit-month-elapsed-other = mois écoulés

## Types de liens (codes de MS Project en français)

link-fs = FD
link-ss = DD
link-ff = FF
link-sf = DF

## Conventions de formatage (lues par Texts::format_labels)

fmt-decimal-sep = ,
fmt-list-sep = ;
fmt-date-order = dmy
fmt-unknown = ?
# Règle de pluriel des unités : below-two (singulier si |v| < 2) ou
# exactly-one (singulier si |v| = 1).
fmt-plural-rule = below-two

## Mois

month-1-long = janvier
month-1-short = janv.
month-2-long = février
month-2-short = févr.
month-3-long = mars
month-3-short = mars
month-4-long = avril
month-4-short = avr.
month-5-long = mai
month-5-short = mai
month-6-long = juin
month-6-short = juin
month-7-long = juillet
month-7-short = juil.
month-8-long = août
month-8-short = août
month-9-long = septembre
month-9-short = sept.
month-10-long = octobre
month-10-short = oct.
month-11-long = novembre
month-11-short = nov.
month-12-long = décembre
month-12-short = déc.

## Jours de la semaine

weekday-mon-short = lun
weekday-mon-initial = L
weekday-tue-short = mar
weekday-tue-initial = M
weekday-wed-short = mer
weekday-wed-initial = M
weekday-thu-short = jeu
weekday-thu-initial = J
weekday-fri-short = ven
weekday-fri-initial = V
weekday-sat-short = sam
weekday-sat-initial = S
weekday-sun-short = dim
weekday-sun-initial = D

## Échelle de temps

tier-quarter = T
tier-week = S

## Barre d'état

status-tasks =
    { $count ->
        [one] { $count } tâche
       *[other] { $count } tâches
    }
status-range = { $start } – { $finish }
status-errors =
    { $count ->
        [0] aucune erreur
        [one] { $count } erreur
       *[other] { $count } erreurs
    }
status-warnings =
    { $count ->
        [0] aucun avertissement
        [one] { $count } avertissement
       *[other] { $count } avertissements
    }
status-no-document = Aucun document ouvert

## Ouverture de fichiers

open-error-title = Impossible d'ouvrir le fichier
open-error-read = Lecture du fichier impossible : { $detail }
open-error-mspdi = Fichier MSPDI illisible : { $detail }

## Nouveau, Enregistrer, Enregistrer sous (lot L13)

menu-new = Nouveau
menu-save = Enregistrer
menu-save-as = Enregistrer sous…
menu-quit = Quitter
new-project-name = Projet1
dialog-save-title = Enregistrer le projet (MSPDI)
save-in-progress = Un enregistrement est déjà en cours.
save-error-title = Impossible d'enregistrer le fichier
save-error-write = Écriture du fichier impossible : { $detail }
save-adjusted =
    { $count ->
        [one] Enregistré : { $count } valeur ajustée au format MSPDI.
       *[other] Enregistré : { $count } valeurs ajustées au format MSPDI.
    }
confirm-unsaved-title = Enregistrer les modifications de « { $name } » ?
confirm-save = Enregistrer
confirm-discard = Ne pas enregistrer
confirm-cancel = Annuler

## Bandeau de messages

message-dismiss = Fermer le message
range-truncated = Plage d'affichage limitée à 20 ans

## Accessibilité et divers

row-label = Ligne { $id } : { $name }
splitter-label = Séparateur entre la table et le Gantt
gantt-scroll-label = Défilement horizontal du Gantt
demo-file-name = Exemple : construction d'une maison
file-filter-mspdi = Fichier MSPDI (XML)
dialog-open-title = Ouvrir un fichier MSPDI
state-load-warning = Préférences d'affichage ignorées : { $detail }

## Menu Édition

menu-edit = Édition
menu-undo = Annuler
menu-redo = Rétablir

## Structure (menu contextuel, confirmation de suppression)

structure-insert = Insérer une tâche
structure-delete = Supprimer la tâche
structure-indent = Abaisser
structure-outdent = Hausser
structure-link = Lier les tâches
structure-unlink = Supprimer les liaisons
confirm-delete-title = Supprimer une tâche récapitulative
confirm-delete-text =
    { $count ->
        [one] La tâche sera supprimée.
       *[other] La récapitulative et ses sous-tâches seront supprimées ({ $count } tâches).
    }
confirm-delete-yes = Supprimer
confirm-delete-no = Annuler

## Refus de commande (une clé par code d'erreur du moteur, `error-<code>`)

command-error-title = Modification refusée
error-unknown = Erreur : { $detail }
error-assignment-duplicate = Ressource déjà affectée : { $detail }
error-assignment-kind_mismatch = Champ contraire au type de la ressource : { $detail }
error-assignment-not_found = Affectation introuvable : { $detail }
error-assignment-zero_units_fixed_work = Unités nulles sur une tâche à travail fixe : { $detail }
error-baseline-exists = Ligne de base déjà enregistrée : { $detail }
error-baseline-rollup_needs_selection = Cumul sans sélection : { $detail }
error-batch-duplicate_label = Étiquette en double : { $detail }
error-batch-unknown_label = Étiquette inconnue : { $detail }
error-batch-unsupported_version = Version de format non prise en charge : { $detail }
error-batch-would_create_errors = Modification qui créerait des erreurs : { $detail }
error-calendar-exception_not_found = Exception de calendrier introuvable : { $detail }
error-calendar-exception_overlap = Exceptions de calendrier qui se chevauchent : { $detail }
error-calendar-not_found = Calendrier introuvable : { $detail }
error-command-not_yet_available = Commande pas encore disponible : { $detail }
error-explain-not_scheduled = Tâche non planifiée : { $detail }
error-file-invalid = Fichier illisible : { $detail }
error-file-open_in_app = Fichier ouvert dans l'application : { $detail }
error-file-outside_root = Chemin non autorisé : { $detail }
error-file-read = Lecture impossible : { $detail }
error-file-too_large = Fichier trop volumineux : { $detail }
error-file-write = Écriture impossible : { $detail }
error-history-empty = Rien à annuler ni à rétablir : { $detail }
error-history-inconsistent = Historique incohérent : { $detail }
error-limit-exceeded = Limite dépassée : { $detail }
error-link-duplicate = Lien déjà présent : { $detail }
error-link-not_found = Lien introuvable : { $detail }
error-link-self = Lien d'une tâche vers elle-même : { $detail }
error-link-summary_descendant = Lien entre une récapitulative et sa sous-tâche : { $detail }
error-param-conflict = Valeurs qui s'excluent : { $detail }
error-param-invalid = Valeur refusée : { $detail }
error-param-mismatch = Valeurs contradictoires : { $detail }
error-progress-duration_below_actual = Durée sous la durée réelle : { $detail }
error-progress-inactive = Tâche inactive : { $detail }
error-progress-invalid_order = Fin réelle avant le début réel : { $detail }
error-progress-summary_not_editable = Avancement d'une récapitulative : { $detail }
error-resource-has_assignments = Ressource affectée : { $detail }
error-resource-not_found = Ressource introuvable : { $detail }
error-session-none = Aucun document : { $detail }
error-session-poisoned = Session indisponible : { $detail }
error-task-computed_field = Champ calculé : { $detail }
error-task-fixed_work_effort_driven = Travail fixe toujours piloté par l'effort : { $detail }
error-task-has_children = Récapitulative avec sous-tâches : { $detail }
error-task-manual_unsupported = Tâche planifiée manuellement : { $detail }
error-task-move_into_subtree = Déplacement impossible : { $detail }
error-task-not_found = Tâche introuvable : { $detail }
error-task-outline_refused = Hiérarchie refusée : { $detail }
error-text-bad_date = Date illisible : { $detail }
error-text-bad_link_type = Type de lien inconnu : { $detail }
error-text-bad_number = Nombre illisible : { $detail }
error-text-empty = Saisie vide : { $detail }
error-text-external_predecessor = Prédécesseur externe : { $detail }
error-text-negative = Valeur négative : { $detail }
error-text-out_of_range = Valeur hors limites : { $detail }
error-text-too_long = Saisie trop longue : { $detail }
error-text-unknown_unit = Unité inconnue : { $detail }

## Édition des cellules (lot L12)

blank-row-label = Ligne vide (nouvelle tâche)
cell-edit-label = Saisie : { $column }
edit-error-unknown-id = Aucune tâche d'ID { $id }
edit-error-not-editable = Colonne calculée, non modifiable

## J7 — L0 : Codes de refus du contrat (ADR-0015)

error-calendar-name_taken = Nom de calendrier déjà pris : { $detail }
error-calendar-base_not_base = Base de calendrier invalide : { $detail }
error-calendar-in_use = Calendrier utilisé : { $detail }
error-gesture-not_draggable = Geste impossible sur cette barre : { $detail }
error-usage-limit = Rapport d'utilisation trop grand : { $detail }
error-control-rejected = Refusé dans l'application : { $detail }
error-control-approval_timeout = Aucune réponse dans le délai : { $detail }
error-control-not_relayable = Commande non relayable : { $detail }
error-control-not_attached = Aucune application rattachée : { $detail }
error-control-app_gone = Application fermée : { $detail }
error-control-screenshot_unavailable = Capture indisponible : { $detail }
error-control-screenshot_too_large = Capture trop grande : { $detail }
error-control-busy = Application occupée : { $detail }
error-control-document_changed = Document de l'application changé : { $detail }

# --- J7 L1 ---
table-label = Table des tâches
nav-cursor-label = Ligne { $id }, { $column }, { $value }
nav-cursor-blank-label = Ligne vide, { $column }
nav-cursor-empty-value = vide

# --- J7 L2 ---

# --- J7 L3 ---
approval-title = L'agent propose sur « { $document } » : { $name }
approval-changed =
    { $count ->
        [0] aucune tâche modifiée
        [one] { $count } tâche modifiée
       *[other] { $count } tâches modifiées
    }
approval-finish = fin { $value }
approval-finish-same = fin inchangée
approval-critical = chemin critique modifié
approval-warnings =
    { $count ->
        [one] { $count } avertissement
       *[other] { $count } avertissements
    }
approval-queued =
    { $count ->
        [one] { $count } autre proposition en attente
       *[other] { $count } autres propositions en attente
    }
approval-accept = Accepter
approval-reject = Refuser
menu-undo-agent = Annuler : agent — { $detail }
menu-undo-agent-ops =
    { $count ->
        [one] { $count } opération
       *[other] { $count } opérations
    }

# --- J7 L4 ---

# --- J7 L5 ---

# --- J7 L6 ---
fmt-percent-suffix = {"\u00A0%"}
column-header-label = Colonne { $column }
column-resize-label = Largeur de la colonne { $column }
column-menu-hide = Masquer la colonne
table-scroll-label = Défilement horizontal de la table

# --- J7 L7 ---
menu-gantt-standard = Diagramme de Gantt
menu-gantt-tracking = Gantt de suivi

# --- J7 L8 ---
gantt-reason-summary = récapitulative (seul le lien est permis)
gantt-reason-started = tâche commencée
gantt-reason-milestone = jalon
gantt-reason-excluded = tâche exclue du calcul
gantt-reason-blank = ligne vide
gantt-reason-overflow = écart hors des dates permises
gantt-reason-same-task = lien d'une tâche vers elle-même
link-dialog-title = Dépendance
link-dialog-from = De
link-dialog-to = À
link-dialog-type = Type
link-dialog-lag = Décalage
link-dialog-delete = Supprimer
link-dialog-stale = Le projet a changé : la dépendance a été relue.
link-dialog-task = { $id } { $name }

# --- J7 L9 ---
task-info-menu = Informations sur la tâche…
task-info-title = Informations sur la tâche — { $id } { $name }
task-info-tab-general = Général
task-info-tab-predecessors = Prédécesseurs
task-info-tab-resources = Ressources
task-info-tab-advanced = Avancé
task-info-tab-notes = Notes
task-info-name = Nom
task-info-duration = Durée
task-info-percent = % achevé
task-info-start = Début
task-info-finish = Fin
task-info-active = Active
task-info-pred-id = ID
task-info-pred-type = Type
task-info-pred-lag = Décalage
task-info-row-field = { $column }, ligne { $count }
task-info-add-predecessor = Ajouter un prédécesseur
task-info-add-resource = Affecter une ressource
task-info-remove-row = Retirer
task-info-resource-name = Ressource
task-info-resource-units = Unités
task-info-no-resource = Aucune ressource dans le projet.
task-info-constraint = Type de contrainte
task-info-constraint-date = Date de contrainte
task-info-deadline = Échéance
task-info-task-type = Type de tâche
task-info-effort-driven = Pilotée par l'effort
task-info-notes = Notes de la tâche
task-info-link-fs = Fin à début (FD)
task-info-link-ss = Début à début (DD)
task-info-link-ff = Fin à fin (FF)
task-info-link-sf = Début à fin (DF)
task-info-constraint-asap = Dès que possible
task-info-constraint-alap = Le plus tard possible
task-info-constraint-mso = Doit commencer le
task-info-constraint-mfo = Doit finir le
task-info-constraint-snet = Début au plus tôt le
task-info-constraint-snlt = Début au plus tard le
task-info-constraint-fnet = Fin au plus tôt le
task-info-constraint-fnlt = Fin au plus tard le
task-info-type-fixed-units = Capacité fixe
task-info-type-fixed-duration = Durée fixe
task-info-type-fixed-work = Travail fixe
task-info-ok = OK
task-info-cancel = Annuler
task-info-stale = Le projet a changé depuis l'ouverture : les valeurs ont été relues, rien n'a été appliqué.
task-info-date-required = Date obligatoire pour ce type de contrainte
task-info-bad-id = ID illisible : { $id }
task-info-duplicate-predecessor = Prédécesseur en double : { $id }
task-info-duplicate-resource = Ressource affectée deux fois : { $name }
task-info-unknown-resource = Ressource inconnue

# --- J7 L10 ---
menu-view-resource-sheet = Feuille des ressources
res-sheet-label = Feuille des ressources
res-col-indicator = Ind.
res-col-name = Nom de la ressource
res-col-kind = Type
res-col-material-label = Étiquette
res-col-initials = Initiales
res-col-group = Groupe
res-col-max-units = Capacité max.
res-col-standard-rate = Taux standard
res-col-overtime-rate = Taux heures sup.
res-col-cost-per-use = Coût/utilisation
res-col-calendar = Calendrier
res-kind-work = Travail
res-kind-material = Matériel
res-kind-cost = Coût
rate-minute = /min
rate-hour = /h
rate-day = /j
rate-week = /sem
rate-month = /mois
rate-year = /an
res-row-label = Ressource { $id } : { $name }
res-blank-row-label = Ligne vide (nouvelle ressource)
res-name-first = Saisir d'abord le nom de la ressource
res-unknown-calendar = Aucun calendrier nommé « { $name } »
res-calendar-none = (aucun)
res-delete-title = Supprimer une ressource affectée
res-delete-text =
    { $count ->
        [one] « { $name } » est affectée à une tâche : l'affectation sera retirée avec elle.
       *[other] « { $name } » a { $count } affectations : elles seront retirées avec elle.
    }

# --- J7 L11 ---
menu-view-resource-usage = Utilisation des ressources
usage-sheet-label = Utilisation des ressources
usage-periods-label = Travail par période
usage-col-expand = +/-
usage-col-name = Nom de la ressource
usage-col-work = Travail
usage-hours-suffix = {"\u00A0"}h
usage-expand = Développer { $name }
usage-collapse = Réduire { $name }
usage-assignment-label = Affectation : { $name } ({ $detail })
usage-limit = Utilisation trop grande pour être affichée : replier des ressources ({ $detail }).

# --- J7 L12 ---
menu-project = Projet
menu-project-info = Informations sur le projet…
menu-working-time = Modifier le temps de travail…
menu-baseline = Ligne de base…
menu-reschedule = Replanifier le travail restant…
scope-project = Projet entier
scope-selection = { $count ->
        [one] Tâche sélectionnée (1)
       *[other] Tâches sélectionnées ({ $count })
    }
wt-title = Modifier le temps de travail
wt-calendar = Pour le calendrier
wt-project-calendar = { $name } (calendrier du projet)
wt-resource-calendar = { $name } (ressource)
wt-new = Nouveau calendrier…
wt-delete = Supprimer le calendrier
wt-name = Nom du calendrier
wt-base-of = Calendrier de base : { $name }
wt-new-name = Nom du nouveau calendrier
wt-new-base = Nouveau calendrier de base
wt-new-copy = Copie de « { $name } »
wt-create = Créer
wt-new-cancel = Ne pas créer
wt-prev-month = Mois précédent
wt-next-month = Mois suivant
wt-month-title = { $name } { $value }
wt-month-unavailable = Calendrier indisponible : { $detail }
wt-state-working = Ouvré : { $value }
wt-state-non-working = Chômé
wt-legend-working = Ouvré
wt-legend-non-working = Chômé
wt-legend-exception = Exception
wt-tab-exceptions = Exceptions
wt-tab-week = Semaine de travail
wt-exc-name = Nom
wt-exc-from = Début
wt-exc-to = Fin
wt-exc-working = Ouvré
wt-exc-ranges = Plages
wt-exc-recurring = Récurrente (lecture seule)
wt-exc-add = Ajouter une exception
wt-week-inherit = Hérité de la base
wt-week-non-working = Jour chômé
wt-week-working = Horaires définis
wt-week-setting = Réglage du { $name }
wt-week-ranges = Plages du { $name }
wt-week-inherited = Base : { $value }
wt-bad-ranges = Plages attendues sous la forme 08:00-12:00, 13:00-17:00
pi-title = Informations sur le projet
pi-direction = Planifier à partir de
pi-from-start = la date de début du projet
pi-from-finish = la date de fin du projet
pi-start = Date de début
pi-finish = Date de fin
pi-status = Date d'état
pi-status-hint = Vide : NA (la date du jour sert de date d'état)
pi-calendar = Calendrier
bl-title = Ligne de base
bl-set = Définir la ligne de base
bl-clear = Effacer la ligne de base
bl-number = Numéro
bl-baseline-0 = Ligne de base
bl-baseline-n = Ligne de base { $count }
bl-rollup = Cumul des lignes de base
bl-rollup-none = Aucun cumul
bl-rollup-all = Vers toutes les tâches récapitulatives
bl-rollup-from = Des sous-tâches vers les récapitulatives sélectionnées
bl-confirm = { $detail } Remplacer la ligne de base enregistrée ?
bl-replace = Remplacer
rs-title = Replanifier le travail restant
rs-after = Replanifier après le

# --- J7 L13 ---
menu-assign-resources = Affecter des ressources…
as-title = Affecter des ressources
as-task-one = Tâche : { $name }
as-task-many = { $count ->
        [one] { $count } tâche sélectionnée
       *[other] { $count } tâches sélectionnées
    }
as-no-task = Aucune tâche sélectionnée
as-filter = Filtrer par nom
as-resource = Nom de la ressource
as-units = Unités
as-units-of = Unités de { $name }
as-assigned = Affectée
as-assigned-all = oui
as-assigned-some = { $count } sur { $value }
as-assign = Affecter
as-remove = Retirer
as-update-units = Modifier les unités
as-close = Fermer
as-no-resource = Aucune ressource
as-hint = Clic sur une ressource pour la choisir (Ctrl ou Cmd + clic : plusieurs) ; unités laissées vides : valeur par défaut.

# --- J7 L14 ---
open-error-read-file = Lecture de « { $name } » impossible : { $detail }
open-error-mspdi-file = « { $name } » : fichier MSPDI illisible : { $detail }
open-error-missing = « { $name } » est introuvable, retiré des fichiers récents : { $detail }
menu-recent = Fichiers récents
menu-recent-empty = Aucun fichier récent
menu-recent-missing = { $name } (introuvable)

# --- J7 L15 ---
recovery-title = Récupérer « { $name } » ({ $count ->
        [one] { $count } modification
       *[other] { $count } modifications
    }) ?
recovery-checkpointed = Les modifications antérieures au dernier point de reprise ne pourront plus être annulées.
recovery-restore = Restaurer
recovery-discard = Ignorer
recovery-later = Plus tard
recovery-partial = Récupération partielle de « { $name } » : { $count ->
        [one] { $count } modification rejouée
       *[other] { $count } modifications rejouées
    }, la suivante est refusée : { $detail }
recovery-failed = Récupération de « { $name } » impossible : { $detail }
recovery-discard-failed = Suppression du journal de reprise impossible : { $detail }
recovery-journal-cut = Reprise après plantage interrompue pour ce document : { $detail }. L'édition continue ; enregistrez régulièrement.
recovery-file-changed = Le fichier a changé sur le disque depuis : la version récupérée s'ouvrira sans chemin (Enregistrer sous), pour ne pas l'écraser.
recovery-kept = Journal d'une autre version de planrs, ou trop grand : il ne se restaure pas ici. Ignorer le supprime.
recovered-file-name = { $stem } (récupéré)

# --- J7 L16 ---

# --- J8 L0 : codes de refus du contrat (ADR-0016) ---
error-paste-unrecognized = Texte collé non reconnu : { $detail }
error-paste-too_large = Texte collé trop grand : { $detail }
error-paste-links_dropped = Liens non recopiés : { $detail }
error-style-ignored = Mise en forme du fichier ignorée : { $detail }
error-style-invalid = Mise en forme invalide : { $detail }
error-pdf-too_many_pages = PDF trop long : { $detail }
error-pdf-glyph_missing = Caractère absent des polices du PDF : { $detail }
error-pdf-logo_invalid = Logo illisible : { $detail }
error-task-manual_finish_before_start = Fin avant le début de la tâche manuelle : { $detail }
error-control-bad_endpoint = Adresse du canal de contrôle invalide : { $detail }

# --- J8 L1 ---

# --- J8 L2 ---

# --- J8 L3 ---

# --- J8 L4 ---

# --- J8 L5 ---

# --- J8 L6 ---

# --- J8 L7 ---

# --- J8 L8 ---

# --- J8 L9 ---
menu-filter = Filtre
filter-all = Toutes les tâches
filter-critical = Tâches critiques
filter-late = Tâches en retard
filter-incomplete = Tâches inachevées
filter-milestones = Jalons
filter-summaries = Tâches récapitulatives
filter-using-resource = Utilisant la ressource…
filter-date-range = Plage de dates…
filter-custom = Filtre personnalisé…
filter-reapply = Réappliquer le filtre
filter-clear = Retirer le filtre
filter-using-named = Utilisant { $name }
status-filter = Filtre : { $name }
filter-custom-title = Filtre personnalisé
filter-resource-title = Utilisant la ressource
filter-date-range-title = Plage de dates
filter-apply = Appliquer
filter-resource = Ressource
filter-no-resource = Aucune ressource dans le projet
filter-use-resource = Affectées à la ressource
filter-from = Du
filter-to = Au
filter-active-only = Tâches actives
filter-outline-max = Niveau au plus
filter-name-contains = Nom contenant
filter-related = Afficher les récapitulatives liées
filter-date-order = Le premier jour suit le dernier.
filter-dates-required = Indiquez le premier et le dernier jour.
filter-resource-missing = Choisissez une ressource.
menu-sort = Trier…
menu-sort-by-id = Trier par ID
sort-title = Trier
sort-by = Trier par
sort-ascending = Croissant
sort-descending = Décroissant
sort-keep-outline = Conserver la structure
sort-apply = Trier les tâches
sort-reset = Réinitialiser (ordre des ID)
menu-outline-level = Niveau hiérarchique
outline-level-n = Niveau { $count }
outline-all = Tous les niveaux
menu-hide-subtasks = Masquer les sous-tâches
menu-show-subtasks = Afficher les sous-tâches
outline-expand-row = Déplier { $name }
outline-collapse-row = Replier { $name }
view-reorder-locked = Retirez le tri ou le filtre pour déplacer, hausser ou abaisser des tâches.
menu-find = Rechercher…
search-label = Rechercher
search-previous = Précédent
search-next = Suivant
search-all-columns = Colonnes visibles
search-close = Fermer la recherche
search-none = Aucun résultat
search-counter = { $id }/{ $count }
search-hidden =
    { $count ->
        [one] ({ $count } masquée par le filtre)
       *[other] ({ $count } masquées par le filtre)
    }

# --- J8 L10 ---
menu-cut = Couper
menu-copy = Copier
menu-paste = Coller
menu-paste-special = Collage spécial…
menu-import-csv = Importer CSV…
paste-report-title = Collage
paste-report-tasks = { $count ->
        [one] { $count } tâche collée
       *[other] { $count } tâches collées
    }
paste-report-links = { $count ->
        [one] { $count } lien non recopié (un bout hors de la copie)
       *[other] { $count } liens non recopiés (un bout hors de la copie)
    }
paste-report-assignments = { $count ->
        [one] { $count } affectation omise (ressource absente)
       *[other] { $count } affectations omises (ressources absentes)
    }
paste-report-resources = { $count ->
        [one] { $count } ressource créée
       *[other] { $count } ressources créées
    }
paste-hint-unrecognized = collez une ligne d'en-tête (Nom, Durée…) suivie des lignes, ou une seule colonne de noms
paste-hint-too-large = 10 000 lignes et 4 Mio au plus
paste-use-shortcut = Le navigateur ne donne pas le presse-papiers à un menu : collez avec Ctrl+V (Cmd+V sur Mac).
ps-title = Collage spécial
ps-source = { $count ->
        [one] Dernière copie : { $count } tâche
       *[other] Dernière copie : { $count } tâches
    }
ps-links = Recréer les liens entre les tâches collées
ps-resources = Affectations
ps-res-match = Par nom de ressource (omises si la ressource n'existe pas)
ps-res-create = Par nom, en créant les ressources absentes
ps-res-drop = Ne pas coller les affectations
ps-nothing = Rien à coller : copiez d'abord des lignes (Ctrl+C).
ci-pick-title = Importer un fichier CSV
ci-pick-filter = Fichiers CSV
ci-title = Importer CSV
ci-file = Fichier : { $name }
ci-separator = Séparateur
ci-encoding = Encodage
ci-language = Langue
ci-auto = Automatique
ci-sep-semicolon = Point-virgule (;)
ci-sep-comma = Virgule (,)
ci-sep-tab = Tabulation
ci-enc-utf8 = UTF-8
ci-enc-utf16le = UTF-16 LE
ci-enc-utf16be = UTF-16 BE
ci-enc-windows1252 = Windows-1252
ci-detected = Retenu : { $detail }
ci-columns = Colonnes
ci-rows = { $count ->
        [one] Aperçu ({ $count } ligne)
       *[other] Aperçu ({ $count } lignes)
    }
ci-recognized = reconnue
ci-unknown = ignorée (inconnue)
ci-unsupported = non prise en charge
ci-duplicate = ignorée (en double)
ci-diagnostics = { $count ->
        [one] { $count } remarque sur les lignes montrées
       *[other] { $count } remarques sur les lignes montrées
    }
ci-diagnostic-line = Ligne { $count } : { $detail }
ci-import = Importer
ci-imported = { $count ->
        [one] « { $name } » importé avec { $count } remarque (valeur par défaut ou colonne ignorée).
       *[other] « { $name } » importé avec { $count } remarques (valeurs par défaut ou colonnes ignorées).
    }
error-csv-error-too_large = Fichier CSV trop grand (64 Mio au plus).
error-csv-error-empty = Fichier CSV vide.
error-csv-error-no_header = Aucune ligne d'en-tête dans le fichier.
error-csv-error-no_known_column = Aucune colonne Nom ni Durée dans l'en-tête.
error-csv-error-too_many_rows = Trop de lignes (200 000 au plus).
error-csv-error-too_many_columns = Trop de colonnes (256 au plus).
error-csv-error-encoding = Le fichier n'est pas dans l'encodage indiqué.
error-csv-error-not_yet_available = Fonction pas encore disponible.

# --- J8 L11 ---
col-mode = Mode
fmt-mode-manual = Manuelle
fmt-mode-auto = Automatique
gantt-status-label = État { $date }

# --- J8 L12 ---
menu-ai-panel = Opérations IA
side-tab-ai = Opérations IA
side-tab-why = Pourquoi cette date ?
side-close = Fermer le volet
ai-no-pending = Aucune proposition d'un agent en attente.
ai-more =
    { $count ->
        [one] { $count } autre tâche modifiée
       *[other] { $count } autres tâches modifiées
    }
ai-critical-entered = devient critique
ai-critical-left = n'est plus critique
ai-added = + { $name }
ai-removed = supprimée : { $name }
ai-no-preview = Pas d'aperçu pour cette opération : { $detail }
ai-history = Historique de la session
ai-history-empty = Aucune proposition traitée pour l'instant.
ai-history-line = { $value } — { $name } sur « { $document } » : { $detail }
ai-outcome-accepted = acceptée
ai-outcome-rejected = refusée
ai-outcome-expired = expirée
ai-outcome-stale = périmée (document fermé ou remplacé)
ai-outcome-failed = échouée ({ $detail })
ai-undo-entry = Annuler cette opération
why-no-task = Sélectionnez une tâche pour voir ce qui fixe ses dates.
why-drivers = Pilotes (cliquer pour sélectionner)
why-unavailable = Explication indisponible : { $detail }

# --- J8 L13 ---
web-start-failed = { $product } ne peut pas démarrer dans ce navigateur (WebGL2 ou WebGPU indisponible ?). Essayez un navigateur récent ou la version de bureau. Détail : { $detail }
dropped-first-only = { $count } fichiers déposés : seul « { $name } » est ouvert.
menu-export = Exporter
menu-export-pdf = PDF…
menu-export-svg = SVG…
menu-export-csv = CSV des tâches…
export-done = Exporté : { $name }
export-error-title = Export impossible
export-error-failed = L'export a échoué : { $detail }

# --- J8 L14 ---

# --- J8 L15 ---

# --- J8 L16 ---
