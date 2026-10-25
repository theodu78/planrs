//! Textes du planning pour le rendu (règle 3, ADR-0016 D7) : implémentation
//! de [`BarTextSource`] sur une session. Les noms, ressources, dates,
//! durées et pourcentages autour des barres, dans l'en-tête et le pied de
//! page sont formatés ici, à partir du modèle et du planning du moteur ;
//! `planrs-render` et `planrs-pdf` ne font que les placer.
//!
//! Formes (hypothèses H-R, `docs/features/render.md`) : dates sans heure
//! (`JJ/MM/AAAA`, `MM/DD/YYYY` en anglais, R-143) ; durées en toutes lettres
//! comme la table (« 5 jours », « 1,5 day ») ; récapitulative et marge en
//! jours ; % entier (« 50 % ») ; ressources comme la colonne « Noms
//! ressources » (« Maçon[50 %];Électricien ») ; « début – fin ».

use planrs_core::{DurationUnit, LinkType, Moment, TaskOutcome, TaskRole, TaskUid, TimeBasis};
use planrs_render::geometry::{BarTextSource, Token};
use planrs_render::style::BarField;

use crate::session::Session;
use crate::text::{
    DisplayLabels, Language, PluralRule, TextContext, display_duration, display_minutes_as, display_percent,
    format_date, format_resource_names,
};

/// Source des textes d'une session, dans une langue.
#[derive(Clone, Copy, Debug)]
pub struct SessionTexts<'a> {
    /// Session (projet et planning du moteur).
    pub session: &'a Session,
    /// Langue des textes.
    pub lang: Language,
}

/// Mots des unités et marques d'une langue (mêmes formes que les `.ftl` de
/// l'interface, J7 lot L6).
#[derive(Clone, Copy, Debug)]
struct Labels(Language);

impl DisplayLabels for Labels {
    fn decimal_sep(&self) -> char {
        match self.0 {
            Language::Fr => ',',
            Language::En => '.',
        }
    }

    fn list_sep(&self) -> &str {
        match self.0 {
            Language::Fr => ";",
            Language::En => ",",
        }
    }

    fn plural(&self) -> PluralRule {
        match self.0 {
            Language::Fr => PluralRule::BelowTwo,
            Language::En => PluralRule::ExactlyOne,
        }
    }

    #[allow(clippy::match_same_arms, reason = "une ligne par langue, lue comme une table")]
    fn unit_word(&self, unit: DurationUnit, basis: TimeBasis, singular: bool) -> &str {
        let elapsed = basis == TimeBasis::Elapsed;
        match (self.0, unit, elapsed, singular) {
            (Language::Fr, DurationUnit::Minute, false, true) => "minute",
            (Language::Fr, DurationUnit::Minute, false, false) => "minutes",
            (Language::Fr, DurationUnit::Hour, false, true) => "heure",
            (Language::Fr, DurationUnit::Hour, false, false) => "heures",
            (Language::Fr, DurationUnit::Day, false, true) => "jour",
            (Language::Fr, DurationUnit::Day, false, false) => "jours",
            (Language::Fr, DurationUnit::Week, false, true) => "semaine",
            (Language::Fr, DurationUnit::Week, false, false) => "semaines",
            (Language::Fr, DurationUnit::Month, false, _) => "mois",
            (Language::Fr, DurationUnit::Minute, true, true) => "minute écoulée",
            (Language::Fr, DurationUnit::Minute, true, false) => "minutes écoulées",
            (Language::Fr, DurationUnit::Hour, true, true) => "heure écoulée",
            (Language::Fr, DurationUnit::Hour, true, false) => "heures écoulées",
            (Language::Fr, DurationUnit::Day, true, true) => "jour écoulé",
            (Language::Fr, DurationUnit::Day, true, false) => "jours écoulés",
            (Language::Fr, DurationUnit::Week, true, true) => "semaine écoulée",
            (Language::Fr, DurationUnit::Week, true, false) => "semaines écoulées",
            (Language::Fr, DurationUnit::Month, true, true) => "mois écoulé",
            (Language::Fr, DurationUnit::Month, true, false) => "mois écoulés",
            (Language::En, DurationUnit::Minute, false, true) => "minute",
            (Language::En, DurationUnit::Minute, false, false) => "minutes",
            (Language::En, DurationUnit::Hour, false, true) => "hour",
            (Language::En, DurationUnit::Hour, false, false) => "hours",
            (Language::En, DurationUnit::Day, false, true) => "day",
            (Language::En, DurationUnit::Day, false, false) => "days",
            (Language::En, DurationUnit::Week, false, true) => "week",
            (Language::En, DurationUnit::Week, false, false) => "weeks",
            (Language::En, DurationUnit::Month, false, true) => "month",
            (Language::En, DurationUnit::Month, false, false) => "months",
            (Language::En, DurationUnit::Minute, true, true) => "eminute",
            (Language::En, DurationUnit::Minute, true, false) => "eminutes",
            (Language::En, DurationUnit::Hour, true, true) => "ehour",
            (Language::En, DurationUnit::Hour, true, false) => "ehours",
            (Language::En, DurationUnit::Day, true, true) => "eday",
            (Language::En, DurationUnit::Day, true, false) => "edays",
            (Language::En, DurationUnit::Week, true, true) => "eweek",
            (Language::En, DurationUnit::Week, true, false) => "eweeks",
            (Language::En, DurationUnit::Month, true, true) => "emonth",
            (Language::En, DurationUnit::Month, true, false) => "emonths",
        }
    }

    #[allow(clippy::match_same_arms, reason = "une ligne par langue, lue comme une table")]
    fn link_code(&self, kind: LinkType) -> &str {
        match (self.0, kind) {
            (Language::Fr, LinkType::FinishToStart) => "FD",
            (Language::Fr, LinkType::StartToStart) => "DD",
            (Language::Fr, LinkType::FinishToFinish) => "FF",
            (Language::Fr, LinkType::StartToFinish) => "DF",
            (Language::En, LinkType::FinishToStart) => "FS",
            (Language::En, LinkType::StartToStart) => "SS",
            (Language::En, LinkType::FinishToFinish) => "FF",
            (Language::En, LinkType::StartToFinish) => "SF",
        }
    }

    fn unknown(&self) -> &'static str {
        "?"
    }

    fn percent_suffix(&self) -> &str {
        match self.0 {
            Language::Fr => "\u{a0}%",
            Language::En => "%",
        }
    }
}

impl SessionTexts<'_> {
    /// Contexte d'écriture des dates.
    fn ctx(&self) -> TextContext {
        TextContext::for_project(self.session.project(), self.lang)
    }

    /// Date sans heure (R-143, partie date).
    fn day(&self, m: Moment) -> String {
        let full = format_date(m, &self.ctx());
        full.split_once(' ').map_or(full.clone(), |(d, _)| d.to_owned())
    }
}

impl BarTextSource for SessionTexts<'_> {
    fn bar_text(&self, uid: TaskUid, f: BarField) -> Option<String> {
        let project = self.session.project();
        let task = project.tasks.get(uid)?;
        let labels = Labels(self.lang);
        let factors = &project.options.factors;
        let scheduled = match self.session.schedule().tasks.get(&uid) {
            Some(TaskOutcome::Scheduled(s)) => Some(s),
            _ => None,
        };
        let text = match f {
            BarField::Name => task.name.clone(),
            BarField::Resources => format_resource_names(
                project.assignments.iter().filter(|a| a.task == uid),
                project,
                &labels,
            ),
            BarField::Start => self.day(scheduled?.start),
            BarField::Finish => self.day(scheduled?.finish),
            BarField::StartFinish => {
                let s = scheduled?;
                format!("{} – {}", self.day(s.start), self.day(s.finish))
            }
            BarField::Duration => {
                let s = scheduled?;
                if s.role == TaskRole::Summary {
                    display_minutes_as(
                        s.duration,
                        DurationUnit::Day,
                        TimeBasis::Working,
                        factors,
                        &labels,
                    )
                } else {
                    display_duration(&task.duration, factors, &labels)
                }
            }
            BarField::Percent => display_percent(scheduled?.percent_complete, &labels),
            BarField::TotalSlack => display_minutes_as(
                scheduled?.total_slack,
                DurationUnit::Day,
                TimeBasis::Working,
                factors,
                &labels,
            ),
        };
        (!text.is_empty()).then_some(text)
    }

    fn token(&self, t: Token) -> String {
        let project = self.session.project();
        match t {
            Token::Project => {
                if project.info.name.is_empty() {
                    self.session.origin().name.clone()
                } else {
                    project.info.name.clone()
                }
            }
            Token::StatusDate => self.day(project.effective_status_date()),
            Token::PrintDate => self.day(self.session.now()),
            Token::File => {
                let origin = self.session.origin();
                origin
                    .path
                    .as_deref()
                    .and_then(|p| p.rsplit(['/', '\\']).next())
                    .filter(|n| !n.is_empty())
                    .map_or_else(|| origin.name.clone(), str::to_owned)
            }
            // Numéros de page : remplacés par la pagination (L7).
            Token::Page | Token::Pages => String::new(),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use super::*;
    use crate::session::DocOrigin;
    use planrs_core::calendar::BuiltinCalendarNames;
    use planrs_core::{Duration, Minutes, OutlineLevel, Project, ProjectInit, ScheduleAnchor, Task};

    fn session() -> Session {
        let start: Moment = "2026-10-19T08:00".parse().unwrap();
        let mut p = Project::new(ProjectInit {
            name: "P".to_owned(),
            anchor: ScheduleAnchor::FromStart(start),
            current_date: start,
            calendar_names: BuiltinCalendarNames::default(),
        });
        let options = p.options;
        let mut t = Task::new(TaskUid::new(1), "Murs".to_owned(), &options);
        t.outline_level = OutlineLevel::new(1).unwrap();
        t.duration =
            Duration::new(Minutes::new(4_800), DurationUnit::Day, TimeBasis::Working, false).unwrap();
        p.tasks.push(t).unwrap();
        Session::from_project(
            p,
            DocOrigin {
                name: "P".to_owned(),
                path: Some("/tmp/chantier/p.xml".to_owned()),
            },
            "2026-10-25T09:00".parse().unwrap(),
            Language::Fr,
        )
    }

    #[test]
    fn bar_texts_come_from_engine() {
        let s = session();
        let fr = SessionTexts {
            session: &s,
            lang: Language::Fr,
        };
        let uid = TaskUid::new(1);
        assert_eq!(fr.bar_text(uid, BarField::Name).as_deref(), Some("Murs"));
        assert_eq!(fr.bar_text(uid, BarField::Start).as_deref(), Some("19/10/2026"));
        assert_eq!(fr.bar_text(uid, BarField::Finish).as_deref(), Some("30/10/2026"));
        assert_eq!(
            fr.bar_text(uid, BarField::StartFinish).as_deref(),
            Some("19/10/2026 – 30/10/2026")
        );
        assert_eq!(fr.bar_text(uid, BarField::Duration).as_deref(), Some("10 jours"));
        assert_eq!(fr.bar_text(uid, BarField::Percent).as_deref(), Some("0\u{a0}%"));
        assert_eq!(fr.bar_text(uid, BarField::TotalSlack).as_deref(), Some("0 jour"));
        assert_eq!(fr.bar_text(uid, BarField::Resources), None);
        assert_eq!(fr.bar_text(TaskUid::new(9), BarField::Name), None);
        let en = SessionTexts {
            session: &s,
            lang: Language::En,
        };
        assert_eq!(en.bar_text(uid, BarField::Finish).as_deref(), Some("10/30/2026"));
        assert_eq!(en.bar_text(uid, BarField::Duration).as_deref(), Some("10 days"));
        assert_eq!(fr.token(Token::Project), "P");
        assert_eq!(fr.token(Token::File), "p.xml");
        assert_eq!(fr.token(Token::PrintDate), "25/10/2026");
        assert_eq!(fr.token(Token::StatusDate), "19/10/2026");
        assert_eq!(fr.token(Token::Page), "");
    }
}
