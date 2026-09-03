# The race camera's FOV is fitted to the PSP's own aspect on every title, and the PS2's own in-game widening option has no counterpart

2026-09-03, found while chasing an apparent PS2-engine-flare size mismatch that turned out to be something else (see [The PS2's engine flare plays and reads as nothing on screen](the-ps2s-engine-flare-plays-and-reads-as.md)). Not a bug in the sense of contradicting a closed reading - `race::AUTHORED_ASPECT` was already closed once, deliberately, as "right whatever the disc" for the 2D/HUD/loading-screen layout math ([`ps2-front-end-layout-is-hardcoded-to-480x272.md`](ps2-front-end-layout-is-hardcoded-to-480x272.md), 2026-08-09/08-23). This is a narrower, still-open question that closure never asked: **should the in-race 3D camera also reproduce the PS2 disc's own runtime `Aspect Ratio` option**, and today it does not, on any title.

**What's confirmed, by source:**

- `crates/game/src/frontend/space.rs:41` - `pub const SCREEN: (f32, f32) = (480.0, 272.0)`, a single global constant.
- `crates/game/src/race.rs:246` - `pub const AUTHORED_ASPECT: f32 = crate::frontend::SCREEN.0 / crate::frontend::SCREEN.1`, fed straight from it.
- `crates/game/src/race/camera.rs`'s `projection()` fits every view's vertical FOV to `AUTHORED_ASPECT` via `oag_render::camera::fit_vertical_fov`, for **every title** - PSP, PS2, Pure, HD/Fury alike. The function's own doc comment is explicit that this is deliberate: `<ExternalCameraFar fov>` "is also authored for **one** viewport shape, the PSP's own, which is the only one the original ever renders into" (confidence 94, verified live against `g_camera_fov_degrees`) - but that confidence is about the *angle*, not about whether the PS2 disc reuses it unwidened at every setting, which was never separately checked.
- This project's own `--aspect psp|ps2|free` flag (`crate::display::Aspect`) is a **different** mechanism: it shapes the letterboxed **viewport** inside the window (`display::viewport`), which then feeds `fit_vertical_fov` as the `aspect` parameter - so choosing `ps2` (4:3) does change the fitted FOV, but by "constrain the presentation rectangle and fit the authored horizontal field into it," not by "multiply the camera's own aspect by 4/3," which is what the disc's option actually does (`docs/ps2/aspect-ratio.md`, confidence 95: "Setting `16:9` widens the 3D camera's aspect by `4/3` and changes nothing else"). The two are different operations that happen to both be called "aspect ratio," and nothing here checks whether they land on the same FOV.

**Not established:** whether the PS2 disc's own `<ExternalCameraFar fov>` value (or whatever the PS2 executable's equivalent global is) is actually the *same* authored angle as the PSP's, or a different one scaled for the PS2's own 4:3/16:9 option - the 94-confidence reading was taken on the PSP, and `g_camera_fov_degrees`'s PS2 counterpart has not been located or read.

## Open

- Whether the PS2 disc authors the same vertical FOV as the PSP, or a different one - unread on the PS2 executable.
- Whether this project's race camera should reproduce the PS2's own `Aspect Ratio` option (a genuine camera-aspect multiply, confidence 95 per `docs/ps2/aspect-ratio.md`) as a selectable in-race setting, separate from `--aspect`'s viewport-shaping role.
- What this does to any pixel-level comparison against a PS2 capture at an uncontrolled `Aspect Ratio` setting: the apparent on-screen size of anything camera-facing (the engine flare's billboard among them) is not comparable between the two builds without knowing which FOV each one actually rendered at.

## Next Steps

- Locate the PS2 executable's own camera-FOV global (the counterpart to `g_camera_fov_degrees`) and read whether it matches the PSP's authored angle.
- If a future pixel comparison against a PS2 capture needs the camera geometry to match, pin the PCSX2 profile's `Aspect Ratio` setting to a known value first and compute the matching `--aspect`/FOV on this side rather than assuming either default lines up.
