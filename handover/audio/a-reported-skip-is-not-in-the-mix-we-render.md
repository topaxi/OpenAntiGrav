# A reported skip is not in the mix we render, and the jump counter cannot see it

2026-09-09. A player report - "audio seems to have some skips from time to
time" - came with a race's worth of `audio:` health lines reading **0 dropped,
0 late callback(s)** throughout, and jump counts swinging between 1 and 1,067
per window. Chasing that produced one landed fix and one still-undiagnosed
symptom, and the two are not the same thing.

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
  This stays a live candidate if the fault ever comes back.
- **No attack click.** Every one of the 43 waveforms a race loads starts at
  exactly 0.000, so a voice starting has nothing to click with.

**The jump counter is not the instrument to tune against.** `JUMP` is 0.25 and
`health.rs`'s own doc comment records the engine bed's 99.9th-percentile step as
0.287 - *above* the threshold - so legitimate content alone crosses it about a
hundred times a second, which the dump confirms at 106/s. Counts in the
hundreds per two-second window are what a loud eight-engine mix looks like.
**Do not raise the threshold to make the line quieter**: the number is honest,
it is the reading of it that was wrong.

**The one blind spot found and not closed:** `output::build`'s late check fires
only at `elapsed > owed * 2`, so a callback a whole buffer period late - a
PipeWire xrun, exactly the class of fault that sounds like a skip - is counted
as nothing at all. `0 late callback(s)` is therefore weaker evidence than it
reads as.

**Also unmodelled, and small:** the hardware loops back to the block flagged
`6` (block 1 on all 18 looping waveforms), while `Mixer::render` wraps to
sample 0, replaying block 0 each loop. All 18 begin at 0.000 and the post-trim
seam lands within 0.03 of the hardware's own on all but two, so this is a
fidelity gap rather than an audible one.

## Next Steps

1. **Get the artifact into a file.** `--tap-audio` records what the device is
   actually handed, which is the only thing that separates "our samples have a
   defect" from "our samples were fine and the device path glitched":
   `cargo run --release -p oag-game -- data/images/pulse-psp-eu.chd --race
   --tap-audio /tmp/skip.wav --tap-seconds 90`. It needs a real stream and a
   real-time loop, so it has to be a windowed run - the headless capture loop
   runs 3-5x real time and its tap would be meaningless.
2. If the tap is clean where a listener hears the skip, it is the device path.
   Tighten the late check first - `owed * 3 / 2`, or measure against a running
   expectation rather than the previous callback - and read `pw-top`'s ERR
   column beside it.
3. If the tap has the artifact in it, find its time offset and correlate
   against the tick log. `scripts/` has nothing for this yet; the throwaway
   used here scanned for zero runs, held-value runs, and step positions modulo
   the chunk size.
4. The loop start is a `loop_start` field on `Sound` and `Voice`, a changed
   wrap in `Mixer::render` (including its interpolation partner, currently
   `sound.frame(0)`) and the same treatment in `Mixer::seek`. Worth doing for
   the fidelity, not for the fault.
