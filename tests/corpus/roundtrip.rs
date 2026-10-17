//! Aller-retour MSPDI sur le corpus (P25, R-133 ; ADR-0010 § 8.2).
//!
//! Pour chaque fichier présent : lecture → écriture (profil observé) →
//! relecture. Avec les valeurs relues (`Computed::Recorded`), le projet relu
//! doit égaler `normalize(projet lu)` et le planning enregistré relu égaler
//! celui du fichier ; avec les valeurs du moteur (`Computed::Schedule`), le
//! projet relu doit égaler `normalize(projet lu)` ; l'ordre réécrit des
//! enfants de chaque entité doit suivre l'ordre source. Verdict en échec si
//! un fichier `msproject` n'est pas identique avec ses valeurs relues.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use planrs_mspdi::write::{Computed, WriteOptions, write};
use planrs_mspdi::{MspdiDocument, ReadOptions, normalize};
use serde::Serialize;

use crate::corpus::manifest::{Entry, Producer};
use crate::mspdi::{entity_children, order_agrees};

/// Résultat de l'aller-retour d'un fichier.
#[derive(Clone, Debug, Serialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "verdicts indépendants publiés dans le rapport, pas un état"
)]
pub struct RoundTrip {
    /// Nom du fichier.
    pub file: String,
    /// Fichier enregistré par MS Project (`producer = msproject`).
    pub msproject: bool,
    /// Projet relu identique à `normalize(projet)`, valeurs relues écrites.
    pub project_identical_recorded: bool,
    /// Planning enregistré relu identique, valeurs relues écrites.
    pub recorded_identical: bool,
    /// Projet relu identique à `normalize(projet)`, valeurs du moteur
    /// écrites.
    pub project_identical_schedule: bool,
    /// Pour chaque entité, éléments réécrits dans l'ordre source.
    pub order_preserved: bool,
    /// `Extras` par entité (`project`, `calendar`, `task`, `resource`,
    /// `assignment`).
    pub extras: BTreeMap<&'static str, usize>,
    /// Calculés non recalculés par planrs, écartés à la lecture, par nom.
    pub dropped_computed: BTreeMap<String, u64>,
    /// Enfants perdus (jour, exception, semaine, plage, lien).
    pub not_preserved: u64,
    /// Diagnostics d'écriture par code (valeurs relues).
    pub write_diagnostics: BTreeMap<String, u32>,
}

impl RoundTrip {
    /// Aller-retour exact avec les valeurs relues.
    #[must_use]
    pub const fn identical(&self) -> bool {
        self.project_identical_recorded && self.recorded_identical
    }
}

/// Aller-retour d'un fichier lu (`doc`) dont `source` est le texte.
///
/// # Panics
///
/// Si un document écrit ne se relit pas (jamais attendu : l'écriture est
/// infaillible et produit du XML bien formé).
#[must_use]
pub fn round_trip(entry: &Entry, doc: &MspdiDocument, source: &str) -> RoundTrip {
    let options = WriteOptions::default();
    let read_options = ReadOptions::new(crate::moment(crate::corpus::CURRENT_DATE));
    let reread = |xml: &[u8]| match planrs_mspdi::read(xml, &read_options) {
        Ok(d) => d,
        Err(e) => panic!("{} : document écrit illisible : {e}", entry.name),
    };
    let expected = normalize(&doc.project, &options);
    let written = write(&doc.project, Computed::Recorded(&doc.recorded), &options);
    let back = reread(&written.xml);
    let schedule = planrs_core::schedule(&doc.project);
    let with_schedule = reread(&write(&doc.project, Computed::Schedule(&schedule), &options).xml);
    let text = String::from_utf8_lossy(&written.xml);
    let before = entity_children(source.trim_start_matches('\u{feff}'));
    let after = entity_children(&text);
    let of = |list: &[(String, Vec<String>)], wanted: &str| -> Vec<Vec<String>> {
        list.iter()
            .filter(|(p, _)| p == wanted)
            .map(|(_, c)| c.clone())
            .collect()
    };
    let order_preserved = [
        "Project",
        "Project/Calendars/Calendar",
        "Project/Tasks/Task",
        "Project/Resources/Resource",
        "Project/Assignments/Assignment",
    ]
    .iter()
    .all(|e| {
        of(&after, e)
            .iter()
            .zip(of(&before, e).iter())
            .all(|(w, s)| order_agrees(w, s))
    });
    let p = &doc.project;
    let mut extras = BTreeMap::new();
    extras.insert("project", p.extras.0.len());
    extras.insert("calendar", p.calendars.iter().map(|c| c.extras.0.len()).sum());
    extras.insert(
        "task",
        p.summary_task
            .iter()
            .chain(p.tasks.iter())
            .map(|t| t.extras.0.len())
            .sum(),
    );
    extras.insert("resource", p.resources.iter().map(|r| r.extras.0.len()).sum());
    extras.insert("assignment", p.assignments.iter().map(|a| a.extras.0.len()).sum());
    let mut write_diagnostics: BTreeMap<String, u32> = BTreeMap::new();
    for d in &written.diagnostics {
        let n = write_diagnostics.entry(d.code.code().to_owned()).or_insert(0);
        *n = n.saturating_add(1);
    }
    RoundTrip {
        file: entry.name.clone(),
        msproject: entry.producer == Producer::Msproject,
        project_identical_recorded: back.project == expected,
        recorded_identical: back.recorded == doc.recorded,
        project_identical_schedule: with_schedule.project == expected,
        order_preserved,
        extras,
        dropped_computed: doc.source.dropped_computed.clone(),
        not_preserved: doc.source.not_preserved,
        write_diagnostics,
    }
}

/// Échecs du verdict : fichiers `msproject` non identiques avec leurs
/// valeurs relues, ou réordonnés.
#[must_use]
pub fn failures(rows: &[RoundTrip]) -> Vec<String> {
    rows.iter()
        .filter(|r| r.msproject && !(r.identical() && r.project_identical_schedule && r.order_preserved))
        .map(|r| format!("{} : aller-retour MSPDI non identique", r.file))
        .collect()
}

fn yes(b: bool) -> &'static str {
    if b { "oui" } else { "**non**" }
}

/// Section Markdown « Aller-retour MSPDI ».
#[must_use]
pub fn markdown(rows: &[RoundTrip]) -> String {
    let mut s = String::new();
    // L'écriture dans une `String` n'échoue pas : résultats ignorés.
    let _ = writeln!(s, "\n## Aller-retour MSPDI\n");
    let _ = writeln!(
        s,
        "Lecture → écriture (profil observé) → relecture (P25, R-133). « Projet » : projet relu = `normalize(projet lu)` ; « planning » : planning enregistré relu identique ; « ordre » : éléments réécrits dans l'ordre source. Relu identique par planrs n'est pas une vérification par MS Project.\n"
    );
    let _ = writeln!(
        s,
        "| Fichier | Projet (valeurs relues) | Planning enregistré | Projet (valeurs du moteur) | Ordre | Extras (projet, calendriers, tâches, ressources, affectations) | Calculés écartés | Perdus | Diagnostics d'écriture |"
    );
    let _ = writeln!(s, "|---|---|---|---|---|---|---|---|---|");
    for r in rows {
        let ex = |k: &str| r.extras.get(k).copied().unwrap_or(0);
        let dropped: Vec<String> = r
            .dropped_computed
            .iter()
            .map(|(k, v)| format!("`{k}` {v}"))
            .collect();
        let diags: Vec<String> = r
            .write_diagnostics
            .iter()
            .map(|(k, v)| format!("`{k}` {v}"))
            .collect();
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} | {} | {}, {}, {}, {}, {} | {} | {} | {} |",
            r.file,
            yes(r.project_identical_recorded),
            yes(r.recorded_identical),
            yes(r.project_identical_schedule),
            yes(r.order_preserved),
            ex("project"),
            ex("calendar"),
            ex("task"),
            ex("resource"),
            ex("assignment"),
            if dropped.is_empty() {
                "–".to_owned()
            } else {
                dropped.join(", ")
            },
            r.not_preserved,
            if diags.is_empty() {
                "–".to_owned()
            } else {
                diags.join(", ")
            },
        );
    }
    let _ = writeln!(
        s,
        "\nPerdu à l'aller-retour (doc utilisateur) : calculés non recalculés par planrs (recalculés par MS Project) ; ancienne forme des exceptions (régénérée par MS Project) ; `Occurrences` d'une exception non saisie par occurrences ; enfants inconnus de `WeekDay`, `Exception`, `WorkWeek` et d'un `PredecessorLink` modélisé ; valeurs illisibles (remplacées par leur défaut, signalées à la lecture) ; liens invalides ; textes au-delà du `maxLength` ; pourcentages non entiers ; `Period` réécrite en quotidienne (mêmes jours)."
    );
    s
}
