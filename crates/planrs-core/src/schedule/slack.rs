//! Marges et critique (spécification J2 § 3.9, R-048 à R-050, R-060,
//! R-067).
//!
//! - Marge totale : `min(M.between(ES, LS), M.between(EF, LF))`, `M` étant
//!   l'échelle de marge (placement de la feuille, temps civil pour une durée
//!   écoulée ; calendrier d'une récapitulative).
//! - Marge libre : `max(0, min(TF, écarts aux successeurs))`, arcs sortants
//!   propres et hérités des récapitulatives ancêtres (R-101 : sans
//!   contrainte ni échéance, la borne à 0 est sans effet).
//! - Critique : `TF ≤ seuil × minutes par jour`.

use super::constraint::{ConstraintRule, Mode, Side};
use super::network::{Fail, NodeIx, NodeKind};
use super::{Calc, Pass, transfer};
use crate::calendar::{Span, TimeScale};
use crate::diagnostic::Diagnostic;
use crate::duration::Minutes;
use crate::model::LinkType;

impl Calc<'_, '_> {
    /// Seuil de criticité en minutes (R-050), saturé.
    pub(super) fn critical_limit(&self) -> Minutes {
        let o = &self.project.options;
        let days = i64::from(o.critical_slack_limit_days);
        let per_day = i64::from(o.factors.minutes_per_day());
        Minutes::new(days.checked_mul(per_day).unwrap_or(i64::MAX))
    }

    /// Marges de toutes les feuilles et récapitulatives actives planifiées.
    pub(super) fn slack(&mut self) {
        for k in 0..self.net.nodes.len() {
            let Some(v) = super::network::ix(k) else { continue };
            let Some(node) = self.net.node(v) else { continue };
            if node.inactive || node.kind == NodeKind::Gate {
                continue;
            }
            let uid = node.uid;
            let Some(s) = self.state(v).copied() else { continue };
            let (Some(early), Some(late)) = (s.early, s.late) else {
                continue;
            };
            match self.node_slack(v, early, late) {
                Ok((tf, fs)) => {
                    if let Some(st) = self.state_mut(v) {
                        st.total_slack = tf;
                        st.free_slack = fs;
                    }
                }
                Err(fail) => {
                    if let Some(st) = self.state_mut(v) {
                        st.degraded = true;
                    }
                    self.report(uid, Pass::Backward, fail);
                }
            }
        }
    }

    /// Marge totale et marge libre du nœud `v`.
    fn node_slack(&mut self, v: NodeIx, early: Span, late: Span) -> Result<(Minutes, Minutes), Fail> {
        let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
        let m = *self
            .net
            .scale(node.place)
            .ok_or(Fail::Internal("échelle hors bornes"))?;
        let tf = m
            .between(early.start, late.start)?
            .min(m.between(early.finish, late.finish)?);
        let mut fs = tf;
        // Arcs propres, puis ceux des agrégats ancêtres (§ 3.9) ; un arc SS
        // ou SF hérité ne compte que pour un nœud de même début que
        // l'ancêtre (comme son plafond en passe arrière, A11).
        let net = self.net;
        let mut owner = Some(v);
        let mut first = true;
        while let Some(o) = owner {
            let anc_start = self.state(o).and_then(|s| s.early).map(|e| e.start);
            for &a in net.outgoing(o).unwrap_or_default() {
                if let Some(gap) = self.gap(v, a, m, early, !first && anc_start != Some(early.start))? {
                    fs = fs.min(gap);
                }
            }
            // Une récapitulative manuelle datée (R-064) ne dépend pas de ses
            // enfants : ses liens et ceux de ses ancêtres ne les concernent pas.
            owner = self
                .net
                .node(o)
                .and_then(|n| n.parent_rollup)
                .filter(|&r| !self.net.is_fixed_rollup(r));
            first = false;
        }
        // R-101 : marge libre bornée à 0 (Q10 : MS Project affiche 0 quand
        // la marge totale est négative) ; sans contrainte ni échéance, la
        // borne est sans effet (FS ≥ 0 reste une propriété, P13).
        Ok((tf, fs.min(tf).max(Minutes::ZERO)))
    }

    /// Contraintes non tenues, après les marges (R-099) : depuis le début,
    /// un plafond ou une fenêtre (option inactive) dont la date au plus tôt
    /// dépasse l'ancrage : début en brut (`ES > s*`), fin en temps ouvré
    /// (`P.between(f*, EF) > 0`) ; `constraint.not_honored` avec la date
    /// saisie.
    pub(super) fn check_constraints(&mut self) {
        for (node, s) in self.net.nodes.iter().zip(&self.st) {
            let (Some(c), Some(early)) = (node.constraint, s.early) else {
                continue;
            };
            let (ConstraintRule::Cap(side), Mode::FromStart) = (c.rule, self.mode) else {
                continue;
            };
            let (Some(date), Some(Ok(a)), Some(scale)) = (c.date, c.anchor, self.net.scale(node.place))
            else {
                continue;
            };
            let late = match side {
                Side::Start => early.start > a.start,
                Side::Finish => scale.between(a.finish, early.finish).is_ok_and(|m| m.get() > 0),
            };
            if late {
                self.diags.push(Diagnostic::ConstraintNotHonored {
                    task: node.uid,
                    kind: c.kind,
                    date,
                });
            }
        }
    }

    /// Écart de l'arc de lien `a` pour le nœud `v` (R-049) : `M.between(ancre propre,
    /// h(ancre au plus tôt du successeur))`. `None` pour un arc ignoré
    /// (hiérarchie, coupé, vers une tâche inactive, cible sans dates, ou arc
    /// SS/SF hérité écarté).
    fn gap(
        &mut self,
        v: NodeIx,
        a: u32,
        m: TimeScale<'_>,
        own: Span,
        skip_start_kind: bool,
    ) -> Result<Option<Minutes>, Fail> {
        if self.net.is_cut(a) || self.is_dropped(a) {
            return Ok(None);
        }
        let arc = *self.net.arc(a).ok_or(Fail::Internal("arc hors bornes"))?;
        let Some(l) = arc.link else { return Ok(None) };
        let start_kind = matches!(l.ty, LinkType::StartToStart | LinkType::StartToFinish);
        if l.to_inactive || (start_kind && skip_start_kind) {
            return Ok(None);
        }
        // Successeur peu sûr : marge libre de repli (R-069).
        if self.unreliable(arc.to)
            && let Some(s) = self.state_mut(v)
        {
            s.late_degraded = true;
        }
        let target = self.net.node(arc.to).ok_or(Fail::Internal("nœud hors bornes"))?;
        // Cible porte : début agrégé des sous-tâches (min ES des enfants, même
        // sous une récapitulative manuelle datée, R-064).
        // Porte en échec (R-069) : le lien n'a pas été appliqué.
        let te = match target.kind {
            NodeKind::Gate if self.state(arc.to).is_some_and(|s| s.early.is_none()) => None,
            NodeKind::Gate => target.twin.and_then(|r| self.state(r)).and_then(|s| s.children),
            _ => self.state(arc.to).and_then(|s| s.early),
        };
        let Some(te) = te else { return Ok(None) };
        let anchor = match l.ty {
            LinkType::FinishToStart | LinkType::StartToStart => te.start,
            LinkType::FinishToFinish | LinkType::StartToFinish => self.finish_anchor(arc.to, te.finish),
        };
        let lag = self.lag_minutes(a)?;
        let sigma = self
            .net
            .scale(l.sigma)
            .ok_or(Fail::Internal("échelle hors bornes"))?;
        let bound = transfer::backward(*sigma, anchor, lag, self.reads_raw(arc.to, l.sigma))?;
        let (mine, bound) = match l.ty {
            LinkType::FinishToStart | LinkType::FinishToFinish => (own.finish, bound),
            // Borne de début d'une feuille : dernier début plaçable (R-047,
            // R-049), comme son plafond en passe arrière.
            LinkType::StartToStart | LinkType::StartToFinish => {
                let node = self.net.node(v).ok_or(Fail::Internal("nœud hors bornes"))?;
                let bound = if node.kind == NodeKind::Leaf || node.fixed.is_some() {
                    m.latest_start(bound, node.duration)?
                } else {
                    bound
                };
                (own.start, bound)
            }
        };
        Ok(Some(m.between(mine, bound)?))
    }
}
