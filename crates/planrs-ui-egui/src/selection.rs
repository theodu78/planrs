//! Sélection de tâches de la vue principale (lot L14) : ensemble d'UID,
//! ancre et tâche active.
//!
//! Un clic simple sélectionne une seule tâche ; Ctrl/Cmd+clic ajoute ou
//! retire une tâche ([`SelectMode::Toggle`]) ; Maj+clic sélectionne la plage
//! de lignes entre l'ancre et la tâche cliquée ([`SelectMode::Range`]).
//! L'ordre des clics est gardé : `link.add` en chaîne (Ctrl+F2) suit l'ordre
//! des clics pour une sélection faite au Ctrl+clic, l'ordre des ID pour une
//! plage (une plage s'enregistre dans l'ordre des lignes **affichées** : une
//! vue filtrée, triée ou repliée ne prend que les lignes visibles, J8 lot
//! L5).
//!
//! La tâche active (`focus`) est celle de `App::selection` : l'application
//! resynchronise la sélection quand elles divergent (clic simple, tâche
//! supprimée, nouveau document).

use std::collections::BTreeSet;

use planrs_core::TaskUid;

use crate::row_view::{DisplayRow, VisibleRows};

/// Mode d'un clic de sélection avec modificateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectMode {
    /// Ctrl/Cmd+clic : ajoute la tâche, ou la retire si elle était
    /// sélectionnée.
    Toggle,
    /// Maj+clic : plage de lignes de l'ancre à la tâche cliquée.
    Range,
}

/// Tâches sélectionnées.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// UID sélectionnés.
    uids: BTreeSet<TaskUid>,
    /// Ancre d'une plage (Maj+clic) : dernier clic simple ou Ctrl+clic.
    anchor: Option<TaskUid>,
    /// Tâche active (dernière cliquée), toujours dans `uids` si présente.
    focus: Option<TaskUid>,
    /// Mêmes UID que `uids`, dans l'ordre des clics (ordre des lignes pour
    /// une plage).
    order: Vec<TaskUid>,
}

impl Selection {
    /// Sélection d'au plus une tâche.
    #[must_use]
    pub fn single(uid: Option<TaskUid>) -> Self {
        uid.map_or_else(Self::default, |uid| Self {
            uids: BTreeSet::from([uid]),
            anchor: Some(uid),
            focus: Some(uid),
            order: vec![uid],
        })
    }

    /// UID sélectionnés.
    #[must_use]
    pub const fn uids(&self) -> &BTreeSet<TaskUid> {
        &self.uids
    }

    /// Tâche active.
    #[must_use]
    pub const fn focus(&self) -> Option<TaskUid> {
        self.focus
    }

    /// Ancre des plages.
    #[must_use]
    pub const fn anchor(&self) -> Option<TaskUid> {
        self.anchor
    }

    /// Vrai si `uid` est sélectionnée.
    #[must_use]
    pub fn contains(&self, uid: TaskUid) -> bool {
        self.uids.contains(&uid)
    }

    /// Nombre de tâches sélectionnées.
    #[must_use]
    pub fn len(&self) -> usize {
        self.uids.len()
    }

    /// Aucune tâche sélectionnée.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.uids.is_empty()
    }

    /// UID dans l'ordre des clics (ordre des lignes pour une plage).
    #[must_use]
    pub fn click_order(&self) -> &[TaskUid] {
        &self.order
    }

    /// Applique un clic avec modificateur sur `uid` ; `rows` donne l'ordre
    /// des lignes affichées d'une plage. Une ancre absente de la vue (tâche
    /// supprimée ou masquée) ramène la plage à la seule tâche cliquée.
    pub fn click(&mut self, uid: TaskUid, mode: SelectMode, rows: VisibleRows<'_>) {
        match mode {
            SelectMode::Toggle => self.toggle(uid),
            SelectMode::Range => self.extend_to(uid, rows),
        }
    }

    /// Ctrl/Cmd+clic.
    fn toggle(&mut self, uid: TaskUid) {
        if self.uids.remove(&uid) {
            self.order.retain(|u| *u != uid);
            self.focus = self.order.last().copied();
            self.anchor = self.focus;
        } else {
            self.uids.insert(uid);
            self.order.push(uid);
            self.focus = Some(uid);
            self.anchor = Some(uid);
        }
    }

    /// Maj+clic : remplace la sélection par la plage ancre → `uid` (ancre
    /// gardée), dans l'ordre des lignes.
    fn extend_to(&mut self, uid: TaskUid, rows: VisibleRows<'_>) {
        let anchor = self.anchor.unwrap_or(uid);
        let (Some(a), Some(b)) = (rows.display_of(anchor), rows.display_of(uid)) else {
            *self = Self::single(Some(uid));
            return;
        };
        let (low, high) = if a <= b { (a, b) } else { (b, a) };
        let order: Vec<TaskUid> = (low.0..=high.0).filter_map(|d| rows.uid(DisplayRow(d))).collect();
        self.uids = order.iter().copied().collect();
        self.order = order;
        self.anchor = Some(anchor);
        self.focus = Some(uid);
    }

    /// Garde les tâches pour lesquelles `keep` est vrai (après une
    /// édition) ; une tâche active ou une ancre retirées passent à la
    /// dernière tâche cliquée restante.
    pub fn retain(&mut self, mut keep: impl FnMut(TaskUid) -> bool) {
        self.order.retain(|uid| keep(*uid));
        self.uids = self.order.iter().copied().collect();
        if self.focus.is_some_and(|uid| !self.uids.contains(&uid)) {
            self.focus = self.order.last().copied();
        }
        if self.anchor.is_some_and(|uid| !self.uids.contains(&uid)) {
            self.anchor = self.focus;
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use planrs_core::{Moment, TaskUid};

    use super::{SelectMode, Selection};
    use crate::app::{App, Env};
    use crate::row_view::{DisplayRow, RowView, VisibleRows};
    use crate::rows::RowModel;

    fn demo_rows() -> (RowModel, RowView) {
        let now: Moment = "2027-01-15T08:00".parse().unwrap();
        let mut app = App::new(crate::UiState::default(), Env::fixed(now));
        app.open_demo();
        (
            app.rows().expect("exemple chargé").clone(),
            app.row_view().clone(),
        )
    }

    fn uid_at(rows: VisibleRows<'_>, i: u32) -> TaskUid {
        rows.uid(DisplayRow(i)).unwrap()
    }

    #[test]
    fn single_and_empty() {
        let empty = Selection::single(None);
        assert!(empty.is_empty());
        assert_eq!(empty.focus(), None);
        let one = Selection::single(Some(TaskUid::new(4)));
        assert_eq!(one.len(), 1);
        assert_eq!(one.focus(), Some(TaskUid::new(4)));
        assert_eq!(one.anchor(), Some(TaskUid::new(4)));
        assert_eq!(one.click_order(), [TaskUid::new(4)]);
    }

    #[test]
    fn toggle_keeps_click_order() {
        let (model, view) = demo_rows();
        let rows = VisibleRows::new(&model, &view);
        let (a, b, c) = (uid_at(rows, 5), uid_at(rows, 1), uid_at(rows, 3));
        let mut s = Selection::single(Some(a));
        s.click(b, SelectMode::Toggle, rows);
        s.click(c, SelectMode::Toggle, rows);
        assert_eq!(s.click_order(), [a, b, c]);
        assert_eq!(s.focus(), Some(c));
        s.click(b, SelectMode::Toggle, rows);
        assert_eq!(s.click_order(), [a, c]);
        assert!(!s.contains(b));
        assert_eq!(s.focus(), Some(c));
        s.click(c, SelectMode::Toggle, rows);
        s.click(a, SelectMode::Toggle, rows);
        assert!(s.is_empty());
        assert_eq!(s.focus(), None);
    }

    #[test]
    fn range_in_row_order_from_anchor() {
        let (model, view) = demo_rows();
        let rows = VisibleRows::new(&model, &view);
        let mut s = Selection::single(Some(uid_at(rows, 6)));
        s.click(uid_at(rows, 3), SelectMode::Range, rows);
        let expected: Vec<TaskUid> = (3..=6).map(|i| uid_at(rows, i)).collect();
        assert_eq!(s.click_order(), expected.as_slice());
        assert_eq!(s.anchor(), Some(uid_at(rows, 6)));
        assert_eq!(s.focus(), Some(uid_at(rows, 3)));
        // Nouvelle plage depuis la même ancre : remplace la précédente.
        s.click(uid_at(rows, 8), SelectMode::Range, rows);
        let expected: Vec<TaskUid> = (6..=8).map(|i| uid_at(rows, i)).collect();
        assert_eq!(s.click_order(), expected.as_slice());
        // Ancre inconnue : la plage se réduit à la tâche cliquée.
        let mut s = Selection::single(Some(TaskUid::new(999_999)));
        s.click(uid_at(rows, 2), SelectMode::Range, rows);
        assert_eq!(s, Selection::single(Some(uid_at(rows, 2))));
    }

    #[test]
    fn retain_moves_focus_to_last_click() {
        let (model, view) = demo_rows();
        let rows = VisibleRows::new(&model, &view);
        let (a, b) = (uid_at(rows, 1), uid_at(rows, 2));
        let mut s = Selection::single(Some(a));
        s.click(b, SelectMode::Toggle, rows);
        s.retain(|uid| uid != b);
        assert_eq!(s.click_order(), [a]);
        assert_eq!(s.focus(), Some(a));
        assert_eq!(s.anchor(), Some(a));
        s.retain(|_| false);
        assert_eq!(s, Selection::default());
    }
}
