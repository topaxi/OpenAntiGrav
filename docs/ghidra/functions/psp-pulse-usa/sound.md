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
| `0x0f` | `FUN_08a2ae08(voice, ...)` | `+0x40`, `+0x44`, `+0x48`, `+0x4c` |
| `0x10` | `FUN_08a2ae6c(voice, ...)` | `+0x50`, defaulting to `1` when it is zero |
| **`0x20`** | **`Sas_SetVoice(voice, ...)`** | **`+0x54`, `+0x58`, `+0x5c`** |
| `0x40` | `FUN_08a2af18(voice, ...)` | `+0x60` |
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
follows from the wrapper above; **85** on the table's shape and base address,
from the loop's stride, its bound, and the `0x6c` increment.

The four bits whose calls are *not* `sceSasSetVoice` are deliberately not named.
Their argument counts fit the obvious guesses - four for ADSR, one for pitch,
two for a keyed volume pair - but an argument count is not evidence, and this
project does not dress a guess as a name.

The `0x80` branch choosing between `FUN_08a2b088` and `FUN_089961e4` on
`DAT_08ac3224` is the same flag that gates `FUN_089960e0` at the top of the
loop, so there are two output paths and one of them is selected globally. Not
investigated.

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
wf + 0x02   s8    a note or pitch base
wf + 0x03   s8    a second one
wf + 0x0a   u16   paired with +0x0c into an envelope call
wf + 0x0c   u16
wf + 0x0e   u16   flags; 0x40 selects loop mode, 0x80 the ADPCM assertion
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

## The command list is a 45-entry jump table

`Scream_StepCommandList` (`0x0898efd8`) is the interpreter `Scream_StartSound`
drives, and it is a straightforward dispatch:

```c
cmd    = *(u32 *)(cue + 8) + pc * 8;              /* 8-byte commands */
opcode = *(u8 *)(cmd + 3);
if (opcode > 0x2c) { error(); return 1; }
r = (*(code **)0x08ac326c)[opcode](handler, cue);
```

Three things follow. The command stride is **8 bytes**, matching the format
page. The opcode is the **high byte of the first word**, which is consistent
with that page's `{ u8 opcode, u24 operand }` reading of a little-endian word
rather than in conflict with it. And **the engine defines 45 opcodes**
(`0x00`-`0x2c`) through a jump table, `g_scream_opcode_table` at
**`0x08ac326c`**, where the format page
observed only nine distinct values in the shipped data - so the banks use a
fifth of the instruction set.

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

## Not determined

- **43 of the 45 opcode handlers.** `0x01` and `0x09` are read; the rest are
  not. The format page's other observed opcodes (`0x05`, `0x06`, `0x15`, `0x16`,
  `0x1e`, `0x22`-`0x24`, `0x29`) are what a bank actually uses, so they are the
  ones worth reading, and eight of the 45 share one handler.
- **Whether `0x01` and `0x09` differ.** They share a handler, so any difference
  must come from the command word rather than the dispatch.
- **The extraction itself.** The rule above is not implemented in
  `oag-formats::sblk` and has not been run against all 39 banks; doing so is
  what would take it from a documented rule to a validated one.
- **What the cue record's `+0x08` points at after fixup.** Its `+0x04` is read
  as the handler's `+0x12`, a `s16` the play loop tests against zero.
- **The name hash**, `FUN_089924ec`. Not needed to extract names - the entry
  array walks linearly - but needed to reproduce a lookup faithfully.
- **What consumes the cue-request list.** `Sound_Play` pushes 0x30-byte nodes
  onto a per-owner list (head `owner+0x58`, count `owner+0x54`); the consumer
  that turns a queued cue string into `Scream_PlaySoundByName` has not been
  found.
- **The name table has not been run against a real bank.** Doing so is what
  takes the layout above from 85 to 90, and it is a `just test-data` sized job.
- **The other four dirty bits**, and the two output paths the `0x80` bit picks
  between.
- **EU cross-verification.** Nothing on this page has been checked against
  `psp-pulse-eu`, which is normally this project's primary target.
- **Nothing here is runtime-verified.** Every claim is static reading.
