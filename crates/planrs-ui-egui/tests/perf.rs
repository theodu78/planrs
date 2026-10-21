//! Fluidité de la vue principale (ADR-0011, décision 12) : coût CPU d'une
//! image (construction egui par [`planrs_ui_egui::ui`] puis tessellation) sur
//! 5 000 tâches et 10 000 liens, fenêtre 1920×1080, 1 px par point, avec un
//! `egui::Context` nu (sans `Harness` : AccessKit n'est actif sur le desktop
//! qu'en présence d'un lecteur d'écran).
//!
//! - `frame_budget_5000_tasks` (ignoré, à lancer en release) : 3 zooms × 3
//!   positions de défilement (début, milieu, fin, par `Action::ScrollToRow`)
//!   × 30 images ; chaque image mesurée reçoit un cran de molette de 37 points
//!   au-dessus de la table, pour forcer la mise en page de nouvelles lignes
//!   (le cache de texte d'egui ne joue pas en faveur de la mesure) ; le Gantt
//!   est défilé horizontalement jusqu'aux barres des lignes visibles, pour
//!   que barres et liens soient dessinés à chaque position. Échec si
//!   une médiane atteint 16 ms ou un 95e centile 32 ms.
//! - `shapes_do_not_grow_with_task_count` (toujours exécuté, debug) : garde
//!   déterministe de la virtualisation, indépendante de la vitesse de la
//!   machine ; `frame_counts_equal_at_1000_and_5000_tasks` (J7 L1,
//!   toujours exécuté) : formes, sommets et galleys égaux à ± 5 % à 1 000
//!   et 5 000 tâches, curseur dessiné et déplacé au clavier.
//! - J7 L1 (ignorés, release) : `nav_down_held` (↓ sur 300 images, une
//!   touche par image), `nav_page_sweep` (Page bas jusqu'à la fin puis
//!   Ctrl+Origine), mêmes seuils que ci-dessus ; `edit_latency` (20
//!   validations de durée par Entrée sur des tâches réparties : image de la
//!   validation, qui recalcule et reconstruit le `RowModel`, plus l'image
//!   suivante), mesure publiée avec sa décomposition (recalcul,
//!   `RowModel::build`, reste), porte au lot L16 (médiane < 100 ms, p95 <
//!   150 ms).
//! - J7 L3 (ignoré, release) : `approval_banner` (bandeau de validation
//!   d'une écriture d'un agent affiché, défilement à chaque image), mêmes
//!   seuils.
//! - J7 L8 (ignoré, release) : `bar_drag` (120 images de glisser d'une
//!   barre, un point par image, aperçu `gesture::preview` recalculé quand
//!   l'écart en jours change, aucun recalcul du planning), puis lâcher (un
//!   seul lot) ; mêmes seuils.
//! - J7 L11 : `usage_scroll` (ignoré, release) : utilisation des
//!   ressources, 1 200 ressources, 5 000 tâches affectées, défilement
//!   vertical (molette) puis horizontal (blocs neufs du moteur), mêmes
//!   seuils ; pire image (un bloc neuf) < 32 ms. Garde toujours exécutée :
//!   `usage_counts_equal_at_1000_and_5000_tasks` (formes, sommets et galleys
//!   de la vue à ± 5 %).
//!
//! Le rendu GPU n'est pas chronométré (lavapipe en CI n'est pas
//! représentatif). Le projet est un MSPDI généré en mémoire par ce fichier :
//! `App` ne s'ouvre que depuis des octets (pas d'injection directe d'un
//! `Project`, voir le compte rendu L8), et `planrs_tests::generate` construit
//! un `Project`, pas un MSPDI.
//!
//! `cargo test -p planrs-ui-egui --release --test perf -- --ignored --nocapture`

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "mesure de temps dans un test : l'horloge est interdite dans la crate, pas dans ses tests"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::{Duration, Instant};

use egui::epaint::{ClippedPrimitive, Primitive, Shape};
use egui::{
    Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, Pos2, RawInput, Rect, TouchPhase, pos2, vec2,
};
use planrs_core::Moment;
use planrs_ui_egui::column::Column;
use planrs_ui_egui::edit::{EditState, commit_edit};
use planrs_ui_egui::nav::RowRef;
use planrs_ui_egui::rows::{RowKind, RowModel};
use planrs_ui_egui::timescale::TimeAxis;
use planrs_ui_egui::view::{OVERSCAN, ROW_H};
use planrs_ui_egui::{
    Action, App, DocOrigin, Env, LanguagePref, ThemePref, TimeScaleZoom, UiState, ViewKind,
};

/// Taille de la fenêtre simulée.
const SCREEN: (f32, f32) = (1920.0, 1080.0);
/// Images mesurées par série (zoom × position).
const FRAMES: usize = 30;
/// Images de chauffe avant chaque série.
const WARMUP: usize = 5;
/// Seuil d'échec de la médiane.
const BUDGET: Duration = Duration::from_millis(16);
/// Seuil d'échec du 95e centile.
const SPIKE: Duration = Duration::from_millis(32);
/// Cran de molette par image mesurée, en points.
const WHEEL_STEP: f32 = 37.0;

/// Récapitulatives du grand réseau.
const SUMMARIES: usize = 250;
/// Feuilles par récapitulative : 250 × (1 + 19) = 5 000 tâches.
const LEAVES_PER_SUMMARY: usize = 19;
/// Liens du grand réseau.
const LINKS: usize = 10_000;
/// Nombre de voies FS parallèles : la feuille `k` suit la feuille `k − 19`
/// (environ 250 tâches en série, soit un peu plus d'un an de planning).
const LANES: usize = 19;

/// « Maintenant » fixe (début du projet généré).
fn now() -> Moment {
    "2026-01-05T08:00".parse().unwrap()
}

/// Semaine type Standard (lundi-vendredi, 8 h-12 h et 13 h-17 h).
const CALENDAR: &str = "<Calendars><Calendar><UID>1</UID><Name>Standard</Name>\
<IsBaseCalendar>1</IsBaseCalendar><BaseCalendarUID>-1</BaseCalendarUID><WeekDays>\
<WeekDay><DayType>1</DayType><DayWorking>0</DayWorking></WeekDay>\
<WeekDay><DayType>2</DayType><DayWorking>1</DayWorking><WorkingTimes><WorkingTime><FromTime>08:00:00</FromTime><ToTime>12:00:00</ToTime></WorkingTime><WorkingTime><FromTime>13:00:00</FromTime><ToTime>17:00:00</ToTime></WorkingTime></WorkingTimes></WeekDay>\
<WeekDay><DayType>3</DayType><DayWorking>1</DayWorking><WorkingTimes><WorkingTime><FromTime>08:00:00</FromTime><ToTime>12:00:00</ToTime></WorkingTime><WorkingTime><FromTime>13:00:00</FromTime><ToTime>17:00:00</ToTime></WorkingTime></WorkingTimes></WeekDay>\
<WeekDay><DayType>4</DayType><DayWorking>1</DayWorking><WorkingTimes><WorkingTime><FromTime>08:00:00</FromTime><ToTime>12:00:00</ToTime></WorkingTime><WorkingTime><FromTime>13:00:00</FromTime><ToTime>17:00:00</ToTime></WorkingTime></WorkingTimes></WeekDay>\
<WeekDay><DayType>5</DayType><DayWorking>1</DayWorking><WorkingTimes><WorkingTime><FromTime>08:00:00</FromTime><ToTime>12:00:00</ToTime></WorkingTime><WorkingTime><FromTime>13:00:00</FromTime><ToTime>17:00:00</ToTime></WorkingTime></WorkingTimes></WeekDay>\
<WeekDay><DayType>6</DayType><DayWorking>1</DayWorking><WorkingTimes><WorkingTime><FromTime>08:00:00</FromTime><ToTime>12:00:00</ToTime></WorkingTime><WorkingTime><FromTime>13:00:00</FromTime><ToTime>17:00:00</ToTime></WorkingTime></WorkingTimes></WeekDay>\
<WeekDay><DayType>7</DayType><DayWorking>0</DayWorking></WeekDay>\
</WeekDays></Calendar></Calendars>";

/// Lien MSPDI (`Type` : 0 FF, 1 FS, 3 SS), sans décalage.
fn link_xml(out: &mut String, predecessor_uid: usize, kind: u8) {
    write!(
        out,
        "<PredecessorLink><PredecessorUID>{predecessor_uid}</PredecessorUID><Type>{kind}</Type>\
         <CrossProject>0</CrossProject><LinkLag>0</LinkLag><LagFormat>7</LagFormat></PredecessorLink>"
    )
    .unwrap();
}

/// Réseau MSPDI déterministe (sans aléa) : `summaries` récapitulatives de
/// `leaves` feuilles chacune ; au plus `max_links` liens entre feuilles
/// proches, dans cet ordre de priorité : FS depuis la feuille `k − 19`
/// (voies parallèles), SS depuis `k − 1`, puis FF depuis `k − 2` pour une
/// feuille sur neuf. Tous les liens remontent la liste : aucun cycle, aucun
/// lien entre une récapitulative et sa descendante. Durées de 4 à 16 heures,
/// un jalon toutes les 97 feuilles.
fn network_xml(summaries: usize, leaves: usize, max_links: usize) -> Vec<u8> {
    let leaf_count = summaries * leaves;
    // Prédécesseurs de chaque feuille, en indices de feuille.
    let mut preds: Vec<Vec<(usize, u8)>> = vec![Vec::new(); leaf_count];
    let mut links = 0;
    for (k, list) in preds.iter_mut().enumerate().skip(LANES) {
        if links < max_links {
            list.push((k - LANES, 1));
            links += 1;
        }
    }
    for (k, list) in preds.iter_mut().enumerate().skip(1) {
        if links < max_links {
            list.push((k - 1, 3));
            links += 1;
        }
    }
    for k in (9..leaf_count).step_by(9) {
        if links < max_links {
            preds[k].push((k - 2, 0));
            links += 1;
        }
    }
    // UID d'une feuille : récapitulative, puis ses feuilles, dans l'ordre.
    let leaf_uid = |k: usize| (k / leaves) * (leaves + 1) + (k % leaves) + 2;

    let mut xml = String::with_capacity(leaf_count * 700);
    xml.push_str(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <Project xmlns=\"http://schemas.microsoft.com/project\"><SaveVersion>14</SaveVersion>\
         <Name>Réseau de mesure</Name><ScheduleFromStart>1</ScheduleFromStart>\
         <StartDate>2026-01-05T08:00:00</StartDate><CurrentDate>2026-01-05T08:00:00</CurrentDate>\
         <CalendarUID>1</CalendarUID><MinutesPerDay>480</MinutesPerDay><MinutesPerWeek>2400</MinutesPerWeek>\
         <DaysPerMonth>20</DaysPerMonth><DurationFormat>7</DurationFormat><CriticalSlackLimit>0</CriticalSlackLimit>",
    );
    xml.push_str(CALENDAR);
    xml.push_str(
        "<Tasks><Task><UID>0</UID><ID>0</ID><Name>Réseau de mesure</Name>\
         <OutlineLevel>0</OutlineLevel><Summary>1</Summary></Task>",
    );
    let mut id = 0;
    for s in 0..summaries {
        id += 1;
        write!(
            xml,
            "<Task><UID>{id}</UID><ID>{id}</ID><Name>Lot {n} — gros œuvre et second œuvre</Name>\
             <OutlineLevel>1</OutlineLevel><Active>1</Active><Manual>0</Manual><Summary>1</Summary>\
             <Milestone>0</Milestone><DurationFormat>7</DurationFormat><ConstraintType>0</ConstraintType></Task>",
            n = s + 1
        )
        .unwrap();
        for j in 0..leaves {
            id += 1;
            let k = s * leaves + j;
            assert_eq!(leaf_uid(k), id);
            let milestone = k % 97 == 96;
            let hours = if milestone { 0 } else { 4 + (k * 7) % 13 };
            write!(
                xml,
                "<Task><UID>{id}</UID><ID>{id}</ID><Name>Tâche {k} : préparation, exécution et contrôle</Name>\
                 <OutlineLevel>2</OutlineLevel><Active>1</Active><Manual>0</Manual><Summary>0</Summary>\
                 <Milestone>{m}</Milestone><Duration>PT{hours}H0M0S</Duration><DurationFormat>7</DurationFormat>\
                 <ConstraintType>0</ConstraintType>",
                m = u8::from(milestone)
            )
            .unwrap();
            for &(p, kind) in &preds[k] {
                link_xml(&mut xml, leaf_uid(p), kind);
            }
            xml.push_str("</Task>");
        }
    }
    xml.push_str("</Tasks></Project>");
    xml.into_bytes()
}

/// Application ouverte sur un réseau généré : français, thème clair.
fn app_with(summaries: usize, leaves: usize, max_links: usize) -> App {
    let state = UiState {
        language: LanguagePref::Fr,
        theme: ThemePref::Light,
        ..UiState::default()
    };
    let mut app = App::new(state, Env::fixed(now()));
    let origin = DocOrigin {
        name: "mesure.xml".to_owned(),
        path: None,
        demo: false,
    };
    app.open_bytes(&network_xml(summaries, leaves, max_links), origin);
    let document = app.document.as_ref().unwrap_or_else(|| {
        panic!("réseau généré refusé : {:?}", app.messages());
    });
    assert_eq!(document.project().tasks.len(), summaries * (leaves + 1));
    assert!(
        document.schedule().diagnostics.is_empty(),
        "{:?}",
        document.schedule().diagnostics.first()
    );
    app
}

/// Relevé d'une image indépendant de la machine : formes de la vue
/// principale, sommets tessellés, galleys (textes) émis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    shapes: usize,
    vertices: usize,
    galleys: usize,
}

/// Textes d'une forme (récursif dans les vecteurs de formes).
fn galleys_in(shape: &Shape) -> usize {
    match shape {
        Shape::Text(_) => 1,
        Shape::Vec(shapes) => shapes.iter().map(galleys_in).sum(),
        _ => 0,
    }
}

/// Sommets des maillages tessellés.
fn vertices_in(primitives: &[ClippedPrimitive]) -> usize {
    primitives
        .iter()
        .map(|p| match &p.primitive {
            Primitive::Mesh(mesh) => mesh.vertices.len(),
            Primitive::Callback(_) => 0,
        })
        .sum()
}

/// Contexte egui nu et horloge simulée des images.
struct Bench {
    ctx: egui::Context,
    time: f64,
    /// Relevé de la dernière image.
    last: Counts,
}

impl Bench {
    fn new() -> Self {
        Self {
            ctx: egui::Context::default(),
            time: 0.0,
            last: Counts::default(),
        }
    }

    /// Une image ; rend la durée de `run_ui` + `tessellate`.
    fn frame(&mut self, app: &mut App, events: Vec<Event>) -> Duration {
        self.time += 1.0 / 60.0;
        // Modificateurs de l'image : ceux de la touche, annoncés d'abord.
        let modifiers = events
            .iter()
            .find_map(|e| match e {
                Event::Key { modifiers, .. } => Some(*modifiers),
                _ => None,
            })
            .unwrap_or_default();
        let mut events = events;
        events.insert(0, Event::ModifiersChanged(modifiers));
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(SCREEN.0, SCREEN.1))),
            time: Some(self.time),
            events,
            ..RawInput::default()
        };
        let t0 = Instant::now();
        let mut output = self.ctx.run_ui(raw, |ui| planrs_ui_egui::ui(ui, app));
        let shapes = std::mem::take(&mut output.shapes);
        let galleys = shapes.iter().map(|c| galleys_in(&c.shape)).sum();
        let primitives = self.ctx.tessellate(shapes, output.pixels_per_point);
        let elapsed = t0.elapsed();
        self.last = Counts {
            shapes: app.last_frame.shapes,
            vertices: vertices_in(&primitives),
            galleys,
        };
        black_box(primitives);
        // Aucun moteur de rendu : les textures sont ignorées.
        output.textures_delta.clear();
        elapsed
    }

    /// Images sans entrée jusqu'à stabilité de la plage de lignes peintes.
    fn settle(&mut self, app: &mut App) {
        for _ in 0..12 {
            self.frame(app, Vec::new());
        }
    }
}

/// Défile le Gantt pour que la barre de la ligne `row` apparaisse près du
/// bord gauche (abscisse donnée par l'axe de l'interface : temps → pixels).
fn scroll_gantt_to_row(app: &mut App, row: usize) {
    let doc = app.document.as_ref().unwrap();
    let start = app.rows().unwrap().row(row).unwrap().bar.unwrap().0;
    let axis = TimeAxis::new(doc.range, app.state.zoom, 0.0);
    let target = (axis.x(start) - 200.0).max(0.0);
    let delta = target - app.view().gantt_scroll_x;
    app.dispatch(Action::ScrollGanttX(delta));
}

/// Cran de molette au-dessus de la table (pointeur au milieu du corps).
fn wheel_events(dy: f32) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos2(300.0, 540.0)),
        Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, dy),
            modifiers: Modifiers::NONE,
            phase: TouchPhase::Move,
            source: MouseWheelSource::Unknown,
        },
    ]
}

/// Centile `p` (0 à 100) d'une série triée.
fn percentile(sorted: &[Duration], p: usize) -> Duration {
    sorted[(sorted.len() - 1) * p / 100]
}

/// Millisecondes, pour l'affichage.
fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000.0
}

#[test]
#[ignore = "mesure, à lancer en --release : cargo test -p planrs-ui-egui --release --test perf -- --ignored --nocapture"]
fn frame_budget_5000_tasks() {
    let open = Instant::now();
    let mut app = app_with(SUMMARIES, LEAVES_PER_SUMMARY, LINKS);
    let links: usize = app
        .document
        .as_ref()
        .unwrap()
        .project()
        .tasks
        .iter()
        .map(|t| t.predecessors.len())
        .sum();
    assert_eq!(links, LINKS);
    eprintln!(
        "perf UI : ouverture (lecture MSPDI, calcul, RowModel) {:.1} ms",
        ms(open.elapsed())
    );
    let len = app.rows().unwrap().len();
    let mut bench = Bench::new();
    bench.settle(&mut app);

    let mut worst_median = Duration::ZERO;
    let mut worst_p95 = Duration::ZERO;
    for zoom in [TimeScaleZoom::Day, TimeScaleZoom::Week, TimeScaleZoom::Quarter] {
        app.dispatch(Action::SetZoom(zoom));
        bench.settle(&mut app);
        assert_eq!(app.state.zoom, zoom);
        let mut all = Vec::with_capacity(3 * FRAMES);
        for (label, row, dy) in [
            ("début", 0, -WHEEL_STEP),
            ("milieu", len / 2, -WHEEL_STEP),
            ("fin", len - 1, WHEEL_STEP),
        ] {
            app.dispatch(Action::ScrollToRow(row));
            bench.settle(&mut app);
            scroll_gantt_to_row(&mut app, row);
            bench.settle(&mut app);
            let painted = app.last_frame.table_rows.clone();
            assert!(
                painted.contains(&row.min(len - 1)) || (row == len - 1 && painted.end == len),
                "{label} : ligne {row} non peinte ({painted:?})"
            );
            for _ in 0..WARMUP {
                bench.frame(&mut app, wheel_events(dy));
            }
            let before = app.last_frame.table_rows.start;
            let mut samples: Vec<Duration> = (0..FRAMES)
                .map(|_| bench.frame(&mut app, wheel_events(dy)))
                .collect();
            let after = app.last_frame.table_rows.start;
            assert_ne!(before, after, "{label} : la molette n'a pas fait défiler");
            all.extend_from_slice(&samples);
            samples.sort();
            let (median, p95) = (percentile(&samples, 50), percentile(&samples, 95));
            eprintln!(
                "perf UI {zoom:?} {label:<6} : médiane {:6.2} ms, p95 {:6.2} ms, max {:6.2} ms, \
                 formes {}, liens {}, lignes {:?}",
                ms(median),
                ms(p95),
                ms(*samples.last().unwrap()),
                app.last_frame.shapes,
                app.last_frame.links_painted,
                app.last_frame.table_rows,
            );
            worst_median = worst_median.max(median);
            worst_p95 = worst_p95.max(p95);
        }
        all.sort();
        eprintln!(
            "perf UI {zoom:?} (3 positions × {FRAMES} images) : médiane {:.2} ms, p95 {:.2} ms",
            ms(percentile(&all, 50)),
            ms(percentile(&all, 95)),
        );
    }
    eprintln!(
        "perf UI : pire médiane {:.2} ms (seuil {} ms), pire p95 {:.2} ms (seuil {} ms)",
        ms(worst_median),
        BUDGET.as_millis(),
        ms(worst_p95),
        SPIKE.as_millis()
    );
    assert!(
        worst_median < BUDGET,
        "pire médiane {worst_median:?} ≥ {BUDGET:?}"
    );
    assert!(worst_p95 < SPIKE, "pire p95 {worst_p95:?} ≥ {SPIKE:?}");
}

/// Formes et lignes peintes de la vue principale après stabilisation, au
/// zoom Semaine, défilée à la ligne `row`.
fn painted(app: &mut App, row: usize) -> (usize, std::ops::Range<usize>) {
    let mut bench = Bench::new();
    bench.settle(app);
    app.dispatch(Action::SetZoom(TimeScaleZoom::Week));
    app.dispatch(Action::ScrollToRow(row));
    bench.settle(app);
    assert_eq!(app.state.zoom, TimeScaleZoom::Week);
    (app.last_frame.shapes, app.last_frame.table_rows.clone())
}

#[test]
fn shapes_do_not_grow_with_task_count() {
    // 5 × 20 = 100 tâches, 2 liens par tâche au plus ; 5 000 tâches, 10 000 liens.
    let mut small = app_with(5, LEAVES_PER_SUMMARY, 200);
    let mut large = app_with(SUMMARIES, LEAVES_PER_SUMMARY, LINKS);
    let (small_shapes, small_rows) = painted(&mut small, 0);
    let len = large.rows().unwrap().len();
    // Plafond de lignes peintes : fenêtre visible + débordement des deux côtés.
    let max_rows = (SCREEN.1 / ROW_H).ceil() as usize + 2 * OVERSCAN + 1;
    assert!(small_shapes > 0, "aucune forme à 100 tâches");
    for row in [0, len / 2, len - 1] {
        let (shapes, rows) = painted(&mut large, row);
        eprintln!(
            "formes : 100 tâches {small_shapes} ({small_rows:?}), 5 000 tâches ligne {row} : {shapes} ({rows:?})"
        );
        assert!(
            shapes <= 2 * small_shapes,
            "ligne {row} : {shapes} formes à 5 000 tâches > 2 × {small_shapes} à 100 tâches"
        );
        assert!(
            rows.len() <= max_rows,
            "ligne {row} : {} lignes peintes > {max_rows}",
            rows.len()
        );
    }
}

/// Appui (et relâchement) d'une touche.
fn key_events(key: Key, modifiers: Modifiers) -> Vec<Event> {
    [true, false]
        .into_iter()
        .map(|pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        })
        .collect()
}

/// Indice de la ligne du curseur (ligne vide : nombre de tâches).
fn cursor_row(app: &App) -> usize {
    let rows = app.rows().unwrap();
    match app.view().cursor.row {
        RowRef::Item(uid) => rows.index_of(uid).unwrap(),
        RowRef::Blank => rows.len(),
    }
}

/// Médiane et 95e centile d'une série (triée ici).
fn stats(mut samples: Vec<Duration>) -> (Duration, Duration, Duration) {
    samples.sort();
    (
        percentile(&samples, 50),
        percentile(&samples, 95),
        *samples.last().unwrap(),
    )
}

/// Seuils de fluidité de la navigation (ADR-0015 D12).
fn assert_budget(label: &str, samples: Vec<Duration>, counts: Counts) {
    let n = samples.len();
    let (median, p95, max) = stats(samples);
    eprintln!(
        "perf UI {label} ({n} images) : médiane {:.2} ms, p95 {:.2} ms, max {:.2} ms, {counts:?}",
        ms(median),
        ms(p95),
        ms(max),
    );
    assert!(median < BUDGET, "{label} : médiane {median:?} ≥ {BUDGET:?}");
    assert!(p95 < SPIKE, "{label} : p95 {p95:?} ≥ {SPIKE:?}");
}

/// Réseau de 5 000 tâches ouvert, zoom Semaine, focus à la table (donné à
/// l'ouverture), images stabilisées.
fn nav_bench() -> (App, Bench) {
    let mut app = app_with(SUMMARIES, LEAVES_PER_SUMMARY, LINKS);
    let mut bench = Bench::new();
    bench.settle(&mut app);
    app.dispatch(Action::SetZoom(TimeScaleZoom::Week));
    bench.settle(&mut app);
    assert_eq!(cursor_row(&app), 0);
    (app, bench)
}

/// J7 L1 : ↓ maintenue, une touche par image sur 300 images.
#[test]
#[ignore = "mesure, à lancer en --release : cargo test -p planrs-ui-egui --release --test perf -- --ignored --nocapture"]
fn nav_down_held() {
    let (mut app, mut bench) = nav_bench();
    for _ in 0..WARMUP {
        bench.frame(&mut app, key_events(Key::ArrowDown, Modifiers::NONE));
    }
    let start = cursor_row(&app);
    let samples: Vec<Duration> = (0..300)
        .map(|_| bench.frame(&mut app, key_events(Key::ArrowDown, Modifiers::NONE)))
        .collect();
    assert_eq!(cursor_row(&app), start + 300, "une ligne par image");
    assert!(app.last_frame.table_rows.contains(&(start + 300)));
    assert_budget("nav_down_held", samples, bench.last);
}

/// J7 L1 : Page bas jusqu'à la fin, puis Ctrl+Origine.
#[test]
#[ignore = "mesure, à lancer en --release : cargo test -p planrs-ui-egui --release --test perf -- --ignored --nocapture"]
fn nav_page_sweep() {
    let (mut app, mut bench) = nav_bench();
    let blank = app.rows().unwrap().len();
    let mut samples = Vec::new();
    while cursor_row(&app) < blank {
        assert!(samples.len() < 1_000, "Page bas n'avance pas");
        samples.push(bench.frame(&mut app, key_events(Key::PageDown, Modifiers::NONE)));
    }
    samples.push(bench.frame(&mut app, key_events(Key::Home, Modifiers::COMMAND)));
    for _ in 0..3 {
        samples.push(bench.frame(&mut app, Vec::new()));
    }
    assert_eq!(cursor_row(&app), 0);
    assert_eq!(app.view().cursor.column, Column::Id);
    assert_eq!(app.last_frame.table_rows.start, 0);
    assert_budget("nav_page_sweep", samples, bench.last);
}

/// J7 L1 : latence d'édition (ADR-0015 D4) : validation d'une durée par
/// Entrée → recalcul → `RowModel` → image suivante, sur 20 tâches
/// réparties. Mesure publiée, sans porte (lot L16) ; décomposition par une
/// copie de la session (même lot appliqué, `RowModel::build` seul).
#[test]
#[ignore = "mesure, à lancer en --release : cargo test -p planrs-ui-egui --release --test perf -- --ignored --nocapture"]
fn edit_latency() {
    let (mut app, mut bench) = nav_bench();
    let labels = planrs_ui_egui::i18n::Texts::new(planrs_ui_egui::Language::Fr)
        .format_labels()
        .clone();
    // Colonne Durée : deux crans à droite depuis l'ID.
    app.dispatch(Action::Nav {
        mv: planrs_ui_egui::nav::NavMove::RowStart,
        extend: false,
    });
    for _ in 0..2 {
        app.dispatch(Action::Nav {
            mv: planrs_ui_egui::nav::NavMove::Right,
            extend: false,
        });
    }
    bench.settle(&mut app);
    assert_eq!(app.view().cursor.column, Column::Duration);
    let len = app.rows().unwrap().len();
    let (mut total, mut apply, mut build) = (Vec::new(), Vec::new(), Vec::new());
    for k in 0..20 {
        let index = (7 + k * len / 20).min(len - 1);
        let row = app.rows().unwrap().row(index).unwrap();
        assert!(matches!(row.kind, RowKind::Leaf | RowKind::Milestone), "{index}");
        let uid = row.uid;
        app.dispatch(Action::ScrollToRow(index));
        app.dispatch(Action::Select(Some(uid)));
        bench.settle(&mut app);
        // Décomposition, hors image, sur une copie du document.
        let doc = app.document.as_ref().unwrap();
        let state = EditState::typed(Some(uid), Column::Duration, "3j");
        let batch = commit_edit(&state, &doc.session, planrs_automation::text::Language::Fr).unwrap();
        let mut copy = doc.clone();
        let t0 = Instant::now();
        copy.session
            .apply(&batch, planrs_automation::ApplyMode::Commit)
            .unwrap();
        apply.push(t0.elapsed());
        let t0 = Instant::now();
        black_box(RowModel::build(&copy, &labels));
        build.push(t0.elapsed());
        // Mesure : saisie ouverte par la frappe, puis Entrée et l'image
        // suivante.
        let generation = app.document.as_ref().unwrap().generation();
        bench.frame(&mut app, vec![Event::Text("3j".to_owned())]);
        assert!(app.view().edit.editing.is_some(), "saisie non ouverte ({index})");
        let enter = bench.frame(&mut app, key_events(Key::Enter, Modifiers::NONE));
        let next = bench.frame(&mut app, Vec::new());
        assert_eq!(app.document.as_ref().unwrap().generation(), generation + 1);
        assert!(app.messages().is_empty(), "{:?}", app.messages());
        total.push(enter + next);
    }
    let (median, p95, max) = stats(total);
    let (apply_median, _, _) = stats(apply);
    let (build_median, _, _) = stats(build);
    eprintln!(
        "perf UI edit_latency (20 validations) : médiane {:.1} ms, p95 {:.1} ms, max {:.1} ms ; \
         décomposition (médianes) : recalcul (Session::apply) {:.1} ms, RowModel::build {:.1} ms, \
         reste (image, tessellation) {:.1} ms ; budget L16 : médiane < 100 ms, p95 < 150 ms",
        ms(median),
        ms(p95),
        ms(max),
        ms(apply_median),
        ms(build_median),
        (ms(median) - ms(apply_median) - ms(build_median)).max(0.0),
    );
}

/// Relevé stabilisé après `events` (une image par lot d'événements).
fn counts_after(app: &mut App, bench: &mut Bench, events: &[Vec<Event>]) -> Counts {
    for e in events {
        bench.frame(app, e.clone());
    }
    bench.settle(app);
    bench.last
}

/// J7 L1 (garde indépendante de la machine) : à 1 000 et 5 000 tâches,
/// défilées en haut au zoom Semaine, formes, sommets et galleys égaux à
/// ± 5 %, au départ puis après des déplacements du curseur au clavier, puis
/// toutes les colonnes affichées et la table défilée jusqu'à la dernière
/// (J7 L6).
#[test]
fn frame_counts_equal_at_1000_and_5000_tasks() {
    let scenarios: Vec<Vec<Vec<Event>>> = vec![
        Vec::new(),
        vec![
            key_events(Key::ArrowDown, Modifiers::NONE),
            key_events(Key::PageDown, Modifiers::NONE),
            key_events(Key::Tab, Modifiers::NONE),
            key_events(Key::PageDown, Modifiers::NONE),
            key_events(Key::ArrowDown, Modifiers::SHIFT),
        ],
    ];
    let mut measured = Vec::new();
    for summaries in [50, SUMMARIES] {
        let mut app = app_with(summaries, LEAVES_PER_SUMMARY, 2 * summaries * LEAVES_PER_SUMMARY);
        let mut bench = Bench::new();
        bench.settle(&mut app);
        app.dispatch(Action::SetZoom(TimeScaleZoom::Week));
        bench.settle(&mut app);
        let x = app.view().gantt_scroll_x;
        app.dispatch(Action::ScrollGanttX(-x));
        bench.settle(&mut app);
        let mut counts: Vec<Counts> = scenarios
            .iter()
            .map(|events| counts_after(&mut app, &mut bench, events))
            .collect();
        // J7 L6 : toutes les colonnes affichées (textes des colonnes de
        // suivi), table défilée horizontalement par Fin.
        for column in Column::ALL {
            if !app.state.column(column).is_some_and(|c| c.visible) {
                app.dispatch(Action::ToggleColumn(column));
            }
        }
        bench.settle(&mut app);
        counts.push(counts_after(
            &mut app,
            &mut bench,
            &[key_events(Key::End, Modifiers::NONE)],
        ));
        assert!(app.view().table_scroll_x > 0.0, "table non défilée");
        eprintln!("{} tâches : {counts:?}", app.rows().unwrap().len());
        measured.push(counts);
    }
    let close = |a: usize, b: usize| a.abs_diff(b) * 20 <= a.max(b);
    for (small, large) in measured[0].iter().zip(&measured[1]) {
        assert!(
            small.shapes > 0 && small.galleys > 0 && small.vertices > 0,
            "{small:?}"
        );
        assert!(close(small.shapes, large.shapes), "formes {small:?} / {large:?}");
        assert!(
            close(small.vertices, large.vertices),
            "sommets {small:?} / {large:?}"
        );
        assert!(
            close(small.galleys, large.galleys),
            "galleys {small:?} / {large:?}"
        );
    }
}

/// J7 L3 : bandeau de validation d'une écriture d'un agent affiché
/// (H-UI-27), 5 000 tâches, défilement de la table à chaque image ; même
/// budget que la vue principale (médiane < 16 ms).
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "mesure, à lancer en --release : cargo test -p planrs-ui-egui --release --test perf -- --ignored --nocapture"]
fn approval_banner() {
    use std::sync::Arc;
    use std::sync::mpsc::sync_channel;

    use planrs_automation::control::{CallState, RelayCall, RelayKind, Reply};

    let (mut app, mut bench) = nav_bench();
    let (tx, rx) = sync_channel(4);
    app.control_inbox(rx);
    let (reply, replies) = sync_channel(4);
    tx.send(RelayCall {
        id: 1,
        kind: RelayKind::Execute {
            tool: "task.update".to_owned(),
            params: serde_json::json!({ "uid": 3, "name": "Proposé par l'agent" }),
        },
        state: Arc::new(CallState::new()),
        reply,
    })
    .unwrap();
    bench.settle(&mut app);
    assert_eq!(replies.try_recv().unwrap(), Reply::PendingApproval { id: 1 });
    assert_eq!(app.relay().unwrap().pending().len(), 1);
    for _ in 0..WARMUP {
        bench.frame(&mut app, wheel_events(-WHEEL_STEP));
    }
    let samples: Vec<Duration> = (0..120)
        .map(|i| {
            let dy = if i % 2 == 0 { -WHEEL_STEP } else { WHEEL_STEP };
            bench.frame(&mut app, wheel_events(dy))
        })
        .collect();
    assert_eq!(
        app.relay().unwrap().pending().len(),
        1,
        "bandeau toujours affiché"
    );
    assert_budget("approval_banner", samples, bench.last);
}

/// Images de clic (appui puis relâchement) au point `p`.
fn click_frames(bench: &mut Bench, app: &mut App, p: Pos2) {
    bench.frame(app, vec![Event::PointerMoved(p)]);
    for pressed in [true, false] {
        bench.frame(
            app,
            vec![Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }],
        );
    }
}

/// Ligne sous l'ordonnée `y` du Gantt d'après la sélection posée par un
/// clic (abscisse `x`, sans barre ni lien supposés : seul l'indice compte).
fn row_clicked(bench: &mut Bench, app: &mut App, x: f32, y: f32) -> Option<usize> {
    click_frames(bench, app, pos2(x, y));
    let uid = app.selection?;
    app.rows().unwrap().index_of(uid)
}

/// J7 L8 : glisser d'une barre sur 120 images (ADR-0015 D12), réseau de
/// 5 000 tâches au zoom Semaine. Le haut de la ligne visée est retrouvé par
/// des clics (la sélection donne la ligne sous le pointeur), puis la barre
/// est saisie en son milieu (abscisse par `TimeAxis::x`).
#[test]
#[ignore = "mesure, à lancer en --release : cargo test -p planrs-ui-egui --release --test perf -- --ignored --nocapture"]
fn bar_drag() {
    let (mut app, mut bench) = nav_bench();
    // Une colonne du Gantt loin des barres du début : x = bord gauche + 5.
    let probe_x = app.view().last_gantt_left + 5.0;
    let probe_row = row_clicked(&mut bench, &mut app, probe_x, 500.0).expect("aucune ligne à y = 500");
    // Haut de la ligne probe_row, au point près : plus petit y qui la donne encore.
    let (mut lo, mut hi) = (500.0 - ROW_H, 500.0_f32);
    while hi - lo > 0.5 {
        let mid = f32::midpoint(lo, hi);
        if row_clicked(&mut bench, &mut app, probe_x, mid) == Some(probe_row) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let top_k = hi;
    // Feuille non commencée près de probe_row, barre amenée à 200 points du bord.
    let row_index = (probe_row..probe_row + 10)
        .find(|&row_index| {
            let row = app.rows().unwrap().row(row_index).unwrap();
            row.kind == RowKind::Leaf && row.bar.is_some() && row.percent.get() == 0
        })
        .expect("aucune feuille près de la ligne visée");
    let uid = app.rows().unwrap().row(row_index).unwrap().uid;
    scroll_gantt_to_row(&mut app, row_index);
    bench.settle(&mut app);
    let y = top_k + (row_index - probe_row) as f32 * ROW_H + ROW_H / 2.0;
    let doc = app.document.as_ref().unwrap();
    let (start, finish) = app.rows().unwrap().row(row_index).unwrap().bar.unwrap();
    let axis = TimeAxis::new(
        doc.range,
        app.state.zoom,
        app.view().last_gantt_left - app.view().gantt_scroll_x,
    );
    let x = f32::midpoint(axis.x(start), axis.x(finish));
    let generation = doc.generation();
    bench.frame(&mut app, vec![Event::PointerMoved(pos2(x, y))]);
    bench.frame(
        &mut app,
        vec![Event::PointerButton {
            pos: pos2(x, y),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }],
    );
    // Au-delà du seuil de glisser d'egui (6 points).
    for i in 1..=WARMUP {
        bench.frame(&mut app, vec![Event::PointerMoved(pos2(x + 2.0 * i as f32, y))]);
    }
    let drag = app.view().drag.unwrap_or_else(|| {
        panic!(
            "glisser non commencé : ligne {row_index}, y {y}, x {x}, sélection {:?}",
            app.selection
        )
    });
    assert_eq!(drag.uid, uid);
    let samples: Vec<Duration> = (0..120)
        .map(|i| {
            let p = pos2(x + (2 * WARMUP + i) as f32, y);
            bench.frame(&mut app, vec![Event::PointerMoved(p)])
        })
        .collect();
    let doc = app.document.as_ref().unwrap();
    assert_eq!(doc.generation(), generation, "recalcul pendant le glisser");
    let days = app.view().drag.unwrap().days;
    assert!(days > 0, "écart nul après 125 points");
    let end = pos2(x + (2 * WARMUP + 120) as f32, y);
    bench.frame(
        &mut app,
        vec![Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    bench.settle(&mut app);
    assert!(app.view().drag.is_none());
    assert_eq!(
        app.document.as_ref().unwrap().generation(),
        generation + 1,
        "{:?}",
        app.messages()
    );
    assert_budget("bar_drag", samples, bench.last);
}

/// Ressources du réseau mesuré par `usage_scroll` (ADR-0015 § 0).
const USAGE_RESOURCES: usize = 1_200;

/// Réseau de [`network_xml`] dont chaque feuille (hors jalons) est affectée
/// à 100 % à la ressource `k mod resources` (travail = durée).
fn network_with_resources(summaries: usize, leaves: usize, max_links: usize, resources: usize) -> Vec<u8> {
    let base = String::from_utf8(network_xml(summaries, leaves, max_links)).unwrap();
    let body = base.strip_suffix("</Project>").unwrap();
    let mut xml = String::with_capacity(base.len() + summaries * leaves * 200);
    xml.push_str(body);
    xml.push_str("<Resources>");
    for r in 1..=resources {
        write!(
            xml,
            "<Resource><UID>{r}</UID><ID>{r}</ID><Name>Équipe {r} — compagnons</Name><Type>1</Type>\
             <MaxUnits>1</MaxUnits></Resource>"
        )
        .unwrap();
    }
    xml.push_str("</Resources><Assignments>");
    let leaf_uid = |k: usize| (k / leaves) * (leaves + 1) + (k % leaves) + 2;
    let mut uid = 0;
    for k in 0..summaries * leaves {
        if k % 97 == 96 {
            continue;
        }
        uid += 1;
        let hours = 4 + (k * 7) % 13;
        write!(
            xml,
            "<Assignment><UID>{uid}</UID><TaskUID>{t}</TaskUID><ResourceUID>{r}</ResourceUID>\
             <Units>1</Units><Work>PT{hours}H0M0S</Work></Assignment>",
            t = leaf_uid(k),
            r = k % resources + 1
        )
        .unwrap();
    }
    xml.push_str("</Assignments></Project>");
    xml.into_bytes()
}

/// Application ouverte sur un réseau affecté, vue Utilisation des
/// ressources, zoom `zoom`, défilée au début.
fn usage_app(summaries: usize, resources: usize, zoom: TimeScaleZoom) -> (App, Bench) {
    let state = UiState {
        language: LanguagePref::Fr,
        theme: ThemePref::Light,
        ..UiState::default()
    };
    let mut app = App::new(state, Env::fixed(now()));
    let origin = DocOrigin {
        name: "utilisation.xml".to_owned(),
        path: None,
        demo: false,
    };
    let leaves = LEAVES_PER_SUMMARY;
    let xml = network_with_resources(summaries, leaves, 2 * summaries * leaves, resources);
    app.open_bytes(&xml, origin);
    let document = app
        .document
        .as_ref()
        .unwrap_or_else(|| panic!("réseau affecté refusé : {:?}", app.messages()));
    assert_eq!(document.project().resources.len(), resources);
    assert!(!document.schedule().assignments.is_empty());
    let mut bench = Bench::new();
    bench.settle(&mut app);
    app.dispatch(Action::SetView(ViewKind::ResourceUsage));
    app.dispatch(Action::SetZoom(zoom));
    bench.settle(&mut app);
    let x = app.view().gantt_scroll_x;
    app.dispatch(Action::ScrollGanttX(-x));
    bench.settle(&mut app);
    assert_eq!(app.state.view, ViewKind::ResourceUsage);
    (app, bench)
}

/// J7 L11 (garde indépendante de la machine) : utilisation des ressources
/// à 1 000 et 5 000 tâches (60 ressources, mêmes premières tâches), en haut
/// puis après des déplacements au clavier : formes, sommets et galleys
/// égaux à ± 5 % (virtualisation des lignes et des périodes).
#[test]
fn usage_counts_equal_at_1000_and_5000_tasks() {
    let moves = vec![
        key_events(Key::ArrowDown, Modifiers::NONE),
        key_events(Key::PageDown, Modifiers::NONE),
        key_events(Key::Tab, Modifiers::NONE),
    ];
    let mut measured = Vec::new();
    for summaries in [50, SUMMARIES] {
        let (mut app, mut bench) = usage_app(summaries, 60, TimeScaleZoom::Week);
        let start = bench.last;
        let after = counts_after(&mut app, &mut bench, &moves);
        let cells = app.view().usage.last_cells.len();
        eprintln!("utilisation, {summaries} récapitulatives : {start:?} {after:?}, {cells} cellules");
        assert!(cells > 0, "aucune cellule");
        measured.push([start, after]);
    }
    let close = |a: usize, b: usize| a.abs_diff(b) * 20 <= a.max(b);
    for (small, large) in measured[0].iter().zip(&measured[1]) {
        assert!(small.shapes > 0 && small.galleys > 0, "{small:?}");
        assert!(close(small.shapes, large.shapes), "formes {small:?} / {large:?}");
        assert!(
            close(small.vertices, large.vertices),
            "sommets {small:?} / {large:?}"
        );
        assert!(
            close(small.galleys, large.galleys),
            "galleys {small:?} / {large:?}"
        );
    }
}

/// J7 L11 : utilisation des ressources, 1 200 ressources, 5 000 tâches
/// affectées ; molette verticale (120 images) puis défilement horizontal au
/// zoom Jour (120 images, un bloc neuf toutes les ~8 images) ; médiane
/// < 16 ms, p95 < 32 ms, pire image (bloc neuf) < 32 ms.
#[test]
#[ignore = "mesure, à lancer en --release : cargo test -p planrs-ui-egui --release --test perf -- --ignored --nocapture"]
fn usage_scroll() {
    let (mut app, mut bench) = usage_app(SUMMARIES, USAGE_RESOURCES, TimeScaleZoom::Week);
    for _ in 0..WARMUP {
        bench.frame(&mut app, wheel_events(-WHEEL_STEP));
    }
    let first = app.view().usage.last_scroll_y;
    let queries = app.view().usage.cache.queries();
    let vertical: Vec<Duration> = (0..120)
        .map(|_| bench.frame(&mut app, wheel_events(-WHEEL_STEP)))
        .collect();
    assert!(
        app.view().usage.last_scroll_y > first,
        "la molette n'a pas fait défiler"
    );
    let fetched_v = app.view().usage.cache.queries() - queries;
    assert_budget("usage_scroll vertical", vertical.clone(), bench.last);

    app.dispatch(Action::SetZoom(TimeScaleZoom::Day));
    bench.settle(&mut app);
    let x = app.view().gantt_scroll_x;
    app.dispatch(Action::ScrollGanttX(-x));
    bench.settle(&mut app);
    let queries = app.view().usage.cache.queries();
    let horizontal: Vec<Duration> = (0..120)
        .map(|_| {
            app.dispatch(Action::ScrollGanttX(300.0));
            bench.frame(&mut app, Vec::new())
        })
        .collect();
    let fetched_h = app.view().usage.cache.queries() - queries;
    assert!(fetched_h > 2, "aucun bloc neuf ({fetched_h})");
    let worst = *horizontal.iter().chain(&vertical).max().unwrap();
    eprintln!(
        "perf UI usage_scroll : blocs neufs {fetched_v} (vertical), {fetched_h} (horizontal), pire image {:.2} ms",
        ms(worst)
    );
    assert_budget("usage_scroll horizontal", horizontal, bench.last);
    assert!(worst < SPIKE, "bloc neuf : {worst:?} ≥ {SPIKE:?}");
}
