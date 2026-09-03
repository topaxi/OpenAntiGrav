# Three PS2 textures are mirrored artwork, not a decode bug - and the sweep that proved it

2026-09-03. Reported from play: *"the PS2 speedpads look less detailed than
their PSP counterparts."* The difference is real and measurable. **The cause is
not `oag_formats::ps2_texture`** - an earlier version of this file said it was,
on partial evidence, and the widened sweep below refutes that.

## What is actually true

Wipeout Pulse's PS2 disc ships `flicker1nonalpha_GLOW` and
`flicker2nonalpha_GLOW` - the speed pad's and weapon pad's glow strips -
**vertically mirrored** relative to the PSP disc's copies of the same artwork.
So does `envtest4bitSingle` (64x64) and one of the two `envmap_stripe1`
copies (32x32). The decoder reproduces what the disc holds; the disc holds a
mirrored picture.

Since the pad's geometry and UVs are **byte-identical** across the two titles
(node 0 is 42 vertices on both, `u` in `[0.0000, 0.5137]`, `v` in
`[0.0000, 3.9961]`, first six UV pairs equal to four decimals), and the texture
tiles about four times vertically, a mirror turns "sharp edge then gradient"
into "gradient then sharp edge" at every repeat. That is exactly the "flatter,
less crisp" the report describes.

## The sweep, and why it settles the question

49 confident PS2-PSP texture pairs across three circuits (`01_Track`,
`02_Track`, `05_Track`), each scored by summed absolute RGBA difference both
as decoded and against the reversed row list, keeping only pairs that are both
close and unambiguous. **37 as-is, 12 mirrored** - and the 12 are only three
distinct images, each appearing once per circuit.

Three independent reasons this cannot be a decoder defect:

1. **No header field discriminates, because textures with byte-identical
   headers land on both sides.** At 64x64 with `+0x08 = 0x00100000` and
   `+0x0c = 0x14`, `achev64`, `AAdc_Crowd`, `trackjump01_sb` and `trackjump_sb`
   all decode as-is at difference **0**, while `envtest4bitSingle` decodes
   mirrored at difference **0**. Same dimensions, same layout, same flags, same
   every field - opposite handedness. A decoder that sees only the blob cannot
   do that.
2. **One circuit's set holds the same image both ways round.** `02_Track` has
   `envmap_stripe1` twice: entry 041 upright (difference 100 as-is) and entry
   045 mirrored (difference 100 flipped). Two entries, two orientations, one
   decoder.
3. **A blanket flip breaks the picture, measured.** Flipping every
   `Layout::Psmt8` decode and rendering a real PS2 race mangles the HUD's
   chevron and lap/timer glyphs, gets the ship's livery wrong, and changes the
   track surface. The current build matches PSP's frame closely.

Ruled out along the way, so nobody re-tests them:

- **`TRXPOS`'s `DIR` field** (bits 59-60), the GS's own transmission-direction
  selector and the one register that could mean "flip this". It is `0` on all
  **403** textures dumped across the three circuits. `SSAY` and `DSAY` are `0`
  on all 403 too.
- The header `flags` word at `+0x02`: mirrored textures appear at both `0x2000`
  and `0x2040`.
- `Layout`: the mirrored set is all `Psmt8`, but so is the as-is majority.
- Size: mirrored ones are 32x32, 64x32 and 64x64, and there are as-is pairs at
  every one of those sizes.

## The V-convention objection, and why it does not survive

The one remaining way this could still be a bug on our side: if the GS's
texture V origin ran opposite to the PSP's, an artist would mirror the source
art to compensate, both consoles would draw the same picture, and only *this*
renderer would differ.

**That reading is refuted by the 37 as-is pairs.** This renderer applies one V
convention to a PS2 model. If it were the wrong one, all 49 paired textures
would draw upside down, not twelve - and a PS2 race frame would show an
inverted HUD, inverted billboards and an inverted sky. It does not: it matches
the PSP frame closely, which is the same evidence the blanket-flip experiment
produced from the other direction (flipping everything is what breaks those).

So the convention is right, mirrored source art draws mirrored, and the two
ports genuinely differ on these three textures. Confidence **85**: three
independent structural arguments agreeing, capped below what a console capture
would give.

## Open

**A confirming capture of a real PS2 speed pad has not been taken.** It is
belt-and-braces rather than load-bearing, per the section above.

Attempted 2026-09-03 and abandoned for a reason worth recording. The harness
itself works exactly as documented - `just pcsx2-boot`, then
`just pcsx2-frame /tmp/out.png <frames> cross --from-state 2` off the grid
savestate already in `~/.cache/oag-pcsx2/PCSX2/sstates/` - and produced clean
512x512 frames at 120, 260, 400, 520 and 700 frames on the first attempt.

**But `--hold cross` alone never reaches a speed pad.** It is throttle with no
steering, so on Moa Therma the craft is into a wall by frame ~520: speed 0
km/h, shield bar red, "OUT GATE?" on the HUD, and every later frame is the same
crashed frame. The same trap the engine-flare thread records for
`just play ps2 --race`, one console over. Nothing was on a pad in any frame
captured, and none was even in view.

Closing it needs steering, which means either a scripted button sequence
through `pcsx2-drive.py press` or a second savestate taken with the craft
already parked on a pad. The latter is much cheaper and is what to do:
**take it once, save it to a slot, and every future PS2 texture question is one
command.**

## Next Steps

1. **Park on a speed pad and save a state.** Boot, drive to the first pad on
   Moa Therma by hand, `just pcsx2-state save 3`. Then
   `just pcsx2-frame /tmp/pad.png 1 none --from-state 3` and compare the
   stripe phase against `just play ps2 --race --autopilot --screenshot`. If
   they match, this thread closes with nothing to change.
2. **If they do not match**, the question is which V convention this renderer
   applies to a PS2 model, not how `psmt8_offset` permutes - the permutation is
   proved correct by the 37 as-is pairs. Look at `mesh::build`'s texcoord
   handling for `vex` version 6 before touching `ps2_texture`.
3. **Worth recording either way**: `+0x08` and `+0x0c` are still marked
   "unknown" in `ps2_texture.rs`'s module docs, and the sweep pins them.
   `+0x0c` takes 9 values across 403 textures and tracks size closely
   (`0x04` at 256x256, `0x14` at 64x64, `0x0c` at 64x32 and 128x16, `0x08` at
   32x32, `0x05` at 8x8 and 16x16), and `+0x08` is a single bit walking with
   it (`0x01000000`, `0x00100000`, `0x00080000`, `0x00040000`, `0x00010000`).
   They look like a size class and a log2-derived allocation hint. Not
   established, but cheap to finish and worth a line in
   `docs/formats/ps2-texture.md`'s "Not determined" section.

## Reproducing

Scratch examples, deleted rather than committed - each is a few dozen lines
over public API:

- **PS2 dump**: `Archives::read_preceding(track_vex)` -> `wad::Directory` ->
  per entry, LZSS-decompress, `ps2_texture::header` + `parse`, write
  `png::encode_rgba(tex.to_rgba())`. Put the fields in the filename:
  `flags`, `layout`, and from `TRXPOS` at `HEADER_LEN + 8 * 16` the `DIR`
  (bits 59-60), `SSAY` (16-26) and `DSAY` (48-58), plus `+0x08` and `+0x0c`.
- **PSP dump**: `race::load(...).track_model.textures`, same PNG writer.
- **Pairing**: group by dimensions; for each PS2 image score every PSP
  candidate as `min(diff(rows), diff(reversed rows))`; keep pairs under 0.1%
  of maximum where the two scores differ by more than that, which is what
  makes a verdict unambiguous rather than noise.
- **Pad UVs**: `pad_model.vertices[..].texcoord` over `node_vertex_ranges[0]`,
  both titles.
