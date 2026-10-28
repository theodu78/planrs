//! Édition des cellules de la table (J4, lot L12) : état de la saisie et
//! traduction du texte saisi en lot de commandes du registre.
//!
//! Aucune date n'est calculée ici (règle 3) : le texte est lu par
//! `planrs_automation::text` (R-140 à R-143) puis envoyé au registre, qui
//! applique R-158 (début ou fin saisis → contrainte) et recalcule. Seule la
//! résolution ID → UID des prédécesseurs se fait côté interface (R-150 :
//! le registre ne connaît que des UID).
//!
//! [`commit_edit`] est pure (aucun egui) : la vue (`view::table`) l'appelle à
//! la validation et pousse le lot rendu dans `Action::ExecOn` (document
//! visé par son numéro d'ouverture).

use std::collections::BTreeMap;

use planrs_automation::command::{
    ConstraintKindJson, DurationJson, LagJson, LinkKey, LinkSpec, LinkTypeJson, PercentJson, ProgressUpdate,
    SetConstraint, TaskAdd, TaskUpdate,
};
use planrs_automation::text::{
    self, DateRole, PredecessorRef, TextContext, TextError, format_date, format_duration,
    format_predecessors, parse_date, parse_duration, parse_percent, parse_predecessors,
};
use planrs_automation::{Batch, Command, CommandError, Patch, Placement, Session, TaskRef};
use planrs_core::{Constraint, Duration, Lag, LinkType, Moment, Project, SchedulingMode, Task, TaskUid};

use crate::app::OpenDocument;
use crate::column::Column;
use crate::i18n::{Arg, Language, TextId, Texts, error_text_id};
use crate::rows::{Row, RowKind, RowModel};

/// Étiquette de la tâche créée par la ligne vide, reprise par les liens du
/// même lot (R-150).
const BLANK_LABEL: &str = "$new";

/// Refus d'une saisie, affiché sous la cellule (la saisie reste ouverte).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// Texte illisible (R-140 à R-143).
    Text(TextError),
    /// Prédécesseur désigné par un ID qui n'existe pas dans le projet.
    UnknownId(u32),
    /// Colonne calculée, non éditable (ID, marge, critique).
    NotEditable,
    /// Saisie vide sur une cellule qui ne peut pas être vide (R-259 : durée,
    /// date d'une tâche manuelle ou d'une récapitulative…).
    NotClearable,
}

impl From<TextError> for EditError {
    fn from(e: TextError) -> Self {
        Self::Text(e)
    }
}

impl EditError {
    /// Message traduit (clés `error-text-*` des refus du moteur, détail dans
    /// la langue de l'interface ; `edit-error-*` sinon).
    #[must_use]
    pub fn message(&self, texts: &Texts) -> String {
        match self {
            Self::Text(e) => {
                let detail = CommandError::text(e.clone()).message(text_language(texts.language()));
                texts.get_args(error_text_id(e.code()), &[("detail", Arg::Str(detail.into()))])
            }
            Self::UnknownId(id) => {
                texts.get_args(TextId::EditErrorUnknownId, &[("id", Arg::Count(i64::from(*id)))])
            }
            Self::NotEditable => texts.get(TextId::EditErrorNotEditable),
            Self::NotClearable => texts.get_args(TextId::CellsNotClearable, &[("count", Arg::Count(1))]),
        }
    }
}

/// Saisie en cours dans une cellule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditState {
    /// Tâche éditée ; `None` = ligne vide de fin (création).
    pub uid: Option<TaskUid>,
    /// Colonne éditée (une de [`editable`]).
    pub column: Column,
    /// Texte saisi.
    pub buffer: String,
    /// Refus de la dernière validation, affiché sous la cellule.
    pub error: Option<EditError>,
    /// Texte à l'ouverture par F2 ou double-clic : valider sans rien
    /// changer n'envoie aucun lot (ni historique, ni contrainte posée par
    /// R-158 sur une date relue). `None` (saisie ouverte par la frappe) :
    /// toujours envoyé.
    pub original: Option<String>,
}

impl EditState {
    /// Saisie ouverte avec `text` comme texte initial (F2, double-clic) :
    /// le valider tel quel ne change rien.
    #[must_use]
    pub fn open(uid: Option<TaskUid>, column: Column, text: String) -> Self {
        Self {
            uid,
            column,
            buffer: text.clone(),
            error: None,
            original: Some(text),
        }
    }

    /// Saisie ouverte par la frappe d'un caractère : `typed` remplace la
    /// valeur, envoyée à la validation même si elle est identique au texte
    /// affiché (comme une saisie complète dans MS Project).
    #[must_use]
    pub fn typed(uid: Option<TaskUid>, column: Column, typed: &str) -> Self {
        Self {
            uid,
            column,
            buffer: typed.to_owned(),
            error: None,
            original: None,
        }
    }
}

/// Saisie en cours dans la table : état transitoire de la vue (jamais
/// sérialisé), tenu par `ViewTransient`.
///
/// La cellule courante (ligne et colonne) est le curseur
/// `ViewTransient::cursor` (J7, lot L1, ADR-0015 D1) : la ligne courante
/// est la tâche sélectionnée (`App::selection`) ou, sans sélection, la
/// ligne vide de fin.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableEdit {
    /// Saisie ouverte, s'il y en a une.
    pub editing: Option<EditState>,
    /// Donner le focus à la zone de saisie (curseur en fin de texte) à la
    /// prochaine image.
    pub focus_pending: bool,
    /// Amener la ligne éditée à l'écran (saisie ouverte par F2 ou la frappe
    /// sur une ligne hors de la plage dessinée, relecture finale J7, F2) :
    /// lu par la vue principale après le dessin des lignes.
    pub reveal: bool,
}

impl TableEdit {
    /// Ouvre une saisie, demande le focus et la ligne à l'écran.
    pub fn start(&mut self, state: EditState) {
        self.editing = Some(state);
        self.focus_pending = true;
        self.reveal = true;
    }
}

/// Colonnes éditables : Nom, Durée, Début, Fin, Prédécesseurs, % achevé
/// (J7, lot L6).
#[must_use]
pub const fn editable(column: Column) -> bool {
    matches!(
        column,
        Column::Name
            | Column::Duration
            | Column::Start
            | Column::Finish
            | Column::Predecessors
            | Column::PercentComplete
    )
}

/// Cellule modifiable pour ce type de ligne (ADR-0018, R-257, R-258) :
/// colonne éditable ([`editable`]) et champ que le registre accepte
/// d'écrire sur cette ligne. Une récapitulative n'a que son Nom et ses
/// Prédécesseurs (durée, dates et % calculés, R-054, R-055) ; une tâche
/// exclue n'a pas de % achevé ; une ligne vide MSPDI (`IsNull`) rien.
#[must_use]
pub fn writable(row: &Row, column: Column) -> bool {
    if !editable(column) {
        return false;
    }
    match row.kind {
        RowKind::Null => false,
        RowKind::Summary => matches!(column, Column::Name | Column::Predecessors),
        RowKind::Excluded => column != Column::PercentComplete,
        RowKind::Leaf | RowKind::Milestone => true,
    }
}

/// Effacement d'une cellule (R-259).
#[derive(Clone, Debug, PartialEq)]
pub enum CellClear {
    /// Opérations qui vident la cellule.
    Ops(Vec<Command>),
    /// Cellule déjà vide (nom vide, aucun lien, 0 %, contrainte par
    /// défaut) : rien à écrire.
    Nothing,
    /// La cellule ne peut pas être vide (durée, date d'une tâche manuelle
    /// ou d'une récapitulative, % d'une récapitulative ou d'une tâche
    /// inactive, colonnes calculées).
    NotClearable,
}

/// Vrai si la tâche `uid` est récapitulative (R-052 : la tâche non vide
/// suivante est plus profonde).
fn is_summary(project: &Project, uid: TaskUid) -> bool {
    let Some(pos) = project.tasks.position(uid) else {
        return false;
    };
    let Some(task) = project.tasks.at(pos) else {
        return false;
    };
    project
        .tasks
        .iter()
        .skip(pos.saturating_add(1))
        .find(|t| !t.is_null)
        .is_some_and(|next| next.outline_level.get() > task.outline_level.get())
}

/// R-259 : opérations qui vident la cellule (`task`, `column`) ; chemin
/// unique de Ctrl+Suppr, de Suppr sur une cellule et de la validation d'une
/// saisie vide.
///
/// - Nom ⇒ nom vide ; Prédécesseurs ⇒ liens retirés (même chemin que la
///   saisie) ; % achevé ⇒ 0 % (récapitulative ou tâche inactive : non
///   effaçable) ;
/// - Début ou Fin d'une tâche automatique ⇒ contrainte par défaut du sens
///   de planification (`task.set_constraint`) ; d'une tâche manuelle ou
///   d'une récapitulative ⇒ non effaçable ;
/// - Durée et colonnes calculées ⇒ non effaçables.
#[must_use]
pub fn clear_cell(task: &Task, project: &Project, column: Column, ctx: &TextContext) -> CellClear {
    let uid = task.uid();
    if task.is_null {
        return CellClear::Nothing;
    }
    let ops = match column {
        Column::Name if task.name.is_empty() => Vec::new(),
        Column::Name => {
            let mut op = update(uid);
            op.name = Patch::Set(String::new());
            vec![Command::TaskUpdate(op)]
        }
        Column::Predecessors => replace_links(task, &[], project, ctx),
        Column::PercentComplete if is_summary(project, uid) || !task.active => {
            return CellClear::NotClearable;
        }
        Column::PercentComplete if task.progress.percent_complete.get() == 0 => Vec::new(),
        Column::PercentComplete => match percent_update(uid, "0", ctx) {
            Ok(op) => vec![op],
            Err(_) => return CellClear::NotClearable,
        },
        Column::Start | Column::Finish
            if is_summary(project, uid) || matches!(task.mode, SchedulingMode::Manual { .. }) =>
        {
            return CellClear::NotClearable;
        }
        Column::Start | Column::Finish => {
            let default = Constraint::default_for(&project.anchor);
            if task.constraint == default {
                Vec::new()
            } else {
                vec![Command::TaskSetConstraint(SetConstraint {
                    uid: TaskRef::Uid(uid),
                    kind: ConstraintKindJson::from(default.kind()),
                    date: None,
                })]
            }
        }
        // Colonnes calculées ou en lecture seule, et champs de la seconde
        // passe UX (à partir du lot L7).
        _ => return CellClear::NotClearable,
    };
    if ops.is_empty() {
        CellClear::Nothing
    } else {
        CellClear::Ops(ops)
    }
}

/// `progress.update { percent_complete: { text } }` d'un % saisi (J7, lot
/// L6) : lu par `parse_percent` pour signaler l'erreur sous la cellule,
/// envoyé en texte avec `%` (que le registre exige ; « 50 » se lit
/// « 50% »). Une récapitulative ou une tâche exclue est refusée par le
/// registre (bandeau).
fn percent_update(uid: TaskUid, buffer: &str, ctx: &TextContext) -> Result<Command, EditError> {
    parse_percent(buffer, ctx)?;
    let trimmed = buffer.trim();
    let text = if trimmed.ends_with('%') {
        trimmed.to_owned()
    } else {
        format!("{trimmed}%")
    };
    Ok(Command::ProgressUpdate(ProgressUpdate {
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
    }))
}

/// Langue des textes du moteur correspondant à la langue de l'interface.
#[must_use]
pub const fn text_language(language: Language) -> text::Language {
    match language {
        Language::Fr => text::Language::Fr,
        Language::En => text::Language::En,
    }
}

/// Lot vide (rien à faire : valeur inchangée, ligne vide laissée vide).
const fn empty_batch(lang: text::Language) -> Batch {
    Batch {
        format_version: Batch::FORMAT_VERSION,
        lang: Some(lang),
        ops: Vec::new(),
        label: None,
    }
}

/// `task.update` vide sur `uid`.
const fn update(uid: TaskUid) -> TaskUpdate {
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
        custom: std::collections::BTreeMap::new(),
    }
}

/// `task.add` en fin de projet (ligne vide), nom vide.
fn add_at_end(name: String) -> TaskAdd {
    TaskAdd {
        name,
        label: None,
        at: Some(Placement::End),
        duration: None,
        milestone: None,
        notes: None,
        start: None,
        finish: None,
    }
}

/// Décalage en JSON sans texte (minutes et unité, ou pourcentage) : relu
/// exactement par `LagJson::to_lag`.
fn lag_json(lag: &Lag, ctx: &TextContext) -> LagJson {
    LagJson {
        text: None,
        ..LagJson::from_lag(lag, ctx)
    }
}

/// Prédécesseur résolu : UID, type, décalage.
type Resolved = (TaskUid, LinkType, Lag);

/// R-150 côté interface : ID (position + 1) → UID. Un ID absent refuse la
/// saisie.
fn resolve(refs: &[PredecessorRef], project: &Project) -> Result<Vec<Resolved>, EditError> {
    refs.iter()
        .map(|r| {
            let position = usize::try_from(r.id).ok().and_then(|id| id.checked_sub(1));
            position
                .and_then(|p| project.tasks.at(p))
                .map(|task| (task.uid(), r.kind, r.lag))
                .ok_or(EditError::UnknownId(r.id))
        })
        .collect()
}

/// Lit une liste de prédécesseurs : la première erreur de texte refuse la
/// saisie.
fn read_predecessors(s: &str, ctx: &TextContext, project: &Project) -> Result<Vec<Resolved>, EditError> {
    let (refs, errors) = parse_predecessors(s, ctx);
    if let Some(e) = errors.into_iter().next() {
        return Err(e.into());
    }
    resolve(&refs, project)
}

/// Lot qui remplace les prédécesseurs de `task` par `wanted` : retrait des
/// anciens absents, modification de ceux dont le type ou le décalage change,
/// ajout des nouveaux. Un lien vers un UID absent du projet (pendant) n'est
/// pas touché. Doublon, cycle, lien vers soi : refus du registre (R-156,
/// R-160).
fn replace_links(task: &Task, wanted: &[Resolved], project: &Project, ctx: &TextContext) -> Vec<Command> {
    let succ = task.uid();
    let old: BTreeMap<TaskUid, (LinkType, Lag)> = task
        .predecessors
        .iter()
        .map(|link| (link.predecessor, (link.kind, link.lag)))
        .collect();
    let mut ops = Vec::new();
    for link in &task.predecessors {
        let kept = wanted.iter().any(|(uid, _, _)| *uid == link.predecessor);
        if !kept && project.tasks.contains(link.predecessor) {
            ops.push(Command::LinkRemove(LinkKey {
                pred: TaskRef::Uid(link.predecessor),
                succ: TaskRef::Uid(succ),
            }));
        }
    }
    for (pred, kind, lag) in wanted {
        let spec = LinkSpec {
            pred: TaskRef::Uid(*pred),
            succ: TaskRef::Uid(succ),
            kind: Some(LinkTypeJson::from(*kind)),
            lag: Some(lag_json(lag, ctx)),
        };
        match old.get(pred) {
            Some((k, l)) if k == kind && l == lag => {}
            Some(_) => ops.push(Command::LinkUpdate(spec)),
            None => ops.push(Command::LinkAdd(spec)),
        }
    }
    ops
}

/// Lot de commandes d'une saisie validée (fonction pure).
///
/// - texte inchangé depuis l'ouverture → lot vide ;
/// - Nom → `task.update name` (ligne vide : `task.add at End`, rien si le
///   nom est vide) ;
/// - Durée → lue par `parse_duration` (R-140) pour signaler l'erreur sous
///   la cellule, envoyée en texte (`task.update duration {text}`) ;
/// - Début, Fin → `parse_date` (R-143, rôle Start ou Finish) →
///   `task.update start|finish` (R-158 appliquée par le registre) ;
/// - Prédécesseurs → `parse_predecessors` (R-142), ID → UID (R-150), puis
///   `link.remove` des anciens absents, `link.update`, `link.add` ;
/// - % achevé → `parse_percent`, puis `progress.update { percent_complete:
///   { text } }` (J7, lot L6 ; refusé sur la ligne vide : une tâche se crée
///   par les autres colonnes) ;
/// - ligne vide et autre colonne que Nom : `task.add` (nom vide) portant la
///   valeur, ou suivi des `link.add` vers l'étiquette `$new`.
///
/// Le lot porte la langue de l'interface (`lang`) : la session est ouverte
/// en français et doit lire « 3d » en anglais.
///
/// # Errors
///
/// [`EditError`] : texte illisible, ID inconnu, colonne non éditable, tâche
/// absente (`text.empty` n'est jamais rendu pour la ligne vide).
pub fn commit_edit(state: &EditState, session: &Session, lang: text::Language) -> Result<Batch, EditError> {
    let mut batch = empty_batch(lang);
    if !editable(state.column) {
        return Err(EditError::NotEditable);
    }
    if state.original.as_deref() == Some(state.buffer.as_str()) {
        return Ok(batch);
    }
    let project = session.project();
    let ctx = TextContext::for_project(project, lang);
    let buffer = state.buffer.as_str();
    batch.ops = match state.uid {
        Some(uid) => {
            let Some(task) = project.tasks.get(uid) else {
                return Ok(batch);
            };
            // Saisie vide (Retour arrière puis Entrée, collage d'une cellule
            // vide) : effacement de R-259 au lieu d'une lecture refusée.
            if buffer.trim().is_empty() && editable(state.column) {
                batch.ops = match clear_cell(task, project, state.column, &ctx) {
                    CellClear::Ops(ops) => ops,
                    CellClear::Nothing => Vec::new(),
                    CellClear::NotClearable => return Err(EditError::NotClearable),
                };
                return Ok(batch);
            }
            let mut op = update(uid);
            match state.column {
                Column::Name => op.name = Patch::Set(buffer.to_owned()),
                Column::Duration => {
                    parse_duration(buffer, &ctx)?;
                    op.duration = Patch::Set(duration_text(buffer));
                }
                Column::Start => op.start = Patch::Set(parse_date(buffer, DateRole::Start, &ctx)?),
                Column::Finish => op.finish = Patch::Set(parse_date(buffer, DateRole::Finish, &ctx)?),
                Column::Predecessors => {
                    let wanted = read_predecessors(buffer, &ctx, project)?;
                    let ops = replace_links(task, &wanted, project, &ctx);
                    batch.ops = ops;
                    return Ok(batch);
                }
                Column::PercentComplete => {
                    batch.ops = vec![percent_update(uid, buffer, &ctx)?];
                    return Ok(batch);
                }
                // Colonnes calculées ou en lecture seule, et champs de la seconde
                // passe UX (à partir du lot L7).
                _ => return Err(EditError::NotEditable),
            }
            vec![Command::TaskUpdate(op)]
        }
        None => blank_row_ops(state.column, buffer, &ctx, project)?,
    };
    Ok(batch)
}

/// Durée envoyée en texte (lue par le registre dans la langue du lot).
fn duration_text(buffer: &str) -> DurationJson {
    DurationJson {
        text: Some(buffer.to_owned()),
        ..DurationJson::default()
    }
}

/// Opérations d'une saisie sur la ligne vide de fin (création d'une tâche).
fn blank_row_ops(
    column: Column,
    buffer: &str,
    ctx: &TextContext,
    project: &Project,
) -> Result<Vec<Command>, EditError> {
    if buffer.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut add = add_at_end(String::new());
    match column {
        Column::Name => buffer.clone_into(&mut add.name),
        Column::Duration => {
            parse_duration(buffer, ctx)?;
            add.duration = Some(duration_text(buffer));
        }
        Column::Start => add.start = Some(parse_date(buffer, DateRole::Start, ctx)?),
        Column::Finish => add.finish = Some(parse_date(buffer, DateRole::Finish, ctx)?),
        Column::Predecessors => {
            let wanted = read_predecessors(buffer, ctx, project)?;
            add.label = Some(BLANK_LABEL.to_owned());
            let mut ops = vec![Command::TaskAdd(add)];
            ops.extend(wanted.iter().map(|(pred, kind, lag)| {
                Command::LinkAdd(LinkSpec {
                    pred: TaskRef::Uid(*pred),
                    succ: TaskRef::Label(BLANK_LABEL.to_owned()),
                    kind: Some(LinkTypeJson::from(*kind)),
                    lag: Some(lag_json(lag, ctx)),
                })
            }));
            return Ok(ops);
        }
        // Colonnes calculées ou en lecture seule, et champs de la seconde
        // passe UX (à partir du lot L7).
        _ => return Err(EditError::NotEditable),
    }
    Ok(vec![Command::TaskAdd(add)])
}

/// Texte initial d'une durée : l'affichage s'il se relit exactement
/// (« 5 jours », « 2,5 days »), sinon l'écriture R-141 du moteur (« 5ej » :
/// « 5 jours écoulés » n'est pas une unité de R-140).
#[must_use]
pub fn duration_edit_text(d: &Duration, display: &str, ctx: &TextContext) -> String {
    if parse_duration(display, ctx).is_ok_and(|read| read == *d) {
        display.to_owned()
    } else {
        format_duration(d, ctx)
    }
}

/// Texte initial d'une date : l'affichage (« lun 02/11/26 », sans heure)
/// s'il se relit au même jour, sinon `JJ/MM/AAAA hh:mm` (R-143 : l'année sur
/// deux chiffres 84 à 99 se lit 1984-1999, une date de 2084 à 2149 ne se
/// relit donc pas).
#[must_use]
pub fn date_edit_text(m: Moment, display: &str, role: DateRole, ctx: &TextContext) -> String {
    if parse_date(display, role, ctx).is_ok_and(|read| read.day() == m.day()) {
        display.to_owned()
    } else {
        format_date(m, ctx)
    }
}

/// Texte initial des prédécesseurs : l'affichage s'il se relit aux mêmes
/// liens, sinon l'écriture R-142 du moteur (« 3FD+2ej »).
#[must_use]
pub fn predecessors_edit_text(task: &Task, project: &Project, display: &str, ctx: &TextContext) -> String {
    let current: Vec<Resolved> = task
        .predecessors
        .iter()
        .map(|link| (link.predecessor, link.kind, link.lag))
        .collect();
    if read_predecessors(display, ctx, project).is_ok_and(|read| read == current) {
        display.to_owned()
    } else {
        format_predecessors(task, project, ctx).0
    }
}

/// Texte initial de la cellule (ligne `row` du modèle, `RowModel::len()` =
/// ligne vide) : le texte affiché, sauf quand il ne se relirait pas à la
/// même valeur (voir [`duration_edit_text`], [`date_edit_text`],
/// [`predecessors_edit_text`]). Ligne vide : texte vide.
#[must_use]
pub fn initial_text(
    doc: &OpenDocument,
    rows: &RowModel,
    row: usize,
    column: Column,
    lang: Language,
) -> String {
    let project = doc.project();
    let ctx = TextContext::for_project(project, text_language(lang));
    initial_text_in(project, rows, row, column, &ctx)
}

/// [`initial_text`] sur un projet et un contexte de texte donnés (copier
/// et remplir un bloc de cellules, ADR-0018 : un contexte pour toutes les
/// cellules).
#[must_use]
pub fn initial_text_in(
    project: &Project,
    rows: &RowModel,
    row: usize,
    column: Column,
    ctx: &TextContext,
) -> String {
    let Some(info) = rows.row(row) else {
        return String::new();
    };
    let display = rows.text(row, column);
    let Some(task) = project.tasks.get(info.uid) else {
        return display.to_owned();
    };
    match column {
        // Récapitulative : durée calculée, affichée telle quelle (le registre
        // refuse de la modifier).
        Column::Duration if info.kind != RowKind::Summary => duration_edit_text(&task.duration, display, ctx),
        Column::Start => info.bar.map_or_else(
            || display.to_owned(),
            |(start, _)| date_edit_text(start, display, DateRole::Start, ctx),
        ),
        Column::Finish => info.bar.map_or_else(
            || display.to_owned(),
            |(_, finish)| date_edit_text(finish, display, DateRole::Finish, ctx),
        ),
        Column::Predecessors => predecessors_edit_text(task, project, display, ctx),
        _ => display.to_owned(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use planrs_automation::command::{LinkKey, LinkSpec, LinkTypeJson};
    use planrs_automation::text::{Language as TextLanguage, TextError};
    use planrs_automation::{ApplyMode, Batch, Command, Placement, Session, TaskRef};
    use planrs_core::{LinkType, TaskUid};

    use super::{EditError, EditState, commit_edit, editable};
    use crate::column::Column;

    /// Session de l'exemple (UID n = ID n).
    fn session() -> Session {
        crate::document::load(crate::DEMO_MSPDI, "2027-01-15T08:00".parse().unwrap())
            .unwrap()
            .session
    }

    fn uid(n: u32) -> TaskUid {
        TaskUid::new(n)
    }

    fn typed(task: Option<TaskUid>, column: Column, text: &str) -> EditState {
        EditState::typed(task, column, text)
    }

    fn ops(state: &EditState) -> Vec<Command> {
        commit_edit(state, &session(), TextLanguage::Fr).unwrap().ops
    }

    #[test]
    fn edit_unchanged_text_sends_nothing() {
        let state = EditState::open(Some(uid(3)), Column::Duration, "10 jours".to_owned());
        let batch = commit_edit(&state, &session(), TextLanguage::Fr).unwrap();
        assert_eq!(batch.ops, Vec::<Command>::new());
        // Ligne vide laissée vide : rien non plus.
        assert_eq!(ops(&typed(None, Column::Name, "  ")), Vec::<Command>::new());
    }

    #[test]
    fn edit_invalid_duration_is_text_error() {
        let err = commit_edit(
            &typed(Some(uid(3)), Column::Duration, "abc"),
            &session(),
            TextLanguage::Fr,
        )
        .unwrap_err();
        assert!(
            matches!(err, EditError::Text(TextError::BadNumber { .. })),
            "{err:?}"
        );
        let err = commit_edit(
            &typed(Some(uid(3)), Column::Start, "31/02/2027"),
            &session(),
            TextLanguage::Fr,
        )
        .unwrap_err();
        assert_eq!(err, EditError::Text(TextError::BadDate));
    }

    #[test]
    fn edit_not_editable_columns_refused() {
        for column in [Column::Id, Column::TotalSlack, Column::Critical] {
            assert!(!editable(column));
            let err =
                commit_edit(&typed(Some(uid(3)), column, "1"), &session(), TextLanguage::Fr).unwrap_err();
            assert_eq!(err, EditError::NotEditable);
        }
    }

    /// Le lot porte la langue de l'interface : « 3d » est lu en anglais.
    #[test]
    fn edit_batch_carries_ui_language() {
        let state = typed(Some(uid(3)), Column::Duration, "3d");
        let batch = commit_edit(&state, &session(), TextLanguage::En).unwrap();
        assert_eq!(batch.lang, Some(TextLanguage::En));
        let mut s = session();
        s.apply(&batch, ApplyMode::Commit).unwrap();
        assert_eq!(
            s.project()
                .tasks
                .get(TaskUid::new(3))
                .unwrap()
                .duration
                .minutes()
                .get(),
            3 * 480
        );
    }

    /// Prédécesseurs : la tâche 9 a « 7;6 » ; « 6;8DD+1j » retire 7, garde
    /// 6, ajoute 8 en DD.
    #[test]
    fn edit_predecessors_diff() {
        let got = ops(&typed(Some(uid(9)), Column::Predecessors, "6;8DD+1j"));
        assert_eq!(got.len(), 2, "{got:?}");
        assert_eq!(
            got[0],
            Command::LinkRemove(LinkKey {
                pred: TaskRef::Uid(TaskUid::new(7)),
                succ: TaskRef::Uid(TaskUid::new(9)),
            })
        );
        let Command::LinkAdd(LinkSpec { pred, kind, lag, .. }) = &got[1] else {
            panic!("{got:?}");
        };
        assert_eq!(pred, &TaskRef::Uid(TaskUid::new(8)));
        assert_eq!(*kind, Some(LinkTypeJson::from(LinkType::StartToStart)));
        assert_eq!(lag.as_ref().and_then(|l| l.minutes), Some(480));
        // Même liste avec un type changé : `link.update`.
        let got = ops(&typed(Some(uid(9)), Column::Predecessors, "7FF;6"));
        assert!(matches!(&got[..], [Command::LinkUpdate(_)]), "{got:?}");
        // Liste vide : tout retirer.
        let got = ops(&typed(Some(uid(9)), Column::Predecessors, ""));
        assert_eq!(got.len(), 2);
        assert!(got.iter().all(|op| matches!(op, Command::LinkRemove(_))));
        // ID inconnu.
        let err = commit_edit(
            &typed(Some(uid(9)), Column::Predecessors, "7;900"),
            &session(),
            TextLanguage::Fr,
        )
        .unwrap_err();
        assert_eq!(err, EditError::UnknownId(900));
        let err = commit_edit(
            &typed(Some(uid(9)), Column::Predecessors, "7XY"),
            &session(),
            TextLanguage::Fr,
        )
        .unwrap_err();
        assert!(
            matches!(err, EditError::Text(TextError::BadLinkType { .. })),
            "{err:?}"
        );
    }

    #[test]
    fn edit_blank_row_creates_task() {
        let got = ops(&typed(None, Column::Name, "Réception"));
        let [Command::TaskAdd(add)] = &got[..] else {
            panic!("{got:?}")
        };
        assert_eq!(add.name, "Réception");
        assert_eq!(add.at, Some(Placement::End));
        let got = ops(&typed(None, Column::Duration, "3j"));
        let [Command::TaskAdd(add)] = &got[..] else {
            panic!("{got:?}")
        };
        assert_eq!(add.name, "");
        assert!(add.duration.is_some());
        // Prédécesseurs sur la ligne vide : la tâche créée est désignée par
        // son étiquette dans le même lot.
        let got = ops(&typed(None, Column::Predecessors, "3;4"));
        assert_eq!(got.len(), 3, "{got:?}");
        let mut s = session();
        let batch = Batch {
            format_version: 1,
            lang: None,
            ops: got,
            label: None,
        };
        s.apply(&batch, ApplyMode::Commit).unwrap();
        let last = s.project().tasks.iter().last().unwrap();
        assert_eq!(last.predecessors.len(), 2);
    }
}
