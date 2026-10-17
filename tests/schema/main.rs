//! Conformité au schéma MSPDI `mspdi_pj15.xsd` (ADR-0010, D18).
//!
//! Le schéma est un fichier public de Microsoft, jamais commité : `cargo
//! xtask schema fetch` l'extrait du SDK de Project 2013 dans
//! `tests/schema/mspdi_pj15.xsd` (ignoré par git, sha256 vérifié). Absent :
//! les tests passent avec un message, sauf sous `PLANRS_SCHEMA=required`
//! (job CI `schema`), où ils échouent.
//!
//! - P27 : la table `planrs_mspdi::schema` égale le XSD analysé (noms et
//!   ordre des séquences, `minOccurs`, `maxOccurs`, `maxLength`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

mod xsd;

use planrs_mspdi::schema::{Parent, Profile, sequence};

/// Empreinte SHA-256 du XSD (révision 2012-07-18).
const XSD_SHA256: &str = "1c208a9732d6ba16f677a9f2fb9d5a629f0c200346b1851998a44765c502eb85";

/// Texte du XSD, empreinte vérifiée ; `None` s'il est absent et que
/// `PLANRS_SCHEMA` ne vaut pas `required`.
fn schema_text() -> Option<String> {
    let path = planrs_tests::tests_path("schema/mspdi_pj15.xsd");
    let required = std::env::var("PLANRS_SCHEMA").is_ok_and(|v| v == "required");
    let Ok(bytes) = std::fs::read(&path) else {
        assert!(
            !required,
            "PLANRS_SCHEMA=required et {} absent : lancer `cargo xtask schema fetch`",
            path.display()
        );
        eprintln!(
            "schéma absent ({}) : test sauté ; `cargo xtask schema fetch` pour le lancer",
            path.display()
        );
        return None;
    };
    assert_eq!(
        planrs_tests::corpus::manifest::sha256_hex(&bytes),
        XSD_SHA256,
        "{} : empreinte inattendue",
        path.display()
    );
    let text = String::from_utf8(bytes).expect("XSD en UTF-8");
    Some(text.trim_start_matches('\u{feff}').to_owned())
}

/// P27 : la table du schéma égale le XSD analysé.
#[test]
fn p27_schema_table_matches_the_xsd() {
    let Some(text) = schema_text() else {
        return;
    };
    let tree = xsd::parse(&text);
    let project = xsd::project_element(&tree).expect("élément Project");
    for parent in Parent::ALL {
        let element =
            xsd::descend(project, parent.path()).unwrap_or_else(|| panic!("{parent:?} absent du XSD"));
        let expected = xsd::sequence(element);
        let table = sequence(parent).strict;
        let got: Vec<xsd::Item> = table
            .iter()
            .map(|s| xsd::Item {
                name: s.name.to_owned(),
                min: s.min,
                many: s.many,
                max_len: s.max_len,
            })
            .collect();
        assert_eq!(got, expected, "{parent:?} : table différente du XSD");
    }
}

/// P27 : le profil observé est une permutation de pj15 plus `GUID`
/// (`WorkWeek` excepté, voir `Parent::WorkWeek`).
#[test]
fn p27_observed_profile_is_a_permutation() {
    for parent in Parent::ALL {
        if parent == Parent::WorkWeek {
            continue;
        }
        let seq = sequence(parent);
        let mut strict: Vec<&str> = seq.names(Profile::Strict).collect();
        let mut observed: Vec<&str> = seq.names(Profile::Observed).filter(|n| *n != "GUID").collect();
        strict.sort_unstable();
        observed.sort_unstable();
        assert_eq!(strict, observed, "{parent:?}");
    }
}
