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
header). Decoder: [`oag_music::wem`](../../crates/music/src/wem.rs), over the
[`atrac9dec`](https://crates.io/crates/atrac9dec) crate.
Ground truth: `crates/formats/tests/wwise_ground_truth.rs` (banks) and
`crates/music/tests/omega_wem_ground_truth.rs` (media, the decoder, and a cue
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
(`weapons`, `shipHD`, `voppler`, every `speech_*` but two). They are legacy
names: the executable never loads them (but `Speech_NGP_Grid`, loaded and
empty), and the events named after them live in the loaded `_NGP` banks (see
"Event names and the stub banks", below).

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
  id. The rest of the base parameters are read only for containers (below);
  for sounds and actor-mixers they are not.

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
about 30 of 46 each. The stub banks are **not** their home (they are legacy
names, below); where these targets went is still open, and nothing here
guesses at them: `Library::resolve_event` reports the target ids and plays
nothing for them.
Of the 3,706 `data00` events, 2,369 resolve to no media - stop and state
actions, music events (read in the Music section, below, but not followed by
`resolve_event`, which reports them in `EventPlan::unread`), and those dangling
targets - which is why "events reaching media" is a count and not a defect.

### Event names and the stub banks (2026-11-02)

Read in the executable and checked against the banks; the evidence is
[`ps4-omega-eu/wwise-sound.md`](../ghidra/functions/ps4-omega-eu/wwise-sound.md).

* **Every bank ships its Wwise listing.** Beside each `.bnk` is a `.txt` of the
  same name (`Weapons_NGP.txt` 351,965 bytes in the patch) with sections
  `Event`, `Switch Group`, `Switch`, `State Group`, `State`, `Custom State`,
  `Game Parameter`, `Effect plug-ins`, `In Memory Audio`, `Streamed Audio`:
  tab-separated id, name and authoring path. The stubs' listings are empty.
* **The executable posts `<cue>[_PC]__<bank>`** and Wwise hashes that whole
  string (FNV-1, lower case). The cue is the code's string (`_321_NGP`), `_PC`
  is added when the emitter group belongs to the player, and the bank is the
  name of the slot the cue plays in (`Weapons_NGP`). So `_321_NGP` is the
  event `_321_NGP__Weapons_NGP` (243270081) and `Go_NGP` is
  `Go_NGP__Weapons_NGP` (744083536), both in `English(US)/Weapons_NGP.bnk`.
  Confidence 92: the listing ids, the music events (`Set_Music_Track_N__frontend`,
  above) and 1,212 of 1,272 trackside emitters agree.
* **`_PC` events are relays.** Kind `0x2103` is an action whose target is
  another event, played in turn; `_321_NGP_PC__Weapons_NGP` is one such action
  to `_321_NGP__Weapons_NGP`. Weapons go further: `NGP_MissileFire__Weapons_NGP`
  is a single relay to `WPN_MISSILE_Fire`. `Library::resolve_event` follows
  `Play` only and returns nothing for a relay; the ground-truth check
  (`posted_events_resolve`) follows both.
* **The stubs are never loaded** (all eleven bank-load call sites read), and
  events named after them are in loaded banks: `*__voppler` in `Ship_NGP.bnk`,
  `_AUTOPILOT__weapons` in `Weapons_NGP.bnk`, `_321_GO__speech` in
  `Speech_NGP.bnk`, 482 `*__Speech_NGP` in `English(US)/frontend.bnk`. The
  bank in an event name is a namespace. Legacy 8-character bank names in the
  data (`track.final.audio`, `crowd`, `env_moa`, `shipHD`) reach a Wwise bank
  through a 50-entry alias table in the executable.
* **Which banks a race has loaded:** `frontend`, `Speech_NGP`, `generaltrack`,
  `speech_fe`, `Ship_NGP`, `Weapons_NGP`, `crowd`, `Speech_NGP_Grid` from boot
  (plus `Init` and `Music`), and the track's `TrackStartup.xml` bank, or
  `env0_zone` (Zone, Nitro Battle) or `env0_det` (Detonator). Each is opened
  under `English(US)/` first, then at the root (watched live once).

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

`oag_music::wem::decode` turns an ATRAC9 `.wem` into the `Pcm` every other codec
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

## Music: a `Music_Track` state per song

Read 2026-10-05 off `data08`'s `Music.bnk` (21,658,812 bytes), the patch's
`data/plugins/music/Definition.xml` (`data09`) and `data/audio/sound/Music.txt`,
in `oag_formats::wwise::music` and `oag_music::omega`. Ground truth:
`crates/formats/tests/wwise_music_ground_truth.rs` and
`crates/game/tests/omega_music_ground_truth.rs`. Omega races play it.

* **`Music.txt` is Wwise's own soundbank listing and it names things.** It is
  shipped beside the bank (138 KB, text) and lists the 33 events, the three
  state groups (`Music_Track`, `Game_FLOW`, `Gameplay_FLOW`), 29 + 6 + 5 states
  and the authoring name of every stream (for example
  `<artist> - <title>__174bpm__DRUMS Front`). Every id in the bank is
  `name_hash(name)`: FNV-1 (32-bit) over the lower-cased name, checked on all
  of them (confidence 98). This replaces the first read's "25 of 28 states
  named by artist spelling": all 28 are named, and the three that did not
  match are a spelling (`Nosia`), a collaboration (`CarbonCommunity`) and a
  remix (`AddiktionRemix`).
* **The playlist is plugin XML and the event number is the location.**
  `Definition.xml` has 29 `PI_Music` entries (`location` `00` to `28`, an
  `Artist`, `PlaylistId` 1 or 2). The bank has `Set_Music_Track_N__frontend`
  for N = 1 to 28, each a pair of `SetState` actions (kind `0x1204`, group at
  body +9, state at +13): `Music_Track` to that song's state, and a second
  state in group id 1000. N is the entry's `location` (checked against the
  artist on every entry that matches by name; confidence 95). **Location 00 has
  no event** (`__Set_Music_Track_28__frontend` is a commented-out duplicate with
  no `Music_Track` action): that entry is unplayable in the original too, as far
  as this bank says.
* **The walk** (all 28 songs, confidence 92): `Play_External_Music` plays the
  root music switch. Its tree is keyed on `Game_FLOW`; the `Gameplay` branch
  goes to a switch on `Gameplay_FLOW` (`Race`, `PostRace`), then to one on a
  group that is in no bank read here, whose default key (`None`) goes to the
  switch on `Music_Track`, whose leaf per state is a music random/sequence
  container. Its playlist is a root and one item that plays one **segment**; the
  segment's children are the song's **tracks**. `Menus` and `Loading` go to a
  different container: the front end's own loop (three stereo stems, 279.9 s,
  embedded in `Music.bnk`'s `DATA`).
* **A song is its segment's tracks played together** (confidence 90). Every
  track is type 0, has one source and one clip starting at the segment start;
  the clips end on the segment's length to the millisecond on all 28 songs.
  Seventeen songs are 7 to 11 stereo stems (154 files), eleven are one
  eight-channel file. Each stem peaks at 0.3-83% of full scale and the unity sums at 8-100% (three
  reach full scale and are scaled down), so they are **parts of a mix, not
  alternatives** (confidence 85). Each stem has one unread property, id 13,
  ranging -100 to 100, which looks like a position (the authoring names say
  `Front`, `Spread`, `SUB`); **how it folds to stereo is not read**, so the
  mix is a unity sum - **chosen, not measured**.
* **Clip timing** (fitted, exact on 28 of 28): a source sample `t` plays at song
  time `play_at + t`, from `max(begin_trim, 0)` to `source_duration +
  end_trim`; all in milliseconds, `end_trim` is 0 or negative. Many songs
  trim their ends (307.5 s of a 453.75 s source) and one starts mid file.

### Layouts

Music nodes are `u8 flags, base parameters, u32 n + n child ids, meter, stingers`
(read forward; confidence 94: 242 of 242 nodes in `data08`, 234 of 234 in
`data00` read to their last byte and every child resolves).

```text
flags(u8)  fx: u8 override, u8 n, [u8 bypass, n*7]   attach u8   bus u32   parent u32
           u8, bundle(4), bundle(8)                  4 bytes: c0 00 00 01 (c3 once)
           state chunk: u32 props (0 on all), u32 groups { u32 id, u8, u16 n, n*(u32,u32) }
           u16 rtpc (0 on all)
children   u32 n, n ids
meter      f64, f64, f32 tempo, u8 num, u8 denom, u8      stingers u32 (0 on all)
segment    f64 duration (ms), u32 markers { u32 id, f64 position, u32 len, name }
switch     u32 rules, rules, u8 continue, u32 depth, depth*u32 group, depth*u8 type,
           u32 tree bytes, u8 mode, tree nodes {u32 key, u32 target | idx u16 + count u16,
           u16 weight, u16 probability} (breadth first; leaves at level depth)
ranseq     u32 rules, rules, u32 items, items {u32 segment, u32 id, u32 children, i32 type,
           i16 loop, i16 min, i16 max, u32 weight, u16 avoid, u8 use weight, u8 shuffle}
rule       u32 n + ids, u32 m + ids, 21 bytes source, 24 bytes destination, u8 flag
           [flag 1: 30 bytes of transition object]
track      u8, u32 n sources (14 each), u32 clips { u32 track, u32 source,
           f64 play at, f64 begin trim, f64 end trim, f64 source duration }, ...
           base parameters (unread), u8 type (byte 5 from the end), i32 look-ahead
```

Refused by name rather than skipped: non-zero state properties, non-zero RTPC
lists, stingers, a rule flag other than 0 or 1, a tree or playlist whose sizes
disagree. None occurs in `data00` or `data08`. The rule's seven inner fields are
**counted, not named** (they decide transitions, nothing about which song
plays). A track's own base parameters are unread; its type is fitted off the tail.

### Census (`data00` / `data05` / `data08`)

| | data00 | data05 | data08 |
| --- | ---: | ---: | ---: |
| segments | 163 | 0 | 169 |
| switches | 10 | 0 | 10 |
| random/sequence containers | 61 | 0 | 63 |
| tracks (all type 0, one clip per source) | 927 | 0 | 993 |
| segment children that resolve | 927 of 927 | - | 993 of 993 |
| switch leaves resolving / `0` / missing | 80 / 1 / 0 | - | 82 / 1 / 0 |
| playlist segments resolving | 1,228 of 1,228 | - | 1,295 of 1,295 |

Counts come from the ground-truth tests, over every bank of each archive.

### What is not read

* **Three `Music_Track` switches.** The root's third group has keys `0` and
  `None` (all 28 states), `3493105359` (9 states) and `4072605221` (19
  states). The 9 + 19 are the union's two halves; whether they are
  `PlaylistId` 1 and 2 is **unchecked**, so it stays open. The race walk uses
  the `None` branch.
* **Front-end flow.** `Menus` is read to play the loop; which of `Menus` and
  `Loading` the original is in at which moment was not watched.
* **Eight-channel streams.** 11 songs; `atrac9dec` refuses them.
* **The property id 13** on tracks, and the track's other base parameters.
* **Looping.** Whether the ranseq's `loop 1` or the segment loops the song was
  not read; the engine plays each track once and advances the playlist.

## Sound containers

`oag_formats::wwise::container` reads a random/sequence container (type 5) and a
switch container (type 6). Both open with the node's base parameters, read as far
as it takes to reach what the kind adds, and end exactly at the body's last byte
(`TrailingBytes` otherwise); a shape not read is refused by name.

```text
base      u8, u8 n fx [, u8, n * 7], u8, u32 bus, u32 parent, u8        (the head)
          u8 n + n ids + n f32, u8 n + n ids + n * 8                    (two bundles)
          u8 positioning   bits 6,7 set; bit 3 or 4 set: u8 1, u32 attenuation follow
          u8 sends         bit 3 set: four u32 bus ids follow
          u8, u8, u16, u8, u8   advanced settings (unread)
          u32 groups, each { u32 id, u8, u16 n, n * { u32, u32 } }      state groups
          u16 rtpc, each { u32, u8, u8, u8, u32, u8, u16 points, points * 12 }
ranseq    u16 loop (0 forever), u16, u16, f32 transition + 2 f32 bounds,
          u16 avoid repeat, u8 transition mode, u8 random mode, u8 mode (0 random,
          1 sequence), u8 flags, u32 n + n child ids, u16 m + m * { u32 id, i32 weight }
switch    u8 group type (1 = state group), u32 group, u32 default switch, u8,
          u32 n + n child ids, u32 cases + cases * { u32 switch, u32 k, k ids },
          u32 params + params * { u32 id, u8, u8, i32, i32 }   (14 bytes each)
```

Census, `crates/formats/tests/wwise_ground_truth.rs` (`Containers`, per archive):

| | `data00` | `data05` | `data08` |
| --- | ---: | ---: | ---: |
| random/sequence containers read | 462 (453 random, 9 sequence) | 19 | 551 (538, 13) |
| refused by name (`BaseParams`: a 3D block that opens a path) | 1 | 0 | 1 |
| switch containers read (state groups) | 525 (340) | 39 (17) | 626 (400) |
| any other refusal | 0 | 0 | 0 |
| child ids / resolving to an object of the archive | 5,460 / 5,460 | 243 / 243 | 6,015 / 6,015 |
| of them whose own parent id names the container | 5,455 | 243 | 6,008 |
| playlist items / containers with avoid-repeat > 0 | 2,116 / 458 | 111 / 19 | 2,449 / 547 |
| containers weighting their picks (flag bit 0) | 0 | 0 | 0 |

The refused one is `Ship_NGP`'s, in both base and patch. Every `Weapons_NGP`
container reads. Measured: the layouts above (exact consumption of the body on
all of the above, a second source in the children's own parent ids), 85.
Unknown, named: the six advanced-settings bytes (their values are not decoded
beyond their length), the switch's `u8` after the default switch, a switch
parameter's two bytes (`0, 1` on most, `2, 1` on a few) and its two `i32` (fade
times in ms, a guess, 55), a random container's transition bytes, and the flag
bits (names recalled from public Wwise documentation, 55; `0x12` or `0x1a` on
every one). No container sets the weight bit, so a weighted pick is refused by
the player rather than guessed.

**What a post plays** (`oag_sound::sfx`, `wwise_start`): one child of a random
container, drawn by the `Rng` passed in; the first playlist item of a sequence;
a switch container is still refused (its group's current value is the game's).
The missile launch (`NGP_MissileFire__Weapons_NGP` -> relay -> `WPN_MISSILE_Fire`)
plays *one layer container* of five layers: four random containers (275399150 has
one item, three others six) and one plain sound, so a post sounds five media, one
per layer; the thread's earlier "resolves to one sounding media" was a
misreading. The player's `NGP_MissileFire_PC` event is not a twin of the NPC one:
it plays its own sound, relays to a second event after 100 ms and sets a state
(`0x1204`), none of which is wired.

**2048 and Omega:** 2048 ships `SBlk` banks, not Wwise: *checked, differs*
(`oag_formats::sblk` opens 2048's `.bnk`; `Bank::parse` refuses them with
`NoHeader`, the `data02` result above).

## Loops

Lane `omega-wwise-held-loops`, 2026-11-12. A Wwise object loops in one of two
ways, both read now (`oag_formats::wwise::hirc::prop::LOOP`,
`container::RanSeq::is_continuous`) and played by `oag_sound::sfx`'s
`wwise_start`:

* **A sound's loop count, property 58** in the node's own property bundle
  (`hirc::node_props`): an `i32`, `0` forever, `n` plays `n` times, absent plays
  once. Values seen over the patch's banks: `0` on 1,077 sounds, `2` on 4, `10`
  on 2, `20` on 4.
* **A continuous random or sequence container** (flag bit 3, `0x1a`) plays item
  after item, `loop count` times over (`0` forever). The held cues' looping
  containers are `0x1a` with loop `0` (magstrip, autopilot, shuriken travel) or
  `200` (the player's autopilot); every one-shot pick read is `0x12`, loop `1`.

Evidence, two independent sources:

1. **The held cues against the one-shots.** Every layer of Omega's held cues
   that should sustain carries one of the two (the probe over
   `_SHIELD`, `_AUTOPILOT`, `_magstrip01`, `_SHURIKENTRAVEL` and the six travel
   and lock events, each with its `_PC` twin; the evidence page's "Held
   loops"); none of the ten one-shot cues this port plays reaches either
   (`a_race_plays_its_posted_cues_from_their_wwise_events` asserts no voice of
   theirs loops).
2. **The media's own loop points.** A sound whose embedded `.wem` carries a
   `smpl` chunk (a loop point) has property 58 = `0`:

   | Archive | prop 58 = `0`: with `smpl` / without | absent: with `smpl` / without | `2`, `10`, `20` |
   | --- | ---: | ---: | ---: |
   | `data08` (patch) | 428 / 648 | 5 / 5,473 | 2 / 8 |
   | `data00` (base) | 214 / 512 | 5 / 5,173 | 2 / 6 |

   433 of `data08`'s embedded sounds carry `smpl`; 428 of them loop forever.

Confidence: **property 58 is the loop count, 85** (fitted, two sources above;
the id is recalled from public Wwise documentation as `Loop`, beside 59
`InitialDelay` already fitted); **the continuous flag and the container's loop
count, 75** (bit names recalled from the same documentation, every held-cue
container agrees).

**How it plays** (chosen where the read stops): a forever sound is a looping
layer, held until the game releases the cue; a counted loop is the media laid
out `n` times, and one past 30 s of media loops instead (the player's
autopilot's 200 plays). A continuous container's one pick stands for every
item it would play, so a held random container repeats its first draw rather
than drawing per pass, and its transition (crossfade mode 5, durations unread)
is not played. A loop that its own event also stops or seeks is laid out to
the cut and ends there (none of the held cues has one). The `smpl` loop
points are not read: the whole media loops, so a sound authored with a lead-in
before its loop start replays the lead-in.

**2048: checked, differs.** 2048 ships `SBlk` banks, not Wwise; its looping is
the `SBlk` reader's. **HD:** `SBlk` too, untouched.

## What is not done

* **Event names.** A bank carries ids only, but its `.txt` listing names every
  event, and the executable's name for a cue is `<cue>[_PC]__<bank>` (above).
  Mapping this project's cues to Omega's cue strings is wiring, not reading.
* **Multi-channel ATRAC9**, above.
* **The unread parts of `HIRC`**: the base parameters of sounds and
  actor-mixers, switch values, `Init.bnk`'s bus and state definitions. Music
  objects are read (above); `EventPlan::unread` still lists the events that
  play one, and `Library::walk_music` is how they are followed.
* **Loop points.** 173 embedded files carry a `smpl` chunk; its loop start and
  end are not read (the loop count is, "Loops" above).
* **The 1,450 dangling Play targets.**
* **Race sound effects on this title** are not wired: the simulation's cues go
  through `oag_sound::sfx`, which reads `SBlk` banks by name. (Music is.) The
  start plays (`oag_sound::sfx`'s Wwise event timeline, the evidence page's
  "What the port plays"); every other cue waits on the evidence page's "Open"
  list (switch containers, positioning). Random containers are read and played.

## Confidence

| Claim | Score | Evidence |
| --- | ---: | --- |
| Chunk walk, `BKHD`, `DIDX`, `HIRC` framing | 94 | tile the file / the chunk exactly on 107 of 107 banks; padding is zero |
| Event, `Play`, sound and music-track layouts | 90 | exact-end on 2,183 of 2,183 Play; embedded size = `DIDX` size on every embedded sound; 0 unresolved media over two archives |
| Property 58 is a sound's loop count, `0` forever | 85 | "Loops": every sustaining held-cue layer carries it or a continuous container, no one-shot does; 428 of 433 `smpl` media in `data08` have it at `0` |
| Container flag bit 3 is *continuous*, its loop count read only then | 75 | "Loops": `0x1a` on every held-cue looping container, `0x12` on every one-shot pick; bit name recalled |
| Parent id at the head of the base parameters | 85 | fitted: resolves to an object or root on every node, 110 roots; the rest of the structure unread |
| `.wem` is ATRAC9 | 93 | four independent statements above |
| Plugin number 12 *means* ATRAC9 | 75 | recalled from the SDK; agrees with the media on every source |
| Plugin number 12 / 1 versus media tag `0xFFFC` / `0xFFFE` | 94 | 7,785 + 10 sources in `data00`, 8,503 + 10 in `data08`, 0 mismatches |
| The 18 bytes of extra data | 90 | every field checked against `fmt ` and the frame count on every file |
| Mono/stereo decode is right | 90 | two independent decoders agree to 1 LSB up to polarity |
| Event id = FNV-1 of `<cue>[_PC]__<bank>` | 92 | listing ids, the music events, 1,212 of 1,272 trackside emitters; executable read ([wwise-sound.md](../ghidra/functions/ps4-omega-eu/wwise-sound.md)) |
| Kind `0x2103` plays its target event | 80 | target is an event on every relay read (the `_PC` twins, the missile); fitted, not from the SDK |
| Random/sequence and switch container layouts | 85 | exact consumption of every body in the three archives bar 2 path-positioned ones (refused by name); children resolve and name the container as parent on 11,706 of 11,718 |
| Which polarity is Sony's | - | not known |
