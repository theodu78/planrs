//! Lignes affichées ≠ lignes du modèle (ADR-0016 D1, P51, P53).
//!
//! [`RowView`] se pose au-dessus du `RowModel` (ordre des ID) : permutation
//! `order` (ligne affichée → ligne du modèle) et son inverse `display_of`,
//! liens renumérotés. Deux types distincts, [`DisplayRow`] et [`ModelRow`],
//! pour que le compilateur trouve chaque site d'indexation (règle 10 de J8 :
//! tout indice de ligne passe par [`RowView::model_row`] ou
//! [`RowView::display_row`]). Contrat posé par le lot L0, construit par
//! filtre, tri et repli en L5 ; jusque-là [`RowView::identity`].

/// Rang d'une ligne affichée (0 = première ligne visible de la table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisplayRow(pub u32);

/// Rang d'une ligne du modèle (position dans le projet, ordre des ID).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelRow(pub u32);

/// Lien entre deux lignes affichées (bouts masqués : lien absent).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisplayLink {
    /// Prédécesseur.
    pub from: DisplayRow,
    /// Successeur.
    pub to: DisplayRow,
    /// Indice du lien dans le `RowModel`.
    pub model_link: u32,
}

/// Marque « ligne masquée » de `display_of`.
pub const HIDDEN: u32 = u32::MAX;

/// Vue des lignes : permutation d'un sous-ensemble des lignes du modèle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowView {
    /// Ligne du modèle de chaque ligne affichée.
    order: Vec<ModelRow>,
    /// Ligne affichée de chaque ligne du modèle ([`HIDDEN`] : masquée).
    display_of: Vec<u32>,
    /// Liens visibles, triés par (plus petite ligne affichée, clé).
    links: Vec<DisplayLink>,
}

impl RowView {
    /// Vue identité de `len` lignes (ordre des ID, rien de masqué) ; plus de
    /// `u32::MAX - 1` lignes : vue vide (jamais atteint, le projet est
    /// borné bien en deçà).
    #[must_use]
    pub fn identity(len: usize) -> Self {
        let Ok(n) = u32::try_from(len) else {
            return Self::default();
        };
        if n == HIDDEN {
            return Self::default();
        }
        Self {
            order: (0..n).map(ModelRow).collect(),
            display_of: (0..n).collect(),
            links: Vec::new(),
        }
    }

    /// Nombre de lignes affichées.
    #[must_use]
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Aucune ligne affichée.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Ligne du modèle d'une ligne affichée.
    #[must_use]
    pub fn model_row(&self, d: DisplayRow) -> Option<ModelRow> {
        usize::try_from(d.0).ok().and_then(|i| self.order.get(i)).copied()
    }

    /// Ligne affichée d'une ligne du modèle (`None` : masquée ou hors vue).
    #[must_use]
    pub fn display_row(&self, m: ModelRow) -> Option<DisplayRow> {
        usize::try_from(m.0)
            .ok()
            .and_then(|i| self.display_of.get(i))
            .copied()
            .filter(|&d| d != HIDDEN)
            .map(DisplayRow)
    }

    /// Liens visibles.
    #[must_use]
    pub fn links(&self) -> &[DisplayLink] {
        &self.links
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P51 (vue identité) : `display_of` est l'inverse de `order`.
    #[test]
    fn row_view_identity_is_its_own_inverse() {
        let v = RowView::identity(4);
        assert_eq!(v.len(), 4);
        for i in 0..4 {
            assert_eq!(v.model_row(DisplayRow(i)), Some(ModelRow(i)));
            assert_eq!(v.display_row(ModelRow(i)), Some(DisplayRow(i)));
        }
        assert_eq!(v.model_row(DisplayRow(4)), None);
        assert_eq!(v.display_row(ModelRow(u32::MAX)), None);
        assert!(RowView::identity(0).is_empty());
        assert_eq!(v.links().len(), 0);
    }
}
