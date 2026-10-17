//! Panneau repliable des diagnostics d'import et de calcul (version
//! minimale) : une ligne par diagnostic, triée par gravité (erreurs, puis
//! avertissements, puis informations), dans l'ordre d'émission à gravité
//! égale. Les textes sont le `Display` des diagnostics (en français, non
//! traduits : voir `docs/ui-needs.md`).

use planrs_core::Severity;

use super::FrameCtx;
use crate::i18n::{Arg, TextId};

/// Rang d'affichage d'une gravité (la plus grave d'abord).
const fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

/// Diagnostics du document, triés par gravité (tri stable).
fn sorted_entries(fc: &FrameCtx<'_>) -> Vec<(Severity, String)> {
    let Some(doc) = fc.doc else {
        return Vec::new();
    };
    let mut entries: Vec<(Severity, String)> = doc
        .import_diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.severity, diagnostic.to_string()))
        .chain(
            doc.schedule
                .diagnostics
                .iter()
                .map(|diagnostic| (diagnostic.severity(), diagnostic.to_string())),
        )
        .collect();
    entries.sort_by_key(|(severity, _)| rank(*severity));
    entries
}

/// Panneau des diagnostics du document ouvert, en bas de la fenêtre
/// (au-dessus de la barre d'état), redimensionnable. Sans diagnostic, il
/// rappelle les compteurs (« aucune erreur »).
pub fn panel(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let entries = sorted_entries(fc);
    let texts = fc.texts;
    egui::Panel::bottom("planrs_diagnostics")
        .resizable(true)
        .default_size(fc.metrics.row_h * 7.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if entries.is_empty() {
                        for id in [TextId::StatusErrors, TextId::StatusWarnings] {
                            ui.label(texts.get_args(id, &[("count", Arg::Count(0))]));
                        }
                    }
                    for (severity, text) in &entries {
                        let color = match severity {
                            Severity::Error => ui.visuals().error_fg_color,
                            Severity::Warning => ui.visuals().warn_fg_color,
                            Severity::Info => ui.visuals().text_color(),
                        };
                        ui.label(egui::RichText::new(text.as_str()).color(color));
                    }
                });
        });
}
