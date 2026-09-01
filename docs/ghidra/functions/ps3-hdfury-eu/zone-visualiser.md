# Zone's audio-spectrum visualiser: sixteen bands, ten segments each

**The open question this page closes**: whether HD/Fury's per-frame
`zoneTexVis` write is audio-reactive, or a stage-progress fraction.
[zone-effectsettings-loader.md](zone-effectsettings-loader.md)'s twenty-sixth
pass found the writer and left the value untraced at confidence 74. It is
audio, and the layout it writes is a bank of **sixteen ten-segment bar
meters**.

The maintainer's own play report - "in a Zone race the floor textures and the
billboards carry an audio-spectrum animation driven by the music"
([effectsettings.md](../../../formats/effectsettings.md)) - is now met by a
decompiled dispatch rather than by inference from key names.

## The band source: `SoundSystem_GetBandLevel`

`Environment_UpdateStageBlend` (`0x003da540`) calls `0x0067a7b8` once per
band, with the band index in `r3` and a float back in `f1`. That address is a
TOC-switching stub (`std r2,0x28(r1)` / `subis` / `addi` / `b`) whose real
target is **`0x00304530`**:

```c
double SoundSystem_GetBandLevel(uint band)
{
  if (0xf < band) {
    return (double)DAT_008b4c7c;          // 0.0
  }
  return (double)*(float *)(PTR_DAT_008b4c5c + band * 4 + 0x24);
}
```

**Sixteen bands**, `0..=15`, clamped, returning `0.0` past the end.
`PTR_DAT_008b4c5c` reads `0x00b6cc90`, so the array is sixteen floats at
`0x00b6ccb4`.

**That object is the sound system, not the renderer.** `FUN_00304be8`, in the
same module, branches on `PTR_DAT_008b4c5c[0xe8] == 2` to pick between the
literals `"Stereo"` and `"Surround"`. A struct whose `+0xe8` selects a speaker
layout is an audio object; a stage-progress counter is not. Confidence **85**.

**There are two band arrays, and the layout closes on itself.**
`SoundSystem_GetBandLevel32` (`0x00304558`) is the same shape with the index
clamped to `0x1f` and the base at `+0x64`:

| offset | size | what |
| --- | ---: | --- |
| `+0x00`..`+0x23` | 36 B | meters, written by `SoundSystem_UpdateMeters` (`0x00304320`) |
| `+0x24`..`+0x63` | **16 floats** | band levels, what the Zone visualiser reads |
| `+0x64`..`+0xe3` | **32 floats** | band levels at twice the resolution |
| `+0xe8` | 1 B | speaker mode, `2` = stereo |

Thirty-two floats from `+0x64` end at `+0xe3`, immediately before `+0xe4` -
the arrays tile the struct exactly, which is the check that says the second
getter's `100` is a base and not a coincidence. Confidence **86**.

## What fills the sixteen bands: an auto-ranging normaliser

`FUN_00307e78`. Found through `get_field_access_context` on
`0x00b6cc90 + 36`, after a `stfsx` sweep of the whole audio module came back
**empty** - there is no indexed float store there, and `get_xrefs_to` is blind
to this binary's TOC-relative loads ([memory.md](memory.md)), so neither of
the two obvious routes finds it. Its first loop runs sixteen times over
`puVar10 + 0x24`, and per band:

```text
raw   = |source[b]|
peak  = peak[b]                     ; puVar10 + 0xe6c + 4b
floor = floor[b]                    ; puVar10 + 0xe2c + 4b

if peak > 0:  peak = max(peak + (peak - floor) * dt * -0.2, 0)
ceiling = peak * 0.9
floor   = (ceiling < floor) ? ceiling
                            : min(floor + (peak - floor) * dt * 0.2, ceiling)
peak    = max(peak, raw)            ; the current sample owns both ends,
floor   = min(floor, raw)           ; instantly, in both directions

band[b] = (peak - floor <= 1e-8) ? 0
                                 : clamp((raw - floor) / (peak - floor), 0, 1)
```

Every constant is a float the function loads from its own TOC, resolved with
`scripts/ps3-toc.py`:

| TOC | value | role |
| --- | --- | --- |
| `+0x77a4` | `0.0` | the low clamp, and `SoundSystem_GetBandLevel`'s own out-of-range default |
| `+0x77a8` | `1.0` | the high clamp |
| `+0x783c` | `-0.2` | peak fall per second |
| `+0x7794` | `0.2` | floor rise per second |
| `+0x7840` | `0.9` | the floor's ceiling, as a fraction of the peak |
| `+0x7844` | `1.0e-8` | the minimum span that counts as a signal |

Confidence **86**, and the **order** above is read at instruction level
(`0x00308010`-`0x00308124`), not taken from the decompiler - which matters
here more than usual, because a float loop with aliased temporaries is
exactly where a decompile reorders stores and the order is what a port
encodes. Each step is a named instruction pair:

| step | instructions |
| --- | --- |
| decay only a positive peak | `fcmpu f13,f9` / `ble 0x003080d8` |
| `peak += (peak-floor) * dt * -0.2`, floored at 0 | `fmadds f0,f0,f7,f13` / `fsel f0,f13,f9,f0` |
| `ceiling = peak * 0.9` | `fmuls f12,f11,f6` |
| floor snaps down when it exceeds the ceiling | `bgt 0x00308010` / `stfs f12,0x0(r10)` |
| else rises, capped at the ceiling | `fmadds f13,f13,f31,f0` / `fsel f0,f0,f13,f12` |
| `peak = max(peak, raw)` | `fcmpu f10,f11` / `stfs f10,0x0(r8)` |
| `floor = min(floor, raw)` | `fcmpu f13,f10` / `stfs f10,0x0(r10)` |
| span against `1e-8`, else store `0.0` | `fcmpu f0,f5` / `ble 0x00308108` / `stfs f9,0x0(r9)` |
| `clamp(ratio, 0, 1)` | `fsel f0,f0,f9,f13` / `fsel f12,f12,f0,f8` |

**This is the part a port could not have guessed**: it is
not a decibel curve at all. A band is measured against **its own** recent
floor and peak, so every band fills its meter on its own material - a
bassline and a hi-hat both read across the full range - and a band with
nothing in it collapses to no span and reads zero rather than reading its own
noise floor. A fixed dB floor, which is what this project had invented, pins
a quiet band at the bottom of its bar for a whole race.

**One step further back is still unrecovered.** `raw` is read at
`(b & 7) * 48 + *(int *)(puVar10 + (b >> 3) * 4 + 0x614)` - two pointers, each
to eight 48-byte records, the band magnitude at offset `0`. What updates
those filter states was not traced. So *how a band is measured* - its centre
frequency, its filter, its window - remains unread, and this project supplies
its own and says so.

**The 32-band array is filled in the same function's second loop**, from a
triple-buffered block at `+0x494 + ring * 0x80` (32 floats each,
`ring = *(int *)(puVar10 + 0x490)`, cycling 0-2) under a mutex pair
(`FUN_006766f8`/`FUN_006770a8`) - which is the cross-thread handoff from
whatever produces the analysis. It is normalised by a whole-block gain rather
than per band. Nothing in this project reads it; recorded because it is the
same data at a second resolution and would be the thing to port if the front
end's `waveTexture` turns out to want it.

## The layout: bars at 1-160, a smooth slot each at 161-176

Read at instruction level from `0x003dba04`, where the function loads the
`zoneTexVis` wrapper (`lwz r10,0x32b0(r30)`) and its pixel buffer
(`lwz r9,0x10(r10)`). Eleven pointers are set up:

| register | init | texel |
| --- | --- | ---: |
| `r16` | `r9 + 0x4` | 1 |
| `r29` | `r9 + 0x8` | 2 |
| `r28` | `r9 + 0xc` | 3 |
| `r27` | `r9 + 0x10` | 4 |
| `r24` | `r9 + 0x14` | 5 |
| `r23` | `r9 + 0x18` | 6 |
| `r22` | `r9 + 0x1c` | 7 |
| `r21` | `r9 + 0x20` | 8 |
| `r20` | `r9 + 0x24` | 9 |
| `r19` | `r9 + 0x28` | 10 |
| `r18` | `r9 + 0x284` | **161** |

The first ten advance by `0x28` - **ten texels** - per loop iteration
(`0x003dbba4`-`0x003dbbc8`); `r18` advances by `0x4`, one texel
(`0x003dbb9c`). The loop counter `r31` runs to `0xf`
(`cmpwi cr7,r31,0xf` at `0x003dbb90`), so sixteen iterations. Therefore:

```text
texel 0                       never written (the load-time zero-fill stands)
texels 1 + 10b ..= 10 + 10b   band b's ten-segment bar
texel  161 + b                band b's smooth slot
```

and `161 = 1 + 10 * 16` exactly - the smooth block begins where the
sixteenth bar ends. Confidence **88**.

### The bar is binary, not a gradient

`0x003dbaa0` onwards is a ladder: compare the level against `1`, store `r17`
into texel 1, branch out if `level <= 1`; compare against `2`, store `r17`
into texel 2, branch out; ... through segment 10. The exit targets
(`0x003dbc30` onwards) store `r15` into every *remaining* segment, and
`r15` is set by `li r15,0` at `0x003dba30`. So **a lit segment carries the
whole colour word and an unlit one carries `0x00000000`** - the alpha byte
included.

`r17` is assembled at `0x003db8f8` (`or r17,r0,r10`) out of four `fctiwz`-
converted colour components, in the same packing idiom the rest of the
function uses. Which authored colour it is was not traced; this project binds
the stage's own `EQ colour tint` there, at the confidence
`zone_grade::ZoneGrade::eq_tint` already records.

### The level: `min(10, trunc(held * 11.0))`

At `0x003dba74`-`0x003dba94`:

```text
f0 = held * f30 ; fctiwz ; stfiwx -> r0
if r0 > 10 { r0 = 10 }
```

`f30` is `lfs f30,-0x5a48(r2)` -> `0x008b797c` -> `0x41300000` = **11.0**.
`fctiwz` rounds toward zero, so the first segment lights at `1/11` and the
tenth from `10/11` on. A `10.0` scale would light the tenth only at exactly
`1.0`; the disc chose eleven and a clamp.

### The smooth slot: the tint scaled 0-255 by the level

At `0x003dbb28`/`0x003dbc70` the same held value is multiplied by
`f31 = lfs -0x5a44(r2)` -> `0x008b7980` -> `0x437f0000` = **255.0**, converted
and clamped to `[0, 255]`. `0x003dbb50`-`0x003dbb88` then multiplies each of
four 0-255 colour components by that value and keeps the high byte of each
product (`mullw` + `rlwinm` per lane), OR-ing them into one packed word
stored at texel `161 + b`. That is the tint scaled smoothly by the level,
beside the quantised bar.

### The ballistics: instant attack, `0.1` linear decay per frame

At `0x003dbbe4`-`0x003dbc24`:

```text
f1   = SoundSystem_GetBandLevel(b) * gain
f13  = held[b]                       ; at r30 + 0x32f8 + 4b
raw[b] = f1                          ; at r30 + 0x32b8 + 4b
if f1 >= f13 { held[b] = f1; draw the bar }
else {
  if held[b] > 0.0 { held[b] -= 0.1 }
  if held[b] < 0.0 { held[b] = 0.0; clear the bar }
}
```

The decay is `lfs f0,-0x5a7c(r2)` -> `0x008b7948` -> `0x3dcccccd` = **0.1**,
and the floor is `lfs f12,-0x5a4c(r2)` -> `0x008b7978` -> `0.0`. The caller
is `Scene_PrepareFrame`, so this is **per rendered frame** and frame-rate
dependent in the original.

`gain` is `lfs f0,0x4(r4)` where `r4` is a base fixed across all sixteen
bands, computed at `0x003dba08`-`0x003dba38` off a TOC pointer. **Not
identified** - do not read it as `EQ brightness` without tracing it; that
key already has a located consumer elsewhere.

## The shipped art confirms the stride, three times over

This is the strongest evidence on the page, because every number in it was
measured **before** the layout above was read, and recorded in
[zone-shader.md](zone-shader.md)'s own alpha histogram of the fifteen
`zoneModeTrack*.gtf`:

| file | alpha plateaus | under this layout |
| --- | --- | --- |
| `zonemodetrack9`/`10` | `31, 32, ..., 40` | **band 3's ten segments**, `1+10*3` .. `10+10*3` |
| `zonemodetrack14` | groups of 4 on a **stride of 10** | four segments of each band; 10 is the band stride |
| `zonemodetrack6`/`7` | `1, 2, ..., 162` | every bar (1-160) plus the first two smooth slots |
| `zonemodetrack8` | `26, 35, 46, 55, 66, 75, 86, 90, 169, 255` | one segment from each of bands 2-8, plus band 8's smooth slot at `169 = 161+8` |

Ten consecutive integers starting at 31 is not something a wrong reading of a
loop stride produces, and `track14`'s stride of 10 is the band stride stated
by the art itself. Confidence **90** on the layout once the art is admitted
as evidence, against the 88 the disassembly alone carries.

It also explains, exactly, why this project's own visualiser was invisible
before: it spread 32 bands linearly over 256 texels, so a surface authored to
show band 3's ten-segment bar instead sampled a near-constant interpolation
of bands 3-5, and every texel carried a *fraction* of the tint where the
original carries all of it or none.

## Names applied

| address | kind | name | confidence |
| --- | --- | --- | ---: |
| `0x00304530` | function | `SoundSystem_GetBandLevel` | 85 |
| `0x0067a7b8` | function | `SoundSystem_GetBandLevel_Stub` | 85 |
| `0x00304558` | function | `SoundSystem_GetBandLevel32` | 84 |
| `0x00307e78` | function | `SoundSystem_UpdateBandLevels` | 86 |
| `0x00304320` | function | `SoundSystem_UpdateMeters` | 76 |
| `0x00308f00` | function | `SoundSystem_Shutdown` | 88 |
| `0x00309230` | function | `SoundSystem_AudioThread` | 86 |
| `0x0030ac48` | function | `SoundSystem_Init` | 84 |
| `0x00307620` | function | `SoundSystem_OpenAudioPort` | 86 |
| `0x0030a668` | function | `SoundSystem_StartMultiStream` | 80 |
| `0x00b6cc90` | data | `g_sound_system` | 80 |

`g_sound_system` is named from `FUN_00304be8`'s `"Stereo"`/`"Surround"`
branch on `+0xe8` and nothing else, which is why it sits at 80 rather than
with the getter.

`SoundSystem_UpdateMeters` sits at 76 rather than with the rest: it is named
from what it writes (`+0x00`..`+0x20`, a pair of smoothed left/right levels
with their own decay) and from its object, not from a caller - it has **no**
`bl` or branch anywhere in the image, so it is reached through a function
pointer and its role as a callback is inferred.

## The band *magnitudes* are not produced by PPU code in this executable

Chased in the same pass, and this is a negative worth as much as the
positives, because it says where to stop. `SoundSystem_UpdateBandLevels`
reads its raw magnitude from
`(b & 7) * 48 + *(int *)(g_sound_system + 0x614 + (b >> 3) * 4)` - two
pointers, eight 48-byte records each. Four independent sweeps:

1. **`get_field_access_context` on `+0x614`, `+0x618` and `+0x61c`.** Between
   them the *entire image* touches those three pointers in exactly two
   places: `SoundSystem_Shutdown`'s free, and
   `SoundSystem_UpdateBandLevels`'s read. (`+0x61c` has a third, a master
   level read at `0x00304cc8`.) No allocation, no fill.
2. **`stw` at displacement `0x614`** program-wide: ten hits, one in this
   module and it is the shutdown's zeroing.
3. **`stfsx` program-wide**: 130 hits, **none** anywhere in the audio module.
4. **The audio thread's own body.** `SoundSystem_AudioThread`
   (`0x00309230`), the entry `sys_ppu_thread_create` is given under the name
   `"Audio MS Update Thread"`, loops on one call - `0x003084c0` - and that
   function is MultiStream bus and volume work end to end. No band analysis
   in it.

**The audio engine is Sony's SCREAM on MultiStream**, named outright by two
error strings (`cellScreamStartSoundSystem`) beside the game's own
`SoundSystem.cpp`. So the most likely reading is that the per-band magnitudes
are produced by the middleware or an SPU job into buffers the game allocates
and hands over, in which case **the sixteen bands' centre frequencies are not
in this executable at all**. Confidence **60** on that explanation - it is
inference from four absences, not a positive trace - but confidence **85**
that no PPU code here writes them, which is the part that bounds the work.

The practical consequence: `oag_audio::spectrum::band_frequencies` is this
project's own choice **permanently**, not a placeholder waiting on a read, and
should stay labelled that way.

## The init chain, mapped - and the buffers are not allocated in it

Followed the remaining thread. `SoundSystem_Init` (`0x0030ac48`) runs three
things before it creates the thread, and none of them allocates the band
blocks:

| address | what it does | evidence |
| --- | --- | --- |
| `0x00307620` | opens the console's audio output and port | `cellAudioOutGetSoundAvailability`, `cellAudioOutConfigure`, `cellAudioInit`, `cellAudioPortOpen`, `cellAudioPortStart` |
| `0x0030a668` | starts MultiStream and opens its buses | eight `FUN_00679ab8(0x8000n)` calls, then the `sys_ppu_thread_create` in the caller |
| `0x003093a8` | configures bus `0x80007`'s eight channels | an eight-iteration setup loop per channel |

So the allocation is further in still. This bounds the earlier negative
rather than lifting it.

### A correction: `+0x620`..`+0x69c` is the 7.1 speaker layout, not a filter bank

Worth recording because it was one step from being written down as the
filter-coefficient table it looks like. `SoundSystem_Init` fills eight
four-float groups there from a TOC table, and the values are **angles**:

| TOC | value |
| --- | ---: |
| `+0x77d0` | `0x3c8efa34` = **π/180** |
| `+0x77d4`, `+0x77d8` | `-30.0`, `+30.0` |
| `+0x77dc`, `+0x77e0` | `-110.0`, `+110.0` |
| `+0x77e4`, `+0x77e8` | `-145.0`, `+145.0` |

Times π/180 into a sin/cos pair, stored as `[cos, 0, -sin, 0]` per speaker.
`0`, `±30`, `±110`, `±145` is the standard 7.1 layout, so this is the
**positional-audio speaker table**, confidence 86. It has nothing to do with
the visualiser; it is recorded here because it sits four bytes past the band
pointers and is the obvious wrong reading of them.

### A second consumer draws the identical bar layout

`FUN_003ce2c0` calls `SoundSystem_GetBandLevel_Stub` at `0x003cea1c`, and the
loop around it is the **same shape instruction for instruction** as
`Environment_UpdateStageBlend`'s: ten pointers advanced by `0x28` each
iteration, the band index in `r3`, the returned float scaled by
`lfs f0,0x4(r4)`, and the same peak-hold compare against a held value. Only
two `bl` sites in the whole image reach that stub, and this is the other one.

**Two unrelated consumers drawing sixteen values as ten-segment bars is
independent corroboration that the array is an equaliser feed**, and it says
the bar layout is the engine's standard way of drawing one rather than
anything Zone-specific. `FUN_003ce2c0` has no `bl` caller either, so it is
reached through a pointer like its sibling; which screen it serves is
unread, though the front end's own `"Music Pulse Base"` / `"Music Pulse
factor"` / `waveTexture` string cluster is the obvious candidate.

### The one reading that is not settled

`SoundSystem_UpdateBandLevels` sources band `b` from
`(b & 7) * 48 + block[b >> 3]` - **two blocks of eight**, and those two
pointers sit immediately before the eight speaker vectors above. So `8` might
be the 7.1 channel count rather than a grouping of sixteen bands, which would
make the array per-channel rather than per-frequency.

Weighed the other way, and why this page still reads it as sixteen frequency
bands: two independent consumers draw it as an equaliser; there is a
**32**-entry sibling array with its own getter, which is a coarse/fine
spectrum pair and not a channel count; and the shipped `zoneModeTrack*.gtf`
alpha tags exactly sixteen ten-texel regions. Confidence **75** on "frequency
bands" specifically, against the **86** the layout and the level curve carry.
Recorded rather than dismissed: a watchpoint on the array during a race with
one speaker driven would settle it in one reading.

## Still open

- **Where `+0x614`/`+0x618` are allocated and to whom they are handed.**
  `SoundSystem_Shutdown` frees them, so something allocates them; neither
  field context nor a displacement search finds it, and it is not in the
  thread-creating init (`0x0030ac48`, 285 instructions, no `0x61x`
  displacement anywhere). It is reached through a register chain. Finding
  the call it is passed to would settle the middleware reading above one way
  or the other.
- **What fills the triple-buffered 32-float block** at `+0x494`, behind the
  mutex - the producer side of the cross-thread handoff, and the same
  question in its second form.
- **What `lfs f0,0x4(r4)` is**, the fixed per-frame gain applied to every
  band before the hold.
- **Which authored colour `r17` carries.** Assembled at `0x003db8f8` from
  four converted floats; the `EQ colour tint` reading is a name match, not a
  trace.

## See also

- [zone-shader.md](zone-shader.md) - what the fragment microcode does with
  `zoneTexVis`, and the alpha histogram this page leans on
- [zone-effectsettings-loader.md](zone-effectsettings-loader.md) - the
  twenty-sixth pass, which found the writer and left this question open
- `docs/formats/effectsettings.md` - the `EQ` key cluster and the play report
