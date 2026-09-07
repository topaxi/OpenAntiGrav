# Two recovered chase-camera behaviours are ported; `headtilt` is not

`crates/render/src/camera/chase.rs` reproduces `Ship_UpdateCameraRigs` whole - rigid radius, `pos_height` after the spring, craft scale applied about the craft after the spring - measured against `data/traces/pad0-boost.csv` at **RMS 0.008 / max 0.060** world units against the shipped model's 4.938 / 6.824. Two things to carry forward. **Neither half was worth landing alone**: `pos_height` after the spring, on its own, is *worse* than what it replaced, which is the shape of a change anyone trying half of it would have reverted off a screenshot. And **a settled camera cannot see any of it** - all the models agree exactly at rest, so every static probe and every `--pose-from --ticks 0` screenshot this project ever took was blind to a 4.9-unit error. **A camera can only be judged while it is lagging.** `crates/game/tests/chase_camera_ground_truth.rs` pins it (`#[ignore]`d; needs a disc image *and* the capture). **Still open:** `<InternalCamera headtilt>` is parsed and not applied - the original rolls the view's up vector by `side * craft[0x844] * headtilt`. **`craft+0x844` is identified as of 2026-08-09 and this item is now a port, not research**: it is a smoothed steering lean, `steer * 0.01` through a rate-limited follower at `craft+0x848` and then a `4/s` first-order filter, saturating at 0.6 - so the craft leans *into* the turn and the arithmetic needs no new capture. Confidence 80 for the arithmetic, 0 for what the input's units are. **A wrong name was live in the Ghidra database on the producing function** - `0x0883fab4` carried `Ship_UpdateStartBoost`, which is really `0x0883fdec`, so two functions shared one name and a search for the start boost landed on a steering filter; reverted to `FUN_0883fab4` rather than renamed, since its two outputs feed two different subsystems. `names.tsv` was never wrong. Also `craft+0x790`'s **second** write site is now fully settled; it runs in an external view and the law is exact.

## Open

- `<InternalCamera headtilt>` is parsed but not applied to the view's up vector.
- The headtilt arithmetic has confidence 80, but confidence 0 for what the input's units are.

## Next Steps

- Apply the headtilt roll (`side * craft[0x844] * headtilt`) using the now-identified `craft+0x844` smoothed-steering-lean arithmetic - this is a port, not research.
- Pin down the units of the headtilt input (currently confidence 0).
