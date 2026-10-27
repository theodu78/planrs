//! Commandes de structure sur la sélection (lot L14) : insérer, supprimer,
//! indenter, désindenter, lier, délier ; monter et descendre (J7 lot L1,
//! H-UI-29).
//!
//! [`plan`] traduit une [`StructureOp`] et la [`Selection`] en **un** lot de
//! commandes du registre (une transaction, une entrée d'historique) ; il ne
//! calcule rien du planning (règle 3) et ne lit que l'ordre et les niveaux
//! hiérarchiques du modèle. Le registre reste seul juge : un doublon, un
//! cycle ou une hiérarchie refusée font refuser tout le lot (R-153, R-156,
//! R-160), refus affiché dans le bandeau.
//!
//! Hiérarchie (R-052) : une tâche est récapitulative si la tâche suivante
//! est plus profonde ; son bloc est la suite des tâches plus profondes qui la
//! suivent (lignes vides comprises, comme `task.delete`). Une tâche dont un
//! ancêtre est aussi sélectionné est retirée du lot de suppression,
//! d'indentation et de désindentation : elle suit son bloc (comme dans MS
//! Project, chaque ligne sélectionnée ne bouge que d'un niveau).

use planrs_automation::command::{LinkKey, LinkSpec, TaskAdd, TaskDelete, TaskMove, TaskOne};
use planrs_automation::{Batch, Command, Placement, TaskRef};
use planrs_core::{Project, TaskUid};

use crate::action::Action;
use crate::app::{App, EditOrigin};
use crate::i18n::{Arg, TextId, Texts};
use crate::selection::Selection;

/// Commande de structure, appliquée à la sélection courante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureOp {
    /// Inser : nouvelle tâche juste avant la première tâche sélectionnée
    /// (ordre des ID), à la fin sans sélection.
    InsertBefore,
    /// Suppr : supprime les tâches sélectionnées ; une récapitulative est
    /// supprimée avec ses sous-tâches (`cascade`) après confirmation.
    Delete,
    /// Alt+Maj+→ : indente les tâches sélectionnées, dans l'ordre des ID.
    Indent,
    /// Alt+Maj+← : désindente les tâches sélectionnées, dans l'ordre des ID.
    Outdent,
    /// Ctrl+F2 : liens FS en chaîne (ordre des clics, ou des ID pour une
    /// plage).
    Link,
    /// Ctrl+Maj+F2 : retire tous les liens entre tâches sélectionnées.
    Unlink,
    /// Alt+Maj+↑ (J7, H-UI-29) : monte les lignes sélectionnées au-dessus
    /// de la tâche sœur précédente (`task.move`).
    MoveUp,
    /// Alt+Maj+↓ (J7, H-UI-29) : descend les lignes sélectionnées sous la
    /// tâche sœur suivante (`task.move`).
    MoveDown,
}

/// Lot préparé par [`plan`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructurePlan {
    /// Lot à appliquer.
    pub batch: Batch,
    /// Nombre de tâches supprimées, sous-tâches comprises, si le lot
    /// supprime au moins une récapitulative : l'interface demande d'abord
    /// confirmation (ADR-0012 § 0 : `cascade:true` obligatoire).
    pub confirm_cascade: Option<usize>,
}

/// Suppression en attente de confirmation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingDelete {
    /// Lot de suppression (avec `cascade`).
    pub batch: Batch,
    /// Nombre de tâches supprimées, sous-tâches comprises.
    pub count: usize,
    /// Clé de cache du document au moment de la demande : une édition, une
    /// annulation ou un autre document entre-temps rendent la demande
    /// caduque.
    pub cache_key: (u64, u64),
}

/// Lot à une suite d'opérations.
fn batch(ops: Vec<Command>) -> Batch {
    Batch {
        format_version: Batch::FORMAT_VERSION,
        lang: None,
        ops,
        label: None,
    }
}

/// Niveau de la tâche à la position `pos`.
fn level_at(project: &Project, pos: usize) -> Option<u16> {
    project.tasks.at(pos).map(|t| t.outline_level.get())
}

/// Vrai si la tâche à `pos` est récapitulative (R-052 : la tâche non vide
/// suivante est plus profonde, lignes vides intercalées ignorées). Même
/// règle que l'automatisation et le moteur.
fn is_summary_at(project: &Project, pos: usize) -> bool {
    planrs_automation::command::is_summary_at(&project.tasks, pos)
}

/// Fin (exclue) du bloc de la tâche à `pos` : tâches suivantes plus
/// profondes, lignes vides comprises (même parcours que `task.delete`).
fn block_end(project: &Project, pos: usize) -> usize {
    let Some(level) = level_at(project, pos) else {
        return pos;
    };
    let start = pos.saturating_add(1);
    let mut end = start;
    for (q, task) in project.tasks.iter().enumerate().skip(start) {
        if task.is_null {
            continue;
        }
        if task.outline_level.get() <= level {
            break;
        }
        end = q.saturating_add(1);
    }
    end
}

/// Positions des tâches sélectionnées existantes, dans l'ordre des ID.
fn positions(project: &Project, selection: &Selection) -> Vec<(usize, TaskUid)> {
    let mut out: Vec<(usize, TaskUid)> = selection
        .uids()
        .iter()
        .filter_map(|uid| project.tasks.position(*uid).map(|pos| (pos, *uid)))
        .collect();
    out.sort_unstable();
    out
}

/// Tâches sélectionnées sans ancêtre sélectionné, dans l'ordre des ID, avec
/// leur position.
fn top_level(project: &Project, selection: &Selection) -> Vec<(usize, TaskUid)> {
    let mut out = Vec::new();
    let mut covered_until = 0_usize;
    for (pos, uid) in positions(project, selection) {
        if pos < covered_until {
            continue;
        }
        covered_until = block_end(project, pos);
        out.push((pos, uid));
    }
    out
}

/// Prépare le lot de `op` sur `selection` ; `None` s'il n'y a rien à faire
/// (sélection vide, un seul maillon à lier, aucun lien entre les tâches
/// sélectionnées).
#[must_use]
pub fn plan(op: StructureOp, project: &Project, selection: &Selection) -> Option<StructurePlan> {
    let ops = match op {
        StructureOp::InsertBefore => {
            let at = positions(project, selection)
                .first()
                .map_or(Placement::End, |(_, uid)| Placement::Before(TaskRef::Uid(*uid)));
            vec![Command::TaskAdd(TaskAdd {
                name: String::new(),
                label: None,
                at: Some(at),
                duration: None,
                milestone: None,
                notes: None,
                start: None,
                finish: None,
            })]
        }
        StructureOp::Delete => return plan_delete(project, selection),
        StructureOp::MoveUp | StructureOp::MoveDown => {
            plan_move(project, selection, op == StructureOp::MoveUp)
                .into_iter()
                .collect()
        }
        StructureOp::Indent | StructureOp::Outdent => top_level(project, selection)
            .into_iter()
            .map(|(_, uid)| {
                let one = TaskOne {
                    uid: TaskRef::Uid(uid),
                };
                if op == StructureOp::Indent {
                    Command::TaskIndent(one)
                } else {
                    Command::TaskOutdent(one)
                }
            })
            .collect(),
        StructureOp::Link => {
            let chain: Vec<TaskUid> = selection
                .click_order()
                .iter()
                .copied()
                .filter(|uid| project.tasks.contains(*uid))
                .collect();
            chain
                .windows(2)
                .filter_map(|pair| match pair {
                    [pred, succ] => Some(Command::LinkAdd(LinkSpec {
                        pred: TaskRef::Uid(*pred),
                        succ: TaskRef::Uid(*succ),
                        kind: Some(planrs_automation::command::LinkTypeJson::Fs),
                        lag: None,
                    })),
                    _ => None,
                })
                .collect()
        }
        StructureOp::Unlink => positions(project, selection)
            .into_iter()
            .filter_map(|(pos, succ)| project.tasks.at(pos).map(|task| (task, succ)))
            .flat_map(|(task, succ)| {
                task.predecessors
                    .iter()
                    .filter(|link| selection.contains(link.predecessor))
                    .map(move |link| {
                        Command::LinkRemove(LinkKey {
                            pred: TaskRef::Uid(link.predecessor),
                            succ: TaskRef::Uid(succ),
                        })
                    })
            })
            .collect(),
    };
    (!ops.is_empty()).then(|| StructurePlan {
        batch: batch(ops),
        confirm_cascade: None,
    })
}

/// Position de la première tâche non vide à partir de `from` (incluse).
fn next_non_null(project: &Project, from: usize) -> Option<usize> {
    project
        .tasks
        .iter()
        .enumerate()
        .skip(from)
        .find(|(_, task)| !task.is_null)
        .map(|(pos, _)| pos)
}

/// Position de la dernière tâche non vide strictement avant `before`.
fn previous_non_null(project: &Project, before: usize) -> Option<usize> {
    (0..before)
        .rev()
        .find(|pos| project.tasks.at(*pos).is_some_and(|task| !task.is_null))
}

/// Monter / descendre (H-UI-29, comme Alt+Maj+↑/↓ de MS Project, S61) :
/// les tâches sélectionnées sans ancêtre sélectionné doivent former une
/// suite de sœurs consécutives (même niveau, blocs contigus). Monter
/// déplace la sœur précédente après la dernière d'entre elles ; descendre
/// déplace la sœur suivante avant la première : **une** commande
/// `task.move`, le bloc sélectionné garde son ordre et sa hiérarchie. Pas
/// de sœur dans ce sens (premier ou dernier enfant, bord du projet) ou
/// sélection non contiguë : rien (hypothèse : au bord, rien ne bouge,
/// comme la navigation).
fn plan_move(project: &Project, selection: &Selection, up: bool) -> Option<Command> {
    let top = top_level(project, selection);
    let (first, _) = *top.first()?;
    let (last, _) = *top.last()?;
    let level = level_at(project, first)?;
    // Sœurs consécutives : chaque bloc commence où finit le précédent.
    let mut expected = first;
    for (pos, _) in &top {
        if *pos != expected || level_at(project, *pos) != Some(level) {
            return None;
        }
        expected = next_non_null(project, block_end(project, *pos)).unwrap_or(usize::MAX);
    }
    let first_uid = project.tasks.at(first)?.uid();
    let last_uid = project.tasks.at(last)?.uid();
    let (neighbour, to) = if up {
        // Sœur précédente : la tâche de même niveau qui précède, sans
        // passer par le parent (niveau inférieur).
        let mut pos = first;
        let found = loop {
            let prev = previous_non_null(project, pos)?;
            let prev_level = level_at(project, prev)?;
            if prev_level < level {
                return None;
            }
            if prev_level == level {
                break prev;
            }
            pos = prev;
        };
        (
            project.tasks.at(found)?.uid(),
            Placement::After(TaskRef::Uid(last_uid)),
        )
    } else {
        let next = next_non_null(project, block_end(project, last))?;
        if level_at(project, next)? != level {
            return None;
        }
        (
            project.tasks.at(next)?.uid(),
            Placement::Before(TaskRef::Uid(first_uid)),
        )
    };
    Some(Command::TaskMove(TaskMove {
        uid: TaskRef::Uid(neighbour),
        to,
    }))
}

/// Lot de suppression : une récapitulative part avec son bloc
/// (`cascade:true`), les tâches de ce bloc ne sont pas reprises.
fn plan_delete(project: &Project, selection: &Selection) -> Option<StructurePlan> {
    let mut ops = Vec::new();
    let mut count = 0_usize;
    let mut cascade_any = false;
    for (pos, uid) in top_level(project, selection) {
        let cascade = is_summary_at(project, pos);
        cascade_any |= cascade;
        count = count.saturating_add(block_end(project, pos).saturating_sub(pos));
        ops.push(Command::TaskDelete(TaskDelete {
            uid: TaskRef::Uid(uid),
            cascade,
        }));
    }
    (!ops.is_empty()).then(|| StructurePlan {
        batch: batch(ops),
        confirm_cascade: cascade_any.then_some(count),
    })
}

impl App {
    /// Applique une commande de structure à la sélection courante. Une
    /// suppression de récapitulative attend la confirmation
    /// ([`App::pending_delete`]) ; rien ne se passe tant qu'une
    /// confirmation est ouverte.
    pub fn run_structure(&mut self, op: StructureOp) {
        self.sync_selection();
        if self.pending_delete.is_some() {
            return;
        }
        let Some(document) = &self.document else {
            return;
        };
        let Some(plan) = plan(op, document.project(), &self.selected) else {
            return;
        };
        if let Some(count) = plan.confirm_cascade {
            self.pending_delete = Some(PendingDelete {
                batch: plan.batch,
                count,
                cache_key: document.cache_key(),
            });
            return;
        }
        // Un refus est affiché par `execute`.
        if let Ok(outcome) = self.execute(&plan.batch, EditOrigin::User)
            && op == StructureOp::InsertBefore
            && let Some(created) = outcome.created.first()
        {
            self.selection = Some(*created);
            self.sync_selection();
        }
    }

    /// Réponse à la confirmation de suppression : `true` applique le lot
    /// (s'il n'est pas devenu caduc), `false` l'abandonne.
    pub fn answer_delete(&mut self, confirmed: bool) {
        let Some(pending) = self.pending_delete.take() else {
            return;
        };
        let current = self.document.as_ref().map(crate::app::OpenDocument::cache_key);
        if confirmed && current == Some(pending.cache_key) {
            // Un refus est affiché par `execute`.
            drop(self.execute(&pending.batch, EditOrigin::User));
        }
    }

    /// Abandonne une confirmation devenue caduque (document modifié, fermé
    /// ou remplacé) : la fenêtre ne reste pas ouverte sur un autre état.
    pub(crate) fn drop_stale_delete(&mut self) {
        let current = self.document.as_ref().map(crate::app::OpenDocument::cache_key);
        if self
            .pending_delete
            .as_ref()
            .is_some_and(|pending| Some(pending.cache_key) != current)
        {
            self.pending_delete = None;
        }
    }

    /// Suppression en attente de confirmation, s'il y en a une.
    #[must_use]
    pub const fn pending_delete(&self) -> Option<&PendingDelete> {
        self.pending_delete.as_ref()
    }
}

/// Fenêtre modale de confirmation d'une suppression de récapitulative :
/// « Supprimer » pousse `ConfirmDelete(true)`, « Annuler », Échap ou un clic
/// hors de la fenêtre `ConfirmDelete(false)`.
pub fn confirm_modal(ctx: &egui::Context, texts: &Texts, pending: &PendingDelete, actions: &mut Vec<Action>) {
    let count = i64::try_from(pending.count).unwrap_or(i64::MAX);
    let response = egui::Modal::new(egui::Id::unique("planrs.confirm-delete")).show(ctx, |ui| {
        ui.set_max_width(420.0);
        ui.heading(texts.get(TextId::ConfirmDeleteTitle));
        ui.label(texts.get_args(TextId::ConfirmDeleteText, &[("count", Arg::Count(count))]));
        ui.add_space(8.0);
        let mut answer = None;
        ui.horizontal(|ui| {
            if ui.button(texts.get(TextId::ConfirmDeleteYes)).clicked() {
                answer = Some(true);
            }
            if ui.button(texts.get(TextId::ConfirmDeleteNo)).clicked() {
                answer = Some(false);
            }
        });
        answer
    });
    match response.inner {
        Some(answer) => actions.push(Action::ConfirmDelete(answer)),
        None if response.should_close() => actions.push(Action::ConfirmDelete(false)),
        None => {}
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
    use planrs_automation::{Batch, Command, Placement, TaskRef};
    use planrs_core::{Arena, Moment, Project, ProjectOptions, Task, TaskUid};

    use super::{StructureOp, block_end, is_summary_at, plan};
    use crate::app::{App, Env};
    use crate::selection::{SelectMode, Selection};

    fn demo() -> App {
        let now: Moment = "2027-01-15T08:00".parse().unwrap();
        let mut app = App::new(crate::UiState::default(), Env::fixed(now));
        app.open_demo();
        app
    }

    fn project(app: &App) -> &Project {
        app.document.as_ref().unwrap().project()
    }

    fn uid_at(app: &App, pos: usize) -> TaskUid {
        project(app).tasks.at(pos).unwrap().uid()
    }

    /// Première récapitulative de l'exemple (position).
    fn first_summary(app: &App) -> usize {
        (0..project(app).tasks.len())
            .find(|pos| is_summary_at(project(app), *pos))
            .unwrap()
    }

    fn ops(batch: &Batch) -> Vec<String> {
        batch.ops.iter().map(|op| op.id().to_owned()).collect()
    }

    #[test]
    fn insert_before_first_in_id_order_or_end() {
        let app = demo();
        let rows = app.visible_rows().unwrap();
        let (a, b) = (uid_at(&app, 4), uid_at(&app, 2));
        let mut sel = Selection::single(Some(a));
        sel.click(b, SelectMode::Toggle, rows);
        let plan = plan(StructureOp::InsertBefore, project(&app), &sel).unwrap();
        assert_eq!(plan.confirm_cascade, None);
        match &plan.batch.ops[..] {
            [Command::TaskAdd(add)] => assert_eq!(add.at, Some(Placement::Before(TaskRef::Uid(b)))),
            other => panic!("{other:?}"),
        }
        let plan = super::plan(StructureOp::InsertBefore, project(&app), &Selection::default()).unwrap();
        match &plan.batch.ops[..] {
            [Command::TaskAdd(add)] => assert_eq!(add.at, Some(Placement::End)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn delete_summary_skips_its_block_and_asks() {
        let app = demo();
        let rows = app.visible_rows().unwrap();
        let s = first_summary(&app);
        let end = block_end(project(&app), s);
        assert!(end > s + 1);
        let mut sel = Selection::single(Some(uid_at(&app, s)));
        sel.click(uid_at(&app, s + 1), SelectMode::Toggle, rows);
        let plan = plan(StructureOp::Delete, project(&app), &sel).unwrap();
        assert_eq!(plan.confirm_cascade, Some(end - s));
        match &plan.batch.ops[..] {
            [Command::TaskDelete(d)] => {
                assert_eq!(d.uid, TaskRef::Uid(uid_at(&app, s)));
                assert!(d.cascade);
            }
            other => panic!("{other:?}"),
        }
        // Une feuille : sans confirmation, sans cascade.
        let leaf = (0..project(&app).tasks.len())
            .find(|p| !is_summary_at(project(&app), *p))
            .unwrap();
        let plan = super::plan(
            StructureOp::Delete,
            project(&app),
            &Selection::single(Some(uid_at(&app, leaf))),
        )
        .unwrap();
        assert_eq!(plan.confirm_cascade, None);
        assert!(matches!(&plan.batch.ops[..], [Command::TaskDelete(d)] if !d.cascade));
    }

    #[test]
    fn nothing_to_do_gives_none() {
        let app = demo();
        let p = project(&app);
        for op in [
            StructureOp::Delete,
            StructureOp::Indent,
            StructureOp::Outdent,
            StructureOp::Link,
            StructureOp::Unlink,
        ] {
            assert_eq!(plan(op, p, &Selection::default()), None, "{op:?}");
        }
        // Un seul maillon : rien à lier.
        assert_eq!(
            plan(StructureOp::Link, p, &Selection::single(Some(uid_at(&app, 1)))),
            None
        );
        // Tâche inconnue : ignorée.
        assert_eq!(
            plan(
                StructureOp::Indent,
                p,
                &Selection::single(Some(TaskUid::new(999_999)))
            ),
            None
        );
    }

    #[test]
    fn link_and_unlink_ops() {
        let app = demo();
        let rows = app.visible_rows().unwrap();
        let (a, b, c) = (uid_at(&app, 6), uid_at(&app, 2), uid_at(&app, 4));
        let mut sel = Selection::single(Some(a));
        sel.click(b, SelectMode::Toggle, rows);
        sel.click(c, SelectMode::Toggle, rows);
        let plan = plan(StructureOp::Link, project(&app), &sel).unwrap();
        assert_eq!(ops(&plan.batch), ["link.add", "link.add"]);
        let pairs: Vec<(TaskRef, TaskRef)> = plan
            .batch
            .ops
            .iter()
            .map(|op| match op {
                Command::LinkAdd(spec) => (spec.pred.clone(), spec.succ.clone()),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            pairs,
            [
                (TaskRef::Uid(a), TaskRef::Uid(b)),
                (TaskRef::Uid(b), TaskRef::Uid(c))
            ]
        );
        // Délier : seulement les liens dont les deux bouts sont sélectionnés.
        let p = project(&app);
        let (succ_pos, succ) = p
            .tasks
            .iter()
            .enumerate()
            .find_map(|(pos, t)| (!t.predecessors.is_empty()).then_some((pos, t.uid())))
            .unwrap();
        let pred = p.tasks.at(succ_pos).unwrap().predecessors[0].predecessor;
        let mut sel = Selection::single(Some(succ));
        assert_eq!(plan_unlink_count(p, &sel), 0);
        sel.click(pred, SelectMode::Toggle, rows);
        assert!(plan_unlink_count(p, &sel) >= 1);
    }

    /// R-052 : une ligne vide entre une tâche et sa première sous-tâche ne
    /// la rend pas feuille ; sa suppression demande toujours la cascade.
    #[test]
    fn summary_ignores_null_row_before_child() {
        let app = demo();
        let s = first_summary(&app);
        let mut p = project(&app).clone();
        let mut null = Task::new(TaskUid::new(999_999), String::new(), &ProjectOptions::default());
        null.outline_level = p.tasks.at(s).unwrap().outline_level;
        null.is_null = true;
        let mut tasks: Vec<Task> = p.tasks.iter().cloned().collect();
        tasks.insert(s + 1, null);
        p.tasks = Arena::try_from(tasks).unwrap();
        assert!(is_summary_at(&p, s));
        assert!(!is_summary_at(&p, s + 1));
        let uid = p.tasks.at(s).unwrap().uid();
        let plan = plan(StructureOp::Delete, &p, &Selection::single(Some(uid))).unwrap();
        assert_eq!(plan.confirm_cascade, Some(block_end(&p, s) - s));
        assert!(matches!(&plan.batch.ops[..], [Command::TaskDelete(d)] if d.cascade));
    }

    /// H-UI-29 : monter = la sœur précédente passe après le bloc ;
    /// descendre = la sœur suivante passe avant ; une commande ; rien au
    /// bord ni pour une sélection non contiguë.
    #[test]
    fn move_up_down_neighbour_sibling() {
        let app = demo();
        let rows = app.visible_rows().unwrap();
        let p = project(&app);
        let s = first_summary(&app);
        // Deux premières sous-tâches de la première récapitulative.
        let (first, second) = (s + 1, s + 2);
        assert!(!is_summary_at(p, first));
        let one = |pos: usize| Selection::single(Some(uid_at(&app, pos)));
        // Premier enfant : rien à monter (le parent n'est pas une sœur).
        assert_eq!(plan(StructureOp::MoveUp, p, &one(first)), None);
        let plan_up = plan(StructureOp::MoveUp, p, &one(second)).unwrap();
        match &plan_up.batch.ops[..] {
            [Command::TaskMove(m)] => {
                assert_eq!(m.uid, TaskRef::Uid(uid_at(&app, first)));
                assert_eq!(m.to, Placement::After(TaskRef::Uid(uid_at(&app, second))));
            }
            other => panic!("{other:?}"),
        }
        let plan_down = plan(StructureOp::MoveDown, p, &one(first)).unwrap();
        match &plan_down.batch.ops[..] {
            [Command::TaskMove(m)] => {
                assert_eq!(m.uid, TaskRef::Uid(uid_at(&app, second)));
                assert_eq!(m.to, Placement::Before(TaskRef::Uid(uid_at(&app, first))));
            }
            other => panic!("{other:?}"),
        }
        // Bloc de deux sœurs : la sœur suivante passe avant le bloc.
        let mut pair = one(first);
        pair.click(uid_at(&app, second), SelectMode::Range, rows);
        if let Some(plan) = plan(StructureOp::MoveDown, p, &pair) {
            assert!(matches!(&plan.batch.ops[..], [Command::TaskMove(m)]
                if m.to == Placement::Before(TaskRef::Uid(uid_at(&app, first)))));
        }
        // Sélection non contiguë (récapitulative et une tâche hors de son
        // bloc) : rien.
        let mut gap = one(s);
        gap.click(uid_at(&app, block_end(p, s) + 1), SelectMode::Toggle, rows);
        assert_eq!(plan(StructureOp::MoveUp, p, &gap), None);
        assert_eq!(plan(StructureOp::MoveDown, p, &Selection::default()), None);
        // Dernière tâche du projet : rien à descendre.
        let last = p.tasks.len() - 1;
        assert_eq!(plan(StructureOp::MoveDown, p, &one(last)), None);
    }

    fn plan_unlink_count(p: &Project, sel: &Selection) -> usize {
        plan(StructureOp::Unlink, p, sel).map_or(0, |plan| plan.batch.ops.len())
    }
}
