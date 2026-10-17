# Traductions anglaises de l'interface (repli sur fr.ftl pour une clé absente).
# Chaque clé correspond à une variante de `TextId` (src/i18n.rs) ; un test
# vérifie que fr.ftl et en.ftl déclarent exactement les mêmes clés.
# Ni U+202F ni points de suspension U+2026 (glyphes absents des polices d'egui).

## Écran d'accueil

welcome-title = Welcome to { $product }
welcome-subtitle = Free MS Project clone — read-only for now
welcome-open-demo = Open the sample: house construction
welcome-open-file = Open an MSPDI file (.xml)...
welcome-web-note = The file stays in your browser; nothing is uploaded.
welcome-language = Language
welcome-theme = Theme

## Menus

menu-file = File
menu-open = Open...
menu-open-demo = Open the sample
menu-close = Close
menu-view = View
menu-zoom = Zoom
menu-theme = Theme
menu-language = Language

## Barre d'outils et zoom

toolbar-fit = Fit
zoom-day = Day
zoom-week = Week
zoom-month = Month
zoom-quarter = Quarter

## Thème et langue (noms de langue dans leur propre langue)

theme-system = System
theme-light = Light
theme-dark = Dark
lang-system = System
lang-fr = Français
lang-en = English

## Colonnes de la table

col-id = ID
col-name = Task Name
col-duration = Duration
col-start = Start
col-finish = Finish
col-predecessors = Predecessors
col-total-slack = Total Slack
col-critical = Critical
yes = Yes
no = No

## Unités de durée (forme longue, hypothèse H-UI-04 : « 5 days », « 1 day? »)

unit-minute-one = minute
unit-minute-other = minutes
unit-hour-one = hour
unit-hour-other = hours
unit-day-one = day
unit-day-other = days
unit-week-one = week
unit-week-other = weeks
unit-month-one = month
unit-month-other = months
unit-minute-elapsed-one = elapsed minute
unit-minute-elapsed-other = elapsed minutes
unit-hour-elapsed-one = elapsed hour
unit-hour-elapsed-other = elapsed hours
unit-day-elapsed-one = elapsed day
unit-day-elapsed-other = elapsed days
unit-week-elapsed-one = elapsed week
unit-week-elapsed-other = elapsed weeks
unit-month-elapsed-one = elapsed month
unit-month-elapsed-other = elapsed months

## Types de liens

link-fs = FS
link-ss = SS
link-ff = FF
link-sf = SF

## Conventions de formatage (lues par Texts::format_labels) ; le point est
## écrit en littéral fluent pour ne pas commencer une valeur par « . ».

fmt-decimal-sep = {"."}
fmt-list-sep = ,
fmt-date-order = mdy
fmt-unknown = ?

## Mois

month-1-long = January
month-1-short = Jan
month-2-long = February
month-2-short = Feb
month-3-long = March
month-3-short = Mar
month-4-long = April
month-4-short = Apr
month-5-long = May
month-5-short = May
month-6-long = June
month-6-short = Jun
month-7-long = July
month-7-short = Jul
month-8-long = August
month-8-short = Aug
month-9-long = September
month-9-short = Sep
month-10-long = October
month-10-short = Oct
month-11-long = November
month-11-short = Nov
month-12-long = December
month-12-short = Dec

## Jours de la semaine

weekday-mon-short = Mon
weekday-mon-initial = M
weekday-tue-short = Tue
weekday-tue-initial = T
weekday-wed-short = Wed
weekday-wed-initial = W
weekday-thu-short = Thu
weekday-thu-initial = T
weekday-fri-short = Fri
weekday-fri-initial = F
weekday-sat-short = Sat
weekday-sat-initial = S
weekday-sun-short = Sun
weekday-sun-initial = S

## Échelle de temps

tier-quarter = Q

## Barre d'état

status-tasks =
    { $count ->
        [one] { $count } task
       *[other] { $count } tasks
    }
status-range = { $start } – { $finish }
status-errors =
    { $count ->
        [0] no errors
        [one] { $count } error
       *[other] { $count } errors
    }
status-warnings =
    { $count ->
        [0] no warnings
        [one] { $count } warning
       *[other] { $count } warnings
    }
status-no-document = No document open

## Ouverture de fichiers

open-in-progress = Opening...
open-error-title = Could not open the file
open-error-read = Could not read the file: { $detail }
open-error-mspdi = Unreadable MSPDI file: { $detail }

## Accessibilité et divers

row-label = Row { $id }: { $name }
splitter-label = Splitter between the table and the Gantt chart
gantt-scroll-label = Gantt chart horizontal scroll
demo-file-name = Sample: house construction
file-filter-mspdi = MSPDI project (XML)
dialog-open-title = Open an MSPDI project
state-load-warning = Display preferences ignored: { $detail }
