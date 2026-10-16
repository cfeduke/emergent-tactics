# ADR 0004: Statuses are a generic table with fixed hooks

Status: accepted · Date: 2026-10-16

## Context

The card library will grow to many buffs and debuffs that are not designed
yet. Each one must be understood by the engine, encoded for the network,
displayed by the client, and summarised by the server. Hardcoding a field
per status would make every addition a cross-cutting change.

## Decision

`StatusKind` is an enum; each entity holds a counter per kind. A status is
described by name, polarity and a tick rule (never, end of round, consumed
at turn start). Behaviour is attached at three fixed hooks in the engine:
start of turn (fired in declaration order), incoming damage modification,
and end of round. The encoder emits one feature per kind; the client and
server treat statuses generically by kind and name.

## Consequences

- Adding a status is: one enum variant, one `StatusDef`, and match arms in
  the hooks it uses. Nothing else changes.
- Any addition changes `FEATURE_LEN` and the library fingerprint, so the
  model must be retrained (warm-started if desired). The search is
  unaffected.
- A status that needs a new hook point (for example, modifying card costs)
  is a rules change and gets its own ADR.
