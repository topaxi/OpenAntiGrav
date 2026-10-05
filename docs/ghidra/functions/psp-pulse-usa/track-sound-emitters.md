# A circuit's own sound emitters: `sound` `0x3e1`, `soundcone` `0x3e9`, `speaker` `0x3cc`

What a track authors so that a power plant hums where the power plant is. The
three `.vex` audio classes [`vex.md`](../../../formats/vex.md) has listed by
name since 2026-07 and nothing had read; the thing
[`positional-audio.md`](positional-audio.md) recovered the *law* for and had no
authored sources to point it at.

All three are one 80-byte node payload, and it decodes whole. The evidence is
two-sided throughout: the executable says which offset it reads, and the disc's
own 1,298 authored nodes say what is in it.

**They are played now, both classes.** `oag_sound::sfx::TrackEmitters`
opens a held looping voice for each `sound` `0x3e1` node the moment the
listener comes inside its radius and stops it when the listener leaves, off
the law on [`positional-audio.md`](positional-audio.md). A `soundcone` `0x3e9`
node joined it 2026-09-09, once [`VexSoundCone_Init`](#what-a-soundcone-is)
settled which authored angle is its half-angle - see that section for the
find and its own open ends. Two numbers came out of wiring `sound` first, and
neither had been measured: **at most eight emitters are inside their radius at
once** on a real lap of `01_Track` (peak 7) or `14_Track` (peak 8), against a
32-voice pool; and 38 nodes name a cue that resolves and binds no waveform -
see
["A second, larger set"](#a-second-larger-set-resolves-and-still-cannot-be-played-38-nodes-on-eight-circuits).

## The three classes are registered, and one of them is never authored

Each has its own registration function, found by the id it passes to
`Vex_RegisterClass` (`0x08908eb8`) - the one place a class id *is* an immediate,
which is why `vex.md`'s "no `li 0x3bf` anywhere" note does not apply to these:

| Class | Id | Registration | Instances on `pulse-psp-usa` |
| --- | --- | --- | --- |
| `sound` | `0x3e1` | `VexSound_RegisterClass` `0x08925e18` | 1,164 |
| `soundcone` | `0x3e9` | `VexSoundCone_RegisterClass` `0x08926058` | 134 |
| `speaker` | `0x3cc` | `VexSpeaker_RegisterClass` `0x08926324` | **0** |

**`speaker` has a registered class and no instance anywhere on the disc** -
swept across all twelve circuits in both directions and all nine Zone variants.
That is real negative evidence, not an unfinished search: whatever `speaker` is
for, Pulse does not use it, and nothing about it can be recovered from this
disc's data.

Confidence **95** for all three rows. `_li a1, 0x3e1` at `0x08925e38`,
`_li a1, 0x3e9` at `0x08926078` and `_li a1, 0x3cc` at `0x08926340` are the only
occurrences of those three immediates in 525,049 instructions, and each sits in
the delay slot of the call to `Vex_RegisterClass`.

## The payload

`VexSound_Init` (`0x089259a4`) is the reader. It keeps the payload pointer at
`instance+0x60` and every offset below is off that pointer.

| Offset | Size | Meaning | Confidence |
| --- | --- | --- | --- |
| `+0x00` | `f32` | cone angle, radians; `-1.0` on a plain `sound`; never below `+0x04` | 75 |
| `+0x04` | `f32` | second cone angle, radians; `-1.0` on a plain `sound`; `40` degrees on all 134 cones | 75 |
| `+0x08` | `u8` | cone enabled: `0` on every `sound`, `1` on every `soundcone` | 80 |
| `+0x09`..`+0x0b` | 3 B | not read; see [exporter leakage](#three-words-are-exporter-leakage) | 70 |
| `+0x0c` | `f32` | copied to the emitter's `+0x3c`, a field nothing has mapped | 92 |
| `+0x10` | `f32` | **radius**, copied to the emitter's `+0x38` | 95 |
| `+0x14` | 8 B | **bank label**, NUL-terminated, 7 characters usable | 95 |
| `+0x1c` | 16 B | **cue name**, NUL-terminated, `~` included | 95 |
| `+0x2c` | `u16` | radius-curve key count; `1` on all 1,298 authored nodes | 90 |
| `+0x2e`..`+0x2f` | 2 B | not read; exporter leakage | 70 |
| `+0x30` | `u32` | offset to the curve's `u16` **time** array, relocated at load | 92 |
| `+0x34` | `u32` | offset to the curve's `u16` **value** array, relocated at load | 92 |
| `+0x38` | `f32` | seconds per curve tick; `1/60` on all 1,298 | 88 |
| `+0x3c` | `u8` | enables a second, unexercised path; `0` on all 1,298 | 80 |
| `+0x3d`..`+0x3f` | 3 B | not read; exporter leakage | 70 |
| `+0x40` | `u16` | the curve's one time key: `0` on all 1,298 | 90 |
| `+0x42` | `u16` | the curve's one value key: the radius, encoded | 95 |
| `+0x44`..`+0x4f` | 12 B | zero on all 1,298 | - |

`Init`'s four load-bearing lines, in its own order:

```c
emitter[0x38] = node[0x10];                       // radius
emitter[0x3c] = node[0x0c];
Sound_Play(1.0f, emitter, 0, node + 0x14, node + 0x1c, &stack);
```

`Sound_Play` (`0x089392b0`) is already named on
[`positional-audio.md`](positional-audio.md): it queues a `0x30`-byte request on
`emitter+0x58` and bumps `emitter+0x54`. Its third argument is a bank *index* and
its fourth a bank *name*, and passing a name skips the index lookup - which is
what pins `+0x14` and `+0x1c` as two separate strings rather than one buffer.

Before that, `Init` allocates `0x70` bytes, calls `SoundEmitter_Init`
(`0x089391c4`) on it - **construction site `0x08925a84`, one of the nine
`positional-audio.md` lists as unowned** - and points `emitter+0x50` at the
instance's own copy of the node's world matrix (`instance+0x70`, 64 bytes, read
through `0x089451dc`). So an authored emitter is placed by the parent transform
chain, exactly like a [`Speedup Pad`](../../../formats/pads.md), and nothing
about where it is lives in its payload.

## The radius is a curve, and the disc's bytes prove the decode

`VexSound_Update` (`0x08925c4c`) does not use `+0x10` at all. It converts the
instance's elapsed time into curve ticks and resamples:

```c
t = instance[0x40] / node[0x38];          // node[0x38] is 1/60, so t is in frames
VexSound_SampleRadiusCurve(t, instance, &out, (int)t);
emitter[0x38] = out;                       // the radius, every frame
```

**`VexSound_Update` is named on shape, not on its slot** - confidence **85**.
It is in the same translation unit, it is update-shaped, and `Init`'s own
`+0x3c` branch calls it; what has *not* been shown is that it sits in the class
descriptor's `+0x24` update slot, because the descriptor is filled in at boot
and reads zero in the file, and the import's unrelocated constants leave the
function with no static xrefs at all. The `5000`/`65535` encoding below does not
rest on this: it rests on the evaluator's own literals against 1,164 stored
bytes.

`VexSound_SampleRadiusCurve` (`0x08925cf0`) reads `u16` count at `+0x2c`, the
two relocated array pointers at `+0x30`/`+0x34`, walks the time array for the
bracketing pair, lerps the two `u16` values, and finishes with two constants
that are visible in the instruction stream:

```asm
08925d0c  lui   t2, 0x459c        ; 0x459c4000 = 5000.0
08925d10  ori   t2, t2, 0x4000
...
08925dfc  lui   a0, 0x477f        ; 0x477fff00 = 65535.0
08925e00  ori   a0, a0, 0xff00
08925e08  div.s f12, f12, f13
```

So a stored key decodes as `key * 5000.0f / 65535.0f`, in world units, and the
exporter's encoding is `floor(radius * 65535 / 5000)`.

**That prediction is exact on all 1,164 `sound` nodes**, radius `24.8` to
`600.0`, with a maximum round-trip error of `0.0748` units. It is what settles
the field: fitting the ratio to the data alone gives `13.10` and then fails on
`01_Track`'s `~groupcraft` (`107.44 -> 1408`, not `1407`) and on `02_Track`'s
`~GROUPCRAFT` (`150.0 -> 1966`, not `1965`). `65535/5000 = 13.107` reproduces
both, and it came out of the executable rather than out of a curve fit.

Every authored node has **one** key, at time `0`, so the radius is a constant in
practice and equals `+0x10` to within the encoding's `0.0748` units. The
mechanism is animatable; Pulse animates none of it.

Confidence **92** on the curve as a whole: the evaluator is read end to end, and
its two constants predict 1,164 of 1,164 stored bytes from an independent float.

## Relocation is why `+0x30` and `+0x34` look like small integers

`Init` opens with the fixup, guarded on a per-file "already relocated" bit:

```c
if ((file[0x14] & 1) == 0) {
    node[0x34] += (int)node;
    node[0x30] += (int)node;
}
```

On disc the two words read `0x40` and `0x42`, which are not pointers and not
values - they are **offsets into the node's own trailing bytes**, and `+0x40`
and `+0x42` are exactly where the two `u16` arrays sit. A reader that takes them
as data gets `64` and `66` and nothing else; a reader that adds the node base
gets the curve.

## Three words are exporter leakage

`+0x08`'s upper three bytes, `+0x2e`..`+0x2f` and `+0x3d`..`+0x3f` are the only
fields nothing reads, and they are not data:

- They take 27, 8 and 10 distinct values across 1,298 nodes, with no dependence
  on bank, cue, radius, circuit or class.
- **They differ between `track.vex` and `track_reversed.vex`** - two exports of
  one scene - while every semantic field in the same node is byte-identical.
  `01_Track` reads `0x00c2f8`/`0x0808`/`0x0815a7`; `01_Track` reversed reads
  `0x2583ab`/`0x0f9f`/`0x0fa2b3`.
- Read as 4-byte words spanning the constants beside them they land in host
  address ranges: `0x0040xxxx` (a Win32 image base), `0x7c34xxxx`, and
  heap-shaped values like `0x00c2f800`.

Confidence **70**: "an uninitialised host pointer the exporter wrote out" fits
every observation, and no reading of them as a parameter survives the
same-scene-two-exports test. Below the bar for a name, so none of them is named.

## The bank and cue fields resolve against the disc's own banks

`Data\Sound\generaltrack.bnk`'s internal label is `gentrak`
([`psp-audio.md`](../../../formats/psp-audio.md)), and `gentrak` is what 568 of
the `sound` nodes write at `+0x14`. Every one of the thirteen labels the nodes
name is a real `SBlk` bank in `DATA.WAD`, and each circuit-specific one is
shipped twice:

| Label | Circuit | Label | Circuit |
| --- | --- | --- | --- |
| `gentrak` | all twelve, shared | `outpost` | `07_Track` |
| `basilic` | `01_Track` | `amphise` | `09_Track` |
| `metropi` | `02_Track` | `arcprim` | `10_Track` |
| `moather` | `03_Track` | `platinu` | `13_Track` |
| `techder` | `04_Track` | `fortcle` | `14_Track` |
| `dekonst` | `05_Track` | `talonsj` | `16_Track` |
| `vertica` | `06_Track` | | |

The cue field carries the bank's own spelling, `~` and all: `basilic`'s twenty
cues include `~OH_CARGO`, `~ELEVATOR`, `~ELEC_PANEL`, `~NEON_ARCH`,
`~POWER_PLANT`, `~TUN_ELEC_LIGHT`, `~ELEC_LIGHT` and `~ELEC_LIGHT_LAR`, and
`01_Track` authors all eight by exactly those names. `~ELEC_LIGHT_LAR` is the
bank's own truncation at its 16-byte name-table width, reproduced verbatim.

**1,277 of 1,298 authored pairs resolve in the bank they name.** Confidence
**95**; the field boundaries are settled twice over, by the two pointers
`Sound_Play` is handed and by a 98.4% hit rate against an independently parsed
format.

### Five references are dangling on twenty-one nodes, and they are the disc's own bugs

| Reference | Nodes | What is wrong |
| --- | --- | --- |
| `fortcle~blueflashlight` | 14 | no such cue in any bank |
| `basilic~groupcraft` | 2 | no such cue in any bank; `metropi` has `~GROUPCRAFT` |
| `fortcle~RED_NEON_TUN` | 2 | no such cue in any bank |
| `techder~neon` | 2 | `~neon` exists, in `gentrak`, not in `techder` |
| `outpostf~AIR_CON_FAN` | 1 | `~AIR_CON_FAN` exists in `outpost`; the **bank** name is wrong |

The last one is the interesting one. `+0x14` is 8 bytes and this node fills all
eight with `outpostf`, leaving no terminator - so the name the scene held was
longer than the field and the exporter truncated it. A bank's own label field is
8 bytes too and tops out at 7 usable characters
([`sblk`](../../../formats/psp-audio.md)), so no bank can ever be called
`outpostf`, and `07_Track` reversed has an air-conditioning fan that cannot
play. Confidence **80** that it never sounds: the name comparison itself is
inside `Scream_FindSoundInBank` and has not been read, so a lookup that
tolerates a truncated bank name is not excluded.

### A second, larger set resolves and still cannot be played: 38 nodes on eight circuits

The dangling list above was built by checking each `bank~cue` pair against the
bank's **name table**. Playing them found a different failure one level down:
a cue that is in the name table and whose command run binds **no waveform at
all**, because every one of its commands is an opcode
[`oag_formats::sblk`](../../../formats/psp-audio.md) does not decode. Measured
by `crates/game/tests/track_audio_ground_truth.rs`, which loads all twelve
circuits' banks and reports each miss with its reason:

| Circuit | Cue | Nodes | Opcodes its commands run |
| --- | --- | --: | --- |
| `03_Track` | `moather~birds` | 5 | **`0x14`** |
| `04_Track` | `techder~SetReg`, `~SetReg_2` | 2, 2 | `0x1e` |
| `05_Track` | `dekonst~CRANE` | 1 | **`0x14`** |
| `06_Track` | `vertica~SetReg_01`, `~SetReg_02` | 2, 2 | `0x1e` |
| `09_Track` | `amphise~SetReg_01`, `~SetReg_02` | 4, 6 | `0x1e` |
| `13_Track` | `platinu~SetReg`, `~SetReg_2` | 2, 3 | `0x1e` |
| `14_Track` | `fortcle~SETREG_01`, `~SETREG_02` | 2, 4 | `0x1e` |
| `16_Track` | `talonsj~SETREG_01`, `~SETREG_02` | 1, 2 | **`0x14`**, `0x1e` |

**The opcodes are read, and they split the set cleanly in two.** `0x1e` is in
every `~SetReg*` cue and in nothing else; the name reads as "set register" and
a control cue that emits no sample is a coherent explanation for all 29 of
those nodes. `0x14` is in `moather~birds` and `dekonst~CRANE` - both named
after sounds - and, decisively, in `talonsj~SETREG_01`/`_02` **beside** their
`0x1e`, which is a cue that both sets a register and does something else.

**Decoded, 2026-09-08, and it refutes the hypothesis this section used to
make.** `0x14` is a bare no-op -
[`sound.md`](sound.md#opcode-0x14-is-a-no-op-corroborated-on-hd-2026-09-08)
found its handler (`Scream_OpNop14`, confidence 82) two instructions long,
`jr ra; li v0,0`, and corroborated the same shape at the same opcode slot on
`ps3-hdfury-eu`. It does not bind a waveform, does not play another cue, and
ignores its own operand bytes. **The 9 nodes are not a decode gap**: Moa
Therma's five birds, De Konstruct's crane and Talon's Junction's two
`~SETREG` cues genuinely author no sound, on this disc and, by the HD
corroboration, by design rather than a Pulse-specific omission. `talonsj`'s
cues carrying `0x14` beside `0x1e` no longer needs the "not all just
registers" reasoning below to explain why they resolve and stay silent - a
cue can carry two commands that both do nothing.

This section previously read: "So `0x14` is the opcode worth decoding... its
likely shapes are 'bind a waveform another way' or 'play a cue in another
bank'... Confidence 88 that `0x14` binds something audible." Kept here,
struck through in spirit rather than deleted, because the reasoning that
produced it was sound given what was known - the two-opcode split really is
exact across eight circuits, and the `talonsj` cues carrying both really did
rule out "`0x14` is just another register op like `0x1e`". It ruled out the
wrong alternative: not "does nothing", but "does the same nothing as
`0x1e`".

Reproduce the table with
`crates/game/tests/track_audio_ground_truth.rs`'s
`every_race_circuit_names_a_bank_beside_itself_and_its_nodes_spell_its_label`,
which prints every unplayed reference with its reason.

## Census

Zone circuits author **none** of the three classes - all nine `zone_track.vex`
files, zero nodes - which is consistent with Zone loading its own environment
rather than filtering the race one.

| File | `sound` | `soundcone` | File | `sound` | `soundcone` |
| --- | --- | --- | --- | --- | --- |
| `01_track` | 86 | 0 | `07_track` | 43 | 9 |
| `01_track_reversed` | 86 | 0 | `07_track_reversed` | 43 | 32 |
| `02_track` | 30 | 0 | `09_track` | 41 | 2 |
| `02_track_reversed` | 30 | 0 | `09_track_reversed` | 41 | 2 |
| `03_track` | 35 | 2 | `10_track` | 35 | 15 |
| `03_track_reversed` | 35 | 2 | `10_track_reversed` | 35 | 2 |
| `04_track` | 48 | 2 | `13_track` | 35 | 19 |
| `04_track_reversed` | 46 | 2 | `13_track_reversed` | 35 | 19 |
| `05_track` | 67 | 2 | `14_track` | 97 | 9 |
| `05_track_reversed` | 67 | 2 | `14_track_reversed` | 97 | 9 |
| `06_track` | 27 | 0 | `16_track` | 39 | 2 |
| `06_track_reversed` | 27 | 0 | `16_track_reversed` | 39 | 2 |

Reproduce with `just view '<image>:PSP_GAME/USRDIR/DATA.WAD' --nodes
'Data\Environments\01_Track\track.vex' --class 0x3e1 --payload --payload-bytes 0`.

## What a `soundcone` is

A `soundcone` is a **self-contained directional emitter**, not a modifier
attached to a `sound`: same 80-byte payload, its own bank, cue and radii, its own
`Transform` parent. What separates it from a `sound` in the data is exactly four
fields:

| Field | `sound`, 1,164 nodes | `soundcone`, 134 nodes |
| --- | --- | --- |
| `+0x00` | `-1.0`, all | 8 values, all whole degrees: 40, 50, 60, 70, 75, 80, 100, 120 |
| `+0x04` | `-1.0`, all | `40` degrees, **all 134** |
| `+0x08` | `0`, all | `1`, all |
| `+0x0c` vs `+0x10` | equal, all | `+0x0c` is `25.0` on all 134; `+0x10` varies |
| `+0x42` | `floor(radius * 13.107)` | `0`, all |

Two angles in radians that are whole degrees on every node is not a coincidence,
and the emitter has a cone half-angle at `+0x40` and a cone-enabled `u8` at
`+0x4c` that `SoundEmitter_Init` defaults to `pi/2` and `0`.

### `VexSoundCone_Init`, 2026-09-09: which angle reaches `+0x40` is read

**`0x08925ff4`** (`psp-pulse-eu`: `0x08925ad0`, byte-identical decompile).
Confidence **90**. Found by decompiling `VexSound_Init` in full rather than the
three-line excerpt this page previously quoted - its own tail (below) calls
`VexSound_Update` behind a gate this page had already named but not traced -
and then searching `get_xrefs_to` on `VexSound_Init` itself, which this
import's usual unrelocated-constants xref-blindness (see
[`positional-audio.md`](positional-audio.md#read-this-before-trusting-an-address-on-this-page))
turned out **not** to block here: `VexSound_Init` has exactly one caller, and
this is it.

```c
undefined4 VexSoundCone_Init(int instance)
{
    int emitter;
    if (VexSound_Init(instance) != 0) {
        emitter = *(int *)(instance + 100);           /* instance+0x64 */
        *(undefined4 *)(emitter + 0x40) = **(undefined4 **)(instance + 0x60);
        *(undefined4 *)(emitter + 0x44) =  *(undefined4 *)(*(int *)(instance + 0x60) + 4);
        *(undefined1 *)(emitter + 0x4c) =  *(undefined1 *)(*(int *)(instance + 0x60) + 8);
    }
    return 1;
}
```

Four lines: call `VexSound_Init` unchanged - which already does everything a
plain `sound` does, including the radius copy and `Sound_Play` - then patch
three emitter fields off the same payload pointer (`instance+0x60`) `VexSound_Init`
itself keeps:

- **`+0x00` (the varying angle) to the emitter's `+0x40` half-angle.** This is
  the answer: `Cone::wide()` in `oag_vex::sound_emitters`, never smaller
  than `+0x04` on any of the 134 nodes, is what
  `SoundEmitter_ComputeVolumeAndAngle`'s `atten *= 1 - e[0x48]/e[0x40]` divides
  by - not a guess between the two, a decompiled assignment.
- **`+0x04` (the constant `40` degrees) to the emitter's `+0x44`** - a field
  nothing in this project's reading of `SoundEmitter_Update` or
  `SoundEmitter_ComputeVolumeAndAngle` reads back. Same standing as `+0x3c`
  below: decoded and named, not consumed by anything found so far.
- **`+0x08` (the enable byte) to the emitter's `+0x4c`**, already known from the
  disc-wide split; this is the write site for it.

`instance+100` (`0x64`) is the emitter pointer `VexSound_Init` itself stores -
a field this page had not previously named, alongside the already-known
`instance+0x60` payload pointer.

**Cross-checked two ways.** Decompiled identically on `psp-pulse-usa` and
`psp-pulse-eu` (the two functions differ only in their internal call
addresses, which is expected of two separate compilations), and the address on
each disc is the sole static reference to the disc's own `VexSound_Init`
(`search_byte_patterns` for the address as a raw pointer finds it once, inside
the disc's own class-descriptor table at `0x08ad2040`/EU `0x08acfc78` - the
table entry sitting in the same slot `sound`'s own table holds `VexSound_Init`
in, at `0x08ad1fb8`/EU equivalent).

### Why `+0x42` reads `0` on every cone: `VexSound_Update` never runs on this disc

`VexSound_Init` and `VexSoundCone_Init`'s own tail (the lines this page had not
previously quoted) both end the same way:

```c
if (payload[0x3c] != 0) {                    /* already documented as "0 on all 1,298" */
    /* ... computes a loop-end-derived duration into instance+0xb0 ... */
    VexSound_Update(instance);
}
```

and **`VexSound_Tick`** (`0x08925bec`, EU `0x089256c8`, confidence 85 -
undocumented until now, found from `Vex_UpdateNodeWorldMatrix`'s own
`get_xrefs_to` list rather than from either `Init` - it is the only caller in
this address range and the shape matches a per-node tick: advance elapsed
time, conditionally resample, always re-point the emitter at the node's
current world matrix) repeats the identical gate before its own call to
`VexSound_Update`:

```c
if (payload[0x3c] != 0) {
    instance[0x40] = wrap_time(instance[0x40] + dt, instance[0xb0]);
    VexSound_Update(instance);
}
Vex_UpdateNodeWorldMatrix(instance);
```

**`get_xrefs_to` on `VexSound_Update` returns exactly these two call sites on
both `psp-pulse-usa` and `psp-pulse-eu`, and both are gated on the same
byte.** `+0x3c` is `0` on all 1,298 authored nodes (already established), so
**`VexSound_Update`'s curve resample never executes for anything Pulse
ships** - not for a `sound`, and not for a `soundcone`. That is what settles
this page's other open question: a cone's `+0x42` (the curve's one value key)
is free to read `0` because nothing ever samples it. The `+0x10` scalar
`Init` copies into the emitter's `+0x38` at construction is therefore the one
radius any authored node ever actually plays, cone or otherwise - not merely
the "shortcut" this page previously called it beside `sample_radius`.

Confidence **85**: two call sites found on each of two independent
compilations, both gated the same way, and the gate byte's value is a
disc-wide, independently-verified fact rather than an assumption. Short of
100 because this is two call sites read by hand, not an exhaustive
disassembly-wide search for every possible tail-call or table-dispatched
entry into `VexSound_Update` - the same caveat this page's own
`VexSound_Update` row already carried before this pass, now narrowed rather
than removed.

## Confidence summary

| Claim | Score | Basis |
| --- | --- | --- |
| The three registration functions and their ids | 95 | only occurrences of three immediates, each in the call's delay slot |
| `+0x10` is the radius, `+0x14`/`+0x1c` the bank and cue | 95 | `Init` reads them; 1,277 of 1,298 pairs resolve against independently parsed banks |
| `+0x42` encodes the radius as `floor(r * 65535 / 5000)` | 95 | both constants read out of the evaluator; exact on 1,164 of 1,164 |
| The radius is a one-key animation curve | 92 | evaluator read end to end |
| `+0x30`/`+0x34` are relocated offsets, not values | 92 | the fixup is the first thing `Init` does |
| `+0x0c` reaches the emitter's unmapped `+0x3c` | 92 | one assignment, read directly |
| `VexSoundCone_Init` and which angle reaches `+0x40` | 90 | full decompile, sole caller of `VexSound_Init`, corroborated on two discs |
| `soundcone` is a directional emitter, `+0x08` its flag | 80 | disc-wide split, 134 against 1,164, no counterexample |
| `outpostf~AIR_CON_FAN` never plays | 80 | field width and the bank format agree; the comparison itself is unread |
| `VexSound_Update` never executes for any authored node | 85 | two call sites on two discs, both gated on the same always-`0` byte; not an exhaustive whole-binary search |
| `VexSound_Tick` is the per-node per-frame tick | 85 | full decompile, one of `VexSound_Update`'s two call sites, only caller of `Vex_UpdateNodeWorldMatrix` in this address range |
| Pure's `woSound` is class `0x393` | 88 | payload bytes read the same shape as Pulse's `sound` and name real Pure banks/cues, on 129 nodes across three circuits - no static reading, so capped below a Ghidra-sourced finding |
| Three words are exporter leakage | 70 | vary between two exports of one scene, no semantic dependence |
| `speaker` is unused by Pulse | 95 | zero instances in 33 `.vex` files |

## What this does not answer

- ~~**Which undecoded opcode binds a waveform**~~ **Answered: none does.**
  `0x14` decoded to a no-op, not a waveform bind - see
  ["A second, larger set"](#a-second-larger-set-resolves-and-still-cannot-be-played-38-nodes-on-eight-circuits)
  above. The 38 nodes stay silent, but the reason is now that their command
  lists genuinely author no sound, not that a handler is missing.
- **`emitter+0x3c` has no meaning.** It is written from `+0x0c` and
  `positional-audio.md`'s three-way-pinned emitter table does not list it, so
  nothing observed so far reads it back.
- ~~**`soundcone`'s init and update are not found**, which is what leaves the two
  angles at 75 and `+0x42` unexplained on cones.~~ **Answered, 2026-09-09.**
  `VexSoundCone_Init` is found and read - see its own section above - which
  settles which angle reaches `+0x40` (confidence 90) and, as a consequence of
  tracing `VexSound_Update`'s own call sites to answer it, why `+0x42` reads
  `0` on every cone (confidence 85). `angle_b`'s own destination
  (emitter `+0x44`) is decoded but nothing has been shown to read it back -
  the same open standing `emitter+0x3c` already has, immediately below.
- ~~**The PS2 and Pure equivalents are unchecked.**~~ **Answered, 2026-09-08, for
  both, by different methods.**

  **PS2 (`pulse-ps2-eu`) shares the class IDs and the exact authored counts.**
  `just view 'data/images/pulse-ps2-eu.chd:54748/WADS2.WAD' --nodes
  'Data\Environments\01_Track\track.vex' --class 0x3e1` (and `0x3e9`, `0x3cc`)
  reads 86 `sound`/0 `soundcone`/0 `speaker` on `01_Track` and 43/9/0 on
  `07_Track` - matching this page's [Census](#census) table on `pulse-psp-usa`
  exactly, node for node. `speaker` stays at zero on the second pressing too,
  which is a second data point for its "unused by Pulse" row above. This is a
  direct ID sweep, valid because Pulse's PS2 and PSP pressings are the same
  title from the same exporter and do not renumber classes against each other
  the way Pure does.

  **Pure (`pure-psp-usa`) authors the same thing, under a class ID this pass
  did not identify.** A direct sweep at Pulse's literal IDs reads zero on
  Pure's `Data\Environments\01_Vineta_K\track.vex`, as expected -
  [`pure-status.md`](../../../formats/pure-status.md#the-class-id-space-is-renumbered)
  already established Pure renumbers the whole `.vex` class space. An ID-blind,
  size-blind check settles it instead: every node payload here carries a
  bank-label/cue-name pair as ASCII text (`+0x14`/`+0x1c`), and Pure ships the
  same `SBlk` container with the same per-circuit-bank convention this page's
  [bank table](#the-bank-and-cue-fields-resolve-against-the-discs-own-banks)
  documents (`just wad sounds <pure>:.../Data.wad` lists `VINETTA`, `SEBCLIM`,
  `CHENGOU`, `GENTRAK`, one per circuit plus the shared general bank, the same
  shape as Pulse's `basilic`/`gentrak`/etc). Extracting a Pure `track.vex`
  whole (`just wad cat`) and running `strings` over the raw bytes finds
  exactly those bank names paired with `~`-prefixed cue names identical to the
  bank's own cue table (`~TRACKLIGHT`, `~BILLBOARDHUM`, `~SEAGULL_CLD`,
  `~STARTLINE`, ...), each one sitting inside a node named `woSound1`,
  `woSound2`, ... in the scene - 48 on `01_Vineta_K`, 56 on
  `10_Sebenco_Climb`, 106 on `04_Chenghou_Project` (three circuits checked).
  The surrounding bytes carry the same shape this page's payload table
  describes: two equal `f32`s where `sound`'s `+0x0c`/`+0x10` sit, and
  `0x3c888889` (`1/60`) where the curve's seconds-per-tick field sits. No
  `woSoundCone`- or `woSpeaker`-named sibling turned up in the same scan, so
  whether Pure distinguishes a cone the same way is unread.

  **So the follow-on this page's Next Steps used to name - "find where Pure's
  circuit banks live, because it ships no `trackstartup.xml`" - turns out not
  to be the hard part.** The banks are trivially named, the same way Pulse's
  are (`oag-wad sounds`, one per circuit).

  **Pure's class ID for `woSound` is found, 2026-09-09, without the Ghidra
  bridge.** A `woSoundN` name belongs to a `Transform` (Pure's renumbered
  `0x6d`), not to the sound node itself - its one child, named
  `woSoundNShape`, is the payload, and every one sampled is class **`0x393`**.
  Found by reading `oag_vex::vex::Node`'s own class ID and name off
  `just view '<pure image>:.../Data.wad' --nodes '<track.vex>'`, not by the
  table-index technique this page used to name as the way in - the node
  already carries both facts, so no exporter class-name table was needed.
  129 nodes across the three circuits, all `0x393`, all still carrying the
  same two-equal-`f32`s shape noted above - **no cone-split signature on any
  of them**, corroborating the no-`woSoundCone`-name finding on the payload's
  own bytes rather than repeating it. See
  [`pure-status.md`](../../../formats/pure-status.md#a-circuits-own-ambience-wosound-is-class-0x393-found-without-ghidra)
  for the full count and the field-layout caveat (Pure's bank string sits at
  a different payload offset than Pulse's, and that was not re-derived here).
