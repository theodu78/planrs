//! `cargo xtask text-ids` : chaque texte de l'interface est traduit et
//! utilisé (ADR-0016 D14).
//!
//! Lit la liste `text_ids! { Variante => "clé", … }` de
//! `crates/planrs-ui-egui/src/i18n.rs` et échoue si :
//! - une clé manque dans `locales/fr.ftl` ou `locales/en.ftl` (ou si un
//!   message de ces fichiers n'est pas dans la liste) ;
//! - une variante n'est écrite `TextId::Variante` nulle part dans le code
//!   livré de l'interface (`crates/planrs-ui-egui/src`, `apps/planrs/src`,
//!   `apps/planrs-web/src`), hors de la liste elle-même.
//!
//! Exception : les clés `error-*` sont lues par leur code de refus
//! (`error_text_id`, clé construite à l'exécution) ; leur couverture est
//! vérifiée par les tests de `i18n.rs`.
//!
//! Limites : recherche textuelle (un `TextId::X` dans un commentaire ou un
//! module de test du code livré compte comme un usage) ; un alias
//! (`use TextId as T`) n'est pas suivi : le code du dépôt n'en a pas.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Liste des identifiants, relative à la racine du dépôt.
const I18N: &str = "crates/planrs-ui-egui/src/i18n.rs";
/// Fichiers de traduction, relatifs à la racine.
const LOCALES: [&str; 2] = [
    "crates/planrs-ui-egui/locales/fr.ftl",
    "crates/planrs-ui-egui/locales/en.ftl",
];
/// Code livré parcouru, relatif à la racine.
const SOURCE_DIRS: [&str; 3] = [
    "crates/planrs-ui-egui/src",
    "apps/planrs/src",
    "apps/planrs-web/src",
];
/// Préfixe des clés lues par leur code de refus.
const DYNAMIC_PREFIX: &str = "error-";

pub fn run() -> ExitCode {
    match check(&repo_root()) {
        Ok(count) => {
            println!(
                "xtask text-ids : OK ({count} textes, traduits en français et en anglais, tous utilisés)"
            );
            ExitCode::SUCCESS
        }
        Err(problems) => {
            for p in &problems {
                eprintln!("xtask text-ids : {p}");
            }
            eprintln!("xtask text-ids : {} problème(s)", problems.len());
            ExitCode::FAILURE
        }
    }
}

/// Vérifie le dépôt de racine `root` ; rend le nombre de textes.
fn check(root: &Path) -> Result<usize, Vec<String>> {
    let i18n =
        std::fs::read_to_string(root.join(I18N)).map_err(|e| vec![format!("{I18N} illisible : {e}")])?;
    let (ids, outside) =
        parse_text_ids(&i18n).ok_or_else(|| vec![format!("{I18N} : liste `text_ids!` introuvable")])?;
    let mut problems = Vec::new();
    let keys: BTreeSet<&str> = ids.iter().map(|(_, k)| k.as_str()).collect();
    if keys.len() != ids.len() {
        problems.push("clé en double dans `text_ids!`".to_owned());
    }
    for locale in LOCALES {
        let text = match std::fs::read_to_string(root.join(locale)) {
            Ok(t) => t,
            Err(e) => {
                problems.push(format!("{locale} illisible : {e}"));
                continue;
            }
        };
        let declared = ftl_keys(&text);
        for key in keys.difference(&declared) {
            problems.push(format!("{locale} : clé « {key} » manquante"));
        }
        for key in declared.difference(&keys) {
            problems.push(format!("{locale} : message « {key} » absent de `text_ids!`"));
        }
    }
    let mut used = used_variants(&outside);
    for dir in SOURCE_DIRS {
        let mut files = Vec::new();
        if let Err(e) = collect_rs(&root.join(dir), &mut files) {
            problems.push(format!("{dir} : {e}"));
        }
        for file in files {
            if file.ends_with("i18n.rs") && file.starts_with(root.join(SOURCE_DIRS[0])) {
                continue;
            }
            match std::fs::read_to_string(&file) {
                Ok(text) => used.extend(used_variants(&text)),
                Err(e) => problems.push(format!("{} illisible : {e}", file.display())),
            }
        }
    }
    for (variant, key) in &ids {
        if !key.starts_with(DYNAMIC_PREFIX) && !used.contains(variant) {
            problems.push(format!("`TextId::{variant}` (« {key} ») utilisé nulle part"));
        }
    }
    if problems.is_empty() {
        Ok(ids.len())
    } else {
        Err(problems)
    }
}

/// Paires (variante, clé) de l'invocation `text_ids! { … }` (ligne qui
/// commence par `text_ids! {`, jusqu'à la première `}` en colonne 0), et le
/// texte du fichier hors de cette liste.
fn parse_text_ids(src: &str) -> Option<(Vec<(String, String)>, String)> {
    let mut ids = Vec::new();
    let mut outside = String::new();
    let mut inside = false;
    let mut found = false;
    for line in src.lines() {
        if !inside && line.starts_with("text_ids! {") {
            inside = true;
            found = true;
            continue;
        }
        if inside {
            if line.starts_with('}') {
                inside = false;
                continue;
            }
            if let Some((variant, rest)) = line.trim().split_once("=>") {
                let key = rest.trim().trim_end_matches(',').trim().trim_matches('"');
                ids.push((variant.trim().to_owned(), key.to_owned()));
            }
            continue;
        }
        outside.push_str(line);
        outside.push('\n');
    }
    (found && !inside).then_some((ids, outside))
}

/// Clés des messages d'un fichier fluent (ni termes `-x`, ni attributs,
/// ni commentaires, ni lignes de suite indentées).
fn ftl_keys(text: &str) -> BTreeSet<&str> {
    text.lines()
        .filter_map(|line| {
            let (key, _) = line.split_once(" =")?;
            let first = key.chars().next()?;
            (first.is_ascii_alphabetic()
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .then_some(key)
        })
        .collect()
}

/// Variantes écrites `TextId::Variante` dans `text`.
fn used_variants(text: &str) -> BTreeSet<String> {
    const MARK: &str = "TextId::";
    let mut out = BTreeSet::new();
    let mut rest = text;
    while let Some(i) = rest.find(MARK) {
        rest = rest.get(i.saturating_add(MARK.len())..).unwrap_or_default();
        let ident: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if ident.starts_with(|c: char| c.is_ascii_uppercase()) {
            out.insert(ident);
        }
    }
    out
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    let mut paths: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_rs(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map_or_else(|| manifest.to_path_buf(), Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]

    use super::*;

    /// Le dépôt passe le contrôle (chaque texte traduit et utilisé).
    #[test]
    fn text_ids_all_used() {
        let result = check(&repo_root());
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn text_ids_parse_list_keys_and_uses() {
        let src = "macro_rules! text_ids { () => {} }\ntext_ids! {\n    // Section.\n    A => \"a-key\",\n    Bb => \"bb\",\n}\nfn f() { TextId::A; }\n";
        let (ids, outside) = parse_text_ids(src).unwrap();
        assert_eq!(
            ids,
            [
                ("A".to_owned(), "a-key".to_owned()),
                ("Bb".to_owned(), "bb".to_owned())
            ]
        );
        assert!(outside.contains("TextId::A;") && !outside.contains("a-key"));
        assert!(parse_text_ids("text_ids! {\n A => \"a\",\n").is_none());

        let ftl = "# c\na-key = x\n    .attr = y\n-term = z\nbb = { $n ->\n   *[other] n\n}\n";
        assert_eq!(ftl_keys(ftl), BTreeSet::from(["a-key", "bb"]));

        let used = used_variants("TextId::A TextId::AbC(x) TextId::ALL TextId::lower");
        assert_eq!(
            used,
            BTreeSet::from(["A".to_owned(), "AbC".to_owned(), "ALL".to_owned()])
        );
    }
}
