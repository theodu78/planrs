//! Couper, copier, coller de lignes dans l'interface (H-UI-39, ADR-0016
//! D3). egui livre `Event::Copy`, `Event::Cut` et `Event::Paste(String)`
//! (jamais Ctrl/Cmd+C/X/V comme touches) ; le tampon interne et son
//! empreinte permettent de reconnaître au collage le dernier texte émis
//! (collage complet, R-244), sinon le collage externe réduit (R-256).
//! Contrat posé par le lot L0, câblé en L10.

use planrs_automation::clip::RowClip;

/// Dernière copie de l'application : tampon et empreinte du texte écrit
/// dans le presse-papiers du système.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClipboardSlot {
    /// Tampon interne.
    pub clip: RowClip,
    /// Empreinte FNV-1a du texte émis (`planrs_automation::clip::fingerprint`).
    pub fingerprint: u64,
}
