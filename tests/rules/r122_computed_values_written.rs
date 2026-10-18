//! R-122 — Écriture : valeurs calculées et politique (`schedule_gaps`,
//! `choose_computed`).

use planrs_core::{
    Assignment, AssignmentUid, CalendarUid, Duration, DurationUnit, ExtraEntry, Extras, Link, Minutes,
    Percent, Resource, ResourceUid, ScheduleAnchor, SchedulingMode, Task, TaskUid, TimeBasis, schedule,
};
use planrs_mspdi::write::{
    Computed, ScheduleGap, WriteCode, WriteOptions, choose_computed, schedule_gaps, write,
};
use planrs_tests::moment;
use planrs_tests::mspdi::{compact, new_project, reread, text};

fn days(n: i64) -> Duration {
    Duration::new(
        Minutes::new(n * 480),
        DurationUnit::Day,
        TimeBasis::Working,
        false,
    )
    .unwrap()
}

fn task(proj: &planrs_core::Project, uid: u32, d: i64) -> Task {
    let mut t = Task::new(TaskUid::new(uid), format!("T{uid}"), &proj.options);
    t.duration = days(d);
    t
}

/// Contenu de l'élément `Task` d'UID `uid` (texte compact).
fn task_xml(xml: &str, uid: u32) -> String {
    let marker = format!("<Task><UID>{uid}</UID>");
    let start = xml.find(&marker).unwrap();
    let rest = xml.get(start..).unwrap();
    rest.get(..rest.find("</Task>").unwrap()).unwrap().to_owned()
}

#[test]
fn r122_schedule_values_are_written() {
    // A 2 j → B 1 j (FS), lun 19/10/2026 08:00 ; UID 0 présent.
    let mut proj = new_project("2026-10-19T08:00");
    proj.summary_task = Some(Task::new(TaskUid::PROJECT_SUMMARY, "P".to_owned(), &proj.options));
    let a = task(&proj, 1, 2);
    let mut b = task(&proj, 2, 1);
    b.predecessors = vec![Link::finish_to_start(TaskUid::new(1))];
    proj.tasks.push(a).unwrap();
    proj.tasks.push(b).unwrap();
    let sched = schedule(&proj);
    let (computed, diags) = choose_computed(&proj, &sched, None);
    assert!(matches!(computed, Computed::Schedule(_)));
    assert_eq!(diags, Vec::new());
    let out = write(&proj, computed, &WriteOptions::default());
    let xml = compact(&text(&out));
    let a = task_xml(&xml, 1);
    assert!(a.contains("<Start>2026-10-19T08:00:00</Start><Finish>2026-10-20T17:00:00</Finish>"));
    assert!(a.contains(
        "<EarlyStart>2026-10-19T08:00:00</EarlyStart><EarlyFinish>2026-10-20T17:00:00</EarlyFinish>"
    ));
    assert!(
        a.contains("<LateStart>2026-10-19T08:00:00</LateStart><LateFinish>2026-10-20T17:00:00</LateFinish>")
    );
    assert!(a.contains("<FreeSlack>0</FreeSlack><TotalSlack>0</TotalSlack><StartSlack>0</StartSlack><FinishSlack>0</FinishSlack>"));
    assert!(a.contains("<Critical>1</Critical>"));
    // UID 0 : durée calculée de la récapitulative du projet (3 j = 24 h).
    let p0 = task_xml(&xml, 0);
    assert!(p0.contains("<Duration>PT24H0M0S</Duration>"), "{p0}");
    assert!(xml.contains("<FinishDate>2026-10-21T17:00:00</FinishDate>"));
    // Relu : valeurs enregistrées = valeurs du moteur ; tâches égales à leur
    // image par l'aller-retour (la valeur calculée n'entre pas dans le
    // modèle, ADR-0004 ; seul le reste d'une tâche sans avancement est
    // dérivé, R-127).
    let back = reread(&out.xml);
    assert_eq!(
        back.project.tasks,
        planrs_mspdi::normalize(&proj, &WriteOptions::default()).tasks
    );
    assert_eq!(
        back.project
            .tasks
            .get(TaskUid::new(1))
            .unwrap()
            .progress
            .remaining_duration,
        planrs_core::Minutes::new(960)
    );
    let rec = &back.recorded.tasks[&TaskUid::new(1)];
    assert_eq!(rec.total_slack, Some(planrs_mspdi::Tenths(0)));
    assert_eq!(back.recorded.project_finish, Some(moment("2026-10-21T17:00")));
}

#[test]
fn r122_slack_in_tenths_inactive_and_manual() {
    // A 1 j et C 73 j seules : TF(A) = 72 j = 34 560 min → 345 600.
    let mut proj = new_project("2026-10-19T08:00");
    proj.tasks.push(task(&proj, 1, 1)).unwrap();
    proj.tasks.push(task(&proj, 2, 73)).unwrap();
    let mut inactive = task(&proj, 3, 1);
    inactive.active = false;
    proj.tasks.push(inactive).unwrap();
    let mut manual = task(&proj, 4, 1);
    manual.mode = SchedulingMode::Manual {
        start: None,
        finish: None,
        duration: None,
    };
    proj.tasks.push(manual).unwrap();
    let sched = schedule(&proj);
    let out = write(&proj, Computed::Schedule(&sched), &WriteOptions::default());
    let xml = compact(&text(&out));
    assert!(task_xml(&xml, 1).contains("<TotalSlack>345600</TotalSlack>"));
    // Inactive : dates d'affichage seulement.
    let i = task_xml(&xml, 3);
    assert!(i.contains("<Start>2026-10-19T08:00:00</Start>"), "{i}");
    assert!(!i.contains("<EarlyStart>") && !i.contains("<TotalSlack>") && !i.contains("<Critical>"));
    // Manuelle sans date : exclue, aucune valeur calculée.
    let m = task_xml(&xml, 4);
    assert!(
        !m.contains("<Start>") && !m.contains("<EarlyStart>") && !m.contains("<Critical>"),
        "{m}"
    );
}

#[test]
fn r122_policy_with_progress() {
    // Une tâche à 50 % : planning incomplet. Sans valeurs relues → omises,
    // `mspdi.write.schedule_incomplete { [progress] }` ; avec → relues.
    let mut proj = new_project("2026-10-19T08:00");
    let mut a = task(&proj, 1, 2);
    a.progress.percent_complete = Percent::new(5_000).unwrap();
    proj.tasks.push(a).unwrap();
    assert_eq!(schedule_gaps(&proj), [ScheduleGap::Progress]);
    let sched = schedule(&proj);
    let (computed, diags) = choose_computed(&proj, &sched, None);
    assert!(matches!(computed, Computed::Omit));
    assert_eq!(
        diags
            .iter()
            .map(|d| (d.code.code(), d.code.clone()))
            .collect::<Vec<_>>(),
        [(
            "mspdi.write.schedule_incomplete",
            WriteCode::ScheduleIncomplete {
                gaps: vec![ScheduleGap::Progress]
            }
        )]
    );
    let omitted = compact(&text(&write(&proj, computed, &WriteOptions::default())));
    assert!(!omitted.contains("<EarlyStart>") && !omitted.contains("<FinishDate>"));
    // Valeurs relues (fichier écrit par MS Project : ici, celles du
    // moteur relues), réécrites telles quelles.
    let first = reread(&write(&proj, Computed::Schedule(&sched), &WriteOptions::default()).xml);
    let (computed, diags) = choose_computed(&proj, &sched, Some(&first.recorded));
    assert!(matches!(computed, Computed::Recorded(_)));
    assert_eq!(diags, Vec::new());
    let again = reread(&write(&proj, computed, &WriteOptions::default()).xml);
    assert_eq!(again.recorded, first.recorded);
}

#[test]
fn r122_gaps_resource_calendars_and_leveling_delay() {
    let mut proj = new_project("2026-10-19T08:00");
    proj.tasks.push(task(&proj, 1, 2)).unwrap();
    let mut r = Resource::new(ResourceUid::new(1), "R".to_owned());
    r.calendar = Some(CalendarUid::new(1));
    proj.resources.push(r).unwrap();
    proj.assignments
        .push(Assignment::new(
            AssignmentUid::new(1),
            TaskUid::new(1),
            Some(ResourceUid::new(1)),
        ))
        .unwrap();
    assert_eq!(schedule_gaps(&proj), [ScheduleGap::ResourceCalendars]);
    // Ignorer le calendrier des ressources : plus de manque.
    proj.tasks
        .get_mut(TaskUid::new(1))
        .unwrap()
        .ignore_resource_calendar = true;
    assert_eq!(schedule_gaps(&proj), []);
    proj.tasks.get_mut(TaskUid::new(1)).unwrap().extras = Extras(vec![ExtraEntry {
        origin: "mspdi".to_owned(),
        key: "LevelingDelay".to_owned(),
        payload: "<LevelingDelay>4800</LevelingDelay>".to_owned(),
    }]);
    assert_eq!(schedule_gaps(&proj), [ScheduleGap::LevelingDelay]);
}

#[test]
fn r122_schedule_of_another_project_is_refused() {
    let mut proj = new_project("2026-10-19T08:00");
    proj.tasks.push(task(&proj, 1, 2)).unwrap();
    let other = schedule(&new_project("2026-10-19T08:00"));
    let out = write(&proj, Computed::Schedule(&other), &WriteOptions::default());
    assert_eq!(out.diagnostics[0].code, WriteCode::ScheduleMismatch);
    assert_eq!(out.diagnostics[0].code.code(), "mspdi.write.schedule_mismatch");
    assert!(!text(&out).contains("<EarlyStart>"));
}

#[test]
fn r122_schedule_from_finish_writes_the_computed_start() {
    let mut proj = new_project("2026-10-19T08:00");
    proj.anchor = ScheduleAnchor::FromFinish(moment("2026-10-23T17:00"));
    proj.tasks.push(task(&proj, 1, 1)).unwrap();
    let sched = schedule(&proj);
    let xml = compact(&text(&write(
        &proj,
        Computed::Schedule(&sched),
        &WriteOptions::default(),
    )));
    assert!(xml.contains("<ScheduleFromStart>0</ScheduleFromStart>"));
    assert!(xml.contains("<FinishDate>2026-10-23T17:00:00</FinishDate>"));
    // `StartDate` = début calculé du moteur (`Schedule::project_start`).
    assert!(xml.contains(&format!("<StartDate>{}:00</StartDate>", sched.project_start)));
}
