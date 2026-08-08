# The sound engine: cue dispatch and the SAS voice table

## The question this page answers

`docs/formats/psp-audio.md` calls per-sound boundaries inside a `.bnk` *"the gap
that matters, because it is what stands between here and extracting individual
sounds"*. Everything about race audio is blocked behind it.

This page attacks it from the code side rather than the data side, and gets far
enough to name the field the answer has to land in. It does not close the gap.

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
handed to the PSP's hardware synth. This page walks the far end back three hops
and finds where those two numbers live in memory.

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

## Where this stops

**The writer of `+0x54` was not found.** The obvious searches do not reach it:
the record base lives in a register at the write site, so `sw ... 0x54(reg)`
returns 135,729 instructions' worth of stack-frame noise across the binary, and
the five `0x08994xxx` functions that call the unlink helper contain no store to
`+0x54` at all. The write is somewhere between the cue-request list and this
table, and finding it is the next step rather than a dead end.

## Not determined

- **Who writes `voice + 0x54`.** The single most valuable next question, because
  it is the join between a `.bnk` and a sounding voice. Suggested attack: set a
  write watchpoint on `0x08b893d0 + 0x54` in PPSSPP and play one known cue -
  a runtime capture answers in one step what static search has not.
- **What consumes the cue-request list.** `Sound_Play` pushes 0x30-byte nodes
  onto a per-owner list (head `owner+0x58`, count `owner+0x54`); the consumer
  that turns a queued cue string into a voice has not been found.
- **The other four dirty bits**, and the two output paths the `0x80` bit picks
  between.
- **EU cross-verification.** Nothing on this page has been checked against
  `psp-pulse-eu`, which is normally this project's primary target.
- **Nothing here is runtime-verified.** Every claim is static reading.
