# ADR-0003: Plain data-oriented world, no ECS

## Status

Accepted.

## Context

Gameplay state needs a home. The default answer in a modern Rust game project is
an ECS, usually `bevy_ecs` used standalone or `hecs`.

The state in question is small and bounded: at most eight ships, a bounded
number of projectiles, one track, one race. It is not an open world with tens of
thousands of heterogeneous entities.

Three requirements shape this:

1. The world must be snapshot-able cheaply, for replays and golden tests.
2. Update order must be explicit and fixed, for determinism.
3. The whole thing must be hashable to one value, for verification.

## Decision

A single `World` struct of plain data with fixed-size arrays.

```rust
struct World {
    tick: u64,
    rng: Rng,
    ships: [Ship; MAX_SHIPS],
    ship_count: u8,
    projectiles: [Projectile; MAX_PROJECTILES],
    race: RaceState,
}
```

`World::tick(&mut self, input: &InputSnapshot)` calls subsystems in a written-out
order. The order is source code, not a scheduler's output.

## Alternatives considered

**`bevy_ecs` standalone.** Good archetype storage, mature, and its system
scheduler handles parallelism well. Rejected on three counts. Its parallel
scheduler is a determinism hazard, and disabling parallelism removes the main
reason to use it. Snapshotting all state generically is awkward, and we need it
constantly. And system ordering becomes a constraint graph rather than a
sequence, which is harder to reason about and harder to compare against a
trace from the original whose order is fixed.

**`hecs` or generational arenas.** Lighter, no scheduler, so requirement 2 is
satisfied. Rejected because with entity counts this small, the ergonomic gain is
marginal, while snapshotting and hashing stay harder than for a plain struct.

**`Vec` instead of fixed arrays.** Simpler to write. Rejected because fixed
capacity means no allocation in the simulation, which removes allocator
behaviour as a variable, and because the caps are genuine game constraints
rather than arbitrary limits.

## Consequences

**Good.** Snapshotting is a clone of one struct. Hashing walks it field by
field. Update order is readable top to bottom. No allocation in the tick path.
The shape mirrors how the original almost certainly stored its state, which
makes comparing against a trace more direct. No dependency.

**Bad.** Adding a new kind of entity means editing `World` and its hash and
snapshot code, where an ECS would need no central change. Systems that only
touch one field still receive `&mut World`, so the compiler cannot prove
disjointness, which will need splitting borrows or passing narrower structs as
the code grows. Fixed capacities must be chosen and enforced. And if a later
title in the lineage genuinely needs many heterogeneous entities, this may need
revisiting.

**Revisit if:** profiling shows the simulation is a bottleneck and needs
parallelism, or a later title's entity model does not fit fixed arrays. Either
would justify a superseding ADR. Neither is true today, and adopting an ECS
speculatively would pay all the costs now for a benefit that may never arrive.
