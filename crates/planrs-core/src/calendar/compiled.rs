//! Forme compilée d'un calendrier et conversions ouvrées (ADR-0008, § 6.4 et
//! 6.5, règles R-020 à R-029).
//!
//! Définition normative : pour un calendrier compilé, **W(t)** est le nombre
//! de minutes ouvrées dans `[Moment::MIN, t)`. W est croissante au sens large,
//! de pente 1 dans les plages et 0 ailleurs ; toutes les conversions en
//! dérivent. Fusionner deux plages contiguës, y compris de part et d'autre de
//! minuit, ne change pas W.
//!
//! Structure : partition triée de `[jour 0, jour 60 631)` en segments. Un
//! segment `Weekly` répète un motif de semaine (semaine par défaut ou semaine
//! datée résolue) ; un segment `Daily` répète un motif de jour (série de jours
//! d'exception). Chaque segment porte le cumul W à son premier jour. Les motifs
//! de jour et de semaine sont internés. Aucune fenêtre, aucun cache paresseux,
//! aucune mutabilité intérieure : la structure est immuable, `Send + Sync` et
//! déterministe ; elle est recompilée à chaque recalcul.
//!
//! Coût d'une conversion, sans allocation : une dichotomie sur les segments,
//! une sur au plus 7 jours, une sur au plus 5 plages.

use std::collections::{BTreeMap, BTreeSet};

use crate::arena::Arena;
use crate::calendar::Direction;
use crate::calendar::model::{Calendar, CalendarKind, DaySetting, TimeRange};
use crate::calendar::recurrence::{DayClaims, expand_exception};
use crate::calendar::resolve::{
    CalendarChain, chain, defined, exceptions_in_order, work_week_owner, work_weeks_in_order,
};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::duration::Minutes;
use crate::error::CalendarError;
use crate::ids::CalendarUid;
use crate::model::Project;
use crate::time::{Day, Moment, TimeOfDay, Week, Weekday};

/// Minutes par jour civil (R-005).
const MINUTES_PER_DAY: i64 = 1_440;
/// Nombre de jours du domaine (1984-01-01 à 2149-12-31).
const DAY_COUNT: i32 = 60_631;

/// Motif d'un jour : plages fusionnées (contiguës réunies), cumul avant
/// chaque plage, total.
#[derive(Clone, Debug, PartialEq, Eq)]
struct DayPattern {
    ranges: Vec<TimeRange>,
    cum: Vec<i64>,
    total: i64,
}

impl DayPattern {
    /// Motif d'un réglage de jour, plages contiguës fusionnées (R-011).
    fn new(setting: &DaySetting) -> Self {
        let mut merged: Vec<(u16, u16)> = Vec::new();
        if let DaySetting::Working(h) = setting {
            for r in h.ranges() {
                let (from, to) = (r.from().minutes(), r.to().minutes());
                match merged.last_mut() {
                    Some((_, end)) if *end >= from => *end = (*end).max(to),
                    _ => merged.push((from, to)),
                }
            }
        }
        let ranges: Vec<TimeRange> = merged
            .into_iter()
            .filter_map(|(a, b)| TimeRange::new(TimeOfDay::new(a).ok()?, TimeOfDay::new(b).ok()?).ok())
            .collect();
        let mut cum = Vec::with_capacity(ranges.len());
        let mut total: i64 = 0;
        for r in &ranges {
            cum.push(total);
            // Au plus 1 440 minutes par jour : jamais de débordement.
            total = total.saturating_add(r.len().get());
        }
        Self { ranges, cum, total }
    }

    /// Minutes ouvrées du motif avant la minute `m` du jour (`0..=1440`).
    fn within(&self, m: i64) -> Option<i64> {
        let count = self.ranges.partition_point(|r| i64::from(r.from().minutes()) < m);
        let Some(j) = count.checked_sub(1) else {
            return Some(0);
        };
        let r = self.ranges.get(j)?;
        let before = *self.cum.get(j)?;
        let into = m.checked_sub(i64::from(r.from().minutes()))?;
        before.checked_add(into.min(r.len().get()))
    }

    /// Minute du jour qui termine la `w`-ième minute ouvrée du motif
    /// (`1 <= w <= total`).
    fn locate(&self, w: i64) -> Option<i64> {
        let j = self.cum.partition_point(|c| *c < w).checked_sub(1)?;
        let r = self.ranges.get(j)?;
        let offset = w.checked_sub(*self.cum.get(j)?)?;
        if offset < 1 || offset > r.len().get() {
            return None;
        }
        i64::from(r.from().minutes()).checked_add(offset)
    }
}

/// Motif de semaine : motif de jour de chaque jour de semaine, cumul avant
/// chaque jour (`prefix[k]` = minutes des jours `0..k`, dimanche = 0), total.
#[derive(Clone, Debug, PartialEq, Eq)]
struct WeekPattern {
    days: Week<u32>,
    prefix: [i64; 8],
    total: i64,
}

impl WeekPattern {
    fn new(days: Week<u32>, patterns: &[DayPattern]) -> Option<Self> {
        let mut prefix = [0_i64; 8];
        let mut acc: i64 = 0;
        for (k, wd) in Weekday::ALL.iter().enumerate() {
            let p = patterns.get(usize::try_from(*days.get(*wd)).ok()?)?;
            acc = acc.checked_add(p.total)?;
            *prefix.get_mut(k.checked_add(1)?)? = acc;
        }
        Some(Self {
            days,
            prefix,
            total: acc,
        })
    }

    /// Wp(d) : minutes du motif répété depuis le jour 0 jusqu'au jour `d`
    /// exclu. Le jour 0 est un dimanche.
    fn cum_at(&self, day: i32) -> Option<i64> {
        let q = i64::from(day.checked_div_euclid(7)?);
        let k = usize::try_from(day.checked_rem_euclid(7)?).ok()?;
        q.checked_mul(self.total)?.checked_add(*self.prefix.get(k)?)
    }
}

/// Règle d'un segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SegmentRule {
    /// Motif de semaine répété.
    Weekly(u32),
    /// Chaque jour identique (série de jours d'exception).
    Daily(u32),
}

/// Segment `[first_day, jour suivant le segment)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Segment {
    first_day: i32,
    rule: SegmentRule,
    /// W au début du segment.
    cum_start: i64,
    /// `Wp(first_day)` pour un segment `Weekly`, 0 sinon.
    wp_start: i64,
}

/// Calendrier compilé : W(t) en segments, conversions ouvrées.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledCalendar {
    uid: CalendarUid,
    /// Motifs de jour internés ; `day_patterns[0]` est le jour chômé.
    day_patterns: Vec<DayPattern>,
    /// Motifs de semaine internés.
    week_patterns: Vec<WeekPattern>,
    /// Partition triée de `[0, 60 631)` ; `segments[0].first_day == 0`.
    segments: Vec<Segment>,
    /// `W(Moment::MAX)`.
    total: i64,
}

/// Motifs en cours d'internement.
struct Patterns {
    days: Vec<DayPattern>,
    day_index: BTreeMap<Vec<TimeRange>, u32>,
    weeks: Vec<WeekPattern>,
    week_index: BTreeMap<[u32; 7], u32>,
}

impl Patterns {
    fn new() -> Option<Self> {
        let mut p = Self {
            days: Vec::new(),
            day_index: BTreeMap::new(),
            weeks: Vec::new(),
            week_index: BTreeMap::new(),
        };
        // Motif n° 0 : jour chômé.
        p.day(&DaySetting::NonWorking)?;
        Some(p)
    }

    fn day(&mut self, setting: &DaySetting) -> Option<u32> {
        let pattern = DayPattern::new(setting);
        if let Some(i) = self.day_index.get(&pattern.ranges) {
            return Some(*i);
        }
        let i = u32::try_from(self.days.len()).ok()?;
        self.day_index.insert(pattern.ranges.clone(), i);
        self.days.push(pattern);
        Some(i)
    }

    fn week(&mut self, settings: &Week<DaySetting>) -> Option<u32> {
        let mut days: Week<u32> = Week::default();
        for wd in Weekday::ALL {
            *days.get_mut(wd) = self.day(settings.get(wd))?;
        }
        let key = Weekday::ALL.map(|wd| *days.get(wd));
        if let Some(i) = self.week_index.get(&key) {
            return Some(*i);
        }
        let i = u32::try_from(self.weeks.len()).ok()?;
        self.weeks.push(WeekPattern::new(days, &self.days)?);
        self.week_index.insert(key, i);
        Some(i)
    }
}

/// Semaine résolue hors exceptions (R-016, étapes 3 à 6) quand la semaine
/// datée du dérivé est `derived_ww` et celle de la base `base_ww`.
fn resolved_week(
    derived: Option<&Calendar>,
    derived_ww: Option<usize>,
    base: &Calendar,
    base_ww: Option<usize>,
) -> Option<Week<DaySetting>> {
    let CalendarKind::Base { week: base_week } = base.kind() else {
        return None;
    };
    let day = |wd: Weekday| -> DaySetting {
        if let Some(d) = derived {
            if let Some(s) = derived_ww
                .and_then(|i| d.work_weeks().get(i))
                .and_then(|w| defined(w.days.get(wd)))
            {
                return s;
            }
            if let CalendarKind::Derived { week, .. } = d.kind()
                && let Some(s) = defined(week.get(wd))
            {
                return s;
            }
        }
        if let Some(s) = base_ww
            .and_then(|i| base.work_weeks().get(i))
            .and_then(|w| defined(w.days.get(wd)))
        {
            return s;
        }
        base_week.get(wd).clone()
    };
    Some(Week::from_fn(day))
}

/// Jour de semaine du jour `day` (le jour 0 est un dimanche).
fn weekday_of(day: i32) -> Option<Weekday> {
    let k = usize::try_from(day.checked_rem_euclid(7)?).ok()?;
    Weekday::ALL.get(k).copied()
}

impl CompiledCalendar {
    /// Compile le calendrier `uid` (ADR-0008, § 6.4) :
    /// 1. chaîne d'héritage ;
    /// 2. exceptions dépliées, posées niveau par niveau (dérivé d'abord), dans
    ///    l'ordre (début, indice), chacune sur les jours encore libres ;
    /// 3. points de coupure : bornes du domaine, des séries d'exception et des
    ///    semaines datées ;
    /// 4. chaque intervalle élémentaire reçoit un motif de jour (exception) ou
    ///    de semaine (précédence R-016) ;
    /// 5. motifs internés, segments adjacents identiques fusionnés, cumuls.
    ///
    /// Un calendrier sans temps ouvré se compile ; ses conversions renvoient
    /// [`CalendarError::NoWorkingTime`].
    ///
    /// # Errors
    ///
    /// Erreurs de [`chain`] ; [`CalendarError::Inconsistent`] si un invariant
    /// interne est violé (jamais attendu).
    pub fn compile(
        calendars: &Arena<CalendarUid, Calendar>,
        uid: CalendarUid,
    ) -> Result<Self, CalendarError> {
        let ch = chain(calendars, uid)?;
        Self::build(ch).ok_or(CalendarError::Inconsistent { calendar: uid })
    }

    /// Compilation d'une chaîne valide ; `None` si un invariant est violé.
    fn build(ch: CalendarChain<'_>) -> Option<Self> {
        let uid = ch.calendar().uid();
        let derived = match ch {
            CalendarChain::Base(_) => None,
            CalendarChain::Derived { calendar, .. } => Some(calendar),
        };
        let base = ch.base();
        let levels = || derived.into_iter().chain([base]);
        let mut patterns = Patterns::new()?;

        // 2. Exceptions : dérivé d'abord, puis base, chacune sur les jours libres.
        let mut claims: DayClaims<u32> = DayClaims::new();
        for c in levels() {
            for (_, e) in exceptions_in_order(c) {
                let p = patterns.day(&e.setting)?;
                // Une récurrence invalide ne peut pas être stockée.
                for r in expand_exception(e).ok()? {
                    claims.claim(r.first().index(), r.last().index(), p);
                }
            }
        }

        // 3. Points de coupure.
        let mut cuts: BTreeSet<i32> = BTreeSet::from([0, DAY_COUNT]);
        for (first, last, _) in claims.iter() {
            cuts.insert(first);
            cuts.insert(last.saturating_add(1));
        }
        for c in levels() {
            for w in c.work_weeks() {
                cuts.insert(w.period.first().index());
                cuts.insert(w.period.last().index().saturating_add(1));
            }
        }
        let cuts: Vec<i32> = cuts.into_iter().collect();

        // 4. Règle de chaque intervalle élémentaire, fusion des voisins égaux.
        let derived_ww = derived.map(work_weeks_in_order).unwrap_or_default();
        let base_ww = work_weeks_in_order(base);
        let mut week_cache: BTreeMap<(Option<usize>, Option<usize>), u32> = BTreeMap::new();
        let mut raw: Vec<(i32, SegmentRule)> = Vec::new();
        for pair in cuts.windows(2) {
            let &[first, _] = pair else {
                return None;
            };
            let rule = if let Some(p) = claims.owner(first) {
                SegmentRule::Daily(p)
            } else {
                let key = (
                    work_week_owner(&derived_ww, first),
                    work_week_owner(&base_ww, first),
                );
                let w = if let Some(w) = week_cache.get(&key) {
                    *w
                } else {
                    let w = patterns.week(&resolved_week(derived, key.0, base, key.1)?)?;
                    week_cache.insert(key, w);
                    w
                };
                SegmentRule::Weekly(w)
            };
            if raw.last().map(|(_, r)| *r) != Some(rule) {
                raw.push((first, rule));
            }
        }

        // 5. Cumuls.
        let mut segments = Vec::with_capacity(raw.len());
        let mut cum: i64 = 0;
        for (i, (first_day, rule)) in raw.iter().enumerate() {
            let end = match raw.get(i.checked_add(1)?) {
                Some((f, _)) => *f,
                None => DAY_COUNT,
            };
            let (work, wp_start) = match *rule {
                SegmentRule::Weekly(w) => {
                    let wp = patterns.weeks.get(usize::try_from(w).ok()?)?;
                    let start = wp.cum_at(*first_day)?;
                    (wp.cum_at(end)?.checked_sub(start)?, start)
                }
                SegmentRule::Daily(p) => {
                    let dp = patterns.days.get(usize::try_from(p).ok()?)?;
                    let n = i64::from(end.checked_sub(*first_day)?);
                    (n.checked_mul(dp.total)?, 0)
                }
            };
            segments.push(Segment {
                first_day: *first_day,
                rule: *rule,
                cum_start: cum,
                wp_start,
            });
            cum = cum.checked_add(work)?;
        }
        Some(Self {
            uid,
            day_patterns: patterns.days,
            week_patterns: patterns.weeks,
            segments,
            total: cum,
        })
    }

    /// UID du calendrier compilé.
    #[must_use]
    pub const fn uid(&self) -> CalendarUid {
        self.uid
    }

    /// Vrai si le calendrier a au moins une minute ouvrée sur le domaine.
    #[must_use]
    pub const fn has_working_time(&self) -> bool {
        self.total > 0
    }

    /// Minutes ouvrées sur tout le domaine : W(`Moment::MAX`).
    #[must_use]
    pub const fn total_working(&self) -> Minutes {
        Minutes::new(self.total)
    }

    /// Erreur d'invariant interne.
    const fn inconsistent(&self) -> CalendarError {
        CalendarError::Inconsistent { calendar: self.uid }
    }

    /// Segment qui contient le jour `day`.
    fn segment(&self, day: i32) -> Option<&Segment> {
        let i = self
            .segments
            .partition_point(|s| s.first_day <= day)
            .checked_sub(1)?;
        self.segments.get(i)
    }

    fn day_pattern(&self, i: u32) -> Option<&DayPattern> {
        self.day_patterns.get(usize::try_from(i).ok()?)
    }

    fn week_pattern(&self, i: u32) -> Option<&WeekPattern> {
        self.week_patterns.get(usize::try_from(i).ok()?)
    }

    /// Motif du jour `day` (dans un segment donné).
    fn pattern_of_day(&self, seg: &Segment, day: i32) -> Option<&DayPattern> {
        match seg.rule {
            SegmentRule::Daily(p) => self.day_pattern(p),
            SegmentRule::Weekly(w) => self.day_pattern(*self.week_pattern(w)?.days.get(weekday_of(day)?)),
        }
    }

    /// W(t), en minutes depuis 1984-01-01 00:00 (`0..=MAX`).
    fn cumulative(&self, t: i64) -> Option<i64> {
        if t >= Moment::MAX.minutes() {
            return Some(self.total);
        }
        let day = i32::try_from(t.checked_div_euclid(MINUTES_PER_DAY)?).ok()?;
        let minute = t.checked_rem_euclid(MINUTES_PER_DAY)?;
        let seg = self.segment(day)?;
        let dp = self.pattern_of_day(seg, day)?;
        let before_day = match seg.rule {
            SegmentRule::Weekly(w) => {
                let wp = self.week_pattern(w)?;
                wp.cum_at(day)?.checked_sub(seg.wp_start)?
            }
            SegmentRule::Daily(_) => i64::from(day.checked_sub(seg.first_day)?).checked_mul(dp.total)?,
        };
        seg.cum_start
            .checked_add(before_day)?
            .checked_add(dp.within(minute)?)
    }

    /// Plus petit t tel que W(t) >= v (`1 <= v <= total`) : la fin de la
    /// v-ième minute ouvrée.
    fn first_reach(&self, target: i64) -> Option<i64> {
        if target < 1 || target > self.total {
            return None;
        }
        // Dernier segment dont le cumul de début est < target : il contient
        // la minute visée (les segments sans travail sont sautés).
        let index = self
            .segments
            .partition_point(|s| s.cum_start < target)
            .checked_sub(1)?;
        let seg = self.segments.get(index)?;
        let rest = target.checked_sub(seg.cum_start)?;
        let (day, within, pattern) = match seg.rule {
            SegmentRule::Weekly(w) => {
                let week = self.week_pattern(w)?;
                // Rang de la minute dans le motif répété depuis le jour 0.
                let rank = seg.wp_start.checked_add(rest)?;
                let weeks = rank.checked_sub(1)?.checked_div_euclid(week.total)?;
                let in_week = rank.checked_sub(weeks.checked_mul(week.total)?)?;
                let k = week.prefix.get(1..)?.partition_point(|p| *p < in_week);
                let within = in_week.checked_sub(*week.prefix.get(k)?)?;
                let weekday = *Weekday::ALL.get(k)?;
                let day = weeks.checked_mul(7)?.checked_add(i64::try_from(k).ok()?)?;
                (day, within, self.day_pattern(*week.days.get(weekday))?)
            }
            SegmentRule::Daily(p) => {
                let pattern = self.day_pattern(p)?;
                let days = rest.checked_sub(1)?.checked_div_euclid(pattern.total)?;
                let within = rest.checked_sub(days.checked_mul(pattern.total)?)?;
                (i64::from(seg.first_day).checked_add(days)?, within, pattern)
            }
        };
        if day >= i64::from(DAY_COUNT) {
            return None;
        }
        day.checked_mul(MINUTES_PER_DAY)?
            .checked_add(pattern.locate(within)?)
    }

    /// W(t) avec l'erreur d'invariant.
    fn w(&self, t: Moment) -> Result<i64, CalendarError> {
        self.cumulative(t.minutes()).ok_or_else(|| self.inconsistent())
    }

    /// Instant `first_reach(v) + shift`.
    fn reach(&self, v: i64, shift: i64) -> Result<Moment, CalendarError> {
        self.first_reach(v)
            .and_then(|m| m.checked_add(shift))
            .and_then(|m| Moment::from_minutes(m).ok())
            .ok_or_else(|| self.inconsistent())
    }

    /// Erreur « aucun temps ouvré ».
    const fn no_working_time(&self) -> CalendarError {
        CalendarError::NoWorkingTime { calendar: self.uid }
    }

    /// Erreur « hors du domaine ».
    const fn out_of_domain(&self, direction: Direction) -> CalendarError {
        CalendarError::OutOfDomain {
            calendar: self.uid,
            direction,
        }
    }

    /// Vrai si la minute `[t, t + 1)` est ouvrée.
    ///
    /// # Errors
    ///
    /// [`CalendarError::Inconsistent`] (jamais attendu).
    pub fn is_working(&self, t: Moment) -> Result<bool, CalendarError> {
        if t >= Moment::MAX {
            return Ok(false);
        }
        let next = Moment::from_minutes(t.minutes().saturating_add(1)).map_err(|_| self.inconsistent())?;
        Ok(self.w(next)? > self.w(t)?)
    }

    /// Plages fusionnées du jour `day` (plages contiguës réunies).
    ///
    /// # Errors
    ///
    /// [`CalendarError::Inconsistent`] (jamais attendu).
    pub fn day_ranges(&self, day: Day) -> Result<&[TimeRange], CalendarError> {
        let d = day.index();
        self.segment(d)
            .and_then(|seg| self.pattern_of_day(seg, d))
            .map(|p| p.ranges.as_slice())
            .ok_or_else(|| self.inconsistent())
    }

    /// R-020 : minutes ouvrées entre `a` et `b`, W(b) − W(a) ; signé
    /// (négatif si `b` précède `a`), antisymétrique et additif.
    ///
    /// # Errors
    ///
    /// [`CalendarError::Inconsistent`] (jamais attendu).
    pub fn working_between(&self, a: Moment, b: Moment) -> Result<Minutes, CalendarError> {
        let diff = self
            .w(b)?
            .checked_sub(self.w(a)?)
            .ok_or_else(|| self.inconsistent())?;
        Ok(Minutes::new(diff))
    }

    /// R-021 à R-023 : instant à `d` minutes ouvrées de `t`, avec v = W(t) :
    /// - `d = 0` : `t`, sans recalage (R-023) ;
    /// - `d > 0` : plus petit u tel que W(u) = v + d ; il tombe sur une fin de
    ///   minute ouvrée, jamais au début de la plage suivante (R-021) ;
    /// - `d < 0` : plus grand s tel que W(s) = v − |d| ; il tombe sur un début
    ///   de minute ouvrée (R-022).
    ///
    /// # Errors
    ///
    /// [`CalendarError::NoWorkingTime`] si le calendrier n'a aucune minute
    /// ouvrée (et `d ≠ 0`) ; [`CalendarError::OutOfDomain`] si la cible sort
    /// du domaine (`After` : v + d > total ; `Before` : v < |d|).
    pub fn add_working(&self, t: Moment, d: Minutes) -> Result<Moment, CalendarError> {
        let d = d.get();
        if d == 0 {
            return Ok(t);
        }
        if self.total == 0 {
            return Err(self.no_working_time());
        }
        let v = self.w(t)?;
        if d > 0 {
            match v.checked_add(d) {
                Some(target) if target <= self.total => self.reach(target, 0),
                _ => Err(self.out_of_domain(Direction::After)),
            }
        } else {
            match d.checked_neg() {
                Some(n) if n <= v => {
                    let target = v
                        .checked_sub(n)
                        .and_then(|x| x.checked_add(1))
                        .ok_or_else(|| self.inconsistent())?;
                    self.reach(target, -1)
                }
                _ => Err(self.out_of_domain(Direction::Before)),
            }
        }
    }

    /// R-024 : plus petit u >= t dont la minute `[u, u + 1)` est ouvrée.
    /// Idempotente.
    ///
    /// # Errors
    ///
    /// [`CalendarError::NoWorkingTime`] ; [`CalendarError::OutOfDomain`]
    /// (`After`) s'il n'y a plus de minute ouvrée après `t`.
    pub fn next_working_moment(&self, t: Moment) -> Result<Moment, CalendarError> {
        if self.total == 0 {
            return Err(self.no_working_time());
        }
        let v = self.w(t)?;
        if v >= self.total {
            return Err(self.out_of_domain(Direction::After));
        }
        self.reach(v.saturating_add(1), -1)
    }

    /// R-025 : plus grand u <= t tel que la minute `[u − 1, u)` est ouvrée.
    /// Idempotente.
    ///
    /// # Errors
    ///
    /// [`CalendarError::NoWorkingTime`] ; [`CalendarError::OutOfDomain`]
    /// (`Before`) s'il n'y a aucune minute ouvrée avant `t`.
    pub fn previous_working_moment(&self, t: Moment) -> Result<Moment, CalendarError> {
        if self.total == 0 {
            return Err(self.no_working_time());
        }
        let v = self.w(t)?;
        if v == 0 {
            return Err(self.out_of_domain(Direction::Before));
        }
        self.reach(v, 0)
    }
}

/// Diagnostic de la compilation du calendrier `uid` : aucun temps ouvré, ou
/// échec. Toute erreur de compilation donne un diagnostic : celles que
/// [`Diagnostic::from_calendar_error`] ne couvre pas deviennent
/// [`Diagnostic::CalendarInternal`] (§ 6.5 : un diagnostic par calendrier en
/// échec).
fn compile_diagnostic(
    uid: CalendarUid,
    result: &Result<CompiledCalendar, CalendarError>,
) -> Option<Diagnostic> {
    match result {
        Ok(cc) if !cc.has_working_time() => Some(Diagnostic::CalendarNoWorkingTime { calendar: uid }),
        Ok(_) => None,
        Err(e) => {
            Some(Diagnostic::from_calendar_error(e).unwrap_or(Diagnostic::CalendarInternal { calendar: uid }))
        }
    }
}

/// Calendriers d'un projet, compilés une fois par recalcul (aucun cache ni
/// invalidation).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarSet {
    compiled: BTreeMap<CalendarUid, Result<CompiledCalendar, CalendarError>>,
}

impl CalendarSet {
    /// Compile chaque calendrier du projet. Rend un diagnostic par calendrier
    /// en échec (héritage) ou sans temps ouvré ; n'échoue jamais.
    #[must_use]
    pub fn compile(project: &Project) -> (Self, Diagnostics) {
        let mut compiled = BTreeMap::new();
        let mut out = Diagnostics::new();
        for c in &project.calendars {
            let result = CompiledCalendar::compile(&project.calendars, c.uid());
            if let Some(d) = compile_diagnostic(c.uid(), &result) {
                out.push(d);
            }
            compiled.insert(c.uid(), result);
        }
        (Self { compiled }, out)
    }

    /// Calendrier compilé `uid`.
    ///
    /// # Errors
    ///
    /// L'erreur de compilation du calendrier, recopiée ; ou
    /// [`CalendarError::UnknownCalendar`].
    pub fn get(&self, uid: CalendarUid) -> Result<&CompiledCalendar, CalendarError> {
        match self.compiled.get(&uid) {
            Some(Ok(c)) => Ok(c),
            Some(Err(e)) => Err(e.clone()),
            None => Err(CalendarError::UnknownCalendar { calendar: uid }),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::panic,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::{CalendarSet, CompiledCalendar, Segment, SegmentRule, compile_diagnostic};
    use crate::arena::Arena;
    use crate::calendar::Direction;
    use crate::calendar::builtin::{BuiltinCalendarNames, standard, twenty_four_hours};
    use crate::calendar::model::{Calendar, CalendarException, DayHours, DaySetting, Recurrence, TimeRange};
    use crate::diagnostic::Diagnostic;
    use crate::duration::Minutes;
    use crate::error::CalendarError;
    use crate::ids::CalendarUid;
    use crate::model::{Project, ProjectInit, ScheduleAnchor};
    use crate::time::{Day, DayRange, Moment, TimeOfDay, Week};

    fn uid(v: u32) -> CalendarUid {
        CalendarUid::new(v)
    }

    fn m(s: &str) -> Moment {
        s.parse().unwrap()
    }

    fn day(s: &str) -> Day {
        s.parse().unwrap()
    }

    fn holiday(from: &str, to: &str) -> CalendarException {
        CalendarException {
            name: String::new(),
            period: DayRange::new(day(from), day(to)).unwrap(),
            setting: DaySetting::NonWorking,
            recurrence: Recurrence::Period,
            occurrences: None,
        }
    }

    fn compile(c: Calendar) -> CompiledCalendar {
        let u = c.uid();
        CompiledCalendar::compile(&Arena::try_from(vec![c]).unwrap(), u).unwrap()
    }

    #[test]
    fn standard_with_two_holidays_segments() {
        let mut c = standard(uid(1), "Standard".to_owned());
        c.add_exception(holiday("2026-11-11", "2026-11-11")).unwrap();
        c.add_exception(holiday("2026-12-24", "2026-12-25")).unwrap();
        let cc = compile(c);
        // Motifs : chômé (0), jour Standard (1) ; une semaine.
        assert_eq!(cc.day_patterns.len(), 2);
        assert_eq!(cc.week_patterns.len(), 1);
        assert_eq!(cc.week_patterns[0].total, 2_400);
        let d1 = day("2026-11-11").index();
        let d2 = day("2026-12-24").index();
        // Semaine, férié, semaine, deux fériés, semaine.
        let wp = |d: i32| cc.week_patterns[0].cum_at(d).unwrap();
        let expected = [
            Segment {
                first_day: 0,
                rule: SegmentRule::Weekly(0),
                cum_start: 0,
                wp_start: 0,
            },
            Segment {
                first_day: d1,
                rule: SegmentRule::Daily(0),
                cum_start: wp(d1),
                wp_start: 0,
            },
            Segment {
                first_day: d1 + 1,
                rule: SegmentRule::Weekly(0),
                cum_start: wp(d1),
                wp_start: wp(d1 + 1),
            },
            Segment {
                first_day: d2,
                rule: SegmentRule::Daily(0),
                // Le mercredi 11/11 (480) n'a pas été travaillé.
                cum_start: wp(d2) - 480,
                wp_start: 0,
            },
            Segment {
                first_day: d2 + 2,
                rule: SegmentRule::Weekly(0),
                cum_start: wp(d2) - 480,
                wp_start: wp(d2 + 2),
            },
        ];
        assert_eq!(cc.segments, expected);
        // Total : la semaine répétée sur tout le domaine, moins les 3 jours
        // fériés ouvrés (mer 11/11, jeu 24/12 et ven 25/12).
        assert_eq!(cc.total, wp(60_631) - 3 * 480);
    }

    #[test]
    fn daily_segment_for_an_exception_range() {
        // Exception ouvrée [09:00,11:00) du lun 02/11 au dim 08/11/2026 : un
        // segment Daily de 7 jours, motif de 120 minutes.
        let mut c = standard(uid(1), "Standard".to_owned());
        let h = DayHours::new(vec![
            TimeRange::new(TimeOfDay::hm(9, 0).unwrap(), TimeOfDay::hm(11, 0).unwrap()).unwrap(),
        ])
        .unwrap();
        c.add_exception(CalendarException {
            setting: DaySetting::Working(h),
            ..holiday("2026-11-02", "2026-11-08")
        })
        .unwrap();
        let cc = compile(c);
        assert_eq!(cc.segments.len(), 3);
        let s = cc.segments[1];
        assert_eq!(s.first_day, day("2026-11-02").index());
        let SegmentRule::Daily(p) = s.rule else {
            panic!("segment Daily attendu");
        };
        assert_eq!(cc.day_patterns[p as usize].total, 120);
        assert_eq!(cc.segments[2].first_day, day("2026-11-09").index());
        assert_eq!(cc.segments[2].cum_start - s.cum_start, 7 * 120);
        // Semaine du 02/11 : 7 × 120 = 840.
        assert_eq!(
            cc.working_between(m("2026-11-02T00:00"), m("2026-11-09T00:00")),
            Ok(Minutes::new(840))
        );
        assert_eq!(
            cc.add_working(m("2026-11-07T10:00"), Minutes::new(120)),
            Ok(m("2026-11-08T10:00"))
        );
    }

    #[test]
    fn domain_bounds() {
        let cc = compile(twenty_four_hours(uid(2), "24 heures".to_owned()));
        assert_eq!(cc.total, 87_308_640);
        assert_eq!(cc.segments.len(), 1);
        assert_eq!(
            cc.working_between(Moment::MIN, Moment::MAX),
            Ok(Minutes::new(87_308_640))
        );
        assert_eq!(
            cc.add_working(Moment::MIN, Minutes::new(87_308_640)),
            Ok(Moment::MAX)
        );
        assert_eq!(
            cc.add_working(Moment::MIN, Minutes::new(87_308_641)),
            Err(CalendarError::OutOfDomain {
                calendar: uid(2),
                direction: Direction::After
            })
        );
        assert_eq!(
            cc.add_working(Moment::MAX, Minutes::new(-87_308_640)),
            Ok(Moment::MIN)
        );
        assert_eq!(cc.next_working_moment(Moment::MIN), Ok(Moment::MIN));
        assert_eq!(cc.previous_working_moment(Moment::MAX), Ok(Moment::MAX));
        assert_eq!(
            cc.next_working_moment(Moment::MAX),
            Err(CalendarError::OutOfDomain {
                calendar: uid(2),
                direction: Direction::After
            })
        );
        assert_eq!(
            cc.previous_working_moment(Moment::MIN),
            Err(CalendarError::OutOfDomain {
                calendar: uid(2),
                direction: Direction::Before
            })
        );
        assert_eq!(cc.is_working(Moment::MAX), Ok(false));
        assert_eq!(cc.is_working(Moment::MIN), Ok(true));
        assert_eq!(
            cc.add_working(Moment::MIN, Minutes::new(i64::MAX)),
            Err(CalendarError::OutOfDomain {
                calendar: uid(2),
                direction: Direction::After
            })
        );
        assert_eq!(
            cc.add_working(Moment::MAX, Minutes::new(i64::MIN)),
            Err(CalendarError::OutOfDomain {
                calendar: uid(2),
                direction: Direction::Before
            })
        );
    }

    #[test]
    fn contiguous_ranges_across_midnight() {
        // Lundi [20:00,24:00), mardi [00:00,04:00) : W ne voit qu'une plage.
        let range = |a: u16, b: u16| {
            DaySetting::Working(
                DayHours::new(vec![
                    TimeRange::new(TimeOfDay::hm(a, 0).unwrap(), TimeOfDay::hm(b, 0).unwrap()).unwrap(),
                ])
                .unwrap(),
            )
        };
        let week = Week::from_fn(|d| match d {
            crate::time::Weekday::Monday => range(20, 24),
            crate::time::Weekday::Tuesday => range(0, 4),
            _ => DaySetting::NonWorking,
        });
        let cc = compile(Calendar::new_base(uid(5), "Nuit".to_owned(), week));
        // Lun 19/10/2026 23:00 + 120 = mar 01:00 ; mar 00:00 − 60 = lun 23:00.
        assert_eq!(
            cc.add_working(m("2026-10-19T23:00"), Minutes::new(120)),
            Ok(m("2026-10-20T01:00"))
        );
        assert_eq!(
            cc.add_working(m("2026-10-20T00:00"), Minutes::new(-60)),
            Ok(m("2026-10-19T23:00"))
        );
        // Minuit est dans la plage : instant ouvré suivant = lui-même.
        assert_eq!(
            cc.next_working_moment(m("2026-10-20T00:00")),
            Ok(m("2026-10-20T00:00"))
        );
        assert_eq!(
            cc.day_ranges(day("2026-10-19")).unwrap(),
            [TimeRange::new(TimeOfDay::hm(20, 0).unwrap(), TimeOfDay::END_OF_DAY).unwrap()]
        );
    }

    #[test]
    fn calendar_set_diagnostics() {
        let mut p = Project::new(ProjectInit {
            name: "P".to_owned(),
            anchor: ScheduleAnchor::FromStart(m("2026-10-19T08:00")),
            current_date: m("2026-10-17T08:00"),
            calendar_names: BuiltinCalendarNames::default(),
        });
        let none = Calendar::new_base(
            uid(4),
            "Rien".to_owned(),
            Week::from_fn(|_| DaySetting::NonWorking),
        );
        p.calendars.push(none).unwrap();
        p.calendars
            .push(Calendar::new_derived(uid(5), "Orphelin".to_owned(), uid(99)))
            .unwrap();
        let (set, diags) = CalendarSet::compile(&p);
        assert_eq!(
            diags.finish(),
            [
                Diagnostic::CalendarBaseMissing {
                    calendar: uid(5),
                    base: uid(99)
                },
                Diagnostic::CalendarNoWorkingTime { calendar: uid(4) },
            ]
        );
        assert!(set.get(uid(1)).unwrap().has_working_time());
        assert!(!set.get(uid(4)).unwrap().has_working_time());
        assert_eq!(
            set.get(uid(5)).unwrap_err(),
            CalendarError::BaseMissing {
                calendar: uid(5),
                base: uid(99)
            }
        );
        assert_eq!(
            set.get(uid(7)).unwrap_err(),
            CalendarError::UnknownCalendar { calendar: uid(7) }
        );
    }

    #[test]
    fn every_compile_failure_has_a_diagnostic() {
        // Résultats fabriqués : un invariant violé (jamais produit par la
        // compilation) et des erreurs hors héritage donnent un diagnostic
        // d'erreur interne, jamais un silence.
        for e in [
            CalendarError::Inconsistent { calendar: uid(6) },
            CalendarError::UnknownCalendar { calendar: uid(6) },
            CalendarError::NegativeSpan,
        ] {
            let d = compile_diagnostic(uid(6), &Err(e)).unwrap();
            assert_eq!(d, Diagnostic::CalendarInternal { calendar: uid(6) });
            assert_eq!(d.severity(), crate::diagnostic::Severity::Error);
        }
        assert_eq!(
            compile_diagnostic(
                uid(5),
                &Err(CalendarError::BaseMissing {
                    calendar: uid(5),
                    base: uid(99)
                })
            ),
            Some(Diagnostic::CalendarBaseMissing {
                calendar: uid(5),
                base: uid(99)
            })
        );
        let a = Arena::try_from(vec![standard(uid(1), "Standard".to_owned())]).unwrap();
        assert_eq!(
            compile_diagnostic(uid(1), &CompiledCalendar::compile(&a, uid(1))),
            None
        );
    }
}
