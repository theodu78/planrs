//! Traductions : identifiants typés ([`TextId`]), fichiers fluent embarqués
//! et choix de la langue.
//!
//! Le code ne passe jamais une clé libre : seulement un [`TextId`], généré
//! avec sa clé par la macro `text_ids!`. Un test vérifie que les deux
//! fichiers `locales/fr.ftl` et `locales/en.ftl` déclarent exactement les clés
//! de [`TextId::ALL`]. Repli : anglais → français → clé brute (journalisée,
//! jamais de panique). L'isolation bidirectionnelle de fluent est coupée
//! (`set_use_isolating(false)`) : ses caractères U+2068/U+2069 manquent aux
//! polices d'egui. Les nombres décimaux et les dates sont pré-formatés par
//! l'interface : fluent ne reçoit que des chaînes et des entiers.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeSet;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::{LanguageIdentifier, langid};

use crate::format::{DateOrder, FormatLabels, PluralRule, UnitLabel, UnitLabels};
use crate::state::LanguagePref;

/// Source fluent française (langue de repli).
const FTL_FR: &str = include_str!("../locales/fr.ftl");
/// Source fluent anglaise.
const FTL_EN: &str = include_str!("../locales/en.ftl");

/// Langue d'affichage résolue (non sérialisée : l'état persiste la
/// préférence, `LanguagePref`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Language {
    /// Français (langue de repli).
    Fr,
    /// Anglais.
    En,
}

impl Language {
    /// Toutes les langues livrées.
    pub const ALL: [Self; 2] = [Self::Fr, Self::En];

    /// Identifiant de langue fluent.
    #[must_use]
    pub fn langid(self) -> LanguageIdentifier {
        match self {
            Self::Fr => langid!("fr-FR"),
            Self::En => langid!("en-US"),
        }
    }

    /// Langue d'une étiquette système (« fr », « fr-CA », « en_US.UTF-8 ») ;
    /// `None` pour une langue non livrée (« de-DE », « C »).
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        let primary = tag.trim().split(['-', '_', '.', '@']).next().unwrap_or_default();
        if primary.eq_ignore_ascii_case("fr") {
            Some(Self::Fr)
        } else if primary.eq_ignore_ascii_case("en") {
            Some(Self::En)
        } else {
            None
        }
    }

    /// Langue du système (bureau : réglages régionaux ; web :
    /// `navigator.language`), si elle est livrée. À appeler par
    /// l'application, jamais par les tests.
    #[must_use]
    pub fn from_system() -> Option<Self> {
        sys_locale::get_locale().and_then(|tag| Self::from_tag(&tag))
    }

    /// Langue effective : la préférence, sinon la langue du système, sinon le
    /// français.
    #[must_use]
    pub fn resolve(pref: LanguagePref, system: Option<Self>) -> Self {
        match pref {
            LanguagePref::Fr => Self::Fr,
            LanguagePref::En => Self::En,
            LanguagePref::System => system.unwrap_or(Self::Fr),
        }
    }

    /// Source fluent embarquée de la langue.
    const fn source(self) -> &'static str {
        match self {
            Self::Fr => FTL_FR,
            Self::En => FTL_EN,
        }
    }
}

/// Génère [`TextId`], sa liste [`TextId::ALL`] et sa méthode `key()`.
macro_rules! text_ids {
    ($($variant:ident => $key:literal),* $(,)?) => {
        /// Identifiant d'un message traduit (clé fluent vérifiée à la
        /// compilation).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum TextId {
            $(
                #[doc = concat!("Message `", $key, "`.")]
                $variant,
            )*
        }

        impl TextId {
            /// Tous les identifiants, dans l'ordre de déclaration.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// Clé fluent du message.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key,)*
                }
            }
        }
    };
}

text_ids! {
    // Écran d'accueil.
    WelcomeTitle => "welcome-title",
    WelcomeSubtitle => "welcome-subtitle",
    WelcomeOpenDemo => "welcome-open-demo",
    WelcomeOpenFile => "welcome-open-file",
    WelcomeNewProject => "welcome-new-project",
    WelcomeWebNote => "welcome-web-note",
    WelcomeLanguage => "welcome-language",
    WelcomeTheme => "welcome-theme",
    // Menus.
    MenuFile => "menu-file",
    MenuOpen => "menu-open",
    ToolbarOpen => "toolbar-open",
    MenuOpenDemo => "menu-open-demo",
    MenuClose => "menu-close",
    MenuView => "menu-view",
    MenuZoom => "menu-zoom",
    MenuTheme => "menu-theme",
    MenuLanguage => "menu-language",
    MenuDiagnostics => "menu-diagnostics",
    // Barre d'outils et zoom.
    ToolbarFit => "toolbar-fit",
    ZoomDay => "zoom-day",
    ZoomWeek => "zoom-week",
    ZoomMonth => "zoom-month",
    ZoomQuarter => "zoom-quarter",
    // Thème et langue.
    ThemeSystem => "theme-system",
    ThemeLight => "theme-light",
    ThemeDark => "theme-dark",
    LangSystem => "lang-system",
    LangFr => "lang-fr",
    LangEn => "lang-en",
    // Colonnes.
    ColId => "col-id",
    ColName => "col-name",
    ColDuration => "col-duration",
    ColStart => "col-start",
    ColFinish => "col-finish",
    ColPredecessors => "col-predecessors",
    ColTotalSlack => "col-total-slack",
    ColCritical => "col-critical",
    // Colonnes J7 (contrat L0, données au lot L6).
    ColPercentComplete => "col-percent-complete",
    ColResourceNames => "col-resource-names",
    ColBaselineStart => "col-baseline-start",
    ColBaselineFinish => "col-baseline-finish",
    ColFinishVariance => "col-finish-variance",
    Yes => "yes",
    No => "no",
    // Unités de durée.
    UnitMinuteOne => "unit-minute-one",
    UnitMinuteOther => "unit-minute-other",
    UnitHourOne => "unit-hour-one",
    UnitHourOther => "unit-hour-other",
    UnitDayOne => "unit-day-one",
    UnitDayOther => "unit-day-other",
    UnitWeekOne => "unit-week-one",
    UnitWeekOther => "unit-week-other",
    UnitMonthOne => "unit-month-one",
    UnitMonthOther => "unit-month-other",
    UnitMinuteElapsedOne => "unit-minute-elapsed-one",
    UnitMinuteElapsedOther => "unit-minute-elapsed-other",
    UnitHourElapsedOne => "unit-hour-elapsed-one",
    UnitHourElapsedOther => "unit-hour-elapsed-other",
    UnitDayElapsedOne => "unit-day-elapsed-one",
    UnitDayElapsedOther => "unit-day-elapsed-other",
    UnitWeekElapsedOne => "unit-week-elapsed-one",
    UnitWeekElapsedOther => "unit-week-elapsed-other",
    UnitMonthElapsedOne => "unit-month-elapsed-one",
    UnitMonthElapsedOther => "unit-month-elapsed-other",
    // Types de liens.
    LinkFs => "link-fs",
    LinkSs => "link-ss",
    LinkFf => "link-ff",
    LinkSf => "link-sf",
    // Conventions de formatage.
    FmtDecimalSep => "fmt-decimal-sep",
    FmtListSep => "fmt-list-sep",
    FmtDateOrder => "fmt-date-order",
    FmtUnknown => "fmt-unknown",
    FmtPluralRule => "fmt-plural-rule",
    // Mois.
    Month1Long => "month-1-long",
    Month1Short => "month-1-short",
    Month2Long => "month-2-long",
    Month2Short => "month-2-short",
    Month3Long => "month-3-long",
    Month3Short => "month-3-short",
    Month4Long => "month-4-long",
    Month4Short => "month-4-short",
    Month5Long => "month-5-long",
    Month5Short => "month-5-short",
    Month6Long => "month-6-long",
    Month6Short => "month-6-short",
    Month7Long => "month-7-long",
    Month7Short => "month-7-short",
    Month8Long => "month-8-long",
    Month8Short => "month-8-short",
    Month9Long => "month-9-long",
    Month9Short => "month-9-short",
    Month10Long => "month-10-long",
    Month10Short => "month-10-short",
    Month11Long => "month-11-long",
    Month11Short => "month-11-short",
    Month12Long => "month-12-long",
    Month12Short => "month-12-short",
    // Jours de la semaine.
    WeekdayMonShort => "weekday-mon-short",
    WeekdayMonInitial => "weekday-mon-initial",
    WeekdayTueShort => "weekday-tue-short",
    WeekdayTueInitial => "weekday-tue-initial",
    WeekdayWedShort => "weekday-wed-short",
    WeekdayWedInitial => "weekday-wed-initial",
    WeekdayThuShort => "weekday-thu-short",
    WeekdayThuInitial => "weekday-thu-initial",
    WeekdayFriShort => "weekday-fri-short",
    WeekdayFriInitial => "weekday-fri-initial",
    WeekdaySatShort => "weekday-sat-short",
    WeekdaySatInitial => "weekday-sat-initial",
    WeekdaySunShort => "weekday-sun-short",
    WeekdaySunInitial => "weekday-sun-initial",
    // Échelle de temps.
    TierQuarter => "tier-quarter",
    TierWeek => "tier-week",
    // Barre d'état.
    StatusTasks => "status-tasks",
    StatusRange => "status-range",
    StatusErrors => "status-errors",
    StatusWarnings => "status-warnings",
    StatusNoDocument => "status-no-document",
    // Ouverture.
    OpenErrorTitle => "open-error-title",
    OpenErrorRead => "open-error-read",
    OpenErrorMspdi => "open-error-mspdi",
    // Nouveau, Enregistrer, Enregistrer sous, confirmation (lot L13).
    MenuNew => "menu-new",
    MenuSave => "menu-save",
    MenuSaveAs => "menu-save-as",
    MenuQuit => "menu-quit",
    NewProjectName => "new-project-name",
    DialogSaveTitle => "dialog-save-title",
    SaveInProgress => "save-in-progress",
    SaveErrorTitle => "save-error-title",
    SaveErrorWrite => "save-error-write",
    SaveAdjusted => "save-adjusted",
    ConfirmUnsavedTitle => "confirm-unsaved-title",
    ConfirmSave => "confirm-save",
    ConfirmDiscard => "confirm-discard",
    ConfirmCancel => "confirm-cancel",
    // Bandeau de messages.
    MessageDismiss => "message-dismiss",
    RangeTruncated => "range-truncated",
    // Accessibilité et divers.
    RowLabel => "row-label",
    SplitterLabel => "splitter-label",
    GanttScrollLabel => "gantt-scroll-label",
    DemoFileName => "demo-file-name",
    FileFilterMspdi => "file-filter-mspdi",
    DialogOpenTitle => "dialog-open-title",
    StateLoadWarning => "state-load-warning",
    // Menu Édition.
    MenuEdit => "menu-edit",
    MenuUndo => "menu-undo",
    MenuRedo => "menu-redo",
    // Structure (lot L14)
    StructureInsert => "structure-insert",
    StructureDelete => "structure-delete",
    StructureIndent => "structure-indent",
    StructureOutdent => "structure-outdent",
    StructureLink => "structure-link",
    StructureUnlink => "structure-unlink",
    ConfirmDeleteTitle => "confirm-delete-title",
    ConfirmDeleteText => "confirm-delete-text",
    ConfirmDeleteYes => "confirm-delete-yes",
    ConfirmDeleteNo => "confirm-delete-no",
    // Refus de commande (une clé par code d'erreur stable du moteur).
    CommandErrorTitle => "command-error-title",
    ErrorUnknown => "error-unknown",
    ErrorAssignmentDuplicate => "error-assignment-duplicate",
    ErrorAssignmentKindMismatch => "error-assignment-kind_mismatch",
    ErrorAssignmentNotFound => "error-assignment-not_found",
    ErrorAssignmentZeroUnitsFixedWork => "error-assignment-zero_units_fixed_work",
    ErrorBaselineExists => "error-baseline-exists",
    ErrorBaselineRollupNeedsSelection => "error-baseline-rollup_needs_selection",
    ErrorBatchDuplicateLabel => "error-batch-duplicate_label",
    ErrorBatchUnknownLabel => "error-batch-unknown_label",
    ErrorBatchUnsupportedVersion => "error-batch-unsupported_version",
    ErrorBatchWouldCreateErrors => "error-batch-would_create_errors",
    ErrorCalendarExceptionNotFound => "error-calendar-exception_not_found",
    ErrorCalendarExceptionOverlap => "error-calendar-exception_overlap",
    ErrorCalendarNotFound => "error-calendar-not_found",
    ErrorCommandNotYetAvailable => "error-command-not_yet_available",
    ErrorExplainNotScheduled => "error-explain-not_scheduled",
    ErrorFileInvalid => "error-file-invalid",
    ErrorFileOpenInApp => "error-file-open_in_app",
    ErrorFileOutsideRoot => "error-file-outside_root",
    ErrorFileRead => "error-file-read",
    ErrorFileTooLarge => "error-file-too_large",
    ErrorFileWrite => "error-file-write",
    ErrorHistoryEmpty => "error-history-empty",
    ErrorHistoryInconsistent => "error-history-inconsistent",
    ErrorLimitExceeded => "error-limit-exceeded",
    ErrorLinkDuplicate => "error-link-duplicate",
    ErrorLinkNotFound => "error-link-not_found",
    ErrorLinkSelf => "error-link-self",
    ErrorLinkSummaryDescendant => "error-link-summary_descendant",
    ErrorParamConflict => "error-param-conflict",
    ErrorParamInvalid => "error-param-invalid",
    ErrorParamMismatch => "error-param-mismatch",
    ErrorProgressDurationBelowActual => "error-progress-duration_below_actual",
    ErrorProgressInactive => "error-progress-inactive",
    ErrorProgressInvalidOrder => "error-progress-invalid_order",
    ErrorProgressSummaryNotEditable => "error-progress-summary_not_editable",
    ErrorResourceHasAssignments => "error-resource-has_assignments",
    ErrorResourceNotFound => "error-resource-not_found",
    ErrorSessionNone => "error-session-none",
    ErrorSessionPoisoned => "error-session-poisoned",
    ErrorTaskComputedField => "error-task-computed_field",
    ErrorTaskFixedWorkEffortDriven => "error-task-fixed_work_effort_driven",
    ErrorTaskHasChildren => "error-task-has_children",
    ErrorTaskManualUnsupported => "error-task-manual_unsupported",
    ErrorTaskMoveIntoSubtree => "error-task-move_into_subtree",
    ErrorTaskNotFound => "error-task-not_found",
    ErrorTaskOutlineRefused => "error-task-outline_refused",
    ErrorTextBadDate => "error-text-bad_date",
    ErrorTextBadLinkType => "error-text-bad_link_type",
    ErrorTextBadNumber => "error-text-bad_number",
    ErrorTextEmpty => "error-text-empty",
    ErrorTextExternalPredecessor => "error-text-external_predecessor",
    ErrorTextNegative => "error-text-negative",
    ErrorTextOutOfRange => "error-text-out_of_range",
    ErrorTextTooLong => "error-text-too_long",
    ErrorTextUnknownUnit => "error-text-unknown_unit",
    // Édition des cellules (lot L12).
    BlankRowLabel => "blank-row-label",
    CellEditLabel => "cell-edit-label",
    EditErrorUnknownId => "edit-error-unknown-id",
    EditErrorNotEditable => "edit-error-not-editable",
    // --- J7 L0 : codes de refus du contrat (ADR-0015) ---
    ErrorCalendarNameTaken => "error-calendar-name_taken",
    ErrorCalendarBaseNotBase => "error-calendar-base_not_base",
    ErrorCalendarInUse => "error-calendar-in_use",
    ErrorGestureNotDraggable => "error-gesture-not_draggable",
    ErrorUsageLimit => "error-usage-limit",
    ErrorControlRejected => "error-control-rejected",
    ErrorControlApprovalTimeout => "error-control-approval_timeout",
    ErrorControlNotRelayable => "error-control-not_relayable",
    ErrorControlNotAttached => "error-control-not_attached",
    ErrorControlAppGone => "error-control-app_gone",
    ErrorControlScreenshotUnavailable => "error-control-screenshot_unavailable",
    ErrorControlScreenshotTooLarge => "error-control-screenshot_too_large",
    ErrorControlBusy => "error-control-busy",
    ErrorControlDocumentChanged => "error-control-document_changed",
    // --- J7 L1 ---
    TableLabel => "table-label",
    NavCursorLabel => "nav-cursor-label",
    NavCursorBlankLabel => "nav-cursor-blank-label",
    NavCursorEmptyValue => "nav-cursor-empty-value",
    // --- J7 L2 ---
    // --- J7 L3 ---
    ApprovalTitle => "approval-title",
    ApprovalChanged => "approval-changed",
    ApprovalFinish => "approval-finish",
    ApprovalFinishSame => "approval-finish-same",
    ApprovalCritical => "approval-critical",
    ApprovalWarnings => "approval-warnings",
    ApprovalQueued => "approval-queued",
    ApprovalAccept => "approval-accept",
    ApprovalReject => "approval-reject",
    MenuUndoAgent => "menu-undo-agent",
    MenuUndoAgentOps => "menu-undo-agent-ops",
    // --- J7 L4 ---
    // --- J7 L5 ---
    // --- J7 L6 ---
    FmtPercentSuffix => "fmt-percent-suffix",
    ColumnHeaderLabel => "column-header-label",
    ColumnResizeLabel => "column-resize-label",
    ColumnMenuHide => "column-menu-hide",
    TableScrollLabel => "table-scroll-label",
    // --- J7 L7 ---
    MenuGanttStandard => "menu-gantt-standard",
    MenuGanttTracking => "menu-gantt-tracking",
    // --- J7 L8 ---
    GanttReasonSummary => "gantt-reason-summary",
    GanttReasonStarted => "gantt-reason-started",
    GanttReasonMilestone => "gantt-reason-milestone",
    GanttReasonExcluded => "gantt-reason-excluded",
    GanttReasonBlank => "gantt-reason-blank",
    GanttReasonOverflow => "gantt-reason-overflow",
    GanttReasonSameTask => "gantt-reason-same-task",
    LinkDialogTitle => "link-dialog-title",
    LinkDialogFrom => "link-dialog-from",
    LinkDialogTo => "link-dialog-to",
    LinkDialogType => "link-dialog-type",
    LinkDialogLag => "link-dialog-lag",
    LinkDialogDelete => "link-dialog-delete",
    LinkDialogStale => "link-dialog-stale",
    LinkDialogTask => "link-dialog-task",
    // --- J7 L9 ---
    TaskInfoMenu => "task-info-menu",
    TaskInfoTitle => "task-info-title",
    TaskInfoTabGeneral => "task-info-tab-general",
    TaskInfoTabPredecessors => "task-info-tab-predecessors",
    TaskInfoTabResources => "task-info-tab-resources",
    TaskInfoTabAdvanced => "task-info-tab-advanced",
    TaskInfoTabNotes => "task-info-tab-notes",
    TaskInfoName => "task-info-name",
    TaskInfoDuration => "task-info-duration",
    TaskInfoPercent => "task-info-percent",
    TaskInfoStart => "task-info-start",
    TaskInfoFinish => "task-info-finish",
    TaskInfoActive => "task-info-active",
    TaskInfoPredId => "task-info-pred-id",
    TaskInfoPredType => "task-info-pred-type",
    TaskInfoPredLag => "task-info-pred-lag",
    TaskInfoRowField => "task-info-row-field",
    TaskInfoAddPredecessor => "task-info-add-predecessor",
    TaskInfoAddResource => "task-info-add-resource",
    TaskInfoRemoveRow => "task-info-remove-row",
    TaskInfoResourceName => "task-info-resource-name",
    TaskInfoResourceUnits => "task-info-resource-units",
    TaskInfoNoResource => "task-info-no-resource",
    TaskInfoConstraint => "task-info-constraint",
    TaskInfoConstraintDate => "task-info-constraint-date",
    TaskInfoDeadline => "task-info-deadline",
    TaskInfoTaskType => "task-info-task-type",
    TaskInfoEffortDriven => "task-info-effort-driven",
    TaskInfoNotes => "task-info-notes",
    TaskInfoLinkFs => "task-info-link-fs",
    TaskInfoLinkSs => "task-info-link-ss",
    TaskInfoLinkFf => "task-info-link-ff",
    TaskInfoLinkSf => "task-info-link-sf",
    TaskInfoConstraintAsap => "task-info-constraint-asap",
    TaskInfoConstraintAlap => "task-info-constraint-alap",
    TaskInfoConstraintMso => "task-info-constraint-mso",
    TaskInfoConstraintMfo => "task-info-constraint-mfo",
    TaskInfoConstraintSnet => "task-info-constraint-snet",
    TaskInfoConstraintSnlt => "task-info-constraint-snlt",
    TaskInfoConstraintFnet => "task-info-constraint-fnet",
    TaskInfoConstraintFnlt => "task-info-constraint-fnlt",
    TaskInfoTypeFixedUnits => "task-info-type-fixed-units",
    TaskInfoTypeFixedDuration => "task-info-type-fixed-duration",
    TaskInfoTypeFixedWork => "task-info-type-fixed-work",
    TaskInfoOk => "task-info-ok",
    TaskInfoCancel => "task-info-cancel",
    TaskInfoStale => "task-info-stale",
    TaskInfoDateRequired => "task-info-date-required",
    TaskInfoBadId => "task-info-bad-id",
    TaskInfoDuplicatePredecessor => "task-info-duplicate-predecessor",
    TaskInfoDuplicateResource => "task-info-duplicate-resource",
    TaskInfoUnknownResource => "task-info-unknown-resource",
    // --- J7 L10 ---
    MenuViewResourceSheet => "menu-view-resource-sheet",
    ResSheetLabel => "res-sheet-label",
    ResColIndicator => "res-col-indicator",
    ResColName => "res-col-name",
    ResColKind => "res-col-kind",
    ResColMaterialLabel => "res-col-material-label",
    ResColInitials => "res-col-initials",
    ResColGroup => "res-col-group",
    ResColMaxUnits => "res-col-max-units",
    ResColStandardRate => "res-col-standard-rate",
    ResColOvertimeRate => "res-col-overtime-rate",
    ResColCostPerUse => "res-col-cost-per-use",
    ResColCalendar => "res-col-calendar",
    ResKindWork => "res-kind-work",
    ResKindMaterial => "res-kind-material",
    ResKindCost => "res-kind-cost",
    RateMinute => "rate-minute",
    RateHour => "rate-hour",
    RateDay => "rate-day",
    RateWeek => "rate-week",
    RateMonth => "rate-month",
    RateYear => "rate-year",
    ResRowLabel => "res-row-label",
    ResBlankRowLabel => "res-blank-row-label",
    ResNameFirst => "res-name-first",
    ResUnknownCalendar => "res-unknown-calendar",
    ResCalendarNone => "res-calendar-none",
    ResDeleteTitle => "res-delete-title",
    ResDeleteText => "res-delete-text",
    // --- J7 L11 ---
    MenuViewResourceUsage => "menu-view-resource-usage",
    UsageSheetLabel => "usage-sheet-label",
    UsagePeriodsLabel => "usage-periods-label",
    UsageColExpand => "usage-col-expand",
    UsageColName => "usage-col-name",
    UsageColWork => "usage-col-work",
    UsageHoursSuffix => "usage-hours-suffix",
    UsageExpand => "usage-expand",
    UsageCollapse => "usage-collapse",
    UsageAssignmentLabel => "usage-assignment-label",
    UsageLimit => "usage-limit",
    // --- J7 L12 ---
    MenuProject => "menu-project",
    MenuProjectInfo => "menu-project-info",
    MenuWorkingTime => "menu-working-time",
    MenuBaseline => "menu-baseline",
    MenuReschedule => "menu-reschedule",
    ScopeProject => "scope-project",
    ScopeSelection => "scope-selection",
    WtTitle => "wt-title",
    WtCalendar => "wt-calendar",
    WtProjectCalendar => "wt-project-calendar",
    WtResourceCalendar => "wt-resource-calendar",
    WtNew => "wt-new",
    WtDelete => "wt-delete",
    WtName => "wt-name",
    WtBaseOf => "wt-base-of",
    WtNewName => "wt-new-name",
    WtNewBase => "wt-new-base",
    WtNewCopy => "wt-new-copy",
    WtCreate => "wt-create",
    WtNewCancel => "wt-new-cancel",
    WtPrevMonth => "wt-prev-month",
    WtNextMonth => "wt-next-month",
    WtMonthTitle => "wt-month-title",
    WtMonthUnavailable => "wt-month-unavailable",
    WtStateWorking => "wt-state-working",
    WtStateNonWorking => "wt-state-non-working",
    WtLegendWorking => "wt-legend-working",
    WtLegendNonWorking => "wt-legend-non-working",
    WtLegendException => "wt-legend-exception",
    WtTabExceptions => "wt-tab-exceptions",
    WtTabWeek => "wt-tab-week",
    WtExcName => "wt-exc-name",
    WtExcFrom => "wt-exc-from",
    WtExcTo => "wt-exc-to",
    WtExcWorking => "wt-exc-working",
    WtExcRanges => "wt-exc-ranges",
    WtExcRecurring => "wt-exc-recurring",
    WtExcAdd => "wt-exc-add",
    WtWeekInherit => "wt-week-inherit",
    WtWeekNonWorking => "wt-week-non-working",
    WtWeekWorking => "wt-week-working",
    WtWeekSetting => "wt-week-setting",
    WtWeekRanges => "wt-week-ranges",
    WtWeekInherited => "wt-week-inherited",
    WtBadRanges => "wt-bad-ranges",
    PiTitle => "pi-title",
    PiDirection => "pi-direction",
    PiFromStart => "pi-from-start",
    PiFromFinish => "pi-from-finish",
    PiStart => "pi-start",
    PiFinish => "pi-finish",
    PiStatus => "pi-status",
    PiStatusHint => "pi-status-hint",
    PiCalendar => "pi-calendar",
    BlTitle => "bl-title",
    BlSet => "bl-set",
    BlClear => "bl-clear",
    BlNumber => "bl-number",
    BlBaseline0 => "bl-baseline-0",
    BlBaselineN => "bl-baseline-n",
    BlRollup => "bl-rollup",
    BlRollupNone => "bl-rollup-none",
    BlRollupAll => "bl-rollup-all",
    BlRollupFrom => "bl-rollup-from",
    BlConfirm => "bl-confirm",
    BlReplace => "bl-replace",
    RsTitle => "rs-title",
    RsAfter => "rs-after",
    // --- J7 L13 ---
    MenuAssignResources => "menu-assign-resources",
    AsTitle => "as-title",
    AsTaskOne => "as-task-one",
    AsTaskMany => "as-task-many",
    AsNoTask => "as-no-task",
    AsFilter => "as-filter",
    AsResource => "as-resource",
    AsUnits => "as-units",
    AsUnitsOf => "as-units-of",
    AsAssigned => "as-assigned",
    AsAssignedAll => "as-assigned-all",
    AsAssignedSome => "as-assigned-some",
    AsAssign => "as-assign",
    AsRemove => "as-remove",
    AsUpdateUnits => "as-update-units",
    AsClose => "as-close",
    AsNoResource => "as-no-resource",
    AsHint => "as-hint",
    // --- J7 L14 ---
    OpenErrorReadFile => "open-error-read-file",
    OpenErrorMspdiFile => "open-error-mspdi-file",
    OpenErrorMissing => "open-error-missing",
    MenuRecent => "menu-recent",
    MenuRecentEmpty => "menu-recent-empty",
    MenuRecentMissing => "menu-recent-missing",
    // --- J7 L15 ---
    RecoveryTitle => "recovery-title",
    RecoveryCheckpointed => "recovery-checkpointed",
    RecoveryRestore => "recovery-restore",
    RecoveryDiscard => "recovery-discard",
    RecoveryLater => "recovery-later",
    RecoveryPartial => "recovery-partial",
    RecoveryFailed => "recovery-failed",
    RecoveryDiscardFailed => "recovery-discard-failed",
    RecoveryJournalCut => "recovery-journal-cut",
    RecoveryFileChanged => "recovery-file-changed",
    RecoveryKept => "recovery-kept",
    RecoveredFileName => "recovered-file-name",
    // --- J7 L16 ---
    // --- J8 L0 : codes de refus du contrat (ADR-0016) ---
    ErrorPasteUnrecognized => "error-paste-unrecognized",
    ErrorPasteTooLarge => "error-paste-too_large",
    ErrorPasteLinksDropped => "error-paste-links_dropped",
    ErrorStyleIgnored => "error-style-ignored",
    ErrorStyleInvalid => "error-style-invalid",
    ErrorPdfTooManyPages => "error-pdf-too_many_pages",
    ErrorPdfGlyphMissing => "error-pdf-glyph_missing",
    ErrorPdfLogoInvalid => "error-pdf-logo_invalid",
    ErrorTaskManualFinishBeforeStart => "error-task-manual_finish_before_start",
    ErrorControlBadEndpoint => "error-control-bad_endpoint",
    // --- J8 L1 ---
    // --- J8 L2 ---
    // --- J8 L3 ---
    // --- J8 L4 ---
    // --- J8 L5 ---
    // --- J8 L6 ---
    // --- J8 L7 ---
    // --- J8 L8 ---
    // --- J8 L9 ---
    MenuFilter => "menu-filter",
    FilterAll => "filter-all",
    FilterCritical => "filter-critical",
    FilterLate => "filter-late",
    FilterIncomplete => "filter-incomplete",
    FilterMilestones => "filter-milestones",
    FilterSummaries => "filter-summaries",
    FilterUsingResource => "filter-using-resource",
    FilterDateRange => "filter-date-range",
    FilterCustom => "filter-custom",
    FilterReapply => "filter-reapply",
    FilterClear => "filter-clear",
    FilterUsingNamed => "filter-using-named",
    StatusFilter => "status-filter",
    FilterCustomTitle => "filter-custom-title",
    FilterResourceTitle => "filter-resource-title",
    FilterDateRangeTitle => "filter-date-range-title",
    FilterApply => "filter-apply",
    FilterResource => "filter-resource",
    FilterNoResource => "filter-no-resource",
    FilterUseResource => "filter-use-resource",
    FilterFrom => "filter-from",
    FilterTo => "filter-to",
    FilterActiveOnly => "filter-active-only",
    FilterOutlineMax => "filter-outline-max",
    FilterNameContains => "filter-name-contains",
    FilterRelated => "filter-related",
    FilterDateOrder => "filter-date-order",
    FilterDatesRequired => "filter-dates-required",
    FilterResourceMissing => "filter-resource-missing",
    MenuSort => "menu-sort",
    MenuSortById => "menu-sort-by-id",
    SortTitle => "sort-title",
    SortBy => "sort-by",
    SortAscending => "sort-ascending",
    SortDescending => "sort-descending",
    SortKeepOutline => "sort-keep-outline",
    SortApply => "sort-apply",
    SortReset => "sort-reset",
    MenuOutlineLevel => "menu-outline-level",
    OutlineLevelN => "outline-level-n",
    OutlineAll => "outline-all",
    MenuHideSubtasks => "menu-hide-subtasks",
    MenuShowSubtasks => "menu-show-subtasks",
    OutlineExpandRow => "outline-expand-row",
    OutlineCollapseRow => "outline-collapse-row",
    ViewReorderLocked => "view-reorder-locked",
    MenuFind => "menu-find",
    SearchLabel => "search-label",
    SearchPrevious => "search-previous",
    SearchNext => "search-next",
    SearchAllColumns => "search-all-columns",
    SearchClose => "search-close",
    SearchNone => "search-none",
    SearchCounter => "search-counter",
    SearchHidden => "search-hidden",
    // --- J8 L10 ---
    MenuCut => "menu-cut",
    MenuCopy => "menu-copy",
    MenuPaste => "menu-paste",
    MenuPasteSpecial => "menu-paste-special",
    MenuImportCsv => "menu-import-csv",
    PasteReportTitle => "paste-report-title",
    PasteReportTasks => "paste-report-tasks",
    PasteReportLinks => "paste-report-links",
    PasteReportAssignments => "paste-report-assignments",
    PasteReportResources => "paste-report-resources",
    PasteHintUnrecognized => "paste-hint-unrecognized",
    PasteHintTooLarge => "paste-hint-too-large",
    PasteUseShortcut => "paste-use-shortcut",
    PsTitle => "ps-title",
    PsSource => "ps-source",
    PsLinks => "ps-links",
    PsResources => "ps-resources",
    PsResMatch => "ps-res-match",
    PsResCreate => "ps-res-create",
    PsResDrop => "ps-res-drop",
    PsNothing => "ps-nothing",
    CiPickTitle => "ci-pick-title",
    CiPickFilter => "ci-pick-filter",
    CiTitle => "ci-title",
    CiFile => "ci-file",
    CiSeparator => "ci-separator",
    CiEncoding => "ci-encoding",
    CiLanguage => "ci-language",
    CiAuto => "ci-auto",
    CiSepSemicolon => "ci-sep-semicolon",
    CiSepComma => "ci-sep-comma",
    CiSepTab => "ci-sep-tab",
    CiEncUtf8 => "ci-enc-utf8",
    CiEncUtf16Le => "ci-enc-utf16le",
    CiEncUtf16Be => "ci-enc-utf16be",
    CiEncWindows1252 => "ci-enc-windows1252",
    CiDetected => "ci-detected",
    CiColumns => "ci-columns",
    CiRows => "ci-rows",
    CiRecognized => "ci-recognized",
    CiUnknown => "ci-unknown",
    CiUnsupported => "ci-unsupported",
    CiDuplicate => "ci-duplicate",
    CiDiagnostics => "ci-diagnostics",
    CiDiagnosticLine => "ci-diagnostic-line",
    CiImport => "ci-import",
    CiImported => "ci-imported",
    ErrorCsvErrorTooLarge => "error-csv-error-too_large",
    ErrorCsvErrorEmpty => "error-csv-error-empty",
    ErrorCsvErrorNoHeader => "error-csv-error-no_header",
    ErrorCsvErrorNoKnownColumn => "error-csv-error-no_known_column",
    ErrorCsvErrorTooManyRows => "error-csv-error-too_many_rows",
    ErrorCsvErrorTooManyColumns => "error-csv-error-too_many_columns",
    ErrorCsvErrorEncoding => "error-csv-error-encoding",
    ErrorCsvErrorNotYetAvailable => "error-csv-error-not_yet_available",
    // --- J8 L11 ---
    ColMode => "col-mode",
    FmtModeManual => "fmt-mode-manual",
    FmtModeAuto => "fmt-mode-auto",
    GanttStatusLabel => "gantt-status-label",
    // --- J8 L12 ---
    MenuAiPanel => "menu-ai-panel",
    SideTabAi => "side-tab-ai",
    SideTabWhy => "side-tab-why",
    SideClose => "side-close",
    AiNoPending => "ai-no-pending",
    AiMore => "ai-more",
    AiCriticalEntered => "ai-critical-entered",
    AiCriticalLeft => "ai-critical-left",
    AiAdded => "ai-added",
    AiRemoved => "ai-removed",
    AiNoPreview => "ai-no-preview",
    AiHistory => "ai-history",
    AiHistoryEmpty => "ai-history-empty",
    AiHistoryLine => "ai-history-line",
    AiOutcomeAccepted => "ai-outcome-accepted",
    AiOutcomeRejected => "ai-outcome-rejected",
    AiOutcomeExpired => "ai-outcome-expired",
    AiOutcomeStale => "ai-outcome-stale",
    AiOutcomeFailed => "ai-outcome-failed",
    AiUndoEntry => "ai-undo-entry",
    WhyNoTask => "why-no-task",
    WhyDrivers => "why-drivers",
    WhyUnavailable => "why-unavailable",
    // --- J8 L13 ---
    // --- J8 L14 ---
    DiagCalendarUnknown => "diag-calendar-unknown",
    DiagCalendarBaseMissing => "diag-calendar-base_missing",
    DiagCalendarBaseNotBase => "diag-calendar-base_not_base",
    DiagCalendarInheritanceCycle => "diag-calendar-inheritance_cycle",
    DiagCalendarNoWorkingTime => "diag-calendar-no_working_time",
    DiagCalendarInternal => "diag-calendar-internal",
    DiagCalendarExceptionsOverlap => "diag-calendar-exceptions_overlap",
    DiagCalendarWorkWeeksOverlap => "diag-calendar-work_weeks_overlap",
    DiagCalendarRecurrenceUnsupported => "diag-calendar-recurrence_unsupported",
    DiagProjectCalendarNotBase => "diag-project-calendar_not_base",
    DiagTaskReservedUid => "diag-task-reserved_uid",
    DiagTaskOutlineJump => "diag-task-outline_jump",
    DiagTaskNullWithData => "diag-task-null_with_data",
    DiagTaskNegativeProgress => "diag-task-negative_progress",
    DiagLinkSelf => "diag-link-self",
    DiagLinkDuplicate => "diag-link-duplicate",
    DiagLinkUnknownTask => "diag-link-unknown_task",
    DiagAssignmentUnknownTask => "diag-assignment-unknown_task",
    DiagAssignmentUnknownResource => "diag-assignment-unknown_resource",
    DiagConversionOutOfDomain => "diag-conversion-out_of_domain",
    DiagLinkCycle => "diag-link-cycle",
    DiagLinkIgnoredInCycle => "diag-link-ignored_in_cycle",
    DiagLinkSummaryDescendant => "diag-link-summary_descendant",
    DiagLinkFinishToSummary => "diag-link-finish_to_summary",
    DiagLinkExpansionLimit => "diag-link-expansion_limit",
    DiagLinkStartFromSummary => "diag-link-start_from_summary",
    DiagLinkInactive => "diag-link-inactive",
    DiagLinkExcludedEndpoint => "diag-link-excluded_endpoint",
    DiagLinkManualNotHonored => "diag-link-manual_not_honored",
    DiagLinkUnschedulable => "diag-link-unschedulable",
    DiagTaskManualUndated => "diag-task-manual_undated",
    DiagTaskManualDatesInverted => "diag-task-manual_dates_inverted",
    DiagTaskCalendarFallback => "diag-task-calendar_fallback",
    DiagProjectCalendarFallback => "diag-project-calendar_fallback",
    DiagTaskUnschedulable => "diag-task-unschedulable",
    DiagConstraintOverride => "diag-constraint-override",
    DiagConstraintNotHonored => "diag-constraint-not_honored",
    DiagConstraintSummaryIgnored => "diag-constraint-summary_ignored",
    DiagConstraintManualIgnored => "diag-constraint-manual_ignored",
    DiagTaskDeadlineMissed => "diag-task-deadline_missed",
    DiagResourceOverallocated => "diag-resource-overallocated",
    DiagResourceAvailabilityIgnored => "diag-resource-availability_ignored",
    DiagAssignmentContourUnsupported => "diag-assignment-contour_unsupported",
    DiagAssignmentZeroUnits => "diag-assignment-zero_units",
    DiagAssignmentDurationOutOfDomain => "diag-assignment-duration_out_of_domain",
    DiagAssignmentCalendarEmpty => "diag-assignment-calendar_empty",
    DiagTaskFixedWorkEffortDriven => "diag-task-fixed_work_effort_driven",
    DiagCalendarTooManySegments => "diag-calendar-too_many_segments",
    DiagAssignmentTooManyParts => "diag-assignment-too_many_parts",
    DiagProgressLinkIgnored => "diag-progress-link_ignored",
    DiagProgressLinkOverridden => "diag-progress-link_overridden",
    DiagProgressOptionUnsupported => "diag-progress-option_unsupported",
    DiagProgressFromFinishInProgress => "diag-progress-from_finish_in_progress",
    DiagProgressActualStartMissing => "diag-progress-actual_start_missing",
    DiagProgressMilestoneRounded => "diag-progress-milestone_rounded",
    DiagConstraintIgnoredInProgress => "diag-constraint-ignored_in_progress",
    DiagBaselineCostNotComputed => "diag-baseline-cost_not_computed",
    DiagBaselineTaskSkipped => "diag-baseline-task_skipped",
    DiagBaselineNothingToClear => "diag-baseline-nothing_to_clear",
    DiagScheduleInternal => "diag-schedule-internal",
    DiagMspdiInvalidValue => "diag-mspdi-invalid_value",
    DiagMspdiUnknownCode => "diag-mspdi-unknown_code",
    DiagMspdiMissingRequired => "diag-mspdi-missing_required",
    DiagMspdiDuplicate => "diag-mspdi-duplicate",
    DiagMspdiDanglingReference => "diag-mspdi-dangling_reference",
    DiagMspdiLinkIgnored => "diag-mspdi-link_ignored",
    DiagMspdiRounded => "diag-mspdi-rounded",
    DiagMspdiUnsupported => "diag-mspdi-unsupported",
    DiagMspdiInconsistent => "diag-mspdi-inconsistent",
    DiagMspdiTruncated => "diag-mspdi-truncated",
    DiagMspdiNotPreserved => "diag-mspdi-not_preserved",
    DiagMspdiHonorConstraintsOff => "diag-mspdi-honor_constraints_off",
    DiagMspdiProgressInconsistent => "diag-mspdi-progress_inconsistent",
    DiagStyleIgnored => "diag-style-ignored",
    DiagMspdiErrorEmpty => "diag-mspdi-error-empty",
    DiagMspdiErrorTooLarge => "diag-mspdi-error-too_large",
    DiagMspdiErrorUtf16NotSupported => "diag-mspdi-error-utf16_not_supported",
    DiagMspdiErrorInvalidUtf8 => "diag-mspdi-error-invalid_utf8",
    DiagMspdiErrorXml => "diag-mspdi-error-xml",
    DiagMspdiErrorUnexpectedEof => "diag-mspdi-error-unexpected_eof",
    DiagMspdiErrorDoctypeNotAllowed => "diag-mspdi-error-doctype_not_allowed",
    DiagMspdiErrorTooDeep => "diag-mspdi-error-too_deep",
    DiagMspdiErrorTooManyElements => "diag-mspdi-error-too_many_elements",
    DiagMspdiErrorTooManyRetained => "diag-mspdi-error-too_many_retained",
    DiagMspdiErrorTextTooLong => "diag-mspdi-error-text_too_long",
    DiagMspdiErrorNotMspdi => "diag-mspdi-error-not_mspdi",
    DiagLocation => "diag-location",
    DiagSubjectProject => "diag-subject-project",
    DiagSubjectCalendar => "diag-subject-calendar",
    DiagSubjectTask => "diag-subject-task",
    DiagSubjectLink => "diag-subject-link",
    DiagSubjectResource => "diag-subject-resource",
    DiagSubjectAssignment => "diag-subject-assignment",
    DiagPassForward => "diag-pass-forward",
    DiagPassBackward => "diag-pass-backward",
    DiagBoundProjectStart => "diag-bound-project-start",
    DiagBoundProjectFinish => "diag-bound-project-finish",
    DiagBoundSummaryGate => "diag-bound-summary-gate",
    DiagBoundSummaryCap => "diag-bound-summary-cap",
    DiagBoundPredecessor => "diag-bound-predecessor",
    DiagBoundSuccessor => "diag-bound-successor",
    DiagWithElement => "diag-with-element",
    DiagWithValue => "diag-with-value",
    DiagWithDetail => "diag-with-detail",
    TemplateClassique => "template-classique",
    TemplateSobre => "template-sobre",
    TemplateContrasteEleve => "template-contraste-eleve",
    TemplatePresentation => "template-presentation",
    TemplateChantier => "template-chantier",
    TemplateImpressionNb => "template-impression-nb",
    TokenProject => "token-project",
    TokenStatusDate => "token-status-date",
    TokenPage => "token-page",
    TokenPages => "token-pages",
    TokenPrintDate => "token-print-date",
    TokenFile => "token-file",
    // --- J8 L15 ---
    // --- J8 L16 ---
}

/// Clé fluent du refus de code `code` : `error-<code>`, points changés en
/// tirets (`task.not_found` → `error-task-not_found`).
#[must_use]
pub fn error_key(code: &str) -> String {
    format!("error-{}", code.replace('.', "-"))
}

/// Message traduit d'un refus de commande, par son code stable
/// (`planrs_automation::error::codes`, `text.*`) ;
/// [`TextId::ErrorUnknown`] pour un code sans clé (jamais attendu : un test
/// vérifie que chaque code du moteur a la sienne).
#[must_use]
pub fn error_text_id(code: &str) -> TextId {
    let key = error_key(code);
    TextId::ALL
        .iter()
        .copied()
        .find(|id| id.key() == key)
        .unwrap_or(TextId::ErrorUnknown)
}

/// Clé fluent d'un diagnostic de code `code` (N5, N9 ; J8 lot L14) :
/// `diag-<code>`, points changés en tirets (`link.cycle` →
/// `diag-link-cycle`, `mspdi.error.xml` → `diag-mspdi-error-xml`).
#[must_use]
pub fn diag_key(code: &str) -> String {
    format!("diag-{}", code.replace('.', "-"))
}

/// Nom affiché d'un modèle de mise en forme (galerie, préférences).
/// « Chantier » s'affiche « Planning travaux » (retour du propriétaire du
/// 2026-10-26, H-UI-63 à H-UI-67) ; l'identifiant stable ne change pas.
#[must_use]
pub const fn template_name(id: planrs_render::TemplateId) -> TextId {
    use planrs_render::TemplateId as T;
    match id {
        T::Classique => TextId::TemplateClassique,
        T::Sobre => TextId::TemplateSobre,
        T::ContrasteEleve => TextId::TemplateContrasteEleve,
        T::Presentation => TextId::TemplatePresentation,
        T::Chantier => TextId::TemplateChantier,
        T::ImpressionNb => TextId::TemplateImpressionNb,
    }
}

/// Libellé d'un jeton d'en-tête ou de pied de page (H-UI-51), pour la
/// fenêtre « Mise en page » ; le jeton écrit dans le style (`{projet}`…)
/// ne se traduit pas.
#[must_use]
pub const fn token_label(token: planrs_render::geometry::Token) -> TextId {
    use planrs_render::geometry::Token as T;
    match token {
        T::Project => TextId::TokenProject,
        T::StatusDate => TextId::TokenStatusDate,
        T::Page => TextId::TokenPage,
        T::Pages => TextId::TokenPages,
        T::PrintDate => TextId::TokenPrintDate,
        T::File => TextId::TokenFile,
    }
}

/// Pseudo-langue de test (J8 lot L14) : lettres accentuées, texte allongé
/// d'au moins 40 % et encadré de crochets (« [Ƒíléé····] »), pour voir une
/// troncature ou une chaîne restée en dur. Les espaces et la ponctuation
/// sont gardés ; seules des lettres Latin-1 (présentes dans les polices
/// d'egui) sont utilisées. Chaîne vide inchangée.
#[must_use]
pub fn pseudo(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(text.len().saturating_mul(2).saturating_add(8));
    out.push('[');
    let mut chars: usize = 0;
    for c in text.chars() {
        chars = chars.saturating_add(1);
        out.push(match c {
            'a' => 'á',
            'e' => 'é',
            'i' => 'í',
            'o' => 'ó',
            'u' => 'ú',
            'y' => 'ý',
            'c' => 'ç',
            'n' => 'ñ',
            'A' => 'Á',
            'E' => 'É',
            'I' => 'Í',
            'O' => 'Ó',
            'U' => 'Ú',
            'Y' => 'Ý',
            'C' => 'Ç',
            'N' => 'Ñ',
            other => other,
        });
    }
    // 40 % arrondi au-dessus, au moins un caractère de remplissage.
    let pad = chars.saturating_mul(2).saturating_add(4) / 5;
    for _ in 0..pad.max(1) {
        out.push('·');
    }
    out.push(']');
    out
}

/// Noms longs des mois, janvier d'abord.
const MONTHS_LONG: [TextId; 12] = [
    TextId::Month1Long,
    TextId::Month2Long,
    TextId::Month3Long,
    TextId::Month4Long,
    TextId::Month5Long,
    TextId::Month6Long,
    TextId::Month7Long,
    TextId::Month8Long,
    TextId::Month9Long,
    TextId::Month10Long,
    TextId::Month11Long,
    TextId::Month12Long,
];

/// Noms courts des mois, janvier d'abord.
const MONTHS_SHORT: [TextId; 12] = [
    TextId::Month1Short,
    TextId::Month2Short,
    TextId::Month3Short,
    TextId::Month4Short,
    TextId::Month5Short,
    TextId::Month6Short,
    TextId::Month7Short,
    TextId::Month8Short,
    TextId::Month9Short,
    TextId::Month10Short,
    TextId::Month11Short,
    TextId::Month12Short,
];

/// Noms courts des jours, dimanche d'abord (ordre de `Weekday::ALL`).
const WEEKDAYS_SHORT: [TextId; 7] = [
    TextId::WeekdaySunShort,
    TextId::WeekdayMonShort,
    TextId::WeekdayTueShort,
    TextId::WeekdayWedShort,
    TextId::WeekdayThuShort,
    TextId::WeekdayFriShort,
    TextId::WeekdaySatShort,
];

/// Initiales des jours, dimanche d'abord (ordre de `Weekday::ALL`).
const WEEKDAY_INITIALS: [TextId; 7] = [
    TextId::WeekdaySunInitial,
    TextId::WeekdayMonInitial,
    TextId::WeekdayTueInitial,
    TextId::WeekdayWedInitial,
    TextId::WeekdayThuInitial,
    TextId::WeekdayFriInitial,
    TextId::WeekdaySatInitial,
];

/// Argument d'un message : chaîne pré-formatée ou entier (pluriels).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg<'a> {
    /// Chaîne déjà formatée (nom, date, durée…).
    Str(Cow<'a, str>),
    /// Entier, pour la sélection plurielle et l'affichage de compteurs.
    Count(i64),
}

/// Messages d'une langue, avec repli sur le français.
pub struct Texts {
    /// Langue demandée.
    language: Language,
    /// Messages de la langue demandée.
    bundle: FluentBundle<FluentResource>,
    /// Messages français, quand la langue demandée n'est pas le français.
    fallback: Option<FluentBundle<FluentResource>>,
    /// Identifiants déjà signalés comme manquants (journalisés une fois).
    missing: RefCell<BTreeSet<TextId>>,
    /// Libellés des formateurs, résolus une fois à la construction.
    labels: FormatLabels,
    /// Pseudo-langue de test ([`pseudo`]) appliquée à chaque message, sauf
    /// aux conventions de formatage (`fmt-*`).
    pseudo: bool,
}

impl std::fmt::Debug for Texts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texts")
            .field("language", &self.language)
            .finish_non_exhaustive()
    }
}

/// Construit le recueil fluent d'une langue à partir de sa source. Ne rate
/// jamais : les erreurs d'analyse sont journalisées et la ressource partielle
/// est utilisée.
fn build_bundle(language: Language, source: &str) -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new(vec![language.langid()]);
    bundle.set_use_isolating(false);
    let resource = match FluentResource::try_new(source.to_owned()) {
        Ok(resource) => resource,
        Err((resource, errors)) => {
            for error in errors {
                log::error!("traductions {language:?} : erreur d'analyse : {error:?}");
            }
            resource
        }
    };
    if let Err(errors) = bundle.add_resource(resource) {
        for error in errors {
            log::error!("traductions {language:?} : {error}");
        }
    }
    bundle
}

impl Texts {
    /// Charge les messages d'une langue (et le français en repli).
    #[must_use]
    pub fn new(language: Language) -> Self {
        let fallback = (language != Language::Fr).then(|| build_bundle(Language::Fr, FTL_FR));
        Self::assemble(
            language,
            build_bundle(language, language.source()),
            fallback,
            false,
        )
    }

    /// Pseudo-langue de test (J8 lot L14) : messages anglais passés par
    /// [`pseudo`] (accents, +40 %, crochets) ; dates, nombres et saisie
    /// comme en anglais ([`Texts::language`] rend `En`). Réservée aux tests
    /// de troncature (`tests/ui/pseudo.rs`), jamais proposée à
    /// l'utilisateur.
    #[must_use]
    pub fn pseudo() -> Self {
        Self::assemble(
            Language::En,
            build_bundle(Language::En, FTL_EN),
            Some(build_bundle(Language::Fr, FTL_FR)),
            true,
        )
    }

    /// Vrai pour la pseudo-langue de test.
    #[must_use]
    pub const fn is_pseudo(&self) -> bool {
        self.pseudo
    }

    /// Recueil construit sur des sources arbitraires (tests du repli).
    #[cfg(test)]
    fn from_sources(language: Language, source: &str, fallback: Option<&str>) -> Self {
        Self::assemble(
            language,
            build_bundle(language, source),
            fallback.map(|fr| build_bundle(Language::Fr, fr)),
            false,
        )
    }

    /// Assemble les recueils puis résout une fois les libellés des
    /// formateurs (lus ensuite par référence à chaque image).
    fn assemble(
        language: Language,
        bundle: FluentBundle<FluentResource>,
        fallback: Option<FluentBundle<FluentResource>>,
        pseudo: bool,
    ) -> Self {
        let mut texts = Self {
            language,
            bundle,
            fallback,
            missing: RefCell::new(BTreeSet::new()),
            labels: FormatLabels::default(),
            pseudo,
        };
        texts.labels = texts.build_format_labels();
        texts
    }

    /// Langue des messages.
    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    /// Message sans argument.
    #[must_use]
    pub fn get(&self, id: TextId) -> String {
        self.format(id, None)
    }

    /// Message avec arguments nommés.
    #[must_use]
    pub fn get_args(&self, id: TextId, args: &[(&str, Arg<'_>)]) -> String {
        let mut fluent_args = FluentArgs::new();
        for (name, arg) in args {
            let value = match arg {
                Arg::Str(text) => FluentValue::from(text.as_ref()),
                Arg::Count(count) => FluentValue::from(*count),
            };
            fluent_args.set(*name, value);
        }
        self.format(id, Some(&fluent_args))
    }

    /// Recherche dans la langue demandée, puis en français, puis rend la clé.
    fn format(&self, id: TextId, args: Option<&FluentArgs<'_>>) -> String {
        let key = id.key();
        for bundle in std::iter::once(&self.bundle).chain(self.fallback.as_ref()) {
            if let Some(pattern) = bundle.get_message(key).and_then(|message| message.value()) {
                let mut errors = Vec::new();
                let text = bundle.format_pattern(pattern, args, &mut errors);
                for error in errors {
                    log::warn!("traduction « {key} » : {error}");
                }
                if self.pseudo && !key.starts_with("fmt-") {
                    return pseudo(&text);
                }
                return text.into_owned();
            }
        }
        if self.missing.borrow_mut().insert(id) {
            log::warn!("traduction manquante : « {key} » ({:?})", self.language);
        }
        key.to_owned()
    }

    /// Libellé d'unité (singulier, pluriel).
    fn unit(&self, one: TextId, other: TextId) -> UnitLabel {
        UnitLabel {
            one: self.get(one),
            other: self.get(other),
        }
    }

    /// Libellés et conventions des formateurs de la langue, résolus une fois
    /// à la construction de `Texts` (aucun travail fluent par image).
    #[must_use]
    pub const fn format_labels(&self) -> &FormatLabels {
        &self.labels
    }

    /// Résout les libellés et conventions des formateurs depuis les `.ftl`.
    fn build_format_labels(&self) -> FormatLabels {
        let date_order = if self.get(TextId::FmtDateOrder).trim() == "mdy" {
            DateOrder::MonthDayYear
        } else {
            DateOrder::DayMonthYear
        };
        let plural_key = self.get(TextId::FmtPluralRule);
        let plural = PluralRule::from_key(&plural_key).unwrap_or_else(|| {
            log::warn!(
                "traductions {:?} : règle de pluriel inconnue « {plural_key} », repli « exactly-one »",
                self.language
            );
            PluralRule::ExactlyOne
        });
        FormatLabels {
            decimal_sep: self.get(TextId::FmtDecimalSep).chars().next().unwrap_or(','),
            list_sep: self.get(TextId::FmtListSep),
            units: UnitLabels {
                minute: self.unit(TextId::UnitMinuteOne, TextId::UnitMinuteOther),
                hour: self.unit(TextId::UnitHourOne, TextId::UnitHourOther),
                day: self.unit(TextId::UnitDayOne, TextId::UnitDayOther),
                week: self.unit(TextId::UnitWeekOne, TextId::UnitWeekOther),
                month: self.unit(TextId::UnitMonthOne, TextId::UnitMonthOther),
                elapsed_minute: self.unit(TextId::UnitMinuteElapsedOne, TextId::UnitMinuteElapsedOther),
                elapsed_hour: self.unit(TextId::UnitHourElapsedOne, TextId::UnitHourElapsedOther),
                elapsed_day: self.unit(TextId::UnitDayElapsedOne, TextId::UnitDayElapsedOther),
                elapsed_week: self.unit(TextId::UnitWeekElapsedOne, TextId::UnitWeekElapsedOther),
                elapsed_month: self.unit(TextId::UnitMonthElapsedOne, TextId::UnitMonthElapsedOther),
            },
            link_fs: self.get(TextId::LinkFs),
            link_ss: self.get(TextId::LinkSs),
            link_ff: self.get(TextId::LinkFf),
            link_sf: self.get(TextId::LinkSf),
            date_order,
            plural,
            yes: self.get(TextId::Yes),
            no: self.get(TextId::No),
            unknown: self.get(TextId::FmtUnknown),
            months_long: MONTHS_LONG.map(|id| self.get(id)),
            months_short: MONTHS_SHORT.map(|id| self.get(id)),
            weekdays_short: WEEKDAYS_SHORT.map(|id| self.get(id)),
            weekday_initials: WEEKDAY_INITIALS.map(|id| self.get(id)),
            quarter_prefix: self.get(TextId::TierQuarter),
            week_prefix: self.get(TextId::TierWeek),
            percent_suffix: self.get(TextId::FmtPercentSuffix),
            mode_manual: self.get(TextId::FmtModeManual),
            mode_auto: self.get(TextId::FmtModeAuto),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};

    use super::{Arg, FTL_EN, FTL_FR, Language, TextId, Texts};
    use crate::format::{DateOrder, PluralRule};
    use crate::state::LanguagePref;
    use fluent_bundle::{FluentArgs, FluentValue};

    /// Clés des messages d'une source fluent ; échoue sur toute erreur
    /// d'analyse.
    fn message_keys(source: &str) -> BTreeSet<String> {
        let resource = match fluent_syntax::parser::parse(source) {
            Ok(resource) => resource,
            Err((_, errors)) => panic!("erreurs d'analyse : {errors:?}"),
        };
        let mut keys = BTreeSet::new();
        for entry in resource.body {
            if let Entry::Message(message) = entry {
                assert!(
                    message.value.is_some(),
                    "message sans valeur : {}",
                    message.id.name
                );
                assert!(
                    keys.insert(message.id.name.to_owned()),
                    "clé en double : {}",
                    message.id.name
                );
            }
        }
        keys
    }

    /// Arguments d'exemple couvrant toutes les variables des messages.
    fn sample_args() -> Vec<(&'static str, Arg<'static>)> {
        vec![
            ("product", Arg::Str("planrs".into())),
            ("count", Arg::Count(3)),
            ("start", Arg::Str("lun 02/11/26".into())),
            ("finish", Arg::Str("lun 17/05/27".into())),
            ("detail", Arg::Str("détail".into())),
            ("id", Arg::Str("3".into())),
            ("name", Arg::Str("Plans".into())),
            ("column", Arg::Str("Durée".into())),
            ("value", Arg::Str("5 jours".into())),
            ("document", Arg::Str("maison.xml".into())),
            ("stem", Arg::Str("maison".into())),
            ("date", Arg::Str("02/11".into())),
            // Diagnostics traduits par code (N5, N9 ; J8 lot L14).
            ("calendar", Arg::Str("1".into())),
            ("referenced_by", Arg::Str("la tâche 3".into())),
            ("base", Arg::Str("2".into())),
            ("chain", Arg::Str("1 -> 2 -> 1".into())),
            ("kept", Arg::Count(1)),
            ("ignored", Arg::Count(2)),
            ("exception", Arg::Count(3)),
            ("task", Arg::Str("3".into())),
            ("predecessor", Arg::Str("2".into())),
            ("successor", Arg::Str("3".into())),
            ("assignment", Arg::Str("4".into())),
            ("resource", Arg::Str("5".into())),
            ("from", Arg::Str("lun 02/11/26".into())),
            ("to", Arg::Str("ven 06/11/26".into())),
            ("minutes", Arg::Count(480)),
            ("tasks", Arg::Str("1, 2, 3".into())),
            ("limit", Arg::Count(64)),
            ("cause", Arg::Str("cause".into())),
            ("pass", Arg::Str("passe avant".into())),
            ("kind", Arg::Str("Doit commencer le".into())),
            ("bound", Arg::Str("le début du projet".into())),
            ("deadline", Arg::Str("lun 02/11/26".into())),
            ("peak", Arg::Count(150)),
            ("contour", Arg::Str("BackLoaded".into())),
            ("task_calendar", Arg::Str("1".into())),
            ("resource_calendar", Arg::Str("2".into())),
            ("other", Arg::Str("2".into())),
            ("parts", Arg::Count(70)),
            ("option", Arg::Str("MoveCompletedEndsForward".into())),
            ("number", Arg::Count(1)),
            ("code", Arg::Count(7)),
            ("reason", Arg::Str("cross_project".into())),
            ("what", Arg::Str("leveling_delay".into())),
            ("dropped", Arg::Count(3)),
            ("size", Arg::Count(300)),
            ("max", Arg::Count(200)),
            ("message", Arg::Str("message".into())),
            ("open", Arg::Str("Project/Tasks".into())),
            ("depth", Arg::Count(2)),
            ("root", Arg::Str("html".into())),
            ("at", Arg::Str("ligne 4, colonne 9 (octet 120)".into())),
            ("line", Arg::Count(4)),
            ("byte", Arg::Count(120)),
            ("head", Arg::Str("tête".into())),
            ("element", Arg::Str("Tasks/Task/Name".into())),
            ("summary", Arg::Str("1".into())),
        ]
    }

    #[test]
    fn i18n_keys_match_text_ids() {
        let expected: BTreeSet<String> = TextId::ALL.iter().map(|id| id.key().to_owned()).collect();
        assert_eq!(expected.len(), TextId::ALL.len(), "clé en double dans text_ids!");
        assert_eq!(message_keys(FTL_FR), expected, "fr.ftl ≠ TextId::ALL");
        assert_eq!(message_keys(FTL_EN), expected, "en.ftl ≠ TextId::ALL");
    }

    #[test]
    fn i18n_no_isolation_marks() {
        for language in Language::ALL {
            let texts = Texts::new(language);
            for &id in TextId::ALL {
                let text = texts.get_args(id, &sample_args());
                assert!(
                    !text.contains(['\u{2068}', '\u{2069}']),
                    "{language:?} {id:?} : {text:?}"
                );
                assert!(!text.is_empty(), "{language:?} {id:?} vide");
            }
        }
    }

    #[test]
    fn i18n_no_narrow_nbsp() {
        assert!(!FTL_FR.contains('\u{202F}'));
        assert!(!FTL_EN.contains('\u{202F}'));
        for language in Language::ALL {
            let texts = Texts::new(language);
            for &id in TextId::ALL {
                assert!(
                    !texts.get_args(id, &sample_args()).contains('\u{202F}'),
                    "{language:?} {id:?}"
                );
            }
        }
    }

    #[test]
    fn i18n_plurals_and_args() {
        let fr = Texts::new(Language::Fr);
        let en = Texts::new(Language::En);
        assert_eq!(
            fr.get_args(TextId::StatusTasks, &[("count", Arg::Count(49))]),
            "49 tâches"
        );
        assert_eq!(
            fr.get_args(TextId::StatusTasks, &[("count", Arg::Count(1))]),
            "1 tâche"
        );
        assert_eq!(
            en.get_args(TextId::StatusTasks, &[("count", Arg::Count(1))]),
            "1 task"
        );
        assert_eq!(
            en.get_args(TextId::StatusErrors, &[("count", Arg::Count(0))]),
            "no errors"
        );
        assert_eq!(
            fr.get_args(TextId::StatusErrors, &[("count", Arg::Count(0))]),
            "aucune erreur"
        );
        assert_eq!(en.get(TextId::MenuFile), "File");
        assert_eq!(fr.get(TextId::MenuFile), "Fichier");
    }

    #[test]
    fn i18n_format_labels() {
        let fr = Texts::new(Language::Fr).format_labels().clone();
        assert_eq!(fr.decimal_sep, ',');
        assert_eq!(fr.list_sep, ";");
        assert_eq!(fr.date_order, DateOrder::DayMonthYear);
        assert_eq!(fr.link_fs, "FD");
        assert_eq!(fr.months_short[0], "janv.");
        assert_eq!(fr.weekdays_short[1], "lun");
        assert_eq!(fr.units.day.other, "jours");
        let en = Texts::new(Language::En).format_labels().clone();
        assert_eq!(en.decimal_sep, '.');
        assert_eq!(en.list_sep, ",");
        assert_eq!(en.date_order, DateOrder::MonthDayYear);
        assert_eq!(en.link_sf, "SF");
        assert_eq!(en.months_short[1], "Feb");
        assert_eq!(en.weekdays_short[0], "Sun");
    }

    /// La règle de pluriel et le préfixe de semaine viennent des clés
    /// `fmt-plural-rule` et `tier-week`, pas de l'ordre des dates.
    #[test]
    fn i18n_plural_rule_and_week_prefix_from_keys() {
        let fr = Texts::new(Language::Fr);
        assert_eq!(fr.format_labels().plural, PluralRule::BelowTwo);
        assert_eq!(fr.format_labels().week_prefix, "S");
        let en = Texts::new(Language::En);
        assert_eq!(en.format_labels().plural, PluralRule::ExactlyOne);
        assert_eq!(en.format_labels().week_prefix, "W");
        // Ordre des dates français, règle anglaise : la clé l'emporte.
        let texts = Texts::from_sources(
            Language::Fr,
            "fmt-date-order = dmy\nfmt-plural-rule = exactly-one\n",
            None,
        );
        assert_eq!(texts.format_labels().date_order, DateOrder::DayMonthYear);
        assert_eq!(texts.format_labels().plural, PluralRule::ExactlyOne);
        let texts = Texts::from_sources(
            Language::En,
            "fmt-date-order = mdy\nfmt-plural-rule = below-two\n",
            None,
        );
        assert_eq!(texts.format_labels().plural, PluralRule::BelowTwo);
        // Valeur inconnue : repli « exactement un », sans panique.
        let texts = Texts::from_sources(Language::En, "fmt-plural-rule = dual\n", None);
        assert_eq!(texts.format_labels().plural, PluralRule::ExactlyOne);
    }

    /// Les libellés sont calculés une fois : deux appels rendent la même
    /// référence.
    #[test]
    fn i18n_format_labels_cached() {
        let texts = Texts::new(Language::Fr);
        assert!(std::ptr::eq(texts.format_labels(), texts.format_labels()));
    }

    /// Variables `$x` référencées par une expression en ligne.
    fn inline_vars(expression: &InlineExpression<&str>, out: &mut BTreeSet<String>) {
        match expression {
            InlineExpression::VariableReference { id } => {
                out.insert(id.name.to_owned());
            }
            InlineExpression::FunctionReference { arguments, .. } => {
                for arg in &arguments.positional {
                    inline_vars(arg, out);
                }
                for named in &arguments.named {
                    inline_vars(&named.value, out);
                }
            }
            InlineExpression::TermReference { arguments, .. } => {
                if let Some(arguments) = arguments {
                    for arg in &arguments.positional {
                        inline_vars(arg, out);
                    }
                    for named in &arguments.named {
                        inline_vars(&named.value, out);
                    }
                }
            }
            InlineExpression::Placeable { expression } => expression_vars(expression, out),
            InlineExpression::StringLiteral { .. }
            | InlineExpression::NumberLiteral { .. }
            | InlineExpression::MessageReference { .. } => {}
        }
    }

    /// Variables d'une expression (sélecteur et variantes compris).
    fn expression_vars(expression: &Expression<&str>, out: &mut BTreeSet<String>) {
        match expression {
            Expression::Select { selector, variants } => {
                inline_vars(selector, out);
                for variant in variants {
                    pattern_vars(&variant.value, out);
                }
            }
            Expression::Inline(inline) => inline_vars(inline, out),
        }
    }

    /// Variables d'un motif.
    fn pattern_vars(pattern: &Pattern<&str>, out: &mut BTreeSet<String>) {
        for element in &pattern.elements {
            if let PatternElement::Placeable { expression } = element {
                expression_vars(expression, out);
            }
        }
    }

    /// Variables de chaque message d'une source fluent.
    fn message_vars(source: &str) -> BTreeMap<String, BTreeSet<String>> {
        let resource = fluent_syntax::parser::parse(source).expect("analyse fluent");
        let mut map = BTreeMap::new();
        for entry in &resource.body {
            if let Entry::Message(message) = entry {
                let mut vars = BTreeSet::new();
                if let Some(value) = &message.value {
                    pattern_vars(value, &mut vars);
                }
                for attribute in &message.attributes {
                    pattern_vars(&attribute.value, &mut vars);
                }
                map.insert(message.id.name.to_owned(), vars);
            }
        }
        map
    }

    #[test]
    fn i18n_same_variables() {
        let fr = message_vars(FTL_FR);
        let en = message_vars(FTL_EN);
        assert_eq!(
            fr.keys().collect::<Vec<_>>(),
            en.keys().collect::<Vec<_>>(),
            "clés FR ≠ EN"
        );
        for (key, fr_vars) in &fr {
            assert_eq!(Some(fr_vars), en.get(key), "variables différentes pour « {key} »");
        }
        // Les arguments d'exemple couvrent toutes les variables utilisées.
        let known: BTreeSet<&str> = sample_args().iter().map(|(name, _)| *name).collect();
        for (key, vars) in &fr {
            for var in vars {
                assert!(
                    known.contains(var.as_str()),
                    "« {key} » : ${var} sans argument d'exemple"
                );
            }
        }
        // Les messages pluriels attendus utilisent bien `$count`.
        for key in ["status-tasks", "status-errors", "status-warnings"] {
            assert!(fr[key].contains("count"), "{key}");
        }
    }

    #[test]
    fn i18n_all_messages_format() {
        for language in Language::ALL {
            let texts = Texts::new(language);
            for count in [0_i64, 1, 2, 49] {
                let mut args = FluentArgs::new();
                for (name, arg) in sample_args() {
                    let value = match arg {
                        Arg::Str(text) => FluentValue::from(text.into_owned()),
                        Arg::Count(_) => FluentValue::from(count),
                    };
                    args.set(name, value);
                }
                for &id in TextId::ALL {
                    // Chaque message existe dans sa propre langue (sans repli).
                    let pattern = texts
                        .bundle
                        .get_message(id.key())
                        .and_then(|message| message.value())
                        .unwrap_or_else(|| panic!("{language:?} : « {} » absent", id.key()));
                    let mut errors = Vec::new();
                    let text = texts.bundle.format_pattern(pattern, Some(&args), &mut errors);
                    assert!(errors.is_empty(), "{language:?} {id:?} : {errors:?}");
                    assert!(!text.trim().is_empty(), "{language:?} {id:?} vide");
                    assert!(!text.contains('{'), "{language:?} {id:?} : {text:?}");
                }
            }
        }
    }

    #[test]
    fn i18n_missing_key_falls_back_then_raw_key() {
        let texts = Texts::from_sources(
            Language::En,
            "menu-file = File\n!!! erreur de syntaxe\nmenu-view = View\n",
            Some("menu-file = Fichier\nmenu-open = Ouvrir...\n"),
        );
        assert_eq!(texts.get(TextId::MenuFile), "File");
        // Ressource partielle utilisée malgré l'erreur d'analyse.
        assert_eq!(texts.get(TextId::MenuView), "View");
        // Repli français.
        assert_eq!(texts.get(TextId::MenuOpen), "Ouvrir...");
        // Clé brute, signalée une seule fois.
        for _ in 0..3 {
            assert_eq!(texts.get(TextId::MenuClose), "menu-close");
        }
        assert_eq!(
            texts.get_args(TextId::StatusTasks, &[("count", Arg::Count(2))]),
            "status-tasks"
        );
        // Les libellés des formateurs, résolus à la construction, ont aussi
        // signalé leurs clés absentes ; les clés présentes ne le sont pas.
        let missing = texts.missing.borrow();
        for id in [TextId::MenuClose, TextId::StatusTasks, TextId::FmtPluralRule] {
            assert!(missing.contains(&id), "{id:?} non signalé");
        }
        for id in [TextId::MenuFile, TextId::MenuView, TextId::MenuOpen] {
            assert!(!missing.contains(&id), "{id:?} signalé à tort");
        }
        // Règle de pluriel absente : repli « exactement un ».
        assert_eq!(texts.format_labels().plural, PluralRule::ExactlyOne);
    }

    #[test]
    fn i18n_missing_variable_does_not_panic() {
        let fr = Texts::new(Language::Fr);
        // Variable absente : fluent rend un repère, l'erreur est journalisée.
        let text = fr.get(TextId::RowLabel);
        assert!(text.starts_with("Ligne"), "{text:?}");
    }

    #[test]
    fn i18n_calendar_names() {
        let fr = Texts::new(Language::Fr).format_labels().clone();
        assert_eq!(
            fr.months_short,
            [
                "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
                "déc."
            ]
        );
        // Dimanche d'abord (ordre de `Weekday::ALL`).
        assert_eq!(
            fr.weekdays_short,
            ["dim", "lun", "mar", "mer", "jeu", "ven", "sam"]
        );
        assert_eq!(fr.weekday_initials, ["D", "L", "M", "M", "J", "V", "S"]);
        assert_eq!(fr.quarter_prefix, "T");
        assert_eq!(fr.months_long[7], "août");
        let en = Texts::new(Language::En).format_labels().clone();
        assert_eq!(
            en.months_short,
            [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"
            ]
        );
        assert_eq!(
            en.weekdays_short,
            ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
        );
        assert_eq!(en.weekday_initials, ["S", "M", "T", "W", "T", "F", "S"]);
        assert_eq!(en.quarter_prefix, "Q");
    }

    #[test]
    fn i18n_french_typography() {
        // Espace insécable U+00A0 (et non une espace ordinaire) avant « : ; ? ! »
        // dans les valeurs françaises ; les codes isolés (« ; », « ? ») sont
        // des conventions de formatage, pas du texte.
        for line in FTL_FR.lines() {
            let Some((key, value)) = line.split_once(" = ") else {
                continue;
            };
            if line.starts_with('#') || key.starts_with("fmt-") {
                continue;
            }
            for mark in [" :", " ;", " ?", " !"] {
                assert!(
                    !value.contains(mark),
                    "« {key} » : espace ordinaire avant ponctuation"
                );
            }
        }
    }

    #[test]
    fn language_system_none_is_fr() {
        assert_eq!(Language::resolve(LanguagePref::System, None), Language::Fr);
        assert_eq!(Language::from_tag("de-DE").unwrap_or(Language::Fr), Language::Fr);
        assert_eq!(
            Language::resolve(LanguagePref::System, Language::from_tag("de-DE")),
            Language::Fr
        );
    }

    #[test]
    fn language_from_tag() {
        assert_eq!(Language::from_tag("fr"), Some(Language::Fr));
        assert_eq!(Language::from_tag("fr-CA"), Some(Language::Fr));
        assert_eq!(Language::from_tag("en_US.UTF-8"), Some(Language::En));
        assert_eq!(Language::from_tag("EN"), Some(Language::En));
        assert_eq!(Language::from_tag("de"), None);
        assert_eq!(Language::from_tag("C"), None);
        assert_eq!(Language::from_tag(""), None);
    }

    #[test]
    fn language_resolve() {
        assert_eq!(Language::resolve(LanguagePref::System, None), Language::Fr);
        assert_eq!(
            Language::resolve(LanguagePref::System, Some(Language::En)),
            Language::En
        );
        assert_eq!(
            Language::resolve(LanguagePref::Fr, Some(Language::En)),
            Language::Fr
        );
        assert_eq!(Language::resolve(LanguagePref::En, None), Language::En);
    }
}
