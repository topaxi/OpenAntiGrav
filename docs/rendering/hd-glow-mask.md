# Wipeout HD's glow mask is what each fragment program writes into the scene's alpha (2026-10-21, `hd-emissive-glow`)

**Answer.** The original's bloom halo on emissive signs and the GO board is the bloom gate reading a **glow mask the fragment
programs write into the scene target's alpha**, and ours drew none of it: every HD model was `GlowMask::Protected`, so the gate's
`Bloom from alpha contribution` term (2.5 on Metropia) was multiplied by zero everywhere. The unlit constant-glow path's "0.7x its
constant" is **not a defect**: it is the program's own `/2`, and ours already matches the original's pre-bloom value there. What
was missing is the mask. Confidence 90 for the mechanism (live read of the original's scene target, below), 85 for the per-program
reading (a static evaluator, checked against the live frame on the sign, the sky, the screens and the road, not on every program).

Code: `oag_rcs::rcsmaterial::fragment::Program::output_alpha` reads the alpha lane of a program, `oag_mesh::mesh::rcs::glow` turns it
into `GpuVertex::glow`, and `mesh_render::GlowMask::StampedByProgram` writes it. Tests: `crates/rcs/src/rcsmaterial/fragment/glow_alpha/tests.rs`,
`crates/render/tests/hd_program_glow_mask.rs` (fails if the model flag, the write mask, the override or the decode is dropped) and the
disc-backed `hd_program_glow_ground_truth.rs` and `crates/raceplay/tests/hd_sky_tint_ground_truth.rs`.

## What the original does (RPCS3, Metropia Harimau time trial start, 2026-10-21)

The method is `scripts/hd-draw-dump.py` (one frame of draw state through `rpcs3_draw_hook.py`) plus a read of the render targets from
VRAM in the same pause (`Write Color Buffers` on, so the targets are in guest memory):

- The scene target is `0x00f50000` (`A8R8G8B8`, surface word `0x3148`); its resolve and the bloom chain's buffers follow
  (`0x02150000` the half-resolution scene, 640x360, pitch `0xa00`; `0x02240000`, `0x02280000`, `0x022c0000`, `0x02300000` the quarter-resolution
  gate and blur buffers, pitch `0x500`). The half-resolution scene reads as an ordinary picture and its **alpha plane is a mask**:
  **255 on the "Be" neon sign's strokes, on the three teal screens and on the lamp discs, 122 flat on the sky, 0 on the lit road
  and walls, a graded 20 to 200 on the track's pad lettering and light bars** (29.6 % of the pixels non-zero).
- The sign's pre-bloom colour in that buffer is `(155, 166, 175)` at the pixel where ours renders `(153, 164, 174)`. The "0.7x" lead
  compared our *final* pixel with the original's *final* one, which is white only because of the bloom; the unlit program is
  `(C * f + fog * (1 - f)) / 2` (`cf_constantcolourglow`, block `#3`: `MAD /2 H0.xyz, R0.wwww, -R0, R1`, then `MOV H0.w, 1`), and halving in
  linear light then encoding is `0.5^(1/2.2) = 0.73`. **Nothing was removed or added to the `/2`.**
- The colour mask says who writes alpha. Of the frame's 812 draws, on the scene target: **587 opaque draws write RGBA**
  (`cm 01010101`); **every one of the 12 opaque draws with the alpha test on, and 53 of the 59 blended draws, have the alpha bit
  clear** (`cm 00010101`); 5 blended draws write it. So the mask is written by the opaque programs and by nothing that is cut out or
  blended (a crowd or a tree that bloomed would blur the stands, which the original does not).
- The programs write it themselves, as the last lane before the colour's final `MAD`: `MOV H0.w, {1}` on the signs, the glow
  tubes, the plasma and the lit billboards (the three teal screens and the GO board: `cf_vex_billboard`), `MOV H0.w, {0}` on the lit
  surfaces, `MOV H0.w, H1` (a texture's alpha) on the emissive pads and lights, and the sky's `COL0.w`: **Metropia authors
  `Lighting.Sky colour` = `255 255 255 122` and the sky reads alpha 122** (the fourth byte reaches the mask unchanged; renderer.md's note that
  `c * c * lowbyte` is written is `c = 1` here).

The bloom chain itself was already the original's (renderer.md, "The bloom chain, read pass by pass"; `oag_post::hd_bloom`), including the
gate `frame.rgb * frame.a * <Bloom from alpha contribution>`. **Nothing in it changed.** Its alpha input was the empty one.

## What ours does now

`output_alpha` evaluates the alpha lane of the program the race resolves for a material, as a constant, a texture channel times a constant,
or `Other`. The census over the 13 circuits' and the Harimau's materials (`crates/render/examples/hd_glow_alpha_census.rs`) gives
about 360 `(family, class)` rows; in short:

| class | what draws it | value |
| --- | --- | --- |
| constant 1 | `cf_constantcolourglow`, `dc_constantglow`, `sign_emissive_glow`, `cf_plasma_glow*`, `mr_uvanim_em_glow`, `cf_vex_billboard` lit variants (the cards) | `1.0` |
| constant 0 | every lit surface (`lambertzeroalpha`, `diffuse_specular`, `track_surface`, ...), 80 % of the table | `0.0` |
| a texel channel | `cf_alpha4glow` (320 slots), `lambert` and the cutouts (not written, see below), the pads and light bars (`weapon_pads`, `diffuse_normal_specular_emmissive`: the `_ne` alpha), `uvanim_diffuse_emissivealpha`, `sign_emissive_alpha`, `signcl_emissive_glow` | `scale * channel` |
| other | 12 families: the waterfalls, the lightcone, the hologram strips, the stand glass (`glasstest`), the weapon halos | **no mask, counted** (`Report::glow_unread`) |

Wiring, in the order the original's draws decide it:

1. **Opaque draws write the program's value; cutouts and blended draws leave the mask alone** (`cm` above). A first version let the cutouts
   write it and over-bloomed Amphiseum's crowd (26.51 to 27.78 grey difference, against 25.96 once they were masked).
2. **A texel-sourced mask is read from the sample the program reads**: the coverage alpha where the renderer already routes it, else the
   second texture (scrolled where the material adds it), the first, or the pad's third binding. `GpuVertex::glow` carries `>= 0` a constant and `< 0`
   `-(scale + 8 * (4 * source + channel))`, decoded by `glow.wesl`'s `program_glow`.
3. **The sky writes its `Sky colour`'s fourth byte** (`sky_cube::build(.., mask)`); a Zone sky and a circuit with none write 0.
4. **A program that never writes `H0.w` writes 0.** The alternative, that `H0` and `R0` share storage and the unwritten lane reads whatever a write
   to `R0.w` left (`cf_billboard1` writes `R0.w` and not `H0.w`), was tried as 1.0 and **made Metropia and Moa Therma worse** (17.1 to 17.8,
   23.4 to 24.0), so it is refuted for that value; the true readback of an aliased lane is not read.

## The GO board: a phase, read out of RAM, and the card's own program

Three findings, the first two by-products of the same frame:

- **The three teal boards are one program.** Metropia's draws 96, 101 and 106 sample the two 512x256 cards (`0x02aa0000` the `FX-350` advert, `0x02ae0000`
  the GO card): `tex0 * (glow.a + glow.rgb * light)` with `MOV H0.w, {1}`. Their alpha is 1, so a bright card blooms: with the mask wired the GO
  glyph reads `255` with the original's halo where it read `174` flat.
- **The board's phase is the gantry node's own time, and the race clock does not give it.** The race manager keeps the node's `+0xc0` inside
  `[3.83, 5.25)` s before the first crossing (gantry-clock.md), the board pulses over that window, and a pair
  that runs ours on the race clock shows a different frame of the loop. `scripts/rpcs3_hd_field.py::gantry_seconds` finds it in RAM: a circuit has about 1,300
  `MeshImporter` nodes (vtable `0x00869fd0`), 984 at 0.0, 295 on the free-running clock and **19 inside a window, all at one value** (Metropia: 5.147 on a restored state, 4.42
  on the pair's frame). `oag-game --gantry-seconds S` (`Options::gantry_seconds`) pins the race manager's board - the timelines with a release window, the card
  included - and nothing else; the pair script passes it. Swept in ours at 3.9, 4.2, 4.5, 4.68, 4.9, 5.15: bright, absent, bright, mid, absent, bright (a pulse of about 0.6 s), so every
  state the original shows is one ours can show.
- **The grab lags the memory read.** With the game paused the RSX is still presenting the frames it was handed, so a screenshot taken at once showed a moment 0.1 to 0.3 s older than the
  RAM the pose, clock and gantry time come from (the HUD's lap clock read `0.21.0` against RAM's `21.2`, and ours at the RAM time pulsed a half cycle off). `OAG_CAPTURE_FLUSH`
  (3 s in `rpcs3-hd-pair.py`) lets it catch up: the HUD then reads `0.12.1` against RAM's `12.208`, about one tick.

**Still open on the board:** the side cards. Ours shows the `FX-350` advert on the two side boards and the original's are plain teal in that frame (the advert's own timeline, not the gantry's;
`adverts.rs` runs it off the race clock); and `cf_vex_billboard`'s program is a multiply by the card (`card * (glow.a * k + glow.rgb * light)`) where ours adds the second texture to the
albedo: the GO card reads right through the halo, Vineta K's yellow `harimau` billboard (`(255, 255, 28)` against ours `(164, 183, 118)`) does not (rcsmaterial.md, "Not drawn, named").

## Per-pair mean grey difference against the original

Mean absolute grey difference over the 1920x1080 frame, 0-255. **Third column** is the A/B that isolates this lane: each pair's own recorded flags rendered with this build, the original's `theirs.png` unchanged
(the mask and the sky; the gantry pin needs a re-capture and is in the last column). **Last column** is the 13 pairs re-made 2026-10-21 against fresh captures of the original (the pair script now pins the gantry phase and waits for
the RSX), so the original moves too: the ones that read worse there (Modesto Heights, Talon's) read better in the third column, so that is the new capture and not the picture. Modesto's bottom half and
Tech de Ra's rose 1 to 2 levels in the re-capture (the original's floor and the HUD read differently); none of them moved in the A/B by more than 0.2.

| Pair | before (shipped ours, same original) | after (this build, same original) | re-made pair, both sides re-captured: before / after |
| --- | ---: | ---: | ---: |
| anulpha-pass | 25.01 | 24.23 | 25.01 / 24.51 |
| chenghou-project | 16.32 | 15.22 | 16.32 / 14.80 |
| metropia | 23.37 | 17.11 | 23.37 / 16.45 |
| moa-therma | 24.96 | 23.43 | 24.97 / 24.45 |
| modesto-heights | 23.55 | 23.44 | 23.55 / 24.33 |
| sebenco-climb | 22.58 | 22.49 | 22.58 / 21.88 |
| sol-2 | 23.30 | 22.76 | 23.30 / 17.93 |
| talons | 20.82 | 20.29 | 20.83 / 21.93 |
| talons-assegai-c1 | 22.27 | 21.74 | 22.27 / 21.41 |
| tech-de-ra | 33.72 | 33.70 | 33.72 / 33.25 |
| the-amphiseum | 26.51 | 25.12 | 26.51 / 24.55 |
| ubermall | 18.08 | 16.84 | 18.08 / 15.78 |
| vineta-k | 26.51 | 25.07 | 26.51 / 23.61 |
| **mean of the 13** | **23.62** | **22.42** | **23.62 / 21.91** |

The HD race pair (`hd-race/talons-pack`) is **unchanged by this lane to the byte** (55.70 against the same original: Talon's Junction authors a `Sky colour` fourth byte of 0 and no glow
program is in view); its re-capture drifted 14 units and 11.7 degrees from the state (the restore-and-pause flake the compare page names), so the re-made number (66.19) is not comparable and the
shipped pair stays the reference. Largest differences left on the 13: Tech de Ra 33.7, Amphiseum 24.6, Modesto 24.3, Moa Therma 24.5, Anulpha 24.5, Vineta K 23.6: materials and light, not the mask.

## Cross-title (CLAUDE.md, 2048 and Omega)

- **Glow mask by program: checked, differs / not checkable.** The reading is of HD's `SHO` fragment microcode; 2048's programs are declared-interface only (`.gxp`, no decoder in this
  project) and Omega's are GNM (no capture path), so neither title's alpha lane can be read. Their `rcsmodel` readers do not go through `mesh::rcs::build`, so the new flag is not set on
  them and nothing changed.
- **Sky colour's fourth byte: checked, applies, not wired.** The HD-lineage `.envsettings` of 2048 and Omega carry the same key; whether their sky draws write it is not checkable (no live frame).
- **The gantry pin: checked, differs.** 2048 and Omega place no gantry (start-gantry.md).
- **Does the bloom finding transfer to Pulse (Fort Gale White's tunnel halo)?** Not directly. The finding is that HD's programs write the mask and ours did not; Pulse's mask is the GE stencil
  stamp and ours already writes it (`GlowMask::Stamped`, glow-mask.md), and Fort Gale's note says the original's mask there is 255 on the halo's pixels. The open question there is whether
  ours' own stamp covers the same pixels, which needs a dump of our mask (`OAG_DUMP_GLOW_MASK` dumps the final frame's alpha only); it is a mask question, as here, and the method (read the
  target's alpha next to the frame, then the colour mask per draw) is the one that found this.

## Open

- The 12 `other` families write no mask. The weapon halos and the hologram strips are the ones a player sees.
- `Aliased` lanes (a program that writes `R0.w` and never `H0.w`: `cf_billboard1`, 75 draws on Metropia) read 0 here; the original's value is unread.
- The glow table's `Bloom from alpha contribution` is read per circuit (`oag_post::hd_bloom::Params`) and untouched; Tech de Ra, Amphiseum and Modesto's remaining gaps are materials and light,
  not the mask.
