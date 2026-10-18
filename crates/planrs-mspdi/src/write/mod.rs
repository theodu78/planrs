//! Écrivain MSPDI (J3, ADR-0010), lots W1 à W3.
//!
//! W1 y pose les tables d'ordre des éléments (`order.rs`, R-088) ; W3
//! l'écriture événementielle (`write`, `write_losses`) et les exports
//! publics. Module privé jusqu'à W3 : les tables ne sont lues que par les
//! tests en attendant l'écrivain.

#[allow(
    dead_code,
    reason = "utilisé par W3 (écrivain) ; seuls les tests le lisent en W1"
)]
pub(crate) mod order;

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::print_stderr,
        clippy::arithmetic_side_effects,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use std::path::PathBuf;

    use quick_xml::Reader;
    use quick_xml::events::Event;

    use super::order::{
        CALENDAR_ORDER, EXCEPTION_ORDER, LINK_ORDER, PROJECT_ORDER, TASK_ORDER, TIMEPERIOD_ORDER,
        WEEKDAY_ORDER, WORKINGTIME_ORDER, WORKWEEK_ORDER, rank,
    };
    use crate::xml::{CALENDAR_FIELDS, EXCEPTION_FIELDS, LINK_FIELDS, PROJECT_FIELDS, TASK_FIELDS};

    /// Toutes les tables, nommées.
    const TABLES: &[(&str, &[&str])] = &[
        ("Project", PROJECT_ORDER),
        ("Task", TASK_ORDER),
        ("PredecessorLink", LINK_ORDER),
        ("Calendar", CALENDAR_ORDER),
        ("WeekDay", WEEKDAY_ORDER),
        ("WorkingTime", WORKINGTIME_ORDER),
        ("TimePeriod", TIMEPERIOD_ORDER),
        ("Exception", EXCEPTION_ORDER),
        ("WorkWeek", WORKWEEK_ORDER),
    ];

    #[test]
    fn r088_tables_have_no_duplicate() {
        for (entity, table) in TABLES {
            for (i, name) in table.iter().enumerate() {
                assert_eq!(rank(table, name), Some(i), "{entity} : {name} en double");
            }
        }
    }

    /// Toute balise que le lecteur retient (listes blanches de `xml.rs`) a un
    /// rang dans la table de son entité : l'écrivain saura où la placer.
    #[test]
    fn r088_reader_whitelists_are_ranked() {
        let lists: &[(&[&str], &[&str])] = &[
            (PROJECT_FIELDS, PROJECT_ORDER),
            (TASK_FIELDS, TASK_ORDER),
            (&["PredecessorLink"], TASK_ORDER),
            (LINK_FIELDS, LINK_ORDER),
            (CALENDAR_FIELDS, CALENDAR_ORDER),
            (&["WeekDays", "Exceptions", "WorkWeeks"], CALENDAR_ORDER),
            (
                &["DayType", "DayWorking", "TimePeriod", "WorkingTimes"],
                WEEKDAY_ORDER,
            ),
            (&["FromTime", "ToTime"], WORKINGTIME_ORDER),
            (&["FromDate", "ToDate"], TIMEPERIOD_ORDER),
            (EXCEPTION_FIELDS, EXCEPTION_ORDER),
            (&["TimePeriod", "WorkingTimes"], EXCEPTION_ORDER),
            (&["TimePeriod", "Name", "WeekDays"], WORKWEEK_ORDER),
        ];
        for (fields, table) in lists {
            for f in *fields {
                assert!(rank(table, f).is_some(), "{f} sans rang");
            }
        }
    }

    /// Table de l'entité désignée par le chemin d'éléments `path` (noms
    /// locaux depuis la racine), ou `None` si l'élément n'est pas une entité
    /// ordonnée.
    fn table_for(path: &[String]) -> Option<(&'static str, &'static [&'static str])> {
        let names: Vec<&str> = path.iter().map(String::as_str).collect();
        let entity = match names.as_slice() {
            ["Project"] => "Project",
            ["Project", "Tasks", "Task"] => "Task",
            ["Project", "Tasks", "Task", "PredecessorLink"] => "PredecessorLink",
            ["Project", "Calendars", "Calendar"] => "Calendar",
            ["Project", "Calendars", "Calendar", .., "WeekDays", "WeekDay"] => "WeekDay",
            [
                "Project",
                "Calendars",
                "Calendar",
                ..,
                "WorkingTimes",
                "WorkingTime",
            ] => "WorkingTime",
            ["Project", "Calendars", "Calendar", .., "TimePeriod"] => "TimePeriod",
            ["Project", "Calendars", "Calendar", "Exceptions", "Exception"] => "Exception",
            ["Project", "Calendars", "Calendar", "WorkWeeks", "WorkWeek"] => "WorkWeek",
            _ => return None,
        };
        TABLES.iter().find(|(e, _)| *e == entity).copied()
    }

    /// Vérifie que les enfants directs `children` d'une entité forment une
    /// sous-suite de sa table (rangs croissants au sens large : une balise
    /// répétée, comme `PredecessorLink`, reste à sa place). Rend le nombre
    /// d'entités vérifiées (0 ou 1), ou l'écart trouvé.
    fn check_children(file: &str, path: &[String], children: &[String]) -> Result<usize, String> {
        let Some((entity, table)) = table_for(path) else {
            return Ok(0);
        };
        let mut previous = 0_usize;
        for child in children {
            let r = rank(table, child)
                .ok_or_else(|| format!("{file} : {entity} : balise {child} absente de la table"))?;
            if r < previous {
                return Err(format!(
                    "{file} : {entity} : {child} (rang {r}) après une balise de rang {previous}"
                ));
            }
            previous = r;
        }
        Ok(1)
    }

    /// Lit `xml` avec quick-xml et vérifie chaque entité ordonnée ; rend le
    /// nombre d'entités vérifiées par nom d'entité, ou le premier écart.
    fn check_document(file: &str, xml: &str) -> Result<Vec<(&'static str, usize)>, String> {
        let mut reader = Reader::from_str(xml);
        // Pile : (nom local, noms des enfants directs).
        let mut stack: Vec<(String, Vec<String>)> = Vec::new();
        let mut counts: Vec<(&'static str, usize)> = TABLES.iter().map(|(e, _)| (*e, 0)).collect();
        let mut bump = |path: &[String], n: usize| {
            if n == 0 {
                return;
            }
            let (entity, _) = table_for(path).unwrap();
            if let Some(c) = counts.iter_mut().find(|(e, _)| *e == entity) {
                c.1 += n;
            }
        };
        loop {
            match reader.read_event().unwrap() {
                Event::Start(e) => {
                    let name: String = AsRef::<str>::as_ref(&e.local_name()).to_owned();
                    if let Some((_, children)) = stack.last_mut() {
                        children.push(name.clone());
                    }
                    stack.push((name, Vec::new()));
                }
                Event::Empty(e) => {
                    let name: String = AsRef::<str>::as_ref(&e.local_name()).to_owned();
                    if let Some((_, children)) = stack.last_mut() {
                        children.push(name.clone());
                    }
                    let mut path: Vec<String> = stack.iter().map(|(n, _)| n.clone()).collect();
                    path.push(name);
                    let n = check_children(file, &path, &[])?;
                    bump(&path, n);
                }
                Event::End(_) => {
                    let path: Vec<String> = stack.iter().map(|(n, _)| n.clone()).collect();
                    let (_, children) = stack.pop().unwrap();
                    let n = check_children(file, &path, &children)?;
                    bump(&path, n);
                }
                Event::Eof => break,
                _ => {}
            }
        }
        Ok(counts)
    }

    /// R-088, corpus : pour chacun des cinq fichiers enregistrés par
    /// MS Project, la suite des enfants directs de `Project`, de chaque
    /// `Task`, `PredecessorLink`, `Calendar`, `WeekDay`, `WorkingTime`,
    /// `TimePeriod`, `Exception` et `WorkWeek` est une sous-suite de la table
    /// de son entité (comportement observé, règle 1). Fichier absent :
    /// message, pas d'échec (`cargo xtask corpus fetch` les télécharge).
    #[test]
    fn r088_corpus_children_follow_order_tables() {
        let dir: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "tests", "corpus", "files"]
            .iter()
            .collect();
        let files = [
            "cogito-advanced-sample-msp2010.xml",
            "practicemap.xml",
            "projinteg.xml",
            "shopmetrics.xml",
            "takeout-system-zh.xml",
        ];
        let mut total: Vec<(&'static str, usize)> = TABLES.iter().map(|(e, _)| (*e, 0)).collect();
        let mut seen = 0;
        for file in files {
            let Ok(xml) = std::fs::read_to_string(dir.join(file)) else {
                eprintln!("R-088 : {file} absent (cargo xtask corpus fetch), ignoré");
                continue;
            };
            seen += 1;
            let counts = check_document(file, &xml).unwrap_or_else(|e| panic!("R-088 : {e}"));
            for (entity, n) in counts {
                if let Some(t) = total.iter_mut().find(|(e, _)| *e == entity) {
                    t.1 += n;
                }
            }
        }
        if seen == 0 {
            return;
        }
        eprintln!("R-088 : {seen} fichiers, entités vérifiées : {total:?}");
        if seen == files.len() {
            // Comptes observés sur les cinq fichiers (sonde de la conception B) :
            // un projet par fichier, 174 tâches, 127 liens, 25 calendriers,
            // 4 exceptions ; aucune semaine datée.
            let count = |entity: &str| total.iter().find(|(e, _)| *e == entity).map_or(0, |(_, n)| *n);
            assert_eq!(count("Project"), 5);
            assert_eq!(count("Task"), 174);
            assert_eq!(count("PredecessorLink"), 127);
            assert_eq!(count("Calendar"), 25);
            assert_eq!(count("Exception"), 4);
            assert_eq!(count("WorkWeek"), 0);
        }
    }

    /// Le test de sous-suite refuse bien une inversion et une balise
    /// inconnue (le contrôle n'est pas vide).
    #[test]
    fn r088_subsequence_check_rejects_disorder() {
        let ok = "<Project><Name>a</Name><Tasks><Task><UID>1</UID><Name>t</Name>\
                  <PredecessorLink><PredecessorUID>2</PredecessorUID></PredecessorLink>\
                  <PredecessorLink><PredecessorUID>3</PredecessorUID></PredecessorLink>\
                  </Task></Tasks></Project>";
        let counts = check_document("ok", ok).unwrap();
        assert!(counts.contains(&("Task", 1)));
        assert!(counts.contains(&("PredecessorLink", 2)));
        let swapped = "<Project><Tasks><Task><Name>t</Name><UID>1</UID></Task></Tasks></Project>";
        assert_eq!(
            check_document("swapped", swapped),
            Err("swapped : Task : UID (rang 0) après une balise de rang 3".to_owned())
        );
        let unknown = "<Project><Tasks><Task><UID>1</UID><FutureX/></Task></Tasks></Project>";
        assert_eq!(
            check_document("unknown", unknown),
            Err("unknown : Task : balise FutureX absente de la table".to_owned())
        );
    }
}
