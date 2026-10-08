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
| `0x001157d0` | `BeamPath_AddWobble` | 90 | The beam's shimmer: three sines moving every sample but 0 along its rows; law and live check in "2026-10-08 `hd-leach-draw`" below. (Read structurally at 60 by `hd-leach-beam`; raised there.) |
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

## 2026-10-08 (`hd-leach-path`): measured live on RPCS3, path law and material read

Two boots of a held LeachBeam (`data/scratch/hd-leach-path/run5`, `run6`; Racebox-free
Fury campaign walk, player placed 40 units behind the nearest rival at that rival's
speed, slot state written, TRIANGLE held), agreeing on every structure below.
`+0xf0` of the pickup slot is the *owner* craft, not a target (a write there was
overwritten by the game).

**Which state is the LeachBeam: 10.** `Weapon_FireFromSlot` (`0x0012d970`)'s state-10
handler (`0x0012dba8`) sets slot `+0x200` bit `0x8000`; the per-slot update
(`0x0012ba80`) calls `LeachBeamManager_Fire` (`0x0013ccd0`, confidence 80), which
writes `slot+0x204 = -1` (the spent state), arms beam 0 (`+0x40 = 1`) when manager
`+0x88` (active count) is 0, calls the beam init `0x00154558` (or `0x00154360` with no
target, index `slot+0x1ac == -1`) and increments `+0x88`. Written on a fresh boot
(slot clean), state 10 fires at once: `(slot+0x204, +0x208)` goes `(-1, 10)`, manager
`+0x88` 0 to 1. State 3 only sets `0x2000` and calls `0x00129048(5)`: nothing
consumes it, the state stays 3 (four boots, up to 14 s held). Earlier "3 or 10"
resolved to **10**, confidence 90. Writing 6, 1, 2 first leaves `0x800`/`0x2000`
bits in `slot+0x200` and a later state 10 did not fire in that boot.

**State machine** (`LeachBeam_Update` `0x001535f8`, jump table `0x001536e0`, state at
`+0x4c`, clock at `+0xc934`; conf 80, read plus live): 2 = ball flight, 0.4 s
(`2.5 * clock`, TOC `-0x22c8`), then clock = 0, state 3 (live: state 3 at clock 0.15);
3 = reveal, to state 4 with clock 0 (`0x00153b2c`; 0.3 s by the TOC `-0x22c0` constant,
the hand-read reveal rate `3.3333` agrees); 4 = held until the owner's end flags,
then 6 (not drawn). `LeachBeam_DrawStrip` draws 3 and 4 only. Live: 3 to 4 to 6 seen
with `n` growing 16, 22, 30, 40, 51, 66 samples as the rival pulls away.

**The path (measured, conf 75).** The strip's samples are the **target's anchor
trail**, not a line:

- `LeachTrail_RecordAnchor` (`0x001169f8`, from `0x000ebfe8`, every tick, every craft):
  anchor = `arc_anchor_point` node `+0x30` plus node `+0x10` (row 1) * `-1.7`, or the
  body's `+0x200`/`+0x1e0` the same way when the hull has no node. If it is more than
  **3.4** from the newest record (TOC `-0x37e4`) the head `craft+0x5ed0` steps (wrap
  at 299 to 0), and the record at `craft+0x110 + head*0x50` is written: `+0x00` anchor,
  `+0x10`/`+0x20` body rows `+0x1d0`/`+0x1e0`, `+0x30` four words from `craft+0x7820`,
  `+0x40` the craft's track progress (`craft+0x7020`, a 0..1 lap fraction); the count
  `craft+0x5ed4` saturates at 299.
- `BeamPath_WalkTargetHistory` (`0x00116c48`): sample 0 is the target's anchor
  (strip `+0x138` is the target, `+0x13c` the shooter; live, sample 0 sits on the rival
  ahead). Then the target's ring newest first, keeping records whose progress is not
  below the shooter's (a lap/wrap fix of +1.0 for a lap difference), and more than
  1e-4 from both anchors; at the first record below it, one more point is the
  inverse-lerp between the last kept and that one at the shooter's progress. The
  progress of the last sample equals the shooter's `+0x7020` exactly (live).
- `BeamPath_BendToTarget` (`0x00116090`, conf 80): `cum[i]` = running segment length,
  `sample[i] = lerp(sample[i], shooter_anchor, cum[i] / cum[n-1])`, so sample 0 stays on
  the target and the last lands on the shooter's anchor. Live: fitted bend fractions
  match `cum/total` to 0.003 over 28 samples; predicted against dumped samples the
  worst point is 2.1 units on a 125-unit beam, ends exact, and the sample count equals
  the prediction in 3 of 6 snapshots (the others one record off at the head, a pause
  between the ring write and the strip build).

**The nodes** (`LeachBeamStrip_BuildRibbon` run in the scratch interpreter,
`data/scratch/hd-leach-path/emu_strip.py`, conf 85): per sample from the shooter end,
`forward = unit(sample[i] - sample[i-1])` (sample 0 reuses sample 1's), `up =
unit(ref - (ref . forward) forward)` with `ref = (0, -1, 0)` (`RibbonNode_FromDirection`
`0x002a3f70`), `right = up x forward`; half-width **1.0** (node `+0x68`), `u +=
0.05 * segment length` starting 0 at the shooter end (`+0x6c`), colour word at node
`+0x60` (not `+0x48`): `0xffffffff`, `t * 0x01010101` or 0 by the reveal law above
(emulated for reveal 1.0/0.5/0.0, window 0 and 0.001). `RibbonBuilder_WriteVertexPair`
(`0x002a4660`) reads half-width `+0x68`, `u` `+0x6c`, colour `+0x60`; three fins
as the Rocket's (`RibbonBuilder_Alloc(300, 3)`), `v` 1 and 0.

**The material** (`hd_leachbeam.rcsmaterial`, `ps3-microcode.py`, conf 85): vertex
program `TC3 = (u + 2 time, v, colour.a, v)`, `TC4.x = u + time`; fragment: `n =
2 * tex0(TC3.xy).r` (noise unit 0), `rgb = tex1(TC4.x + n, TC3.w).rgb * colour.rgb`,
`alpha = tex1.a * |facing| * colour.a * saturate(0.75 * z)`; `time` is engine
parameter `0x906b67ba`. Which GTF is unit 0 and unit 1, the samplers' wrap state, and
the blend of this draw (it is not `RibbonEffects_Render`'s) are **unread**.

**Film** (`run7/rec.mp4`, 8 fps sheet `run7/fr/beam_sheet.png`): thin white and violet
crackling arcs from the player to the rival, a few units wide, not a smoke ribbon.

**Not drawn, and why.** The draw needs (1) an anchor trail per craft recorded every
tick, (2) the HD state timeline on top of our Pulse-lineage beam, (3) a pipeline with
a second texture, the `time` parameter and a blend nobody read. (3) is the blocker:
any texture assignment or blend would be chosen. Nothing wired.

**Omega:** checked, differs (PS4 shaders, `ps3-microcode.py` cannot read them); the
ring and walker are EBOOT code, not decoded for Omega.

## 2026-10-08 (`hd-leach-draw`): the draw state, the wobble, the walk's direction; wired on HD

Boot `data/scratch/hd-leach-draw/c4`, `c5` on RPCS3 (own config, `scripts/rpcs3-trace`-style session,
`scripts/rsx-draw-list.py --samplers --const N` added for this), a held beam (state 10) with the
player placed behind the nearest rival, then the pushbuffer of the paused frame read. A complete frame
is rare (1 in 15-25 pauses); the beam's hold was kept alive across retries by rewriting the beam clock
(`+0xc934`) while paused, which only drives states 2 and 3 (capture aid, nothing else touched).

**The draw** (draw 923 of a 1123-draw frame, and draw 28 of a tail-only frame; fragment program
`0x00744c41` disassembles to the `hd_leachbeam` program above; conf 90):

| State | Value | Evidence |
| --- | --- | --- |
| blend | on; colour and alpha factors `SRC_ALPHA, ONE` (regs `0x314 = 0x03020302`, `0x318 = 0x00010001`), `FUNC_ADD` | draw regs; `hd_leachbeam.rcsmaterial`'s own state word `0x79` decodes to the same (`hd_unlit_probe`) |
| depth | test on, `LEQUAL`, write off | `0xa74/0xa70/0xa6c = 1/0/0x203` |
| cull, alpha test | cull off, alpha test off | `0x183c`, `0x304` |
| colour mask | RGB (`0x00010101`, alpha not written) | `0x324` |
| unit 0 | 128x128 DXT5, 8 mips, `REPEAT` u/v/w, linear | `0x1a00` block: fmt `0x00088829`, address `0x60710101`, rect `0x00800080`; the `hd_waketrail_clouds.gtf` (22016 bytes), the second texture `leachbeam_triangle.rcsmodel`'s material names |
| unit 1 | 256x256 DXT5, 9 mips, `REPEAT`, linear | fmt `0x00098829`, rect `0x01000100`; `hd_leechbeam_glow.gtf` (87552 bytes) |

The fragment program reads unit 0 as the noise and unit 1 as the glow, so the units are fixed by size
and by the program, conf 85 for the names. Topology: 3 fins, `(a0, a1, b0), (a1, b0, b1)` per segment per
fin (the index buffer), 36-byte vertices `{pos, normal, uv, colour}`, half-width 1.0 (pair 2.0 apart), `u`
0.019 per 0.38 units, colour `0xffffffff` in state 4. All as the Rocket's ribbon, and as `emu_strip.py`.

**`time`** is the engine clock in seconds: the fragment constants `c[464]`/`c[466]` of the draw read 161.59
and 190.09 on two boots at about 160 and 190 s of play; `c[467]` is the eye. conf 80.

**The wobble `0x001157d0`** (decompile plus raw disassembly, **verified on the live beam**, conf 90). The
drawn strip (`+0x6420`) carries three `{frequency, amplitude, phase, last sine}` groups at `+0x63c4`,
`+0x63d4`, `+0x63e4`: frequencies 3.0, 2.9, 2.58, amplitudes 0.3, 0.5, 1.2 (TOC `-0x3814..-0x3800`,
read from `0x008a9cb8`), the three phases counting up by 1/60 per call (all three equal, 10.93 and
10.47 on two beams). Per call, `i` from `n - 1` down to 1: `arc += p[i] - p[i-1]` (the records' `+0x40`
progress, a 0..1 lap fraction), `s_k = sin((150 * arc + phase_k) * f_k)`, `D = sum(s_k * amp_k)`,
`taper(i) = 2i/n` up to the middle then `2 - 2i/n`, and `sample[i].pos += row0 * taper * D + row1 *
taper` where `row0`/`row1` are the record's `+0x10`/`+0x20` (the body's rows 0 and 1). Sample 0 is not
moved. **Check:** from the dumped beam's progress (0.58344 first, 0.55055 last) and phases the three
sines come out -0.7513, -0.9927 and 0.2265, the object's stored last sines to four places
(`oag_fx::leach_strip` test `the_wobble_sines_reproduce_the_live_beams_stored_values`). The 0.0 triples of
the `+0x50` strip are the strip that is never drawn.

**Which craft the walk follows** (`0x001179c8`, conf 80): it stores the pair, computes a signed progress
difference with `0x000d0398`, and sets `+0x138`/`+0x13c` from its sign before `BeamPath_CheckLineOfSight`,
`BeamPath_Build`, `BeamPath_AddWobble`. So the walked craft is the one **ahead**, and a beam at a craft
behind walks the shooter's own trail; the bend goes onto the other craft's anchor. (The earlier "target
ring" reading held because the live targets were ahead.) Names: `BeamPath_Rebuild`.

**Held duration** (live poll, `c4/out001.txt`): state 2 about 0.45 s, 3 about 0.45 s, 4 until 3.7 s after
the press (clock 2.6) then 6 and 0 - on this build's slow clock; the end is the owner's flags, not drawn.

**Wired** (`oag_fx::leach_strip`, `oag_raceplay::leach_strip`, `Style::leach_strip`, `beam.wesl`'s `leach`
branch): `WeaponModels::leach_strip` names the two textures on HD only; every craft's anchor trail is
recorded per tick render-side; ball 0.4 s, reveal 0.3 s, then held, nothing after a break; the strip
replaces Pulse's ribbon on HD (Pulse and Pure frames byte-identical, checked). Open and **chosen, not
measured**: the phases start at 0 when our beam starts; our progress is unwrapped track distance; the
single-level texture (the original has mips); the node at the walked end (`u` 0 in the emulation of
`0x001164e0`, accumulated here); the anchor falls back to the body's pose when a hull has no locator.
In a straight chase view the strip looked faint; **`hd-leach-bright` (below) found that was the
staging, not the strip**.

**Omega:** checked, differs (PS4 shaders); **2048:** has no LeachBeam strip of this family.

## 2026-10-08 (`hd-leach-bright`): why the strip read faint, and what the live draw says

Two boots of RPCS3 (`data/scratch/hd-leach-bright/c1`, `c2`; a held LeachBeam placed behind the
nearest rival, pushbuffer plus the strip's own memory read paused). Both agree on every row below.

**The faint streak was the staging, conf 70** (the original's straight-chase facing table is one frame; `t1`, `t2` and `t3` are the same paused frame): The `hd-leach-draw` comparison frame (`--force-leach-lock
276:1` at tick 330) renders a strip of **4 samples spaced 24-66 units apart** (the countdown, speed 0.7:
the leader's anchor trail has barely started, so the walk is one long segment plus the bend), where the
original's held beam has 22-38 samples about 3.5 apart. That strip is one flat quad nearly along the view
axis, so the program's `|dot(n, v)|` is near 0. `hd_leach_strip_ground_truth` uses the same 4-sample
staging; it still shows the strip but says nothing about its brightness. With a dense trail
(`--autopilot --force-leach-lock 800:3`, 18 samples over 83 units) ours draws a white-cyan thread toward the rival
(`ours/f800_3.png`, turbo streaks swamp it; the clean read is the strip on against off at identical sim state,
`ours/onoff.png`, below): a thread forward, but the larger share of its light is at the hull's tail.

**The per-node facing agrees.** Sum over the three fins of `|n . v|`, the program's own term, from the
original's dumped samples and its eye (`c[467]` of the draw), against ours from `--autopilot
--force-leach-lock 940:3` (19 samples over 67 units), nodes counted from the shooter:

| node from the shooter | 0 | 3 | 6 | 9 | 12 | 15 | far end |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| original (`c2/t1`, straight chase, 22 samples, 76 units) | 1.62 | 0.79 | 0.56 | 0.34 | 0.23 | 0.13 | 0.06 |
| ours (`ours/dbg_e3`, 19 samples, 67 units) | 1.80 | 1.12 | 0.71 | 0.30 | 0.18 | 0.13 | 0.07 |

The beam is thick and bright for the first 15-20 units and a thin thread beyond, in both. The fin
normals dumped from the draw match `oag_fx::rocket_smoke::fin_axes` on the matching fin (cosine 1.0; fin 0
flips sign between nodes, which `abs()` does not see), the half-width is 1.0 on every pair, `u` advances
0.0498 per unit, and the vertex colour is `ffffffff` (alpha 1) in state 4. Residual, **the open near-end
lead** (not attributed): the eye is **13.8-15.3** from the shooter's anchor in the original (three frames, `b0`,
`t1`, `v1`) and **7.7** in ours (eye to body 9.0), and that is where the brightness lives (sums 0.8-1.8 in the
first 15-20 units). Ours resolves the hull locator (`anchors[slot]` is `Some`) and puts the anchor 2.4 along the
model z and 2.25 along its y from the body origin. Projected into `t1` with the draw's own matrix (`c[256..259]`),
the original's anchor sits on the hull's rear engine cluster and the path then rises over the spine
(`c2/t1_anchor.png`). Strip on against off at identical sim state (`--autopilot --force-leach-lock 800:3`,
ticks 845-880, `ours/onoff.png`): our beam's light is a large blob at the hull's tail with a weak thread
forward. Whether the anchor sits at the right place on the hull is unmeasured: the body pose read in the same
paused frame did not pair with the anchor sample (`place.read_pose` came back 62 units off while the eye was
13.8 from the anchor), so the offset in the body frame is the next measurement.

**Read off the live draw, not on the static page** (conf 85, two boots):

| Item | Measured | Ours |
| --- | --- | --- |
| noise unit sampler | address word `0x60710101`: gamma field (bits 20-22) = 7, so R, G, B are sRGB-decoded before the program; the same word on the glow (unit 1) | the noise's red was used raw; **now decoded** in `beam.wesl` (`2 * srgb(noise.r)`), the glow's colour left as every HD ribbon has it (`decodes_source` false, the rocket-smoke precedent). The piecewise sRGB curve is **chosen, not measured** (the rest of the project uses 2.2) |
| both units' filter word | `0x02063e80`: mag LINEAR, min `LINEAR_LINEAR` (trilinear), LOD bias -1.5 | one level, linear. With a -1.5 bias the chain starts at its sharper levels, so the single level differs little (a 256 glow reaches 12.8 texels per unit at the near nodes); mips **not built**, open and low value |
| `u` at the walked end (idx 0) | 0.0 on the last node of both dumps, after a node at 3.6 and 7.6 | accumulated; **now 0** (`nodes()`), one segment sweeps the texture back as the original's does |
| `time` | `c[466]` = 140.64 (c1, `0x430ca3f9`) and 161.6 earlier: engine seconds | as before |
| program | the leach fragment program is the 18-instruction one with the facing term (found by its microcode in the dump); its address changes boot to boot | unchanged |

The noise change moves the glow lookup only (the decoded noise averages 0.10 against 0.33 raw): before/after
at four dense frames differ by 0 to 7,200 pixels with the frame's mean luminance unchanged to 0.01
(`data/scratch/hd-leach-bright/ba/`). It is a correctness change to the program's inputs, **not** the
brightness fix; the brightness gap was the staging.

**Three things this lane's tools got wrong, so the previous pages should be read with them:**

- `0x1824` (DRAW_INDEX_ARRAY) carries several 256-index batches in one method; `rsx-draw-list.py`
  and `rpcs3_draw_hook.py` kept only the last argument, so a draw's `idx` count and its dumped vertex
  range described the last batch only (a 21-segment strip showed as `idx122`, 8 nodes). Both now read
  every batch (`d["batches"]`, `idx` is the sum). The earlier dumps do not hold the near-shooter vertices.
- **fp addresses are not stable between boots.** At `0x00744c41` one boot held the leach program and
  another held `hd_waketrail`'s; identify a draw by its microcode, not by its address.
- The film's **side arcs** around the ship (thin white and violet crackle left and right) are not the
  strip: they are the per-craft `hd_waketrail` ribbons (draws with two 128x128 units, one per craft, program
  with the `0.1 * time` scroll and no facing term). Open for a lane of their own (HD wakes).

Not settled: the original reveals from sample 0 (the walked end) toward the shooter in a dump taken
mid-reveal; ours follows the static law (`frac <= lo` white), the same direction. The near-end anchor (above).

**Regression gate:** the Pulse frame of `hd-leach-draw` (`--force-leach-lock 276:1`, tick 330, 960x544) renders
sha `9d510ab01c91fcc2...` with this tree and with the tree before it, the same sha as that lane's reference.
**Omega:** checked, differs (PS4 shaders); **2048:** no such strip.
