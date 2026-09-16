# ADR-0052: `World` and `Race::tick` widen to N players ahead of split screen, multi-window or network play

## Status

Accepted; extends [ADR-0003](0003-no-ecs.md).

## Context

A handover thread laid out three features - split screen, multiple OS windows,
network play - and found they share one prerequisite: the simulation has to
think in terms of N players feeding N inputs into one tick, instead of one.
That thread asked for this decision to become an ADR before implementation;
the implementation landed first (2026-09-16, five commits, `just` clean, ~80
disc-backed ground-truth
tests passing), and this ADR records the reasoning the thread asked for,
written from what shipped rather than from a plan.

Before this change the engine was **hard single-player**, not single-player by
default: `World::race` was one `RaceState`, `InputSnapshot` was a single struct
with no player index, `oag-input`'s `Pad` merged every attached gamepad into
one logical stream by design, and every read of "the player" in `oag-game` was
a literal `ships[0]` or `places()[0]`. `World::ships: [Ship; MAX_SHIPS]` already
held room for a full eight-craft grid - the grid itself was never the
constraint - but nothing beside it was indexed the same way for a *human*.

## Decision

Five changes, landing together as one prerequisite:

1. **`World::race` becomes `[RaceState; MAX_PLAYERS]`**, `MAX_PLAYERS` defined
   *as* `MAX_SHIPS` (not a second `8`) so `race[i]` and `ships[i]` can never
   drift apart. `ShipState::standing: oag_race::Standing` was already tracked
   per-craft for all eight slots - lap, place, finished - so only the
   *race-wide* state (clock, Zone counters, best lap) needed widening, not the
   per-craft placement math.
2. **`World::controllers: [Controller; MAX_PLAYERS]` answers a separate
   question**: `Ai` (default), `Local` or `Remote`. A slot's timing and a
   slot's pilot are different axes - every slot is now timed, but only some are
   human - so collapsing them (`has a RaceState` implying `is human`) would
   have made the array useless for ranking eight timed craft against each
   other. `Local`/`Remote` are distinguished because they differ in what the
   simulation is handed (a device this frame vs. a wire, possibly a
   prediction to be replayed), which matters to a reconciliation pass that
   cannot ask the input layer, since gameplay crates may not depend on it.
3. **`Race::tick` takes `oag_gameplay::PlayerInputs`** - a fixed
   `[InputSnapshot; MAX_PLAYERS]` - instead of one snapshot, indexed the same
   way as `World::race`/`ships`/`controllers` so there is one index and no
   mapping table to keep in step. The player *step* still runs once, for
   `World::primary_slot`, and returns one `Evaluated`: stepping two human
   craft a tick means N cameras, N `Evaluated`s and N HUDs, which is split
   screen's own scope, and doing it here would not have kept this change
   behaviour-preserving. The loop bound is now the only thing left to change.
4. **`oag_input::pad::Assignment` maps each device to a player slot**,
   replacing `Pad`'s "every pad is one merged stream" design. It is a `Vec` of
   `(GamepadId, slot)` pairs, not a map: which slot a device drives reaches
   simulation state, and a `HashMap`'s iteration order is not stable per
   process - the same determinism concern [ADR-0002](0002-determinism-model.md)
   and [ADR-0003](0003-no-ecs.md) raise about `World` itself, extended here to
   the table that decides how a device's input *becomes* an `InputSnapshot`.
   `Controls::buttons` became `[Input; MAX_PLAYERS]` for the same reason: an
   edge is per pilot, and `pressed = held & !held_last` over one shared
   `Input` would let one player's release clear another's unconsumed press.
5. **`oag-headless-sim`, a second `[[bin]]` in the `oag-game` crate**, ticks
   `Race`/`RaceSim` through the existing `[lib] oag_game` with no renderer, no
   window and no disc attached - proof the tick loop has no hidden coupling to
   any of the three. It is a second binary in this crate, not a crate of its
   own, because workspace-layout's rule 2 forbids any crate depending on
   `oag-game`: a future headless server cannot simply be some other crate
   importing `oag_game::race`. `oag-trace` hit the identical wall earlier and
   chose to duplicate the logic it needed (`locate` in
   `crates/trace/src/replay.rs`) rather than depend on the composition root -
   this is the same choice, made the cheap way a same-crate binary allows,
   over hoisting `RaceSim`/`Race::tick` into a new gameplay-side crate.
   `Setup::headless` is its disc-free input: every field is its own type's
   empty value, spelled out rather than defaulted, because a `Setup` that
   invented a plausible track is exactly the stand-in this project's "never
   invent what the assets already author" rule forbids.

**Behaviour is unchanged by all five.** `World::SINGLE_PLAYER` (one `Local` in
slot 0, seven `Ai`) is what `World::new` installs - the grid the engine
already had hard-coded, now as data - and the default `pad::Assignment` puts
every device on slot 0, matching the old merged-stream read exactly. Every
HUD/lap/Zone read that used to say `ships[0]`/`places()[0]` now goes through
`World::primary_slot`/`Race::player_slot`/`Race::player_standing`, which
resolve to `0` under the default, so a normal single-player race reads exactly
what it read before.

**The state hash moved, following this project's own isolate-then-verify
protocol for a legitimate constant change**: with the new fields' writes
stubbed out and nothing else changed, `crates/gameplay/tests/determinism.rs`'s
old reference constants reproduced bit for bit at both tick counts for both
scenarios - proof the movement was new bytes in the stream, not a behaviour
change - before the constants were updated, with both history entries dated
and recording the old values. `crates/physics/tests/determinism.rs` did not
move, which is the expected shape: `ShipState` gained nothing. The new
constants were captured on Linux/x86-64 only; CI's three-OS `determinism` job
is the one surface this ADR could not check before landing.

## Alternatives considered

**`race: Vec<RaceState>` instead of a fixed array.** Rejected on
[ADR-0003](0003-no-ecs.md)'s own grounds: a fixed array keeps `World`
snapshot-as-one-`memcpy`-shaped and puts no allocation in the simulation. A
`Vec` would also need a length invariant kept in step with `ships`/`ship_count`
by convention instead of by the type system.

**Fold `Controller` into `RaceState` itself**, one field instead of two arrays.
Rejected: a caller asking "is this slot human" would have to reach into
race-timing state to answer a question about *who*, not *when*, and the two
already change on different schedules - a controller assignment is set once
per race, a `RaceState` updates every tick.

**A `HashMap<GamepadId, usize>` for `pad::Assignment`.** Rejected for the
determinism reason [ADR-0002](0002-determinism-model.md)/[ADR-0003](0003-no-ecs.md)
already establish for simulation state, extended here even though
`pad::Assignment` itself lives in `oag-input`, not `oag-gameplay`: which slot a
device drives still reaches the `InputSnapshot`s the simulation consumes, so
an unstable iteration order is a hazard at one remove rather than none.

**Step every human slot's craft in this change**, rather than deferring the
loop bound to split screen. Rejected: `Evaluated` is one camera's worth of
physics-evaluation detail, and restructuring it to N results, N cameras and N
HUDs is real, separate work with its own design questions (see the thread's
open item on remote-player prediction vs. rollback) - bundling it here would
have made this change something other than a behaviour-preserving prerequisite.

**A new crate (`oag-net` or otherwise) depending on `Race`/`RaceSim` for a
future server.** Blocked outright by workspace-layout's rule 2. The two live
options were hoisting `RaceSim`/`Race::tick` out of `oag-game` into a
gameplay-side crate, or a second `[[bin]]` inside `oag-game` itself reusing the
existing `[lib]`. The hoist is real, disruptive restructuring with no consumer
yet to justify it; the second binary needed no new dependency edge and bent no
rule, so it was chosen first, matching `oag-trace`'s own precedent of working
beside rule 2 rather than against it.

## Consequences

**Good.** Split screen, multi-window and network play can each now be built as
additions - a viewport/camera-region layer, a per-window surface, a transport -
rather than needing further changes to the simulation's own shape. The
already-placeholdered `oag-replay` (M7) is unblocked for the same reason: it
needs the identical per-player input/state shape a recording format would.
`Standing` already being correct per-craft for all eight slots meant this
change was narrower than "add N of everything" would have been - only the
race-wide clock needed to become an array.

**Bad, and worth stating plainly.**

*`RaceState::mode`/`laps_target` are race-wide fields, now replicated eight
times with only slot 0 authoritative.* A known wart, not an oversight -
funnelled through `World::mode()`/`World::laps_target()` so there is one
reader rather than sixty, but hoisting them out into something genuinely
shared would mean splitting `oag-race`'s own recovered `RaceState`, which was
out of scope for this change.

*The player step still runs once.* `Race::tick` accepts N inputs but only
`primary_slot` is ever stepped, so nothing in this codebase has yet exercised
what happens when two human-controlled slots are stepped in the same tick -
that is exactly what split screen's own work has to prove, not something this
ADR can claim in advance.

*No ADR existed for this decision before the code did.* The thread that asked
for one scoped the implementing pass to five refactors, not a design document,
so the reasoning shipped as doc comments on `World::race`, `Controller` and
`PlayerInputs` first and this ADR second. The decisions were made and tested
either way; this ADR is the record catching up to them, not the other way
round this project's methodology otherwise prefers.

*Determinism constants moved, and only one platform confirmed it.* The
isolation proof shows the movement is new bytes and not new behaviour on the
machine that ran it. CI's three-OS `determinism` job is what confirms the same
holds on Windows and macOS, and had not run against this change as of writing.

*The server-authoritative/client-prediction/reconciliation network design from
the handover thread is still just that - a design, not a decision recorded
here.* This ADR covers the shared prerequisite only. The transport, the
message format, and the still-open remote-player-prediction question all
remain for whichever ADR follows when `oag-net` is actually shaped.
