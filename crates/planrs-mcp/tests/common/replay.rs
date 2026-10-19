//! Harnais de rejeu des scénarios MCP (lot L18), partagé par
//! `crates/planrs-mcp/tests/replay.rs` (en mémoire, HTTP) et
//! `apps/planrs-cli/tests/mcp_replay.rs` (vrai binaire en stdio).
//!
//! Un scénario (`tests/mcp/scenarios/<nom>.jsonl`) a exactement la forme du
//! journal `--journal` du serveur (`planrs_automation::journal::Entry`) :
//! pour chaque appel, une ligne de requête (`params`, sans `result`) puis une
//! ligne de résultat de même `seq` (`params` à `null`, `result` =
//! `{"ok": …}` ou `{"error": …}`). Le jeton `$TMP` des chaînes désigne le
//! dossier temporaire du rejeu : il est remplacé avant l'appel et remis à sa
//! place dans le journal produit, qui doit alors être égal au scénario
//! octet pour octet (déterminisme, vérification 4 du cahier).
//!
//! `PLANRS_BLESS=1` réécrit le scénario (résultats compris) depuis le
//! journal produit, et `tests/mcp/expected/<nom>.xml` depuis le fichier
//! enregistré : à relire avant de versionner.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::missing_panics_doc,
    clippy::arithmetic_side_effects,
    clippy::string_slice,
    reason = "harnais de test partagé : un échec doit s'arrêter net, chaque cible n'en utilise qu'une partie"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use planrs_automation::journal::{Entry, parse_line};
use serde_json::{Value, json};

/// Jeton du dossier temporaire dans les scénarios.
pub const TMP_TOKEN: &str = "$TMP";

/// « Maintenant » figé des scénarios (`--now`).
pub const NOW: &str = "2026-10-19T08:00";

/// Racine du dépôt (les deux crates qui incluent ce fichier sont à deux
/// niveaux de la racine : `crates/planrs-mcp`, `apps/planrs-cli`).
pub fn repo_root(manifest_dir: &str) -> PathBuf {
    Path::new(manifest_dir).join("..").join("..")
}

/// Chemin du scénario `name`.
pub fn scenario_path(root: &Path, name: &str) -> PathBuf {
    root.join("tests/mcp/scenarios").join(format!("{name}.jsonl"))
}

/// Chemin du MSPDI attendu du scénario `name`.
pub fn expected_xml_path(root: &Path, name: &str) -> PathBuf {
    root.join("tests/mcp/expected").join(format!("{name}.xml"))
}

/// Vrai si `PLANRS_BLESS=1` (régénération des attendus).
pub fn blessing() -> bool {
    std::env::var("PLANRS_BLESS").is_ok_and(|v| v == "1")
}

/// Un appel du scénario.
#[derive(Clone, Debug)]
pub struct Call {
    /// Nom de l'outil MCP.
    pub tool: String,
    /// Paramètres (avec le jeton `$TMP`).
    pub params: Value,
    /// Résultat attendu (`{"ok": …}` / `{"error": …}`), absent avant
    /// `PLANRS_BLESS=1`.
    pub expected: Option<Value>,
}

/// Scénario chargé.
#[derive(Clone, Debug)]
pub struct Scenario {
    /// Nom (`build_30`).
    pub name: String,
    /// Texte du fichier, fins de ligne `\r` retirées.
    pub text: String,
    /// Appels, dans l'ordre.
    pub calls: Vec<Call>,
}

impl Scenario {
    /// Charge `tests/mcp/scenarios/<name>.jsonl`.
    pub fn load(root: &Path, name: &str) -> Self {
        let path = scenario_path(root, name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} : {e}", path.display()))
            .replace('\r', "");
        let mut calls: Vec<(u64, Call)> = Vec::new();
        for (n, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let Entry {
                seq,
                tool,
                params,
                result,
                ..
            } = parse_line(line).unwrap_or_else(|e| panic!("{name}.jsonl:{} : {e}", n + 1));
            match result {
                None => calls.push((
                    seq,
                    Call {
                        tool,
                        params,
                        expected: None,
                    },
                )),
                Some(r) => {
                    let Some((s, call)) = calls.last_mut() else {
                        panic!("{name}.jsonl:{} : résultat sans requête", n + 1)
                    };
                    assert_eq!(*s, seq, "{name}.jsonl:{} : seq du résultat", n + 1);
                    assert_eq!(call.tool, tool, "{name}.jsonl:{} : outil du résultat", n + 1);
                    call.expected = Some(r);
                }
            }
        }
        Self {
            name: name.to_owned(),
            text,
            calls: calls.into_iter().map(|(_, c)| c).collect(),
        }
    }
}

/// Forme échappée JSON d'une chaîne (sans les guillemets).
fn json_escaped(s: &str) -> String {
    let quoted = serde_json::to_string(s).unwrap();
    quoted[1..quoted.len() - 1].to_owned()
}

/// Remplace `$TMP` par `tmp` dans toutes les chaînes de `v`.
pub fn substitute(v: &Value, tmp: &Path) -> Value {
    let text = serde_json::to_string(v).unwrap();
    let dir = json_escaped(&tmp.display().to_string());
    serde_json::from_str(&text.replace(TMP_TOKEN, &dir)).unwrap()
}

/// Remet le jeton `$TMP` à la place du dossier `tmp` dans un texte JSON
/// (journal produit) ; un séparateur `\` qui suit le jeton devient `/`
/// (Windows).
pub fn unsubstitute(text: &str, tmp: &Path) -> String {
    let dir = json_escaped(&tmp.display().to_string());
    text.replace('\r', "")
        .replace(&dir, TMP_TOKEN)
        .replace(&format!("{TMP_TOKEN}\\\\"), &format!("{TMP_TOKEN}/"))
}

/// Résultat d'un appel tel que le journal l'écrit.
pub fn logged(is_error: Option<bool>, structured: Option<Value>) -> Value {
    match (structured, is_error) {
        (Some(v), Some(true)) => json!({ "error": v }),
        (Some(v), _) => json!({ "ok": v }),
        (None, _) => json!({ "ok": null }),
    }
}

/// Résultat ramené à la forme du scénario (jeton `$TMP`).
pub fn normalized(result: &Value, tmp: &Path) -> Value {
    let text = serde_json::to_string(result).unwrap();
    serde_json::from_str(&unsubstitute(&text, tmp)).unwrap()
}

/// Compare les résultats obtenus aux attendus, appel par appel.
pub fn assert_results(scenario: &Scenario, got: &[Value]) {
    assert_eq!(
        got.len(),
        scenario.calls.len(),
        "{} : nombre d'appels",
        scenario.name
    );
    for (i, (call, got)) in scenario.calls.iter().zip(got).enumerate() {
        let Some(expected) = &call.expected else {
            panic!(
                "{} : appel {} ({}) sans résultat attendu : régénérer avec PLANRS_BLESS=1",
                scenario.name,
                i + 1,
                call.tool
            )
        };
        assert_eq!(
            got,
            expected,
            "{} : appel {} ({}) : résultat différent de l'attendu",
            scenario.name,
            i + 1,
            call.tool
        );
    }
}

/// Compare le journal produit (jeton `$TMP` remis) au scénario, ligne à
/// ligne puis octet pour octet.
pub fn assert_journal(scenario: &Scenario, produced: &str, tmp: &Path) {
    let produced = unsubstitute(produced, tmp);
    for (n, (a, b)) in produced.lines().zip(scenario.text.lines()).enumerate() {
        assert_eq!(a, b, "{} : ligne {} du journal produit", scenario.name, n + 1);
    }
    assert_eq!(produced, scenario.text, "{} : journal produit", scenario.name);
}

/// `PLANRS_BLESS=1` : réécrit le scénario depuis le journal produit et
/// l'attendu MSPDI depuis le fichier enregistré.
pub fn bless(root: &Path, scenario: &Scenario, produced: &str, tmp: &Path, saved_xml: Option<&Path>) {
    let path = scenario_path(root, &scenario.name);
    std::fs::write(&path, unsubstitute(produced, tmp)).unwrap();
    eprintln!("PLANRS_BLESS : {} réécrit", path.display());
    if let Some(xml) = saved_xml {
        let dest = expected_xml_path(root, &scenario.name);
        std::fs::copy(xml, &dest).unwrap();
        eprintln!("PLANRS_BLESS : {} réécrit", dest.display());
    }
}

/// Dossier temporaire propre à un test (retiré à la fin du test).
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("planrs-replay-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Chemin canonique : celui que renvoie un serveur confiné (`--root`).
        Self(std::fs::canonicalize(&dir).unwrap_or(dir))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
