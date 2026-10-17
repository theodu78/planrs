//! Modèle de lignes partagé par la table et le Gantt.
//!
//! Tout ce qui coûte O(n) se fait une fois, hors de l'image : à l'ouverture
//! et au changement de langue, jamais pendant le défilement ni le zoom. Les
//! textes des cellules sont pré-formatés (ligne par ligne, une entrée par
//! colonne de [`Column::ALL`]) ; les liens sont triés par ligne minimale.

use std::ops::Range;

use planrs_core::{LinkType, Moment, TaskUid};

use crate::app::OpenDocument;
use crate::column::Column;
use crate::format::FormatLabels;

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
}

impl RowModel {
    /// Construit le modèle d'un document avec les libellés d'une langue.
    ///
    /// Corps provisoire : modèle vide.
    #[must_use]
    pub fn build(_doc: &OpenDocument, _labels: &FormatLabels) -> Self {
        Self::default()
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

    /// Indice de la ligne d'une tâche (O(n), sur sélection seulement).
    #[must_use]
    pub fn index_of(&self, uid: TaskUid) -> Option<usize> {
        self.rows.iter().position(|row| row.uid == uid)
    }

    /// Liens dont l'intervalle de lignes coupe `rows` (culling par
    /// intervalle : un lien qui traverse la fenêtre est gardé même si ses deux
    /// extrémités sont hors champ).
    ///
    /// Corps provisoire : parcours linéaire, sans exploiter le tri ni
    /// `max_span`.
    pub fn links_touching(&self, rows: Range<usize>) -> impl Iterator<Item = &LinkView> + '_ {
        let _ = self.max_span;
        self.links.iter().filter(move |link| {
            let low = usize::try_from(link.from.min(link.to)).unwrap_or(usize::MAX);
            let high = usize::try_from(link.from.max(link.to)).unwrap_or(usize::MAX);
            low < rows.end && high >= rows.start
        })
    }
}
