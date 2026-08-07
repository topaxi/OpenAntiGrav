# The loading screen on PS2: same wave, four divergences

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748, PAL), image base
`0x00100000`.

The PSP side is [loading screen](../psp-pulse-usa/loading-screen.md), and it is
authoritative: per [goals](../../../overview/goals.md), PSP is the target and
the PS2 build is corroboration. This page records where the two agree - which
is almost everywhere - and the four places they do not.

**The names here are applied**, from [names.tsv](names.tsv). See
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

## The short version

The PS2 build has **the same procedurally generated wave**, not a video. Every
tuning constant is byte-identical, including the 24-float heartbeat envelope,
which appears exactly once in each of the three binaries checked (PSP USA, PSP
EU, PS2 EU) and is the same 96 bytes in all three. The `loading` plugin on the
PS2 disc is likewise tips-only - no `.PSS`, no `.IPF`, no movie reference of
any kind - even though the PS2 does have the storage and decode budget for one.

So the answer to "is the wavy loading screen a video on the platform where it
could have been" is **no**, and that is now checked rather than assumed.

## What matches exactly

`Loading_DrawWave` (`0x001d95a8`) is the PS2 twin of PSP `0x0890a8e4`. Reading
them side by side, these are identical:

| Constant | Both builds |
| --- | --- |
| Phase wrap | 24 |
| Envelope | `0 0 10 40 70 99 70 40 10 40 70 99 70 40 30 20 10 5 0 0 0 0 0 0` |
| Envelope divisor and floor | `/99.0`, `+0.1` (PS2 spells it `* 0.01010101`) |
| X-envelope divisor | `/255.0` (PS2: `* 0.003921569`) |
| Layer rates | two floats, applied per layer from a 2-entry table |
| Random impulse | `(rand * 4.656613e-10 - 0.5) * 15.0` |
| Damping | `* 0.985` |
| Slew step and deadzone | `0.5`, `4.0` |
| Blend weights | `0.7`, `0.3` |
| Colour lerp | `0xff000000` to `0xff808080` |
| Ramp bounds | `(10, 350)` and `(30, 286)` |
| Blend func args | `(0, 2, 10, ...)` |
| X step | 2 |
| Texture | `data/defaults/loading/LoadingPulseOverlay.mip`, same literal |

The surrounding structure matches too. `Loading_Show` (`0x001da0a0`) has the
same `if (param_1 == 2) param_1 = current;` idiom, the same six-case tip filter
comparing against `"Campaign"`, `"Racebox"`, `"adhoc"`, `"online"`,
`"Turbo"`, `"Control Type"` and `"Weapon Pads"`, the same `rand() % count`
pick-with-retry, the same copy of name / body / title / four coordinates /
`ShowHelp` into a global tip record at `g_loading_tip` (`0x002f9c88`), and even
the same otherwise-unexplained `(0x1b0, 0x3c | 0x1e, 0x20, 0x20)` call that the
PSP makes to `FUN_0891f0c0`. Confidence **90** that these are the same
function ported, not two independent implementations.

Call sites, from the `jal` delay slots, mirror the PSP's:

| Call site | In | Arg |
| --- | --- | ---: |
| `0x00102768` | `FUN_00102720` (main loop) | 0 |
| `0x001027c8` | `FUN_00102720` (main loop) | 4 |
| `0x00123bf4` | `FUN_00123b98` | 6 |
| `0x00123d08` | `FUN_00123cc0` | 4 |
| `0x00124cbc` | `FUN_00124ae8` | 4 |

## Divergence 1: no thread, an object instead

PSP spawns a real thread named `"Load screen"` and stores the type in the
global `g_loading_screen_type`. PS2 allocates a 0x38-byte object
(`FUN_00209bc0(0x38)`), constructs it with `Loading_ScreenConstruct`
(`0x001d9a90`), and registers it; the screen type lives at object `+0x20` as
well as in the global `g_loading_screen_type` (`0x0028285c`). The draw side,
`Loading_ScreenDraw` (`0x001d9c20`), is a `while (true)` loop bracketed by
`FUN_0020a5c0`/`FUN_0020a5e0` around a named critical section - the name string
is `"loading screen"` at `0x002bc0f8`, lower-cased where the PSP's thread name
is `"Load screen"`.

The literal `"Load screen"` does not exist in `SCES_547.48` at all, which is
why the PSP's fastest anchor does not transfer. Confidence **85**.

## Divergence 2: type 0 is a publisher screen, and type 6 draws nothing

PSP type 0 is `Loading_DrawBootLogo`, two untraced sprites. PS2 type 0 loads
one of five language-specific SCEE screens, selected by `FUN_00202a40()`:

```
Data/Defaults/SCEE_Presents_Eu.pct    (default)
Data/Defaults/SCEE_Presents_Ger.pct   (1)
Data/Defaults/SCEE_Presents_Fra.pct   (2)
Data/Defaults/SCEE_Presents_Ita.pct   (3)
Data/Defaults/SCEE_Presents_Spa.pct   (4)
```

drawn centred, **with no wave**. That is a European-publisher plate the PSP USA
build has no equivalent of. Confidence **88**: five sibling literals behind one
switch, and the naming is unambiguous.

Types 2 and 4 load `Data/Defaults/Loading/LoadingBackTop.pct` (or a runtime
name whose extension `Loading_ScreenConstruct` rewrites to `.pct` with
`strrchr(name, '.')`), draw it full-screen, then call the text and the wave.
`.pct` throughout, where the PSP uses `.mip` - the tip XML still says `.mip`
and the loader rewrites it, which is the mechanism by which one shared XML
serves both discs.

**Type 6 draws nothing on PS2.** Both `Loading_ScreenConstruct` and
`Loading_ScreenDraw` are plain compare chains over `{0, 2, 4}`; 6 falls out of
both without loading a texture or reaching a draw branch. On PSP, type 6 shares
the tip path with type 4 and renders the full screen. Confidence **75** - the
control flow is unambiguous, but no runtime check was made and the surrounding
object may do something not read here.

There is also **no type-3 call site on PS2**, where the PSP's
`InGame_Destruct` passes 3 to tear the screen down.

## Divergence 3: no fade, no freeze

PSP passes a time-based alpha into the wave: `Loading_Ramp255(30, 60, t)` with
`t` in 33,333 us ticks, so the screen fades in between 1.0 s and 2.0 s. PS2
calls both the text and the wave with a **hard-coded `0xff`**:

```c
FUN_001da658(0xff, 0, 0, 0, 0);   /* text */
Loading_DrawWave(0xff);
```

Separately, PSP gates the whole motion update on `g_loading_finished`, so the
wave freezes in place while the screen fades out. **The PS2 inner loop has no
such gate** - the oscillators integrate unconditionally for as long as the
screen is up. Confidence **85** for both, read directly off the two
decompilations.

## Divergence 4: geometry, and one constant that was not rescaled

| | PSP | PS2 |
| --- | ---: | ---: |
| X loop bound | `0x1e0` = 480 | `0x280` = 640 |
| Columns | 240 | 320 |
| Quad size | 2 x 32 | 2 x 45.71430 |
| Band top | 220.0 | 357.14285 |
| Band bottom | 252 (top + 32) | 402.85715 |

`45.71430 / 32 = 1.428571` exactly, and `357.14285 / 1.428571 = 250.0`
exactly. So the quad is the PSP's 32 units scaled by 10/7, and the pre-scale
base is **250, not 220** - the band sits lower in its own space. What the PS2
build takes its vertical extent to be, and therefore why 10/7 rather than
448/272, is *not determined*; see below.

The X-envelope bounds were **not rescaled**. `Loading_Ramp255(10, 350, x)` and
`(30, 286, x)` are the same on both, while the loop now runs to 640, so the
alpha and amplitude ramps finish at 55 % of the PS2 width against 73 % of the
PSP's and the wave is at full amplitude across a proportionally wider strip.
Confidence **88**; the constants are literals in both.

The PS2 also does **not mirror-tile the texture**. PSP computes
`u = x & 0x3f; if (u > 0x1f) u = 0x3f - u` - a triangle wave that mirrors the
32-pixel strip. PS2 reads the texture object's own width and height through
vtable slots `+0x1c` and `+0x24` and uses `(0,0)`-to-`(w,h)` for every quad.
Confidence **85**.

## The text: three runs, not four

`FUN_001da658` draws the tip body, the tip title at 1.2x in `0xfffbf67a`, and
`MSC_LOADING` - the same three the PSP draws, in the same order, with the same
colour. **The PSP's fourth run, `MSC_LOAD_HELP`, is absent**: the string does
not exist anywhere in `SCES_547.48`, and there is no `ShowHelp`-gated block.
The `ShowHelp` attribute is still parsed and stored (`DAT_002f9d64`), it just
has no consumer. Confidence **88**.

One more difference worth having: PS2 **rescales the tip's XML coordinates at
runtime**, multiplying X by `framebuffer_w * (1/480)` and Y by
`framebuffer_h * (1/272)`, i.e. it treats the shared XML's numbers as
PSP-space and maps them onto whatever the PS2 framebuffer is. The wave's own Y
constants are hard-coded and get no such treatment. That inconsistency is in
the shipped code, not in this reading.

## The disc side

| Asset | PSP `Data.wad` | PS2 `WADS2.WAD` |
| --- | --- | --- |
| `Data\Plugins\loading\Definition.xml` (`8a156c09`) | entry 1126, 4,361 B | entry 3625, 6,283 B (lzss) |
| `Data/Defaults/Loading/LoadingBackTop.pct` (`294a8091`) | absent | entry 137, 2,317 B (lzss) |
| `data/defaults/loading/LoadingPulseOverlay.mip` (`d857f34b`) | entry 68, 2,064 B | **not found** |

The PS2 copy of the plugin XML is the *unminified* one - XML declaration,
comments, indentation - and holds **22** `<PI_LoadingScreen>` entries against
the PSP's 26. Both are 100 % static `.mip` `ImageSrc` values with zero movie
references, which is the disc-side half of the no-video conclusion on the
platform that could most easily have had one. Confidence **95**, the files were
decompressed and read.

**`LoadingPulseOverlay.mip` was not located on the PS2 disc.** It is not in
`WADS2.WAD`'s or `WADSP.WAD`'s directories, and its hash does not appear
anywhere in the 85 MiB of `PRERACE.WAD` scanned as raw bytes. The hash function
is not in doubt - the same hasher found the other two rows of the table above
inside `WADS2.WAD`. What is not ruled out is the blob living *inside* an
LZSS-compressed entry as a nested archive, which a raw scan cannot see. Left
open rather than called absent; see below.

## Symbols

Same names as the PSP page wherever the function is the same function, so the
two trees can be read side by side.

| Address | Kind | Name | Conf |
| --- | --- | --- | ---: |
| `0x001da0a0` | function | `Loading_Show` | 88 |
| `0x001d9a90` | function | `Loading_ScreenConstruct` | 85 |
| `0x001d9c20` | function | `Loading_ScreenDraw` | 85 |
| `0x001d95a8` | function | `Loading_DrawWave` | 90 |
| `0x001da658` | function | `Loading_DrawText` | 85 |
| `0x001d9408` | function | `Loading_Ramp255` | 88 |
| `0x001d9558` | function | `Loading_SlewToward` | 85 |
| `0x001d9470` | function | `Loading_LerpColour` | 85 |
| `0x0028285c` | data | `g_loading_screen_type` | 85 |
| `0x00282834` | data | `g_loading_wave_phase` | 85 |
| `0x00282838` | data | `g_loading_wave_rates` | 85 |
| `0x002bc098` | data | `g_loading_wave_envelope` | 90 |
| `0x002f9c88` | data | `g_loading_tip` | 82 |

`Loading_Ramp255` (`0x001d9408`), `Loading_SlewToward` (`0x001d9558`) and
`Loading_LerpColour` (`0x001d9470`) are byte-for-byte the same three helpers as
their PSP counterparts once the calling convention is discounted: the clamped
`((t - lo) * 255) / (hi - lo)` ramp, the move-toward-with-deadzone slew called
with `(0.5, 4.0)`, and the four-channel packed-colour lerp. They are named from
that correspondence plus their own call sites, not from strings.

`g_loading_wave_envelope` at `0x002bc098` is the 24-float heartbeat table, the
same 96 bytes as the PSP's, found once in the binary. `g_loading_wave_phase`
(`0x00282834`) is its index, wrapping at 24; `g_loading_wave_rates`
(`0x00282838`) is the two-entry layer-rate table.

## Not determined

- **Where `LoadingPulseOverlay.mip` lives on the PS2 disc, or whether it ships
  at all.** The next step is decompressing `WADS2.WAD`'s 5,861 compressed
  entries and scanning each for a nested directory carrying `d857f34b`. If it
  genuinely is absent, the PS2 wave draws with whatever
  `FUN_0010c1e0` returns for a missing name, and that is worth knowing.
- Why the vertical scale is 10/7 and what framebuffer height the wave's
  hard-coded `357.14285` assumes. The text path scales by `1/272` at runtime;
  the wave does not, so the two disagree about the coordinate space.
- Whether type 6 really renders nothing, which wants a runtime capture rather
  than another reading.
- `FUN_00202a40`, the language selector behind the five SCEE plates.
- Nothing on this page is verified at runtime.
