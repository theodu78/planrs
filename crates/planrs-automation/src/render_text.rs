//! Textes du planning pour le rendu (règle 3, ADR-0016 D7) : implémentation
//! de [`BarTextSource`] sur une session. Les noms, ressources, dates,
//! durées et pourcentages autour des barres, dans l'en-tête et le pied de
//! page sont formatés ici, à partir du modèle et du planning du moteur ;
//! `planrs-render` et `planrs-pdf` ne font que les placer.
//!
//! Contrat posé par le lot L0, implémenté en L4 : jusque-là aucun texte.

use planrs_core::TaskUid;
use planrs_render::geometry::{BarTextSource, Token};
use planrs_render::style::BarField;

use crate::session::Session;
use crate::text::Language;

/// Source des textes d'une session, dans une langue.
#[derive(Clone, Copy, Debug)]
pub struct SessionTexts<'a> {
    /// Session (projet et planning du moteur).
    pub session: &'a Session,
    /// Langue des textes.
    pub lang: Language,
}

impl BarTextSource for SessionTexts<'_> {
    fn bar_text(&self, uid: TaskUid, f: BarField) -> Option<String> {
        let _ = (uid, f);
        None
    }

    fn token(&self, t: Token) -> String {
        let _ = t;
        String::new()
    }
}
