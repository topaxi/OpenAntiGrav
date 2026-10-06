# Billboards on the PS2: the original draws the adverts into a 128x128 target too

Binary: `SCES_547.48` from `pulse-ps2-eu.chd` (`SCES-54748`); no function is named
here, the evidence is a live PCSX2 capture. The PSP side of the same mechanism is
[`psp-pulse-usa/billboards.md`](../psp-pulse-usa/billboards.md).

## What was measured (2026-10-06, PCSX2 v2.7.494, own display and data path)

A race on Talon's Junction (`16_Track`), a savestate on the grid, then GS
single-frame dumps (`GSDumpSingleFrame`, see
[`pcsx2-debugger.md`](../../../reverse-engineering/pcsx2-debugger.md)).

1. **A 128x128 offscreen pass exists.** In a dump taken near the start line the GS
   `FRAME_1` register points at `fbp 0x1a4`, `fbw 2` (128 pixels wide) for a short
   run of draws, between the 512-wide main passes: two clear sprites
   (`rgba (255,0,0,0)` then `(0,0,0,255)`), then the model's draws. That pass
   holds 208 prims of `321Go` (32x32 texture): the start gantry, drawn through
   its own camera. The main pass then samples it (`TEX0 tbp 0x3480 = fbp 0x1a4 * 32`)
   with a 32-prim group on the "GO" board (`u 0.125..0.875, v 0.5..1.0`).
2. **A second pass is the advert.** In two dumps taken with the camera on
   Talon's Junction's slot 7 quad (the craft was written onto the quad's front
   through PINE, 28 units out, because the quad is behind the start), the offscreen
   pass holds 10 + 14 prims (8x8 and 4x16 textures) in one dump and 76 prims (8x8
   texture) in a later one, i.e. content that changes with time, and **a main-pass
   group of exactly 31 prims samples it**. `16_Track`'s slot 7 draw has 31
   triangles on the PS2 disc (and on the PSP's). Both passes appear together in the
   same frame, the gantry's and the advert's, as on the PSP.
3. **The pass is not drawn every frame.** A dump 348 frames into the race
   (ship past the gantry and far from every quad) has no 128-wide target at all.
   The original gates the cards on something this project does not model yet
   (range or visibility of the quad). The project draws every card every frame.

**Confidence: 88** that the PS2 original renders adverts through their own camera
into a 128x128 target and samples it on the `billboardN` quad, as the PSP does.
Measured for slot 7 and for slot 8 (the gantry); slots 1, 2 and 6 use the same
code and the same disc files and were not captured separately.

## What the project does on PS2

The same code as on the PSP: `oag_raceplay::adverts` draws each card into a
128x128 target and `Scene::new` points the placeholder materials at it. A readback
test (`crates/raceplay/src/adverts/tests.rs`) shows every PS2 card carries a
picture (identical to the PSP's, pixel for pixel in the card), and a second test
(`tests/scene_build.rs`) shows every placeholder with a card is rebound.

The previous lane's "ticks 300, 700 and 1500 are pixel-identical to `main`" is
explained: those frames show none of the PS2 track's six quads. `--camera-pose`
placed on slot 7, 2 and 1 shows the card on each (`16_Track` PS2 slot 1, 2, 6, 7
quads render whole; the PS2 disc has no third slot-1 quad at `(-522, 12, -385)`
that the PSP track has, so one fewer draw: six against seven).

## Not settled here

- **The card's content at a given time.** Fourteen screenshots from the quad's
  front over 11 seconds are all white on this side, which this lane could not
  tell from "the card is in its blank phase". They are not evidence either way.
  The original's card pass had real content in its dumps (above), so the
  sampled region in that view is the open question: **an orientation or window
  question on the quad's UVs** (the raw GS UVs of the slot 7 quad run `v` 1.0 to
  2.0 and decrease downward). In this project's render the slot 2 and slot 1 hoardings
  show their lettering upside down from the front, the slot 7 panel upright at
  the start; whether the original does the same on those quads is unmeasured.
- **The animation clock.** The card was blank at every sampled time from the
  grid to 11 s after it; this project's cards leave the blank state within 1 s of
  the scenery clock. The original's cards do not appear to run off the race
  clock. Measure it with several quads in view.
- **A frame gate** (point 3 above).


## 2026-10-06 (billboards-3 lane): the V flip is inherited from the PSP, not measured here

`Adverts::flip_v` (see [the PSP page](../psp-pulse-usa/billboards.md)'s 2026-10-06
billboards-3 section) applies to the PS2 render too, because both load the same title
data and the PSP is the reference. No PS2 GS register was read for it. The teleport
recipe above trips the craft's respawn flash when the destination is off the track
(`o-t-8.png`: speed 180, the energy bar red, the whole frame white), which is what the
earlier "all white" screenshots at a quad were, so a front view of a hoarding on this
side needs a position the physics keeps.
