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
one basic block into four adjacent fields. **Which colour plays which role is
not here** - the drawing code that reads `+0x95c` has not been traced - so
`oag_hd::loading::PALETTE`'s role assignment is a reading of a screenshot at
confidence 60.

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
| `0xd` | *no ModeManager* | `% 3` |
| `0x15` | `MPArcade` | `% 3` |
| `0xe` | *no ModeManager* | fixed `1` |
| `6` | *no ModeManager* | `% 2` |
| otherwise | e.g. `3` = `SPArcade` | `% 3` or `% 4`, on a byte at `*0x00b979fc + 1` |

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

The **draw is not** wired, and what blocks it has moved. It is no longer the
range: the moduli are read and eleven of the mode ids are named, so an
Eliminator race draws from five and a single race from three or four. What
blocks it now is two smaller things:

- **The asset path.** This build loads one illustration at boot; drawing per
  race means loading all five, which is a change to how `oag_game::loading`
  reads rather than to a constant.
- **The byte at `*0x00b979fc + 1`**, which decides whether the common
  `SPArcade` case is `% 3` or `% 4`. Unread, so even the single-race deck is one
  card wide of settled.

Neither is a reason to approximate. The screen shows the feature both
observations of the running game showed, and says so.
