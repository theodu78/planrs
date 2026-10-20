//! Grille commune de la table des tâches et de la feuille des ressources
//! (J7, ADR-0015 D1 ; extraite au lot L10) : peinture virtualisée, éditeur
//! de cellule et navigation de [`crate::nav`].
//!
//! Contrat J7 (lot L0) : le trait [`Sheet`] ; aucune implémentation.

use planrs_automation::Batch;
use planrs_automation::Session;
use planrs_automation::text::Language;

use crate::nav::RowRef;

/// Données d'une grille : lignes, colonnes, textes, saisie.
pub trait Sheet {
    /// Clé d'une ligne (UID).
    type Key: Copy + Ord;
    /// Colonne.
    type Col: Copy + Eq;

    /// Nombre de lignes, ligne vide de fin non comprise.
    fn len(&self) -> usize;

    /// Vrai sans ligne (hors ligne vide).
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clé de la ligne `row`, `None` hors grille.
    fn key(&self, row: usize) -> Option<Self::Key>;

    /// Colonnes visibles et leur largeur (points logiques).
    fn columns(&self) -> &[(Self::Col, u16)];

    /// Texte de la cellule (vide hors grille).
    fn text(&self, row: usize, col: Self::Col) -> &str;

    /// La colonne se saisit-elle ?
    fn editable(&self, col: Self::Col) -> bool;

    /// Lot de la saisie `text` dans la cellule, lu par `planrs-automation`.
    ///
    /// # Errors
    ///
    /// Message à afficher sous la cellule.
    fn commit(
        &self,
        row: RowRef<Self::Key>,
        col: Self::Col,
        text: &str,
        s: &Session,
        lang: Language,
    ) -> Result<Batch, String>;
}
