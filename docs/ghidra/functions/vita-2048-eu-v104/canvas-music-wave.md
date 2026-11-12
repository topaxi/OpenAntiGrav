# The 2048 campaign map: the music wave, the hidden events and the tap that finds them

Functions in `eboot.elf` (Wipeout 2048, Vita, `PCSF00007` patch v1.04), image base `0x81000000`,
Ghidra program `/2048/eboot-vita-2048-eu-v104.elf`. 2026-11-12, lane `w2048-lattice-reveal-re`.
Static reading only (decompile and disassembly); **no live Vita3K run backs any row**. Decompile-only
rows are scored on the decompile (an unambiguous few-line function at 85, as elsewhere on this
binary's pages); the `.fft` file census is an arithmetic invariant over every file that ships and is
scored as such.

Question (the maintainer, from play): on the campaign map the lattice moves with the music; the
fifteen hidden prototype events (`m_options & 0x10`) stay still, show nothing until the player
taps their spot, and then draw a tile. The cell texture and the mesh are
[canvas3d-lattice.md](canvas3d-lattice.md); the tiles, rings and the state table are
[campaign-map-projection.md](campaign-map-projection.md).

## 1. The bounce: a precomputed spectrum, not a live analysis

### The data: one `.fft` file beside every music track (confidence 92)

Every `music_stereo.at9` and `frontend_stereo.at9` in the base `data.psarc` has a sibling
`*_stereo.fft`. The name comes from `"%s\%s_stereo%s"` with `".fft"` (`0x814c3d38`, `0x814c3d30`,
the `MusicManager.cpp` string block). Layout, little-endian:

| Offset | Type | Value |
| --- | --- | --- |
| `+0x0` | `char[4]` | `AFEX` (compared at `0x8125f1de` against `0x814c3d04`) |
| `+0x4` | `u32` | 1 |
| `+0x8` | `u32` | bands, 12 in every file |
| `+0xc` | `u32` | frames |
| `+0x10` | `f32[frames][bands]` | one row of 12 levels per frame, 0 to 1 |

**Census: 13 of 13 files** (`data/audio/music/01` to `11`, `FEMusic`, `FEDemoMusic`) satisfy
`size = 16 + 4 * bands * frames` exactly, and `frames / 30` is the track's length (`FEMusic`
9060 frames = 302.0 s, the loop's measured 302 s in `docs/formats/2048-status.md`; track `01` 9031
= 301.0 s). So a row is one 30th of a second. In every file each band's maximum over the track is
at most 1.0 and at least one band reaches 1.0: the data is normalised per track. Band means for
`FEMusic`, low to high: 0.134, 0.084, 0.089, 0.107, 0.058, 0.039, 0.025, 0.013, 0.015, 0.040,
0.019, 0.004. Reproduce with any PSARC reader: list `data.psarc` entries ending `.fft` and check
the header and the size identity.

### The analyser (`g_MusicAnalysis`, `0x818c3d58`)

| Address | Name | Conf | What it does |
| --- | --- | --- | --- |
| `0x8125f686` | `MusicAnalysis_StaticInit` | 75 | Zeroes the object, mutex `"MusicAnalysisMutex"`, constants `+0x860` = 4.0, `+0x864` = 6.0, `+0x868` = 0.05, per-band gains `+0x820..` = 1.0 |
| `0x8125f1de` | `MusicAnalysis_LoadFft` | 80 | Opens the file, reads the 16-byte header to `+4..+0x13`, checks `AFEX`, allocates and reads `bands * frames` floats to `+0x14`, sets the loaded flag `+0` |
| `0x8125f372` | `MusicAnalysis_SampleAt` | 75 | `(time, obj)`: row `i = uint(time * 30)`, linear blend of rows `i` and `i + 1` by the fraction into `+0x1c..` (12 current levels); time past the end or negative zeroes them |
| `0x8125f460` | `MusicAnalysis_Update` | 70 | `(dt, obj, advance)`: per band an automatic gain (below), then pushes one 64-byte row of 16 floats into a **30-row ring** at `+0x5c`, write index `+0x7dc` |
| `0x8125f66c` | `MusicAnalysis_GetHistoryRow` | 85 | `(obj, k)`: the row written `k` updates ago, `k` clamped to 29 (`+0x5c + ((idx - k) mod 30) * 0x40`) |

`FUN_81264c44`, called from the music player's per-frame update (`0x8125fcfa`), samples the
playing stream's position into `MusicAnalysis_SampleAt` and then calls `MusicAnalysis_Update`.
The loader runs on the stream thread (`FUN_8126494c`).

The gain (confidence 60; the vector ops were read, the peak-hold row was not separated from a
decompile artefact): with `v = level * gain[k]`, the gain falls by `dt / 0.05` while `v >= 0.85`
(floor 1.0) and rises by `dt / 6.0` while `level > 0.005` (cap 8.0); the stored value is
`min(v, 1)`, held on a fall with a fall rate that grows by `4.0 * dt` per frame. The decompile
compares against the ring row being overwritten (30 updates old), which looks wrong; read the
disassembly of `0x8125f460` before porting the hold.

### The wave (`Canvas3D_UpdateCellHeights` `0x81071794`, music branch; confidence 80 for the law)

Runs when `canvas+0x5ed40` (on) and `g_MusicAnalysis.loaded` (`0x818c3d58`) are both set. Centre
`C = (canvas+0x5ed44, +0x5ed48)` and radius `R = canvas+0x5ed50`, in lattice units. For every cell
`(col, row)` inside the box `[C - R, C + R]` whose priority (`canvas + 0x2f0c4 + 4 * (row * 401 +
col)`) is **below 5**:

```text
d2 = (col * 0.5 - C.x)^2 + (row * 0.8660254 - C.y)^2
f  = clamp((R^2 - d2) / R^2, 0, 1)
k  = min(uint((1 - f) * 30), 29)
a' = clamp(a + int(600 * f * (h[k].band1 - h[k + 1].band1)), 0, 255)
```

with `h[k]` the analyser's row `k` updates ago and `band1` the row's second float (`+4`, read at
`0x810719fc` and `0x81071a0c`: `adds r0, #0x4`), `600` the literal `0x44160000` (`0x81071a14`), and
`a` the cell's alpha byte (`cell + 0xe7`), the height the vertex program raises the cell by
([canvas3d-lattice.md](canvas3d-lattice.md), "What the shaders compute"). Because each frame adds
the one-frame difference of a delayed signal, the sum telescopes: **a cell at distance `d` follows
band 1 delayed by `30 * d^2 / R^2` updates and scaled by `600 * f`**, so the band-1 envelope runs
outward from the centre as a ring and fades to nothing at `R`. Band 1 is the second-lowest of the
12 (the bass).

**Where it runs** (confidence 75). `TouchCampaign_Update` (`0x810f2f04`, at `0x810f34b2..0x810f352e`)
takes the current event (the one `TouchCampaign_ApplyEventStates` picks: `*(0x81545468)+0x32588`
when set, else the first event from `"2048 - Event 1"` along `+0x2cc` below progress 3), its canvas
button (`event+0x1ec`), and writes: ripple off (`+0x5ed24` = 0, `+0x5ed3c` = 0), music on
(`+0x5ed40` = 1), `C` = the button's centre (vtable `+0x14`, `CanvasButton_GetCentre` `0x813c525c`:
`((col + 1) * 0.5, (row + 0.5) * 0.8660254)`, the hexagon's centre), `R` = the button's `+0x30`,
**7.0** (stored by the map builder `FUN_810f041e` at `0x810f0a58..0x810f0a60`, `movt r2, #0x40e0`).
The other tiers' pages (`FUN_8110213a` at `0x811025be..`, and the sites `0x810f63ae`, `0x810f9508`)
do the same; `TouchCampaign_SetMapActive` (`0x810efff6`, `0x810f03f0`) stores its show flag into
`+0x5ed40`, so the wave stops when the map is hidden. **The whole map does not bounce: only cells
within 7 units (14 columns, about 8 rows) of the current event's tile.**

**Decay and upload** (`Canvas3D_Update` `0x81072220`, confidence 80): when `canvas+0x5e0c8 <= 0`, a
time accumulator `0x8151d3f8 += dt` gives `n = uint(acc * 230) & 0xff` (the remainder kept), and every
cell of the 90 x 32 mesh window loses `n` from its alpha (floor 0): **230 alpha units a second**.
Then each `canvasTex` texel's alpha moves **15 % of the way** to the cell's alpha per frame
(`0.15`, frame-rate dependent: the original eases per frame, not per second).

**Wave speed**: 7 units per 30 analyser updates. The analyser updates once per music-player
update; that the player updates once per front-end frame at 60 Hz is inferred from the 60 Hz
eases measured in [campaign-map-projection.md](campaign-map-projection.md), "The scroll", not
measured on this path (confidence 60): 7 units in 0.5 s.

### The ripple (`+0x5ed24`): writer not found

The same function's other branch is a radial ripple: `t = +0x5ed3c += dt` while below the duration
`+0x5ed38`, centre `+0x5ed28/+0x5ed2c`, radius `+0x5ed34`, and

```text
a = clamp(int(min(4 t, 1) * clamp((T - t) / T) * min((R^2 - d2) / R^2, 1) * 128 * (g(d2 * 0.15 - 5.5 t - pi) + 1)), 0, 255)
```

(set, not added; `g` is `SceLibm_4A496BC0`, a sine or cosine, not identified). **Nothing found turns
it on**: every `movw #0xed24` site in the program was read (`0x810f34d4`, `0x811025e0`, `0x811028aa`,
`0x810f63ae`, `0x810f9508` write 0; `0x81070dca`, `0x81072eee` are the constructor and the static
initialiser; the rest are other objects' fields). Not to be wired until a writer is found.

## 2. A tap on the lattice (`Canvas3D_TapCell` `0x81071de4`; confidence 80)

`Canvas3D_Update` (`0x81072220`), while `canvas+0x5ed20` is set and `FUN_810f2154()` returns 0, turns
each touch of the pad record (`FUN_8122d664(DAT_818b9ea8, 0)`: a single touch at `+0x371`/`+0x37a`/
`+0x37c`, then a list of `+0x2ec` points at `+0x2f0`) into a cell with `Canvas3D_PickCellAtTouch`
(`0x810715d2`: unprojects the point through the canvas camera to the plane `z = 0.5`, `col =
uint(x / 0.5)` capped 400, `row = uint(y / 0.8660254)` capped 119) and calls `Canvas3D_TapCell(canvas,
cell)`:

1. It asks each of the 700 canvas items whether it owns the cell (vtable `+0x18`; for the hex button
   `CanvasButton_OwnsCell` `0x813c5134`: **0 when the button's state `+0x3c` is 0**, else whether the
   cell is one of its six).
2. **An owner**: state 1 or 2: nothing. If `canvas+0x5e0c4` names another item: nothing. **State 3:
   `CanvasItem_SetState(button, 2)` and the owning event's vtable `+0x70(2, 0)`**, i.e.
   `GameModeBase_SetProgress(event, 2, false)`. That is the reveal.
3. **No owner**: the ground bump. Around the cell, every cell whose priority is below 5 gains alpha
   up to a cap (gain / cap per tap): on the cell's row, columns -3 to +3: 5/50, 10/150, 16/200,
   **20/250**, 16/200, 10/150, 5/50; on the rows above and below, columns -2 to +2: 5/50, 8/100,
   12/150, 8/100, 5/50; two rows away, columns -1 to +1: 4/40, 8/80, 4/40. Stored as the last tapped
   cell at `canvas+0x5ebc4`.

The touch space: `PickCellAtTouch` with its fourth argument 1 (both call sites) maps a 1920 x 780
window at `y - 110` (the first branch is 1920 x 1088); which Vita panel fills the pad record's touch
fields is **not read** (confidence 40 that it is the rear pad). A port's mouse click on a hexagon is
our input choice either way.

## 3. What the hidden events are (`GameModeBase_SetProgress`, `Campaign_UnlockEvents`)

**Progress word `+0x2d8` to button state** (`TouchCampaign_ApplyEventStates` `0x810ef78c`, read again
here, confidence 80): 0 with the lock test (`m_options & 0x10`) set, state 0; 0 open, state 1;
**1, state 3**; 2, state 2 grey; 3 green; 4 yellow.

**Who writes 1** (`Campaign_UnlockEvents` `0x812b5b6c`, confidence 80): for every event with
progress 0, not skipped (`+0x2e5`, `FUN_812b40b4`) and unlockable (`GameModeBase_IsUnlockable`
`0x812b3a76`), it calls `SetProgress(1)` when the lock test (vtable `+0x74`, `m_options & 0x10`) or
`m_options & 8` is set, and `SetProgress(2)` otherwise. It runs from the map builder (`FUN_810f041e`,
`0x810f0800`) and from the race-end hook (`FUN_812b4872`, `0x812b49d8`). So **a hidden prototype event
goes to progress 1, state 3, the moment it becomes unlockable**, and only the tap takes it to 2.

**Unlockable** (`0x812b3a76`, confidence 80): `+0x2e4` set (likely `M_bForceAlwaysUnlocked`, 55),
or both

- `profile+0x3257c` (the XP total, [xp-award.md](xp-award.md)) `>= Profile_GetRankThresholdXP(profile,
  m_RankRequired)` (`0x8101361a`: 0 for rank 0, the `XPRanks.xml` threshold `table[rank - 1]` for
  1 to 50, the XP total itself above 50); `m_RankRequired` is `+0x2e0` (`GameModeBase_RegisterFields`
  registers the string `0x814dc390` at offset `0x2e0`, `0x812b1eb0`, confidence 90); and
- `+0x2d4` (the required event) is null or has progress 3 or more.

So **the gate is the player's XP rank**: `Profile_GetXPRank(xp) >= m_RankRequired`, the rank `XPRanks.xml`
gives (rank 10 = 420,000 XP, 20 = 1,020,000, 30 = 1,720,000, 50 = 3,580,000). The fifteen events'
`m_RankRequired` is 10 to 50 (Ship Challenges) and 25 to 34 (Phantom Challenges)
([2048-campaign.md](../../../formats/2048-campaign.md)).

**`GameModeBase_SetProgress`** (`0x812b3f90`, disassembled, confidence 80): `(event, value, force)`;
saves the old value at `+0x2dc`; with `force` stores and returns 1; refuses a lower value (returns 0);
on a rise to 3 calls `Profile_AwardXP(profile, 0)` (`0x810134ce`) and on a rise to 4 award 1 (plus
award 0 if it was below 3); which `XPAwards.xml` rows indices 0 and 1 are was not checked; crossing from below 3 to 3 or more opens `+0x2cc` and `+0x2d0` (next and
branch) with `SetProgress(2)` if they are at 0; **stores the value before** the unlock test, which
only decides the return value; then `Campaign_AwardProgressTrophies` (`0x812b3b3c`, platform trophies
by event name and mode, confidence 65). This is the writer of `+0x2d8` the earlier sweep missed (it
uses `str.w r5, [r4, #0x2d8]`, at `0x812b3ff6`, `0x812b4074`, `0x812b409c`, `0x812b40aa`).

**Persistence: unread.** `+0x2d8` is not a registered XML field (`GameModeBase_RegisterFields`
registers `+0x2d0` and `+0x2d4`, not `+0x2d8`), so the save writes it through another path. Recipe:
a Vita3K write watchpoint, or a memory diff of the profile save before and after a reveal; the other
`str.w [*, #0x2d8]` sites (`0x812b0a76`, `0x812b0a92` in `FUN_812b0a0e`, which sets 3 or 4 from a
trophy-like query for `m_options & 4` events) are not the serialiser.

## 4. Is a hidden event still? Four cases, not one

| Event | Button state | Painted | Music wave | Tap |
| --- | --- | --- | --- | --- |
| Hidden, not unlockable (progress 0, `m_options & 0x10`) | 0 | no | **moves**, plain ground | ground bump (`OwnsCell` is 0 at state 0) |
| Hidden, unlockable (progress 1) | 3 | no (`Canvas3D_PaintItems` `0x810712d2` calls only vtable `+8`, `CanvasItem_Paint`, which paints states 1 and 2 only) | written per cell, then **flattened** | reveal |
| Found, open (progress 2) | 2 | grey prism, priority 7 | none (priority 7 is not below 5) | selects (`TouchCampaign_SelectEvent`) |
| Any tile, state 1 or 2 | 1, 2 | priority 6 or 7 | none | nothing on the canvas |

**The flattening** (`CanvasButton_FlattenCells` `0x813c5164`, vtable `+0x1c`, confidence 85 on the
code): every frame, for a button whose state is not 0, `Canvas3D_Update` calls it after the decay and
upload loop; it averages the alpha of its six cells and writes the mean back to all six. So an
unlockable hidden event **is not motionless**: it rises and falls with the wave as one flat hexagon
while the cells around it ripple individually, its six triangles never differ in height, and the
vertex program's occlusion shades none of its inner vertices. That this is "how a player spots
them" is an inference, confidence 55. Outside radius 7 of the current event nothing moves at all,
hidden or not.

The state-3 ring blink of [campaign-map-projection.md](campaign-map-projection.md) ("`+0x2d8` = 1,
state 3, the ring blinks"): `CanvasButton_PaintIconAndRing` does swap modes 3 and 4 for a state-3
button, but it is reached only from `CanvasItem_Paint`, which returns early for state 3, and the chain
walks do not give the side challenges a ring mode. **A state-3 button draws nothing** (confidence 75).

**The found tile** is the ordinary state-2 tile: the grey (`0x817327cc`) hexagonal prism of
[campaign-map-projection.md](campaign-map-projection.md), "The prism", with its icon (the icon writer
runs for state 2). No hexagon-or-triangle choice is made per event: the canvas button is always the
3 x 2 hexagon (`CanvasButton_BuildCellList` `0x813c529c`: cells `(col..col+2) x (row, row+1)`).

## Names

| Address | Name | Conf | Evidence |
| --- | --- | --- | --- |
| `0x81071de4` | `Canvas3D_TapCell` | 80 | Section 2, decompiled |
| `0x810715d2` | `Canvas3D_PickCellAtTouch` | 65 | Unprojection and caps decompiled; the touch space is open |
| `0x813c5134` | `CanvasButton_OwnsCell` | 85 | Six-line decompile, vtable `0x8150abb8` slot 6 |
| `0x813c5164` | `CanvasButton_FlattenCells` | 85 | Decompiled, vtable slot 7, the only caller loop in `Canvas3D_Update` |
| `0x813c525c` | `CanvasButton_GetCentre` | 85 | Decompiled, vtable slot 5 |
| `0x813c529c` | `CanvasButton_BuildCellList` | 85 | Decompiled, vtable slot 4 |
| `0x812b3f90` | `GameModeBase_SetProgress` | 80 | Disassembled; called as vtable `+0x70` (`0x81515ff8` in the base vtable) |
| `0x812b3a76` | `GameModeBase_IsUnlockable` | 80 | Decompiled; same test inlined in `SetProgress` |
| `0x812b5b6c` | `Campaign_UnlockEvents` | 80 | Decompiled |
| `0x812b3b3c` | `Campaign_AwardProgressTrophies` | 65 | Decompiled; the trophy object `0x818ba6c8` is not read |
| `0x8101361a` | `Profile_GetRankThresholdXP` | 80 | Decompiled; table written by `XPRanks_Load` (`0x81012ba0`) |
| `0x8125f686` | `MusicAnalysis_StaticInit` | 75 | Decompiled |
| `0x8125f1de` | `MusicAnalysis_LoadFft` | 80 | Decompiled; matches the 13 files |
| `0x8125f372` | `MusicAnalysis_SampleAt` | 75 | Decompiled |
| `0x8125f460` | `MusicAnalysis_Update` | 70 | Decompiled; hold detail at 60 |
| `0x8125f66c` | `MusicAnalysis_GetHistoryRow` | 85 | Five-line decompile |
| `0x818c3d58` | `g_MusicAnalysis` | 80 | Data: the analyser object |

`Canvas3D_UpdateCellHeights` (`0x81071794`, row on [canvas3d-lattice.md](canvas3d-lattice.md) at 60)
is read in full here, both branches, the music one checked in disassembly: 80 on this page's
evidence. Its row is left to that page's owner.

## Corrections to other pages

- [xp-award.md](xp-award.md), "What XP gates": **a rank does gate an unlock**: the fifteen side
  events, through `m_RankRequired` and `GameModeBase_IsUnlockable` (section 3).
- [campaign-map-projection.md](campaign-map-projection.md), "The ring": the rank hypothesis (45) is
  read (80), and a progress-1 (state 3) button draws no ring.
- [2048-campaign.md](../../../formats/2048-campaign.md), "The unlock graph": the "likely also
  rank-gates them" is read: it does, and they then wait at progress 1 for a tap.

## What to wire, in order

1. The XP total and the rank (`XPRanks.xml`); without it no hidden event becomes revealable.
2. `Campaign_UnlockEvents` as a rule: progress 0 to 1 for a hidden event, 0 to 2 for any other, on
   map open and at race end.
3. The reveal: a click on any of a state-3 event's six cells sets progress 2, persisted in the
   profile; the event then draws as the grey state-2 tile and joins pad navigation.
4. The wave: the music path exposes the playing position each frame (an output, ADR-0018); the front
   end samples the track's own `.fft` at `position * 30` (never a spectrum computed from PCM), keeps
   the 30-row band-1 history, and applies the law above around the current event with `R = 7`, the
   230/s decay, the 15 % per-frame ease and the six-cell average for state-3 buttons. The baked
   lattice picture inside that disc becomes per frame (a render cost to plan).

## Omega

**Checked, differs.** Omega's `eboot.bin` carries `MusicAnalysisMutex` (`0x01838d2d`),
`s_musicAnalysisTDMutex` and the `"%s\%s_stereo%s"` pattern (`0x01838d40`), so the analyser code
ships. But **no `.fft` entry exists** in any of the nine merged packages (`data00` to `data09`,
TOC listing, 46,014 entries; no `frontend_stereo` either): Omega's music is Wwise
([omega-status.md](../../../formats/omega-status.md), "Music plays"), so the analyser has no file to
load and the music branch's `loaded` test fails. Whether Omega's front end shows this map at all is
open (its front end is HD's). HD: **checked, differs** (no `Canvas3D`,
[canvas3d-lattice.md](canvas3d-lattice.md)).
