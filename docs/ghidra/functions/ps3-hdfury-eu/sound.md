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
| `0x05` | `0x006274b0` | `Scream_DoGrainPlayChild` | 82 | Resolve a child cue by index or name, play it with a computed volume and pan |
| `0x06` | `0x00625988` | `Scream_DoGrainStopChild` | 78 | Resolve a child cue by index or name, stop every active voice currently playing it |
| `0x08` | `0x00625fc0` | `Scream_DoGrainBranch` | 84 | `snd_SFX_GRAIN_TYPE_BRANCH` - resolve a child cue by index or name, bounds-check the index, replace this voice's own playback state with it |
| `0x19` | `0x00625e88` | `Scream_DoGrainAlternate` | 80 | Pick a random one of the next N key-ons (never repeating the previous pick), jump the program counter to it |
| `0x22` | `0x00623690` | `Scream_DoGrainGuard` | 84 | Three-way compare a named variable against an immediate; skip the next grain unless the comparison holds |
| `0x23` | `0x00623770` | `Scream_DoGrainMarker` | 78 | No-op (two instructions: `li r3,0; blr`) - the interpretation as a goto marker rests on `0x24`'s behaviour, not on anything this function does itself |
| `0x24` | `0x006255f8` | `Scream_DoGrainGoto` | 80 | Scan a marker table by id, set the skip count to jump to the match; recursion-depth-guarded at 8 |

Confidence is capped at 84 for every grain opcode by the rubric's
"decompilation only, consistent call sites" band - none of this is
runtime-verified, and no second binary corroborates it yet (see
[Not corroborated on PSP](#not-corroborated-on-psp-yet)). `Scream_OpKeyOn`
and its callees sit above that cap: they also carry an exact arithmetic
invariant across many real files, not just a decompiled reading - see
[below](#0x010x09---scream_opkeyon-and-the-codec-dispatch-chain).

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

## Guard `0x22`: a three-way variable-versus-immediate skip

```c
// FUN_00623690(voice, _, operand), operand is a 4-byte record
byte var_ref = operand[1];
byte value = (var_ref < 0) ? sysvar_table[~var_ref] : voice->local_vars[var_ref];  // +0xa4
byte mode = operand[2];
byte threshold = operand[3];
bool keep_going;
if (mode == 1)      keep_going = (value == threshold);
else if (mode == 2) keep_going = (value >  threshold);
else                keep_going = (value <  threshold);   // mode 0
if (!keep_going) voice->pc += 1;                          // +0xa2, a `short`
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
of the jump logic; the two should not be conflated. This matches
[`psp-pulse-usa/sound.md`](../../../ghidra/functions/psp-pulse-usa/sound.md#not-determined)'s
own hypothesis, *"`0x23` looks like a marker and `0x24` like a goto"*, from
the PSP side of the same engine - independent, if not yet corroborating,
since the PSP handlers themselves were not reachable this session (see below).

Confidence: `0x24`'s mechanism is read as cleanly as `0x22`'s (**80**); `0x23`
being a no-op is certain, but the *name* "marker" is inferred entirely from
what `0x24` does with it, not from anything in `0x23` itself, so it is capped
lower (**78**) than the certainty of the two instructions alone would suggest.

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
counter. Not corroborated on PSP this session (see below).

Confidence **80**: the random-pick-with-no-immediate-repeat shape and the
pc arithmetic are both unambiguous reads; capped in the probable band because
`FUN_0062a618` (the RNG call) and the log call's exact fields were not
separately verified.

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

## Not corroborated on PSP yet

The table's base and stride are cross-binary-confirmed (above), but the
individual `0x22`/`0x23`/`0x24`/`0x19` **handler readings** are not: PSP's
`g_scream_opcode_table` (`0x08ac326c`, single TOC, none of PPC64's problem)
gives their PSP addresses directly by the same index arithmetic
(`0x0018a594`/`0x0018a658`/`0x0018a660`/`0x0018a178`), but
`decompile_function`, `disassemble_function` and `create_function` all
refused on them this session - that address range is outside whatever
`psp-pulse-usa`'s auto-analysis already covered, and `disassemble_bytes`
reported success with zero instructions decoded rather than an error. Not
chased further; a fresh analysis pass (or `create_function` after first fixing
whatever blocks it) is what a second-binary corroboration of the four
handler readings on this page needs.

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
