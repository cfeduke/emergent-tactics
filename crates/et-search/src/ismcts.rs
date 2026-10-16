//! Single-Observer Information Set Monte Carlo Tree Search with
//! prior-guided (PUCT) selection.
//!
//! One tree is built from the root player's point of view. Every iteration
//! samples a concrete state consistent with the root observation (a
//! determinization), walks the tree applying actions to that state, expands
//! a leaf by asking the evaluator for priors and a value, and backs the
//! value up. Because each determinization may make different actions legal
//! at the same node, children track how often they were *available* and
//! the exploration term uses that instead of the parent's visit count.

use std::time::Instant;

use et_engine::{Action, GameState, Observation, Player};
use rand::Rng;
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};

use crate::evaluator::outcome_value;
use crate::{Difficulty, Evaluator};

/// Everything the search knew about one root action. Sent to the client
/// verbatim so the AI's reasoning can be inspected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub action: Action,
    pub prior: f32,
    pub visits: u32,
    /// Mean backed-up value from the root player's perspective, in [-1, 1].
    pub value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStats {
    pub iterations: u32,
    pub determinizations: u32,
    pub elapsed_ms: u64,
    /// Root value estimate: visit-weighted mean of the candidates.
    pub root_value: f32,
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub action: Action,
    pub stats: SearchStats,
}

type NodeId = usize;

struct Child {
    action: Action,
    prior: f32,
    /// Times this action was legal when its parent was selected through.
    available: u32,
    node: Option<NodeId>,
}

struct Node {
    /// Player to move at this node. Fixed by the action sequence.
    player: Player,
    visits: u32,
    /// Sum of backed-up values from `player`'s perspective.
    value_sum: f32,
    expanded: bool,
    children: Vec<Child>,
}

impl Node {
    fn new(player: Player) -> Self {
        Self { player, visits: 0, value_sum: 0.0, expanded: false, children: Vec::new() }
    }
    fn mean(&self) -> f32 {
        if self.visits == 0 { 0.0 } else { self.value_sum / self.visits as f32 }
    }
}

struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    fn new(root_player: Player) -> Self {
        Self { nodes: vec![Node::new(root_player)] }
    }

    fn add(&mut self, player: Player) -> NodeId {
        self.nodes.push(Node::new(player));
        self.nodes.len() - 1
    }

    /// Value of `child` from `parent`'s perspective.
    fn q_for_parent(&self, parent: NodeId, child: &Child) -> f32 {
        match child.node {
            None => 0.0,
            Some(c) => {
                let m = self.nodes[c].mean();
                if self.nodes[c].player == self.nodes[parent].player { m } else { -m }
            }
        }
    }
}

/// Run ISMCTS from `obs` and return the chosen action with statistics.
pub fn search(obs: &Observation, legal: &[Action], difficulty: &Difficulty, evaluator: &dyn Evaluator, rng: &mut dyn rand::RngCore) -> SearchResult {
    let started = Instant::now();
    let root_player = obs.to_move;
    let mut tree = Tree::new(root_player);

    // Root expansion uses the real observation and the real legal set.
    let root_eval = evaluator.evaluate(obs, legal, rng);
    let mut root_priors: Vec<f32> = root_eval.priors.iter().map(|(_, p)| *p).collect();
    if let Some((alpha, weight)) = difficulty.root_noise
        && root_priors.len() > 1
    {
        let noise = dirichlet(alpha, root_priors.len(), rng);
        for (p, n) in root_priors.iter_mut().zip(noise) {
            *p = (1.0 - weight) * *p + weight * n;
        }
    }
    tree.nodes[0].children = root_eval.priors.iter().zip(root_priors).map(|((a, _), p)| Child { action: *a, prior: p, available: 0, node: None }).collect();
    tree.nodes[0].expanded = true;

    // A fixed set of determinizations, cycled round-robin.
    let d = difficulty.determinizations.max(1).min(difficulty.iterations.max(1));
    let worlds: Vec<GameState> = (0..d).map(|_| obs.sample_state(rng)).collect();

    for i in 0..difficulty.iterations {
        let mut state = worlds[(i % d) as usize].clone();
        let mut path: Vec<NodeId> = vec![0];
        let mut node = 0;

        // Selection and expansion.
        let leaf_value: f32 = loop {
            if let Some(o) = state.outcome() {
                break outcome_value(o, state.to_move);
            }
            if !tree.nodes[node].expanded {
                let legal_here = state.legal_actions();
                let leaf_obs = state.observe(state.to_move);
                let eval = evaluator.evaluate(&leaf_obs, &legal_here, rng);
                tree.nodes[node].children = eval.priors.iter().map(|(a, p)| Child { action: *a, prior: *p, available: 0, node: None }).collect();
                tree.nodes[node].expanded = true;
                break eval.value;
            }
            let legal_here = state.legal_actions();
            let action = select(&mut tree, node, &legal_here, difficulty.exploration);
            state.apply(&action, rng);
            let next_player = state.to_move;
            let child_idx = tree.nodes[node].children.iter().position(|c| c.action == action).expect("child exists");
            let next = match tree.nodes[node].children[child_idx].node {
                Some(n) => n,
                None => {
                    let n = tree.add(next_player);
                    tree.nodes[node].children[child_idx].node = Some(n);
                    n
                }
            };
            path.push(next);
            node = next;
        };

        // Backpropagation. `leaf_value` is from the perspective of the
        // player to move at the leaf, which is `tree.nodes[node].player`.
        let leaf_player = tree.nodes[node].player;
        for &n in &path {
            let nd = &mut tree.nodes[n];
            nd.visits += 1;
            nd.value_sum += if nd.player == leaf_player { leaf_value } else { -leaf_value };
        }
    }

    // Root statistics and final choice.
    let root = &tree.nodes[0];
    let candidates: Vec<Candidate> = root
        .children
        .iter()
        .map(|c| Candidate {
            action: c.action,
            prior: c.prior,
            visits: c.node.map(|n| tree.nodes[n].visits).unwrap_or(0),
            value: tree.q_for_parent(0, c),
        })
        .collect();
    let total: u32 = candidates.iter().map(|c| c.visits).sum();
    let root_value = if total == 0 { root_eval.value } else { candidates.iter().map(|c| c.value * c.visits as f32).sum::<f32>() / total as f32 };
    let action = choose(&candidates, difficulty.temperature, rng).unwrap_or(Action::EndTurn);

    SearchResult {
        action,
        stats: SearchStats {
            iterations: difficulty.iterations,
            determinizations: d,
            elapsed_ms: started.elapsed().as_millis() as u64,
            root_value,
            candidates,
        },
    }
}

/// PUCT over the children legal in the current determinization. Children
/// for actions the evaluator never saw get the mean prior.
fn select(tree: &mut Tree, node: NodeId, legal: &[Action], c_puct: f32) -> Action {
    let mean_prior = {
        let ch = &tree.nodes[node].children;
        if ch.is_empty() { 1.0 } else { ch.iter().map(|c| c.prior).sum::<f32>() / ch.len() as f32 }
    };
    for a in legal {
        if !tree.nodes[node].children.iter().any(|c| c.action == *a) {
            tree.nodes[node].children.push(Child { action: *a, prior: mean_prior, available: 0, node: None });
        }
    }
    for c in tree.nodes[node].children.iter_mut() {
        if legal.contains(&c.action) {
            c.available += 1;
        }
    }
    let mut best: Option<(f32, Action)> = None;
    for c in &tree.nodes[node].children {
        if !legal.contains(&c.action) {
            continue;
        }
        let n = c.node.map(|i| tree.nodes[i].visits).unwrap_or(0);
        let q = tree.q_for_parent(node, c);
        let u = c_puct * c.prior * (c.available as f32).sqrt() / (1.0 + n as f32);
        let score = q + u;
        if best.is_none_or(|(b, _)| score > b) {
            best = Some((score, c.action));
        }
    }
    best.expect("at least one legal child").1
}

/// Dirichlet(alpha) sample via normalised Gamma draws.
fn dirichlet(alpha: f32, n: usize, rng: &mut dyn rand::RngCore) -> Vec<f32> {
    use rand_distr::{Distribution, Gamma};
    let g = Gamma::new(alpha as f64, 1.0).expect("alpha > 0");
    let mut v: Vec<f32> = (0..n).map(|_| g.sample(rng) as f32).collect();
    let sum: f32 = v.iter().sum();
    if sum > 0.0 {
        for x in &mut v {
            *x /= sum;
        }
    }
    v
}

/// Sample from visit counts raised to 1/temperature; temperature 0 is argmax.
fn choose(candidates: &[Candidate], temperature: f32, rng: &mut dyn rand::RngCore) -> Option<Action> {
    if candidates.is_empty() {
        return None;
    }
    if temperature <= 0.0 {
        return candidates.iter().max_by_key(|c| c.visits).map(|c| c.action);
    }
    let weights: Vec<f64> = candidates.iter().map(|c| (c.visits as f64).powf(1.0 / temperature as f64)).collect();
    if weights.iter().sum::<f64>() <= 0.0 {
        return candidates.choose(rng).map(|c| c.action);
    }
    let dist = rand::distr::weighted::WeightedIndex::new(&weights).ok()?;
    Some(candidates[rng.sample(dist)].action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RolloutEvaluator;
    use et_engine::{CardKind, GameState};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn search_prefers_lethal_strike() {
        let mut r = ChaCha8Rng::seed_from_u64(1);
        let mut s = GameState::new(&mut r);
        s.entities[1].lifepool = 4;
        // Ensure a Strike is in hand.
        let e = &mut s.entities[0];
        let id = (0..e.deck_list.len()).find(|&i| e.deck_list[i] == CardKind::Strike).unwrap() as u8;
        if !e.hand.contains(&id) {
            e.draw_pile.retain(|&x| x != id);
            e.discard_face_down.retain(|&x| x != id);
            e.hand.push(id);
        }
        let obs = s.observe(s.to_move);
        let legal = s.legal_actions();
        let d = Difficulty { name: "t".into(), iterations: 300, determinizations: 10, temperature: 0.0, exploration: 1.4, root_noise: None };
        let res = search(&obs, &legal, &d, &RolloutEvaluator { max_depth: 40 }, &mut r);
        assert_eq!(res.action, Action::Play(CardKind::Strike), "{:?}", res.stats.candidates);
        let strike = res.stats.candidates.iter().find(|c| c.action == Action::Play(CardKind::Strike)).unwrap();
        assert!(strike.value > 0.9, "lethal should back up as a near-certain win: {}", strike.value);
    }

    #[test]
    fn visits_sum_to_iterations() {
        let mut r = ChaCha8Rng::seed_from_u64(2);
        let s = GameState::new(&mut r);
        let obs = s.observe(s.to_move);
        let d = Difficulty { name: "t".into(), iterations: 100, determinizations: 4, temperature: 1.0, exploration: 1.4, root_noise: Some((0.5, 0.25)) };
        let res = search(&obs, &s.legal_actions(), &d, &RolloutEvaluator { max_depth: 40 }, &mut r);
        let total: u32 = res.stats.candidates.iter().map(|c| c.visits).sum();
        assert_eq!(total, 100);
        assert_eq!(res.stats.determinizations, 4);
    }
}
