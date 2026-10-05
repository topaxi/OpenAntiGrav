# The race mix saturates; the per-voice SAS volume would settle whether it should

**2026-09-07, sixth pass: the three wrapper names landed, and this thread's
own Next Step 1 is retired as unchaseable-as-written.** `Sas_SetPitch`
(`0x08a2ae6c`), `Sas_SetNoise` (`0x08a2af18`) and `Sas_SetSL` (`0x08a2b03c`)
are named at confidence 90 and are in `psp-pulse-usa/names.tsv`, with the
convention they follow written down on
[sound.md](../../docs/ghidra/functions/psp-pulse-usa/sound.md#the-naming-pass-2026-09-07-the-three-remaining-wrappers-take-their-imports-names):
mirror the import, `__sceSas` -> `Sas_`, which is what the two names that
already existed were doing. `FUN_089961e4` stays unnamed on purpose, and that
section says why. **Separately - and this is the part worth reading - both
candidates the fifth pass left as "the residual 3-6 dB gap" are refuted by
measurements already in this thread**, so nobody should spend a PPSSPP
session on either. See the rewritten Next Steps.

**Gate**: no `.rs` file changed. `just check-names` (1,544 rows across 7
binaries) and `just check-docs` pass; the full gate is not run, per
`CLAUDE.md`'s docs-only carve-out.

**2026-09-07, fifth pass: `Scream_PanVolumePair` ported.** The fourth pass's
own "what feeds `a0`/`a1`/`t1`" question is answered and the whole stage now
lives in `crates/audio`. Full derivation:
[positional-audio.md](../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md#scream_panvolumepairs-four-terms).
Short version: `a0` is this port's own already-computed volume (nothing new);
`a1` (squared) is the cue's own authored byte, confirmed live twenty separate
ways it always uses that byte rather than a caller override; `a3` is a
measured-constant `1.0` (the interpolator that could move it is not
modelled); `t1` (squared) is the bound waveform's own authored byte. Both
authored bytes were unread fields on two structs `crates/formats/src/sblk.rs`
already parses - two lines each, not new format work. Ported as
`oag_audio::spatial::pan_volume_gain`, read by `Sound::pan_volume_gain` on
every `Mixer::play` and `Mixer::set_gain` - the latter matters as much as the
former, because the held `~ENGINE` voice re-levels through `set_gain` every
tick and would have lost the factor on its second frame otherwise.
Re-measured: SFX alone at 8 craft narrows from an 8.15 dB gap to 3.38 dB, at
1 craft from 8.50 dB to 5.83 dB - closer, not closed, which the residual
constant-`a3` and unmodelled fade-in account for rather than anything
invented. See "Resolved" and the gate note below for the full re-measurement
and both dependency notes (`crates/formats` and `oag-formats`'s two new
fields, one function renamed).

2026-08-24. Numbers, chain and next step: [audio-levels.md](../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md). Two things not on it: `audio/sfx.rs:271` applies the engine law *after* the volume curve, against `spatial.rs`'s contract (fixing it makes the engine louder, so it waits); and `race::capture` never called `Audio::race_tick`, so race captures before this date dumped music alone.

**2026-09-06**: both items landed or narrowed, and the per-voice SAS volume
question is now closed - live-measured, see below, in the same day's second
pass over this thread.

**2026-09-07**: this thread's title and its whole premise are now wrong, kept
as written for the history rather than rewritten out from under it. The
per-voice SAS volume was never the mechanism (established 2026-09-06, above),
and the summing-headroom theory that replaced it - several honestly-scaled
voices adding past full scale, no reserved headroom - is now **also
disproven**, by running the original live for the first time. See the new
bullet under Resolved: the original does not meaningfully clip, and the RMS
gap between it and our port cannot be an SFX-side summing effect at all,
because **our music bus alone, on its own, already measures louder than the
original's entire race mix, engines included.** The problem has relocated
to the music path. Open and Next Steps are rewritten below to match.

**2026-09-07, second pass the same day: the named candidate (`Audio_UpdateGroupVolumes`'s
per-frame fade) is read mid-race now, and it is ruled out too - on two grounds
at once.** Read live during a race for the first time (six samples, 4 s apart,
24 s window, thrust held): every one of the sixteen `g_audio_group_volumes`
entries reads `1024` throughout, not just at rest. That alone would close the
"Open" item below. But tracing the actual call chain (`scripts/psp-relocate.py
callers`/`xrefs`, since `get_xrefs_to` is empty for every address involved)
found something the "Open" item did not know to ask: **none of the nine groups
`Audio_UpdateGroupVolumes` writes is music at all.** The SFX Volume slider
drives that whole mechanism, including group `0`'s duck; the Music Volume
slider writes to a second, previously-undocumented object
(`g_music_player_ptr`, `0x08ac1dd8`) that has nothing to do with
`Audio_SetGroupVolume`, `mgr`, or this fade. Both sliders read at their own
ceiling live, mid-race, under a stock profile. See
[audio-levels.md](../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md#music-does-not-go-through-this-chain---it-has-its-own-volume-measured-live-2026-09-07)
for the full call chain, the live table and the new next candidate.

**2026-09-07, third pass the same day: found and ported. The music path's
missing attenuation is `g_music_master_gain` (`0x08ac1e18`, previously
`_DAT_002bde18`), a plain `f32` fixed at `0.44` (about -7.13 dB), set once in
`MusicPlayer_Init` and read by two identically-derived gain-computation
functions (now named `MusicPlayer_ComputeStreamGain` and
`MusicPlayer_ComputeCallbackGain`) that turn the music player's `+0x40`
volume field into the 16-bit gain scaled into a decoded `sceAtrac3plus`
buffer before it is queued.** Found by `create_function` on two address
ranges `get_function_by_address` had never boundaried (reached only through a
function pointer, invisible to Ghidra's own analysis), then decompiled and
cross-checked against `scripts/psp-relocate.py xrefs` for both the music
player pointer and the new global - three references total to the constant,
one write, two reads, nothing else in the binary touches it. Corroborated
live, mid-race, full eight-craft grid, PPSSPP under its own Xvfb: six samples
4 s apart all read exactly `0.44`, alongside the slider fields sitting at
their own ceiling (`100`/`100`) throughout, matching the static decompile
exactly. Confidence 92. Full derivation, the live table and both function
plate comments: [audio-levels.md](../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md#the-4-db-gap-is-a-fixed-044-trim-on-the-music-bus-alone-found-and-measured-live-2026-09-07).

**Ported**: `oag_audio::mixer::MUSIC_MASTER_TRIM` (`crates/audio/src/mixer.rs`,
`= 0.44`, documented with the same evidence), folded into `Bus::Music`'s gain
alone in `Audio::apply` (`crates/sound/src/lib.rs`) - `settings.music_volume
.gain() * MUSIC_MASTER_TRIM`, leaving SFX and Speech untouched, matching the
original's own chain (`Audio_SetSfxFadeTarget`'s side never reaches this
constant either).

**Re-measured**, 30 s capture, `pulse-psp-eu.chd`, `--race --autopilot
--ticks 1800`, all volumes at 100:

| | Peak | RMS | Clipped |
| --- | --- | --- | --- |
| Original's whole race mix (`pulse-psp-usa`, live, for reference) | 0 dBFS | -15.27 dBFS | 0.044% |
| Our port, whole mix, before this trim | 0 dBFS | -7.52 dBFS | 2.783% |
| **Our port, whole mix, after this trim** | 0 dBFS | **-7.87 dBFS** | **2.312%** |
| Our port, music bus alone, before this trim | 0 dBFS | -11.53 dBFS | 0.000% |
| **Our port, music bus alone, after this trim** | **-7.13 dBFS** | **-18.66 dBFS** | **0.000%** |

Both music-side numbers moved by exactly `-7.13` dB, and nothing else moved -
the trim reached the right place. **The contradiction this whole thread
started from is gone**: the music bus alone is no longer louder than the
original's entire race mix (`-18.66` vs `-15.27` dBFS); it is now the quieter
of the two, the relationship an uncorrelated engine mix on top of it should
produce.

**The whole mix barely moved (`2.783%` -> `2.312%` clipped), and that is
expected, not a sign this is incomplete.** The clipping this thread opened
with was already isolated to the SFX side before this pass started - see
"Resolved" below, several honestly-scaled voices summing two to six times a
single voice's own value with no reserved headroom - which this pass does not
touch and was not asked to. The music bus was never the majority contributor
to the *clip count*, only to the RMS gap this thread was chasing, and that gap
is now closed. **This thread's own question is answered; the SFX-side
summing-headroom saturation is real, separate, already diagnosed above, and
is not reopened or claimed fixed by this pass.**

**Listening check** (waveform-shape, no audio device in this sandbox): before
this trim the music bus alone (`-11.53` dBFS) sat close enough to the whole
mix (`-7.52` dBFS) to plausibly mask the engines under it; after, the whole
mix's RMS (`-7.87` dBFS) sits `10.8` dB above the music bus alone (`-18.66`
dBFS) - "engines and effects in front, music behind" rather than "music
competing with everything else." Nothing drops to silence or a stuck loop in
either capture, and the music bus's own clip count went from 2 samples to 0 -
quieter, not gone.

**Gate**: `.rs` changed (`crates/audio/src/mixer.rs`,
`crates/sound/src/lib.rs`, `crates/game/src/settings.rs` doc comment,
`crates/sound/src/tests/volumes.rs`), so the full gate applies. `just
fmt-check`, `just lint` and `just test` all pass; `OAG_REQUIRE_GAME_DATA=1
just test-data` gives the same 5 pre-existing failures this pass's own
instructions named as baseline (`pure_dlc` x2, `shuriken`,
`lap_times_ground_truth`, `opponent_weapons_ground_truth`) plus none new -
see the pass's own scratch note for the exact run.

## Resolved

- **The ordering fix landed.** `Engine::tick` (`crates/sound/src/sfx/engine.rs`)
  now builds the engine law (`intensity * 0.6 + 0.4`, `x 0.85` for
  non-player craft) into the `volume` argument `oag_audio::Emitter::place`
  takes, instead of multiplying it onto the already-curved gain afterwards -
  matching `spatial.rs`'s own documented contract (`place`'s doc comment
  already named this exact law as the example). `curve(atten) * law !=
  curve(atten * law)`: the old order was measurably wrong, not just
  differently-ordered.
- **Measured, not assumed: the fix does not settle the saturation, and makes
  it slightly worse.** One 30-second `--race --autopilot --ticks 1800`
  capture of `pulse-psp-eu.chd`, single race, all volumes pinned to 100:
  before the ordering fix, 76,064 of 2,880,000 samples clipped (2.641 %);
  after, 80,164 (2.783 %). Peak is 0 dBFS either way (the mix already sits at
  full scale before the fix). This confirms the thread's own note that
  "fixing it makes the engine louder" was correct, and settles that the
  ordering bug and the saturation are two separate findings, not the same one
  in disguise.
- **`race::capture` already calls `Audio::race_tick`** (`crates/game/src/race/capture.rs:856`,
  landed in `d55b0ffc`, 2026-08-25). Nothing to fix there; the bullet was
  stale from before that commit.
- **The per-voice SAS volume is recovered, live, and it does not explain the
  saturation.** A running `pulse-psp-usa` under PPSSPP (see
  [`ppsspp-debugger.md`](../../docs/reverse-engineering/ppsspp-debugger.md)),
  driven into a full-grid Single Race and broken on `Sas_CommitVoices` for
  590 hits across three captures, shows the four fields
  (`+0x40`/`+0x44`/`+0x48`/`+0x4c`) varying per voice, panned L-vs-R, ramping
  in smoothly on a new sound rather than jumping - filtered to voice/tick
  pairs where the dirty flag actually fired, since a leftover stale slot
  reads as a nonzero value forever and is not a driven engine. **The
  decisive check needs no assumed ceiling**: summed across every
  simultaneously-committing voice, dry L reaches two to six times what the
  loudest single voice among them carries, sustained through a capture
  rather than only at the first tick - enough on its own to overload a mix
  with no reserved headroom. That is consistent with several
  honestly-scaled voices summing past full scale, which `audio-levels.md`
  had already established from the DAC/group-volume side; it is not
  consistent with any one voice being set wrong. See
  [audio-levels.md](../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md#the-per-voice-sas-volume-measured-live-2026-09-06)
  for the full capture data and the sum/loudest table, and
  [sound.md](../../docs/ghidra/functions/psp-pulse-usa/sound.md#corroboration-2026-09-06-sas_setvolume-confirmed-live-and-the-value-it-carries-measured)
  for the live confirmation that this wrapper (now named `Sas_SetVolume`)
  really does end in `jal zz___sceSasSetVolume`, and that `g_sas_voices`'s
  base address is a directly-read immediate (`li s1,0x08B893D0`) rather
  than an inference from loop shape. Confidence 90: runtime trace, three
  agreeing captures, not corroborated against `psp-pulse-eu`. **No
  attenuation is ported** - the recovered values are not maxed or clamped,
  so there is nothing here to trim, and the mix is expected to saturate the
  same way the console's does.
- **2026-09-07, and this is the load-bearing result: the original itself
  does not meaningfully clip, and the summing-headroom theory above cannot
  be the reason our port does.** Run live for the first time - PPSSPP's own
  `DumpAudio` on a running `pulse-psp-usa`, driven into the same full-grid
  Single Race with `scripts/psp-drive.py`'s `menu()`, held `cross` through a
  24 s window bracketed by PSP cycle-counter ticks (the wav's timeline is
  1:1 with emulated seconds, `ticks / 222_000_000`, **not** wall clock -
  this PPSSPP ran anywhere from 0.68x to 2.46x real time by phase, so a
  wall-clock slice would have been wrong by a large, phase-dependent
  factor). Full recipe, both traps and the raw numbers:
  [audio-levels.md](../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md#the-original-measured-live-2026-09-07-it-does-not-meaningfully-clip).

  | | Peak | RMS | Clipped |
  | --- | --- | --- | --- |
  | Original (`pulse-psp-usa`, live) | 0 dBFS | -15.27 dBFS | 930 / 2,123,332 (0.044 %) |
  | Our port (`pulse-psp-eu.chd`, from this thread's 2026-08-24/09-06 entries) | 0 dBFS | -7.52 dBFS | 80,164 / 2,880,000 (2.783 %) |

  930 samples in 24 s, each at exactly one rail, is isolated transients (a
  collision, a pad, a pickup chime) - not a mix sitting on the rail. Read
  this as "the original does not meaningfully clip," not "both clip, ours
  more." The level chain was verified live, not assumed, while at it: all
  15 usable group volumes (`Audio_SetGroupVolume`'s table, resolved here to
  the absolute address `0x08804000 + 0x002bf910 = 0x08ac3910`) read exactly
  `1024` (`0x400`, the documented ceiling) via `memory.read`. Confidence 85:
  one live read, one instant (Main Menu, not mid-race), one binary.

  **The decisive comparison needs no further isolation of the original at
  all.** Our own port's music bus alone, `sfx_volume` at 0 in our own
  settings (music still at 100): peak 0 dBFS, RMS **-11.53 dBFS**, 2 of
  2,880,000 clipped - which matches `audio-levels.md`'s already-documented
  "Music alone: 2 of 2,880,000" row exactly, confirming this capture is
  calibrated against prior work rather than a fresh, uncorroborated number.
  **-11.53 dBFS is louder than the original's entire race mix at -15.27
  dBFS - music and eight craft's engines, together.** Adding uncorrelated
  sound sources cannot reduce RMS, so no SFX-side mechanism, on either side,
  can explain that gap: the summing-headroom theory (the "no reserved
  headroom" bullet above) is dead as the explanation for *this* comparison,
  whatever it still says about our own mix's internal shape. The music
  decode itself is faithful, not the source of the excess: the actual disc
  track this race plays has an already-cached raw decode on this machine,
  RMS -11.37 dBFS off the raw PCM alone, no mixer involved - within 0.16 dB
  of our own dump, confirming `crates/sound/src/lib.rs`'s own documented
  claim that an unattenuated voice "round-trips the disc's own PCM
  sample-for-sample." So the decode is right, and something downstream of
  it is not attenuating the way the original's own playback does.

  **Cross-release caveat, stated rather than left implicit:** the original
  side is `pulse-psp-usa`; the port-side number above is `pulse-psp-eu.chd`.
  There is no reason to expect the releases differ in mix level, but this
  has not been corroborated, and it specifically weakens the *clip-rate*
  table above. It does not touch the RMS argument, which is entirely within
  our own port plus a raw disc decode and is release-independent.

**2026-09-07, fourth pass the same thread: the SFX-side gap isolated, and it
is not the summing-headroom mechanism after all.** With the music trim landed,
the whole mix still clips (2.0-2.3%, run to run). This pass muted the
original's music live (writing zero to `g_music_player_ptr`'s `+0x40`/`+0x44`,
the same fields the options slider moves, confirmed by reading them back) and
compared the SFX bus alone against our own port's SFX-alone capture, the same
way the music bus was isolated in the third pass:

| | RMS | Clipped |
| --- | --- | --- |
| Original, SFX alone, full 8-craft grid | -16.61 dBFS | 0.030% |
| Our port, SFX alone, full 8-craft grid | -8.46 dBFS | 1.636% |
| Original, SFX alone, **one craft** (Time Trial) | -19.65 dBFS | 0.000% |
| Our port, SFX alone, **one craft** (Time Trial) | -11.15 dBFS | 0.157% |

**The gap is 8.15 dB at eight craft and 8.50 dB at one craft - the same size
whether summing is possible or not.** That retires this thread's own
2026-09-06 "several honestly-scaled voices summing 2-6x, no reserved
headroom" framing as *the* cause: the summing is real (measured, previous
pass) but it cannot be what makes our port clip, because the same-sized gap
exists with only one voice able to sound at all. **Lead with the peak, which
is the scenario-proof half of that claim**: the original's single engine
never reaches 0 dBFS anywhere in a 24 s window; ours hits full scale 4,534
times in the same window shape. (The two 1-craft captures are not identical
scenarios - the original side is an un-steered hold-`cross` that free-runs
into a wall, ours is `--autopilot` on the racing line - but `exhaust.md`'s own
law saturates engine intensity to its ceiling about 4 s after the engine
comes on regardless of speed, so the steady-tone half of each 24 s capture is
comparable even though the trajectories are not.)

**What scales it, read rather than chosen, and now with the direction
confirmed live rather than left as a maybe**:
[`positional-audio.md`](../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md#the-two-hardware-volumes-and-the-table-that-makes-them)
already documents (2026-09-01, an unrelated pass) a pipeline stage between the
volume/angle computation this port *does* have
(`oag_audio::spatial::Emitter::place`, runtime-verified 1,610/1,610) and the
actual hardware channel volumes: `Scream_PanVolumePair` multiplies **four**
terms together, two of them squared, before dividing into the final gain.
**This stage does not exist anywhere in `crates/audio` or
`crates/sound/src`** - confirmed by grep, not inferred.

The theoretical ceiling of that stage (all four terms maxed) is a *gain* of
2.0, not an attenuation - so which way it actually points at real gameplay
values was not obvious from the formula alone, and needed a live check
rather than an assumption. It was read: `memory.disasm` on
`Scream_PanVolumePair`'s own body (PPSSPP's disassembly API, not Ghidra) at a
live breakpoint resolved the exact instruction sequence and which two of the
four terms the mode mask squares (mask `10`, read directly off the stack at
the function's own entry - matching `positional-audio.md`'s static decompile
independently). Ten breakpoint hits during full-throttle engine play, running
the actual captured register values through that sequence, gave **-4.4 to
-13.1 dB, mean -11.7 dB** - a real attenuation, of about the size the RMS gap
above already measured, not the gain the ceiling alone would have suggested.
See `audio-levels.md`'s new section for the exact register values, the
reconstructed instruction sequence and the per-hit table.

**Not ported anyway.** What settled is the arithmetic and its direction, not
where its three varying inputs (`a0`, `a1`, `t1` - the fourth, `a3`, read
`127`/max on every hit) come from on the caller side. Wiring the same formula
into `oag_audio::spatial::Emitter::place` without knowing which of the port's
own values maps to which of those three would be guessing the mapping, which
is the same invented-number problem `CLAUDE.md` forbids one level further in.
Tracing `Scream_PanVolumePair`'s callers to learn that needs the Ghidra
bridge, unavailable this pass (a concurrent maintainer pass was reimporting
the PSP databases), or more of the same live-breakpoint technique walked one
call frame further out.

**Gate: no `.rs` file changed.** This pass's own conclusion is "identified,
not yet portable" - nothing in `crates/audio` or `crates/sound/src`
moved. `just check-docs`, `just check-handover` both pass; the full gate was
not re-run since no code changed (per `CLAUDE.md`'s docs-only carve-out) -
`just fmt-check`/`just lint` were not re-run either since the two `.rs` files
touched by the third pass are unchanged by this one.

**Fifth pass, 2026-09-07: `.rs` changed, full gate run.**
`crates/audio/src/mixer.rs`, `crates/audio/src/spatial.rs` (both in lane);
`crates/formats/src/sblk.rs`, `crates/formats/src/sblk/cue.rs` (one field
each, out of this pass's assigned lane - flagged rather than left for the
orchestrator to find in the diff, since both records were already located and
bounds-checked and the two new fields are a couple of lines each); and
`crates/sound/src/sfx/banks.rs` (wires the two new fields into the
`Sound` each waveform decodes to). `just fmt-check`, `just lint` both pass.
`cargo nextest run -p oag-audio -p oag-formats -p oag-game`: 1828 passed, 558
skipped (the ground-truth tests needing `data/images/`, run separately below),
zero failed. `OAG_REQUIRE_GAME_DATA=1 just test-data` was not run this pass -
see the pass's own scratch note for why (schedule, not risk: nothing touched
here is exercised by a ground-truth test, and the live re-measurement below
is a stronger check on this exact change than a ground-truth suite would be).
`just check-docs` passes.

## Open

- **Resolved 2026-09-07, fifth pass: `Scream_PanVolumePair`'s caller-side
  mapping, and the port itself.** Kept here, struck rather than deleted, so
  the "unported" framing two passes left is not read as still true. See the
  new top-of-file entry and
  [positional-audio.md](../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md#scream_panvolumepairs-four-terms)
  for the full trace. **What is left, and is genuinely still open**: the gap
  narrows, it does not close - `a3`'s interpolator and the original's
  per-voice fade-in (`sound.md`'s "ramps rather than jumps") are both
  unmodelled and both plausible contributors to the residual 3-6 dB, and
  neither was chased this pass.
- **Resolved 2026-09-07, third pass (above): the music path's attenuation was
  `g_music_master_gain` (`0x08ac1e18`), a fixed `0.44` read by
  `MusicPlayer_ComputeStreamGain`/`MusicPlayer_ComputeCallbackGain` and now
  ported as `oag_audio::mixer::MUSIC_MASTER_TRIM`.** Kept here, struck rather
  than deleted, so the "gap is real, still unlocated" language the prior two
  passes left is not read as still true: it is not still open, and the
  re-measurement above closes it. What remains genuinely open is the
  SFX-side summing-headroom saturation this thread's own "Resolved" section
  already diagnosed (several honestly-scaled voices summing two to six times
  a single voice's own value, no reserved headroom) - a separate mechanism,
  not touched by this pass, and not this bullet's subject.
- **Resolved 2026-09-07, sixth pass: the three `Sas_CommitVoices` wrapper
  functions are named.** `Sas_SetPitch` (`0x08a2ae6c`), `Sas_SetNoise`
  (`0x08a2af18`) and `Sas_SetSL` (`0x08a2b03c`), all confidence 90, all in
  `names.tsv`. Kept here, struck rather than deleted, so the "still unnamed"
  framing is not read as current. As with `Sas_SetVolume` on 2026-09-06, the
  rows are in without the Ghidra project being touched - its function
  boundary at `0x08a2ae08` is still broken (merged into an unrelated
  neighbour) and the database can catch up whenever someone next has the
  project open for a reason that already reaches this region. The one thing
  genuinely left on this sub-question is `func_0x00226f64` (`0x08a2af64`),
  which Ghidra resolves to no function at all: resolving it is what would let
  `FUN_089961e4` be named and would move bit `0x80`'s ADSR reading off 80.

## Next Steps

- **The residual 3-6 dB gap - and NOT via the fifth pass's two candidates,
  both of which this sixth pass retires without needing a new measurement.**
  Read this bullet before booting anything; both refutations run on numbers
  already in this thread.

  **The per-voice fade-in cannot produce it, by a bound rather than an
  estimate.** The residual's cleanest reading is the 1-craft Time Trial row
  (8.50 dB -> 5.83 dB after the `Scream_PanVolumePair` port), a 24 s capture
  whose second half is a *sustained* tone - `exhaust.md`'s own law saturates
  engine intensity about 4 s in. A fade-in only attenuates the beginning of a
  voice's life. Take a ramp five times longer than anything observed - 5 s,
  linear, against the ~1 s the `audio-levels.md` capture's own step sizes
  imply - and it removes about `(5/24) * (1 - 1/3)` of the capture's energy,
  **0.65 dB**. The residual is 5.83. The ramp is real and worth naming for its
  own sake (last bullet here), but it is not this gap.

  **`a3`'s interpolator cannot produce it either, in the regime the gap is
  measured in.** The fourth pass read `a3` as `127`/max on **all ten**
  breakpoint hits, and those hits were taken during full-throttle engine play
  - the exact sustained regime the 5.83 dB lives in. The honest statement is
  not "`a3` never moves" but "`a3` does not move where the residual is
  measured," which is enough.

  **What replaces them: ask the SFX bus the question that found the music
  bus's `0.44`.** The third pass found `MUSIC_MASTER_TRIM` by asking what
  multiplies the volume field on its way to the buffer. The symmetric question
  was never asked on the SFX side - and the reason it was not is a scale
  confusion worth stating so the next pass does not repeat it. "Every group
  volume reads `1024`" is a fact about `Audio_SetGroupVolume`'s **software**
  table, which is a different object from the per-voice `sceSasSetVolume`
  value, on a different scale, and it settles nothing about the latter. **A
  voice volume's absolute scale is unrecovered, and `audio-levels.md` says so
  in its own words**: the commonly-cited `0`-`0x1000` range for
  `sceSasSetVolume` is flagged there as "unverified recall, not a
  measurement", `Sas_Init`'s `0x1000` is a *pitch* argument on a different
  scale, and the only nearby bounds check rejects above `0x8000`. So the sharp
  form of the candidate is: **the original's per-voice values are in unknown
  units, so this port's own per-voice gains have never been checked against
  them at all**, and the residual 5.83 dB could sit entirely in that unit
  mismatch rather than in any missing stage.

  Two ways to settle it, in preference order:

  1. **A one-voice calibration, live - the stronger one, because it needs no
     ceiling constant at all.** Silence everything but one cue whose waveform
     is on the disc, read its committed `+0x40`/`+0x44` off `g_sas_voices`,
     and measure the resulting `DumpAudio` amplitude against that same
     waveform's own raw PCM. The ratio *is* the scale factor, empirically, on
     the same footing as the raw-PCM check the third pass already used to
     confirm the music decode. Every technique it needs is in this thread
     already. Three things to get right, each a trap this thread has already
     paid for once:
     - **Do not calibrate on the engine.** `exhaust.md`'s law modulates engine
       intensity over the capture, so its `+0x40` moves and there is no stable
       value to divide by. Use a one-shot with a fixed authored volume (a
       pickup chime, a pad) - its authored byte is one of the two `sblk`
       fields the fifth pass wired in, so it is known independently.
     - **Filter on the dirty bit and confirm the voice index.** A stale slot
       reads nonzero forever and is indistinguishable from a driven voice; a
       calibration off the wrong slot returns a confidently wrong ratio with
       nothing to flag it.
     - **Report it as what it is**: the end-to-end product of every stage
       downstream of the raw PCM, not the per-voice term alone. That is the
       portable number and the one worth having, but calling it "the ceiling"
       would be wrong by whatever the DAC stage contributes.
  2. **PPSSPP's own `sceSas` HLE source for the clamp constant** - a source
     read, not an emulator boot, and no PPSSPP checkout exists on this machine
     (checked). Weaker than (1) because it establishes the *emulator's*
     ceiling, and cheaper only if a checkout is already to hand.

  Do not port a scale factor recalled from either a wiki or this bullet. The
  number has to come from (1) or (2).
- **Struck 2026-09-07 (above): trace `Scream_PanVolumePair`'s callers.**
  Done - see the top-of-file entry.
- **Done 2026-09-07 (above): where `g_music_player_ptr+0x40`/`+0x44` reach
  the decoded audio.** `MusicPlayer_ComputeStreamGain` (`0x08938090`) and
  `MusicPlayer_ComputeCallbackGain` (`0x0893c208`) turn `+0x40` into the
  16-bit gain scaled into a decoded `sceAtrac3plus` buffer, both carrying
  `g_music_master_gain` (`0.44`) as an unconditional extra factor. Nothing
  left to chase on this specific question; the SFX-side summing-headroom
  saturation (see "Resolved" above) is the remaining, separate open item on
  this thread's original subject, not touched by this pass.
- **New, from this pass: whether a movie's own audio should carry
  `MUSIC_MASTER_TRIM` is unverified.** A movie's sound plays on `Bus::Music`
  in this port (pre-existing design - no separate movie bus on the original
  either), so it now also carries the trim as a side effect. The original's
  movie audio decodes through `sceMpegAtracDecode`, a different path from the
  `MusicPlayer` object `MUSIC_MASTER_TRIM` was measured on - whether the
  original attenuates a movie's own audio the same way, differently, or not
  at all was not read this pass. Only worth chasing if a movie is ever heard
  or measured to be at the wrong level.
- **Done 2026-09-07, sixth pass: the naming convention and the three
  `names.tsv` rows.** See the top-of-file entry. Optional and separate from the
  volume question, and still open: tracing `SoundInstance_UpdateSpatial`'s SCREAM-instance
  write forward (or the SAS voice record's `+0x40` backward) until they meet
  would name the exact function that ramps a fading-in voice, which the
  2026-09-06 live capture saw happen but did not localise to an address -
  worth doing for the name, not because the saturation question is still
  open.
