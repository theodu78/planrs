//! Palette unique de la table et du Gantt, claire et sombre.
//!
//! Choisie sur le thème **résolu** (`ctx.theme()`), jamais sur la préférence.
//! Nuances propres (barres bleues, chemin critique rouge, récapitulatives et
//! jalons sombres, comme l'usage de MS Project), non copiées (hypothèse).

use std::sync::Arc;

use egui::Color32;
use planrs_render::style::{BarRole, FontChoice, PlanStyle, Rgb, TemplateId, Variant, template};

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

    /// Palette tirée d'un style du planning (ADR-0016 D5) pour une variante.
    ///
    /// Le séparateur table / Gantt prend la grille marquée (égaux en J7). Un
    /// rôle de barre absent (style non validé) prend le texte courant.
    #[must_use]
    pub fn from_style(style: &PlanStyle, variant: Variant) -> Self {
        let c = style.tokens(variant);
        let rgb = |v: Rgb| Color32::from_rgb(v.0, v.1, v.2);
        let fill = |role: BarRole| rgb(c.bars.get(&role).map_or(c.text, |p| p.fill));
        let outline = |role: BarRole| rgb(c.bars.get(&role).map_or(c.text, |p| p.outline.unwrap_or(p.fill)));
        Self {
            background: rgb(c.bg),
            text: rgb(c.text),
            text_strong: rgb(c.text_strong),
            text_weak: rgb(c.text_muted),
            grid: rgb(c.grid),
            grid_major: rgb(c.grid_major),
            header_bg: rgb(c.header_bg),
            selection: rgb(c.selection),
            bar: fill(BarRole::Task),
            bar_outline: outline(BarRole::Task),
            critical: fill(BarRole::Critical),
            milestone: fill(BarRole::Milestone),
            summary: fill(BarRole::Summary),
            link: rgb(c.link),
            status_date: rgb(c.status_line),
            splitter: rgb(c.grid_major),
            non_working: rgb(c.non_working),
            summary_critical: fill(BarRole::SummaryCritical),
            message_error_bg: rgb(c.message_error_bg),
            message_warning_bg: rgb(c.message_warning_bg),
            cursor: rgb(c.cursor),
            baseline: fill(BarRole::Baseline),
            progress: fill(BarRole::Progress),
        }
    }

    /// Palette du thème résolu.
    #[must_use]
    pub const fn for_theme(theme: egui::Theme) -> Self {
        match theme {
            egui::Theme::Light => Self::LIGHT,
            egui::Theme::Dark => Self::DARK,
        }
    }
}

/// Variante du style pour un thème résolu de l'interface.
#[must_use]
pub const fn variant_for(theme: egui::Theme) -> Variant {
    match theme {
        egui::Theme::Light => Variant::Light,
        egui::Theme::Dark => Variant::Dark,
    }
}

/// Famille de police du planning (Inter Regular), chargée seulement quand
/// le style effectif la demande (ADR-0016 D9, J8 lot L11).
pub const PLAN_FAMILY: &str = "planrs-plan";

/// Famille Inter `SemiBold` (textes appuyés : récapitulatives).
pub const PLAN_FAMILY_STRONG: &str = "planrs-plan-semibold";

/// Inter Regular (OFL-1.1, `assets/fonts`, mention dans `NOTICE`).
static INTER_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");

/// Inter `SemiBold`.
static INTER_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");

/// Installe les deux familles Inter du planning dans `ctx` (sans effet si
/// elles y sont déjà ; egui ne relit que les noms). Les polices d'egui
/// suivent en repli pour les caractères absents d'Inter.
pub fn install_plan_fonts(ctx: &egui::Context) {
    use egui::epaint::text::{FontData, FontInsert, FontPriority, InsertFontFamily};
    for (name, bytes) in [(PLAN_FAMILY, INTER_REGULAR), (PLAN_FAMILY_STRONG, INTER_SEMIBOLD)] {
        ctx.add_font(FontInsert::new(
            name,
            FontData::from_static(bytes),
            vec![InsertFontFamily {
                family: egui::FontFamily::Name(name.into()),
                priority: FontPriority::Highest,
            }],
        ));
    }
}

/// Style du planning lu par l'écran à une image (J8, lot L11, ADR-0016
/// D5) : style effectif du document (modèle par défaut des préférences pour
/// un document sans style, H-UI-43) et variante du thème résolu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenStyle {
    /// Style résolu.
    pub style: Arc<PlanStyle>,
    /// Variante (clair ou sombre).
    pub variant: Variant,
    /// Phase (0 ou 1) de chaque ligne du modèle pour les bandes de phase
    /// (H-UI-53) ; vide sans bandes de phase. Calculée une fois par
    /// génération du document, jamais pendant une image.
    pub phases: Arc<Vec<u8>>,
}

impl Default for ScreenStyle {
    /// Classique clair (J7).
    fn default() -> Self {
        Self {
            style: Arc::new(template(TemplateId::Classique)),
            variant: Variant::Light,
            phases: Arc::new(Vec::new()),
        }
    }
}

impl ScreenStyle {
    /// Hauteur d'une ligne (densité du style : 18, 22 ou 28 px).
    #[must_use]
    pub fn row_h(&self) -> f32 {
        f32::from(self.style.density.row_px())
    }

    /// Le style demande la police Inter (table et Gantt).
    #[must_use]
    pub fn inter(&self) -> bool {
        self.style.font.family == FontChoice::Inter
    }
}

/// Cache du style effectif et de ses palettes (J8, lot L11) : relu
/// seulement quand le document (ouverture, génération) ou le modèle par
/// défaut changent ; la palette du thème courant est servie sans calcul.
#[derive(Clone, Debug)]
pub struct StyleCache {
    /// Clé du style gardé : (ouverture, génération) du document, modèle
    /// par défaut.
    key: Option<(Option<(u64, u64)>, TemplateId)>,
    /// Style gardé.
    style: Arc<PlanStyle>,
    /// Palettes claire et sombre du style gardé.
    palettes: Option<(Palette, Palette)>,
    /// Phases gardées et leur clé (document).
    phases: (Option<(u64, u64)>, Arc<Vec<u8>>),
    /// Lectures faites (tests).
    builds: u64,
}

impl Default for StyleCache {
    fn default() -> Self {
        Self {
            key: None,
            style: ScreenStyle::default().style,
            palettes: None,
            phases: (None, Arc::new(Vec::new())),
            builds: 0,
        }
    }
}

impl StyleCache {
    /// Style et palette pour la clé `(document, défaut)` et le thème
    /// `theme` ; `resolve` n'est appelé que si la clé change.
    pub fn screen(
        &mut self,
        doc_key: Option<(u64, u64)>,
        default: TemplateId,
        theme: egui::Theme,
        resolve: impl FnOnce() -> PlanStyle,
    ) -> (ScreenStyle, Palette) {
        let key = (doc_key, default);
        if self.key != Some(key) || self.palettes.is_none() {
            let style = resolve();
            self.palettes = Some((
                Palette::from_style(&style, Variant::Light),
                Palette::from_style(&style, Variant::Dark),
            ));
            self.style = Arc::new(style);
            self.key = Some(key);
            self.builds = self.builds.saturating_add(1);
        }
        let (light, dark) = self.palettes.unwrap_or((Palette::LIGHT, Palette::DARK));
        let palette = match theme {
            egui::Theme::Light => light,
            egui::Theme::Dark => dark,
        };
        let screen = ScreenStyle {
            style: Arc::clone(&self.style),
            variant: variant_for(theme),
            phases: Arc::new(Vec::new()),
        };
        (screen, palette)
    }

    /// Phases des lignes du document `doc_key`, calculées par `compute`
    /// seulement si le document a changé.
    pub fn phases(&mut self, doc_key: Option<(u64, u64)>, compute: impl FnOnce() -> Vec<u8>) -> Arc<Vec<u8>> {
        if self.phases.0 != doc_key || doc_key.is_none() {
            self.phases = (doc_key, Arc::new(compute()));
        }
        Arc::clone(&self.phases.1)
    }

    /// Résolutions faites depuis la création (tests du cache).
    #[must_use]
    pub const fn builds(&self) -> u64 {
        self.builds
    }

    /// Style gardé (hauteur de ligne de la navigation entre deux images).
    #[must_use]
    pub fn style(&self) -> &PlanStyle {
        &self.style
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

    use planrs_render::style::{TemplateId, Variant, template};

    use super::{Palette, Rgb};

    /// Rapport de contraste WCAG en millièmes, par la fonction entière
    /// partagée avec le rendu (`planrs_render::style::contrast_ratio`).
    fn contrast(a: Color32, b: Color32) -> u32 {
        planrs_render::style::contrast_ratio(Rgb(a.r(), a.g(), a.b()), Rgb(b.r(), b.g(), b.b()))
    }

    fn luminance(c: Color32) -> u32 {
        planrs_render::style::luminance_ppm(Rgb(c.r(), c.g(), c.b()))
    }

    fn palettes() -> [(&'static str, Palette); 2] {
        [("clair", Palette::LIGHT), ("sombre", Palette::DARK)]
    }

    #[test]
    fn palette_contrast_helper_sane() {
        assert_eq!(contrast(Color32::BLACK, Color32::WHITE), 21_000);
        assert_eq!(contrast(Color32::WHITE, Color32::WHITE), 1000);
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
                    assert!(ratio >= 4500, "{name} : {text_label} sur {label} = {ratio}‰");
                }
            }
            let weak = contrast(p.text_weak, p.background);
            assert!(weak >= 3000, "{name} : texte atténué = {weak}‰");
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
                        ratio >= 4500,
                        "{name} : {text_label} sur bandeau {label} = {ratio}‰"
                    );
                }
                assert_ne!(bg, p.background, "{name} : bandeau {label} non teinté");
            }
            let accent = contrast(p.critical, p.message_error_bg);
            assert!(accent >= 3000, "{name} : liseré critique sur bandeau = {accent}‰");
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
                assert!(ratio >= 3000, "{name} : {label} sur fond = {ratio}‰");
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
                assert!(ratio >= 3000, "{name} : curseur sur {label} = {ratio}‰");
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
                assert!(ratio >= 3000, "{name} : ligne de base sur {label} = {ratio}‰");
            }
            for (label, bar) in [("barre", p.bar), ("barre critique", p.critical)] {
                let ratio = contrast(p.progress, bar);
                assert!(ratio >= 3000, "{name} : avancement dans la {label} = {ratio}‰");
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
                assert!(ratio >= 3000, "{name} : {label} sur jour chômé = {ratio}‰");
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

    /// Le modèle Classique reproduit J7 champ par champ (ADR-0016 D6), dans
    /// les deux thèmes ; l'impression part du clair.
    #[test]
    fn classique_equals_j7_palette() {
        let style = template(TemplateId::Classique);
        assert_eq!(Palette::from_style(&style, Variant::Light), Palette::LIGHT);
        assert_eq!(Palette::from_style(&style, Variant::Dark), Palette::DARK);
        assert_eq!(Palette::from_style(&style, Variant::Print), Palette::LIGHT);
    }

    /// Le style n'est relu que si le document ou le modèle par défaut
    /// changent ; le thème ne fait que choisir la palette.
    #[test]
    fn style_cache_rebuilds_on_key_only() {
        let mut cache = super::StyleCache::default();
        let sobre = || template(TemplateId::Sobre);
        let (a, light) = cache.screen(Some((1, 0)), TemplateId::Sobre, egui::Theme::Light, sobre);
        let (b, dark) = cache.screen(Some((1, 0)), TemplateId::Sobre, egui::Theme::Dark, || {
            panic!("relu")
        });
        assert_eq!(cache.builds(), 1);
        assert_eq!(a.style, b.style);
        assert_eq!(
            light,
            Palette::from_style(&template(TemplateId::Sobre), Variant::Light)
        );
        assert_eq!(
            dark,
            Palette::from_style(&template(TemplateId::Sobre), Variant::Dark)
        );
        assert_eq!(b.variant, Variant::Dark);
        cache.screen(Some((1, 1)), TemplateId::Sobre, egui::Theme::Dark, sobre);
        cache.screen(Some((1, 1)), TemplateId::Classique, egui::Theme::Dark, sobre);
        assert_eq!(cache.builds(), 3);
        let (classique, light) =
            super::StyleCache::default().screen(None, TemplateId::Classique, egui::Theme::Light, || {
                template(TemplateId::Classique)
            });
        assert_eq!(light, Palette::LIGHT);
        assert_eq!(classique, super::ScreenStyle::default());
        assert!((classique.row_h() - 22.0).abs() < f32::EPSILON);
        assert!(!classique.inter());
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
