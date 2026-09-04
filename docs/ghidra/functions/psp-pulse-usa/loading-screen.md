# The loading screen: the wave is procedural, not a movie

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`.

**The names here are applied**, from [names.tsv](names.tsv). See
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

## The question this page answers

Pulse's loading screen carries a rippling, liquid-looking horizontal band. The
obvious hypotheses were a movie (a third `.PMF` beside the ones in
[video](frontend-video.md)) or a pre-rendered frame sequence. **Both are
wrong.** The band is generated at runtime: a randomly excited, damped
travelling waveform, drawn as a strip of small textured quads using one 32x32
texture, `data/defaults/loading/LoadingPulseOverlay.mip`. No video is
involved, and there is no animation frame sequence for it anywhere on the disc.

The frame sequence that *does* exist on the disc and looks liquid -
`Data\Tex\caustics\save.NN.mip`, 32 frames of water caustics - belongs to the
in-game renderer, not to the loading screen. That is recorded at the bottom of
this page so the next person does not re-follow the same lead.

## Entry point and the screen type

`Loading_Show` (`0x0890aed8`) is the only way in. It takes the screen type as
its single argument, stores it in `g_loading_screen_type` (`0x08abf45c`),
resets `g_loading_start_time` (`0x08abf460`) from
`sceKernelGetSystemTimeLow`, and creates the thread:

```c
g_loading_thread_id = sceKernelCreateThread("Load screen", Loading_ThreadMain,
                                            0x10, 96000, 0x4000, 0);
sceKernelStartThread(g_loading_thread_id, 0, 0);
```

The literal `"Load screen"` at `0x08a8811c` is the thread name, which is what
makes the pairing unambiguous. Confidence **90**.

`g_loading_screen_type` (`0x08abf45c`) was listed as *not determined* on
[main loop](main-loop.md). It is now determined. Every call site passes a
constant, read out of the `jal`'s delay slot:

| Call site | In | Arg | Meaning |
| --- | --- | ---: | --- |
| `0x08807274` | `Game_MainLoop` | 0 | boot logo screen |
| `0x088072f0` | `Game_MainLoop` | 4 | tip screen |
| `0x08813380` | `FUN_08813328` | 4 | tip screen |
| `0x08821084` | `FUN_08820e90` | 4 | tip screen |
| `0x0888ca64` | `FUN_0888c408` | 6 | tip screen, second flavour |
| `0x08813144` | `InGame_Destruct` | 3 | tear down |

Type 2 is never passed in; it is written by the code itself
(`Loading_DrawBootLogo` sets it once the logo has faded), and `Loading_Show`
special-cases it - `if (param_1 != 2)` guards both the type store and the
timer reset, so passing 2 means "keep whatever is showing". Types 1 and 5 are
reachable by the dispatch but never selected. Confidence **88**: the values
are constants in delay slots, the dispatch is a plain compare chain, and the
type-4 branch is the one that reads the tip table.

`Loading_ThreadMain` (`0x0890b52c`) dispatches on it:

| Type | Draws |
| --- | --- |
| 0 | `Loading_DrawBootLogo` - two sprites, fade in then out, then sets type 2 |
| 2 | nothing; the thread idles and only watches for the finish flag |
| 3 | clears the tip and draws the wave over black |
| 4, 6 | tip background image, then the wave, then four text runs |
| anything else | as 4/6 but with no background texture |

## The wave

`Loading_DrawWave` (`0x0890a8e4`) is the whole effect, and it is short. Called
once per rendered frame from `Loading_DrawBackdrop` (`0x0890ae0c`), which
first blits the full-screen background (source 512x256, destination 480x272)
tinted from black to white by the fade alpha.

The texture is bound once, on first use:

```c
if (DAT_08af269c == 0) {
  DAT_08af269c = 1;
  FUN_089277ac(&DAT_08af26c0, "data/defaults/loading/LoadingPulseOverlay.mip", 0);
  ...
}
Gfx_BindTexture(&DAT_08af26c0);
```

`Gfx_BindTexture` (`0x08928460`) was already named from
[exhaust](exhaust.md) at confidence 75, so "this is a texture bind, and this
is the texture the wave samples" is not an inference first made here.

That is the **direct xref** that [WAD subsystem](wad-subsystem.md) was missing
when it used this filename only to demonstrate a hash-normalisation rule. The
string is at `0x08a88098`, referenced from `0x0890a970` inside this function
and from nowhere else in the binary.

Decoding the blob (`Data.wad` entry 68, hash `d857f34b`, 2,064 bytes) with the
[PSP texture](../../../formats/psp-texture.md) reader gives **32x32, 8 bits per
pixel**, a 256-entry palette ramping black to `(222,255,255)` - Pulse's cyan -
and a pixel grid of dark noise cut by a few bright horizontal scanlines. It is
a glow strip, not an image of anything. Confidence **95**: the header
arithmetic is exact and the decode was rendered.

Two further facts about the blob, measured while writing the renderer half
(`oag_render::loading::Pipeline`) and worth not re-deriving. **Every one of the
256 palette entries carries alpha 255.** The strip's shape therefore lives
entirely in the black-to-cyan colour ramp and its alpha channel carries
nothing, which is why the reimplementation samples the texel's rgb and takes
its alpha from the across-screen ramp instead. And **bit 0 of `+0x07` is set**,
so the pixels are stored in the GE's swizzled layout - the
[PSP texture](../../../formats/psp-texture.md) reader unswizzles them, and
reading the blob linearly gives a plausible-looking but wrong strip.
Confidence **95**, both being direct reads of the decoded bytes.

### The geometry

The draw loop walks screen X in steps of 2, from 0 to 480, and emits three
quads per column. Each quad is 2 pixels wide and 32 tall, sampling the 32x32
texture:

```c
FUN_08810f68(0x20, 0x20, 2, 0x20);           /* src 32x32 -> dst 2x32 */
...
u = x & 0x3f;  if (u > 0x1f) u = 0x3f - u;   /* triangle wave: mirrored tiling */
FUN_08810f84(x, (int)(y + 220.0), 0, u, 0, 0, 0);
```

So the strip is 240 columns wide, sits around **y = 220** on a 272-line
display, and the texture is mirror-tiled horizontally with period 64 - which
is why the scanlines never show a seam. Confidence **88**.

### The motion

Two oscillator layers are carried in one six-float stack array. Reading the
decompiler's `local_70[6]` as three pairs makes the algorithm obvious:
`[0..1]` are the slew-limited display offsets, `[2..3]` the raw tracked
offsets, `[4..5]` a per-layer energy term.

Per column, per layer *i*:

```c
target   = energy[i] * (x_envelope / 255.0) * (pulse / 99.0 + 0.1);
raw[i]  += (target - raw[i]) * rate[i];               /* rate = {0.1, 0.05} */
disp[i]  = Loading_SlewToward(disp[i], raw[i], 0.5, 4.0);
energy[i] = (energy[i] + (rand() * 2^-31 - 0.5) * 15.0) * 0.985;
```

Three points matter:

1. **The array is reset to zero at the top of every call, and the walk
   advances along X, not along time.** The randomness is therefore *spatial*:
   one frame's wave is a single random walk read left to right, flat at the
   left edge and fully developed at the right. Re-running it every frame is
   what makes it wriggle.
2. `rand()` is `FUN_089731c4` scaled by `4.656613e-10` = 2^-31, so the impulse
   is uniform in +/-7.5 and `0.985` is the per-column damping.
3. The two per-layer vertices are emitted at `raw[i] + 220`, and the third at
   `disp[0] * 0.7 + disp[1] * 0.3 + 220` - the slew-limited blend is a
   *separate, calmer* third band, not a filter on the other two.
   `Loading_SlewToward` (`0x0890a3e8`) only moves when the gap exceeds 4.0,
   and then by at most 0.5, so that band lags visibly.

`x_envelope` and the fade are both `Loading_Ramp255` (`0x0890a280`), a clamped
integer ramp `((t - lo) * 255) / (hi - lo)`:

| Call | Meaning |
| --- | --- |
| `Loading_Ramp255(10, 350, x)` | alpha across the screen, tinting `0xff000000` to `0xff808080` |
| `Loading_Ramp255(30, 286, x)` | wave amplitude across the screen |
| `Loading_Ramp255(30, 60, t)` | screen fade-in, `t` in 33,333 us ticks - 1.0 s to 2.0 s |

Confidence **96** for `local_70`'s own state recurrence (`raw`/`disp`/
`energy` evolving correctly column to column), up from 85 - see "Confirmed
at runtime: the recurrence matches exactly" below. **90** for the emitted
draw arguments matching that state (`0x0890ab60`'s and `0x0890ac10`'s own
`(x, y)`), narrower for a reason recorded in the "narrowly" runtime section
just after it - both were read from the decompiler alone before this pass.

### The pulse

`g_loading_wave_phase` (`0x08abf470`) advances once per frame and wraps at 24.
It indexes `g_loading_wave_envelope` at `0x08a88034`, 24 floats read straight
out of `.rodata`:

```
0  0  10  40  70  99  70  40  10  40  70  99  70  40  30  20  10  5  0  0  0  0  0  0
```

Two peaks of 99 with a shallow trough between them, a decay tail, then six
frames of silence: a **heartbeat**, and the reason the game is called Pulse.
At the thread's 30 Hz that is one beat every 0.8 s. The value enters the
amplitude as `pulse / 99.0 + 0.1`, so the wave never goes fully flat - it idles
at a tenth of full amplitude. Confidence **95**, up from 90: the table is a
literal, its index is bounded by the wrap, the divisor 99.0 matches its own
maximum, and a live capture crossing a call boundary read `phase` advancing
`1 -> 2` with `g_loading_wave_envelope[2]` = `10.0` read back at the same
instant - the table index and the table itself agree with each other live,
not just on paper.

`g_loading_wave_rates` at `0x08abf474` holds the two layer rates, `{0.1,
0.05}`. Confidence **95**, up from 85 - read live in every capture below
rather than assumed from the decompiler's literals.

`g_loading_finished` (`0x08abf454`) gates both the motion update and the
`Loading_SlewToward` call, so when loading completes the wave freezes in place
while the screen fades out. `Loading_ThreadMain` then counts five more frames
(`DAT_08abf458 > 4`) before `sceKernelExitDeleteThread`. Confidence **80**.

### Confirmed at runtime: the tip screen fires at cold boot, before the state machine exists

The type table above reads `0x088072f0` in `Game_MainLoop` as passing `4` (tip
screen), and it was unclear whether that call is reachable only after front-end
navigation or fires unconditionally at boot. **It fires unconditionally, and
immediately**: captured 2026-08-27 against `pulse-psp-usa.chd` in PPSSPP
v1.20.4 (`PPSSPPHeadless`, CPU paused at `pc=0x08804000` per `--debugger`'s
break-at-start, breakpoint armed at `Loading_Show` before the first
`cpu.resume`), the first two hits of `Loading_Show` are:

| Hit | `a0` (type) | Emulated time | State machine |
| --- | ---: | ---: | --- |
| 0 | 0 (boot logo) | 0.017 s | not yet constructed (`0x08b31784` unreadable) |
| 1 | 4 (tip/wave screen) | 4.154 s | not yet constructed |

No button was pressed between them or before. Both calls land before
`0x08b31784`'s state machine pointer is valid at all - consistent with the
observed cold-boot state sequence on [main loop](main-loop.md) starting at
`LogoFMV`, since these two `Loading_Show` calls are earlier than that.

**This retires the "reaching a type-4 screen needs ~3 minutes of front-end
navigation" difficulty** that the runtime-capture next step was scoped around:
a type-4, wave-bearing screen is up within about four seconds of any cold
boot, no menu walk and no sighted navigation required. The remaining capture
(`g_loading_wave_phase` and the emitted vertex Y values across a few frames)
can be taken directly off this same boot, breaking inside `Loading_DrawWave`
instead of `Loading_Show`. Confidence **92**: a direct breakpoint capture, two
hits, the type and ordering matching the static table exactly, on one binary
and one emulator version.

### Confirmed at runtime: the recurrence matches exactly

Whether `local_70`'s own state evolution - not yet whether that state
reaches the screen unchanged, see the next section - is what the game
actually computes, not just a plausible reading of the decompiler, is
closed. `scripts/psp-loading-wave-capture.py` breaks once per column inside
`Loading_DrawWave`, right after both layers update (`0x0890abbc`), and reads
`local_70[6]` off the function's own `sp` - Ghidra reports the array as
`Stack[-0x70]` inside a frame the prologue allocates with
`addiu sp,sp,-0x70`, so at runtime the array's address is simply `sp` after
the prologue, no offset arithmetic needed. Two captures against
`pulse-psp-usa.chd` in PPSSPP v1.20.4, chained straight off the cold-boot
`Loading_Show(4)` hit above with no idle time in between (the loading thread
tears itself down fast enough, running unthrottled, that pausing to
reconnect misses the whole window):

| Capture | Columns | Recurrence exact | Energy impulse within bound | Reset at call boundary |
| --- | ---: | --- | --- | --- |
| 1 | 127 | 126/126, max diff 3.4e-8 | 252/252 | not reached |
| 2 | 295 | 293/293, max diff 2.7e-7 | 586/586 | 1/1 |

"Recurrence exact" predicts each column's `raw`/`disp` in Python from the
*previous* column's captured state plus the current column's `x` and
`pulse` (the only unpredictable input, `rand()`'s impulse into `energy`,
only feeds the column after next, so it is checked separately as a bound
rather than predicted) - the same predict-then-diff shape as
`psp-watch-soundemitter.py`. Both max diffs are float rounding noise, not
model error. Capture 2 ran long enough to cross a `Loading_DrawWave` call
boundary: `x` wrapped back to 0, `disp`/`raw` for both layers read exactly
`0.0` there, and `g_loading_wave_phase` advanced `1 -> 2` with
`g_loading_wave_envelope[2]` (`10.0`) read back live at the same instant -
confirming the reset-to-zero, the phase advance, and the envelope indexing
together, not just the recurrence inside one call.

**`Loading_SlewToward`'s own gate and step size are live-confirmed, and only
for one layer.** In capture 2, `raw0` peaked at `4.4546` - past the `4.0`
gate - around `x` ~= 322, and `disp0` then stepped in exact `-0.5`
increments across 80 consecutive columns, matching the gate and the step
literal exactly, not just the pass-through case. `raw1` peaked at `3.4384`
and never crossed `4.0` in either capture, so `disp1` stayed exactly `0.0`
throughout both - layer 1's slew path itself is still unexercised, only its
no-op case is.

Confidence **96**: a live, bit-level match over hundreds of consecutive
column transitions plus one observed call boundary, on one binary and one
emulator version - the residual risk is the untested case of `pulse != 0`
mid-capture (`phase` stayed at 1 for all of capture 1, and 1 then 2 for
capture 2, both landing on `g_loading_wave_envelope` entries of 0 and 10,
never one of the 99-peaks), which a longer capture would close but does not
change the arithmetic being checked.

**Two traps this capture hit, both now documented in the script itself**:
`Loading_Show`'s own breakpoint went unhit for minutes of real wall-clock
even though `g_loading_screen_type` had visibly already flipped to 4 in
memory - a leftover breakpoint from an earlier debugger connection was
still armed, and PPSSPP v1.20.4 only fires the most-recently-added
execution breakpoint (see `ppsspp-debugger.md#each_hit_any`), so the stale
one silently outranked the new one with no error at all. And the loading
thread's own teardown can take the whole `PPSSPPHeadless` process down with
it mid-capture (`--timeout` has nothing left to wait for once nothing else
drives the front end forward) - both captures above ended in the debugger's
socket closing out from under a `cpu.resume` call, not a clean stop, which
is why the row counts are what they are rather than a round number.

### Confirmed at runtime, narrowly: the emitted draw arguments match too

The recurrence check above only ever reads `local_70` itself - it does not
confirm those values are what actually reaches the draw call, which is a
separate claim (a bug between the two would draw a wrong wave from
correctly-computed state). `scripts/psp-loading-wave-emit-capture.py`
closes that gap by breaking on the generic quad emitter both call sites
target (`0x08810f84` - `0x08804000 + 0xcf84`, the unrelocated
`func_0x0000cf84` call target; confirmed against PPSSPP's own live
`memory.disasm`, not just Ghidra's, since Ghidra's function manager has no
`Function` object at that address despite the doc naming it `FUN_08810f84`
below) and reading `a0`/`a1` - the emitted `(x, y)` - directly, predicting
`y` from `local_70` and comparing.

Three captures, **52 of 52 emitted `y` values matched their prediction
exactly**, across both the per-layer call site (`ra=0x0890ab60`) and the
third-band call site (`ra=0x0890ac10`). That confirms the wiring: the right
register carries the right value, and the `220.0` literal from `f26` is
used. `trunc.w.s` itself (`0890ab3c`) is a static disassembly reading, not
something this capture independently confirms - see why below.

**This is narrower than it sounds, and confidence reflects that: 90, not
96.** All three captures landed entirely inside `x=0..14` - a handful of
columns, well before `raw`/`disp` develop any real amplitude - so every
predicted `y` in every capture was `220`, from every call site, with `raw`/
`disp` themselves within a fraction of `0.0`. A wrong-register or
wrong-constant bug would still have shown up as a mismatch even at `y=220`,
but `220.0` truncated, rounded or floored are all `220` - so this capture
cannot tell truncation apart from rounding, and a spurious scale or
multiply on a near-zero `raw` would not have shown up either. It also never
distinguished `layer0` from `layer1` (both predict the same `220` there, so
the classifier's first-match tie-break always reads `layer0`). See "Not
determined".

**A fourth trap, found getting this capture to work at all**: arming this
breakpoint only after reaching the tip screen (the approach that works for
the column breakpoint above) never fires, because the boot logo screen
(type 0, drawn first) already calls this same generic emitter for its own
two sprites, and that JIT-compiles the block before this script gets a
chance to arm anything - `cpu.breakpoint.list` shows it armed exactly as it
should, but a breakpoint on already-JIT-compiled code does not take, with
no error and no distinguishing symptom. The fix is arming it before the
very first `cpu.resume`, from cold boot, then filtering hits in Python by
return address. That filtering also removes the need for a separate
sp-learning step: caught at the callee's very first instruction, before its
own prologue runs, `sp` is still `Loading_DrawWave`'s own `sp` - `local_70`'s
address directly, since `jal` never touches `sp`. And this breakpoint is far
hotter than any other on this page - every quad drawn anywhere this early in
boot goes through it - which comes with its own cost: all three runs stopped
within the same narrow band, about 130-140 total hits, `PPSSPPHeadless`
left running and burning CPU rather than exiting cleanly. Not root-caused;
recorded so a future capture attempt does not waste time rediscovering it.

### What it looks like

Reimplementing the loop above in a scratch script and plotting the three bands
reproduces the expected shape directly: pinned flat at the left edge, opening
into a ragged travelling ripple towards the right, amplitude swelling on the
99-frames of the envelope. That is now a corroborating check of a
runtime-confirmed reading, not a stand-in for one.

## The tip screen (types 4 and 6)

`Loading_Show` reads the `loading` plugin, which is
`Data\Plugins\loading\Definition.xml` - `Data.wad` entry 1126, hash
`8a156c09`, 4,361 bytes. It contains 26 `<PI_LoadingScreen>` elements and
nothing else:

```xml
<PI_LoadingScreen name="Bomb">
<Title TitleSrc="PRO_STATS_BOMB" TitleX="300" TitleY="40"></Title>
<Values LoadingGroup="Pickup" ShowHelp="true"
        ImageSrc="Data\Defaults\Loading\Pulse\Bomb.mip"></Values>
<Text StringSrc="MSC_LOAD_BOMB" StringY="70"></Text>
</PI_LoadingScreen>
```

`LoadingGroup` takes six values - `Pickup`, `gameplay`, `gamemode`,
`features`, `speedclass`, `promotion`. Every `ImageSrc` is a `.mip`. **There is
not one `.PMF`, `.pmf` or movie reference in the file**, which is the
disc-side half of the "no video" conclusion.

`Loading_Show` filters the 26 by game mode (a `switch` on each widget's
`+0xa0`, comparing the widget's own name against `"Campaign"`, `"Racebox"`,
`"adhoc"`, `"online"`, `"Photo Mode"`, `"Turbo"`, `"Control Type"`,
`"Weapon Pads"`), then picks one at random with `FUN_089731c4() % count`,
retrying while the chosen widget refuses. It copies the chosen entry into
`g_loading_tip` (`0x08af25c0`, a 0x80-byte name buffer with the rest of the
record following):

| Offset from `g_loading_tip` | Source field | XML attribute |
| --- | --- | --- |
| `+0x00` | widget `+0xa4` | `Values ImageSrc` |
| `+0xa0` | widget `+0x124` | `Text StringSrc` |
| `+0x80` | widget `+0x144` | `Title TitleSrc` |
| `+0xc0`, `+0xc4` | widget `+0x174`, `+0x178` | `TitleX`, `TitleY` |
| `+0xc8`, `+0xcc` | widget `+0x164`, `+0x168` | `StringX`, `StringY` |
| `+0xd8` | literal 1 | tip is valid |
| `+0xd9` | widget `+0x17c` | `ShowHelp` |

The mapping is confirmed from the other end: `Loading_DrawText`
(`0x0890a4ec`) draws the title at `+0xc0`/`+0xc4` and the body at
`+0xc8`/`+0xcc`, and gates the `MSC_LOAD_HELP` line on `+0xd9` - exactly the
`ShowHelp="true"` attribute. Confidence **85** for the whole table, which is
two independent readings agreeing.

Four text runs are drawn, in this order: the tip body (`Text StringSrc`),
`MSC_LOAD_HELP` at (240,160) if `ShowHelp`, the tip title (`Title TitleSrc`)
at 1.2x scale in `0xfffbf67a`, and `MSC_LOADING` at x=50. Their buffers are
sized by `strlen` in `Loading_ThreadMain` before the thread's 80,000-byte
display list is allocated.

## The caustics are not this

`Data\Tex\caustics\save.01.tga` through `save.31.tga`, odd frames only, are
16 literal strings at `0x08a88178`-`0x08a88358`, immediately after
`LoadingPulseOverlay.mip` in `.rodata`. **That adjacency is the trap**: they
belong to a different subsystem.

All 32 frames are on the disc (`Data.wad` entries 947-978, hashes verified
against `Data\Tex\caustics\save.NN.mip`, 5,584 bytes each, 128x64 at 4bpp with
a mip chain and an alpha-only white palette). Decoded, they are exactly what
the name says - a tiling water-caustic ripple loop, and the most
liquid-looking asset in the game.

But the only loader is `Texture_LoadEffectSurfaces` (`0x0890c958`), which is
called from `FUN_08821d90`, an in-game subsystem init that also calls the
already-named `Texture_LoadEngineFlare`; the same function goes on to load
`Data\Tex\Weapons\absorb_surface.mip` and
`Data\Tex\Weapons\leachbeam_surface.mip`. The only consumer of the resulting
handle array `g_caustic_frames` (`0x08b62d90`) is
`Texture_BindCausticFrame` (`0x0890dcfc`), which selects a frame with
`(int)(track_time * 30.0) & 0xf` and binds it, and whose three callers are all
in the mesh/track draw path. Nothing in `0x0890a280`-`0x0890b9xx`, the loading
screen's own address range, touches them. Confidence **85** that the caustics
are in-game water and have nothing to do with the loading screen.

## EU corroboration

The EU binary (`data/extracted/psp-eu/.../BOOT.BIN`) carries the same
`.rodata` block with identical internal spacing:

| String / table | USA file offset | EU file offset | Delta |
| --- | --- | --- | ---: |
| `MSC_LOAD_HELP` | `0x284098` | `0x283878` | - |
| wave envelope (24 floats, byte-identical) | `0x2840b4` | `0x283894` | `+0x1c` both |
| `data/defaults/loading/LoadingPulseOverlay.mip` | `0x284118` | `0x2838f8` | `+0x80` both |
| `Load screen` | `0x28419c` | `0x28397c` | `+0x104` both |
| `Data\Tex\caustics\save.01.tga` | `0x2841f8` | `0x2839d8` | `+0x160` both |

The 96-byte envelope table matches byte for byte and appears exactly once in
each binary. This is a stronger corroboration than the function-level diff that
[EU corroboration](../psp-pulse-eu/corroboration.md) failed to get for
`Loading_ThreadMain` itself, and it is region-independent: the effect is the
same on both discs. Confidence **90**.

## PS2 corroboration

**The PS2 build has the same wave, and it is not a video there either** - which
matters, because the PS2 is the platform where a video would have been cheap.
The same 96-byte envelope, the same eleven tuning constants, the same tip
filter, and a `loading` plugin XML on the PS2 disc that is likewise 100 %
static `.mip` images with no movie reference. Four things do diverge - no
thread, a five-language SCEE publisher plate for type 0, no time-based fade and
no freeze-on-finish, and a wider draw with two ramp bounds that were never
rescaled. Per [goals](../../../overview/goals.md) this page stays
authoritative; the divergences are tabulated on
[PS2 loading screen](../ps2-pulse-eu/loading-screen.md).

## Symbols

| Address | Kind | Name | Conf |
| --- | --- | --- | ---: |
| `0x0890aed8` | function | `Loading_Show` | 90 |
| `0x0890b52c` | function | `Loading_ThreadMain` | 88 |
| `0x0890a8e4` | function | `Loading_DrawWave` | 88 |
| `0x0890ae0c` | function | `Loading_DrawBackdrop` | 82 |
| `0x0890a4ec` | function | `Loading_DrawText` | 85 |
| `0x0890ac68` | function | `Loading_DrawBootLogo` | 75 |
| `0x0890a280` | function | `Loading_Ramp255` | 88 |
| `0x0890a3e8` | function | `Loading_SlewToward` | 85 |
| `0x0890a2fc` | function | `Loading_LerpColour` | 88 |
| `0x0890a2c4` | function | `Loading_LerpByte` | 85 |
| `0x0890c958` | function | `Texture_LoadEffectSurfaces` | 78 |
| `0x0890dcfc` | function | `Texture_BindCausticFrame` | 85 |
| `0x08abf45c` | data | `g_loading_screen_type` | 88 |
| `0x08abf448` | data | `g_loading_thread_id` | 90 |
| `0x08abf460` | data | `g_loading_start_time` | 85 |
| `0x08abf470` | data | `g_loading_wave_phase` | 85 |
| `0x08abf474` | data | `g_loading_wave_rates` | 85 |
| `0x08a88034` | data | `g_loading_wave_envelope` | 90 |
| `0x08abf454` | data | `g_loading_finished` | 80 |
| `0x08af25c0` | data | `g_loading_tip` | 85 |
| `0x08af2699` | data | `g_loading_tip_show_help` | 85 |
| `0x08b62d90` | data | `g_caustic_frames` | 85 |

`Loading_LerpColour` (`0x0890a2fc`) unpacks a `0xAARRGGBB` literal into four
bytes, runs each through `Loading_LerpByte` (`0x0890a2c4`) against the second
colour's channel with the same 0-255 parameter, and repacks. Every fade on
this screen goes through it. `Loading_DrawBootLogo` is `0x0890ac68`, reached
only for type 0; it draws a 480x64 sprite at (16,104) and an optional 128x32
sprite at (176,136), and its 75 is because neither sprite's source has been
traced back to a filename.

## Not determined

- **The internal recurrence is runtime-verified except for the `pulse != 0`
  case.** See "Confirmed at runtime: the recurrence matches exactly" above -
  both captures happened to land on `g_loading_wave_envelope` entries of 0
  and 10, never one of the table's two 99-peaks, so the `pulse / 99.0 + 0.1`
  amplitude term is exercised structurally but not at its largest live
  value. A longer `scripts/psp-loading-wave-capture.py --hits` run, or one
  timed to start nearer a beat, would close this; nothing about the
  arithmetic being checked would change.
- **The emitted draw arguments are runtime-verified only at undeveloped
  amplitude.** See "Confirmed at runtime, narrowly" above - all three
  `scripts/psp-loading-wave-emit-capture.py` captures landed inside
  `x=0..14`, where every predicted `y` is `220` regardless of layer or band,
  so the wiring is confirmed but a value-dependent bug (as opposed to a
  wiring bug) would not have shown up. `layer0` versus `layer1` has also
  never been distinguished for the same reason - both predict the same
  number that early. Closing this needs a capture that reaches well past
  x=14 *and* catches the draw arguments, which no run has done in the same
  pass yet; the script's own docstring records the ~130-140-hit ceiling that
  has stopped every attempt so far, unresolved.
- The exact signatures of `FUN_08810f68` and `FUN_08810f84`. Read here as
  `(src_w, src_h, dst_w, dst_h)` and `(x, y, z, u, v, ...)` from four
  consistent call sites, which is enough for this page but not enough to
  name them - and, per the runtime capture above, `FUN_08810f84` currently
  has no `Function` object in Ghidra's own database at all, despite being
  reachable and named from this page; PPSSPP's live `memory.disasm` is what
  confirmed the address, not Ghidra's function manager.
- `FUN_08813328`, `FUN_08820e90` and `FUN_0888c408`, the three non-main-loop
  callers of `Loading_Show`, and therefore what distinguishes type 6 from
  type 4. Both take the tip path; no difference in behaviour was found.
- Why types 1 and 5 exist in the dispatch but are never selected.
- `FUN_08821d90`, the in-game texture-init aggregator. Six subsystem loaders
  in a row, only two of them named. Confidence in the *role* is about 70;
  a name is not written down for it because "which subsystems" is the part
  that matters and that is unread.
