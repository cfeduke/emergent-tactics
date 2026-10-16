# ADR 0003: The search never sees the opponent's true deck

Status: accepted · Date: 2026-10-16

## Context

Deck lists are private. The server holds the full game state, including
both deck lists, and runs the AI's search in the same process. Handing the
search the true state would be the easiest implementation and would make
the AI silently cheat.

## Decision

The search operates only on an `Observation`, which contains the viewer's
own deck and the opponent's *revealed* information: cards played face up,
exiled cards, cards seen before a reshuffle (known to be in a hidden zone,
zone unknown), and zone counts. `Observation::sample_state` fills the
opponent's unseen cards by sampling from the opponent archetype's card pool.
Neither the observation nor the encoder carries the opponent's deck list.

The engine tracks cards by physical copy so that "seen" is exact across
reshuffles rather than a lossy multiset guess.

## Consequences

- Early-game play is hedged because the AI genuinely does not know what it
  faces. This is correct, not a weakness to patch.
- The sampling prior is uniform over the pool. A prior learned from how
  decks are actually built is a later improvement and lives in the
  determinizer, not the model.
- A test constructs an opponent deck that differs from the stock deck and
  checks that samples do not always match it, which catches a sampler that
  peeks.
