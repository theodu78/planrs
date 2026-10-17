//! Performance informative de la lecture (ADR-0009 § 7.8) : un MSPDI généré
//! de 5 000 tâches et 10 000 liens, cible < 100 ms en `--release`. Aucun
//! seuil en CI en J2 : le test est ignoré par défaut et affiche la médiane.
//!
//! `cargo test -p planrs-mspdi --release --test perf -- --ignored --nocapture`

#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    reason = "test de mesure : les interdits de la règle 10 ne s'appliquent pas"
)]

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use planrs_mspdi::{ReadOptions, read};

/// Document MSPDI de `n` tâches (une récapitulative toutes les 10), chaque
/// tâche liée aux deux précédentes, à la façon d'un fichier de MS Project
/// (valeurs calculées comprises).
fn generated(n: u32) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <Project xmlns=\"http://schemas.microsoft.com/project\"><SaveVersion>14</SaveVersion>\
         <StartDate>2026-10-19T08:00:00</StartDate><CalendarUID>1</CalendarUID><Calendars><Calendar>\
         <UID>1</UID><Name>Standard</Name><IsBaseCalendar>1</IsBaseCalendar><WeekDays>\
         <WeekDay><DayType>1</DayType><DayWorking>0</DayWorking></WeekDay>",
    );
    for d in 2..=6 {
        let _ = write!(
            xml,
            "<WeekDay><DayType>{d}</DayType><DayWorking>1</DayWorking><WorkingTimes>\
             <WorkingTime><FromTime>08:00:00</FromTime><ToTime>12:00:00</ToTime></WorkingTime>\
             <WorkingTime><FromTime>13:00:00</FromTime><ToTime>17:00:00</ToTime></WorkingTime>\
             </WorkingTimes></WeekDay>"
        );
    }
    xml.push_str("<WeekDay><DayType>7</DayType><DayWorking>0</DayWorking></WeekDay></WeekDays></Calendar></Calendars><Tasks>");
    for uid in 1..=n {
        let (level, summary) = if uid % 10 == 1 { (1, 1) } else { (2, 0) };
        let _ = write!(
            xml,
            "<Task><UID>{uid}</UID><ID>{uid}</ID><Name>Tâche {uid}</Name><Active>1</Active><Manual>0</Manual>\
             <Type>1</Type><IsNull>0</IsNull><OutlineLevel>{level}</OutlineLevel><Priority>500</Priority>\
             <Start>2026-10-19T08:00:00</Start><Finish>2026-10-20T17:00:00</Finish><Duration>PT16H0M0S</Duration>\
             <DurationFormat>7</DurationFormat><Estimated>0</Estimated><Milestone>0</Milestone><Summary>{summary}</Summary>\
             <Critical>0</Critical><EarlyStart>2026-10-19T08:00:00</EarlyStart><EarlyFinish>2026-10-20T17:00:00</EarlyFinish>\
             <LateStart>2026-10-19T08:00:00</LateStart><LateFinish>2026-10-20T17:00:00</LateFinish>\
             <FreeSlack>0</FreeSlack><TotalSlack>0</TotalSlack><ConstraintType>0</ConstraintType>\
             <CalendarUID>-1</CalendarUID><PercentComplete>0</PercentComplete>"
        );
        for back in [1, 2] {
            if uid > back + 1 {
                let _ = write!(
                    xml,
                    "<PredecessorLink><PredecessorUID>{}</PredecessorUID><Type>1</Type><CrossProject>0</CrossProject>\
                     <LinkLag>0</LinkLag><LagFormat>7</LagFormat></PredecessorLink>",
                    uid - back
                );
            }
        }
        xml.push_str("</Task>");
    }
    xml.push_str("</Tasks></Project>");
    xml
}

#[test]
#[ignore = "mesure informative, à lancer en --release"]
fn read_5000_tasks() {
    let xml = generated(5_000);
    let opts = ReadOptions::new("2026-01-01T00:00".parse().unwrap());
    let doc = read(xml.as_bytes(), &opts).unwrap();
    assert_eq!(doc.project.tasks.len(), 5_000);
    assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics.first());
    let mut times: Vec<Duration> = (0..15)
        .map(|_| {
            let t = Instant::now();
            let _ = read(xml.as_bytes(), &opts).unwrap();
            t.elapsed()
        })
        .collect();
    times.sort();
    println!(
        "lecture MSPDI, 5 000 tâches, {} Kio : médiane {:?} (cible < 100 ms en release)",
        xml.len() / 1024,
        times.get(7)
    );
}
