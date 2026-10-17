//! Manifeste du corpus, format 2 (`tests/corpus/manifest.toml`).
//!
//! Toutes les clés d'une entrée sont obligatoires (`mirrors` et `note`
//! peuvent être vides) ; une clé inconnue est refusée. Le manifeste est figé :
//! on ajoute des lignes ; un retrait passe par un ADR et garde le nom.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Version du format lue par ce harnais.
pub const FORMAT: u32 = 2;

/// Manifeste.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Version du format.
    pub format: u32,
    /// Fichiers, dans l'ordre du manifeste.
    #[serde(default)]
    pub file: Vec<Entry>,
}

/// Une ligne du manifeste (voir `tests/corpus/README.md`).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Nom dans `tests/corpus/files/`, sans séparateur.
    pub name: String,
    /// Provenance lisible.
    pub source: String,
    /// Adresse de téléchargement (https).
    pub url: String,
    /// Adresses de repli (https), dans l'ordre ; peut être vide.
    pub mirrors: Vec<String>,
    /// Licence SPDX, ou `none` (tous droits réservés).
    pub license: String,
    /// Droits et usage : copie privée, jamais commitée ni publiée.
    pub rights: String,
    /// Date de récupération, `AAAA-MM-JJ`.
    pub date: String,
    /// Taille en octets (> 0).
    pub size: u64,
    /// SHA-256, 64 hexadécimaux minuscules.
    pub sha256: String,
    /// Producteur présumé du fichier.
    pub producer: Producer,
    /// `SaveVersion` attendu.
    pub save_version: i64,
    /// Remarque libre ; peut être vide.
    pub note: String,
    /// Plancher : nombre minimal de cas identiques sur ce fichier.
    pub floor: u32,
}

/// Producteur présumé d'un fichier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Producer {
    /// Enregistré par MS Project (signature complète observée).
    Msproject,
    /// Probablement enregistré par MS Project.
    MsprojectProbable,
    /// Inconnu.
    Unknown,
}

impl Producer {
    /// Écriture du manifeste.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Msproject => "msproject",
            Self::MsprojectProbable => "msproject-probable",
            Self::Unknown => "unknown",
        }
    }
}

/// Lit le manifeste.
///
/// # Errors
///
/// Fichier illisible ou TOML non conforme au format (clé manquante ou
/// inconnue, type faux).
pub fn load(path: &Path) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{} : {e}", path.display()))?;
    parse(&text)
}

/// Analyse le texte du manifeste.
///
/// # Errors
///
/// TOML non conforme au format.
pub fn parse(text: &str) -> Result<Manifest, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

/// Problèmes de contenu du manifeste (vide si valide).
#[must_use]
pub fn validate(manifest: &Manifest) -> Vec<String> {
    let mut problems = Vec::new();
    if manifest.format != FORMAT {
        problems.push(format!("format {} inconnu (attendu {FORMAT})", manifest.format));
    }
    let mut names = BTreeSet::new();
    let mut hashes = BTreeSet::new();
    for e in &manifest.file {
        let n = &e.name;
        if !is_file_name(n) {
            problems.push(format!("{n:?} : nom invalide"));
        }
        if !names.insert(n.as_str()) {
            problems.push(format!("{n} : nom en double"));
        }
        if e.source.trim().is_empty() {
            problems.push(format!("{n} : source vide"));
        }
        if !e.url.starts_with("https://") {
            problems.push(format!("{n} : url non https"));
        }
        if e.mirrors.iter().any(|m| !m.starts_with("https://")) {
            problems.push(format!("{n} : miroir non https"));
        }
        if !is_license(&e.license) {
            problems.push(format!("{n} : licence attendue en SPDX ou « none »"));
        }
        if e.rights.trim().is_empty() {
            problems.push(format!("{n} : droits vides"));
        }
        if !is_iso_date(&e.date) {
            problems.push(format!("{n} : date attendue au format AAAA-MM-JJ"));
        }
        if e.size == 0 {
            problems.push(format!("{n} : taille nulle"));
        }
        if !is_sha256(&e.sha256) {
            problems.push(format!("{n} : sha256 attendu en 64 hexadécimaux minuscules"));
        }
        if !hashes.insert(e.sha256.as_str()) {
            problems.push(format!("{n} : fichier en double (même empreinte)"));
        }
    }
    problems
}

/// Somme des planchers du manifeste.
#[must_use]
pub fn floor_sum(manifest: &Manifest) -> u64 {
    manifest
        .file
        .iter()
        .fold(0_u64, |acc, e| acc.saturating_add(u64::from(e.floor)))
}

/// Nom de fichier simple : non vide, sans séparateur, ni caché ni `..`.
#[must_use]
pub fn is_file_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\'])
}

/// Licence : `none`, ou expression SPDX simple (identifiants séparés par
/// `OR`, `AND`, `WITH`).
#[must_use]
pub fn is_license(value: &str) -> bool {
    if value == "none" {
        return true;
    }
    let mut words = value.split(' ');
    let id = |w: &str| {
        !w.is_empty()
            && w.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-.+".contains(&b))
    };
    let Some(first) = words.next() else {
        return false;
    };
    if !id(first) {
        return false;
    }
    // Alternance opérateur / identifiant.
    loop {
        match (words.next(), words.next()) {
            (None, _) => return true,
            (Some("OR" | "AND" | "WITH"), Some(w)) if id(w) => {}
            _ => return false,
        }
    }
}

/// Date `AAAA-MM-JJ` (forme seulement).
#[must_use]
pub fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

/// 64 hexadécimaux minuscules.
#[must_use]
pub fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// SHA-256 en hexadécimal minuscule.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in &digest {
        // L'écriture dans une `String` n'échoue pas.
        let _ = write!(out, "{byte:02x}");
    }
    out
}
