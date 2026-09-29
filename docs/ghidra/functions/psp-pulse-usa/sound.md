# The sound engine: cue dispatch and the SAS voice table

## The question this page answers

`docs/formats/psp-audio.md` calls per-sound boundaries inside a `.bnk` *"the gap
that matters, because it is what stands between here and extracting individual
sounds"*. Everything about race audio is blocked behind it.

This page attacks it from the code side rather than the data side, and gets most
of the way. It **confirms the engine is Sony's SCREAM** from the binary's own
copyright string rather than by inference, **decodes the bank's name table** so
that per-sound names are extractable, and **names the two fields** a sound's
offset and length are finally written to. It also reaches the cue record from
the runtime side and finds it agrees, field for field, with what the format page
measured from the data side. What remains is the middle - the command-list
interpreter that turns a cue into a waveform offset - and that is one named
function away rather than an open-ended search.

**Target note.** `psp-pulse-eu` is this project's preferred first target for new
investigation, and this page is `psp-pulse-usa`. That is deliberate: the cue
dispatch below was already recovered at USA addresses while
[pads](pads.md), [contact response](contact-response.md) and [exhaust](exhaust.md)
were written, and splitting one thread across two binaries costs more than it
buys. Cross-verification against EU is not done and is listed under "Not
determined".

## Two ends of a rope that do not yet meet

**The near end** is a cue by name. `Sound_Play` (`0x089392b0`, 37 call sites)
takes a cue string and queues a request; the sites are in
[pads](pads.md#the-sound-and-a-mode-constant-that-is-not-zones) (`"SPEEDUPPAD"`),
[contact response](contact-response.md) (`"COLLISIONS"`, `"ABSORB"`) and
[exhaust](exhaust.md#exhaust_updateenginesound) (`"~ENGINE"`).

**The far end** is `sceSasSetVoice`, where a waveform's address and length are
handed to the PSP's hardware synth. This page walks the far end back to where
those two numbers live in memory, and walks the near end forward to the cue
record. They stop one function apart.

## The far end, walked back

```text
__sceSasSetVoice          0x08a76c5c   import stub
  <- Sas_SetVoice         0x08a2aebc   thin guarded wrapper, 1 call site
    <- Sas_CommitVoices   0x0898c7e8   the per-frame commit loop
```

`Sas_SetVoice` is four lines and adds nothing but a guard:

```c
undefined4 Sas_SetVoice(voice, addr, size, loop) {
  if (DAT_08b2d300 == 0) return 0x80420100;      /* SAS not initialised */
  return __sceSasSetVoice(&DAT_08b2d340, voice, addr, size, loop);
}
```

`&DAT_08b2d340` is the SAS core handle. Sony's signature is
`sceSasSetVoice(core, voice, vagAddr, size, loopmode)`, so the wrapper's four
arguments are the voice index and then **address, size, loop mode** in that
order. Confidence **95**: the call is direct and the argument order follows from
a published prototype.

## The voice table

`Sas_CommitVoices` (`0x0898c7e8`) is a **32-iteration loop over 0x6c-byte
records** based at `0x08b893d0` - `g_sas_voices` - and 32 is exactly
`sceSasCore`'s voice count.
Each record carries a **dirty-flag word at `+0x3c`** whose bits select which
hardware call to make, and the loop clears the word once it has committed:

| Bit | Call made | Record fields passed |
| --- | --- | --- |
| `0x0f` | `Sas_SetVolume(voice, ...)` (`0x08a2ae08`) | `+0x40`, `+0x44`, `+0x48`, `+0x4c` |
| `0x10` | `Sas_SetPitch(voice, ...)` (`0x08a2ae6c`) | `+0x50`, defaulting to `1` when it is zero |
| **`0x20`** | **`Sas_SetVoice(voice, ...)`** (`0x08a2aebc`) | **`+0x54`, `+0x58`, `+0x5c`** |
| `0x40` | `Sas_SetNoise(voice, ...)` (`0x08a2af18`) | `+0x60` |
| `0x80` | `FUN_08a2b088` or `FUN_089961e4` | `+0x64`, `+0x68` |

So:

```text
voice + 0x54   waveform address
voice + 0x58   waveform length in bytes
voice + 0x5c   loop mode
```

**That is the field the `.bnk` question has to resolve to.** Whatever decodes a
bank's per-sound directory ultimately writes a byte offset into `+0x54` and a
byte count into `+0x58`. Confidence **88** on the three field meanings, which
follows from the wrapper above; the table's base address is now confidence
**92** rather than the 85 this was first read at - `memory.disasm` against a
running `pulse-psp-usa` (2026-09-06, see the corroboration section below)
shows `Sas_CommitVoices` loading it as a plain immediate, `li s1,0x08B893D0`,
matching the address this page already used exactly, rather than only
the loop's stride/bound/increment this was inferred from originally.

The four bits whose calls are *not* `sceSasSetVoice` were originally left
unnamed on an argument-count guess alone - four for ADSR, one for pitch, two
for a keyed volume pair - and that guess turned out backwards.

### Correction, 2026-09-06: the guess was backwards, and `+0x40` is the per-voice volume

Read at the call site rather than guessed at: `FUN_08a2ae08` has no function
object in the Ghidra project (Ghidra reports "no function found" for it), so
this is read from raw disassembly rather than a decompile - `jal 0x00272c24`
at `0x08a2ae50`, which is `0x08804000 + 0x00272c24 = 0x08a76c24` -
**`__sceSasSetVolume`**, already named in the Ghidra project from its one
other call site
([`audio-levels.md`](audio-levels.md)'s `Sas_Init`, which the same page's
"not recovered" section believed was the *only* call). It is not: this is a
second, and it is per-voice, per-frame. Its four fields
(`+0x40`, `+0x44`, `+0x48`, `+0x4c`) land exactly on `sceSasSetVolume`'s real
signature past `(core, voice)` - `volumeL, volumeR, volumeEffectL,
volumeEffectR` - four fields, not an ADSR curve's four fields as guessed.

The other two call targets resolve the same way, against Ghidra's own already-named import stubs:

| Bit | Wrapper | Resolves to | Address |
| --- | --- | --- | --- |
| `0x0f` | `Sas_SetVolume` (`0x08a2ae08`) | `__sceSasSetVolume` | `0x08a76c24` |
| `0x10` | `Sas_SetPitch` (`0x08a2ae6c`) | `__sceSasSetPitch` | `0x08a76c7c` |
| `0x40` | `Sas_SetNoise` (`0x08a2af18`) | `__sceSasSetNoise` | `0x08a76c84` |

`0x10`'s guess ("pitch") was right; `0x40` was left unguessed and is a noise-generator
enable, not a volume. Bit `0x80`'s two fields are not a "keyed volume pair"
either: `FUN_089961e4` decodes `+0x64`/`+0x68`'s bitfields into curve-mode and
rate arguments and calls `func_0x00226f64` (`0x08a2af64`) with mask `0xf` -
Ghidra does not resolve that inner call to a function, so what it ultimately
reaches is not confirmed by name. What corroborates the ADSR reading instead:
`Sas_SetSL` (`0x08a2b03c`), itself called from inside `FUN_089961e4`'s own
body (not a separate dirty-bit handler), calls `func_0x00272c34` =
`0x08a76c34` = **`__sceSasSetSL`** (sustain level) the same guarded way
`Sas_SetVoice` calls `__sceSasSetVoice`. Bit `0x80` is ADSR rate/curve configuration, the thing
bit `0x0f` was wrongly guessed to be.

Confidence **90** on `0x0f`/`0x10`/`0x40`'s three resolved call targets: each
is a direct `jal` through the same `(0x53ba8, voice, ...)` guarded dispatch
already read for `Sas_SetVoice`, landing on an import stub Ghidra has
independently named from its own symbol table - not an inference from
argument count. Confidence **80** on `0x80`'s ADSR reading: `__sceSasSetSL`
is confirmed the same way, but it is reached one call deeper
(`FUN_089961e4` -> `Sas_SetSL` -> `__sceSasSetSL`) rather than directly
from the dirty-bit dispatch, and the unresolved `func_0x00226f64` call
alongside it is not independently confirmed. All three wrappers are named in
the section below; the fourth, `FUN_089961e4`, is not, and that section says
why.

### The naming pass, 2026-09-07: the three remaining wrappers take their imports' names

The 2026-09-06 correction above resolved five call targets and named exactly
one of them (`Sas_SetVolume`), leaving three resolved-but-unnamed wrappers
behind because the convention for this shape - a thin guarded call to an
import Ghidra has already named from its own symbol table - had not been
settled. It had, in fact, by the two names that already existed:
`Sas_SetVoice` wraps `__sceSasSetVoice` and `Sas_SetVolume` wraps
`__sceSasSetVolume`, both **mirroring the import's own verb-noun with the
`__sceSas` prefix replaced by this project's `Sas_` subsystem prefix**. That
is the convention, applied here to the remaining three:

| Address | Name | Wraps | Confidence |
| --- | --- | --- | --- |
| `0x08a2ae6c` | `Sas_SetPitch` | `__sceSasSetPitch` (`0x08a76c7c`) | 90 |
| `0x08a2af18` | `Sas_SetNoise` | `__sceSasSetNoise` (`0x08a76c84`) | 90 |
| `0x08a2b03c` | `Sas_SetSL` | `__sceSasSetSL` (`0x08a76c34`) | 90 |

`Sas_SetSL` keeps the import's own abbreviation rather than expanding to
`Sas_SetSustainLevel`: mirroring is the whole content of the convention, and a
wrapper whose name differs from the import it guards is the one thing that
would make the pair hard to find from either end. It is a judgement call and
is recorded as one.

**Why all three are 90 even though bit `0x80`'s ADSR reading is 80.** A name
here claims what the function *calls*, not what its caller does with the
result. Each of the three is a direct `jal` onto an import stub Ghidra named
independently from its own symbol table, which is what the 90 above is scored
on. The 80 belongs to the *interpretation* of bit `0x80` as ADSR
rate/curve configuration - an inference from `__sceSasSetSL` being one of the
calls reached, which the name `Sas_SetSL` does not restate. For the same
reason the name is not weakened by `Sas_SetSL` being reached from inside
`FUN_089961e4`'s body rather than from the dirty-bit dispatch directly: a name
describes the function, not its call path.

**`FUN_089961e4` is deliberately left unnamed**, though the 2026-09-06
correction listed it alongside the other three. It is not a thin wrapper: it
decodes `+0x64`/`+0x68`'s bitfields into curve-mode and rate arguments and
calls `func_0x00226f64` (`0x08a2af64`), which Ghidra does not resolve to a
function at all. Any name for it would have to encode the confidence-80 ADSR
interpretation rather than a resolved import, and this project's own rule
(names carry their confidence; below 70 they do not happen at all) is better
served by leaving `FUN_089961e4` as it is than by a `_q` name that reads as a
conclusion. Resolving `func_0x00226f64` is what would change that.

No Ghidra project was open for this pass and none is needed: the three
`names.tsv` rows land regardless, per this project's convention, exactly as
`Sas_SetVolume`'s row did while Ghidra's own function boundary at its address
was (and still is) broken. `just check-names` verifies the rows against this
page offline.

**What this settles for [audio-levels.md](audio-levels.md#what-is-not-recovered-and-why-it-is-the-next-thing-to-read):**
the per-voice SAS volume is written every frame through `+0x40`/`+0x44` (dry
L/R) and `+0x48`/`+0x4c` (effect-send L/R) of the voice record, via a second,
previously-missed call to `__sceSasSetVolume`. **What is still not settled**
is what numeric value lands there: nothing above traces `+0x40` back to a
writer. `SoundInstance_UpdateSpatial` (`0x08939e58`,
[positional-audio.md](positional-audio.md)) is the only known producer of a
comparable pair - it calls `Scream_SetSoundVolume` (`0x0898d614`, named
2026-09-07, see [positional-audio.md](positional-audio.md#scream_panvolumepairs-four-terms))
with the
`SoundEmitter_ComputeVolumeAndAngle` volume - but that function is SCREAM's
own software fade/ramp engine over a *SCREAM instance*, not the SAS voice
record `Sas_CommitVoices` walks, and nothing read here connects the two.
That link, not this one, is the next thing to read before "does the original
reserve headroom per voice" has an answer instead of a guess.

The `0x80` branch choosing between `FUN_08a2b088` and `FUN_089961e4` on
`DAT_08ac3224` is the same flag that gates `FUN_089960e0` at the top of the
loop, so there are two output paths and one of them is selected globally. Not
investigated.

### Corroboration, 2026-09-06: `Sas_SetVolume` confirmed live, and the value it carries measured

This correction's own reading of `FUN_08a2ae08` was raw disassembly against
Ghidra's static database, with no function object to decompile. A running
`pulse-psp-usa` (PPSSPP v1.20.4, see
[`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md))
settles it independently, twice over, across two separate boots:

- `memory.disasm` at `0x08a2ae08`, live, shows the same code this page
  already read - ending in `jal zz___sceSasSetVolume` at `0x08a2ae50`,
  PPSSPP's own debugger naming the import stub itself, no manual image-base
  arithmetic needed - guarded by a `DAT_08b2d300`-style "is SAS initialised"
  check, the same shape `Sas_SetVoice` uses. Ghidra's own
  `get_function_by_address` still resolves `0x08a2ae08` to the *interior* of
  the unrelated neighbour `FUN_08a2ade0` (a generic table-dispatch helper
  with nothing to do with SAS) - a Ghidra function-boundary defect around
  this address, not a wrong reading of what the disc actually runs.
- `memory.disasm` at `Sas_CommitVoices`'s own entry (`0x0898c7e8`) shows the
  call site directly: `jal z_un_08a2ae08` at `0x0898c868`, with `s2` (the
  loop's per-voice record pointer) loaded four instructions earlier from
  `li s1,0x08B893D0` - the table base this page already names
  `g_sas_voices`, as a plain 32-bit immediate rather than anything needing
  an image-base correction. That resolves a mismatch the first version of
  this corroboration flagged (the decompiled loop showed the base as the
  bare literal `0xafc38`, which is neither this address nor that address
  under the usual `+0x08804000` convention): the live disassembly shows the
  true immediate directly, and `0xafc38` was the decompiler's own artifact
  on this literal, not evidence of a different real base.

**This wrapper is named here: `Sas_SetVolume`, confidence 92.** The same
guarded-call shape `Sas_SetVoice` already carries for `__sceSasSetVoice`
(confidence 95), now confirmed for `__sceSasSetVolume` the same way - a
direct `jal` to an already-Ghidra-named import stub, read live from the
executing binary rather than inferred - one point short of `Sas_SetVoice`'s
because this reading additionally depends on `Sas_CommitVoices`'s own
disassembly for the call site (`Sas_SetVoice`'s single call site needed no
second read). Not applied to the Ghidra project itself: its function
boundary is broken at this address (above), so a rename would first need a
`create_function` split off `FUN_08a2ade0`, which risks the shared
project's other work more than a docs-only pass should. The row is in
`names.tsv` regardless, per project convention - the database can catch up
whenever someone next has the project open for a reason that already
touches this region.

**The same live session captured the four fields this wrapper reads
(`+0x40`/`+0x44`/`+0x48`/`+0x4c`) during an actual race**, filtered to the
voice/tick pairs where the record's dirty flag genuinely fired (a stale,
never-committing voice slot can hold a leftover nonzero value indefinitely
and is not evidence of anything - three captures each carry at least one).
See
[audio-levels.md](audio-levels.md#the-per-voice-sas-volume-measured-live-2026-09-06)
for the value, its variability across voices, and whether it explains the
race mix's saturation (it does not, and that page says why).

## The record is also a free-list node

`SoundVoice_Unlink` (`0x08995224`) unlinks a record from a singly-linked
list headed at `DAT_08ac36d4`, walking it by `*record`. So **`+0x00` is a next
pointer** and
the 32 records double as the allocator's free list. It also zeroes `+0x08` and
`+0x0c` on release (written in the decompiler as `(&DAT_08b893d8)[i * 0x1b]`,
which is the same array at a `u32` stride). Five callers, all in the
`0x08994xxx` voice-manager cluster. Confidence **65**, so the name carries `_q`.

## This is Sony's SCREAM, and that is now a finding

`docs/formats/psp-audio.md` recorded *"whether this is Sony's SCREAM engine"* as
**a strong inference rather than a finding**, because `SBlk` is SCREAM's bank
magic and the PS2 disc ships `IOP/SCREAM.IRX` but the PSP build has no
equivalent file to point at.

The binary says so itself. Nineteen `SCREAM` strings, including a copyright
line:

```
0x08aa4238   " SCREAM PSP    (c)2006 Sony Computer Entertainment America\n"
0x08aa3d90   "SCREAM: ERROR! You must call snd_RegisterMainMemAllocator ..."
0x08aa3e00   "SCREAM: Couldn't find named bank -> %s\n"
0x08aa3e28   "SCREAM: Didn't find sound named -> %s\n"
0x08aa3ca8   "SCREAM: Didn't find child sound named -> %s\n"
0x08aa3a30   "SCREAM ERROR: THIS SYSTEM ONLY SUPPORTS ADPCM VOICE DATA!\n"
```

Confidence **99**. A verbatim vendor copyright string is as direct as this kind
of evidence gets, and `snd_RegisterMainMemAllocator` is a published SCREAM API
name. The practical consequence is larger than the label: SCREAM is a documented
middleware, so its structure names are worth looking up rather than deriving.

The ADPCM line is also a small independent confirmation of
[psp-audio.md](../../../formats/psp-audio.md)'s codec finding - this build
refuses anything that is not ADPCM.

## The name table, decoded

`Scream_FindSoundInBank` (`0x08992304`) is the lookup those two error strings
guard, and it reads the structure `psp-audio.md` describes but could not
interpret. Signature: `(bank, name, out_bank) -> sound_index`, `-1` on miss.

Two gates first:

```c
if (*bank != 0x6b6c4253 || (bank[2] & 0x100) == 0) return -1;
```

`0x6b6c4253` is `"SBlk"` little-endian. `bank[2]` is the word at **`+0x08`**,
which `psp-audio.md` records as *"772/260, not determined"* - and 772 is `0x304`
while 260 is `0x104`, so **both have bit `0x100` set**. That bit means *this bank
carries a name table*; it is a capability flag, not a size.

Then, with `names = bank[0xe]`, the **`+0x38` name-block offset** already
documented:

```c
entries = *(u32 *)(names + 0x08);                  /* the entry array */
bucket  = *(u16 *)(names + 0x18 + hash(name) * 2); /* head index */
for (e = entries + bucket * 0x14; *e != 0; e += 0x14)
    if (memcmp(e, name, 0x10) == 0) return *(i16 *)(e + 0x10);
```

So the name block is:

```text
names + 0x00   char[8]   the bank's own name, already documented
names + 0x08   u32       offset of the entry array
names + 0x18   u16[]     hash buckets, indexed by hash(name)
```

and each entry is **0x14 bytes**:

```text
entry + 0x00   char[16]  the sound's name, compared with a 16-byte memcmp
entry + 0x10   u16       the sound's index within the bank
```

Confidence **85**. Every stride and offset is a literal in the decompilation and
the two gates are unambiguous; it has not been run against a real bank yet,
which is the one step between this and 90.

**The practical consequence is that per-sound names are extractable now.** The
hash in `FUN_089924ec` is only needed to *jump* to a bucket - walking the entry
array linearly to its terminator yields every `{name, index}` pair without
reverse-engineering the hash at all. That turns `psp-audio.md`'s *"where each
sound starts"* from one problem into two smaller ones: the names are readable,
and what remains is the index-to-waveform-offset step.

`Scream_PlaySoundByName` (`0x08991b20`) is the caller and shows the whole shape:
resolve the bank by name if one was not passed (`Scream_FindBankByName`,
`0x08992264`), resolve the
sound to an index here, then hand `(bank, index, ...)` to `FUN_08991c70`. **That
last call is where an index becomes a waveform offset**, and it is the function
to read next.

The `param_1 == NULL` branch walks a bank list by `+0x30`, recursing, so a cue
name with no bank named resolves against every loaded bank in turn.

## A cue is a sound, and the runtime agrees with the format page

`Scream_StartSound` (`0x0898f864`) is what the index finally reaches, through
two thin dispatchers. Its first three lines settle what a "cue" is:

```c
if (index < 0 || index >= *(i16 *)(bank + 0x16)) return 0;
cue = *(u32 *)(bank + 0x1c) + index * 0xc;
```

**That is exactly the cue region [psp-audio.md](../../../formats/psp-audio.md)
already measured** - offset field at `+0x1c`, stride 12, one per playable sound -
reached here from the opposite direction, and it bounds the index against
`+0x16`, which that page already documents as `cue_count`. The two readings were
derived independently - one from strides across all 39 banks, one from the
runtime's own bounds check - and they agree on every figure. Corroboration
rather than new ground, which is worth having on a page whose central claim is
still open.

So **the name table's "sound index" is a cue index**, and the chain from a cue
string to a playing sound is now unbroken:

```text
"SPEEDUPPAD"  ->  Scream_FindSoundInBank   name table       -> cue index
              ->  Scream_StartSound        bank+0x1c + i*12 -> cue record
              ->  ...                                       -> waveform offset
              ->  Sas_QueueSetVoice        voices[v].addr/size
              ->  Sas_CommitVoices         sceSasSetVoice
```

The cue record's other fields, which the format page lists only as "the third
word indexes the command table":

| Offset | Read as | Use |
| --- | --- | --- |
| `+0x00` | `s8` | default for the caller's first `-1` argument |
| `+0x01` | `s8` | copied into the handler at `+0x17` |
| `+0x02` | `s16` | default for the caller's second `-1` argument |
| `+0x04` | `u8` | **must be non-zero or the cue does not play at all** |
| `+0x06` | `u16` | flags: `0x4000` is set on every play, `0x2` triggers a preload |
| `+0x08` | `u32` | the command-table index, dereferenced **as a pointer** at runtime |

`+0x08` is the interesting one. The format page reads it as an index into the
command table; the runtime dereferences it (`*(u32 *)(cue + 8) + 4`), so SCREAM
fixes it up from an index to a pointer at load time. Both readings are right
about different moments.

Confidence **85** for the cue-record fields, from direct reads in one function.
`+0x16` and the `+0x1c` stride were already at 94 from the data side; the
runtime agreeing does not raise a number, it removes a way of being wrong.

## The waveform descriptor, and why the format page already had it

`Scream_KeyOnVoice` (`0x0899456c`) is the last link. It reads a pointer from
`g_sas_voices[v] + 0x14`, and every argument it hands to `Sas_QueueSetVoice`
comes out of what that pointer addresses:

```c
wf = *(u32 *)(&g_sas_voices[v] + 0x14);
Sas_QueueSetVoice(v, *(u32 *)(wf + 0x10),      /* address */
                     *(u32 *)(wf + 0x14),      /* size    */
                     (*(u16 *)(wf + 0x0e) & 0x40) != 0,
                     (*(u16 *)(wf + 0x0e) & 0x80) != 0);
```

So a **waveform descriptor** carries, at least:

```text
wf + 0x00   s8    read by FUN_0898f5e8; a priority, hypothesised (see the pitch section)
wf + 0x01   s8    volume, 60..127 on every PSP descriptor
wf + 0x02   s8    centre note   - the pitch, decoded 2026-09-16, see below
wf + 0x03   s8    centre fine   - 1/128ths of a semitone, 0x7f in tune
wf + 0x04   s16   pan, degrees, reduced modulo 360
wf + 0x08   s8    pitch-bend range down, semitones (Scream_ComputeVoiceNote)
wf + 0x09   s8    pitch-bend range up
wf + 0x0a   u16   paired with +0x0c into an envelope call
wf + 0x0c   u16
wf + 0x0e   u16   flags; 0x40 selects loop mode, 0x80 marks NOT-ADPCM
wf + 0x10   u32   the waveform's offset
wf + 0x14   u32   the waveform's length
```

**This is the record [psp-audio.md](../../../formats/psp-audio.md) already
found and could not generalise.** That page reports, of `frontend.bnk`, *"ten
24-byte parameter records whose last two words are an offset and a length that
tile the waveform section exactly"* - and 24 bytes is `0x18`, whose last two
words sit at exactly `+0x10` and `+0x14`. The two readings were derived with no
knowledge of each other, one by tiling byte ranges across a bank and one out of
the runtime's argument list, and they land on the same two fields of the same
sized record.

The page's conclusion from the data side was that *"the race banks do not have a
record array of that shape at that offset, so the same reading does not
generalise"*. The runtime says the **shape** does generalise - it is the only
shape `Sas_QueueSetVoice` can be fed - so what differs between banks is where
the array lives, not what it is. The header already carries `waveform_count` at
`+0x1a`, which is the count that array must have.

Confidence **85**: the field offsets are direct reads, and the corroboration
with an independently derived 24-byte record is strong, but the stride itself is
the format page's measurement rather than this function's, and no bank has been
parsed this way yet.

**What this leaves is a data question rather than a code one.** Locating a
24-byte, `waveform_count`-long array whose last two words tile the waveform
section is a search over 39 banks with a very sharp test - the offsets and
lengths must partition the section exactly, which is what made `frontend.bnk`
findable in the first place.

## Solved: where each sound starts

`Scream_OpKeyOn` (`0x0898fc78`) is the opcode handler that binds a waveform to a
voice, and its first two lines are the answer:

```c
descriptor = *(u32 *)(bank + 0x34) + (command_word & 0xffffff);
...
g_sas_voices[v].waveform = descriptor;      /* +0x14 */
```

`bank + 0x34` is the **parameter block offset**, straight out of the header
[psp-audio.md](../../../formats/psp-audio.md) already documents. The `& 0xffffff`
is the command's **u24 operand**. So:

```text
waveform_descriptor = parameter_block + operand
waveform offset     = descriptor + 0x10
waveform length     = descriptor + 0x14
```

**That is the whole of "where each sound starts", and it is computable from the
data alone.** No runtime, no emulator: walk the command table, take every
command whose opcode binds a waveform, add its operand to the parameter block
offset, and read the last two words of the 24-byte record it lands on.

### Which opcodes, and the confirmation

Reading `g_scream_opcode_table`'s 45 entries, **exactly two point at this
handler: `0x01` and `0x09`.**

`psp-audio.md` records, independently and from the data side, that
`frontend.bnk`'s *"ten commands are all opcode `0x01` with operands 0, 24,
48 ... 216, pointing at ten 24-byte parameter records whose last two words are
an offset and a length that **tile the waveform section exactly**"*.

Operands 0, 24, 48 ... 216 are multiples of 24 **because that bank's ten
descriptors happen to be laid out contiguously**, which is what let a tiling
search find them without knowing the rule. The rule is the addition above, and
it does not require contiguity - which is exactly why the same search failed on
the race banks. The page's conclusion that *"the same reading does not
generalise"* was right about the search and wrong about the structure.

Confidence **90**. The arithmetic is a direct read; the opcode identification is
a table lookup; and an independently derived observation about a real bank
matches it in both the record size and the two field positions.

### The rest of the table

Eight consecutive opcodes, `0x0c` through `0x13`, share a single handler
(`0x0898efa8`), which is the shape of a family taking an index rather than eight
distinct instructions. The other 35 are one handler each and none is read.

The descriptor's remaining fields, from this function and
[the key-on](#the-waveform-descriptor-and-why-the-format-page-already-had-it):
`+0x01` is a note, and `+0x04` is reduced modulo `0x168` (360) both here and in
the voice record, so it is an **angle in degrees** - a pan position.

## How a cue's list runs: alternates, end, bend and loop, 2026-09-29

Read to play every Pulse cue as the timeline it authors (see
[psp-audio.md](../../../formats/psp-audio.md#every-pulse-cue-plays-its-timeline-2026-09-29)).
`Scream_StepCommandList` (`0x0898efd8`, decompiled again this session) runs the
handler at the program counter `+0x4a`, then - for a non-negative return - does
the **repeat step**, increments the counter, ends the list when it passes
`cue[4] - 1`, and otherwise stores `handler + 0x48 = (s16)next.word1 + return`.

| Address | Name | Confidence | What it does |
| --- | --- | --- | --- |
| `0x0898e178` | `Scream_OpAlternate` (existing row) | 88 | Operand byte 0 is the count `N`, byte 1 the stride `S`, byte 2 the last pick. Draws `rand % N`, and if it equals the last pick advances one, wrapping. Then `pc += pick * S`, `+0x50 = 1` (repeat armed), `+0x52 = S + 1`, `+0x54 = (N - pick - 1) * S`; returns 0. The stepper's repeat step decrements `+0x52` after every command and, at zero, clears `+0x50` and adds `+0x54` to the counter. So **one block of `S` commands runs and everything after the whole group of `N * S` commands runs after it** - a group in the middle of a list is a choice, not the end of the cue. All 3 Pulse groups checked read `N` equal to the number of key-ons that follow (`.COLLISIONS` 15, its Zone bank 10, `CANNONEXPLWALL` 9). A second `0x19` while the repeat is armed prints an error and returns `-1` |
| `0x0898eaa8` | `Scream_OpEnd` | 85 | `pc = cue[4] - 1`, returns 0. The stepper's own increment then runs off the end, so **the list ends here** and later commands are not reached by it. Table entry only; Ghidra had no function there (created 2026-09-29). Reading the code is 95; that nothing else reaches the commands after it (a marker seek by `FUN_0898dc4c` is unread) is why 85 |
| `0x0898e2b0` | `Scream_OpRandomBend` (existing row) | 85 | Returns 0, so it adds no delay. `Scream_SetSoundBend(handler, ((rand % 0x7fff) * 0xffff / 0x7fff - 0x8000) * (s8)operand / 100)`: one draw per execution, applied to the handler and its children. Through `Scream_ComputeVoiceNote` a negative bend scales the descriptor's `+0x08`, a positive one its `+0x09` (semitones), so the audible detune is at most that range - and **zero on a descriptor whose range bytes are zero**: `~SHIELD`, `ROCKET`, `MISSILEEXPWALL` are unaffected, `PLASMAHITSHIP`'s three layers span 5/2, 1/1 and 3/3 |
| `0x0898dfd0` | `Scream_OpNop14` (existing row) | 82 | `jr ra; li v0,0`. Adds nothing |
| `0x0898dfe0` | `Scream_OpLoopBack_q` | 65 | Scans backward from the counter for the nearest `0x15`, sets the counter to just before it (the stepper's increment lands on the `0x15`), sets flag `0x40` at `+0x16` the first time and returns 1 the later times (one extra tick on the next delay). A loop with no exit in the list, so it runs until the handler is killed. `_q`: the flag's other users and any exit are unread. `~BLOWUP` is `[key-on loop, 0x15, key-on, 0x1a 0, 0x16]` - the second waveform re-keyed every 43-44 ticks (0.17 s) for as long as the explosion holds the handler - and 20 Pulse cues carry the `[0x15, 0x16, 0x1a]` shape. **Not modelled by the timeline walk** |

`0x1e`/`0x1f`/`0x20`/`0x21` return 0 (`0x1e` disassembled: two stores and
`li v0,0`), so they add no time either; they matter only to a guard (`0x22`) or
a `-1..-4` parameter sentinel, and a cue with either is left to the flat pick.

## The command list is a 45-entry jump table

`Scream_StepCommandList` (`0x0898efd8`) is the interpreter `Scream_StartSound`
drives, and it is a straightforward dispatch:

```c
cmd    = *(u32 *)(cue + 8) + pc * 8;              /* 8-byte commands */
opcode = *(u8 *)(cmd + 3);
if (opcode > 0x2c) { error(); return 1; }
r = (*(code **)0x08ac326c)[opcode](handler, cue, cmd);
```

**Three arguments, not two** - `handler`/`cue` per the dispatch above, and
`cmd` itself in `$a2`, verified in the raw disassembly rather than the
decompile: `move a2,a0` where `a0` was just computed as `cmd` a few
instructions earlier, in the branch's delay slot before `jalr a3`. Every
handler this page names below receives its own command's 8 bytes directly,
which is what makes `Scream_OpAlternate`'s `param_3[0]`/`[1]`/`[2]` (below)
`cmd + 0`/`+ 1`/`+ 2` - the three operand bytes - rather than something a
level removed. The `lui a1,0x2c; addiu a1,a1,-0xd94` pair building
`0x2bf26c` two instructions before the call is an independent, in-code
confirmation of `g_scream_opcode_table`'s raw address from inside this
already-analysed function - the same `real = pseudo + 0x08804000` correction
[below](#four-opcodes-corroborated-against-hd-2026-09-04) needed for the
table's *entries*, here on the table's own base.

Three things follow. The command stride is **8 bytes**, matching the format
page. The opcode is the **high byte of the first word**, which is consistent
with that page's `{ u8 opcode, u24 operand }` reading of a little-endian word
rather than in conflict with it. And **the engine defines 45 opcodes**
(`0x00`-`0x2c`) through a jump table, `g_scream_opcode_table` at
**`0x08ac326c`**, where the format page
observed only nine distinct values in the shipped data - a count corrected to
27 on 2026-09-16, see [the census](#the-census-the-format-pages-nine-distinct-values-should-have-been)
below.

`handler + 0x4a` is the program counter, and a cue ends when it passes
`*(i8 *)(cue + 4) - 1`. **That refines the note above**: `cue + 0x04` is the
cue's command count, so "must be non-zero to play" and "is the command count"
are the same fact seen twice.

`handler + 0x50`/`+0x52`/`+0x54` are a repeat mechanism - a flag, a countdown,
and a signed jump added to the program counter when it reaches zero - so the
command list has loops. `handler + 0x48` accumulates the command's second word,
which is therefore a duration.

Reading the 45 handlers is the obvious next job, and one of them is what writes
`g_sas_voices[v] + 0x14`.

## Where this stops

**The writer of `+0x54` is `Sas_QueueSetVoice` (`0x0898bf4c`)**, found by
searching for the instruction that sets the dirty bit rather than the one that
stores the address - only two `ori ..., 0x20` sites exist in the whole sound
module:

```c
undefined4 Sas_QueueSetVoice(voice, addr, size, loop, only_adpcm) {
  if (only_adpcm != 0) printf("SCREAM ERROR: THIS SYSTEM ONLY SUPPORTS ADPCM VOICE DATA!\n");
  voices[voice].addr  = addr;   /* +0x54 */
  voices[voice].size  = size;   /* +0x58 */
  voices[voice].loop  = loop;   /* +0x5c */
  voices[voice].dirty |= 0x20;  /* +0x3c */
  return 1;
}
```

The decompiler writes the four stores as `(&DAT_08b89424)[voice * 0x1b]` and
friends; `0x08b89424 - 0x08b893d0` is `0x54`, and `0x1b` words is `0x6c` bytes,
so these are the same table and the same fields as the commit loop reads.
Confidence **92** - it is a direct store to the exact offsets, and its argument
order matches `sceSasSetVoice`'s.

**The chain is complete end to end.** Every link from a cue string to
`sceSasSetVoice` is read, and the one that mattered - how a sound's index
becomes a byte offset - turns out to be plain arithmetic on data the format page
already had.

## Four opcodes corroborated against HD, 2026-09-04

[`ps3-hdfury-eu/sound.md`](../ps3-hdfury-eu/sound.md) decoded `0x19`, `0x22`,
`0x23` and `0x24` on Wipeout HD's binary and left them **not corroborated on
PSP** - the four handler addresses it derived from `g_scream_opcode_table`
(`0x0018a178`/`0x0018a594`/`0x0018a658`/`0x0018a660`) refused
`decompile_function`, `disassemble_function` and `create_function` alike, and
`disassemble_bytes` reported success with zero instructions decoded.

**The blocker was the same wart [`workflow.md`](../../workflow.md#reading-an-unrelocated-database-until-it-is-reimported)
already names for `jal` targets and manually-found jump-table bases, one
shape further: a data table of function pointers the auto-analyzer never
walked.** `g_scream_opcode_table`'s 40 entries read back as `0x0018xxxx`-range
values - nowhere near `.text` (`0x08804000`-`0x08a76a3b`) - because the table
itself sits in `.data`, outside whatever the analyzer recognised as a jump
table to fix up. `real = pseudo + 0x08804000`, this binary's own image base,
same as every other instance of this wart: index `0x19` (25) reads
`0x0018a178`, `+ 0x08804000` is `0x0898e178`, and that address already had a
function on it - `disassemble_bytes` had been decoding real code the whole
time, just under a name nothing pointed `decompile_function` at. The same
correction unblocks `0x22` (`0x0898e594`), `0x23` (`0x0898e658`) and `0x24`
(`0x0898e660`); the last needed `create_function` first (auto-analysis had not
found its entry either), which now succeeds cleanly on the corrected address.

| Opcode | Address | Name | Confidence | What it does |
| --- | --- | --- | --- | --- |
| `0x19` | `0x0898e178` | `Scream_OpAlternate` | 88 | Pick a random one of the next N key-ons, re-rolling once - advance one and wrap - if the draw matches the cache from the cue's last play; jump the program counter to it |
| `0x22` | `0x0898e594` | `Scream_OpGuard` | 88 | Three-way compare a variable (a runtime global for a negative index, a per-voice local for a non-negative one) against an immediate; skip the next grain unless the comparison holds |
| `0x23` | `0x0898e658` | `Scream_OpMarker` | 82 | No-op - `return 0`, nothing else. Same two-instruction shape as HD's `li r3,0; blr` |
| `0x24` | `0x0898e660` | `Scream_OpGoto` | 88 | Scan a marker table by id, jump the program counter to the match; recursion-depth-guarded at 8, decrementing the counter and logging instead past the guard |

Each is the same algorithm as HD's reading, decompiled independently on a
different CPU (Allegrex MIPS, not PPC64) and a different binary, with no
shared analysis between the two sessions that found them:

```c
// Scream_OpAlternate (0x0898e178) - PSP
if (*(char *)(param_1 + 0x50) == '\0') {          // first call for this voice
    pick = rand() % (int)*param_3;                 // count is param_3[0] here
    if (pick == param_3[2]) {                       // cache from the last play
        pick += 1;
        if (pick == *param_3) pick = 0;             // wrap
    }
    param_3[2] = pick;                               // write the cache back
    voice->pc += pick * param_3[1];                  // param_3[1]: voices per alternate
    ...
    voice->pc_dirty = 1;                             // guards re-entry, +0x50
    return 0;
}
// second call for this voice: log and refuse, exactly as HD's handler does
```

That is `Scream_DoGrainAlternate` (HD, `0x00625e88`) field for field: the same
`rand() % count`, the same "roll once, advance-and-wrap on a repeat" shape
rather than reject-and-retry, the same per-voice re-entry guard gating a
one-shot roll per play, and the same cache write-back into the operand's own
byte. `Scream_OpGuard`'s three-way compare and `Scream_OpGoto`'s marker-table
scan plus recursion-depth guard (capped at 8, the same figure HD's page
records) match their HD counterparts the same way. `Scream_OpMarker` being a
bare `return 0` on both binaries is as certain as a two-instruction function
can be; **the *name* "marker" still rests entirely on what `Scream_OpGoto`
does with it**, exactly as HD's own page says of `Scream_DoGrainMarker` -
that inference is not strengthened by a second binary agreeing the function
itself does nothing.

Confidence **88** for `0x19`/`0x22`/`0x24`, **82** for `0x23`: per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md), a
second binary corroborating a decompiled reading is one of the two legs the
95-100 band needs and puts the claim in the 85-94 band on both sides now,
short of Established by the missing leg - a runtime trace. `0x23` stays lower
because agreement that a function does nothing corroborates the *shape*, not
the *name* built on top of it. This raises the same four rows on
[`ps3-hdfury-eu/sound.md`](../ps3-hdfury-eu/sound.md#opcodes-read-so-far) from
their prior 80/84/78/80.

## Opcode `0x14` is a no-op, corroborated on HD, 2026-09-08

[`track-sound-emitters.md`](track-sound-emitters.md#a-second-larger-set-resolves-and-still-cannot-be-played-38-nodes-on-eight-circuits)
recorded a confidence-88 hypothesis that `0x14` binds a waveform another way,
built on 9 of its 38 silent-cue nodes sharing that opcode and none other.
`g_scream_opcode_table[0x14]` (`0x08ac326c + 0x14*4` = `0x08ac32bc`) reads
`0x0898dfd0` directly - no `+0x08804000` pseudo-address correction needed this
time, and the read is cross-checked against the table's own already-confirmed
`0x19` entry four slots later (`0x0898e178`, matching
[the corroboration above](#four-opcodes-corroborated-against-hd-2026-09-04)
exactly) rather than trusted alone. `create_function` was needed - the
auto-analyzer had not walked this table either - and the result is two
instructions:

```
0898dfd0  jr ra
0898dfd4  li v0,0
```

**A bare no-op, the same shape as `0x23`/`Scream_OpMarker`.** HD's own table
(`0x00927614 + 0x14*4` = `0x00927664`) resolves through the same OPD-address
indirection [ps3-hdfury-eu/sound.md's dispatch table section](../ps3-hdfury-eu/sound.md#the-real-dispatch-table) describes,
to `0x006234c0`, TOC `0x008bd3c4` confirmed with `scripts/ps3-toc.py toc`
before trusting the decompile - `li r3,0; blr`, field for field the same
shape on a different CPU, no shared analysis between the two reads. Named
`Scream_OpNop14` (PSP) and `Scream_DoGrainNop14` (HD), matching this page's
`Scream_Op*`/HD's `Scream_DoGrain*` convention.

**This refutes the hypothesis rather than confirming it.** `0x14` does not
bind a waveform, another cue, or anything else - it ignores its own operand
bytes and returns unconditionally. The 9 nodes waiting on it
(`moather~birds`, `dekonst~CRANE`, both `talonsj~SETREG` cues) are not a
decode gap: their command list genuinely authors no sound, on this disc and,
by the HD corroboration, apparently by design in the engine rather than a
Pulse-specific omission. `talonsj~SETREG`'s cues carrying `0x14` *beside*
`0x1e` no longer needs explaining as "these aren't all just registers" -
a cue can validly carry two commands that both do nothing.

Confidence **82** - the same figure `0x23` carries and for the same reason:
two-instruction agreement across two binaries corroborates that the function
does nothing, which is unambiguous, but does not by itself explain *why* the
disc authors an opcode for it.

**`0x14` is not an isolated stub - its neighbour `0x15` is the same shape, and
that is where the discriminating check landed.** Reading the table's own
`0x19` entry proved the base and stride are right, but not that index 20
specifically is `0x14` rather than 19 or 21; the check that does is reading
the *neighbours* and finding three distinct addresses 8 bytes apart
(`0x0898dfd0`/`0898dfd8`/`0898dfe0`). The first two are both `jr ra; li v0,0`
- `0x15` (`0x0898dfd8`, `Scream_OpNop15`, also corroborated on HD as
`Scream_DoGrainNop15`, `0x006234c8`) does exactly what `0x14` does. The third,
`0x16` (`0x0898dfe0`, not otherwise read this session), is not a stub: it
scans the command list **backward** from the current program counter for a
command whose opcode byte is `0x15`, sets a flag bit at `+0x16` (`0x40`) the
first time it finds one, returns `1` if that bit was already set, and calls
an unread error function (`FUN_08996708(8, ...)`) if the scan runs off the
start without finding one. That is a marker-shaped role one direction removed
from `0x24`/`Scream_OpGoto`'s forward scan for `0x23` - **`0x15` has a
confirmed consumer, `0x14` does not.** No opcode found scanning for `0x14`
specifically, so unlike `0x15` its "why does the disc author this" stays
genuinely open rather than answered by a sibling opcode.

## The pitch: a waveform's rate is its descriptor's centre note, 2026-09-16

`docs/formats/psp-audio.md` carried the rate every waveform plays at as a
placeholder 44,100 Hz, with the descriptor's first word as the candidate
field. This section traces every writer of the SAS voice record's pitch field
(`+0x50`, the one bit `0x10` in the dirty word commits through `Sas_SetPitch`)
back to the bank bytes it is computed from, and then watches the hardware call
live. The answer is two signed bytes and an integer table walk, and the walk is
now [`oag_formats::sblk::pitch`](../../../../crates/formats/src/sblk/pitch.rs).

### The writer set is closed

Searching the whole binary for `ori ..., 0x10` inside the `0x0898xxxx`-
`0x0899xxxx` sound module finds one store to `+0x50`:

```c
undefined4 Sas_QueuePitch(voice, pitch) {                 /* 0x0898bf14 */
  (&DAT_08b89420)[voice * 0x1b] = pitch;                  /* +0x50 */
  (&DAT_08b8940c)[voice * 0x1b] |= 0x10;                  /* +0x3c, dirty */
  return 1;
}
```

`0x08b89420 - 0x08b893d0 = 0x50`, `0x1b` words is `0x6c` bytes: the same
table and the same field `Sas_CommitVoices` reads and hands to
`__sceSasSetPitch`. It has **exactly two callers**, `Scream_KeyOnVoice`
(`0x0899456c`) and `Scream_UpdateVoicePitch` (`0x0898f25c`), and both compute
the argument the same way, so every pitch word this binary ever commits comes
out of the chain below. Confidence **92** - a direct store to the exact offset,
the sibling of `Sas_QueueSetVoice` in shape and address, and the two-caller
count is Ghidra's xref list on a function it resolves cleanly.

### The chain

```text
Scream_StartSound        0x0898f864   handler+0x34 = 0x3c (note 60), +0x35 = 0 (fine)
                                      handler+0x14 = pitch offset (arg flag 0x10, else 0)
                                      handler+0x36 = pitch bend   (arg flag 0x20, else 0)
Scream_OpKeyOn           0x0898fc78   copies +0x34/+0x35 to voice+0x18/+0x19,
                                      +0x36 to voice+0x24, +0x14 to voice+0x26,
                                      the descriptor pointer to voice+0x14
Scream_KeyOnVoice        0x0899456c   note, fine  = Scream_ComputeVoiceNote(wf, bend, offset, note, fine)
                                      pitch       = Scream_VoicePitch(wf+0x02, wf+0x03, note, fine)
                                      Sas_QueuePitch(voice, pitch)
Scream_UpdateVoicePitch  0x0898f25c   the same two calls per voice, every update, with
                                      +0x36 = +0x42 + +0x40 and +0x14 = +0x46 + +0x44
```

`Scream_ComputeVoiceNote` (`0x089950a0`, confidence **88**) is the
modulation:

```c
void Scream_ComputeVoiceNote(wf, bend, offset, note, fine, int *out_note, uint *out_fine) {
  if (bend < 0) v = wf[0x08] * bend * 0x80 / 0x8000;   /* bend-down range, semitones */
  else          v = wf[0x09] * bend * 0x80 / 0x7fff;   /* bend-up range */
  total     = note * 0x80 + fine + offset + v;         /* 1/128ths of a semitone */
  *out_note = total / 0x80;                            /* toward zero */
  *out_fine = total < 0 ? -((-total) & 0x7f) : total & 0x7f;
}
```

`Scream_VoicePitch` (`0x08994fec`, confidence **90**) reads the descriptor's
`+0x02` and `+0x03` as **signed bytes** (`sll 0x18; sra 0x18` on both), takes
the absolute value of a negative centre note, and multiplies the result by
`0x1278b >> 16` when it did:

```text
08995018  bgez a3, 0x08995040          ; centre note >= 0: plain
08995020  subu a3, zero, a3            ; else negate ...
0899503c  andi a3, a3, 0xffff
08995068  jal  Scream_NoteToPitch
08995078  lui  a0, 0x1
0899507c  addiu a0, a0, 0x278b         ; 0x1278b
08995080  mult a2, a0
08995088  srl  a2, a2, 0x10            ; pitch * 0x1278b >> 16
```

`Scream_NoteToPitch` (`0x089952c4`, confidence **90**) is Sony's standard
note-to-pitch: `(centre_note, centre_fine, note, fine) -> pitch`, with
`0x1000` meaning "the sample's own rate". The fine offset is
`centre_fine + fine - 0x7f`, borrowing a semitone from `note` while negative,
so a centre fine of `0x7f` is in tune and `0` is one semitone flat. The
semitone distance `note - centre` splits into an octave shift of `0x1000` and
a 12-entry Q15 table, and the fine offset indexes a 128-entry Q15 table:

| Table | Address | Entries | Closed form |
| --- | --- | --- | --- |
| `g_scream_semitone_table` | `0x08ac36dc` | 12 x u32, `0x8000 .. 0xf1a1` | `floor(32768 * 2^(i/12))`, all 12 |
| `g_scream_fine_table` | `0x08ac370c` | 128 x u32, `0x8000 .. 0x878c` | `floor(32768 * 2^(i/1536))`, all 128 |

The two abut - 560 bytes, 140 words, no slack - and the closed forms hold on
every entry (checked in `crates/formats/src/sblk/pitch/tests.rs`), so a fine
step is 1/128 of a semitone and the note is a MIDI note. Confidence **92** on
both labels: the addresses are `lui/addiu` pairs in the function that indexes
them, and the closed form is what a wrong base could not produce.

**The default play is note 60, fine 0, always.** `Scream_StartSound`'s
`sb a0, 0x34(s0)` at `0x0898fa58` (with `a0 = 0x3c`) and `sb zero, 0x35(s0)`
at `0x0898fa5c` are the only stores to those two handler bytes anywhere in
the sound module - an instruction search over the whole binary for `sb` to
`0x34(`/`0x35(` finds no other in `0x0898xxxx`-`0x0899xxxx` - so a
descriptor's rate at rest is `Scream_VoicePitch(centre, fine, 60, 0)` and
nothing in a cue's command list can change it. What the game adds is the
offset and bend through `Scream_UpdateVoicePitch`, which is how the engine
note follows speed; that is the caller's business, not the bank's.

### What `0x1278b` is, and is not

`0x1278b / 0x10000 = 1.154465`. It is not `48000 / 44100` (`1.088`), and not
that times `2^(1/12)` either (`0x12735`). It is recorded as read and not
explained. What it *does*: with every centre note on both Pulse discs and
both Pure discs negative (0 of 2,334 key-on descriptors positive, none
`-128`), the multiply is universal, and it puts `(-86, 66)` on exactly
`0x400`, `(-74, 66)` on `0x800` and `(-62, 66)` on `0x1000` - 11,025, 22,050
and 44,100 Hz, the rates the disc's banks are full of. A reading of the
bytes without it lands nowhere round.

### Confirmed live: 190 of 190

PPSSPP v1.20.4, `pulse-psp-usa.chd`, own profile, breakpoint at
`__sceSasSetPitch` (`0x08a76c7c`), reading `a1` (voice) and `a2` (pitch) at
every hit and, off the voice record itself, the descriptor pointer at
`+0x14`, the note/fine at `+0x18`/`+0x19` and the bend/offset at
`+0x24`/`+0x26` - so each hit's prediction is computed from the same inputs
the game used, through the port:

| Where | Hits | Matched | Distinct pitch words |
| --- | ---: | ---: | --- |
| Front end (menus, `frontend.bnk`) | 40 | **40** | `0x260`, `0x35c`, `0x400` |
| Time Trial on Talon's Junction, driven | 150 | **150** | 47, `0x35c` .. `0x908` |

The front end is the discriminating case the format page asked for: three
different words out of one bank, each exactly what its descriptor's
`(centre, fine)` predicts, which falsifies "one rate per bank" outright. The
race capture exercises the modulation too - 90 of its hits carry a non-zero
pitch offset (down to `-1252`) and 73 a non-zero bend - and every one of
those reproduces, so `Scream_ComputeVoiceNote`'s rounding is right as well.
Every hit's `ra` was `0x08a2aeb0`, inside `Sas_SetPitch`, as the table above
already said it would be.

Confidence **95** on the decode as a whole: a decompiled chain with every
link read, an integer port that reproduces 190 live hardware calls to the
bit, and a data-side invariant (exact standard rates falling out of a walk
that was not fitted to them) on three discs. What is *not* claimed is an
authored sample rate: the bytes encode a transposition against the SAS
core's 44,100 Hz, and `(note, fine)` cancelling against the centre gives
`0x1000`, so "a 16 kHz recording pitched down" and "a 15,569 Hz recording"
are the same descriptor. The PS2's banks are byte-identical but its
`SCREAM.IRX` was not read; whether its arithmetic (on a 48 kHz SPU2 base)
lands on the same rates is open, and `oag_game` plays PS2 banks through the
PSP walk on the strength of the bytes being the same.

### Names owed and taken

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x0898bf14` | function | `Sas_QueuePitch` | 92 |
| `0x08994fec` | function | `Scream_VoicePitch` | 90 |
| `0x089952c4` | function | `Scream_NoteToPitch` | 90 |
| `0x089950a0` | function | `Scream_ComputeVoiceNote` | 88 |
| `0x0898f25c` | function | `Scream_UpdateVoicePitch` | 85 |
| `0x08ac36dc` | data | `g_scream_semitone_table` | 92 |
| `0x08ac370c` | data | `g_scream_fine_table` | 92 |

`Scream_UpdateVoicePitch` is 85 rather than 88 because what triggers it is
not read - it walks a handler's voices (`FUN_089930ac(handler, i)` yields
them) and re-pitches each, and the sums it forms first (`+0x42 + +0x40` into
the bend, `+0x46 + +0x44` into the offset) read as base-plus-modulation
pairs without the writers of any of the four being traced.

**Not named:** `FUN_0898f5e8`, which `Scream_OpKeyOn` calls with the
handler's `+0x3a` (a volume, `0x400` at rest) and the descriptor, and whose
result goes to the voice allocator as its first argument. It reads the
descriptor's `+0x00` as a signed byte and, when flag bits `0x6` of `+0x0e`
select a divisor (`/5` or `/2`), scales that byte down as the volume falls
below `0x3b6`. **Hypothesis: a voice priority that quiet plays give up**,
with `+0x00` the descriptor's authored priority - which would make the
24-byte record `priority, volume, centre note, centre fine, pan, ...`, Sony's
usual tone layout. Below 70, so no name; the allocator (`FUN_089953c0` /
`FUN_08994550`) is unread and would settle it.

## Five more opcodes, 2026-09-16: the ones the banks actually use

The format page's observed-in-data list had four opcodes with no PSP handler
read: `0x05`, `0x06`, `0x1e`, `0x29`, plus `0x08` beside `0x05`. All five are
read now, off `g_scream_opcode_table` at `0x08ac326c` - which, unlike the
2026-09-04 pass found, now reads real `0x0898xxxx` addresses directly (the
`0x19` entry is `0x0898e178`, matching that pass exactly, so the base and
stride are the confirmed ones). Three corroborate HD's readings; two are new.

| Opcode | Address | Name | Confidence | What it does |
| --- | --- | --- | --- | --- |
| `0x05` | `0x08990100` | `Scream_OpPlayChild` | 85 | Resolve the 32-byte child record's cue (index at `+0x0c`, or name at `+0x10` looked up in every bank then this one); volume from `+0x00` and pan from `+0x04`, each through the `-1..-5` sentinel scheme (`-5` random, `-1..-4` the handler's four register bytes at `+0x4c`); `Scream_StartSound` with the parent's volume x child volume / 127, its pan, its pitch offset and bend (`+0x46`, `+0x42`) and its four registers, flags `0x8000007c`; link the child to the parent (`FUN_089929a4`) |
| `0x06` | `0x0898ed5c` | `Scream_OpStopChild` | 85 | Resolve the child the same way, then walk the parent's child list (`handler+0x2c`, linked by `+0x30`) and unlink-and-kill (`FUN_08993944(child, 1, 0, 0)`) every instance whose cue is the resolved one, until none is left |
| `0x08` | `0x0898eb6c` | `Scream_OpBranch` | 85 | Resolve the child; kill this handler's own voices (`FUN_08994b54(handler+0x18, 0)`); if the cue's flag `0x8` is set, ask `FUN_0898f6a4` for an existing instance and kill it unless it is this one; then **replace the handler's cue, defaults, command list and bank with the child's** and reset the program counter (`+0x4a = -1`), so the interpreter carries on inside the other cue. **No bounds check on the index on PSP** - HD's `Scream_DoGrainBranch` has one against `cue_count` and the `snd_SFX_GRAIN_TYPE_BRANCH invalid sound index` string; this build reads `+0x0c` and indexes with it |
| `0x1e` | `0x0898e41c` | `Scream_OpSetRegister` | 88 | Two lines: operand byte 0 is a register index, byte 1 the value. Non-negative writes the handler's own byte at `+0x4c + i`; negative writes the global at `0x08ac3247 - i`. The same two address spaces `Scream_OpGuard` compares against, and `talonsj`'s `~SETREG` cues are named for it |
| `0x29` | `0x0898ea4c` | `Scream_OpKeyOff` | 85 | One call: `FUN_089949d0(handler+0x18, 0)`, which for every voice in the handler's bitmask that is in state `1` sets its bit in `g_sas_keyoff_pending` (`0x08b8f784`) and clears it from the key-on mask (`0x08b8f780`). `FUN_089947c8` flushes that mask a voice at a time through `FUN_0898c050` -> `Sas_SetKeyOff` (`0x08a2adcc`) -> `__sceSasSetKeyOff` (`0x08a76c6c`), so the opcode releases the handler's voices into their ADSR release rather than cutting them |

`Sas_SetKeyOff` is named on the same mirroring convention as the other four
`Sas_Set*` wrappers (confidence **90**: a direct `jal 0x08a76c6c` onto the
import stub Ghidra already names, behind the usual "is SAS initialised"
guard), and `g_sas_keyoff_pending` at **88** - its consumer is read through
to the import; the sibling masks at `0x08b8f780` and `0x08b8f788` are
touched by the same functions and left unnamed, their consumers unread.

`0x05`, `0x06` and `0x08` are each the same algorithm HD's
[`Scream_DoGrainPlayChild` / `StopChild` / `Branch`](../ps3-hdfury-eu/sound.md#opcodes-read-so-far)
read on a different CPU with no shared analysis - the same second-binary
corroboration the 2026-09-04 pass gave `0x19`/`0x22`/`0x23`/`0x24` - with
the one real difference (the missing bounds check) recorded above rather
than smoothed over. That lifts HD's three from 82/78/84 into the 85 band on
both sides. `0x1e` and `0x29` have no HD reading yet.

Not read from this pass: `FUN_0898f6a4` (the flag-`0x8` "find an existing
instance" the branch consults), `FUN_089929a4` (the parent-child link) and
`FUN_08993944` (the kill), which the three child opcodes lean on and which
would be the next three to name.

### The census the format page's "nine distinct values" should have been

Counting the opcode byte of every command in every bank (Pulse USA
`Data.wad` + `FE.wad`, Pure EU, PS2 EU) gives **27 distinct opcodes on
Pulse**, not nine, and `0x09` - the second key-on `KEY_ON_OPCODES` carries
from the table - appears in **none** of them. By count on Pulse USA's
`Data.wad`: `0x01` 880, `0x23` 238, `0x1a` 231, `0x24` 230, `0x22` 205,
`0x05` 195, `0x1e` 130, `0x1b` 122, `0x29` 90, `0x14` 89, `0x04` 69, `0x20`
66, `0x06` 58, `0x15`/`0x16` 49 each, `0x26` 48, `0x08` 46, `0x19` 42,
`0x25` 24, then `0x28` 4, `0x0a` 4, `0x2b` 3, `0x17`/`0x18`/`0x1f`/`0x21` 2
each, `0x1c` 1. The heavy unread ones were read in the same session:

| Opcode | Address | Name | Confidence | What it does |
| --- | --- | --- | --- | --- |
| `0x1a` | `0x0898e270` | `Scream_OpRandomDelay` | 85 | Returns `rand() % (operand + 1)`. A handler's non-negative return is what `Scream_StepCommandList` **adds to the next command's own delay word** (`handler+0x48 = next.word1 + r`), so this is a random extra wait before the next grain |
| `0x1b` | `0x0898e2b0` | `Scream_OpRandomBend` | 85 | A random value in `-0x8000..0x7fff`, scaled by operand byte 0 as a percentage, written as the sound's pitch bend through `Scream_SetSoundBend` (`0x0898f174`, **85**): that resolves the handle, writes `+0x42` on the handler and every child, and calls `Scream_UpdateVoicePitch`. Through the descriptor's own bend ranges (`+0x08`/`+0x09`, semitones) that is a random detune of up to the authored range |
| `0x1f` | `0x0898e458` | `Scream_OpSetRegisterRandom` | 88 | Register `byte0 = rand() in byte1..=byte2`, same local/global addressing as `0x1e` |
| `0x20` | `0x0898e4dc` | `Scream_OpIncRegister` | 88 | Register `byte0 += 1`, saturating at `127` |
| `0x21` | `0x0898e538` | `Scream_OpDecRegister` | 88 | Register `byte0 -= 1`, saturating at `-128` |
| `0x25` | `0x0898e768` | `Scream_OpGotoRandomMarker` | 88 | Pick a marker id in `byte0..=byte1`, scan the cue's commands for an opcode `'#'` (`0x23`) whose byte 0 is that id, set the program counter to it; the same 8-deep recursion guard (`DAT_08ac3268`) and error path as `Scream_OpGoto`. HD's own `snd_DoGrain` strings name a "Goto Random Marker" |
| `0x26` | `0x0898e8d4` | `Scream_OpWaitForVoices` | 78 | If the handler has any voice open (`+0x18`/`+0x1c` masks) or has keyed one on (flag `0x10` at `+0x16`), step the program counter back onto itself and return 1. That return goes through the same rule as `0x1a`'s: `Scream_StepCommandList` re-increments the counter (landing on this command again) and writes `handler+0x48 = word1 + 1`, a non-zero delay, which is what `Scream_StartSound`'s `while (... && handler+0x48 == 0)` loop exits on. So the command list blocks here, re-tested once the delay runs out, until the handler's voices have ended. 78 rather than higher because what clears the masks and the flag at voice end is not read |
| `0x04` | `0x0898de84` | `Scream_OpConfigureLfo_q` | 60 | Operand byte 0 selects one of the handler's four `0x34`-byte modulator slots (initialised in `Scream_StartSound`'s four-iteration loop at `+0x5c`); byte 1 enables it, and the rest of the 16-byte parameter record fills a type (`+0x03`), rate (`+0x06`), flags (`+0x08`, bit `0x2` = random start phase, `& 0x7ff << 16`), phase (`+0x0a`) and depth (`+0x0c`), then `FUN_08996a90(slot)` starts it, or `FUN_08996ba8` stops it when byte 1 is zero. Rate, phase, depth and a random phase are an LFO's fields; the two functions that run it are unread, hence `_q` |

Still unread, by count: `0x28` (4), `0x0a` (4), `0x2b` (3), `0x17`, `0x18`,
`0x1c`, `0x27`; and `0x16` (49) is decompiled but unnamed. `0x25` is the
obvious candidate for where the 15% of Pulse grains the goto reading made
"unreachable" go - a random goto is a goto whose target no static walk that
only follows `0x24` can pick - but that is a hypothesis, not a measurement:
the reachability walk has not been re-run with `0x25`'s marker ranges
included, and it is what would turn 15% into a number.

## The master tick and the delay word, 2026-09-29

A command's second word is a **delay in master ticks**, and one master tick
is **258.4 Hz**. Both were needed to play Pulse's Zone announcer as a sequence
(see [psp-audio.md](../../../formats/psp-audio.md#a-cue-can-be-a-sequence-and-zone_n-is-one)),
and both come from code this page had not read: the three functions between
`Audio_OutputThread` and `Scream_StepCommandList`.

| Address | Name | Confidence | What it does |
| --- | --- | --- | --- |
| `0x08993eb4` | `Scream_MasterTick` | 88 | Increments a pending-tick count (`0x08ac3688`) on every call, and while it is positive runs one tick: `++0x08ac35e0`, `Scream_TickHandlers`, the voice commit, and every fourth tick a slower update |
| `0x08993124` | `Scream_TickHandlers` | 80 | Walks the live handler list (`0x08ac35a0`); a handler whose type nibble is `5` (a playing cue) and that is not paused (`+0x16 & 2`) goes to `Scream_TickCommandList` |
| `0x0898db80` | `Scream_TickCommandList` | 88 | `handler + 0x48 -= 1`, then `while (delay < 1 && pc != -1) Scream_StepCommandList(handler)` |

**The delay word.** `Scream_StepCommandList` runs the command at `pc`, advances
`pc`, and - unless the list ended - stores `handler + 0x48 = (s16)next.word1 +
result`, where `next.word1` is the *second word of the next command* and
`result` is the handler's return (zero for `Scream_OpKeyOn` and
`Scream_OpPlayChild`, both read this session). `Scream_StartSound` seeds
`handler + 0x48` from command 0's second word and runs commands while it is
zero. So the second word is the wait **before its own command**, counted from
the previous command's execution: zero runs in the same tick, `d` runs `d`
ticks later. `Scream_OpPlayChild` calls `Scream_StartSound` for the child, so
the child's own list starts at that moment and the parent's next delay counts
from the child *command*, not from the child's end.

**The tick.** `Audio_OutputThread` (`0x0898ca54`) has four passes of an inner
loop per output buffer. The raw disassembly shows `jal 0x08993eb4` at
`0x0898cb00`, `0x0898cbf8` and `0x0898cc00`: **three master ticks per pass**,
with the mixer call `0x0898c7e8` and a 0x400-byte copy between the first and
second tick and again after the third, so two 256-frame grains per pass. The
buffer is `0x2000` bytes and is handed to `Audio_OutputPannedBlocking` as 2,048
frames. So 12 ticks per 2,048 frames at 44,100 Hz, `44100 * 3 / 512 = 258.398`
per second.

**Measured live, PPSSPP 1.20.4, Pulse USA, silent, own instance.** Reading the
tick counter `0x08ac35e0` and `cpu.status`'s cycle counter (222 MHz) at
stepping stops, three consecutive 6.0 s windows of emulated time:

| Window | Ticks | Emulated s | Ticks per s |
| --- | --- | --- | --- |
| 1 | 1548 | 6.006 | 257.7 |
| 2 | 1560 | 6.006 | 259.7 |
| 3 | 1546 | 5.985 | 258.3 |

Confidence **90** for the rate (static count and live count agree to within
the window's own resolution of one tick in about 1,550). The delay-word
reading is **88**: three decompiled functions agree, and the Zone announcer's
own data is consistent with it (the second word of a `zone_N` cue's CLEAR
grain is 140-250 ticks, 0.54-0.97 s, after a number that lasts 0.52-0.76 s -
a wait, not a position).

**What is not read.** The phase of the tick against the moment a cue is
started (up to one tick, 3.9 ms, either way); `FUN_0898dc4c`, the sibling that
seeks the list to a marker; and whether any other build's tick is the same - Wipeout HD, Pulse's PS2 pressing
and the Vita are not measured. The tick belongs to a build rather than to a bank's
byte order, so `oag_title::SequenceTick` carries it per title: `Psp` for Pulse
(lent, and labelled chosen, to its PS2 pressing) and `Unknown` for the rest.

## Not determined

- **24 of the 45 opcode handlers, plus one read but not confidently named.**
  `0x01`, `0x04` (`_q`), `0x05`, `0x06`, `0x08`, `0x09`, `0x14`, `0x15`,
  `0x19`, `0x1a`, `0x1b`, `0x1e`, `0x1f`, `0x20`, `0x21`, `0x22`, `0x23`,
  `0x24`, `0x25`, `0x26` and `0x29` are named and confidence-scored;
  `0x16`'s handler is decompiled (a backward scan for `0x15`, see
  [above](#opcode-0x14-is-a-no-op-corroborated-on-hd-2026-09-08)) but its
  *purpose* is not established well enough to name past the rubric's
  50-confidence floor... **update 2026-09-29: named `Scream_OpLoopBack_q`**
  ([above](#how-a-cues-list-runs-alternates-end-bend-and-loop-2026-09-29)).
  The rest are not read, and
  eight of the 45 share one handler. Of the 27 opcodes the Pulse banks
  actually use, five (`0x0a`, `0x17`, `0x18`, `0x1c`, `0x28`)
  have no named handler, and together they are 65 of the 2,881 commands in `Data.wad`.
- **Whether `0x01` and `0x09` differ.** They share a handler, so any difference
  must come from the command word rather than the dispatch.
- ~~**The extraction itself.**~~ **Done.** Both rules are implemented in
  `oag-formats::sblk` and validated against all 39 banks: 595 waveform spans
  tile every waveform section exactly and number each bank's declared
  `waveform_count`, and 607 names resolve one per cue. See
  [psp-audio.md](../../../formats/psp-audio.md#where-each-sound-starts). That
  takes the key-on arithmetic from 90 to **94**, the rubric's cap for a reading
  that makes an arithmetic invariant come out exactly across many real files.
- ~~**What the cue record's `+0x08` points at after fixup.**~~ **Settled from
  the data side.** On disc it is a **byte offset into the command table biased
  by nothing**, so `first_command = *(u32 *)(cue + 0x08) / 8` and the count is
  `*(u8 *)(cue + 0x04)` - the same `cue + 4` this page already reads as the
  command count. The runs so derived **tile each bank's command table exactly**
  on all 83 banks across the PSP and PS2 discs, and the five zero-count cues
  that sit outside are the ones `Scream_StartSound`'s own `cue + 0x04` gate
  refuses to play. See
  [psp-audio.md](../../../formats/psp-audio.md#a-cue-owns-a-run-of-the-command-table).
  What the *fixup itself* does - which function walks the cues at load and adds
  the base - is still unread; the arithmetic no longer needs it. Its `+0x04` is
  also read as the handler's `+0x12`, a `s16` the play loop tests against zero.
- ~~**Which command in a cue's run selects the waveform that sounds.**~~
  **Decoded.** `0x19` is `Scream_OpAlternate`, corroborating
  [HD's reading](#four-opcodes-corroborated-against-hd-2026-09-04): a random
  pick among the key-ons that follow, never repeating the immediately
  previous pick. Wired into `oag_game::audio::sfx::Banks::pick`
  (`crates/game/src/audio/sfx/banks.rs`), 2026-09-04.
- **The name hash**, `FUN_089924ec`. Not needed to extract names - the entry
  array walks linearly - but needed to reproduce a lookup faithfully.
- **What consumes the cue-request list.** `Sound_Play` pushes 0x30-byte nodes
  onto a per-owner list (head `owner+0x58`, count `owner+0x54`); the consumer
  that turns a queued cue string into `Scream_PlaySoundByName` has not been
  found.
- ~~**The name table has not been run against a real bank.**~~ **Run against
  all 39.** One `{name, cue}` pair per cue in every bank, no invalid index, no
  unprintable byte, and the `Sound_Play` strings this page's neighbours
  recovered - `"SPEEDUPPAD"`, `"~ENGINE"`, `"ABSORB"` - all resolve in the bank
  that should hold them. That takes the layout from 85 to **94**. One
  correction: the entry-array offset at `names + 0x08` is relative to **the name
  block**, not to the section - it reads `0x98` in all 39 banks, which is
  exactly `0x18 + 64 * 2`, so the hash has **64 buckets**.
- **The other four dirty bits**, and the two output paths the `0x80` bit picks
  between.
- **EU cross-verification.** Nothing on this page has been checked against
  `psp-pulse-eu`, which is normally this project's primary target.
- ~~**Nothing here is runtime-verified.**~~ **The pitch chain is**, 190 of
  190 hits at `__sceSasSetPitch` (see
  [the pitch section](#the-pitch-a-waveforms-rate-is-its-descriptors-centre-note-2026-09-16)),
  and `Sas_SetVolume`'s call site and the voice table's base were read live
  on 2026-09-06. The cue dispatch and the opcode handlers are still static
  reading only.
