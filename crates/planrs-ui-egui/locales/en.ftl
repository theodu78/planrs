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
error-control-busy = Application busy: { $detail }
error-control-document_changed = Application document changed: { $detail }

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
recovery-file-changed = The file changed on disk since then: the recovered version opens without a path (Save As), so it cannot overwrite it.
recovery-kept = Journal from another planrs version, or too large: it cannot be restored here. Discard deletes it.
recovered-file-name = { $stem } (recovered)

# --- J7 L16 ---

# --- J8 L0 : codes de refus du contrat (ADR-0016) ---
error-paste-unrecognized = Pasted text not recognized: { $detail }
error-paste-too_large = Pasted text too large: { $detail }
error-paste-links_dropped = Links not copied: { $detail }
error-style-ignored = File formatting ignored: { $detail }
error-style-invalid = Invalid formatting: { $detail }
error-pdf-too_many_pages = PDF too long: { $detail }
error-pdf-glyph_missing = Character missing from the PDF fonts: { $detail }
error-pdf-logo_invalid = Unreadable logo: { $detail }
error-task-manual_finish_before_start = Manual task finish before its start: { $detail }
error-control-bad_endpoint = Invalid control channel address: { $detail }

# --- J8 L1 ---

# --- J8 L2 ---

# --- J8 L3 ---

# --- J8 L4 ---

# --- J8 L5 ---

# --- J8 L6 ---

# --- J8 L7 ---

# --- J8 L8 ---

# --- J8 L9 ---
menu-filter = Filter
filter-all = All tasks
filter-critical = Critical tasks
filter-late = Late tasks
filter-incomplete = Incomplete tasks
filter-milestones = Milestones
filter-summaries = Summary tasks
filter-using-resource = Using resource…
filter-date-range = Date range…
filter-custom = Custom filter…
filter-reapply = Reapply filter
filter-clear = Clear filter
filter-using-named = Using { $name }
status-filter = Filter: { $name }
filter-custom-title = Custom filter
filter-resource-title = Using resource
filter-date-range-title = Date range
filter-apply = Apply
filter-resource = Resource
filter-no-resource = No resource in the project
filter-use-resource = Assigned to the resource
filter-from = From
filter-to = To
filter-active-only = Active tasks
filter-outline-max = Outline level at most
filter-name-contains = Name containing
filter-related = Show related summary rows
filter-date-order = The first day is after the last one.
filter-dates-required = Enter the first and the last day.
filter-resource-missing = Choose a resource.
menu-sort = Sort…
menu-sort-by-id = Sort by ID
sort-title = Sort
sort-by = Sort by
sort-ascending = Ascending
sort-descending = Descending
sort-keep-outline = Keep outline structure
sort-apply = Sort tasks
sort-reset = Reset (ID order)
menu-outline-level = Outline level
outline-level-n = Level { $count }
outline-all = All levels
menu-hide-subtasks = Hide subtasks
menu-show-subtasks = Show subtasks
outline-expand-row = Expand { $name }
outline-collapse-row = Collapse { $name }
view-reorder-locked = Remove the sort or the filter to move, indent or outdent tasks.
menu-find = Find…
search-label = Find
search-previous = Previous
search-next = Next
search-all-columns = Visible columns
search-close = Close search
search-none = No match
search-counter = { $id }/{ $count }
search-hidden =
    { $count ->
        [one] ({ $count } hidden by the filter)
       *[other] ({ $count } hidden by the filter)
    }

# --- J8 L10 ---
menu-cut = Cut
menu-copy = Copy
menu-paste = Paste
menu-paste-special = Paste Special…
menu-import-csv = Import CSV…
paste-report-title = Paste
paste-report-tasks = { $count ->
        [one] { $count } task pasted
       *[other] { $count } tasks pasted
    }
paste-report-links = { $count ->
        [one] { $count } link not copied (one end outside the copy)
       *[other] { $count } links not copied (one end outside the copy)
    }
paste-report-assignments = { $count ->
        [one] { $count } assignment skipped (missing resource)
       *[other] { $count } assignments skipped (missing resources)
    }
paste-report-resources = { $count ->
        [one] { $count } resource created
       *[other] { $count } resources created
    }
paste-hint-unrecognized = paste a header row (Name, Duration…) followed by the rows, or a single column of names
paste-hint-too-large = 10,000 lines and 4 MiB at most
paste-use-shortcut = The browser does not give the clipboard to a menu: paste with Ctrl+V (Cmd+V on Mac).
ps-title = Paste Special
ps-source = { $count ->
        [one] Last copy: { $count } task
       *[other] Last copy: { $count } tasks
    }
ps-links = Recreate the links between the pasted tasks
ps-resources = Assignments
ps-res-match = By resource name (skipped if the resource does not exist)
ps-res-create = By name, creating missing resources
ps-res-drop = Do not paste assignments
ps-nothing = Nothing to paste: copy rows first (Ctrl+C).
ci-pick-title = Import a CSV file
ci-pick-filter = CSV files
ci-title = Import CSV
ci-file = File: { $name }
ci-separator = Separator
ci-encoding = Encoding
ci-language = Language
ci-auto = Automatic
ci-sep-semicolon = Semicolon (;)
ci-sep-comma = Comma (,)
ci-sep-tab = Tab
ci-enc-utf8 = UTF-8
ci-enc-utf16le = UTF-16 LE
ci-enc-utf16be = UTF-16 BE
ci-enc-windows1252 = Windows-1252
ci-detected = Using: { $detail }
ci-columns = Columns
ci-rows = { $count ->
        [one] Preview ({ $count } row)
       *[other] Preview ({ $count } rows)
    }
ci-recognized = recognized
ci-unknown = ignored (unknown)
ci-unsupported = not supported
ci-duplicate = ignored (duplicate)
ci-diagnostics = { $count ->
        [one] { $count } remark on the rows shown
       *[other] { $count } remarks on the rows shown
    }
ci-diagnostic-line = Line { $count }: { $detail }
ci-import = Import
ci-imported = { $count ->
        [one] “{ $name }” imported with { $count } remark (default value or ignored column).
       *[other] “{ $name }” imported with { $count } remarks (default values or ignored columns).
    }
error-csv-error-too_large = CSV file too large (64 MiB at most).
error-csv-error-empty = Empty CSV file.
error-csv-error-no_header = No header row in the file.
error-csv-error-no_known_column = No Name or Duration column in the header.
error-csv-error-too_many_rows = Too many rows (200,000 at most).
error-csv-error-too_many_columns = Too many columns (256 at most).
error-csv-error-encoding = The file is not in the selected encoding.
error-csv-error-not_yet_available = Feature not available yet.

# --- J8 L11 ---
col-mode = Task Mode
fmt-mode-manual = Manually Scheduled
fmt-mode-auto = Auto Scheduled
gantt-status-label = Status { $date }

# --- J8 L12 ---
menu-ai-panel = AI operations
side-tab-ai = AI operations
side-tab-why = Why this date?
side-close = Close the panel
ai-no-pending = No pending proposal from an agent.
ai-more =
    { $count ->
        [one] { $count } more changed task
       *[other] { $count } more changed tasks
    }
ai-critical-entered = becomes critical
ai-critical-left = no longer critical
ai-added = + { $name }
ai-removed = deleted: { $name }
ai-no-preview = No preview for this operation: { $detail }
ai-history = Session history
ai-history-empty = No proposal handled yet.
ai-history-line = { $value } — { $name } on "{ $document }": { $detail }
ai-outcome-accepted = accepted
ai-outcome-rejected = rejected
ai-outcome-expired = expired
ai-outcome-stale = stale (document closed or replaced)
ai-outcome-failed = failed ({ $detail })
ai-undo-entry = Undo this operation
why-no-task = Select a task to see what sets its dates.
why-drivers = Drivers (click to select)
why-unavailable = No explanation: { $detail }

# --- J8 L13 ---
web-start-failed = { $product } cannot start in this browser (WebGL2 or WebGPU unavailable?). Try a recent browser or the desktop version. Detail: { $detail }
dropped-first-only = { $count } files dropped: only "{ $name }" is opened.
menu-export = Export
menu-export-pdf = PDF…
menu-export-svg = SVG…
menu-export-csv = Tasks CSV…
export-done = Exported: { $name }
export-error-title = Cannot export
export-error-failed = The export failed: { $detail }

# --- J8 L14 ---
diag-calendar-unknown = unknown calendar { $calendar }, referenced by { $referenced_by }
diag-calendar-base_missing = calendar { $calendar } inherits from calendar { $base }, which does not exist
diag-calendar-base_not_base = calendar { $calendar } inherits from calendar { $base }, which is not a base calendar
diag-calendar-inheritance_cycle = inheritance loop between calendars: { $chain }
diag-calendar-no_working_time = calendar { $calendar } has no working time: no duration can be placed on it
diag-calendar-internal = calendar { $calendar } could not be compiled (internal inconsistency, please report)
diag-calendar-exceptions_overlap = calendar { $calendar }: exceptions #{ $kept } and #{ $ignored } overlap; #{ $kept } wins on the shared days
diag-calendar-work_weeks_overlap = calendar { $calendar }: work weeks #{ $kept } and #{ $ignored } overlap; #{ $kept } wins on the shared days
diag-calendar-recurrence_unsupported = calendar { $calendar }: the recurrence of exception #{ $exception } is not supported; the exception is ignored
diag-project-calendar_not_base = the project calendar ({ $calendar }) is not a base calendar
diag-task-reserved_uid = a task has UID 0, reserved for the project summary task
diag-task-outline_jump = task { $task } skips an outline level (or the first level is not 1)
diag-task-null_with_data = blank row { $task } carries links or assignments, ignored
diag-task-negative_progress = task { $task } has a negative actual or remaining duration
diag-link-self = task { $task } is linked to itself
diag-link-duplicate = several links from task { $predecessor } to task { $successor }
diag-link-unknown_task = task { $successor } is linked to task { $predecessor }, which does not exist
diag-assignment-unknown_task = assignment { $assignment } refers to task { $task }, which does not exist
diag-assignment-unknown_resource = assignment { $assignment } refers to resource { $resource }, which does not exist
diag-conversion-out_of_domain = calendar { $calendar }: { $minutes } min from { $from } fall outside the date range (1984-2149)
diag-link-cycle = circular links between tasks { $tasks }
diag-link-ignored_in_cycle = the link { $predecessor } -> { $successor } is ignored to break a cycle
diag-link-summary_descendant = the link { $predecessor } -> { $successor } joins a summary task to one of its subtasks: ignored
diag-link-finish_to_summary = the link { $predecessor } -> { $successor } (finish-to-finish or start-to-finish) to a summary task applies to each of its subtasks
diag-link-expansion_limit = { $ignored ->
        [one] { $ignored } finish-to-finish or start-to-finish link to a summary task ignored
       *[other] { $ignored } finish-to-finish or start-to-finish links to a summary task ignored
    }: expanding them to the subtasks would exceed { $limit } arcs
diag-link-start_from_summary = the link { $predecessor } -> { $successor } (start-to-start or start-to-finish) from a summary task caps the start of each of its subtasks (scheduled from finish)
diag-link-inactive = the link { $predecessor } -> { $successor } starts from an inactive task: no effect
diag-link-excluded_endpoint = the link { $predecessor } -> { $successor } touches an unscheduled task: ignored
diag-link-manual_not_honored = manual task { $successor } does not honor its link with task { $predecessor }
diag-link-unschedulable = the link { $predecessor } -> { $successor } to a summary task cannot be applied ({ $cause }): ignored
diag-task-manual_undated = manual task { $task } has neither start nor finish: not scheduled
diag-task-manual_dates_inverted = manual task { $task } finishes before it starts: finish moved to the start
diag-task-calendar_fallback = calendar { $calendar } of task { $task } is unusable: project calendar used
diag-project-calendar_fallback = the project calendar ({ $calendar }) is unusable: built-in Standard calendar used
diag-task-unschedulable = task { $task } cannot be scheduled ({ $pass }: { $cause })
diag-constraint-override = the "{ $kind }" constraint of task { $task } is honored despite { $bound }
diag-constraint-not_honored = the "{ $kind }" constraint on { $date } of task { $task } is not honored: links take precedence
diag-constraint-summary_ignored = the "{ $kind }" constraint of summary task { $task } is ignored
diag-constraint-manual_ignored = the "{ $kind }" constraint of manual task { $task } is ignored by scheduling
diag-task-deadline_missed = task { $task } finishes on { $finish }, after its deadline of { $deadline }
diag-resource-overallocated = resource { $resource } is overallocated from { $from } to { $to } (peak { $peak }%)
diag-resource-availability_ignored = dated availability of resource { $resource } is ignored: constant capacity
diag-assignment-contour_unsupported = contour { $contour } of assignment { $assignment } is placed as flat
diag-assignment-zero_units = assignment { $assignment } has zero units: the task duration is kept
diag-assignment-duration_out_of_domain = the duration of assignment { $assignment } is out of range
diag-assignment-calendar_empty = calendars { $task_calendar } (task) and { $resource_calendar } (resource) of assignment { $assignment } share no working time
diag-task-fixed_work_effort_driven = task { $task } is fixed work without being effort driven
diag-calendar-too_many_segments = the combination of calendars { $calendar } and { $other } exceeds { $limit } segments
diag-assignment-too_many_parts = task { $task } has { $parts } work assignments (more than { $limit }): its duration is that of the longest
diag-progress-link_ignored = the link { $predecessor } -> { $successor } has no effect: in-progress task { $successor } is not split
diag-progress-link_overridden = the actual dates of completed task { $successor } override its link with task { $predecessor }
diag-progress-option_unsupported = progress option { $option } is not followed: default values used
diag-progress-from_finish_in_progress = in-progress task { $task } in a project scheduled from finish: its remaining work is placed as soon as possible
diag-progress-actual_start_missing = started task { $task } has no actual start: its scheduled start is used
diag-progress-milestone_rounded = milestone { $task } is set to 100% complete
diag-constraint-ignored_in_progress = the "{ $kind }" constraint of in-progress task { $task } is ignored for its dates
diag-baseline-cost_not_computed = baseline { $number }: costs copied as is, not recalculated
diag-baseline-task_skipped = { $count ->
        [one] { $count } unscheduled task without a baseline
       *[other] { $count } unscheduled tasks without a baseline
    }
diag-baseline-nothing_to_clear = baseline { $number }: nothing to clear
diag-schedule-internal = internal scheduling inconsistency (please report): { $detail }
diag-mspdi-invalid_value = unreadable or out-of-range value: default applied
diag-mspdi-unknown_code = unknown code ({ $code }): default applied
diag-mspdi-missing_required = required element missing
diag-mspdi-duplicate = duplicate element: the first one is kept
diag-mspdi-dangling_reference = reference to a missing UID
diag-mspdi-link_ignored = link ignored ({ $reason })
diag-mspdi-rounded = value rounded to the minute
diag-mspdi-unsupported = unsupported feature ({ $what })
diag-mspdi-inconsistent = tolerated inconsistency
diag-mspdi-truncated = { $dropped ->
        [one] { $dropped } diagnostic dropped beyond the limit
       *[other] { $dropped } diagnostics dropped beyond the limit
    }
diag-mspdi-not_preserved = content not read, lost on save
diag-mspdi-honor_constraints_off = "Honor constraints" turned off: links take precedence over constraints
diag-mspdi-progress_inconsistent = { $count ->
        [one] { $count } remaining value read from the file replaced by the calculation
       *[other] { $count } remaining values read from the file replaced by the calculation
    }
diag-style-ignored = document style ignored ({ $reason }): default template applied
diag-mspdi-error-empty = empty document
diag-mspdi-error-too_large = document too large: { $size } bytes (limit { $max })
diag-mspdi-error-utf16_not_supported = UTF-16 document: only UTF-8 is supported
diag-mspdi-error-invalid_utf8 = invalid UTF-8 at { $at }
diag-mspdi-error-xml = malformed XML at { $at }: { $message }
diag-mspdi-error-unexpected_eof = unexpected end of document at { $at }{ $depth ->
        [0] {""}
       *[other] {" "}(open elements: { $open })
    }
diag-mspdi-error-doctype_not_allowed = DOCTYPE declaration refused at { $at }
diag-mspdi-error-too_deep = nesting too deep at { $at } (limit { $max } levels)
diag-mspdi-error-too_many_elements = too many elements (limit { $max })
diag-mspdi-error-too_many_retained = too many retained elements at { $at } (limit { $max })
diag-mspdi-error-text_too_long = text too long at { $at } (limit { $max } bytes)
diag-mspdi-error-not_mspdi = not an MSPDI document: root "{ $root }" instead of "Project"
diag-location = line { $line }, column { $column } (byte { $byte })
diag-subject-project = the project
diag-subject-calendar = calendar { $id }
diag-subject-task = task { $id }
diag-subject-link = link { $predecessor } -> { $successor }
diag-subject-resource = resource { $id }
diag-subject-assignment = assignment { $id }
diag-pass-forward = forward pass
diag-pass-backward = backward pass
diag-bound-project-start = the project start
diag-bound-project-finish = the project finish
diag-bound-summary-gate = the bound of summary task { $summary }
diag-bound-summary-cap = the cap of summary task { $summary }
diag-bound-predecessor = the link from task { $predecessor }
diag-bound-successor = the link to task { $successor }
diag-with-element = { $head }: { $element }
diag-with-value = { $head } = "{ $value }"
diag-with-detail = { $head }, { $detail }
template-classique = Classic
template-sobre = Clean
template-contraste-eleve = High contrast
template-presentation = Presentation
template-chantier = Works schedule
template-impression-nb = Black and white print
token-project = Project name
token-status-date = Status date
token-page = Page number
token-pages = Page count
token-print-date = Print date
token-file = File name

# --- J8 L15 ---
menu-task = Task
menu-format = Format
menu-help = Help
menu-mark-percent = Mark { $percent }% complete
toolbar-percent = { $percent }%
toolbar-percent-hint = Mark the selection { $percent }% complete
menu-toggle-mode = Toggle manual / auto scheduling
toolbar-mode = Mode
toolbar-mode-hint = Switch the selection between manual and auto scheduling
menu-toggle-active = Activate / inactivate
toolbar-active = Active
toolbar-active-hint = Activate or inactivate the selection
toolbar-filter = Custom filter
toolbar-search = Find
toolbar-print-mode = Print mode
menu-page-setup = Page Setup…
menu-print-mode = Print mode
menu-gallery = Layout templates…
menu-format-bars = Format bars…
menu-chart-only = Chart only
menu-highlight-critical = Highlight the critical path
menu-about = About { $product }
about-text = { $product } { $version } — free software (MIT or Apache-2.0)
context-format-bars = Format bars of this type
task-cmd-nothing = Nothing to change in the selection.
task-cmd-no-selection = Select one or more tasks first.
gallery-title = Layout templates
gallery-apply = Apply
gallery-current = Current template
gallery-default = Default
gallery-make-default = Set as default
gallery-default-hint = Template of documents without a chosen layout
gallery-user-templates = Personal templates
gallery-no-user-template = No personal template.
gallery-save-name = Template name
gallery-save = Save as personal template
gallery-saved = Personal template “{ $name }” saved.
gallery-name-invalid = A name of 1 to 64 characters is expected.
gallery-full = 32 personal templates at most: replace one (same name).
gallery-close = Close
gallery-preview-empty = No preview
page-setup-title = Page Setup
print-mode-title = Print mode
ps-paper = Paper
ps-landscape = Landscape
ps-portrait = Portrait
ps-margins = Margins (mm)
ps-margin-top = Top
ps-margin-right = Right
ps-margin-bottom = Bottom
ps-margin-left = Left
ps-fit = Fit
ps-fit-scale = Scale (%)
ps-fit-wide = Pages wide
ps-fit-wide-tall = Pages wide and tall
ps-pages-tall = Pages tall
ps-repeat-table = Table repeated on every page
ps-header = Header
ps-footer = Footer
ps-zone-left = Left
ps-zone-center = Center
ps-zone-right = Right
ps-insert-token = Insert into the header center
ps-title-block = Title block: first center line as a title
ps-columns = Printed columns
ps-chart-only = Chart only (table hidden)
ps-preview = Preview
ps-page-of = Page { $page } of { $pages }
ps-prev = Previous page
ps-next = Next page
ps-scale = Scale: { $scale }%
ps-export-pdf = Export to PDF…
ps-export-svg = Export to SVG…
ps-save = Save with the project
ps-close = Close
ps-error = Page layout failed: { $detail }
ps-column-name = Name
fp-title = Format: { $role }
fp-role = Bar type
fp-role-task = Task bars
fp-role-critical = Critical bars
fp-role-summary = Summary bars
fp-role-summary-critical = Critical summary bars
fp-role-milestone = Milestones
fp-role-milestone-critical = Critical milestones
fp-role-progress = Progress
fp-role-baseline = Baseline
fp-role-inactive = Inactive tasks
fp-role-manual = Manually scheduled tasks
fp-role-slack = Slack
fp-role-deadline = Deadline
fp-color = Color
fp-custom-color = Custom color
fp-swatch = Color { $color }
fp-phase-swatch = Phase color { $color }
fp-thickness = Thickness (% of the row)
fp-ends = Ends
fp-ends-square = Square
fp-ends-rounded = Rounded
fp-text-left = Text on the left
fp-text-right = Text on the right
fp-text-inside = Text inside
fp-none = None
fp-duration-after-name = Duration after the name
fp-details = Details
fp-contrast-low = Contrast too low on the background: { $ratio }:1 (3:1 at least).
fp-use-suggestion = Use { $color }
fp-revert = Revert to template
fp-save-template = Save as template…
fp-close = Close
fp-phase = Phase color of “{ $name }”
fp-phase-children = Applied to subtasks
fp-phase-remove = Remove the color
field-name = Name
field-name-duration = Name (duration)
field-resources = Resources
field-start = Start
field-finish = Finish
field-start-short = Start (short)
field-finish-short = Finish (short)
field-duration = Duration
field-duration-short = Duration (short)
field-percent = % complete
field-total-slack = Total slack
field-start-finish = Start – finish

# --- J8 L16 ---
