//! Style du document : commandes `style.list`, `style.get`, `style.apply`
//! (ADR-0016 D7, H-UI-52, H-UI-59 à H-UI-61).
//!
//! Le style est une donnée du document (`Project.extras`, origine `planrs`,
//! clé `style` : modèle de base et surcharges), modifiée par `style.apply`
//! (annulable, marque le document modifié). Contrat posé par le lot L0 :
//! entrées du registre réservées au jalon 8 (`command.not_yet_available`),
//! exécution en L6.

use planrs_render::style::TemplateId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Modèle de mise en forme intégré, en JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TemplateJson {
    /// Apparence de J7.
    Classique,
    /// Sobre.
    Sobre,
    /// Contraste élevé.
    ContrasteEleve,
    /// Présentation client.
    Presentation,
    /// Chantier.
    Chantier,
    /// Impression noir et blanc.
    ImpressionNb,
}

impl From<TemplateJson> for TemplateId {
    fn from(t: TemplateJson) -> Self {
        match t {
            TemplateJson::Classique => Self::Classique,
            TemplateJson::Sobre => Self::Sobre,
            TemplateJson::ContrasteEleve => Self::ContrasteEleve,
            TemplateJson::Presentation => Self::Presentation,
            TemplateJson::Chantier => Self::Chantier,
            TemplateJson::ImpressionNb => Self::ImpressionNb,
        }
    }
}

impl From<TemplateId> for TemplateJson {
    fn from(t: TemplateId) -> Self {
        match t {
            TemplateId::Classique => Self::Classique,
            TemplateId::Sobre => Self::Sobre,
            TemplateId::ContrasteEleve => Self::ContrasteEleve,
            TemplateId::Presentation => Self::Presentation,
            TemplateId::Chantier => Self::Chantier,
            TemplateId::ImpressionNb => Self::ImpressionNb,
        }
    }
}

/// `style.apply` : applique un modèle, une surcharge, ou revient au modèle
/// par défaut des préférences (`reset`). Une entrée d'annulation par appel.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StyleApply {
    /// Modèle de base.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateJson>,
    /// Surcharge (`StylePatch` de `planrs-render`, lue et bornée par
    /// `parse_doc_style` : au plus 64 Kio).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<serde_json::Value>,
    /// Retire le style du document (modèle par défaut des préférences ;
    /// plus rien d'écrit dans le fichier).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reset: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_json_matches_render_keys() {
        for id in TemplateId::ALL {
            let json = TemplateJson::from(id);
            assert_eq!(TemplateId::from(json), id);
            assert_eq!(
                serde_json::to_value(json).ok(),
                Some(serde_json::Value::String(id.key().to_owned()))
            );
        }
    }
}
