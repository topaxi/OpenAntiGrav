# PSP sound bank

**Status: understood.** The container, the `SBlk` descriptor header, the audio
codec, **where each individual sound starts** and **what each sound is called**
are all decoded, implemented in
[`oag-formats::sblk`](../../crates/formats/src/sblk.rs) and validated across all
39 banks on the PSP disc. What remains open is about *playing* a bank rather
than reading one: 43 of the 45 command opcodes, and the sample rate each
waveform runs at.

These are the `03000000` blobs from the [WAD](wad.md) census - 3 in `FE.wad`,
36 in `Data.wad`. The known name `Data\Sound\frontend.bnk` is one of them.

## Container

```text
+0x00  u32  version         3 in every bank
+0x04  u32  section_count   2 in every bank
+0x08  section[section_count], 8 bytes each: { u32 offset, u32 size }
```

Section 0 is the `SBlk` descriptor block, section 1 the waveform data. The
framing has no slack anywhere: section 0 starts immediately after the table,
section 1 starts where section 0 stops, and section 1 ends exactly at the end of
the blob. All three hold on all 39 banks.

## The `SBlk` header

```text
+0x00  char[4]  "SBlk"
+0x04  u32      version, 3
+0x08  u32      772, or 260 when the voice-state block is absent
+0x0c  u32      zero
+0x10  u32      zero
+0x14  u16      zero
+0x16  u16      cue_count
+0x18  u16      command_count
+0x1a  u16      waveform_count
+0x1c  u32      cue table offset, always 64
+0x20  u32      command table offset
+0x24  u32      20544, the same in every bank
+0x28  u32      waveform data size
+0x2c  u32      waveform data size again
+0x30  u32      zero
+0x34  u32      parameter block offset
+0x38  u32      name block offset
+0x3c  u32      voice-state block offset, or zero
```

Five regions follow, in header order:

| Region | Offset field | Stride | Count | Holds |
| --- | --- | ---: | --- | --- |
| Cues | `+0x1c` (64) | 12 | `cue_count` | one per playable sound; the third word indexes the command table |
| Commands | `+0x20` | 8 | `command_count` | `{ u8 opcode, u24 operand, u32 argument }` |
| Parameters | `+0x34` | - | - | small integers: levels, indices, pitches |
| Name | `+0x38` | - | - | the bank's own name, then a `u16` index list |
| Voices | `+0x3c` | 16 | `cue_count` | all zero on disc; runtime state |

Both strides are exact across all 39 banks: `(command_offset - 64)` is `12 *
cue_count` every time, and `(parameter_offset - command_offset)` is `8 *
command_count` every time. The voice block, where present, is exactly `16 *
cue_count` bytes and runs to the end of the descriptor section.

`+0x24`'s 20544 never varies and is not a length or an offset into anything in
the file; the shape of it suggests an audio-RAM base address, which would be a
question for the executable rather than the data.

## The bank knows its own name

The name block opens with an **8-byte field holding up to 7 characters** and a
forced NUL. 35 of the 39 banks carry one, and they are immediately recognisable
as Wipeout Pulse content: `FRNTEND`, `HUD`, `SHIP`, `SHIP_ZM`, `SPEECH`,
`gentrak`, `elim_vo`, `zone_vo`, and one pair per track - `basilic`, `metropi`,
`moather`, `techder`, `dekonst`, `vertica`, `outpost`, `amphise`, `arcprim`,
`platinu`, `fortcle`, `talonsj`.

Four banks have a name short enough to be stored whole - three distinct names,
since `HUD` ships in both `FE.wad` and `Data.wad` - and for every one of them,
`Data\Sound\<name>.bnk` hashes to that bank's own WAD entry hash: `HUD` twice,
`SHIP`, `SPEECH`. That is the check that says the field is a name rather than a
label.

### The field is an abbreviation, not a truncation

This page previously read the field as a *truncation* of the path stem, and
treated it as a [name-mining](wad.md#the-name-hash) source: recover the full
track names and a dozen more archive entries resolve. An exhaustive search over
completions up to four characters of `[a-z0-9_]` found nothing, which was read
as the track banks living elsewhere in the tree.

**Both readings were wrong, and the executable settles it.** `psp-pulse-usa`
`BOOT.BIN` contains exactly 14 `.bnk` path strings - the complete set, confirmed
by a full string scan over the binary. Twelve are literal paths, and every one
hashes to a real archive entry holding an `SBlk` bank, on **both** discs:

| Path | String at | Hash | USA | EU | Bytes | Self-name |
| --- | --- | --- | --- | --- | --: | --- |
| `Data\Sound\frontend.bnk` | `0x08a88f2c` | `75a91641` | #856, FE #22 | #855, FE #22 | 24,416 | `FRNTEND` |
| `Data\Sound\FRONT_END_VO.bnk` | `0x08a88f44` | `a3aada5f` | #857 | #856 | 409,568 | |
| `Data\Sound\generaltrack.bnk` | `0x08a88f94` | `1cae70da` | #858, FE #24 | #857, FE #24 | 104,392 | `gentrak` |
| `Data\Sound\hud.bnk` | `0x08a88f80` | `15632e8b` | #859, FE #23 | #858, FE #23 | 128,336 | `HUD` |
| `Data\Sound\ship.bnk` | `0x08a7d0b8` | `cbd73678` | #860 | #859 | 247,348 | `SHIP` |
| `Data\Sound\ship_zone.bnk` | `0x08a7d060` | `bf4670d6` | #861 | #860 | 561,584 | `SHIP_ZM` |
| `Data\Sound\speech.bnk` | `0x08a7d11c` | `50796558` | #862 | #861 | 173,280 | `SPEECH` |
| `Data\Sound\speech_elim.bnk` | `0x08a7d100` | `5f6a9e46` | #863 | #862 | 215,740 | `elim_vo` |
| `Data\Sound\speech_results.bnk` | `0x08a88f60` | `7c778043` | #864 | #863 | 176,124 | |
| `Data\Sound\speech_zone.bnk` | `0x08a7d09c` | `e66cdc25` | #865 | #864 | 155,656 | `zone_vo` |
| `Data\Sound\weapons.bnk` | `0x08a7d0cc` | `01bec824` | #866 | #865 | 652,012 | |
| `Data\Sound\ZONE_ENV.bnk` | `0x08a7d048` | `ca7270f1` | #867 | #866 | 166,544 | |

The `String at` column is the `psp-pulse-usa` `BOOT.BIN` address, so the whole
table is one Ghidra query away from being re-derived rather than rediscovered.
The two remaining strings are the `%s` templates at `0x08a7d0e4` and
`0x08a7d07c`.

USA and EU indices are `Data.wad` entry numbers and differ by exactly one
throughout - the EU archive has one fewer entry before this run - while every
byte size is identical and the three `FE.wad` indices are unchanged. The hashes
are derived from the name and so are region-independent by construction; the
cross-disc agreement on sizes is the part that corroborates.

**No Ghidra labels were created for these addresses**, so no
[`names.tsv`](../ghidra/functions/psp-pulse-usa/names.tsv) rows are owed - the
evidence is this table. Labelling them later would bring the ADR-0005
obligation with it.

Lining the self-names up against the paths shows why the completion search could
never have worked. `FRNTEND` is not a prefix of `frontend` (that would be
`fronten`); `gentrak` is not a prefix of `generaltrack` (`general`); `SHIP_ZM`
is not a prefix of `ship_zone` (`ship_zo`); and `elim_vo` and `zone_vo` share no
prefix at all with `speech_elim` and `speech_zone`. **The field is a short
label chosen by hand, not the first seven bytes of anything.** The three that
resolved did so because their label happens to equal their stem, not because
truncation round-trips.

Tested directly, that is exactly what happens: of the self-names, only `SPEECH`,
`HUD` and `SHIP` hash to a real entry as `Data\Sound\<name>.bnk`. `FRNTEND`,
`gentrak`, `elim_vo`, `zone_vo` and `SHIP_ZM` all miss - even though the table
above proves those five banks *are* under `Data\Sound\`. The earlier inference
that a failed completion meant the bank lived elsewhere in the tree does not
hold.

Confidence **95**: each path is a literal string in the executable *and* hashes
to an archive entry whose payload is an `SBlk` bank, on both the USA and EU
discs, which are independent confirmations of the same name.

### What is still open

The remaining two of the fourteen strings are `Data\Sound\speech_%s.bnk` and
`Data\Sound\speech_zone_%s.bnk`, formatted with a language at runtime. **No
expansion resolves on the EU disc** - twelve language names were tried
(`english`, `french`, `spanish`, `german`, `italian`, `dutch`, `portuguese`,
`russian`, `japanese`, `korean`, `chinese`, `usa`) and every one missed, so
either this build ships only the unsuffixed banks or the token is not a plain
language name.

**The 24 per-circuit banks are not named by any executable string.** The full
scan returns 14 `.bnk` paths and no `Data\Sound\%s.bnk`-style template, so the
circuit banks are addressed some other way. The lead worth following is the
`soundregister="N"` attribute the track `Definition.xml` carries, already parsed
into context by `crates/game/src/catalogue.rs` with its meaning unrecorded -
name mining from the self-name field is now known to be a dead end for them.

## Where each sound starts

`Scream_OpKeyOn` is the opcode handler that hands a waveform to the hardware
synth, and its arithmetic is computable from the file alone. Walk the command
table; for every command whose opcode is `0x01` or `0x09`:

```text
descriptor      = parameter_block_offset + (first_word & 0x00ffffff)
waveform offset = *(u32 *)(descriptor + 0x10)
waveform length = *(u32 *)(descriptor + 0x14)
```

The opcode is the **high byte of the first word**; the low 24 bits are a byte
offset from the parameter block (header `+0x34`) to a 24-byte descriptor. Both
offsets in the header are indices into the **descriptor section**, and the
waveform offset is an index into the **waveform section**, biased by nothing.
That was determined empirically rather than assumed: the smallest offset in
every one of the 39 banks is 0, and the largest offset plus its length is the
section length exactly.

Implemented as [`Bank::sounds`](../../crates/formats/src/sblk.rs). The
descriptor's other fields, from the runtime: `+0x01` a note, `+0x04` an angle in
degrees, `+0x0e` a flags word whose `0x40` selects loop mode and `0x80` asserts
ADPCM. Those are exposed as raw values, not interpreted.

### What the ground-truth test proves

The rule is checked against every bank on the disc. The figures are exact, not
approximate:

| Check | Result |
| --- | --- |
| Banks | 39 |
| Commands with a key-on opcode | 916 |
| Descriptors that resolved | **916 of 916** |
| Distinct `(offset, length)` spans | 595 |
| Spans landing inside the waveform section | **595 of 595** |
| Spans a whole number of PS-ADPCM blocks at both ends | **595 of 595** |
| Distinct spans equal to the header's `waveform_count` | **39 of 39 banks** |
| Banks whose spans tile the waveform section with no gap | **39 of 39** |
| Partially overlapping or nested spans | **0** |

916 commands against 595 spans is the reuse the rule allows: a bank may bind one
waveform from several commands, and **33 of the 39** do.

Three of those are strong on their own and none of them is what the rule reads.
**`waveform_count` is a header field the arithmetic never touches**, and the
number of distinct spans matches it in all 39 banks. **Block alignment** is the
check that separates a wrong base from a bank with holes in it: a base off by
anything but a multiple of 16 breaks it everywhere, and it breaks nowhere.
**Exact tiling** is the sharpest: 595 spans partitioning 39 sections totalling
about 8.4 MB with no gap and no overlap is not something a wrong reading
produces.

### The codec's own terminator agrees

The check that settles it is one the command table knows nothing about.
PS-ADPCM carries a terminator in each block's flag byte - 1 end, 3 loop end, 5
start and end, 7 end and mute - so if these spans are where the encoder stopped,
every span carries one at its tail and none in its body.

**595 of 595 spans have their terminator on the last block or the one before
it. None has one earlier, and none is missing one.** The penultimate position is
the common case: the encoder flags the last block it wrote and appends a block
of run-out.

A terminator is one flag value in eight. A span boundary off by a single block
would drop a terminator into the middle of its neighbour, and none of the
595 does. That is the codec and the command table, two things with no knowledge
of each other, agreeing on the same 595 boundaries.

### The audio survives extraction

Each span is decoded on its own and measured with the same mean-step-over-RMS
metric the whole-section decode uses. **Mean 0.321 over 593 spans**, against
about 1.41 for white noise. That is higher than the whole-section figure of
0.220, and it should be: a section's measurement averages its quiet content in
with its loud content, while a span's does not.

35 spans - **10 distinct waveforms**, replicated across the circuit banks that
share an ambience library - measure at or above 1.0, topping out at 1.418.
Tracing them through the name table below names them: `~AMB`, `~FLYBY_DIST`,
`~HAWK`, `~MONORAIL`, `CANNONEXPLWALL`, and `outpost`'s wind. **Those are the
sounds that are supposed to be flat.** Wind and an explosion against a wall are
noise by design, and this metric cannot tell "correctly decoded noise" from
"incorrectly decoded anything" - which is why it is reported per span and
asserted on the mean rather than the worst.

## Every sound has a name

The name block (header `+0x38`) is a 64-bucket hash table:

```text
names + 0x00   char[8]    the bank's own name
names + 0x08   u32        offset of the entry array, relative to the name block
names + 0x18   u16[64]    hash buckets, indexed by hash(name)
entry + 0x00   char[16]   the sound's name
entry + 0x10   u16        the cue it resolves to
```

Two things are settled by the data. The entry-array offset is **relative to the
name block, not to the section**: it reads `0x98` in all 39 banks even though
their name blocks sit at wildly different offsets, and `0x18 + 64 * 2` is
exactly `0x98`. And the bucket count follows from the two offsets rather than
from the hash's range, which is still unread.

**The hash is not needed.** A bucket's chain is a run of the array terminated by
a NUL-named entry, so walking every bucket's chain enumerates the whole table -
which is exactly the set a lookup could find. Implemented as
[`Bank::sound_names`](../../crates/formats/src/sblk.rs). The table only exists
when bit `0x100` of the header word at `+0x08` is set; every bank on the disc
has it.

| Check | Result |
| --- | --- |
| Banks with the name-table bit clear | **0 of 39** |
| Names recovered | 607 |
| Banks where the name count equals `cue_count` | **39 of 39** |
| Banks whose names index every cue exactly once | **39 of 39** |
| Names pointing at a cue that does not exist | **0** |
| Names carrying a byte outside printable ASCII | **0** |

The second row is the one that matters, and it is stronger than the first:
equal counts with every index in range would still allow two names on one cue
and another cue unnamed. Sorted, each bank's recovered indices are exactly
`0..cue_count`, so **every cue is named exactly once**. The corroboration is
better still: **the strings the executable passes to `Sound_Play` turn up in these
tables verbatim.** `"SPEEDUPPAD"`, recovered while reading
[speed pads](../ghidra/functions/psp-pulse-usa/pads.md), resolves in `hud.bnk`.
`"~ENGINE"`, from [exhaust](../ghidra/functions/psp-pulse-usa/exhaust.md),
resolves in both `ship.bnk` and `ship_zone.bnk`. `"ABSORB"`, from
[contact response](../ghidra/functions/psp-pulse-usa/contact-response.md),
resolves in `weapons.bnk`. None of the three was known from the bank side, and a
wrong stride or a wrong base cannot manufacture a string the disassembler found
on its own.

`contact-response.md`'s `"COLLISIONS"` is in `ship.bnk` as **`".COLLISIONS"`**,
with a leading dot. SCREAM's error strings distinguish a sound from a *child*
sound, so the prefix is probably that distinction; it is not investigated, which
is why the ground-truth test checks the three that match exactly rather than
four with a rule for the fourth.

The tables read as Wipeout Pulse content throughout: `weapons.bnk` names 42 cues
including `QUAKELAUNCH`, `SHURIKENEXPL` and `~REPULSORTRAVEL`, and the circuit
banks name their ambience - `~AMB_SIREN`, `~TUN_ELEC_LIGHT`, `~startlineneon`.

Confidence **94** for both rules, the rubric's cap for a reading that makes an
arithmetic invariant come out exactly across many real files. Nothing here has
been run under an emulator.

## The waveforms are PS-ADPCM

Sony's 16-byte block: one predictor/shift byte, one flag byte, then 14 bytes
holding 28 four-bit residuals. The predictor is the high nibble of byte 0 and
selects one of five published filters; the shift is the low nibble, clamped at
12; the flag byte is one of eight defined values.

Nothing in the bank declares the codec. It is established by census over the
disc's **546,681 blocks**:

| Check | Passing | Rate |
| --- | ---: | ---: |
| Predictor 0-4 and shift 0-12 | 546,450 | 99.958% |
| Flag byte in 0-7 | 546,450 | 99.958% |

A byte drawn at random satisfies the first 5/16 of the time and the second 1/32
of the time. At half a million blocks, 99.96% is not something an unrelated
format produces. The 231 exceptions are scattered mid-stream rather than
clustered, and the hardware does not fault on an out-of-range shift either, so
the decoder clamps rather than refusing.

Waveform data is always a whole number of blocks, on all 39 banks.

### The decode is real audio, not a plausible-looking one

In-spec header bytes say the *framing* is PS-ADPCM. They say nothing about the
filter coefficients or the direction of the shift, and getting either wrong
still produces 28 samples per block - it just produces noise.

So the ground-truth test decodes every bank and measures the **mean sample step
over the RMS level**. White noise sits at about 1.41; music and speech sit far
below it. Measured across all 39 banks: **mean 0.220, worst 0.408**. Rendering
one to a waveform confirms the same thing by eye - engine loops and speech with
clear dynamics, not hash.

## Evidence summary

Every structural claim above holds on 39 of 39 banks:

- 2 sections, first starting at the section table's end, sections abutting, last
  ending exactly on the blob.
- The waveform size stated **three times** - the section table, `+0x28`, `+0x2c`
  - and agreeing every time. Three independent statements of one number is what
  says the header is being read at the right offsets rather than plausibly.
- Cue stride exactly 12, command stride exactly 8, voice stride exactly 16, each
  against its own declared count.
- `+0x1c` always 64, `+0x24` always 20544, `+0x14` and `+0x30` always zero.
- Waveform data a whole number of 16-byte blocks.
- 595 waveform spans tiling 39 waveform sections exactly, numbering each bank's
  declared `waveform_count`, every one block-aligned, every one carrying a
  PS-ADPCM terminator in its final two blocks and nowhere earlier.
- 607 sound names indexing every cue of every bank exactly once, every name
  printable, and three of them strings the disassembler found independently.

Confidence: **94** for the container and the `SBlk` header, and **94** for the
per-sound boundaries and the name table. **92** for
PS-ADPCM: the flag and predictor census is corpus-wide and overwhelming, and the
roughness measurement rules out a wrong-but-well-framed decode, but neither is
an exact identity and nothing has been run under an emulator. Per the
[rubric](../reverse-engineering/confidence-rubric.md) data agreement caps at 94
either way.

**Validated against one disc**, `pulse-psp-usa`. *Wipeout Pure*'s UMD was
checked and **holds no `SBlk` bank at all**: not one entry in any of its three
archives begins with that magic, even though its executable names
`Data\Sound\*.bnk` paths. Pure's 25 RIFF entries are `WAVE_FORMAT_EXTENSIBLE`,
two channels at 44.1 kHz. So `SBlk` looks like a Pulse-era container rather than
a lineage-wide one, and whatever indexes Pure's streams has not been looked at.
Confidence **85** on the absence, from a magic scan over all 1,229 entries. See
the [Pure probe](pure-status.md).

## Reproducing

```sh
just test-data     # runs the ground-truth test
oag-wad extract 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad' -o /tmp/fe
```

Entry `00022_75a91641` is `Data\Sound\frontend.bnk`. The ground-truth test is
[`crates/formats/tests/audio_ground_truth.rs`](../../crates/formats/tests/audio_ground_truth.rs)
and reports:

```text
banks          39
adpcm blocks   546681
in spec        546450
defined flags  546450
named          35
name matches   4 of 4 short enough to check
roughness      mean 0.220, worst 0.408 (white noise is ~1.41)
```

The per-sound rule has its own test in the same file,
`every_psp_sound_bank_splits_into_waveforms_that_tile_it`, which reports:

```text
banks             39
key-on commands   916
sounds resolved   916
distinct spans    595
banks reusing one 33 of 39
spans == wf count 39 of 39
tiles exactly     39 of 39
outside section   0
misaligned        0
overlaps          0 partial, 0 nested
no name table     0
names             607
names == cues     39 of 39
names index 0..n  39 of 39
bad cue / unprintable  0 / 0
cue strings found ["SPEEDUPPAD", "~ENGINE", "ABSORB"]
adpcm terminator  595 in the last two blocks, 0 earlier, 0 absent
span roughness    mean 0.321, worst 1.418 over 593 spans (white noise is ~1.41)
as rough as noise 35 spans, 10 distinct waveforms
```

## Not determined

- ~~**Where each sound starts.**~~ **Solved and validated** - see [above](#where-each-sound-starts).
  The rule came out of the executable; running it across all 39 banks is what
  made it a finding rather than a hypothesis. Worth recording is what the
  earlier search got wrong: `frontend.bnk`'s operands are 0, 24, 48 ... 216
  because that bank's descriptors happen to be laid out contiguously, and a
  tiling search over contiguous 24-byte records therefore found it and failed on
  the race banks. **The rule does not require contiguity.** The conclusion that
  "the same reading does not generalise" was right about the search and wrong
  about the structure.

- ~~**Per-sound names.**~~ **Solved and validated** - see [above](#every-sound-has-a-name).

- **The command opcodes.** Nine distinct values seen in the data; the engine
  defines **45**, dispatched through a jump table at `0x08ac326c`. Two are now
  traced - `0x01` and `0x09` both bind a waveform, above - and eight more share
  a single handler, which is the shape of a family taking an index. The other 35
  are unread.
- **`+0x24` = 20544.** Still not determined; the shape of it suggests an
  audio-RAM base address.
- **Which cue owns which commands.** A name resolves to a cue, and a cue's
  `+0x08` is its command list, so the last link between a name and a waveform is
  the cue record. Probed by hand while validating the above: reading `+0x08` as
  a byte offset into the descriptor section and converting it to a command index
  with `(cue[0x08] - command_table_offset) / 8`, with `+0x04` as the count,
  attributes the noisy spans to plausible cues - `~FLYBY_DIST`, `~HAWK`,
  `~MONORAIL` - but it does not resolve on every bank, and it is not
  implemented. Worth an hour, and it would make `sblk` able to report a sound by
  name rather than by span.
- ~~**`+0x08` = 772/260.**~~ **Partly settled.** The runtime gates its
  name lookup on `bank[2] & 0x100`, and 772 is `0x304` while 260 is `0x104`, so
  **bit `0x100` means "this bank carries a name table"** - a capability flag
  rather than a size. The other bits are still unread. See
  [the sound engine](../ghidra/functions/psp-pulse-usa/sound.md#the-name-table-decoded).
- **The sample rate of each waveform.** Not in the header. PS-ADPCM carries no
  rate, so it comes from a per-sound pitch value, and the 24-byte parameter
  records' first word is the obvious candidate - `0x42aa7f00`, `0x42a15000`,
  `0x42a76400` in `frontend.bnk`, which vary per sound and cluster tightly.
- ~~**Whether this is Sony's SCREAM engine.**~~ **Settled: it is.** The PSP
  executable carries nineteen `SCREAM` strings including the verbatim copyright
  line `" SCREAM PSP    (c)2006 Sony Computer Entertainment America"` and a
  reference to `snd_RegisterMainMemAllocator`, a published SCREAM API name.
  Confidence 99. One of those strings, `"THIS SYSTEM ONLY SUPPORTS ADPCM VOICE
  DATA"`, independently corroborates the codec above. See
  [the sound engine](../ghidra/functions/psp-pulse-usa/sound.md).
