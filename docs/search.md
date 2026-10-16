# How the AI decides: ISMCTS

Implemented in `crates/et-search/src/ismcts.rs`. This is Single-Observer
Information Set MCTS (Cowling, Powley & Whitehouse, 2012) with AlphaZero's
prior-guided selection rule (PUCT).

## One decision, step by step

1. **Observe.** The server hands the search the `Observation` for the player
   to move: own hand and deck, both sides' public state, the opponent's
   revealed cards and zone counts. Never the opponent's deck list.
2. **Root expansion.** The evaluator returns a prior over the legal actions
   and a value estimate. With the rollout evaluator the prior is uniform
   and the value is a random playout; with a network both come from the
   model.
3. **Determinize.** Sample `determinizations` concrete worlds consistent
   with the observation: shuffle the own draw pile, fill the opponent's
   hidden zones from their archetype's card pool. Iterations cycle through
   these worlds round-robin.
4. **Select.** From the root, descend the tree. At each node only the
   actions legal in *this* world are considered. Each candidate scores
   `Q + c · prior · sqrt(available) / (1 + visits)`, where `available`
   counts how often the action was legal when the node was visited. The
   best scoring action is applied to the world.
5. **Expand.** On reaching a node never expanded, ask the evaluator for
   priors and a value from that node's observation, create its children,
   and stop descending. A terminal state yields +1/-1/0 instead.
6. **Back up.** Add the leaf value to every node on the path, negated for
   nodes where the other player is to move.
7. **Choose.** After all iterations, the root children's visit counts form
   a distribution. Temperature 0 picks the most visited; higher
   temperatures sample, which is how weaker difficulties stay plausible
   rather than random.

The `ai_decision` message carries each root candidate's prior, visits and
mean value, so the client shows exactly what the search believed.

## Why information sets matter here

A plain MCTS would search one concrete world and, for example, plan around
the opponent holding no Defend. Cycling determinizations means a node's
statistics average over many possible opponent hands. The tree is keyed by
the acting player's information, not by hidden state, so a line that only
works in one world cannot dominate.

Fewer determinizations make the AI more confident and more exploitable;
more make it hedge. That knob is part of the difficulty preset for exactly
that reason.

## What the search produces for training

Each root decision yields a training sample:

- **input**: the encoded observation (`encode_observation`),
- **policy target**: the normalised root visit distribution,
- **value target**: the eventual game result from this player's
  perspective, filled in when the game ends.

Self-play runs games with the search on both sides, collects these samples,
and the network is trained to predict the policy and value directly. The
trained network then replaces random rollouts as the evaluator, which makes
each iteration far more informative, which produces better samples, and so
on. Difficulty levels come from which checkpoint is used plus the search
budget and temperature.
