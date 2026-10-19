//! Plancher de parité avec MS Project.
//!
//! La parité se mesure en **nombre de cas** reproduits à l'identique sur un
//! manifeste figé (`tests/corpus/manifest.toml`, puis `tests/oracle/`), et non
//! en pourcentage d'un corpus mouvant : ajouter un fichier difficile ouvre une
//! nouvelle ligne sans faire baisser la parité mesurée (ADR-0004, écart au
//! brief qui parlait de pourcentage).
//!
//! Le harnais du corpus (`tests/corpus`, ADR-0009) échoue si un fichier
//! présent passe sous son `floor`, ou si la parité globale passe sous
//! [`CORPUS_FLOOR`] (planchers des fichiers absents déduits : un absent ne
//! compte ni pour ni contre). Quand la parité monte, on relève les planchers
//! du manifeste et [`CORPUS_FLOOR`] dans le même commit ; on ne les baisse
//! jamais, sauf ADR (fichier retiré, valeur enregistrée reconnue fausse).

/// Somme des `floor` du manifeste du corpus (vérifiée par le harnais).
/// Mesuré le 2026-10-19 : A1 51, A2 18, A4 42, A6 4, A7 8 (159 cas ; J2 : 91,
/// A4 22, A7 0 ; J3 : contraintes FNET de A7, R-090, R-094 ; calendrier du
/// projet dérivé de A4, R-113 ; J5 : placement par affectation sur A6).
pub const CORPUS_FLOOR: u32 = 123;

/// Cas de l'oracle MS Project (`tests/oracle`), zéro sans licence.
pub const ORACLE_FLOOR: u32 = 0;

/// Plancher total en nombre de cas identiques (ADR-0004).
pub const FLOOR: u32 = match CORPUS_FLOOR.checked_add(ORACLE_FLOOR) {
    Some(v) => v,
    None => u32::MAX,
};
