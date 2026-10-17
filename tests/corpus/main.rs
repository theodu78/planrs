//! Harnais du corpus public (coquille, ADR-0004 ; harnais complet en J2).
//!
//! Le corpus est décrit par `tests/corpus/manifest.toml` (versionné) ; les
//! fichiers eux-mêmes vivent dans `tests/corpus/files/`, ignoré par git
//! (droits d'auteur). En J2, le harnais effacera les valeurs calculées de
//! chaque fichier, laissera le moteur recalculer, comparera aux dates de MS
//! Project (tâches manuelles exclues) et vérifiera le plancher
//! `planrs_core::parity::FLOOR`.
//!
//! État J1 : seul le format du manifeste est vérifié.

// Tests : les interdits de la règle 10 ne s'appliquent pas (un échec doit paniquer).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;

use serde::Deserialize;

/// Manifeste figé du corpus.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    /// Version du format du manifeste.
    format: u32,
    /// Fichiers du corpus, chacun identifié par son empreinte.
    #[serde(default)]
    file: Vec<Entry>,
}

/// Une ligne du manifeste (voir `tests/corpus/README.md`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    /// Nom du fichier dans `tests/corpus/files/`.
    name: String,
    /// Provenance lisible (éditeur, support de formation…).
    source: String,
    /// Adresse de téléchargement.
    url: String,
    /// Date de récupération, `AAAA-MM-JJ`.
    date: String,
    /// Empreinte SHA-256 du fichier, 64 caractères hexadécimaux minuscules.
    sha256: String,
    /// Remarque libre (conversion MPXJ, tâches manuelles, particularités).
    #[serde(default)]
    #[allow(dead_code, reason = "lu par le harnais complet en J2")]
    note: String,
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[test]
fn manifest_is_well_formed() {
    let path = planrs_tests::tests_path("corpus/manifest.toml");
    let text = std::fs::read_to_string(&path).expect("lecture du manifeste");
    let manifest: Manifest = toml::from_str(&text).expect("manifeste TOML valide");
    assert_eq!(manifest.format, 1, "format de manifeste inconnu");

    let mut names = BTreeSet::new();
    let mut hashes = BTreeSet::new();
    for entry in &manifest.file {
        assert!(
            !entry.name.is_empty() && !entry.name.contains('/'),
            "nom invalide : {:?}",
            entry.name
        );
        assert!(!entry.source.is_empty(), "{} : source manquante", entry.name);
        assert!(
            entry.url.starts_with("https://") || entry.url.starts_with("http://"),
            "{} : url invalide",
            entry.name
        );
        assert!(
            is_iso_date(&entry.date),
            "{} : date attendue au format AAAA-MM-JJ",
            entry.name
        );
        assert!(
            is_sha256(&entry.sha256),
            "{} : sha256 attendu en 64 hexadécimaux minuscules",
            entry.name
        );
        assert!(
            names.insert(entry.name.as_str()),
            "{} : nom en double",
            entry.name
        );
        assert!(
            hashes.insert(entry.sha256.as_str()),
            "{} : fichier en double (même empreinte)",
            entry.name
        );
    }
}

#[test]
fn format_helpers() {
    assert!(is_iso_date("2026-10-17"));
    assert!(!is_iso_date("17/10/2026"));
    assert!(is_sha256(&"a".repeat(64)));
    assert!(!is_sha256(&"A".repeat(64)));
}
