# The race mix saturates; the per-voice SAS volume would settle whether it should

2026-08-24. Numbers, chain and next step: [audio-levels.md](../docs/ghidra/functions/psp-pulse-usa/audio-levels.md). Two things not on it: `audio/sfx.rs:271` applies the engine law *after* the volume curve, against `spatial.rs`'s contract (fixing it makes the engine louder, so it waits); and `race::capture` never called `Audio::race_tick`, so race captures before this date dumped music alone.

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

## Resolved

- **The ordering fix landed.** `Engine::tick` (`crates/game/src/audio/sfx/engine.rs`)
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
  [`ppsspp-debugger.md`](../docs/reverse-engineering/ppsspp-debugger.md)),
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
  [audio-levels.md](../docs/ghidra/functions/psp-pulse-usa/audio-levels.md#the-per-voice-sas-volume-measured-live-2026-09-06)
  for the full capture data and the sum/loudest table, and
  [sound.md](../docs/ghidra/functions/psp-pulse-usa/sound.md#corroboration-2026-09-06-sas_setvolume-confirmed-live-and-the-value-it-carries-measured)
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
  [audio-levels.md](../docs/ghidra/functions/psp-pulse-usa/audio-levels.md#the-original-measured-live-2026-09-07-it-does-not-meaningfully-clip).

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
  of our own dump, confirming `crates/game/src/audio.rs`'s own documented
  claim that an unattenuated voice "round-trips the disc's own PCM
  sample-for-sample." So the decode is right, and something downstream of
  it is not attenuating the way the original's own playback does.

  **Cross-release caveat, stated rather than left implicit:** the original
  side is `pulse-psp-usa`; the port-side number above is `pulse-psp-eu.chd`.
  There is no reason to expect the releases differ in mix level, but this
  has not been corroborated, and it specifically weakens the *clip-rate*
  table above. It does not touch the RMS argument, which is entirely within
  our own port plus a raw disc decode and is release-independent.

## Open

- **The music path attenuates on the original during a race and does not on
  ours - mechanism unlocated.** `audio-levels.md`'s own `Audio_UpdateGroupVolumes`
  (`0x0893ac70`, confidence 82, already documented before this thread's
  2026-09-07 entry) runs **once a frame** and writes nine of the sixteen
  groups (`0, 2, 3, 4, 5, 0xa, 0xc, 0xd, 0xe`) from a per-frame fade scalar,
  not from `Audio_Init`'s one-time ceiling - so even the *baseline* these
  groups carry is not pinned at `1024` the way the idle Main-Menu read above
  found it. Group `0` additionally carries a second multiplier that ramps
  toward `1.0` or retreats toward a lower value on a flag, previously read
  only as "almost certainly music under speech" and left unchased since its
  trigger was unrecovered. This thread's 2026-09-07 measurement gives that
  mechanism a second, independent reason to be worth reading: whichever
  group turns out to be music, something needs to attenuate it *during a
  race* for the original's numbers above to make sense, and nothing yet
  measures what these groups do while racing rather than idle.
- **Three `Sas_CommitVoices` wrapper functions are still unnamed** -
  `FUN_08a2ae6c` (`__sceSasSetPitch`), `FUN_08a2af18` (`__sceSasSetNoise`)
  and `FUN_08a2b03c` (`__sceSasSetSL`), all resolved with confidence 90 -
  though the fourth, `FUN_08a2ae08` (`__sceSasSetVolume`), now is: named
  `Sas_SetVolume` as of 2026-09-06, confidence 92, following the
  `Sas_SetVoice` precedent for this exact "guarded call to a named import"
  shape. Not applied to the Ghidra project itself, since its function
  boundary is broken at that address (merged into an unrelated neighbour) -
  the `names.tsv` row is in regardless, per project convention, and the
  database can catch up whenever someone next has the project open for a
  reason that already touches this region.

## Next Steps

- **Read the group-volume table mid-race, not at rest.** The 2026-09-07
  read found all 15 groups at `1024` while sitting idle at the Main Menu;
  it was not re-read during the race capture itself (a `memory.read` while
  the CPU is running costs ~520 ms per `ppsspp-debugger.md`, which would
  have stalled the tick-bracketed timing that capture needed). Reading the
  same table live, mid-race, on `pulse-psp-usa`, is the direct test of
  `Audio_UpdateGroupVolumes`'s per-frame fade scalar and is the most
  promising single next step for the music-path question above.
- Decide and apply a naming convention for the remaining three wrapper
  functions, then add their `names.tsv` rows. Optional and separate from the
  volume question: tracing `SoundInstance_UpdateSpatial`'s SCREAM-instance
  write forward (or the SAS voice record's `+0x40` backward) until they meet
  would name the exact function that ramps a fading-in voice, which the
  2026-09-06 live capture saw happen but did not localise to an address -
  worth doing for the name, not because the saturation question is still
  open.
