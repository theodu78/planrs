//! Modèle de lignes partagé par la table et le Gantt.
//!
//! Tout ce qui coûte O(n) se fait une fois, hors de l'image : à l'ouverture
//! et au changement de langue, jamais pendant le défilement ni le zoom. Les
//! textes des cellules sont pré-formatés (ligne par ligne, une entrée par
//! colonne de [`Column::ALL`]) ; les liens sont triés par ligne minimale.

use std::collections::BTreeMap;
use std::ops::Range;

use planrs_core::{LinkType, Moment, Project, Task, TaskOutcome, TaskRole, TaskSchedule, TaskUid, TimeBasis};

use crate::app::OpenDocument;
use crate::column::Column;
use crate::format::{
    FormatLabels, format_date, format_duration, format_minutes_as, format_predecessors, format_slack,
};

/// Nature d'une ligne, pour le dessin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// Ligne vide (tâche nulle).
    Null,
    /// Tâche feuille planifiée.
    Leaf,
    /// Tâche récapitulative.
    Summary,
    /// Jalon.
    Milestone,
    /// Tâche exclue du calcul (inactive…).
    Excluded,
}

/// Une ligne de la vue (ordre des ID, tri interdit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// UID de la tâche.
    pub uid: TaskUid,
    /// ID affiché (position + 1).
    pub id: u32,
    /// Niveau hiérarchique (≥ 1).
    pub level: u16,
    /// Nature de la ligne.
    pub kind: RowKind,
    /// Sur le chemin critique.
    pub critical: bool,
    /// Début et fin de la barre, issus du planning (aucun calcul ici).
    pub bar: Option<(Moment, Moment)>,
}

/// Lien à dessiner, en indices de ligne.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkView {
    /// Ligne du prédécesseur.
    pub from: u32,
    /// Ligne du successeur.
    pub to: u32,
    /// Type de lien.
    pub kind: LinkType,
    /// Lien critique (les deux extrémités sont critiques).
    pub critical: bool,
}

/// Lignes, textes pré-formatés et liens d'un document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowModel {
    /// Lignes dans l'ordre des ID.
    rows: Vec<Row>,
    /// `rows.len() × Column::COUNT` textes, ligne par ligne.
    texts: Vec<String>,
    /// Liens triés par `min(from, to)`.
    links: Vec<LinkView>,
    /// Plus grand écart `|from − to|` d'un lien (bornage de la recherche).
    max_span: u32,
    /// Indice de ligne de chaque tâche.
    index: BTreeMap<TaskUid, usize>,
}

impl RowModel {
    /// Construit le modèle d'un document avec les libellés d'une langue.
    ///
    /// Une ligne par tâche, dans l'ordre des ID ; nature de la ligne :
    /// `Null` (ligne vide), `Excluded` (exclue du calcul, barre d'affichage
    /// R-062 si elle existe), `Summary`, `Milestone` (jalon, ou feuille de
    /// durée nulle : hypothèse H-G1), sinon `Leaf`. Textes : hypothèses
    /// H-UI-08 (durée d'une récapitulative dans l'unité par défaut du projet,
    /// en temps ouvré), H-UI-09 (marge en jours ouvrés), H-UI-10 (tâche
    /// exclue : colonnes calculées vides, sauf les dates d'affichage d'une
    /// inactive ; ligne vide : seul l'ID).
    #[must_use]
    pub fn build(doc: &OpenDocument, labels: &FormatLabels) -> Self {
        let project = &doc.project;
        let count = project.tasks.len();
        let mut rows = Vec::with_capacity(count);
        let mut texts = Vec::with_capacity(count.saturating_mul(Column::COUNT));
        let mut index = BTreeMap::new();
        for (position, task) in project.tasks.iter().enumerate() {
            let uid = task.uid();
            let outcome = doc.schedule.tasks.get(&uid);
            let row = make_row(project, task, position, outcome);
            push_texts(&mut texts, project, task, &row, outcome, labels);
            index.insert(uid, position);
            rows.push(row);
        }
        let (links, max_span) = collect_links(project, &rows, &index);
        Self {
            rows,
            texts,
            links,
            max_span,
            index,
        }
    }

    /// Nombre de lignes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Aucune ligne.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Ligne d'indice `i`, si elle existe.
    #[must_use]
    pub fn row(&self, i: usize) -> Option<&Row> {
        self.rows.get(i)
    }

    /// Texte pré-formaté d'une cellule ; chaîne vide hors bornes.
    #[must_use]
    pub fn text(&self, i: usize, c: Column) -> &str {
        i.checked_mul(Column::COUNT)
            .and_then(|base| base.checked_add(c.ordinal()))
            .and_then(|k| self.texts.get(k))
            .map_or("", String::as_str)
    }

    /// Indice de la ligne d'une tâche (O(log n)).
    #[must_use]
    pub fn index_of(&self, uid: TaskUid) -> Option<usize> {
        self.index.get(&uid).copied()
    }

    /// Tous les liens, triés par `min(from, to)` croissant.
    #[must_use]
    pub fn links(&self) -> &[LinkView] {
        &self.links
    }

    /// Plus grand écart `|from − to|` d'un lien.
    #[must_use]
    pub const fn max_span(&self) -> u32 {
        self.max_span
    }

    /// Liens dont l'intervalle de lignes `[min, max]` coupe `rows` (culling
    /// par intervalle : un lien qui traverse la fenêtre est gardé même si ses
    /// deux extrémités sont hors champ).
    ///
    /// Balayage des liens triés par `min` : on saute par dichotomie ceux dont
    /// `min + max_span < rows.start` (leur `max` est forcément avant la
    /// fenêtre), puis on parcourt tant que `min < rows.end` en filtrant
    /// `max >= rows.start`. Coût O(log L + liens dont `min` est dans
    /// `[rows.start − max_span, rows.end)`) : O(L) au pire si un lien très
    /// long existe, ce qui reste négligeable (10 000 paires d'entiers).
    pub fn links_touching(&self, rows: Range<usize>) -> impl Iterator<Item = &LinkView> + '_ {
        let span = usize::try_from(self.max_span).unwrap_or(usize::MAX);
        let skip = self
            .links
            .partition_point(|link| link_low(link).saturating_add(span) < rows.start);
        let tail = self.links.get(skip..).unwrap_or(&[]);
        tail.iter()
            .take_while(move |link| link_low(link) < rows.end)
            .filter(move |link| link_high(link) >= rows.start)
    }
}

/// Ligne minimale d'un lien.
fn link_low(link: &LinkView) -> usize {
    usize::try_from(link.from.min(link.to)).unwrap_or(usize::MAX)
}

/// Ligne maximale d'un lien.
fn link_high(link: &LinkView) -> usize {
    usize::try_from(link.from.max(link.to)).unwrap_or(usize::MAX)
}

/// Planning d'une tâche planifiée, s'il existe.
const fn scheduled(outcome: Option<&TaskOutcome>) -> Option<&TaskSchedule> {
    match outcome {
        Some(TaskOutcome::Scheduled(s)) => Some(s),
        _ => None,
    }
}

/// Ligne d'une tâche (sans ses textes).
fn make_row(project: &Project, task: &Task, position: usize, outcome: Option<&TaskOutcome>) -> Row {
    let uid = task.uid();
    let id = project.task_id(uid).map_or_else(
        || {
            u32::try_from(position)
                .ok()
                .and_then(|p| p.checked_add(1))
                .unwrap_or(u32::MAX)
        },
        planrs_core::TaskId::get,
    );
    let (kind, critical, bar) = if task.is_null {
        (RowKind::Null, false, None)
    } else {
        match outcome {
            Some(TaskOutcome::Scheduled(s)) => {
                let kind = if s.role == TaskRole::Summary {
                    RowKind::Summary
                } else if task.milestone || task.duration.is_zero() {
                    RowKind::Milestone
                } else {
                    RowKind::Leaf
                };
                (kind, s.critical, Some((s.start, s.finish)))
            }
            Some(TaskOutcome::Excluded { display, .. }) => (
                RowKind::Excluded,
                false,
                display.map(|span| (span.start, span.finish)),
            ),
            None => (RowKind::Excluded, false, None),
        }
    };
    Row {
        uid,
        id,
        level: task.outline_level.get(),
        kind,
        critical,
        bar,
    }
}

/// Ajoute les `Column::COUNT` textes d'une ligne, dans l'ordre de
/// [`Column::ALL`].
fn push_texts(
    texts: &mut Vec<String>,
    project: &Project,
    task: &Task,
    row: &Row,
    outcome: Option<&TaskOutcome>,
    l: &FormatLabels,
) {
    let f = &project.options.factors;
    let schedule = scheduled(outcome);
    for column in Column::ALL {
        let text = if row.kind == RowKind::Null && column != Column::Id {
            String::new()
        } else {
            match column {
                Column::Id => row.id.to_string(),
                Column::Name => task.name.clone(),
                Column::Duration => match (row.kind, schedule) {
                    (RowKind::Summary, Some(s)) => format_minutes_as(
                        s.duration,
                        project.options.default_duration_unit,
                        TimeBasis::Working,
                        f,
                        l,
                    ),
                    _ => format_duration(&task.duration, f, l),
                },
                Column::Start => row
                    .bar
                    .map(|(start, _)| format_date(start, l))
                    .unwrap_or_default(),
                Column::Finish => row
                    .bar
                    .map(|(_, finish)| format_date(finish, l))
                    .unwrap_or_default(),
                Column::Predecessors => format_predecessors(&task.predecessors, project, f, l),
                Column::TotalSlack => schedule
                    .map(|s| format_slack(s.total_slack, f, l))
                    .unwrap_or_default(),
                Column::Critical => schedule
                    .map(|s| if s.critical { l.yes.clone() } else { l.no.clone() })
                    .unwrap_or_default(),
            }
        };
        texts.push(text);
    }
}

/// Liens entre lignes connues, triés par `min(from, to)` (tri stable :
/// ordre des successeurs puis de leurs prédécesseurs à égalité), et plus
/// grand écart.
fn collect_links(project: &Project, rows: &[Row], index: &BTreeMap<TaskUid, usize>) -> (Vec<LinkView>, u32) {
    let mut links = Vec::new();
    for (to, task) in project.tasks.iter().enumerate() {
        let Ok(to_u32) = u32::try_from(to) else {
            continue;
        };
        let to_critical = rows.get(to).is_some_and(|r| r.critical);
        for link in &task.predecessors {
            let Some(&from) = index.get(&link.predecessor) else {
                continue;
            };
            let Ok(from_u32) = u32::try_from(from) else {
                continue;
            };
            let from_critical = rows.get(from).is_some_and(|r| r.critical);
            links.push(LinkView {
                from: from_u32,
                to: to_u32,
                kind: link.kind,
                critical: from_critical && to_critical,
            });
        }
    }
    links.sort_by_key(|link| link.from.min(link.to));
    let max_span = links
        .iter()
        .map(|link| link.from.abs_diff(link.to))
        .max()
        .unwrap_or(0);
    (links, max_span)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::panic,
        reason = "tests : un échec doit s'arrêter net et lisiblement"
    )]

    use super::{RowKind, RowModel};
    use crate::app::{DocOrigin, OpenDocument};
    use crate::column::Column;
    use crate::i18n::{Language, Texts};

    fn demo() -> OpenDocument {
        let loaded = crate::document::load(crate::DEMO_MSPDI, "2026-10-17T08:00".parse().unwrap()).unwrap();
        OpenDocument {
            project: loaded.project,
            schedule: loaded.schedule,
            import_diagnostics: loaded.import_diagnostics,
            range: loaded.range,
            origin: DocOrigin {
                name: "demo".to_owned(),
                path: None,
                demo: true,
            },
            generation: 1,
        }
    }

    /// Ligne d'ID `id` (indice `id − 1`).
    fn row_texts(model: &RowModel, id: usize) -> Vec<&str> {
        Column::ALL.iter().map(|&c| model.text(id - 1, c)).collect()
    }

    #[test]
    fn rows_demo_fr() {
        let doc = demo();
        let model = RowModel::build(&doc, Texts::new(Language::Fr).format_labels());
        assert_eq!(model.len(), 49);
        assert!(!model.is_empty());

        let first = model.row(0).unwrap();
        assert_eq!(first.id, 1);
        assert_eq!(first.level, 1);
        assert_eq!(first.kind, RowKind::Summary);
        assert!(first.critical);
        assert_eq!(
            row_texts(&model, 1),
            [
                "1",
                "Études et permis",
                "31 jours",
                "lun 02/11/26",
                "mar 15/12/26",
                "",
                "0 jour",
                "Oui"
            ]
        );

        let permit = model.row(6).unwrap();
        assert_eq!(permit.kind, RowKind::Milestone);
        assert_eq!(permit.level, 2);
        assert_eq!(model.text(6, Column::Name), "Permis obtenu");
        assert_eq!(model.text(6, Column::Duration), "0 jour");

        assert_eq!(model.row(1).unwrap().kind, RowKind::Leaf);
        assert_eq!(model.text(12, Column::Predecessors), "12FD+2 jours");
        assert_eq!(model.text(25, Column::Predecessors), "25FD-1 jour");
        assert_eq!(model.text(30, Column::Predecessors), "30DD");
        assert_eq!(model.text(17, Column::Predecessors), "17FF");
        assert_eq!(model.text(23, Column::Predecessors), "25DF");
        assert_eq!(model.text(8, Column::Predecessors), "7;6");
        // Ligne 41 « Pose de la cuisine », SNET lun 26/04/2027 08:00 (R-093) :
        // la contrainte la retarde du ven 16/04 au lun 26/04 ; marge 5 jours
        // (LS lun 03/05), non critique ; aucun diagnostic.
        assert_eq!(
            row_texts(&model, 41)[2..],
            ["3 jours", "lun 26/04/27", "mer 28/04/27", "38", "5 jours", "Non"]
        );
        assert!(doc.schedule.diagnostics.is_empty());
        // Tâche non critique : marge positive, « Non ».
        assert_eq!(model.text(5, Column::TotalSlack), "5 jours");
        assert_eq!(model.text(5, Column::Critical), "Non");
        assert!(!model.row(5).unwrap().critical);
        // Dernière ligne : jalon de fin du projet.
        let last = model.row(48).unwrap();
        assert_eq!(last.kind, RowKind::Milestone);
        assert_eq!(
            last.bar,
            Some((doc.schedule.project_finish, doc.schedule.project_finish))
        );
        // Barres issues du planning, ID = position + 1, index inverse cohérent.
        for (i, task) in doc.project.tasks.iter().enumerate() {
            let row = model.row(i).unwrap();
            assert_eq!(row.uid, task.uid());
            assert_eq!(usize::try_from(row.id).unwrap(), i + 1);
            assert_eq!(model.index_of(task.uid()), Some(i));
            let Some(planrs_core::TaskOutcome::Scheduled(s)) = doc.schedule.tasks.get(&task.uid()) else {
                panic!("tâche non planifiée : {}", task.name);
            };
            assert_eq!(row.bar, Some((s.start, s.finish)));
        }
        // Jours fériés du calendrier de l'exemple, importés jusqu'à
        // l'affichage : la tâche 11 (3 jours) commence le jeudi 24/12 et
        // saute Noël et le week-end ; la tâche 3 (10 jours) saute le 11/11.
        assert_eq!(model.text(10, Column::Duration), "3 jours");
        assert_eq!(model.text(10, Column::Start), "jeu 24/12/26");
        assert_eq!(model.text(10, Column::Finish), "mar 29/12/26");
        assert_eq!(model.text(2, Column::Duration), "10 jours");
        assert_eq!(model.text(2, Column::Start), "lun 09/11/26");
        assert_eq!(model.text(2, Column::Finish), "lun 23/11/26");
        // Hors bornes : chaîne vide.
        assert_eq!(model.text(49, Column::Id), "");
        assert_eq!(model.text(usize::MAX, Column::Critical), "");
        assert!(model.row(49).is_none());
    }

    #[test]
    fn rows_demo_en() {
        let model = RowModel::build(&demo(), Texts::new(Language::En).format_labels());
        assert_eq!(
            row_texts(&model, 1),
            [
                "1",
                "Études et permis",
                "31 days",
                "Mon 11/2/26",
                "Tue 12/15/26",
                "",
                "0 days",
                "Yes"
            ]
        );
        assert_eq!(
            row_texts(&model, 41)[2..],
            ["3 days", "Mon 4/26/27", "Wed 4/28/27", "38", "5 days", "No"]
        );
        assert_eq!(model.text(12, Column::Predecessors), "12FS+2 days");
        assert_eq!(model.text(23, Column::Predecessors), "25SF");
        assert_eq!(model.text(8, Column::Predecessors), "7,6");
    }

    #[test]
    fn rows_links_sorted() {
        let doc = demo();
        let model = RowModel::build(&doc, Texts::new(Language::Fr).format_labels());
        let links = model.links();
        let total: usize = doc.project.tasks.iter().map(|t| t.predecessors.len()).sum();
        assert_eq!(links.len(), total);
        assert_eq!(links.len(), 46);
        assert!(
            links
                .windows(2)
                .all(|w| w[0].from.min(w[0].to) <= w[1].from.min(w[1].to))
        );
        let max_span = links.iter().map(|l| l.from.abs_diff(l.to)).max().unwrap();
        assert_eq!(model.max_span(), max_span);
        // 28 → 43 (indices 27 → 42) est le lien le plus long.
        assert_eq!(max_span, 15);
        // Lien début à fin 25 → 24 : successeur au-dessus du prédécesseur.
        let sf = links
            .iter()
            .find(|l| l.kind == planrs_core::LinkType::StartToFinish)
            .unwrap();
        assert_eq!((sf.from, sf.to), (24, 23));
        // Critique si et seulement si les deux extrémités sont critiques.
        for l in links {
            let from = model.row(usize::try_from(l.from).unwrap()).unwrap();
            let to = model.row(usize::try_from(l.to).unwrap()).unwrap();
            assert_eq!(l.critical, from.critical && to.critical, "{l:?}");
        }
    }

    #[test]
    fn rows_links_touching_window() {
        let model = RowModel::build(&demo(), Texts::new(Language::Fr).format_labels());
        let brute = |rows: std::ops::Range<usize>| -> Vec<_> {
            model
                .links()
                .iter()
                .filter(|l| {
                    let low = usize::try_from(l.from.min(l.to)).unwrap();
                    let high = usize::try_from(l.from.max(l.to)).unwrap();
                    low < rows.end && high >= rows.start
                })
                .copied()
                .collect()
        };
        for start in 0..52 {
            for len in 0..20 {
                let rows = start..start + len;
                let got: Vec<_> = model.links_touching(rows.clone()).copied().collect();
                assert_eq!(got, brute(rows.clone()), "{rows:?}");
            }
        }
        // Fenêtre 30..35 traversée par le lien 28 → 43 (indices 27 → 42),
        // dont aucune extrémité n'est visible.
        assert!(model.links_touching(30..35).any(|l| (l.from, l.to) == (27, 42)));
        assert_eq!(model.links_touching(0..0).count(), 0);
        assert_eq!(model.links_touching(100..200).count(), 0);
        assert_eq!(model.links_touching(0..usize::MAX).count(), 46);
        // Modèle vide : aucun lien.
        assert_eq!(RowModel::default().links_touching(0..10).count(), 0);
    }
}
