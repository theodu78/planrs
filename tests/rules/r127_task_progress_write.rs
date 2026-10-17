//! R-127 — Travail et avancement d'une tâche à l'écriture.

use planrs_core::{Minutes, Percent, Task, TaskUid, Work};
use planrs_mspdi::write::WriteCode;
use planrs_tests::mspdi::{compact, new_project, reread, text, written};

#[test]
fn r127_percentages_are_whole_and_work_is_iso() {
    let mut proj = new_project("2026-10-19T08:00");
    let opts = proj.options;
    for (uid, pct) in [(1, 3_333), (2, 6_650), (3, 10_000)] {
        let mut t = Task::new(TaskUid::new(uid), format!("T{uid}"), &opts);
        t.progress.percent_complete = Percent::new(pct).unwrap();
        t.work = Work::new(Minutes::new(960)).unwrap();
        t.progress.actual_work = Work::new(Minutes::new(90)).unwrap();
        t.progress.remaining_work = Work::new(Minutes::new(870)).unwrap();
        t.progress.actual_duration = Minutes::new(240);
        t.progress.remaining_duration = Minutes::new(240);
        proj.tasks.push(t).unwrap();
    }
    let out = written(&proj);
    let xml = compact(&text(&out));
    // 3 333 → 33 (arrondi signalé) ; 6 650 → 67 (moitié loin de zéro) ;
    // 10 000 → 100.
    for v in ["33", "67", "100"] {
        assert!(
            xml.contains(&format!("<PercentComplete>{v}</PercentComplete>")),
            "{v}"
        );
    }
    let rounded: Vec<&str> = out
        .diagnostics
        .iter()
        .filter(|d| d.code == WriteCode::Rounded)
        .map(|d| d.element.as_str())
        .collect();
    assert_eq!(
        rounded,
        ["Tasks/Task/PercentComplete", "Tasks/Task/PercentComplete"]
    );
    assert_eq!(out.diagnostics[0].code.code(), "mspdi.write.rounded");
    assert!(xml.contains("<Work>PT16H0M0S</Work>"));
    assert!(xml.contains("<ActualWork>PT1H30M0S</ActualWork>"));
    assert!(xml.contains("<RemainingWork>PT14H30M0S</RemainingWork>"));
    assert!(xml.contains("<ActualDuration>PT4H0M0S</ActualDuration>"));
    let back = reread(&out.xml).project;
    let t = back.tasks.get(TaskUid::new(1)).unwrap();
    assert_eq!(t.progress.percent_complete, Percent::new(3_300).unwrap());
    assert_eq!(t.progress.actual_work.minutes(), Minutes::new(90));
    assert_eq!(t.work.minutes(), Minutes::new(960));
}
