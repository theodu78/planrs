//! Géométrie partagée du planning (ADR-0016 D5, H-UI-42, P55).
//!
//! Contrat posé par le lot L0, implémenté en L4 : un seul axe entier, les
//! rectangles de barre, le tracé des liens et le placement des textes, en
//! 1/16 px (`i64`). Le SVG et le PDF lisent la [`Scene`] construite par
//! [`layout`] ; l'écran appelle les mêmes fonctions ligne visible par ligne
//! visible. Aucune date n'est calculée ici (règle 3) : les instants viennent
//! du planning du moteur, les textes de [`BarTextSource`] (implémenté dans
//! `planrs-automation`).
//!
//! Squelettes : chaque fonction rend une valeur vide (aucune ligne, aucun
//! point) jusqu'à L4, jamais une panique.

use planrs_core::{LinkType, Moment, Project, Schedule, TaskUid};

use crate::axis::Axis;
pub use crate::metrics::{FontRef, FontWeight, InterMetrics, TextMeasure};
use crate::request::{Lang, RangeSpec, RenderReport, Zoom};
use crate::style::{BarField, BarRole, PlanStyle, Variant};

/// Axe temps → abscisse, en 1/16 px : l'axe entier d'`axis.rs`, seul axe du
/// dépôt (l'interface l'adopte en L11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Axis16 {
    #[allow(dead_code, reason = "contrat L0, lu par la géométrie de L4")]
    pub(crate) inner: Axis,
}

/// Rectangle en 1/16 px.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect16 {
    /// Abscisse du bord gauche.
    pub x: i64,
    /// Ordonnée du bord haut.
    pub y: i64,
    /// Largeur (≥ 0).
    pub w: i64,
    /// Hauteur (≥ 0).
    pub h: i64,
}

/// Entrée de la géométrie d'une ligne (valeurs du moteur, règle 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowGeomInput {
    /// Tâche.
    pub uid: TaskUid,
    /// Rôle de la barre.
    pub role: BarRole,
    /// Début planifié.
    pub start: Option<Moment>,
    /// Fin planifiée.
    pub finish: Option<Moment>,
    /// % achevé en centièmes (10 000 = 100 %).
    pub percent: u16,
    /// Ligne de base 0 (début, fin).
    pub baseline: Option<(Moment, Moment)>,
}

/// Géométrie d'une barre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarGeom {
    /// Rôle.
    pub role: BarRole,
    /// Rectangle englobant (losange compris).
    pub rect: Rect16,
    /// Bande d'avancement.
    pub progress: Option<Rect16>,
    /// Ligne de base.
    pub baseline: Option<Rect16>,
}

/// Tracé orthogonal d'un lien : au plus 6 points et la pointe.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkPath {
    /// Points en 1/16 px.
    pub points: Vec<(i64, i64)>,
    /// Pointe de flèche (trois sommets), si le tracé en a une.
    pub arrow: Option<[(i64, i64); 3]>,
}

/// Textes d'une barre, déjà résolus par la source (règle 3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedTexts {
    /// À gauche.
    pub left: Option<String>,
    /// À droite.
    pub right: Option<String>,
    /// Dedans.
    pub inside: Option<String>,
}

/// Texte placé (H-UI-46 : jamais rogné, tronqué avec « … »).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedText {
    /// Texte final (tronqué au besoin).
    pub text: String,
    /// Abscisse du début du texte.
    pub x16: i64,
    /// Ordonnée de la ligne de base.
    pub y16: i64,
    /// Largeur mesurée (H-R11).
    pub width16: i64,
    /// Police.
    pub font: FontRef,
}

/// Textes placés autour d'une barre.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlacedTexts {
    /// Dans l'ordre gauche, dedans, droite.
    pub items: Vec<PlacedText>,
}

/// Jeton d'en-tête ou de pied de page (H-UI-51).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token {
    /// `{projet}`.
    Project,
    /// `{date_etat}`.
    StatusDate,
    /// `{page}`.
    Page,
    /// `{pages}`.
    Pages,
    /// `{date_impression}`.
    PrintDate,
    /// `{fichier}`.
    File,
}

/// Source des textes affichés autour des barres et dans l'en-tête (règle
/// 3 : formatés par `planrs-automation`, jamais calculés ici).
pub trait BarTextSource {
    /// Texte du champ `f` de la tâche `uid` (`None` : rien à afficher).
    fn bar_text(&self, uid: TaskUid, f: BarField) -> Option<String>;
    /// Valeur d'un jeton d'en-tête ou de pied de page.
    fn token(&self, t: Token) -> String;
}

/// Fenêtre rendue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Viewport {
    /// Plage de dates.
    pub range: RangeSpec,
    /// Échelle.
    pub zoom: Zoom,
    /// Variante du style.
    pub variant: Variant,
    /// Langue des étiquettes.
    pub lang: Lang,
}

/// Ligne de la scène.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneRow {
    /// Tâche (`None` : ligne vide).
    pub uid: Option<TaskUid>,
    /// Ordonnée du haut de la ligne.
    pub y16: i64,
    /// Hauteur.
    pub h16: i64,
    /// Niveau hiérarchique.
    pub level: u16,
    /// Récapitulative (repère de la pagination, H-UI-48).
    pub summary: bool,
    /// Barre.
    pub bar: Option<BarGeom>,
    /// Textes placés.
    pub texts: PlacedTexts,
}

/// Scène du planning, lue par le SVG et le PDF.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scene {
    /// Largeur totale.
    pub width16: i64,
    /// Hauteur de l'en-tête (échelle).
    pub header16: i64,
    /// Lignes dans l'ordre d'affichage.
    pub rows: Vec<SceneRow>,
    /// Liens.
    pub links: Vec<LinkPath>,
}

/// Géométrie de la barre d'une ligne. Squelette L0 : rectangle vide.
#[must_use]
pub fn bar_geom(row: &RowGeomInput, axis: &Axis16, st: &PlanStyle, y16: i64) -> BarGeom {
    let _ = (axis, st);
    BarGeom {
        role: row.role,
        rect: Rect16 {
            y: y16,
            ..Rect16::default()
        },
        progress: None,
        baseline: None,
    }
}

/// Tracé d'un lien entre deux barres. Squelette L0 : aucun point.
#[must_use]
pub fn link_path(from: &BarGeom, to: &BarGeom, kind: LinkType) -> LinkPath {
    let _ = (from, to, kind);
    LinkPath::default()
}

/// Place les textes d'une barre dans la zone `clip` (abscisses). Squelette
/// L0 : rien de placé.
#[must_use]
pub fn place_texts(
    bar: &BarGeom,
    texts: &ResolvedTexts,
    m: &dyn TextMeasure,
    clip: (i64, i64),
) -> PlacedTexts {
    let _ = (bar, texts, m, clip);
    PlacedTexts::default()
}

/// Scène complète d'un projet. Squelette L0 : scène vide, rapport à zéro.
#[must_use]
pub fn layout(
    p: &Project,
    s: &Schedule,
    st: &PlanStyle,
    src: &dyn BarTextSource,
    vp: &Viewport,
) -> (Scene, RenderReport) {
    let _ = (p, s, st, src, vp);
    (Scene::default(), RenderReport::default())
}
