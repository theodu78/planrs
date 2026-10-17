//! Échelle de temps : zoom discret, plage de jours, projection temps → pixels
//! et cellules de l'en-tête à deux niveaux.
//!
//! Sens unique : un `Moment` donne une abscisse, jamais l'inverse (aucune date
//! produite à partir d'un pixel, règle 3). L'écart à l'origine se calcule en
//! minutes entières (`i64`), converti une seule fois en `f32` pour le dessin.

use planrs_core::{Day, Moment, Project, Schedule, Weekday};
use serde::{Deserialize, Serialize};

use crate::format::FormatLabels;
use crate::geom::i64_to_f32;
use crate::i18n::TextId;

/// Minutes par jour civil.
const MINUTES_PER_DAY: i64 = 1_440;

/// Zoom discret de l'échelle de temps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeScaleZoom {
    /// Semaines / jours (36 px par jour).
    Day,
    /// Mois / semaines (12 px par jour).
    #[default]
    Week,
    /// Trimestres / mois (3,5 px par jour).
    Month,
    /// Années / trimestres (1,2 px par jour).
    Quarter,
}

impl TimeScaleZoom {
    /// Tous les zooms, du plus fin au plus grossier.
    pub const ALL: [Self; 4] = [Self::Day, Self::Week, Self::Month, Self::Quarter];

    /// Pixels logiques par jour civil.
    #[must_use]
    pub const fn px_per_day(self) -> f32 {
        match self {
            Self::Day => 36.0,
            Self::Week => 12.0,
            Self::Month => 3.5,
            Self::Quarter => 1.2,
        }
    }

    /// Unités des deux niveaux de l'en-tête : (haut, bas).
    #[must_use]
    pub const fn tiers(self) -> (TierUnit, TierUnit) {
        match self {
            Self::Day => (TierUnit::Week, TierUnit::Day),
            Self::Week => (TierUnit::Month, TierUnit::Week),
            Self::Month => (TierUnit::Quarter, TierUnit::Month),
            Self::Quarter => (TierUnit::Year, TierUnit::Quarter),
        }
    }

    /// Zoom immédiatement plus fin (borné à [`TimeScaleZoom::Day`]).
    #[must_use]
    pub const fn finer(self) -> Self {
        match self {
            Self::Day | Self::Week => Self::Day,
            Self::Month => Self::Week,
            Self::Quarter => Self::Month,
        }
    }

    /// Zoom immédiatement plus grossier (borné à
    /// [`TimeScaleZoom::Quarter`]).
    #[must_use]
    pub const fn coarser(self) -> Self {
        match self {
            Self::Day => Self::Week,
            Self::Week => Self::Month,
            Self::Month | Self::Quarter => Self::Quarter,
        }
    }

    /// Libellé traduit (boutons et menu de zoom).
    #[must_use]
    pub const fn label(self) -> TextId {
        match self {
            Self::Day => TextId::ZoomDay,
            Self::Week => TextId::ZoomWeek,
            Self::Month => TextId::ZoomMonth,
            Self::Quarter => TextId::ZoomQuarter,
        }
    }
}

/// Unité d'un niveau de l'en-tête.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TierUnit {
    /// Jour.
    Day,
    /// Semaine (commence au `week_start_day` du projet).
    Week,
    /// Mois civil.
    Month,
    /// Trimestre civil.
    Quarter,
    /// Année civile.
    Year,
}

/// Plage de jours affichée, fin exclusive (minuit du jour suivant le dernier
/// jour).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayRange {
    /// Premier jour (x = 0 au minuit de ce jour).
    pub start: Day,
    /// Jour suivant le dernier jour affiché.
    pub end: Day,
}

/// Plage par défaut d'un document (calculée à l'ouverture, entiers
/// seulement).
///
/// Corps provisoire : du jour de début au lendemain du jour de fin du
/// projet. Règle définitive (ADR-0011, P8) : début − 7 jours reculé au début
/// de semaine, fin + 21 jours, étendue à la date d'état, plafonnée à 20 ans.
#[must_use]
pub fn default_range(_project: &Project, schedule: &Schedule) -> DayRange {
    let start = schedule.project_start.day();
    let last = schedule.project_finish.day().max(start);
    DayRange {
        start,
        end: last.checked_add_days(1).unwrap_or(last),
    }
}

/// Zoom ajusté à l'ouverture : `Week` si toute la plage tient dans
/// `gantt_width_px`, sinon `Month` (ADR-0011, P9).
///
/// Corps provisoire : `Week`.
#[must_use]
pub fn fit_zoom(_range: DayRange, _gantt_width_px: f32) -> TimeScaleZoom {
    TimeScaleZoom::Week
}

/// Minute de `Moment` du minuit d'un jour.
fn day_minutes(d: Day) -> i64 {
    i64::from(d.index()).saturating_mul(MINUTES_PER_DAY)
}

/// Projection temps → pixels, figée pour une image. Aucune méthode inverse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeAxis {
    /// Minute du début de la plage (minuit de `range.start`).
    origin_minutes: i64,
    /// Minute de la fin exclusive de la plage.
    end_minutes: i64,
    /// Pixels par minute (`px_per_day / 1440`).
    px_per_minute: f32,
    /// Abscisse écran du début de la plage (défilement horizontal inclus).
    left: f32,
}

impl TimeAxis {
    /// Axe de la plage `range` au zoom `zoom`, le début de la plage étant à
    /// l'abscisse écran `left`.
    #[must_use]
    pub fn new(range: DayRange, zoom: TimeScaleZoom, left: f32) -> Self {
        let origin_minutes = day_minutes(range.start);
        let end_minutes = day_minutes(range.end).max(origin_minutes);
        Self {
            origin_minutes,
            end_minutes,
            px_per_minute: zoom.px_per_day() / i64_to_f32(MINUTES_PER_DAY),
            left,
        }
    }

    /// Abscisse écran d'une minute absolue, bornée à la plage.
    fn x_minutes(&self, minutes: i64) -> f32 {
        let delta = minutes
            .clamp(self.origin_minutes, self.end_minutes)
            .saturating_sub(self.origin_minutes);
        self.left + i64_to_f32(delta) * self.px_per_minute
    }

    /// Abscisse écran d'un instant (borné à la plage).
    #[must_use]
    pub fn x(&self, m: Moment) -> f32 {
        self.x_minutes(m.minutes())
    }

    /// Abscisse écran du minuit d'un jour (borné à la plage).
    #[must_use]
    pub fn x_day(&self, d: Day) -> f32 {
        self.x_minutes(day_minutes(d))
    }

    /// Largeur totale du contenu, en pixels.
    #[must_use]
    pub fn content_width(&self) -> f32 {
        i64_to_f32(self.end_minutes.saturating_sub(self.origin_minutes)) * self.px_per_minute
    }
}

/// Cellule visible d'un niveau de l'en-tête.
#[derive(Clone, Debug, PartialEq)]
pub struct TierCell {
    /// Abscisse écran du bord gauche.
    pub x0: f32,
    /// Abscisse écran du bord droit.
    pub x1: f32,
    /// Étiquette (vide si aucune variante ne tient).
    pub label: String,
}

/// Cellules d'un niveau qui coupent `clip` (abscisses gauche, droite),
/// frontières calculées dans le sens direct (jour → pixel).
///
/// Corps provisoire : aucune cellule.
#[must_use]
pub fn tier_cells(
    _axis: &TimeAxis,
    _unit: TierUnit,
    _week_start: Weekday,
    _clip: (f32, f32),
    _labels: &FormatLabels,
) -> Vec<TierCell> {
    Vec::new()
}

/// Nouveau défilement horizontal après un changement de zoom, qui garde sous
/// le pointeur (à `anchor_dx` pixels du bord gauche du Gantt) le même point
/// du contenu. Arithmétique en pixels seulement ; jamais négatif.
#[must_use]
pub fn zoom_anchor(scroll_x: f32, anchor_dx: f32, from: TimeScaleZoom, to: TimeScaleZoom) -> f32 {
    let ratio = to.px_per_day() / from.px_per_day();
    let scrolled = (scroll_x + anchor_dx) * ratio - anchor_dx;
    if scrolled.is_finite() {
        scrolled.max(0.0)
    } else {
        0.0
    }
}
