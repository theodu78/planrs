//! Scène : lignes, barres, liens, avant l'écriture SVG (lot L7).
//!
//! Tout est en entiers : abscisses et ordonnées en 1/16 px. Les dates des
//! barres viennent du planning (règle 3) ; aucun calcul de date ici.

use std::collections::BTreeMap;

use planrs_core::{LinkType, Project, Schedule, Task, TaskOutcome, TaskRole, TaskUid};

use crate::axis::{Axis, SUB};
use crate::request::RowFilter;

/// Hauteur d'une ligne (22 px).
pub(crate) const ROW_H: i64 = 22 * SUB;
/// Demi-hauteur d'une barre (5 px).
pub(crate) const BAR_HALF: i64 = 5 * SUB;
/// Demi-diagonale d'un losange de jalon (6 px).
pub(crate) const MILESTONE_HALF: i64 = 6 * SUB;
/// Segment de sortie d'un lien (6 px).
const STUB: i64 = 6 * SUB;
/// Longueur de la pointe de flèche (4 px).
pub(crate) const ARROW: i64 = 4 * SUB;
/// Demi-largeur de la pointe (0,8 × 4 px ≈ 51/16 px).
pub(crate) const ARROW_HALF: i64 = 51;
/// Débord admis d'une barre hors de la plage avant d'être rognée (64 px) :
/// le reste est masqué par la découpe, la sortie reste courte.
const OVERFLOW: i64 = 64 * SUB;

/// Nature d'une ligne (mêmes cas que l'interface).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowKind {
    /// Ligne vide.
    Null,
    /// Feuille planifiée.
    Leaf,
    /// Récapitulative.
    Summary,
    /// Jalon (ou feuille de durée nulle).
    Milestone,
    /// Exclue du calcul (barre d'affichage R-062 si elle existe).
    Excluded,
}

/// Ligne retenue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RowScene {
    /// ID affiché.
    pub id: u32,
    /// Niveau hiérarchique.
    pub level: u16,
    /// Nature.
    pub kind: RowKind,
    /// Chemin critique.
    pub critical: bool,
    /// Abscisses (1/16 px) du début et de la fin de la barre.
    pub bar: Option<(i64, i64)>,
    /// Nom tel quel (tronqué et échappé à l'écriture).
    pub name: String,
}

/// Lien entre deux lignes retenues.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinkScene {
    /// Ligne du prédécesseur.
    pub from: usize,
    /// Ligne du successeur.
    pub to: usize,
    /// Type.
    pub kind: LinkType,
    /// Critique (les deux extrémités le sont).
    pub critical: bool,
}

impl LinkScene {
    /// Ligne la plus basse : le lien n'est écrit que si elle est montrée.
    pub(crate) fn last_row(&self) -> usize {
        self.from.max(self.to)
    }
}

/// Scène complète.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Scene {
    /// Lignes retenues par le filtre, dans l'ordre des ID.
    pub rows: Vec<RowScene>,
    /// Tâches du projet (lignes vides comprises).
    pub total: usize,
    /// Liens dont les deux extrémités sont retenues, triés par ligne la plus
    /// basse puis par ordre d'apparition.
    pub links: Vec<LinkScene>,
}

/// Planning d'une tâche : nature, critique, début et fin.
fn row_parts(task: &Task, outcome: Option<&TaskOutcome>, axis: &Axis) -> (RowKind, bool, Option<(i64, i64)>) {
    let lo = axis.left16().saturating_sub(OVERFLOW);
    let hi = axis.right16().saturating_add(OVERFLOW);
    let clamp = |x: i64| x.clamp(lo, hi);
    if task.is_null {
        return (RowKind::Null, false, None);
    }
    match outcome {
        Some(TaskOutcome::Scheduled(s)) => {
            let kind = if s.role == TaskRole::Summary {
                RowKind::Summary
            } else if task.milestone || task.duration.is_zero() {
                RowKind::Milestone
            } else {
                RowKind::Leaf
            };
            let x0 = clamp(axis.x16(s.start));
            let x1 = if kind == RowKind::Milestone {
                x0
            } else {
                clamp(axis.x16(s.finish)).max(x0)
            };
            (kind, s.critical, Some((x0, x1)))
        }
        Some(TaskOutcome::Excluded { display, .. }) => (
            RowKind::Excluded,
            false,
            display.map(|span| {
                let x0 = clamp(axis.x16(span.start));
                (x0, clamp(axis.x16(span.finish)).max(x0))
            }),
        ),
        None => (RowKind::Excluded, false, None),
    }
}

/// Construit la scène : lignes du filtre dans l'ordre des ID, liens entre
/// lignes retenues.
pub(crate) fn build(project: &Project, schedule: &Schedule, filter: &RowFilter, axis: &Axis) -> Scene {
    let mut rows = Vec::new();
    let mut index: BTreeMap<TaskUid, usize> = BTreeMap::new();
    let mut kept: Vec<&Task> = Vec::new();
    for (position, task) in project.tasks.iter().enumerate() {
        let uid = task.uid();
        let (kind, critical, bar) = row_parts(task, schedule.tasks.get(&uid), axis);
        let level = task.outline_level.get();
        if filter.critical_only && !critical {
            continue;
        }
        if filter.uids.as_ref().is_some_and(|set| !set.contains(&uid)) {
            continue;
        }
        if filter.max_outline.is_some_and(|max| level > max) {
            continue;
        }
        let id = u32::try_from(position)
            .ok()
            .and_then(|p| p.checked_add(1))
            .unwrap_or(u32::MAX);
        index.insert(uid, rows.len());
        kept.push(task);
        rows.push(RowScene {
            id,
            level,
            kind,
            critical,
            bar,
            name: task.name.clone(),
        });
    }
    let mut links = Vec::new();
    for (to, task) in kept.iter().enumerate() {
        let to_critical = rows.get(to).is_some_and(|r| r.critical);
        for link in &task.predecessors {
            let Some(&from) = index.get(&link.predecessor) else {
                continue;
            };
            if from == to {
                continue;
            }
            let from_critical = rows.get(from).is_some_and(|r| r.critical);
            links.push(LinkScene {
                from,
                to,
                kind: link.kind,
                critical: from_critical && to_critical,
            });
        }
    }
    // Tri stable : l'ordre d'apparition départage.
    links.sort_by_key(LinkScene::last_row);
    Scene {
        rows,
        total: project.tasks.len(),
        links,
    }
}

/// Extrémités d'une barre vues par un lien (losange pour un jalon).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ends {
    /// Début.
    pub start: i64,
    /// Fin.
    pub finish: i64,
    /// Centre vertical.
    pub y: i64,
}

impl Ends {
    /// Extrémités de la ligne `row` d'ordonnée centrale `y`, si elle a une
    /// barre.
    pub(crate) fn of(row: &RowScene, y: i64) -> Option<Self> {
        let (x0, x1) = row.bar?;
        if row.kind == RowKind::Null {
            return None;
        }
        Some(if row.kind == RowKind::Milestone {
            Self {
                start: x0.saturating_sub(MILESTONE_HALF),
                finish: x0.saturating_add(MILESTONE_HALF),
                y,
            }
        } else {
            Self {
                start: x0,
                finish: x1,
                y,
            }
        })
    }
}

/// Direction de la pointe de flèche.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrowDir {
    /// Vers la droite (entrée par le début).
    Right,
    /// Vers la gauche (entrée par la fin).
    Left,
    /// Vers le bas (entrée par le dessus).
    Down,
    /// Vers le haut (entrée par le dessous).
    Up,
}

/// Tracé d'un lien : polyligne orthogonale, flèche au dernier point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkPath {
    /// Points (2 à 6).
    pub points: Vec<(i64, i64)>,
    /// Direction de la flèche.
    pub dir: ArrowDir,
}

/// Tracé d'un lien `kind` de `p` à `s` : même tracé que `link_path` de
/// l'interface (H-UI), en entiers.
pub(crate) fn link_path(kind: LinkType, p: Ends, s: Ends) -> LinkPath {
    let (p0, p1, yp) = (p.start, p.finish.max(p.start), p.y);
    let (s0, s1, ys) = (s.start, s.finish.max(s.start), s.y);
    let below = ys >= yp;
    let half_row = ROW_H / 2;
    let ymid = if below {
        yp.saturating_add(half_row)
    } else {
        yp.saturating_sub(half_row)
    };
    let path = |points: Vec<(i64, i64)>, dir| LinkPath { points, dir };
    match kind {
        LinkType::FinishToStart => {
            let xa = s0.saturating_add(STUB.min(s1.saturating_sub(s0) / 2));
            if xa >= p1.saturating_add(STUB / 2) {
                let (edge, dir) = if below {
                    (ys.saturating_sub(BAR_HALF), ArrowDir::Down)
                } else {
                    (ys.saturating_add(BAR_HALF), ArrowDir::Up)
                };
                path(vec![(p1, yp), (xa, yp), (xa, edge)], dir)
            } else {
                let xo = p1.saturating_add(STUB);
                let xi = s0.saturating_sub(STUB);
                path(
                    vec![(p1, yp), (xo, yp), (xo, ymid), (xi, ymid), (xi, ys), (s0, ys)],
                    ArrowDir::Right,
                )
            }
        }
        LinkType::StartToStart => {
            let xl = p0.min(s0).saturating_sub(STUB);
            path(vec![(p0, yp), (xl, yp), (xl, ys), (s0, ys)], ArrowDir::Right)
        }
        LinkType::FinishToFinish => {
            let xr = p1.max(s1).saturating_add(STUB);
            path(vec![(p1, yp), (xr, yp), (xr, ys), (s1, ys)], ArrowDir::Left)
        }
        LinkType::StartToFinish => {
            let xl = p0.saturating_sub(STUB);
            if s1.saturating_add(STUB) <= xl {
                path(vec![(p0, yp), (xl, yp), (xl, ys), (s1, ys)], ArrowDir::Left)
            } else {
                let xr = s1.saturating_add(STUB);
                path(
                    vec![(p0, yp), (xl, yp), (xl, ymid), (xr, ymid), (xr, ys), (s1, ys)],
                    ArrowDir::Left,
                )
            }
        }
    }
}

/// Triangle de la pointe : pointe en `tip`, orientée selon `dir`.
pub(crate) fn arrow_points(tip: (i64, i64), dir: ArrowDir) -> [(i64, i64); 3] {
    let (x, y) = tip;
    match dir {
        ArrowDir::Right => [
            tip,
            (x.saturating_sub(ARROW), y.saturating_add(ARROW_HALF)),
            (x.saturating_sub(ARROW), y.saturating_sub(ARROW_HALF)),
        ],
        ArrowDir::Left => [
            tip,
            (x.saturating_add(ARROW), y.saturating_sub(ARROW_HALF)),
            (x.saturating_add(ARROW), y.saturating_add(ARROW_HALF)),
        ],
        ArrowDir::Down => [
            tip,
            (x.saturating_sub(ARROW_HALF), y.saturating_sub(ARROW)),
            (x.saturating_add(ARROW_HALF), y.saturating_sub(ARROW)),
        ],
        ArrowDir::Up => [
            tip,
            (x.saturating_add(ARROW_HALF), y.saturating_add(ARROW)),
            (x.saturating_sub(ARROW_HALF), y.saturating_add(ARROW)),
        ],
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use super::*;

    const fn ends(start: i64, finish: i64, row: i64) -> Ends {
        Ends {
            start: start * SUB,
            finish: finish * SUB,
            y: (11 + 22 * row) * SUB,
        }
    }

    #[test]
    fn fs_direct_enters_from_above() {
        let path = link_path(LinkType::FinishToStart, ends(0, 50, 0), ends(80, 120, 1));
        assert_eq!(path.dir, ArrowDir::Down);
        assert_eq!(path.points.len(), 3);
        assert_eq!(path.points[0], (50 * SUB, 11 * SUB));
        assert_eq!(path.points[2], (86 * SUB, 28 * SUB));
    }

    #[test]
    fn fs_folded_when_successor_starts_before() {
        let path = link_path(LinkType::FinishToStart, ends(0, 50, 0), ends(20, 60, 1));
        assert_eq!(path.dir, ArrowDir::Right);
        assert_eq!(path.points.len(), 6);
        assert_eq!(path.points[5], (20 * SUB, 33 * SUB));
    }

    #[test]
    fn other_kinds_end_on_expected_side() {
        let p = ends(10, 50, 0);
        let s = ends(30, 70, 2);
        assert_eq!(
            link_path(LinkType::StartToStart, p, s).points.last(),
            Some(&(30 * SUB, 55 * SUB))
        );
        let ff = link_path(LinkType::FinishToFinish, p, s);
        assert_eq!(ff.points.last(), Some(&(70 * SUB, 55 * SUB)));
        assert_eq!(ff.dir, ArrowDir::Left);
        let sf = link_path(LinkType::StartToFinish, p, s);
        assert_eq!(sf.points.len(), 6);
        assert_eq!(sf.points.last(), Some(&(70 * SUB, 55 * SUB)));
        let up = link_path(LinkType::FinishToStart, ends(0, 10, 3), ends(40, 60, 1));
        assert_eq!(up.dir, ArrowDir::Up);
    }
}
