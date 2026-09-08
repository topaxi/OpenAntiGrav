# The 50/60 Hz refresh mode is a player's answer, not a detected region

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

[`movie-paths.md`](movie-paths.md) left one thing open: `Movie_ResolveSourcePath`
picks `Intro512.pss` over `Intro640.pss` - and `bg512.ipf` over `bg640.ipf` - on
the global `0x0027a85c`, whose sole writer `FUN_0010b030` was reached from "a
numbered case in the dispatcher `FUN_00186ed8` that nobody has traced". That case
is traced here, and it does not end at a region byte or a console query. **It
ends in the disc's own front-end XML**, at a screen that asks the player whether
their television does 60 Hz.

That answers a question the pressing itself raises: Pulse was released on PS2 in
**PAL territories only**, and the disc still carries `INTRO640.PSS` and
`BG640.IPF`. It carries them because the 60 Hz cut is reachable on a PAL console
by answering yes.

**The names below are applied**, from [names.tsv](names.tsv): the global is
`g_refresh_mode` (`0x0027a85c`).

| Claim | Confidence |
| --- | --- |
| `g_refresh_mode` (`0x0027a85c`) has exactly one writer, `Video_SetRefreshMode` | 95 |
| That function's two callers are cases 4 and 5 of `BackendController_RunTask`'s six-entry jump table | 95 |
| The case number is an index into a six-name table, matched from an XML attribute | 95 |
| Case 4 is `Switch50` and case 5 is `Switch60` | 95 |
| The disc authors both, on `RefreshTestFail` and `RefreshTestScreen` | 98 |
| Non-zero `0x0027a85c` means 50 Hz, so `512` is the 50 Hz cut and `640` the 60 Hz one | 90 |

## The chain, bottom up

### `Video_SetRefreshMode` (`0x0010b030`), confidence 90

Formerly `FUN_0010b030`. Takes one argument, stores it to `0x0027a85c` in the
delay slot of its own first branch - so the global is written on both paths -
then sets four pixel-aspect floats and calls `0x00102b78`.

```
0010b044  beqz    $s0, 0x0010b088       ; s0 = the argument
0010b048  sw      $s0, -22436($v0)      ; v0 = lui 0x28, so 0x0027a85c
0010b04c  lui     $at, 0x3f4c / ori 0xcccd   ; 0.8
0010b05c  lui     $at, 0x3f92 / ori 0x4925   ; 1.1428571
          ...                           ; into 0x0027f2f4/f8 and 0x0027f0c0/c4
0010b088  lui     $at, 0x3f80                ; 1.0, into the same four
0010b0b0  jal     0x00102b78                 ; GS mode setup, not read
```

Argument non-zero takes the `0.8`/`1.1428571` path - a non-square PAL pixel
aspect - and zero takes the `1.0` one. That is the reading
[`pulse-disc-layout.md`](../../../ps2/pulse-disc-layout.md) already carried at 85;
what is new is that both branches store, and that the argument is a literal at
each call site rather than anything computed.

### `BackendController_RunTask` (`0x00186ed8`), confidence 90

Formerly `FUN_00186ed8`. A virtual (its only reference in the image is the
vtable slot `0x002959ac`, in the vtable based at `0x00295938`). It reads its
object's `+0x98` as a case number, bounds it with `sltiu ... 6`, and jumps
through a **six-entry** table at `0x002ac5f0`:

| Case | Target | What it does |
| --- | --- | --- |
| 0 | `0x00186f20` | not read |
| 1 | `0x0018701c` | not read |
| 2 | `0x001870e8` | not read |
| 3 | `0x00187128` | not read |
| 4 | `0x00187158` | `Video_SetRefreshMode(1)` |
| 5 | `0x00187168` | `Video_SetRefreshMode(0)` |

The table's six entries are bounded on both sides in `.rodata`: `0x002ac608` is
the `basic_string` literal, and the `sltiu` agrees with the count.

### The case number comes from a string, and the six strings are on the disc

`BackendController_ParseAttributes` (`0x001867d0`), confidence 90. It tests the
element name against `Values` (`0x002ac4a0`), walks the element's attributes for
one named `task` (`0x002ac4a8`), copies its value into a 32-byte stack buffer,
then:

```
00186858  addiu   $v0, $zero, -1
00186860  sw      $v0, 152($s2)          ; +0x98 = -1, "no task"
00186868  sll     $v1, $s0, 2            ; s0 = 0..5
0018686c  addiu   $v0, $v0, -4048        ; v0 = lui 0x28, so 0x0027f030
00186878  jal     0x00111740             ; compare(value, names[s0])
0018687c  lw      $a1, 0($v1)
00186880  beql    $v0, $zero, 0x00186888
00186884  sw      $s0, 152($s2)          ; on a match, +0x98 = the index
0018688c  slti    $v0, $s0, 6
```

`g_backend_task_names` (`0x0027f030`), confidence 90 - six pointers, in the
order the loop indexes them, with a zero pair before them and an unrelated table
after:

| Index | Pointer | String |
| --- | --- | --- |
| 0 | `0x002ac460` | `Launch` |
| 1 | `0x002ac468` | `Kill` |
| 2 | `0x002ac470` | `Pause` |
| 3 | `0x002ac478` | `Run` |
| 4 | `0x002ac480` | `Switch50` |
| 5 | `0x002ac490` | `Switch60` |

So **case 4 is `Switch50` and case 5 is `Switch60`**, and `Switch50` is the one
that passes `1` - which is what makes non-zero `0x0027a85c` mean 50 Hz, and
`Intro512.pss`/`bg512.ipf` the 50 Hz cut.

The class name is on the disc too: `BackendController` (`0x002ac618`), passed to
a registration call at `0x001871bc`.

## The disc authors it, on three screens

`WADS2.WAD` entry `#3642` (hash `31b50f2e`) - the PS2 front-end screen tree - is
the only file in the archive that contains either name. Reproduce with:

```sh
just wad extract data/images/pulse-ps2-eu.chd:54748/WADS2.WAD -o /tmp/wads2
grep -rl 'Switch50\|Switch60' /tmp/wads2
```

The flow, as authored:

```xml
<Screen name="SwitchRefreshMode">
  ...  <Values idstring="REFRESH_QUESTION" .../>
  <Menu name="Switch" focus="true">
    <Entry idstring="FE_YES"></Entry>
    <Entry idstring="FE_NO"></Entry>
  </Menu>
  <Redirect>
    <Entry item="Switch" equals="FE_YES" goto="RefreshTestScreen" previous="false"></Entry>
    <Entry item="Switch" equals="FE_NO"  goto="LogoFMV"></Entry>
  </Redirect>
</Screen>

<Screen name="RefreshTestScreen">
  <BackendController>
    <Values task="Switch60"></Values>
  </BackendController>
  ...  <Values idstring="REFRESH_TEST_QUESTION" .../>
  <Menu name="Retry" focus="true">
    <Entry idstring="FE_NO"></Entry>
    <Entry idstring="FE_YES"></Entry>
  </Menu>
  <Redirect>
    <Entry item="Retry" equals="FE_YES" goto="LogoFMV"></Entry>
    <Entry item="Retry" equals="FE_NO"  goto="RefreshTestFail"></Entry>
  </Redirect>
</Screen>

<Screen name="RefreshTestFail">
  <BackendController>
    <Values task="Switch50"></Values>
  </BackendController>
  <Redirect>
    <Default goto="SwitchRefreshMode" previous="false"></Default>
  </Redirect>
</Screen>
```

It is the familiar PAL-console 60 Hz test, done in data: ask, switch, ask again
in the new mode, and switch back if the screen went blank and the player could
not answer yes. `SwitchRefreshMode` is reached from `AutoSaveBootWarning`'s
redirect, so it sits in the boot chain **ahead of `LogoFMV`** - which is why the
intro is not the first thing a PS2 disc shows.

`Switch50`/`Switch60` are the only two of the six task names the archive
authors; `Launch`, `Kill`, `Pause` and `Run` appear nowhere in it, so cases 0-3
are either driven from elsewhere or unused, and are not read here.

## A second consumer, which corroborates the reading

The Screen Offset settings screen (`0x001bc380`, per
[`aspect-ratio.md`](../../../ps2/aspect-ratio.md)) uses the same global as an
**index**, not a flag:

```
001bc504  lw      $v1, -22436($v1)      ; 0x0027a85c
001bc508  addiu   $v0, $v0, 20432       ; 0x00284fd0
001bc510  sll     $v1, $v1, 2
001bc51c  lw      $v0, 0($v1)           ; offsets_x[mode]
```

Four such blocks, one per d-pad direction, over two two-element arrays at
`0x00284fd0` and `0x00284fd8`. So the game keeps a **separate screen offset for
each refresh mode**, which is what a 50/60 Hz flag would need and a region
constant would not.

## What the mode does not change: the resolution

Every other consumer of `0x0027a85c` moves the picture or corrects its pixels.
None of them resizes anything, and the whole image has exactly five readers -
`0x0018faa8`, the two in `Movie_ResolveSourcePath`, the Screen Offset cluster
around `0x001bc504`, `0x00216d78` and `0x0022336c`:

- `0x00216d70` builds the DISPLAY registers. It indexes the same two-element
  offset arrays, then adds a **constant per mode**: `DX = offset_x + 640`,
  `DY = offset_y + 52` at 60 Hz against `+ 680` and `+ 72` at 50 Hz. Those are
  positions in the raster, in DISPLAY's own half-pixel units - the number of
  lines the game draws is not among them.
- `0x0022336c` picks `(256, 32)` at 50 Hz and `(0, 0)` at 60 Hz, again an
  offset pair.
- `0x0018faa8` uses `mode != 0` as an index into the two-float pixel-aspect
  array `Video_SetRefreshMode` wrote.

So the frame stays **640x448 in both modes** and there is one asset set behind
it, which is the reading
[`aspect-ratio.md`](../../../ps2/aspect-ratio.md) already measured from the data
side at 92 ("there is no second, aspect-keyed asset set anywhere on the disc").
A 50 Hz PAL raster is 576 lines, so those 448 sit inside it, shifted down by the
`+72`; a 60 Hz one is 480 and they sit in it shifted by `+52`.

**The movies are the one exception, and they are exactly the exception this page
is about.** `INTRO512.PSS` is 512x512 at 25 fps and `INTRO640.PSS` is 640x448 at
29.97 - the 50 Hz cut really is the taller, narrower one - and the two `.IPF`
backdrops split the same way (225 frames against 270). They are a second encode
of one film, not a second authoring of the artwork.

## What this corrects

- [`aspect-ratio.md`](../../../ps2/aspect-ratio.md) called `0x0027a85c` "the
  region index". It is not a region index. It is the refresh mode, written from
  exactly one place, reached from exactly two authored XML attributes, and both
  values are reachable on the one pressing that exists.
- [`pulse-disc-layout.md`](../../../ps2/pulse-disc-layout.md) and
  [`movie-paths.md`](movie-paths.md) both described the `512`/`640` split as
  PAL/NTSC. The *frames* are still 25 fps and 30000/1001, so the split is real;
  what changes is that a PAL disc reaches the 60 Hz side by the player answering
  a question, so "the EU disc plays `512`" is a default, not a fact about the
  pressing.

## Not read

- Cases 0-3 of the jump table, and the rest of `BackendController`'s vtable.
- `0x00102b78`, the GS mode-setup call `Video_SetRefreshMode` tails into.
- Where the boot chain enters `AutoSaveBootWarning`, and whether the answer is
  persisted to the memory card or asked on every boot. Nothing here needs it,
  and the front-end XML does not save the `Switch` menu (`save` is absent on it,
  where the Aspect Ratio list carries `save="true"`), which suggests it is asked
  every boot - **not verified**, so it is written here as a suggestion and not
  scored.

## See also

- [`movie-paths.md`](movie-paths.md) - the function this unblocks
- [`ipf.md`](../../../formats/ipf.md) - the backdrop container, whose frame rate
  is inherited from this flag
- [`aspect-ratio.md`](../../../ps2/aspect-ratio.md) - the other setting that
  moves the PS2's picture, and a different global
