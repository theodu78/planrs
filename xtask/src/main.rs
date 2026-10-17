//! Tâches de maintenance du dépôt (motif cargo-xtask), lancées par
//! `cargo xtask <tâche>` grâce à l'alias de `.cargo/config.toml`.
//!
//! Tâches :
//! - `layers` : vérifie la règle 9 (couches strictes). Aucune crate du
//!   workspace, hors [`GUI_ALLOWED`], ne doit dépendre, directement ou
//!   transitivement, d'egui, eframe, winit ou rfd.

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

use cargo_metadata::{Metadata, MetadataCommand, PackageId};

/// Crates du workspace autorisées à dépendre d'une bibliothèque d'interface.
///
/// La CLI (`planrs-cli`) n'y figure pas exprès : c'est un binaire headless,
/// et `planrs mcp` parle à l'application ouverte par un canal (ADR-0002),
/// pas en liant egui.
const GUI_ALLOWED: &[&str] = &["planrs-ui-egui", "planrs", "planrs-web"];

/// Paquets d'interface interdits sous `planrs-ui-egui` (règle 9).
const GUI_PACKAGES: &[&str] = &["egui", "eframe", "winit", "rfd"];

/// Préfixes de la famille egui (`egui-winit`, `egui_kittest`…), interdits aussi.
const GUI_PREFIXES: &[&str] = &["egui-", "egui_"];

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    if task.as_deref() == Some("layers") {
        return run_layers();
    }
    eprintln!("usage : cargo xtask <tâche>\n\ntâches :\n  layers   vérifie les couches (règle 9)");
    ExitCode::FAILURE
}

fn run_layers() -> ExitCode {
    let metadata = match MetadataCommand::new().exec() {
        Ok(metadata) => metadata,
        Err(error) => {
            eprintln!("xtask layers : `cargo metadata` a échoué : {error}");
            return ExitCode::FAILURE;
        }
    };
    let graph = Graph::from_metadata(&metadata);
    let violations = graph.violations();
    if violations.is_empty() {
        println!(
            "xtask layers : OK ({} crates vérifiées, exemptées : {})",
            graph.members.len(),
            GUI_ALLOWED.join(", ")
        );
        return ExitCode::SUCCESS;
    }
    for violation in &violations {
        eprintln!(
            "xtask layers : `{}` dépend de `{}` (chemin : {})",
            violation.member,
            violation.forbidden,
            violation.path.join(" -> ")
        );
    }
    eprintln!(
        "xtask layers : {} violation(s) de la règle 9 (couches strictes)",
        violations.len()
    );
    ExitCode::FAILURE
}

/// Dépendance interdite trouvée depuis une crate du workspace.
#[derive(Debug, PartialEq, Eq)]
struct Violation {
    member: String,
    forbidden: String,
    /// Chaîne de noms de paquets, du membre au paquet interdit inclus.
    path: Vec<String>,
}

/// Graphe de dépendances résolu, indexé par nom de paquet.
///
/// Les arêtes viennent de la résolution de `cargo metadata` : toutes les
/// sortes de dépendances (normales, build, dev des membres) et toutes les
/// plateformes. Les features y sont unifiées sur tout le workspace, ce qui
/// peut produire un faux positif mais jamais un faux négatif.
#[derive(Debug, Default)]
struct Graph {
    /// Noms des membres du workspace.
    members: BTreeSet<String>,
    /// Nom de paquet -> noms de ses dépendances directes.
    edges: BTreeMap<String, BTreeSet<String>>,
}

impl Graph {
    fn from_metadata(metadata: &Metadata) -> Self {
        let names: BTreeMap<&PackageId, &str> = metadata
            .packages
            .iter()
            .map(|package| (&package.id, package.name.as_str()))
            .collect();
        let name_of = |id: &PackageId| {
            names
                .get(id)
                .map_or_else(|| id.repr.clone(), |name| (*name).to_owned())
        };

        let members = metadata.workspace_members.iter().map(name_of).collect();
        let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        if let Some(resolve) = &metadata.resolve {
            for node in &resolve.nodes {
                let deps = node.deps.iter().map(|dep| name_of(&dep.pkg)).collect();
                edges.insert(name_of(&node.id), deps);
            }
        }
        Self { members, edges }
    }

    /// Liste les membres non exemptés qui atteignent un paquet d'interface.
    fn violations(&self) -> Vec<Violation> {
        self.members
            .iter()
            .filter(|member| !GUI_ALLOWED.contains(&member.as_str()))
            .flat_map(|member| self.forbidden_paths_from(member))
            .collect()
    }

    /// Parcours en largeur depuis `member` ; un chemin par paquet interdit atteint.
    fn forbidden_paths_from(&self, member: &str) -> Vec<Violation> {
        let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
        let mut seen: BTreeSet<&str> = BTreeSet::from([member]);
        let mut queue: std::collections::VecDeque<&str> = std::collections::VecDeque::from([member]);
        let mut found = Vec::new();
        while let Some(current) = queue.pop_front() {
            if current != member && is_gui_package(current) {
                found.push(Violation {
                    member: member.to_owned(),
                    forbidden: current.to_owned(),
                    path: rebuild_path(&parent, current),
                });
                // Inutile de descendre sous un paquet déjà signalé.
                continue;
            }
            for dep in self.edges.get(current).into_iter().flatten() {
                if seen.insert(dep.as_str()) {
                    parent.insert(dep.as_str(), current);
                    queue.push_back(dep.as_str());
                }
            }
        }
        found
    }
}

fn is_gui_package(name: &str) -> bool {
    GUI_PACKAGES.contains(&name) || GUI_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

fn rebuild_path(parent: &BTreeMap<&str, &str>, end: &str) -> Vec<String> {
    let mut path = vec![end.to_owned()];
    let mut current = end;
    while let Some(previous) = parent.get(current) {
        path.push((*previous).to_owned());
        current = previous;
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    // Tests : les interdits de la règle 10 ne s'appliquent pas (échec = panique attendue).
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::{Graph, Violation};

    fn graph(members: &[&str], edges: &[(&str, &[&str])]) -> Graph {
        Graph {
            members: members.iter().map(|name| (*name).to_owned()).collect(),
            edges: edges
                .iter()
                .map(|(from, to)| {
                    (
                        (*from).to_owned(),
                        to.iter().map(|name| (*name).to_owned()).collect(),
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn clean_graph_has_no_violation() {
        let graph = graph(
            &["planrs-core", "planrs-ui-egui"],
            &[
                ("planrs-core", &["chrono"]),
                ("planrs-ui-egui", &["planrs-core", "egui"]),
            ],
        );
        assert_eq!(graph.violations(), Vec::new());
    }

    #[test]
    fn transitive_gui_dependency_is_reported_with_its_path() {
        let graph = graph(
            &["planrs-core", "planrs-render"],
            &[
                ("planrs-render", &["planrs-core", "helper"]),
                ("helper", &["egui-winit"]),
                ("egui-winit", &["winit"]),
            ],
        );
        assert_eq!(
            graph.violations(),
            vec![Violation {
                member: "planrs-render".to_owned(),
                forbidden: "egui-winit".to_owned(),
                path: vec![
                    "planrs-render".to_owned(),
                    "helper".to_owned(),
                    "egui-winit".to_owned()
                ],
            }]
        );
    }

    #[test]
    fn non_exempt_member_depending_on_ui_crate_is_reported() {
        let graph = graph(
            &["planrs-automation", "planrs-ui-egui"],
            &[
                ("planrs-automation", &["planrs-ui-egui"]),
                ("planrs-ui-egui", &["rfd"]),
            ],
        );
        let violations = graph.violations();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].forbidden, "rfd");
    }
}
