//! Lecture événementielle du XML (quick-xml) et arbre borné par sous-arbre
//! (ADR-0009, A19).
//!
//! Le document n'est jamais chargé en arbre complet : seuls les sous-arbres
//! utiles sont construits (`Calendars/Calendar`, `Tasks/Task`, les champs
//! scalaires de `Project`, le `TaskUID` de chaque `Assignments/Assignment`) ;
//! le reste est sauté et compté. La construction utilise une pile explicite :
//! aucune récursion, la profondeur est bornée par [`Limits::max_depth`].
//! Toutes les limites sont vérifiées au fil de la lecture, avant
//! l'allocation qu'elles protègent.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::Limits;
use crate::diag::{ImportCode, Sink};
use crate::error::{LineIndex, MspdiError};

/// Élément XML retenu : nom local, texte concaténé, enfants retenus,
/// position (octet du début de la balise ouvrante).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Element {
    /// Nom local (préfixe d'espace de noms retiré).
    pub(crate) name: String,
    /// Texte, CDATA et références résolues, concaténés, sans rognage.
    pub(crate) text: String,
    /// Enfants retenus, dans l'ordre du document.
    pub(crate) children: Vec<Element>,
    /// Octet du début de la balise ouvrante.
    pub(crate) at: u64,
}

/// Champs scalaires de `Project` retenus (§ 7.6).
pub(crate) const PROJECT_FIELDS: &[&str] = &[
    "SaveVersion",
    "GUID",
    "Name",
    "Title",
    "Subject",
    "Category",
    "Company",
    "Manager",
    "Author",
    "CreationDate",
    "ScheduleFromStart",
    "StartDate",
    "FinishDate",
    "CalendarUID",
    "DefaultStartTime",
    "DefaultFinishTime",
    "MinutesPerDay",
    "MinutesPerWeek",
    "DaysPerMonth",
    "DurationFormat",
    "WorkFormat",
    "DefaultTaskType",
    "NewTasksEffortDriven",
    "NewTasksEstimated",
    "HonorConstraints",
    "WeekStartDay",
    "StatusDate",
    "CurrentDate",
    "CriticalSlackLimit",
    "MultipleCriticalPaths",
];

/// Sections d'interface : leur présence signe un enregistrement par
/// MS Project (`SourceInfo::ui_sections`).
pub(crate) const UI_SECTIONS: &[&str] = &[
    "Views",
    "Tables",
    "Filters",
    "Groups",
    "Maps",
    "Reports",
    "Drawings",
    "DataLinks",
    "VBAProjects",
];

/// Résultat de la lecture XML, avant la correspondance typée.
#[derive(Debug, Default)]
pub(crate) struct RawDocument {
    /// Espace de noms de la racine, s'il est déclaré.
    pub(crate) namespace: Option<String>,
    /// Octet du début de la racine `Project`.
    pub(crate) root_at: u64,
    /// Champs scalaires de `Project` (enfants sautés).
    pub(crate) fields: Vec<Element>,
    /// Éléments `Calendars/Calendar`.
    pub(crate) calendars: Vec<Element>,
    /// Éléments `Tasks/Task`.
    pub(crate) tasks: Vec<Element>,
    /// Éléments `Assignments/Assignment`, réduits à leur `TaskUID`.
    pub(crate) assignments: Vec<Element>,
    /// Sections d'interface présentes, dans l'ordre du document, sans doublon.
    pub(crate) ui_sections: Vec<String>,
    /// Éléments sautés.
    pub(crate) skipped_elements: u64,
}

/// Destination d'un sous-arbre retenu, une fois fermé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dest {
    Field,
    Calendar,
    Task,
    Assignment,
}

/// Conteneur de premier niveau.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Container {
    Calendars,
    Tasks,
    Assignments,
}

/// Rôle d'un élément ouvert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// `Project`.
    Root,
    /// Conteneur parcouru (`Calendars`, `Tasks`, `Assignments`).
    Container(Container),
    /// Élément retenu ; `top` pour la racine du sous-arbre ; `only` restreint
    /// les enfants retenus à un nom (les autres sont sautés).
    Build {
        top: Option<Dest>,
        only: Option<&'static str>,
    },
    /// Élément sauté (lui et tout son contenu).
    Skip,
}

/// Élément ouvert.
struct Frame {
    name: String,
    role: Role,
}

/// État de la lecture.
struct State<'s, 'l> {
    limits: Limits,
    open: Vec<Frame>,
    /// Pile des éléments retenus en construction (parallèle aux cadres `Build`).
    building: Vec<Element>,
    doc: RawDocument,
    elements: u64,
    root_seen: bool,
    sink: &'s mut Sink<'l>,
}

/// Lit le document : contrôles structurels (erreurs) et anomalies tolérées
/// (diagnostics dans `sink`).
pub(crate) fn read_document(
    text: &str,
    limits: Limits,
    lines: &LineIndex<'_>,
    sink: &mut Sink<'_>,
) -> Result<RawDocument, MspdiError> {
    let mut reader = Reader::from_str(text);
    {
        let config = reader.config_mut();
        config.expand_empty_elements = true;
        config.check_end_names = true;
        config.trim_text(false);
    }
    let mut state = State {
        limits,
        open: Vec::new(),
        building: Vec::new(),
        doc: RawDocument::default(),
        elements: 0,
        root_seen: false,
        sink,
    };
    loop {
        let at = reader.buffer_position();
        let event = match reader.read_event() {
            Ok(e) => e,
            Err(e) => {
                return Err(MspdiError::Xml {
                    at: lines.locate(reader.error_position()),
                    message: e.to_string(),
                });
            }
        };
        match event {
            Event::Start(e) => state.start(&e, at, lines)?,
            Event::End(_) => state.end(),
            Event::Empty(e) => {
                // `expand_empty_elements` les déplie : branche gardée par sûreté.
                state.start(&e, at, lines)?;
                state.end();
            }
            Event::Text(t) => state.text(&t.xml10_content(), at, lines)?,
            Event::CData(t) => state.text(&t.xml10_content(), at, lines)?,
            Event::GeneralRef(r) => {
                let resolved = if r.is_char_ref() {
                    r.resolve_char_ref().ok().flatten().map(String::from)
                } else {
                    quick_xml::escape::resolve_xml_entity(&r).map(str::to_owned)
                };
                let piece = resolved.unwrap_or_else(|| {
                    let raw = format!("&{};", &*r);
                    state
                        .sink
                        .push(ImportCode::Inconsistent, None, at, &state.path(), Some(&raw));
                    raw
                });
                state.text(&piece, at, lines)?;
            }
            Event::Decl(d) => {
                if let Some(Ok(enc)) = d.encoding() {
                    let enc = enc.trim().to_ascii_lowercase();
                    if enc != "utf-8" && enc != "utf8" {
                        state
                            .sink
                            .push(ImportCode::Inconsistent, None, at, "?xml/encoding", Some(&enc));
                    }
                }
            }
            Event::DocType(_) => {
                return Err(MspdiError::DoctypeNotAllowed { at: lines.locate(at) });
            }
            Event::Comment(_) | Event::PI(_) => {}
            Event::Eof => {
                if !state.open.is_empty() || !state.root_seen {
                    return Err(MspdiError::UnexpectedEof {
                        at: lines.locate(at),
                        open: state.open.iter().map(|f| f.name.clone()).collect(),
                    });
                }
                return Ok(state.doc);
            }
        }
    }
}

/// Nom local d'une balise (préfixe retiré), en texte.
fn local_name(e: &BytesStart<'_>) -> String {
    e.local_name().as_ref().to_owned()
}

/// Espace de noms déclaré par la racine pour son propre préfixe.
fn root_namespace(e: &BytesStart<'_>) -> Option<String> {
    let qname = e.name();
    let wanted = match qname.prefix() {
        Some(p) => format!("xmlns:{}", p.as_ref()),
        None => "xmlns".to_owned(),
    };
    e.attributes()
        .flatten()
        .find_map(|a| (a.key.as_ref() == wanted).then(|| a.value.clone().into_owned()))
}

impl State<'_, '_> {
    /// Chemin des éléments ouverts sous la racine (« Tasks/Task/UID »).
    fn path(&self) -> String {
        let names: Vec<&str> = self.open.iter().skip(1).map(|f| f.name.as_str()).collect();
        names.join("/")
    }

    fn start(&mut self, e: &BytesStart<'_>, at: u64, lines: &LineIndex<'_>) -> Result<(), MspdiError> {
        self.elements = self.elements.saturating_add(1);
        if self.elements > self.limits.max_elements {
            return Err(MspdiError::TooManyElements {
                max: self.limits.max_elements,
            });
        }
        if self.open.len() >= usize::from(self.limits.max_depth) {
            return Err(MspdiError::TooDeep {
                at: lines.locate(at),
                max: self.limits.max_depth,
            });
        }
        let name = local_name(e);
        let role = match self.open.last().map(|f| f.role) {
            None => {
                if self.root_seen {
                    return Err(MspdiError::Xml {
                        at: lines.locate(at),
                        message: "plusieurs éléments racine".to_owned(),
                    });
                }
                if name != "Project" {
                    return Err(MspdiError::NotMspdi { root: name });
                }
                self.root_seen = true;
                self.doc.namespace = root_namespace(e);
                self.doc.root_at = at;
                Role::Root
            }
            Some(Role::Root) => match name.as_str() {
                "Calendars" => Role::Container(Container::Calendars),
                "Tasks" => Role::Container(Container::Tasks),
                "Assignments" => Role::Container(Container::Assignments),
                n if PROJECT_FIELDS.contains(&n) => Role::Build {
                    top: Some(Dest::Field),
                    only: Some(""),
                },
                n => {
                    if UI_SECTIONS.contains(&n) && !self.doc.ui_sections.iter().any(|s| s == n) {
                        self.doc.ui_sections.push(name.clone());
                    }
                    Role::Skip
                }
            },
            Some(Role::Container(c)) => match (c, name.as_str()) {
                (Container::Calendars, "Calendar") => Role::Build {
                    top: Some(Dest::Calendar),
                    only: None,
                },
                (Container::Tasks, "Task") => Role::Build {
                    top: Some(Dest::Task),
                    only: None,
                },
                (Container::Assignments, "Assignment") => Role::Build {
                    top: Some(Dest::Assignment),
                    only: Some("TaskUID"),
                },
                _ => Role::Skip,
            },
            Some(Role::Build { only: Some(want), .. }) if name.as_str() != want => Role::Skip,
            Some(Role::Build { .. }) => Role::Build {
                top: None,
                only: None,
            },
            Some(Role::Skip) => Role::Skip,
        };
        match role {
            Role::Skip => self.doc.skipped_elements = self.doc.skipped_elements.saturating_add(1),
            Role::Build { .. } => self.building.push(Element {
                name: name.clone(),
                text: String::new(),
                children: Vec::new(),
                at,
            }),
            Role::Root | Role::Container(_) => {}
        }
        self.open.push(Frame { name, role });
        Ok(())
    }

    fn end(&mut self) {
        let Some(frame) = self.open.pop() else {
            return;
        };
        let Role::Build { top, .. } = frame.role else {
            return;
        };
        let Some(done) = self.building.pop() else {
            return;
        };
        match top {
            None => {
                if let Some(parent) = self.building.last_mut() {
                    parent.children.push(done);
                }
            }
            Some(Dest::Field) => self.doc.fields.push(done),
            Some(Dest::Calendar) => self.doc.calendars.push(done),
            Some(Dest::Task) => self.doc.tasks.push(done),
            Some(Dest::Assignment) => self.doc.assignments.push(done),
        }
    }

    fn text(&mut self, piece: &str, at: u64, lines: &LineIndex<'_>) -> Result<(), MspdiError> {
        let max = self.limits.max_text_bytes;
        let too_long = || MspdiError::TextTooLong {
            at: lines.locate(at),
            max,
        };
        if piece.len() > max {
            return Err(too_long());
        }
        if !matches!(self.open.last().map(|f| f.role), Some(Role::Build { .. })) {
            return Ok(());
        }
        if let Some(current) = self.building.last_mut() {
            if current.text.len().saturating_add(piece.len()) > max {
                return Err(too_long());
            }
            current.text.push_str(piece);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::string_slice,
        clippy::arithmetic_side_effects,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use proptest::prelude::*;

    use super::{RawDocument, read_document};
    use crate::Limits;
    use crate::diag::{ImportCode, ImportDiagnostic, Sink};
    use crate::error::{LineIndex, MspdiError};

    fn read_with(text: &str, limits: Limits) -> (Result<RawDocument, MspdiError>, Vec<ImportDiagnostic>) {
        let lines = LineIndex::new(text);
        let mut sink = Sink::new(&lines);
        let r = read_document(text, limits, &lines, &mut sink);
        (r, sink.out)
    }

    fn read(text: &str) -> (Result<RawDocument, MspdiError>, Vec<ImportDiagnostic>) {
        read_with(text, Limits::default())
    }

    const SMALL: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <SaveVersion>14</SaveVersion>
  <Name>A &amp; B &#233;&#x41;</Name>
  <Title><![CDATA[x < y]]></Title>
  <Views><View><Name>Gantt</Name></View></Views>
  <Maps/>
  <Resources><Resource><UID>1</UID></Resource></Resources>
  <Calendars>
    <Calendar><UID>1</UID><WeekDays><WeekDay><DayType>1</DayType></WeekDay></WeekDays></Calendar>
  </Calendars>
  <Tasks>
    <Task><UID>0</UID><Name/></Task>
    <Task><UID>1</UID><PredecessorLink><PredecessorUID>0</PredecessorUID></PredecessorLink></Task>
  </Tasks>
  <Assignments>
    <Assignment><UID>5</UID><TaskUID>1</TaskUID><TimephasedData><Type>1</Type></TimephasedData></Assignment>
  </Assignments>
</Project>
"#;

    #[test]
    fn subtrees_and_fields() {
        let (doc, diags) = read(SMALL);
        let doc = doc.unwrap();
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            doc.namespace.as_deref(),
            Some("http://schemas.microsoft.com/project")
        );
        let names: Vec<&str> = doc.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["SaveVersion", "Name", "Title"]);
        assert_eq!(doc.fields[1].text, "A & B éA");
        assert_eq!(doc.fields[2].text, "x < y");
        assert_eq!(doc.ui_sections, ["Views", "Maps"]);
        // Sautés : Views, View, Name ; Maps ; Resources, Resource, UID ;
        // UID et TimephasedData, Type de l'affectation.
        assert_eq!(doc.skipped_elements, 10);
        assert_eq!(doc.calendars.len(), 1);
        let cal = &doc.calendars[0];
        assert_eq!(cal.children[1].children[0].children[0].text, "1");
        assert_eq!(doc.tasks.len(), 2);
        assert_eq!(doc.tasks[0].children[1].name, "Name");
        assert_eq!(doc.tasks[1].children[1].children[0].text, "0");
        assert_eq!(doc.assignments.len(), 1);
        assert_eq!(doc.assignments[0].children.len(), 1);
        assert_eq!(doc.assignments[0].children[0].text, "1");
        // Position : début de la balise.
        assert_eq!(&SMALL[usize::try_from(doc.tasks[1].at).unwrap()..][..6], "<Task>");
    }

    #[test]
    fn prefixed_root() {
        let (doc, _) = read(r#"<p:Project xmlns:p="urn:x"><p:Name>n</p:Name></p:Project>"#);
        let doc = doc.unwrap();
        assert_eq!(doc.namespace.as_deref(), Some("urn:x"));
        assert_eq!(doc.fields[0].text, "n");
    }

    #[test]
    fn tolerated_anomalies() {
        let (doc, diags) =
            read(r#"<?xml version="1.0" encoding="ISO-8859-1"?><Project><Name>&nbsp;x</Name></Project>"#);
        assert_eq!(doc.unwrap().fields[0].text, "&nbsp;x");
        let codes: Vec<&ImportCode> = diags.iter().map(|d| &d.code).collect();
        assert_eq!(codes, [&ImportCode::Inconsistent, &ImportCode::Inconsistent]);
        assert_eq!(diags[1].element, "Name");
        assert_eq!(diags[1].value.as_deref(), Some("&nbsp;"));
    }

    #[test]
    fn structural_errors() {
        assert!(matches!(read("<Projet/>").0, Err(MspdiError::NotMspdi { root }) if root == "Projet"));
        assert!(matches!(
            read("<!DOCTYPE p [<!ENTITY a \"b\">]><Project/>").0,
            Err(MspdiError::DoctypeNotAllowed { .. })
        ));
        match read("<Project><Tasks><Task>").0 {
            Err(MspdiError::UnexpectedEof { open, .. }) => assert_eq!(open, ["Project", "Tasks", "Task"]),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            read(r#"<?xml version="1.0"?>"#).0,
            Err(MspdiError::UnexpectedEof { open, .. }) if open.is_empty()
        ));
        assert!(matches!(read("<Project></Tasks>").0, Err(MspdiError::Xml { .. })));
        assert!(matches!(
            read("<Project/><Project/>").0,
            Err(MspdiError::Xml { .. })
        ));
        assert!(matches!(
            read("<Project><Name>a &b</Name></Project>").0,
            Err(MspdiError::Xml { .. })
        ));
        match read("<Project>\n  <Tasks></Task></Tasks></Project>").0 {
            Err(MspdiError::Xml { at, .. }) => assert_eq!(at.line, 2),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn limits() {
        let deep = format!("{}{}", "<Project>", "<a>".repeat(100_000));
        assert!(matches!(read(&deep).0, Err(MspdiError::TooDeep { max: 64, .. })));
        let many = format!("<Project>{}</Project>", "<a/>".repeat(20));
        let limits = Limits {
            max_elements: 10,
            ..Limits::default()
        };
        assert!(matches!(
            read_with(&many, limits).0,
            Err(MspdiError::TooManyElements { max: 10 })
        ));
        let long = format!("<Project><Name>{}</Name></Project>", "x".repeat(20 * 1024 * 1024));
        assert!(matches!(read(&long).0, Err(MspdiError::TextTooLong { .. })));
        // Texte retenu fait de morceaux : la somme est bornée aussi.
        let limits = Limits {
            max_text_bytes: 8,
            ..Limits::default()
        };
        let pieces = "<Project><Name>abcd&amp;efgh</Name></Project>";
        assert!(matches!(
            read_with(pieces, limits).0,
            Err(MspdiError::TextTooLong { .. })
        ));
    }

    #[test]
    fn every_prefix_is_ok_or_err() {
        for end in 0..=SMALL.len() {
            if let Some(prefix) = SMALL.get(..end) {
                let _ = read(prefix);
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]
        /// Octets modifiés, insérés ou supprimés : jamais de panique.
        #[test]
        fn mutations_never_panic(
            edits in proptest::collection::vec((any::<u16>(), any::<u8>(), 0_u8..3), 1..8)
        ) {
            let mut bytes = SMALL.as_bytes().to_vec();
            for (pos, byte, op) in edits {
                let len = bytes.len().max(1);
                let p = usize::from(pos) % len;
                match op {
                    0 if p < bytes.len() => bytes[p] = byte,
                    1 => bytes.insert(p.min(bytes.len()), byte),
                    _ if p < bytes.len() => { bytes.remove(p); }
                    _ => {}
                }
            }
            if let Ok(text) = std::str::from_utf8(&bytes) {
                let _ = read(text);
            }
        }
    }
}
