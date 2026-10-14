# ADR-0069: Remote craft are predicted in one timeline and corrected on a threshold

## Status

**Proposed (2026-10-14). Not accepted: the maintainer decides.** Relates to
[ADR-0007](0007-fixed-timestep-vs-original.md) (a fixed 60 Hz step),
[ADR-0002](0002-determinism-model.md) (determinism),
[ADR-0052](0052-world-and-race-tick-widen-to-n-players.md) (N players in one world)
and [ADR-0055](0055-replays-are-inputs-and-a-ghost-is-poses.md) (inputs are the truth).
Evidence: `docs/networking/README.md`, "Reconciliation cost", "Remote-input
predictability" and "What a correction looks like".

## Context

Online play is server-authoritative with client prediction and replay-based
reconciliation (decisions 1 to 6 of `docs/networking/README.md`). One question was
left open and gated on measurement: draw the other players' craft in the past,
interpolated between snapshots (option A), or simulate every craft in the present
and roll back on a difference (option B). A racing pack changes a craft's motion on
contact every tick, so A resolves a contact against a state about a round trip
stale. Rocket League's own GDC 2018 slides tried interpolating the ball, found the
car and the ball in two timelines that disagreed on a hit, and shipped
predict-everything.

## Decision (proposed)

1. **Option B.** The client simulates every craft in one present timeline: its own
   craft on its real input, each remote craft on that craft's last input it has
   heard of. The local craft and the remote craft are in the same `RaceSim`, so a
   contact resolves in the present on every client.
2. **Correct on a state threshold, not on every difference.** A client restores the
   server's state and replays the ticks since (`Race::reconcile`, through
   `Race::tick_sim_only`) only when some craft is more than a distance `d` from where
   the server has it. `d` is about 1 world unit on the measured data (half a tick of
   travel at 120 units/s), **chosen, not measured**; a larger distance is cheaper and
   shows more.
3. **The server buffers each client's input**, consumes one per tick, throttles the
   client up or down, and repeats the previous input on an empty buffer (the
   Rocket League design). No client input delay by default.
4. **The correction is smoothed in the renderer only**, for a remote person's craft: a
   view-only offset that decays (half-life 6 ticks, chosen), never in the
   simulation, never in the hash. A correction past a snap distance (20 units) is
   drawn where it lands.
5. **Interpolation is the fallback** for a stalled link or a craft beyond the replay
   window cap, not a second mechanism for ordinary play.

## Why

Measured on the desktop (Pulse, 12 circuits, 8 craft; `docs/networking/README.md`):

- A replay is cheap: a 6-tick rollback is 0.42 ms, 2.5 percent of a frame.
- Last-input prediction is good for keyboard input (2.6 to 4.8 strict corrections a
  second) and a storm for continuous input under the strict policy (53 a second for
  seven remote craft, 11.6 times the simulation at 200 ms), which is why decision 2
  names a threshold. At 1 unit the same pack needs 1.4 to 1.8 corrections a second at
  1.07 to 1.36 times the simulation, with a remote error under one tick of travel.
- Only discrete events make a large error: a sideshift (about 4.5 units), contact,
  a respawn. The smoothing makes the first a glide; the second and third are drawn
  where they land.

## Conditions not yet met

The ADR stays Proposed until these are answered:

1. **The Galaxy S24 cost** of the replay. The proposed pass condition is a 100 ms
   round trip costing at most 25 percent of a frame; the strict policy at 100 ms is
   exactly that if the phone is 10 times slower than the desktop, a threshold policy
   is a tenth of it. Unmeasured.
2. **Weapons** (option D, weapons as server-confirmed events). A replayed input
   stream never fires, so every number above has contact and walls and no missiles. A
   missile that pops into view after a rollback, or a dodge on screen undone by one,
   is untested.
3. **An analog human stream.** The only human input recorded is keyboard (three
   `stick_x` values); the continuous case is proxied by a driver, which is more
   reactive than a hand. The 53-a-second figure is an upper proxy.
4. **The own craft's correction.** It moves the camera and is not smoothed; smoothing
   it needs a camera offset too.

## Consequences

- The simulation must be bit-exact on every client including modded rules; the
  content fingerprint must match to join (decision 6).
- `RaceSim` is snapshotted on demand and restored from the server's state, not every
  tick (a snapshot costs as much as a tick).
- The threshold, the half-life and the snap distance are tuning knobs with no
  original behind them.
