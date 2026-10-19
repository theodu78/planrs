# Traductions anglaises de l'interface (repli sur fr.ftl pour une clé absente).
# Chaque clé correspond à une variante de `TextId` (src/i18n.rs) ; un test
# vérifie que fr.ftl et en.ftl déclarent exactement les mêmes clés.
# Pas de U+202F (absente des polices d'egui) ; points de suspension U+2026
# « … » (présents dans les polices d'egui, qui s'en servent pour tronquer).

## Écran d'accueil

welcome-title = Welcome to { $product }
welcome-subtitle = Free MS Project clone — read-only for now
welcome-open-demo = Open the sample: house construction
welcome-open-file = Open an MSPDI file (.xml)…
welcome-web-note = The file stays in your browser; nothing is uploaded.
welcome-language = Language
welcome-theme = Theme

## Menus

menu-file = File
menu-open = Open an MSPDI file (.xml)…
menu-open-demo = Open the sample
menu-close = Close
menu-view = View
menu-zoom = Zoom
menu-theme = Theme
menu-language = Language
menu-diagnostics = Diagnostics

## Barre d'outils et zoom

toolbar-open = Open…
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

## Unités de durée (forme longue, hypothèse H-UI-04 : « 5 days », « 1 day? » ;
## écoulé avec le préfixe « e » de MS Project : « 5 edays », ADR-0011 décision 9)

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
unit-minute-elapsed-one = eminute
unit-minute-elapsed-other = eminutes
unit-hour-elapsed-one = ehour
unit-hour-elapsed-other = ehours
unit-day-elapsed-one = eday
unit-day-elapsed-other = edays
unit-week-elapsed-one = eweek
unit-week-elapsed-other = eweeks
unit-month-elapsed-one = emonth
unit-month-elapsed-other = emonths

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
# Règle de pluriel des unités : below-two (singulier si |v| < 2) ou
# exactly-one (singulier si |v| = 1).
fmt-plural-rule = exactly-one

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
tier-week = W

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

open-in-progress = Opening…
open-error-title = Could not open the file
open-error-read = Could not read the file: { $detail }
open-error-mspdi = Unreadable MSPDI file: { $detail }

## Nouveau, Enregistrer, Enregistrer sous (lot L13)

menu-new = New
menu-save = Save
menu-save-as = Save As…
new-project-name = Project1
dialog-save-title = Save the project (MSPDI)
save-in-progress = A save is already in progress.
save-error-title = Could not save the file
save-error-write = Cannot write the file: { $detail }
save-adjusted =
    { $count ->
        [one] Saved: { $count } value adjusted to the MSPDI format.
       *[other] Saved: { $count } values adjusted to the MSPDI format.
    }
confirm-unsaved-title = Save changes to “{ $name }”?
confirm-save = Save
confirm-discard = Don't Save
confirm-cancel = Cancel

## Bandeau de messages

message-dismiss = Dismiss
range-truncated = Display range limited to 20 years

## Accessibilité et divers

row-label = Row { $id }: { $name }
splitter-label = Splitter between the table and the Gantt chart
gantt-scroll-label = Gantt chart horizontal scroll
demo-file-name = Sample: house construction
file-filter-mspdi = MSPDI file (XML)
dialog-open-title = Open an MSPDI file
state-load-warning = Display preferences ignored: { $detail }

## Menu Édition

menu-edit = Edit
menu-undo = Undo
menu-redo = Redo

## Refus de commande (une clé par code d'erreur du moteur, `error-<code>`)

command-error-title = Change refused
error-unknown = Error: { $detail }
error-batch-duplicate_label = Duplicate label: { $detail }
error-batch-unknown_label = Unknown label: { $detail }
error-batch-unsupported_version = Unsupported format version: { $detail }
error-batch-would_create_errors = Change would introduce errors: { $detail }
error-calendar-exception_not_found = Calendar exception not found: { $detail }
error-calendar-exception_overlap = Overlapping calendar exceptions: { $detail }
error-calendar-not_found = Calendar not found: { $detail }
error-command-not_yet_available = Command not available yet: { $detail }
error-file-invalid = Unreadable file: { $detail }
error-file-open_in_app = File open in the application: { $detail }
error-file-outside_root = Path not allowed: { $detail }
error-file-read = Read failed: { $detail }
error-file-too_large = File too large: { $detail }
error-file-write = Write failed: { $detail }
error-history-empty = Nothing to undo or redo: { $detail }
error-history-inconsistent = Inconsistent history: { $detail }
error-limit-exceeded = Limit exceeded: { $detail }
error-link-duplicate = Link already exists: { $detail }
error-link-not_found = Link not found: { $detail }
error-link-self = Task linked to itself: { $detail }
error-link-summary_descendant = Link between a summary task and its subtask: { $detail }
error-param-invalid = Value refused: { $detail }
error-param-mismatch = Contradictory values: { $detail }
error-session-none = No document: { $detail }
error-session-poisoned = Session unavailable: { $detail }
error-task-computed_field = Computed field: { $detail }
error-task-has_children = Summary task with subtasks: { $detail }
error-task-manual_unsupported = Manually scheduled task: { $detail }
error-task-move_into_subtree = Move not possible: { $detail }
error-task-not_found = Task not found: { $detail }
error-task-outline_refused = Outline change refused: { $detail }
error-text-bad_date = Unreadable date: { $detail }
error-text-bad_link_type = Unknown link type: { $detail }
error-text-bad_number = Unreadable number: { $detail }
error-text-empty = Empty entry: { $detail }
error-text-external_predecessor = External predecessor: { $detail }
error-text-negative = Negative value: { $detail }
error-text-out_of_range = Value out of range: { $detail }
error-text-too_long = Entry too long: { $detail }
error-text-unknown_unit = Unknown unit: { $detail }

## Cell editing (lot L12)

blank-row-label = Blank row (new task)
cell-edit-label = Edit: { $column }
edit-error-unknown-id = No task with ID { $id }
edit-error-not-editable = Computed column, read-only
