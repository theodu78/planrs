//! Bandeau de reprise après plantage (J7, lot L15) : « Récupérer
//! « plan.xml » (3 modifications) », Restaurer / Ignorer / Plus tard.
//!
//! Un orphelin à la fois (le premier de la liste de l'application) ; si un
//! point de reprise a eu lieu, le bandeau le dit : l'historique antérieur
//! n'est plus annulable après la restauration. Desktop seulement (le web n'a
//! pas de journal) ; le bandeau lui-même ne dépend d'aucune plateforme.

use crate::action::{Action, RecoverChoice};
use crate::i18n::{Arg, TextId};
use crate::view::FrameCtx;

/// Orphelin proposé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveryOffer<'a> {
    /// Identifiant du journal.
    pub id: &'a str,
    /// Nom du document.
    pub name: &'a str,
    /// Modifications journalisées depuis la base.
    pub entries: usize,
    /// Un point de reprise a eu lieu.
    pub checkpointed: bool,
}

/// Dessine le bandeau de reprise (rien sans orphelin).
pub fn recovery_banner(ui: &mut egui::Ui, fc: &mut FrameCtx<'_>, offer: Option<RecoveryOffer<'_>>) {
    let Some(offer) = offer else {
        return;
    };
    let texts = fc.texts;
    let fg = fc.palette.text_strong;
    let frame = egui::Frame::side_top_panel(ui.style()).fill(fc.palette.message_warning_bg);
    egui::Panel::top("planrs_recovery_banner")
        .frame(frame)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let count = i64::try_from(offer.entries).unwrap_or(i64::MAX);
                let title = texts.get_args(
                    TextId::RecoveryTitle,
                    &[
                        ("name", Arg::Str(offer.name.into())),
                        ("count", Arg::Count(count)),
                    ],
                );
                ui.label(egui::RichText::new(title).strong().color(fg));
                if offer.checkpointed {
                    ui.label(egui::RichText::new(texts.get(TextId::RecoveryCheckpointed)).color(fg));
                }
                let id = offer.id;
                let choices = [
                    (TextId::RecoveryRestore, RecoverChoice::Restore(id.to_owned())),
                    (TextId::RecoveryDiscard, RecoverChoice::Discard(id.to_owned())),
                    (TextId::RecoveryLater, RecoverChoice::Later),
                ];
                for (label, choice) in choices {
                    if ui.button(texts.get(label)).clicked() {
                        fc.actions.push(Action::Recover(choice));
                    }
                }
            });
        });
}
