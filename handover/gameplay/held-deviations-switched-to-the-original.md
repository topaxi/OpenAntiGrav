# The held deviations are switched to the original's laws: what is left

2026-10-21, lane `deviations-to-original`. The maintainer's decision (2026-10-20): "Original
shall always be the default". Every law this project held back from the original's on purpose
for our AI or an unread field now runs the original's:

| Law | Was | Now | Evidence |
| --- | --- | --- | --- |
| Pure bank-to-yaw | Pulse's `30`, chosen | `50`, measured (92) | [pure-physics.md](../../docs/physics/pure-physics.md#bank-to-yaw-on-2026-10-21-deviations-to-original) |
| Pure inertia frame | HD's rotated, chosen | Pulse's world axes, read (85) | [pure-physics.md](../../docs/physics/pure-physics.md#the-inertia-frame-is-pulses-world-axes-2026-10-21-deviations-to-original) |
| Pulse magstrip escape | held out, chosen | escapes, read and seen firing live on Vertica (92) | [airborne-ground-truth.md](../../docs/physics/airborne-ground-truth.md#a-magstrip-passes-the-gate-and-the-escape-fires-on-one-2026-10-21-deviations-to-original-pulse-magstrip-hold) |
| HD escape gate | `Floor` only, chosen | the mesh tag: walls, and magstrips below full lock (88 tag, 80 walls) | [hover-four-point.md](../../docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md#hit0x28-is-the-marchs-mesh-shape-tag-2026-10-21-deviations-to-original) |
| Zone turbo/roll-payout add | not ported | the shared tail (85) | [zone-start.md](../../docs/ghidra/functions/psp-pulse-usa/zone-start.md) |

2048's in-air bias was switched by the lane before (`8dc59c76e`).

## Open

- ~~**Our magnet hold loses the craft on `06_Track`.**~~ Closed 2026-10-21 (`pulse-magstrip-hold`):
  the crest escapes are the original's law
  ([airborne-ground-truth.md](../../docs/physics/airborne-ground-truth.md#on-verticas-crest-the-original-escapes-too-2026-10-21-pulse-magstrip-hold)).
- ~~**The `06_Track` RAPIER contact rise (`154` to `419`) as a driver item.**~~ Closed 2026-10-21
  (`ai-rejudge-after-switches`): not the crest. The contact is at samples 800-960 on runs where the
  lone Triakis followed the reference plan at its yaw share unchecked; that fallback is now verified
  on the hull and stepped down where it scrapes, and the row is back at `154`
  ([ai.md](../../docs/gameplay/ai.md#the-fallback-plan-is-verified-2026-10-21)). What it left:
  three of the row's 45 runs find no share that laps (a slow share stalls in the 1196-1200 gap);
  `14_Track` PHANTOM laps 1.6 s slower on a lower share with no less contact; route plans keep the
  unchecked fallback.
- **Pure scales its turbo add by `craft+0x288` while it is below `1.0`** (`0x0892f568`), on both
  engine branches; the field is unread and ours adds the full turbo on Pure.
- ~~**Pure's AI under bank-to-yaw `50`.**~~ Closed 2026-10-21 (`ai-rejudge-after-switches`): the
  noise bar on the combined state (bank-to-yaw `50` and the world-axis frame) is inside on every
  metric; the Ace walls (`k = 7`) are gone with the frame on. Laps `0.2`-`0.3` s longer (p
  `0.02`-`0.05`), the physics' own
  ([pure-physics.md](../../docs/physics/pure-physics.md#the-noise-bar-on-the-combined-state-2026-10-21-ai-rejudge-after-switches)).
  Left: an Ace grinding tail (two seeds at one torque, `2175` and `1617` wall ticks; the base has one
  at `1099`).
- **HD's escape off a wall was not seen live**: 247 probe hits, all floors. The class numbering
  (`0` wall, `2` reset) is read off two uses, not HD's loader.

## Next Steps

1. ~~`06_Track` hold~~ and ~~the 06 RAPIER driver item~~: done (above). A same-input number on the
   crest is still open: replay the captured `held` stream through ours (`psp-magstrip-escape.py`
   writes it per tick). About 2 hours.
2. Pure `craft+0x288`: find its writers in Pure's image (`psp-relocate.py`) and watch it across
   a Turbo in PPSSPP Pure. About an hour once a Pure state exists.
3. The verified fallback for route plans (`hull_plans_for`'s routes still follow the reference's
   route unchecked), and a share that clears `06_Track`'s short gap. Start from
   `crates/raceplay/tests/ai_contact_where.rs`. About half a day.
