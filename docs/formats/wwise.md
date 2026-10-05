# Wwise sound banks and media: Wipeout: Omega Collection's audio

**Status: banks understood, media identified and (for mono and stereo) decoded.**
The `.bnk` container, its `HIRC` object framing, events, `Play` actions,
sounds and music tracks read exactly over all 107 banks in the base and patch
archives. Every `.wem` is **Sony's ATRAC9**; mono and stereo ATRAC9 decode in
process and agree with FFmpeg's independent decoder; **four-, six- and
eight-channel files do not decode yet**. Nothing maps a Wwise event to this
project's cue vocabulary, and nothing plays sound in a race on this title.

Reader: [`oag_formats::wwise`](../../crates/formats/src/wwise.rs) (banks) and
[`oag_formats::wwise::wem`](../../crates/formats/src/wwise/wem.rs) (media
header). Decoder: [`oag_game::wem`](../../crates/game/src/wem.rs), over the
[`atrac9dec`](https://crates.io/crates/atrac9dec) crate.
Ground truth: `crates/formats/tests/wwise_ground_truth.rs` (banks) and
`crates/game/tests/omega_wem_ground_truth.rs` (media, the decoder, and a cue
played through the mixer).

**This is not the PSP's `.bnk`.** [`sblk`](../../crates/formats/src/sblk.rs) is
Studio Liverpool's own bank ([psp-audio.md](psp-audio.md)); its reader answers a
Wwise file with "unsupported bank version 1145588546, expected 3", which is the
ASCII `BKHD` read as a `u32`. The ten `.bnk` files in `data02` are the *old*
container (six little-endian, four byte-swapped, all under
`Data/environments/`) and `oag_formats::sblk` already reads them; the Wwise
reader refuses them with `Error::NoHeader`.

## Scope

Omega ships 53 Wwise banks in the base `data00.psarc` and the same 53 again in
the patch's `data08.psarc`, plus the patch's `data05.psarc` with one more
(`English(US)/frontend.bnk`). Beside them are 714 loose `.wem` files in `data00`
and 734 in `data08`, under `Data/audio/sound/` (`data/audio/sound/` in the
patch) with `English(US)/` for speech. Every bank is **bank generator version
118** (Wwise 2017.1, recalled from the version number and not read off the disc). No other archive carries either.

## The container

Chunks `{ char[4] tag, u32 size, body }`, little-endian, tiling the file exactly
(107 of 107 banks):

| Chunk | Where | Read |
| --- | --- | --- |
| `BKHD` | every bank, first | yes |
| `DIDX`, `DATA` | 33 of 53 banks in each archive, always together | yes |
| `HIRC` | 37 of 53, and the patch's `frontend` | yes |
| `INIT`, `STMG`, `ENVS`, `PLAT` | `Init.bnk` only | no |
| `STID` (bank names) | none | absent |

Sixteen banks in each archive are a `BKHD` and nothing else - 32 bytes
(`weapons`, `shipHD`, `voppler`, every `speech_*` but two). The names exist and
their content is not in them; where it is has not been established (see
"Where the references go").

**`BKHD`**: `u32` version (118), `u32` bank id (the FNV-1 hash of the bank's
name), `u32` language id (0 for SFX), `u16` alignment, `u16` device-allocated,
`u32` project id, then zero padding to the chunk's size (0 to 232 bytes; zero in
all 107). **`DIDX`**: 12-byte `{ u32 media id, u32 offset, u32 size }` records,
offsets into `DATA`'s body; every range is inside `DATA`, and every record is a
RIFF/WAVE `.wem` at its first byte, or the head of one (below).

**`HIRC`**: `u32` count, then objects `{ u8 type, u32 size, u32 id, body }` with
`size` counting the id. They tile the chunk and the count is the header's (75 of
75). Object types seen, with their counts in `data00`: 2 sound (6,873), 3 action
(4,109), 4 event (3,706), 5 random/sequence container (463), 6 switch
container (525), 7 actor-mixer (239), 8 bus (96), 9 layer container (1,789), 10
music segment (163), 11 music track (927), 12 music switch (10), 13 music
random/sequence (61), plus settings, attenuations and effects.

## Events to media

`event -> action -> target -> the sounds below it -> media id`.

* **Event** (type 4): `u32 n`, then `n` action ids, exactly.
* **Action** (type 3): `u16` kind, `u32` target id, `u8` flags, then per kind.
  Kind `0x0403` is **Play**: two property bundles (`u8 n`, `n` ids, `n` 4-byte
  values; then the same with 8-byte ranged values), a `u8` fade curve and a
  `u32` bank id, and that accounts for the body **exactly on 2,183 of 2,183**
  `data00` Play actions (2,264 of 2,264 in `data08`). The bank id is always the
  action's own bank. Other kinds (`0x2103`, `0x1204`, `0x1e03`, `0x1901`, ...)
  are not read.
* **Sound** (type 2): a *source*, then the node's base parameters. A source is
  `u32` plugin id, `u8` stream type (0 embedded in a bank, 1 streamed, 2
  prefetch), `u32` media id, `u32` in-memory size, `u8` bits (bit 0 *language
  specific*). Media reaches the player one of two ways: embedded, found by
  `DIDX` in **some** bank (140 embedded sounds have their `DIDX` record in a
  different bank from the one that names them), or as a loose
  `<media id>.wem` - **the loose file's name is the media id in decimal, and
  bit 0 says whether it is under `English(US)/`**: 614 of 614 language-specific
  loose sources are and 222 of 222 others are not.
* **Music track** (type 11): `u8` flags, `u32 n`, `n` sources; the rest
  (playlist, clip automation) is unread. All 927 (`data00`) have one source.
* **The parent id** of a node (sound, container, actor-mixer) is at the head of
  its base parameters, after a variable effect list, an `u8` and the override bus
  id. The rest of the base parameters (positioning, sends, states, RTPCs) are
  not read: nothing here needs them, and a fit for the whole structure was not
  attempted.

The source's plugin id is `AKMAKECLASSID(type, company, number)`, `type |
company << 4 | number << 16`. **Codec plugins** (`0x000N0001`): number 1 is PCM
(10 sounds), number 12 is ATRAC9 (6,858 of 6,873 `data00` sounds, plus every
music track source). Five sounds in `data00` and seven in `data08` use *source* plugins instead
(`0x00640002` and `0x00660002`, and in `data08` `0x00650002` too), which carry no
media; they look like Wwise's built-in silence, sine and tone generators, which
is recalled and not read off the disc.

### What was measured, and what it closes

Every distinct media id the banks name resolves: **`data00` 2,467 names
(714 loose `.wem`, 1,753 embedded, 0 unresolved); `data08` 2,588 (725 loose,
1,863 embedded, 0 unresolved)**, with 9 loose files in `data08` no bank names.
That single count is the check that the source layout, `DIDX`, the loose naming
and the language directory are all read right, since a wrong offset in any of
them leaves media unresolved.

| | `data00` | `data08` | `data05` |
| --- | ---: | ---: | ---: |
| banks | 53 | 53 | 1 |
| `DIDX` records | 2,122 | 2,257 | 60 |
| objects | 19,538 | 21,552 | 2,478 |
| events | 3,706 | 3,830 | 752 |
| Play actions | 2,183 | 2,264 | 510 |
| ... target in the same bank | 1,473 | 1,550 | 484 |
| ... target in no bank | 710 | 714 | 26 |
| sounds | 6,873 | 7,527 | 599 |
| music tracks | 927 | 993 | 0 |
| nodes with a parent in a bank / roots / dangling | 9,779 / 110 / 0 | 10,845 / 118 / 0 | 771 / 4 / 0 |
| events that reach at least one media | 1,337 | 1,367 | 484 |

### Where the references go

**1,450 of the 4,957 Play actions name an object no shipped bank defines**
(counted within each archive's own banks; the base and the patch together do
not close them either). They cluster: `Ship_NGP.bnk` 401 of 486 in the base,
and the eight HD-heritage circuit banks (`env1_vinetak` to `env8_anulphapass`)
about 30 of 46 each. The empty stub banks (`shipHD`, `weapons`, `speech_*`) are
the likeliest home for what is missing. Nothing here guesses at them:
`Library::resolve_event` reports the target ids and plays nothing for them.
Of the 3,706 `data00` events, 2,369 resolve to no media - stop and state
actions, music segments (their child lists are unread), and those dangling
targets - which is why "events reaching media" is a count and not a defect.

**Prefetch.** A streamed source may have its first bytes in `DIDX`: 164
(`data00`) and 175 (`data08`) embedded records are a **head** of a loose
`.wem` - the loose file starts with exactly those bytes and its `data` chunk
runs on past them. Every one has a loose file. They are the soundtrack's
streams; the bank holds the first few kilobytes so playback can start before
the disk answers.

## The `.wem`

RIFF/WAVE, and **the RIFF size field is not the file's length minus eight** on
the loose files (short by 28 bytes on stereo, 92 on four channels, 236 on
eight), so the chunks are walked and the size is never consulted.

| Where | Files | Chunks |
| --- | ---: | --- |
| loose, `data00` and `data08` | 714 and 734 | `fmt `, `JUNK`, `data` on all |
| embedded, `data00` (whole files) | 1,958 | `fmt `+`JUNK`+`data` 1,052; `fmt `+`data` 733; with `smpl` (loop points) 170; with `smpl`, `cue `, `LIST` 3 |

Codec by `fmt ` tag: **`0xFFFC` on all 1,448 loose files and on 1,956
(`data00`) or 2,080 (`data08`) embedded ones; `0xFFFE` on the 2 embedded PCM** (mono 16-bit at 32 or 44.1 kHz; that is how
Wwise writes PCM, the extensible tag, not `0x0001`). Channels: loose `data00`
665 stereo, 38 four-channel, 11 eight-channel; embedded `data00` 738 mono, 1,209
stereo, 2 six-channel, 7 eight-channel. Sample rate: every loose file is 48 kHz;
295 embedded files are 24 kHz. `data08`'s numbers are alike (omega_wem tests pin
both).

### The codec is ATRAC9 (confidence 93)

`fmt ` is 36 bytes: `WAVEFORMATEX` and 18 bytes of extra data. Four
independent things agree that it is Sony's ATRAC9 rather than a Wwise-specific
codec:

1. **The extra data holds a valid ATRAC9 configuration word.** Bytes 6-9 are
   four bytes that open `0xFE` (ATRAC9's sync), then a 4-bit sample-rate index,
   a 3-bit channel-config index, a validation bit, an 11-bit *frame bytes - 1*
   and a 2-bit superframe index. Over **every** ATRAC9 `.wem` in both archives,
   loose and embedded: sync `0xFE`; validation bit 0; superframe index 0 (one
   frame per block); rate index 7 with `fmt` at 48 kHz, or 4 with `fmt` at 24 kHz;
   **frame bytes equals `nBlockAlign`**; and the channel-config index tracks the
   channel count (0 mono, 2 stereo, 3 six channels, 4 eight, 5 four - the
   ATRAC9 table's own assignment). Five fields that must all agree with `fmt `
   and with each other, on 4,000-odd files.
2. **The frame arithmetic is ATRAC9's.** The first extra word is the samples
   per frame (256 at 48 kHz, 128 at 24 kHz) and the last is the encoder delay,
   one frame's worth; `frames * frame_samples - delay - samples` is between 0 and
   one frame on every file, so `samples` is the length after the delay and before
   the padding.
3. **Two independent ATRAC9 decoders accept the data.** FFmpeg's decoder decodes
   all 400 loose `data00` files sampled (371 stereo, 24 four-channel, 5
   eight-channel) with `-xerror` and no message; random bytes in the same
   container stop at "Invalid scalefactor coding mode!". `atrac9dec` (a port of
   LibAtrac9) decodes all mono and stereo files, both loose and embedded.
4. **The bank says so.** Every source whose plugin is codec number 12 has
   media tagged `0xFFFC`, and every one whose plugin is codec number 1 has media
   tagged `0xFFFE` (PCM), **with no exception**: 7,785 ATRAC9 sources in `data00`
   (6,858 sounds and 927 music-track sources) and 8,503 in `data08` (7,510 and
   993), and 10 PCM sounds in each, each media found the way a player finds it
   (`omega_data00_codec_plugin_names_the_media_tag`). That the number 12 *means*
   ATRAC9 is `AKCODECID_ATRAC9` in Wwise's codec numbering, **recalled from the
   SDK's header and not read off the disc** (confidence 75); items 1-3 do not
   depend on it.

Extra data, 18 bytes (confidence 90; the layout is fitted, every field checked
against `fmt ` or the frame count):

```text
+0x00  u16     samples per frame       256 at 48 kHz, 128 at 24 kHz
+0x02  u32     channel config          channels | 1 << 8 | speaker mask << 12
+0x06  u8[4]   ATRAC9 config word      read big-endian
+0x0a  u32     samples                 after the delay, before the padding
+0x0e  u32     delay                   samples to skip: one frame's worth
```

The channel config is Wwise's `AkChannelConfig`: mask `0x4` mono, `0x3` stereo,
`0x603` four channels (front and side), `0x60f` six (5.1 with side surrounds),
`0x63f` eight (7.1). PCM's six extra bytes are a zero `u16` and the same config.

## Decoding

`oag_game::wem::decode` turns an ATRAC9 `.wem` into the `Pcm` every other codec
here produces, skipping the delay and trimming the padding, so the length is
exactly `samples`. **In process, no cache**: ADR-0024's order is a library
first, our own decoder only for a format that is Wipeout's, `ffmpeg` last, and
ATRAC9 is Sony's with a pure-Rust decoder available (`atrac9dec`, MIT, a port
of LibAtrac9, no dependencies). It is young and single-author, so it was measured:

* **Mono and stereo: 2,612 of 2,612 decode** (loose 665 + embedded 1,947 in
  `data00`; `data08` likewise), each to exactly its declared length, and none
  is refused.
* **Against FFmpeg: 59 of 59 mono and stereo files** (a random sample of
  embedded media, 24 and 48 kHz) agree to within **one least-significant bit**,
  the great majority identical - **with opposite polarity**: the crate's output
  is the negation of FFmpeg's, sample for sample. One is Sony's polarity and one
  is not, and which is not known; it is inaudible on its own and is not
  corrected here. Confidence that mono and stereo decode is right: 90 (two
  independent decoders agree; neither is Sony's).
* **Four-, six- and eight-channel files: refused, all of them** - 49 loose
  (38 four-channel, 11 eight) and 9 embedded (2 six, 7 eight) per archive - with
  "invalid scale factor mode" or "scale factor out of bounds" part-way in.
  FFmpeg decodes the four- and eight-channel files it was given without error.
  The crate's README rates multi-channel "partial", and this is that. These
  are the soundtrack's stems and a few beds, not the effects; a decoder fix (or
  a second crate) is what closes it.

### One circuit's cue, end to end

`a_tech_de_ra_cue_plays_through_the_mixer`: in the patch's
`env12_techdera.bnk`, the first event in file order whose plan reaches an
embedded stereo ATRAC9 sound is `0x52c539f2`, which plays media `833645939`:
1,612,544 frames, 33.595 s, decoded, wrapped in an `oag_audio::Sound`, played
through `oag_audio::Mixer` at unity and written as a WAV (`OAG_WEM_OUT=<dir>`).
Rendered RMS 0.1354, the decoded clip's own; peak 1.0; the mixer clipped no
sample (the WAV's mono mix, with left and right 0.68 correlated, is 0.124). **94.7% of its energy is below 200 Hz** and 5%
between 200 and 1,000: a low bed, not an effect - which "first qualifying event"
gives, and is *chosen, not measured* as the cue to play. Nothing was played on
any device.

## Music: a `Music_Track` state per artist

Read 2026-10-05 off `data08`'s `Music.bnk` (21,658,812 bytes) and the patch's
`data/plugins/music/Definition.xml` (`data09`), by the lead, with a scratch
probe (`data/scratch/omega-music/hirc.py`, `states.py`; not committed). Nothing
is wired yet.

* **The playlist is plugin XML.** `Definition.xml` holds 29 `PI_Music`
  entries: `name` is the song title, `<Values location="00">` to `"28"`,
  `Artist`, an empty `Label` and `PlaylistId` 1 or 2. `Definition_demo.xml`
  holds 28.
* **`Music.bnk`'s objects**: 62 actions, 33 events, 169 music segments, 993
  music tracks, 10 music switches and 63 music random/sequence containers;
  nothing else.
* **The switch group is `Music_Track`** (confidence 85): `fnv1("music_track")`
  is the group id of 57 actions and appears in 4 of the 10 music switches.
  `fnv1("race")` appears in one music switch.
* **Its states are the artist names, with spaces removed** (confidence 88): 28
  SetState actions (kind `0x1204`, group at body `+9`, state at `+13`) set
  `Music_Track` to 28 distinct values. 25 of them are FNV-1 hashes of an
  artist from `Definition.xml` with spaces removed, or of its first credited
  name when the artist is a collaboration (`BoysNoize`, `TheChemicalBrothers`,
  `BlackSunEmpire`, `DJKentaro`, `SwedishHouseMafia`, ...). Three hashes
  (`404193461`, `3252658421`, `4067886831`) match no spelling tried; Code
  Manta, Burufunk and Noisia are the unmatched artists. Code Manta is
  `location="00"`, and 29 entries against 28 states suggests one entry with no
  state of its own. Unverified.
* **Music track media are ATRAC9 streams** (confidence 90): all 993 tracks
  carry one source with plugin `0x000C0001`. 990 are streamed (type 1) with a
  prefetch head in the bank's `DIDX` and 3 are embedded. 168 distinct media
  ids, 165 of them loose `<id>.wem` in the archives. Of those 165, **154 are
  stereo and 11 are 8-channel** (48 kHz), so most of the soundtrack decodes
  with what `oag_formats::wwise::wem` already does.
* **Not read**: the music switch's association tree (state to child), segment
  child lists and playlist containers, so the chain from a state to its
  segment's tracks is inferred, not walked. `PlaylistId` 1 vs 2 is unread.

## What is not done

* **Event names.** A bank carries ids only: an event id is a hash of a name the
  banks do not contain. Mapping Wwise events to the cue names the simulation
  emits needs the simulation's own list beside the ids, and a hash function to
  test candidate names against (FNV-1 over the lower-case name is what a bank id
  is; the same for events is not verified).
* **Multi-channel ATRAC9**, above.
* **The unread parts of `HIRC`**: the rest of node base parameters, container
  playlists, music segments/switches/sequences (so an event that plays one is
  reported in `EventPlan::unread`), `Init.bnk`'s bus and state definitions.
* **Loop points.** 173 embedded files carry a `smpl` chunk; nothing reads it.
* **The 1,450 dangling Play targets.**
* **Race audio on this title** is not wired: the simulation's cues go through
  `oag_game::audio::sfx`, which reads `SBlk` banks by name.

## Confidence

| Claim | Score | Evidence |
| --- | ---: | --- |
| Chunk walk, `BKHD`, `DIDX`, `HIRC` framing | 94 | tile the file / the chunk exactly on 107 of 107 banks; padding is zero |
| Event, `Play`, sound and music-track layouts | 90 | exact-end on 2,183 of 2,183 Play; embedded size = `DIDX` size on every embedded sound; 0 unresolved media over two archives |
| Parent id at the head of the base parameters | 85 | fitted: resolves to an object or root on every node, 110 roots; the rest of the structure unread |
| `.wem` is ATRAC9 | 93 | four independent statements above |
| Plugin number 12 *means* ATRAC9 | 75 | recalled from the SDK; agrees with the media on every source |
| Plugin number 12 / 1 versus media tag `0xFFFC` / `0xFFFE` | 94 | 7,785 + 10 sources in `data00`, 8,503 + 10 in `data08`, 0 mismatches |
| The 18 bytes of extra data | 90 | every field checked against `fmt ` and the frame count on every file |
| Mono/stereo decode is right | 90 | two independent decoders agree to 1 LSB up to polarity |
| Which polarity is Sony's | - | not known |
