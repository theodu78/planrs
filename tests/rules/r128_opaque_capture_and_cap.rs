//! R-128 — Capture opaque à la lecture et plafond.
//!
//! Lot W2 (J3, ADR-0010 amendé). Tout enfant direct de `Project`,
//! `Tasks/Task` ou `Calendars/Calendar` que l'écrivain ne régénère pas
//! (ensembles W de `planrs_mspdi::written`) est conservé en `Extras`
//! (origine `mspdi`, clé = nom qualifié, charge = octets bruts de l'élément,
//! balise englobante comprise), dans l'ordre du document ; un lien abandonné
//! pour une cause non fautive est conservé entier sur la tâche successeur ;
//! plafond de 64 Mio et d'éléments retenus, abandon déterministe.

use planrs_core::{Subject, Task, TaskUid};
use planrs_mspdi::{
    ImportCode, Limits, MAX_OPAQUE_LOSSES, MspdiDocument, OpaqueLossReason, ReadOptions, read,
};
use planrs_tests::{moment, tests_path};

fn opts() -> ReadOptions {
    ReadOptions::new(moment("2026-01-01T00:00"))
}

fn fixture() -> MspdiDocument {
    let bytes = std::fs::read(tests_path("fixtures/mspdi/roundtrip.xml")).unwrap();
    read(&bytes, &opts()).unwrap()
}

fn keys(extras: &planrs_core::Extras) -> Vec<&str> {
    extras.0.iter().map(|e| e.key.as_str()).collect()
}

fn task(doc: &MspdiDocument, uid: u32) -> &Task {
    doc.project.tasks.get(TaskUid::new(uid)).unwrap()
}

fn payload<'d>(extras: &'d planrs_core::Extras, key: &str) -> &'d str {
    extras.0.iter().find(|e| e.key == key).unwrap().payload.as_str()
}

#[test]
fn r128_project_children_not_written_are_captured_in_document_order() {
    // Champs de projet non régénérés (BuildNumber, LastSaved, FYStartDate,
    // CurrencyDigits, CurrencySymbol), élément inconnu, section d'interface,
    // définitions de codes et d'attributs, Resources et Assignments entiers.
    let doc = fixture();
    let p = &doc.project;
    assert_eq!(
        keys(&p.extras),
        [
            "BuildNumber",
            "LastSaved",
            "FYStartDate",
            "CurrencyDigits",
            "CurrencySymbol",
            "FutureElement",
            "Views",
            "OutlineCodes",
            "WBSMasks",
            "ExtendedAttributes",
            "Resources",
            "Assignments",
        ]
    );
    assert!(p.extras.0.iter().all(|e| e.origin == "mspdi"));
    // Octets bruts : attributs, échappements et indentation intacts.
    assert_eq!(
        payload(&p.extras, "FutureElement"),
        "<FutureElement version=\"2\"><Inner>valeur &lt;future&gt;</Inner></FutureElement>"
    );
    assert_eq!(
        payload(&p.extras, "CurrencySymbol"),
        "<CurrencySymbol>€</CurrencySymbol>"
    );
    let assignments = payload(&p.extras, "Assignments");
    assert!(assignments.starts_with("<Assignments>\n\t\t<Assignment>"));
    assert!(assignments.ends_with("</Assignment>\n\t</Assignments>"));
    assert_eq!(assignments.matches("<TimephasedData>").count(), 2);
    let resources = payload(&p.extras, "Resources");
    assert_eq!(resources.matches("<Resource>").count(), 3);
    assert_eq!(resources.matches("<Baseline>").count(), 2);
    // `Assignments` reste aussi parcouru : affectations comptées (R-083).
    assert_eq!(doc.recorded.tasks[&TaskUid::new(2)].assignments, 1);
    assert_eq!(doc.recorded.tasks[&TaskUid::new(3)].assignments, 1);
    // Les champs régénérés ne sont pas capturés et sont lus.
    assert_eq!(p.info.title, "Aller-retour & préservation");
}

#[test]
fn r128_task_children_not_written_are_captured() {
    let doc = fixture();
    // UID 0 : coût et ligne de base.
    let summary = doc.project.summary_task.as_ref().unwrap();
    assert_eq!(keys(&summary.extras), ["Cost", "Baseline"]);
    // UID 1 : hyperliens ; `Notes` et `WBS` sont régénérés, donc lus.
    let t1 = task(&doc, 1);
    assert_eq!(keys(&t1.extras), ["Hyperlink", "HyperlinkAddress"]);
    assert_eq!(
        payload(&t1.extras, "HyperlinkAddress"),
        "<HyperlinkAddress>https://example.org/spec?a=1&amp;b=2</HyperlinkAddress>"
    );
    assert_eq!(t1.notes, "Note <&> ligne 1\r\nligne 2 🚧 中文");
    assert_eq!(t1.wbs.as_deref(), Some("1"));
    // UID 2 : `LevelingDelay` lu (valeur enregistrée) et capturé ; élément
    // vide gardé sous sa forme `<Hyperlink/>` ; élément préfixé gardé sous
    // son nom qualifié ; commentaire entre deux champs perdu, commentaire
    // interne à une charge conservé ; deux lignes de base dans l'ordre.
    let t2 = task(&doc, 2);
    assert_eq!(
        keys(&t2.extras),
        [
            "Work",
            "StartSlack",
            "FinishSlack",
            "Cost",
            "Contact",
            "LevelingDelay",
            "LevelingDelayFormat",
            "Hyperlink",
            "x:Foo",
            "ExtendedAttribute",
            "OutlineCode",
            "Baseline",
            "Baseline",
        ]
    );
    assert_eq!(
        doc.recorded.tasks[&TaskUid::new(2)].leveling_delay,
        Some(planrs_mspdi::Tenths(4_800))
    );
    assert_eq!(
        payload(&t2.extras, "LevelingDelay"),
        "<LevelingDelay>4800</LevelingDelay>"
    );
    assert_eq!(payload(&t2.extras, "Hyperlink"), "<Hyperlink/>");
    assert_eq!(
        payload(&t2.extras, "x:Foo"),
        "<x:Foo xmlns:x=\"urn:t\" a=\"1\">texte <![CDATA[<brut> & non échappé]]></x:Foo>"
    );
    let baselines: Vec<&str> = t2
        .extras
        .0
        .iter()
        .filter(|e| e.key == "Baseline")
        .map(|e| e.payload.as_str())
        .collect();
    assert!(baselines[0].contains("<Number>0</Number>"));
    assert!(baselines[0].contains("<!-- commentaire interne : conservé dans la charge -->"));
    assert!(baselines[1].contains("<Number>1</Number>"));
    assert!(
        t2.extras
            .0
            .iter()
            .all(|e| !e.payload.contains("entre deux champs"))
    );
    // UID 4 : aucune charge.
    assert!(task(&doc, 4).extras.is_empty());
}

#[test]
fn r128_abandoned_links_captured_whole_on_successor() {
    // UID 3 : lien gardé 2 → 3 ; lien vers un autre projet (CrossProject 1)
    // et lien de type inconnu (9) abandonnés et conservés entiers, dans
    // l'ordre du document, avant `TimephasedData`.
    let doc = fixture();
    let t3 = task(&doc, 3);
    assert_eq!(t3.predecessors.len(), 1);
    assert_eq!(t3.predecessors[0].predecessor, TaskUid::new(2));
    assert_eq!(
        keys(&t3.extras),
        ["PredecessorLink", "PredecessorLink", "TimephasedData"]
    );
    let cross = &t3.extras.0[0].payload;
    assert!(cross.starts_with("<PredecessorLink>\n\t\t\t\t<PredecessorUID>7</PredecessorUID>"));
    assert!(cross.contains("<CrossProjectName>C:\\Projets\\Externe.mpp\\7</CrossProjectName>"));
    assert!(cross.ends_with("</PredecessorLink>"));
    assert!(t3.extras.0[1].payload.contains("<Type>9</Type>"));
}

#[test]
fn r128_unknown_children_of_kept_link_are_counted_and_reported() {
    // `FutureLinkField` d'un lien gardé : `Link` n'a pas d'`Extras`, l'enfant
    // est perdu, compté et signalé une fois par fichier (valeur = nombre).
    let doc = fixture();
    let d: Vec<_> = doc
        .diagnostics
        .iter()
        .filter(|d| {
            d.code
                == ImportCode::Unsupported {
                    what: "predecessor_link_child".to_owned(),
                }
        })
        .collect();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].value.as_deref(), Some("1"));
    assert_eq!(d[0].element, "Tasks/Task/PredecessorLink");
}

#[test]
fn r128_calendar_children_not_written_are_captured() {
    // `IsBaselineCalendar` et un élément inconnu ; les deux formes
    // d'exception sont lues (R-081 : `Exceptions` fait foi), pas capturées.
    let doc = fixture();
    let cal = doc.project.calendars.iter().next().unwrap();
    assert_eq!(keys(&cal.extras), ["IsBaselineCalendar", "FutureCalendarField"]);
    assert_eq!(cal.exceptions().len(), 1);
    assert_eq!(cal.exceptions()[0].name, "Pont");
}

#[test]
fn r128_source_info_counts_kept_bytes() {
    let doc = fixture();
    let p = &doc.project;
    let mut all: Vec<&planrs_core::Extras> = vec![&p.extras];
    all.extend(p.calendars.iter().map(|c| &c.extras));
    all.extend(p.summary_task.iter().map(|t| &t.extras));
    all.extend(p.tasks.iter().map(|t| &t.extras));
    let bytes: usize = all.iter().flat_map(|e| e.0.iter()).map(|e| e.payload.len()).sum();
    assert_eq!(doc.source.opaque_bytes, u64::try_from(bytes).unwrap());
    assert!(doc.source.opaque_dropped.is_empty());
    assert_eq!(doc.source.opaque_dropped_total, 0);
}

#[test]
fn r128_prefixed_root_captures_nothing() {
    // `<p:Project>` : les charges porteraient un préfixe non déclaré une fois
    // réémises ; rien n'est capturé, `mspdi.unsupported/prefixed_namespace`.
    let xml = r#"<p:Project xmlns:p="urn:x"><p:StartDate>2026-10-19T08:00:00</p:StartDate><p:Foo>1</p:Foo><p:Tasks><p:Task><p:UID>1</p:UID><p:Baseline/></p:Task></p:Tasks></p:Project>"#;
    let doc = read(xml.as_bytes(), &opts()).unwrap();
    assert!(doc.project.extras.is_empty());
    assert!(task(&doc, 1).extras.is_empty());
    assert!(doc.diagnostics.iter().any(|d| d.code
        == ImportCode::Unsupported {
            what: "prefixed_namespace".to_owned()
        }));
}

#[test]
fn r128_unresolved_entity_drops_the_payload() {
    // `&nbsp;` non déclaré : réémise telle quelle, la charge rendrait le
    // document mal formé ; perte listée (16 octets : `<Foo>&nbsp;</Foo>`),
    // l'entité est signalée par `mspdi.inconsistent`.
    let xml = "<Project><StartDate>2026-10-19T08:00:00</StartDate><Tasks><Task><UID>1</UID>\
               <Foo>&nbsp;</Foo><Bar>&lt;&#233;</Bar></Task></Tasks></Project>";
    let doc = read(xml.as_bytes(), &opts()).unwrap();
    let t = task(&doc, 1);
    assert_eq!(keys(&t.extras), ["Bar"]);
    assert_eq!(t.extras.0[0].payload, "<Bar>&lt;&#233;</Bar>");
    let lost = &doc.source.opaque_dropped;
    assert_eq!(lost.len(), 1);
    assert_eq!(
        (
            lost[0].subject,
            lost[0].key.as_str(),
            lost[0].bytes,
            lost[0].reason
        ),
        (
            Some(Subject::Task(TaskUid::new(1))),
            "Foo",
            17,
            OpaqueLossReason::UnresolvedEntity
        )
    );
    assert_eq!(doc.source.opaque_dropped_total, 1);
    assert!(doc.diagnostics.iter().any(|d| d.code == ImportCode::Inconsistent));
    assert!(!doc.diagnostics.iter().any(|d| d.code.code() == "mspdi.truncated"));
}

#[test]
fn r128_byte_cap_drops_every_later_capture() {
    // Plafond de 40 octets : `<A>0123456789</A>` (17) et `<B>…</B>` (17)
    // gardés (34) ; `<C>…</C>` (17) dépasse : abandonné, ainsi que toute
    // capture suivante, même `<D/>` (4 octets) qui tiendrait. Un seul
    // diagnostic `mspdi.truncated` (2 charges, 21 octets).
    let limits = Limits {
        max_opaque_bytes: 40,
        ..Limits::default()
    };
    let xml = "<Project><StartDate>2026-10-19T08:00:00</StartDate><A>0123456789</A><B>0123456789</B>\
               <C>0123456789</C><Tasks><Task><UID>1</UID><D/></Task></Tasks></Project>";
    let doc = read(xml.as_bytes(), &ReadOptions { limits, ..opts() }).unwrap();
    assert_eq!(keys(&doc.project.extras), ["A", "B"]);
    assert!(task(&doc, 1).extras.is_empty());
    assert_eq!(doc.source.opaque_bytes, 34);
    let lost: Vec<_> = doc
        .source
        .opaque_dropped
        .iter()
        .map(|l| (l.subject, l.key.as_str(), l.bytes, l.reason))
        .collect();
    assert_eq!(
        lost,
        [
            (Some(Subject::Project), "C", 17, OpaqueLossReason::Cap),
            (
                Some(Subject::Task(TaskUid::new(1))),
                "D",
                4,
                OpaqueLossReason::Cap
            ),
        ]
    );
    let truncated: Vec<_> = doc
        .diagnostics
        .iter()
        .filter(|d| d.code.code() == "mspdi.truncated")
        .collect();
    assert_eq!(truncated.len(), 1);
    assert_eq!(
        truncated[0].code,
        ImportCode::OpaqueTruncated {
            dropped: 2,
            bytes: 21
        }
    );
}

#[test]
fn r128_each_capture_counts_as_a_retained_element() {
    // `max_retained` 10 : StartDate, Task, UID construits (3) ; 7 captures
    // `<X/>` gardées sur 10 000, les autres abandonnées sans erreur, 1 000
    // pertes listées au plus.
    let limits = Limits {
        max_retained: 10,
        ..Limits::default()
    };
    let xml = format!(
        "<Project><StartDate>2026-10-19T08:00:00</StartDate><Tasks><Task><UID>1</UID>{}</Task></Tasks></Project>",
        "<X/>".repeat(10_000)
    );
    let doc = read(xml.as_bytes(), &ReadOptions { limits, ..opts() }).unwrap();
    assert_eq!(task(&doc, 1).extras.0.len(), 7);
    assert_eq!(doc.source.opaque_dropped_total, 10_000 - 7);
    assert_eq!(doc.source.opaque_dropped.len(), MAX_OPAQUE_LOSSES);
    assert_eq!(Limits::default().max_opaque_bytes, 64 * 1024 * 1024);
}
