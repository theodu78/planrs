//! Scène, géométrie partagée et modèles (lot L4 de J8) : H-UI-42, H-UI-44,
//! H-UI-46, H-UI-47, H-UI-53, H-UI-54, H-R11, P55 (partie scène).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]

mod common;

use common::{add, at, link, project, project_p, uid, xml_texts};
use planrs_core::{BaselineNumber, LinkType, Moment, Project, Schedule, TaskBaseline};
use planrs_render::geometry::{
    ArrowDir, Axis16, BarGeom, Class, FontRef, FontWeight, InterMetrics, Item, NoTexts, Part, Prim, Rect16,
    ResolvedTexts, RowGeomInput, Scene, TEXT_GAP, TextMeasure, TextSide, Viewport, arrow_dir, bar_geom,
    layout, link_path, place_texts, row_geom_input,
};
use planrs_render::style::{BarRole, PlanStyle, TemplateId, Variant, template};
use planrs_render::{GanttRequest, Lang, RangeSpec, RenderLimits, RowFilter, Theme, Zoom, svg_styled};
use proptest::prelude::*;

const BODY: FontRef = FontRef {
    weight: FontWeight::Regular,
    size16: 192,
};

fn scene(p: &Project, s: &Schedule, id: TemplateId, variant: Variant, lang: Lang) -> Scene {
    layout(p, s, &template(id), &NoTexts, &Viewport::new(variant, lang)).0
}

fn texts_of(scene: &Scene, class: Class) -> Vec<&str> {
    scene
        .items
        .iter()
        .filter(|i| i.class == class)
        .filter_map(|i| match &i.prim {
            Prim::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// Exemple chiffré : barre de 3 « Murs » de P au zoom Semaine (Classique).
/// Plage : lun 12/10/2026 (début − 7 j) ; début lun 26/10 08:00 =
/// 14 j × 1 440 + 480 = 20 640 min → x16 = 280 × 16 + 20 640 × 192 / 1 440
/// = 4 480 + 2 752 = 7 232 (452 px) ; fin ven 06/11 17:00 = 37 020 min →
/// 4 936, largeur 2 184 (136,5 px). Ligne 3 (indice 2) : haut 44 + 2 × 22
/// = 88 px, centre 99 px, demi-hauteur 5 px.
#[test]
fn geometry_bar_rect_examples() {
    let p = project_p();
    let s = planrs_core::schedule(&p);
    let sc = scene(&p, &s, TemplateId::Classique, Variant::Light, Lang::Fr);
    let murs = sc.rows[2].bar.unwrap();
    assert_eq!(murs.role, BarRole::Critical);
    assert_eq!(
        murs.rect,
        Rect16 {
            x: 7_232,
            y: 99 * 16 - 80,
            w: 2_184,
            h: 160
        }
    );
    // Fondations : lun 19/10 08:00 → 7 × 1 440 + 480 = 10 560 min → 1 408.
    let fondations = sc.rows[1].bar.unwrap();
    assert_eq!(fondations.rect.x, 4_480 + 1_408);
    // Jalon 4 : losange de demi-diagonale 6 px centré à la fin de 3.
    let livraison = sc.rows[3].bar.unwrap();
    assert_eq!(livraison.rect.x + livraison.rect.w / 2, murs.rect.right());
    assert_eq!(livraison.rect.w, 192);
    // Récapitulative 1 : de début de 2 à fin de 4, crochets de 8 px.
    let go = sc.rows[0].bar.unwrap();
    assert_eq!(go.rect.x, fondations.rect.x);
    assert_eq!(go.rect.h, 128);
    // H-UI-42 : la scène et l'appel direct donnent le même rectangle.
    let axis = Axis16::new(&p, &s, RangeSpec::Auto, Zoom::Week, 7_305, 280 * 16);
    let input = row_geom_input(p.tasks.get(uid(3)).unwrap(), s.tasks.get(&uid(3))).unwrap();
    assert_eq!(
        bar_geom(&input, &axis, &template(TemplateId::Classique), 88 * 16),
        murs
    );
}

fn geom(start: &str, finish: &str, row: i64) -> BarGeom {
    let p = project("L", "2026-11-02T08:00");
    let s = planrs_core::schedule(&p);
    let axis = Axis16::new(
        &p,
        &s,
        RangeSpec::Span {
            from: "2026-11-02".parse().unwrap(),
            to: "2026-11-30".parse().unwrap(),
        },
        Zoom::Day,
        7_305,
        0,
    );
    let input = RowGeomInput {
        uid: uid(1),
        role: BarRole::Task,
        start: Some(at(start)),
        finish: Some(at(finish)),
        percent: 0,
        stop: None,
        baseline: None,
    };
    bar_geom(&input, &axis, &template(TemplateId::Classique), row * 22 * 16)
}

/// H-R7 : FS direct par le dessus, SS et FF par le côté, SF replié ; au plus
/// 6 points, flèche sur le dernier.
#[test]
fn link_path_fs_ss_ff_sf() {
    let a = geom("2026-11-02T08:00", "2026-11-03T17:00", 0);
    let b = geom("2026-11-05T08:00", "2026-11-06T17:00", 1);
    let fs = link_path(&a, &b, LinkType::FinishToStart);
    assert_eq!(fs.points.len(), 3);
    assert_eq!(fs.points[0], (a.rect.right(), a.yc16));
    assert_eq!(fs.points[2], (b.rect.x + 6 * 16, b.yc16 - 80));
    assert_eq!(arrow_dir(&fs), Some(ArrowDir::Down));
    let ss = link_path(&a, &b, LinkType::StartToStart);
    assert_eq!(ss.points.last(), Some(&(b.rect.x, b.yc16)));
    assert_eq!(arrow_dir(&ss), Some(ArrowDir::Right));
    let ff = link_path(&a, &b, LinkType::FinishToFinish);
    assert_eq!(ff.points.last(), Some(&(b.rect.right(), b.yc16)));
    assert_eq!(arrow_dir(&ff), Some(ArrowDir::Left));
    let sf = link_path(&a, &b, LinkType::StartToFinish);
    assert_eq!(sf.points.len(), 6);
    assert_eq!(sf.points.last(), Some(&(b.rect.right(), b.yc16)));
    assert_eq!(arrow_dir(&sf), Some(ArrowDir::Left));
    // FS replié : successeur qui commence avant la fin du prédécesseur.
    let c = geom("2026-11-03T08:00", "2026-11-04T17:00", 2);
    let folded = link_path(&a, &c, LinkType::FinishToStart);
    assert_eq!(folded.points.len(), 6);
    assert_eq!(arrow_dir(&folded), Some(ArrowDir::Right));
    // Lien vers le haut.
    let up = link_path(
        &c,
        &geom("2026-11-10T08:00", "2026-11-11T17:00", 0),
        LinkType::FinishToStart,
    );
    assert_eq!(arrow_dir(&up), Some(ArrowDir::Up));
    for path in [fs, ss, ff, sf, folded, up] {
        assert!(path.points.len() <= 6);
        assert_eq!(path.arrow.map(|a| a[0]), path.points.last().copied());
    }
}

fn texts(right: Option<&str>, left: Option<&str>, inside: Option<&str>) -> ResolvedTexts {
    ResolvedTexts {
        left: left.map(str::to_owned),
        right: right.map(str::to_owned),
        inside: inside.map(str::to_owned),
        font: BODY,
    }
}

fn bar_at(x: i64, w: i64) -> BarGeom {
    let mut b = geom("2026-11-02T08:00", "2026-11-03T17:00", 0);
    b.rect.x = x;
    b.rect.w = w;
    b
}

/// H-UI-46 : barre finissant à 8 px du bord ⇒ nom à gauche, entier.
#[test]
fn h_ui_46_text_flips_left() {
    let clip = (0, 1_000 * 16);
    let bar = bar_at(800 * 16, 192 * 16);
    let placed = place_texts(&bar, &texts(Some("Peinture"), None, None), &InterMetrics, clip);
    let t = &placed.items[0];
    assert_eq!(t.side, TextSide::Left);
    assert_eq!(t.text, "Peinture");
    assert_eq!(t.x16 + t.width16 + TEXT_GAP, bar.rect.x);
    // Place à droite : reste à droite.
    let bar = bar_at(100 * 16, 50 * 16);
    let placed = place_texts(&bar, &texts(Some("Peinture"), None, None), &InterMetrics, clip);
    assert_eq!(placed.items[0].side, TextSide::Right);
    assert_eq!(placed.items[0].x16, bar.rect.right() + TEXT_GAP);
    // Texte à gauche sans place : basculé à droite.
    let bar = bar_at(10 * 16, 50 * 16);
    let placed = place_texts(&bar, &texts(None, Some("Fondations"), None), &InterMetrics, clip);
    assert_eq!(placed.items[0].side, TextSide::Right);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// H-UI-46 : un texte placé tient toujours dans la zone (jamais rogné) ;
    /// il est entier ou tronqué avec « … », et sa largeur est mesurée.
    #[test]
    fn h_ui_46_ellipsis_never_clipped(
        x in -200_i64..1_200,
        w in 0_i64..600,
        name in "[A-Za-zéœ ]{0,60}",
        side in 0_u8..3,
        clip_w in 0_i64..1_000,
    ) {
        let clip = (0, clip_w * 16);
        let bar = bar_at(x * 16, w * 16);
        let t = match side {
            0 => texts(Some(&name), None, None),
            1 => texts(None, Some(&name), None),
            _ => texts(None, None, Some(&name)),
        };
        for item in place_texts(&bar, &t, &InterMetrics, clip).items {
            prop_assert!(item.x16 >= clip.0, "{item:?}");
            prop_assert!(item.x16 + item.width16 <= clip.1, "{item:?}");
            prop_assert_eq!(item.width16, InterMetrics.width16(&item.text, BODY));
            prop_assert!(item.text == name || item.text.ends_with('…'));
            if item.side == TextSide::Inside {
                prop_assert!(item.x16 >= bar.rect.x && item.x16 + item.width16 <= bar.rect.right());
            }
        }
    }
}

/// H-UI-44 : dans chaque modèle (sauf Classique, J7 consigné), la barre et
/// le jalon critiques portent un indice non coloré : motif, ou contour
/// ≥ 2 px plus épais que la tâche.
#[test]
fn h_ui_44_critical_has_non_color_cue() {
    // P plus un jalon final critique « Réception » (7→8).
    let mut p = project_p();
    let t8 = add(&mut p, "Réception", 0, 1);
    link(&mut p, uid(7), t8, LinkType::FinishToStart);
    let s = planrs_core::schedule(&p);
    for id in TemplateId::ALL {
        let st = template(id);
        for variant in [Variant::Light, Variant::Dark, Variant::Print] {
            let sc = scene(&p, &s, id, variant, Lang::Fr);
            for (crit, plain) in [
                (BarRole::Critical, BarRole::Task),
                (BarRole::MilestoneCritical, BarRole::Milestone),
            ] {
                assert!(
                    sc.items.iter().any(|i| i.class == Class::Bar(crit)),
                    "{id:?} {crit:?}"
                );
                let hatched = sc.items.iter().any(|i| i.class == Class::Pattern(crit));
                let width = sc.paints[&Class::Bar(crit)]
                    .stroke
                    .map_or(0, |_| sc.paints[&Class::Bar(crit)].width16);
                let thicker = width >= 32 && width > i64::from(st.bars[&plain].outline16);
                assert_eq!(
                    hatched || thicker,
                    id != TemplateId::Classique,
                    "{id:?} {variant:?} {crit:?}"
                );
            }
        }
    }
}

/// H-UI-47 : la légende ne montre que les rôles présents ; un projet sans
/// ligne de base n'a pas d'entrée « Ligne de base ».
#[test]
fn h_ui_47_legend_only_present_roles() {
    let mut p = project_p();
    let s = planrs_core::schedule(&p);
    let sc = scene(&p, &s, TemplateId::Sobre, Variant::Light, Lang::Fr);
    assert_eq!(
        sc.legend,
        [BarRole::Critical, BarRole::SummaryCritical, BarRole::Milestone]
    );
    let legend: Vec<&str> = sc
        .items
        .iter()
        .filter(|i| i.part == Part::Legend)
        .filter_map(|i| match &i.prim {
            Prim::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(legend, ["Tâche critique", "Récapitulative critique", "Jalon"]);
    assert!(sc.height16 > sc.rows.last().unwrap().y16 + sc.rows.last().unwrap().h16);
    // Ligne de base sur 6 : l'entrée apparaît ; avancement sur 2 aussi.
    p.tasks.get_mut(uid(6)).unwrap().baselines.insert(
        BaselineNumber::PRIMARY,
        TaskBaseline {
            start: Some(at("2026-11-09T08:00")),
            finish: Some(at("2026-11-12T17:00")),
            ..TaskBaseline::default()
        },
    );
    let s = planrs_core::schedule(&p);
    let sc = scene(&p, &s, TemplateId::Sobre, Variant::Light, Lang::En);
    assert!(sc.legend.contains(&BarRole::Baseline));
    assert!(texts_of(&sc, Class::Text).contains(&"Baseline"));
    // Classique (J7) : pas de légende.
    assert_eq!(
        scene(&p, &s, TemplateId::Classique, Variant::Light, Lang::Fr).legend,
        []
    );
}

fn band_rows(sc: &Scene, class: Class) -> Vec<u32> {
    sc.items
        .iter()
        .filter(|i| i.class == class)
        .filter_map(|i| match i.part {
            Part::Row(r) => Some(r),
            _ => None,
        })
        .collect()
}

/// H-UI-53 : Présentation, bandes de phase (1 et ses enfants sur la bande
/// A, 5 et les siens sur la bande B) ; Sobre, zébrage.
#[test]
fn h_ui_53_phase_bands() {
    let p = project_p();
    let s = planrs_core::schedule(&p);
    let pres = scene(&p, &s, TemplateId::Presentation, Variant::Light, Lang::Fr);
    assert_eq!(band_rows(&pres, Class::Band(1)), [4, 5, 6]);
    // Bande A = fond : rien à peindre.
    assert_eq!(band_rows(&pres, Class::Band(0)), Vec::<u32>::new());
    let sobre = scene(&p, &s, TemplateId::Sobre, Variant::Light, Lang::Fr);
    assert_eq!(band_rows(&sobre, Class::Band(1)), [1, 3, 5]);
    let classique = scene(&p, &s, TemplateId::Classique, Variant::Light, Lang::Fr);
    assert_eq!(band_rows(&classique, Class::Band(1)), Vec::<u32>::new());
}

/// H-UI-54 : date d'état 02/11 ⇒ « État 02/11 » dans l'échelle, aujourd'hui
/// en pointillé distinct.
#[test]
fn h_ui_54_status_label() {
    let mut p = project_p();
    p.status_date = Some(at("2026-11-02T17:00"));
    p.current_date = at("2026-11-05T10:00");
    let s = planrs_core::schedule(&p);
    let sc = scene(&p, &s, TemplateId::Sobre, Variant::Light, Lang::Fr);
    assert_eq!(texts_of(&sc, Class::StatusLabel), ["État 02/11"]);
    let lines = |sc: &Scene, class| sc.items.iter().filter(|i: &&Item| i.class == class).count();
    assert_eq!(lines(&sc, Class::StatusLine), 1);
    assert_eq!(lines(&sc, Class::Today), 1);
    assert_ne!(
        sc.paints[&Class::Today].dash16,
        sc.paints[&Class::StatusLine].dash16
    );
    let en = scene(&p, &s, TemplateId::Sobre, Variant::Light, Lang::En);
    assert_eq!(texts_of(&en, Class::StatusLabel), ["Status 11/02"]);
    // Classique (J7) : ligne seule, ni étiquette ni aujourd'hui.
    let cl = scene(&p, &s, TemplateId::Classique, Variant::Light, Lang::Fr);
    assert_eq!(texts_of(&cl, Class::StatusLabel), Vec::<&str>::new());
    assert_eq!(lines(&cl, Class::StatusLine), 1);
    assert_eq!(lines(&cl, Class::Today), 0);
}

/// Chantier : numéros de semaine (« S44 · 26 » : lun 26/10/2026) et jours
/// chômés grisés.
#[test]
fn chantier_week_numbers_and_non_working() {
    let p = project_p();
    let s = planrs_core::schedule(&p);
    let sc = scene(&p, &s, TemplateId::Chantier, Variant::Light, Lang::Fr);
    assert!(
        texts_of(&sc, Class::Text)
            .iter()
            .any(|t| t.starts_with("S44 · 26")),
        "{:?}",
        texts_of(&sc, Class::Text)
    );
    assert!(sc.items.iter().any(|i| i.class == Class::NonWorking));
    assert!(
        !scene(&p, &s, TemplateId::Classique, Variant::Light, Lang::Fr)
            .items
            .iter()
            .any(|i| i.class == Class::NonWorking)
    );
}

fn request(theme: Theme, lang: Lang) -> GanttRequest {
    GanttRequest {
        range: RangeSpec::Auto,
        zoom: Zoom::Week,
        filter: RowFilter::default(),
        language: lang,
        theme,
        limits: RenderLimits::default(),
    }
}

/// Mêmes entrées, mêmes octets, pour chaque modèle et variante ; documents
/// bien formés, noms présents.
#[test]
fn svg_deterministic() {
    let (p, s) = common::house();
    for id in TemplateId::ALL {
        let st = template(id);
        for theme in [Theme::Light, Theme::Dark] {
            for lang in [Lang::Fr, Lang::En] {
                let req = request(theme, lang);
                let (a, ra) = svg_styled(&p, &s, &req, &st, &NoTexts);
                let (b, rb) = svg_styled(&p.clone(), &planrs_core::schedule(&p), &req, &st, &NoTexts);
                assert_eq!(a, b, "{id:?}");
                assert_eq!(ra, rb);
                assert_eq!(ra.bytes, a.len());
                let texts = xml_texts(&a);
                assert!(texts.iter().any(|t| t == "Études et permis"), "{id:?}");
            }
        }
    }
}

/// Projet généré : `n` feuilles en chaînes, quelques récapitulatives,
/// jalons, liens de tous types.
fn generated(spec: &[(u8, u8, u8)]) -> Project {
    let mut p = project("G", "2026-10-19T08:00");
    let mut prev = None;
    for (i, &(days, level, kind)) in spec.iter().enumerate() {
        let level = if i == 0 { 1 } else { u16::from(level % 3) + 1 };
        let u = add(&mut p, &format!("T{i}"), i64::from(days % 6), level);
        if let Some(pr) = prev
            && kind % 3 != 0
        {
            let lk = match kind % 4 {
                0 => LinkType::FinishToStart,
                1 => LinkType::StartToStart,
                2 => LinkType::FinishToFinish,
                _ => LinkType::StartToFinish,
            };
            link(&mut p, pr, u, lk);
        }
        prev = Some(u);
    }
    p
}

fn bar_of(st: &PlanStyle, p: &Project, s: &Schedule, axis: &Axis16, i: usize, top: i64) -> Option<BarGeom> {
    let task = p.tasks.iter().nth(i)?;
    let input = row_geom_input(task, s.tasks.get(&task.uid()))?;
    Some(bar_geom(&input, axis, st, top))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// P55 (partie scène) : pour tout projet généré et tout zoom, les
    /// rectangles de barre, les points des liens et l'abscisse de la date
    /// d'état de la scène égalent ceux des fonctions de géométrie appelées
    /// ligne par ligne (chemin de l'écran, L11).
    #[test]
    fn p55_scene_matches_geometry(
        spec in proptest::collection::vec((0_u8..8, 0_u8..3, 0_u8..6), 1..25),
        zoom in 0_u8..4,
        id in 0_usize..6,
        status in 0_i64..60,
    ) {
        let mut p = generated(&spec);
        p.status_date = Some(Moment::from_minutes(at("2026-10-12T00:00").minutes() + status * 1_440).unwrap());
        let s = planrs_core::schedule(&p);
        let zoom = [Zoom::Day, Zoom::Week, Zoom::Month, Zoom::Quarter][usize::from(zoom)];
        let st = template(TemplateId::ALL[id]);
        let vp = Viewport { zoom, ..Viewport::new(Variant::Light, Lang::Fr) };
        let (sc, _) = layout(&p, &s, &st, &NoTexts, &vp);
        let axis = Axis16::new(&p, &s, RangeSpec::Auto, zoom, vp.limits.max_days, sc.table16);
        let row_h = i64::from(st.density.row_px()) * 16;
        let mut bars = Vec::new();
        for (i, row) in sc.rows.iter().enumerate() {
            let top = sc.header16 + i64::try_from(i).unwrap() * row_h;
            prop_assert_eq!(row.y16, top);
            let screen = bar_of(&st, &p, &s, &axis, i, top);
            prop_assert_eq!(row.bar, screen);
            bars.push(screen);
        }
        let mut links = Vec::new();
        for (to, task) in p.tasks.iter().enumerate() {
            for l in &task.predecessors {
                let from = p.tasks.iter().position(|t| t.uid() == l.predecessor).unwrap();
                if let (Some(Some(a)), Some(Some(b))) = (bars.get(from), bars.get(to)) {
                    let path = link_path(a, b, l.kind);
                    if !path.points.is_empty() {
                        links.push((from.max(to), path));
                    }
                }
            }
        }
        links.sort_by_key(|(last, _)| *last);
        let links: Vec<_> = links.into_iter().map(|(_, path)| path).collect();
        prop_assert_eq!(&sc.links, &links);
        let x = axis.x16(p.effective_status_date());
        let drawn: Vec<i64> = sc.items.iter().filter(|i| i.class == Class::StatusLine).filter_map(|i| match &i.prim {
            Prim::Segments { segs } => segs.first().map(|s| s.0),
            _ => None,
        }).collect();
        if x >= axis.left16() && x <= axis.right16() {
            prop_assert_eq!(drawn, vec![x]);
        } else {
            prop_assert!(drawn.is_empty());
        }
    }
}
