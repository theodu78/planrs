//! Lecture événementielle du XML (quick-xml) et arbre borné par sous-arbre
//! (ADR-0009, A19).
//!
//! Le document n'est jamais chargé en arbre complet : seuls les sous-arbres
//! utiles sont construits (`Calendars/Calendar`, `Tasks/Task`, les champs
//! scalaires de `Project`, le `TaskUID` de chaque `Assignments/Assignment`) ;
//! le reste est sauté et compté. Dans un sous-arbre retenu, seuls les enfants
//! que l'import lit sont gardés ([`Kind`], listes blanches comme
//! [`PROJECT_FIELDS`]) : `Baseline`, `ExtendedAttribute`, `TimephasedData`
//! ou un élément inconnu sont sautés, si bien qu'un arbre retenu a au plus
//! six niveaux, quelle que soit la profondeur de l'entrée (aucune récursion,
//! même à la libération). La construction utilise une pile explicite. Toutes
//! les limites sont vérifiées au fil de la lecture, avant l'allocation
//! qu'elles protègent, dont le nombre d'éléments retenus
//! ([`Limits::max_retained`]).

use std::borrow::Cow;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::Limits;
use crate::diag::{ImportCode, Sink};
use crate::error::{LineIndex, MspdiError};

/// Élément XML retenu : nom local, texte concaténé, enfants retenus,
/// position (octet du début de la balise ouvrante).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Element<'a> {
    /// Nom local (préfixe d'espace de noms retiré), emprunté à l'entrée.
    pub(crate) name: Cow<'a, str>,
    /// Texte, CDATA et références résolues, concaténés, sans rognage ;
    /// emprunté à l'entrée tant qu'il tient en un morceau.
    pub(crate) text: Cow<'a, str>,
    /// Enfants retenus, dans l'ordre du document.
    pub(crate) children: Vec<Element<'a>>,
    /// L'élément a des enfants, retenus ou non : son texte (indentation)
    /// n'est pas lu.
    pub(crate) structured: bool,
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

/// Champs scalaires d'une tâche (`Tasks/Task`) lus par l'import (`task.rs`).
/// Toute lecture d'un nouveau champ doit l'ajouter ici, sinon il est sauté.
pub(crate) const TASK_FIELDS: &[&str] = &[
    "UID",
    "ID",
    "GUID",
    "Name",
    "Type",
    "IsNull",
    "CreateDate",
    "WBS",
    "OutlineLevel",
    "Priority",
    "Start",
    "Finish",
    "Duration",
    "DurationFormat",
    "ManualStart",
    "ManualFinish",
    "ManualDuration",
    "Manual",
    "EffortDriven",
    "Estimated",
    "Milestone",
    "Summary",
    "Critical",
    "Active",
    "Recurring",
    "IgnoreResourceCalendar",
    "CalendarUID",
    "ConstraintType",
    "ConstraintDate",
    "Deadline",
    "EarlyStart",
    "EarlyFinish",
    "LateStart",
    "LateFinish",
    "TotalSlack",
    "FreeSlack",
    "LevelingDelay",
    "PercentComplete",
    "PercentWorkComplete",
    "PhysicalPercentComplete",
    "ActualStart",
    "ActualFinish",
    "ActualDuration",
    "RemainingDuration",
    "Notes",
];

/// Champs d'un `PredecessorLink` lus par l'import (`task.rs`).
pub(crate) const LINK_FIELDS: &[&str] = &["PredecessorUID", "Type", "CrossProject", "LinkLag", "LagFormat"];

/// Champs scalaires d'un calendrier (`Calendars/Calendar`).
pub(crate) const CALENDAR_FIELDS: &[&str] = &["UID", "Name", "GUID", "IsBaseCalendar", "BaseCalendarUID"];

/// Champs scalaires d'une exception (`Exceptions/Exception`).
pub(crate) const EXCEPTION_FIELDS: &[&str] = &[
    "Name",
    "Type",
    "Period",
    "DaysOfWeek",
    "MonthItem",
    "MonthPosition",
    "Month",
    "MonthDay",
    "EnteredByOccurrences",
    "Occurrences",
    "DayWorking",
];

/// Forme d'un élément retenu : décide lesquels de ses enfants sont retenus
/// (et sous quelle forme) ; les autres sont sautés et comptés.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Champ scalaire : aucun enfant retenu.
    Scalar,
    /// `Tasks/Task`.
    Task,
    /// `PredecessorLink`.
    Link,
    /// `Calendars/Calendar`.
    Calendar,
    /// `WeekDays`.
    WeekDays,
    /// `WeekDay`.
    WeekDay,
    /// `WorkingTimes`.
    WorkingTimes,
    /// `WorkingTime`.
    WorkingTime,
    /// `TimePeriod`.
    TimePeriod,
    /// `Exceptions`.
    Exceptions,
    /// `Exception`.
    Exception,
    /// `WorkWeeks`.
    WorkWeeks,
    /// `WorkWeek`.
    WorkWeek,
    /// `Assignments/Assignment` : son seul `TaskUID`.
    Assignment,
}

/// Forme de l'enfant `name` d'un élément retenu de forme `parent` ; `None` :
/// enfant sauté.
fn child_kind(parent: Kind, name: &str) -> Option<Kind> {
    let scalar = |known: bool| known.then_some(Kind::Scalar);
    match (parent, name) {
        (Kind::Task, "PredecessorLink") => Some(Kind::Link),
        (Kind::Task, n) => scalar(TASK_FIELDS.contains(&n)),
        (Kind::Link, n) => scalar(LINK_FIELDS.contains(&n)),
        (Kind::Calendar | Kind::WorkWeek, "WeekDays") => Some(Kind::WeekDays),
        (Kind::Calendar, "Exceptions") => Some(Kind::Exceptions),
        (Kind::Calendar, "WorkWeeks") => Some(Kind::WorkWeeks),
        (Kind::Calendar, n) => scalar(CALENDAR_FIELDS.contains(&n)),
        (Kind::WeekDays, "WeekDay") => Some(Kind::WeekDay),
        (Kind::WeekDay | Kind::Exception, "WorkingTimes") => Some(Kind::WorkingTimes),
        (Kind::WeekDay | Kind::Exception | Kind::WorkWeek, "TimePeriod") => Some(Kind::TimePeriod),
        (Kind::WeekDay, n) => scalar(matches!(n, "DayType" | "DayWorking")),
        (Kind::WorkingTimes, "WorkingTime") => Some(Kind::WorkingTime),
        (Kind::WorkingTime, n) => scalar(matches!(n, "FromTime" | "ToTime")),
        (Kind::TimePeriod, n) => scalar(matches!(n, "FromDate" | "ToDate")),
        (Kind::Exceptions, "Exception") => Some(Kind::Exception),
        (Kind::Exception, n) => scalar(EXCEPTION_FIELDS.contains(&n)),
        (Kind::WorkWeeks, "WorkWeek") => Some(Kind::WorkWeek),
        (Kind::WorkWeek, n) => scalar(n == "Name"),
        (Kind::Assignment, n) => scalar(n == "TaskUID"),
        // Champ scalaire, ou enfant inconnu d'une liste.
        (Kind::Scalar | Kind::WeekDays | Kind::WorkingTimes | Kind::Exceptions | Kind::WorkWeeks, _) => None,
    }
}

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
pub(crate) struct RawDocument<'a> {
    /// Espace de noms de la racine, s'il est déclaré.
    pub(crate) namespace: Option<String>,
    /// Octet du début de la racine `Project`.
    pub(crate) root_at: u64,
    /// Champs scalaires de `Project` (enfants sautés).
    pub(crate) fields: Vec<Element<'a>>,
    /// Éléments `Calendars/Calendar`.
    pub(crate) calendars: Vec<Element<'a>>,
    /// Éléments `Tasks/Task`.
    pub(crate) tasks: Vec<Element<'a>>,
    /// Éléments `Assignments/Assignment`, réduits à leur `TaskUID`.
    pub(crate) assignments: Vec<Element<'a>>,
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
    /// Élément retenu ; `top` pour la racine du sous-arbre ; `kind` décide
    /// des enfants retenus (les autres sont sautés).
    Build { top: Option<Dest>, kind: Kind },
    /// Élément sauté (lui et tout son contenu).
    Skip,
}

/// Élément ouvert.
struct Frame<'a> {
    name: Cow<'a, str>,
    role: Role,
}

/// État de la lecture.
struct State<'a, 's, 'l> {
    /// Entrée.
    input: &'a str,
    limits: Limits,
    open: Vec<Frame<'a>>,
    /// Pile des éléments retenus en construction (parallèle aux cadres `Build`).
    building: Vec<Element<'a>>,
    doc: RawDocument<'a>,
    elements: u64,
    /// Éléments retenus (construits), bornés par [`Limits::max_retained`].
    retained: u64,
    root_seen: bool,
    sink: &'s mut Sink<'l>,
}

/// Lit le document : contrôles structurels (erreurs) et anomalies tolérées
/// (diagnostics dans `sink`).
pub(crate) fn read_document<'a>(
    text: &'a str,
    limits: Limits,
    lines: &LineIndex<'_>,
    sink: &mut Sink<'_>,
) -> Result<RawDocument<'a>, MspdiError> {
    let mut reader = Reader::from_str(text);
    {
        let config = reader.config_mut();
        config.expand_empty_elements = true;
        config.check_end_names = true;
        config.trim_text(false);
    }
    let mut state = State {
        input: text,
        limits,
        open: Vec::new(),
        building: Vec::new(),
        doc: RawDocument::default(),
        elements: 0,
        retained: 0,
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
            Event::Text(t) => state.text(t.xml10_content(), at, lines)?,
            Event::CData(t) => state.text(t.xml10_content(), at, lines)?,
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
                state.text(Cow::Owned(piece), at, lines)?;
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
                        open: state.open.iter().map(|f| f.name.to_string()).collect(),
                    });
                }
                return Ok(state.doc);
            }
        }
    }
}

/// Nom local d'une balise (préfixe retiré), emprunté à l'entrée : la balise
/// commence à l'octet `at` par `<` suivi du nom qualifié. Repli sur une copie
/// si l'entrée ne correspond pas (jamais attendu).
fn local_name<'a>(input: &'a str, at: u64, e: &BytesStart<'_>) -> Cow<'a, str> {
    let qname = e.name();
    let qname = qname.as_ref();
    let borrowed = usize::try_from(at)
        .ok()
        .and_then(|a| a.checked_add(1))
        .and_then(|start| input.get(start..start.checked_add(qname.len())?))
        .filter(|s| *s == qname)
        .and_then(|s| s.rsplit(':').next());
    borrowed.map_or_else(|| Cow::Owned(e.local_name().as_ref().to_owned()), Cow::Borrowed)
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

impl<'a> State<'a, '_, '_> {
    /// Chemin des éléments ouverts sous la racine (« Tasks/Task/UID »).
    fn path(&self) -> String {
        let names: Vec<&str> = self.open.iter().skip(1).map(|f| f.name.as_ref()).collect();
        names.join("/")
    }

    fn start(&mut self, e: &BytesStart<'_>, at: u64, lines: &LineIndex<'_>) -> Result<(), MspdiError> {
        self.elements = self.elements.saturating_add(1);
        if self.elements > self.limits.max_elements {
            return Err(MspdiError::TooManyElements {
                max: self.limits.max_elements,
            });
        }
        let max_depth = self.limits.max_depth.min(crate::MAX_DEPTH);
        if self.open.len() >= usize::from(max_depth) {
            return Err(MspdiError::TooDeep {
                at: lines.locate(at),
                max: max_depth,
            });
        }
        let name = local_name(self.input, at, e);
        let role = match self.open.last().map(|f| f.role) {
            None => {
                if self.root_seen {
                    return Err(MspdiError::Xml {
                        at: lines.locate(at),
                        message: "plusieurs éléments racine".to_owned(),
                    });
                }
                if name != "Project" {
                    return Err(MspdiError::NotMspdi {
                        root: name.into_owned(),
                    });
                }
                self.root_seen = true;
                self.doc.namespace = root_namespace(e);
                self.doc.root_at = at;
                Role::Root
            }
            Some(Role::Root) => match name.as_ref() {
                "Calendars" => Role::Container(Container::Calendars),
                "Tasks" => Role::Container(Container::Tasks),
                "Assignments" => Role::Container(Container::Assignments),
                n if PROJECT_FIELDS.contains(&n) => Role::Build {
                    top: Some(Dest::Field),
                    kind: Kind::Scalar,
                },
                n => {
                    if UI_SECTIONS.contains(&n) && !self.doc.ui_sections.iter().any(|s| s == n) {
                        self.doc.ui_sections.push(name.to_string());
                    }
                    Role::Skip
                }
            },
            Some(Role::Container(c)) => match (c, name.as_ref()) {
                (Container::Calendars, "Calendar") => Role::Build {
                    top: Some(Dest::Calendar),
                    kind: Kind::Calendar,
                },
                (Container::Tasks, "Task") => Role::Build {
                    top: Some(Dest::Task),
                    kind: Kind::Task,
                },
                (Container::Assignments, "Assignment") => Role::Build {
                    top: Some(Dest::Assignment),
                    kind: Kind::Assignment,
                },
                _ => Role::Skip,
            },
            Some(Role::Build { kind, .. }) => {
                // Le texte d'un élément qui a des enfants n'est jamais lu
                // (indentation) : il n'est pas gardé, même si l'enfant est
                // sauté.
                if let Some(parent) = self.building.last_mut() {
                    parent.structured = true;
                    parent.text = Cow::Borrowed("");
                }
                child_kind(kind, &name).map_or(Role::Skip, |kind| Role::Build { top: None, kind })
            }
            Some(Role::Skip) => Role::Skip,
        };
        match role {
            Role::Skip => self.doc.skipped_elements = self.doc.skipped_elements.saturating_add(1),
            Role::Build { .. } => {
                self.retained = self.retained.saturating_add(1);
                if self.retained > self.limits.max_retained {
                    return Err(MspdiError::TooManyRetained {
                        at: lines.locate(at),
                        max: self.limits.max_retained,
                    });
                }
                self.building.push(Element {
                    name: name.clone(),
                    text: Cow::Borrowed(""),
                    children: Vec::new(),
                    structured: false,
                    at,
                });
            }
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

    fn text(&mut self, piece: Cow<'a, str>, at: u64, lines: &LineIndex<'_>) -> Result<(), MspdiError> {
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
            if current.structured {
                return Ok(());
            }
            if current.text.len().saturating_add(piece.len()) > max {
                return Err(too_long());
            }
            if current.text.is_empty() {
                current.text = piece;
            } else {
                current.text.to_mut().push_str(&piece);
            }
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

    fn read_with(text: &str, limits: Limits) -> (Result<RawDocument<'_>, MspdiError>, Vec<ImportDiagnostic>) {
        let lines = LineIndex::new(text);
        let mut sink = Sink::new(&lines);
        let r = read_document(text, limits, &lines, &mut sink);
        (r, sink.out)
    }

    fn read(text: &str) -> (Result<RawDocument<'_>, MspdiError>, Vec<ImportDiagnostic>) {
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
        let names: Vec<&str> = doc.fields.iter().map(|f| f.name.as_ref()).collect();
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
    fn only_read_children_are_retained() {
        // Sous `Task` et `Calendar`, seuls les champs lus par l'import sont
        // construits ; le reste est sauté et compté, à tout niveau. Le texte
        // d'un champ qui a des enfants (même sautés) n'est pas lu.
        let text = "<Project><Tasks><Task><UID>1</UID><Baseline><Number>0</Number></Baseline>\
                    <ExtendedAttribute><FieldID>1</FieldID></ExtendedAttribute><X/><Name>a<b/>c</Name>\
                    <PredecessorLink><PredecessorUID>2</PredecessorUID><Junk/></PredecessorLink></Task></Tasks>\
                    <Calendars><Calendar><UID>1</UID><Foo/><WeekDays><WeekDay><DayType>1</DayType><Bar/>\
                    </WeekDay></WeekDays></Calendar></Calendars></Project>";
        let (doc, diags) = read(text);
        let doc = doc.unwrap();
        assert!(diags.is_empty(), "{diags:?}");
        let names =
            |e: &super::Element<'_>| e.children.iter().map(|c| c.name.to_string()).collect::<Vec<_>>();
        let task = &doc.tasks[0];
        assert_eq!(names(task), ["UID", "Name", "PredecessorLink"]);
        assert_eq!(task.children[1].text, "");
        assert_eq!(names(&task.children[2]), ["PredecessorUID"]);
        let cal = &doc.calendars[0];
        assert_eq!(names(cal), ["UID", "WeekDays"]);
        assert_eq!(names(&cal.children[1].children[0]), ["DayType"]);
        // Baseline, Number, ExtendedAttribute, FieldID, X, b, Junk, Foo, Bar.
        assert_eq!(doc.skipped_elements, 9);
    }

    #[test]
    fn read_lists_agree_with_the_schema_table() {
        // Les listes des champs lus (J2) ne nomment que des éléments que la
        // table du schéma classe comme lus (M, M?, D, C, F), ou `GUID`.
        use crate::schema::{Class, Parent, sequence};
        let lists: [(Parent, &[&str]); 4] = [
            (Parent::Project, super::PROJECT_FIELDS),
            (Parent::Task, super::TASK_FIELDS),
            (Parent::PredecessorLink, super::LINK_FIELDS),
            (Parent::Calendar, super::CALENDAR_FIELDS),
        ];
        for (parent, names) in lists
            .into_iter()
            .chain([(Parent::Exception, super::EXCEPTION_FIELDS)])
        {
            for name in names {
                let class = sequence(parent).class(name);
                let read = matches!(
                    class,
                    Some(
                        Class::Modeled | Class::ModeledOpt | Class::Derived | Class::Computed | Class::Format
                    )
                );
                // `LevelingDelay` : conservé (P) et lu pour le harnais (R-083).
                assert!(
                    read || *name == "GUID" || *name == "LevelingDelay",
                    "{parent:?}/{name} : {class:?}"
                );
            }
        }
    }

    #[test]
    fn retained_budget() {
        // Name, Task, UID, Name : le quatrième élément retenu dépasse 3.
        let limits = Limits {
            max_retained: 3,
            ..Limits::default()
        };
        let text = "<Project><Name>a</Name><Tasks><Task><UID>1</UID><Name>x</Name></Task></Tasks></Project>";
        assert!(matches!(
            read_with(text, limits).0,
            Err(MspdiError::TooManyRetained { max: 3, .. })
        ));
        // Les éléments sautés ne comptent pas.
        let skipped = format!(
            "<Project><Tasks><Task><UID>1</UID>{}</Task></Tasks></Project>",
            "<X/>".repeat(1_000)
        );
        let doc = read_with(&skipped, limits).0.unwrap();
        assert_eq!(doc.skipped_elements, 1_000);
    }

    #[test]
    fn depth_is_capped_whatever_the_limit() {
        let limits = Limits {
            max_depth: u16::MAX,
            ..Limits::default()
        };
        let deep = format!("<Project>{}", "<a>".repeat(1_000));
        assert!(matches!(
            read_with(&deep, limits).0,
            Err(MspdiError::TooDeep {
                max: crate::MAX_DEPTH,
                ..
            })
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
