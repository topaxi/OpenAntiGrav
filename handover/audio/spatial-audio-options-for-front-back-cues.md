---
status: blocked-maintainer
titles: [all]
updated: 2026-11-03
summary: "SPATIAL AUDIO built (original, rear_cue default, headphones via SADIE II D1 + hrtf); open: a listening judgement, Deck and S24 cost, elevation"
---

# Spatial audio options for front/back cues

## Built (2026-11-03, lane `spatial-audio-wire`)

The maintainer picked: one lane, all three values, default `rear cue`, "behind"
is the camera, `original` on every title. Landed in `oag-audio`/`oag-sound`:

- `SPATIAL AUDIO` row (`audio.spatial_audio`: `original`, `rear_cue`,
  `headphones`) on the audio page, every title (one `menu.toml` page serves all).
- Listener `forward` from the camera; per-voice `front`; `Mixer::set_spatial_mode`.
  `original` is bit-identical (test). `rear_cue` is a one-pole low-pass and a
  0.8 gain behind, bit-identical in front. Both **chosen, not measured**.
- `headphones`: `oag_audio::binaural`, `hrtf` 0.9.0 and 200 directions of
  **SADIE II subject D1, 48 kHz, 256 taps**. The licence is settled: Zenodo
  record 12092466 (version 2-2), its `LICENSE.txt` is the Apache License 2.0.
  Shipped with the licence and attribution in `assets/audio/hrtf/`; converter
  `scripts/sofa-to-hrtf-sphere.py`. One shared convolver, a block FIFO per voice,
  built on a thread when the setting asks for it.
- Cost, one headphone voice, 512 frames, dev profile (opt-level 2 deps), load
  average about 35 on 24 cores: 78 to 98 us per voice (0.75 % of a core for
  one, 29 % for 32). The rear cue is a few multiply-adds per sample.
- Builds checked: `wasm32-unknown-unknown` (oag-audio, oag-sound),
  `aarch64-linux-android` and `aarch64-apple-ios` (oag-audio).
- WAV pairs (front vs behind per mode, and a circling source) were written for
  the lead; nothing was played.

Still open: nobody has listened (is the rear cue enough, does headphones click on
a passing craft); Deck cost is measured (below), S24 cost is an estimate;
elevation is dropped (horizontal plane only); a headphone voice is 512 frames
late against a plain one; the dump path (`--dump-audio`) builds the sphere on a
thread, so a headphones dump is not reproducible to the sample.


2026-11-03, a measure lane. **Nothing is decided and nothing is built.** The
maintainer wants front/back cues and plans to try them ON by default first,
with an `original` setting value. This page ranks the options and names what
would settle the open claims. Pick one, or send back for a prototype.

## What the engine does today

- `oag_audio::spatial` ports Pulse's law:
  `pan = dot(normalize(emitter - listener), listener_right)`, equal power,
  linear falloff, doppler, cone
  ([positional-audio.md](../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md)).
  A source behind and a source ahead give the same `pan`.
- **That is the original's behaviour, measured**: the angle goes through
  `acos`, which is even, so "dead ahead and dead behind both produce 0"
  (positional-audio.md, "The angle loses front from back"). Pulse's
  `Scream_PanVolumePair` does have a rear-half branch (angle above 179, one
  table channel negated, i.e. a phase-inverted channel), but "the game's own
  emitters never reach it" except at exactly hard right. So the `original`
  setting value is today's code, and a front/back cue is an addition that
  needs a "chosen, not measured" label.
- **Where voices are placed**: `crates/sound/src/sfx.rs` computes a `Placed`
  (gain, pan, distance) per cue per tick against an `oag_audio::Listener` and
  hands the mixer `Play { pan }` / `Mixer::set_pan`. `Mixer::render`
  (`crates/audio/src/mixer.rs`) turns one `pan` into one `[L, R]` gain pair per
  voice per call, then resamples a stereo-duplicated source. Voice pool: 32
  (Pulse, Pure), 128 (HD, `HD_VOICES`). A voice with `pan: None` (music,
  announcer) is left alone and must stay that way.
- **Where it runs**: not in the cpal callback. A render-ahead thread
  (`oag-audio-render`, `output::render::Ahead`) mixes 512-frame blocks into a
  ring, with a floor of 60 ms of device buffer (`MIN_BUFFER`); the cpal
  callback only copies the ring and `spread`s stereo into the device's
  channels (extra channels silent). The browser runs the same render-ahead
  loop beside an AudioWorklet (`docs/tools/web.md`, "Sound"). So any option
  below runs off the realtime thread and pays no extra output latency beyond
  its own block.

## What every option needs from the engine

1. **A forward axis.** `Listener` carries only `position` and `right`.
   Front/back needs the camera's forward (and up for elevation); that is
   another column of the same matrix the code already reads. Small.
2. **A decision on "behind what".** The original's listener is the chase
   camera, which sits behind the player's craft. A craft just behind the
   player can be in front of the camera. Whether "behind" means relative to the
   camera or to the player's craft is the maintainer's call. Not resolved here.
3. **Per-voice direction, not a scalar.** `Voice.pan: Option<f32>` becomes a
   direction (azimuth, optionally elevation) or a front/back scalar. The
   sim-facing `Placed` and `VoicePlace` types change; `set_pan` callers in
   `sfx.rs` (about a dozen sites) follow.
4. **The `original` value** keeps today's path exactly (stereo `pan_gains`),
   so the new path must be a branch inside `Mixer::render`, not a replacement.
5. **Fixed-block hazard.** `Mixer::render_tick` renders one tick (735 frames at
   44.1 kHz, 800 at 48 kHz) and the live path renders 512; the mixer promises
   the offline dump and the live stream match for the same control sequence.
   The block-convolution options (1, 2, 4) need a fixed internal block with a
   per-voice FIFO, or that promise breaks and tick-sized calls fall out of
   step with the convolver.
6. **Sources are mono-in-stereo today.** `Sound::frame` duplicates a mono
   source to both channels; a binaural stage wants the mono sample, so mono
   cues feed it directly and any stereo cue stays on the plain path.

## Comparison

CPU figures: **B** = scratch benchmark on this machine, **E** = estimate from
the crate's own docs, **-** = none. Rule 6 is the same for every option, see
below the table.

| # | Option | Licence | Weight | Maintenance | wasm/web | Android, iOS | Deck | CPU per positioned voice | Block / latency | Headphones vs speakers | Wiring effort |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1a | `hrtf` 0.9.0 (Fyrox author) | MIT (crate); ships **no** HRIR data | `rustfft`, `rubato`, `byteorder`; pure Rust | updated 2026-08-06, 100 stars, README lists a click artefact on fast sources | Pure Rust so it should build; **unverified**, not tried | Pure Rust, should build; unverified | Same, x86-64 | **B**: 0.25 to 0.9 % of one core, see below | exact `steps * block_len` frames per call, per-voice overlap state | headphones; speakers get comb filtering and a timbre change | M: fixed-block FIFO, per-voice state, crossfade for the click, HRIR data choice |
| 1b | `firewheel-ircam-hrtf` 0.5.0 | MIT or Apache-2.0 | needs the Firewheel graph engine (`firewheel` ^0.12) | 2026-08-09 | unverified | unverified | n/a | **E**: crate warns "avoid more than 32-64 HRTF emitters, especially on less powerful devices" | Firewheel's | headphones | L: adopting Firewheel is a mixer replacement, not a plug-in; rejected unless the mixer is replaced anyway |
| 1c | `fyrox-sound` 1.0.1 | MIT | whole engine, same `hrtf` inside | 2026-03-28 | n/a | n/a | n/a | as 1a | its own | headphones | Not usable: replaces the mixer; take `hrtf` directly |
| 1d | `audionimbus` 0.16.1 over Steam Audio 4.8.1 (`audionimbus-sys` 4.8.1) | wrapper MIT or Apache-2.0; Steam Audio Apache-2.0 | C++ library, about 180 MB with `auto-install` | wrapper updated 2026-11-02, 212 stars | **No wasm listed** for Steam Audio; not confirmed either way | Android and iOS listed by Steam Audio; the wrapper's support unchecked | Linux x86-64 listed | **-**: Valve gives no numeric figure | `frameSize` set by the host, no stated constraint | headphones (default HRTF), also ambisonics and SOFA | M by code, but a native C++ dependency on every target plus the web hole |
| 1e | `kira` 0.12.6 spatial tracks | MIT or Apache-2.0 | audio engine | 2026-10-28, 268k recent downloads | n/a | n/a | n/a | n/a | n/a | **Panning and attenuation only, no HRTF** per its docs | Not an answer to this question |
| 1f | `rodio` 0.23.0 `SpatialPlayer`, `oddio` 0.7.4 | MIT or Apache-2.0 | engine | rodio 2026-10-11; oddio 2023-10-15 (stale) | n/a | n/a | n/a | n/a | n/a | simple ear-distance model (rodio); doppler and propagation delay (oddio); no HRTF | Not an answer |
| 2 | Public HRIR dataset + own convolution (needs a convolver such as 1a's, or `sofar`) | per dataset, see "Datasets" | dataset size 0.7 to a few MB | n/a | n/a | n/a | n/a | as the convolver | as the convolver | headphones | M, plus a data decision and attribution file |
| 3 | Cheap rear cue (low-pass plus slight attenuation behind) | ours | none | n/a | yes | yes | yes | **E**: one biquad per voice, under 0.01 % of a core (a few multiply-adds per sample) | per-sample, no block, state-free except 1 filter state | works on both; a weak cue, front/back confusion remains | S: filter state in `Voice`, cutoff and gain from `dot(dir, forward)` |
| 4 | Ambisonics (1st or 2nd order) + binaural decode (Resonance Audio style) | Resonance Audio Apache-2.0 but **archived 2023-11-11**; own implementation is ours; SADIE II decoder data Apache-2.0 | none for a hand-written encoder, or a C++ library for Resonance | Resonance dead; own code is ours | yes (own code) | yes | yes | **E**: encode is a few multiply-adds per voice (4 gains at 1st order, 9 at 2nd), decode is one fixed convolution for all voices | per-voice no block; the decoder runs on a bus once | headphones via HRTF decode; **speakers via a loudspeaker decoder** | L: encoder plus bus plus decoder, rotation by head yaw is free |
| 5 | Real multichannel through cpal (5.1, 7.1) | cpal 0.18.2 Apache-2.0 (already a dependency) | none | active | Not today: the ring and the worklet are stereo (a worklet can output more channels if the destination allows; not tried) | phones are stereo | Deck is stereo | **E**: a panner per voice, cheaper than 1 | per-sample | **Speakers and home theatre only**; headphone users get nothing | L: `spread` today puts silence in extra channels; needs a channel layout, a multichannel ring, a downmix for the rest |
| 6 | Omega's own 3D audio (patch 1.03) | game data | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | **Pending**: unread; lane `omega-3d-audio-read` owns it. Its answer may be the `original` value for Omega and could change the ranking. |

Rule 6 (`just check-deps`), all options: `cargo tree -p oag-net -e normal`
matches neither `oag-audio` nor `oag-sound` (checked 2026-11-03, no output),
so `oag-server` links no audio. Every option lands in `oag-audio` or
`oag-sound`; a new third-party audio crate stays out of the server as long as
it is a dependency of those two only. Rule 6 is a deny-list of crate names
(`FORBIDDEN_FOR_SERVER`); a new audio crate is out of the server by not being
reached, and could be added to that list as a tripwire (optional).

### The `hrtf` benchmark (B)

Scratch benchmark, `hrtf` 0.9.0, release build, one thread, Ryzen 9 7900,
**machine contended: load average 50 on 24 cores** during the run, so read
the numbers as an upper-leaning order of magnitude, not a result.

- Sphere: one real IRCAM file, `IRC_1002_C.bin` from
  `mrDIMAS/hrir_sphere_builder` `hrtf_base/IRCAM` (44.1 kHz, header: 187 points, 1,110
  indices so 370 faces, 512 taps; `HrirSphere::len()` is the tap count and
  reads 558 after the load-time resample to 48 kHz), loaded with `HrirSphere::from_file`.
  A 6-point, 128-tap synthetic sphere measured 1.2x to 4x cheaper depending
  on the block size, which is why that first run is not quoted.
- Method: `HrtfProcessor::process_samples` on a mono sine, source moving, 400
  calls per run, minimum of 7 runs.
- Per source, cost as a share of one core of audio time:

| block x steps | frames per call | us per call | % of a core per source | 32 sources | 128 sources |
| --- | --- | --- | --- | --- | --- |
| 128 x 4 | 512 | 91 | 0.86 | 27 % | 110 % |
| 128 x 6 | 768 | 122 | 0.76 | 24 % | 98 % |
| 128 x 8 | 1,024 | 169 | 0.79 | 25 % | 101 % |
| 256 x 2 | 512 | 27 | 0.25 | 8 % | 33 % |
| 256 x 3 | 768 | 52 | 0.32 | 10 % | 42 % |

- The spread across block sizes follows the overlap-save arithmetic: each FFT
  of `block + taps - 1` points yields only `block` useful samples, about 19 %
  at block 128 and 31 % at block 256 with 558 taps. A larger block amortises
  better; the trade is fewer interpolation steps per call, which raises the
  click risk the crate documents. Whether the FFT length hitting a power of
  two matters was not tested. A finer sweep is needed before any claim.
- **Estimate, not measured**: a Steam Deck or an S24 core runs roughly 2 to 3
  times slower than this core, so a 256-sample block costs about 0.6 to 1 % of
  a core per voice. Not every pooled voice is positioned and audible, and the
  original gates sources beyond 200 units, so a race holds far fewer than 32
  convolving voices. Pulse measured that a race left alone is down to the
  player's engine about two seconds after the flag (engine radius 50,
  positional-audio.md, "A consequence the numbers do not advertise"); that
  bounds the Pulse count at the start, not through a real race, and the other
  titles were not counted.
- To reproduce: a standalone cargo project (add an empty `[workspace]`
  table) on `hrtf = "0.9.0"` that calls `HrirSphere::from_file(path, 48000)`,
  then `HrtfProcessor::new(sphere, steps, block)` and `process_samples` in a
  timed loop.

## Datasets (option 2, and the data the convolver in 1a needs)

`hrtf` ships no data. Shipping anything we cannot redistribute is out, so the
licence of each is the gate. Entries marked **unsettled** are third-party
copies of a notice or secondary mentions, not the owner's current text.

| Dataset | Terms found | Status |
| --- | --- | --- |
| SADIE II (University of York) | [Zenodo 12092466](https://zenodo.org/records/12092466) lists **Apache-2.0**; WAV at 44.1, 48, 96 kHz plus AES69 SOFA; subjects H3 to H20 and D1, D2; also ships Ambisonic decoder configs | Licence read off the record's metadata; **to settle**: read the `LICENSE.txt` in the download and confirm 12092466 is the current record (an older 10886409 exists) |
| IRCAM Listen | Notice reproduced by the Ambisonic Toolkit and by Debian's libmysofa copyright file: "public and available for any use", asks for acknowledgement; 51 subjects, 187 directions at 15 degrees | **Unsettled**: only third-party copies; IRCAM's own page failed to load here (TLS). Settle with an archive.org snapshot of IRCAM's download page |
| MIT KEMAR | [sound.media.mit.edu/resources/KEMAR.html](https://sound.media.mit.edu/resources/KEMAR.html): "Copyright 1994 by the MIT Media Laboratory", "free with no restrictions on use", cite the authors for research or commercial use; 44.1 kHz, 710 positions | Read off the owner's page; acceptable with attribution. Single dummy head |
| CIPIC | Third-party copies: the Regents grant permission "for any purpose, educational, research or commercial", acknowledgement for commercial products; a 2001 paper mentions a restricted release that cannot be used commercially | **Unsettled**: the official page returned HTTP 410; settle with the official licence text |

The `hrtf` crate's own sphere files come from the IRCAM set (the repository's
`hrtf_base/IRCAM`), so option 1a inherits the IRCAM question unless we
build spheres from SADIE II or KEMAR (the format is a plain binary: magic
`HRIR`, rate, taps, vertex and index counts, indices, then position and two
HRIRs per vertex; `mrDIMAS/hrir_sphere_builder` is MIT). `sofar` 0.4.0
(MIT or Apache-2.0, "Pure Rust SOFA reader and HRTF renderer", partitioned
convolution, bundles BSD-3-Clause libmysofa and a KD-tree) reads SADIE II's
SOFA files directly; its platform support is not stated in what was read.

## Reading the table

- Options 1b, 1c, 1e, 1f do not fit this engine: they replace the mixer or do
  not do front/back.
- Option 5 helps only speaker owners and the web path is stereo today, so it
  cannot be the default.
- The real choice is between **3** (cheap, weak cue, works for speakers and
  headphones and on the web), **1a plus a redistributable sphere** (the
  strongest headphone cue, heaviest wiring and CPU, pure Rust), and **4**
  (cost flat in the number of voices, so it scales to the 128-voice HD pool,
  but the most code and a decoder data choice; Resonance Audio itself is
  archived). Steam Audio (1d) is the only mature turnkey option but has a
  native build per target and no confirmed web build.
- A layered answer is possible (3 as the fallback when the HRTF data or CPU
  is missing, or on the web), and it is a choice, not a finding.

## Recommendation (not a decision)

The constraint that separates the options is **ON by default**. A default-on
setting reaches people on speakers (Deck, phones, laptops, the web build), and
`cpal` gives no reliable headphone detection. Binaural rendering on speakers
colours the sound (comb filtering, a changed timbre), so a binaural option
(1a, 4 with an HRTF decode) is a poor default and a good opt-in.

Recommended shape, for the maintainer to accept or change:

1. **Default ON: option 3**, the cheap rear cue. It costs almost nothing, is
   safe on speakers and headphones, builds everywhere including the web, and
   needs only the forward axis plus a per-voice filter. It is a weaker cue, and
   that is the price of a safe default. Label it **chosen, not measured**.
2. **Setting values**: `original` (today's `pan_gains`, measured: the original
   cannot tell front from back), `rear cue` (option 3, the default), and a
   later `headphones` value for a binaural stage. The binaural value must say
   in its name that it is for headphones.
3. **Binaural stage later, as 1a with a SADIE II sphere** (Apache-2.0, the
   one dataset whose terms were read off the owner's record) once its licence
   file is read, or **4** if the 128-voice HD pool makes per-voice convolution
   too dear. 1a is pure Rust and cheapest to try; 4 scales flat. Choosing
   between them needs a listening prototype, which this lane could not make.
4. Do not pursue 1b, 1c, 1e, 1f; 5 only if a speaker-owner request appears;
   1d only if a native dependency per target is accepted.
5. Wait for item 6 of the table (Omega's 3D audio) before fixing the Omega
   value of `original`.

## Open

1. **The maintainer's pick**, and the "behind what" axis (camera or craft).
2. IRCAM Listen and CIPIC terms are **unsettled** (above). SADIE II needs its
   `LICENSE.txt` read from the download.
3. Wasm, Android and iOS builds of `hrtf`, `sofar`, `rustfft` and
   `audionimbus` were not tried here (no cross toolchains run, nothing
   built for those targets). `rustfft` has SIMD paths per architecture; the
   wasm one is unverified.
4. Steam Audio's WebAssembly support: not found in the README, the guide or a
   search. Unknown, not "no".
5. Omega's patch-1.03 3D audio is unread (`omega-3d-audio-read`). If it is a
   binaural or object-based path, it defines the `original` value for Omega
   and may favour one option.
6. **Deck cost is measured** (`docs/ghidra/functions/psp-pulse-usa/positional-audio.md`,
   "Cost on a Steam Deck"): a headphone voice is about 41 us per 512 frames (0.38 %
   of a core), 32 voices 8.6 %, a real Pulse race 1.11 % (`original`) against
   2.44 % (`headphones`), no dropped or late buffer. Done: `headphones` is capped at
   32 HRTF voices (the rest take `rear_cue`, crossfaded; HD-size mix 14.6 % -> 6.9 % of a core on the desktop, Deck confirmation not run). Open: an
   ended one-shot renders a 512-frame drain tail, probably why short cues cost
   more per voice (not profiled); the S24 is still an estimate; the count of live placed voices in a
   real race is unlogged (the Deck race above implies about 40). Measuring
   harness: a throwaway `crates/audio/examples/spatial_cost.rs` that builds
   a `Mixer`, plays N placed looping voices in a mode and times `render` over
   512-frame chunks, run on the Deck under `~/.cache/oag/deck.lock`.
7. Whether the click `hrtf` documents on fast-moving sources is audible on a
   passing craft: cannot be judged without playback, and no audio plays from
   spawned processes on this machine.
8. 2048/Omega check: this page is about the shared mixer, so it applies to
   every title; the Omega-specific part is item 5 above.

## Next Steps

1. Maintainer picks an option (or a layered pair) and the reference frame for
   "behind".
2. If 3: one `oag-wire` lane, S: forward axis in `Listener`, a per-voice
   biquad and gain from `dot(direction, forward)`, an `original` setting value
   that keeps `pan_gains` only, a golden test over the filter coefficients.
3. If 1a or 4: first settle the dataset licence (item 2), then a prototype
   that renders a WAV of one source circling the listener, so the cue can be
   judged by someone with headphones.
4. Whichever is picked: settle the cross-platform build question for the new
   dependency before wiring it.
