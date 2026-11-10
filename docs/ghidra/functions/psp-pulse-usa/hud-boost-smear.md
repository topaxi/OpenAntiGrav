# A speed pad's boost on the HUD: every widget drawn n times, and a glow stamp under the speed bar

**Binary:** `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`. **Read 2026-10-21** (`pulse-hud-boost-glow`):
two GE lists of boost frames, the framebuffer's alpha read out of EDRAM, three disassemblies and live
PPSSPP reads (v1.20.4, software renderer) of the widgets' own fields.

**The brief's claim, "a screen-space pass over the HUD whose strength follows speed or boost": false as
worded.** The GE list of a glowing frame ends HUD widgets, the bloom's bright pass, its two blurs and its
composite, and nothing else. What the player sees is two things the HUD's own draw code does while a
speed pad's boost runs: it draws every widget several times, and it stamps the bloom's glow mask under the
speed bar's fill, so the bloom the frame already has blooms the bar. It follows the **0.8 s boost timer's
triangle**, not the speed: a craft at 504 km/h with no boost draws a crisp HUD, and one that has just
crossed a pad at 540 km/h draws the smear and the glow.

| Address | Name | Kind | Conf |
| --- | --- | --- | ---: |
| `0x0881bf50` | `Hud_Update` | function | 80 (count: this page) |
| `0x0881a3e0` | `Hud_UpdateSpeedBar` | function | 88 (stamp: this page) |
| `0x088a7a5c` | `Image_Draw` | function | 80 |
| `0x088ce668` | `Text_Draw` | function | 85 |

## The count

`Hud_Update` (`0x0881c2cc`-`0x0881c3b0`) reads the player's exhaust flare, `*(craft+0x78)` with `craft =
*(g_race_manager + 0x2c0)`: `+0xb4` is the boost length (`0.8`, written by the constructor) and `+0xb8` the
boost timer, armed to `0.8` by a speed pad (`pads.md`) and by a perfect start (`perfect-start.md`) and counted
down by `Exhaust_Update`. It stores the same integer at `*(hud+0x274)+0xa8` and `*(hud+0x278)+0xa8`:

```text
if flare+0xb8 == 0:  n = 1
else:                r = clamp(flare+0xb8 / flare+0xb4, 0, 1)
                     w = r <= 0.5 ? 2r : 2(1 - r)          a triangle, 0 at both ends, 1 at the middle
                     n = trunc(10 * w)                      (lui 0x4120 = 10.0)
```

Live, 150 consecutive `Hud_Update` hits from a boost 0.43 s old: `n` read `9, 9, 8, ... 2, 1` as the timer ran
`0.366 -> 0` in steps of `0.01666`, and `1` for every frame after; a write watchpoint on `+0xa8` over 25
frames named only the two store sites (`0x0881c3b0` the count, `0x0881c2e8` the idle `1`).

## The copies

Both widget draws read the count and draw their widget that many times:

- `Image_Draw` (`0x088a7a5c`, the Image widget's draw slot, vtable `0x08acce34 + 0x44`): `s2 = max(1, n)` when
  the widget's ghost vector (`+0x114`, `+0x118`) is not both zero. Every vertex colour's alpha is divided by
  `s2`, and the strip is drawn `s2` times (`0x088a8688`-`0x088a86f8`), copy `i` after `Gu_Offset(trunc(1808 -
  dx*i), trunc(1912 - dy*i))` - `1808, 1912` is `2048 - 240, 2048 - 136`, the PSP screen's own offset, so a
  copy lands `1808 - trunc(1808 - dx*i)` whole pixels along the vector: one step is a pixel, `ceil` for a
  positive component and `-floor` for a negative one (truncation, not rounding).
- `Text_Draw` (`0x088ce668`): the same count and the same alpha division, one ordinary draw of the string
  first and then `n` copies (`0x088cf56c`-`0x088cf5d0`), only while the HUD's shake translation (`*(hud+0x274)
  + 0x3c/+0x40`) is exactly zero, `flags & 4` is clear and the global `0x08ab0818` is non-zero. The direction
  is a table of 32 unit vectors (cosines at `0x08ab1c78`, sines at `0x08ab1cf8`) indexed by a byte at
  `widget+0x172`.
- The direction of an Image is the unit vector from the screen's centre `(240, 136)` to the widget's top-left
  corner: three widgets read live, `SpeedBar` at `(306, 220)` `(0.618, 0.786)`, `ShieldBar` at `(306, 246)`
  `(0.514, 0.857)`, the speed bar's background `(0.617, 0.787)` - each equal to `(x - 240, y - 136)`
  normalised. Four others read live were not reproduced by that rule (a marker at `(0.453, 0.891)`, one at
  `(-0.432, -0.902)`): their vector was written when the widget was at another place.

**On the frame** (`t47` in the run's scratch, a GE list at `n = 2`): every HUD prim of the frame appears twice
at identical size, the speed bar's quads with vertex alpha `0x7f` and `0x30` where the rest frames carry `0xff`
and `0x60` (`trunc(255 / 2)`, `trunc(96 / 2)`); each string appears six times where it appears twice at rest
(an outline and a fill, times three).

## The glow stamp

`Image_Draw` also reads `widget+0xf4` (`0x088a8644`-`0x088a8684`): when positive it opens the alpha write
(`Gu_PixelMask(0)`), enables the stencil test with `KEEP, KEEP, REPLACE` and `ALWAYS`, reference `+0xf4`, for its
draws, and masks alpha again after. That is the same way the exhaust ribbon stamps the glow mask
(`bloom.md`). `Hud_UpdateSpeedBar` writes the speed bar's fill widget's `+0xf4` (`0x0881a4e4`-`0x0881a530`):

```text
n = *(hud+0x274) + 0xa8
SpeedBar.+0xf4 = n >= 2 ? trunc(n / 10 * 255) : 0
```

- Read out of EDRAM (`psp-trace.py --edram-every`, the framebuffer's alpha channel) in the frames a boost
  was running: **`76` (`n = 3`) and `51` (`n = 2`)** under exactly the fill's rectangle (`x 305..436, y
  219..246`, the width the fill had that tick) and the scene's baseline `4` everywhere else, the text and the
  shield bar included. A frame's GE list at the same moment carries `STENCILTEST 0xff3301` (`0x33 = 51`) and
  stencil op `REPLACE` (`0x20000`) on the fill's two quads and only there.
- The bloom (`bloom.md`) runs after every HUD widget, so its bright pass multiplies the bar's colour by the
  stamp: at the peak (`n = 10`) the mask is `255`, a bar of `(125, 239, 192)` bright-passes to `(125, 239, 192)`
  and the blurs and composite add `+40 .. +65` over it. The bar saturates to white with a halo past its tip,
  which is what the maintainer saw ("the bar is too far out on the right for it to be the boost bloom
  alone": the copies spread right and down, and the blur's reach is five texels of 240).

## Ours

`oag_hud::boost_smear` (`crates/hud/src/boost_smear.rs`) is the count, the stamp and the copies:
`Readout::boost_phase` (the flare's `boost_timer / BOOST_SECONDS`, `oag_raceplay::telemetry`) through
`draw_list`, driven by `oag_title::HudArt::boost_smear` (`oag_pulse::hud::ART`, shared by the PSP and PS2
pressings). `Frame::glow` carries the speed bar's stamped rectangles; `oag_game::hud_views::draw` hands them
to `Scene::stamp_glow` and `oag_post::bloom::Bloom::stamp_glow`, which runs the bloom's bright pass, two blurs
and composite over those rectangles alone in a lane of its own (the bright pass reads the HUD's target under
each rectangle) and adds the result onto the HUD's target. The scene's bright pass ran before the HUD existed
in our order, so it cannot see the bar; a chain over the rectangles is what its read of the HUD comes to,
because the scene's own mask under the bar is the baseline `4` and truncates to nothing in the original's blurs. A frame with no boost is byte-identical.

**Measured against the original** (`talons-alongside`, Single Race, Assegai, `scripts/psp-pulse-pair.py --field
--run-ticks 28`, the flare's timer read at the pose's stop as `0.457` of its length, `n = 9`; ours posed with
`--hud-boost-phase 0.457028`): RMSE against the original's frame, ours without the feature -> with it, speed bar
`105.7 -> 18.7`, the whole bar block (speed and shield) `86.6 -> 54.1`, the lap-time text block `45.6 -> 22.5`.
The shield bar is red in the original's frame (a state not reproduced, left).

**At rest** (`pulse-psp/talons-white`, every Pulse PSP grid pair is a frame with no boost): the HUD crops are
byte-identical to a build without the change; the only difference between the two frames is the craft's plume,
where `main` moved on from the binary compared. A timer of zero draws the plain list
(`a_frame_with_no_boost_is_the_plain_frame`), and `--hud-boost-phase 0` equals no flag byte for byte
(`hud_boost_glow_ground_truth`).

## Measured, chosen, open

- **Measured:** the count, its triangle, the `/ 10` stamp, the copies' alpha division and truncated shifts, the
  Image direction on three widgets, the stamp's value and rectangle in EDRAM.
- **Chosen, not measured:** a text widget's direction is the line from the centre to its anchor (the original
  quantises it to one of 32 table entries by a byte whose writer was not read); every draw of the frame is
  smeared, the lock-on reticle included; and for a split screen the stamps take the whole target. (The glow chain
  used to add the bar's colour alone; it now reads the HUD's target under each stamped copy, so a half-covered copy
  brings the scene's colour in, as the original's bright pass does: `weapon-absorb-pairs.md`, "Pulse: the stamp's
  bright pass reads the framebuffer", speed bar RMSE `19.1 -> 14.7` on the boosting pair.)
- **Open:** the shake gate of the text copies (`+0x3c/+0x40`) is not modelled because ours has no shake; the
  shield bar's colour in the glow frame; a frame at the boost's start and end (`n = 0, 1`) against the original;
  whether a `Model` widget smears: moot on Pulse, whose pickup icon is an Image (last section).
- **The boost timer and its trigger are ours already**: `Exhaust::boost_timer` runs from a pad hit and a perfect
  start with their own tests (`crates/raceplay/src/tests/pads.rs`).

## Other titles

- **Pulse PS2: checked, applies, copies wired through the shared `oag_pulse::hud::ART`; the glow stamp is not
  wired** (the PS2's bloom is `Ps2Bloom`, which has no stamp pass; `Scene::stamp_glow` is the PSP chain's).
- **Pure: checked, applies, not wired.** One widget draw (`FUN_088c9ce8`, `0x088c9dec`) carries the same
  `lui 0x44e2` copy loop; Pure's `Hud_Update` and the count's source are unread, so `boost_smear` stays `None`.
- **HD and Omega: checked, differs.** HD's boost-driven pass is `FunkLayerZoom`, a radial zoom ring over the
  screen edge for about a second after a boost starts ([funklayer-zoom.md](../ps3-hdfury-eu/funklayer-zoom.md));
  its HUD draws once.
- **2048: not checked** (a Vita HUD of HD's lineage; no frame at a boost).

## `Image_Draw`'s glow-mask stamp (2026-10-29, `pulse-glow-rest`)

`Image_Draw` (`0x088a7a5c`) ends its draw array with a stencil stamp: when the field at `param_1 + 0xf4` is above zero it runs
`Gu_PixelMask(0)`, `Gu_Enable(3)`, `Gu_StencilOp(0, 0, 2)` (KEEP, KEEP, REPLACE) and `Gu_StencilFunc(1, *(param_1 + 0xf4), 0xff)`
before the `Gu_DrawArray(4, 0x19f, ...)` over the widget's vertices, then `Gu_PixelMask(0xff000000)` and `Gu_Disable(3)` after. The
field is `RealGlow` (`Image_ParseValues` (`0x088a6f38`, confidence 75: a table of attribute names and destination offsets, `Text_ParseValues`'s shape) passes `param_1 + 0xf4` as the destination of the
`"RealGlow"` string at `0x08a7f294`, next to `Color1..4`, `DeltaWidth`, `Centred`, `AutoLoad`). So an `Image` stamps the authored
value as written, not scaled by alpha (`Text_Draw` scales). Confidence 80 (decompile; the stamp's quad, the whole vertex array, is
read from the draw call). `FUN_0889812c` (the other `Image` draw, flag `+0x2c & 0x200`) does not read `+0xf4`.


## `Mode3D_DrawQueued` has no copy loop (2026-11-02, `pulse-boost-hud-3d`)

**The earlier reading "`Mode3D_DrawQueued` (`0x088b8548`) carries the same `lui 0x44e2` copy loop at `0x088b8748`" is
false** (confidence 90: full disassembly and decompile read). `0x44e20000` and `0x44ef0000` are `1808.0` and
`1912.0`, the GE screen-offset base (`2048 - 240`, `2048 - 136`), which `Image_Draw` also uses but there inside a
per-copy loop. In `Mode3D_DrawQueued` the same constants are the argument of one `Gu_SetOffset` call
(`0x088b8748`-`0x088b8778`): `Gu_SetOffset(trunc(widget+0x9c + 1808.0), trunc(widget+0xa0 + 1912.0))`, i.e. the
`Mode3D` widget's `OriginX`/`OriginY` shifting the viewport, then `b 0x088b8870`. There is no branch back, no
counter and no load of the HUD's copy count (`*(hud+0x274)+0xa8`); the function sets the projection and view
matrices (`Gu_SetMatrix`, the orthographic branch when `widget+0x94 == 1`), the offset, and ends with
`ViewCull_BuildViewProjection(64.0)`. It draws nothing itself, so it cannot draw anything several times.

Scan of the 44 functions in `0x088b7000`-`0x088bb000` (the `Mode3D` and `Model` widget code) for a load at
`0x274(` or `0xa8(`: no function loads the HUD pointer's count; the `0xa8` hits are the `Mode3D` widget's `farZ`
(`Mode3D_DrawQueued`, `Mode3D_EnterView`), a transform field (`Model_BuildTransform`) and a byte of an unrelated
widget. So nothing in the 3D widget code reads the boost count. The `Model` widget's draw itself (the `Vex`
renderer behind it) was not read, and no boost GE list was inspected for a model's prims, so "the 3D widgets draw
once" is **unmeasured, not measured** (our 3D widgets - pickup icon, countdown, sights - are drawn once). Pure's
`0x088c9dec` `lui 0x44e2` hit, cited above, should be re-read the same way before it is trusted as a copy loop.

## The pickup icon at a boost: Image widgets, drawn n times (2026-11-02, `pulse-model-widget-smear`)

**The question** (the lane's claim): is the pickup icon, taken as a Model HUD widget drawn through its own Vex model, drawn once or as
offset copies during a speed-pad boost at `n >= 2`? **Falsifier written before the capture:** the icon's geometry appears once in a
boost GE list as it does at rest (drawn once), or appears `n` times (smears).

**Result: the pickup icon is not a model, and it smears.** Confidence 90 (runtime trace: three GE lists at `n = 1, 6, 10`, the
count read live; one boot, one track, one held pickup, so seen once on the boot axis; no second binary). The hexagon the player sees
is 13 distinct 4-vertex `0x19f` quads, the vertex type of `Image_Draw`, with bounding boxes inside x 207..273, y 5..65, and each
quad is drawn exactly as many times as the HUD's count says. The authored widgets are `<Image name="PickupBackground">` and
`<Image name="TurboIcon">`-style icons, `x="240" y="35" Centred="true"` (66 x 60 and 48 x 48; `crates/hud/src/pickup_look_tests.rs`
carries the authored `Values`, `docs/ui/hud.md` "The pickup icon is found by name").

**No Model widget drew in any captured frame** (confidence 75): the three lists use only two projection matrices, the scene's
perspective and the 480 x 272 orthographic one the HUD's Images and text use, so no `Mode3D` widget (which sets its own
projection, `Mode3D_DrawQueued`) was on screen. That is a statement about these frames (a race with the countdown over and no
lock-on). Whether another Pulse layout names a `<Model>` widget is **not established here**: `docs/ui/hud.md` shows the
26 `Data\HUD\*.vex` files are Pure's `Speed3D`/`Shield3D`/`Overlay3D` and says no Pulse layout names one, but that is a
reading of the layout files I did not repeat; Pulse's countdown is a 3D model (`docs/ui/hud.md`, "Position at `x=0, y=0`"), which
would be the place to look, and it is off the screen in a race. So "does a Model widget smear" stays **open for Pulse's countdown
and for Pure's gauge models**, and closed for the pickup icon.

Method: Absorb held in `race-talons-venom-alongside` (Single Race, Assegai, Talon's Junction, lap 2), PPSSPP 1.20.4 software
renderer, `gpu.record.dump` (`scripts/psp-ge-dump.py dump`, `census`), the HUD's count read live at `*(hud+0x274)+0xa8` with
`hud = a0` at `Hud_Update` (`0x0881bf50`). For the `n = 10` frame the flare's timer (`flare+0xb8`) was written to 0.4 s every 3 ms
for the dump's duration (the count is a pure function of the timer, so this stands in for a pad hit; `n` read 9 to 10 meanwhile).
The `n = 6` frame is a natural pad boost (timer 0.25 s at the start of the dump; the count fell from 6 to 5 across it). The rest
frame is a dump taken with the timer 0 and the count 1.

| Frame | Timer | `n` | Copies of each pickup quad | Text prims (`0x183`) | Scene 3D prims (`vt 289`) | Stencil ref of the HUD draws |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| rest | 0 | 1 | 1 (2 for two layers with the same box) | 31 | 49 | `0x0c` |
| natural pad boost | 0.25 s | 6 | 6 (12) | 211 | 85 | `0x99` = 153 = `trunc(6/10 * 255)` |
| forced timer | 0.4 s | 10 | 10 (20) | 331 | 85 | `0xff` |

(`vt 289` differs between rest and the others because the rest frame is another part of the track; the two boost frames agree to
the prim. The stencil refs are the speed bar's glow stamp above, `n / 10 * 255`, and back the counts.) The 13 images' set is
identical in all three frames; only the copy count moves.

**Per-copy law, read off the 10-copy frame** (GE `OFFSETX`/`OFFSETY` words in 1/16 pixel units, and the first vertex's colour; each
chunk of the dump is a `PRIM` followed by the words that set the *next* draw's offset): the backdrop is drawn with `OFFSETX` 1808
throughout and `OFFSETY` 1912, 1913, ... 1921 for copies 0 to 9 (copy `i` at `1912 + i`: a shift of `i` whole pixels up, none
sideways), and every copy's vertex alpha is `0x0c` where the rest frame's is `0x80` (`trunc(128 / 10)`). The icon quads, whose boxes
differ from the backdrop's (top-left (226, 20) against (207, 5)), carry the **same** offsets.

**The direction is the line from the screen centre to the widget's authored `x`/`y`, not to the drawn box's top-left** (confidence
80: one widget set with 13 quads of two different boxes sharing one vector, plus the three non-centred widgets read before, which
fit the same rule because for them the authored point is the top-left). For the pickup, `(240, 35) - (240, 136) = (0, -101)`,
normalised `(0, -1)`, so `trunc(1912 - dy * i)` is `1912 + i` and `trunc(1808 - dx * i)` is `1808`: exactly what was read. The
box's top-left would give `(-0.245, -0.969)`, a 0 px shift at `i = 1` and a sideways step from `i = 5`, which was not seen, and the
icon quads' different top-lefts would give different offsets, which was not seen either. `Image_ParseValues` reads `Centred` next
to `x`/`y` (`0x088a6f38`), so a Centred widget's anchor is its middle. The four "not reproduced" markers listed under "The copies"
are consistent with this if they are Centred (not checked).

**Ours now matches (wired):** `oag_hud::boost_smear` takes a sprite's direction from the layout's authored `x`/`y`: the
middle for a `Centred="true"` widget (`Sprite::centred`, matched to the drawn box by its middle, which a fade or fold keeps),
the top-left for any other. The pickup's copies step exactly `(0, -1)` (`boost_smear::tests`,
`a_centred_widgets_copies_step_away_from_its_authored_point`, red with the centred lookup off, green with it). A boosting pair
(`scripts/psp-pulse-pair.py --field --layout forward --team Assegai --track 16_Track --state race-talons-venom-alongside.ppst
--run-ticks 100`; original and ours both at boost timer 0.683 s) reads the pickup box (207, 5, 66, 60) at RMSE 6720 before and
6549 after, of 65535; the scene behind the box differs between the two sides, so the box's metric is mostly scene, not smear.
Text widgets keep the chosen direction (`widget+0x172` unread). Titles: Pure, HD, 2048 and Omega carry no `boost_smear` law, so
nothing to port; Pulse PS2 shares `oag_pulse::hud::ART` and gets it (**checked, applies**, not measured on PS2).

The pair of 39-vertex `vt 289` batches (textures `1802816`, `1641536`, about 1.7 units across, screen (236, 165)) after the scene's
last sprite appears in all three frames, once each, with the same boxes: a scene model between ship sprites, not a HUD widget
(removing them with `scripts/psp-ge-replay.py --nop-prims` changes a 14 x 7 px box at (229..243, 160..167) and nothing else).

Other titles: **checked, differs** for HD and Omega (HD's boost pass is `FunkLayerZoom`, above); 2048 **not checkable** here (no
frame at a boost). Pure: **not checked**, its pickup icon and gauge models are `Model` widgets and its count source is unread
(`pure-boost-reread`). Pulse PS2: **checked, applies, not measured**: the copies draw through the shared art ("Other titles").
