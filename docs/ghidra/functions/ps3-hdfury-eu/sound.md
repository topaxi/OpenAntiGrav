# The SCREAM grain-opcode dispatch table, and guard `0x22`

2026-08-27. [`psp-audio.md`](../../../formats/psp-audio.md#a-cue-that-plays-other-cues)
records two open items on the "a cue that plays other cues" finding: locate
the handler(s) it calls "the handler was never located", and decode guard
`0x22`, the mechanism that picks which of a parent cue's children actually
plays (the severity and ship/wall split on `.COLLISIONS`). Both are done, and
finding the dispatch table's base turned up a third: opcode `0x19`, the
[still-open](../../../formats/psp-audio.md#which-of-a-cues-waveforms-sounds-is-still-open)
"which alternate sounds" question, is now decoded too. A pointer table at
`0x008c0060` was suspected as the way in and was not it - see
[Dead end](#dead-end-0x008c0060-is-not-a-dispatch-table) below.

## The real dispatch table

`0x00927614` is an array of PPC64 OPD-address entries, one per grain opcode.
What pins its base and stride is a **shape match against
[`psp-pulse-usa/sound.md`](../../../ghidra/functions/psp-pulse-usa/sound.md#the-command-list-is-a-45-entry-jump-table)'s**
own, independently-found table on a completely different CPU and binary:
indices `0x01` and `0x09` share one handler on both (`0x00626728` here,
`0x0898bc78` there); indices `0x0c`-`0x13`, eight in a row, share a second
handler on both (`0x006254c8` here, `0x0898afa8` there). An off-by-one base
would misalign both runs at once, on two unrelated binaries - a fingerprint
match, not an opcode-number coincidence. **On top of that**, index `0x08`
resolves to a function whose only error path prints `"SCREAM:
snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d\n"` - the binary's own name
for grain type `0x08`, landing on exactly the index
[`psp-audio.md`](../../../formats/psp-audio.md#a-cue-that-plays-other-cues)
predicted. Every opcode index in this page follows from the same arithmetic,
not from a further guess.

```text
scripts/ps3-toc.py u32 <0x00927614 + 4*opcode>   # -> an OPD address
scripts/ps3-toc.py u32 <that OPD address>        # -> the code address (func)
```

Every function reached this way carries TOC `0x008bd3c4` (module B, per
[memory.md](memory.md#every-function-has-its-own-toc-and-ghidra-uses-one-for-all-of-them)) -
Ghidra renders every TOC-relative load in them against the wrong base, so
every string and global reference below was checked with `scripts/ps3-toc.py`,
never trusted from `decompile_function` directly. Where Ghidra's decompile
shows a symbol like `PTR_s_R2Button_008b0170`, that name is the wrong-TOC
artifact the trap on [memory.md](memory.md) warns about - it is not a real
button-input reference, it is `0x2c98(r2)` under this function's *real* TOC,
which is a completely different global.

### Opcodes read so far

| Opcode | Address | Name | Confidence | What it does |
| --- | --- | --- | --- | --- |
| `0x01`/`0x09` | `0x00626728` | `Scream_OpKeyOn` | 90 | Bind a waveform descriptor to a voice and dispatch it to the ADPCM/PCM codec split - see [below](#0x010x09---scream_opkeyon-and-the-codec-dispatch-chain) |
| `0x05` | `0x006274b0` | `Scream_DoGrainPlayChild` | 85 | Resolve a child cue by index or name, play it with a computed volume and pan |
| `0x06` | `0x00625988` | `Scream_DoGrainStopChild` | 85 | Resolve a child cue by index or name, stop every active voice currently playing it |
| `0x08` | `0x00625fc0` | `Scream_DoGrainBranch` | 85 | `snd_SFX_GRAIN_TYPE_BRANCH` - resolve a child cue by index or name, bounds-check the index, replace this voice's own playback state with it |
| `0x19` | `0x00625e88` | `Scream_DoGrainAlternate` | 88 | Pick a random one of the next N key-ons (never repeating the previous pick), jump the program counter to it |
| `0x22` | `0x00623690` | `Scream_DoGrainGuard` | 88 | Three-way compare a named variable against an immediate; skip the next grain unless the comparison holds |
| `0x23` | `0x00623770` | `Scream_DoGrainMarker` | 82 | No-op (two instructions: `li r3,0; blr`) - the interpretation as a goto marker rests on `0x24`'s behaviour, not on anything this function does itself |
| `0x24` | `0x006255f8` | `Scream_DoGrainGoto` | 88 | Scan a marker table by id, set the skip count to jump to the match; recursion-depth-guarded at 8 |
| `0x14` | `0x006234c0` | `Scream_DoGrainNop14` | 82 | No-op (`li r3,0; blr`), corroborating [`psp-pulse-usa`'s `Scream_OpNop14`](../psp-pulse-usa/sound.md#opcode-0x14-is-a-no-op-corroborated-on-hd-2026-09-08) at the same opcode slot - found from that page's side, 2026-09-08, refuting [`track-sound-emitters.md`](../psp-pulse-usa/track-sound-emitters.md#a-second-larger-set-resolves-and-still-cannot-be-played-38-nodes-on-eight-circuits)'s prior hypothesis that `0x14` binds a waveform |
| `0x15` | `0x006234c8` | `Scream_DoGrainNop15` | 82 | No-op (`li r3,0; blr`), same shape as `0x14` - corroborating `Scream_OpNop15`. Unlike `0x14`, PSP's `0x16` is a confirmed consumer: it scans backward for a command carrying this opcode |

Confidence is capped at 84 for every grain opcode not yet corroborated
elsewhere, by the rubric's "decompilation only, consistent call sites"
band - none of this is runtime-verified. **`0x19`/`0x22`/`0x23`/`0x24` moved
past that cap on 2026-09-04**: a second binary (`psp-pulse-usa`) now
corroborates all four, independently decompiled on a different CPU with no
shared analysis - see
[Corroborated on PSP](#corroborated-on-psp-2026-09-04), which replaces the
"Not corroborated" section below. That is one of the two legs the 95-100
band needs, putting the four in 85-94, short of Established by the other
leg - a runtime trace. **`0x05`/`0x06`/`0x08` followed on 2026-09-16**, the
same way: `psp-pulse-usa`'s `Scream_OpPlayChild`, `Scream_OpStopChild` and
`Scream_OpBranch` read as the same three algorithms, with one real
difference recorded rather than smoothed - the PSP branch has **no** bounds
check on the child index, where this binary's does - see
[psp-pulse-usa/sound.md](../psp-pulse-usa/sound.md#five-more-opcodes-2026-09-16-the-ones-the-banks-actually-use).
`Scream_OpKeyOn` and its callees sit above the
single-binary cap for a different reason: they also carry an exact
arithmetic invariant across many real files, not just a decompiled reading -
see [below](#0x010x09---scream_opkeyon-and-the-codec-dispatch-chain).

### `0x08` - the located handler, `snd_SFX_GRAIN_TYPE_BRANCH`

```c
// FUN_00625fc0(voice, _, operand), lightly re-flowed from the decompile
local_40 = voice->bank;                         // param_1 + 0xb0
record = (operand & 0xffffff) + bank->base;      // bank->base at +0x34
index = *(int *)(record + 0xc);                  // the child's cue index, or -1
if (index < 0) {
    name = record + 0x10;                        // the child's name, exactly per psp-audio.md's layout
    index = lookup_by_name(0, name);
    if (index < 0) index = lookup_by_name(voice->bank, name);
    if (index < 0) { if (!quiet) printf("SCREAM: Didn't find child sound named -> %s\n", name); return -1; }
}
if (index >= bank->cue_count) {                   // *(short *)(bank + 0x16)
    printf("SCREAM: snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d\n", index);
    return -1;
}
cue = bank->cue_table[index];                     // *(int *)(bank + 0x1c), stride 0xc
// ... replaces this voice's own cue pointer, key-on fields and bank pointer with `cue`'s
```

The bounds check against `bank->cue_count` (a `short` at `+0x16`) is the
mechanism `psp-audio.md`'s corpus census already named: `weapons_det.bnk`
holds child index **65** in a **55-cue** bank, and this is exactly the branch
that would print `snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d` for it.
Evidence, checked against `scripts/ps3-toc.py` rather than Ghidra's own
resolution:

```text
$ python3 scripts/ps3-toc.py toc 0x00625fc0
0x00625fc0 toc=0x008bd3c4 exact
$ python3 scripts/ps3-toc.py resolve 0x00625fc0 0x2ca0
0x008c0064 -> 0x007cfad0 'SCREAM: snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d\n'
```

`0x2ca0(r2)` is the `lwz r3,0x2ca0(r2)` at `0x00626040`, the not-found-index
branch's format-string load. This is confidence **84**: the decompiled control
flow is unambiguous, and the function's *own* error message names the grain
type it implements - as strong as a decompilation-only reading gets without a
runtime trace or a second binary.

### `0x01`/`0x09` - `Scream_OpKeyOn`, and the codec dispatch chain

2026-09-02. `psp-audio.md` recorded HD's second waveform codec as identified by
data alone (`oag_formats::sblk::decode_pcm16`, confidence 85) with the actual
PS3 dispatch function unresolved - the error string it decoded from was
attributed to *a* function via `scripts/ps3-toc.py attrib`, but that function
was not itself read. It is now, and it closes the loop: three functions, each
one a direct PS3 analogue of an already-named PSP function.

```text
Scream_OpKeyOn (0x00626728)      opcode 0x01/0x09's handler
  -> Scream_KeyOnVoice (0x00630310)   reads the descriptor's mode word
       -> CellMs_QueueVoice (0x00633c80)  dispatches on the NOT_ADPCM_FLAG bit
```

**`Scream_OpKeyOn`** computes the waveform descriptor exactly as
`psp-pulse-usa/sound.md`'s `Scream_OpKeyOn` does - `descriptor = *(u32
*)(bank+0x34) + (command_word & 0xffffff)` - then copies the descriptor's
offset (`+0x10`), length (`+0x14`) and mode word (`+0x0e`) into a per-voice
runtime record before calling `Scream_KeyOnVoice`. The command-operand
arithmetic is byte-for-byte the PSP reading; only the runtime struct it copies
into differs, which is expected for a different console generation.

**`Scream_KeyOnVoice`** reads that per-voice record's mode word back and
extracts bit 6 (`LOOP_FLAG`) and bit 7 (`NOT_ADPCM_FLAG`) as two separate
one-bit arguments, then calls `CellMs_QueueVoice(voice, addr, size, loop,
not_adpcm)` - the exact argument order of PSP's `Sas_QueueSetVoice(voice,
addr, size, loop, only_adpcm)`.

**`CellMs_QueueVoice`** is where the split happens:

```c
// CellMs_QueueVoice(voice, addr, size, loop, not_adpcm), re-flowed
if (not_adpcm) {
    payload = addr + 0x10;              // skip a 16-byte header
    payload_size = size - 0x10;
    if (loop) {
        loop_start = payload + *(u32 *)addr * 2;       // word[0] * 2 bytes
        loop_len   = payload_size - *(u32 *)addr * 2;
    }                                    // loop_start/loop_len left zero otherwise
} else {
    puts("SCREAM: ERROR! Unknown voice type in bank - must be ADPCM or PCM"); // dead in practice - see below
    // ADPCM: walk 16-byte blocks looking for a loop-start/end-mute flag ((flags & 6) == 6)
    ...
}
cellMSStreamSetInfo(voice, payload, payload_size, loop_start, loop_len, ...);
```

This directly corroborates `decode_pcm16`, from decompiled code rather than
only from an error string and a byte-level census:

- The **16-byte header skip** (`addr + 0x10`, `size - 0x10`) is
  `oag_formats::sblk::PCM16_HEADER_LEN`, read off the same two fields
  (`descriptor+0x10`/`+0x14`) this project's own `Sound` struct already reads.
- The **`* 2`** factor is 2 bytes per sample - 16-bit PCM, matching
  `decode_pcm16`'s `i16` reads.
- Header word `0`, used here as a **loop-start sample offset**, is 0 on every
  span this project measured - consistent with `psp-audio.md`'s finding that
  word 0 is zero on every sampled span, and explaining *why* it is zero rather
  than leaving it an unexplained constant: HD's own content always loops from
  the very start.
- Header word `1` (`docs/formats/psp-audio.md`'s sample-count invariant, exact
  on all 315 loop-flagged spans) is **not read by this function at all** -
  `size` already carries the byte length from the bank's own descriptor, so a
  redundant sample count baked into the payload by whatever authored it is
  metadata this runtime path has no need for. `decode_pcm16` was already
  honest about not interpreting it; this explains why that was the right call
  rather than an unfinished one.
- **The condition really is exactly `NOT_ADPCM_FLAG`**, not some other bit:
  `Scream_KeyOnVoice` passes bit 7 of the mode word as `not_adpcm`, and that
  bit is `oag_formats::sblk::NOT_ADPCM_FLAG` by definition. The ADPCM branch
  independently corroborates the *other* codec: it walks 16-byte blocks
  looking for a flag byte matching `(flags & 6) == 6`, i.e. one of PS-ADPCM's
  loop-start (`6`) or end-and-mute (`7`) flag values, at the exact `PS-ADPCM`
  block stride this project's own `decode_adpcm` uses.
- **The `"Unknown voice type"` error is unreachable from real bank data.** The
  argument it guards is a single bit (`(mode >> 7) & 1`), so it can only ever
  be `0` or `1` - both handled paths. This is a defensive "should never
  happen" catch, not evidence of a third voice type; it is consistent with
  `psp-audio.md`'s finding that MP3/ATRAC3 (real PS3 third-codec candidates)
  are ruled out, not merely untested.

**Confidence 90** for all three functions, above `decompile_function`'s
"decompilation only" cap of 84 because it is *also* corroborated by an exact
arithmetic invariant across many real files (the 315-file header-word match,
already in `psp-audio.md`), and because the argument order, field offsets and
overall shape reproduce an already-established PSP function
(`psp-pulse-usa/sound.md`'s `Scream_OpKeyOn`/`Sas_QueueSetVoice`) almost
exactly. Not "Established" (95+): no runtime trace exists for this binary, and
there is no second PS3 disc to corroborate against - the codec itself is a
PS3-only addition with nothing on PSP or PS2 to compare it to.

`0x00633c80`'s backend is genuinely a PS3-specific one, not `sceSasCore`: the
error strings under the `puts`/`printf` calls name real Sony PS3 SDK-shaped
calls, `cellMSStreamSetInfo()` and `CellMSCoreInit`, which is why this
function is named under a `CellMs_` prefix rather than reusing PSP's `Sas_` -
different low-level backend, same role in SCREAM's own dispatch.

All three functions carry TOC `0x008bd3c4` (module B, per `scripts/ps3-toc.py
toc`), inside the per-function-TOC defect [memory.md](memory.md) warns about,
so every string above was cross-checked with `scripts/ps3-toc.py str`/`attrib`
against the address `decompile_function` showed - unlike memory.md's original
example, they **agreed** on all three functions here, string for string. That
is worth recording rather than assuming away: it means `decompile_function`'s
own rendering happened to be trustworthy for this trio specifically, not that
the defect stopped applying generally. Still cross-check by hand before citing
a string from anywhere in module B; do not take this page's agreement as
license to skip the check elsewhere.

### `0x05` - plays a child with a computed volume and pan

`FUN_006274b0` reads the **same 32-byte record** `psp-audio.md` already
documented for the exclusive `0x05`/`0x08` forms, and resolves two fields the
page had marked unread:

- `+0x00` (`psp-audio.md`: *"volume, 0..127"*) - confirmed, clamped to
  `0..0x7f` before use.
- `+0x04` (`psp-audio.md`: *"unread; 0, or a small negative number"*) - this
  function reduces it modulo `0x168` (360) after the same clamp, so it is a
  **pan angle in degrees**, exactly the reading
  [`psp-pulse-usa/sound.md`](../../../ghidra/functions/psp-pulse-usa/sound.md#the-rest-of-the-table)
  already gives the same field shape on PSP's own key-on record.
- `+0x0c`/`+0x10` - the child index-or-name pair, resolved identically to
  `0x08`, then handed with the volume and pan to `FUN_00626e78` (not read).

Both `+0x00` and `+0x04` are **not always literal values**: a byte `< 0` in
either field is an indirect reference. `FUN_006274b0` reads it as
`table[-6 - value]` where `table` is the global at `scripts/ps3-toc.py resolve
0x006274b0 0x2c74` (`0x008c0038`, a runtime pointer - its target was not read,
see [Not determined](#not-determined)). Guard `0x22` below indexes the *same*
global with a different formula (`~value`, i.e. `-1 - value`), so the two
opcodes agree on the table but not on the offset applied to reach it - an
open detail, not a contradiction; each opcode's own operand byte may simply
use a different small range.

Confidence **82**: the record layout and the two previously-unread fields are
a direct, unambiguous read; one rung below `0x08` because the variable
indirection scheme is observed but not itself resolved.

### `0x06` - stops a child rather than playing it

`FUN_00625988` resolves a child cue by the same index-or-name scheme, then
computes `cue_table + index * 0xc` (the resolved cue's own record address,
not its content) and passes that as a **key** to `FUN_0062e1c0` in a loop:

```c
do {
    found = FUN_0062e1c0(voice, key);   // unlinks a matching node from
} while (found != 0);                   // voice->active_list (+0x78, chained +0x7c)
                                         // and calls FUN_0062de08(node, 1, 0, 0)
```

`FUN_0062e1c0` searches a linked list at `voice + 0x78` for a node whose
`+0x04` field equals `key`, unlinks it, and releases it - repeated until no
more match. This is "stop every currently active instance of this child cue",
not a play command; it happens to share the by-index-or-name resolver with
`0x05` and `0x08` because all three need to turn the same 32-byte record into
a resolved cue. **This opcode is outside the `0x05`/`0x08` exclusivity
`psp-audio.md` documented** - that finding was specifically about which of the
two *play* forms a record uses, and still holds; `0x06` is a third, distinct
consumer of the same record shape.

Confidence **78**: the resolve-and-loop control flow is as clean as the other
two, but "stop" is inferred from the unlink-and-release shape rather than from
a string naming it, so it sits a rung below `0x08`.

## The pitch: HD's own scale and base rate, 2026-09-16

`psp-audio.md`'s "Neither is HD's" caveat asked what `Scream_KeyOnVoice`
(`0x00630310`) computes from a descriptor's `+0x02`/`+0x03` and what it hands
the mixer as a rate. It runs the **same three-function chain** `Scream_OpKeyOn`
does for the codec split above, in the `param_2 == 0` branch:

```text
Scream_KeyOnVoice        0x00630310   note, fine = FUN_0062e998(wf, bend=+0x48, offset=+0x4a, note=+0x18, fine=+0x19)
  Scream_ComputeVoiceNote  0x0062e998   the same bend/offset modulation as PSP's function of the same role
  Scream_VoicePitch        0x0062ec38   abs() of a negative centre, then * 0x10f4a >> 16
    Scream_NoteToPitch       0x0062ea98   the table walk, byte-identical tables
  CellMs_SetVoiceRate      0x006332e0   pitch -> Hz (* 48000/4096), validates, commits
```

**The table walk is untouched.** `Scream_NoteToPitch`'s semitone table
(`0x0091e71c`, 12 entries) and fine table (`0x0091e74c`, 128 entries), read
directly off HD's memory, are byte-for-byte `psp-pulse-usa`'s
`g_scream_semitone_table`/`g_scream_fine_table` (`0x8000`, `0x879c`, ...,
`0xf1a1` and `0x8000`, `0x800e`, ..., checked against both tables' own closed
forms in `crates/formats/src/sblk/pitch/tests.rs`). What differs is two
platform-specific constants, both read from disassembly rather than guessed:

- **`Scream_VoicePitch`'s scale is `0x10f4a`, not PSP's `0x1278b`.**
  Disassembly of `0x0062ec38`: `lis r0,0x1; ori r0,r0,0xf4a; mullw r3,r3,r0;
  rlwinm r3,r3,0x10,0x10,0x1f` - an immediate, not a memory read, so nothing
  to cross-check against the per-function-TOC defect below. `0x10f4a / 0x10000
  = 1.059875`, close to but not exactly a semitone (`2^(1/12) = 1.059463`) -
  recorded as read, same as PSP's own unexplained `0x1278b`.
- **The base rate is 48,000 Hz, not the PSP SAS core's 44,100.**
  `CellMs_SetVoiceRate` (`0x006332e0`, called from `Scream_KeyOnVoice` right
  after the pitch chain) computes `iVar3 = (int)((float)pitch *
  fRam008c02c4)` and, past a buffer-size/rate validation this project has not
  fully traced, hands `iVar3` to `_opd_FUN_0060e3d0` - the value is bounds-
  checked against `96,000` and `0x5dc00` (384,000), both sane ceilings for a
  rate in Hz and nonsensical for anything else, which is why this is read as a
  direct pitch-to-Hz conversion rather than some other unit. `fRam008c02c4` is
  the float at `0x008c02c4`, `0x413b8000` = `11.71875` = `48000 / 4096` -
  **independently confirmed** with `scripts/ps3-toc.py resolve 0x006332e0
  0x2f00`, which agrees with Ghidra's own TOC resolution
  (`0x008c02c4 -> 0x413b8000`) rather than trusting it alone, per
  [memory.md](memory.md)'s warning that this binary's TOC resolution is not
  reliable by default.

### Corpus check: the largest cluster lands exactly on 48,000 Hz

`cargo run -p oag-formats --example hd_rate_probe`, re-run with the new walk
wired into `Sound::sample_rate` (the walk is selected by the bank's own
[`ByteOrder`](../../../../crates/formats/src/byte_order.rs), `Big` on every HD
bank):

| | PSP walk (before) | HD's own walk (now) |
| --- | ---: | ---: |
| Within 0.2% of a standard rate | 5,763 of 6,548 | 5,773 of 6,548 |
| `(centre -60, fine 0)`, the largest single cluster, 1,803 descriptors | 48,051 Hz | **48,000 Hz exactly** |

The near-standard-rate *count* barely moves - both walks are close enough
that most descriptors stay on the same side of the 0.2% line - but which
descriptors are exact swaps: under the borrowed PSP arithmetic, 1,803
descriptors landed 51 Hz off a round number; under HD's own arithmetic, that
same cluster is exact and the previously-exact `fine 66` family (`(centre
-62, fine 66)`, 1,313 descriptors: 44,100 -> 44,051, 0.11% off) is not. A
wrong reading does not snap 1,803 descriptors onto a round number by
coincidence, and reproducing the pitch-scale family (`fine` clustered at
`0`/`124`/`126`) landing exactly while the note/fine-modulation family
(`fine` clustered at `65`-`69`) lands 0.1% off both read as intentional
rather than as this port choosing wrong: 0.1% is about two cents, inaudible,
and matching the real engine is the rule over a tidier-looking substitute.

### The 775 that land nowhere near a standard rate are not new

**Neither walk resolves them, and re-checking says they were never expected
to.** Grouped by `centre_fine` (`by_fine` in the probe), every fine value that
is far under HD's walk is *also* far under the PSP walk, by nearly the same
margin (both walks agree with each other to within ~0.1% on the same
descriptor, the same gap the exact clusters show) - they are not two
different failure modes, they are the same non-round tail this project
already has a precedent for tolerating: `psp-audio.md` already documents
`speech.bnk`'s 38 PSP descriptors landing at 18,002 Hz and the circuit
ambiences at 15,569 Hz, neither a standard rate and neither read as a bug.
HD's own 32-descriptor `(centre -45, fine 0)` cluster - the "32 at 114,287 Hz"
the format page flagged - resolves to 114,188 Hz under HD's own walk, a 0.09%
move, the same small shift every other descriptor gets. It is a large,
consistent transposition (about 15.7 semitones above the `-60` reference),
authored identically across 32 waveforms, which reads as a deliberately
high-pitched sound (a chime or alert, going by the count) rather than a
decode fault: both walks agree on it closely, which is corroboration, not a
discrepancy to resolve.

**PS-ADPCM and 16-bit PCM are not two different rate schemes**: 486 of 5,381
PS-ADPCM descriptors (9.0%) and 289 of 1,167 PCM16 descriptors (24.8%) are far
from a standard rate under HD's walk, both codecs represented in the far set,
so codec type does not gate which formula applies.

### Confidence: 90

A decompiled chain with every link read, immediates confirmed by direct
disassembly (`Scream_VoicePitch`'s scale) and independently re-resolved
against the per-function TOC (`CellMs_SetVoiceRate`'s base-rate float,
matching Ghidra's own resolution rather than trusting it alone), tables read
straight off memory and checked byte-for-byte against `psp-pulse-usa`'s, and
a corpus invariant (the largest cluster landing on an exact standard rate)
that was not fitted to the data. Held at 90 rather than pushed to the
established trio's precedent for the same reason that trio was capped there:
**no runtime trace exists for this binary** - there is no RPCS3 breakpoint
confirming what `CellMs_SetVoiceRate` actually hands the mixer, the way
`__sceSasSetPitch` was confirmed live on the PSP. A breakpoint at
`_opd_FUN_0060e3d0` (`0x0060e3d0`), reading its rate argument on a driven race
capture the way the PSP capture read `__sceSasSetPitch`, is what would move
this to the 95+ band; see
[the RPCS3 debugger doc](../../../reverse-engineering/rpcs3-debugger.md) for
the harness.

### Names taken

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x0062e998` | function | `Scream_ComputeVoiceNote` | 90 |
| `0x0062ea98` | function | `Scream_NoteToPitch` | 90 |
| `0x0062ec38` | function | `Scream_VoicePitch` | 90 |
| `0x006332e0` | function | `CellMs_SetVoiceRate` | 90 |

## Guard `0x22`: a three-way variable-versus-immediate skip

```c
// FUN_00623690(voice, _, operand), operand is a 4-byte record
byte var_ref = operand[1];
byte value = (var_ref < 0) ? sysvar_table[~var_ref] : voice->local_vars[var_ref];  // +0xa4
byte mode = operand[2];
byte threshold = operand[3];
// mode is bounded, not a catch-all else: values other than 0/1/2 skip nothing
// and fall straight through - re-verified against the live decompile 2026-09-04,
// which uses `mode == 1`/`mode == 2`/`mode == 0` rather than a bare else.
if (mode == 1)      { if (value != threshold) voice->pc += 1; }
else if (mode == 2) { if (value <= threshold) voice->pc += 1; }
else if (mode == 0) { if (threshold <= value) voice->pc += 1; }
return 0;
```

`voice + 0xa2` is read here as the interpreter's **program counter**, not a
separate skip-count field: [`psp-pulse-usa/sound.md`](../../../ghidra/functions/psp-pulse-usa/sound.md#the-command-list-is-a-45-entry-jump-table)
already establishes that shape on the PSP side (*"`handler + 0x4a` is the
program counter"*, a different offset for a different generation's voice
struct, same role). On a failed comparison this function adds `1` to it
directly - advancing past the next grain rather than maintaining a separate
counter of how many to skip. Two things make the reading unambiguous rather
than merely plausible:

1. **The three comparison modes are direct register compares with no
   TOC-relative loads involved** - `cmpw`/`bgt`/`blt`/`beq` against a byte
   already in a register, so none of this reading depends on the per-function
   TOC defect at all. Verified against the raw disassembly, not the
   decompile, specifically because the decompile's text for mode `0`
   (*"skip if `threshold <= value`"*) reads oddly next to the assembly's
   `bgt`-to-continue - they agree once the branch target is checked (`bgt`
   branches to the **no-skip** return, so *not*-greater falls through to the
   skip path), but it is worth flagging as the kind of place a decompile can
   mislead even without a TOC problem.
2. **The same field is written consistently by three independently-read
   functions.** `0x08`'s handler initialises it to `0xffff` (`-1`) when
   starting a new voice - a pre-increment convention, consistent with a pc
   that reaches `0` on the first fetch. `0x19` (below) advances it by a
   computed jump distance. `0x24` (below) assigns it a jump target outright.
   All three read as operations on one running position, not as increments to
   a separate counter.

This is exactly the mechanism [`psp-audio.md`](../../../formats/psp-audio.md#what-it-does-not-decide)
described as missing: *"a parent's grains are guarded by `0x22`, whose operand
is not decoded"*. It is decoded now, structurally - what remains open is
**which `var_ref` value the severity variable is, and which the ship/wall
variable is**. That is not a decompilation question; `sysvar_table`
(`0x008c0038`, resolved via `scripts/ps3-toc.py resolve 0x00623690 0x2c74`) is
a runtime pointer whose target was not read, and `voice->local_vars` is
populated by whatever set up the voice, not by anything in this function. Per
this project's rule against inventing what the data does not state, no
mapping from a `var_ref` value to "severity" or "ship vs wall" is recorded
here - reading `sysvar_table`'s contents, or catching a real collision with a
breakpoint on this function, are the two ways to get one.

**One consequence worth carrying back to the format page and the Rust side**:
`0x22`'s condition depends on live voice state (`local_vars`) and a runtime
global, neither of which a static WAD read can see. `cue_tree_sounds`
returning every reachable leaf and leaving the choice to the caller - the
"honest gap" `psp-audio.md` already describes - is not a placeholder for a
static answer this page could now supply; the real selection needs the
simulation to hand the interpreter the actual collision context. Nothing in
`crates/formats/src/sblk/child.rs` should change on the strength of this
finding alone.

### `0x23` and `0x24`: a marker and its goto

`FUN_00623770` (opcode `0x23`) is `li r3,0x0; blr` - two instructions, nothing
else. `FUN_006255f8` (opcode `0x24`) scans a table of 8-byte entries
(`*(u16*)(cue+something) + i*8`, matched against a marker id in the operand)
and, on a match, **assigns** `voice + 0xa2` (the same pc field `0x22` reads)
to `i - 1` - a jump target, not an increment. A separate path in the same
function, taken when a recursion-depth guard at the same runtime global `0x22`
reads is exceeded (`sysvar_table + 0x20`, an `int`, capped at 8), *decrements*
`voice + 0xa2` by `1` instead - a distinct, error-reporting action, not part
of the jump logic; the two should not be conflated. **Corroborated on PSP,
2026-09-04** (see [below](#corroborated-on-psp-2026-09-04)) - `Scream_OpGoto`
scans the same 8-byte-entry table, assigns the same pc field the same way on
a match, and decrements it the same way past the same recursion-depth guard,
capped at the same figure, 8.

Confidence: `0x24`'s mechanism is read as cleanly as `0x22`'s (**88**); `0x23`
being a no-op is certain, but the *name* "marker" is inferred entirely from
what `0x24` does with it, not from anything in `0x23` itself, so it is capped
lower (**82**) than the certainty of the two instructions alone would suggest.

### `0x19` - alternate selection, decoded

`FUN_00625e88` (opcode `0x19`) is the answer to the thread's other open item,
[`psp-audio.md`](../../../formats/psp-audio.md#which-of-a-cues-waveforms-sounds-is-still-open)'s
*"which command in a cue's run selects the waveform that sounds"*, spotted
only while confirming the table's base (see above) and not originally in
scope:

```c
// FUN_00625e88(voice, _, operand), first call for this voice (voice + 0xa8 == 0)
count = operand[1];                        // the key-on group size - matches the thread's reading
pick = rand() % count;
if (operand[3] == pick) {                  // avoid repeating the immediately previous pick
    pick += 1;
    if (pick == count) pick = 0;
}
operand[3] = pick;                         // cached for next time this cue plays
voice->pc += pick * operand[2];            // operand[2]: "voices per alternate" - jumps forward that many grains
voice->last_alternate_span = (count - pick - 1) * operand[2];  // +0xac
voice->pc_dirty = 1;                       // +0xa8, guards re-entry
```

On a **second** call for the same voice (`+0xa8` already set), it takes an
error path instead - a log call (`FUN_0062341a8(10, cue_id, name, pc+1, 0)`)
and returns failure, so a cue is only allowed to roll its alternate once per
play.

This confirms the thread's operand-layout reading directly: `operand[1]` is
the group count and `operand[2]` is voices-per-alternate, read here from the
same bytes the thread's `b * count` arithmetic already predicted from data
alone. It also lands cleanly on the pc reading above: the jump distance is
literally `pick * voicesPerAlternate` grains forward, which is exactly how a
"pick one of N key-ons that follow" selector would use a running program
counter. **Corroborated on PSP, 2026-09-04** (see
[below](#corroborated-on-psp-2026-09-04)).

Confidence **88**: the random-pick-with-no-immediate-repeat shape and the
pc arithmetic are both unambiguous reads and now cross-binary corroborated;
short of Established because `FUN_0062a618` (the RNG call) and the log call's
exact fields were not separately verified, and neither reading is
runtime-traced.

## Dead end: `0x008c0060` is not a dispatch table

The thread that started this page named `0x008c0060` (r2 `0x008ad4d8`) as
where to look for `snd_DoGrain`. It is not a table indexed by opcode or
anything else - it is simply where module B's linker-packed small-data area
happens to place two format-string pointers back to back
(`0x008c0060` = *"Didn't find child sound named"*, `0x008c0064` = *"invalid
sound index %d"*, both confirmed above). Scanning which functions load each
consecutive word in that region turns up **unrelated functions scattered
across a wide code range** - proof it is a sequence of independently-declared
globals, not a table walked by index. The real dispatch table
(`0x00927614`) was found the other way: by locating *where the OPD address of
a known handler is itself stored as data* (`image.word_addresses(opd_addr)`,
not the code address), which is what a real indirect-call table holds on
PPC64. Writing this down so the next session does not re-walk `0x008c0060`
expecting a table.

## Corroborated on PSP, 2026-09-04

The table's base and stride were already cross-binary-confirmed (above); the
individual `0x22`/`0x23`/`0x24`/`0x19` **handler readings** now are too.
PSP's `g_scream_opcode_table` (`0x08ac326c`, single TOC, none of PPC64's
problem) gives their PSP addresses by the same index arithmetic
(`0x0018a594`/`0x0018a658`/`0x0018a660`/`0x0018a178`), and a prior session's
`decompile_function`, `disassemble_function` and `create_function` all
refused on them - that address range read as outside whatever
`psp-pulse-usa`'s auto-analysis already covered, and `disassemble_bytes`
reported success with zero instructions decoded rather than an error.

**The blocker was the address, not the code.** Those four values are raw
`.text`-relative offsets the table stores unrelocated - the same wart
[`workflow.md`](../../workflow.md#reading-an-unrelocated-database-until-it-is-reimported)
already documents for `jal` targets and manually-found jump-table bases on
`psp-pulse-usa`, one shape further: a data table of function pointers the
auto-analyzer never walked, so nothing patched its entries to real
addresses. `real = pseudo + 0x08804000` (this binary's own image base)
resolves all four to real, already-disassemblable code -
`decompile_function` had never been *wrong* about them, it had been pointed
at addresses six megabytes short of where the functions actually are. Full
writeup, decompiled handlers and the corrected addresses:
[`psp-pulse-usa/sound.md`](../psp-pulse-usa/sound.md#four-opcodes-corroborated-against-hd-2026-09-04).

Each PSP handler is the same algorithm as this page's own reading, field for
field: `Scream_OpAlternate`'s `rand() % count` with the identical
roll-once-advance-and-wrap re-roll and per-voice re-entry guard;
`Scream_OpGuard`'s three-way compare; `Scream_OpGoto`'s marker-table scan and
recursion-depth guard capped at 8; `Scream_OpMarker`'s bare `return 0`. Two
binaries, two different CPUs (PPC64 here, Allegrex MIPS there), no shared
analysis between the sessions that found them - a fingerprint match, the same
kind the table's own base and stride rested on. Confidence for all four moved
from 80/84/78/80 to 88/88/82/88 in the table above; see the confidence
rubric's reasoning on the PSP page linked above rather than repeating it
here.

## `Music_BuildFeshipTrackPath`, the front end's second track

2026-09-08. [`hd-status.md`](../../../formats/hd-status.md#two-front-end-axes-one-of-them-unread)
records `FEship.mp3` as a second front-end track with an unread trigger; this is
that trigger's builder, not its caller. `0x0017acc0`, TOC `0x008ad4d8` (checked
with `scripts/ps3-toc.py toc`, not trusted from `decompile_function` - see the
TOC trap noted [above](#the-real-dispatch-table)):

```text
$ python3 scripts/ps3-toc.py resolve 0x0017acc0 -0x1400
0x008ac0d8 -> 0x007881c8 'Data\Music\FEMusic\FEship.mp3'
$ python3 scripts/ps3-toc.py resolve 0x0017acc0 -0x13fc
0x008ac0dc -> 0x007881e8 'Data\Music\FEMusic\FEship_surround.mp3'
```

The function copies one of the two strings byte-for-byte onto the stack -
`param_1 == 1` picks the 29-byte stereo path, anything else the 38-byte
surround one, the same stereo/surround axis every other front-end template
picks on - then forwards the buffer to `FUN_00310b30(handle, path, 1, 0, 0,
0)`, which opens the handle if a flag at `+0x44` says it needs it and always
calls a further `FUN_003034e8`, matching an open-then-play shape. Confidence
**78**: the copy and the branch are unambiguous decompilation, matching the
disc's own two `FEship*.mp3` filenames exactly, but single-binary and
uncorroborated by any second source.

**Not determined: who calls it.** `search_instructions(mnemonic=bl,
operand_pattern=17acc0)` finds zero direct call sites anywhere in the ELF, and
the address never appears as an immediate operand
(`search_instructions(operand_pattern=878db0)` - its own `.opd` descriptor -
is also empty), so it is reached only through an indirect dispatch this
session did not locate. **2026-09-15: re-run on the post-`lvlx` image and
re-confirmed, this time by the validated whole-image literal-address method
(`toolchain.md#ps3`'s "Two PS3 read errors" section), not just the two direct
searches above.** `search_byte_patterns` for the function's own `.opd`
descriptor address (`00878db0`) as a raw big-endian 4-byte literal
(`00 87 8d b0`) returns zero hits anywhere in the image - no TOC slot, table,
or object field holds this function's `.opd` entry as a *stored* 4-byte
word, which is stronger than the direct-`bl` search alone but does not rule
out every indirect route: a pointer assembled at runtime from separate
`lis`/`addi` halves (or otherwise computed rather than loaded whole) would
not show up as a single 4-byte literal either, and this pass did not check
for that shape. The method was validated first, not trusted blind: the same
`search_byte_patterns` call against `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
already-known `0x00c81a5c` target (bytes `00 c8 1a 5c`) finds it at
`0x008b71cc`, and `0x008bd3c4 - 0x61f8 = 0x008b71cc` exactly - reproducing
`Scene_PrepareFrame @ 0x003aaf8c: lwz r9,-0x61f8(r2)` with zero other hits,
the same self-check `toolchain.md` prescribes. A second search for the raw
code address (`0x0017acc0`, bytes `00 17 ac c0`) finds exactly one hit,
`0x00878db0` - its own `.opd` entry, the expected occurrence, and nowhere
else. So the negative holds post-reimport, on a method now proven to catch
what a `bl`/immediate-operand sweep alone would miss: nothing anywhere in the
image, code or data, holds a static reference to this function by either
address. The `.opd` table's physical neighbours (`FUN_006926b0`,
`FUN_0017af80`, `FUN_0017b4b8`, ...) are unrelated functions (particle-effect
timing, a refcounted-object destructor) - adjacency in `.opd` is link order,
not a call table, and is not evidence of a group.

## `ZONEBAR_TRANS`, a call site found

2026-09-15. A handover thread on HD's Zone ladder names `ZONEBAR_TRANS`, a cue
string in `env0_zone.bnk` (not `speech_zone.bnk`,
which only carries the fifteen `MR_*` names), as a candidate for the Zone
ladder's second, non-verbal class-change cue, but had no traced call site for
it. Cues in this binary are looked up by name, not by a precomputed id or
hash - see `0x08`'s `lookup_by_name(bank, name)` pattern
[above](#0x08---the-located-handler-snd_sfx_grain_type_branch) - so "the id
the ELF carries" for a cue is the literal string itself, and the question
reduces to finding what references that string.

`search_strings("ZONEBAR")` finds the literal at `0x0077dba0`, distinct from
the unrelated `"ZoneBar%d"` format string at `0x0077bb98`. `get_xrefs_to` on
`0x0077dba0` returns exactly one code reference (a data reference from a TOC
slot at `0x008a6eec` aside): `0006c9f8` inside `.opd.FUN_0006c600`
(`0006c600`-`0006cbaf`, unnamed, no static caller found in this pass either -
`get_function_callers` returns two `.opd` thunks and neither was chased
further). The decompile loads the string into a local
(`puVar7 = PTR_s_ZONEBAR_TRANS_008a6eec;`) at the top of the function, then
only uses it inside the branch gated on a distance/threshold test
(`dVar16` against `param_2+0x334`/`+0x33c`) that also increments a `0`-`14`
wrapping index at `iVar9+4` - the same wraparound shape the fifteen-rung Zone
ladder already uses elsewhere in this codebase. On that branch it calls two
things in sequence: `FUN_00310bd8(bank, subsys, "ZONEADVANCE", 0)` - a
four-argument `(bank, subsys, name, priority)` shape matching the
voice-slot-allocate-by-name pattern this page already reads for other cues -
immediately followed by `FUN_002ffa58(uVar3, uVar4, puVar7 /* "ZONEBAR_TRANS" */,
0x400, 0, 0, 0, 0)`, an eight-argument call whose own decompile is partly
unresolved (register-passed parameters Ghidra typed as `in_r8`/`in_r9`/
`in_r13` rather than a clean prototype), so its exact role - a second cue
trigger alongside `ZONEADVANCE`, versus a differently-shaped event such as a
UI/telemetry notification - is not settled from decompilation alone.

**Both `PTR_` symbol names were checked, not trusted** - this page's own
opening paragraph warns that a `PTR_s_*` name under this binary can be a
wrong-TOC artifact, so trusting `decompile_function`'s naming here without
checking would repeat exactly the mistake that warning exists to prevent.
`FUN_0006c600`'s own TOC, read from the program context register
(`AssignPs3R2FromOpd.java`'s output, the same ground truth
`toolchain.md#ps3`'s validated method reads), is `0x008ad4d8` - TOC A, the
default Ghidra assumes without the fix, so a mismatch was possible but not
guaranteed. Checked directly: `0x008ad4d8 - 0x008a6eec = 0x65ec`, and the
disassembly at the xref address itself, `0006c9f8`, is `lwz r5,-0x65ec(r2)` -
an exact match. The sibling load resolves the same way:
`0x008ad4d8 - 0x008a6ef0 = 0x65e8`, and `0006ca40` disassembles to
`lwz r5,-0x65e8(r2)`. Both TOC slots were also read directly rather than
inferred from the symbol name - `0x008a6eec` holds `0x0077dba0` (the
`"ZONEBAR_TRANS"` string address from `search_strings`, confirmed
independently), and `0x008a6ef0` holds `0x0077dbb0`, which
`inspect_memory_content` reads as the literal `"ZONEADVANCE"` (12 bytes,
null-terminated). So both loads, both pointer targets and both string
contents are confirmed at the disassembly and raw-memory level, not just
from `decompile_function`'s rendering - the `ZONEADVANCE` corroboration
below is not resting on an unchecked symbol name.

**Confidence 65, no rename** (below the 70-confidence bar for a plain name
and this pass did not chase `FUN_002ffa58` far enough to justify even a `_q`
guess): the branch shape (class-change gate, ladder increment, cue-name
literal used exactly once) and both string references are now confirmed at
the disassembly/raw-memory level, not just decompilation, but the callee
that actually consumes the `ZONEBAR_TRANS` string is not itself resolved,
and that unknown is what the score is capped on. This closes the *static*
half of the open question - a call site exists, which the page this
addendum answers had marked absent - without yet closing the semantic one.
Corroboration: the maintainer's own play observation (recorded in the
zone-ladder thread) is that a Zone class change is "a spoken class name and
a non-verbal tone at once, not one or the other" - independent evidence
that a second, non-verbal trigger exists to be found, matching this site's
shape (fired in the same branch as, and immediately beside, the confirmed
`ZONEADVANCE` cue-name call).

**Next**: decompile `FUN_002ffa58` and its callee `FUN_00679688` to determine
whether the eight-argument call is itself a cue trigger (and if so, whether
`0x400` is a bus/category flag) or a different kind of event.

## `ZONEBAR_TRANS` is a genuine second cue, not a UI/telemetry event, and its dispatch primitive is named

2026-09-15, picked at random by `/oag-handover` (narrowed to this thread and
the rendering one by the invocation's own arguments, then chosen between the
two by `shuf`). Answers the "Next" item directly above.

**`FUN_00679688` is a bare cross-TOC trampoline, four instructions, nothing
else**: `std r2,0x28(r1); addis r2,r2,1; subi r2,r2,0x114; b 0x0062c500` -
save the caller's TOC, load the callee's, tail-branch. It has four callers
(`FUN_002fdc00`, `FUN_002ffa58`, `FUN_0031c090`, `FUN_0031cb50`), none of them
Zone-specific, so it is compiler-generated glue at a TOC boundary rather than
a function this project's naming convention has anything to say about - left
unnamed, the same way this page already leaves two `.opd` thunks unchased.

**Its target, `FUN_0062c500`, is unambiguous and is renamed
`Sound_PlayNamedCue`.** Full disassembly read end to end (`0062c500`-`0062c79c`,
no gaps, no `lvlx`/`lvrx`/`stvlx`/`stvrx` - the Cell-vector holes this
binary's own trap list warns about are not present here, so nothing was
silently dropped from the decompile the way `MR_Z_SUP`'s dismissal worried
this page's own earlier passes). Structurally it is `SCREAM`'s generic
"resolve then play" entry point, in this order:

1. Resolve a bank - either handed a pointer directly (flag bit `0x20000000`
   set, which is what `FUN_002ffa58` always sets - see below) or looked up by
   name via `bl 0x0062ab60`.
2. Resolve a sound inside that bank by name via `bl 0x0062a8f0`, unless a
   literal index was already supplied (flag bit `0x40000000`).
3. Check the bank's own header word against magic `0x6b6c4253` (`lis
   r0,0x6b6c; ori r0,r0,0x4253` at `0062c55c`-`0062c560`, matching the
   decompile's `*piVar4 != 0x6b6c4253` exactly) to pick between two dispatch
   calls: the ordinary case, `bl 0x0062c338(bank, index, priority, bus, ...)` -
   a `PlaySound`-shaped six-argument call - or, on the magic match, `bl
   0x00626e78` with two bytes read out of a per-sound record at
   `bank+0x1c + index*12`.
4. On either resolution failure, print one of two `SCREAM` diagnostics,
   gated by two suppression flags read first.

**Both diagnostic strings were checked against the TOC-artifact trap this
page's own opening paragraphs warn about, not trusted from Ghidra's
`PTR_s_*` auto-naming.** `search_strings` finds `"SCREAM: Couldn't find named
bank -> %s\n"` at `0x007cfcd0` and `"SCREAM: Didn't find sound named ->
%s\n"` at `0x007cfcf8`. `Sound_PlayNamedCue`'s own TOC is `0x008bd3c4` (TOC
B, the one covering `0x32d5e0`-`0x7579c0`, which `0x0062c500` falls inside);
`0x008bd3c4 + 0x2d2c = 0x008c00f0` and `0x008bd3c4 + 0x2d30 = 0x008c00f4`,
and `inspect_memory_content` on those two TOC slots reads back `0x007cfcd0`
and `0x007cfcf8` exactly - the same two string addresses `search_strings`
found independently. Both displacements the disassembly actually uses
(`lwz r3,0x2d2c(r2)` before the bank-not-found print, `lwz r3,0x2d30(r2)`
before the sound-not-found print) resolve to the right string on the right
TOC, not a same-shaped wrong one. **Confidence 85**: full disassembly read,
no unresolved gaps, two independent TOC-slot resolutions landing on the
exact right strings, and a call shape (resolve bank, resolve sound, dispatch
to play) that matches nothing else this page has documented as well as it
matches "generic cue player."

**This settles the open question: `ZONEBAR_TRANS` is played through exactly
this primitive, so it is a real, second SCREAM cue trigger - not a
UI/telemetry notification.** The maintainer's own play observation ("a
spoken class name and a non-verbal tone at once") now has a confirmed
mechanism on both sides: `FUN_00310bd8` (unrenamed, but read as a control
this pass - an eight-slot priority-ranked voice allocator taking `(bank,
subsys, name, priority)`, matching this page's existing description of the
`ZONEADVANCE` call) for the voice line, `Sound_PlayNamedCue` for the tone.
`ZONEBAR_TRANS`'s own confidence (previously capped at 65 specifically
because "the callee that actually consumes the string is not itself
resolved") rises to **82** with that callee now read; still short of
`Sound_PlayNamedCue`'s own 85 because the *reason* `ZONEBAR_TRANS` uses this
path rather than the ordinary voice-line path - a UI cue bus, a different
priority band, something else - is still not read off the `0x400` argument
`FUN_002ffa58` hands it (see below).

**`FUN_002ffa58` itself is read enough to describe, not enough to name.**
It builds a play-request struct on its own stack (a 16-byte name/id block at
`r1+0xd0`, an optional 8-float parameter block copied in when `r21 != 0`, and
a flags word `0x20000003`/`0x20000023` whose bit 2 is exactly the
`0x20000000` bank-pointer-supplied flag `Sound_PlayNamedCue` tests - the two
functions agree on this bit by construction, not by naming coincidence), then
either calls `FUN_006766f8`/`FUN_006770a8` (read here only by shape - lock
and unlock around a global at `TOC+0x76b4`, never chased into their own
bodies) before or after handing the struct to `FUN_00679688`. That
lock/unlock pair is the reason this function stays at confidence ~65 rather
than crossing 70: the request-building half is unambiguous, the
synchronisation half is not chased. Below the 70-confidence bar for a plain
name, above 50 for a hypothesis - written down rather than renamed:
"`Sound_PlayCueRequest_q`" would be the name if the lock/unlock pair turns
out to be exactly that.

**Not settled by this pass, and worth naming so the next one does not
re-derive it**: why `ZONEBAR_TRANS` goes through `Sound_PlayNamedCue`
(bank/sound resolved by name, magic-gated dispatch) while the confirmed
`ZONEADVANCE` call beside it goes through `FUN_00310bd8`'s priority-slot
allocator instead - two different primitives fired one after another in the
same branch, and nothing read this pass says whether that is "voice line vs.
UI tone" as a general SCREAM convention or specific to this one call site.

## The "voice line vs. tone" split is a general SCREAM convention, and `FUN_00310bd8` is named

2026-09-15, later the same day. Answers the item directly above with a
caller census rather than a guess, per the same page's own "on this binary a
known string is a better handle than a known field" lesson - here the known
strings are the ones already sitting in each caller's own decompile.

`FUN_00310bd8` (`0x00310bd8`, the `ZONEADVANCE` control call earlier in this
page) has eighteen callers total (`get_function_callers`), including `Zone_UpdateCraftClass`
and `Detonator_UpdateRace` by name already. Five of the sampled callers pass a
literal, human-readable cue-name string directly as the third argument, and
every one is a discrete, state-gated announcement rather than a continuous or
parametrised effect:

- `Zone_UpdateCraftClass` -> `"ZONEADVANCE"` (already established).
- `Detonator_UpdateRace` -> `PTR_s_EMPREADY_008a6d1c` / `PTR_s_EMPFULL_008a6d20`,
  gated on the same kind of threshold-fraction test `zone-advance.md` reads for
  the Zone ladder - Detonator's EMP charge, not Zone's speed class, but the
  identical "state crosses a threshold, queue a named voice line" shape.
- `FUN_0005e948` -> `PTR_s_TOURN_COMPLETE_008a6a60` / `PTR_s_RACE_COMPLETE_008a6a64`
  / `PTR_s_SESS_COMPLETE_008a6a68`, gated on a session-progress bitmask - the
  milestone-completion announcer lines, unconnected to either ladder.
- `FUN_00067120` (itself called from the top of `Detonator_UpdateRace`) walks
  a pointer at `param_1+0x2e20` in stride-3 steps and passes `puVar16[1]`/
  `puVar16[2]` - two per-step cue-name pointers read out of an array, not
  literals - to two back-to-back `FUN_00310bd8` calls whenever the step
  index changes. That shape - an indexed walk yielding a pair of cue-name
  pointers per step - is the same family as `zone-speed-class-table.md`'s
  `g_ZoneSpeedClassTable`, and **is a concrete lead for this thread's own
  still-open "does Detonator have the same `{threshold, stringId}` shape"
  Next Step**: `param_1+0x2e20` (distinct from `Detonator_UpdateRace`'s own
  `+0x2e10` step counter) is where to point `get_xrefs_to`/`inspect_memory_content`
  next, rather than starting from `RaceManager->+0x2e10` cold. Not chased
  further this pass - it is a different Next Step's scope, not this one's.

Zero sampled callers of the `Sound_PlayNamedCue` trampoline pass a bare
string literal at all: `FUN_002fdc00` (the largest of the four, an adaptive
music/ambient intensity controller - volume ramps, a decaying `local_70`
counter, per-parameter float overrides) and its two siblings all build a
parameter struct on their own stack and hand a pointer to it through the
trampoline, with any name reference already resolved to a pointer field
inside that struct rather than passed as a literal. **The split is
structural, not incidental**: `Sound_QueueAnnouncerCue` (renamed from
`FUN_00310bd8`, confidence 85 - eighteen callers, five sampled and all
literal-named discrete announcements, matching this page's own control call
and two other subsystems' milestone/warning lines) is SCREAM's voice-line
queue - a scarce, priority-ranked resource for the small number of
simultaneous spoken lines a race can play, which is exactly why it needs
priority arbitration and `Sound_PlayNamedCue` does not. `ZONEBAR_TRANS`
going through the latter is therefore consistent with it being a tone or
effect layered under the spoken line rather than competing with it for a
voice slot - matching the maintainer's own "a spoken class name and a
non-verbal tone at once" observation exactly, on the mechanism this time
rather than only on the pairing.

## `FUN_0006c600` named `Zone_UpdateCraftClass`, and its callers found

2026-09-15, closing the "no static caller found" gap the addendum above left
open. `get_function_callers` on `FUN_0006c600` (explicit program) returns two
real functions, not thunks: `.opd.FUN_0003f6d8` and `.opd.FUN_0006d958`, and
neither has further callers of its own to chase - this is the top of the call
chain reachable statically.

**`FUN_0006d958`'s call shape is decisive.** It reads `uVar5 =
*(uint*)PTR_g_GameState_008a6e90` (the live racer count) and, when nonzero,
walks a pointer array at its own `param_2+0xe8` once per racer, calling
`FUN_0006c600(param_1 /* dt */, param_2+0x2dd8, *slot)` each time. So
`FUN_0006c600` runs **once per craft, every tick it is reached**, and its own
`param_2` is not a craft or a race manager but a **sub-object embedded inside
a larger "world" struct at offset `0x2dd8`**, and its `param_3` is the craft
pointer from the racer array.

That the embedded sub-object is the same one `FUN_0006c600` sees as its own
`param_2` (offset `0`) is not assumed, it is arithmetic that closes twice over,
reading `FUN_0006d958`'s own body against `FUN_0006c600`'s:

| Field, relative to `FUN_0006c600`'s `param_2` | `FUN_0006d958`'s absolute offset | `0x2dd8 +` |
| --- | --- | --- |
| `+0x334` (upper threshold) | `+0x310c` | `0x2dd8+0x334 = 0x310c` |
| `+0x31c` (distance denominator) | `+0x30f4` | `0x2dd8+0x31c = 0x30f4` |
| `iVar5+0x54` (per-checkpoint accumulator, `iVar5=cp*4`) | `iVar20*4+0x54` | same field, same stride |
| `iVar11*0xc+0xd0` (12-byte per-checkpoint blend record) | `uVar29*0xc+0xd0` | same field, same stride |

Four independent fields, exact arithmetic, no rounding - this is the
"arithmetic invariant" tier of evidence, not a guess at the base offset.

`FUN_0003f6d8` calls it once, unconditionally, as
`FUN_0006c600(dt, param_2+0x34f8, *(int*)(param_2+0x13e8))` - a *different*
outer struct and a *different* embedded offset, but the same shape: one
sub-object pointer plus one craft pointer, specifically the craft
`*(param_2+0x13e8)` points at (every other read of that field in the same
function is guarded by local-player checks), i.e. this caller runs the same
update for the local player's own craft outside the main per-tick sweep.

**`param_3`'s craft-identifying offset corroborates independently.**
`FUN_0006c600` reads `param_3+0x7a60` (as `iVar15`) to index two parallel
arrays. [`engine-trail.md`](engine-trail.md) already established `craft+0x7a60`
as "the owner index" (which of the fixed eight racer slots this craft is), and
[`zone-effectsettings-loader.md`](zone-effectsettings-loader.md#craftarrayn-0x640-still-no-writer-and-the-offset-sweep-is-now-genuinely-exhaustive-rather-than-blind)
independently names `+0x7a60` "the craft-identifying offset" for the same
reason - a small fixed list of eight slots (`+0xe8`..`+0x104` on the world
struct, the exact array `FUN_0006d958` walks above) matched by an id field at
`+0x7a60` on each candidate. Three pages, three unrelated readings, one
offset.

**The `0`-`14` wrapping index the earlier addendum found is a per-racer Zone
ladder rung, not a stage value read anywhere else.** It lives at
`param_2(zoneState) + (param_3->+0x7a60)*4 + 0x24` - an eight-slot array (one
per racer slot) immediately after a scalar pointer field at `zoneState+0x20`
that a HUD-facing object is written through (`+0x530`/`+0x53c`, at the very end
of the function, through a critically-damped-spring ease of a bar position
towards `zoneState+iVar5+0x2c`). That end-of-function block runs
unconditionally, every call, independent of which branch the threshold test
took - it is the on-screen Zone bar's fill animation, not the class-change
event itself.

**This settles `zone-effectsettings-loader.md`'s open "is `FUN_0006c600` the
writer" question, in the negative.** Its disassembly (both branches, all
paths) never writes offset `0x640` on anything - no `stw`/`stb`/`sth`/`std`/
`stfs` with that displacement, and no `li r_, 0x640` / `addi r_, r_, 0x640`
either. Every write it makes lands inside the `zoneState` sub-object
(`param_2`, offsets `0x24`-`0x54`, `0xd0`-`0xdc`, `0x130`-`0x13c`) or on
`param_3` itself at small offsets (`0x54`, `0x5f40`, `0x6a64`) or on
`targetObj = *(param_2+iVar5+0x134)` at `0x21c`/`0x200`/`0x4`. None of those
is `craftArray[n]->+0x640` or the global `H[e]` at `0x008c2cb8` - different
base, different index space (`H` is indexed `0`/`1` by environment, this
function's arrays are indexed by racer slot or by checkpoint id, never by
environment). So `FUN_0006c600` advances a **separate, HUD/audio-facing**
per-racer ladder counter (`ZONEADVANCE`/`ZONEBAR_TRANS`, the bar's spring
target), not the render-facing stage value `Environment_UpdateStageBlend`
reads. This is a genuine negative on a real candidate, not a restatement of
"still not found" - it removes one specific function from the search.

**Confidence 72, named `Zone_UpdateCraftClass`** (`docs/ghidra/functions/
ps3-hdfury-eu/zone-advance.md` carries the full page per ADR-0005). Above the
70-confidence bar: decompilation is unambiguous, both callers are read and
their own bodies corroborate the sub-object's field layout by exact
arithmetic (not just a plausible offset), and the craft-identifying `+0x7a60`
reading agrees with two independent prior pages. Capped in the 70-84
"decompilation only, consistent call sites" band because there is no runtime
trace and `FUN_002ffa58`'s exact role (the addendum above's open item) is
still unresolved - it is a callee's ambiguity, not this function's own, but
the score does not separate the two.

## The master tick is 240 Hz, and `c_CLEAR` is silent by construction, 2026-09-29

Wipeout HD's SCREAM counts a command's delay word in **master ticks of 240 Hz**.
Both halves were open in [psp-audio.md](../../../formats/psp-audio.md#a-cue-can-be-a-sequence-and-zone_n-is-one)
(HD's tick "not measured", `c_CLEAR` "unread") and both are settled here: the
rate from the executable and from a live count, `c_CLEAR` from the goto handler
above. The Pulse counterpart is
[psp-pulse-usa/sound.md](../psp-pulse-usa/sound.md#the-master-tick-and-the-delay-word-2026-09-29).

### The chain, found from the opcode dispatch table

The dispatch table's one reader, `lwz r4,0x2c94(r2)` at `0x006252b0` (TOC
`0x008bd3c4`, so `0x008c0058`, which holds `0x00927614`), is the command stepper;
its callers lead up to the tick in four hops. Every one is TOC-checked with
`scripts/ps3-toc.py toc` (`0x008bd3c4`, `exact`).

| Address | Name | Confidence | What it does |
| --- | --- | --- | --- |
| `0x00631a70` | `Scream_AccumulateTicks` | 85 | Once per audio-thread pass: `acc += *0x0091ea60` (a float), and while `acc > 1.0` (`0x008c023c`) calls `Scream_MasterTick` and subtracts 1.0 |
| `0x0062e518` | `Scream_MasterTick` | 90 | Locks the sound system, and while its pending count (`0x013bc5b8`) is positive: `++0x013bc500`, `Scream_TickHandlers`, a slower update every fourth tick. Same shape as [Pulse's](../psp-pulse-usa/sound.md#the-master-tick-and-the-delay-word-2026-09-29) |
| `0x0062e098` | `Scream_TickHandlers` | 82 | Walks the live handler list; a handler whose type nibble (`*h & 0x1f`) is `5` and that is not paused (`+0x22 & 2`) goes to `Scream_TickCommandList` |
| `0x006253d0` | `Scream_TickCommandList` | 88 | `handler + 0xa0 -= 1`, then `while (delay < 1 && pc != -1) Scream_StepCommandList(handler)` |
| `0x00625218` | `Scream_StepCommandList` | 88 | Runs the command at `handler + 0xa2` through the table, advances the pc, and sets `handler + 0xa0 = (s16)next.word1 + result` |

`Scream_TickCommandList` and `Scream_StepCommandList` are the delay-word reading
Pulse's page records, field for field (Pulse's `+0x48` is `+0xa0` here, `+0x4a`
is `+0xa2`): **the second word of a command is the wait before that command,
counted from the previous one, in master ticks**, and a wait of zero runs in the
same tick. Confidence 88, two binaries agreeing.

**The step is `2.56f`**: `0x0091ea60` reads `0x4023d70a`, `0x008c023c` reads
`0x3f800000` (1.0). So a pass runs two or three ticks (the accumulator was read
at 0.84, 0.12, 0.96, 0.80, 0.64 at five stops) and **the ticks of one pass run
back to back**: a cue started in a pass has its ticks quantised to the audio
pass, 10.7 ms wide, where Pulse's are 3.9 ms.

**What paces a pass.** `SoundSystem_AudioThread` (`0x00309230`) calls
`FUN_00679a18` (a stub for `Scream_AccumulateTicks`) when `g_sound_system+0xf90`
is set (read live: 1), then `FUN_006799f8` (a stub for `0x00602c20`), the
cellMS block generator whose error strings name `cellMSSystemGetNeededMemorySize`.
That function loops `usleep(800)` until the `cellAudio` read index has moved by
`iVar24` blocks, where `iVar24` is 1 when the MS grain word `*0x008bf984` is set
and the block size `*0x008bf980` is 0x80 or 0x100, and otherwise 2. Live: the
grain word is 0, so **a pass waits for two `cellAudio` blocks**. A `cellAudio`
block is 256 frames at 48 kHz (the API's own fixed size, not read from this
binary), so a pass is 512 frames = 10.667 ms, 93.75 passes a second:

```text
48000 / 512 * 2.56 = 240.0 ticks per second   (2.56f gives 239.99999)
```

### Measured live

RPCS3 `v0.0.42-19980-028d1e8f`, HD EU (`BCES-00664`), Recompiler (LLVM), audio
renderer Null, Xvfb :94, Main Menu, gdb stub. `scripts/rpcs3-hd-sound-tick.py`
resumes the emulator for 10 s of wall clock, interrupts it, and reads the tick
counter `0x013bc500` at each stop:

| Window | Ticks | Wall s (resume to stop reply) | Ticks per s |
| --- | --- | --- | --- |
| 1 | 2402 | 10.020 | 239.71 |
| 2 | 2403 | 10.021 | 239.81 |
| 3 | 2404 | 10.020 | 239.93 |
| 4 | 2404 | 10.040 | 239.44 |

The same static and live number is not a coincidence of one path: a second
caller of `Scream_MasterTick`, `FUN_006334a0` (returns 200, reached only through
its OPD `0x008a34d0`), would add ticks on top of 240 if it ran, and the count is
240 within the window's own resolution (the stop-reply latency, 20-40 ms of
10 s). Not read: what registers `FUN_006334a0`.

Confidence **90** for 240 Hz at HD's EU build: the static derivation (step,
threshold, two-block pass) and four live windows agree to 0.2 %. The wall clock
includes the pause round trip, so the live figure is a slight under-read by
construction. Not measured: HD's US build (same code is expected, unread), a
race rather than the front end (the audio thread is the same one), the phase of
a cue's start against the tick.

### `c_CLEAR` is a goto and its own marker

`Scream_DoGrainGoto` (`0x006255f8`, decompile above) scans **the cue's own
commands** from the first for a command whose byte 0 is `#` (`0x23`) and whose
byte 1 equals the goto's own byte 1, sets `pc` to one before it and returns 0.
HD's `c_CLEAR` (cue 21 of `speech_zone.bnk`, commands 71-72) is
`0x24 operand 0`, `0x23 operand 0`: the goto lands on the marker beside it and
the list ends. **The cue is silent.** It is not a missing "clear": each
`zone_N`'s one waveform is the whole line ("zone", the number, "clear" in one
recording; 1.7-2.0 s, three loudness bursts in the rendered envelope), keyed twice at
pan angles 30 and 330 degrees, 5 ticks (20.8 ms) apart, then the silent child
280-350 ticks (1.17-1.46 s) later. Why the child is there is not established;
**hypothesis, no score**: it holds the cue alive past the line's end so the
announcer queue (`Sound_QueueAnnouncerCue`, `0x00310bd8`) does not start the next
line early.

The rate and the reading are wired: `oag_title::SequenceTick::Ps3` (240 Hz, gotos
followed), `oag_formats::sblk::timeline::WalkModel`. Seven HD sfx cues
(`shieldactive`, `autopilot_eng`, `disengaging`, `PLASMA`, `PLASMAHITWALL`,
`QUAKELAUNCH`, `LEACH`) and all fifteen `zone_N` lines now play their authored
timeline.

## Not determined

- **`sysvar_table`'s contents** (`0x008c0038`). A runtime pointer, not a
  static table - reading what it points at needs a live process, not a
  disassembly.
- **Which `var_ref` value means severity, and which means ship-vs-wall**, for
  `.COLLISIONS`' guard `0x22` grains specifically. Blocked on the item above,
  or on a breakpoint during a real collision.
- **Why `0x05`'s variable-indirect fields index `sysvar_table` at `-6 - value`
  while `0x22`'s indexes it at `-1 - value`.** Both read the same global;
  the differing offset was not chased further this session.
- **`FUN_00626e78`** (`0x05`'s and `0x08`'s callee that actually starts
  playback), **`FUN_0062de08`** (`0x06`'s voice-release callee) and
  **`FUN_0062a618`** (`0x19`'s RNG call) - none was read.
- **Opcodes `0x00`-`0x04`, `0x07`, `0x09`-`0x18`, `0x1a`-`0x21`,
  `0x25`-`0x2c`.** The table covers at least 40 entries past `0x927614`;
  only seven are read.
