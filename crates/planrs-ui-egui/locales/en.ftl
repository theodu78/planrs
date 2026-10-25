# Traductions anglaises de l'interface (repli sur fr.ftl pour une clé absente).
# Chaque clé correspond à une variante de `TextId` (src/i18n.rs) ; un test
# vérifie que fr.ftl et en.ftl déclarent exactement les mêmes clés.
# Pas de U+202F (absente des polices d'egui) ; points de suspension U+2026
# « … » (présents dans les polices d'egui, qui s'en servent pour tronquer).

## Écran d'accueil

welcome-title = Welcome to { $product }
welcome-subtitle = Free MS Project clone — open, edit, save
welcome-open-demo = Open the sample: house construction
welcome-open-file = Open an MSPDI file (.xml)…
welcome-new-project = New project
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
col-percent-complete = % Complete
col-resource-names = Resource Names
col-baseline-start = Baseline Start
col-baseline-finish = Baseline Finish
col-finish-variance = Finish Variance
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
menu-quit = Quit
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

## Structure (menu contextuel, confirmation de suppression)

structure-insert = Insert Task
structure-delete = Delete Task
structure-indent = Indent
structure-outdent = Outdent
structure-link = Link Tasks
structure-unlink = Unlink Tasks
confirm-delete-title = Delete a summary task
confirm-delete-text =
    { $count ->
        [one] The task will be deleted.
       *[other] The summary task and its subtasks will be deleted ({ $count } tasks).
    }
confirm-delete-yes = Delete
confirm-delete-no = Cancel

## Refus de commande (une clé par code d'erreur du moteur, `error-<code>`)

command-error-title = Change refused
error-unknown = Error: { $detail }
error-assignment-duplicate = Resource already assigned: { $detail }
error-assignment-kind_mismatch = Field does not match the resource type: { $detail }
error-assignment-not_found = Assignment not found: { $detail }
error-assignment-zero_units_fixed_work = Zero units on a fixed-work task: { $detail }
error-baseline-exists = Baseline already set: { $detail }
error-baseline-rollup_needs_selection = Roll-up needs a selection: { $detail }
error-batch-duplicate_label = Duplicate label: { $detail }
error-batch-unknown_label = Unknown label: { $detail }
error-batch-unsupported_version = Unsupported format version: { $detail }
error-batch-would_create_errors = Change would introduce errors: { $detail }
error-calendar-exception_not_found = Calendar exception not found: { $detail }
error-calendar-exception_overlap = Overlapping calendar exceptions: { $detail }
error-calendar-not_found = Calendar not found: { $detail }
error-command-not_yet_available = Command not available yet: { $detail }
error-explain-not_scheduled = Task not scheduled: { $detail }
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
error-param-conflict = Mutually exclusive values: { $detail }
error-param-invalid = Value refused: { $detail }
error-param-mismatch = Contradictory values: { $detail }
error-progress-duration_below_actual = Duration below the actual duration: { $detail }
error-progress-inactive = Inactive task: { $detail }
error-progress-invalid_order = Actual finish before actual start: { $detail }
error-progress-summary_not_editable = Progress of a summary task: { $detail }
error-resource-has_assignments = Resource has assignments: { $detail }
error-resource-not_found = Resource not found: { $detail }
error-session-none = No document: { $detail }
error-session-poisoned = Session unavailable: { $detail }
error-task-computed_field = Computed field: { $detail }
error-task-fixed_work_effort_driven = Fixed work is always effort driven: { $detail }
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

## J7 — L0: Contract refusal codes (ADR-0015)

error-calendar-name_taken = Calendar name already taken: { $detail }
error-calendar-base_not_base = Invalid base calendar: { $detail }
error-calendar-in_use = Calendar in use: { $detail }
error-gesture-not_draggable = Bar cannot be dragged this way: { $detail }
error-usage-limit = Usage report too large: { $detail }
error-control-rejected = Rejected in the application: { $detail }
error-control-approval_timeout = No answer in time: { $detail }
error-control-not_relayable = Command cannot be relayed: { $detail }
error-control-not_attached = No attached application: { $detail }
error-control-app_gone = Application closed: { $detail }
error-control-screenshot_unavailable = Screenshot unavailable: { $detail }
error-control-screenshot_too_large = Screenshot too large: { $detail }

# --- J7 L1 ---
table-label = Task table
nav-cursor-label = Row { $id }, { $column }, { $value }
nav-cursor-blank-label = Blank row, { $column }
nav-cursor-empty-value = empty

# --- J7 L2 ---

# --- J7 L3 ---
approval-title = The agent proposes on “{ $document }”: { $name }
approval-changed =
    { $count ->
        [0] no task changed
        [one] { $count } task changed
       *[other] { $count } tasks changed
    }
approval-finish = finish { $value }
approval-finish-same = finish unchanged
approval-critical = critical path changed
approval-warnings =
    { $count ->
        [one] { $count } warning
       *[other] { $count } warnings
    }
approval-queued =
    { $count ->
        [one] { $count } more proposal waiting
       *[other] { $count } more proposals waiting
    }
approval-accept = Accept
approval-reject = Reject
menu-undo-agent = Undo: agent — { $detail }
menu-undo-agent-ops =
    { $count ->
        [one] { $count } operation
       *[other] { $count } operations
    }

# --- J7 L4 ---

# --- J7 L5 ---

# --- J7 L6 ---
fmt-percent-suffix = %
column-header-label = Column { $column }
column-resize-label = Width of the { $column } column
column-menu-hide = Hide column
table-scroll-label = Task table horizontal scroll

# --- J7 L7 ---
menu-gantt-standard = Gantt Chart
menu-gantt-tracking = Tracking Gantt

# --- J7 L8 ---
gantt-reason-summary = summary task (only linking is allowed)
gantt-reason-started = task already started
gantt-reason-milestone = milestone
gantt-reason-excluded = task excluded from scheduling
gantt-reason-blank = blank row
gantt-reason-overflow = offset outside the allowed dates
gantt-reason-same-task = task linked to itself
link-dialog-title = Task Dependency
link-dialog-from = From
link-dialog-to = To
link-dialog-type = Type
link-dialog-lag = Lag
link-dialog-delete = Delete
link-dialog-stale = The project changed: the dependency was reloaded.
link-dialog-task = { $id } { $name }

# --- J7 L9 ---
task-info-menu = Task Information…
task-info-title = Task Information — { $id } { $name }
task-info-tab-general = General
task-info-tab-predecessors = Predecessors
task-info-tab-resources = Resources
task-info-tab-advanced = Advanced
task-info-tab-notes = Notes
task-info-name = Name
task-info-duration = Duration
task-info-percent = % complete
task-info-start = Start
task-info-finish = Finish
task-info-active = Active
task-info-pred-id = ID
task-info-pred-type = Type
task-info-pred-lag = Lag
task-info-row-field = { $column }, row { $count }
task-info-add-predecessor = Add predecessor
task-info-add-resource = Assign resource
task-info-remove-row = Remove
task-info-resource-name = Resource
task-info-resource-units = Units
task-info-no-resource = No resources in the project.
task-info-constraint = Constraint type
task-info-constraint-date = Constraint date
task-info-deadline = Deadline
task-info-task-type = Task type
task-info-effort-driven = Effort driven
task-info-notes = Task notes
task-info-link-fs = Finish-to-Start (FS)
task-info-link-ss = Start-to-Start (SS)
task-info-link-ff = Finish-to-Finish (FF)
task-info-link-sf = Start-to-Finish (SF)
task-info-constraint-asap = As Soon As Possible
task-info-constraint-alap = As Late As Possible
task-info-constraint-mso = Must Start On
task-info-constraint-mfo = Must Finish On
task-info-constraint-snet = Start No Earlier Than
task-info-constraint-snlt = Start No Later Than
task-info-constraint-fnet = Finish No Earlier Than
task-info-constraint-fnlt = Finish No Later Than
task-info-type-fixed-units = Fixed Units
task-info-type-fixed-duration = Fixed Duration
task-info-type-fixed-work = Fixed Work
task-info-ok = OK
task-info-cancel = Cancel
task-info-stale = The project changed since this window opened: values were reloaded, nothing was applied.
task-info-date-required = A date is required for this constraint type
task-info-bad-id = Unreadable ID: { $id }
task-info-duplicate-predecessor = Duplicate predecessor: { $id }
task-info-duplicate-resource = Resource assigned twice: { $name }
task-info-unknown-resource = Unknown resource

# --- J7 L10 ---
menu-view-resource-sheet = Resource Sheet
res-sheet-label = Resource sheet
res-col-indicator = Ind.
res-col-name = Resource Name
res-col-kind = Type
res-col-material-label = Material Label
res-col-initials = Initials
res-col-group = Group
res-col-max-units = Max. Units
res-col-standard-rate = Std. Rate
res-col-overtime-rate = Ovt. Rate
res-col-cost-per-use = Cost/Use
res-col-calendar = Calendar
res-kind-work = Work
res-kind-material = Material
res-kind-cost = Cost
rate-minute = /min
rate-hour = /hr
rate-day = /day
rate-week = /wk
rate-month = /mon
rate-year = /yr
res-row-label = Resource { $id }: { $name }
res-blank-row-label = Blank row (new resource)
res-name-first = Enter the resource name first
res-unknown-calendar = No calendar named "{ $name }"
res-calendar-none = (none)
res-delete-title = Delete an assigned resource
res-delete-text =
    { $count ->
        [one] "{ $name }" is assigned to one task: the assignment will be removed with it.
       *[other] "{ $name }" has { $count } assignments: they will be removed with it.
    }

# --- J7 L11 ---
menu-view-resource-usage = Resource Usage
usage-sheet-label = Resource usage
usage-periods-label = Work per period
usage-col-expand = +/-
usage-col-name = Resource Name
usage-col-work = Work
usage-hours-suffix = h
usage-expand = Expand { $name }
usage-collapse = Collapse { $name }
usage-assignment-label = Assignment: { $name } ({ $detail })
usage-limit = Usage too large to display: collapse some resources ({ $detail }).

# --- J7 L12 ---
menu-project = Project
menu-project-info = Project Information…
menu-working-time = Change Working Time…
menu-baseline = Baseline…
menu-reschedule = Reschedule Uncompleted Work…
scope-project = Entire project
scope-selection = { $count ->
        [one] Selected task (1)
       *[other] Selected tasks ({ $count })
    }
wt-title = Change Working Time
wt-calendar = For calendar
wt-project-calendar = { $name } (Project Calendar)
wt-resource-calendar = { $name } (resource)
wt-new = Create New Calendar…
wt-delete = Delete calendar
wt-name = Calendar name
wt-base-of = Base calendar: { $name }
wt-new-name = New calendar name
wt-new-base = Create new base calendar
wt-new-copy = Make a copy of “{ $name }”
wt-create = Create
wt-new-cancel = Do not create
wt-prev-month = Previous month
wt-next-month = Next month
wt-month-title = { $name } { $value }
wt-month-unavailable = Calendar unavailable: { $detail }
wt-state-working = Working: { $value }
wt-state-non-working = Nonworking
wt-legend-working = Working
wt-legend-non-working = Nonworking
wt-legend-exception = Exception
wt-tab-exceptions = Exceptions
wt-tab-week = Work Week
wt-exc-name = Name
wt-exc-from = Start
wt-exc-to = Finish
wt-exc-working = Working
wt-exc-ranges = Times
wt-exc-recurring = Recurring (read only)
wt-exc-add = Add exception
wt-week-inherit = Inherited from base
wt-week-non-working = Nonworking day
wt-week-working = Specific working times
wt-week-setting = Setting for { $name }
wt-week-ranges = Times for { $name }
wt-week-inherited = Base: { $value }
wt-bad-ranges = Times expected as 08:00-12:00, 13:00-17:00
pi-title = Project Information
pi-direction = Schedule from
pi-from-start = Project Start Date
pi-from-finish = Project Finish Date
pi-start = Start date
pi-finish = Finish date
pi-status = Status date
pi-status-hint = Empty: NA (today serves as status date)
pi-calendar = Calendar
bl-title = Baseline
bl-set = Set baseline
bl-clear = Clear baseline
bl-number = Number
bl-baseline-0 = Baseline
bl-baseline-n = Baseline { $count }
bl-rollup = Roll up baselines
bl-rollup-none = No rollup
bl-rollup-all = To all summary tasks
bl-rollup-from = From subtasks into selected summary tasks
bl-confirm = { $detail } Replace the saved baseline?
bl-replace = Replace
rs-title = Reschedule Uncompleted Work
rs-after = Reschedule after

# --- J7 L13 ---
menu-assign-resources = Assign Resources…
as-title = Assign Resources
as-task-one = Task: { $name }
as-task-many = { $count ->
        [one] { $count } selected task
       *[other] { $count } selected tasks
    }
as-no-task = No task selected
as-filter = Filter by name
as-resource = Resource Name
as-units = Units
as-units-of = Units of { $name }
as-assigned = Assigned
as-assigned-all = yes
as-assigned-some = { $count } of { $value }
as-assign = Assign
as-remove = Remove
as-update-units = Change Units
as-close = Close
as-no-resource = No resource
as-hint = Click a resource to choose it (Ctrl or Cmd + click: several); units left empty: default value.

# --- J7 L14 ---
open-error-read-file = Could not read “{ $name }”: { $detail }
open-error-mspdi-file = “{ $name }”: unreadable MSPDI file: { $detail }
open-error-missing = “{ $name }” was not found and was removed from recent files: { $detail }
menu-recent = Recent files
menu-recent-empty = No recent files
menu-recent-missing = { $name } (not found)

# --- J7 L15 ---
recovery-title = Recover “{ $name }” ({ $count ->
        [one] { $count } change
       *[other] { $count } changes
    })?
recovery-checkpointed = Changes made before the last recovery point can no longer be undone.
recovery-restore = Restore
recovery-discard = Discard
recovery-later = Later
recovery-partial = “{ $name }” partly recovered: { $count ->
        [one] { $count } change replayed
       *[other] { $count } changes replayed
    }, the next one was refused: { $detail }
recovery-failed = Cannot recover “{ $name }”: { $detail }
recovery-discard-failed = Cannot delete the recovery journal: { $detail }
recovery-journal-cut = Crash recovery stopped for this document: { $detail }. Editing continues; save often.

# --- J7 L16 ---
