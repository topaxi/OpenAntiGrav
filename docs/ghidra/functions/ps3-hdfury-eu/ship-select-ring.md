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
same pulse, ease and slide**: read in full and measured live on 2026-11-25, see "Track Select" below.

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
| `0x001b0c28` | `HexSelection_SetCell` | 85 |
| `0x001b0320` | `HexSelection_ClearCell` | 70 |
| `0x001b2a48` | `HexSelection_Destroy` | 70 |
| `0x002096f0` | `TrackHexSelection_Update` | 85 |
| `0x0020a3a0` | `TrackHexSelection_Build` | 80 |
| `0x002092a8` | `TrackHexSelection_Refill` | 80 |
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
The settled frame's centre cell reads (149, 6, 20): `HD_Blue` times 221/255, the alpha inside
`Hexagon_HD_Faded.gtf` (see "The cells"), where `0x64ff0000` drawn twice cannot give a green or blue
term at all.

## The cells

Each cell is three `Image` children on one 92 x 80 quad: the hexagon (`+0xa4`, `Faded`, UV 74 x 64 of
the 128 x 64 art), the picture (the ship thumbnail or circuit icon) and the lock (`+0xa8`,
`hexlock_HD.gtf`, UV 75 x 64). `Hexagon_HD_Faded.gtf` (`DATA06.PSARC`) is `Hexagon_HD.gtf`'s shape
at alpha **221 of 255** inside (read off the decoded texture; `Hexagon_HD.gtf` is 255). `_Build`
loads it unconditionally: there is no front-end-style branch.

`HexSelection_SetCell` (`0x001b0c28`) colours a cell by its widget column `i`, with
`cc = int((Columns - 1) * 0.5)`:

```
i == cc            hex = HD_Blue (+0x104), lock = SelectedLockCol,
                   picture = white, or 0x64000000 when the cell is locked
i == Columns       spare column: picture 0x12000000; hex and lock keep _Build's *Fade colours
otherwise          i < cc:  a = i / cc,              b = (i + 1) / cc
                   i > cc:  a = 1 - (i - cc - 1) / cc, b = 1 - (i - cc) / cc
                   hex     = lerp(t, HexColFade, HexCol)                  t = a, a, b, b over the vertices
                   lock    = lerp(t, SelectedLockColFade, SelectedLockCol)
                   picture = lerp(t, 0x12000000, 0x64000000)            (locked or not)
```

**The locked flag** is `_SetCell`'s stack byte at `0x87` (the `in_stack_00000087` test at the
head of the body). With it clear on the selected column the picture (`+4`) takes `0xffffffff`, with it set it takes
`0x64000000`; on every other column it is not read. After the colours, the same byte sets the lock
image's visibility: clear hides it (`flags & ~4`), set gives it the hexagon's own bit 4. So a locked
cell on the red column is a black silhouette at 100/255 under a flat grey padlock, which is the
`149 -> 91` step the pair's red cells show (149 x (1 - 100/255) = 91). Confidence 90.

The `(a, a, b, b)` vertex order is left pair, right pair: `a` and `b` are continuous across
neighbouring columns only that way (0, 0.5, 1 | centre | 1, 0.5, 0 on five columns). So the fill is
one linear ramp across the widget, `HexCol` at the selected column's edges and `HexColFade` at the
outer ones. Confidence 90 for the hexagon's colour: the code, Omega's copy, and the frames. With the
art's 221/255 it gives cell centres of 38.1 and 27.2 one and two columns out on Team Selection
(measured 38.5 and 27.5) and 40.8, 35.3, 29.9 and 24.4 on Track Select's nine columns (measured 41,
35, 30, 24), and the eight flat patches sampled on `hd/menu-ship-select` come out equal to the
original's pixel or one level off. `HexCol`, `HexColFade`, `SelectedLockCol` and
`SelectedLockColFade` default to `0x64808080`, `0x32808080`, `0xff808080`, `0xff161616`
(Track Select authors none).

**The picture's quad on Track Select is 50 x 50 over 256 x 256 texels**: both `_SetCell` call sites
of `TrackHexSelection` pass those (`0x00209a34..` and `TrackHexSelection_Refill`, TOC `0x23dc` = 50.0,
`0x23e0` = 256.0), and the frame agrees (Vineta K's sails span 194 of 256 texels, 37.9 units, 36.4 px
at Safe Area 96, measured 36). Confidence 90.

**Wired** (`oag_ui_screens::picker::hd::hex`, both grids): the `Faded` art, the 92 x 80 quad over
74 x 64 texels, the hexagon's ramp (`edge_share`), `HD_Blue` once, Track Select's 50 unit icon, and
the picture and the padlock: every model's picture (locked or not) with the colours above, then
`hexlock_HD.gtf` (128 x 64, its 35 x 48 texel art centred in the 75 x 64 window) on the same 92 x 80
quad, flat on the selected column and ramped across the others from `SelectedLockColFade`. The class
default of that colour, `0xff161616`, is what an unauthored grid takes.

Measured on the pair `hd/menu-ship-select` (mean grey difference, whole frame 2.54 to 2.24; the hex
column box (255, 360, 625, 960) 3.18 to 0.39; one locked cell each of the selected, next and outer
columns 4.59, 5.51 and 4.42 to under 0.13) and `hd/menu-track-select` (3.54 to 3.52; the grid box
(180, 440, 860, 720) 1.13 to 0.95). The icon ramp is Track Select's only change: no circuit locks
there. Tests: `picker::hd::tests::cells::*`, `picker::hd::track::tests::the_grid_puts_a_circuit_in_each_hexagon_and_red_on_the_chosen_column`
and `selection_screens_ground_truth::hd_reads_its_own_team_selection` (the art's name and window,
the colour ends and the silhouettes off the disc's own teams).

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
  with the new window. Team Selection only: the deferral guard is in `TeamSelectionScreen_Update`.
  **Track Select slides too since 2026-11-25** (`hd-front-end-ranked`), with its own frame count: see
  "Track Select" below.
- **Ring**: one `Hexagon_HD_THICK_OUT.gtf` quad, drawn once after every cell (z + 50), x fixed at
  the centre column, y eased by half per tick (`Slide::ring_offset`); pulse colour
  `pulse_argb(HD_Blue, pulse_phase(Picker::seconds))`, opaque. Track Select shares the art, the
  pulse and the fill (`hex::draw_cells`).
- **Selected fill**: `HD_Blue`, once.
- **Constants**: pitch 72 by 83, half row 42, quad 92x80.
- **Omega**: the same code; `FrontEnd::team_hex_ring_peak` carries the red (238, 7, 7) its update
  computes inline, and its colour template's `HD_Blue` is the same `0xffee0707`.
- **Left as read, not wired**: the refill frame on Team Selection (the snap and the refill land on
  one tick here; Track Select's is read and one frame, below), the pulse's zero point on Team
  Selection (the clock starts with the picker, not the page's entry; Track Select's is measured,
  below), the ring's ease on a model-row change on Team Selection
  (`_SetRow` also writes the ring target; wired on Track Select), Omega's `Scale="0.8"` against the
  72-unit pitch, and a held direction's input repeat.
- **Compare pairs** (the pair as recorded against the worktree's binary, `--menu-page ship-select`,
  settled): `hd/menu-ship-select` mean grey 3.04 to 2.73, rmse 0.054 to 0.047, edge 0.50 to 0.51;
  `omega/menu-ship-select` 20.17 to 20.08, rmse 0.185 to 0.185, edge 0.32 to 0.33. The settled
  still draws at pulse phase 3 (a settled `1.0 s` clock), where the ring is exactly the cell's fill
  colour: invisible on the selected red, as it is on the original at that phase of its sawtooth.

## Track Select (2026-11-25, `hd-front-end-ranked`)

`TrackHexSelection_Update` (`0x002096f0`, vtable slot 3 at `0x00868090`) read in full, its constants resolved
through its own TOC (`scripts/ps3-toc.py resolve 0x002096f0 <disp>`, TOC `0x008ad4d8`), and checked against
the live object in RPCS3 (found by its vtable word; `Columns` 9, `Rows` 2, 12 entries; `scripts/hd_track_hex.py`).

| Field | Meaning | Source |
| --- | --- | --- |
| `+0x110` | row, 0 = forward, 1 = reverse (the two `Rows`) | `0x002097e8`, `0x0020988c` |
| `+0x118`, `+0x11c` | previous and current circuit (the column the centre shows) | `0x002099a4`, `0x002099b0` |
| `+0x114`, `+0x115` | a row-change flag, and a refill flag (set with each step, cleared by the refill) | `0x002097f4`, `0x002099e8` |
| `+0x120` | circuits per row (12) | the wrap at `0x00209998..0x002099b0` |
| `+0x148` | the direction glyph child (the `>>>` under the forward hexagon), placed from the ring's own x and y | `0x00209bf4..0x00209cc0` |
| `+0x14c` | one counter, two jobs: the input cooldown and the glyph's scroll clock | `0x00209738..0x00209758` |

- **Input cooldown** (`+0x14c`, constants `0x23c8` = 0.0, `0x23f8` = -0.2, `0x2408` = 1.0): each update adds `dt`; above 1.0 it
  is set to 0.0. A press (left, right, up, down) is accepted only when the counter is at or above 0.0 and sets it to **-0.2**.
  So a direction steps at most once every 0.2 s: the slide's 0.1 s is shorter, so **no press ever lands mid-slide and there is
  no deferral** (`_Step`'s `timer == 0` parity guard is never false here either). Read live: a press moved the counter from
  1.0 down to -0.131 and it climbed 0.0167 a frame. The scrolling `>>>` glyph is the same counter ("The direction
  glyph", below).
- **Up and down** (pad words 0 and 1) only change `+0x110`, set the ring target through `HexSelection_SetRow` (`0x001b0920`) and
  turn the glyph by pi (`0x23cc` = 3.14159) on the reverse row; the lattice does not move. **Left and right** (words 2 and 3)
  move `+0x11c` by one, wrapping at `+0x120`, and call `HexSelection_Step(+0x118, +0x11c)` at `0x002099ec`. The class's own
  `_Step` then sets the 0.1 s timer and flips the parity, exactly as on Team Selection.
- **The step and the widget's update are one frame.** `HexSelection_Update` is called from the end of this function
  (`bl` at `0x00209cc4`), after the step: the step frame's update already takes one frame off the timer. Read live, the first frame
  after a press had `timer` 0.079 to 0.083 and the columns at `i * 72 - 12`, i.e. **a sixth of a pitch** (Team Selection's update
  order is the other way: its widget updates before the screen steps). A sample at timer 0.014 had every column at
  `i * 72 - 61.9 = 72 * (1 - 10 * 0.014)`, the spare column visible at 586.1. So frame `k` after the step reads timer
  `0.1 - (k + 1) / 60`, the columns stand `(k + 1) / 6` of a pitch in, and **frame 5 is the snap** (timer 0, parity flipped, the
  columns back in their slots in one frame).
- **The refill and the snap are one frame.** At the top of the update (`0x00209770..0x00209790`, `0x00209e50..0x00209e68`) the
  refill (`FUN_002092a8`) runs when the timer less `dt` is at or below 0.0 and `+0x115` is set; the update then ends in
  `HexSelection_Update`, which takes the timer to 0 and snaps the columns. Read live: the frame the timer read 0.0 had the columns
  in their slots and the refill flag clear. The spare column is filled at the step (`0x00209a34..0x00209ad8`, index `cc + new`
  forward, `new - cc` backward, `0.5` = `0x23d8` for `cc`), as on Team Selection.
- **Parity follows the circuit's index.** RAM `+0xb4` read 0, 1, 0, 1 at circuits 0 to 3 and 0 at 4 and 0 (the wrap on 12 entries
  keeps it), with column y `[0, 42, 0, 42, ...]` at parity 0 and `[42, 0, ...]` at parity 1. So Track Select's lattice is
  `HexGrid::phase_by_entry`, like Team Selection's.
- **Row change.** `SetRow` writes the ring's target y (`row * 83 + 42 * odd`); the end-of-update ease then closes half the
  distance every frame (83 to 41.5 to 20.75 and so on), the same ease as after a column step.

**The pulse's zero point (measured).** The widget is built once and kept: its `+0x10c` (built) byte is 1 while the screen is
up and 0 when it is left, and `HexSelection_Update` returns at once while it is 0. The phase `+0xe0` read 0.6212 on leaving and
the next read after re-entering (a few frames later) was 0.8972: **the counter is not reset by a visit, it counts the widget's
own updates since the boot's first visit**, `+0.05` a frame (read at 60 fps: 0.4721, 0.5211, 0.5711, ...), wrapping past 2.
It is the same count as the circuit model's turntable angle (`track-selection-screen.md`, "The angle"), which also persists. So
there is no page-entry zero: a still of the original needs the counter read out of RAM. The picture lags that read: with
the game stopped, ten grabs showed the ring's colour 0.118 to 0.228 of a phase behind `+0xe0` (mean 0.176 = 3.5 frames; the ring
is white to `HD_Blue` for phase 0 to 1 and `HD_Blue` to black for 1 to 2, so the colour reads the phase). The stopped
step grabs put the picture's lattice at the RAM frame less about 3 (at RAM frame 3 the lattice stood 11.3 px = a sixth of a
pitch at 0.96, the Safe Area; the lag varies by a frame or so between grabs, 2 to 4).

**Wired (`hd-front-end-ranked`).** The grid draws `Picker::slide` (a `Slide` that follows the picker's column and, for a grid of
rows, its row: `Slide::follow_row`; `Slide::leading` for the step frame's tick); `track::read` sets `phase_by_entry`; the
ring's colour is `Picker::ring_seconds`, which a still pins with `--menu-ring-seconds` (the pair script reads `+0xe0` and the
turntable angle in one stopped grab, `scripts/hd_track_hex.py`, `rpcs3-hd-menu-pair.py`). Tests:
`track::tests::a_circuit_step_slides_the_lattice_a_pitch_and_snaps_on_the_fifth_frame`,
`slide::tests::a_leading_slide_has_moved_a_sixth_of_a_pitch_on_its_step_frame`, `the_ring_eases_to_the_other_row_by_halves`,
`the_rings_pulse_follows_its_own_clock`, and the disc-backed
`hd_ship_preview_ground_truth::the_track_grids_ring_and_slide_follow_their_clocks`. Pair `hd/menu-track-select` (the original
re-captured with the ring phase read, ours drawn with the base binary and the same flags against the new): the ring's left band
27.4 to 2.9 mean grey, the ring box (460, 480, 560, 560) 19.7 to 14.3, the grid region (180, 440, 860, 720) 3.99 to 3.68; a
second capture of the same state (ring phase 1.78 against 0.59 in the first) read 26.99 to 2.06, 19.61 to 14.15 and 3.98 to
3.67, so the re-capture noise is under 0.15. The stills at circuit 0 are byte-identical to the base binary's with the flag
absent, and every adopted HD and Omega menu pair (campaign, grid, cell, ship, track) re-renders byte-identical (the other pairs
were not re-captured: their pages draw none of this). With the fly-by frame fitted (`compare-menu-movie-phase.py`, frame 442) the
whole frame reads 4.08 (base binary, same original) to 3.77; the adopted pair before this lane read 4.35 on an older capture.
The step: twelve stopped grabs at frames 3 to 8 after a press, one per circuit step 1 to 11 and 0, ours drawn at RAM frame less 3
(`--menu-picker-step-seconds`, the circuit's `NN_Track` id read off the `circuits:` loader line: the forward list is
`01 08 03 04 02 06 05 07 17 19 21 23`, not index + 1), grid region mean grey 13.73 without the slide to 6.50 with it (every
trial improves; frame 8 is the snap, 5.13 to 4.30 there). The lag is a frame or so loose between grabs. The input cooldown is
wired as `Picker::step_gate` (12 whole frames) (`a_direction_inside_the_cooldown_of_the_last_one_taken_is_dropped`): the pad's four directions
on a grid of rows are taken only 0.2 s after the last one taken. RAM: `right` held for 1.1 s stepped at 0.05, 0.56 and 0.88 s,
so the input layer's own repeat (about 0.5 s then 0.3 s) is a separate floor over the 0.2 s one.

**Not wired, in the box:** the ring counter's persistence across visits in live play (a new `Picker` starts its clock at 0 on every visit, so only stills pinned with `--menu-ring-seconds` carry the original's count; the turntable and the glyph's counter restart the same way; carrying one f32 in the session needs an offset rather than a pin).

### The direction glyph

`TrackHexSelection_Build` (`0x0020a3a0`) makes one `Image` child (`+0x148`, z + 40) on
`Data\FE\Images\TrackDirectionSheet.gtf` (TOC `0x2430`; 32 x 64 texels, eight 32 x 8 frames of four
arrows, one arrow tinted, the tint walking left to right; frames 0 to 3 in the Fury red, 4 to 7 in the
HD teal, red again in Omega's own copy). Quad **40 x 10** (`+0xac`, `+0xb0`), UV window 32 x 8, white.
The end of `TrackHexSelection_Update` (`0x00209b88..0x00209cc0`) drives it:

- **Frame**: texel row `int(c * 4) * 8`, plus 32 when `FrontEnd_IsFuryStyle` is false, `c` the counter
  at `+0x14c`. So one style's four frames in a second.
- **Hidden** while `c < 0` (the 0.2 s after an accepted press) and on a locked circuit.
- **Position**: the ring's x, and the ring's y plus **24** (TOC `0x2414`), so it follows the ring's ease.
- **Turned by pi** (`+300`) on the reverse row.

Confidence 90: the code on two titles, and the frames (HD's glyph sits and measures where the law puts
it; `hd/menu-track-select`'s glyph box goes 39.7 to 0.19 mean grey). **Wired**
(`oag_ui_screens::picker::hd::track::direction_frame`); `c` is the time since the picker's last move
less the 0.2 s, or the screen's own clock before any move. **Chosen, not measured**: that origin (the
original carries `c` across visits, and the pair script does not read it, so which arrow is tinted in a
still is the clock's luck: HD's pair matches, Omega's is one arrow off).

**The key repeat is not read.** `TrackHexSelection_Update` asks the pad through `FUN_006770c8(pad, n)`;
what a held direction does is that call's producer plus the 0.2 s above. One observation (`right` held
1.1 s stepped at 0.05, 0.56 and 0.88 s) is not enough to set an initial delay and a period. Recipe:
decompile `0x006770c8` and the writer of the bits it reads for the two constants; check them by holding
`right` for 3 s on RPCS3 and logging every change of `+0x11c` with `scripts/hd_track_hex.py`.

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
cells' colours are under "The cells" above.

## Omega

**Track Select: ported** (test `omega_campaign_screens_ground_truth::omegas_track_grid_slides_on_a_step_and_pulses_on_its_own_clock`). `/omega` `eboot-ps4-omega-eu-v107.bin` registers `TrackHexSelection` at `FUN_0152bab0` (the string at
`0x0188547f`, vtable `0x01989fd8`), slot 3 is `FUN_015298d0`: the same update as HD's `0x002096f0` instruction for instruction as far
as read: the counter at `+0x200` (clamped to 0 above 1.0), `0xbe4ccccd` (-0.2) stored after a press, the refill (`FUN_0152b650`) when
`+0x174 - dt <= 0` and `+0x1a1` is set, `FUN_0145b490` (the step: 0.1 stored to `+0x174` as `0x3dcccccd`, the parity byte `+0x130`
toggled, the direction at `+0x178`, with the extra direction argument of Team Selection's) called from the press, and
`FUN_01459330` (`HexSelection_Update`: `p += dt * 3`, wrapped past 2, the ring colour inline `255 - 17 p` and `255 - 248 p`, then
`(p - 1) * -238 + 238` and `(p - 1) * -7 + 7`; the slide `(i + dir * (1 - 10 t)) * pitch`) called at the end, so the same
order and the same frame counts. No live read (shadPS4 has no memory readout). Ours reads Omega's `Track_Selection` XML through the same
`track::read`, so the slide is wired through the same path: a step still at 0.05 s moves 35,080 pixels of its grid and panel
region, and the adopted `omega/menu-track-select` is byte-identical. The addresses in the Omega paragraph below
(`FUN_0142ea90`, `FUN_0142caa0`, `FUN_0142f0a0`) do not start those functions in `eboot-ps4-omega-eu-v107.bin`: `0x0142caa0` falls
inside `FUN_0142c7f0`, a menu-list update, so they belong to another Omega build and the v107 ones are the ones above; the
**Team Selection** copy in v107 was not located again.

**The cells and the glyph: ported, one value differs** (v107, `/omega/eboot-ps4-omega-eu-v107.bin`; test
`selection_screens_ground_truth::omega_reads_its_own_team_selection`). `FUN_0145a380`
(`_Build`) loads `Hexagon_HD_Faded.2x.gtf`, `hexlock_HD.2x.gtf` and `Hexagon_HD_THICK_OUT.2x.gtf` on the
same quad, 92 x 80 times the widget's `Scale`, which scales the 72, 83 and 42 pitches too. `FUN_0145af70`
(`_SetCell`) has the same ramp and vertex order, but **writes `0xff808080` over the selected column's
hexagon** where HD looks up `HD_Blue`, and both shadPS4 frames show that grey column
(`FrontEnd::hex_selected_fill`; `omega/menu-ship-select` 20.09 to 19.30). Its Faded art is alpha 219 in
the `.2x.gnf` the executable names and 215 in the 1x `.gnf` ours loads.
`FUN_0152add0` and `FUN_015298d0` carry the direction glyph with the same sheet, frame rule, 24 unit
drop and pi turn; the glyph measures 38 x 9.75 px on the 1:1 frame, so `Scale` does not apply to it.
**2048: checked, applies, not wired**: its executable names the same four textures; its Team
Selection is drawn by 2048's own path here.

**The picture and padlock wiring is inherited by Omega, not read there.** Omega draws through the
same `hex` module, and its sheet carries the 1x `hexlock_HD` at 128 x 64
(`selection_screens_ground_truth::omega_reads_its_own_team_selection`), so the 75 x 64 window holds.
Its `_SetCell` was not re-read for the locked branch. The pair `omega/menu-ship-select` moved the
wrong way, 19.30 to 19.45 (hex column box (200, 330, 660, 960) 47.57 to 48.61), while
`omega/menu-track-select` went 18.54 to 18.47: the frame draws locks near (62, 62, 62) on every
column, which ours matched before no better than now, and its `Team_Selection_Definition.xml` authors
the same `SelectedLockCol`/`SelectedLockColFade` as HD. That frame's half-row phase is also inverted
against ours, so its box figures are not cell for cell and do not attribute the 0.15 to colour; the
dark locks are read by eye. Next: read `FUN_0145af70`'s lock colours against `HexSelection_SetCell`'s.

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
- `_SetCell`'s stack byte at `0x87` is the cell's locked flag (its effect is under "The cells";
  `TrackHexSelection_Refill` passes the per-circuit byte `_Build` computes); what unlocks a Team
  Selection row is `team-selection-loyalty.md`'s.
