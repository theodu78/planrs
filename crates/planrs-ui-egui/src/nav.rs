//! Navigation au clavier dans une grille (J7, ADR-0015 D1 ; H-UI-13 à
//! H-UI-17, P45) : fonctions pures sur des indices, sans egui, partagées
//! par la table des tâches et la feuille des ressources.
//!
//! Le déplacement se calcule sur (indice de ligne, indice de colonne
//! visible) dans une [`GridShape`] ; la vue traduit ensuite l'indice en
//! [`CellCursor`] (UID ou ligne vide). Aucune indexation directe : toute
//! position rendue reste dans la grille (règle 10).
//!
//! Contrat J7 (lot L0) : types et signatures ; [`step`] borne seulement la
//! position et [`ensure_visible`] ne demande aucun défilement jusqu'au lot
//! L1.

/// Déplacement demandé (H-UI-13 à H-UI-15).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavMove {
    /// Ligne précédente (↑, Maj+Entrée).
    Up,
    /// Ligne suivante (↓, Entrée).
    Down,
    /// Colonne précédente (←, Maj+Tab).
    Left,
    /// Colonne suivante (→, Tab).
    Right,
    /// Page précédente (lignes visibles − 1).
    PageUp,
    /// Page suivante.
    PageDown,
    /// Premier champ de la ligne (Origine, Ctrl+←).
    RowStart,
    /// Dernier champ de la ligne (Fin, Ctrl+→).
    RowEnd,
    /// Première ligne (Ctrl+↑).
    FirstRow,
    /// Dernière ligne (Ctrl+↓).
    LastRow,
    /// Premier champ de la première ligne (Ctrl+Origine).
    First,
    /// Dernier champ de la dernière ligne (Ctrl+Fin).
    Last,
}

/// Position dans la grille : indice de ligne, indice de colonne visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridPos {
    /// Ligne (ligne vide de fin comprise).
    pub row: usize,
    /// Colonne visible.
    pub col: usize,
}

/// Forme de la grille.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridShape {
    /// Lignes, ligne vide de fin comprise.
    pub rows: usize,
    /// Colonnes visibles.
    pub cols: usize,
    /// Lignes visibles (pas d'une page).
    pub page: usize,
}

/// Position après le déplacement `mv` dans `shape` (P45 : toujours dans la
/// grille, idempotente au bord ; grille vide ⇒ (0, 0)).
///
/// Squelette J7 L0 : la position est seulement bornée à la grille (lot L1).
#[must_use]
pub fn step(pos: GridPos, mv: NavMove, shape: GridShape) -> GridPos {
    let _ = mv;
    GridPos {
        row: pos.row.min(shape.rows.saturating_sub(1)),
        col: pos.col.min(shape.cols.saturating_sub(1)),
    }
}

/// Ligne désignée par le curseur (défaut : la ligne vide).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowRef<U> {
    /// Ligne d'un élément (UID).
    Item(U),
    /// Ligne vide de fin (nouvelle tâche, nouvelle ressource).
    #[default]
    Blank,
}

/// Cellule courante d'une grille (remplace `TableEdit.blank` au lot L1) ;
/// défaut : ligne vide, colonne par défaut, aucune ancre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellCursor<U, C> {
    /// Ligne.
    pub row: RowRef<U>,
    /// Colonne.
    pub column: C,
    /// Ancre de l'extension de la sélection (Maj+flèches, H-UI-16).
    pub anchor: Option<U>,
}

impl<U, C: Default> Default for CellCursor<U, C> {
    fn default() -> Self {
        Self {
            row: RowRef::Blank,
            column: C::default(),
            anchor: None,
        }
    }
}

/// Défilement qui rend visible l'élément `index` (étendue `extent` par
/// élément, `len` éléments, fenêtre de `view` à partir de `offset`), avec
/// une ligne de marge sauf aux bords (H-UI-17) ; `None` : rien à faire, ou
/// valeur non finie.
///
/// Squelette J7 L0 : `None` (lot L1).
#[must_use]
pub fn ensure_visible(offset: f32, index: usize, extent: f32, view: f32, len: usize) -> Option<f32> {
    let _ = (offset, index, extent, view, len);
    None
}

#[cfg(test)]
mod tests {
    use super::{GridPos, GridShape, NavMove, step};

    #[test]
    fn nav_contract_step_stays_in_grid() {
        let shape = GridShape {
            rows: 3,
            cols: 2,
            page: 20,
        };
        let far = GridPos { row: 9, col: 9 };
        assert_eq!(step(far, NavMove::Down, shape), GridPos { row: 2, col: 1 });
        assert_eq!(step(far, NavMove::Up, GridShape::default()), GridPos::default());
    }
}
