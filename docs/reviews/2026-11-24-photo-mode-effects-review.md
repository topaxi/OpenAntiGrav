# Photo mode effects review, 2026-11-24

A review pass over photo mode's effects: the picture controls, the accumulated depth of field and
motion blur shots, the rewind, the free lens, and what a saved shot carries. What the mode is
meant to do is [photo mode](../ui/photo-mode.md); this page is where it does something else.
Findings are graded with the rubric of [2026-08-18-code-review.md](2026-08-18-code-review.md).

Nothing was fixed on the way past; the fixes came after and each finding says where. Every finding has a test that was
written and run, a capture, or exact arithmetic. What could not be proven is listed apart, under
"Unconfirmed". Two reads were made, one after the other and blind to each other; "The second
read" says which found what.

## What was read

Read in full, on `main` at `455aeeb43`:

- `crates/post/src/photo_fx.rs`, `crates/post/shaders/photo_fx.wesl`, `crates/post/src/photo_fx/tests.rs`;
- `crates/raceplay/src/photo.rs`, `photo/effects.rs`, `photo/shot.rs`, `photo/rewind.rs`,
  `scene/photo_fx.rs`, and `tests/photo_rewind.rs`;
- `crates/render/src/camera/free.rs`;
- `crates/game/src/app/session/photo.rs`, `photo_rewind.rs`, `photo_meta.rs`, and
  `crates/game/src/app/from_png.rs`;
- `docs/ui/photo-mode.md` and `docs/formats/shot-png.md`.

Read in part, for the paths photo mode takes: `crates/raceplay/src/scene/frame.rs` (`finish_view`,
`prepare_view`), `crates/raceplay/src/camera.rs` (`view`, `projection`, `vertical_fov`),
`crates/game/src/app/session/draw.rs` (the race arm, the resolve, the HUD and the submit),
`crates/game/src/shot_meta.rs` (`Meta`, `flags`, `camera_of`), `crates/title/src/photo.rs`
(`Accumulation`), `crates/game/tests/photo_mode_ground_truth.rs` (the picture control and motion
blur tests).

Sampled only, by search: `crates/present/src/upscale.rs` (the presentation target a shot reads),
`crates/game/src/photo.rs` (the screen; its layout was out of scope), `crates/post/src/hd_zoom.rs`
(the speed blur's ring), the touch gestures. The temporal upscaler's own code was not read.

## What is already strong

- **The neutral case is structural, not numeric.** A control at its opening value is `None` or
  not `active()`, the pass is not built and nothing is encoded, so "byte-identical at neutral" does
  not depend on a round trip through the transfer curve. Both the unit test and the ground truth
  check it.
- **The depth linearisation matches the projection.** `eye_distance` inverts
  `z = far (d - near) / (d (far - near))`, the right-handed 0 to 1 perspective every race view is
  built with (`oag_core::math::camera::perspective` through `oag_render::camera::projection`, one
  builder), with the same `NEAR_PLANE` and the scene's own `far`. No view of a race uses the
  reversed projection.
- **The uniform is four `vec4`s on both sides**, 64 bytes, with the size asserted.
- **The accumulation does not overflow or drift.** 130 frames of 255 is 33,150, inside 16 bits,
  and the mean is rounded to nearest. Any input abandons the shot, so no control can change under
  it. (The sum holds up to 257 frames; nothing asserts a title's count stays under that.)
- **The live race comes back to the bit.** The rewind's snapshot, the set-aside recording and the
  pre-roll are covered tick by tick on a fixture and on discs.
- **The game's own motion blur is off while the mode is open**, read fresh each frame, with a
  ground-truth test that a shot is the same file at `high` as at `off`.
- **Per-title data is what the page says**: HD 7 and 120 frames, 2048 7 and 130, ship shutter
  `0.05` on both; Pulse, Pure and Omega accumulate nothing.

## Findings

Sizes are of a 1920x1080 picture unless a capture's own size is given. The captures are HD/Fury
EU, Time Trial, Anulpha Pass, tick 702, the `close` lens turned by `hold left 8`, 1440x816, debug
build, headless `--drive`, and Pulse EU, the default race, tick 400. The HD script is the motion
blur test's in `photo_mode_ground_truth.rs` as far as the mode. A test named here is in the tree,
`#[ignore = "photo-fx-review finding N"]`, and fails today when run with `OAG_REVIEW_FINDINGS=1`;
without it the test returns at once, because `just test-data` runs every ignored test and an open
finding must not turn the gate red. A fix removes the guard and the `#[ignore]` together.

### PF1. An accumulated depth of field shot is not the picture the preview shows

**Fixed** in `5e6674042`. Every accumulated frame now runs the preview's pass at the full aperture,
and the lens ring is sized so the band's ends move at most 1/4320 of the height and no depth more
than a quarter of the pass's blur there. The test passes; the two captures, rerun, give a saved shot
0.18 and 0.22 levels from the preview over the craft (detail kept 0.997 and 0.995). The default
shot is the same picture to the pixel. `a_lens_sample_frame_asks_the_pass_for_the_previews_blur`
fails if a lens-sample frame asks the pass for less blur than the preview again.

**Impact**: High - on HD and 2048 the saved shot differs from what the player framed, from a soft
craft at the opening values to seven ghost craft with the focus set far.
**Priority**: P1. **Confidence**: 95. **Faithfulness**: no. **Effort**: M.

The preview is the single-frame pass. It follows HD's curve: sharp from `d - c` to `d + 2c`, and
never wider than the aperture's widest blur (`crates/post/src/photo_fx.rs:50-57`,
`photo_fx.wesl:54-67`). On HD and 2048 the saved shot is seven frames drawn through a centre and
a ring of six lens samples (`crates/raceplay/src/photo/shot.rs:81-89`, `146-161`). A lens sample
is a thin lens: it holds the one distance `d` and moves a surface at `z` by the ring's radius
times `|1 - d/z|`. It has no sharp band and no widest blur, and seven samples are seven copies,
not a disc.

- **Test** `tests::photo_rewind::the_accumulated_shot_keeps_the_band_the_preview_shows_sharp`: at
  the opening aperture 40 and focus 20 (`d` = 11), a ring sample moves the near focus (7.26
  units) 2.78 px and the far focus (18.48 units) 2.19 px.
- **Capture, opening values.** Over the craft (`0.40..0.52` by `0.68..0.78` of the picture) the
  preview differs from the plain shot by a mean of 0.015 levels, at most 1. The saved shot differs
  from the preview by a mean of 2.52, at most 79, and 10.3 percent of the pixels by more than 8.
  Detail kept over the craft (mean step between neighbours): preview 1.000, saved 0.819. Read as
  a picture, the hull's edges and decals are doubled. Over the whole picture the saved shot holds
  0.75 of the preview's detail (1.214 against 1.614).
- **Capture, focus 100.** The preview blurs the craft softly. The saved shot is seven overlaid
  pictures of the craft and the road. Over the craft the two differ by a mean of 31.1 levels and
  98.1 percent of the pixels by more than 8; over the whole picture 52.6 percent.
- **Arithmetic for that capture.** `d` = 493 and the craft is about 12.4 units from the eye, so a
  ring sample moves it `0.5 * 0.025 * 0.4 * (493 - 12.4) / 12.4` = 0.19 of the height: 158 px of 816.
  The preview's widest blur at that aperture is 0.01 of the height, 8 px.

`photo-mode.md` says "the saved shot was not compared with the preview"; this is that comparison.
Reproduce: in the mode `press start`, `hold down 1` three times, `hold right 1`, (for the second
capture `hold down 1` twice and `hold right 260`), `press start; screenshot preview.png; press
cross; until photo_job=none within 800`.

### PF2. A lens sample turns the camera, so the focus plane is sharp only on the axis

**Fixed** in `5e6674042`: `LensShift::apply` shears the frustum (off-axis) instead of turning it.
The test passes, and checks four points of the focus plane under a 0.07-unit sample (half the preview's widest
blur at the opening aperture), which the rotation moved 6.9 px.

**Impact**: Medium - the same shot smears the plane of focus toward the frame's edge, more at a
wide field.
**Priority**: P2. **Confidence**: 95. **Faithfulness**: no. **Effort**: S.

`LensShift::apply` (`crates/raceplay/src/photo.rs:231-235`) moves the eye and then rotates the view
so the focus point is back on the axis. A rotation holds a sphere through the eye and the focus
point, not the plane at the focus distance. A point on that plane at tangent `u` off the axis
moves by about `u^2` times the sample's own shift. The existing test checks the axis only.

- **Test** `tests::photo_rewind::a_lens_sample_keeps_the_focus_plane_sharp_at_the_edge_of_the_frame`:
  `close` lens, field 65, 16:9, aperture 40. A point on the focus plane at the right edge moves
  6.92 px in the first ring sample.
- **Arithmetic**: the shift is linear in the aperture, so about 17 px at aperture 100. A point at
  infinity is meant to move 13.5 px there (half the widest blur), so at the edge the plane of
  focus smears more than the far field is meant to. At the free lens's widest field, 100 degrees,
  `u^2` at the edge is 4.49 against 1.28: about 3.5 times these figures.

A sheared (off-axis) projection holds the plane exactly; the rotation is the cheap stand-in.

### PF3. A saved shot does not record the picture controls, and `--from-png` redraws without them

**Impact**: Medium - the file exists for rendering bug reports, and a report about a graded or
blurred shot cannot be remade from it.
**Priority**: P2. **Confidence**: 95. **Faithfulness**: n.a. **Effort**: S.

`Meta` (`crates/game/src/shot_meta.rs:27-53`) has no field for exposure, saturation, speed blur,
depth of field, aperture, focus, motion blur, shutter or the number of frames, and
`Session::shot_pending` (`crates/game/src/app/session/photo_meta.rs:142-168`) writes none. `flags`
(`shot_meta.rs:230-271`) names none either. `from_png::prepare`
(`crates/game/src/app/from_png.rs:76-87`) restores the camera, the size and the visibility rule.
`[render] motion_blur` is the player's setting, while the mode drew with it off. `shot-png.md`
names the HUD as the one thing the text does not record.

- **Capture**: `--png-info` on PF1's first shot (depth of field on) prints no key for any control.
  `--from-png` on it prints `state hash at tick 702: MATCH` and draws a sharp frame: 29.2 percent
  of the pixels differ from the saved shot by more than 8, and the detail is 3.381 against 1.214.
  Nothing in the output says the picture was processed.

### PF4. Saturation 0 on Pulse leaves colour in the picture

**Fixed** in `c9c2c1778`: with a control on, `Scene::finish_view` adds the glow of Pulse, the PS2 and Pure to
the frame ahead of the pass, so the controls cover it as they cover HD's. On Pulse, at saturation 0 the worst pixel's
chroma on the same capture is at most 2 (92 before; PS2 and Pure take the same path, not run here); the Pulse ground truth now asserts no pixel keeps a chroma above 2
(it fails at 92 with the change dropped). With every control at its opening the picture is byte-identical to
`main`'s.

**Impact**: Medium - a black and white photo is not one on Pulse, PS2 and Pure. The glow escapes
every picture control there.
**Priority**: P2. **Confidence**: 85. **Faithfulness**: no. **Effort**: M.

On Pulse the glow is prepared from the frame before the pass runs
(`crates/raceplay/src/scene/frame.rs:241-267`, the pass at `289-291`) and added over the HUD
afterwards (`crates/game/src/app/session/draw.rs:803-808`; Pure's at `766-769`). `photo-mode.md`
says the exposure does not scale that glow and the depth of field does not blur it. It does not
say the saturation misses it, and the ground truth's bound hides it: it asks for a mean chroma
under a tenth of the neutral shot's.

- **Capture, Pulse**: at saturation 0 the mean chroma (largest channel less smallest) is 2.87
  against 28.85 at neutral, which passes that test. But 108,732 of 1,175,040 pixels (9.25
  percent) have a chroma above 8 and the largest is 92: an orange advert and a blue wash at the
  start gate, where the glow lands.
- **Control, HD**: the same steps on HD, whose glow is composited before the pass, give a chroma
  of 0 at every pixel.

The colour is put on the glow by the order of the passes and by the control, not by a run with
the glow off (no setting turns it off), hence 85.

### PF5. A blurred near surface is drawn almost opaque out to its blur radius

**Fixed** in `8a8cb1ffb`. Nearer surfaces that reach a pixel are a layer laid over its own circle's
average by the share of it they cover. The test passes and now checks the whole profile: 127 at the
edge outside and 135 inside it, 94 five pixels out, 0 from 22.

**Impact**: Medium - in the preview and in every single-frame shot, on every title, something
near and out of focus grows by its blur radius and keeps a hard edge.
**Priority**: P2. **Confidence**: 90. **Faithfulness**: no. **Effort**: M.

`gather` (`crates/post/shaders/photo_fx.wesl:72-99`) starts the sum at the centre with weight 1
and adds each tap whose own circle reaches the pixel with weight up to 1. A tap that does not
reach is left out of the sum **and** out of the weight. So on a sharp pixel beside a blurred near
surface, `k` taps on that surface give it `k / (k + 1)` of the pixel, whatever share of the circle
they are. The sharp pixel's own colour should stand for every tap that did not reach.

- **Test** `photo_fx::tests::a_blurred_near_edge_thins_out_over_the_sharp_surface_behind_it`: a
  white near field (1.5 units, a circle of 23 px) left of a black surface in focus, 1080 lines,
  full aperture. From the edge outward the black surface reads 242, 242, 240, 239, 239, 238, ...
  204, 170, 128, and 0 from 22 px on. Five pixels out, 36 percent of the circle lies on the near
  surface and the pixel is 238 of 255, not 92.

The far side is right: a tap behind the pixel is held to the pixel's own radius, and the far
field does not halo over a sharp craft (the existing test).

### PF6. The gather's 56 taps are spread over the widest reach, whatever the pixel's own radius

**Fixed** in `8a8cb1ffb`. An 8x8 tile pass and a dilation give each pixel the widest blur that can
reach it, the taps spread over that, and each tap is a 2x2 quad. Both tests pass: stripes under a
2 px circle are left at 33 of 255, under the widest blur at 2.

**Impact**: Low-Medium - a surface a little out of focus is not blurred at all, and at the wide
end fine detail comes through. Both get worse as the picture gets larger.
**Priority**: P2. **Confidence**: 90. **Faithfulness**: no. **Effort**: M.

The 56 taps sit on a spiral over `constants.depth.z`, the widest radius, rounded to whole pixels,
the same offsets at every pixel (`photo_fx.wesl:76-81`). The reach scales with the picture's
height (`crates/post/src/photo_fx.rs:152`) and the tap count does not. The innermost tap is
`sqrt(0.5 / 56)` = 0.0945 of the reach out: 2.55 px at 1080 lines and full aperture, 5.1 px at
2160. The unit tests run at 64 lines, where the reach is 1.6 px.

- **Test** `photo_fx::tests::a_surface_a_little_out_of_focus_is_blurred_at_1080_lines`: one-pixel
  stripes under a 2 px circle of confusion come out untouched, 255 of 255. No tap is inside the
  circle.
- **Test** `photo_fx::tests::the_widest_blur_at_1080_lines_flattens_one_pixel_stripes`: under the
  widest blur (27 px, a disc of about 2,290 pixels of which 56 are read) the stripes are left at
  a spread of 55 of 255. A disc average is flat.

The captured frames at aperture 40 and 816 lines (reach 8 px) read as smooth in the far field.

### PF7. The HUD in a motion blur shot is not the HUD on screen

**Impact**: Low-Medium - with the HUD row on, a blurred shot shows ghosted readouts and can show
a message that was not on screen.
**Priority**: P2. **Confidence**: 90. **Faithfulness**: n.a. **Effort**: S.

Each frame of a shot is read from the presentation target after the HUD is drawn
(`crates/game/src/app/session/draw.rs:780-802`, `photo_rewind.rs:318-351`), and each frame stands
at its own tick of the shutter. The HUD is drawn from that tick, so its readouts are averaged.

- **Capture**: HD, HUD row on, track blur, shutter 100 (15 ticks). The plain shot's lap clock
  reads `0.06.0` and its speed 160 KM/H. The blurred shot's clock is `0.05.8` with other digits
  ghosted behind the last two, its speed reads 158, and the thrust gauge reads 100 percent
  against 0 (the recorded input, not the paused pad). A text label beside LAP is in the blurred
  shot and not in the plain one; with the HUD row off the same shot has no label, so it is the
  HUD's. Which HUD element it is was not identified.

### PF8. The MOTION BLUR row is offered to a race that cannot draw one

**Fixed** in `93360d0ae`: `Race::photo_offer` tells the effects whether the race records, and without a
recording neither blur row is offered (a blur already chosen goes with it). The test runs unguarded and passes.

**Impact**: Low - with recording off the row and its shutter work, and the shot is silently one
frame.
**Priority**: P3. **Confidence**: 95. **Faithfulness**: n.a. **Effort**: S.

The row is offered when the title accumulates (`crates/game/src/app/session/photo.rs:185-192`,
`crates/raceplay/src/photo.rs:280-288`). A blur needs the recording
(`crates/raceplay/src/photo/shot.rs:71`). `photo-mode.md` says that with no recording "a motion
blur is not offered". `[replay] record = false` is the case.

- **Test** `tests::photo_rewind::a_race_with_no_recording_is_not_offered_a_motion_blur`: a race
  that records nothing, opened as HD. The row is on offer, track blur is chosen, and `Plan::new`
  returns no plan.

### PF9. The lens ring is sized for the lens's field, not the field the frame is drawn with

**Fixed** in `aae7b93ad`. The test's premise changed with PF1's fix (`5e6674042`): the ring is no longer half of
the preview's blur (`shot::ring_radius`, sized to the sharp band), so the test is restated: a ring sample moves
infinity the same share of the picture at rest and at speed. `Plan::new` takes the drawn field
(`Race::drawn_fov_degrees`); the restated test fails at 0.81 of the share with `photo.fov()` put back.

**Impact**: Low - at speed the accumulated half of the blur is narrower than intended.
**Priority**: P3. **Confidence**: 90. **Faithfulness**: no. **Effort**: S.

`Plan::new` sizes the ring from `photo.fov()` (`crates/raceplay/src/photo/shot.rs:83`). The frame
is drawn with the speed widen of the player's craft added to that field
(`crates/raceplay/src/camera.rs:897`), then the player's setting and the window's fit.

- **Test** `tests::photo_rewind::the_lens_ring_is_half_the_previews_blur_in_the_field_the_frame_is_drawn_with`:
  the lens says 65 degrees, the craft flies at 150 units a second, the frame is drawn at 76.25. A
  ring sample moves infinity 0.00406 of the height against the 0.005 it stands for: 0.81.

The same widen means the lens's stated range (10 to 100, 3 to 40 on `track`) is not the drawn
range. That is by design (the first frame is the one the pause left) and is not counted here.

### PF10. The free lens does not always start where it was seeded

**Fixed** in `76c997163` (the two proving tests run unguarded and pass): the free rig carries its own least field
(the `close` lens's 10, or the seed's own when narrower) and reach (`FREE_RANGE`, or the seed's own distance), which
`fly` and the touch path clamp to. **Not changed:** `Free::from_orientation` still brings a pitch past 1.5 rad
back to it, so a `close` lens at that pitch still jumps by up to 4.1 degrees on choosing `free`.

**Impact**: Low - the picture jumps on choosing `free` from a `track` lens zoomed in past 10
degrees, or from a camera more than 400 units from the craft.
**Priority**: P3. **Confidence**: 95. **Faithfulness**: n.a. **Effort**: S.

`seed_free` holds the seed's field to the `close` lens's range, 10 to 100
(`crates/raceplay/src/photo.rs:456-459`); the `track` lens zooms to 3. It does not hold the eye to
`FREE_RANGE`, and `fly` does on every tick (`photo.rs:704`), input or not. `photo-mode.md` says
the free lens starts from the camera the other lens put on screen.

- **Test** `photo::tests::the_free_lens_seeded_from_a_zoomed_track_lens_keeps_its_field`: the track
  lens at 3 degrees, then `free`: the field is 10.
- **Test** `photo::tests::the_free_lens_seeded_beyond_its_range_does_not_move_with_nothing_held`:
  a seed 450 units from the craft is drawn there once and moves 50.4 units on the next tick with
  nothing held. Whether any circuit's nearest `Camera` node is that far from the craft was not
  measured.

`Free::from_orientation` also brings a pitch past 1.5 rad back to it, which its own comment says;
the `close` lens can reach that pitch, so the same choice can jump by up to 4.1 degrees there.

### PF11. The pass builds two bind groups a frame and is kept until the race ends

**Fixed** in `34a018127`: the main pass's and the tile pass's groups are kept per scene and depth view
(`ViewGroup` keys on several views now), the copy group lives with the targets, and the scene drops the pass on the
first frame drawn after the mode closed. `a_frame_on_the_same_views_builds_no_bind_group` and
`the_picture_controls_pass_is_freed_when_photo_mode_ends` fail with either dropped.

**Impact**: Low - a steady stream of bind groups while a control is moved, and memory held after
the mode closes.
**Priority**: P3. **Confidence**: 85. **Faithfulness**: n.a. **Effort**: S.

`PhotoFx::render` calls `perfprobe::bind_group` twice on every call
(`crates/post/src/photo_fx.rs:320-351`). The scene view, the depth view, the buffer and the
scratch are stable between resizes, so both can be kept, as the motion blur keeps its own. Once
built, the pass is never released (`crates/raceplay/src/scene/photo_fx.rs:39-48`; nothing sets the
slot back to `None`): three pipelines and a scratch target of the render size, 33 MiB at
3840x2160, stay for the life of the scene after one visit. Read from the code and counted, not
measured on a device.

### PF12. `Constants::new` panics on a rectangle that starts past the target

**Fixed** in `34a018127`: the first pixel is held inside the target, and `render` skips a rectangle with no pixel in
it. The test runs unguarded and passes.

**Impact**: Low - not reachable today: the mode draws one whole view from the target's origin.
**Priority**: P3. **Confidence**: 95. **Faithfulness**: n.a. **Effort**: S.

`crates/post/src/photo_fx.rs:147-148` clamps the last pixel between the first and the target's
edge. With the first past the edge, `clamp` panics.

- **Test** `photo_fx::tests::a_rectangle_past_the_target_is_empty_and_does_not_panic`: a viewport
  at x 64 of a 64-wide target: `min > max. min = 64, max = 63`.

## Unconfirmed

Not proven, so not findings. Each says how to check it.

- **A window resize during a shot of several frames stalls it.** `photo_job_collect`
  (`crates/game/src/app/session/photo_rewind.rs:335-340`) adds a frame only when its size is the
  first frame's. A frame of another size is neither added nor counted, nothing cancels the shot,
  and a resize is not one of the inputs that abandon it. The progress bar would stand until a
  key is pressed. Both reads reached this from the code; neither ran it, because `--drive` has no
  resize step. Check: a windowed HD race, a 120-frame blur started, the window resized, and
  `photo_job` watched in the state log.
- **The temporal upscaler across a motion blur's frames.** The history is dropped between lens
  samples (`shot.rs:232-239`) and kept across a blur's frames, which are an eighth of a tick
  apart at the longest shutter. Whether the velocity the upscaler reprojects with is per tick or
  per frame there was not read. Check: the motion blur ground truth with `--reconstruction fsr3`
  against native, by the detail kept on the hull.
- **The speed blur's ring during an accumulated shot.** The ring's history steps with the mode's
  own tick (`crates/raceplay/src/photo.rs:383-385`), and the mode does not step while a shot is
  under way, so every frame of the shot is streaked from the history of the frame before the
  shot. Check: a depth of field shot with speed blur 100 against its preview at the frame's edge.

## Checked and not a defect

- **Blurs average display-encoded values.** The gather, and the mean of a shot's frames, work on
  encoded bytes; only the grade works in linear light. A white light that crosses a black pixel
  for a quarter of the shutter saves as 64, where a mean in linear light encodes to 137. The
  game's own motion blur averages the same way, and `photo_fx.wesl`'s header says so for the
  gather: the house convention and a choice, not a slip. The second read graded it high.
- **Lens samples when blur and depth of field are both on.** The samples are taken in turn, so
  each sees the whole shutter. On HD 120 frames give the centre 18 and each ring sample 17:
  balanced. On 2048 130 frames give three neighbouring ring samples one frame more, which moves
  the lens's centre by `2/130` of the ring's radius: 0.2 px at full aperture.
- **A shutter that reaches back past tick 0** draws tick 0 for those frames. The craft stood on
  the grid before the race began, so that is the right picture.
- **The shutter.** `(v * 0.01)^2 * 0.25` seconds is 15 ticks at 100 and 0.6 at the opening 20; the
  ship's `0.05` gives 0.75. The frames run from the shutter's start to the shot's tick in order.
- **Depth of field at aperture 0** draws the centre pixel alone, weight 1: the frame is unchanged.
- **A first frame of no pixels** would be counted and not summed, but the presentation target is
  never smaller than 1x1.

## Summary table

| ID | Finding | Impact | Priority | Confidence | Effort | Evidence | Found by |
| --- | --- | --- | --- | ---: | --- | --- | --- |
| PF1 | Accumulated depth of field is not the previewed picture (fixed, `5e6674042`) | High | P1 | 95 | M | test, two captures | both |
| PF2 | Lens sample rotates: the focus plane moves at the frame's edge (fixed, `5e6674042`) | Medium | P2 | 95 | S | test | both |
| PF3 | A shot does not record the picture controls; `--from-png` redraws without them | Medium | P2 | 95 | S | capture | both |
| PF4 | Saturation 0 leaves the glow's colour on Pulse (fixed, `c9c2c1778`) | Medium | P2 | 85 | M | capture, control | first |
| PF5 | A blurred near surface is nearly opaque out to its radius (fixed, `8a8cb1ffb`) | Medium | P2 | 90 | M | test | second |
| PF6 | 56 taps over the widest reach: small blurs untouched, wide ones leak (fixed, `8a8cb1ffb`) | Low-Med | P2 | 90 | M | two tests | both |
| PF7 | The HUD in a motion blur shot is averaged over the shutter | Low-Med | P2 | 90 | S | capture | first |
| PF8 | MOTION BLUR offered with no recording (fixed, `93360d0ae`) | Low | P3 | 95 | S | test | both |
| PF9 | Lens ring sized for the lens's field, not the drawn one (fixed, `aae7b93ad`) | Low | P3 | 90 | S | test | first |
| PF10 | The free lens does not always start where it was seeded (fixed, `76c997163`) | Low | P3 | 95 | S | two tests | both |
| PF11 | Two bind groups a frame; the pass is kept until the race ends (fixed, `34a018127`) | Low | P3 | 85 | S | code | first |
| PF12 | `Constants::new` panics on a rectangle past the target (fixed, `34a018127`) | Low | P3 | 95 | S | test | second |

## Suggested order

1. **PF1 and PF2 together**: they are the same seven frames. Decide what the accumulated shot is
   meant to be against the preview before touching the geometry. Until then the single-frame
   pass is the picture the player framed.
2. **PF5 and PF6 together**: one gather, its weights and where its taps go.
3. **PF3**: add the controls to the text and say in `--from-png`'s output when a shot was
   processed.
4. **PF7**: small and local (PF4, PF8 to PF12 are fixed).

## The second read

The first read is this lane's. The second is Gemini (`agy`, model `gemini-4-hydrogen-xhigh`),
read-only, given the file list and the claims to check and none of the first read's conclusions.
Its first run over the whole scope ended at its 1,150 s limit with no output. It was run again as
two narrower prompts (the pass, the lens and the plan; the session, the shot and the file), which
returned 22 lines, three of them "none found" (the uniform's layout, the rewind's restore, the
GPU resources). Each of the others was checked in the code, and by a test or a capture where it
became a finding.

| Second read's line | Verdict | Where |
| --- | --- | --- |
| The gather averages encoded values | True, a convention | "Checked and not a defect" |
| `Constants::new` panics past the target; NaN from an extreme exposure | First half confirmed by test; the sliders are integers 0 to 100, so no NaN | PF12 |
| The gather's centre weight against dropped taps: hard near halos | Confirmed by test | PF5, new |
| 56 taps over the full reach; small circles reach no tap | Confirmed by test; the wide end was the first read's | PF6, widened |
| `LensShift::apply` rotates instead of shearing | Confirmed by test | PF2, both |
| Centre and ring of equal weight; samples unbalanced against the blur's frames | True and negligible, 0.2 px | "Checked and not a defect" |
| `Plan::frame` piles frames on tick 0 | True and the right picture | "Checked and not a defect" |
| The ring matches the pass only at infinity; near surfaces spread far wider | Confirmed by test and capture; it led to PF1's second capture | PF1, both |
| `seed_free` does not hold the eye to the range `fly` holds | Confirmed by test; how far a node can be was not measured | PF10, new half |
| `seed_free` holds the field to 10 | Confirmed by test | PF10, both |
| A craft change and a lens change on one tick seed from the old craft's camera | The picture does not move: the eye is stored relative to the new craft | not a defect |
| A resize mid-shot stalls the job | Read the same way by both, not run | "Unconfirmed" |
| A first readback of no pixels | Not reachable, the target is at least 1x1 | "Checked and not a defect" |
| The file omits the picture controls and the HUD; `flags` too | Confirmed by capture | PF3, both |
| MOTION BLUR offered without a recording | Confirmed by test | PF8, both |
| The 16-bit sum overflows past 257 frames | True, not reachable with 120 and 130 | "What is already strong" |
| `ships[slot]` could index out of bounds | The subject is always a live slot | not a defect |
| `to_string_pretty` could panic on the render table | The text was written and read back on every capture here | not shown |

Found by the first read alone: PF4, PF7, PF9, PF11. By the second alone: PF5, PF12, and the
halves of PF6 and PF10 named above.
