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
//!   machine.
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

use egui::{
    Event, Modifiers, MouseWheelSource, MouseWheelUnit, Pos2, RawInput, Rect, TouchPhase, pos2, vec2,
};
use planrs_core::Moment;
use planrs_ui_egui::timescale::TimeAxis;
use planrs_ui_egui::view::{OVERSCAN, ROW_H};
use planrs_ui_egui::{Action, App, DocOrigin, Env, LanguagePref, ThemePref, TimeScaleZoom, UiState};

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

/// Contexte egui nu et horloge simulée des images.
struct Bench {
    ctx: egui::Context,
    time: f64,
}

impl Bench {
    fn new() -> Self {
        Self {
            ctx: egui::Context::default(),
            time: 0.0,
        }
    }

    /// Une image ; rend la durée de `run_ui` + `tessellate`.
    fn frame(&mut self, app: &mut App, events: Vec<Event>) -> Duration {
        self.time += 1.0 / 60.0;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(SCREEN.0, SCREEN.1))),
            time: Some(self.time),
            events,
            ..RawInput::default()
        };
        let t0 = Instant::now();
        let mut output = self.ctx.run_ui(raw, |ui| planrs_ui_egui::ui(ui, app));
        let primitives = self
            .ctx
            .tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
        let elapsed = t0.elapsed();
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
