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
  not a hold divergence. The "pinned" craft was one crest, the lip of Vertica's climbing magstrip
  wall (spline rows `2750`-`2768`), crossed about 280 times across the perturbed runs with 1-3
  escape ticks a pass, mostly on the derived rear probe. The original under player input escapes
  there too (VENOM 4 ticks a pass, heights to `-0.21`; RAPIER 2, to `-0.05`), and ours' probe
  heights agree within about `0.2` row by row
  ([airborne-ground-truth.md](../../docs/physics/airborne-ground-truth.md#on-verticas-crest-the-original-escapes-too-2026-10-21-pulse-magstrip-hold)).
  The `ai_clean_lap_gate` `06_Track` RAPIER contact rise (`154` to `419`) is a driver item on a
  faithful law; its BASELINE row stays. Left open: ours was compared against our AI's line and
  speed, not the captured player input replayed through ours.
- **Pure scales its turbo add by `craft+0x288` while it is below `1.0`** (`0x0892f568`), on both
  engine branches; the field is unread and ours adds the full turbo on Pure.
- **Pure's AI under bank-to-yaw `50`.** The noise bar found Ace race walls past noise (`k = 7`);
  unperturbed, the inertia switch made Pure's field cleaner than before the lane, but the bar was
  not rerun on the combined state.
- **HD's escape off a wall was not seen live**: 247 probe hits, all floors. The class numbering
  (`0` wall, `2` reset) is read off two uses, not HD's loader.

## Next Steps

1. ~~`06_Track` hold~~: done, the escape is the original's (above). If the driver item is
   picked up, the crest is rows `2750`-`2768`; a replay of the captured `held` stream through
   ours (`psp-magstrip-escape.py` writes it per tick) would turn "within about `0.2`" into a
   same-input number. About 2 hours.
2. Pure `craft+0x288`: find its writers in Pure's image (`psp-relocate.py`) and watch it across
   a Turbo in PPSSPP Pure. About an hour once a Pure state exists.
3. Re-run Pure's noise bar on the current tree (`pure-physics.md`, "The noise bar"), about 15
   minutes a state, before any Pure driver change.
