//! Échelle de temps : zoom discret, plage de jours, projection temps → pixels
//! et cellules de l'en-tête à deux niveaux.
//!
//! Sens unique : un `Moment` donne une abscisse, jamais l'inverse (aucune date
//! produite à partir d'un pixel, règle 3). L'écart à l'origine se calcule en
//! minutes entières (`i64`), converti une seule fois en `f32` pour le dessin.

use chrono::{Datelike, NaiveDate};
use planrs_core::{Day, Moment, Project, Schedule, Weekday};
use serde::{Deserialize, Serialize};

use crate::format::{DateOrder, FormatLabels};
use crate::geom::{i64_to_f32, index_to_f32};
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

/// Étendue maximale de la plage affichée : 20 ans (7 305 jours). Au-delà, la
/// largeur du contenu ne se représente plus au pixel près en `f32` au zoom le
/// plus fin ; la fin est alors tronquée (ADR-0011, décision 4).
pub const MAX_RANGE_DAYS: i32 = 7_305;

/// Marge avant le début du projet, en jours (avant recul au début de
/// semaine).
const LEAD_DAYS: i32 = 7;

/// Marge après la fin du projet, en jours (place pour les étiquettes).
const TRAIL_DAYS: i32 = 21;

/// Taille de police supposée pour estimer la largeur d'une étiquette.
const LABEL_FONT_PX: f32 = 13.0;

/// Largeur moyenne d'un caractère, en fraction de la taille de police.
const LABEL_CHAR_RATIO: f32 = 0.6;

/// Marge horizontale totale réservée autour d'une étiquette de cellule.
const LABEL_PAD_PX: f32 = 4.0;

/// Garde-fou : nombre maximal de cellules produites pour un niveau.
const MAX_CELLS: usize = 4_096;

/// Recule `d` jusqu'au jour de semaine `week_start` (au plus 6 pas) ; s'arrête
/// au bord du domaine.
fn align_to_week_start(d: Day, week_start: Weekday) -> Day {
    let mut day = d;
    for _ in 0..6 {
        if day.weekday() == week_start {
            break;
        }
        match day.checked_add_days(-1) {
            Ok(prev) => day = prev,
            Err(_) => break,
        }
    }
    day
}

/// Plage par défaut d'un document (calculée à l'ouverture, entiers
/// seulement).
///
/// Début : jour de début du projet − 7 jours, reculé au
/// `week_start_day` du projet. Fin (exclusive) : jour de fin du projet + 21
/// jours. La plage est étendue pour contenir le jour de la date d'état
/// effective, puis plafonnée à [`MAX_RANGE_DAYS`] jours après le début (la fin
/// est tronquée). Toute sortie du domaine des `Day` se replie sur la borne du
/// domaine : jamais de panique. Hypothèse d'affichage H-UI (ADR-0011).
#[must_use]
pub fn default_range(project: &Project, schedule: &Schedule) -> DayRange {
    let week_start = project.options.week_start_day;
    let first = schedule.project_start.day();
    let last = schedule.project_finish.day().max(first);
    let mut start = first.checked_add_days(-LEAD_DAYS).unwrap_or(Day::MIN);
    let mut end = last.checked_add_days(TRAIL_DAYS).unwrap_or(Day::MAX);

    let status = project.effective_status_date().day();
    if status < start {
        start = status;
    }
    let status_end = status.checked_add_days(1).unwrap_or(Day::MAX);
    if status_end > end {
        end = status_end;
    }
    start = align_to_week_start(start, week_start);

    let min_end = start.checked_add_days(1).unwrap_or(Day::MAX);
    let max_end = start.checked_add_days(MAX_RANGE_DAYS).unwrap_or(Day::MAX);
    if end > max_end {
        log::warn!("plage d'affichage tronquée à {MAX_RANGE_DAYS} jours");
    }
    DayRange {
        start,
        end: end.max(min_end).min(max_end),
    }
}

/// Nombre de jours d'une plage (0 si elle est vide ou inversée).
fn range_days(range: DayRange) -> i64 {
    i64::from(range.end.index())
        .saturating_sub(i64::from(range.start.index()))
        .max(0)
}

/// Zoom ajusté à l'ouverture : `Week` si toute la plage tient dans
/// `gantt_width_px` (`jours × 12 ≤ largeur`), sinon `Month` (ADR-0011,
/// décision 4). Une largeur non finie donne `Month`.
#[must_use]
pub fn fit_zoom(range: DayRange, gantt_width_px: f32) -> TimeScaleZoom {
    let needed = i64_to_f32(range_days(range)) * TimeScaleZoom::Week.px_per_day();
    if gantt_width_px.is_finite() && needed <= gantt_width_px {
        TimeScaleZoom::Week
    } else {
        TimeScaleZoom::Month
    }
}

/// Minute de `Moment` du minuit d'un jour (même origine que
/// `Moment::minutes`).
fn day_minutes(d: Day) -> i64 {
    i64::from(d.index()).saturating_mul(MINUTES_PER_DAY)
}

/// Projection temps → pixels, figée pour une image. Aucune méthode inverse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeAxis {
    /// Premier jour de la plage.
    start: Day,
    /// Fin exclusive de la plage.
    end: Day,
    /// Minute du début de la plage (minuit de `range.start`).
    origin_minutes: i64,
    /// Minute de la fin exclusive de la plage.
    end_minutes: i64,
    /// Pixels par jour civil.
    px_per_day: f32,
    /// Pixels par minute (`px_per_day / 1440`).
    px_per_minute: f32,
    /// Abscisse écran du début de la plage (défilement horizontal inclus).
    left: f32,
}

impl TimeAxis {
    /// Axe de la plage `range` au zoom `zoom`, le début de la plage étant à
    /// l'abscisse écran `left` (0 si `left` n'est pas fini).
    #[must_use]
    pub fn new(range: DayRange, zoom: TimeScaleZoom, left: f32) -> Self {
        let end = range.end.max(range.start);
        let origin_minutes = day_minutes(range.start);
        let end_minutes = day_minutes(end);
        let px_per_day = zoom.px_per_day();
        Self {
            start: range.start,
            end,
            origin_minutes,
            end_minutes,
            px_per_day,
            px_per_minute: px_per_day / i64_to_f32(MINUTES_PER_DAY),
            left: if left.is_finite() { left } else { 0.0 },
        }
    }

    /// Abscisse écran d'une minute absolue. Hors de la plage, l'abscisse est
    /// hors du contenu mais reste finie : l'écart est entier, borné par le
    /// domaine des `Moment` (< 2^27 minutes), puis converti une seule fois.
    fn x_minutes(&self, minutes: i64) -> f32 {
        let delta = minutes.saturating_sub(self.origin_minutes);
        let x = self.left + i64_to_f32(delta) * self.px_per_minute;
        if x.is_finite() { x } else { self.left }
    }

    /// Abscisse écran d'un instant.
    #[must_use]
    pub fn x(&self, m: Moment) -> f32 {
        self.x_minutes(m.minutes())
    }

    /// Abscisse écran du minuit d'un jour.
    #[must_use]
    pub fn x_day(&self, d: Day) -> f32 {
        self.x_minutes(day_minutes(d))
    }

    /// Largeur totale du contenu, en pixels.
    #[must_use]
    pub fn content_width(&self) -> f32 {
        i64_to_f32(self.end_minutes.saturating_sub(self.origin_minutes)) * self.px_per_minute
    }

    /// Abscisse écran du début de la plage.
    #[must_use]
    pub fn left(&self) -> f32 {
        self.left
    }

    /// Pixels par jour civil du zoom de l'axe.
    #[must_use]
    pub fn px_per_day(&self) -> f32 {
        self.px_per_day
    }

    /// Plus grand indice `k` de `0..n` tel que le minuit du jour
    /// `start + k` soit à gauche de `x` (ou 0) : dichotomie sur des jours
    /// entiers, abscisses calculées dans le sens direct seulement.
    fn first_day_at_or_before(&self, x: f32) -> Day {
        let n = self.end.index().saturating_sub(self.start.index());
        let (mut lo, mut hi) = (0_i32, n);
        // Invariant : x_day(start + lo) ≤ x ou lo = 0 ; x_day(start + hi) > x ou hi = n.
        while hi.saturating_sub(lo) > 1 {
            let mid = lo.saturating_add(hi.saturating_sub(lo) / 2);
            let Ok(day) = self.start.checked_add_days(mid) else {
                break;
            };
            if self.x_day(day) <= x {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        self.start.checked_add_days(lo).unwrap_or(self.start)
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

/// Date chrono d'un `Day` (du 1er du mois `month` de l'année `year`) → `Day`.
fn first_of_month(year: i32, month: u32) -> Option<Day> {
    NaiveDate::from_ymd_opt(year, month, 1).and_then(|d| Day::from_naive(d).ok())
}

/// Début de l'unité qui contient `d` (repli sur le premier jour du domaine
/// si ce début le précède).
fn unit_start(d: Day, unit: TierUnit, week_start: Weekday) -> Day {
    let date = d.to_naive();
    let (year, month) = (date.year(), date.month());
    let start = match unit {
        TierUnit::Day => Some(d),
        TierUnit::Week => Some(align_to_week_start(d, week_start)),
        TierUnit::Month => first_of_month(year, month),
        TierUnit::Quarter => {
            let q_month = (month.saturating_sub(1) / 3).saturating_mul(3);
            first_of_month(year, q_month.saturating_add(1))
        }
        TierUnit::Year => first_of_month(year, 1),
    };
    start.unwrap_or(Day::MIN)
}

/// Début de l'unité suivante, `cur` étant un début d'unité ; `None` hors du
/// domaine.
fn next_unit_start(cur: Day, unit: TierUnit) -> Option<Day> {
    let date = cur.to_naive();
    let (year, month) = (date.year(), date.month());
    let add_months = |n: u32| {
        let zero_based = month.saturating_sub(1).saturating_add(n);
        let year = year.checked_add(i32::try_from(zero_based / 12).ok()?)?;
        first_of_month(year, (zero_based % 12).saturating_add(1))
    };
    match unit {
        TierUnit::Day => cur.checked_add_days(1).ok(),
        TierUnit::Week => cur.checked_add_days(7).ok(),
        TierUnit::Month => add_months(1),
        TierUnit::Quarter => add_months(3),
        TierUnit::Year => first_of_month(year.checked_add(1)?, 1),
    }
}

/// Intervalles `[début, fin)` de l'unité qui coupent `clip` (abscisses
/// gauche, droite), avec leurs abscisses écran. Les cellules partielles aux
/// bords sont conservées (abscisses non rognées).
pub(crate) fn tier_spans(
    axis: &TimeAxis,
    unit: TierUnit,
    week_start: Weekday,
    clip: (f32, f32),
) -> Vec<(Day, f32, f32)> {
    let mut spans = Vec::new();
    let (c0, c1) = clip;
    if !c0.is_finite() || !c1.is_finite() {
        return spans;
    }
    let lo = c0.max(axis.left);
    let hi = c1.min(axis.left + axis.content_width());
    if hi <= lo || axis.end <= axis.start {
        return spans;
    }
    let mut cur = unit_start(axis.first_day_at_or_before(lo), unit, week_start);
    while spans.len() < MAX_CELLS {
        let next = match next_unit_start(cur, unit) {
            Some(next) if next > cur => next,
            // Bord du domaine : la dernière cellule s'arrête à la fin de la plage.
            _ if axis.end > cur => axis.end,
            _ => break,
        };
        let (x0, x1) = (axis.x_day(cur), axis.x_day(next));
        if x1 > lo {
            spans.push((cur, x0, x1));
        }
        if x1 >= hi || next >= axis.end {
            break;
        }
        cur = next;
    }
    spans
}

/// Largeur estimée d'une étiquette (`caractères × 0,6 × 13 px`), sans mesure
/// de police.
fn estimated_width(text: &str) -> f32 {
    index_to_f32(text.chars().count()) * LABEL_CHAR_RATIO * LABEL_FONT_PX
}

/// Rang (dimanche = 0) d'un jour de semaine, pour les tables de libellés.
fn sunday_index(w: Weekday) -> usize {
    Weekday::ALL.iter().position(|x| *x == w).unwrap_or(0)
}

/// Libellé d'un tableau (`""` hors bornes).
fn pick(names: &[String], i: usize) -> &str {
    names.get(i).map_or("", String::as_str)
}

/// Variantes d'étiquette d'une cellule, de la plus longue à la plus courte
/// (hypothèse d'affichage H-UI ; formats inspirés de l'échelle de temps de
/// MS Project, non vérifiés).
fn label_variants(day: Day, unit: TierUnit, l: &FormatLabels) -> Vec<String> {
    let date = day.to_naive();
    let year = date.year();
    let month_i = usize::try_from(date.month0()).unwrap_or(0);
    let dom = date.day();
    let long = pick(&l.months_long, month_i);
    let short = pick(&l.months_short, month_i);
    let mdy = l.date_order == DateOrder::MonthDayYear;
    match unit {
        TierUnit::Day => {
            let initial = pick(&l.weekday_initials, sunday_index(day.weekday()));
            vec![format!("{initial} {dom}"), dom.to_string()]
        }
        TierUnit::Week => {
            let full = if mdy {
                format!("{short} {dom}")
            } else {
                format!("{dom} {short}")
            };
            vec![full, dom.to_string()]
        }
        TierUnit::Month => {
            let yy = year.rem_euclid(100);
            let medium = if mdy {
                format!("{short} '{yy:02}")
            } else {
                format!("{short} {yy:02}")
            };
            let initial: String = long.chars().take(1).flat_map(char::to_uppercase).collect();
            vec![
                format!("{long} {year}"),
                long.to_owned(),
                medium,
                short.to_owned(),
                initial,
            ]
        }
        TierUnit::Quarter => {
            let q = (date.month0() / 3).saturating_add(1);
            let prefix = &l.quarter_prefix;
            vec![format!("{prefix}{q} {year}"), format!("{prefix}{q}")]
        }
        TierUnit::Year => vec![year.to_string()],
    }
}

/// Jours de quelques dates représentatives d'une unité (tous les mois, jour
/// à deux chiffres) : les variantes les plus longues de la langue en sortent.
fn representative_days(unit: TierUnit) -> Vec<Day> {
    let dom = match unit {
        TierUnit::Day | TierUnit::Week => 28,
        TierUnit::Month | TierUnit::Quarter | TierUnit::Year => 1,
    };
    (1..=12_u32)
        .filter_map(|m| NaiveDate::from_ymd_opt(2026, m, dom))
        .filter_map(|d| Day::from_naive(d).ok())
        .collect()
}

/// Durée minimale d'une unité, en jours (février pour un mois…).
const fn min_unit_days(unit: TierUnit) -> i64 {
    match unit {
        TierUnit::Day => 1,
        TierUnit::Week => 7,
        TierUnit::Month => 28,
        TierUnit::Quarter => 90,
        TierUnit::Year => 365,
    }
}

/// Rang de variante commun à toutes les cellules d'un niveau : la plus longue
/// variante dont toutes les formes de la langue tiennent dans la plus petite
/// cellule possible de l'unité. Étiquettes homogènes et stables pendant le
/// défilement (le choix ne dépend pas des cellules visibles) ; `None` si
/// aucune ne tient.
fn label_level(unit: TierUnit, px_per_day: f32, l: &FormatLabels) -> Option<usize> {
    let width = i64_to_f32(min_unit_days(unit)) * px_per_day;
    let samples: Vec<Vec<String>> = representative_days(unit)
        .into_iter()
        .map(|d| label_variants(d, unit, l))
        .collect();
    let levels = samples.iter().map(Vec::len).max().unwrap_or(0);
    (0..levels).find(|&k| {
        samples.iter().all(|variants| {
            variants
                .get(k)
                .is_none_or(|v| estimated_width(v) + LABEL_PAD_PX <= width)
        })
    })
}

/// Cellules d'un niveau qui coupent `clip` (abscisses gauche, droite),
/// frontières calculées dans le sens direct (jour → pixel).
///
/// Semaines alignées sur `week_start` ; mois, trimestres et années sur le
/// calendrier civil. Étiquettes (« L 2 » / « 2 » ; « 2 nov. » / « 2 » ;
/// « novembre 2026 » / « novembre » / « nov. 26 » / « nov. » / « N » ;
/// « T4 2026 » / « T4 » ; « 2026 ») : une seule variante par niveau, la plus
/// longue dont la largeur estimée (`caractères × 0,6 × 13 px` + marge) tient
/// dans la plus petite cellule de l'unité au zoom de l'axe ; vide si aucune.
#[must_use]
pub fn tier_cells(
    axis: &TimeAxis,
    unit: TierUnit,
    week_start: Weekday,
    clip: (f32, f32),
    labels: &FormatLabels,
) -> Vec<TierCell> {
    let level = label_level(unit, axis.px_per_day, labels);
    tier_spans(axis, unit, week_start, clip)
        .into_iter()
        .map(|(day, x0, x1)| TierCell {
            x0,
            x1,
            label: level
                .and_then(|k| label_variants(day, unit, labels).into_iter().nth(k))
                .unwrap_or_default(),
        })
        .collect()
}

/// Nouveau défilement horizontal après un changement de zoom, qui garde sous
/// le pointeur (à `anchor_dx` pixels du bord gauche du Gantt) le même point
/// du contenu : `(scroll_x + anchor_dx) × to / from − anchor_dx`, borné à 0.
/// Arithmétique en pixels seulement ; jamais négatif ni non fini.
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use chrono::NaiveDate;
    use planrs_core::{Day, Moment, TimeOfDay, Weekday};

    use super::*;

    fn day(y: i32, m: u32, d: u32) -> Day {
        Day::from_naive(NaiveDate::from_ymd_opt(y, m, d).unwrap()).unwrap()
    }

    fn moment(y: i32, m: u32, d: u32, h: u16) -> Moment {
        Moment::at(day(y, m, d), TimeOfDay::hm(h, 0).unwrap()).unwrap()
    }

    fn labels_fr() -> FormatLabels {
        let months_long = [
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ];
        let months_short = [
            "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
        ];
        FormatLabels {
            months_long: months_long.map(String::from),
            months_short: months_short.map(String::from),
            weekday_initials: ["D", "L", "M", "M", "J", "V", "S"].map(String::from),
            weekdays_short: ["dim", "lun", "mar", "mer", "jeu", "ven", "sam"].map(String::from),
            quarter_prefix: "T".to_owned(),
            date_order: DateOrder::DayMonthYear,
            ..FormatLabels::default()
        }
    }

    fn labels_en() -> FormatLabels {
        let months_long = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        let months_short = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        FormatLabels {
            months_long: months_long.map(String::from),
            months_short: months_short.map(String::from),
            weekday_initials: ["S", "M", "T", "W", "T", "F", "S"].map(String::from),
            quarter_prefix: "Q".to_owned(),
            date_order: DateOrder::MonthDayYear,
            ..FormatLabels::default()
        }
    }

    fn demo_range() -> DayRange {
        let now = moment(2027, 1, 15, 8);
        let doc = planrs_mspdi::read(crate::DEMO_MSPDI, &planrs_mspdi::ReadOptions::new(now)).unwrap();
        let schedule = planrs_core::schedule(&doc.project);
        default_range(&doc.project, &schedule)
    }

    #[test]
    fn zoom_tables() {
        assert_eq!(TimeScaleZoom::Day.px_per_day(), 36.0);
        assert_eq!(TimeScaleZoom::Quarter.px_per_day(), 1.2);
        assert_eq!(TimeScaleZoom::Month.tiers(), (TierUnit::Quarter, TierUnit::Month));
        assert_eq!(TimeScaleZoom::Day.finer(), TimeScaleZoom::Day);
        assert_eq!(TimeScaleZoom::Quarter.coarser(), TimeScaleZoom::Quarter);
        assert_eq!(TimeScaleZoom::Week.coarser().finer(), TimeScaleZoom::Week);
    }

    #[test]
    fn timescale_x_monotonic() {
        let range = DayRange {
            start: day(2026, 10, 26),
            end: day(2027, 6, 7),
        };
        for zoom in TimeScaleZoom::ALL {
            let axis = TimeAxis::new(range, zoom, 100.0);
            assert_eq!(axis.x_day(range.start), 100.0);
            let mut prev = f32::NEG_INFINITY;
            let mut m = moment(2026, 10, 20, 0);
            let stop = moment(2027, 6, 15, 0);
            while m < stop {
                let x = axis.x(m);
                assert!(x.is_finite() && x > prev, "{zoom:?} {m}");
                prev = x;
                m = m.checked_add(planrs_core::Minutes::new(97)).unwrap();
            }
            // Un jour vaut px_per_day pixels.
            let one = axis.x_day(day(2026, 10, 27)) - axis.x_day(range.start);
            assert!((one - zoom.px_per_day()).abs() < 1e-3);
            assert!((axis.content_width() - 224.0 * zoom.px_per_day()).abs() < 1e-2);
        }
    }

    /// Équivalent documenté de `timescale_x_clamped_to_range` : l'axe ne borne
    /// pas les abscisses (une barre hors plage serait sinon écrasée au bord),
    /// mais tout `Moment` du domaine, y compris ses bornes, donne une abscisse
    /// finie, du bon côté du contenu ; un `left` non fini est remplacé par 0.
    #[test]
    fn timescale_x_clamped_to_range() {
        let range = DayRange {
            start: day(2026, 10, 26),
            end: day(2027, 6, 7),
        };
        let axis = TimeAxis::new(range, TimeScaleZoom::Day, 10.0);
        let right = 10.0 + axis.content_width();
        let before = axis.x(Moment::MIN);
        let after = axis.x(Moment::MAX);
        assert!(before.is_finite() && before < 10.0);
        assert!(after.is_finite() && after > right);
        assert_eq!(axis.x_day(range.end), right);
        assert!(axis.x_day(Day::MIN).is_finite() && axis.x_day(Day::MAX).is_finite());
        let nan_axis = TimeAxis::new(range, TimeScaleZoom::Day, f32::NAN);
        assert_eq!(nan_axis.x_day(range.start), 0.0);
        // Plage inversée : largeur nulle, rien d'infini.
        let empty = TimeAxis::new(
            DayRange {
                start: range.end,
                end: range.start,
            },
            TimeScaleZoom::Day,
            0.0,
        );
        assert_eq!(empty.content_width(), 0.0);
        assert!(tier_cells(&empty, TierUnit::Day, Weekday::Monday, (0.0, 500.0), &labels_fr()).is_empty());
    }

    /// house.xml : début du projet 2026-11-02 (lundi) − 7 j = 2026-10-26, déjà
    /// un lundi (début de semaine par défaut) ; fin 2027-05-17 + 21 j =
    /// 2027-06-07 (exclusive) ; la date d'état 2027-01-15 est déjà dans la
    /// plage. Longueur : 6 (oct.) + 30 + 31 + 31 + 28 + 31 + 30 + 31 + 6 (juin)
    /// = 224 jours.
    #[test]
    fn default_range_demo() {
        let range = demo_range();
        assert_eq!(range.start, day(2026, 10, 26));
        assert_eq!(range.start.weekday(), Weekday::Monday);
        assert_eq!(range.end, day(2027, 6, 7));
        assert_eq!(range_days(range), 224);
    }

    #[test]
    fn default_range_extends_to_status_date_and_caps() {
        let now = moment(2027, 1, 15, 8);
        let mut doc = planrs_mspdi::read(crate::DEMO_MSPDI, &planrs_mspdi::ReadOptions::new(now)).unwrap();
        // Date d'état après la fin : la plage s'étend jusqu'à son lendemain.
        doc.project.status_date = Some(moment(2027, 9, 1, 17));
        let schedule = planrs_core::schedule(&doc.project);
        let range = default_range(&doc.project, &schedule);
        assert_eq!(range.end, day(2027, 9, 2));
        // Date d'état avant le début : début reculé puis aligné au lundi.
        doc.project.status_date = Some(moment(2026, 9, 3, 17)); // jeudi
        let range = default_range(&doc.project, &schedule);
        assert_eq!(range.start, day(2026, 8, 31));
        // Date d'état très lointaine : plage plafonnée à 20 ans.
        doc.project.status_date = Some(moment(2120, 1, 1, 8));
        let range = default_range(&doc.project, &schedule);
        assert_eq!(range_days(range), i64::from(MAX_RANGE_DAYS));
        // Début de semaine dimanche.
        doc.project.status_date = None;
        doc.project.options.week_start_day = Weekday::Sunday;
        let range = default_range(&doc.project, &schedule);
        assert_eq!(range.start, day(2026, 10, 25));
    }

    #[test]
    fn fit_zoom_demo_is_month() {
        let range = demo_range();
        // 224 × 12 = 2 688 px > 900 px.
        assert_eq!(fit_zoom(range, 900.0), TimeScaleZoom::Month);
        assert_eq!(fit_zoom(range, f32::NAN), TimeScaleZoom::Month);
        assert_eq!(fit_zoom(range, 2_688.0), TimeScaleZoom::Week);
    }

    #[test]
    fn fit_zoom_short_project_is_week() {
        // 6 semaines : 42 × 12 = 504 px ≤ 900 px.
        let range = DayRange {
            start: day(2026, 11, 2),
            end: day(2026, 12, 14),
        };
        assert_eq!(fit_zoom(range, 900.0), TimeScaleZoom::Week);
        assert_eq!(fit_zoom(range, 503.0), TimeScaleZoom::Month);
    }

    #[test]
    fn tier_cells_month_boundaries() {
        let range = DayRange {
            start: day(2026, 10, 26),
            end: day(2027, 6, 7),
        };
        let axis = TimeAxis::new(range, TimeScaleZoom::Week, 0.0);
        let cells = tier_cells(
            &axis,
            TierUnit::Month,
            Weekday::Monday,
            (0.0, axis.content_width()),
            &labels_fr(),
        );
        // octobre (partiel) … juin (partiel) : 9 cellules.
        assert_eq!(cells.len(), 9);
        let first = &cells[0];
        assert_eq!(first.x0, axis.x_day(day(2026, 10, 1)));
        assert!(first.x0 < 0.0, "cellule partielle conservée");
        assert_eq!(first.x1, axis.x_day(day(2026, 11, 1)));
        assert_eq!(cells[1].x0, axis.x_day(day(2026, 11, 1)));
        assert_eq!(cells[1].x1, axis.x_day(day(2026, 12, 1)));
        assert_eq!(cells[1].label, "novembre 2026");
        assert_eq!(cells[3].x0, axis.x_day(day(2027, 1, 1)));
        assert_eq!(cells[3].label, "janvier 2027");
        // Contiguïté.
        for w in cells.windows(2) {
            assert_eq!(w[0].x1, w[1].x0);
        }
        // Culling : une fenêtre étroite au milieu ne donne que les mois visibles.
        let x_dec = axis.x_day(day(2026, 12, 10));
        let few = tier_cells(
            &axis,
            TierUnit::Month,
            Weekday::Monday,
            (x_dec, x_dec + 50.0),
            &labels_fr(),
        );
        assert_eq!(few.len(), 1);
        assert_eq!(few[0].x0, axis.x_day(day(2026, 12, 1)));
        // Semaines alignées sur le lundi.
        let weeks = tier_cells(
            &axis,
            TierUnit::Week,
            Weekday::Monday,
            (x_dec, x_dec + 200.0),
            &labels_fr(),
        );
        assert_eq!(weeks[0].x0, axis.x_day(day(2026, 12, 7)));
        assert_eq!(weeks[0].label, "7 déc.");
        assert_eq!(weeks.len(), 3);
        // Mois étroit (zoom trimestre) : initiale ou vide.
        let q = TimeAxis::new(range, TimeScaleZoom::Quarter, 0.0);
        let narrow = tier_cells(&q, TierUnit::Month, Weekday::Monday, (0.0, 200.0), &labels_fr());
        assert!(narrow.iter().all(|c| c.label.chars().count() <= 1), "{narrow:?}");
        assert_eq!(narrow[1].label, "N");
    }

    #[test]
    fn tier_cells_quarter_labels_fr() {
        let range = DayRange {
            start: day(2026, 10, 26),
            end: day(2027, 6, 7),
        };
        let axis = TimeAxis::new(range, TimeScaleZoom::Month, 0.0);
        let cells = tier_cells(
            &axis,
            TierUnit::Quarter,
            Weekday::Monday,
            (0.0, axis.content_width()),
            &labels_fr(),
        );
        let got: Vec<&str> = cells.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(got, ["T4 2026", "T1 2027", "T2 2027"]);
        assert_eq!(cells[1].x0, axis.x_day(day(2027, 1, 1)));
        assert_eq!(cells[2].x0, axis.x_day(day(2027, 4, 1)));
        // Court si la cellule est étroite (zoom trimestre : ≈ 110 px).
        let q = TimeAxis::new(range, TimeScaleZoom::Quarter, 0.0);
        let short = tier_cells(
            &q,
            TierUnit::Quarter,
            Weekday::Monday,
            (0.0, q.content_width()),
            &labels_fr(),
        );
        assert_eq!(short[1].label, "T1 2027");
        let en = tier_cells(
            &axis,
            TierUnit::Quarter,
            Weekday::Monday,
            (0.0, 400.0),
            &labels_en(),
        );
        assert_eq!(en[0].label, "Q4 2026");
        // Années et jours.
        let years = tier_cells(
            &q,
            TierUnit::Year,
            Weekday::Monday,
            (0.0, q.content_width()),
            &labels_fr(),
        );
        let got: Vec<&str> = years.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(got, ["2026", "2027"]);
        let d = TimeAxis::new(range, TimeScaleZoom::Day, 0.0);
        let days = tier_cells(&d, TierUnit::Day, Weekday::Monday, (0.0, 100.0), &labels_fr());
        assert_eq!(days[0].label, "L 26");
        assert_eq!(days.len(), 3);
        let en_weeks = tier_cells(&axis, TierUnit::Week, Weekday::Monday, (0.0, 30.0), &labels_en());
        assert_eq!(en_weeks[0].label, "26");
        let w = TimeAxis::new(range, TimeScaleZoom::Week, 0.0);
        let en_weeks = tier_cells(&w, TierUnit::Week, Weekday::Monday, (0.0, 30.0), &labels_en());
        assert_eq!(en_weeks[0].label, "Oct 26");
        let en_months = tier_cells(
            &axis,
            TierUnit::Month,
            Weekday::Monday,
            (100.0, 101.0),
            &labels_en(),
        );
        assert_eq!(en_months[0].label, "November");
    }

    #[test]
    fn zoom_anchor_keeps_point() {
        let (scroll, dx) = (300.0_f32, 120.0_f32);
        for from in TimeScaleZoom::ALL {
            for to in TimeScaleZoom::ALL {
                let new_scroll = zoom_anchor(scroll, dx, from, to);
                // Point de contenu sous le curseur, exprimé en jours : inchangé.
                let before = (scroll + dx) / from.px_per_day();
                let after = (new_scroll + dx) / to.px_per_day();
                if new_scroll > 0.0 {
                    assert!((before - after).abs() < 1e-3, "{from:?} → {to:?}");
                }
                assert!(new_scroll >= 0.0 && new_scroll.is_finite());
            }
        }
        assert_eq!(
            zoom_anchor(0.0, 10.0, TimeScaleZoom::Day, TimeScaleZoom::Quarter),
            0.0
        );
        assert_eq!(
            zoom_anchor(f32::NAN, 10.0, TimeScaleZoom::Day, TimeScaleZoom::Week),
            0.0
        );
    }
}
