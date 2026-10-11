# `<Text DisplayType="TELETYPE">`: the `Text` widget has no type-in

Pulse PSP (USA) `BOOT.BIN` (`/pulse/BOOT-psp-pulse-usa.BIN`, image base `0x08804000`), 2026-10-11, lane
`pulse-typein`. Static read through the Ghidra bridge, then a live PPSSPP v1.20.4 capture (software renderer, own
profile, muted). Five names recovered, all below.

## Result

**A `<Text>` authored `DisplayType="TELETYPE"` draws its whole string on its first frame, exactly like a `NORMAL` one.**
The original has no per-character reveal on the `Text` widget: `TELETYPE` sets one flag bit that nothing reads. The port
already draws these whole, which is therefore correct, and this lane adds the attribute to the parsed data without
acting on it. Confidence 85 for the static law, 70 for the live confirmation (the captured frames show prompts and
screen titles complete from their first visible frame; which widget in each frame is the `TELETYPE`-authored one was
not separated, see "Live" below).

The earlier note in `menu-sounds.md` that "the original types those in silently" was an inference from the missing
sound call and is withdrawn: the original does not type them in at all.

## The parser

`Text_ParseValues` (`0x088cdb34`, confidence 90) is the `Values`-element handler of the `Text` class. It reads
`IDString`, `String`, `Color`, `BorderColor`, `ScaleX`, `ScaleY`, `Scale`, `WidthLimited`, `PulseMultiplier`, `RealGlow`,
`Align`, `VertAlign`, `Cached`, `flash`, `pulse`, `blurX`, `blurY`, `CalcBlur` and `DisplayType`. `DisplayType` is kept as
a string and compared with `strcasecmp` (case-insensitive) at the end of the function:

| Authored | Effect on the flag byte at `widget+0x173` |
| --- | --- |
| `NORMAL` | clears `0x40` |
| `SCROLLING` | sets `0x40` and `0x20` |
| `TELETYPE` | clears `0x40`, sets `0x20` |
| absent | nothing; the constructor (`Text_Construct`, `0x088ccbe8`, confidence 90) cleared `0x20` and `0x40` |

So the two spellings differ in bit `0x40` only.

## Who reads the bits

Every `lb` of `+0x173` in the binary was listed (41 sites). Inside the `Text` class (`0x088cc18c` to `0x088d0294`) the
readers are:

- `Text_Draw` (`0x088ce668`, confidence 85) reads only bits `0x01`, `0x02`, `0x04` and `0x08` (colour, flash and
  pulse state), **never `0x20` or `0x40`**. It prints the whole string with one call, `FUN_08919b60(string, width, height,
  flag)`; there is no character count, no timer and no clip.
- `Text_Enqueue` (`0x088cd740`, confidence 80) only culls on screen position and queues the draw.
- `Text_Update` (`0x088cd860`, confidence 80, a virtual slot) is the single place that reads `0x20`: `if (0x20 &&
  string != 0 && 0x40) FUN_088cdb2c(widget)`. **`FUN_088cdb2c` (`0x088cdb2c`) is an empty function** (`return`), and the
  call needs `0x40`, which `TELETYPE` clears. Only `SCROLLING` reaches the stub, and it does nothing.
- `FUN_088cd64c` clears `0x20` (and `0x200` of `+0x2c`); its callers are `FUN_088cd610`, `FUN_088b4ed8` and
  `FUN_088d1e04`, which is the update of a `Text` subclass (constructor `FUN_088d0294`, vtable `0x08acf614`) that
  owns two child `Text`s and scrolls them with its own float positions, not through this flag.
- `FUN_088cd680` sets `0x20` when `0x40` is set (the load-string path).

No float, tick or character index is anywhere in the path, so there is no reveal rate to recover.

## Live

PPSSPP v1.20.4, `pulse-psp-usa.chd`, own `HOME`, dummy audio, Xvfb. Recorded at 15 fps with `ffmpeg x11grab` while a
script walked the front end by the debugger's state names and read 12-frame windows at 5 fps:

- **Name entry** (`NameSetup2FromBoot`): the `ENTER YOUR NAME` title and the `CONFIRM` prompt slide in whole.
- **Main Menu** entry and **Profile** / **Manage Profiles** entry: the plate titles `MAIN MENU`, `PROFILE`,
  `MANAGE PROFILES` cross-fade whole over the previous title; the rows fade in whole.
- No frame in any window shows a partial string being completed. A letter-by-letter reveal at any plausible rate
  (`>= 5` characters a second) would show as a shorter string in at least two of the 12 frames of a window.

**Not separated:** which widget in these frames carries the `TELETYPE` authoring. `FE_CONFIRM_BUTTON` and `FE_CONFIRM`
(Name entry's prompt) are authored `TELETYPE` (census below), so that prompt is one; the plate titles are likely not.
`BOOT_PRESS_ANY2` was not reached: the title screen's own text (`BOOT_PRESS_START`) pulses and is not `TELETYPE`.

## What the discs author

`crates/ui/tests/text_display_type_ground_truth.rs`, a census over every front-end entry of the title's archives that
reads as text:

| Pressing | `TELETYPE` | `SCROLLING` | `NORMAL` |
| --- | ---: | ---: | ---: |
| Pulse PSP EU | 58 | 0 | 2 |
| Pulse PSP USA | 58 | 0 | (not counted) |
| Pulse PS2 EU | 218 | 0 | 4 |
| Pure PSP USA | 48 | 0 | 0 |
| HD/Fury PS3 EU | 105 | 0 | 12 |

Pulse PSP's 29 distinct `TELETYPE` labels include `BOOT_PRESS_ANY2`, `FE_CONFIRM`, `FE_CONFIRM_BUTTON`, `FE_BACK`,
`FE_BACK_BUTTON`, `FE_MENU`, `FE_ONLINE`, the `PRO_STATS*` titles and `ER_PHOTO_*CAM`.

## Other titles

- **Pulse PS2:** checked, applies, not wired - the same `.cod` front end authors `TELETYPE` (218 texts); the PS2
  executable was not read, so whether its `Text` class types in is **not checkable** here. The port draws whole.
- **Pure PSP:** checked, applies, not wired - 48 authored; no Pure binary is loaded in the Ghidra project, so its
  `Text` class is **not checkable**. The port draws whole.
- **HD/Fury and Omega:** checked, **open**. HD authors 105 `TELETYPE` texts and its executable carries `DisplayType` (`0x00791430`),
  `teletype` (`0x00791458`) and `TELETYPE` (`0x00791500`) in a table read by an attribute parser (data references at
  `0x008af628` and `0x008af65c`); Omega's carries `teletype` and `TELETYPE` (`0x01820539`, `0x0182059d`) and 2048's
  (`0x814408dc` and neighbours). HD's `Text` class is a different build and may type in; its reader was not
  read, nor was it captured on RPCS3. Not wired: the port draws whole on all of them, which is unverified there.

## Names recovered

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x088ccbe8` | `Text_Construct` | 90 |
| `0x088cdb34` | `Text_ParseValues` | 90 |
| `0x088cd740` | `Text_Enqueue` | 80 |
| `0x088cd860` | `Text_Update` | 80 |
| `0x088ce668` | `Text_Draw` | 85 |
