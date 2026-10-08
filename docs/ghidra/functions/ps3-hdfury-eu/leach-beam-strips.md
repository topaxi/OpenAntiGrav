# The LeachBeam strips: a PPU ribbon, not a ThickLine pool, read statically

Read 2026-10-08 (`hd-leach-beam`) from `/hdfury/EBOOT-ps3-hdfury-eu.elf` with
Ghidra and `data/scratch/hd-weapon-blasts/ppcdis.py` (capstone) where the
decompiler truncates at AltiVec. **Nothing here was run on RPCS3 or in the
scratch interpreter**: every law is a static read, and the page says which.
Nothing is wired. Ghidra's caller lists are incomplete for this code (it
truncates `0x001164e0` at its first AltiVec byte and never indexed that
function's calls), so every caller below comes from a raw scan of the text
segment for `bl` targets.

## Answer to the pool question: negative

The two strips are **not** one of `RibbonEffects_Construct`'s ThickLine pools
([rocket-trail.md](rocket-trail.md)). They are two `0x63d0`-byte objects that
each hold their own `RibbonBuilder_Alloc(300, 3)` builder (`0x00115770`:
`RibbonBuilder_Alloc(this, 300, 3); *this = 2`), so the builder's model word is
**2**, which indexes `leachbeam_triangle` in the ribbon-model table
`0x00920fc0` (`rockettrail`, `waketrail`, `leachbeam_triangle`,
`RocketTrail_Shadow_triangle`). Confidence 80 (the index-to-model step is
rocket-trail.md's reading of `RibbonBuilder_Flush`; not re-run here). The
object drives the builder itself, on the PPU:

| address | name | confidence | evidence |
| --- | --- | ---: | --- |
| `0x00153288` | `LeachBeam_DrawStrip` | 80 | Reads the beam's state word `+0x4c`: **4** calls `0x001164e0(this + 0x6420, 1.0, 0.0)`; **3** calls it with `(3.3333 * *(this + 0x10000 - 0x36cc), 0.001)`; any other state draws nothing. Only the strip at **`+0x6420`** is ever drawn: a raw scan finds one caller of `0x001164e0`, at `0x001532ec`, and none for the `+0x50` strip. Reached through a vtable (no direct `bl`). |
| `0x001164e0` | `LeachBeamStrip_BuildRibbon` | 82 | `RibbonBuilder_Reset` (`0x002a45f8`), then per sample `RibbonBuilder_AddSample` (`0x002a4c20`, at `0x001166d0` and `0x00116800`), then `RibbonBuilder_Flush` (`0x002a51e8`, at `0x00116814`). Only caller `0x153288`. |
| `0x001157d0` | `BeamPath_AddWobble_q` | 60 | Adds a sine displacement to every stored sample (calls `0x00676fe8` per axis, the `0.0166667`, `2.9`, `2.58`, `1.2`, `150` constants at TOC `-0x3810..-0x37fc`), the beam's shimmer. Read structurally, not run. |
| `0x00116090` | `BeamPath_BendToTarget_q` | 60 | Moves each sample toward the point it is handed by `cumulative_length[i] / total` (`0x002a3bf0` is the lerp), then rebuilds the cumulative lengths at `+0x5f10`. |
| `0x00117518` | `BeamPath_Build_q` | 60 | Appends the shooter's anchor (below) as sample 0, calls the walker `0x00116c48`, then `0x00116090` with the target's anchor. |
| `0x00115e30` | `BeamPath_CheckLineOfSight_q` | 55 | Takes the two craft at `+0x138`/`+0x13c`, runs a world query (`0x000a96d8`) and tests the chord length against a table value; returns 0 or 1. |

## What `0x001164e0(strip, reveal, window)` draws

Static read of `0x001164e0` (instruction addresses in the listing):

```text
count     = strip[0x144]                  (samples, filled by BeamPath_Build_q)
for idx = count-1 down to 0:              (target end first, shooter last)
    frac  = idx / (count - 1)
    lo    = reveal * (1 + window) - window     remap(reveal, 0..1 -> -window..1)
    hi    = reveal * (1 + window)              remap(reveal, 0..1 ->  0..1+window)
    colour = 0xffffffff              if frac <= lo
           = 0x00000000              if frac >= hi
           = t * 0x01010101          between, t = (hi - frac) / (hi - lo)
    u     += 0.05 * |sample[idx] - sample[idx-1]|        (0 at the target end)
    node   = {position = sample[idx], direction = sample[idx] - sample[idx-1],
              colour, u, 1.0 in the stack record at +0xe8}   -> RibbonBuilder_AddSample
```

So `reveal` is a **reveal progress from the shooter toward the target**: 0
shows nothing, 1 shows all, and `window` is the width of the soft edge. In
state 3 the beam grows over `1 / 3.3333 = 0.3 s` of the object's own clock
(`+0xc934`, not read) with a 0.001 edge, in state 4 it is fully there with a
hard edge. All four colour bytes carry the alpha, so the vertex colour is
white scaled by `t`. Confidence 70: read twice by hand and not emulated; the
`+0xe8` = 1.0 slot is most likely the half-width (the Rocket's builder record
carries its width there) but that was not read off `0x002a4c20`.

## The material

`hd_leachbeam.rcsmaterial` (DATA02 `/data/ribboneffects/materials/`),
`scripts/ps3-microcode.py` on both programs. Vertex program: `TC0 = colour.rgb`,
`TC1 = eye - position`, `TC2 = normal`, `TC3 = (u + 2 * c[210].x, v, colour.a)`,
`TC4 = (v-attribute.x + c[210].x, position.yz)` - `c[210].x` is a single
engine parameter (hash `0x906b67ba`) that scrolls both. Fragment program:

```text
n    = tex0(TC3.xy).r * 2                    unit 0, the noise lookup
facing = |dot(norm(TC2), norm(TC1))|         SRC0_ABS, @0x0f: two-sided
rgb  = tex1(TC4.x + n, ...).rgb * TC0.rgb
alpha = tex1.a * facing * TC3.z * saturate(window_z * 0.75)
```

The `MUL H0.w, H0, |H2.xxxx|` at `@0x0f` carries NV40's `SRC0_ABS`, so this ribbon
is **two-sided** like the Rocket's and the engine trail's (engine-trail.md,
"Two-sided facing term"). Confidence 80 (instruction read, `ps3-microcode.py`
prints the `|x|`). The texture names are the DATA02 `hd_leechbeam_glow.gtf`
(`/data/ribboneffects/textures/`) and the material's sampler hashes; which unit
holds which was not resolved.

## What produces the samples: the shooter and target anchors

`BeamPath_Build_q` and its walker `0x00116c48` look up a **named node** on
each craft with `0x00109ab8(craft, name)` where the name pointer is TOC
`-0x3828` = `0x007834a0`, the string **`arc_anchor_point`**, then place the
point at `node.position + node.row1 * -1.7` (TOC `-0x3824`; the craft's
anchor pushed 1.7 along its own up row). The shooter's anchor is sample 0, the
target's anchor is where `0x00116090` bends the end to. In between the walker
copies samples out of **the target craft's own 300-sample history ring**
(`craft + 0x150 + i * 0x50`, count `craft + 0x5ed4`, the same record shape as
the strip) that lie between the two - so the beam is **not a straight line**:
it follows where the target has flown. Confidence 50: the anchor lookup and the
history-ring walk are direct reads of the loop's addressing, but the selection
rule (the comparisons at `0x00116e30..0x00116f58`, a squared-distance test
against TOC `-0x37dc`) and the ring's own writer were not read, and a path
that is a straight line cannot be told from one that follows the ring without a
live dump.

## What is open, and where to go next

1. **A live dump.** `scripts/rpcs3-hd-weapon.py --state 3` gives the player a
   LeachBeam; the beam object is `RaceManager`'s manager child (`0xc9c0`
   bytes, constructed at `0x00153fb0`). Read `+0x4c` (state), `+0x6420 + 0x144`
   (count) and `+0x6420 + 0x150 + i * 0x50` (samples) over several frames: that
   settles the path, the reveal, and whether `+0xe8` is the half-width, in one
   session. The `+0x50` strip is constructed and never drawn by `0x153288`; a
   raw scan for a second drawer is the other question.
2. **`0x00116c48`'s selection rule and `0x002a4c20`'s read of the stack
   record** (width at `+0xe8`, `u` at `+0xec`).
3. **The wobble `0x001157d0`'s phase/amplitude source** (`+0x63c4..+0x63f4`
   hold two `{phase, amp, rate}` triples set by `0x00115cc0`'s constructor to
   zero and by the owner afterwards).

Nothing was drawn this lane: a straight shooter-to-target line, a guessed
width and the reveal timing would be exactly the plausible stand-in this
project's rules forbid, and the path is the part still unmeasured.

## Omega

**Checked, differs.** Omega's `data00.psarc` ships
`Data/ribboneffects/leachbeam_triangle.rcsmodel`,
`HD_Leachbeam{,_1,_2}.rcsmaterial` and its `eboot.bin` names `leachbeam_triangle`
beside `enginetrail_bluered_triangle`, so the ribbon family carries forward.
Its materials are PS4 shaders, which `ps3-microcode.py` cannot read, so the
fragment law above is HD's alone; the executable is not decoded here and no
PS4 capture path exists. Not wired on Omega.
