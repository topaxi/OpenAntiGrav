# `MagstripWake` in Wipeout HD / Fury (PS3 `EBOOT.elf`)

Functions in `EBOOT.elf` (Wipeout HD Fury, `BCES-00664`, EU), PowerPC64, image
base `0`. The full object description, evidence and caveats are on
[`ps4-omega-eu/ships-effects.md`](../ps4-omega-eu/ships-effects.md) (section
"2026-10-05"); this page is the HD half, written 2026-10-05 by the
magfloor-omega-re lane, static reading only (no RPCS3 run). Addresses are
the code addresses Ghidra shows as `.opd` entries (`_opd_FUN_0010a0c0` is the code
at `0x0010a0c0`).

**The question the brief asked: does HD have the class? Yes (confidence 85).**
`MagstripWake.cpp` (`0x00782fa0`), `"MagstripWake"` (`0x00782f90`),
`"arc_anchor_point"` (`0x007834a0`, `0x00782fb8`),
`"**** WARNING **** : Ship has no arc_anchor_point locator"` (`0x00782fd0`),
`MagStripArc_vp`/`MagStripArc_fp` (`0x007a0d18`, `0x007a0d28`),
`Data/Tex/HD_electric_arc_8x8.gtf` (`0x007a0d38`), `Data/Tex/HD_ElectricArc_Contact.gtf`
(`0x007a0d70`) and `"Send due m_overMagStrip change - now %i\n"` (`0x007a9320`) are
all present. This is the class Omega and 2048 carry forward, not a newer one.

## Constructors - `0x0010a0c0` and `0x00109e40`

`MagstripWake_Construct` is `0x0010a0c0`; `MagstripWake_ConstructAlt` is `0x00109e40`.
They are byte-for-byte the same body (a complete and a base-object constructor
of the same class, a compiler pair), and both: call a base constructor, store the vtable
(`PTR_PTR_008a98d0`, the table at `0x00863868`), store `PTR_s_MagstripWake_cpp_008a98d8`
at `param_1[0xc]`, resolve `"arc_anchor_point"` against the ship (`FUN_006762e8` on the
ship's locator set, then a walk up the parent chain), allocate two `0x200`-byte objects
(the two ribbons), a `0x5e0` arc pool, OR `6` into the flags word and increment the
live-instance counter (a global block at `+0xb80`). **85** for `0x0010a0c0`: the same tag
string, the same `arc_anchor_point` lookup, the same `| 6` and counter increment as the
PS4 and Vita constructors. **75** for `0x00109e40` (identical body, but which one the
ship constructor calls was not checked).

## Vtable `0x00863868` (slot, function descriptor, code)

Slots 3, 5, 7, 12 and 13 hold `.opd` descriptors in the `0x0087540x` run; the others
are base class (`0x00885axx`/`0x00885cxx`). Descriptors at `0x00875408` ->
`0x00109028`, `0x00875428` -> `0x00109350`, `0x00875438` -> `0x001095e0`, `0x00875440`
-> `0x00109720`, `0x00875448` -> `0x00109858`.

- **Slot 5, `0x00109028` = `MagstripWake_EnqueueRender`, conf. 78.** Appends
  `{param_1, 0x4d000000 | (inst+0x11c & 0xfffff)}` to the list at `*(PTR_DAT_008a98c0) + 0x630`
  (count at `+0x44b0`): the same body as the PS4's `0x012e2720`. This is the
  cross-binary agreement the PS4 name rests on, and `renderer.md` already
  attributes this site to `MagstripWake.cpp` through the TOC slot `008a98c0`.
- **Slot 3, `0x00109350`** is *not* the PS4 update. It draws the two ribbons
  (`FUN_002aa1c8` on `inst+0x50` and `inst+0x54`) and calls `FUN_002bbd60` with the
  speed read at `ship_body+0x4c4` (the PS4's `+0x4b8`) and `ship+0x7820`/`+0x7830`
  (the ribbons' lateral frame) - which is the tail of the PS4's draw `0x012e2770`.
  So HD's slot order is not the PS4's and **slot index is not evidence across the
  two builds**; only slot 5 is matched. The arc update and arc quad build are
  presumably `0x00109858` (slot 7) and `0x001095e0`/`0x00109720` (slots 13, 12);
  **none of those were read**, and they stay unnamed (below 50).
  **Corrected 2026-10-05 (magstrip-hd-measure): that presumption was wrong** - `0x001095e0`
  and `0x00109720` are destructors and `0x00109858` is the ribbons' update; the arc pool is
  `0x002bbd60`/`0x002bb530`/`0x002bc7b0`, see the last section.
- `0x001092d0`/`0x00109198`/`0x00109568` etc. in the descriptor run are other
  vtable-adjacent code and were not read.

## What HD does with the arc textures and the sound

- **Textures** `Data/Tex/HD_electric_arc_8x8.gtf` and `HD_ElectricArc_Contact.gtf`
  ship in `DATA02.PSARC` (`/data/tex/hd_electric_arc_8x8.gtf`, 349,696 bytes;
  `/data/tex/hd_electricarc_contact.gtf`, 5,632 bytes), plus siblings
  `hd_electric_arc_8x8_nonanchored.gtf` (349,696), `psys/tex/hd_electric_arc_8x8.gtf`
  and `hd_electricity_1x4.gtf` (699,264). **No literal in the EBOOT names the
  `_nonanchored` or `electricity_1x4` files** (strings search), so they are unreferenced
  by name from the executable (they may be reached by a built path; not checked).
- **No `WO_MAGSTRIP_*` string exists in the HD EBOOT**, and no `MagStrip_Player`/`_NPC`.
  HD ships a different particle file instead: `/data/psys/wo_magstrip_lightning.pob`
  (3,360 bytes, `DATA02`) which **no string in the EBOOT references**. It, and its textures
  `psys/tex/pulse_mag_sprite1_orange_rings.gtf` and `weapons/textures/mag_lightning2_add_glow.gtf`,
  are found-but-unwired as far as static strings go (the name could be built by a format
  string; not searched).
- **Sound:** `~magstrip01` is a cue in `shiphd.bnk` (`DATA01`, `/data/sound/shiphd.bnk`, at
  file offset `0x19790`, beside `~jet..` cues; a `### Magstrip` section label follows at
  `0x198a8`). Its two string literals in the EBOOT (`0x00781080`, `0x00781dd0`) have no direct code
  references (they are TOC-addressed); the start site was **not located**.
- **Rumble:** `enter_mag_rumble.xml`, `travel_mag_rumble.xml`, `exit_mag_rumble.xml`
  (`/data/xml/rumble/`, 356, 548, 356 bytes) ship for the magstrip pad; the same three
  ship in the Omega `data00.psarc`. Who plays them was not searched.

## 2026-10-05, magstrip-omega-law lane: the replicated `m_overMagStrip` and the HD anchors

Static reading, `EBOOT.elf`; full evidence for the shared law is on
[`ps4-omega-eu/ships-effects.md`](../ps4-omega-eu/ships-effects.md) ("2026-10-05,
magstrip-omega-law lane").

### `ShipNet_CheckSendState` - `0x00337d78`, conf 65

The only reader of the string `"Send due m_overMagStrip change - now %i\n"` (`0x007a9320`,
TOC slot `0x008b5688`, reached as `-0x7d3c(r2)` on the `r2 = 0x008bd3c4` half of the two-TOC
split; found by searching `lwz ..,-0x7d3c(r2)`, three hits, one in game code). It is a
per-ship **should this state be sent** test over a record at `param + slot * 0x24 + 0x10`,
compared against the last-sent record at `param + slot * 0x28 + 0x160`: it returns 1 (and,
when the debug flag `param+0x2c0` is set, prints why) on a time overrun
(`"Send due to time"`), a position error beyond `param+0x2a4`, a rotation error beyond
`param+0x2ac`, or a **flag-word change**. The flag word is at record `+0x2c`:

- bit `0x400` changed: prints the next string (`-0x7d40(r2)`, the other replicated boolean).
- **bit `0x200` changed: prints `m_overMagStrip` with `(flags >> 9) & 1`.**

So the replicated field `m_overMagStrip` is **bit 9 of the ship's network flag word** on
HD (the PS4 packs the same boolean as bit 1 of a byte at `+0x41` of a `0x21`-byte record,
`FUN_012ed9b0`). Both are filled from the ship's own over-the-strip flag; on the PS4 that is
`controller+0x5d0`, written by `FUN_0131b510` as `(probe hit a triangle of surface type 3)`
- the same literal Pulse's `craft+0x240` uses. The HD writer of the flag word was **not**
located (the HD analogue of `FUN_012ed9b0` is unread), so on the PS3 this page confirms the
field's existence, its width and that it is replicated, not its producer: the producer
claim rests on the PS4 and Pulse (85).

### Hull anchors, HD (conf 90)

All 39 `data/ships/<hull>/locators.vex` carry exactly one `arc_anchor_point` node (class
id `110`, on the centreline), and **match the Omega copies hull for hull**; the table and the
`#[ignore]`d test (`crates/game/tests/magstrip_anchor_ground_truth.rs`) are on the Omega
page.

## 2026-10-05, magstrip-hd-measure lane: HD's own arc pool, read and measured live

Static reading of `EBOOT.elf` plus **two RPCS3 boots** (`rpcs3-drive.py capture --region`,
a scratch directory, not kept, `live2`; Talon's Junction, Fury grid cell). The
vtable slots earlier assumed to hold the arc update were not it:

| Address | Name | Conf | What it is |
| --- | --- | --- | --- |
| `0x001095e0` | `MagstripWake_DeletingDestruct` | 80 | resets the vtable, decrements the live-instance counter, frees the two ribbons (`+0x50`, `+0x54`) and the arc pool (`+0x58`), then `FUN_006761f8(this)` (free) |
| `0x00109720` | `MagstripWake_Destruct` | 80 | the same body without the free |
| `0x00109858` | `MagstripWake_UpdateRibbons` | 70 | calls the pool's `0x002bc7b0` (gated on `ship+0x5f43 == 0`) and the ribbons' `0x002a8f58`, then offsets the ribbons by `+/- clamp(speed - [0], [1], ...) * [2]`: the PS4 ribbon law, a second source for it |
| `0x002bbd60` | `MagstripArcs_Update` | 85 | nine slots of `0xa0` bytes: age, `0.85` smoothing, shed, jitter |
| `0x002bb530` | `MagstripArcs_Spawn` | 75 | picks the first free slot, sets life, contact scale, spread (the decompiler stops at bad data; the disassembly was read) |
| `0x002bc7b0` | `MagstripArcs_Draw` | 65 | VMX vertex build: 40-byte float vertices, position, uv at `+0x10/+0x14`, RGBA floats at `+0x18..+0x24` |

### The tuning block is initialised data, and it is read, not a `kIntensity`

All the arc constants live in one table of 22 floats at `0x008c2610` (TOC slot `0x6380`),
**initialised in the image, not run-time**, and the same bytes read back from live memory on both
boots (so the PS4's runtime-zero `DAT_020e52a0` has no HD counterpart for these):

| Offset | Value | Used by |
| --- | --- | --- |
| `+0x00,+0x04` | `0.2`, `1.1` | arc life (`0.2 + 0.9u`) |
| `+0x08` | `0.4` | start reach (`START_REACH`) |
| `+0x0c,+0x10` | `3.0`, `5.0` | slow-speed reach |
| `+0x14,+0x18` | `15.0`, `22.0` | fast reach; also the shed radius `lerp(15, 22, 0.6) = 19.2` (`368.64` squared) |
| `+0x1c,+0x20` | `10.0`, `200.0` | the speed blend's ends: `t = (speed - 10) / 190` |
| `+0x24` | `0.8` | strip half width (`HALF_WIDTH`) |
| `+0x28` | `0.55` | ahead draw offset (`0.55 + u`) |
| `+0x2c,+0x30` | `0.125`, `0.2` | **body brightness sample range** |
| `+0x34` | `0.3` | **body vertex alpha** |
| `+0x38,+0x3c` | `0.05`, `0.1` | **contact brightness sample range** |
| `+0x40` | `0.25` | **contact vertex alpha** |
| `+0x44` | `0.4` | read by the draw/update setup (unresolved) |
| `+0x48,+0x4c` | `3.5`, `7.0` | contact scale |
| `+0x50,+0x54` | `0.6`, `0.8` | spread: `0.6` slow, `0.8` fast |

Agrees with the PS4 reading on life, reach, speed blend, scale, spread, shed radius and the
`0.85`/`0.15` smoothing (`x = 0.85 x + 0.15 sample`, from TOC `0x63a4`/`0x63a8`).

**Disagrees on brightness and alpha** (conf 80, static, table confirmed live): the PS4 reading
has body brightness `0.0862u + 0.01875` (sample up to `0.7`) and a fixed alpha `0xb2` in a
`0..255` byte colour; HD's sample is `0.125..0.2` (smoothed mean `0.1625`), the contact's
`0.05..0.1` (mean `0.075`), and the vertex is **float RGBA with alpha `0.3` (body) and `0.25`
(contact)**. HD's `MagstripArcs_Spawn` never writes the brightness fields, so a slot's brightness
ramps from whatever the slot held (`0.85^n`, about 15 ticks to settle). HD names no `kIntensity`
(string search: zero hits); the PS4's `DAT_020e52a0 + 0x1e0` has no HD analogue found.

### The jitter scales: measured, live (conf 90)

`FUN_00676fe8` is `_FSin`. Both class initialisers (`0x002bd750`, `0x002bde18`) store, at
`0x00ad8984 + 4k`, `sin(arg_k) * 0.6 + 0.4` with `arg = 0, pi/4, pi/2, 3pi/4, pi`. **Read back
from RPCS3 on two cold boots, in race: `0.4, 0.824264, 1.0, 0.824264, 0.4`** (the same bytes at
three captures), term for term what `MagstripArcs_Update` multiplies the five jitter terms by.
The PS4's `DAT_02134210..20` is almost certainly the same table (not read there).

`MagstripArcs_Update` also pairs the draws: each tick it draws a fresh offset
`spread * (2u - 1)` for terms 0, 2 and 4; term 1 reuses term 0's draw and term 3 reuses
term 2's, unless `rand() & 7 == 0` (1 in 8) draws a fresh one (conf 85).

### What a frame shows (not a number)

`run1.mp4` (RPCS3's recorder, 30 fps, `rpcs3-drive.py record`),
frames 31-36 of the 28 s window against ours at ticks 1136-1141 (`ours_consec.png`,
`orig_consec.png`): the original's arcs look several times wider and whiter on screen than ours,
and wavier (smooth, not zig-zag). **This is not a matched pose**: the original is ~0:18 on the
lavender grid with the default walk's hull, ours is tick 1136 over the dark floor with `feisar_c1`,
and additive arcs on a bright floor plus HD's bloom look different from the same arcs on a dark one. The world widths agree
(`+0x24 = 0.8`), so the gap is not geometry: the fragment program `MagStripArc_fp` (compiled
into the executable, no file on the disc) and HD's bloom are the open candidates. **No intensity
can be measured from these frames**, since there is no arc-free frame at the same pose.

## 2026-10-05, magstrip-arc-fp lane: `MagStripArc_fp` and `MagStripArc_vp` decoded, the draw's state read

Static reading of `EBOOT.elf` (Ghidra, plus a capstone pass over the VMX-heavy draw that the
bridge's disassembler stops inside), reproduced by `scripts/ps3-microcode.py fp 0x0092f180` and
`vp 0x0092f400`. No live capture this lane; the programs are not data that varies at run time.

### Where the programs are (conf 90)

The shader pair is initialised data, not a file on the disc. The TOC (`r2 = 0x008ad4d8`, from the
tuning table's own slot: `0x008b3858` holds `0x008c2610` at `r2 + 0x6380`) carries, in order:

| TOC slot | Offset from `r2` | Holds |
| --- | --- | --- |
| `0x008b3894` | `+0x63bc` | `0x007a0d18`, the string `MagStripArc_vp` |
| `0x008b3898` | `+0x63c0` | `0x0092f400`, the vertex `SHO` block |
| `0x008b389c` | `+0x63c4` | `0x007a0d28`, the string `MagStripArc_fp` |
| `0x008b38a0` | `+0x63c8` | `0x0092f180`, the fragment `SHO` block |

`MagstripArcs_InitClass` (`0x002bd750`, the pool's class initialiser, the one that also builds the
jitter table) looks each program up by name (`lwz r3, 0x63bc(r2)` then `0x006775b8`, and `0x63c4`),
then resolves two parameter indices by hash into the tuning block's `+0x58` (the vertex program's
`viewProj`) and `+0x5c` (the fragment program's texture sampler). The draw reads them back.

### The fragment program, whole (conf 90 for the instruction list, 85 for the reading)

```text
sampler 0x7d99f28d  unit 0          (HD_electric_arc_8x8 / HD_ElectricArc_Contact, bound per batch)
0x50 bytes of code, no inline constant, no patch slot
@0  TEX R1, f[TC0] unit0
@1  MOV H0, f[COL0]
@2  MUL R0.xzw, H0.xyyz, R1.wwww
@3  MUL H0.xyz, R0.xzww, R1
@4  MUL H0.w,   H0, R1            END
```

H0 and R0 alias on the RSX (`H0` is the low half of `R0`, so `H0.w` lives in `R0.y`, which `@2`
does not write): the five instructions read as

```text
out.rgb = vertex.rgb * tex.a * tex.rgb
out.a   = vertex.a   * tex.a
```

**There is no constant and no scale. The vertex alpha reaches the alpha output only, never the
colour. No transfer function appears anywhere**, so the original adds gamma-space values into its
target, the same finding as the HD engine tube (`engine-trail.md`).

### The vertex program, whole (conf 90)

```text
0  MOV o[TC0].xy, v[8].xyxx           (uv)
1  MUL R0, v[0].yyyy, c[1]
2  MAD R0, v[0].xxxx, c[0], R0
3  MAD R0, v[0].zzzz, c[2], R0
4  MOV o[COL0], v[3]                  (vertex colour, untouched)
5  ADD o[POS], R0, c[3]               END
```

A bare `viewProj` (parameter `0xe252323b`, `c256`) transform. The vertex colour passes through
unscaled.

### What the draw uploads and sets (conf 80)

`MagstripArcs_Draw` (`0x002bc7b0..0x002bd6f0`; 40-byte float vertices, position at `+0x00`, uv at
`+0x10/+0x14`, RGBA at `+0x18..+0x24`). Every call between the build and the return:
`Rsx_SetMethod`-class state writes (`0x00677ff8`/`0x005c1d0c`), `0x00677468` (the matrix upload,
4 rows through `0x100..0x130`), `0x00677478` twice (texture bind, per batch: `+0x20` then `+0x24`
of the class block), the vertex attribute pointers (`0x00678018`), the draw (`0x00678028`). **No
fragment-parameter setter and no float constant is uploaded**; the fragment program has none to
receive.

State, from the wrappers (`Rsx_SetMethod`, `Rsx_SetBlendFunc` at `0x005c1fe0`,
`Rsx_SetBlendEquation` at `0x005c2130`, `Rsx_SetDepthMask` at `0x005c2524`, already named in
`material-state.md`):

| Call | Value | Reading |
| --- | --- | --- |
| method `0x183c` | `0` | cull face off |
| method `0x304` | `0` | alpha-test related, off (method id unresolved) |
| method `0xa74` | `1` | depth test on |
| method `0x310` | `1` | blend enable |
| `Rsx_SetBlendEquation(0x8006)` | `FUNC_ADD` | |
| `Rsx_SetBlendFunc(1, 1)` | source `ONE`, destination `ONE` (colour and alpha) | **additive, measured on HD, not inferred from the PS4** |
| `0x679058(0x203)` | `LEQUAL` | depth function |
| `Rsx_SetDepthMask(0)` | | **depth write off** |

### Brightness and alpha, read in the build (conf 80)

The vertex colour is the arc's own brightness, **with no multiplier**: the body quads store
`[slot+0x48]` into `+0x18/+0x1c/+0x20` of each vertex and the tuning float `0.3`
(`0x008c2610 + 0x34`) into `+0x24`; the last quad's far edge stores `[slot+0x48] * 0.3`
(`0x008b38ac` holds `0.3`, so the soft edge exists on HD too); the contact quad stores
`[slot+0x94]` and `0.25` (`+0x40`). The update (`0x002bbd60`) smooths each as
`x = 0.85 x + 0.15 * (lo + (hi - lo) * u)`: body `0.125..0.2`, contact `0.05..0.1`, scale
`3.5..7`. `MagstripArcs_Spawn` writes neither brightness.

### What this settles, and what it does not

- **`INTENSITY = 3.0` is not in the original's shading.** Vertex colour, vertex program and
  fragment program carry no gain (conf 85). Deleted.
- **With the original's law the arcs are faint in our frame** (brightness at most `0.2`, times the
  texture's colour and alpha). `cmp_g1138.png`: top the old
  `INTENSITY = 3`, bottom the original's math, same tick (1138), same `feisar_c1` autopilot ace
  pose. The same arcs are there, thin and dim, and the contact glows are nearly gone. The
  original's frames (`magstrip-hd-measure`, not a matched pose) read wider and whiter.
- **Residual gap, named, not tuned**: (1) HD's bloom on the arcs: bloom is ported (see
  `renderer.md`), but whether the arc's destination is the bloom source at the same weight is
  unchecked; (2) the destination alpha: the original's `ONE, ONE` adds `vertex.a * tex.a` into the
  frame's alpha, which this port keeps for its glow stamp (so a bloom keyed on it is not
  reproduced for arcs); (3) the sampler's sRGB-remap bit (the same unread bit as the engine tube).
  A matched-pose capture of an arc-free and an arc frame is what would measure it.

### Names

| Address | Name | Conf | What it is |
| --- | --- | --- | --- |
| `0x0092f180` | `MagStripArc_fp` | 90 | the fragment program's `SHO` block, five instructions |
| `0x0092f400` | `MagStripArc_vp` | 90 | the vertex program's `SHO` block, a bare `viewProj` transform |
| `0x002bd750` | `MagstripArcs_InitClass` | 70 | class initialiser: looks both programs up by name, resolves the two parameter indices, builds the five jitter terms |
| `0x002bc7b0` | `MagstripArcs_Draw` | 80 (was 65 above) | read whole this lane: vertex build, brightness and alpha stores, the state block |

## 2026-10-05, magstrip-arc-gap lane: why our arcs were fainter, and the destination alpha

Static reading plus six RPCS3 boots (own display, config and pad; a scratch directory, not kept
to `run6`). **No matched-pose arc/arc-free pair was obtained** (below), so the size of the effect against
the original is unmeasured; what is established is a missing feed.

### The finding: arcs feed HD's bloom through the frame alpha (conf 75, static, not pose-verified)

1. The original's blend is `ONE, ONE` on **both** channels: `Rsx_SetBlendFunc` is the two-argument
   form that writes one factor for RGB and alpha (`material-state.md`, conf 92), and the arc draw
   passes `(1, 1)` (conf 80). `MagStripArc_fp` writes `a = vertex.a * tex.a` (0.3 body, 0.25
   contact), so every arc fragment **adds** that into the scene target's alpha.
2. That alpha is the glow mask HD's bloom gate reads: `gate = frame.rgb * frame.a * <alpha
   contribution> + frame.rgb * pow(luma, exponent) * <frame contribution>` (`renderer.md`, "The
   bloom chain", conf 90). **Talon's Junction authors `Bloom from alpha contribution = 3.0`,
   `from frame contribution = 0.03`, `exponent = 4.0`** (`track.envsettings`, read off the disc).
3. So an arc fragment's gate input is about `rgb * 0.3 * tex.a * 3.0 = 0.9 * tex.a * rgb`, near the
   pixel's own colour, while the luminance term is `0.03 * luma^4`, negligible for a 0.1-0.2 arc.
   **The arcs are bloomed at close to full strength through alpha, not through luminance.** The
   previous port kept the frame's alpha (`ZERO, ONE_MINUS_SRC_ALPHA`, the PS4 reading), so the
   gate saw nothing: arcs without their halo.
4. The original's frames show it: soft white halos at the contact points several times the
   geometry's size (`on064.png`, run1 `018.png`/`019.png`).

Our side: texture upload is `Rgba8Unorm` (no sRGB decode, correct for a program with no transfer
function), the add is raw into the linear target (the engine-tube precedent), and our bloom gate
already reads scene alpha. Changed: the arc's alpha blend is `ONE, ONE` and its fragment alpha is
`vertex.a * tex.a` (`Style::alpha_is_fragment`). Ours, `feisar_c1` autopilot ace, tick 1138:
`cmpA_crop.png` (top before, bottom after): the arcs gain the
halo and read more present; **they are still thinner than the original's**, and the original's floor
here is a different, brighter one, so no ratio is claimed.

### Live reads (RPCS3, Talon's Junction, Fury grid cell)

- The wake object is reachable: `*(0x008ad4d8 - 0x49fc)` is the ship table base `Q`, `*(Q + 0x14c)`
  the player ship, `*(ship + 0x6940)` the `MagstripWake` (vtable `0x00863868`, matches
  `*(0x008a98d0)`), `*(wake + 0x58)` the 9 x `0xa0` arc pool (conf 85: one boot per read,
  `run3`, vtable and pointer chain confirmed).
- **Contact brightness read live**: the field at slot `+0x14` holds `0.056, 0.082, 0.079, 0.056, 0.080,
  0.074, 0.076, 0.076` on eight live slots, inside the table's `0.05..0.1` (the contact scale
  `3.5..7` sits at `+0x10`: `4.06, 5.68, 5.73, 3.64, 5.43`). The offsets in this page's earlier
  sections (`+0x48`, `+0x94`) are relative to a different base and were not reconciled; the body
  brightness field was not identified.
- What blocked the matched pair: (a) the original's variable timestep means a stopped craft never
  stayed still (airbrakes did not stop it within the window; consecutive frames differ by a mean 36
  levels even for the best on/off pair, `run2`); (b) the pause menu's Photo Mode freezes the sim but
  the grab did not change between frames (diff 0.0) and a whole-pool zero write crashed the
  application (`run5`); (c) code patches are not seen by RPCS3's recompiler, so the draw cannot be
  disabled by a write. A zero of the table's brightness ranges does hide arcs after about 15 ticks
  (`run2`, off frames), but at a different pose.

### Still open

Sampler sRGB-remap bit on the two arc textures (the engine has no per-texture gamma control,
`renderer.md`, conf 90, so it is expected clear); whether the scene alpha saturates at 1.0 as the
original's 8-bit surface does (ours is `Rgba16Float`, the gate clamps at `surface()`); the arc
width gap, which needs a pose-matched pair to size.
