//! « Informations sur la tâche » (J7, lot L9, H-UI-19) : fenêtre modale à
//! cinq onglets (Général, Prédécesseurs, Ressources, Avancé, Notes) dont
//! « OK » produit **un seul lot** du registre (une entrée d'annulation).
//!
//! Le brouillon ([`TaskInfoDraft`]) est fait de textes : il est lu à
//! l'ouverture par [`TaskInfoDraft::from_session`] (écritures R-141, R-142,
//! R-143 du moteur, toujours relisibles) et traduit en lot par
//! [`TaskInfoDraft::to_batch`], fonction pure testée sans egui. Aucune date
//! n'est calculée ici (règle 3) : les textes sont lus par
//! `planrs_automation::text`, le registre applique et recalcule. Seule la
//! résolution ID → UID des prédécesseurs se fait côté interface (R-150),
//! comme dans la table.
//!
//! Comparaison **par texte** : un champ dont le texte n'a pas changé depuis
//! l'ouverture n'envoie rien (une date relue ne pose pas de contrainte par
//! R-158, un décalage nul relu ne change pas d'unité). Ordre du lot :
//! `task.update`, `link.add`, `link.update`, `link.remove`,
//! `assignment.add`, `assignment.update`, `assignment.remove`,
//! `task.set_constraint`, `task.set_deadline`, `progress.update`.

use std::collections::BTreeMap;

use planrs_automation::command::{
    AssignmentAdd, AssignmentKey, AssignmentUpdate, ConstraintKindJson, DurationJson, LagJson, LinkKey,
    LinkSpec, LinkTypeJson, PercentJson, ProgressUpdate, ResourceRef, SetConstraint, SetDeadline,
    TaskTypeJson, TaskUpdate, UnitsJson,
};
use planrs_automation::text::{
    DateRole, Language, TextContext, TextError, format_date, format_duration, format_lag, parse_date,
    parse_duration, parse_lag, parse_percent, write_decimal,
};
use planrs_automation::{Batch, Command, CommandError, Patch, Session, TaskRef};
use planrs_core::{
    ConstraintKind, Decimal, LinkType, ResourceKind, ResourceUid, TaskOutcome, TaskRole, TaskType, TaskUid,
};

use crate::edit::EditError;
use crate::i18n::{self, Arg, TextId, Texts, error_text_id};

/// Onglet de la fenêtre.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tab {
    /// Nom, durée, % achevé, début, fin, active.
    #[default]
    General,
    /// Liens vers les prédécesseurs (S67).
    Predecessors,
    /// Affectations.
    Resources,
    /// Contrainte, échéance, type de tâche, pilotage par l'effort (S66).
    Advanced,
    /// Notes.
    Notes,
}

impl Tab {
    /// Onglets, dans l'ordre d'affichage.
    pub const ALL: [Self; 5] = [
        Self::General,
        Self::Predecessors,
        Self::Resources,
        Self::Advanced,
        Self::Notes,
    ];

    /// Titre de l'onglet.
    #[must_use]
    pub const fn title(self) -> TextId {
        match self {
            Self::General => TextId::TaskInfoTabGeneral,
            Self::Predecessors => TextId::TaskInfoTabPredecessors,
            Self::Resources => TextId::TaskInfoTabResources,
            Self::Advanced => TextId::TaskInfoTabAdvanced,
            Self::Notes => TextId::TaskInfoTabNotes,
        }
    }
}

/// Onglet Général (textes saisis).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct General {
    /// Nom.
    pub name: String,
    /// Durée (R-140, R-141).
    pub duration: String,
    /// % achevé (« 50% »).
    pub percent: String,
    /// Début (R-143) ; vide pour une tâche sans dates.
    pub start: String,
    /// Fin (R-143).
    pub finish: String,
    /// Tâche active.
    pub active: bool,
}

/// Ligne de l'onglet Prédécesseurs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRow {
    /// ID du prédécesseur (position + 1, R-150) ; une ligne dont l'ID est
    /// vide est ignorée.
    pub predecessor: String,
    /// Type de lien.
    pub kind: LinkType,
    /// Décalage (R-141, « +2j », « -50% ») ; vide = aucun.
    pub lag: String,
}

impl LinkRow {
    /// Ligne neuve (bouton « Ajouter ») : ID vide, FD, sans décalage.
    #[must_use]
    pub const fn blank() -> Self {
        Self {
            predecessor: String::new(),
            kind: LinkType::FinishToStart,
            lag: String::new(),
        }
    }
}

/// Ligne de l'onglet Ressources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssignRow {
    /// Ressource affectée.
    pub resource: ResourceUid,
    /// Unités (« 50% ») pour une ressource de travail, quantité (« 5 »)
    /// pour une ressource matérielle, ignorées pour une ressource coût.
    pub units: String,
}

/// Onglet Avancé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Advanced {
    /// Type de contrainte.
    pub constraint: ConstraintKind,
    /// Date de contrainte (types datés seulement).
    pub constraint_date: String,
    /// Échéance ; vide = aucune.
    pub deadline: String,
    /// Type de tâche.
    pub task_type: TaskType,
    /// Pilotée par l'effort.
    pub effort_driven: bool,
}

/// Brouillon de la fenêtre, figé à l'ouverture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskInfoDraft {
    /// Tâche.
    pub uid: TaskUid,
    /// `cache_key` du document à l'ouverture (numéro d'ouverture,
    /// génération) : un « OK » sur un document changé depuis est refusé.
    pub key: (u64, u64),
    /// Récapitulative (lu à l'ouverture, non éditable) : durée, %, dates,
    /// type de tâche et pilotage grisés.
    pub summary: bool,
    /// Onglet Général.
    pub general: General,
    /// Prédécesseurs, dans l'ordre du modèle (liens pendants omis).
    pub links: Vec<LinkRow>,
    /// Affectations, dans l'ordre du projet (sans ressource omises).
    pub assignments: Vec<AssignRow>,
    /// Onglet Avancé.
    pub advanced: Advanced,
    /// Notes.
    pub notes: String,
}

/// Refus d'un champ, affiché dans son onglet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    /// Onglet du champ.
    pub tab: Tab,
    /// Champ (`name`, `duration`, `percent`, `start`, `finish`,
    /// `predecessor`, `lag`, `resource`, `units`, `constraint_date`,
    /// `deadline`, `task_type`, `effort_driven`, `notes`, `active`) ; vide
    /// si le refus ne désigne aucun champ connu.
    pub field: &'static str,
    /// Ligne de l'onglet Prédécesseurs ou Ressources, s'il y a lieu.
    pub row: Option<usize>,
    /// Message traduit.
    pub message: String,
}

/// Langue de l'interface pour une langue des textes du moteur.
const fn ui_language(lang: Language) -> i18n::Language {
    match lang {
        Language::Fr => i18n::Language::Fr,
        Language::En => i18n::Language::En,
    }
}

/// Pourcentage en centièmes de % (« 5000 » → « 50% »).
fn percent_text(hundredths: i64, ctx: &TextContext) -> String {
    let mut out = String::new();
    let value = Decimal::new(hundredths, 2).unwrap_or(Decimal::integer(0));
    write_decimal(&mut out, value, decimal_sep(ctx));
    out.push('%');
    out
}

/// Séparateur décimal du contexte.
const fn decimal_sep(ctx: &TextContext) -> char {
    match ctx.decimal {
        planrs_automation::text::DecimalMark::Comma => ',',
        planrs_automation::text::DecimalMark::Dot => '.',
    }
}

/// Texte des unités d'une affectation selon le type de la ressource :
/// pourcentage (travail), quantité (matériel), vide (coût).
pub(crate) fn units_text(kind: Option<&ResourceKind>, units: i64, ctx: &TextContext) -> String {
    match kind {
        Some(ResourceKind::Material { .. }) => {
            let mut out = String::new();
            let value = Decimal::new(units, 4).unwrap_or(Decimal::integer(0));
            write_decimal(&mut out, value, decimal_sep(ctx));
            out
        }
        Some(ResourceKind::Cost) => String::new(),
        Some(ResourceKind::Work) | None => percent_text(units, ctx),
    }
}

/// Rôle de lecture d'une date de contrainte : début pour les contraintes
/// de début, fin pour les contraintes de fin (R-143).
const fn constraint_role(kind: ConstraintKind) -> DateRole {
    match kind {
        ConstraintKind::MustFinishOn
        | ConstraintKind::FinishNoEarlierThan
        | ConstraintKind::FinishNoLaterThan => DateRole::Finish,
        _ => DateRole::Start,
    }
}

impl TaskInfoDraft {
    /// Brouillon de la tâche `uid` : textes écrits dans la langue `lang`
    /// (celle de l'interface) ; `None` si la tâche n'existe pas ou est une
    /// ligne vide MSPDI.
    #[must_use]
    pub fn from_session(s: &Session, uid: TaskUid, key: (u64, u64), lang: Language) -> Option<Self> {
        let project = s.project();
        let task = project.tasks.get(uid)?;
        if task.is_null {
            return None;
        }
        let ctx = TextContext::for_project(project, lang);
        let outcome = s.schedule().tasks.get(&uid);
        let (dates, summary) = match outcome {
            Some(TaskOutcome::Scheduled(sched)) => {
                (Some((sched.start, sched.finish)), sched.role == TaskRole::Summary)
            }
            Some(TaskOutcome::Excluded { display, .. }) => (display.map(|d| (d.start, d.finish)), false),
            None => (None, false),
        };
        let percent = match outcome {
            Some(TaskOutcome::Scheduled(sched)) => sched
                .progress
                .map_or(task.progress.percent_complete, |p| p.percent_complete),
            _ => task.progress.percent_complete,
        };
        let general = General {
            name: task.name.clone(),
            duration: format_duration(&task.duration, &ctx),
            percent: percent_text(percent.get(), &ctx),
            start: dates
                .map(|(start, _)| format_date(start, &ctx))
                .unwrap_or_default(),
            finish: dates
                .map(|(_, finish)| format_date(finish, &ctx))
                .unwrap_or_default(),
            active: task.active,
        };
        let links = task
            .predecessors
            .iter()
            .filter_map(|link| {
                let id = project.task_id(link.predecessor)?;
                Some(LinkRow {
                    predecessor: id.get().to_string(),
                    kind: link.kind,
                    lag: if is_zero_lag(&link.lag) {
                        String::new()
                    } else {
                        format_lag(&link.lag, &ctx)
                    },
                })
            })
            .collect();
        let assignments = project
            .assignments
            .iter()
            .filter(|a| a.task == uid)
            .filter_map(|a| {
                let resource = a.resource?;
                let kind = project.resources.get(resource).map(|r| &r.kind);
                Some(AssignRow {
                    resource,
                    units: units_text(kind, a.units.get(), &ctx),
                })
            })
            .collect();
        let constraint = &task.constraint;
        let advanced = Advanced {
            constraint: constraint.kind(),
            constraint_date: constraint
                .date()
                .map(|d| format_date(d, &ctx))
                .unwrap_or_default(),
            deadline: task.deadline.map(|d| format_date(d, &ctx)).unwrap_or_default(),
            task_type: task.task_type,
            effort_driven: task.effort_driven,
        };
        Some(Self {
            uid,
            key,
            summary,
            general,
            links,
            assignments,
            advanced,
            notes: task.notes.clone(),
        })
    }

    /// Lot de commandes du brouillon (fonction pure) : seuls les champs dont
    /// le texte a changé depuis l'ouverture (brouillon relu sur `s`) sont
    /// envoyés, dans l'ordre du module. Rien de changé (ou tâche disparue)
    /// ⇒ `Ok(None)`. Le lot porte la langue `lang` (celle des textes).
    ///
    /// # Errors
    ///
    /// Tous les champs illisibles (texte, ID inconnu, doublon, date de
    /// contrainte manquante), chacun avec son onglet.
    pub fn to_batch(&self, s: &Session, lang: Language) -> Result<Option<Batch>, Vec<FieldError>> {
        let Some(old) = Self::from_session(s, self.uid, self.key, lang) else {
            return Ok(None);
        };
        let project = s.project();
        let ctx = TextContext::for_project(project, lang);
        let texts = Texts::new(ui_language(lang));
        let mut errors = Vec::new();
        let mut ops = Vec::new();
        let uid = self.uid;

        // task.update : Général, type et pilotage (Avancé), notes.
        let mut update = empty_update(uid);
        let g = &self.general;
        if g.name != old.general.name {
            update.name = Patch::Set(g.name.clone());
        }
        if g.duration != old.general.duration {
            match parse_duration(&g.duration, &ctx) {
                Ok(_) => {
                    update.duration = Patch::Set(DurationJson {
                        text: Some(g.duration.clone()),
                        ..DurationJson::default()
                    });
                }
                Err(e) => errors.push(text_error(&texts, Tab::General, "duration", None, e)),
            }
        }
        for (field, text, old_text, role) in [
            ("start", &g.start, &old.general.start, DateRole::Start),
            ("finish", &g.finish, &old.general.finish, DateRole::Finish),
        ] {
            if text == old_text {
                continue;
            }
            match parse_date(text, role, &ctx) {
                Ok(m) if field == "start" => update.start = Patch::Set(m),
                Ok(m) => update.finish = Patch::Set(m),
                Err(e) => errors.push(text_error(&texts, Tab::General, field, None, e)),
            }
        }
        if g.active != old.general.active {
            update.active = Patch::Set(g.active);
        }
        let a = &self.advanced;
        if a.task_type != old.advanced.task_type {
            update.task_type = Patch::Set(TaskTypeJson::from(a.task_type));
        }
        if a.effort_driven != old.advanced.effort_driven {
            update.effort_driven = Patch::Set(a.effort_driven);
        }
        if self.notes != old.notes {
            update.notes = Patch::Set(self.notes.clone());
        }
        if update != empty_update(uid) {
            ops.push(Command::TaskUpdate(update));
        }

        self.link_ops(&old, s, &ctx, &texts, &mut ops, &mut errors);
        self.assignment_ops(&old, s, &ctx, &texts, &mut ops, &mut errors);

        self.advanced_ops(&old, &ctx, &texts, &mut ops, &mut errors);

        // progress.update (dernier : l'avancement voit la durée nouvelle).
        if g.percent != old.general.percent {
            match parse_percent(&g.percent, &ctx) {
                Ok(_) => ops.push(Command::ProgressUpdate(percent_update(uid, &g.percent))),
                Err(e) => errors.push(text_error(&texts, Tab::General, "percent", None, e)),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }
        if ops.is_empty() {
            return Ok(None);
        }
        Ok(Some(Batch {
            format_version: Batch::FORMAT_VERSION,
            lang: Some(lang),
            ops,
            label: None,
        }))
    }

    /// `task.set_constraint` et `task.set_deadline` de l'onglet Avancé.
    fn advanced_ops(
        &self,
        old: &Self,
        ctx: &TextContext,
        texts: &Texts,
        ops: &mut Vec<Command>,
        errors: &mut Vec<FieldError>,
    ) {
        let a = &self.advanced;
        let uid = self.uid;
        // task.set_constraint.
        let constraint_changed = a.constraint != old.advanced.constraint
            || (a.constraint.is_dated() && a.constraint_date != old.advanced.constraint_date);
        if constraint_changed {
            let date = if a.constraint.is_dated() {
                if a.constraint_date.trim().is_empty() {
                    errors.push(FieldError {
                        tab: Tab::Advanced,
                        field: "constraint_date",
                        row: None,
                        message: texts.get(TextId::TaskInfoDateRequired),
                    });
                    None
                } else {
                    match parse_date(&a.constraint_date, constraint_role(a.constraint), ctx) {
                        Ok(m) => Some(m),
                        Err(e) => {
                            errors.push(text_error(texts, Tab::Advanced, "constraint_date", None, e));
                            None
                        }
                    }
                }
            } else {
                None
            };
            ops.push(Command::TaskSetConstraint(SetConstraint {
                uid: TaskRef::Uid(uid),
                kind: ConstraintKindJson::from(a.constraint),
                date,
            }));
        }

        // task.set_deadline.
        if a.deadline != old.advanced.deadline {
            let deadline = if a.deadline.trim().is_empty() {
                Some(Patch::Clear)
            } else {
                match parse_date(&a.deadline, DateRole::Finish, ctx) {
                    Ok(m) => Some(Patch::Set(m)),
                    Err(e) => {
                        errors.push(text_error(texts, Tab::Advanced, "deadline", None, e));
                        None
                    }
                }
            };
            if let Some(deadline) = deadline {
                ops.push(Command::TaskSetDeadline(SetDeadline {
                    uid: TaskRef::Uid(uid),
                    deadline,
                }));
            }
        }
    }

    /// `link.add`, `link.update`, `link.remove` de l'onglet Prédécesseurs.
    fn link_ops(
        &self,
        old: &Self,
        s: &Session,
        ctx: &TextContext,
        texts: &Texts,
        ops: &mut Vec<Command>,
        errors: &mut Vec<FieldError>,
    ) {
        let project = s.project();
        let Some(task) = project.tasks.get(self.uid) else {
            return;
        };
        // Lignes d'origine par UID du prédécesseur.
        let old_rows: BTreeMap<TaskUid, &LinkRow> = old
            .links
            .iter()
            .filter_map(|row| resolve_id(&row.predecessor, s).map(|uid| (uid, row)))
            .collect();
        let mut seen: BTreeMap<TaskUid, usize> = BTreeMap::new();
        let (mut adds, mut updates) = (Vec::new(), Vec::new());
        for (i, row) in self.links.iter().enumerate() {
            let id_text = row.predecessor.trim();
            if id_text.is_empty() {
                continue;
            }
            let Some(pred) = resolve_id(id_text, s) else {
                let message = id_text.parse::<u32>().map_or_else(
                    |_| {
                        texts.get_args(
                            TextId::TaskInfoBadId,
                            &[("id", Arg::Str(id_text.to_owned().into()))],
                        )
                    },
                    |id| EditError::UnknownId(id).message(texts),
                );
                errors.push(FieldError {
                    tab: Tab::Predecessors,
                    field: "predecessor",
                    row: Some(i),
                    message,
                });
                continue;
            };
            if seen.insert(pred, i).is_some() {
                errors.push(FieldError {
                    tab: Tab::Predecessors,
                    field: "predecessor",
                    row: Some(i),
                    message: texts.get_args(
                        TextId::TaskInfoDuplicatePredecessor,
                        &[("id", Arg::Str(id_text.to_owned().into()))],
                    ),
                });
                continue;
            }
            let previous = old_rows.get(&pred);
            let lag_changed = previous.is_none_or(|p| p.lag.trim() != row.lag.trim());
            let lag = if lag_changed && !row.lag.trim().is_empty() {
                match parse_lag(&row.lag, ctx) {
                    Ok(lag) => Some(LagJson {
                        text: None,
                        ..LagJson::from_lag(&lag, ctx)
                    }),
                    Err(e) => {
                        errors.push(text_error(texts, Tab::Predecessors, "lag", Some(i), e));
                        continue;
                    }
                }
            } else if lag_changed && previous.is_some() {
                // Décalage effacé : zéro explicite.
                Some(LagJson {
                    text: None,
                    ..LagJson::from_lag(&planrs_core::Lag::ZERO, ctx)
                })
            } else {
                None
            };
            let spec = |kind: Option<LinkTypeJson>, lag: Option<LagJson>| LinkSpec {
                pred: TaskRef::Uid(pred),
                succ: TaskRef::Uid(self.uid),
                kind,
                lag,
            };
            match previous {
                None => adds.push(Command::LinkAdd(spec(Some(LinkTypeJson::from(row.kind)), lag))),
                Some(p) => {
                    let kind = (p.kind != row.kind).then(|| LinkTypeJson::from(row.kind));
                    if kind.is_some() || lag.is_some() {
                        updates.push(Command::LinkUpdate(spec(kind, lag)));
                    }
                }
            }
        }
        ops.extend(adds);
        ops.extend(updates);
        for pred in old_rows.keys() {
            if !seen.contains_key(pred) && task.predecessors.iter().any(|link| link.predecessor == *pred) {
                ops.push(Command::LinkRemove(LinkKey {
                    pred: TaskRef::Uid(*pred),
                    succ: TaskRef::Uid(self.uid),
                }));
            }
        }
    }

    /// `assignment.add`, `assignment.update`, `assignment.remove` de
    /// l'onglet Ressources.
    fn assignment_ops(
        &self,
        old: &Self,
        s: &Session,
        ctx: &TextContext,
        texts: &Texts,
        ops: &mut Vec<Command>,
        errors: &mut Vec<FieldError>,
    ) {
        let project = s.project();
        let old_rows: BTreeMap<ResourceUid, &AssignRow> =
            old.assignments.iter().map(|row| (row.resource, row)).collect();
        let mut seen: BTreeMap<ResourceUid, usize> = BTreeMap::new();
        let (mut adds, mut updates) = (Vec::new(), Vec::new());
        for (i, row) in self.assignments.iter().enumerate() {
            let Some(resource) = project.resources.get(row.resource) else {
                errors.push(FieldError {
                    tab: Tab::Resources,
                    field: "resource",
                    row: Some(i),
                    message: texts.get(TextId::TaskInfoUnknownResource),
                });
                continue;
            };
            if seen.insert(row.resource, i).is_some() {
                errors.push(FieldError {
                    tab: Tab::Resources,
                    field: "resource",
                    row: Some(i),
                    message: texts.get_args(
                        TextId::TaskInfoDuplicateResource,
                        &[("name", Arg::Str(resource.name.clone().into()))],
                    ),
                });
                continue;
            }
            let previous = old_rows.get(&row.resource);
            let changed = previous.is_none_or(|p| p.units.trim() != row.units.trim());
            // Unités (travail) ou quantité (matériel), relues pour signaler
            // l'erreur sur la ligne ; vides : valeur par défaut du registre.
            let mut units = None;
            let mut quantity = None;
            if changed && !row.units.trim().is_empty() {
                match &resource.kind {
                    ResourceKind::Work => {
                        let json = units_json(&row.units, true);
                        match json.to_units(ctx) {
                            Ok(_) => units = Some(json),
                            Err(e) => {
                                errors.push(command_error(texts, Tab::Resources, "units", Some(i), &e, ctx));
                                continue;
                            }
                        }
                    }
                    ResourceKind::Material { .. } => {
                        let json = units_json(&row.units, false);
                        match json.to_quantity(ctx) {
                            Ok(_) => quantity = Some(json),
                            Err(e) => {
                                errors.push(command_error(texts, Tab::Resources, "units", Some(i), &e, ctx));
                                continue;
                            }
                        }
                    }
                    ResourceKind::Cost => {}
                }
            }
            match previous {
                None => adds.push(Command::AssignmentAdd(AssignmentAdd {
                    task: TaskRef::Uid(self.uid),
                    resource: ResourceRef::Uid(row.resource),
                    units,
                    work: None,
                    quantity,
                    delay: None,
                })),
                Some(_) if units.is_some() || quantity.is_some() => {
                    updates.push(Command::AssignmentUpdate(AssignmentUpdate {
                        task: TaskRef::Uid(self.uid),
                        resource: ResourceRef::Uid(row.resource),
                        units: units.map_or(Patch::Keep, Patch::Set),
                        work: Patch::Keep,
                        quantity: quantity.map_or(Patch::Keep, Patch::Set),
                        delay: Patch::Keep,
                    }));
                }
                Some(_) => {}
            }
        }
        ops.extend(adds);
        ops.extend(updates);
        for resource in old_rows.keys() {
            if !seen.contains_key(resource) {
                ops.push(Command::AssignmentRemove(AssignmentKey {
                    task: TaskRef::Uid(self.uid),
                    resource: ResourceRef::Uid(*resource),
                }));
            }
        }
    }
}

/// Décalage nul (temps ou pourcentage), quelle que soit son unité.
fn is_zero_lag(lag: &planrs_core::Lag) -> bool {
    match lag.value {
        planrs_core::LagValue::Time { minutes, .. } => minutes == planrs_core::Minutes::ZERO,
        planrs_core::LagValue::Percent { percent } => percent.get() == 0,
    }
}

/// R-150 côté interface : ID (position + 1, texte) → UID.
fn resolve_id(text: &str, s: &Session) -> Option<TaskUid> {
    let id = text.trim().parse::<usize>().ok()?;
    let task = s.project().tasks.at(id.checked_sub(1)?)?;
    (!task.is_null).then(|| planrs_core::HasUid::uid(task))
}

/// Unités en texte : `%` ajouté pour une ressource de travail s'il manque
/// (« 50 » vaut 50 %, comme la colonne % achevé).
pub(crate) fn units_json(text: &str, percent: bool) -> UnitsJson {
    let trimmed = text.trim();
    let text = if percent && !trimmed.ends_with('%') {
        format!("{trimmed}%")
    } else {
        trimmed.to_owned()
    };
    UnitsJson {
        value: None,
        text: Some(text),
    }
}

/// `task.update` vide sur `uid`.
const fn empty_update(uid: TaskUid) -> TaskUpdate {
    TaskUpdate {
        uid: TaskRef::Uid(uid),
        name: Patch::Keep,
        duration: Patch::Keep,
        milestone: Patch::Keep,
        notes: Patch::Keep,
        start: Patch::Keep,
        finish: Patch::Keep,
        active: Patch::Keep,
        task_type: Patch::Keep,
        effort_driven: Patch::Keep,
        work: Patch::Keep,
        mode: Patch::Keep,
    }
}

/// `progress.update { percent_complete: { text } }`, `%` ajouté s'il
/// manque (le registre l'exige ; « 50 » se lit « 50% »).
fn percent_update(uid: TaskUid, text: &str) -> ProgressUpdate {
    let trimmed = text.trim();
    let text = if trimmed.ends_with('%') {
        trimmed.to_owned()
    } else {
        format!("{trimmed}%")
    };
    ProgressUpdate {
        task: TaskRef::Uid(uid),
        percent_complete: Some(PercentJson {
            value: None,
            text: Some(text),
        }),
        actual_start: None,
        actual_finish: None,
        actual_duration: None,
        remaining_duration: None,
        percent_work_complete: None,
        actual_work: None,
        physical_percent_complete: None,
    }
}

/// Refus d'un texte (mêmes messages que sous une cellule de la table).
fn text_error(texts: &Texts, tab: Tab, field: &'static str, row: Option<usize>, e: TextError) -> FieldError {
    FieldError {
        tab,
        field,
        row,
        message: EditError::from(e).message(texts),
    }
}

/// Refus d'une valeur lue par le registre (unités).
fn command_error(
    texts: &Texts,
    tab: Tab,
    field: &'static str,
    row: Option<usize>,
    e: &CommandError,
    ctx: &TextContext,
) -> FieldError {
    FieldError {
        tab,
        field,
        row,
        message: command_message(texts, e, ctx.language),
    }
}

/// Message traduit d'un refus du registre (comme le bandeau).
#[must_use]
pub fn command_message(texts: &Texts, e: &CommandError, lang: Language) -> String {
    let detail = e.message(lang);
    texts.get_args(error_text_id(e.code), &[("detail", Arg::Str(detail.into()))])
}

/// Onglet et champ d'un refus du registre sur le lot `batch` : opération
/// fautive (`op_index`) et champ (`field`) ; à défaut, Général sans champ.
#[must_use]
pub fn locate(batch: &Batch, error: &CommandError) -> (Tab, &'static str) {
    let op = error.op_index.and_then(|i| batch.ops.get(i));
    let field = error.field.as_deref().unwrap_or_default();
    match op {
        Some(Command::TaskUpdate(_)) => match field {
            "name" => (Tab::General, "name"),
            "duration" | "work" | "milestone" => (Tab::General, "duration"),
            "start" => (Tab::General, "start"),
            "finish" => (Tab::General, "finish"),
            "active" => (Tab::General, "active"),
            "type" => (Tab::Advanced, "task_type"),
            "effort_driven" => (Tab::Advanced, "effort_driven"),
            "notes" => (Tab::Notes, "notes"),
            _ => (Tab::General, ""),
        },
        Some(Command::LinkAdd(_) | Command::LinkUpdate(_)) => {
            if field == "lag" {
                (Tab::Predecessors, "lag")
            } else {
                (Tab::Predecessors, "predecessor")
            }
        }
        Some(Command::LinkRemove(_)) => (Tab::Predecessors, "predecessor"),
        Some(Command::AssignmentAdd(_) | Command::AssignmentUpdate(_) | Command::AssignmentRemove(_)) => {
            if field == "units" || field == "quantity" {
                (Tab::Resources, "units")
            } else {
                (Tab::Resources, "resource")
            }
        }
        Some(Command::TaskSetConstraint(_)) => (Tab::Advanced, "constraint_date"),
        Some(Command::TaskSetDeadline(_)) => (Tab::Advanced, "deadline"),
        Some(Command::ProgressUpdate(_)) => (Tab::General, "percent"),
        _ => (Tab::General, ""),
    }
}

/// Type de lien et son libellé (FD, DD, FF, DF).
pub const LINK_TYPES: [(LinkType, TextId); 4] = [
    (LinkType::FinishToStart, TextId::TaskInfoLinkFs),
    (LinkType::StartToStart, TextId::TaskInfoLinkSs),
    (LinkType::FinishToFinish, TextId::TaskInfoLinkFf),
    (LinkType::StartToFinish, TextId::TaskInfoLinkSf),
];

/// Types de contrainte et leur libellé (S66).
pub const CONSTRAINTS: [(ConstraintKind, TextId); 8] = [
    (ConstraintKind::AsSoonAsPossible, TextId::TaskInfoConstraintAsap),
    (ConstraintKind::AsLateAsPossible, TextId::TaskInfoConstraintAlap),
    (ConstraintKind::MustStartOn, TextId::TaskInfoConstraintMso),
    (ConstraintKind::MustFinishOn, TextId::TaskInfoConstraintMfo),
    (ConstraintKind::StartNoEarlierThan, TextId::TaskInfoConstraintSnet),
    (ConstraintKind::StartNoLaterThan, TextId::TaskInfoConstraintSnlt),
    (
        ConstraintKind::FinishNoEarlierThan,
        TextId::TaskInfoConstraintFnet,
    ),
    (ConstraintKind::FinishNoLaterThan, TextId::TaskInfoConstraintFnlt),
];

/// Types de tâche et leur libellé.
pub const TASK_TYPES: [(TaskType, TextId); 3] = [
    (TaskType::FixedUnits, TextId::TaskInfoTypeFixedUnits),
    (TaskType::FixedDuration, TextId::TaskInfoTypeFixedDuration),
    (TaskType::FixedWork, TextId::TaskInfoTypeFixedWork),
];

// --- Fenêtre (egui) -------------------------------------------------------

/// Ressource proposée dans l'onglet Ressources (relevée à l'ouverture).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceChoice {
    /// UID.
    pub uid: ResourceUid,
    /// Nom affiché.
    pub name: String,
}

/// État de la fenêtre ouverte : brouillon, onglet, refus affichés.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskInfoState {
    /// Brouillon.
    pub draft: TaskInfoDraft,
    /// ID de la tâche à l'ouverture (titre).
    pub id: u32,
    /// Nom de la tâche à l'ouverture (titre).
    pub name: String,
    /// Onglet affiché.
    pub tab: Tab,
    /// Refus de la dernière tentative (champs, ou registre).
    pub errors: Vec<FieldError>,
    /// Le projet avait changé au dernier « OK » : brouillon relu, rien
    /// appliqué.
    pub stale: bool,
    /// Ressources du projet, dans l'ordre du projet.
    pub resources: Vec<ResourceChoice>,
}

impl TaskInfoState {
    /// État d'ouverture de la tâche `uid` (`None` : tâche absente).
    #[must_use]
    pub fn open(s: &Session, uid: TaskUid, key: (u64, u64), lang: Language) -> Option<Self> {
        let draft = TaskInfoDraft::from_session(s, uid, key, lang)?;
        let project = s.project();
        let id = project.task_id(uid).map_or(0, planrs_core::TaskId::get);
        let resources = project
            .resources
            .iter()
            .map(|r| ResourceChoice {
                uid: planrs_core::HasUid::uid(r),
                name: r.name.clone(),
            })
            .collect();
        Some(Self {
            name: draft.general.name.clone(),
            draft,
            id,
            tab: Tab::General,
            errors: Vec::new(),
            stale: false,
            resources,
        })
    }

    /// Refus du champ `field` (ligne `row`) de l'onglet courant ou donné.
    fn error_for(&self, tab: Tab, field: &str, row: Option<usize>) -> Option<&str> {
        self.errors
            .iter()
            .find(|e| e.tab == tab && e.field == field && e.row == row)
            .map(|e| e.message.as_str())
    }
}

/// Onglet de « Informations sur la tâche » (nom du contrat ADR-0017).
pub type TaskInfoTab = Tab;

/// Ouvre « Informations sur la tâche » de `uid` sur l'onglet `tab`
/// (Tâche → Notes du ruban, H-UI-84) ; `None` : tâche absente.
///
/// Lot L0 (squelette) : l'onglet Général reste affiché ; l'onglet demandé
/// est servi en L3.
#[must_use]
pub fn open_on(
    tab: TaskInfoTab,
    s: &Session,
    uid: TaskUid,
    key: (u64, u64),
    lang: Language,
) -> Option<TaskInfoState> {
    let _ = tab;
    TaskInfoState::open(s, uid, key, lang)
}

/// Réponse de la fenêtre pour une image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Rien : la fenêtre reste ouverte.
    Open,
    /// « OK ».
    Ok,
    /// « Annuler » ou Échap.
    Cancel,
}

/// Identifiant de la fenêtre (couche modale d'egui).
#[must_use]
pub fn modal_id() -> egui::Id {
    egui::Id::unique("planrs.task-info")
}

/// Largeur des champs de texte.
const FIELD_W: f32 = 260.0;

/// Champ de texte d'une ligne, nommé `label` pour l'accessibilité ; grisé
/// si `enabled` est faux.
fn text_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    enabled: bool,
    width: f32,
) -> egui::Response {
    let response = ui.add_enabled(enabled, egui::TextEdit::singleline(value).desired_width(width));
    let name = label.to_owned();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, enabled, name.clone()));
    response
}

/// Message de refus (couleur d'erreur), s'il y en a un.
fn error_line(ui: &mut egui::Ui, message: Option<&str>) {
    if let Some(message) = message {
        ui.colored_label(ui.visuals().error_fg_color, message);
    }
}

/// Liste déroulante de `choices`, nommée `label`.
fn combo<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    label: &str,
    texts: &Texts,
    value: &mut T,
    choices: &[(T, TextId)],
    enabled: bool,
) {
    let selected = choices
        .iter()
        .find(|(v, _)| *v == *value)
        .map(|(_, t)| texts.get(*t))
        .unwrap_or_default();
    ui.add_enabled_ui(enabled, |ui| {
        let response = egui::ComboBox::from_id_salt(salt)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for (v, t) in choices {
                    ui.selectable_value(value, *v, texts.get(*t));
                }
            })
            .response;
        let name = label.to_owned();
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, enabled, name.clone()));
    });
}

/// Dessine la fenêtre modale (une image) et rend la réponse de
/// l'utilisateur. Échap ferme (sauf liste déroulante ouverte, qu'il ferme
/// d'abord) ; un clic hors de la fenêtre ne la ferme pas (brouillon gardé).
pub fn show(ctx: &egui::Context, texts: &Texts, state: &mut TaskInfoState) -> Outcome {
    let mut outcome = Outcome::Open;
    let armed = super::enter_armed(ctx, Some(notes_id()));
    let response = egui::Modal::new(modal_id()).show(ctx, |ui| {
        ui.set_width(560.0);
        let title = texts.get_args(
            TextId::TaskInfoTitle,
            &[
                ("id", Arg::Count(i64::from(state.id))),
                ("name", Arg::Str(state.name.clone().into())),
            ],
        );
        ui.heading(title);
        if state.stale {
            ui.colored_label(ui.visuals().warn_fg_color, texts.get(TextId::TaskInfoStale));
        }
        ui.horizontal(|ui| {
            for tab in Tab::ALL {
                let failed = state.errors.iter().any(|e| e.tab == tab);
                let mut text = egui::RichText::new(texts.get(tab.title()));
                if failed {
                    text = text.color(ui.visuals().error_fg_color);
                }
                if ui.selectable_label(state.tab == tab, text).clicked() {
                    state.tab = tab;
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| match state.tab {
                Tab::General => general_tab(ui, texts, state),
                Tab::Predecessors => predecessors_tab(ui, texts, state),
                Tab::Resources => resources_tab(ui, texts, state),
                Tab::Advanced => advanced_tab(ui, texts, state),
                Tab::Notes => notes_tab(ui, texts, state),
            });
        // Refus sans champ connu (registre) : sous les onglets.
        for e in state.errors.iter().filter(|e| e.field.is_empty()) {
            ui.colored_label(ui.visuals().error_fg_color, e.message.as_str());
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(texts.get(TextId::TaskInfoOk)).clicked() {
                outcome = Outcome::Ok;
            }
            if ui.button(texts.get(TextId::TaskInfoCancel)).clicked() {
                outcome = Outcome::Cancel;
            }
        });
    });
    let escape = response.is_top_modal
        && !response.any_popup_open
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    if escape {
        outcome = Outcome::Cancel;
    }
    if outcome == Outcome::Open
        && super::enter_pressed(ctx, armed, response.is_top_modal, response.any_popup_open)
    {
        outcome = Outcome::Ok;
    }
    outcome
}

/// Onglet Général.
fn general_tab(ui: &mut egui::Ui, texts: &Texts, state: &mut TaskInfoState) {
    let editable = !state.draft.summary;
    let errors: Vec<(&'static str, Option<String>)> = ["name", "duration", "percent", "start", "finish"]
        .into_iter()
        .map(|f| (f, state.error_for(Tab::General, f, None).map(str::to_owned)))
        .collect();
    let message = |field: &str| {
        errors
            .iter()
            .find(|(f, _)| *f == field)
            .and_then(|(_, m)| m.clone())
    };
    let g = &mut state.draft.general;
    egui::Grid::new("planrs.task-info.general")
        .num_columns(2)
        .show(ui, |ui| {
            for (field, label, value, enabled) in [
                ("name", TextId::TaskInfoName, &mut g.name, true),
                ("duration", TextId::TaskInfoDuration, &mut g.duration, editable),
                ("percent", TextId::TaskInfoPercent, &mut g.percent, editable),
                ("start", TextId::TaskInfoStart, &mut g.start, editable),
                ("finish", TextId::TaskInfoFinish, &mut g.finish, editable),
            ] {
                let label = texts.get(label);
                ui.label(&label);
                ui.vertical(|ui| {
                    text_field(ui, &label, value, enabled, FIELD_W);
                    error_line(ui, message(field).as_deref());
                });
                ui.end_row();
            }
            ui.allocate_space(egui::Vec2::ZERO);
            ui.checkbox(&mut g.active, texts.get(TextId::TaskInfoActive));
            ui.end_row();
        });
}

/// Onglet Prédécesseurs (S67) : ID, nom (lu), type, décalage.
fn predecessors_tab(ui: &mut egui::Ui, texts: &Texts, state: &mut TaskInfoState) {
    let errors = state.errors.clone();
    let mut remove = None;
    let id_label = texts.get(TextId::TaskInfoPredId);
    let type_label = texts.get(TextId::TaskInfoPredType);
    let lag_label = texts.get(TextId::TaskInfoPredLag);
    egui::Grid::new("planrs.task-info.links")
        .num_columns(4)
        .show(ui, |ui| {
            ui.strong(&id_label);
            ui.strong(&type_label);
            ui.strong(&lag_label);
            ui.allocate_space(egui::Vec2::ZERO);
            ui.end_row();
            for (i, row) in state.draft.links.iter_mut().enumerate() {
                let n = i.saturating_add(1);
                let named = |label: &str| {
                    texts.get_args(
                        TextId::TaskInfoRowField,
                        &[
                            ("column", Arg::Str(label.to_owned().into())),
                            ("count", Arg::Count(i64::try_from(n).unwrap_or(i64::MAX))),
                        ],
                    )
                };
                ui.vertical(|ui| {
                    text_field(ui, &named(&id_label), &mut row.predecessor, true, 60.0);
                    error_line(ui, find(&errors, Tab::Predecessors, "predecessor", Some(i)));
                });
                combo(
                    ui,
                    ("planrs.task-info.link-type", i),
                    &named(&type_label),
                    texts,
                    &mut row.kind,
                    &LINK_TYPES,
                    true,
                );
                ui.vertical(|ui| {
                    text_field(ui, &named(&lag_label), &mut row.lag, true, 90.0);
                    error_line(ui, find(&errors, Tab::Predecessors, "lag", Some(i)));
                });
                if ui.button(texts.get(TextId::TaskInfoRemoveRow)).clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove
        && i < state.draft.links.len()
    {
        state.draft.links.remove(i);
        state.errors.retain(|e| e.tab != Tab::Predecessors);
    }
    if ui.button(texts.get(TextId::TaskInfoAddPredecessor)).clicked() {
        state.draft.links.push(LinkRow::blank());
    }
}

/// Refus de `field` (ligne `row`) d'un onglet, dans une liste copiée.
fn find<'a>(errors: &'a [FieldError], tab: Tab, field: &str, row: Option<usize>) -> Option<&'a str> {
    errors
        .iter()
        .find(|e| e.tab == tab && e.field == field && e.row == row)
        .map(|e| e.message.as_str())
}

/// Onglet Ressources : ressource, unités.
fn resources_tab(ui: &mut egui::Ui, texts: &Texts, state: &mut TaskInfoState) {
    if state.resources.is_empty() {
        ui.label(texts.get(TextId::TaskInfoNoResource));
        return;
    }
    let errors = state.errors.clone();
    let resources = state.resources.clone();
    let mut remove = None;
    let name_label = texts.get(TextId::TaskInfoResourceName);
    let units_label = texts.get(TextId::TaskInfoResourceUnits);
    egui::Grid::new("planrs.task-info.assignments")
        .num_columns(3)
        .show(ui, |ui| {
            ui.strong(&name_label);
            ui.strong(&units_label);
            ui.allocate_space(egui::Vec2::ZERO);
            ui.end_row();
            for (i, row) in state.draft.assignments.iter_mut().enumerate() {
                let n = i64::try_from(i.saturating_add(1)).unwrap_or(i64::MAX);
                let named = |label: &str| {
                    texts.get_args(
                        TextId::TaskInfoRowField,
                        &[
                            ("column", Arg::Str(label.to_owned().into())),
                            ("count", Arg::Count(n)),
                        ],
                    )
                };
                let current = resources
                    .iter()
                    .find(|r| r.uid == row.resource)
                    .map(|r| r.name.clone())
                    .unwrap_or_default();
                ui.vertical(|ui| {
                    let response = egui::ComboBox::from_id_salt(("planrs.task-info.resource", i))
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for r in &resources {
                                ui.selectable_value(&mut row.resource, r.uid, &r.name);
                            }
                        })
                        .response;
                    let name = named(&name_label);
                    response
                        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, name.clone()));
                    error_line(ui, find(&errors, Tab::Resources, "resource", Some(i)));
                });
                ui.vertical(|ui| {
                    text_field(ui, &named(&units_label), &mut row.units, true, 90.0);
                    error_line(ui, find(&errors, Tab::Resources, "units", Some(i)));
                });
                if ui.button(texts.get(TextId::TaskInfoRemoveRow)).clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove
        && i < state.draft.assignments.len()
    {
        state.draft.assignments.remove(i);
        state.errors.retain(|e| e.tab != Tab::Resources);
    }
    // « Ajouter » : première ressource pas encore affectée.
    let free = resources
        .iter()
        .find(|r| !state.draft.assignments.iter().any(|a| a.resource == r.uid));
    if ui
        .add_enabled(
            free.is_some(),
            egui::Button::new(texts.get(TextId::TaskInfoAddResource)),
        )
        .clicked()
        && let Some(r) = free
    {
        state.draft.assignments.push(AssignRow {
            resource: r.uid,
            units: String::new(),
        });
    }
}

/// Onglet Avancé (S66) : contrainte, échéance, type, pilotage.
fn advanced_tab(ui: &mut egui::Ui, texts: &Texts, state: &mut TaskInfoState) {
    let editable = !state.draft.summary;
    let date_error = state
        .error_for(Tab::Advanced, "constraint_date", None)
        .map(str::to_owned);
    let deadline_error = state
        .error_for(Tab::Advanced, "deadline", None)
        .map(str::to_owned);
    let a = &mut state.draft.advanced;
    egui::Grid::new("planrs.task-info.advanced")
        .num_columns(2)
        .show(ui, |ui| {
            let label = texts.get(TextId::TaskInfoConstraint);
            ui.label(&label);
            combo(
                ui,
                "planrs.task-info.constraint",
                &label,
                texts,
                &mut a.constraint,
                &CONSTRAINTS,
                true,
            );
            ui.end_row();
            let label = texts.get(TextId::TaskInfoConstraintDate);
            ui.label(&label);
            ui.vertical(|ui| {
                text_field(
                    ui,
                    &label,
                    &mut a.constraint_date,
                    a.constraint.is_dated(),
                    FIELD_W,
                );
                error_line(ui, date_error.as_deref());
            });
            ui.end_row();
            let label = texts.get(TextId::TaskInfoDeadline);
            ui.label(&label);
            ui.vertical(|ui| {
                text_field(ui, &label, &mut a.deadline, true, FIELD_W);
                error_line(ui, deadline_error.as_deref());
            });
            ui.end_row();
            let label = texts.get(TextId::TaskInfoTaskType);
            ui.label(&label);
            combo(
                ui,
                "planrs.task-info.task-type",
                &label,
                texts,
                &mut a.task_type,
                &TASK_TYPES,
                editable,
            );
            ui.end_row();
            ui.allocate_space(egui::Vec2::ZERO);
            ui.add_enabled(
                editable,
                egui::Checkbox::new(&mut a.effort_driven, texts.get(TextId::TaskInfoEffortDriven)),
            );
            ui.end_row();
        });
}

/// Onglet Notes.
/// Identifiant de la zone des notes (multiligne : Entrée n'y valide pas la
/// fenêtre).
fn notes_id() -> egui::Id {
    egui::Id::unique("planrs.task-info.notes")
}

fn notes_tab(ui: &mut egui::Ui, texts: &Texts, state: &mut TaskInfoState) {
    let label = texts.get(TextId::TaskInfoNotes);
    let response = ui.add(
        egui::TextEdit::multiline(&mut state.draft.notes)
            .id(notes_id())
            .desired_width(f32::INFINITY)
            .desired_rows(10),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::MultilineTextInput, true, label.clone()));
}
