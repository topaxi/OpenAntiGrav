# The loading screen's constructor

**`LoadingScreen_Construct`, `0x002b3bb0`. Confidence 85.**

The function that decides what Wipeout HD's loading screen shows: which of the
five illustrated features is up, which of two screen types it is, and the four
front-end colours it draws with. Found from the asset side - see
[hd-loading.md](../../../formats/hd-loading.md), which is the screen itself -
and read here because the layout is in code and not in any shipped XML.

## Finding it

The two log strings are at `0x007a0350` and `0x007a0370`:

```text
LOADING SCREEN TYPE type == %i
Feature type == %i
```

Ghidra reports one `[DATA]` reference to each, into the TOC at `0x008b3610` and
`0x008b3618`, and **no reference to those TOC slots** - the per-function TOC
defect this binary is full of, see [memory.md](memory.md). `scripts/ps3-toc.py
attrib` resolves it from outside:

```sh
$ scripts/ps3-toc.py attrib 0x007a0350
== 'LOADING SCREEN TYPE type == %i\n' @ 0x007a0350
   0x002b3bb0
   0x002b48d8
```

Two functions, which is the constructor pair this codebase's own rule covers:
only `0x002b3bb0` is *called* (from `0x002b9a28`), so it is the one named.
`0x002b48d8` is referenced from data alone (`0x00882210`) and is left
`FUN_`.

## The palette: four globals, in order

The first thing the constructor does after zeroing the object is resolve four
front-end globals through the palette lookup and store the results adjacent:

| TOC | string | stored at |
| --- | --- | --- |
| `+0x611c` | `FE_HD_BG` | `object+0x95c` |
| `+0x6120` | `FE_HD_Grey` | `object+0x960` |
| `+0x6124` | `FE_HD_Blue` | `object+0x964` |
| `+0x6128` | `FE_HD_LightGrey` | `object+0x968` |

The `FE_` prefix is the lookup's; the globals in `skin.xml` are `HD_BG`,
`HD_Grey`, `HD_Blue`, `HD_LightGrey`. Their **values differ by archive** -
`DATA06` gives `HD_BG` as `0xffffffff` and the served copy does not - which is
why this screen is white-and-blue on one and black-and-red on another, and it
is the same open question about `skin.xml`'s six copies that
[hd-frontend.md](../../../formats/hd-frontend.md) already carries.

Confidence **90** on the set and the order: four `lwz` out of one TOC block in
one basic block into four adjacent fields.

### The roles, from the draw function's usage counts

`FUN_002b61c8` - the function immediately after this one, and the only other
reader of `+0x95c` - reads them at these rates, per screen variant (the function
has two near-identical blocks, one per loading-screen type):

| global | reads | what it feeds |
| --- | ---: | --- |
| `HD_BG` | 1 | passed to `0x00678138` before the variant branch, guarded on the screen kind not being 0 or 7 - the ground |
| `HD_Grey` | ~10 | everything else drawn |
| `HD_Blue` | 1 | unpacked to components and stacked for a draw |
| `HD_LightGrey` | 1 | the same, one element |

**The names mislead.** Each palette names its colours against its own ground:
`DATA00` has `HD_BG` black with `HD_Grey` at `ff969696`, `DATA06` has `HD_BG`
white with `HD_Grey` at `ff646464`. So `HD_Grey` is the ink in both and
"light grey" is the *dark* half-alpha one (`7f646464`) on the served copy -
reading it as the text colour draws a heading dimmer than its own body.

Confidence **80** on ground/ink/accent, from the counts and the values together.
`HD_LightGrey` is **not identified**: it is one element and the only translucent
one, and `oag_game::loading` draws it as the progression bar's trough on that
shape alone - a hypothesis at 55.

## The feature is a random draw, and the range is the race mode's

`object+0x440` is the *loading screen type* and `object+0x444` the *feature
type*; both are logged with the two strings above, which is what ties the field
offsets to the names.

The type is `1` when a byte at `0x009384e1` is clear **and** the mode
(`g_GameState`'s, at `*(int *)(*0x00936fe8 + 0xe0)`) is `0x11`, which is
`MPTournament`; `0` otherwise. Every observation so far - two independent runs -
logged type `0`.

The feature is drawn from a counter, `FUN_006762f8`, reduced modulo a range that
the same mode field selects:

| mode | what it is | range |
| --- | --- | ---: |
| `8` | `SPElimination` | `% 5` |
| `0x14` | `MPElimination` | `% 5` |
| `0xd` | *no ModeManager* | `% 3`, indices `0 1 3` (see [2026-10-04](#2026-10-04-the-deck-the-title-ids-and-the-trough)) |
| `0x15` | `MPArcade` | `% 3`, indices `0 1 3` |
| `0xe` | *no ModeManager* | fixed `1` |
| `6` | *no ModeManager* | `% 2`, indices `0 1` |
| otherwise | e.g. `3` = `SPArcade` | `% 3` or `% 4`, on the byte at `0x00b979fd` (the Fury-content flag, read `1` live) |

**So "which feature is up" is not a rotation and not a fixed choice: it is a
draw, and the deck depends on what you are about to race.** Eliminator gets all
five; the two modes with no `ModeManager` of their own get three and one.

The mode field is `g_GameState`'s, and eleven of its twenty-two ids are named -
[mode-manager.md](mode-manager.md#the-mode-enum-22-ids-eleven-of-them-named),
which this page's own question is what prompted. `0x11` - the id that makes this
a *type 1* screen rather than type 0 - is `MPTournament`, so the second screen
type is the online-tournament one.

What is still open is whether `FUN_006762f8` is a random source or a frame
counter, and what the seven unnamed ids are.

Confidence **85** on the shape - the moduli and the branch structure are plain
in the decompile, and this function is below the `0x32d5e0` TOC break so
Ghidra's strings here are the ones the code loads. **Not** verified against a
running game beyond the two `type == 2` observations, which are consistent with
a `% 5` draw and prove nothing about the range.

## What this does and does not license

`oag_hd::loading::PALETTE` is wired: the four names are read out of the
source's own `FEGlobals`, so the reimplemented screen is tinted by the disc
rather than by a scheme this project chose.

The **draw is wired, deck included**, since 2026-10-04:
`oag_game::loading::Screen::for_mode` draws from the mode's own deck
(`oag_hd::loading::DECK`), see the section below. Speed Lap and Zone have no
named executable id and draw from the default deck - chosen, not measured.

## 2026-10-04: the deck, the title ids and the trough

All against `EBOOT-ps3-hdfury-eu.elf`, with a live RPCS3 boot of the EU Fury
disc (`BCES-00664`) as the arbiter. Lane `hd-loading-screen`.

| address | name | confidence | what |
| --- | --- | ---: | --- |
| `0x002b61c8` | `LoadingScreen_Draw` | 80 | draws the screen per kind (`+4` = 0, 7, 8, 4); the only reader of `+0x95c..+0x968` |
| `0x002b7cd0` | `LoadingScreen_BuildText` | 85 | resolves the caption, the feature's title, heading and paragraph ids by `+0x444` |
| `0x002b86a0` | `LoadingScreen_LoadAssets` | 78 | loads the feature's `.gtf` by `+0x444` and `FrontEnd_IsFuryStyle`, builds the strings, creates `LoadingScreenThread` |
| `0x006762f8` | `Libc_Rand` | 88 | the decompiler's whole body is `rand()` |
| `0x00b979fc` | `g_FuryContentFlags` | 78 | a small flags block; byte `+1` is the Fury-content flag, byte `+0` gates `PurchaseGameRedirect` (`0x0021ba78`) |

### The byte at `0x00b979fd` is "the Fury content is present" - confidence 78

The constructor reads it as `*(byte *)(*(int *)(toc + 0x613c) + 1)`: the TOC
slot holds the address `0x00b979fc` and the `+1` is applied to the *address*, so
this is the byte at `0x00b979fd` and not a pointer dereference. That is what
made it look like an object read from twenty places: it is a block read from
**46 functions** (`scripts/ps3-toc.py attrib 0x00b979fc`), thirteen of which read
byte `+1`. Four readers agree on one meaning:

- `0x00029948` (profile defaults): while it is `0` the profile's `FE_Style`
  is forced to `HD` (`0x48 0x44`) and `PlayedFuryBefore` is not created.
- `0x00183b38` (front-end background): while it is `0`, `BackgroundAnimFury_Load`
  is skipped and the Fury bits at `+0x34` are not set.
- `0x001a8f50` (menu list update): while it is `0`, an item whose flags carry
  `0x8000000` is swapped for the `FE_FURY_REQUIRED` item.
- `LoadingScreen_Construct`: `((b - 1) >> 31) + 4`, so three while it is `0`
  and four otherwise.

**Live value: `00 01 00 00` at `0x00b979fc`**, read over the GDB stub of an RPCS3
boot of the EU Fury disc at the loading screen (mode `3`, `0x009384e1` = `0`).
So on this disc the default deck is four wide, and `Feature type == 3` was
logged in that same single-race load - impossible under a modulus of three.

**What is not found: its writer.** No store reaches it through its own TOC slot
(a scan of every instruction in the 46 functions for stores based on the slot
register found none), and no absolute `lis 0xb9 / stb 0x79fd` pair exists in code.
It is passed by address (`[PARAM]`) to about twenty functions, so the writer
is a callee of one of them. Next address to try: the `[PARAM]` sites
`0x0021ba9c`, `0x0005b3c8`, `0x00229a58`. Ghidra's one `[WRITE]` xref, at
`0x00631920`, is TOC mis-resolution (that function stores through
`0x008c02xx` pointers) and is not this.

### The deck is a set of indices, not a size - confidence 88

Re-reading the constructor's branch targets: the mode's range is not always
`0..n`. `0xd` and `0x15` reduce `rand()` modulo three, and a result of `2`
stores the literal `3` (`*(param_1 + 0x444) = 3`), so their deck is
**`{0, 1, 3}`**, skipping Pilot Assist. The full law is
`oag_hd::loading::DECK`:

| mode id | indices |
| ---: | --- |
| `8`, `0x14` | `0 1 2 3 4` |
| `0xd`, `0x15` | `0 1 3` |
| `0xe` | `1` |
| `6` | `0 1` |
| any other | `0 1 2 3`, or `0 1 2` while the Fury-content byte is `0` |

When `0x009384e1` is set every mode takes the last row (and screen type `1`
is never chosen); that byte has no other reader in this screen's functions and
is not modelled.

### Which index is which feature - confidence 92

Three consumers index the same `+0x444`, and they agree:

- `LoadingScreen_LoadAssets` picks the `.gtf` from ten TOC slots,
  `Barrel_Roll`, `Side_Shift_Tap`, `Pilot_Assist`, `Absorb`, `Flip` for
  `0..4`, each `_fury` first and plain second, on `FrontEnd_IsFuryStyle()`.
- `LoadingScreen_BuildText` picks three ids per index (resolved with
  `scripts/ps3-toc.py resolve 0x002b7cd0 <disp>`, this function's TOC being the
  default `0x008ad4d8`):

  | index | title | heading | paragraph |
  | ---: | --- | --- | --- |
  | 0 | `MAN_2_BR` | `FE_INSTRUCTIONS` | `FE_BR_INST` |
  | 1 | `MAN_2_SS` | `FE_INSTRUCTIONS` | `FE_SS_INST` |
  | 2 | `FE_PILOT_ASSIST` | `ONL_CON_DESC` | `FE_PA_INST` |
  | 3 | `IG_HUD_ABSORB` | `FE_INSTRUCTIONS` | `FE_ABSORB_INST` |
  | 4 | `FE_FLIP` | `FE_INSTRUCTIONS` | `FE_FLIP_INST` |

  **This closes the three title ids the page left unguessed**, and shows they
  were never `FE_`-namespaced: the screen reuses existing strings. `ONL_CON_DESC`
  is why Pilot Assist's panel reads `DESCRIPTION` where the other four read
  `INSTRUCTIONS`.
- The draw places the title (`+0x480`) at the left panel's head and the heading
  (`+0x488`) at the right's, in one row, with the paragraph (`+0x47c`) under it.

Two live frames pin the indices from outside: `Feature type == 1` logged over a
Side Shift screen and `== 2` over Pilot Assist, each in its own boot, read
against the screenshot. Indices 0, 3 and 4 are read, not seen.

### `HD_LightGrey` is the unfilled dots of the progression bar - confidence 88

Raised from 55. Two pieces of evidence:

- A live frame caught during the screen's fade-in (the bar not yet started) shows the whole bar as dark translucent grey dots and the
  filled part, in the frame after, as `HD_Blue` (red on this disc) dots at the
  left edge - `HD_Blue` over `HD_LightGrey` on one `dot.gtf` grid.
- In `LoadingScreen_Draw` the `+0x964` (Blue) and `+0x968` (LightGrey) reads sit
  back to back (`0x002b69d4`, `0x002b6b40`), each unpacked ARGB to RGBA and
  handed to the same vertex-quad call `0x006782e8` ahead of the four bracket
  draws, and the quads' right edge is the float `0x44db0000` = `1752.0`,
  which is the bar's right edge in the 1920 frame (the live frame's bar ends
  at 1447 of 1600 px, `1752/1920 * 1600 = 1460`).

So the trough this build draws flat is the right colour on the right element.
The residual 12: only the dotted texture itself is not reproduced.

### `FUN_006762f8` is `rand()` - confidence 88

Its decompile is the single call `rand()`. The three live boots logged
different features each time (`0 3 1`, `3 2`), which a frame counter that
starts at zero on every boot would not do. Whether the game seeds it is not
read.

### One boot constructs more than one screen

Each of three scripted boots logged two or three `LOADING SCREEN TYPE` /
`Feature type` pairs, and only the **last** matched the frame on screen. The
earlier ones are constructions that were not on screen when the frames were
taken; why the constructor runs more than once per boot is **not** read (the
twin at `0x002b48d8` is a candidate, unchecked).
