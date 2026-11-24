# `HexSelection`: Ship Select's team ring, its pulse and the lattice slide on a step

2026-11-24 (`shipselect-ring-re`). The `NAVIGATE TEAM` honeycomb on Team Selection is the widget class
`HexSelection`. This page reads its code: what the selection ring is, how it pulses, why it looks
displaced on a step, and how the lattice slides one column per team step. Everything is read statically
from `/hdfury/EBOOT-ps3-hdfury-eu.elf` (GUI bridge, `program=` by path; `scripts/ps3-toc.py toc` says
`exact`, TOC `0x008ad4d8`, for every function named below) and checked against the step grabs on
[team-selection-model.md](team-selection-model.md), "The banner and the hex column on a step". No
breakpoint ran (RPCS3 was in use by another lane): nothing here is above 85.

## How the class was found

`scripts/ps3-toc.py attrib 0x0078b348` (the string `HexSelection`) gives one user, `0x001b0b58`, the
static registration: it builds the prototype at `0x009a5ad4` with the class name and stores the vtable
`0x00866120` (TOC slots `-0x44`, `-0x40`, `-0x4c`). Vtable words resolve through OPD entries:

| Slot | OPD | Code | Role |
| --- | --- | --- | --- |
| 3 | `0x0087a9c0` | `0x001b10f8` | `HexSelection_Update` (same slot as `Image_Update`, [image-widget.md](image-widget.md)) |
| 12 | `0x0087a958` | `0x001b0620` | unread |
| 13 | `0x0087a960` | `0x001b0650` | destructor (restores the vtable, calls the base) |
| 14 | `0x0087a9e0` | `0x001b2360` | `HexSelection_Build` |
| 15 | `0x0087a9f0` | `0x001b2a48` | `HexSelection_Destroy` (releases every child, frees the arrays) |
| 17 | `0x0087a9b0` | `0x00699a70` | `HexSelection_Construct` (the factory clone) |
| 18 | `0x0087a950` | `0x001b03f0` | `HexSelection_ParseXml` |

`TrackHexSelection` (string `0x00791bd8`, registration `0x00209f68`, vtable `0x00868090`) is a
subclass: its slot 18 is the same `0x001b03f0`, its slot 3 is `0x002096f0`, which calls
`HexSelection_Update` directly (`bl` at `0x00209cc4`) and `HexSelection_Step` (`bl` at `0x002099ec`).
That is the second, independent call site for both prototypes. **Track Select's grid therefore runs the
same pulse, ease and slide** (scope for a wire lane; not measured on Track Select).

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x001b0b58` | `HexSelection_RegisterPrototype` | 80 |
| `0x00699a70` | `HexSelection_Construct` | 80 |
| `0x001b03f0` | `HexSelection_ParseXml` | 85 |
| `0x001b2360` | `HexSelection_Build` | 80 |
| `0x001b10f8` | `HexSelection_Update` | 85 |
| `0x001b09b8` | `HexSelection_Step` | 85 |
| `0x001b0920` | `HexSelection_SetRow` | 75 |
| `0x001b0c28` | `HexSelection_SetCell` | 70 |
| `0x001b0320` | `HexSelection_ClearCell` | 70 |
| `0x001b2a48` | `HexSelection_Destroy` | 70 |
| `0x002096f0` | `TrackHexSelection_Update` | 70 |
| `0x00200498` | `Colour_LerpArgb` | 85 |

Scores: decompilation unambiguous, two call sites (the Team screen and `TrackHexSelection`) and Omega's
own copy (below) agree, and the slide's 6 frames land on the 84 grabs' frames 4 to 10. Short of 90 for
want of a watchpoint. `_SetCell` and `_ClearCell` take part of their arguments on the stack and are read
only as far as the colours.

## Fields

`st` is the `HexSelection` object (`0x110` bytes, `HexSelection_Construct` zeroes it).

| Offset | Meaning | Source |
| --- | --- | --- |
| `+0xa4/+0xa8/+0xac` | textures `Hexagon_HD_Faded.gtf`, `hexlock_HD.gtf`, `Hexagon_HD_THICK_OUT.gtf` | `_Build`, TOC `-0x20`, `-0x1c`, `-0x18` |
| `+0xb0` | the ring: one `Image` child (z = widget z + 50) | `_Build` |
| `+0xb4` | column parity bit (which columns sit half a row low) | `_Step` toggles it |
| `+0xb8` | `Columns` | XML |
| `+0xbc` | `Columns + 1` (one spare column for the slide) | `_Build` |
| `+0xc0` | `Rows` | XML |
| `+0xc4`, `+0xc8` | per column: cell triples, column `Image` | `_Build` |
| `+0xcc` | centre column `int((Columns - 1) * 0.5)` (2 for 5) | `_Build`, TOC `-0x30` = 0.5 |
| `+0xd0` | ring row | `_SetRow` |
| `+0xd4`, `+0xd8`, `+0xdc` | column pitch **72.0**, row pitch **83.0**, half row **42.0** | `_Build` literals `0x42900000`, `0x42a60000`, `0x42280000` |
| `+0xe0` | pulse phase | `_Update` only |
| `+0xe4`, `+0xe8` | ring target x, y | `_Step`, `_SetRow`, `_Build` |
| `+0xec` | slide timer, seconds | `_Step` sets 0.1, `_Update` counts down |
| `+0xf0` | slide direction, -1 or +1 | `_Step` |
| `+0xf4/+0xf8/+0xfc/+0x100` | `SelectedLockCol`, `SelectedLockColFade`, `HexCol`, `HexColFade` | XML; defaults `0xff808080`, `0xff161616`, `0x64808080`, `0x32808080` |
| `+0x104` | colour handle of `HD_Blue` | `_Construct`: `FUN_0015b778(TOC -0x48 = "HD_Blue")` |
| `+0x108` | number of entries (teams) | the screen |
| `+0x10c` | built/active | `_Build` sets, `_Destroy` clears |

**Authored data read.** `HexSelection_ParseXml` matches exactly `Columns`, `Rows`, `SelectedLockCol`,
`SelectedLockColFade`, `HexCol`, `HexColFade` (TOC `-0x64..-0x50`). **`SelectedColumnCol` is not a key**:
the XML's `0x64ff0000` is dead. The selected column's fill is `HD_Blue` (`+0x104`), which the Fury archive
serves as `0xffac0717` (172, 7, 23) and the HD style as `0xff8ac0ca` (teal) ([hd-frontend.md](../../../formats/hd-frontend.md)).
The settled frame's centre cell reads (151, 6, 20) ([campaign-screens.md](../../../ui/campaign-screens.md),
"The `NAVIGATE TEAM` honeycomb"): 0.88, 0.86, 0.87 of `HD_Blue` on the three channels, one common
factor (the `Faded` art's coverage, unread), where `0x64ff0000` drawn twice cannot give a green or blue
term at all.

## The ring

`_Build` (`0x001b2360`) creates the ring as one `Image` (`+0xb0`) on `Hexagon_HD_THICK_OUT.gtf`, quad
92 x 80 (`+0xac`, `+0xb0` of the image), UV window 74 x 64 texels (`+0xd4`, `+0xd8`), UV origin 0. It
is a child of the selection, not of a column: its offset is `(cc * 72, row * 83 + 42 * odd)` where
`odd = (cc odd) XOR parity`. The cells use the same 92 x 80 quad (`Faded` 74 x 64 texels, the lock
75 x 64).

**One quad of the thick outline art**, not the thin `Hexagon_HD_OUTLINE.gtf` repeated: the 6 px pale
band ours reproduces by stacking six inset outlines is this texture's own thickness (both ship in
`DATA02.PSARC`; `Hexagon_HD_Faded.gtf` in `DATA06.PSARC`).

### The pulse (`HexSelection_Update`, `0x001b10f8`, `0x001b1188..0x001b1250`)

```
p += dt * 3.0                      // TOC -0x28 = 3.0; dt is the update's seconds, as for Image_Update
if p > 2.0: p -= 2.0               // TOC -0x2c
if p < 1.0: c = lerp(p,     0xffffffff, HD_Blue)      // white -> HD_Blue
else:       c = lerp(p - 1, HD_Blue,    0xff000000)   // HD_Blue -> black
Image_SetVertexColours(ring, c)
```

`Colour_LerpArgb` (`0x00200498`) is a per-byte `a + t (b - a)`, truncated, alpha included. So the ring's
colour is a sawtooth of period **2/3 s (40 frames at 60 Hz)**: white, through pink, to `HD_Blue`
at 1/3 s, through dark red to black at 2/3 s, then a hard cut back to white. Nothing else writes `+0xe0`
(not `_Step`, not `_Build`; `_Construct` zeroes it), so **the phase runs free from the widget's
construction and a step does not restart it**. The 20 ring grabs (white at 19, 41, 69; dark red at 21;
"absent" at 11 to 17 and 23) come from different steps at unrelated phases, so they neither confirm nor
refute the period. The colours they do test. The ring's pale band in the settled frame
([campaign-screens.md](../../../ui/campaign-screens.md), "The cursor ring") reads (200, 92, 102) on the
sides: on the line `lerp(white, (172, 7, 23), t)` that is t = 0.663, 0.657, 0.659 from R, G and B, one t
for all three channels; the flats (193, 81, 91) give 0.75, 0.70, 0.71, and `Track Creation`'s band
(223, 169, 174) 0.39, 0.35, 0.35. An additive ring over the cell's (149, 6, 20) would need (51, 86, 82),
green above red, which is off that line: the ring replaces the pixels it covers, so the black end of the
ramp is an opaque black band, the grabs' "absent" on a dark cell. Confidence 85 (law; the code plus a
three-channel fit), 75 (that "absent" is black).

The update is gated on `+0x10c` (set by `_Build`), so the phase advances only while the widget is
built, from 0. Nothing else in `TeamSelectionScreen_Update` (`0x00254348..0x002570a0`) or
`TrackHexSelection_Update` stores to the hex object's `+0xe0`, `+0xb4` or `+0xec` (the three stores
at those displacements in the screen update are on other widgets: `0x00254730` a pose block,
`0x00254f7c` the object at `st+0x188`, `0x0025578c` the `Logo` image).

### The ring's position (`0x001b1110..0x001b1188`)

```
ring.x += (target_x - ring.x) * 0.5          // TOC -0x30 = 0.5, per update, not scaled by dt
ring.y += (target_y - ring.y) * 0.5
```

The target x is fixed at `cc * 72`; only the target y changes on a team step (the parity flips, so the
ring heads half a row, 42 units, up or down to where the incoming column sits). At 60 Hz the error halves
per frame (under 1 unit after 6 frames of a 42-unit move). Confidence 80 (the ease is per call: at
another frame rate it would differ, unmeasured).

**The ring is not displaced sideways.** What the grabs show as "the ring to the right of the cell at
frames 4 to 10" is the columns sliding left under a ring that stays put (next section). Confidence 80.

## The lattice slide

### The step (`HexSelection_Step`, `0x001b09b8`)

`TeamSelectionScreen_Update` calls it at `0x002554bc` as `_Step(hex = st+0x190, old = st[0x100], new)`,
then stores `new` into `st[0x100]`. With `n = +0x108`:

```
forward  = (new == old + 1) or (old == n-1 and new == 0)      // precisely: new > old, except the wrap
dir      = forward ? -1 : +1                                   // +0xf0
if timer == 0: parity ^= 1                                     // +0xb4
timer    = 0.1                                                 // TOC -0x74 = 0x3dcccccd
target   = (cc * 72, row * 83 + 42 * ((cc odd) XOR parity))
```

(`old == 0 and new == n-1` is backward; `new == old` returns early.) The screen arms a refill flag
`st[0x103c] = 1` at `0x00254fb0`, and only when the timer is 0 (`0x00254f90..0x00254f9c`: a new selection
while `+0xec > 0` returns through `0x00254d20` without stepping). **A press during a slide is deferred,
not dropped**: the new selection (`r20`) is recomputed every frame from the focused entry
(`0x00254e9c..0x00254ee0`), and `st[0xf4]`, the selection it is compared against, is written only after
a step (`0x002554d0`). So the comparison retries each frame and the step fires on the first frame after
the timer reaches 0: a held direction steps at most once per 0.1 s (plus the input repeat). Confidence 70.
The `timer == 0` guard on the parity flip in `_Step` is therefore never false on Team Selection.

### The update (`0x001b1250..0x001b1580`)

While `timer > 0`: `timer = max(timer - dt, 0)`, then for every column `i` of `0..Columns`:

```
timer > 0 : x_i = (i + dir * (1 - 10 * timer)) * 72        // TOC -0x24 = 10.0; y_i unchanged
timer == 0: x_i = i * 72,  y_i = 42 if (i odd) XOR parity else 0
```

The spare column (`Columns`, index `+0xbc - 1`) is made visible (`+0x34 |= 4`) during the slide and
placed where the incoming column enters: for `dir = +1` at `x = -10 * timer * 72` (from -72 to 0), else at
`x = (dir * (1 - 10 * timer) + Columns) * 72` (from 360 to 288); its y is `42 * parity`.

So a **forward step (Right) moves the lattice one column (72 units) to the left, linearly over 0.1 s
(6 frames at 60 Hz)**, a backward step to the right; no easing (`1 - 10 t` is linear in the timer). At
the end the columns snap back to their slots with the parity flipped, which is the same picture shifted
one column, because adjacent columns alternate their half-row.

**What the cells show meanwhile.** At the step the screen fills only the spare column with the incoming
team (`0x00255510..0x002555fc`: index `cc + new` forward, `new - cc` backward, wrapped by the team count,
through `_SetCell` / `_ClearCell`). Every other column keeps the old window's content during the slide.
The whole window is refilled by `0x00255ea4..0x002560ec` (all `Columns`, `_SetCell` at `0x0025604c`),
reached from `0x00254f4c` when the refill flag is set and `+0xec <= 0`, and that clears the flag: the
refill lands on the first screen update after the slide timer reaches 0. Whether that is the same frame
as the snap or one later depends on the update order of screen and widget (unread; one frame at most).

### Against the grabs

`team-selection-model.md` measured: old team's picture through frames 0 to 3, moving at 4, half way by 8,
the new team's rest picture at 11 (picture lag 3). The code's 6 update frames from the step land on
frames 4 to 9 or 10 after the press with that lag, half way after 3 (frame 7 to 8): consistent. "Distances
to both ends exceed the gap between them at frames 4 to 7": a translation, not a blend, as read.

## Wired (2026-11-24, `shipselect-ring-wire`)

Every row of the table below is wired in `oag_ui_screens::picker::hd::{hex, slide}`, on Team
Selection (HD, Fury and Omega through the same code) and, where marked, Track Select:

- **Slide, spare column, content, step deferral**: `slide::Slide`, ticked by `Picker::tick`. A
  step that lands while a slide runs is held, and fires on the first tick after it ends. Frame 0 is
  the tick the step lands on; frame 3 is half a pitch; at frame 6 the columns are in their slots
  with the new window. Team Selection only: the deferral guard is in `TeamSelectionScreen_Update`;
  `TrackHexSelection_Update`'s own call site (`bl` at `0x002099ec`) is unread, so **Track Select
  does not slide** (open).
- **Ring**: one `Hexagon_HD_THICK_OUT.gtf` quad, drawn once after every cell (z + 50), x fixed at
  the centre column, y eased by half per tick (`Slide::ring_offset`); pulse colour
  `pulse_argb(HD_Blue, pulse_phase(Picker::seconds))`, opaque. Track Select shares the art, the
  pulse and the fill (`hex::draw_cells`).
- **Selected fill**: `HD_Blue`, once.
- **Constants**: pitch 72 by 83, half row 42, quad 92x80.
- **Omega**: the same code; `FrontEnd::team_hex_ring_peak` carries the red (238, 7, 7) its update
  computes inline, and its colour template's `HD_Blue` is the same `0xffee0707`.
- **Left as read, not wired**: the refill frame (the snap and the refill land on one tick here),
  the pulse's zero point (the clock starts with the picker, not the page's entry), the cells'
  edge-fade corners, the ring's ease on a model-row change (`_SetRow` also writes the ring target),
  Omega's `Scale="0.8"` against the 72-unit pitch, and a held direction's input repeat.
- **Compare pairs** (the pair as recorded against the worktree's binary, `--menu-page ship-select`,
  settled): `hd/menu-ship-select` mean grey 3.04 to 2.73, rmse 0.054 to 0.047, edge 0.50 to 0.51;
  `omega/menu-ship-select` 20.17 to 20.08, rmse 0.185 to 0.185, edge 0.32 to 0.33. The settled
  still draws at pulse phase 3 (a settled `1.0 s` clock), where the ring is exactly the cell's fill
  colour: invisible on the selected red, as it is on the original at that phase of its sawtooth.

## Ready to wire

Against ours (`oag_ui_screens::picker::hd::hex`, `crates/ui-screens/src/picker/hd/hex.rs`) and the
targets in [team-selection-model.md](team-selection-model.md) ("The banner and the hex column on a step":
hex box on k04, k06, k08, k10 at 16.8, 21.5, 22.6, 14.1 against a floor near 4).

| Item | The original (law, data) | Ours now | Confidence |
| --- | --- | --- | --- |
| Lattice slide | forward step: every column `x -= 72 * (1 - 10 t)`, `t` from 0.1 s to 0 linear, backward `+=`; then snap to the slot with parity flipped and the new window's content | holds the old phase `PHASE_HOLD_SECONDS` (4 frames), then cuts to the new | 85 |
| Spare column | one extra column enters from the side the lattice moves away from (forward: from `x = 360` to `288`; backward: `-72` to `0`), showing the incoming team, y `42 * parity` | none | 80 |
| Content during the slide | the old window; refill after the timer hits 0 | new window at once | 75 |
| Ring pulse | colour sawtooth, period 2/3 s: white to `HD_Blue` (Fury `0xffac0717`, HD style `0xff8ac0ca`) over 1/3 s, to black over 1/3 s, cut to white, drawn opaque (replacing, not adding); phase 0 when the widget is built, never reset on a step (the build's offset from the page entry is unread: a wire lane's choice) | fixed pale pink `193/81/91` | 85 |
| Step during a slide | deferred to the slide's end, not dropped | steps at once | 70 |
| Ring art | one quad of `Hexagon_HD_THICK_OUT.gtf`, 92 x 80 units, UV 74 x 64 texels, at the cell's own rect | six inset copies of `Hexagon_HD_OUTLINE.gtf` | 85 |
| Ring position | x fixed at the centre column; y eases `y += (target - y) / 2` per frame to `row * 83 + 42 * odd` after a step | fixed on the cell | 80 |
| Selected fill | `HD_Blue`, not `SelectedColumnCol` (not a parsed key) | `SelectedColumnCol` twice | 85 |
| Widget constants | pitch 72, row 83, half row 42, cell 92 x 80 | 72.2, 82.8, 41.4 measured off one frame | 85 |

The ready-to-wire laws need no fitting: every number above is a literal or an authored value. The
cells' fade toward the edges (`_SetCell`: per-vertex lerp `HexColFade` to `HexCol` and
`SelectedLockColFade` to `SelectedLockCol` by column distance `i / cc`, icons `0x12000000` to
`0x64000000`) is read but its vertex order (which pair of corners gets which end) is not; 60.

## Omega

**Wired (slide, step, ease, ring: `shipselect-ring-wire`); checked, differs (ring colour).** Read in
`/omega/eboot-ps4-omega-eu.bin` (the `eboot.bin` in the shared Ghidra project; addresses are this build's:
`0x015a3b80`, the Team Selection fit that [../ps4-omega-eu/team-selection-pose.md](../ps4-omega-eu/team-selection-pose.md)
cites for `v107`, does not start a function here, it falls inside `FUN_015a3810`): `HexSelection` registers at `FUN_0142ea90` with vtable
`0x0191b5b0`; slot 3 is `FUN_0142caa0`, the same update: ring ease 0.5 per call, pulse `p += dt * 3`
wrapped at 2, slide `(i + dir (1 - 10 t)) * pitch` with the same spare column. The step is
`FUN_0142f0a0` (found by the store of `0x3dcccccd` to `+0x160`): the same 0.1 s timer, the same direction
rule and parity flip, **plus a fourth argument**: with a nonzero one and no more than two entries it takes
`dir = -arg` (a two-entry list has no "forward" by index). **The ring colour differs**: Omega computes it
inline, white to (238, 7, 7) for `p < 1` (`R = 255 - 17 p`, `G = B = 255 - 248 p`) and (238, 7, 7) to black
for `p >= 1`, not a lookup of `HD_Blue`. The `Hexagon_HD_THICK_OUT.gtf` string is referenced by
Omega's build (`FUN_0142cf20`). Omega's grid authors `Scale="0.8"`; whether the update's 72-unit pitch
scales with it is unread. Omega functions are not renamed here (no Omega page for this class).

## 2048

**Checked, class ships, law not read.** `/2048/eboot-vita-2048-eu-v104.elf` carries `HexSelection`
(`0x8143a9b0`, referenced from `0x8109a0ec`), `TrackHexSelection`, `SelectedLockColFade` and
`Data\FE\Images\Hexagon_HD_THICK_OUT.gtf`, so the widget is in 2048's front end; its update was not
followed (ARM vtable not walked). 2048's Team Selection is drawn by 2048's own path in this engine
([team-selection-model.md](team-selection-model.md), "Omega and 2048"), so nothing is wired for it here.
Next: the registration that uses `0x8109a0ec`, its vtable's slot 3, and the constants 3.0, 2.0, 10.0, 0.1.

## Still unread, with the next step

- The update order of screen and widget at the snap frame (one frame at most). Recipe: an RPCS3 breakpoint
  on `0x001b09b8` at a Right press, then per frame `+0xec`, `+0xe0` and the column offsets
  (`*(+0xc8)[i] + 0x40`), against the frame the refill (`0x0025604c`) runs.
- The pulse's absolute phase at page entry (it starts at 0 at construction; when the screen constructs
  it relative to the page entry is unread).
- `_SetCell`'s vertex order for the fade, and the meaning of its stack byte at `0x87` (the icon is drawn
  `0x64000000`, a black silhouette, when it is set: likely "locked").
