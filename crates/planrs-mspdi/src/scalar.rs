//! Valeurs scalaires de MSPDI : booléens, entiers, dates, heures, durées
//! ISO 8601 et dixièmes de minute (R-070, R-072, R-073, R-075, R-076), en
//! lecture, et leurs formateurs inverses pour l'écriture (R-089, J3).
//!
//! Chaque fonction est pure et totale : une écriture invalide rend `None` ou
//! une erreur typée, jamais une panique. L'appelant transforme l'échec en
//! diagnostic d'import et applique la valeur par défaut (correspondance
//! indulgente, ADR-0009). Aucun flottant : les décimales des durées sont lues
//! en entiers, puis arrondies par la règle unique R-001.

use chrono::NaiveDate;
use planrs_core::{Minutes, Moment, TimeOfDay, round_div};

/// Booléen MSPDI (`xsd:boolean`) : `1`/`true` ou `0`/`false`, espaces de tête
/// et de fin ignorés (R-076). Toute autre écriture rend `None`.
#[must_use]
pub fn parse_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

/// Entier signé (`xsd:int`, `xsd:integer`) sur 64 bits, espaces ignorés.
/// Dépassement ou écriture invalide : `None`.
#[must_use]
pub fn parse_int(s: &str) -> Option<i64> {
    s.trim().parse().ok()
}

/// Curseur sur les octets d'une valeur (lecture sans indexation paniquante).
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    const fn new(s: &'a str) -> Self {
        Self {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) {
        self.pos = self.pos.saturating_add(1);
    }

    /// Consomme `b` s'il est le prochain octet.
    fn eat(&mut self, b: u8) -> bool {
        if self.peek() == Some(b) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    /// Exactement `n` chiffres décimaux.
    fn fixed_digits(&mut self, n: usize) -> Option<u32> {
        let mut v: u32 = 0;
        for _ in 0..n {
            let d = self.peek().filter(u8::is_ascii_digit)?;
            v = v.checked_mul(10)?.checked_add(u32::from(d.checked_sub(b'0')?))?;
            self.bump();
        }
        Some(v)
    }

    /// Une suite non vide de chiffres, en `i128` ; `None` si vide. Le
    /// booléen vaut vrai si la valeur déborde (les chiffres sont consommés).
    fn digits_i128(&mut self) -> Option<(i128, bool)> {
        let mut v: i128 = 0;
        let mut count: usize = 0;
        let mut overflow = false;
        while let Some(d) = self.peek().filter(u8::is_ascii_digit) {
            match v
                .checked_mul(10)
                .and_then(|x| x.checked_add(i128::from(d.saturating_sub(b'0'))))
            {
                Some(x) => v = x,
                None => overflow = true,
            }
            count = count.saturating_add(1);
            self.bump();
        }
        (count > 0).then_some((v, overflow))
    }
}

/// Partie décimale lue : valeur ramenée à 9 chiffres, et vrai si des
/// chiffres non nuls au-delà du neuvième ont été ignorés.
fn fraction_nanos(c: &mut Cursor<'_>) -> Option<(i128, bool)> {
    let mut value: i128 = 0;
    let mut count: u32 = 0;
    let mut dropped = false;
    while let Some(d) = c.peek().filter(u8::is_ascii_digit) {
        let digit = i128::from(d.checked_sub(b'0')?);
        if count < 9 {
            value = value.checked_mul(10)?.checked_add(digit)?;
            count = count.saturating_add(1);
        } else if digit != 0 {
            dropped = true;
        }
        c.bump();
    }
    if count == 0 && !dropped {
        return None;
    }
    let pad = 10_i128.checked_pow(9_u32.saturating_sub(count))?;
    Some((value.checked_mul(pad)?, dropped))
}

/// Instant lu dans un `xsd:dateTime` MSPDI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParsedMoment {
    /// Instant, à la minute (secondes tronquées, R-004).
    pub moment: Moment,
    /// Vrai si des secondes ou fractions non nulles ont été tronquées.
    pub rounded: bool,
    /// Vrai si un suffixe de fuseau (`Z`, `±hh:mm`) a été ignoré.
    pub zone_ignored: bool,
}

/// Échec de lecture d'une date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DateTimeError {
    /// Écriture invalide.
    #[error("date invalide")]
    Syntax,
    /// Date valide hors du domaine 1984-2149 (R-003).
    #[error("date hors du domaine 1984-2149")]
    OutOfDomain,
}

/// Heure `hh:mm[:ss[.fff]]` : minutes depuis minuit (0 à 1 440) et
/// troncature des secondes. `24:00` n'est admis que sans secondes.
fn clock(c: &mut Cursor<'_>) -> Option<(u16, bool)> {
    let h = c.fixed_digits(2)?;
    if !c.eat(b':') {
        return None;
    }
    let m = c.fixed_digits(2)?;
    let mut rounded = false;
    if c.eat(b':') {
        let s = c.fixed_digits(2)?;
        if s >= 60 {
            return None;
        }
        rounded = s != 0;
        if c.eat(b'.') {
            let (nanos, dropped) = fraction_nanos(c)?;
            rounded = rounded || nanos != 0 || dropped;
        }
    }
    if m >= 60 || h > 24 || (h == 24 && (m != 0 || rounded)) {
        return None;
    }
    let minutes = u16::try_from(h.checked_mul(60)?.checked_add(m)?).ok()?;
    Some((minutes, rounded))
}

/// R-075 : `AAAA-MM-JJThh:mm[:ss[.fff]]`, sans fuseau. Un suffixe `Z` ou
/// `±hh:mm` est ignoré et signalé (`zone_ignored`) ; les secondes sont
/// tronquées (R-004) ; `24:00:00` désigne le lendemain à 00:00 (XSD).
///
/// # Errors
///
/// [`DateTimeError::Syntax`] pour une écriture invalide,
/// [`DateTimeError::OutOfDomain`] hors de 1984-2149.
pub fn parse_datetime(s: &str) -> Result<ParsedMoment, DateTimeError> {
    let mut c = Cursor::new(s.trim());
    let date = (|| {
        let y = c.fixed_digits(4)?;
        if !c.eat(b'-') {
            return None;
        }
        let mo = c.fixed_digits(2)?;
        if !c.eat(b'-') {
            return None;
        }
        let d = c.fixed_digits(2)?;
        if !c.eat(b'T') {
            return None;
        }
        let (minutes, rounded) = clock(&mut c)?;
        let zone_ignored = zone_suffix(&mut c)?;
        c.at_end().then_some(())?;
        let date = NaiveDate::from_ymd_opt(i32::try_from(y).ok()?, mo, d)?;
        Some((date, minutes, rounded, zone_ignored))
    })();
    let (date, minutes, rounded, zone_ignored) = date.ok_or(DateTimeError::Syntax)?;
    // 24:00 : minuit du lendemain (le 31/12/2149 à 24:00 est `Moment::MAX`).
    let (date, minutes) = if minutes == 1_440 {
        (date.succ_opt().ok_or(DateTimeError::OutOfDomain)?, 0)
    } else {
        (date, minutes)
    };
    let naive = date
        .and_hms_opt(u32::from(minutes / 60), u32::from(minutes % 60), 0)
        .ok_or(DateTimeError::Syntax)?;
    let (moment, _) = Moment::from_naive_floor(naive).map_err(|_| DateTimeError::OutOfDomain)?;
    Ok(ParsedMoment {
        moment,
        rounded,
        zone_ignored,
    })
}

/// Suffixe de fuseau : rien (`false`), `Z` ou `±hh:mm` (`true`) ; `None` si
/// l'écriture est invalide.
fn zone_suffix(c: &mut Cursor<'_>) -> Option<bool> {
    if c.at_end() {
        return Some(false);
    }
    if c.eat(b'Z') {
        return Some(true);
    }
    if c.eat(b'+') || c.eat(b'-') {
        let h = c.fixed_digits(2)?;
        if !c.eat(b':') {
            return None;
        }
        let m = c.fixed_digits(2)?;
        return (h <= 14 && m < 60).then_some(true);
    }
    None
}

/// R-077 : heure `hh:mm[:ss]` d'une plage (`FromTime`, `ToTime`). Rend l'heure
/// et la troncature des secondes ; `None` si l'écriture est invalide.
/// `00:00:00` reste minuit : c'est l'appelant qui lit une fin à 00:00 comme
/// 24:00.
#[must_use]
pub fn parse_time(s: &str) -> Option<(TimeOfDay, bool)> {
    let mut c = Cursor::new(s.trim());
    let (minutes, rounded) = clock(&mut c)?;
    if !c.at_end() {
        return None;
    }
    TimeOfDay::new(minutes).ok().map(|t| (t, rounded))
}

/// Durée ISO 8601 lue (R-070), en minutes signées.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsoDuration {
    /// Minutes, arrondies par R-001 ; négatives pour `-P…`.
    pub minutes: i64,
    /// Vrai si des secondes (ou fractions) ont été arrondies à la minute.
    pub rounded: bool,
}

/// Échec de lecture d'une durée ISO 8601.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum IsoDurationError {
    /// Écriture invalide.
    #[error("durée ISO 8601 invalide")]
    Syntax,
    /// Années ou mois non nuls : longueur variable, refusée.
    #[error("durée en années ou en mois refusée")]
    YearsOrMonths,
    /// Valeur trop grande.
    #[error("durée hors limites")]
    Overflow,
}

/// Nanosecondes dans une minute.
const NANOS_PER_MINUTE: i128 = 60_000_000_000;
/// Nanosecondes dans une seconde.
const NANOS_PER_SECOND: i128 = 1_000_000_000;

/// R-070 : durée `[-]PnDTnHnMnS` (`xsd:duration`) en minutes. Un jour vaut
/// 24 heures ; des décimales sont admises dans la dernière composante
/// seulement ; les secondes sont arrondies à la minute par R-001 (`rounded`).
/// Années et mois non nuls refusés (longueur variable) ; semaines (`W`) hors
/// `xsd:duration`, refusées.
///
/// Exemples : `PT80H0M0S` → 4 800 ; `PT8H30M0S` → 510 ; `PT0H0M30S` → 1 ;
/// `PT1.5H` → 90 ; `P2D` → 2 880 ; `-PT8H` → −480.
///
/// # Errors
///
/// [`IsoDurationError`].
pub fn parse_iso_duration(s: &str) -> Result<IsoDuration, IsoDurationError> {
    let mut c = Cursor::new(s.trim());
    let negative = c.eat(b'-');
    if !negative {
        let _ = c.eat(b'+');
    }
    if !c.eat(b'P') {
        return Err(IsoDurationError::Syntax);
    }
    // Composantes dans l'ordre ISO : Y, M, D, puis T, H, M, S.
    // Rang du dernier désignateur lu (0 = aucun) pour imposer l'ordre.
    let mut last_rank: u8 = 0;
    let mut in_time = false;
    let mut components: u32 = 0;
    let mut time_components: u32 = 0;
    let mut saw_fraction = false;
    let mut total: i128 = 0;
    let mut years_or_months = false;
    loop {
        if c.at_end() {
            break;
        }
        if c.eat(b'T') {
            if in_time {
                return Err(IsoDurationError::Syntax);
            }
            in_time = true;
            continue;
        }
        if saw_fraction {
            // Une décimale n'est admise que dans la dernière composante.
            return Err(IsoDurationError::Syntax);
        }
        let (int, int_overflow) = c.digits_i128().ok_or(IsoDurationError::Syntax)?;
        let frac = if c.eat(b'.') {
            saw_fraction = true;
            Some(fraction_nanos(&mut c).ok_or(IsoDurationError::Syntax)?)
        } else {
            None
        };
        let designator = c.peek().ok_or(IsoDurationError::Syntax)?;
        c.bump();
        let (rank, seconds): (u8, i128) = match (in_time, designator) {
            (false, b'Y') => (1, 0),
            (false, b'M') => (2, 0),
            (false, b'D') => (3, 86_400),
            (true, b'H') => (4, 3_600),
            (true, b'M') => (5, 60),
            (true, b'S') => (6, 1),
            _ => return Err(IsoDurationError::Syntax),
        };
        if rank <= last_rank {
            return Err(IsoDurationError::Syntax);
        }
        last_rank = rank;
        components = components.saturating_add(1);
        if in_time {
            time_components = time_components.saturating_add(1);
        }
        let (frac_nanos, dropped) = frac.unwrap_or((0, false));
        if int_overflow {
            return Err(IsoDurationError::Overflow);
        }
        if rank <= 2 {
            if int != 0 || frac_nanos != 0 || dropped {
                years_or_months = true;
            }
            continue;
        }
        let unit = seconds
            .checked_mul(NANOS_PER_SECOND)
            .ok_or(IsoDurationError::Overflow)?;
        let part = int
            .checked_mul(unit)
            .and_then(|v| v.checked_add(frac_nanos.checked_mul(seconds)?))
            .ok_or(IsoDurationError::Overflow)?;
        total = total.checked_add(part).ok_or(IsoDurationError::Overflow)?;
        if dropped {
            // Chiffres au-delà de la nanoseconde : la valeur n'est plus exacte,
            // l'arrondi est signalé.
            total = total.checked_add(1).ok_or(IsoDurationError::Overflow)?;
        }
    }
    if components == 0 || (in_time && time_components == 0) {
        return Err(IsoDurationError::Syntax);
    }
    if years_or_months {
        return Err(IsoDurationError::YearsOrMonths);
    }
    let signed = if negative {
        total.checked_neg().ok_or(IsoDurationError::Overflow)?
    } else {
        total
    };
    let minutes = round_div(signed, NANOS_PER_MINUTE).ok_or(IsoDurationError::Overflow)?;
    let rounded = signed.checked_rem(NANOS_PER_MINUTE).is_some_and(|r| r != 0);
    Ok(IsoDuration { minutes, rounded })
}

/// R-072 : `LinkLag` (dixièmes de minute) → minutes, `round_div(lag, 10)`
/// (R-001). Le booléen vaut vrai si un reste a été arrondi. Exemples :
/// 4 800 → 480 ; −4 800 → −480 ; 25 → 3 (arrondi) ; −25 → −3.
#[must_use]
pub fn tenths_to_minutes(tenths: i64) -> (Minutes, bool) {
    // |tenths / 10| tient toujours dans un i64 : le repli ne sert jamais.
    let minutes = round_div(i128::from(tenths), 10).unwrap_or(0);
    (
        Minutes::new(minutes),
        tenths.checked_rem(10).is_some_and(|r| r != 0),
    )
}

/// R-073 (hypothèse pure) : un décalage en pourcentage est écrit
/// `LinkLag` = pourcentage × 10, donc la valeur en 1/10 000 vaut
/// `LinkLag × 10`. Exemple : 500 → 5 000 (50 %). `None` en cas de
/// débordement.
#[must_use]
pub const fn percent_lag_from_link_lag(link_lag: i64) -> Option<i64> {
    link_lag.checked_mul(10)
}

// ---------------------------------------------------------------------------
// Écriture (J3, lot W1, R-089) : formateurs inverses des lecteurs ci-dessus.
//
// Entiers seulement, aucune panique : les formateurs écrivent dans un tampon
// `String` fourni par l'appelant (réutilisé d'un champ à l'autre, aucune
// allocation par champ) ; les dates sont formatées à la main, sans
// `chrono::format` (trop lent pour 60 000 dates). Chaque formateur `push_*`
// a sa variante `format_*` qui rend une `String` neuve (tests, usages rares).
// ---------------------------------------------------------------------------

/// Chiffre décimal `d` (0 à 9) en caractère ASCII ; `'0'` au-delà (jamais
/// atteint : les appelants passent un reste de division par 10).
fn digit(d: u32) -> char {
    char::from_digit(d, 10).unwrap_or('0')
}

/// Écrit `v` sur exactement deux chiffres (`v` < 100 ; au-delà, les deux
/// derniers chiffres).
fn push_2(out: &mut String, v: u32) {
    out.push(digit(v.checked_div(10).unwrap_or(0).checked_rem(10).unwrap_or(0)));
    out.push(digit(v.checked_rem(10).unwrap_or(0)));
}

/// Écrit un entier non signé en décimal, sans zéro de tête.
fn push_u64(out: &mut String, v: u64) {
    // 20 chiffres suffisent pour u64::MAX.
    let mut buf = ['0'; 20];
    let mut n = v;
    let mut len: usize = 0;
    loop {
        if let Some(slot) = buf.get_mut(len) {
            *slot = digit(u32::try_from(n.checked_rem(10).unwrap_or(0)).unwrap_or(0));
        }
        len = len.saturating_add(1);
        n = n.checked_div(10).unwrap_or(0);
        if n == 0 || len >= buf.len() {
            break;
        }
    }
    for c in buf.iter().take(len).rev() {
        out.push(*c);
    }
}

/// R-089 : entier signé en décimal (`xsd:int`, `xsd:integer`), sans zéro de
/// tête ni signe `+` : `-28800`, `0`, `9600`. Inverse de [`parse_int`].
pub fn push_int(out: &mut String, v: i64) {
    if v < 0 {
        out.push('-');
    }
    push_u64(out, v.unsigned_abs());
}

/// [`push_int`] dans une `String` neuve.
#[must_use]
pub fn format_int(v: i64) -> String {
    let mut out = String::with_capacity(20);
    push_int(&mut out, v);
    out
}

/// R-089 : booléen écrit `1` ou `0` (forme observée dans le corpus ; inverse
/// de [`parse_bool`]).
#[must_use]
pub const fn format_bool(b: bool) -> &'static str {
    if b { "1" } else { "0" }
}

/// R-089 : instant écrit `AAAA-MM-JJTHH:MM:00` (secondes toujours nulles,
/// heure locale sans fuseau ; inverse de [`parse_datetime`]).
/// [`Moment::MAX`] (31/12/2149 à 24:00) s'écrit `2150-01-01T00:00:00`, que
/// la lecture rend à `Moment::MAX`. Exemple : lun 19/10/2026 08:00 →
/// `2026-10-19T08:00:00`.
pub fn push_datetime(out: &mut String, m: Moment) {
    use chrono::{Datelike, Timelike};
    let dt = m.to_naive();
    // Année du domaine : 1984 à 2150, toujours quatre chiffres.
    let year = u32::try_from(dt.year()).unwrap_or(0);
    push_2(out, year.checked_div(100).unwrap_or(0));
    push_2(out, year.checked_rem(100).unwrap_or(0));
    out.push('-');
    push_2(out, dt.month());
    out.push('-');
    push_2(out, dt.day());
    out.push('T');
    push_2(out, dt.hour());
    out.push(':');
    push_2(out, dt.minute());
    out.push_str(":00");
}

/// [`push_datetime`] dans une `String` neuve.
#[must_use]
pub fn format_datetime(m: Moment) -> String {
    let mut out = String::with_capacity(19);
    push_datetime(&mut out, m);
    out
}

/// R-089 : heure d'une plage (`FromTime`, `ToTime`) écrite `HH:MM:00`.
/// 24:00 (fin de jour) s'écrit `00:00:00` : c'est l'inverse de R-077, qui
/// lit une fin de plage à 00:00 comme 24:00. Exemples : 17:00 →
/// `17:00:00` ; 24:00 → `00:00:00`.
pub fn push_time(out: &mut String, t: TimeOfDay) {
    let minutes = u32::from(t.minutes()).checked_rem(1_440).unwrap_or(0);
    push_2(out, minutes.checked_div(60).unwrap_or(0));
    out.push(':');
    push_2(out, minutes.checked_rem(60).unwrap_or(0));
    out.push_str(":00");
}

/// [`push_time`] dans une `String` neuve.
#[must_use]
pub fn format_time(t: TimeOfDay) -> String {
    let mut out = String::with_capacity(8);
    push_time(&mut out, t);
    out
}

/// R-089 : durée écrite `PT{h}H{m}M0S` (heures et minutes, secondes nulles),
/// ouvrée comme écoulée (la base est portée par `DurationFormat`) ; une
/// durée négative s'écrit `-PT…`. Inverse de [`parse_iso_duration`].
/// Exemples : 510 → `PT8H30M0S` ; 4 800 → `PT80H0M0S` ; 0 → `PT0H0M0S` ;
/// 87 308 640 → `PT1455144H0M0S` ; −480 → `-PT8H0M0S`.
pub fn push_iso_duration(out: &mut String, minutes: Minutes) {
    let v = minutes.get();
    if v < 0 {
        out.push('-');
    }
    let abs = v.unsigned_abs();
    out.push_str("PT");
    push_u64(out, abs.checked_div(60).unwrap_or(0));
    out.push('H');
    push_u64(out, abs.checked_rem(60).unwrap_or(0));
    out.push_str("M0S");
}

/// [`push_iso_duration`] dans une `String` neuve.
#[must_use]
pub fn format_iso_duration(minutes: Minutes) -> String {
    let mut out = String::with_capacity(16);
    push_iso_duration(&mut out, minutes);
    out
}

/// R-089 : minutes → dixièmes de minute (`LinkLag` temporel, marges
/// `TotalSlack`/`FreeSlack`), inverse exact de R-072 et R-083 :
/// minutes × 10. Exemples : 960 → 9 600 ; −2 880 → −28 800. `None` en cas
/// de débordement (jamais sur le domaine : |minutes| ≤ 87 308 640).
#[must_use]
pub const fn minutes_to_tenths(minutes: Minutes) -> Option<i64> {
    minutes.get().checked_mul(10)
}

/// R-089 : pourcentage du modèle (1/10 000) → pourcentage entier MSPDI
/// (`PercentComplete`…), `round_div(v, 100)` (R-001). Le booléen vaut vrai
/// si un reste a été arrondi (perte, signalée par l'écrivain). Exemples :
/// 5 000 → (50, faux) ; 5 050 → (51, vrai) ; 10 000 → (100, faux).
#[must_use]
pub fn percent_to_whole(percent: i64) -> (i64, bool) {
    // |v / 100| tient toujours dans un i64 : le repli ne sert jamais.
    let whole = round_div(i128::from(percent), 100).unwrap_or(0);
    (whole, percent.checked_rem(100).is_some_and(|r| r != 0))
}

/// R-089 : décalage en pourcentage du modèle (1/10 000) → `LinkLag`
/// (pourcentage × 10, hypothèse R-073) : `round_div(p, 10)` (R-001), inverse
/// de [`percent_lag_from_link_lag`] sur les multiples de 10. Le booléen vaut
/// vrai si un reste a été arrondi. Exemples : 5 000 (50 %) → (500, faux) ;
/// 5 005 → (501, vrai) ; −2 500 → (−250, faux).
#[must_use]
pub fn link_lag_from_percent_lag(percent: i64) -> (i64, bool) {
    // |p / 10| tient toujours dans un i64 : le repli ne sert jamais.
    let lag = round_div(i128::from(percent), 10).unwrap_or(0);
    (lag, percent.checked_rem(10).is_some_and(|r| r != 0))
}

/// Caractère interdit dans un texte XML 1.0 : contrôles U+0000 à U+001F
/// sauf tabulation, saut de ligne et retour chariot, et les non-caractères
/// U+FFFE, U+FFFF (les demi-codets ne peuvent pas figurer dans un `str`).
#[must_use]
pub const fn is_forbidden_xml_char(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}')
}

/// R-089 : texte échappé pour le contenu d'un élément : `&` → `&amp;`,
/// `<` → `&lt;`, `>` → `&gt;`, `\r` → `&#13;` (sans quoi la relecture le
/// normaliserait en `\n`) ; un caractère interdit en XML 1.0
/// ([`is_forbidden_xml_char`]) est remplacé par U+FFFD. Rend vrai si au
/// moins un caractère a été remplacé (perte, déclarée par l'écrivain).
/// Exemple : `a<b & c\r\n` → `a&lt;b &amp; c&#13;\n`.
pub fn push_escaped_text(out: &mut String, s: &str) -> bool {
    let mut replaced = false;
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            c if is_forbidden_xml_char(c) => {
                out.push('\u{FFFD}');
                replaced = true;
            }
            c => out.push(c),
        }
    }
    replaced
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::arithmetic_side_effects,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::{
        DateTimeError, IsoDuration, IsoDurationError, format_bool, format_datetime, format_int,
        format_iso_duration, format_time, is_forbidden_xml_char, link_lag_from_percent_lag,
        minutes_to_tenths, parse_bool, parse_datetime, parse_int, parse_iso_duration, parse_time,
        percent_lag_from_link_lag, percent_to_whole, push_escaped_text, tenths_to_minutes,
    };
    use planrs_core::{Minutes, Moment, TimeOfDay};

    fn m(s: &str) -> Moment {
        s.parse().unwrap()
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "aide de test : se compare directement au résultat de parse_iso_duration"
    )]
    fn iso(minutes: i64, rounded: bool) -> Result<IsoDuration, IsoDurationError> {
        Ok(IsoDuration { minutes, rounded })
    }

    #[test]
    fn booleans_and_integers() {
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool(" true "), Some(true));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("2"), None);
        assert_eq!(parse_bool("TRUE"), None);
        assert_eq!(parse_bool(""), None);
        assert_eq!(parse_int(" -65533 "), Some(-65_533));
        assert_eq!(parse_int("+7"), Some(7));
        assert_eq!(parse_int("1.5"), None);
        assert_eq!(parse_int("99999999999999999999"), None);
        assert_eq!(parse_int(""), None);
    }

    #[test]
    fn iso_durations() {
        assert_eq!(parse_iso_duration("PT80H0M0S"), iso(4_800, false));
        assert_eq!(parse_iso_duration("PT0H0M0S"), iso(0, false));
        assert_eq!(parse_iso_duration("PT8H30M0S"), iso(510, false));
        assert_eq!(parse_iso_duration("PT1.5H"), iso(90, false));
        assert_eq!(parse_iso_duration("P2D"), iso(2_880, false));
        assert_eq!(parse_iso_duration("P1DT1H"), iso(1_500, false));
        // 30 s = 0,5 min → 1 (moitié loin de zéro), arrondi signalé.
        assert_eq!(parse_iso_duration("PT0H0M30S"), iso(1, true));
        assert_eq!(parse_iso_duration("PT0H0M29S"), iso(0, true));
        assert_eq!(parse_iso_duration("PT0H0M0.5S"), iso(0, true));
        assert_eq!(parse_iso_duration("-PT8H"), iso(-480, false));
        assert_eq!(parse_iso_duration("-PT0H0M30S"), iso(-1, true));
        // Années et mois : refusés s'ils sont non nuls.
        assert_eq!(parse_iso_duration("P1M"), Err(IsoDurationError::YearsOrMonths));
        assert_eq!(parse_iso_duration("P1Y"), Err(IsoDurationError::YearsOrMonths));
        assert_eq!(parse_iso_duration("P0Y0M0DT8H0M0S"), iso(480, false));
        // Écritures invalides.
        for bad in [
            "",
            "P",
            "PT",
            "8H",
            "PT8",
            "PT8X",
            "PT1.5H30M",
            "PT8H8H",
            "PT30M8H",
            "P1W",
            "PTT8H",
            "P.5D",
            "P1D T1H",
        ] {
            assert_eq!(parse_iso_duration(bad), Err(IsoDurationError::Syntax), "{bad}");
        }
        // 10^14 h tient en minutes ; 10^40 h déborde.
        assert_eq!(
            parse_iso_duration("PT99999999999999H"),
            iso(5_999_999_999_999_940, false)
        );
        assert_eq!(
            parse_iso_duration("PT9999999999999999999999999999999999999999H"),
            Err(IsoDurationError::Overflow)
        );
    }

    #[test]
    fn datetimes() {
        let p = parse_datetime("2009-11-09T08:00:00").unwrap();
        assert_eq!(p.moment, m("2009-11-09T08:00"));
        assert!(!p.rounded && !p.zone_ignored);
        let p = parse_datetime("2026-08-07T18:18:20").unwrap();
        assert_eq!(p.moment, m("2026-08-07T18:18"));
        assert!(p.rounded);
        let p = parse_datetime("2026-08-07T18:18").unwrap();
        assert_eq!(p.moment, m("2026-08-07T18:18"));
        let p = parse_datetime("2026-08-07T18:18:00.000").unwrap();
        assert!(!p.rounded);
        let p = parse_datetime("2026-08-07T18:18:00Z").unwrap();
        assert!(p.zone_ignored);
        let p = parse_datetime("2026-08-07T18:18:00+02:00").unwrap();
        assert_eq!(p.moment, m("2026-08-07T18:18"));
        assert!(p.zone_ignored);
        // 24:00:00 = lendemain 00:00.
        assert_eq!(
            parse_datetime("2026-10-19T24:00:00").unwrap().moment,
            m("2026-10-20T00:00")
        );
        assert_eq!(
            parse_datetime("1983-12-31T08:00:00"),
            Err(DateTimeError::OutOfDomain)
        );
        assert_eq!(parse_datetime("2150-01-01T00:00:00").unwrap().moment, Moment::MAX);
        assert_eq!(
            parse_datetime("2150-01-01T00:01:00"),
            Err(DateTimeError::OutOfDomain)
        );
        for bad in [
            "",
            "2026-10-19",
            "2026-13-01T08:00:00",
            "2026-02-30T08:00:00",
            "2026-10-19T25:00:00",
            "2026-10-19T24:00:01",
            "2026-10-19T08:60:00",
            "2026-10-19 08:00:00",
            "26-10-19T08:00:00",
            "2026-10-19T08:00:00+2",
            "2026-10-19T08:00:00garbage",
        ] {
            assert_eq!(parse_datetime(bad), Err(DateTimeError::Syntax), "{bad}");
        }
    }

    #[test]
    fn times_of_day() {
        assert_eq!(
            parse_time("08:00:00"),
            Some((TimeOfDay::hm(8, 0).unwrap(), false))
        );
        assert_eq!(parse_time("00:00:00"), Some((TimeOfDay::MIDNIGHT, false)));
        assert_eq!(parse_time("24:00:00"), Some((TimeOfDay::END_OF_DAY, false)));
        assert_eq!(parse_time("08:00:30"), Some((TimeOfDay::hm(8, 0).unwrap(), true)));
        assert_eq!(parse_time("8:00:00"), None);
        assert_eq!(parse_time("08:00:00Z"), None);
    }

    #[test]
    fn tenths_and_percent_lags() {
        assert_eq!(tenths_to_minutes(4_800), (Minutes::new(480), false));
        assert_eq!(tenths_to_minutes(-4_800), (Minutes::new(-480), false));
        assert_eq!(tenths_to_minutes(25), (Minutes::new(3), true));
        assert_eq!(tenths_to_minutes(-25), (Minutes::new(-3), true));
        assert_eq!(tenths_to_minutes(i64::MIN).0.get(), -922_337_203_685_477_581);
        assert_eq!(percent_lag_from_link_lag(500), Some(5_000));
        assert_eq!(percent_lag_from_link_lag(-250), Some(-2_500));
        assert_eq!(percent_lag_from_link_lag(i64::MAX), None);
    }

    #[test]
    fn written_integers_and_booleans() {
        for v in [0, 1, -1, 9_600, -28_800, i64::MAX, i64::MIN, 1_000_000_007] {
            assert_eq!(format_int(v), v.to_string());
            assert_eq!(parse_int(&format_int(v)), Some(v));
        }
        assert_eq!(format_bool(true), "1");
        assert_eq!(format_bool(false), "0");
        assert_eq!(parse_bool(format_bool(true)), Some(true));
        assert_eq!(parse_bool(format_bool(false)), Some(false));
    }

    #[test]
    fn written_datetimes() {
        assert_eq!(format_datetime(m("2026-10-19T08:00")), "2026-10-19T08:00:00");
        assert_eq!(format_datetime(Moment::MIN), "1984-01-01T00:00:00");
        assert_eq!(format_datetime(Moment::MAX), "2150-01-01T00:00:00");
        assert_eq!(format_datetime(m("2149-12-31T23:59")), "2149-12-31T23:59:00");
        // Aller-retour sur tout le domaine, une minute sur 997 (premier) :
        // toutes les heures et tous les jours sont parcourus.
        let mut minute = 0_i64;
        while minute <= Moment::MAX.minutes() {
            let mo = Moment::from_minutes(minute).unwrap();
            let p = parse_datetime(&format_datetime(mo)).unwrap();
            assert_eq!(p.moment, mo);
            assert!(!p.rounded && !p.zone_ignored);
            minute += 997;
        }
        let p = parse_datetime(&format_datetime(Moment::MAX)).unwrap();
        assert_eq!(p.moment, Moment::MAX);
    }

    #[test]
    fn written_times() {
        assert_eq!(format_time(TimeOfDay::hm(8, 0).unwrap()), "08:00:00");
        assert_eq!(format_time(TimeOfDay::hm(17, 30).unwrap()), "17:30:00");
        assert_eq!(format_time(TimeOfDay::MIDNIGHT), "00:00:00");
        // 24:00 → 00:00:00 (inverse de R-077, qui lit une fin à 00:00 comme 24:00).
        assert_eq!(format_time(TimeOfDay::END_OF_DAY), "00:00:00");
        for minutes in 0..1_440_u16 {
            let t = TimeOfDay::new(minutes).unwrap();
            assert_eq!(parse_time(&format_time(t)), Some((t, false)));
        }
    }

    #[test]
    fn written_iso_durations() {
        assert_eq!(format_iso_duration(Minutes::new(510)), "PT8H30M0S");
        assert_eq!(format_iso_duration(Minutes::new(4_800)), "PT80H0M0S");
        assert_eq!(format_iso_duration(Minutes::ZERO), "PT0H0M0S");
        assert_eq!(format_iso_duration(Minutes::new(87_308_640)), "PT1455144H0M0S");
        assert_eq!(format_iso_duration(Minutes::new(-480)), "-PT8H0M0S");
        assert_eq!(format_iso_duration(Minutes::new(-1)), "-PT0H1M0S");
        // Bornes de i64 : aucune panique ; i64::MIN s'écrit par sa valeur absolue.
        assert_eq!(
            format_iso_duration(Minutes::new(i64::MIN)),
            "-PT153722867280912930H8M0S"
        );
        for v in (-100_000_i64..=100_000)
            .step_by(7)
            .chain([87_308_640, -87_308_640])
        {
            assert_eq!(
                parse_iso_duration(&format_iso_duration(Minutes::new(v))),
                iso(v, false)
            );
        }
    }

    #[test]
    fn written_tenths_and_percentages() {
        assert_eq!(minutes_to_tenths(Minutes::new(960)), Some(9_600));
        assert_eq!(minutes_to_tenths(Minutes::new(-2_880)), Some(-28_800));
        assert_eq!(minutes_to_tenths(Minutes::new(i64::MAX)), None);
        for v in [-87_308_640_i64, -1, 0, 1, 480, 87_308_640] {
            let tenths = minutes_to_tenths(Minutes::new(v)).unwrap();
            assert_eq!(tenths_to_minutes(tenths), (Minutes::new(v), false));
        }
        assert_eq!(percent_to_whole(5_000), (50, false));
        assert_eq!(percent_to_whole(5_050), (51, true));
        assert_eq!(percent_to_whole(5_049), (50, true));
        assert_eq!(percent_to_whole(10_000), (100, false));
        assert_eq!(percent_to_whole(0), (0, false));
        assert_eq!(link_lag_from_percent_lag(5_000), (500, false));
        assert_eq!(link_lag_from_percent_lag(5_005), (501, true));
        assert_eq!(link_lag_from_percent_lag(-2_500), (-250, false));
        assert_eq!(link_lag_from_percent_lag(-5_005), (-501, true));
        assert_eq!(link_lag_from_percent_lag(i64::MIN).0, -922_337_203_685_477_581);
        for p in (-1_000_000_i64..=1_000_000).step_by(10) {
            let (lag, lossy) = link_lag_from_percent_lag(p);
            assert!(!lossy);
            assert_eq!(percent_lag_from_link_lag(lag), Some(p));
        }
    }

    #[test]
    fn escaped_text() {
        let mut out = String::new();
        assert!(!push_escaped_text(&mut out, "a<b & c>\r\n\té"));
        assert_eq!(out, "a&lt;b &amp; c&gt;&#13;\n\té");
        let mut out = String::new();
        assert!(push_escaped_text(&mut out, "x\u{0}y\u{1F}z\u{FFFF}"));
        assert_eq!(out, "x\u{FFFD}y\u{FFFD}z\u{FFFD}");
        assert!(is_forbidden_xml_char('\u{B}'));
        assert!(!is_forbidden_xml_char('\t'));
        assert!(!is_forbidden_xml_char('\u{FFFD}'));
        assert!(!is_forbidden_xml_char('\u{10FFFF}'));
    }
}
