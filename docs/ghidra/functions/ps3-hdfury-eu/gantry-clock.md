# The start gantry's clock: what drives HD's `GO` after the release

**2026-10-04, `hd-go-pulse` lane.** Binary `EBOOT.BIN` of `hdfury-ps3-eu-dec.iso`
(Ghidra `/hdfury/EBOOT-ps3-hdfury-eu.elf`), read on the GUI bridge. Static
reads only; matched against the 2026-10-04 RPCS3 capture in
[`start-gantry.md`](../../../rendering/start-gantry.md#hds-countdown-on-rpcs3-measured-2026-10-04).

**Answer.** The billboard's Edge curve is sampled at the gantry node's own
animation time. That time free-runs off a global clock, and the race manager's
per-frame update keeps it inside a window that depends on the player's lap,
resetting it to the window's start whenever it reads outside. Before the first
line crossing the window is **`[3.83, 5.25)` s**. On the release the time
(about 3.37 s, frame ~202, still the red board) is outside it, so it **jumps to
3.83 s (frame 229.8), where `GO` is already lit, and then loops every 86 ticks
at 60 Hz**. That is both mismatches the capture left open: `GO` bright on the
step frame, and the pulse phase. **Confidence 85** for the law (every link read
in the decompile and the disassembly, constants read as bit patterns, one
static identity argument below, no live watchpoint); the capture's three dark
centres agree with it to two ticks.

## The names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x002c0d78` | `AnimNode_GetTime` | 85 |
| `0x002be978` | `MeshImporter_SetTime` | 85 |
| `0x002be9c0` | `MeshImporter_AdvanceTime` | 80 |
| `0x0005e948` | `RaceManager_Update` | 72 |
| `0x00055488` | `RaceManager_ResetGantryTime` | 78 |
| `0x00056368` | `RaceManager_SetPhase` | 75 |

## The chain, link by link

### 1. The curve time is the gantry node's `+0xc0`

`Billboard_UpdateAndRender` (`0x003a5f68`, [billboards.md](billboards.md)) passes
`Billboard_UpdateInstanceUvs` its clock in `f1`, and `f1` comes from
`bl 0x0067a6f8` at `0x003a61b0` with `r3 = slot+0xc` (`lwz r3,0xc(r31)` at
`0x003a61ac`). `0x0067a6f8` is a TOC stub to `0x002c0d78`:

```c
float AnimNode_GetTime(node) {
    if (node->type != MeshImporter)           // +8 vs *0x008b3988 = opd 0x00875228
        node = first descendant of that type;  // _opd_FUN_006b2108, depth-first
    return *(float *)(node + 0xc0);
}
```

It is `AnimNode_FindTransformValueField` (`0x002c11c8`, [plasma.md](plasma.md))
returning the float instead of its address. The type tag `*0x008b3988` is the
one `MeshImporter`'s constructor (`0x002bef90`) stores at `+8`, which also
zeroes `+0xc0` (`param_1[0x30] = 0`).

**The debug override is idle.** `0x003a61b8`-`0x003a61cc` compare the float at
`*(TOC-0x6438)` = `0x008c3570` against the TOC float at `0x008b6fa4`
(`0xbf800000`, -1.0; the function's TOC is `0x008bd3c4`, from its OPD at
`0x0088b740`). Only when they differ is the clock replaced by that global and
the tree reset (`0x003a6844`). `0x008c3570` is initialised to `0xbf800000`, so
in a normal race the branch is not taken. Its writer (through the TOC slot
`0x008b7da4`) was not read.

### 2. `+0xc0` accumulates a global clock; slot `+0x40` sets it

`MeshImporter`'s vtable is `0x00869fd0` (`*0x008b3944`). Two of its OPDs:

- slot `+0x14` (`0x00869fe4` -> OPD `0x008823a0`) = `0x002be9c0`,
  `MeshImporter_AdvanceTime`: `dt = clock+0x44 - node+0x15c`, where `clock` is
  `*(TOC+0x645c)` or, when that is null, `*(TOC+0x6460)`; then `node+0xc0 += dt`
  unless the pause byte `+0x140` is set (`0x002bea24`-`0x002bea2c`), and
  `node+0x15c = clock+0x44`. No fmod, no loop, no clamp: the time is
  unbounded. It then samples the node's own animated children
  (`_opd_FUN_002deb68`) when `+0xc0` changed.
- slot `+0x40` (`0x0086a010` -> OPD `0x00882398`) = `0x002be978`,
  `MeshImporter_SetTime`: `stfs f1,0xc0(r3)`, then re-stamps `+0x15c` from the
  same clock so the next advance starts from zero.

So `AnimNode_UpdateTransformTree` (`0x002c1b30`, plasma.md), which calls slot
`+0x40` on every anim-class node of a tree with `f1` in hand, **is a set-time on
this class**, not a keyframe evaluator: plasma.md's 2026-09-25 reading resolved
the slot through the wrong vtable. The consequence for that thread, recorded
here and not chased: Plasma's and the missile's `UV_offset`/`Shockwave_scalar`
are bound to the node's animation time.

`AnimCurve_EvaluateChannels` then takes `fmodf(time, 13.333)`
([billboards.md](billboards.md)); 5.25 s is far inside that period, so the
fmod never acts on the gantry.

### 3. The race manager holds the gantry's time in a window

`RaceManager_Update` (`0x0005e948`, OPD `0x008723f0`, TOC `0x008ad4d8`; called
by twelve per-mode update functions, adds its `f1` to several race timers)
runs the block at `0x0005ee20`-`0x0005f178` in race phases 2 to 4 (the
`subi r0,r11,0x2; cmplwi r0,0x2` at `0x0005eb94`, `r11 = this+0x1970`; phase 2
takes a short detour through `0x0005ee04` first). With one viewport it reads
`T = AnimNode_GetTime(*(this+0x1950)+0x40)` (`0x0005f138`-`0x0005f140`), finds
the player's ship and its lap counter `ship+0x7810` (`0x0005ef70`-`0x0005ef78`,
the field `endrace-loyalty.md` reads as `laps completed + 1`), and picks a
window by lap. The bounds are TOC floats, read as bit patterns:

| Condition (`lap = ship+0x7810`, `total = *(gamestate+0xc)`) | Window `[from, to)` s | TOC slots | Bits |
| --- | --- | --- | --- |
| `lap == 0` (before the first line crossing) | `[3.83, 5.25)` | `0x008a6a74`, `0x008a6a90` | `0x40751eb8`, `0x40a80000` |
| `lap == total - 1` | `[9.5, 9.9)` | `0x008a6a94`, `0x008a6a78` | `0x41180000`, `0x411e6666` |
| `lap == total`, or `total <= ship+0x7814 - 2` | `[12.35, 13.3)` | `0x008a6a7c`, `0x008a6a80` | `0x4145999a`, `0x4154cccd` |
| any other lap | `[6.017, 9.3)` | `0x008a6a8c`, `0x008a6a88` | `0x40c08b44`, `0x4114cccd` |

If `T < from` or `T >= to` it calls `AnimNode_UpdateTransformTree(from,
node)` (`0x0005f694`, `0x0005f6ac`, `0x0005f760`), i.e. `SetTime(from)`. The
`[12.35, 13.3)` branch also queues an announcer cue once per player
(`0x0005f0e0`, `Sound_QueueAnnouncerCue`), and its string `*0x008a6a84` =
`0x0077ce90` is **`FINAL_LAP`**: that branch is the final lap, so `lap` counts
the current lap from 1 and `lap == total - 1` is the lap before it. The `lap == 0` window also sets a second tree,
`*(*TOC-0x6c48)+0x2ac -> +0xa4`, to the same time (`0x00083a58`), not
identified. With split screen (`gamestate+0xe4 > 1`) each player keeps its own
float at `this+0x2dd0` instead of the shared node; not followed.

**Which board state each later window shows is not settled.** On Pulse's
timeline 9.333 s is `Final_Lap`'s first key and 12.33 s the chequered state's
([start-gantry.md](../../../rendering/start-gantry.md)), but the `FINAL_LAP`
cue sits in the `[12.35, 13.3)` branch, one lap after the window that lands
near 9.333 s. HD's own asset timeline at those times was not walked for this
page. These windows are the triggers for the gantry's later states, which this
project still clips away (`oag_render::gantry::clip_to_panel`). Not wired: see
below.

### 4. `this+0x1950` is the billboard's own node

`RaceManager_Construct` (`0x0005d478`-`0x0005d488`) stores
`*(*(TOC-0x6bc0) + 0x20)` at `+0x1950`, `*(TOC-0x6bc0)` being `0x00ad73b4`, and
immediately calls `SetTime(0.0)` on its `+0x40` (`0x0005d4a0`-`0x0005d4b0`).
`Billboard_ConstructResource` (`0x0029a6a8`) registers itself at
`*0x008b2d70 + 4 + Num*4` = `0x00ad73b4 + Num*4` (decompile line
`*(puVar8 + param_4*4 + 4) = param_1`), stores the loaded `.vex` resource at its
own `+0x40` (`param_1[0x10]`), and calls `Billboard_LoadModelAndBind(Num, path,
this, this[0x10])` through `0x006791d8`, which stores that fourth argument at
`slot+0xc`. So `0x00ad73b4 + 0x20` is billboard `Num == 8`, the gantry, and the
node the race manager sets is the node `Billboard_UpdateAndRender` reads.
**Confidence 85 on the identity**, static only; a live read of both pointers
would make it 90+.

### 5. What zeroes it

`RaceManager_ResetGantryTime` (`0x00055488`) calls `SetTime(*0x008a6860)` on
the same node, and `*0x008a6860` is `0.0`. Its one direct caller is
`0x00043510` (a restart-shaped routine that reseeds and re-resolves a team
record), and it is also an OPD (`0x00872268`) in a vtable. **When in the
countdown that runs was not traced**, so the start tick is not read from code.

`RaceManager_SetPhase` (`0x00056368`) stores `+0x1970` and clears the timers at
`+0x1968`/`+0x196c`/`+0x1974`; phases 0-4 dispatch through a jump table at
`*(TOC-0x6c00)` that was not walked. **That phase 2 starts on the release is
the capture, not the code**: the green step lands on the race clock's zero and
the craft's first movement, within one 30 fps frame (confidence 75).

## Against the capture

Under this law, release on tick 273, the time on tick `273 + k` is
`3.83 + (k mod 86) / 60`. The asset's own lit-white samples (the ground-truth
test's `lit_white`) are zero for `k` in 18-30, 59-67 and 104-116, centres +24,
+63, +110. The capture's dark centres, both boots: **+23, +61, +109**. The held
loop this replaces gave +50, +89, +129. Side by side at matched ticks:
`data/scratch/hd-go-pulse/strip_a_step.png` and `strip_b_pulses.png` (local
scratch, not committed).

One tick is soft: the window check runs in the race manager and the advance
in the node's own update, and their order inside a frame was not read. Either
order gives an 86-tick loop. They differ only in whether the reset frame
shows 3.83 or 3.83 + dt.

## Open

- The lap windows are not played. Ours keeps looping `GO` after the line
  crossing, where the original jumps to 6.017 s. Wiring them needs HD's own
  asset states at 6.017-13.3 s read, `ship+0x7810` reconciled with
  `standing.lap`, and the states `clip_to_panel` removes put back.
- What calls `RaceManager_ResetGantryTime` during a countdown, and so the start
  tick in code. The capture bounds 70 from below only, and two cues point
  later: at tick 271 our backdrop is already part way to green where the
  original is red 2 ticks before the step, and the capture's digits come
  about 13 ticks later than ours.
- The phase jump table at `*(TOC-0x6c00)`.
