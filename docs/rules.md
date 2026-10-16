# Emergent Tactics rules, version 1

Authoritative text. `crates/et-engine` implements exactly this; when they
disagree, one of them is a bug.

## Glossary

- **Entity**: one of the two participants, human or AI.
- **Archetype**: template giving an entity its starting attributes and deck.
- **Match**: a sequence of rounds ending in a win or a draw.
- **Round**: the first entity's turn followed by the second entity's turn.
- **Turn**: one entity's start phase, main phase, and end phase.
- **Resource**: the per-turn budget spent to play cards. Refills to the
  resource limit at the start of each turn; unspent resource is lost.
- **Resource limit**: base value from the archetype plus any temporary bonuses
  active this turn. Bonuses are additive and expire at end of turn.
- **Lifepool**: hit points. At 0 or less the entity loses.
- **Block**: absorbs damage before lifepool. Reset to 0 at the start of the
  owner's turn, so it only protects during the opponent's turn.
- **Damage**: reduced by Block first, then lifepool. Affected by Vulnerable.
- **Direct life loss**: reduces lifepool, ignoring Block and Vulnerable.
- **Status**: a buff or debuff on an entity, identified by kind, with a
  counter N (0 means absent). Applying a status again adds to N. Each kind
  declares how N ticks:
  - *Never*: lasts for the match (Fury).
  - *End of round*: N decreases by 1 at the end of every round, including
    the round it was applied in; at 0 it is removed (Vulnerable).
  - *Consumed at turn start*: fires once at the owner's next turn start,
    then is removed (Stand Ground).
  Statuses interact with the game only through fixed hooks: start of turn,
  incoming damage, and end of round. New statuses add a kind and hook
  behaviour; they never add new hook points without a rules change.
- **Vulnerable N**: while N > 0 the entity takes 1 additional damage from each
  instance of damage.
- **Availability**: whether a card or archetype may be used by players,
  NPCs, or both. Enforced when a deck is built. The engine, search and model
  do not distinguish; the AI must understand player-only cards to play
  against them.
- **Exile**: a face-up pile outside the deck. Exiled cards never return. All
  Power cards are exiled when played.

## Zones

Each entity has a draw pile (face down, order unknown to both players), a
hand (visible only to its owner), a discard pile, and an exile pile (public).

**Deck lists are private.** The opponent knows an entity's archetype and
deck size, but not its contents. A card becomes known to the opponent when
it is played. After a reshuffle, a played card is known to exist somewhere
in the owner's hidden zones but its zone is unknown. The AI's search models
the opponent's unknown cards by sampling from the archetype's card pool; it
is never given the true list.

The discard pile has two parts. Cards played go to the discard **face up**
and are public. Cards discarded unplayed at end of turn go to the discard
**face down**; the opponent sees only how many. This is the main source of
hidden information: the opponent cannot tell which unplayed cards sit in the
discard versus the draw pile.

Hand limit is 10. Cards drawn beyond the limit go to the discard face down.

When the draw pile is empty and a draw is required, the entire discard pile
is shuffled to form the new draw pile, then drawing continues. If both piles
are empty, drawing stops.

## Turn structure

Start phase, in this order:

1. Block is set to 0.
2. Start-of-turn status effects fire in status declaration order, which is
   Fury, then Stand Ground, then Vulnerable (no effect):
   - **Fury** (permanent): lose 2 life directly. If still above 0, gain
     +1 resource limit this turn.
   - **Stand Ground** (consumed): gain +2 resource limit this turn and
     +2 Block per stack. Then it is removed.
3. Resource is set to the resource limit.
4. Draw `draw amount` cards.

Main phase: play any number of cards one at a time, each costing its resource
cost, then end the turn. Cards resolve fully before the next is played. The
order of cards played this turn is tracked and public.

End phase:

1. All cards in hand go to the discard face down.
2. Temporary resource-limit bonuses are cleared.
3. If this was the second entity's turn, the round ends: every debuff on
   either entity loses 1 N, and expired debuffs are removed.

A match does not draw at match start; the first draw is the first entity's
turn-one draw.

## Winning

An entity whose lifepool is 0 or less loses immediately, checked after every
effect that changes lifepool. The other entity wins. If 30 full rounds
complete with no winner, the match is a draw (assumed value, pending review).

## Archetypes

An archetype defines lifepool, draw amount, resource limit, any statuses the
entity starts the match with, the pool of cards its decks may contain, and a
stock deck. Archetypes and cards each carry an availability.

### Warrior (available to both)

- lifepool 20, draw amount 4, resource limit 3, no starting statuses
- Pool: Strike, Defend, Stand Ground, Charge!, Fury
- Stock deck: Strike ×4, Defend ×3, Stand Ground, Charge!, Fury

## Cards

| Name | Type | Archetype | Availability | Cost | Effect |
|------|------|-----------|--------------|-----:|--------|
| Strike | Attack | Any | Both | 1 | Deal 4 damage to the target. |
| Defend | Skill | Any | Both | 1 | Gain 3 Block. |
| Stand Ground | Skill | Warrior | Both | 2 | At the start of your next turn gain +2 resource limit and +2 Block. |
| Charge! | Attack | Warrior | Both | 2 | Deal 4 damage to the target, then apply Vulnerable 2 to the target. |
| Fury | Power | Warrior | Both | 1 | Exile. At the start of each of your turns lose 2 life directly; if still above 0, gain +1 resource limit this turn. |

Charge! deals its damage before applying Vulnerable, so its own hit does not
benefit. Example timeline for the first entity playing Charge! in round R:
target has Vulnerable 2 for the rest of round R, Vulnerable 1 during round
R+1, and it is removed at the end of round R+1.

## Actions

An action is either *play a card of a given name* or *end turn*. Duplicate
cards are interchangeable, so the action space is the number of distinct
cards plus one. Targeting is implicit with two entities.

## Library fingerprint

Everything above that a trained model depends on (cards, statuses,
archetypes, encoder dimensions) is digested into a 64-bit fingerprint.
Models are exported with it and the game server refuses an inference service
whose fingerprint differs from its engine's.
