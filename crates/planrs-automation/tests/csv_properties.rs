//! P28 (robustesse) — Import CSV : octets arbitraires, mutations d'un CSV
//! valide et CSV synthétiques (guillemets, séparateurs, encodages) ne font
//! jamais paniquer l'import (règle 10) ; le résultat est soit un projet
//! valide, soit une erreur du fichier entier.
//!
//! Nombre de cas fixé ici : la variable `PROPTEST_CASES` de la CI (10 000)
//! vise les propriétés du moteur.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

use planrs_automation::csv::{CsvImportOptions, import};
use planrs_core::{Moment, ScheduleAnchor};
use proptest::prelude::*;

fn options() -> CsvImportOptions {
    CsvImportOptions::new(
        ScheduleAnchor::FromStart("2026-10-19T08:00".parse::<Moment>().unwrap()),
        "2026-10-17T08:00".parse::<Moment>().unwrap(),
    )
}

const SAMPLE: &str = "N°;N° unique;Nom;Durée;Début;Fin;Prédécesseurs;Niveau hiérarchique;Jalon;Notes;% achevé\r\n\
1;1;Phase;10j;19/10/2026 08:00;23/10/2026 17:00;;1;Non;;0%\r\n\
2;2;\"A;b\";3j;;;;2;Non;\"l1\r\nl2\";50%\r\n\
3;3;B;2,5j?;;;\"2FD+1j;1DD\";2;Non;'=x;\r\n\
4;4;Fin;;;;3;1;Oui;;\r\n";

/// Import sans panique ; un projet importé est cohérent avec ses UID.
fn check(bytes: &[u8]) {
    if let Ok(i) = import(bytes, &options()) {
        for t in &i.project.tasks {
            assert!(t.uid().get() > 0);
            for l in &t.predecessors {
                assert!(i.project.tasks.contains(l.predecessor));
                assert_ne!(l.predecessor, t.uid());
            }
        }
        let _ = planrs_core::schedule(&i.project);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_024))]

    #[test]
    fn p28_arbitrary_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        check(&bytes);
    }

    #[test]
    fn p28_mutated_sample_never_panics(
        edits in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>(), 0_u8..3), 1..8),
    ) {
        let mut bytes = SAMPLE.as_bytes().to_vec();
        for (at, byte, kind) in edits {
            if bytes.is_empty() {
                break;
            }
            let i = at.index(bytes.len());
            match kind {
                0 => bytes[i] = byte,
                1 => bytes.insert(i, byte),
                _ => {
                    bytes.remove(i);
                }
            }
        }
        check(&bytes);
    }

    #[test]
    fn p28_synthetic_csv_never_panics(
        cells in prop::collection::vec("[A-Za-z0-9 ;,\t\"'=+\\-@?%:/.éÉ\r\n]{0,12}", 1..40),
        sep in prop_oneof![Just(';'), Just(','), Just('\t')],
        utf16 in any::<bool>(),
    ) {
        let mut text = format!("Nom{sep}Durée{sep}Prédécesseurs{sep}Niveau hiérarchique{sep}N° unique\n");
        for (k, c) in cells.iter().enumerate() {
            text.push_str(c);
            text.push(if k % 5 == 4 { '\n' } else { sep });
        }
        if utf16 {
            let mut bytes = vec![0xFF, 0xFE];
            for u in text.encode_utf16() {
                bytes.extend(u.to_le_bytes());
            }
            check(&bytes);
        } else {
            check(text.as_bytes());
        }
    }
}

#[test]
fn p28_sample_imports() {
    let i = import(SAMPLE.as_bytes(), &options()).unwrap();
    assert_eq!(i.project.tasks.len(), 4);
    // Durée de la récapitulative ignorée (ligne 2) ; aucun autre problème que
    // des écarts de dates (information).
    let codes: Vec<_> = i.diagnostics.iter().map(|d| (d.line, d.code.code())).collect();
    assert_eq!(codes[0], (2, "csv.summary_duration_ignored"));
    assert!(
        codes[1..].iter().all(|(_, c)| *c == "csv.date_mismatch"),
        "{codes:?}"
    );
}
