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
//!
//! Capture opaque (R-128, J3) : orthogonale au rôle de lecture, elle copie
//! en octets bruts, balise englobante comprise, tout enfant direct de
//! `Project`, `Tasks/Task` ou `Calendars/Calendar` que l'écrivain ne
//! régénère pas (ensembles W de [`crate::written`]). Le sous-arbre capturé
//! n'est pas construit : seules ses bornes (octet du `<` ouvrant, octet après
//! le `>` fermant) sont lues, et la charge est empruntée à l'entrée. Chaque
//! capture compte comme un élément retenu ; le total des charges est borné
//! par [`Limits::max_opaque_bytes`] ([`OpaqueBudget`]).

use std::borrow::Cow;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use planrs_core::{ExtraEntry, Extras, Subject};

use crate::diag::{ImportCode, Sink};
use crate::error::{LineIndex, MspdiError};
use crate::written;
use crate::{EXTRAS_ORIGIN, Limits, MAX_OPAQUE_LOSSES, OpaqueLossReason};

/// Charge opaque capturée (R-128) : élément complet, balise englobante
/// comprise, emprunté à l'entrée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Opaque<'a> {
    /// Nom qualifié de l'élément, tel qu'écrit (clé de l'entrée d'`Extras`).
    pub(crate) key: Cow<'a, str>,
    /// Octets bruts de l'élément.
    pub(crate) payload: &'a str,
    /// Octet du `<` ouvrant.
    pub(crate) at: u64,
}

/// Charge opaque abandonnée, en attente de son entité (UID lu plus tard).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Lost<'a> {
    /// Nom qualifié de l'élément.
    pub(crate) key: Cow<'a, str>,
    /// Taille, balise englobante comprise.
    pub(crate) bytes: u64,
    /// Cause.
    pub(crate) reason: OpaqueLossReason,
    /// Octet du `<` ouvrant.
    pub(crate) at: u64,
}

impl Lost<'_> {
    /// Perte rattachée à son entité.
    pub(crate) fn attach(&self, subject: Option<Subject>) -> PendingLoss {
        PendingLoss {
            subject,
            key: self.key.to_string(),
            bytes: self.bytes,
            reason: self.reason,
            at: self.at,
        }
    }
}

/// Perte rattachée à son entité, position encore en octets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingLoss {
    pub(crate) subject: Option<Subject>,
    pub(crate) key: String,
    pub(crate) bytes: u64,
    pub(crate) reason: OpaqueLossReason,
    pub(crate) at: u64,
}

/// Budget des charges opaques (R-128) : total gardé borné, abandon
/// déterministe de toute capture suivant le premier dépassement, pertes
/// listées au plus [`MAX_OPAQUE_LOSSES`] fois.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct OpaqueBudget {
    /// Plafond ([`Limits::max_opaque_bytes`]).
    max_bytes: u64,
    /// Octets gardés.
    pub(crate) kept_bytes: u64,
    /// Plafond atteint : toute capture suivante est abandonnée.
    truncated: bool,
    /// Charges abandonnées par le plafond, leur taille, la première position.
    cap_dropped: u64,
    cap_dropped_bytes: u64,
    first_cap_drop: Option<u64>,
    /// Charges abandonnées, toutes causes.
    pub(crate) dropped_total: u64,
    /// Pertes listées.
    listed: usize,
}

impl OpaqueBudget {
    pub(crate) fn new(max_bytes: u64) -> Self {
        Self {
            max_bytes,
            ..Self::default()
        }
    }

    /// Admet une charge de `bytes` octets commençant à `at` ; `false` :
    /// abandonnée (plafond déjà atteint, ou atteint par elle).
    pub(crate) fn admit(&mut self, bytes: u64, at: u64) -> bool {
        let total = self.kept_bytes.checked_add(bytes);
        match total {
            Some(t) if !self.truncated && t <= self.max_bytes => {
                self.kept_bytes = t;
                true
            }
            _ => {
                self.cap_drop(bytes, at);
                false
            }
        }
    }

    /// Abandon par le plafond : toute capture suivante l'est aussi.
    fn cap_drop(&mut self, bytes: u64, at: u64) {
        self.truncated = true;
        self.cap_dropped = self.cap_dropped.saturating_add(1);
        self.cap_dropped_bytes = self.cap_dropped_bytes.saturating_add(bytes);
        if self.first_cap_drop.is_none() {
            self.first_cap_drop = Some(at);
        }
    }

    /// Compte une perte ; vrai si elle doit être listée.
    pub(crate) fn note_loss(&mut self) -> bool {
        self.dropped_total = self.dropped_total.saturating_add(1);
        if self.listed < MAX_OPAQUE_LOSSES {
            self.listed = self.listed.saturating_add(1);
            true
        } else {
            false
        }
    }

    /// Diagnostic unique `mspdi.truncated` (`OpaqueTruncated`) si le
    /// plafond a abandonné des charges.
    pub(crate) fn report(&self, sink: &mut Sink<'_>) {
        if let Some(at) = self.first_cap_drop {
            sink.push(
                ImportCode::OpaqueTruncated {
                    dropped: self.cap_dropped,
                    bytes: self.cap_dropped_bytes,
                },
                Some(Subject::Project),
                at,
                "",
                None,
            );
        }
    }
}

/// Entrées d'`Extras` (origine `mspdi`) des charges, dans l'ordre donné.
pub(crate) fn extras(opaque: &[Opaque<'_>]) -> Extras {
    Extras(
        opaque
            .iter()
            .map(|o| ExtraEntry {
                origin: EXTRAS_ORIGIN.to_owned(),
                key: o.key.to_string(),
                payload: o.payload.to_owned(),
            })
            .collect(),
    )
}

/// Captures de la correspondance typée : entrée, budget, pertes rattachées.
pub(crate) struct Opaques<'b, 'a> {
    /// Entrée.
    pub(crate) input: &'a str,
    /// Budget partagé avec la lecture XML.
    pub(crate) budget: &'b mut OpaqueBudget,
    /// Pertes rattachées à leur entité.
    pub(crate) losses: &'b mut Vec<PendingLoss>,
}

impl<'a> Opaques<'_, 'a> {
    /// Rattache les pertes d'un élément à son entité.
    pub(crate) fn attach(&mut self, lost: &[Lost<'_>], subject: Option<Subject>) {
        self.losses.extend(lost.iter().map(|l| l.attach(subject)));
    }

    /// Capture un élément retenu entier (lien abandonné, R-128) ; `None` :
    /// abandonné (plafond, bornes), perte listée.
    pub(crate) fn capture(&mut self, e: &Element<'_>, subject: Option<Subject>) -> Option<Opaque<'a>> {
        let bytes = e.end.saturating_sub(e.at);
        let span = usize::try_from(e.at)
            .ok()
            .zip(usize::try_from(e.end).ok())
            .and_then(|(a, b)| self.input.get(a..b))
            .filter(|p| p.starts_with('<'));
        let reason = match span {
            Some(payload) if self.budget.admit(bytes, e.at) => {
                return Some(Opaque {
                    key: Cow::Borrowed(payload_key(payload)),
                    payload,
                    at: e.at,
                });
            }
            Some(_) => OpaqueLossReason::Cap,
            None => OpaqueLossReason::OutOfBounds,
        };
        if self.budget.note_loss() {
            self.losses.push(PendingLoss {
                subject,
                key: e.name.to_string(),
                bytes,
                reason,
                at: e.at,
            });
        }
        None
    }
}

/// Nom qualifié en tête d'une charge (`<Nom …`), emprunté.
pub(crate) fn payload_key(payload: &str) -> &str {
    let rest = payload.strip_prefix('<').unwrap_or(payload);
    let end = rest
        .find(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/')
        .unwrap_or(rest.len());
    rest.get(..end).unwrap_or(rest)
}

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
    /// Octet suivant la fin de la balise fermante (`/>` d'un élément vide).
    pub(crate) end: u64,
    /// Enfants directs sautés (non retenus), capturés ou non.
    pub(crate) skipped_children: u64,
    /// Charges opaques capturées parmi ses enfants directs (tâche,
    /// calendrier), dans l'ordre du document.
    pub(crate) opaque: Vec<Opaque<'a>>,
    /// Captures abandonnées parmi ses enfants directs (listées seulement).
    pub(crate) lost: Vec<Lost<'a>>,
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
    /// Entrée (découpe des liens abandonnés capturés, R-128).
    pub(crate) input: &'a str,
    /// Charges opaques, enfants directs de `Project`, ordre du document.
    pub(crate) opaque: Vec<Opaque<'a>>,
    /// Captures abandonnées, enfants directs de `Project`.
    pub(crate) lost: Vec<Lost<'a>>,
    /// Budget des charges opaques.
    pub(crate) budget: OpaqueBudget,
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

/// Porteur d'une capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Owner {
    /// `Project`.
    Project,
    /// Élément retenu en construction (tâche, calendrier).
    Element,
}

/// Capture en cours (jamais imbriquée : un sous-arbre capturé ne contient ni
/// tâche ni calendrier).
struct Capture {
    /// Profondeur de l'élément capturé (indice de son cadre).
    depth: usize,
    /// Octet du `<` ouvrant.
    at: u64,
    /// Longueur du nom qualifié.
    qname_len: usize,
    owner: Owner,
    /// Abandonnée d'avance (plafond atteint à l'ouverture).
    doomed: bool,
    /// Entité non résolue rencontrée.
    tainted: bool,
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
    /// Capture opaque active (racine sans préfixe).
    capture_enabled: bool,
    capture: Option<Capture>,
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
        doc: RawDocument {
            input: text,
            budget: OpaqueBudget::new(limits.max_opaque_bytes),
            ..RawDocument::default()
        },
        elements: 0,
        retained: 0,
        root_seen: false,
        capture_enabled: true,
        capture: None,
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
            // Après la balise fermante (ou après `/>` d'un élément vide déplié).
            Event::End(_) => state.end(reader.buffer_position()),
            Event::Empty(e) => {
                // `expand_empty_elements` les déplie : branche gardée par sûreté.
                state.start(&e, at, lines)?;
                state.end(reader.buffer_position());
            }
            Event::Text(t) => state.text(t.xml10_content(), at, lines)?,
            Event::CData(t) => state.text(t.xml10_content(), at, lines)?,
            Event::GeneralRef(r) => {
                let resolved = if r.is_char_ref() {
                    r.resolve_char_ref().ok().flatten().map(String::from)
                } else {
                    quick_xml::escape::resolve_xml_entity(&r).map(str::to_owned)
                };
                if resolved.is_none()
                    && let Some(capture) = &mut state.capture
                {
                    capture.tainted = true;
                }
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

    /// Rôle de la racine : `Project`, unique ; préfixe → aucune capture.
    fn root_role(
        &mut self,
        name: &str,
        e: &BytesStart<'_>,
        at: u64,
        lines: &LineIndex<'_>,
    ) -> Result<Role, MspdiError> {
        if self.root_seen {
            return Err(MspdiError::Xml {
                at: lines.locate(at),
                message: "plusieurs éléments racine".to_owned(),
            });
        }
        if name != "Project" {
            return Err(MspdiError::NotMspdi {
                root: name.to_owned(),
            });
        }
        self.root_seen = true;
        self.doc.namespace = root_namespace(e);
        self.doc.root_at = at;
        if e.name().prefix().is_some() {
            // R-128 : les charges porteraient un préfixe non déclaré une fois
            // réémises ; rien n'est capturé.
            self.capture_enabled = false;
            self.sink.push(
                ImportCode::Unsupported {
                    what: "prefixed_namespace".to_owned(),
                },
                Some(Subject::Project),
                at,
                "",
                Some(name),
            );
        }
        Ok(Role::Root)
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
            None => self.root_role(&name, e, at, lines)?,
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
        let parent_role = self.open.last().map(|f| f.role);
        self.begin_capture(parent_role, &name, e, at);
        match role {
            Role::Skip => {
                self.doc.skipped_elements = self.doc.skipped_elements.saturating_add(1);
                if matches!(parent_role, Some(Role::Build { .. }))
                    && let Some(parent) = self.building.last_mut()
                {
                    parent.skipped_children = parent.skipped_children.saturating_add(1);
                }
            }
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
                    at,
                    ..Element::default()
                });
            }
            Role::Root | Role::Container(_) => {}
        }
        self.open.push(Frame { name, role });
        Ok(())
    }

    /// Ouvre une capture si `name`, enfant direct de `Project`, d'une tâche
    /// ou d'un calendrier, n'est pas régénéré par l'écrivain (R-128).
    fn begin_capture(&mut self, parent_role: Option<Role>, name: &str, e: &BytesStart<'_>, at: u64) {
        if self.capture.is_some() || !self.capture_enabled {
            return;
        }
        let owner = match parent_role {
            Some(Role::Root) => written::project_captures(name).then_some(Owner::Project),
            Some(Role::Build { kind: Kind::Task, .. }) => {
                written::task_captures(name).then_some(Owner::Element)
            }
            Some(Role::Build {
                kind: Kind::Calendar, ..
            }) => written::calendar_captures(name).then_some(Owner::Element),
            _ => None,
        };
        let Some(owner) = owner else {
            return;
        };
        // Chaque capture compte comme un élément retenu ; au-delà du plafond,
        // abandon (comme pour le plafond d'octets).
        let doomed = if self.doc.budget.truncated || self.retained >= self.limits.max_retained {
            true
        } else {
            self.retained = self.retained.saturating_add(1);
            false
        };
        self.capture = Some(Capture {
            depth: self.open.len(),
            at,
            qname_len: e.name().as_ref().len(),
            owner,
            doomed,
            tainted: false,
        });
    }

    fn end(&mut self, end: u64) {
        let Some(frame) = self.open.pop() else {
            return;
        };
        if let Role::Build { top, .. } = frame.role
            && let Some(mut done) = self.building.pop()
        {
            done.end = end;
            self.finish_element(top, done);
        }
        if self.capture.as_ref().is_some_and(|c| c.depth == self.open.len())
            && let Some(capture) = self.capture.take()
        {
            self.finish_capture(&capture, end);
        }
    }

    /// Clôt une capture : charge gardée sur son porteur, ou perte listée.
    fn finish_capture(&mut self, capture: &Capture, end: u64) {
        let bytes = end.saturating_sub(capture.at);
        let span = usize::try_from(capture.at)
            .ok()
            .zip(usize::try_from(end).ok())
            .and_then(|(a, b)| self.input.get(a..b));
        let key: Cow<'a, str> = span
            .and_then(|p| p.get(1..capture.qname_len.checked_add(1)?))
            .map_or(Cow::Borrowed(""), Cow::Borrowed);
        let budget = &mut self.doc.budget;
        let outcome = if capture.tainted {
            Err(OpaqueLossReason::UnresolvedEntity)
        } else if capture.doomed {
            budget.cap_drop(bytes, capture.at);
            Err(OpaqueLossReason::Cap)
        } else if let Some(payload) = span {
            if budget.admit(bytes, capture.at) {
                Ok(payload)
            } else {
                Err(OpaqueLossReason::Cap)
            }
        } else {
            Err(OpaqueLossReason::OutOfBounds)
        };
        let (opaque, lost) = match capture.owner {
            Owner::Project => (&mut self.doc.opaque, &mut self.doc.lost),
            Owner::Element => {
                let Some(owner) = self.building.last_mut() else {
                    return;
                };
                (&mut owner.opaque, &mut owner.lost)
            }
        };
        match outcome {
            Ok(payload) => opaque.push(Opaque {
                key,
                payload,
                at: capture.at,
            }),
            Err(reason) => {
                if self.doc.budget.note_loss() {
                    lost.push(Lost {
                        key,
                        bytes,
                        reason,
                        at: capture.at,
                    });
                }
            }
        }
    }

    /// Range un élément retenu fermé.
    fn finish_element(&mut self, top: Option<Dest>, done: Element<'a>) {
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
        // Les éléments sautés ne sont pas construits ; chaque capture opaque
        // compte comme un élément retenu (R-128) : Task, UID, puis une seule
        // capture ; les 999 suivantes sont abandonnées, sans erreur.
        let skipped = format!(
            "<Project><Tasks><Task><UID>1</UID>{}</Task></Tasks></Project>",
            "<X/>".repeat(1_000)
        );
        let doc = read_with(&skipped, limits).0.unwrap();
        assert_eq!(doc.skipped_elements, 1_000);
        assert_eq!(doc.tasks[0].opaque.len(), 1);
        assert_eq!(doc.budget.dropped_total, 999);
        assert_eq!(doc.tasks[0].lost.len(), 999);
    }

    #[test]
    fn r128_capture_bounds_and_roles() {
        // Bornes : du `<` ouvrant à l'octet après le `>` fermant ; `<X/>`
        // déplié : après `/>`. Capture orthogonale au rôle : `LevelingDelay`
        // construit et capturé, `Assignments` parcouru et capturé.
        let text = "<Project><Title>t</Title><Foo a=\"1\">x<b/>&amp;</Foo>\n<Tasks><Task><UID>1</UID>\
                    <LevelingDelay>48</LevelingDelay><X/><!-- c --><Baseline><Number>0</Number></Baseline>\
                    </Task></Tasks><Calendars><Calendar><UID>1</UID><Bar>1</Bar></Calendar></Calendars>\
                    <Assignments><Assignment><TaskUID>1</TaskUID></Assignment></Assignments></Project>";
        let (doc, diags) = read(text);
        let doc = doc.unwrap();
        assert!(diags.is_empty(), "{diags:?}");
        let pairs = |o: &[super::Opaque<'_>]| {
            o.iter()
                .map(|o| (o.key.to_string(), o.payload.to_owned()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            pairs(&doc.opaque),
            [
                ("Foo".to_owned(), "<Foo a=\"1\">x<b/>&amp;</Foo>".to_owned()),
                (
                    "Assignments".to_owned(),
                    "<Assignments><Assignment><TaskUID>1</TaskUID></Assignment></Assignments>".to_owned()
                ),
            ]
        );
        assert_eq!(
            pairs(&doc.tasks[0].opaque),
            [
                (
                    "LevelingDelay".to_owned(),
                    "<LevelingDelay>48</LevelingDelay>".to_owned()
                ),
                ("X".to_owned(), "<X/>".to_owned()),
                (
                    "Baseline".to_owned(),
                    "<Baseline><Number>0</Number></Baseline>".to_owned()
                ),
            ]
        );
        let names: Vec<&str> = doc.tasks[0].children.iter().map(|c| c.name.as_ref()).collect();
        assert_eq!(names, ["UID", "LevelingDelay"]);
        assert_eq!(
            pairs(&doc.calendars[0].opaque),
            [("Bar".to_owned(), "<Bar>1</Bar>".to_owned())]
        );
        assert_eq!(doc.assignments.len(), 1);
        let kept: u64 = [&doc.opaque, &doc.tasks[0].opaque, &doc.calendars[0].opaque]
            .iter()
            .flat_map(|o| o.iter())
            .map(|o| u64::try_from(o.payload.len()).unwrap())
            .sum();
        assert_eq!(doc.budget.kept_bytes, kept);
        // Positions : `<` ouvrant.
        assert_eq!(
            &text[usize::try_from(doc.tasks[0].opaque[1].at).unwrap()..][..4],
            "<X/>"
        );
    }

    #[test]
    fn r128_prefixed_root_is_not_captured() {
        let (doc, diags) = read(
            r#"<p:Project xmlns:p="urn:x"><p:Foo>1</p:Foo><p:Tasks><p:Task><p:UID>1</p:UID><p:X/></p:Task></p:Tasks></p:Project>"#,
        );
        let doc = doc.unwrap();
        assert!(doc.opaque.is_empty() && doc.tasks[0].opaque.is_empty());
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].code,
            ImportCode::Unsupported {
                what: "prefixed_namespace".to_owned()
            }
        );
    }

    #[test]
    fn r128_unresolved_entity_drops_the_capture() {
        let (doc, diags) = read("<Project><Foo>&nbsp;x</Foo><Bar>&#233;</Bar></Project>");
        let doc = doc.unwrap();
        assert_eq!(doc.opaque.len(), 1);
        assert_eq!(doc.opaque[0].payload, "<Bar>&#233;</Bar>");
        assert_eq!(doc.lost.len(), 1);
        assert_eq!(doc.lost[0].key, "Foo");
        assert_eq!(doc.lost[0].reason, crate::OpaqueLossReason::UnresolvedEntity);
        assert_eq!(doc.lost[0].bytes, 18);
        assert_eq!(diags[0].code, ImportCode::Inconsistent);
    }

    #[test]
    fn r128_byte_cap_is_deterministic() {
        // Plafond de 10 octets : `<A>12</A>` (9) gardé, `<B>12</B>` (9)
        // dépasse et toute capture suivante est abandonnée, même `<C/>` (4)
        // qui tiendrait.
        let limits = Limits {
            max_opaque_bytes: 10,
            ..Limits::default()
        };
        let text = "<Project><A>12</A><B>12</B><C/></Project>";
        let doc = read_with(text, limits).0.unwrap();
        assert_eq!(doc.opaque.len(), 1);
        assert_eq!(doc.budget.kept_bytes, 9);
        let lost: Vec<(&str, u64)> = doc.lost.iter().map(|l| (l.key.as_ref(), l.bytes)).collect();
        assert_eq!(lost, [("B", 9), ("C", 4)]);
        assert!(doc.lost.iter().all(|l| l.reason == crate::OpaqueLossReason::Cap));
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
