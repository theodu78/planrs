//! Historique d'annulation : une entrée par lot commité (R-164, lot L2).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : rempli par Session (L2) ; attribut à retirer en L2"
)]

use serde::Serialize;

use crate::command::Batch;
use crate::delta::Delta;

/// Historique d'une session : pile d'annulation, pile de rétablissement,
/// entrées abandonnées par le budget.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct History {
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    dropped: u64,
}

/// Entrée d'historique.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryEntry {
    pub label: Option<String>,
    pub ops: Batch,
    pub delta: Delta,
    pub bytes: usize,
}

/// Vue de l'historique (`history.list`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct HistoryView {
    /// Entrées annulables, de la plus ancienne à la plus récente.
    pub entries: Vec<HistoryItem>,
    /// Entrées rétablissables, de la prochaine à la plus lointaine.
    pub redo: Vec<HistoryItem>,
    /// Entrées abandonnées par le budget (R-164).
    pub dropped: u64,
}

/// Résumé d'une entrée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HistoryItem {
    /// Libellé du lot.
    pub label: Option<String>,
    /// Nombre d'opérations.
    pub ops: usize,
    /// Taille approchée du delta.
    pub bytes: usize,
}

impl History {
    /// Position courante : entrées commitées depuis l'ouverture, abandonnées
    /// comprises (clé de `saved_at`).
    pub(crate) fn position(&self) -> u64 {
        let len = u64::try_from(self.undo.len()).unwrap_or(u64::MAX);
        self.dropped.saturating_add(len)
    }

    /// Nombre d'entrées annulables.
    pub(crate) fn len(&self) -> usize {
        self.undo.len()
    }

    /// Vue de l'historique.
    pub(crate) fn view(&self) -> HistoryView {
        let item = |e: &HistoryEntry| HistoryItem {
            label: e.label.clone(),
            ops: e.ops.ops.len(),
            bytes: e.bytes,
        };
        HistoryView {
            entries: self.undo.iter().map(item).collect(),
            redo: self.redo.iter().rev().map(item).collect(),
            dropped: self.dropped,
        }
    }
}
