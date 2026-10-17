//! Panneau repliable des diagnostics d'import et de calcul (version
//! minimale) : une ligne par diagnostic, triée par gravité (erreurs, puis
//! avertissements, puis informations), dans l'ordre d'émission à gravité
//! égale.
//!
//! Les entrées viennent toutes prêtes de `FrameCtx::diagnostics`
//! (`DiagnosticsView`, construite une fois par document et par langue par
//! `App`) : aucune mise en texte ni aucun tri pendant l'image. Les textes
//! sont le `Display` des diagnostics (en français, non traduits ; « → »
//! remplacé par « -> », absent des polices d'egui : voir `docs/ui-needs.md`,
//! N5).

use planrs_core::Severity;

use super::FrameCtx;
use crate::i18n::{Arg, TextId};

/// Panneau des diagnostics, en bas de la fenêtre (au-dessus de la barre
/// d'état), redimensionnable. Sans document : « Aucun document ouvert ».
/// Document sans diagnostic : rappel des compteurs (« aucune erreur »).
pub fn panel(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>) {
    let texts = fc.texts;
    let has_doc = fc.doc.is_some();
    let entries = &fc.diagnostics.entries;
    egui::Panel::bottom("planrs_diagnostics")
        .resizable(true)
        .default_size(fc.metrics.row_h * 7.0)
        .show(ui, |ui| {
            if !has_doc {
                ui.label(texts.get(TextId::StatusNoDocument));
                return;
            }
            if entries.is_empty() {
                for id in [TextId::StatusErrors, TextId::StatusWarnings] {
                    ui.label(texts.get_args(id, &[("count", Arg::Count(0))]));
                }
                return;
            }
            // Lignes virtualisées (une ligne de texte chacune, tronquée ; le
            // texte entier s'affiche au survol) : coût par image borné par la
            // hauteur du panneau, pas par le nombre de diagnostics.
            let row_h = ui.text_style_height(&egui::TextStyle::Body);
            egui::ScrollArea::vertical().auto_shrink([false, true]).show_rows(
                ui,
                row_h,
                entries.len(),
                |ui, range| {
                    for (severity, text) in entries.get(range).unwrap_or_default() {
                        let color = match severity {
                            Severity::Error => ui.visuals().error_fg_color,
                            Severity::Warning => ui.visuals().warn_fg_color,
                            Severity::Info => ui.visuals().text_color(),
                        };
                        ui.add(egui::Label::new(egui::RichText::new(text.as_str()).color(color)).truncate());
                    }
                },
            );
        });
}
