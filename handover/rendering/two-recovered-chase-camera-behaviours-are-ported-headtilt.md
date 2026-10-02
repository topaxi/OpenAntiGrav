# Two recovered chase-camera behaviours are ported; `headtilt` is not

`crates/render/src/camera/chase.rs` reproduces `Ship_UpdateCameraRigs` whole - rigid radius, `pos_height` after the spring, craft scale applied about the craft after the spring - measured against `data/traces/pad0-boost.csv` at **RMS 0.008 / max 0.060** world units against the shipped model's 4.938 / 6.824. Two things to carry forward. **Neither half was worth landing alone**: `pos_height` after the spring, on its own, is *worse* than what it replaced, which is the shape of a change anyone trying half of it would have reverted off a screenshot. And **a settled camera cannot see any of it** - all the models agree exactly at rest, so every static probe and every `--pose-from --ticks 0` screenshot this project ever took was blind to a 4.9-unit error. **A camera can only be judged while it is lagging.** `crates/game/tests/chase_camera_ground_truth.rs` pins it (`#[ignore]`d; needs a disc image *and* the capture). **`headtilt` landed 2026-10-02**, in the *internal* rig only - `craft+0x844` has no reader in the chase rigs, so the chase camera is untouched (RMS above still holds). `oag_physics::controls::update_camera_lean` ports `FUN_0883fab4`'s first filter (`entity+0x848` rate-limited follower, `entity+0x844` smoothed at 4/s; the `0.6`/`0.3` clamp the delta, not the lean) as unhashed ship state `camera_lean`, and `oag_render::camera::internal::view` rolls the up vector by `side * lean * headtilt`, the side row being the ship's left. Measured on PPSSPP: lean RMS 0.0003 over a 480-tick capture (`data/traces/talons-junction-steer-lean.csv`, pinned by `camera_lean_ground_truth.rs`). See `camera.md`, "`craft+0x844` is a smoothed steering lean".

**2026-10-01, default view:** a fresh original profile flies `OPT_CLOSE` (eye `(-11.25, +3.0)` in the craft's frame on two cold boots), so `CameraView::default()` is now `Close`; it had been `far`, a choice, and made our craft look 1.4x smaller than the original's. The rig itself was already right. See `camera.md`, "The default view is `OPT_CLOSE`". Not found: where the original writes that default into a fresh profile.

## Open

- The roll's sign and scale are supported (best fit at `+0.3 * lean` for a ship authoring 0.3) but 0.26 rad of the recorded tripod up is unexplained: `tgt += craft[0x810]` (a look-around offset) and the node rows behind `craft+0x374`/`+0x37c` are neither ported nor fitted. Confidence 70 on the roll, 90 on the lean.
- **The original shapes the analog stick before the record the lean and the steering read**: a scripted `stick_x=-0.5` settles at a record of about -11, ours is -50. A gap in the stick path, found here, not fixed here.
- Omitted, chosen not measured: the `+/-10` terms from `entity+0x8a4`/`0x8a8` and the negate on `entity+0x860 & 2`.
- The writer of `OPT_CLOSE` into a fresh profile (a literal, or a default argument of `Settings_GetString`'s caller) is not located; the measurement does not need it, a code-level confirmation would.
- Existing `settings.toml` files written by earlier builds say `camera_view = "far"` and keep it; not migrated, since a written default cannot be told from a choice.

## Next Steps

- Port the stick shaping (record from analog axis) once its law is read; it moves yaw at partial deflection as well as the lean.
- Read `tgt += craft[0x810]` and the node rows to close the 0.26 rad.
