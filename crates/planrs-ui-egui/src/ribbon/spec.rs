//! Contenu du ruban (H-UI-70, `docs/plans/ribbon.md` § 2.2, ADR-0017 D3) :
//! table [`RIBBON`] onglet → groupes → éléments, lue par le moteur de
//! dessin (`view::ribbon`, lot L2).
//!
//! Chaque élément porte une [`RibbonCmd`] : [`RibbonCmd::action`] et
//! [`RibbonCmd::enabled`] sont **les seules** sources de l'action poussée
//! et de l'activation, partagées avec l'entrée jumelle de la barre de
//! menus (libellé [`RibbonCmd::menu_label`]) ; aucune fonction n'est
//! dupliquée, chaque commande pousse une [`Action`] existante.
//!
//! Lot L0 : table, actions et prédicats posés ; le lot L2 dessine, ajoute
//! les jumeaux manquants aux menus et y branche ces fonctions.

use planrs_automation::helpers::PercentStep;
use planrs_render::style::BarRole;

use super::icon_data::Icon;
use super::{Enabled, RibbonOp, RibbonTab};
use crate::action::{Action, DialogKind, FormatOp, TaskCmd, TimelineOp};
use crate::clipboard::ClipboardOp;
use crate::i18n::TextId;
use crate::io::SaveKind;
use crate::keymap::{Binding, UiBinding};
use crate::state::{GanttStyle, ViewKind};
use crate::structure::StructureOp;
use crate::view::FrameCtx;
use crate::view::side_panel::SideOp;
use crate::view::task_view::TaskViewOp;

/// Éléments au plus dans un groupe (règle 10 : plafond écrit).
pub const MAX_GROUP_ITEMS: usize = 64;

/// Taille et forme d'un élément (H-UI-71).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemSize {
    /// Grand bouton : icône 32 px, libellé dessous (deux lignes au plus).
    Large,
    /// Petit bouton : icône 16 px, libellé à droite, trois par colonne.
    Small,
    /// Bouton texte de la rangée 0 % … 100 % (libellé avec `$percent`).
    PercentRow,
    /// Liste déroulante (libellé + « ▾ ») : menu dessiné par le moteur.
    Dropdown,
    /// Case à cocher ou bascule (état reflété, H-UI-74).
    Check,
}

/// Élément du ruban.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemSpec {
    /// Taille et forme.
    pub size: ItemSize,
    /// Icône (aucune : rangée des pourcentages, cases).
    pub icon: Option<Icon>,
    /// Libellé (celui de l'entrée de menu quand il est identique).
    pub label: TextId,
    /// Phrase d'aide de l'info-bulle (H-UI-73).
    pub tip: TextId,
    /// Liaison de `KEYMAP` dont le raccourci est affiché (H-UI-73).
    pub binding: Option<Binding>,
    /// Commande.
    pub cmd: RibbonCmd,
}

/// Groupe nommé d'un onglet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupSpec {
    /// Nom du groupe (sous le groupe).
    pub label: TextId,
    /// Icône principale (groupe replié en bouton à menu, H-UI-75).
    pub icon: Icon,
    /// Éléments, dans l'ordre.
    pub items: &'static [ItemSpec],
    /// Rang de réduction quand la largeur manque (1 : réduit d'abord ;
    /// H-UI-75, de droite à gauche).
    pub shrink_rank: u8,
}

/// Commande d'un élément du ruban : une par élément distinct du § 2.2
/// (un même bouton présent sur deux onglets garde sa commande).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonCmd {
    /// Accès rapide : Enregistrer.
    Save,
    /// Accès rapide : Annuler.
    Undo,
    /// Accès rapide : Rétablir.
    Redo,
    /// Liste des vues (Tâche → Affichage).
    ViewMenu,
    /// Diagramme de Gantt.
    GanttChart,
    /// Gantt de suivi.
    TrackingGantt,
    /// Feuille des ressources.
    ResourceSheet,
    /// Utilisation des ressources.
    ResourceUsage,
    /// Coller.
    Paste,
    /// Couper.
    Cut,
    /// Copier.
    Copy,
    /// Collage spécial…
    PasteSpecial,
    /// Mettre en forme les barres (type de la tâche active).
    FormatBars,
    /// Galerie des modèles de mise en forme.
    Gallery,
    /// Marquer n % achevé (R-252).
    MarkPercent(PercentStep),
    /// Lier les tâches.
    Link,
    /// Délier.
    Unlink,
    /// Indenter (abaisser).
    Indent,
    /// Désindenter (hausser).
    Outdent,
    /// Désactiver (bascule, R-255).
    Inactivate,
    /// Planifier manuellement (bascule, R-253).
    ManualSchedule,
    /// Insérer une tâche (Inser).
    InsertTask,
    /// Insérer une récapitulative (H-UI-83).
    InsertSummary,
    /// Insérer un jalon (H-UI-83).
    InsertMilestone,
    /// Informations sur la tâche.
    TaskInformation,
    /// Notes de la tâche (H-UI-84).
    TaskNotes,
    /// Ajouter à la chronologie (bascule, H-UI-79).
    AddToTimeline,
    /// Rechercher.
    Find,
    /// Défiler jusqu'à la tâche (H-UI-84).
    ScrollToTask,
    /// Affecter les ressources.
    AssignResources,
    /// Informations sur le projet.
    ProjectInformation,
    /// Modifier le temps de travail.
    WorkingTime,
    /// Définir la référence.
    SetBaseline,
    /// Mettre à jour le projet (« Replanifier »).
    UpdateProject,
    /// Mode impression.
    PrintMode,
    /// Mise en page…
    PageSetup,
    /// Exporter en PDF…
    ExportPdf,
    /// Liste Trier (Trier…, Par ID).
    SortMenu,
    /// Liste Mode plan (niveaux, tous, masquer, afficher).
    OutlineMenu,
    /// Liste Filtre (préréglages, personnalisé, effacer).
    FilterMenu,
    /// Liste Échelle de temps (Jour … Trimestre).
    TimescaleMenu,
    /// Zoom avant.
    ZoomIn,
    /// Zoom arrière.
    ZoomOut,
    /// Projet entier.
    EntireProject,
    /// Case Chronologie (H-UI-78).
    TimelineVisible,
    /// Case Volet Opérations IA.
    AiPanel,
    /// Case Diagnostics.
    Diagnostics,
    /// Liste Thème.
    ThemeMenu,
    /// Liste Langue.
    LanguageMenu,
    /// Case Réduire le ruban (H-UI-72).
    CollapseRibbon,
    /// Liste Colonnes (une case par colonne).
    ColumnsMenu,
    /// Case Tâches critiques (surbrillance).
    CriticalTasks,
    /// Case Mois relatifs.
    RelativeMonths,
    /// Case Graphique seul.
    ChartOnly,
}

/// Pas de « marquer n % », dans l'ordre de la rangée (H-UI-35).
pub const PERCENT_STEPS: [(PercentStep, i64); 5] = [
    (PercentStep::P0, 0),
    (PercentStep::P25, 25),
    (PercentStep::P50, 50),
    (PercentStep::P75, 75),
    (PercentStep::P100, 100),
];

/// Valeur affichée d'un pas de pourcentage.
#[must_use]
pub const fn percent_value(step: PercentStep) -> i64 {
    match step {
        PercentStep::P0 => 0,
        PercentStep::P25 => 25,
        PercentStep::P50 => 50,
        PercentStep::P75 => 75,
        PercentStep::P100 => 100,
    }
}

impl RibbonCmd {
    /// Action poussée par un clic (celle du jumeau de menu) ; `None` pour
    /// une liste déroulante, dont le menu pousse lui-même ses actions.
    /// Les cases poussent la valeur inverse de leur état ([`Self::checked`]).
    #[must_use]
    pub fn action(self, fc: &FrameCtx<'_>) -> Option<Action> {
        Some(match self {
            Self::Save => Action::Save,
            Self::Undo => Action::Undo,
            Self::Redo => Action::Redo,
            Self::ViewMenu
            | Self::SortMenu
            | Self::OutlineMenu
            | Self::FilterMenu
            | Self::TimescaleMenu
            | Self::ThemeMenu
            | Self::LanguageMenu
            | Self::ColumnsMenu => return None,
            // Comme le menu Affichage : le style du Gantt ramène à la vue
            // Gantt ; le moteur pousse aussi `SetGanttStyle` (voir
            // [`Self::gantt_style`]).
            Self::GanttChart | Self::TrackingGantt => Action::SetView(ViewKind::Gantt),
            Self::ResourceSheet => Action::SetView(ViewKind::ResourceSheet),
            Self::ResourceUsage => Action::SetView(ViewKind::ResourceUsage),
            Self::Paste => Action::Clipboard(ClipboardOp::Paste),
            Self::Cut => Action::Clipboard(ClipboardOp::Cut),
            Self::Copy => Action::Clipboard(ClipboardOp::Copy),
            Self::PasteSpecial => Action::Clipboard(ClipboardOp::PasteSpecial),
            Self::FormatBars => format_bars_action(fc),
            Self::Gallery => Action::ShowDialog(DialogKind::Gallery),
            Self::MarkPercent(step) => Action::TaskCmd(TaskCmd::MarkPercent(step)),
            Self::Link => Action::Structure(StructureOp::Link),
            Self::Unlink => Action::Structure(StructureOp::Unlink),
            Self::Indent => Action::Structure(StructureOp::Indent),
            Self::Outdent => Action::Structure(StructureOp::Outdent),
            Self::Inactivate => Action::TaskCmd(TaskCmd::ToggleActive),
            Self::ManualSchedule => Action::TaskCmd(TaskCmd::ToggleMode),
            Self::InsertTask => Action::Structure(StructureOp::InsertBefore),
            Self::InsertSummary => Action::Structure(StructureOp::InsertSummary),
            Self::InsertMilestone => Action::Structure(StructureOp::InsertMilestone),
            Self::TaskInformation => Action::ShowTaskInformation,
            Self::TaskNotes => Action::ShowTaskNotes,
            Self::AddToTimeline => Action::Timeline(TimelineOp::ToggleSelection),
            Self::Find => Action::TaskView(TaskViewOp::OpenSearch),
            Self::ScrollToTask => Action::ScrollToTask,
            Self::AssignResources => Action::ShowDialog(DialogKind::AssignResources),
            Self::ProjectInformation => Action::ShowDialog(DialogKind::ProjectInformation),
            Self::WorkingTime => Action::ShowDialog(DialogKind::WorkingTime),
            Self::SetBaseline => Action::ShowDialog(DialogKind::Baseline),
            Self::UpdateProject => Action::ShowDialog(DialogKind::Reschedule),
            Self::PrintMode => Action::ShowDialog(DialogKind::PageSetup { print_mode: true }),
            Self::PageSetup => Action::ShowDialog(DialogKind::PageSetup { print_mode: false }),
            Self::ExportPdf => Action::Export(SaveKind::Pdf),
            Self::ZoomIn => Action::ZoomBy(1),
            Self::ZoomOut => Action::ZoomBy(-1),
            Self::EntireProject => Action::FitZoom,
            Self::TimelineVisible => Action::Timeline(TimelineOp::SetVisible(!fc.state.timeline_visible)),
            Self::AiPanel => Action::SidePanel(SideOp::Toggle),
            Self::Diagnostics => Action::ToggleDiagnostics,
            Self::CollapseRibbon => Action::Ribbon(RibbonOp::ToggleCollapsed),
            Self::CriticalTasks => Action::SetHighlightCritical(!fc.task_view.state.highlight_critical),
            Self::RelativeMonths => Action::Format(FormatOp::SetRelativeMonths(!relative_months(fc))),
            Self::ChartOnly => Action::SetChartOnly(!fc.state.chart_only),
        })
    }

    /// Style du Gantt poussé avec [`Action::SetView`] par les éléments de
    /// vue Gantt (comme le menu Affichage).
    #[must_use]
    pub const fn gantt_style(self) -> Option<GanttStyle> {
        match self {
            Self::GanttChart => Some(GanttStyle::Standard),
            Self::TrackingGantt => Some(GanttStyle::Tracking),
            _ => None,
        }
    }

    /// Activation, partagée avec le jumeau de menu (H-UI-70) ; grisé :
    /// la raison affichée (H-UI-73). Prédicats des menus de J8 : document
    /// ouvert, sélection, historique, verrou du tri et du filtre (H-UI-32).
    #[must_use]
    pub fn enabled(self, fc: &FrameCtx<'_>) -> Enabled {
        let history = fc.doc.map(|doc| doc.session.history());
        match self {
            Self::ThemeMenu
            | Self::LanguageMenu
            | Self::CollapseRibbon
            | Self::AiPanel
            | Self::Diagnostics
            | Self::TimelineVisible => return Enabled::YES,
            Self::Undo if history.as_ref().is_none_or(|h| h.entries.is_empty()) => {
                return Enabled::no(TextId::RibbonReasonNothingToUndo);
            }
            Self::Redo if history.as_ref().is_none_or(|h| h.redo.is_empty()) => {
                return Enabled::no(TextId::RibbonReasonNothingToRedo);
            }
            _ => {}
        }
        if fc.doc.is_none() {
            return Enabled::no(TextId::RibbonReasonNoDocument);
        }
        let locked = fc.task_view.state.reorder_locked();
        if locked
            && matches!(
                self,
                Self::Indent | Self::Outdent | Self::InsertSummary | Self::InsertMilestone
            )
        {
            return Enabled::no(TextId::ViewReorderLocked);
        }
        if self.needs_selection() && fc.selection.is_none() {
            return Enabled::no(TextId::RibbonReasonNoSelection);
        }
        Enabled::YES
    }

    /// Commande qui agit sur la sélection (grisée sans tâche active).
    #[must_use]
    pub const fn needs_selection(self) -> bool {
        matches!(
            self,
            Self::MarkPercent(_)
                | Self::Link
                | Self::Unlink
                | Self::Indent
                | Self::Outdent
                | Self::Inactivate
                | Self::ManualSchedule
                | Self::TaskInformation
                | Self::TaskNotes
                | Self::AddToTimeline
                | Self::ScrollToTask
                | Self::Cut
                | Self::Copy
        )
    }

    /// État d'une case ou d'une bascule (H-UI-74) ; `None` pour un bouton.
    /// Les bascules de tâche (Désactiver, Planifier manuellement, Ajouter à
    /// la chronologie) lisent la tâche active : lot L2.
    #[must_use]
    pub fn checked(self, fc: &FrameCtx<'_>) -> Option<bool> {
        match self {
            Self::TimelineVisible => Some(fc.state.timeline_visible),
            Self::AiPanel | Self::Inactivate | Self::ManualSchedule | Self::AddToTimeline => Some(false),
            Self::Diagnostics => Some(fc.state.diagnostics_open),
            Self::CollapseRibbon => Some(fc.state.ribbon.collapsed),
            Self::CriticalTasks => Some(fc.task_view.state.highlight_critical),
            Self::RelativeMonths => Some(relative_months(fc)),
            Self::ChartOnly => Some(fc.state.chart_only),
            _ => None,
        }
    }

    /// Libellé de l'entrée jumelle de la barre de menus (H-UI-70 ; jumeaux
    /// nouveaux ajoutés aux menus en L2).
    #[must_use]
    pub const fn menu_label(self) -> TextId {
        match self {
            Self::Save => TextId::MenuSave,
            Self::Undo => TextId::MenuUndo,
            Self::Redo => TextId::MenuRedo,
            Self::ViewMenu | Self::GanttChart => TextId::MenuGanttStandard,
            Self::TrackingGantt => TextId::MenuGanttTracking,
            Self::ResourceSheet => TextId::MenuViewResourceSheet,
            Self::ResourceUsage => TextId::MenuViewResourceUsage,
            Self::Paste => TextId::MenuPaste,
            Self::Cut => TextId::MenuCut,
            Self::Copy => TextId::MenuCopy,
            Self::PasteSpecial => TextId::MenuPasteSpecial,
            Self::FormatBars => TextId::MenuFormatBars,
            Self::Gallery => TextId::MenuGallery,
            Self::MarkPercent(_) => TextId::MenuMarkPercent,
            Self::Link => TextId::StructureLink,
            Self::Unlink => TextId::StructureUnlink,
            Self::Indent => TextId::StructureIndent,
            Self::Outdent => TextId::StructureOutdent,
            Self::Inactivate => TextId::MenuToggleActive,
            Self::ManualSchedule => TextId::MenuToggleMode,
            Self::InsertTask => TextId::StructureInsert,
            Self::InsertSummary => TextId::MenuInsertSummary,
            Self::InsertMilestone => TextId::MenuInsertMilestone,
            Self::TaskInformation => TextId::TaskInfoMenu,
            Self::TaskNotes => TextId::MenuTaskNotes,
            Self::AddToTimeline => TextId::TimelineAdd,
            Self::Find => TextId::MenuFind,
            Self::ScrollToTask => TextId::RibbonScrollToTask,
            Self::AssignResources => TextId::MenuAssignResources,
            Self::ProjectInformation => TextId::MenuProjectInfo,
            Self::WorkingTime => TextId::MenuWorkingTime,
            Self::SetBaseline => TextId::MenuBaseline,
            Self::UpdateProject => TextId::MenuReschedule,
            Self::PrintMode => TextId::MenuPrintMode,
            Self::PageSetup => TextId::MenuPageSetup,
            Self::ExportPdf => TextId::MenuExportPdf,
            Self::SortMenu => TextId::RibbonSort,
            Self::OutlineMenu => TextId::RibbonOutline,
            Self::FilterMenu => TextId::RibbonFilter,
            Self::TimescaleMenu => TextId::MenuZoom,
            Self::ZoomIn => TextId::RibbonZoomIn,
            Self::ZoomOut => TextId::RibbonZoomOut,
            Self::EntireProject => TextId::ToolbarFit,
            Self::TimelineVisible => TextId::RibbonTimeline,
            Self::AiPanel => TextId::MenuAiPanel,
            Self::Diagnostics => TextId::MenuDiagnostics,
            Self::ThemeMenu => TextId::MenuTheme,
            Self::LanguageMenu => TextId::MenuLanguage,
            Self::CollapseRibbon => TextId::RibbonCollapse,
            Self::ColumnsMenu => TextId::RibbonColumns,
            Self::CriticalTasks => TextId::MenuHighlightCritical,
            Self::RelativeMonths => TextId::MenuRelativeMonths,
            Self::ChartOnly => TextId::MenuChartOnly,
        }
    }
}

/// « Mois relatifs » du style effectif du document (faux sans document).
fn relative_months(fc: &FrameCtx<'_>) -> bool {
    fc.doc
        .is_some_and(|doc| fc.state.effective_style(doc.project()).scale.relative_months)
}

/// « Mettre en forme les barres » : type de la barre de la tâche active,
/// sinon les barres de tâches (même règle que le menu Format, que L2 fait
/// passer par cette fonction).
fn format_bars_action(fc: &FrameCtx<'_>) -> Action {
    let role = fc
        .selection
        .and_then(|uid| {
            let rows = fc.visible()?;
            let row = (0..rows.len()).find_map(|i| {
                rows.row(crate::row_view::DisplayRow::at(i))
                    .filter(|r| r.uid == uid)
            })?;
            crate::view::gantt::row_role(row, fc.task_view.state.highlight_critical)
        })
        .unwrap_or(BarRole::Task);
    Action::Format(FormatOp::OpenBars {
        role,
        task: fc.selection,
    })
}

impl RibbonTab {
    /// Libellé de l'onglet (S80 : Task, Resource, Project, View, Format).
    #[must_use]
    pub const fn label(self) -> TextId {
        match self {
            Self::Task => TextId::RibbonTabTask,
            Self::Resource => TextId::RibbonTabResource,
            Self::Project => TextId::RibbonTabProject,
            Self::View => TextId::RibbonTabView,
            Self::Format => TextId::RibbonTabFormat,
        }
    }

    /// Groupes de l'onglet, dans l'ordre ([`RIBBON`]).
    #[must_use]
    pub fn groups(self) -> &'static [GroupSpec] {
        RIBBON
            .iter()
            .find(|(tab, _)| *tab == self)
            .map_or(&[], |(_, groups)| *groups)
    }
}

/// Élément d'une ligne de la table.
const fn item(
    size: ItemSize,
    icon: Option<Icon>,
    label: TextId,
    tip: TextId,
    binding: Option<Binding>,
    cmd: RibbonCmd,
) -> ItemSpec {
    ItemSpec {
        size,
        icon,
        label,
        tip,
        binding,
        cmd,
    }
}

/// Élément sans raccourci.
const fn plain(size: ItemSize, icon: Option<Icon>, label: TextId, tip: TextId, cmd: RibbonCmd) -> ItemSpec {
    item(size, icon, label, tip, None, cmd)
}

/// Liaison d'une commande du registre.
const fn command(id: &'static str) -> Binding {
    Binding::Command(id)
}

/// Liaison propre à l'interface.
const fn ui(binding: UiBinding) -> Binding {
    Binding::Ui(binding)
}

use ItemSize::{Check, Dropdown, Large, PercentRow, Small};

/// Barre d'accès rapide (H-UI-76) : icônes seules, à gauche des onglets.
pub const QUICK_ACCESS: &[ItemSpec] = &[
    item(
        Small,
        Some(Icon::Save),
        TextId::MenuSave,
        TextId::RibbonTipSave,
        Some(command("project.save")),
        RibbonCmd::Save,
    ),
    item(
        Small,
        Some(Icon::Undo2),
        TextId::MenuUndo,
        TextId::RibbonTipUndo,
        Some(command("history.undo")),
        RibbonCmd::Undo,
    ),
    item(
        Small,
        Some(Icon::Redo2),
        TextId::MenuRedo,
        TextId::RibbonTipRedo,
        Some(command("history.redo")),
        RibbonCmd::Redo,
    ),
];

/// Vues du Gantt et des ressources (onglets Tâche, Ressource, Affichage).
const GANTT_CHART: ItemSpec = plain(
    Large,
    Some(Icon::ChartGantt),
    TextId::MenuGanttStandard,
    TextId::RibbonTipGanttChart,
    RibbonCmd::GanttChart,
);
const TRACKING_GANTT: ItemSpec = plain(
    Large,
    Some(Icon::ChartNoAxesGantt),
    TextId::MenuGanttTracking,
    TextId::RibbonTipTrackingGantt,
    RibbonCmd::TrackingGantt,
);
const RESOURCE_SHEET_LARGE: ItemSpec = plain(
    Large,
    Some(Icon::Users),
    TextId::MenuViewResourceSheet,
    TextId::RibbonTipResourceSheet,
    RibbonCmd::ResourceSheet,
);
const RESOURCE_USAGE_LARGE: ItemSpec = plain(
    Large,
    Some(Icon::ChartColumn),
    TextId::MenuViewResourceUsage,
    TextId::RibbonTipResourceUsage,
    RibbonCmd::ResourceUsage,
);
const FORMAT_BARS: ItemSpec = plain(
    Large,
    Some(Icon::Paintbrush),
    TextId::MenuFormatBars,
    TextId::RibbonTipFormatBars,
    RibbonCmd::FormatBars,
);

/// Pourcentage de la rangée.
const fn percent(step: PercentStep) -> ItemSpec {
    plain(
        PercentRow,
        None,
        TextId::RibbonPercent,
        TextId::MenuMarkPercent,
        RibbonCmd::MarkPercent(step),
    )
}

/// Onglet Tâche.
const TASK_GROUPS: &[GroupSpec] = &[
    GroupSpec {
        label: TextId::RibbonGroupView,
        icon: Icon::ChartGantt,
        items: &[plain(
            Dropdown,
            Some(Icon::ChartGantt),
            TextId::MenuGanttStandard,
            TextId::RibbonTipViewMenu,
            RibbonCmd::ViewMenu,
        )],
        shrink_rank: 8,
    },
    GroupSpec {
        label: TextId::RibbonGroupClipboard,
        icon: Icon::ClipboardPaste,
        items: &[
            plain(
                Large,
                Some(Icon::ClipboardPaste),
                TextId::MenuPaste,
                TextId::RibbonTipPaste,
                RibbonCmd::Paste,
            ),
            plain(
                Small,
                Some(Icon::Scissors),
                TextId::MenuCut,
                TextId::RibbonTipCut,
                RibbonCmd::Cut,
            ),
            plain(
                Small,
                Some(Icon::Copy),
                TextId::MenuCopy,
                TextId::RibbonTipCopy,
                RibbonCmd::Copy,
            ),
            plain(
                Small,
                Some(Icon::ClipboardList),
                TextId::MenuPasteSpecial,
                TextId::RibbonTipPasteSpecial,
                RibbonCmd::PasteSpecial,
            ),
        ],
        shrink_rank: 7,
    },
    GroupSpec {
        label: TextId::RibbonGroupFormatting,
        icon: Icon::Paintbrush,
        items: &[
            FORMAT_BARS,
            plain(
                Small,
                Some(Icon::SwatchBook),
                TextId::RibbonGalleryShort,
                TextId::RibbonTipGallery,
                RibbonCmd::Gallery,
            ),
        ],
        shrink_rank: 6,
    },
    GroupSpec {
        label: TextId::RibbonGroupSchedule,
        icon: Icon::Link,
        items: &[
            percent(PercentStep::P0),
            percent(PercentStep::P25),
            percent(PercentStep::P50),
            percent(PercentStep::P75),
            percent(PercentStep::P100),
            item(
                Small,
                Some(Icon::Link),
                TextId::RibbonLink,
                TextId::RibbonTipLink,
                Some(command("link.add")),
                RibbonCmd::Link,
            ),
            item(
                Small,
                Some(Icon::Unlink),
                TextId::RibbonUnlink,
                TextId::RibbonTipUnlink,
                Some(command("link.remove")),
                RibbonCmd::Unlink,
            ),
            item(
                Small,
                Some(Icon::IndentIncrease),
                TextId::RibbonIndent,
                TextId::RibbonTipIndent,
                Some(command("task.indent")),
                RibbonCmd::Indent,
            ),
            item(
                Small,
                Some(Icon::IndentDecrease),
                TextId::RibbonOutdent,
                TextId::RibbonTipOutdent,
                Some(command("task.outdent")),
                RibbonCmd::Outdent,
            ),
            plain(
                Check,
                Some(Icon::CircleOff),
                TextId::RibbonInactivate,
                TextId::RibbonTipInactivate,
                RibbonCmd::Inactivate,
            ),
        ],
        shrink_rank: 5,
    },
    GroupSpec {
        label: TextId::RibbonGroupTasks,
        icon: Icon::Pin,
        items: &[plain(
            Large,
            Some(Icon::Pin),
            TextId::RibbonManualSchedule,
            TextId::RibbonTipManualSchedule,
            RibbonCmd::ManualSchedule,
        )],
        shrink_rank: 4,
    },
    GroupSpec {
        label: TextId::RibbonGroupInsert,
        icon: Icon::ListPlus,
        items: &[
            item(
                Large,
                Some(Icon::ListPlus),
                TextId::RibbonInsertTask,
                TextId::RibbonTipInsertTask,
                Some(command("task.add")),
                RibbonCmd::InsertTask,
            ),
            plain(
                Small,
                Some(Icon::ListTree),
                TextId::RibbonInsertSummary,
                TextId::RibbonTipInsertSummary,
                RibbonCmd::InsertSummary,
            ),
            plain(
                Small,
                Some(Icon::Diamond),
                TextId::RibbonInsertMilestone,
                TextId::RibbonTipInsertMilestone,
                RibbonCmd::InsertMilestone,
            ),
        ],
        shrink_rank: 3,
    },
    GroupSpec {
        label: TextId::RibbonGroupProperties,
        icon: Icon::Info,
        items: &[
            item(
                Large,
                Some(Icon::Info),
                TextId::RibbonInformation,
                TextId::RibbonTipInformation,
                Some(ui(UiBinding::TaskInformation)),
                RibbonCmd::TaskInformation,
            ),
            plain(
                Small,
                Some(Icon::NotebookPen),
                TextId::RibbonNotes,
                TextId::RibbonTipNotes,
                RibbonCmd::TaskNotes,
            ),
            plain(
                Check,
                Some(Icon::Timeline),
                TextId::TimelineAdd,
                TextId::RibbonTipAddToTimeline,
                RibbonCmd::AddToTimeline,
            ),
        ],
        shrink_rank: 2,
    },
    GroupSpec {
        label: TextId::RibbonGroupEditing,
        icon: Icon::Search,
        items: &[
            item(
                Large,
                Some(Icon::Search),
                TextId::RibbonFind,
                TextId::RibbonTipFind,
                Some(ui(UiBinding::Find)),
                RibbonCmd::Find,
            ),
            plain(
                Small,
                Some(Icon::Locate),
                TextId::RibbonScrollToTask,
                TextId::RibbonTipScrollToTask,
                RibbonCmd::ScrollToTask,
            ),
        ],
        shrink_rank: 1,
    },
];

/// Onglet Ressource.
const RESOURCE_GROUPS: &[GroupSpec] = &[
    GroupSpec {
        label: TextId::RibbonGroupView,
        icon: Icon::Users,
        items: &[RESOURCE_SHEET_LARGE, RESOURCE_USAGE_LARGE],
        shrink_rank: 2,
    },
    GroupSpec {
        label: TextId::RibbonGroupAssignments,
        icon: Icon::UserPlus,
        items: &[plain(
            Large,
            Some(Icon::UserPlus),
            TextId::MenuAssignResources,
            TextId::RibbonTipAssignResources,
            RibbonCmd::AssignResources,
        )],
        shrink_rank: 1,
    },
];

/// Onglet Projet.
const PROJECT_GROUPS: &[GroupSpec] = &[
    GroupSpec {
        label: TextId::RibbonGroupProperties,
        icon: Icon::FileText,
        items: &[
            plain(
                Large,
                Some(Icon::FileText),
                TextId::MenuProjectInfo,
                TextId::RibbonTipProjectInformation,
                RibbonCmd::ProjectInformation,
            ),
            plain(
                Large,
                Some(Icon::CalendarCog),
                TextId::MenuWorkingTime,
                TextId::RibbonTipWorkingTime,
                RibbonCmd::WorkingTime,
            ),
        ],
        shrink_rank: 4,
    },
    GroupSpec {
        label: TextId::RibbonGroupSchedule,
        icon: Icon::CalendarCheck,
        items: &[plain(
            Large,
            Some(Icon::CalendarCheck),
            TextId::RibbonSetBaseline,
            TextId::RibbonTipSetBaseline,
            RibbonCmd::SetBaseline,
        )],
        shrink_rank: 3,
    },
    GroupSpec {
        label: TextId::RibbonGroupStatus,
        icon: Icon::RefreshCw,
        items: &[plain(
            Large,
            Some(Icon::RefreshCw),
            TextId::RibbonUpdateProject,
            TextId::RibbonTipUpdateProject,
            RibbonCmd::UpdateProject,
        )],
        shrink_rank: 2,
    },
    GroupSpec {
        label: TextId::RibbonGroupPrint,
        icon: Icon::Printer,
        items: &[
            plain(
                Large,
                Some(Icon::Printer),
                TextId::MenuPrintMode,
                TextId::RibbonTipPrintMode,
                RibbonCmd::PrintMode,
            ),
            plain(
                Small,
                Some(Icon::FileCog),
                TextId::MenuPageSetup,
                TextId::RibbonTipPageSetup,
                RibbonCmd::PageSetup,
            ),
            plain(
                Small,
                Some(Icon::FileDown),
                TextId::RibbonExportPdf,
                TextId::RibbonTipExportPdf,
                RibbonCmd::ExportPdf,
            ),
        ],
        shrink_rank: 1,
    },
];

/// Onglet Affichage.
const VIEW_GROUPS: &[GroupSpec] = &[
    GroupSpec {
        label: TextId::RibbonGroupTaskViews,
        icon: Icon::ChartGantt,
        items: &[GANTT_CHART, TRACKING_GANTT],
        shrink_rank: 6,
    },
    GroupSpec {
        label: TextId::RibbonGroupResourceViews,
        icon: Icon::Users,
        items: &[
            plain(
                Small,
                Some(Icon::Users),
                TextId::MenuViewResourceSheet,
                TextId::RibbonTipResourceSheet,
                RibbonCmd::ResourceSheet,
            ),
            plain(
                Small,
                Some(Icon::ChartColumn),
                TextId::MenuViewResourceUsage,
                TextId::RibbonTipResourceUsage,
                RibbonCmd::ResourceUsage,
            ),
        ],
        shrink_rank: 5,
    },
    GroupSpec {
        label: TextId::RibbonGroupData,
        icon: Icon::Filter,
        items: &[
            plain(
                Dropdown,
                Some(Icon::ArrowUpDown),
                TextId::RibbonSort,
                TextId::RibbonTipSort,
                RibbonCmd::SortMenu,
            ),
            plain(
                Dropdown,
                Some(Icon::ChevronsDownUp),
                TextId::RibbonOutline,
                TextId::RibbonTipOutline,
                RibbonCmd::OutlineMenu,
            ),
            plain(
                Dropdown,
                Some(Icon::Filter),
                TextId::RibbonFilter,
                TextId::RibbonTipFilter,
                RibbonCmd::FilterMenu,
            ),
        ],
        shrink_rank: 4,
    },
    GroupSpec {
        label: TextId::RibbonGroupZoom,
        icon: Icon::Maximize,
        items: &[
            plain(
                Dropdown,
                Some(Icon::CalendarRange),
                TextId::RibbonTimescale,
                TextId::RibbonTipTimescale,
                RibbonCmd::TimescaleMenu,
            ),
            item(
                Small,
                Some(Icon::ZoomIn),
                TextId::RibbonZoomIn,
                TextId::RibbonTipZoomIn,
                Some(ui(UiBinding::ZoomFiner)),
                RibbonCmd::ZoomIn,
            ),
            item(
                Small,
                Some(Icon::ZoomOut),
                TextId::RibbonZoomOut,
                TextId::RibbonTipZoomOut,
                Some(ui(UiBinding::ZoomCoarser)),
                RibbonCmd::ZoomOut,
            ),
            plain(
                Large,
                Some(Icon::Maximize),
                TextId::RibbonEntireProject,
                TextId::RibbonTipEntireProject,
                RibbonCmd::EntireProject,
            ),
        ],
        shrink_rank: 3,
    },
    GroupSpec {
        label: TextId::RibbonGroupSplitView,
        icon: Icon::Timeline,
        items: &[
            plain(
                Check,
                None,
                TextId::RibbonTimeline,
                TextId::RibbonTipTimeline,
                RibbonCmd::TimelineVisible,
            ),
            plain(
                Check,
                None,
                TextId::MenuAiPanel,
                TextId::RibbonTipAiPanel,
                RibbonCmd::AiPanel,
            ),
            plain(
                Check,
                None,
                TextId::MenuDiagnostics,
                TextId::RibbonTipDiagnostics,
                RibbonCmd::Diagnostics,
            ),
        ],
        shrink_rank: 2,
    },
    GroupSpec {
        label: TextId::RibbonGroupInterface,
        icon: Icon::SunMoon,
        items: &[
            plain(
                Dropdown,
                Some(Icon::SunMoon),
                TextId::MenuTheme,
                TextId::RibbonTipTheme,
                RibbonCmd::ThemeMenu,
            ),
            plain(
                Dropdown,
                Some(Icon::Languages),
                TextId::MenuLanguage,
                TextId::RibbonTipLanguage,
                RibbonCmd::LanguageMenu,
            ),
            item(
                Check,
                Some(Icon::ChevronsUpDown),
                TextId::RibbonCollapse,
                TextId::RibbonTipCollapse,
                Some(ui(UiBinding::ToggleRibbon)),
                RibbonCmd::CollapseRibbon,
            ),
        ],
        shrink_rank: 1,
    },
];

/// Onglet Format.
const FORMAT_GROUPS: &[GroupSpec] = &[
    GroupSpec {
        label: TextId::RibbonGroupColumns,
        icon: Icon::Columns3,
        items: &[plain(
            Dropdown,
            Some(Icon::Columns3),
            TextId::RibbonColumns,
            TextId::RibbonTipColumns,
            RibbonCmd::ColumnsMenu,
        )],
        shrink_rank: 4,
    },
    GroupSpec {
        label: TextId::RibbonGroupBarStyles,
        icon: Icon::Paintbrush,
        items: &[
            FORMAT_BARS,
            plain(
                Check,
                None,
                TextId::RibbonCriticalTasks,
                TextId::RibbonTipCriticalTasks,
                RibbonCmd::CriticalTasks,
            ),
            plain(
                Check,
                None,
                TextId::MenuRelativeMonths,
                TextId::RibbonTipRelativeMonths,
                RibbonCmd::RelativeMonths,
            ),
        ],
        shrink_rank: 3,
    },
    GroupSpec {
        label: TextId::RibbonGroupChartStyle,
        icon: Icon::SwatchBook,
        items: &[plain(
            Large,
            Some(Icon::SwatchBook),
            TextId::MenuGallery,
            TextId::RibbonTipGallery,
            RibbonCmd::Gallery,
        )],
        shrink_rank: 2,
    },
    GroupSpec {
        label: TextId::RibbonGroupShowHide,
        icon: Icon::PanelLeftClose,
        items: &[plain(
            Check,
            Some(Icon::PanelLeftClose),
            TextId::MenuChartOnly,
            TextId::RibbonTipChartOnly,
            RibbonCmd::ChartOnly,
        )],
        shrink_rank: 1,
    },
];

/// Contenu du ruban : onglet → groupes, dans l'ordre d'affichage (§ 2.2).
pub const RIBBON: &[(RibbonTab, &[GroupSpec])] = &[
    (RibbonTab::Task, TASK_GROUPS),
    (RibbonTab::Resource, RESOURCE_GROUPS),
    (RibbonTab::Project, PROJECT_GROUPS),
    (RibbonTab::View, VIEW_GROUPS),
    (RibbonTab::Format, FORMAT_GROUPS),
];

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use std::collections::BTreeSet;

    use super::{ItemSize, MAX_GROUP_ITEMS, PERCENT_STEPS, QUICK_ACCESS, RIBBON, percent_value};
    use crate::i18n::{Language, TextId, Texts};
    use crate::keymap::{KEYMAP, binding_action};
    use crate::ribbon::RibbonTab;

    /// Tous les textes de la table (onglets, groupes, libellés, aides,
    /// libellés des jumeaux).
    fn spec_text_ids() -> BTreeSet<TextId> {
        let mut ids = BTreeSet::new();
        for (tab, groups) in RIBBON {
            ids.insert(tab.label());
            for group in *groups {
                ids.insert(group.label);
                for item in group.items {
                    ids.extend([item.label, item.tip, item.cmd.menu_label()]);
                }
            }
        }
        for item in QUICK_ACCESS {
            ids.extend([item.label, item.tip, item.cmd.menu_label()]);
        }
        ids
    }

    /// ADR-0017 (lot L0) : chaque texte de la table existe en français et
    /// en anglais (non vide, sans clé brute).
    #[test]
    fn ribbon_spec_text_ids_exist() {
        let ids = spec_text_ids();
        assert!(ids.len() > 60, "{}", ids.len());
        for language in [Language::Fr, Language::En] {
            let texts = Texts::new(language);
            for id in &ids {
                let text = texts.get(*id);
                assert!(!text.is_empty() && text != id.key(), "{language:?} : {id:?}");
            }
        }
    }

    /// H-UI-69 : cinq onglets dans l'ordre, groupes non vides, plafonds
    /// écrits ; rangs de réduction distincts par onglet (H-UI-75).
    #[test]
    fn ribbon_spec_shape() {
        let tabs: Vec<RibbonTab> = RIBBON.iter().map(|(tab, _)| *tab).collect();
        assert_eq!(tabs, RibbonTab::ALL);
        for (tab, groups) in RIBBON {
            assert!(!groups.is_empty(), "{tab:?}");
            assert_eq!(tab.groups(), *groups);
            let ranks: BTreeSet<u8> = groups.iter().map(|g| g.shrink_rank).collect();
            assert_eq!(ranks.len(), groups.len(), "{tab:?}");
            for group in *groups {
                assert!(!group.items.is_empty() && group.items.len() <= MAX_GROUP_ITEMS);
                for item in group.items {
                    // Grands et petits boutons ont une icône (H-UI-77).
                    if matches!(item.size, ItemSize::Large | ItemSize::Small) {
                        assert!(item.icon.is_some(), "{:?}", item.cmd);
                    }
                }
            }
        }
        assert_eq!(QUICK_ACCESS.len(), 3);
    }

    /// Raccourcis affichés : liaisons présentes dans `KEYMAP`, et leur
    /// action est celle de l'élément quand elle ne dépend pas de l'état.
    #[test]
    fn ribbon_spec_bindings_in_keymap() {
        for item in RIBBON
            .iter()
            .flat_map(|(_, groups)| groups.iter())
            .flat_map(|g| g.items.iter())
            .chain(QUICK_ACCESS)
        {
            if let Some(binding) = item.binding {
                assert!(KEYMAP.iter().any(|(_, _, b)| *b == binding), "{binding:?}");
                assert!(binding_action(binding).is_some(), "{binding:?}");
            }
        }
    }

    #[test]
    fn ribbon_percent_values() {
        for (step, pct) in PERCENT_STEPS {
            assert_eq!(percent_value(step), pct);
        }
    }
}
