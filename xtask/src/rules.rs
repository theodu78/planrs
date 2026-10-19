//! `cargo xtask rules` : chaque règle de calcul a ses tests (règle 4 de
//! `CLAUDE.md`, ADR-0013 D10).
//!
//! Lit `docs/scheduling-rules.md` ; pour chaque fiche `## R-xxx` dont le
//! statut n'est pas `retirée`, exige :
//! - une ligne `- **Tests** :` non vide ;
//! - au moins une fonction `fn rxxx_…` (identifiant de la règle sur trois
//!   chiffres, en minuscules) dans un fichier `.rs` de `tests/` ou de
//!   `crates/` (dossiers `target` ignorés).
//!
//! Les blocs de code clôturés (```` ``` ````) du document sont ignorés : la
//! fiche modèle de l'en-tête n'est pas une règle. Sortie triée par
//! identifiant ; code de sortie 1 si une fiche est en défaut.
//!
//! Limite : la recherche des fonctions est textuelle (`fn r170_` dans un
//! commentaire compte aussi) ; elle complète, sans la remplacer, la relecture
//! de la ligne **Tests**.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Document des règles, relatif à la racine du dépôt.
const RULES_DOC: &str = "docs/scheduling-rules.md";
/// Dossiers parcourus pour trouver les tests, relatifs à la racine.
const SOURCE_DIRS: &[&str] = &["tests", "crates"];

pub fn run() -> ExitCode {
    let root = repo_root();
    let doc = match std::fs::read_to_string(root.join(RULES_DOC)) {
        Ok(doc) => doc,
        Err(error) => {
            eprintln!("xtask rules : {RULES_DOC} illisible : {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut tested = BTreeSet::new();
    for dir in SOURCE_DIRS {
        if let Err(error) = collect_test_ids(&root.join(dir), &mut tested) {
            eprintln!("xtask rules : {error}");
            return ExitCode::FAILURE;
        }
    }
    let rules = parse_rules(&doc);
    let missing = check(&rules, &tested);
    if missing.is_empty() {
        let active = rules.iter().filter(|r| !r.retired).count();
        println!(
            "xtask rules : OK ({active} règles actives, {} retirées, chacune avec ses tests)",
            rules.len().saturating_sub(active)
        );
        return ExitCode::SUCCESS;
    }
    for m in &missing {
        eprintln!("xtask rules : R-{:03} : {}", m.id, m.what.message());
    }
    eprintln!("xtask rules : {} manque(s)", missing.len());
    ExitCode::FAILURE
}

/// Racine du dépôt : le dossier parent de la crate `xtask`.
fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map_or_else(|| manifest.to_path_buf(), Path::to_path_buf)
}

/// Fiche d'une règle, telle que lue dans le document.
#[derive(Debug, PartialEq, Eq)]
struct Rule {
    /// Numéro (`R-170` → 170).
    id: u32,
    /// Statut `retirée`.
    retired: bool,
    /// Ligne **Tests** présente et non vide.
    has_tests_line: bool,
}

/// Manque relevé sur une fiche.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Lack {
    /// Pas de ligne **Tests**, ou ligne vide.
    TestsLine,
    /// Aucune fonction `fn rxxx_`.
    TestFunction,
}

impl Lack {
    const fn message(&self) -> &'static str {
        match self {
            Self::TestsLine => "ligne « Tests » absente ou vide",
            Self::TestFunction => "aucune fonction de test « fn rNNN_ » dans tests/ ni crates/",
        }
    }
}

/// Manque d'une règle.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Missing {
    id: u32,
    what: Lack,
}

/// Numéro d'un titre de fiche `## R-170 — …` ; `None` pour tout autre titre.
fn heading_id(line: &str) -> Option<u32> {
    let rest = line.strip_prefix("## R-")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

/// Fiches du document, dans l'ordre, blocs de code exclus.
fn parse_rules(doc: &str) -> Vec<Rule> {
    let mut rules: Vec<Rule> = Vec::new();
    let mut in_fence = false;
    let mut current: Option<Rule> = None;
    for line in doc.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if line.starts_with("## ") || line.starts_with("### ") || line.starts_with("# ") {
            if let Some(rule) = current.take() {
                rules.push(rule);
            }
            current = heading_id(line).map(|id| Rule {
                id,
                retired: false,
                has_tests_line: false,
            });
            continue;
        }
        let Some(rule) = current.as_mut() else { continue };
        if let Some(rest) = trimmed.strip_prefix("- **Statut** :") {
            rule.retired |= rest.contains("retirée");
        } else if let Some(rest) = trimmed.strip_prefix("- **Tests** :") {
            rule.has_tests_line |= !rest.trim().is_empty();
        }
    }
    if let Some(rule) = current {
        rules.push(rule);
    }
    rules
}

/// Manques des fiches actives, triés par numéro.
fn check(rules: &[Rule], tested: &BTreeSet<u32>) -> Vec<Missing> {
    let mut out = Vec::new();
    for rule in rules.iter().filter(|r| !r.retired) {
        if !rule.has_tests_line {
            out.push(Missing {
                id: rule.id,
                what: Lack::TestsLine,
            });
        }
        if !tested.contains(&rule.id) {
            out.push(Missing {
                id: rule.id,
                what: Lack::TestFunction,
            });
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Numéros des fonctions `fn rNNN_…` d'un source (trois chiffres au moins).
fn test_ids(source: &str, out: &mut BTreeSet<u32>) {
    for (at, _) in source.match_indices("fn r") {
        let Some(rest) = source.get(at.saturating_add(4)..) else {
            continue;
        };
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.len() < 3 {
            continue;
        }
        let after = rest.get(digits.len()..).unwrap_or_default();
        if !after.starts_with('_') {
            continue;
        }
        // Le caractère avant `fn` doit séparer (pas `myfn r001_`).
        let before = source.get(..at).and_then(|s| s.chars().next_back());
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        if let Ok(id) = digits.parse() {
            out.insert(id);
        }
    }
}

/// Parcourt `dir` récursivement (dossiers `target` et cachés exclus).
fn collect_test_ids(dir: &Path, out: &mut BTreeSet<u32>) -> Result<(), String> {
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|e| format!("{} : lecture impossible : {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{} : lecture impossible : {e}", dir.display()))?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    pending.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "rs") {
                let source = std::fs::read_to_string(&path)
                    .map_err(|e| format!("{} : lecture impossible : {e}", path.display()))?;
                test_ids(&source, out);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{Lack, Missing, check, parse_rules, test_ids};

    /// Fixture minimale : modèle en bloc de code (ignoré), une règle testée,
    /// une sans fonction, une sans ligne Tests, une retirée.
    const DOC: &str = "# Règles

```
## R-001 — Titre court
- **Tests** : modèle, pas une règle.
```

### Section

## R-010 — Testée
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r010_x.rs`.

## R-011 — Sans fonction
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r011_x.rs`.

## R-012 — Sans ligne
- **Statut** : `hypothèse`.
- **Tests** :

## R-013 — Retirée
- **Statut** : `retirée` (doublon de R-010).
";

    const SOURCE: &str = "
#[test]
fn r010_works() {}
fn r12_too_short() {}
fn myfn r011_glued() {}
// fn r012 sans tiret bas
fn r012x() {}
";

    #[test]
    fn xtask_rules_detects_missing_test() {
        let rules = parse_rules(DOC);
        let ids: Vec<u32> = rules.iter().map(|r| r.id).collect();
        assert_eq!(ids, [10, 11, 12, 13]);
        let mut tested = BTreeSet::new();
        test_ids(SOURCE, &mut tested);
        assert_eq!(tested.iter().copied().collect::<Vec<_>>(), [10]);
        let missing = check(&rules, &tested);
        assert_eq!(
            missing,
            [
                Missing {
                    id: 11,
                    what: Lack::TestFunction
                },
                Missing {
                    id: 12,
                    what: Lack::TestsLine
                },
                Missing {
                    id: 12,
                    what: Lack::TestFunction
                },
            ]
        );
        // Une fonction de plus suffit à R-011 ; R-013, retirée, n'est jamais
        // exigée.
        test_ids("fn r011_now() {}\nfn r012_now() {}", &mut tested);
        assert_eq!(
            check(&rules, &tested),
            [Missing {
                id: 12,
                what: Lack::TestsLine
            }]
        );
    }
}
