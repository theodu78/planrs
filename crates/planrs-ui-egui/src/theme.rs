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
    /// Fond des jours chômés du calendrier du projet (week-ends, jours
    /// fériés) dans le Gantt, aux zooms Jour et Semaine (J7, lot L7, N3).
    pub non_working: Color32,
    /// Barre à crochets d'une récapitulative critique.
    pub summary_critical: Color32,
    /// Fond du bandeau d'un message d'erreur (texte en `text_strong`).
    pub message_error_bg: Color32,
    /// Fond du bandeau d'un avertissement (texte en `text_strong`).
    pub message_warning_bg: Color32,
    /// Cadre de la cellule courante de la table (J7, lot L1) : contraste
    /// ≥ 3:1 sur le fond, le fond alterné et la sélection.
    pub cursor: Color32,
    /// Barre de la ligne de base 0 du Gantt de suivi (J7, lot L7,
    /// H-UI-22) : contraste ≥ 3:1 sur le fond et sur les jours chômés.
    pub baseline: Color32,
    /// Bande d'avancement du Gantt de suivi, dessinée dans la barre (J7,
    /// lot L7, H-UI-22) : contraste ≥ 3:1 sur la barre et sur la barre
    /// critique.
    pub progress: Color32,
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
        non_working: Color32::from_rgb(0xED, 0xEF, 0xF2),
        summary_critical: Color32::from_rgb(0x8E, 0x1F, 0x1C),
        message_error_bg: Color32::from_rgb(0xFD, 0xEC, 0xEA),
        message_warning_bg: Color32::from_rgb(0xFF, 0xF4, 0xD6),
        cursor: Color32::from_rgb(0x1F, 0x5F, 0xBF),
        baseline: Color32::from_rgb(0x80, 0x86, 0x8F),
        progress: Color32::from_rgb(0x10, 0x14, 0x1A),
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
        non_working: Color32::from_rgb(0x2A, 0x2C, 0x31),
        summary_critical: Color32::from_rgb(0xFF, 0x8A, 0x85),
        message_error_bg: Color32::from_rgb(0x3A, 0x14, 0x14),
        message_warning_bg: Color32::from_rgb(0x3A, 0x30, 0x12),
        cursor: Color32::from_rgb(0x8A, 0xB4, 0xF8),
        baseline: Color32::from_rgb(0x8E, 0x94, 0x9C),
        progress: Color32::from_rgb(0x0F, 0x23, 0x40),
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use egui::Color32;

    use super::Palette;

    /// Luminance relative WCAG 2.x d'une couleur opaque (calcul de dessin,
    /// hors moteur : les flottants sont admis ici).
    fn luminance(color: Color32) -> f32 {
        let channel = |c: u8| {
            let c = f32::from(c) / 255.0;
            if c <= 0.039_28 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    /// Rapport de contraste WCAG entre deux couleurs (≥ 1).
    fn contrast(a: Color32, b: Color32) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    fn palettes() -> [(&'static str, Palette); 2] {
        [("clair", Palette::LIGHT), ("sombre", Palette::DARK)]
    }

    #[test]
    fn palette_contrast_helper_sane() {
        let ratio = contrast(Color32::BLACK, Color32::WHITE);
        assert!((ratio - 21.0).abs() < 0.01, "{ratio}");
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn palette_text_contrast() {
        for (name, p) in palettes() {
            for (label, bg) in [
                ("fond", p.background),
                ("en-tête", p.header_bg),
                ("sélection", p.selection),
            ] {
                for (text_label, text) in [("texte", p.text), ("texte fort", p.text_strong)] {
                    let ratio = contrast(text, bg);
                    assert!(ratio >= 4.5, "{name} : {text_label} sur {label} = {ratio:.2}");
                }
            }
            let weak = contrast(p.text_weak, p.background);
            assert!(weak >= 3.0, "{name} : texte atténué = {weak:.2}");
        }
    }

    /// Bandeau de messages (D-fix-9) : texte fort et texte courant lisibles
    /// (WCAG AA, 4,5:1) sur les fonds teintés d'erreur et d'avertissement,
    /// dans les deux thèmes ; le liseré critique se distingue du fond.
    #[test]
    fn palette_message_banner_contrast() {
        for (name, p) in palettes() {
            for (label, bg) in [
                ("erreur", p.message_error_bg),
                ("avertissement", p.message_warning_bg),
            ] {
                for (text_label, text) in [("texte fort", p.text_strong), ("texte", p.text)] {
                    let ratio = contrast(text, bg);
                    assert!(
                        ratio >= 4.5,
                        "{name} : {text_label} sur bandeau {label} = {ratio:.2}"
                    );
                }
                assert_ne!(bg, p.background, "{name} : bandeau {label} non teinté");
            }
            let accent = contrast(p.critical, p.message_error_bg);
            assert!(
                accent >= 3.0,
                "{name} : liseré critique sur bandeau = {accent:.2}"
            );
            // Erreur rougeâtre, avertissement jaunâtre.
            assert!(p.message_error_bg.r() > p.message_error_bg.b(), "{name}");
            assert!(p.message_warning_bg.g() > p.message_warning_bg.b(), "{name}");
        }
    }

    #[test]
    fn palette_graphics_contrast() {
        for (name, p) in palettes() {
            for (label, color) in [
                ("barre", p.bar),
                ("critique", p.critical),
                ("jalon", p.milestone),
                ("récapitulative", p.summary),
                ("récapitulative critique", p.summary_critical),
                ("lien", p.link),
                ("date d'état", p.status_date),
            ] {
                let ratio = contrast(color, p.background);
                assert!(ratio >= 3.0, "{name} : {label} sur fond = {ratio:.2}");
            }
        }
    }

    /// J7 L1 : cadre du curseur lisible sur le fond, le fond alterné de la
    /// table et la ligne sélectionnée (≥ 3:1, WCAG 1.4.11).
    #[test]
    fn palette_cursor_contrast() {
        for (name, p) in palettes() {
            let zebra = p.background.lerp_to_gamma(p.header_bg, 0.6);
            for (label, bg) in [
                ("fond", p.background),
                ("fond alterné", zebra),
                ("sélection", p.selection),
            ] {
                let ratio = contrast(p.cursor, bg);
                assert!(ratio >= 3.0, "{name} : curseur sur {label} = {ratio:.2}");
            }
        }
    }

    /// J7 L7 (H-UI-22, N3) : ligne de base lisible sur le fond et sur les
    /// jours chômés ; bande d'avancement lisible dans la barre (courante et
    /// critique) ; barres, jalons et liens lisibles sur un jour chômé ;
    /// jours chômés distincts du fond ; ligne de base distincte de la barre.
    #[test]
    fn palette_tracking_and_non_working_contrast() {
        for (name, p) in palettes() {
            assert_ne!(p.non_working, p.background, "{name}");
            for (label, bg) in [("fond", p.background), ("jour chômé", p.non_working)] {
                let ratio = contrast(p.baseline, bg);
                assert!(ratio >= 3.0, "{name} : ligne de base sur {label} = {ratio:.2}");
            }
            for (label, bar) in [("barre", p.bar), ("barre critique", p.critical)] {
                let ratio = contrast(p.progress, bar);
                assert!(ratio >= 3.0, "{name} : avancement dans la {label} = {ratio:.2}");
            }
            for (label, color) in [
                ("barre", p.bar),
                ("critique", p.critical),
                ("jalon", p.milestone),
                ("récapitulative", p.summary),
                ("lien", p.link),
                ("date d'état", p.status_date),
            ] {
                let ratio = contrast(color, p.non_working);
                assert!(ratio >= 3.0, "{name} : {label} sur jour chômé = {ratio:.2}");
            }
            assert_ne!(p.baseline, p.bar, "{name}");
            // Ligne de base grise : aucune composante dominante nette.
            let spread = p.baseline.r().abs_diff(p.baseline.b());
            assert!(spread < 0x20, "{name} : ligne de base teintée");
        }
    }

    #[test]
    fn palette_critical_distinct_from_bar() {
        for (name, p) in palettes() {
            assert_ne!(p.critical, p.bar, "{name}");
            // Rouge dominant pour le critique, bleu dominant pour la barre.
            assert!(p.critical.r() > p.critical.b(), "{name}");
            assert!(p.bar.b() > p.bar.r(), "{name}");
            assert_ne!(p.summary_critical, p.summary, "{name}");
        }
    }

    #[test]
    fn palette_light_dark_differ() {
        assert_ne!(Palette::LIGHT, Palette::DARK);
        assert!(luminance(Palette::LIGHT.background) > luminance(Palette::DARK.background));
        assert_eq!(Palette::for_theme(egui::Theme::Light), Palette::LIGHT);
        assert_eq!(Palette::for_theme(egui::Theme::Dark), Palette::DARK);
    }

    #[test]
    fn palette_opaque() {
        for (name, p) in palettes() {
            for color in [
                p.background,
                p.text,
                p.text_strong,
                p.text_weak,
                p.grid,
                p.grid_major,
                p.header_bg,
                p.selection,
                p.bar,
                p.bar_outline,
                p.critical,
                p.milestone,
                p.summary,
                p.link,
                p.status_date,
                p.splitter,
                p.non_working,
                p.summary_critical,
                p.message_error_bg,
                p.message_warning_bg,
                p.cursor,
                p.baseline,
                p.progress,
            ] {
                assert_eq!(color.a(), 255, "{name} : {color:?}");
            }
        }
    }
}
