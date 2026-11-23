# Zone's billboard equaliser: what draws the bars of the stage target

**The question.** In an HD Zone race, billboard slots 1 to 7 show one shared texture: the
Environment stage block's 512 x 512 `A8R8G8B8` target (`0x00c7dfb0 + 0x3378`, view at
`+0x32b4`), read live as a 32 x 32 grid of 16-pixel cells with bars in it
([billboards.md](billboards.md), "what a Zone race's shared `"billboard"` texture is"). What draws
the bars, from what, and in what colour?

**The answer.** `Environment_UpdateStageBlend` (`0x003da540`, called from `Scene_PrepareFrame`)
draws into that target every frame, right after it writes the `zoneTexVis` lookup
([zone-visualiser.md](zone-visualiser.md)). It reads the same sixteen held band levels: the
billboard bars and the floor's ten-segment bars are one feed drawn twice. Confidence **80** for the
law below (static read at instruction level, every input read live; the bars' pixels themselves
were not caught live, see "Live reads").

## The pass, in order (`0x003dbb98`-`0x003dbea0`, decompiled lines read against the listing)

1. Bind the target (`Rsx_SetRenderTargets` against the target block `+0x3338`/`+0x333c`), colour
   mask on, no depth.
2. **Unless the style is 7, a fade quad over the whole target**: `Prim_SetBlendMode(ctx, 4)`
   (`0x005e5770`), then `FUN_005e9e50` (the quad emitter, [zone-sky.md](zone-sky.md)) with corners
   `(0,0)`-`(512,512)` (`0x44000000` four times) and colour word `0x00000060` on all four corners.
   Blend state 4 is handler `0x005e5ee0` of the jump table at `0x005e5e04`: blend on, equation
   `0x8006` (add), `SRC_ALPHA`/`ONE_MINUS_SRC_ALPHA` on colour, `ONE`/`ZERO` on alpha. The colour
   word packs `R<<24 | G<<16 | B<<8 | A` (below), so this is black at alpha `0x60`: each frame a
   texel keeps `1 - 96/255` of its colour and its alpha becomes `0x60`. That is the trails.
3. `Prim_SetBlendMode(ctx, 0)`: blend off (handler `0x005e5e7c` writes method `0x310 = 0`).
4. **The style's draw**, switched on the word at `0x008c2d2c` (`g_ZoneEqualiserStyle`, below).
   Style 0, per band `b = 0..15` (`0x003dbc9c` onwards, loop on `cmpwi r31,0xf`):

   ```text
   x0 = 32 b + 1            x1 = x0 + 30
   top = 510 - 508 * held[b]
   body: quad (x0, top) - (x1, 510),        colour = the tint halved, every byte >> 1
   cap:  quad (x0, top) - (x1, top + 16),   colour = the tint
   ```

   Constants are this function's own TOC floats (`r2 = 0x008bd3c4`): `-0x5a38` 32.0, `-0x5a50` 1.0,
   `-0x5a34` 30.0, `-0x5a40` -508.0, `-0x5a3c` 510.0, `-0x5a6c` 526.0 (the cap's bottom,
   `526 - 508 * held = top + 16`). `held[b]` is `+0x32f8 + 4b`, the same held level the
   `zoneTexVis` bar reads (`*pfVar78` before the pointer advances). A silent band's body is empty and
   its cap covers rows 510-511, which the grid's bottom line covers.
5. The tail: `+0x3374 = +0x3300` and `+0x3354 = +0x32c0`, then `FUN_005e5d40` against the target and
   `FUN_005e5858(target, +0x3370)` with a full quad. This lays the grid: line texels read alpha `0xff`
   where the fade leaves every other texel at `0x60`, so the grid is drawn after the fade, and ours
   draws it over the bars. What `FUN_005e5d40` binds is not read.

**The two floats are band 2.** `+0x3300 = +0x32f8 + 8` and `+0x32c0 = +0x32b8 + 8`: `+0x3374` is
`held[2]` and `+0x3354` is `raw[2]` (about 94 Hz, the bass), latched once a frame. Read live:
`0.9000 / 0.8311` against `held[2] = 0.9000`, `raw[2] = 0.8311`, and `0.6496 / 0.3736` against
`0.6496 / 0.3736`, two samples, exact. Styles 4 and 6 compare `+0x3354` against `-0x5a60` 0.7
(decompiled lines 1886, 2374). Confidence **92**. That closes the brief's "what do `+0x3374` and
`+0x3354` do": they are the stage pass's bass level for its other styles, not a parameter of
style 0.

**The gain is 1.0.** `raw[b] = SoundSystem_GetBandLevel(b) * entry[+0x34]`, the entry at
`0x008c2cb8 + n * 0x38` (`n = *0x008c1430`, the player). Read live `1.0` at stages 1 and 2, its
`.data` default too. That closes [zone-visualiser.md](zone-visualiser.md)'s open "what
`lfs f0,0x4(r4)` is" (`r4 = entry + 0x30`). Confidence **85**.

## The colours (`0x003db530`-`0x003db900`)

`r5 = 0x00c7dfb0 + n * 0x250 + 0x150`, `r6 = ... + max(n - 1, 0) * 0x250 + 0x150` (the
effectSettings stage table at `+0x1000`): `lwz 0x1000(r5)` is stage `n`'s `EQ colour tint`,
`lwz 0x1004(r5)` its `EQ analogue colour tint`
([zone-effectsettings-loader.md](zone-effectsettings-loader.md), the `+0x150`/`+0x154` rows). Each
byte is cross-faded: `trunc(byte_n * w) + trunc(byte_prev * (1 - w))`, clamped to 255, with
`w = entry[+0x18]` (`f6 = lfs 0x8(r10)`, `r10 = entry + 0x10`; `f5 = 1.0 - f6`). That is
`oag_tables::effectsettings::cross_fade_rgba8`.

- `EQ colour tint` feeds the bars: the `zoneTexVis` segment word `r17`, the target's cap word and its
  halved body word.
- `EQ analogue colour tint` feeds the `zoneTexVis` smooth slots `161 + b`.
  Ours wires both since 2026-11-23 (`mesh_render::zone::EqTints`, `ZoneGrade::eq_tints`); the split moves the Detonator
  floor, whose track texture indexes the smooth slots ([detonator-look.md](detonator-look.md), "The floor's violet").

**Read live (RPCS3, Vineta K, stage 2).** Stage 2's two words are `11e6ffff` / `7df910ff` (bytes as
authored, `R G B A`). The `zoneTexVis` lookup's lit segments read `0xff11e6ff` (`A R G B`) and slot
161 at `held[0] = 0.660` reads `0xa752a30a`, the analogue tint `7d f9 10 ff` times 0.66 per byte
within one count. Confidence **90** on which key feeds which, closing zone-visualiser.md's open
"which authored colour `r17` carries".

**The target's colour words** are packed as `A | B<<8 | G<<16 | R<<24` (cap) and the same with every
byte halved (body, alpha `0x7f`); the fade's `0x00000060` in the same packing is black at alpha
`0x60`, which is what the live target shows. Confidence **75**: the packing is read off the
decompile's lane names, matched to `r17`'s live order, and not seen in a lit target.

## The style selector

`g_ZoneEqualiserStyle` (`0x008c2d2c`, the word after the two stage entries; `.data` initial `1`)
is written every frame at `0x003db968`-`0x003dba00` from entry 0's **requested** stage
(`entry + 0x04`):

| requested stage | 0 1 2 | 3 4 | 5 6 | 7 | 8 | 9 | 10 | 11 | 12 13 14 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| style | 0 | 7 | 8 | 5 | 1 | 4 | 3 | 7 | 5 |

Read live: `0` at stages 1 and 2 (four samples over two boots). `FUN_003cddc0` writes `rand() & 7`
into the same word ([zone-effectsettings-loader.md](zone-effectsettings-loader.md)); this pass
rewrites it before its own read every frame. Confidence **85**. Styles 1 to 8 are branches at
`0x003dbb98` onwards (decompiled lines 1581, 1755, 1830, 1867, 1994, 2093, 2391, 2507) and are
**not read**; style 6 runs 32 iterations (`0x1f`), style 1 starts from `+0x32fc` (`held[1]`).

## Live reads (RPCS3, 2026-11-20, four cold boots)

`scripts/rpcs3-hd-zone-equaliser.py` (Racebox, Zone, Vineta K, Feisar, Null audio, private config
via `scripts/emu-env.sh`): reads, with no pause, the stage block `+0x32b0..+0x3380`, the stage
entries, the fifteen stages' EQ tints and the `zoneTexVis` texels, and the target through the GDB
stub (the `/proc` copy is all zero until the stub has paused the emulator once).

| run | outcome |
| --- | --- |
| 1 | stage 1 at 4 and 6 s after the release (bands all 0), stage 2 at 40 s (bands live, the two floats, `zoneTexVis`, tints); target texels not found (the header's address was not read through the stub) |
| 2 | a dropped RACE TYPE press loaded Detonator (`GetMode()==8`), stopped |
| 3 | the script attached the GDB stub right after the release, and the stub **halts the emulator on connect** (`scripts/rpcs3_debugger.py`): the race was frozen on the grid, bands 0. Target via the stub: exactly two values, lines `0xff000000` and ground `0x60000000` |
| 4 | the same attach, the same frozen grid |
| 5 | stub attached only after the music: the menu walk stopped on `Launch Game`, no race |

So the bars were not caught in the target; runs 3 and 4 say nothing about the music. The ground
reads `0x60000000` there; the first read at the race start (billboards.md) found `0x60010101`. Ours
keeps the ground at `1/255` (the start pair's value), rounding the fade to nearest so it holds there:
**chosen**, a one-count difference in a texel the Zone grade darkens to black.

**Recipe for the missing read** (one boot): the same script as committed (it attaches the stub only
once a held band passes 0.3), then `target-gdb-<k>.bin` against `meta.json`'s `held`: band `b`'s
column `x = 32 b + 5` should be the cap colour from row `ceil(510 - 508 held[b] - 0.5)` for 16 rows,
the halved tint below to row 509, ground at `x = 32 b`.

## Names applied

| address | kind | name | confidence |
| --- | --- | --- | ---: |
| `0x005e5770` | function | `Prim_SetBlendMode` | 80 |
| `0x008c2d2c` | data | `g_ZoneEqualiserStyle` | 85 |

`Prim_SetBlendMode(ctx, mode)` stores `mode` at `*(ctx + 4) + 0x2c` and sets the dirty bit when it
changes; the dispatcher at `0x005e5dd0` (`cmplwi r4,5`, jump table `0x005e5e04`) turns it into RSX
blend methods: `0` blend off, `4` `SRC_ALPHA/ONE_MINUS_SRC_ALPHA`, `5` `DST_ALPHA/ONE_MINUS_DST_ALPHA`
(handler `0x005e5e94`). Every handler read is consistent with the name; handlers 1-3 not read.

## In this engine

`oag_raceplay::zone_equaliser::Equaliser::step` runs the pass once a frame (`Scene::does_frame_work`)
on the held bands `oag_mesh::mesh_render::zone::Hold` already computes for `zoneTexVis` (one hold,
two consumers, as in the original), the stage's cross-faded `EQ colour tint`
(`ZoneGrade::equaliser_tint`) and the requested stage's style (`ZoneGrade::equaliser_style`). Style 0
draws; every other style puts the target back to the bare grid and says so in the load report. The
state is sixteen 512-row stripes (a bar column is uniform across its 30 pixels), expanded under the
grid and uploaded only when a texel changed. Tests: `zone_equaliser::tests` (silence is the bare grid
byte for byte; a held level's bar, cap and body rows; the fade's trail and its fixed point; style
gating).

`hd/zone-vineta-k-z5` (race start, silent): mean grey **7.27 -> 7.27**, RMSE 0.083 -> 0.083, edge
0.29 -> 0.29.

## Other titles

- **Omega: checked, applies, not wired.** `Scene_PrepareFrame_q` reads `-508.0` and `526.0` from `.rodata`
  (`0x01804c34`, `0x01804c3c`, the read at `0x015c8ea0`), the same pair as HD's style 0; the
  neighbouring constants differ (`31.0`, `528.0`, `544.0`), and the loop is unread.
  `Adverts::zone_equaliser_bars` is `false`: the bare grid.
- **2048: checked, differs.** No `526.0` (`00 80 03 44`) anywhere in the v1.04 EBOOT; its Zone band
  source is untraced ([zone-visualiser.md](zone-visualiser.md)). `false`: the bare grid.

## Still open

- Styles 1 to 8 (stages 3 to 14), and what `FUN_005e5d40`/`+0x3370` lay over the bars.
- A target read with the bars lit (recipe above).
