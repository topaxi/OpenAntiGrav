# PSP sound bank

**Status: understood.** The container, the `SBlk` descriptor header, the audio
codec, **where each individual sound starts** and **what each sound is called**
are all decoded, implemented in
[`oag-formats::sblk`](../../crates/formats/src/sblk.rs) and validated across all
39 banks on the PSP disc. What remains open is about *playing* a bank rather
than reading one: 41 of the 45 command opcodes, and the sample rate each
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

### `speech_zone.bnk` names the zone announcer, one ladder per title

`zone_vo` (`e66cdc25`) sat in the bank table above with a path and a byte count
and no cue list. Read with `oag-wad sounds`, it turns out to be the Zone
milestone announcer, and each title's own copy is different disc data rather
than one shared table:

| Title | Numbered cues | Extra cues |
| --- | --- | --- |
| Pulse (USA and EU, `Data.wad`) | `zone_5`, `10`, `15`, `20`, `25`, `30`, `40`, `50`, `60`, `70`, `80`, `90`, `100` | - |
| Pure (USA and EU, `Data.wad`) | `zone_5`, `10`, `15`, `20`, `25`, `30`, `40`, `50`, `75`, `100` | `zone_bronze`/`silver`/`gold`, `bronze_med`/`silver_med`/`gold_med`, `ship_destroyed`, `ready`, `go` |
| Wipeout HD/Fury (`DATA01.PSARC`) | `zone_5`, `10`, `15`, `20`, `25`, `30`, `35`, `40`, `45`, `50`, `60`, `70`, `80`, `90`, `100` | `ready`, `321_GO`, `go`, `RS_1_READY`, `RS_2_GO`, `energycritical`, `c_CLEAR`, `PERFECT_LAP`, `NEW_LAP_REC`, `ZONEMALE`, fourteen `MR_*` speed-class names (`MR_SVE`, `MR_VEN`, `MR_SFL`, `MR_FLA`, `MR_SRA`, `MR_RAP`, `MR_SPH`, `MR_PHA`, `MR_SUP`, `MR_ZEN`, `MR_SUZ`, `MR_Z_SUB`, `MR_Z_M1`, `MR_Z_SUP`), `HBEAT`/`HBEAT_GO` |

Every numbered cue reads as two waveforms on Pulse and HD (an alternate take,
the same shape [`Bank::pick`](../../crates/game/src/audio/sfx.rs) already draws
between elsewhere) and one on Pure. `Data\Sound\speech_zone.bnk` hashes to
`e66cdc25` on all three - confirmed by name (`oag-wad hash 'Data\Sound\speech_zone.bnk'`
against the archive's own directory), not by assuming Pulse's spelling carries
over.

**What ties a number to a moment in the run is read on Pulse only, and even
there it stops short of the executable.** `Zone_Update` (`0x0882f5cc`) walks a
milestone table, `g_zone_milestones`, whose thresholds - read live at
`0x08ab0bc8` - are `6, 11, 16, 21, 26, 31, 41, 51, 61, 71, 81, 91, 101`: the
zone number *after* the step that just happened, one more than the numbered
cue's own name (`zone_5` fires on reaching zone `6`, i.e. five zones survived).
Thirteen thresholds, thirteen numbered cues, same ascending order on both
sides - see
[`zone-mode.md`](../ghidra/functions/psp-pulse-usa/zone-mode.md#the-ten-second-step)
for the call site and why the dispatch between them resolves no further: the
function the table's second word feeds sits outside this project's Ghidra
database entirely. Pure and HD's own executables have not been read at all for
this, so their ladders are attributed by the same "ascending thresholds, one
title fact" pattern [`oag_title::ZoneCraft`] and [`oag_title::ZoneCircuit`]
already carry, not by a second Zone_Update.

**HD's speed-class cues are a second, independent ladder, now wired.** `MR_VEN`,
`MR_FLA`, `MR_RAP`, `MR_PHA` and friends read as Venom/Flash/Rapier/Phantom-
shaped speed-class names, matching HD's Zone HUD's `SpeedClass` and
`NextSpeedClass` text widgets (`docs/ui/hud.md`). **2026-08-31: HD ships them
twice.** Extracted and parsed directly (`oag_formats::sblk`, not assumed): a
**dedicated** bank, `Data\Sound\speech_class.bnk`, names `ZONEMALE` at cue `0`
plus fourteen consecutive cues at `1`-`14` - `MR_SVE`, `MR_VEN`, `MR_SFL`,
`MR_FLA`, `MR_SRA`, `MR_RAP`, `MR_SPH`, `MR_PHA`, `MR_SUP`, `MR_ZEN`, `MR_SUZ`,
`MR_Z_SUB`, `MR_Z_M1`, `MR_Z_SUP` - nothing else. The same fourteen, in the
same order, are also folded into the general `speech_zone.bnk` at cue indices
`26`-`39`. That is a 14/14 order match against [`oag_title::ZoneStages`]'
(`crates/title/src/race.rs`) own fourteen non-`Start` HD stage names, and it is
what `oag_title::ZoneClassAnnouncer` and
[`crate::audio::sfx::ClassAnnouncer`](../../crates/game/src/audio/sfx/announcer.rs)
now play - fired on the same `ZoneStages::stage_for` edge the HUD text and the
colour grade already key off, since no call site in HD's own executable has
been read for this ladder either. See `crates/hd/tests/hd_title_ground_truth.rs`'s
`the_speed_class_announcer_names_the_disc_s_own_dedicated_bank_by_cue_index` for
the ground truth.

**All fourteen decode.** Checked by playing the bank, not just parsing its
directory. `MR_SUZ` ("Super Zen") used to be the one miss: both of its own
waveforms land in [the second codec](#a-third-of-hds-waveforms-are-not-ps-adpcm-and-are-16-bit-pcm),
with no PS-ADPCM alternate to fall back to - unlike `MR_Z_SUB`/`MR_Z_M1`/`MR_Z_SUP`,
which each carry one PS-ADPCM take alongside two in the other codec and so
played regardless. Since `decode_pcm16` identified that codec, `MR_SUZ` plays
too, and `crates/game/tests/sfx_ground_truth.rs`'s
`wipeout_hd_s_speed_class_announcer_decodes_all_fourteen_cues` pins the exact
set so a codec regression shows up as a changed list rather than an unnoticed
drift.

**The two ladders can overlap, and nothing arbitrates it.** `ZONE_STAGES`
steps at zones 2, 3, 5, 7, 12, 16, 20, 27, 35, 42, 50, 60, 75;
`ZONE_ANNOUNCER.milestones` are 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 60, 70,
80, 90, 100. Five zone numbers - 5, 20, 35, 50, 60 - are on both lists, so at
each one this port raises a milestone announcement and a class announcement on
the same tick, both `Bus::Speech`, both `gain: 1.0`, both mixed in the same
`Audio::race_tick` call - "Zone 5" and "Flash" (or whichever class lands
there) playing over each other. Whether the original does the same, ducks one
under the other, or the two ladders' triggers never coincide there for a
reason unrecovered on this axis, is unread; ruled out is that the tables
themselves disagree - they were both read straight off the disc. A maintainer
who plays past one of the five zones above can settle which case this is.

**A candidate for the non-verbal half of a class change, found the same day
and deliberately left unwired - and the maintainer's own play now
corroborates that it exists.** `env0_zone.bnk` (HD's Zone environment sound
bank, not `speech_zone.bnk`) names a cue `ZONEBAR_TRANS` - plausibly "Zone bar
transition," the HUD ladder widget's own animation - but nothing traces a call
site for it, and a single unread label is not the fourteen-way order match the
`MR_*` ladder has. **Asked directly, the maintainer describes the class-change
sound as a spoken class name and a non-verbal tone together, not one or the
other** - independent evidence that a second, non-verbal cue really is there
to find, strengthening the case for chasing `ZONEBAR_TRANS`'s trigger next
rather than treating it as a name-only guess. **This entry previously also
listed `HBEAT_ZCHANGE` as a
third cue beside `HBEAT`/`HBEAT_GO`; that was checked directly this session
and is wrong - `speech_zone.bnk` names exactly 42 cues (`cue_count`, read off
the bank's own header), `0`-`41`, and no `HBEAT_ZCHANGE` string appears
anywhere in the file.** Corrected here rather than left to be re-discovered
wrong a second time.

Ported as [`crate::audio::sfx::Announcer`](../../crates/game/src/audio/sfx/announcer.rs):
one bank loaded per race, the numbered cues decoded by name, and a cue fires
when `RaceState::zone` reaches a threshold the loaded title names one for. A
title with no announcer entry plays nothing rather than guessing at Pulse's
ladder - **2026-08-28: Wipeout 2048 is no longer that title.** Its own
`speech_zone_NGP.bnk` and a fifteen-entry ladder matching HD's were read off
the executable's own control flow (a real decompiled dispatch, not a name
probe) rather than off the shipped audio - this title's `data.psarc` is still
not extracted in this tree, so unlike the other three the bank's actual
contents have not been checked against the reading. See
[zone-audio.md](../ghidra/functions/vita-2048-eu-v104/zone-audio.md).

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
degrees, `+0x0e` a flags word whose `0x40` selects loop mode and `0x80` marks a
waveform that is **not** PS-ADPCM. Those are exposed as raw values, not
interpreted; see [the `0x80` polarity](#0x80-means-not-adpcm-and-this-page-had-it-backwards).

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

## A cue owns a run of the command table

The name table gives a cue index and [`Bank::sounds`](#where-each-sound-starts)
gives the bank's waveforms, but until 2026-08-23 nothing joined the two: a name
resolved to a cue and a cue's `+0x08` was only "the third word indexes the
command table". The join is one division:

```text
first command = *(u32 *)(cue + 0x08) / 8
command count =  *(u8 *)(cue + 0x04)
```

`+0x08` is a **byte offset into the command table, biased by nothing**;
`COMMAND_LEN` is 8, so the division is the whole of it. `Scream_StepCommandList`
(`0x0898efd8`) steps the same field by 8 per command and ends the list when the
program counter passes `*(i8 *)(cue + 4) - 1`, which is where the count comes
from - SCREAM fixes the offset up to a pointer at load time, so the runtime
never shows the file's own form.

Implemented as [`Bank::cue`, `Bank::cue_named` and
`Bank::cue_sounds`](../../crates/formats/src/sblk/cue.rs).

### The base was chosen by elimination, not by intuition

An earlier probe read `+0x08` as an offset into the *descriptor section* and
subtracted the command table's own offset. It resolved on some banks and not
others, and this page recorded it as "does not resolve on every bank". Rather
than refine that reading, seven candidate bases were run against every bank on
both discs:

| Reading of `cue + 0x08` | Banks where every cue lands in the table |
| --- | ---: |
| raw index | 0 of 83 |
| **`v / 8`** | **83 of 83** |
| `(v - command_offset) / 8` (the earlier probe) | 0 of 83 |
| `(v - parameter_offset) / 8` | 0 of 83 |
| `(v & 0xffffff) / 8` | 83 of 83 |
| `v & 0xffffff` | 0 of 83 |
| `v - command_offset` | 0 of 83 |

The two survivors are the same rule: no playable cue has anything in `+0x08`'s
top byte, so the mask changes nothing. Five of the seven are not close - they
place fewer than 140 of 1,282 cues inside the table.

### The runs tile the command table exactly

That is the evidence, and it is the same shape as the waveform-span finding:

| Check | Result |
| --- | --- |
| Banks (39 PSP + 44 PS2) | 83 |
| Playable cues | 1,282 |
| Cues landing inside the command table | **1,282 of 1,282** |
| Banks whose cue runs **tile** the table with no gap or overlap | **83 of 83** |
| Cues the runtime refuses to play (`+0x04` zero) | 5 |
| Distinct `+0x08` values among those five | **1**, `0xfffffff8` |

Nothing in the format declares that a bank's cues partition its command table,
and a wrong base cannot make it come out: the runs would overlap or leave holes.

The five exclusions are not a threshold chosen to make this pass. They are
`Scream_StartSound`'s own gate - its first check after the bounds test is that
`cue + 0x04` is non-zero - and every one of them stores the same sentinel
where a real offset would be.

### The names agree with the descriptor flags

A second check, from a direction the arithmetic knows nothing about. The name
table and the waveform descriptor's `+0x0e` flag word are written by different
parts of whatever built these banks and neither refers to the other, so if the
cue-to-command join is right, the two should line up:

| | Waveforms reached | Of those, looping |
| --- | ---: | ---: |
| Cues named with a leading `~` | 1,101 | **616** |
| Cues named without one | 800 | **6** |

A plainly named cue essentially never loops - 0.75% against 56%. A wrong join
would scatter the flag at the same rate on both rows.

It is deliberately **not** claimed as a biconditional. `~` means *hand the
caller a voice handle* (`Sound_Play`'s out-parameter, see
[ADR-0018](../architecture/adr/0018-audio-mixer-architecture.md)), and a
one-shot like `~SPARKS` wants a handle too, so `~` does not imply looping. The
one-way implication is what holds and what is asserted.

Named cues resolving to no waveform at all: **247 of 1,287**. Those run some
part of the 41 unread opcodes instead of a key-on, which is an open question
rather than a failure of the rule - except where the cue plays *another cue*,
which is [decoded below](#a-cue-that-plays-other-cues).

Confidence **94**, the rubric's cap for a reading that makes an arithmetic
invariant come out exactly across many real files - reached from an exact
tiling on 83 banks and corroborated by a flag word the rule never touches.

### Which of a cue's waveforms sounds is still open

**Decoded on HD, 2026-08-27.** The heading is kept as-is, matching this page's
own convention for a section a later finding resolves.

`ship.bnk`'s `.COLLISIONS` binds **fifteen** waveforms, `ship_zone.bnk`'s ten,
and `SHIP_ZM`'s `~ENGINE` nine. Something chooses, and it is one of the 41
unread opcodes.

The candidate is `0x19`, which sits immediately before a run of key-ons and
**counts them**. Its three operand bytes read `(a, b, c)` from the top of the
normalised word down:

| Reading | Where the count is | What `b` is |
| --- | --- | --- |
| Pulse, Pure (PSP/PS2 SCREAM) | `c`, the low byte | 1, or 2 for a stereo pair |
| Wipeout HD (PS3 SCREAM) | `a`, the high byte | 1 mono, 2 stereo |

`b * count` predicts the run length in **530 of the 576** occurrences that have
a run at all, across all six discs - `00 01 0f` before fifteen key-ons,
`00 02 0a` before twenty, HD's `10 01 00` before sixteen and `04 02 00` before
eight. A further 29 occurrences have no key-on after them at all, which is the
`0x19`-sits-after-its-key-ons case and is counted separately rather than
folded in.

**This was a lead, and it is a finding now.** `0x19`'s handler is read on HD's
binary - [ps3-hdfury-eu/sound.md](../ghidra/functions/ps3-hdfury-eu/sound.md#0x19---alternate-selection-decoded) -
and it chooses **randomly among the `count` key-ons that follow, never
repeating the same pick two plays in a row**, then jumps the interpreter's
program counter forward by `pick * b` grains to land on it. The operand byte
layout above is confirmed directly rather than inferred from run-length
arithmetic: `a` (HD) is read as the group count and `b` as voices-per-
alternate in the decompiled handler itself. One detail beyond what the static
byte layout could show: HD's handler **writes the chosen pick back into the
operand's own third byte** as a per-cue "last alternate" cache, so a cue's
command data is mutated at runtime, not read-only once loaded - worth knowing
before assuming any SBLK command byte reflects only what shipped on disc.
**Corroborated on PSP, 2026-09-04**: `Scream_OpAlternate`
(`docs/ghidra/functions/psp-pulse-usa/sound.md#four-opcodes-corroborated-against-hd-2026-09-04`)
reads field for field the same algorithm, independently decompiled on a
different CPU with no shared analysis between the two sessions that found
them.

Where the count sits at opposite ends of the operand on the two platforms is
likely endianness rather than two designs, though this is a lead and not
traced through: PSP's opcode byte is the word's most-significant byte
(`*(u8 *)(cmd + 3)` on a little-endian word) and count sits at `cmd + 0`, the
byte *furthest* from the opcode; if HD's opcode byte is the equivalent
most-significant byte of a big-endian word at `cmd + 0`, its count at
`operand[1]` (`cmd + 1`) is the byte *closest* to the opcode - the same
relative position, read through opposite byte orders, not confirmed by
tracing HD's own opcode-byte offset this session.

An earlier version of this section reported **61 of 87** for a weaker form of
the same rule. That number came from a scan whose run-length walk did not stop
at the cue boundary, so a cue's last group absorbed the next cue's key-ons; the
walk is clipped now.

What the *data* says regardless of the opcode is that these are **alternates,
not layers**: `.COLLISIONS`'s fifteen samples all fall between 0.20 s and
0.35 s, which is fifteen recordings of one event. `oag_game::audio::sfx` plays
one of them, chosen by its own generator, and says so - **`Banks::pick`
matches "random, never repeats the immediately previous pick" as of
2026-09-04** (`crates/game/src/audio/sfx/banks.rs`), the same
roll-once-advance-and-wrap shape the decoded opcode uses rather than a naive
reject-and-retry.

## A cue that plays other cues

Some cues bind no waveform of their own. Their grains **play other cues**,
which bind the waveforms - so `cue_sounds` comes back empty for a cue that is
perfectly well formed and audibly does something on the original hardware.

Two opcodes do it, `0x05` and `0x08`. Both carry a 24-bit operand that is an
offset from the parameter block, exactly as a key-on's is, and both land on a
**32-byte** record:

```text
+0x00  u32       volume, 0..127
+0x04  i32       unread; 0, or a small negative number
+0x08  u32       unread; zero in every record seen
+0x0c  u32       the child's cue index, or 0xffffffff
+0x10  char[16]  the child's name, empty when +0x0c is an index
```

**The two forms are exclusive**, and Wipeout HD's own `EBOOT.elf` - which
ships SCREAM with its debug strings intact - names one message per form:

| Address | String |
| --- | --- |
| `0x007cfaa0` | `SCREAM: Didn't find child sound named -> %s\n` |
| `0x007cfad0` | `SCREAM: snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d\n` |

Those strings are also where the vocabulary on this page comes from: SCREAM
calls a command a **grain**, and `0x007d0bf0`..`0x007d0e40` carry
`snd_DoGrain` errors naming "Loop Start", "Loop End", "Loop Continue", "Goto
Marker", "Goto Random Marker" and "Random/Cycle Play blocks" - a fair map of
what the 41 still-unread opcodes are for.

### The evidence

Across all 230 bank entries on the five PSP/PS2 discs and Wipeout HD, of
**1,461** child grains:

| Form | Count | Where |
| --- | --- | --- |
| In-range cue index, name field empty | 1,153 | every PSP and PS2 grain |
| `0xffffffff` and a name the bank holds | 300 | Wipeout HD only |
| `0xffffffff` and a name in *another* bank | 1 | `env0_det.bnk` asks for `.COLLISIONS` |
| Neither | 7 | see below |

**Not one record carries both an index and a name.** That exclusivity is the
assertion that says the two fields are what they are taken to be: a wrong
stride or a wrong offset does not produce 1,153 in-range indices *and* 300
exact name-table hits *and* zero overlaps.

The seven that are neither are worth keeping rather than smoothing away. Six
grains in `speech_results.bnk` set `0xffffffff` with no name at all; one in
`weapons_det.bnk` holds index **65** in a **55-cue** bank - which is precisely
what `snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d` exists to print. The
data contains the error the binary complains about.

Confidence **85**: two independent encodings of the same relation, an exact
exclusivity across the corpus, and both forms named by strings in the
executable.

**2026-08-27: the handler is located, in HD's `EBOOT.elf`, and the two forms
do differ.** `0x05` (`Scream_DoGrainPlayChild`) resolves the child and plays
it with a computed volume and pan; `0x08` (`Scream_DoGrainBranch`, named by
its own `snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d` error string)
bounds-checks the index against the bank's cue count - exactly the check the
`weapons_det.bnk` index-65-in-55-cues record above would fail - and replaces
this voice's own playback state with the resolved cue outright. See
[ps3-hdfury-eu/sound.md](../ghidra/functions/ps3-hdfury-eu/sound.md#0x08---the-located-handler-snd_sfx_grain_type_branch).
Still capped at 85: the read is against HD's binary, not PSP's or PS2's, so
this is one binary's decompilation, not a second encoding of *this* engine's
own PSP/PS2 build.

### What it does not decide

**Which child plays.** A parent's grains are guarded by `0x22`. Its opcode is
now decoded too - see
[ps3-hdfury-eu/sound.md](../ghidra/functions/ps3-hdfury-eu/sound.md#guard-0x22-a-three-way-variable-versus-immediate-skip):
a three-way compare of a named interpreter variable against an immediate,
skipping the grain unless it holds. What is *not* decoded is which variable
means severity and which means ship-vs-wall, and that variable's value is
runtime state a static WAD read cannot see regardless -
`oag_formats::sblk::Bank::cue_tree_sounds` returning every reachable leaf and
leaving the caller to choose is therefore not a placeholder for a static
answer; it is the shape the format actually needs. The same honest gap as
[which alternate sounds](#which-of-a-cues-waveforms-sounds-is-still-open), one
level up.

A structural reading of the surrounding grains is *suggestive* and is recorded
here as a hypothesis only: `0x23` looks like a marker and `0x24` like a goto,
with the marker id in operand byte 1. **Corroborated 2026-08-27 on HD's own
binary**: `0x23`'s handler is a two-instruction no-op and `0x24`'s scans a
marker table and sets the same skip-count field `0x22` writes - see the sound
page above. Under the marker/goto reading every goto in every bank on all six
discs finds its marker, and 3.4% of HD's grains and 1.8% of Pure's become
unreachable - but 15% of Pulse's do, and the check cannot tell
the two byte readings apart on the goto side at all, so **`0x22`, `0x23` and
`0x24` stay undecoded.**

## `0x80` means "not ADPCM", and this page had it backwards

**Corrected 2026-08-23.** A waveform descriptor's `+0x0e` was documented here
and on [the sound engine page](../ghidra/functions/psp-pulse-usa/sound.md) as
*"`0x80` asserts ADPCM"*. It asserts the opposite, and the decompilation that
was already on that page says so:

```c
undefined4 Sas_QueueSetVoice(voice, addr, size, loop, only_adpcm) {
  if (only_adpcm != 0) printf("SCREAM ERROR: THIS SYSTEM ONLY SUPPORTS ADPCM VOICE DATA!\n");
  ...
```

`Scream_KeyOnVoice` passes `(wf+0x0e & 0x80) != 0` as that fifth argument, and
the whole of what the function does with a non-zero one is **complain**. A
system that only supports ADPCM prints that when handed something that is not
ADPCM. So the bit marks a non-ADPCM waveform and the PSP refuses to play it.

The reading was written the natural way round and never tested, because on a
PSP disc nothing distinguishes the two: **the bit is never set**.

### Two independent confirmations

**A build that refuses the bit ships nothing that sets it.** Censused over
every waveform span on four discs:

| Disc | Spans | Carrying `0x80` |
| --- | ---: | ---: |
| `pulse-psp-usa` | 916 | **0** |
| `pulse-ps2-eu` | 985 | **0** |
| `pure-psp-usa` | 461 | **0** |
| `pure-psp-eu` | 461 | **0** |

**And where the bit *is* used, it predicts the payload.** Wipeout HD runs on
hardware that decodes more than one codec, and about a third of its spans set
it. Grouped by the descriptor's mode word, against the share of each span's own
bytes that are in PS-ADPCM spec:

| Mode word | `0x80` | Spans | In spec |
| --- | :---: | ---: | ---: |
| `0x0100` | clear | 1,733 | **100.00%** |
| `0x0101` | clear | 1,341 | **100.00%** |
| `0x0141` | clear | 563 | 99.93% |
| `0x0105` | clear | 480 | **100.00%** |
| `0x0140` | clear | 466 | 99.92% |
| `0x0185` | **set** | 377 | **3.52%** |
| `0x0180` | **set** | 316 | **8.99%** |
| `0x01c0` | **set** | 171 | **1.72%** |
| `0x01c5` | **set** | 158 | **0.09%** |

That is a clean split with nothing in between, over ~5,600 spans, from a field
the byte census knows nothing about. The bit is a codec selector.

### What the other codec is

**Identified 2026-09-02 as 16-bit PCM, big-endian, behind a 16-byte header** -
see [the fuller section below](#a-third-of-hds-waveforms-are-not-ps-adpcm-and-are-16-bit-pcm)
for the evidence. `oag_formats::sblk::Sound::is_adpcm` is still the predicate
that says which decoder applies; `oag_game::audio::sfx` used to **drop** a
non-ADPCM waveform rather than run the ADPCM decoder over it, which would have
produced 28 samples of noise per block - `CLAUDE.md`'s plausible-looking
stand-in exactly - and now calls `oag_formats::sblk::decode_pcm16` instead.

Confidence **90**: a decompiled argument whose only use is an error message,
plus a 4-disc zero census, plus a near-perfect correlation on a fifth disc that
uses the bit. What stops it being higher is that no emulator has been run.

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

## The PS2 ships the same container, byte for byte

Despite the page's title, this format is not PSP-only within Pulse. The PS2
disc's `WADS2.WAD` holds **44 `SBlk` banks**, and
[`sblk::Bank::parse`](../../crates/formats/src/sblk.rs) accepts every one of
them **with no endian switch and no relaxed framing**. That is a real result
rather than a weak one, because `Bank::parse` refuses anything whose sections do
not close exactly: parsing at all *is* the finding.

| Check | PS2 result |
| --- | --- |
| `03000000` entries in `WADS2.WAD` | 44 |
| Parsed by the unmodified PSP reader | **44 of 44** |
| PS-ADPCM blocks | 646,287 |
| Blocks with an in-spec predictor and shift | 645,984 (99.95%) |
| Banks with a self-name | 40 |
| Sound names recovered | 680 |
| Banks where the name count equals `cue_count` | **44 of 44** |
| Banks whose waveform spans tile the section exactly | **44 of 44** |
| Banks whose cue runs tile the command table exactly | **44 of 44** |

The self-names are the same set the PSP carries - `HUD`, `SHIP`, `SHIP_ZM`,
`basilic`, `metropi`, `talonsj` and so on - and the banks the game asks for by
path resolve identically: `Data\Sound\hud.bnk`, `ship.bnk`, `ship_zone.bnk` and
`weapons.bnk` all hash to entries in `WADS2.WAD`, and the four cues
`oag_game::audio::sfx` wires resolve to **the same frame counts on all three
discs** - `pulse-psp-usa`, `pulse-psp-eu` and `pulse-ps2-eu` - down to the
individual sample. `SPEEDUPPAD` is 20,384 frames on every one.

`WADSP.WAD`, the PS2's `FE.wad` counterpart, holds **none**; the PSP splits
three banks into `FE.wad` and the PS2 does not.

This is why nothing in the game's audio path branches on the platform. The
consequence for [ADR-0025](../architecture/adr/0025-a-boot-chain-carries-its-provenance.md)'s
axis language is that sound banks are a *lineage* fact for Pulse rather than a
per-release one.

Contrast **HD**, whose `.bnk` is the same container big-endian with two
alignment changes and is deliberately not implemented - see `HANDOVER.md`.

## Wipeout Pure ships the same container too - and this page said otherwise

**Corrected 2026-08-23.** This page carried, at confidence 85, the claim that
*"not one entry in any of Pure's three archives begins with that magic"*. Pure
has **29 `SBlk` banks**, every one of which `Bank::parse` accepts unmodified,
and the six cues `oag_game::audio::sfx` fires all resolve in them.

### The trap, which is worth more than the correction

The claim was not sloppy - it was *precisely true and useless*. **No bank
begins with `SBlk`.** The magic sits at offset `0x18`, after the eight-byte
container header and the two eight-byte section-table entries, and a scan for
the magic at offset 0 finds nothing on **any** disc, Pulse included. The Pulse
census never noticed because it went through
[`looks_like_bank`](../../crates/formats/src/sblk.rs), which reads `0x18`.

So a search whose result would have been identical on a disc known to be full
of banks was taken as evidence of absence. The general form: **a magic-scan
miss is only evidence when the scan is run against a positive control.** This
is the same shape as the `frontend.bnk` contiguity trap
[above](#the-field-is-an-abbreviation-not-a-truncation) - a search that worked for a
reason other than the one assumed.

The "25 RIFF entries at 44.1 kHz" in the old text are real and are a different
thing; they are not what the `.bnk` paths point at.

### What Pure carries

| Check | Pure USA | Pure EU |
| --- | ---: | ---: |
| `SBlk` banks | 29 | 29 |
| Parsed by the unmodified reader | 29 | 29 |
| Waveform spans | 461 | 461 |
| Spans carrying `0x80` (not PS-ADPCM) | **0** | **0** |

The bank *names* are Pulse's, hash for hash: `Data\Sound\hud.bnk`,
`ship.bnk`, `ship_zone.bnk`, `weapons.bnk`, `speech.bnk`, `frontend.bnk`,
`generaltrack.bnk` all resolve on both pressings. So do the cue strings -
`SPEEDUPPAD`, `.COLLISIONS`, `ABSORB`, `~ENGINE`, `~SHIELD` and `shieldactive`
are all present, and **a Pure race makes the same six sounds a Pulse race
does with no code branch at all**.

What differs is per-cue detail rather than structure, and one difference is
load-bearing: Pure's `~SHIELD` binds **four** waveforms of which only **two**
carry the loop flag, where Pulse's binds two and both do. That is what forced
the loop flag to be kept per waveform rather than per cue.

Pure's ship bank is also thinner - six cues to Pulse's nine, and it adds
`~AMBIENCE` and `rumble`, which Pulse has not got.

## Wipeout HD: the same container, byte-swapped whole

**2026-08-23.** HD's 50 `.bnk` entries - spread over the seven PSARCs, several
of them the same name in more than one archive - all parse with
[`sblk`](../../crates/formats/src/sblk.rs), and every rule this page recovered
from the PSP executable works on them unchanged.

`HANDOVER.md` had recorded HD's banks as measured but deliberately
unimplemented, on the grounds that a framing-only relaxation would hand back a
`Bank` whose `sounds()`, `sound_names()` and `decode_adpcm()` were all
unverified. Each of those now has a number against it.

### What differs, and it is two alignments

| | PSP / PS2 / Pure | HD |
| --- | --- | --- |
| Byte order | little | **big** - the magic reads `klBS` |
| Section 0 starts at | 24, the table's own end | **32** |
| Section 1 starts | where section 0 ends | 0, 4, 8 or 12 bytes later |
| Section 1 ends | exactly on the blob | **exactly on the blob** |
| `+0x24` | 20544 | 20544 |

Both differences are padding, so the two *starts* are checked as "aligned, at
or after, within one 16-byte block" rather than "exactly at".

**The alignment enforced is 4, not 16**, and the distinction is worth keeping
straight because HD's own values would satisfy either: it starts section 0 at
32 and section 1 at a multiple of 4 past section 0's end. The check cannot be
16, because **the PSP's section 0 starts at 24** - the section table's own end,
8-aligned - so a 16-byte rule would reject every Pulse and Pure bank while
reading like a statement about HD's padding. See
`oag_formats::sblk::SECTION_ALIGN`.

**The tail stays exact**, on all 50 - it is the one check that says the blob has
been read to its end rather than into the middle of something else, and it costs
nothing to keep.

The order is **sniffed off the magic** rather than passed in, because the
container is byte-swapped a `u32` at a time and so `SBlk` and `klBS` tell the
two apart by themselves. The ground-truth test also asserts that every HD bank
**fails** to parse little-endian, so "big-endian" is a statement rather than a
permissive reader.

### The three rules transfer intact

| Check | Result |
| --- | --- |
| `.bnk` entries across the seven PSARCs | 50 |
| Parsed | **50 of 50** |
| Also parsing little-endian | **0** |
| Cue runs tiling the command table | **50 of 50** |
| Waveform spans tiling their section | **50 of 50** |
| Names recovered | 1,963 |
| Banks where names equal `cue_count` | **50 of 50** |

Three exact tilings and a name table that covers every cue, on a disc from a
different console and a different generation, off arithmetic read out of a PSP
executable. That is the strongest evidence this page has that the arithmetic is
the format's rather than one build's.

### A third of HD's waveforms are not PS-ADPCM, and are 16-bit PCM

Grouped by [the `0x80` flag](#0x80-means-not-adpcm-and-this-page-had-it-backwards):

| Spans | In PS-ADPCM spec |
| ---: | ---: |
| 5,381 with `0x80` **clear** | **99.98%** |
| 1,167 with `0x80` **set** | **7.19%** |

**Identified 2026-09-02: the second codec is 16-bit PCM, big-endian, behind a
16-byte header.** `oag_formats::sblk::decode_pcm16` is the decoder; its own
doc comment carries the full evidence, repeated here:

- The PS3 executable (`ps3-hdfury-eu/EBOOT.elf`, `0x007d0818`) holds the
  string `"SCREAM: ERROR! Unknown voice type in bank - must be ADPCM or
  PCM\n\tSCREAM does not know how to handle this data"` - SCREAM's own error
  message names exactly two voice types. **The function that owns the string
  is now resolved: `CellMs_QueueVoice` (`0x00633c80`)**, reached through
  `Scream_OpKeyOn` (`0x00626728`, the PS3 analogue of the opcode `0x01`/`0x09`
  handler `psp-pulse-usa/sound.md` already names `Scream_OpKeyOn`) and
  `Scream_KeyOnVoice` (`0x00630310`, the PS3 analogue of PSP's
  `Scream_KeyOnVoice`). Its decompiled body is a direct read of the
  ADPCM/PCM split: `NOT_ADPCM_FLAG` set skips a 16-byte header and multiplies
  a header field by 2 (bytes per sample); `NOT_ADPCM_FLAG` clear walks
  16-byte blocks for a PS-ADPCM loop/mute flag. See
  [`docs/ghidra/functions/ps3-hdfury-eu/sound.md`](../ghidra/functions/ps3-hdfury-eu/sound.md#0x010x09---scream_opkeyon-and-the-codec-dispatch-chain)
  for the full decompile and evidence.
- Skipping the 16-byte header and reading the rest as big-endian `i16`, every
  one of the 1,167 not-PS-ADPCM spans has a mean roughness (mean absolute
  sample-to-sample step, divided by the span's RMS) of **0.257** against about
  1.41 for white noise, with only one span over 1.0 at all. This is the same
  measurement this page's earlier "0.94 and 1.04... not 16-bit PCM in either
  order" claim made, over the same corpus, and it does not reproduce: that
  claim did not skip a header, and the header alone does not explain the gap
  (the file's own scratch probe found the two figures equal whether or not
  the header is skipped, since 8 header samples do not move a span-length
  mean). What actually changed between the two measurements was not
  re-derived; the newer one is a direct measurement against real data and is
  what this page now stands on.
- A labelled span - `weapons.bnk`'s `~SHIELD` cue, whose two not-PS-ADPCM
  waveforms used to be dropped - decodes to a spectrogram of stable horizontal
  harmonic bands, consistent with a sustained shield-hum effect and not with
  noise.
- For the 315 of 1,167 spans that also carry `LOOP_FLAG` (`+0x0e`'s `0x40`),
  the header's second big-endian `u32` (bytes 4..8) equals the sample count
  exactly on every one: `header_word * 2 + 16 == span length`. Spans without
  the loop flag read zero there instead. What that word means when it is not
  the sample count is not known, and decoding does not depend on it - the
  other 12 header bytes are zero on every span sampled and are not otherwise
  interpreted.

**Confidence: 90**, up from 85 once `CellMs_QueueVoice` was decompiled. Two
independent quantitative measurements, a primary-source string and now a
decompiled call site all agree - past `decompile_function`'s own
"decompilation only" ceiling of 84 because the arithmetic invariant (the
315-file header-word match, above) is independent, corroborating evidence
rather than a second reading of the same decompile. Not "Established" (95+):
no runtime trace exists for this binary, and this codec is a PS3-only
addition with no PSP or PS2 build to corroborate it against as a second
binary.

`oag_formats::sblk::Sound::is_adpcm` is still the predicate for which decoder
applies; `oag_game::audio::sfx` now calls `decode_pcm16` rather than dropping
what it rejects.

### HD makes eight of the nine sounds

The difference between HD and the other titles turned out to be **which file a
cue is in**, and nothing else - the cue strings are identical across the
lineage. That is a five-field path table per title
([`oag_title::SoundBanks`](../../crates/title/src/race.rs)), not a new axis:

| Cue | Pulse and Pure | Wipeout HD |
| --- | --- | --- |
| `SPEEDUPPAD` | `hud.bnk` | **`weapons.bnk`** - HD has no `hud.bnk` |
| `.COLLISIONS` | `ship.bnk` | **`shiphd.bnk`** |
| `ABSORB`, `~SHIELD`, `~ROCKLOCK` | `weapons.bnk` | `weapons.bnk` |
| `shieldactive` | `speech.bnk` | `speech.bnk` |

One spelling - `Data\Sound\...` - reaches a WAD and a PSARC alike, because
`oag_assets::psarc` folds case and separators, so no caller branches on the
container.

**One cue does not load on HD, and says why in the race's own report:**

```text
sfx: SPEEDUPPAD -> 7 waveform(s) from Data\Sound\weapons.bnk
sfx: .COLLISIONS not loaded: .COLLISIONS binds no waveform: its 4 command(s) are all opcodes this does not read
sfx: .COLLISIONS -> 112 waveform(s) from Data\Sound\shiphd.bnk
sfx: ABSORB -> 6 waveform(s) from Data\Sound\weapons.bnk
sfx: ~ENGINE not loaded: "~ENGINE" names no cue in shipHD
sfx: ~SHIELD -> 8 waveform(s) from Data\Sound\weapons.bnk
sfx: shieldactive -> 2 waveform(s) from Data\Sound\speech.bnk
sfx: ~ROCKLOCK -> 2 waveform(s) from Data\Sound\weapons.bnk
```

`~ROCKLOCK` is the clearest end-to-end proof of the PCM decoder: **every** one
of its waveforms was in the second codec, so before `decode_pcm16` existed the
cue failed the loader's own `waveforms.is_empty()` check and did not resolve
at all - it could not have loaded any other way.

- **`~ENGINE` does not exist on HD.** Its ship audio is a per-event set -
  `c_CShipWall`, `c_CShipShip`, `c_GShipShip`, `c_ElecArcA`..`D`,
  `c_CrackLoopL/C/R` - which is a different design and not a renamed cue.
  Nothing is substituted; the held voice never opens.
- **`.COLLISIONS` binds no waveform of its own, and plays other cues instead.**
  This page recorded it for a day as "all four commands are among the 43 unread
  opcodes", which was true and was a dead end wearing the shape of a finding.
  Two of the four are `0x08`, [the child-sound grain](#a-cue-that-plays-other-cues),
  and the tree under it reaches **112 waveforms**, all PS-ADPCM:

  ```text
  .COLLISIONS
  |- c_CShipShip -> c_CShipShipS, c_CShipShipM, c_CShipShipL
  \- c_CShipWall -> c_CShipWallS, c_CShipWallM, c_CShipWallL
  ```

  **Nothing is wired off the split.** Which child a parent plays is guarded by
  opcode `0x22`, whose operand is not decoded, so `oag_game::audio::sfx` takes
  every reachable leaf and chooses among them with its own generator - the same
  approximation it already makes among a single cue's alternates, one level
  further down. The game does know whether it hit a wall or a ship, and
  `oag_render::sparks::severity` already computes a severity band, but pairing
  either against these names would be a mapping invented here rather than one
  read off the disc.

The `~SHIELD` line is also the not-PS-ADPCM path working end to end: all eight
of HD's `~SHIELD` waveforms decode now, where two used to be silently dropped.
`~ROCKLOCK`, above, is the sharper version of the same proof - a cue that
could not load at all until the second codec did.

### HD's cue edges are Pulse's, and that is an assumption

Worth stating plainly, because the rest of this page is measurement. The cue
*names* and the *banks* above are HD's own data, read off HD's own disc. The
**moments they fire on** are not: every trigger this project has -
`Ship_ApplySpeedupPad`, `ShipCollisionFx_Trigger`, `FUN_08840640`,
`Shield_Activate` - was read out of a **PSP Pulse** executable, and **no HD
dispatch has ever been looked at**.

Applying them is a bet that two games in one series with the same designer,
the same cue names and the same middleware fire those cues at the same moments.
That is a reasonable bet and it is not a reading. Recorded at confidence
**50** - below this project's naming threshold, which is why it is written here
as a caveat rather than implied by the code.

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

The cue rule and the PS2 claim have their own file,
[`crates/formats/tests/sblk_cue_ground_truth.rs`](../../crates/formats/tests/sblk_cue_ground_truth.rs),
which runs over both discs at once and reports:

```text
banks          83
  raw index                  banks   0/83  cues   138/1282
  v / 8                      banks  83/83  cues  1282/1282
  (v - command_offset) / 8   banks   0/83  cues   714/1282
  (v - parameter_offset) / 8 banks   0/83  cues     0/1282
  (v & 0xffffff) / 8         banks  83/83  cues  1282/1282
  v & 0xffffff               banks   0/83  cues   138/1282
  v - command_offset         banks   0/83  cues    75/1282
playable cues  1282
empty cues     5, every one storing +0x08 = [0xfffffff8]
tiles exactly  83 of 83
named cues     1287, reaching no waveform 247
~name          616 of 1101 waveforms loop
plain name     6 of 800 waveforms loop
ps2 banks      44
adpcm blocks   646287, in spec 645984
names          680
spans tile     44 of 44
```

### Reading a bank without a debugger

`oag-wad sounds` lists every bank in an archive and every cue each one names,
with the waveforms the cue resolves to:

```sh
just wad sounds 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' --bank SHIP
```

```text
#860 cbd73678  SHIP      9 cues, 34 commands, 22 waveforms, 239 KiB
    .COLLISIONS        cue   1  cmds   1..18  15 waveform(s), 0 looping, 4.23s total
    EXPLBIG            cue   3  cmds  19..22   3 waveform(s), 0 looping, 2.39s total
    EXPLSMALL          cue   2  cmds  18..19   1 waveform(s), 0 looping, 0.59s total
    FLIP               cue   5  cmds  23..25   2 waveform(s), 0 looping, 0.63s total
    MALFUNCTION        cue   6  cmds  25..28   1 waveform(s), 1 looping, 1.61s total
    RESET              cue   4  cmds  22..23   1 waveform(s), 0 looping, 0.19s total
    ~ENGINE            cue   0  cmds   0..1    1 waveform(s), 1 looping, 1.21s total
```

The seconds are at [the assumed rate](#not-determined), which is not recovered.
`--cue` filters the same way, so `--cue COLLISION` over a whole archive finds
every bank that has one.

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
- ~~**Which cue owns which commands.**~~ **Solved and validated** - see
  [above](#a-cue-owns-a-run-of-the-command-table). The earlier probe recorded
  here read `+0x08` as a *descriptor-section* offset,
  `(cue[0x08] - command_table_offset) / 8`, and did not resolve on every bank.
  It is simpler than that: the field is a byte offset into the command table
  biased by nothing, so the base subtraction was the error. **Which one of a
  cue's several waveforms sounds is still open** and is the interesting
  remainder.
- ~~**`+0x08` = 772/260.**~~ **Partly settled.** The runtime gates its
  name lookup on `bank[2] & 0x100`, and 772 is `0x304` while 260 is `0x104`, so
  **bit `0x100` means "this bank carries a name table"** - a capability flag
  rather than a size. The other bits are still unread. See
  [the sound engine](../ghidra/functions/psp-pulse-usa/sound.md#the-name-table-decoded).
- **The sample rate of each waveform.** Not in the header. PS-ADPCM carries no
  rate, so it comes from a per-sound pitch value, and the 24-byte parameter
  records' first word is the obvious candidate - `0x42aa7f00`, `0x42a15000`,
  `0x42a76400` in `frontend.bnk`, which vary per sound and cluster tightly.

  **This is now load-bearing rather than academic**, because the game plays
  these waveforms: `oag_formats::sblk::ASSUMED_SAMPLE_RATE` is **44,100 and is
  a placeholder**, chosen because it is the rate the PS2's own voice archive
  uses ([ps2-voice.md](ps2-voice.md)) and because it puts the recovered cues at
  plausible lengths - a collision impact at 0.28 s, `~ENGINE` at 1.21 s. That
  is the whole argument for it. Every consumer goes through the one named
  constant so that decoding the field is a one-line change.

  One observation worth carrying forward: read as four bytes rather than as a
  float, the high byte of that word is `0x42` on all three `frontend.bnk`
  examples while the next varies (`0xaa`, `0xa1`, `0xa7`). That reads more like
  a constant note plus a per-sound fine-tune than like a single float, which is
  the opposite of what this entry originally assumed. Not pursued.

  The technique that would settle it is already in this tree: the 48 kHz music
  finding came from cross-correlating PS2 PCM against rate-declaring PSP
  ATRAC3plus. `speech.bnk` against `PRERACE.WAD` is the same pairing.
- ~~**Whether this is Sony's SCREAM engine.**~~ **Settled: it is.** The PSP
  executable carries nineteen `SCREAM` strings including the verbatim copyright
  line `" SCREAM PSP    (c)2006 Sony Computer Entertainment America"` and a
  reference to `snd_RegisterMainMemAllocator`, a published SCREAM API name.
  Confidence 99. One of those strings, `"THIS SYSTEM ONLY SUPPORTS ADPCM VOICE
  DATA"`, independently corroborates the codec above. See
  [the sound engine](../ghidra/functions/psp-pulse-usa/sound.md).
