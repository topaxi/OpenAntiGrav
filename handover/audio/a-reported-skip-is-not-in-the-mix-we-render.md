# A reported skip is not in the mix we render, and the jump counter cannot see it

2026-09-09. A player report - "audio seems to have some skips from time to
time" - came with a race's worth of `audio:` health lines reading **0 dropped,
0 late callback(s)** throughout, and jump counts swinging between 1 and 1,067
per window. Chasing that produced one landed fix and one still-undiagnosed
symptom, and the two are not the same thing.

**2026-09-15: the tap was taken, twice, and both Next Step 1 and the late-check
blind spot are closed.** A maintainer racing HD/Fury on an idle machine read a
race's worth of `audio:` warnings - `0 dropped, 0 late` on every line, jumps
and clips moving, `worst ... at frame 136` and `491` recurring - as the machine
struggling. `--tap-audio` runs of Pulse (30 s, Talon's Junction) and HD (40 s),
both `--race --autopilot`, windowed, 48 kHz, 1,024-frame buffers:

- **No gap and no held sample.** Longest run of zeros after the race starts:
  1 frame, in both. Longest held run outside the leading silence: 43 frames,
  and it is at `-32768` - the clamp's own flat top, not a stall.
- **The 512-frame `CHUNK_FRAMES` seam is clean.** Steps over 0.25 fall
  uniformly modulo 512, 1,024 and 800 (Pulse: bin 0 holds 9 against a mean
  of 7.6; HD has too few events to bias at all). The candidate this thread
  left "untested and cannot be, this way" is tested, and is not it.
- **The recurring `frame 136` is a cue's own attack.** The samples around
  every one of them are a full-scale transient decaying off the clamp
  (`1.0 1.0 0.892 0.6 0.309 ...`), which is what a one-shot's attack looks
  like and what a seam does not. That it recurs at one offset is what the
  mixer's own structure predicts: a `play` lands between two `Mixer::render`
  passes, so a voice always starts at a 512-frame chunk boundary and its
  attack sits a fixed distance into a chunk. The tap is *consistent* with
  that - in HD, offsets 136 and 446 (mod 512) hold four events each against
  a mean of 0.4, and the maintainer's run on another machine had its
  windows' worst at 136/137 and 491 as well - but four events in a bin is
  about an 8 % fluke on its own, so read it as the prediction holding, not
  as proven. What would prove it: log the tick a `play` was issued on and
  correlate against the event's frame index.
- **Rates, for the record.** Pulse: 130 steps/s over 0.25, 0.127 % clipped.
  HD: 5.5/s, 0.127 % clipped. Both taps' 99.9th-percentile step is under
  0.30, consistent with the 0.287 `health.rs` already records.

What changed in code: `Health::report` now logs at `warn` only when a
**fault** moved (dropped, late, refused) and at `trace` when only the content
readings did, with the clip count also given as a share of the window's
samples; the counters and both thresholds are exactly as they were. And the
late check is `owed * 3 / 2` rather than `owed * 2` - at that bound a 60 s HD
race counted one late callback, the race's own load stall (`frame: 138.5 ms`
beside it, `0 dropped` because the ring covered it), with `pw-top` reporting
zero xruns for the client throughout. `0 late callback(s)` now rules out a
callback half a period late, not only one a whole period late.

## What landed

**The PS-ADPCM run-out block was being played, and on a looping waveform it was
played once per loop.** `docs/formats/psp-audio.md` already recorded that the
encoder "flags the last block it wrote and appends a block of run-out", 595
spans out of 595 - but `load_named_cue` handed `decode_adpcm` the whole span,
so every voice played one block past the terminator the hardware stops at.
`oag_formats::sblk::adpcm_played` is the trim, and it looks at the last two
blocks alone: the census was widened to all four PSP images first - 1,980 spans,
every one terminated on the last block or the one before it - and the PS2 and
PS3 discs were not surveyed, so on those the trim is a no-op rather than a
guess. The measurements, and the loop seams either side of it, are on
[psp-audio.md](../../docs/formats/psp-audio.md#the-run-out-block-is-not-audio-and-playback-has-to-trim-it).

**The reporter cannot hear it any more.** After the trim landed, the same
player who raised it listened again and found no clicks or skips - while the
`audio:` warning lines carried on exactly as before, which is the counter and
not the mix (see "the jump counter is not the instrument", below). That is one
person's ears rather than a measurement, and it is the only evidence connecting
the trim to the symptom: nothing in the capture below changed audibly, and the
`--tap-audio` capture that would have caught the artifact was never taken. So
treat the fix as **correct on its own evidence and plausibly the cause**, not as
a diagnosed one.

## Open

**The reported skip was never reproduced under an instrument.** What was ruled out, on a 60 s deterministic `--dump-audio` capture of
a Talon's Junction race (`--race --autopilot --ticks 3600`):

- **No gaps.** Not one run of zeros longer than 30 frames in 2.88 M, and no
  held-sample run longer than 29. A skip that a player hears as a dropout is a
  gap, and the mixer is not producing one.
- **No control zipper at the tick boundary.** Steps over 0.4 are uniform in
  their position modulo 800, the frames one tick renders at 48 kHz - so the
  per-buffer constant gain and pan in `Mixer::render` are not stepping audibly
  *there*. **The real-time path's own 512-frame `CHUNK_FRAMES` boundary was not
  tested and cannot be, this way**: `--dump-audio` forces the null backend,
  which renders one tick per call and has no such boundary. The user's run did.
  *Tested 2026-09-15 on the real path, via `--tap-audio` - clean; see the top
  of this file.*
- **No attack click.** Every one of the 43 waveforms a race loads starts at
  exactly 0.000, so a voice starting has nothing to click with.

**The jump counter is not the instrument to tune against.** `JUMP` is 0.25 and
`health.rs`'s own doc comment records the engine bed's 99.9th-percentile step as
0.287 - *above* the threshold - so legitimate content alone crosses it about a
hundred times a second, which the dump confirms at 106/s. Counts in the
hundreds per two-second window are what a loud eight-engine mix looks like.
**Do not raise the threshold to make the line quieter**: the number is honest,
it is the reading of it that was wrong.

**Closed 2026-09-15 (kept so the "weaker evidence" reading is not repeated):**
`output::build`'s late check fired only at `elapsed > owed * 2`, so a callback
a whole buffer period late - a PipeWire xrun, exactly the class of fault that
sounds like a skip - was counted as nothing at all. It is `owed * 3 / 2` now;
see the top of this file for what that measured.

**Also unmodelled, and small:** the hardware loops back to the block flagged
`6` (block 1 on all 18 looping waveforms), while `Mixer::render` wraps to
sample 0, replaying block 0 each loop. All 18 begin at 0.000 and the post-trim
seam lands within 0.03 of the hardware's own on all but two, so this is a
fidelity gap rather than an audible one.

## Next Steps

1. ~~Get the artifact into a file.~~ Done 2026-09-15, twice, with no artifact
   in either; the recipe stays here because it is the one that works:
   `cargo run --release -p oag-game -- data/images/pulse-psp-eu.chd --race
   --autopilot --tap-audio /tmp/skip.wav --tap-seconds 30`, windowed - the
   headless capture loop runs 3-5x real time and its tap would be meaningless.
2. ~~Tighten the late check.~~ Done 2026-09-15, `owed * 3 / 2`. The running
   expectation variant was not taken: a sound card's clock and the wall clock
   drift, and an expectation that is never re-anchored turns that into a
   false late callback some minutes into a long session.
3. If a skip is ever heard again, tap it and correlate the offset against the
   tick log. `scripts/` has nothing for this yet; the throwaway used both
   times scanned for zero runs, held-value runs, and step positions modulo
   the buffer, the chunk and the tick - and, the part that identified the
   recurring offset, the exact frame distance between events at the same
   offset.
4. The loop start is a `loop_start` field on `Sound` and `Voice`, a changed
   wrap in `Mixer::render` (including its interpolation partner, currently
   `sound.frame(0)`) and the same treatment in `Mixer::seek`. Worth doing for
   the fidelity, not for the fault.

## From the HANDOVER.md index (moved 2026-09-25)

the PS-ADPCM run-out block was played once per loop and is now trimmed; the reporter can no longer hear the skip, and 2026-09-15's two `--tap-audio` runs (Pulse and HD, windowed) found no gap, no held sample and a clean 512-frame chunk seam, with the recurring `frame 136` shown to be a cue's own attack on the 1,024-frame trigger grid. The `audio:` line is a `warn` only for a fault (dropped, late, refused) now and `trace` for content-only windows, and the late check is `owed * 3 / 2`. Open only for the loop-start fidelity gap and the "if it ever comes back" recipe
