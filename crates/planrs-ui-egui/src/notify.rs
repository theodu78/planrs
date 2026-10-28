//! Messages à l'utilisateur (seconde passe UX, ADR-0019 D1, H-UI-104).
//!
//! Trois voies : les **notifications** (en bas à droite, effacées seules
//! après [`lifetime`]), le **bandeau** (en haut, jusqu'à décision) et
//! l'**historique** (les [`MAX_HISTORY`] derniers messages, quelle que soit
//! leur voie). Le temps est celui d'egui (`ctx.input(|i| i.time)`, en
//! secondes) : déterministe sous kittest.
//!
//! Contrat posé par le lot L0 : la pile de messages d'`App` vit ici, avec
//! le comportement d'avant la passe (tout au bandeau, sans doublon, les
//! [`MAX_MESSAGES`] plus récents) tant que [`MessageStore::toasts_live`]
//! est faux ; le lot L1 l'active, dessine les notifications et l'historique
//! (`view::chrome`) et déduit la sorte de chaque message.

use planrs_core::Severity;

use crate::i18n::TextId;

/// Messages gardés au bandeau au plus (les plus récents).
pub const MAX_MESSAGES: usize = 3;

/// Notifications affichées au plus (la plus ancienne part, H-UI-104).
pub const MAX_TOASTS: usize = 3;

/// Messages gardés dans l'historique au plus (H-UI-104).
pub const MAX_HISTORY: usize = 20;

/// Sorte d'un message : décide de sa voie et de sa durée (H-UI-104).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MessageKind {
    /// Information (compte rendu d'un collage…) : notification de 5 s.
    #[default]
    Info,
    /// Avertissement : notification de 8 s.
    Warning,
    /// Refus d'une saisie ou d'une commande (erreur non bloquante) :
    /// notification de 10 s.
    Refusal,
    /// Attend une décision (confirmation de sortie, reprise, ouverture ou
    /// enregistrement échoués, état illisible) : bandeau, sans délai.
    Blocking,
}

/// Durée de vie d'une notification, en secondes ; `None` pour le bandeau
/// (`Blocking`), qui reste jusqu'à décision.
#[must_use]
pub const fn lifetime(kind: MessageKind) -> Option<f64> {
    match kind {
        MessageKind::Info => Some(5.0),
        MessageKind::Warning => Some(8.0),
        MessageKind::Refusal => Some(10.0),
        MessageKind::Blocking => None,
    }
}

/// Sorte d'un message publié par `App::push_titled` : ouverture,
/// enregistrement et export échoués ⇒ `Blocking` (une décision ou un
/// constat qui ne doit pas s'effacer seul) ; erreur d'une commande ⇒
/// `Refusal` ; avertissement ⇒ `Warning` ; le reste ⇒ `Info` (compte rendu
/// d'un collage compris, H-UI-104). Le lot L1 complète la liste des titres
/// bloquants (confirmation, reprise, état illisible).
#[must_use]
pub const fn kind_for(severity: Severity, title: Option<TextId>) -> MessageKind {
    match (title, severity) {
        (Some(TextId::OpenErrorTitle | TextId::SaveErrorTitle | TextId::ExportErrorTitle), _) => {
            MessageKind::Blocking
        }
        (Some(TextId::PasteReportTitle), _) | (_, Severity::Info) => MessageKind::Info,
        (_, Severity::Error) => MessageKind::Refusal,
        (_, Severity::Warning) => MessageKind::Warning,
    }
}

/// Message affiché à l'utilisateur.
#[derive(Clone, Debug, PartialEq)]
pub struct UserMessage {
    /// Sorte (voie et durée).
    pub kind: MessageKind,
    /// Gravité.
    pub severity: Severity,
    /// Titre affiché au-dessus du message (« Impossible d'ouvrir le
    /// fichier », « Modification refusée »), s'il y en a un.
    pub title: Option<TextId>,
    /// Texte déjà traduit, en langage d'utilisateur.
    pub text: String,
    /// Détail technique repliable (code, opération, texte du moteur ;
    /// H-UI-105), s'il y en a un.
    pub detail: Option<String>,
    /// Instant de publication (secondes d'egui), relancé par un doublon.
    pub posted_at: f64,
    /// Identifiant, unique dans la vie de la pile (posé par
    /// [`MessageStore::post`]).
    pub id: u64,
}

impl UserMessage {
    /// Message de sorte `kind`, sans détail ; instant et identifiant posés
    /// à la publication.
    #[must_use]
    pub const fn new(kind: MessageKind, severity: Severity, title: Option<TextId>, text: String) -> Self {
        Self {
            kind,
            severity,
            title,
            text,
            detail: None,
            posted_at: 0.0,
            id: 0,
        }
    }
}

/// Pile des messages : notifications ([`MAX_TOASTS`]), bandeau
/// ([`MAX_MESSAGES`]) et historique ([`MAX_HISTORY`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MessageStore {
    /// Notifications, la plus ancienne d'abord.
    toasts: Vec<UserMessage>,
    /// Bandeau, le plus ancien d'abord.
    banner: Vec<UserMessage>,
    /// Historique, le plus ancien d'abord.
    history: Vec<UserMessage>,
    /// Prochain identifiant.
    next_id: u64,
    /// Les notifications sont servies (lot L1) ; faux : tout va au bandeau,
    /// comme avant la passe.
    toasts_live: bool,
}

impl MessageStore {
    /// Pile dont les messages non bloquants vont aux notifications (lot L1,
    /// tests du contrat).
    #[must_use]
    pub fn with_toasts() -> Self {
        Self {
            toasts_live: true,
            ..Self::default()
        }
    }

    /// Les notifications sont-elles servies ?
    #[must_use]
    pub const fn toasts_live(&self) -> bool {
        self.toasts_live
    }

    /// Publie `m` : identifiant neuf ; un message de même texte déjà dans sa
    /// voie est retiré (le nouveau le remplace, minuterie relancée) ; voie
    /// pleine : le plus ancien part. Copie dans l'historique.
    pub fn post(&mut self, mut m: UserMessage) {
        self.next_id = self.next_id.wrapping_add(1);
        m.id = self.next_id;
        if !m.posted_at.is_finite() {
            m.posted_at = 0.0;
        }
        let (lane, cap) = if self.toasts_live && m.kind != MessageKind::Blocking {
            (&mut self.toasts, MAX_TOASTS)
        } else {
            (&mut self.banner, MAX_MESSAGES)
        };
        lane.retain(|old| old.text != m.text);
        lane.push(m.clone());
        let excess = lane.len().saturating_sub(cap);
        lane.drain(..excess);
        self.history.push(m);
        let excess = self.history.len().saturating_sub(MAX_HISTORY);
        self.history.drain(..excess);
    }

    /// Retire les notifications échues à l'instant `now` (secondes) ; la
    /// notification survolée `hovered` est gardée et sa minuterie relancée
    /// (survol = pause). `now` non fini : rien.
    pub fn expire(&mut self, now: f64, hovered: Option<u64>) {
        if !now.is_finite() {
            return;
        }
        for toast in &mut self.toasts {
            if Some(toast.id) == hovered {
                toast.posted_at = now;
            }
        }
        self.toasts
            .retain(|toast| lifetime(toast.kind).is_none_or(|life| now - toast.posted_at < life));
    }

    /// Ferme le message `id` (notification ou bandeau) ; les autres restent.
    pub fn dismiss(&mut self, id: u64) {
        self.toasts.retain(|m| m.id != id);
        self.banner.retain(|m| m.id != id);
    }

    /// Vide le bandeau (« Fermer le message » d'avant la passe).
    pub fn clear_banner(&mut self) {
        self.banner.clear();
    }

    /// Garde, au bandeau et dans les notifications, les messages pour
    /// lesquels `keep` est vrai (l'historique est inchangé).
    pub fn retain(&mut self, mut keep: impl FnMut(&UserMessage) -> bool) {
        self.toasts.retain(&mut keep);
        self.banner.retain(&mut keep);
    }

    /// Messages du bandeau, le plus ancien d'abord.
    #[must_use]
    pub fn banner(&self) -> &[UserMessage] {
        &self.banner
    }

    /// Notifications, la plus ancienne d'abord.
    #[must_use]
    pub fn toasts(&self) -> &[UserMessage] {
        &self.toasts
    }

    /// Historique, le plus ancien d'abord.
    #[must_use]
    pub fn history(&self) -> &[UserMessage] {
        &self.history
    }
}

/// Textes posés par le lot L0 pour les messages, servis par le lot L1
/// (notifications, historique, détails repliables) : titre de la cloche,
/// compte des non lus, « Détails », « Fermer », historique vide.
pub const L1_TEXTS: [TextId; 6] = [
    TextId::NotifyHistory,
    TextId::NotifyUnread,
    TextId::NotifyDetails,
    TextId::NotifyClose,
    TextId::NotifyHistoryEmpty,
    TextId::NotifyHistoryTitle,
];

/// Textes posés par le lot L0 pour les comptes rendus et refus du collage
/// et des liens (D2, D3), servis par le lot L2 : liens non repris
/// (reformulé, sans « un bout »), lignes collées sur des cellules, valeurs
/// hors de la table, paire déjà liée, « Remplir à droite ».
pub const L2_TEXTS: [TextId; 5] = [
    TextId::PasteLinksDropped,
    TextId::PasteRowsOnCells,
    TextId::PasteValuesOutside,
    TextId::LinkAlreadyLinked,
    TextId::MenuFillRight,
];

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::float_cmp,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use planrs_core::Severity;

    use super::{MAX_HISTORY, MAX_MESSAGES, MAX_TOASTS, MessageKind, MessageStore, UserMessage, lifetime};

    fn message(kind: MessageKind, text: &str, at: f64) -> UserMessage {
        UserMessage {
            posted_at: at,
            ..UserMessage::new(kind, Severity::Info, None, text.to_owned())
        }
    }

    /// H-UI-104 : 5, 8, 10 s ; `Blocking` jamais ; une notification part
    /// à l'échéance, pas avant ; le survol la garde.
    #[test]
    fn notify_lifetimes() {
        assert_eq!(lifetime(MessageKind::Info), Some(5.0));
        assert_eq!(lifetime(MessageKind::Warning), Some(8.0));
        assert_eq!(lifetime(MessageKind::Refusal), Some(10.0));
        assert_eq!(lifetime(MessageKind::Blocking), None);
        let mut store = MessageStore::with_toasts();
        store.post(message(MessageKind::Info, "info", 1.0));
        store.post(message(MessageKind::Warning, "warning", 1.0));
        store.post(message(MessageKind::Refusal, "refus", 1.0));
        store.post(message(MessageKind::Blocking, "décision", 1.0));
        store.expire(5.9, None);
        assert_eq!(store.toasts().len(), 3);
        store.expire(6.0, None);
        assert_eq!(store.toasts().len(), 2);
        store.expire(10.9, None);
        assert_eq!(store.toasts().len(), 1);
        let refusal = store.toasts()[0].id;
        // Survolée à 10,9 s : minuterie relancée, gardée à 11 s et 20,8 s.
        store.expire(10.95, Some(refusal));
        store.expire(20.8, None);
        assert_eq!(store.toasts().len(), 1);
        store.expire(f64::NAN, None);
        assert_eq!(store.toasts().len(), 1);
        store.expire(21.0, None);
        assert_eq!(store.toasts().len(), 0);
        store.expire(1.0e9, None);
        assert_eq!(store.banner().len(), 1, "le bandeau ne s'efface pas seul");
    }

    /// H-UI-104 : 3 notifications au plus (la plus ancienne part), 20
    /// messages d'historique, `dismiss` ne retire que son message ; un
    /// doublon remplace l'ancien ; L0 : sans notifications, tout va au
    /// bandeau (3 au plus), comme avant la passe.
    #[test]
    fn notify_store_bounds() {
        let mut store = MessageStore::with_toasts();
        for i in 0..25 {
            store.post(message(MessageKind::Info, &i.to_string(), 0.0));
        }
        let texts: Vec<&str> = store.toasts().iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, ["22", "23", "24"]);
        assert_eq!(store.toasts().len(), MAX_TOASTS);
        assert_eq!(store.history().len(), MAX_HISTORY);
        assert_eq!(store.history()[0].text, "5");
        store.post(message(MessageKind::Blocking, "a", 0.0));
        store.post(message(MessageKind::Blocking, "b", 0.0));
        let a = store.banner()[0].id;
        store.dismiss(a);
        let banner: Vec<&str> = store.banner().iter().map(|m| m.text.as_str()).collect();
        assert_eq!(banner, ["b"]);
        assert_eq!(store.toasts().len(), MAX_TOASTS);
        let middle = store.toasts()[1].id;
        store.dismiss(middle);
        let texts: Vec<&str> = store.toasts().iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, ["22", "24"]);
        // Doublon : remplace, identifiant neuf.
        store.post(message(MessageKind::Info, "22", 3.0));
        let texts: Vec<&str> = store.toasts().iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, ["24", "22"]);
        assert_eq!(store.toasts()[1].posted_at, 3.0);
        // Ids uniques.
        let mut ids: Vec<u64> = store.history().iter().map(|m| m.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), store.history().len());

        let mut legacy = MessageStore::default();
        assert!(!legacy.toasts_live());
        for i in 0..5 {
            legacy.post(message(MessageKind::Info, &i.to_string(), 0.0));
        }
        legacy.post(message(MessageKind::Info, "3", 0.0));
        let banner: Vec<&str> = legacy.banner().iter().map(|m| m.text.as_str()).collect();
        assert_eq!(banner, ["2", "4", "3"]);
        assert_eq!(legacy.banner().len(), MAX_MESSAGES);
        assert_eq!(legacy.toasts().len(), 0);
        legacy.retain(|m| m.text != "4");
        assert_eq!(legacy.banner().len(), 2);
        legacy.clear_banner();
        assert_eq!(legacy.banner().len(), 0);
        assert_eq!(legacy.history().len(), 6);
    }
}
