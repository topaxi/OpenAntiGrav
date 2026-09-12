# An Ace craft should lap every circuit, in every speed class, without touching a wall

Opened 2026-09-12 as the umbrella the four narrower AI threads sit under. The
standard this thread is measured against, stated by the user:

> An elite/Ace AI should be able to drive every track perfectly when alone on
> the track, without wall contact, preferring airbrakes over slowing down, and
> for each speed class.

## Why a new thread rather than a step on an existing one

[outpost-7-loses-35-shield-a-lap-to-its-own-walls.md](outpost-7-loses-35-shield-a-lap-to-its-own-walls.md)
carries six completed steps of exactly this work on **one circuit at one speed
class**, and its step 6 names the residual: `corner_target`'s windowed
curvature understates the true local apex by 1.66x/1.85x even at
`Tuning::curvature_span = 11`. That thread stays scoped to `07_Track`; this one
is the board that says which *other* rows are in the same state, and it opens
three axes that have never been measured at all:

- **Speed class.** Every solo measurement this project has ever taken is at
  `VENOM` - `solo_lap_tuned` in `crates/game/tests/race_ground_truth.rs` hardcodes
  `class: "VENOM"`. `FLASH`, `RAPIER` and `PHANTOM` are unmeasured.
- **Wall contact as its own quantity.** `Solo` reports best clean lap, respawns,
  laps and `lost_at`. Shield is not in it, and where it has been read by hand it
  mixes roll charge and shield-pad pickups into the wall term - which is why
  `07_Track` was the only circuit whose wall attrition could be read at all.
- **Team.** `Tuning::max_turn_rate` is one global swept constant (1.8), while the
  yaw rate a hull actually achieves is `steer * Turning.amount / (5 * I_yy)` off
  per-craft authored data - 1.556 for the one team measured. Whether the AI's
  belief should be derived per craft is unread.

## The board

Per-circuit, per-class, lone Ace, `Mode::SingleRace`. Only rows with something
to report are kept; a clean row is a single line. Pulse PSP USA is the full
grid; Pure and HD are tracked only where a problem shows, and teams only on a
circuit already known to be a problem.

| title | circuit | class | team | clean lap | respawns | wall-contact ticks | shield/lap | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| _(empty - first pass has not run)_ | | | | | | | | |

## Open

- The board has never been measured at any class above `VENOM`
- Wall contact is not a measured quantity anywhere in the harness
- Whether `Tuning::max_turn_rate` should be derived from the flown craft's own
  `Turning.amount`/`I_yy` rather than a global 1.8 is unread, and it is the axis
  that decides whether the AI accommodates each team
- "Prefers airbrakes over slowing down" is unbuilt: `pace::throttle` cuts thrust
  to zero both in the margin band and past it

## Next Steps

- Parameterise the solo harness over `SpeedClass` and add a wall-contact count
- Fill the board on Pulse, all twelve forward circuits, all four classes
- Then attack rows in descending order of loss
