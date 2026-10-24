//! Scénario de bout en bout de l'usage quotidien (J7, lot L16, vérification
//! 5) : un seul test, `e2e_daily_use`, qui enchaîne sur l'exemple les gestes
//! livrés par les lots L1 à L15 — navigation et saisie au clavier,
//! « Informations sur la tâche », gestes dans le Gantt, Gantt de suivi,
//! feuille et utilisation des ressources, temps de travail, ligne de base —
//! puis annule tout (projet égal à l'ouverture), rétablit tout, enregistre
//! par un `FileService` en mémoire et relit les octets écrits.
//!
//! Les touches sont de vrais événements egui (comme l'utilisateur) ; les vues
//! et les fenêtres sont ouvertes par le réducteur (`Action`), comme le font
//! les menus. Aucune date n'est calculée ici : les positions du Gantt viennent
//! de `TimeAxis` (sens date → pixel) et les attendus du moteur.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use planrs_core::{BaselineNumber, Constraint, LinkType, Minutes, Moment, Project, Task, TaskUid, Units};
use planrs_ui_egui::action::DialogKind;
use planrs_ui_egui::column::Column;
use planrs_ui_egui::document;
use planrs_ui_egui::grid;
use planrs_ui_egui::i18n::{Arg, Language, TextId, Texts};
use planrs_ui_egui::nav::RowRef;
use planrs_ui_egui::resource_rows::{COLUMNS, ResourceColumn};
use planrs_ui_egui::rows::RowKind;
use planrs_ui_egui::{Action, App, GanttStyle, TimeScaleZoom, ViewKind};

use crate::files::{SaveReply, sim};
use crate::{fr, now, open_demo, screen_axis};

/// Projet du document.
fn project(app: &App) -> &Project {
    app.document.as_ref().unwrap().project()
}

fn task(app: &App, uid: TaskUid) -> &Task {
    project(app).tasks.get(uid).unwrap()
}

/// (entrées annulables, entrées rétablissables) de l'historique.
fn history(app: &App) -> (usize, usize) {
    let history = app.document.as_ref().unwrap().session.history();
    (history.entries.len(), history.redo.len())
}

/// Aucun message au bandeau.
fn quiet(app: &App) {
    assert!(app.messages().is_empty(), "{:?}", app.messages());
}

/// Indice de la ligne du curseur de la table (ligne vide : nombre de tâches).
fn cursor_row(app: &App) -> usize {
    let rows = app.rows().unwrap();
    match app.view().cursor.row {
        RowRef::Item(uid) => rows.index_of(uid).unwrap(),
        RowRef::Blank => rows.len(),
    }
}

fn uid_at(app: &App, i: usize) -> TaskUid {
    app.rows().unwrap().row(i).unwrap().uid
}

/// Libellé d'accessibilité de la ligne `i` de la table des tâches.
fn row_label(app: &App, i: usize) -> String {
    let rows = app.rows().unwrap();
    let row = rows.row(i).unwrap();
    Texts::new(Language::Fr).get_args(
        TextId::RowLabel,
        &[
            ("id", Arg::Count(i64::from(row.id))),
            ("name", Arg::Str(rows.text(i, Column::Name).to_owned().into())),
        ],
    )
}

/// Ordonnée écran du centre de la ligne `i`.
fn row_y(harness: &Harness<'static, App>, i: usize) -> f32 {
    let label = row_label(harness.state(), i);
    harness.get_by_label(&label).rect().center().y
}

/// Abscisses écran du début et de la fin de la barre de la ligne `i`.
fn bar_x(harness: &Harness<'static, App>, i: usize) -> (f32, f32) {
    let app = harness.state();
    let (start, finish) = app.rows().unwrap().row(i).unwrap().bar.unwrap();
    let axis = screen_axis(app);
    (axis.x(start), axis.x(finish))
}

/// Appuie sur `key` avec `modifiers`, puis laisse l'interface se stabiliser.
fn press(harness: &mut Harness<'static, App>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

/// Frappe `text`.
fn type_text(harness: &mut Harness<'static, App>, text: &str) {
    harness.event(Event::Text(text.to_owned()));
    harness.run();
}

/// Clic simple au point `p`.
fn click_at(harness: &mut Harness<'static, App>, p: Pos2) {
    harness.event(Event::PointerMoved(p));
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
}

/// Clique sur l'unique widget portant `label`.
fn click_label(harness: &mut Harness<'static, App>, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

/// Remplace le texte du champ nommé `label` par `text`.
fn retype(harness: &mut Harness<'static, App>, label: &str, text: &str) {
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, label)
        .focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.event(Event::Text(text.to_owned()));
    harness.run();
}

/// Libellé « champ, ligne n » d'une ligne de fenêtre.
fn row_field(field: TextId, row: usize) -> String {
    Texts::new(Language::Fr).get_args(
        TextId::TaskInfoRowField,
        &[
            ("column", Arg::Str(fr(field).into())),
            ("count", Arg::Count(i64::try_from(row).unwrap())),
        ],
    )
}

/// Presse en `from`, glisse en quatre pas jusqu'à `to`, lâche.
fn drag(harness: &mut Harness<'static, App>, from: Pos2, to: Pos2) {
    harness.hover_at(from);
    harness.step();
    harness.drag_at(from);
    harness.step();
    for k in 1..=4_u8 {
        let t = f32::from(k) / 4.0;
        harness.hover_at(from + (to - from) * t);
        harness.step();
    }
    harness.drop_at(to);
    harness.run();
}

/// Première feuille non commencée d'au moins 3 jours : (ligne, UID).
fn unstarted_leaf(app: &App) -> (usize, TaskUid) {
    let rows = app.rows().unwrap();
    (0..rows.len())
        .find_map(|i| {
            let row = rows.row(i)?;
            let (start, finish) = row.bar?;
            let long = start.minutes_until(finish) >= Minutes::new(3 * 1_440);
            let t = task(app, row.uid);
            let fresh = t.progress.percent_complete.get() == 0 && t.progress.actual_start.is_none();
            (row.kind == RowKind::Leaf && long && fresh).then_some((i, row.uid))
        })
        .expect("aucune feuille non commencée de 3 jours")
}

/// Zoom Jour, ligne `i` visible, début de sa barre à 150 points du bord
/// gauche du Gantt.
fn show_row(harness: &mut Harness<'static, App>, i: usize) {
    harness.state_mut().dispatch(Action::SetZoom(TimeScaleZoom::Day));
    harness.state_mut().dispatch(Action::ScrollToRow(i));
    harness.run();
    let app = harness.state();
    let (start, _) = app.rows().unwrap().row(i).unwrap().bar.unwrap();
    let dx = screen_axis(app).x(start) - app.view().last_gantt_left - 150.0;
    harness.state_mut().dispatch(Action::ScrollGanttX(dx));
    harness.run();
}

fn scheduled_start(app: &App, uid: TaskUid) -> Moment {
    app.document
        .as_ref()
        .unwrap()
        .schedule()
        .scheduled(uid)
        .unwrap()
        .start
}

/// Libellé d'accessibilité de la ligne `i` de la feuille des ressources
/// (ligne vide au-delà).
fn res_row_label(app: &App, i: usize) -> String {
    let rows = &app.view().resources.rows;
    match rows.row(i) {
        Some(row) => app.texts().get_args(
            TextId::ResRowLabel,
            &[
                ("id", Arg::Count(i64::try_from(row.id).unwrap())),
                (
                    "name",
                    Arg::Str(rows.text(i, ResourceColumn::Name).to_owned().into()),
                ),
            ],
        ),
        None => app.texts().get(TextId::ResBlankRowLabel),
    }
}

/// Clic sur la cellule (ligne `i`, `column`) de la feuille des ressources.
fn res_click_cell(harness: &mut Harness<'static, App>, i: usize, column: ResourceColumn) {
    let rect = harness.get_by_label(&res_row_label(harness.state(), i)).rect();
    let (left, width) = grid::column_span(&COLUMNS, rect.left(), column).unwrap();
    click_at(harness, pos2(left + width / 2.0, rect.center().y));
}

/// Frappe `text` dans la feuille des ressources (saisie ouverte), Entrée.
fn res_type_enter(harness: &mut Harness<'static, App>, text: &str) {
    type_text(harness, text);
    assert!(
        harness.state().view().resources.edit.editing.is_some(),
        "saisie de la feuille ouverte"
    );
    press(harness, Modifiers::NONE, Key::Enter);
}

/// Ligne de base `n` posée sur au moins une tâche.
fn has_baseline(app: &App, n: u8) -> bool {
    let number = BaselineNumber::new(n).unwrap();
    project(app)
        .tasks
        .iter()
        .any(|t| t.baselines.contains_key(&number))
}

/// Vérification 5 du lot L16 : usage quotidien de bout en bout.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "un seul scénario linéaire, lisible de haut en bas"
)]
fn e2e_daily_use() {
    let mut s = sim(SaveReply::Ok);
    let harness = &mut s.harness;
    open_demo(harness);
    let initial = project(harness.state()).clone();
    assert_eq!(cursor_row(harness.state()), 0);
    assert_eq!(harness.state().view().cursor.column, Column::Name);

    // 1. ↓ ×3, Origine, Tab ×2 (ID → Nom → Durée), F2, Ctrl+A, « 3j », Entrée.
    for _ in 0..3 {
        press(harness, Modifiers::NONE, Key::ArrowDown);
    }
    assert_eq!(cursor_row(harness.state()), 3);
    press(harness, Modifiers::NONE, Key::Home);
    press(harness, Modifiers::NONE, Key::Tab);
    press(harness, Modifiers::NONE, Key::Tab);
    assert_eq!(harness.state().view().cursor.column, Column::Duration);
    let edited = uid_at(harness.state(), 3);
    assert_eq!(
        harness.state().rows().unwrap().row(3).unwrap().kind,
        RowKind::Leaf
    );
    press(harness, Modifiers::NONE, Key::F2);
    assert!(
        harness.state().view().edit.editing.is_some(),
        "F2 ouvre la saisie"
    );
    // F2 garde le texte en place (« 1 jour ») : tout sélectionner avant de taper.
    press(harness, Modifiers::COMMAND, Key::A);
    type_text(harness, "3j");
    press(harness, Modifiers::NONE, Key::Enter);
    let app = harness.state();
    quiet(app);
    assert!(app.view().edit.editing.is_none(), "Entrée valide");
    assert_eq!(task(app, edited).duration.minutes().get(), 3 * 480);
    assert_eq!(history(app), (1, 0));

    // 2. Maj+Entrée : une ligne plus haut, aucune saisie (H-UI-13).
    let row_after_enter = cursor_row(harness.state());
    press(harness, Modifiers::SHIFT, Key::Enter);
    let app = harness.state();
    assert_eq!(cursor_row(app), row_after_enter - 1);
    assert!(app.view().edit.editing.is_none());

    // 3. Maj+F2, renommer, OK : un seul lot.
    let renamed = uid_at(harness.state(), cursor_row(harness.state()));
    press(harness, Modifiers::SHIFT, Key::F2);
    assert_eq!(
        harness.state().view().dialog.as_ref().map(|d| d.kind),
        Some(DialogKind::TaskInformation(renamed))
    );
    retype(harness, &fr(TextId::TaskInfoName), "Tâche revue");
    click_label(harness, &fr(TextId::TaskInfoOk));
    let app = harness.state();
    quiet(app);
    assert!(app.view().dialog.is_none(), "fenêtre fermée");
    assert_eq!(task(app, renamed).name, "Tâche revue");
    assert_eq!(history(app), (2, 0));

    // 4. Glisser une barre de +2 jours : contrainte « début au plus tôt »
    // (R-240), un seul lot.
    let (i, moved) = unstarted_leaf(harness.state());
    show_row(harness, i);
    let day = harness.state().state.zoom.px_per_day();
    let start = scheduled_start(harness.state(), moved);
    let y = row_y(harness, i);
    let (x0, x1) = bar_x(harness, i);
    let from = pos2(f32::midpoint(x0, x1), y);
    drag(harness, from, from + egui::vec2(2.0 * day, 0.0));
    let app = harness.state();
    quiet(app);
    let expected = start.checked_add(Minutes::new(2 * 1_440)).unwrap();
    assert_eq!(
        task(app, moved).constraint,
        Constraint::StartNoEarlierThan { date: expected }
    );
    assert_eq!(history(app), (3, 0));

    // 5. Tracer un lien : de la barre déplacée vers une ligne suivante.
    let target = (i + 1..i + 6)
        .find(|&j| {
            let app = harness.state();
            app.rows().unwrap().row(j).is_some_and(|row| {
                row.kind == RowKind::Leaf
                    && !task(app, row.uid)
                        .predecessors
                        .iter()
                        .any(|l| l.predecessor == moved)
            })
        })
        .expect("aucune ligne cible");
    let target_uid = uid_at(harness.state(), target);
    let (x0, x1) = bar_x(harness, i);
    let from = pos2(f32::midpoint(x0, x1), row_y(harness, i));
    let to = pos2(from.x, row_y(harness, target));
    drag(harness, from, to);
    let app = harness.state();
    quiet(app);
    assert!(
        task(app, target_uid)
            .predecessors
            .iter()
            .any(|l| l.predecessor == moved && l.kind == LinkType::FinishToStart),
        "{:?}",
        task(app, target_uid).predecessors
    );
    assert_eq!(history(app), (4, 0));

    // 6. Gantt de suivi.
    harness
        .state_mut()
        .dispatch(Action::SetGanttStyle(GanttStyle::Tracking));
    harness.run();
    assert_eq!(harness.state().state.gantt_style, GanttStyle::Tracking);
    quiet(harness.state());

    // 7. Feuille des ressources : « Grutier » sur la ligne vide, capacité 50 %.
    harness
        .state_mut()
        .dispatch(Action::SetView(ViewKind::ResourceSheet));
    harness.run();
    assert_eq!(harness.state().state.view, ViewKind::ResourceSheet);
    assert!(
        project(harness.state()).resources.is_empty(),
        "l'exemple n'a aucune ressource"
    );
    res_click_cell(harness, 0, ResourceColumn::Name);
    res_type_enter(harness, "Grutier");
    let app = harness.state();
    quiet(app);
    assert_eq!(project(app).resources.len(), 1);
    assert_eq!(app.view().resources.rows.text(0, ResourceColumn::Name), "Grutier");
    assert_eq!(history(app), (5, 0));
    res_click_cell(harness, 0, ResourceColumn::MaxUnits);
    res_type_enter(harness, "50");
    let app = harness.state();
    quiet(app);
    let crane = project(app).resources.iter().next().unwrap();
    assert_eq!(crane.name, "Grutier");
    assert_eq!(crane.max_units, Units::new(5_000).unwrap());
    assert_eq!(history(app), (6, 0));

    // 8. Utilisation des ressources : la vue s'affiche sans message.
    harness
        .state_mut()
        .dispatch(Action::SetView(ViewKind::ResourceUsage));
    harness.run();
    assert_eq!(harness.state().state.view, ViewKind::ResourceUsage);
    quiet(harness.state());

    // 9. Temps de travail : exception nommée « Pont » sur le calendrier du projet.
    harness.state_mut().dispatch(Action::SetView(ViewKind::Gantt));
    harness.run();
    let calendar = project(harness.state()).calendar;
    let exceptions_before = project(harness.state())
        .calendars
        .get(calendar)
        .unwrap()
        .exceptions()
        .len();
    harness
        .state_mut()
        .dispatch(Action::ShowDialog(DialogKind::WorkingTime));
    harness.run();
    click_label(harness, &fr(TextId::WtExcAdd));
    let row = exceptions_before + 1;
    retype(harness, &row_field(TextId::WtExcName, row), "Pont");
    retype(harness, &row_field(TextId::WtExcFrom, row), "14/05/2027");
    retype(harness, &row_field(TextId::WtExcTo, row), "14/05/2027");
    click_label(harness, &fr(TextId::TaskInfoOk));
    let app = harness.state();
    quiet(app);
    assert!(app.view().dialog.is_none(), "fenêtre fermée");
    let exceptions = project(app).calendars.get(calendar).unwrap().exceptions();
    assert_eq!(exceptions.len(), exceptions_before + 1);
    let bridge = exceptions
        .iter()
        .find(|e| e.name == "Pont")
        .expect("exception « Pont »");
    assert_eq!(bridge.period.first().to_string(), "2027-05-14");
    assert_eq!(history(app), (7, 0));

    // 10. Ligne de base 0 définie, puis effacée (projet entier).
    assert!(!has_baseline(harness.state(), 0));
    harness
        .state_mut()
        .dispatch(Action::ShowDialog(DialogKind::Baseline));
    harness.run();
    click_label(harness, &fr(TextId::TaskInfoOk));
    let app = harness.state();
    quiet(app);
    assert!(app.view().dialog.is_none());
    assert!(has_baseline(app, 0), "ligne de base 0 posée");
    assert_eq!(history(app), (8, 0));
    harness
        .state_mut()
        .dispatch(Action::ShowDialog(DialogKind::Baseline));
    harness.run();
    click_label(harness, &fr(TextId::BlClear));
    click_label(harness, &fr(TextId::TaskInfoOk));
    let app = harness.state();
    quiet(app);
    assert!(app.view().dialog.is_none());
    assert!(!has_baseline(app, 0), "ligne de base 0 effacée");
    assert_eq!(history(app), (9, 0));
    let edited = project(app).clone();
    assert_ne!(edited, initial);

    // 11. Annuler tout ⇒ projet égal à l'ouverture, document propre.
    while history(harness.state()).0 > 0 {
        harness.state_mut().dispatch(Action::Undo);
        harness.run();
    }
    let app = harness.state();
    quiet(app);
    assert_eq!(project(app), &initial);
    assert_eq!(history(app), (0, 9));
    assert!(!app.document.as_ref().unwrap().is_dirty());

    // 12. Rétablir tout ⇒ projet égal à l'état édité.
    while history(harness.state()).1 > 0 {
        harness.state_mut().dispatch(Action::Redo);
        harness.run();
    }
    let app = harness.state();
    quiet(app);
    assert_eq!(project(app), &edited);
    assert_eq!(history(app), (9, 0));
    assert!(app.document.as_ref().unwrap().is_dirty());

    // 13. Enregistrer (`FileService` en mémoire) et relire : mêmes tâches,
    // même ressource.
    press(harness, Modifiers::COMMAND, Key::S);
    let requests = s.requests.borrow();
    assert_eq!(requests.len(), 1, "une demande d'enregistrement");
    let bytes = &requests[0].bytes;
    let app = s.harness.state();
    assert!(!app.document.as_ref().unwrap().is_dirty(), "enregistré");
    let reread = document::load(bytes, now()).unwrap();
    let summary = |p: &Project| -> Vec<(String, Minutes)> {
        p.tasks
            .iter()
            .map(|t| (t.name.clone(), t.duration.minutes()))
            .collect()
    };
    assert_eq!(reread.session.project().tasks.len(), edited.tasks.len());
    assert_eq!(summary(reread.session.project()), summary(&edited));
    let resources: Vec<&str> = reread
        .session
        .project()
        .resources
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(resources, ["Grutier"]);
    assert_eq!(
        reread.session.project().tasks.get(moved).unwrap().constraint,
        edited.tasks.get(moved).unwrap().constraint
    );
}
