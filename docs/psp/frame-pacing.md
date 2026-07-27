# Frame pacing and the simulation timestep

**The original does not use a fixed timestep.** It computes a real elapsed
delta each frame and passes it straight into the update.

This is the answer to the project's longest-standing open question, and it is
not the answer that was assumed. Recovered from `BOOT.BIN` (Pulse PSP,
UCUS-98712) at image base `0x08804000`.

## What the original does

### Delta time is measured, not fixed

`0x08804978`, called once per main-loop iteration:

```
088049b4  jal 0x08a7719c            ; sceKernelGetSystemTimeWide
088049cc  subu a0, v0, a0           ; dt_ticks = now - previous
088049ec  cvt.s.w f20, f20
08804a08  lwc1 f14, 0x44(s0)        ; game->secondsPerTick
08804a0c  mul.s f20, f20, f14       ; dt = ticks * secondsPerTick
08804a10  c.lt.s ...                ; clamp low at 0.0
08804a24  c.le.s ...                ; clamp high at 0.067
08804a9c  jal 0x0894402c            ; scene graph update(dt)
```

`game+0x44` is `1.0f / sceKernelUSec2SysClock(1000000)`, set once at
construction. So `dt` is real seconds, clamped to `[0, 0.067]`. The upper clamp
is almost exactly four frames at 60 Hz.

Confidence: **92**.

### Fixed-timestep overrides exist and are dead

The same function contains three fixed-step paths, each behind a global:

| Constant | Value | Guard |
| --- | --- | --- |
| `0x3D088889` | 1/30 | `0x08abf5d8` |
| `0x3C888889` | 1/60 | `0x08abf5d9` |
| `0x3D088889` | 1/30 | `0x08abf5cc` |

All three globals have **zero write cross-references** and are zero in `.data`.
`0x08abf5d8` is also read by HUD code to suppress the HUD, so this is a
video-capture mode, disabled in retail.

Confidence: **88**. A write through an unresolved pointer cannot be excluded.

### Presentation is vblank-locked, at one or two vblanks

`0x0891e1b8`, the end-of-frame present:

```c
n = 2;                                          // default
if (*(char *)(display + 0x116c)) n = 1;         // set while racing
n = (n - sceDisplayGetVcount()) + last_vcount;
if (n == 0) sceKernelDelayThread(50);
else for (; n > 0; n--) sceDisplayWaitVblankStartCB();
```

`display + 0x116c` is written in exactly three places: the display constructor
sets it to 0, the **`InGame`** state constructor (`0x08812d58`) sets it to 1,
and the `InGame` destructor sets it back.

So **racing presents at one vblank (~59.94 Hz) and menus at two (~29.97 Hz)**.

Confidence: **87**.

Note there is no catch-up. If a frame overruns, `n` goes negative and the frame
presents immediately, so the next `dt` is simply larger.

### 1/60 is nevertheless the authored rate

Fourteen sites load `0x3C888889` (1/60); only the pacing code loads 1/30. They
share one idiom:

```c
n = (int)(dt / 0.016666668f);
for (i = 0; i < n; i++)
    x += (target - x) * k;
```

Seen at `0x088f157c` (four times), `0x0885efd0`, `0x08875900`, `0x0885e28c`,
`0x0893af20` and others. **Every exponential-smoothing coefficient in the engine
is tuned for a 1/60 s step.** The remainder is discarded: there is no
accumulator carry.

Confidence: **90**.

## What this means for OpenAntiGrav

**60 Hz is now evidence-backed rather than assumed**, but as the *authoring*
rate, not as something the original actually steps at.

The original is frame-rate dependent. Ours is not, deliberately. See
[ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md) for the
decision and its consequences, and
[determinism](../architecture/determinism.md) for the rules that follow.

The practical consequences:

1. A dropped frame in the original produces a 2/60 step, and any integrator
   using raw `dt` diverges from a fixed-step reimplementation from that point
   on. Trace comparison has to account for this.
2. Where the original sub-steps at 1/60 and discards the remainder, we should
   reproduce that exactly, including the discard. It is observable behaviour,
   not an implementation detail.
3. The PSP LCD runs at ~59.94 Hz, not 60. Over a five-minute race that is about
   18 frames of drift against a true 60 Hz clock.

## Still unknown

**Whether the craft physics integrator sub-steps at 1/60 or integrates raw
`dt`.** This is the single most important follow-up for M4: it decides whether
ship handling is frame-rate dependent in the original.

The 1/60 sub-stepping was confirmed in effects, camera and visual smoothing
code. The physics integrator was not located. Suggested starting points:
`"antigrav_height_adjust"` at `0x08a7b1ac`, referenced from `0x08839880`, and
the handling-parameter loader near `"%s HANDLING STATS"` at `0x08a7b9a0`.

## Verified at runtime

The previous version of this page ended by saying nothing here had been checked
against the game running, and proposing exactly that check. It has now been done,
with [PPSSPP's websocket debugger](../reverse-engineering/ppsspp-debugger.md),
during an actual Time Trial on Talon's Junction. What was sampled is `dt` at
`craft+0x1c8` - the value this frame hands the craft update - over 120
consecutive calls of `Ship_UpdateCraft`, rather than `f20` at `0x08804a9c`.

**Delta time really is variable.** Over 120 ticks: minimum `0.016396`, maximum
`0.016973`, **72 distinct values**. A fixed step would have produced one.

**Its mean is the vblank rate, not 60 Hz.** Mean and median both `0.016683`,
which is `1/59.94` to five decimal places, against `1/60 = 0.016667`. That is
this page's "racing presents at one vblank" and "the PSP LCD runs at ~59.94 Hz"
showing up in the number the simulation is actually fed. Independent confirmation
of two separate findings from one measurement.

**The three fixed-timestep globals are still zero**, read before and after those
120 ticks of racing. This closes the hedge this page carried at confidence 88 -
"a write through an unresolved pointer cannot be excluded" - as far as it can be
closed: no such write happens during boot, the front end, or a race. Raise to
**93**. Not 100, because a mode never entered here could still write them.

**`display+0x116c` reads 1 while racing**, which is what this page predicts from
the `InGame` constructor, and `g_game+0x44` reads `1e-6`, confirming the timer's
units are microseconds. Both **runtime-verified**.

One caveat that should not be dropped: this is an emulator, and `dt` derives from
emulated `sceKernelGetSystemTimeWide`. The *jitter* is PPSSPP's timing as much as
the game's. What carries over to hardware is the shape - variable, not fixed -
and the mean sitting on the vblank period, since that is imposed by the present
path rather than by wall-clock noise.

**Still not verified:** whether the craft physics integrator sub-steps at 1/60 or
integrates raw `dt`, which is the question above and is unaffected by any of
this.
