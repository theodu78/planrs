//! Palette unique de la table et du Gantt, claire et sombre.
//!
//! Choisie sur le thème **résolu** (`ctx.theme()`), jamais sur la préférence.
//! Nuances propres (barres bleues, chemin critique rouge, récapitulatives et
//! jalons sombres, comme l'usage de MS Project), non copiées (hypothèse).

use egui::Color32;

/// Couleurs de dessin de la vue principale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Fond des corps de table et de Gantt.
    pub background: Color32,
    /// Texte courant.
    pub text: Color32,
    /// Texte appuyé (récapitulatives).
    pub text_strong: Color32,
    /// Texte atténué (lignes vides, tâches exclues).
    pub text_weak: Color32,
    /// Grille fine (niveau bas de l'échelle, séparateurs de colonnes).
    pub grid: Color32,
    /// Grille marquée (niveau haut de l'échelle).
    pub grid_major: Color32,
    /// Fond des en-têtes.
    pub header_bg: Color32,
    /// Fond de la ligne sélectionnée (table et Gantt).
    pub selection: Color32,
    /// Barre de tâche.
    pub bar: Color32,
    /// Contour de barre de tâche.
    pub bar_outline: Color32,
    /// Barre et lien du chemin critique.
    pub critical: Color32,
    /// Losange de jalon.
    pub milestone: Color32,
    /// Barre à crochets de récapitulative.
    pub summary: Color32,
    /// Lien entre tâches.
    pub link: Color32,
    /// Ligne de la date d'état.
    pub status_date: Color32,
    /// Séparateur table / Gantt.
    pub splitter: Color32,
}

impl Palette {
    /// Palette claire.
    pub const LIGHT: Self = Self {
        background: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        text: Color32::from_rgb(0x1F, 0x1F, 0x1F),
        text_strong: Color32::from_rgb(0x00, 0x00, 0x00),
        text_weak: Color32::from_rgb(0x6B, 0x70, 0x78),
        grid: Color32::from_rgb(0xE3, 0xE5, 0xE8),
        grid_major: Color32::from_rgb(0xC4, 0xC8, 0xCE),
        header_bg: Color32::from_rgb(0xF3, 0xF4, 0xF6),
        selection: Color32::from_rgb(0xDC, 0xE8, 0xF7),
        bar: Color32::from_rgb(0x3F, 0x7C, 0xC4),
        bar_outline: Color32::from_rgb(0x2B, 0x5C, 0x99),
        critical: Color32::from_rgb(0xC9, 0x30, 0x2C),
        milestone: Color32::from_rgb(0x26, 0x26, 0x26),
        summary: Color32::from_rgb(0x26, 0x26, 0x26),
        link: Color32::from_rgb(0x4A, 0x4F, 0x57),
        status_date: Color32::from_rgb(0x7B, 0x3F, 0xB8),
        splitter: Color32::from_rgb(0xC4, 0xC8, 0xCE),
    };

    /// Palette sombre.
    pub const DARK: Self = Self {
        background: Color32::from_rgb(0x1E, 0x1F, 0x22),
        text: Color32::from_rgb(0xE6, 0xE6, 0xE6),
        text_strong: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        text_weak: Color32::from_rgb(0x9A, 0xA0, 0xA8),
        grid: Color32::from_rgb(0x33, 0x36, 0x3C),
        grid_major: Color32::from_rgb(0x4A, 0x4E, 0x56),
        header_bg: Color32::from_rgb(0x26, 0x28, 0x2C),
        selection: Color32::from_rgb(0x2D, 0x3F, 0x57),
        bar: Color32::from_rgb(0x6F, 0xA8, 0xE8),
        bar_outline: Color32::from_rgb(0x4A, 0x86, 0xC8),
        critical: Color32::from_rgb(0xFF, 0x6B, 0x66),
        milestone: Color32::from_rgb(0xE6, 0xE6, 0xE6),
        summary: Color32::from_rgb(0xE6, 0xE6, 0xE6),
        link: Color32::from_rgb(0xB5, 0xBA, 0xC2),
        status_date: Color32::from_rgb(0xC4, 0x9B, 0xFF),
        splitter: Color32::from_rgb(0x4A, 0x4E, 0x56),
    };

    /// Palette du thème résolu.
    #[must_use]
    pub const fn for_theme(theme: egui::Theme) -> Self {
        match theme {
            egui::Theme::Light => Self::LIGHT,
            egui::Theme::Dark => Self::DARK,
        }
    }
}
