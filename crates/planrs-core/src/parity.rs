//! Plancher de parité avec MS Project.
//!
//! La parité se mesure en **nombre de cas** reproduits à l'identique sur un
//! manifeste figé (`tests/corpus/manifest.toml`, puis `tests/oracle/`), et non
//! en pourcentage d'un corpus mouvant : ajouter un fichier difficile ouvre une
//! nouvelle ligne sans faire baisser la parité mesurée (ADR-0004, écart au
//! brief qui parlait de pourcentage).
//!
//! Le harnais du corpus (J2) échouera si le nombre de cas reproduits passe
//! sous [`FLOOR`]. Quand la parité monte, on relève le plancher dans le même
//! commit ; on ne le baisse jamais.

/// Nombre minimal de cas (corpus public + oracle) que le moteur doit
/// reproduire à l'identique. Zéro tant que le harnais du corpus n'existe pas.
pub const FLOOR: u32 = 0;
