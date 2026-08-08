# PSP sound bank

**Status: partial.** The container, the `SBlk` descriptor header and the audio
codec are decoded, implemented in
[`oag-formats::sblk`](../../crates/formats/src/sblk.rs) and validated across all
39 banks on the PSP disc. **Where each individual sound starts is now solved
too** - see "Not determined" below for the rule - but it is read out of the
executable rather than implemented here, so the status stays `partial` until
`sblk` splits a bank into its sounds and that is validated against all 39.

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

Confidence: **94** for the container and the `SBlk` header. **92** for
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

## Not determined

- ~~**Where each sound starts.**~~ **Solved, from the executable.** A cue's
  command list is walked, and every command whose opcode is `0x01` or `0x09`
  binds a waveform:

  ```text
  descriptor = parameter_block_offset + (command_word & 0xffffff)
  waveform offset = descriptor + 0x10
  waveform length = descriptor + 0x14
  ```

  The operand is a byte offset from the **parameter block** (header `+0x34`) to
  a 24-byte descriptor whose last two words are the offset and length. This
  page's own `frontend.bnk` observation is the confirmation: ten opcode-`0x01`
  commands with operands 0, 24, 48 ... 216 landing on ten 24-byte records whose
  last two words tile the waveform section exactly. Those operands are multiples
  of 24 because that bank's descriptors happen to be contiguous, which is what
  let a tiling search find them; **the rule does not require contiguity, which is
  why the same search failed on the race banks.** The conclusion that "the same
  reading does not generalise" was right about the search and wrong about the
  structure. Confidence 90. **Not yet implemented in
  [`sblk`](../../crates/formats/src/sblk.rs) or run across all 39 banks** - that
  is what would turn a documented rule into a validated one. See
  [the sound engine](../ghidra/functions/psp-pulse-usa/sound.md#solved-where-each-sound-starts).

- **The command opcodes.** Nine distinct values seen in the data; the engine
  defines **45**, dispatched through a jump table at `0x08ac326c`. Two are now
  traced - `0x01` and `0x09` both bind a waveform, above - and eight more share
  a single handler, which is the shape of a family taking an index. The other 35
  are unread.
- **`+0x24` = 20544.** Still not determined; the shape of it suggests an
  audio-RAM base address.
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
