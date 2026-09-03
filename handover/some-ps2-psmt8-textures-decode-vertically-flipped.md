# Some PS2 `PSMT8` textures decode vertically flipped, and a blanket flip is proved wrong

2026-09-03. Reported from play: *"the PS2 speedpads look less detailed than
their PSP counterparts."* They do, it is real, and it is **not** a pad problem -
it is `oag_formats::ps2_texture`.

## The finding

Wipeout Pulse's PS2 and PSP discs ship the **same pad artwork**, and this
project's own PS2 decode places it upside down.

Pinned three ways, none of which leaves room for "the PS2 asset is just
different":

1. **Geometry and UVs are byte-identical between the two titles.** Node 0 of
   `01_Track`'s `Speedup Pad` model is 42 vertices on both, `u` in
   `[0.0000, 0.5137]` and `v` in `[0.0000, 3.9961]` on both, and the first six
   UV pairs match to four decimals. So nothing downstream compensates for a
   difference in the texture, and nothing is meant to.
2. **The decoded images match exactly under a vertical flip.** Comparing PS2's
   decode against PSP's `.tga` (which needs no swizzle at all, so it is ground
   truth), summed absolute difference over every RGBA byte:

   | Texture | Layout | as decoded | flipped |
   | --- | --- | ---: | ---: |
   | `flicker1nonalpha_GLOW` 64x32 | `Psmt8` | 490,224 | **0** |
   | `flicker2nonalpha_GLOW` 64x32 | `Psmt8` | 523,980 | **0** |
   | `envtest4bitSingle` 64x64 | `Psmt8` | 94,260 | **0** |
   | `weapon_under` 128x64 | `Psmt8` | 5,660 | **0** |
   | `envmap_stripe1` 32x32 | `Psmt8` | 181,104 | **100** |
   | `billboard1/2/6/7/8` 8x8 | `Linear` | **0** | 15,300-24,480 |

   Zero, not "close". And the 64x64 is the one that settles the *shape* of the
   error, because it has 22 distinct rows rather than four: decoded row `y`
   equals PSP row `63 - y`, exactly, for every row that is unique enough to
   pin. It is a flip, not a shift and not a scramble - the run of 22 distinct
   rows survives intact and in order.
3. **Rolling the pad texture back makes the picture match.** Rendering one
   isolated pad plate on each title through `capture::capture_from`, PS2
   against PSP goes from RMSE **0.178** to **0.024** once the pad's glow
   texture is corrected. The residual is the PS2 exporter's own 253-vs-255
   quantisation.

What it looks like: PSP's gold bands carry a bright highlight ridge and fine
sub-striping; PS2's read flatter and softer, because the texture tiles about
four times vertically (`v` to 3.996) and a flip turns "sharp edge then
gradient" into "gradient then sharp edge" at every repeat.

## Open

**A blanket flip is wrong, and that is measured, not feared.** Flipping every
`Layout::Psmt8` decode and rendering a real PS2 race breaks the picture badly:
the HUD's chevron and lap/timer glyphs come out mangled, the ship's livery is
wrong, and the track surface changes. The current, unflipped build matches
PSP's frame far more closely. So most PS2 `PSMT8` textures decode **correctly**
today.

The discriminator is not found. What it is **not**:

- **Not `Layout`.** The flipped set is entirely `Psmt8`, but so is the
  correct-as-is majority.
- **Not the header's `flags` word** (`+0x02`, `0x2000` or `0x2040`). Flipped
  textures appear at `0x2000` (32x32, 64x64, 128x64) and at `0x2040` (both
  64x32 pad glows).
- **Not simply size**, though it correlates: everything confirmed flipped is
  128x64 or smaller, and six 256x256 sky/track pairs read as-is - one of them
  (`sebenco_peak_Top_B_nomip`) at difference **0**, which is a real exact
  counter-example rather than a weak pairing.

## Next Steps

1. **Get ground truth from the real console before changing any code.** The
   PCSX2 harness exists and is documented
   (`docs/reverse-engineering/pcsx2-debugger.md`); a savestate on the grid
   makes a capture one command,
   `just pcsx2-frame out.png <frames> cross --from-state 2`. Photograph a
   speed pad on the real disc and settle which way up its stripes run. Until
   that exists, "PSP is right so PS2 must match it" is an assumption - a
   legitimate one, but the two ports really could differ, and this is exactly
   the kind of question a capture answers in minutes and a decompiler does
   not.
2. **Widen the pairing sweep.** The evidence above is one circuit,
   `01_Track`. Dump every PS2 texture-set entry and every PSP model texture
   across all 40 tracks, pair by size and by min(as-is, flipped) difference,
   and tabulate the verdict against **every** header field, not just `flags` -
   `+0x08` (u32) and `+0x0c` (u8) are both still marked "unknown, correlates
   with `+0x0c`" in `ps2_texture.rs`'s own module docs, and an unread field
   that predicts a flip is exactly what this looks like. That is the cheapest
   route to the discriminator and needs no emulator.
3. **Only then change `psmt8_offset` or its caller.** Do not flip
   unconditionally - it is already proved to break the HUD - and do not add a
   size threshold by eye. The 256x256 exact-match counter-example has to be
   explained, not stepped around.

## Reproducing

Everything above came from scratch examples that were deleted rather than
committed; each is a few dozen lines over public API and is quicker to rewrite
than to maintain:

- Pad UVs: `race::load` both titles, print `pad_model.vertices[..].texcoord`
  over `node_vertex_ranges[0]`.
- Texture dumps: PS2 through `Archives::read_preceding(track)` ->
  `wad::Directory` -> `ps2_texture::parse` (the header's `flags` and `layout`
  are worth putting in the filename); PSP straight off
  `track_model.textures`. Write both with `oag_formats::png::encode_rgba`.
- Pairing: group by dimensions, score each candidate by summed absolute RGBA
  difference both as-is and against the reversed row list, keep the pairs that
  land under about 2% of the maximum.
- The plate picture: `crates/render/tests/pad_alpha_test_ground_truth.rs`
  already has the capture harness; isolate one node's vertex range first or
  ten pads frame as ten specks.
