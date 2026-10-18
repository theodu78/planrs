//! P19 « pas de fuite » (ADR-0004, ADR-0009 § 7.8) : les valeurs calculées
//! par MS Project ne doivent pas entrer dans le `Project`. Retirer du texte
//! XML les `Start`/`Finish` des tâches automatiques, `Early*`, `Late*`,
//! `*Slack`, `Critical`, `Summary` et la `Duration` des récapitulatives ne
//! change pas le projet lu.
//!
//! L'égalité porte sur le `Project` entier, entrée unique du moteur, hors
//! `Extras`, et sur `schedule(&doc.project)`. Les `Extras` (charges opaques,
//! R-128) sont comparées par clés : le retrait textuel, naïf, touche aussi
//! les `Start`/`Finish`/`Duration` imbriqués dans une charge (`Baseline`),
//! dont le contenu change donc légitimement ; `StartSlack` et `FinishSlack`,
//! valeurs dérivées que l'écrivain ne régénère pas, sont capturées par
//! construction (ADR-0010 amendé) et retirées des clés comparées.
//!
//! P19 reformulée (J3, juges 1 et 2) : aucune clé d'`Extras` d'origine
//! `mspdi` n'appartient à l'ensemble W des balises que l'écrivain régénère
//! pour son entité (`planrs_mspdi::written`), sinon émission double ; seule
//! exception, le `PredecessorLink` abandonné d'une tâche.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

use std::path::{Path, PathBuf};

use planrs_core::{Extras, Project, schedule};
use planrs_mspdi::written::{CALENDAR_WRITTEN, PROJECT_WRITTEN, TASK_CAPTURED_WRITTEN, TASK_WRITTEN};
use planrs_mspdi::{EXTRAS_ORIGIN, ReadOptions, read};

fn opts() -> ReadOptions {
    ReadOptions::new("2026-01-01T00:00".parse().unwrap())
}

fn repo(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(relative)
}

fn xml_files(dir: &Path) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("xml"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect();
    out.sort();
    out
}

/// Valeur du premier élément `<name>…</name>` d'un fragment.
fn first(fragment: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = fragment.find(&open)? + open.len();
    let end = fragment[start..].find(&close)? + start;
    Some(fragment[start..end].trim().to_owned())
}

/// Retire toutes les occurrences de `<name>…</name>` (et `<name/>`).
fn strip(fragment: &str, name: &str) -> String {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let mut out = String::with_capacity(fragment.len());
    let mut rest = fragment;
    while let Some(i) = rest.find(&open) {
        out.push_str(&rest[..i]);
        let after = &rest[i..];
        let Some(j) = after.find(&close) else {
            rest = after;
            break;
        };
        rest = &after[j + close.len()..];
    }
    out.push_str(rest);
    out.replace(&format!("<{name}/>"), "")
}

/// Le document sans les valeurs calculées. Récapitulative : déduite de la
/// hiérarchie (R-052), pas du drapeau `Summary`.
fn without_calculated(xml: &str) -> String {
    let (Some(a), Some(b)) = (xml.find("<Tasks>"), xml.find("</Tasks>")) else {
        return xml.to_owned();
    };
    let (head, body, tail) = (&xml[..a], &xml[a..b], &xml[b..]);
    let parts: Vec<&str> = body.split("<Task>").collect();
    // Niveau des tâches non vides, dans l'ordre (premier morceau : `<Tasks>`).
    let levels: Vec<Option<u32>> = parts[1..]
        .iter()
        .map(|p| {
            let null = first(p, "IsNull").is_some_and(|v| v == "1" || v == "true");
            let level = first(p, "OutlineLevel").and_then(|v| v.parse().ok()).unwrap_or(1);
            (!null).then_some(level)
        })
        .collect();
    let mut summary = vec![false; levels.len()];
    let mut next: Option<u32> = None;
    for (i, level) in levels.iter().enumerate().rev() {
        if let Some(l) = level {
            summary[i] = next.is_some_and(|n| n > *l) || *l == 0;
            next = Some(*l);
        }
    }
    let mut out = String::from(head);
    out.push_str(parts[0]);
    for (i, part) in parts[1..].iter().enumerate() {
        let manual = first(part, "Manual").is_some_and(|v| v == "1" || v == "true");
        let mut p = (*part).to_owned();
        for name in [
            "EarlyStart",
            "EarlyFinish",
            "LateStart",
            "LateFinish",
            "TotalSlack",
            "FreeSlack",
            "StartSlack",
            "FinishSlack",
            "Critical",
            "Summary",
        ] {
            p = strip(&p, name);
        }
        if !manual {
            p = strip(&strip(&p, "Start"), "Finish");
            if summary[i] {
                p = strip(&p, "Duration");
            }
        }
        out.push_str("<Task>");
        out.push_str(&p);
    }
    out.push_str(tail);
    out
}

/// Balises retirées par `without_calculated` mais capturées (non régénérées).
const STRIPPED_CAPTURED: &[&str] = &["StartSlack", "FinishSlack"];

/// Toutes les `Extras` du projet, par entité : (entité, extras).
fn all_extras(p: &Project) -> Vec<(String, &Extras)> {
    let mut out = vec![("projet".to_owned(), &p.extras)];
    out.extend(
        p.calendars
            .iter()
            .map(|c| (format!("calendrier {}", c.uid().get()), &c.extras)),
    );
    out.extend(p.summary_task.iter().map(|t| ("tâche 0".to_owned(), &t.extras)));
    out.extend(
        p.tasks
            .iter()
            .map(|t| (format!("tâche {}", t.uid().get()), &t.extras)),
    );
    out
}

/// Le projet sans ses `Extras`.
fn without_extras(p: &Project) -> Project {
    let mut p = p.clone();
    p.extras = Extras::new();
    let uids: Vec<_> = p.tasks.iter().map(planrs_core::Task::uid).collect();
    for uid in uids {
        p.tasks.get_mut(uid).unwrap().extras = Extras::new();
    }
    if let Some(t) = &mut p.summary_task {
        t.extras = Extras::new();
    }
    let uids: Vec<_> = p
        .calendars
        .iter()
        .map(planrs_core::calendar::Calendar::uid)
        .collect();
    for uid in uids {
        p.calendars.get_mut(uid).unwrap().extras = Extras::new();
    }
    p
}

/// Clés des `Extras` par entité, hors balises retirées et capturées.
fn extras_keys(p: &Project) -> Vec<(String, Vec<String>)> {
    all_extras(p)
        .into_iter()
        .map(|(who, e)| {
            let keys =
                e.0.iter()
                    .filter(|x| !STRIPPED_CAPTURED.contains(&x.key.as_str()))
                    .map(|x| x.key.clone())
                    .collect();
            (who, keys)
        })
        .collect()
}

fn check(name: &str, xml: &str) {
    let full = read(xml.as_bytes(), &opts()).unwrap_or_else(|e| panic!("{name} : {e}"));
    let stripped_xml = without_calculated(xml);
    assert_ne!(stripped_xml.len(), xml.len(), "{name} : rien n'a été retiré");
    let stripped = read(stripped_xml.as_bytes(), &opts()).unwrap_or_else(|e| panic!("{name} : {e}"));
    assert_eq!(
        without_extras(&full.project),
        without_extras(&stripped.project),
        "{name} : une valeur calculée a fuité"
    );
    assert_eq!(
        extras_keys(&full.project),
        extras_keys(&stripped.project),
        "{name} : le retrait des valeurs calculées a changé les captures"
    );
    assert_eq!(
        schedule(&full.project),
        schedule(&stripped.project),
        "{name} : le planning dépend d'une valeur calculée"
    );
}

#[test]
fn p19_fixtures_no_leak() {
    for (name, xml) in xml_files(&repo("tests/fixtures/mspdi")) {
        if xml.contains("<Task>") {
            check(&name, &xml);
        }
    }
}

#[test]
fn p19_corpus_no_leak() {
    for (name, xml) in xml_files(&repo("tests/corpus/files")) {
        check(&name, &xml);
    }
}

/// P19 reformulée : aucune clé d'`Extras` d'origine `mspdi` n'appartient à
/// W de son entité (sauf le `PredecessorLink` abandonné d'une tâche).
fn check_keys_not_written(name: &str, xml: &str) {
    let doc = read(xml.as_bytes(), &opts()).unwrap_or_else(|e| panic!("{name} : {e}"));
    let p = &doc.project;
    let mut owners: Vec<(String, &Extras, &[&str], &[&str])> =
        vec![("projet".to_owned(), &p.extras, PROJECT_WRITTEN, &[])];
    for c in &p.calendars {
        owners.push((
            format!("calendrier {}", c.uid().get()),
            &c.extras,
            CALENDAR_WRITTEN,
            &[],
        ));
    }
    for t in p.summary_task.iter().chain(p.tasks.iter()) {
        owners.push((
            format!("tâche {}", t.uid().get()),
            &t.extras,
            TASK_WRITTEN,
            TASK_CAPTURED_WRITTEN,
        ));
    }
    for (who, extras, written, allowed) in owners {
        for e in &extras.0 {
            assert_eq!(e.origin, EXTRAS_ORIGIN, "{name}, {who}");
            assert!(
                !written.contains(&e.key.as_str()) || allowed.contains(&e.key.as_str()),
                "{name}, {who} : clé régénérée capturée : {}",
                e.key
            );
            assert!(
                e.payload.starts_with(&format!("<{}", e.key)) && e.payload.ends_with('>'),
                "{name}, {who} : charge mal découpée : {}",
                e.payload
            );
        }
    }
}

#[test]
fn p19_extras_keys_are_not_written_tags() {
    let mut files = xml_files(&repo("tests/fixtures/mspdi"));
    files.extend(xml_files(&repo("tests/corpus/files")));
    files.extend(
        xml_files(&repo("crates/planrs-ui-egui/assets/demo"))
            .into_iter()
            .filter(|(n, _)| n == "house.xml"),
    );
    for (name, xml) in files {
        check_keys_not_written(&name, &xml);
    }
}

#[test]
fn stripping_helpers() {
    assert_eq!(strip("<a><B>1</B><C>2</C><B>3</B></a>", "B"), "<a><C>2</C></a>");
    assert_eq!(strip("<a><B/></a>", "B"), "<a></a>");
    assert_eq!(first("<x><B> 7 </B></x>", "B").as_deref(), Some("7"));
    let xml = "<Project><Tasks><Task><UID>1</UID><OutlineLevel>1</OutlineLevel><Duration>D</Duration>\
               <Start>s</Start></Task><Task><UID>2</UID><OutlineLevel>2</OutlineLevel><Duration>E</Duration>\
               </Task></Tasks></Project>";
    let out = without_calculated(xml);
    assert!(!out.contains("<Duration>D</Duration>") && out.contains("<Duration>E</Duration>"));
    assert!(!out.contains("<Start>"));
}
