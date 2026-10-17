# Traductions françaises de l'interface (langue de repli).
# Chaque clé correspond à une variante de `TextId` (src/i18n.rs) ; un test
# vérifie que fr.ftl et en.ftl déclarent exactement les mêmes clés.
# Typographie : espace insécable U+00A0 avant « : ; ? ! », jamais U+202F
# (absente des polices d'egui) ; points de suspension U+2026 « … » (présents
# dans les polices d'egui, qui s'en servent pour tronquer).

## Écran d'accueil

welcome-title = Bienvenue dans { $product }
welcome-subtitle = Clone libre de MS Project — lecture seule pour l'instant
welcome-open-demo = Ouvrir l'exemple : construction d'une maison
welcome-open-file = Ouvrir un fichier MSPDI (.xml)…
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

open-in-progress = Ouverture en cours…
open-error-title = Impossible d'ouvrir le fichier
open-error-read = Lecture du fichier impossible : { $detail }
open-error-mspdi = Fichier MSPDI illisible : { $detail }

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
