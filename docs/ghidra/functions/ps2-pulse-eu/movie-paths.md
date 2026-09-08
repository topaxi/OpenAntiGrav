# Movie source paths: how a `Movie` widget's `src` becomes a filename

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

The PS2 front end names its two movies in
[front-end XML](../../../formats/fexml.md), and **neither name is a file on the
disc**:

| Screen | `Movie` widget `src` |
| --- | --- |
| `LogoFMV`, `Play Intro` | `Data\Movies\Intro.pss` |
| `FE Screen` | `Data\Movies\Backdrop.ipf` |

What the disc holds under `DATA/MOVIES/` is `INTRO512.PSS`, `INTRO640.PSS`,
`BG512.IPF` and `BG640.IPF`. One function closes that gap, and it is the same
function [pulse-disc-layout.md](../../../ps2/pulse-disc-layout.md) already knew
as the intro's 512-vs-640 selector: it does the backdrop too.

**The name below is applied**, from [names.tsv](names.tsv).

## `Movie_ResolveSourcePath` (`0x0019b168`), confidence 90

Formerly `FUN_0019b168`. Rewrites the widget's `src` in place to the file the
running video mode wants, then hands the widget on to `0x001dff80` - the open
call - with three of its own fields.

```c
// param_1 is the Movie widget; +0xc0 is its src, a std::string.
param_1->flags(+0x2c) |= 6;

if (find(src, "\\Intro.pss") >= 0 || find(src, "Data\\Movies\\Intro") >= 0) {
    src = g_video_mode(0x0027a85c) ? "Data\\Movies\\Intro512.pss"
                                   : "Data\\Movies\\Intro640.pss";
} else if (find(src, "\\Backdrop.ipf") >= 0 || find(src, "Data\\Movies\\bg") >= 0) {
    src = g_video_mode(0x0027a85c) ? "Data\\Movies\\bg512.ipf"
                                   : "Data\\Movies\\bg640.ipf";
}
FUN_001dff80(param_1 + 0x98, param_1->+0xc8, param_1->+0xd8, param_1->+0xe4);
```

### The eight string operands, in address order

Every literal is formed as `lui s0,0x2b` plus a negative `addiu`, so the address
is `0x2b0000 - offset`. The table is contiguous, which is itself worth noting:
one author wrote all eight together.

| Instruction pair | Address | String | Role |
| --- | --- | --- | --- |
| `0019b174` / `0019b17c` | `0x002aeeb8` | `\Intro.pss` | test |
| `0019b1d8` / `0019b1e0` | `0x002aeec8` | `Data\Movies\Intro` | test |
| `0019b228` / `0019b230` | `0x002aeee0` | `Data\Movies\Intro512.pss` | assign, mode != 0 |
| `0019b264` / `0019b26c` | `0x002aef00` | `Data\Movies\Intro640.pss` | assign, mode == 0 |
| `0019b2a0` / `0019b2a8` | `0x002aef20` | `\Backdrop.ipf` | test |
| `0019b2e4` / `0019b2ec` | `0x002aef30` | `Data\Movies\bg` | test |
| `0019b334` / `0019b33c` | `0x002aef40` | `Data\Movies\bg512.ipf` | assign, mode != 0 |
| `0019b370` / `0019b378` | `0x002aef58` | `Data\Movies\bg640.ipf` | assign, mode == 0 |

The two calls each literal feeds are distinct and that is what separates a test
from an assignment: `0x0020b8e8` takes three arguments, returns a signed value
the code branches on with `bgezl` (`0019b1d0`, `0019b210`, `0019b2dc`,
`0019b31c`), and is called only on the four `test` rows - a `find` returning a
position or a negative miss. `0x0020b3a8` takes two, returns nothing the caller
reads, and is called only on the four `assign` rows. Neither has been read; the
`basic_string` literal that sits immediately after this table in `.rodata`
says what kind of object `+0xc0` is.

### The selector is one global, read twice

`lui v0,0x28` / `lw v0,-0x57a4(v0)` is `0x0027a85c`, at `0019b220` for the
intro branch and `0019b32c` for the backdrop branch - **the same address**, and
the same sense both times: non-zero picks `512`, zero picks `640`.

That global is `g_refresh_mode`, and its sole writer is `Video_SetRefreshMode`
(`0x0010b030`), which sets non-square PAL pixel-aspect constants (`0.8`,
`1.1428`) when passed `1` and a no-correction set when passed `0`. So `512` is
the 50 Hz cut and `640` the 60 Hz one, which the measured frame rates of the two
`.PSS` files confirm independently: 25 fps and 30000/1001.

**What decides it is now read: the player does, on a first-boot screen** - see
[refresh-mode.md](refresh-mode.md). `Video_SetRefreshMode`'s two callers are
cases 4 and 5 of a six-entry task dispatcher, and those two cases are the names
`Switch50` and `Switch60`, which the disc's own front-end XML authors on
`RefreshTestFail` and `RefreshTestScreen`. So this is a refresh rate the player
chose, not a region the console reported, and both cuts are reachable on the one
(PAL-only) pressing that exists. What is new *here* is that the *backdrop* rides
the same global.

### Why 90 and not higher

The control flow, the operand addresses and the string contents are all read
directly, and both branches are structurally identical, which is a check in
itself. Held under 95 because `0x0020b8e8` and `0x0020b3a8` are inferred from
their argument counts, return use and call sites rather than read. The other
reason this sat under 95 - that what decides `0x0027a85c` was an untraced case
in `FUN_00186ed8` - no longer applies; see [refresh-mode.md](refresh-mode.md).

## What this settles for the reimplementation

Two things, both of which this build had wrong:

1. **The PS2's `src` already carries its extension.** `Movie_ParseAttributes` on
   the PSP appends `.PMF`; the PS2 tests for `\Intro.pss` and `\Backdrop.ipf`
   *with* the extension, so nothing is appended there. This build appended
   anyway and asked for `Data\Movies\Backdrop.ipf.PMF`. Fixed in
   `crate::screen::Movie::entry_name`.
2. **The XML's names are not filenames, and the mapping is per-region.** A
   loader that goes looking for `Backdrop.ipf` on the disc finds nothing.
   `crate::boot::LOOSE_MOVIES` is this table, with `512` first because the disc
   this project reads is EU/PAL.

## Not read

- `0x001dff80`, the call this function tails into with the widget's `+0x98`,
  `+0xc8`, `+0xd8` and `+0xe4`. Presumed the player open, unverified.
- `param_1->+0x2c |= 6` - two flag bits set unconditionally before any name is
  resolved. Which bits, unknown.
- The IPU decode itself. `libipu` appears by name in one assertion string
  (`0x002d8de0`, "Need to re-setup libipu since sceMpegGetPicture was
  aborted"), so the PS2 build links Sony's IPU library; the container around it
  is documented in [ipf.md](../../../formats/ipf.md) and decoded out of process
  rather than reimplemented.
