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
and composite over those rectangles alone in a lane of its own and adds the result onto the HUD's target. The
scene's bright pass ran before the HUD existed in our order, so it cannot see the bar; a chain over the
rectangles is what its read of the HUD comes to, because the scene's own mask under the bar is the baseline
`4` and truncates to nothing in the original's blurs. A frame with no boost is byte-identical.

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
  smeared, the lock-on reticle included; the glow chain adds the bar's colour alone (`1 - (1 - a)^k` of it, the
  scene's colour under a half-covered bar is not added), and for a split screen the stamps take the whole
  target.
- **Open:** the 3D widgets (`Mode3D_DrawQueued`, `0x088b8548`, carries the same `lui 0x44e2` loop at
  `0x088b8748`) are drawn once in ours; the shake gate of the text copies (`+0x3c/+0x40`) is not modelled
  because ours has no shake; the shield bar's colour in the glow frame; a frame at the boost's start and end
  (`n = 0, 1`) against the original.
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
