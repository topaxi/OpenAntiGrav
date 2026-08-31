# The race thumps, the mix is clean, and the device buffer was 10.7 ms

Reported from play on 2026-08-31: a thump under load, *"a bit like a bad bass
drum"*, alongside `[WARN ] [calloop] Received an event for non-existence source`
in the terminal. Refined over the session, by the person who could hear it:

- it happens **every few seconds**, not once;
- **collision sounds trigger it much more often**;
- it **cuts a playing cue** - "speed pad boost sound abruptly stops";
- and, decisively, *"sounds a bit like the bump happens when a sound stops"*;
- **no log line ever accompanies it**, on either the old code or this branch;
- **vastly more demanding games on the same machine are clean**.

## What the calloop line is, and why it is not this

winit's Wayland backend removes and re-inserts a key-repeat timer source on
every key press (`winit-0.30.13/src/platform_impl/linux/wayland/seat/keyboard/mod.rs:167`,
and the `remove` twenty lines above it). Slot 3 is the first slab slot after the
three sources winit inserts permanently at startup - the Wayland queue, the
user-event channel and the awakener - so `id: 3, version: 10` is that timer on
its tenth reuse. Under load a dispatched batch still carries an event for the
token just removed, and calloop warns. The dropped event is a stale repeat tick
for a key this game tracks itself through `Controls::set_key`. It is filtered to
`calloop=error` in `init_logging` (`crates/game/src/main.rs` and
`crates/view/src/logging.rs`); `RUST_LOG=warn,calloop=warn` puts it back.

## What was measured, and what each measurement ruled out

**The mixer's arithmetic is not saturating.** A 60 s autopilot race dumped with
`--dump-audio` (`cargo run --release -p oag-game -- data/images/pulse-psp-eu.chd
--race --autopilot --screenshot /tmp/r.png --ticks 3600 --dump-audio /tmp/r.wav`)
peaks at 12,588/32,767 - **-8.3 dBFS, zero full-scale samples**. Clipping was a
live theory and this retires it for that run. Note the dump forces the null
backend by construction, so it exercises `render_tick` and not the callback.

**A cue running to its own end does not click.** Probed off
`pulse-psp-eu.chd` through `Banks::load`: every one-shot's last sounding sample
is under 0.02 of full scale - `SPEEDUPPAD` and `.COLLISIONS` are exactly 0.0000.
So "the boost sound abruptly stops" is not the boost sound ending.

**But every cue is loud, and the held ones are cut mid-waveform.** The same
probe: `SPEEDUPPAD` peaks at 0.955, `.COLLISIONS` at 0.993, `~ENGINE` at 0.958,
`~SHIELD` at 0.949, `~ROCKLOCK` at 0.949. `Mixer::stop` cleared the slot
outright, so stopping any of those put a step from most of full scale to zero
into the mix, at whatever point the playhead had reached. **That is the one
thing in the code that matches "the bump happens when a sound stops"**, and it
is what this branch now fixes - see `RELEASE_FRAMES` in `crates/audio/src/mixer.rs`.

**The device path is clean at idle.** `cargo run -p oag-audio --example
device-report --release -- 8` on this machine: `Default Audio Device`, 48 kHz,
**512-frame buffer (10.7 ms)**, and over 8 s of a looping tone through the real
mixer, 0 dropped, 0 jumps, 0 late callbacks. That is a floor, not a race.

**Real-time scheduling is not the answer.** PipeWire's `data-loop.0` genuinely
runs SCHED_OTHER here (no rtkit, no `realtime` group, `ulimit -r` 0), which was
a live theory until the report that heavier games on the same machine are fine.
Recorded because it is true and someone will find it again, not because it
explains this.

**cpal's `realtime` feature was tried and reverted** - do not re-add it without
reading this. On Linux `audio_thread_priority` promotes a thread *only* through
rtkit over D-Bus (`rt_linux.rs:92`), and cpal declares the crate
`default-features = false`, so plain `features = ["realtime"]` compiles to the
blanket no-op fallback (`audio_thread_priority-0.35.1/src/lib.rs:100-120`) and
does nothing at all. The feature that works is `realtime-dbus`, which pulls
libdbus into the workspace and still does nothing on a machine with no rtkit.

## The measurement that settled it

Played on this branch with the instrument in, 2026-08-31:

```
[ERROR] audio: output stream error: A buffer underrun or overrun occurred.
[WARN ] audio: 0 dropped, 0 jump(s) in the mix, 6 late callback(s)
        (worst 81.1 ms over), 0 voice(s) refused, 0 sample(s) clipped
        - of 2975 buffer(s) of 512 frame(s) so far
```

Read the columns. **The mix is clean** - nothing dropped, nothing
discontinuous, no voice refused, not one sample clipped - and the callback is
arriving up to **96 ms late against a 10.7 ms buffer**, with ALSA reporting the
xrun that follows. Nothing about the samples was ever wrong. There were simply
not enough of them queued to survive a stall.

512 frames is what `default_output_config` chose and what `Output::open` took
verbatim. `TARGET_BUFFER_SECONDS` now asks for 40 ms instead, clamped into
whatever range the device offers: 1,920 frames here, measured through
`device-report`. That is 3.7x the slack against the same stall.

**It is a mitigation, not a cure.** A 96 ms gap is longer than 40 ms of buffer,
so the deepest stalls will still be heard; what changes is that the ordinary
ones stop being.

## Two hypotheses tested and refuted, so nobody tests them twice

**The engine's loop is not the seam.** The player's own guess was that `~ENGINE`
loops and the wrap is the thump - it is 1.211 s long and held for the whole
race, which fits "regular, every few seconds, even if I hit nothing" exactly.
It is wrong, and measurably so: rendered four loop periods through a real
`Mixer`, the step across the wrap is **0.002 of full scale** against a median
step of 0.055 and a 99.9th percentile of 0.287 in the same bed. The sample was
authored to loop over its whole decoded buffer and it does.

**And the PS-ADPCM loop flags are not usable as read.** Every block carries a
flag byte at `+1` which this crate decodes past;
`pulse-psp-eu.chd`'s `~ENGINE` is 1,907 blocks flagged `0, 6, 2 x 1903, 3, 255`
- a lead-in, a loop-start mark, the body, a loop-end mark, and a trailing block
whose flag is not one of the eight defined values. Honouring those marks - loop
28..53,368 rather than 0..53,396 - was implemented, measured, and **reverted**:
it takes the seam from 0.002 to **0.176**, sixty times worse. Either the marks
are one block off from the conventional reading or the encoder set them
decoratively; what is certain is that the waveform's own seam is at 0 and the
flags do not point at it. `.COLLISIONS` is flagged the same way
(`0 x 434, 1, 7`) and its terminator block decodes to silence, so it costs
nothing there either.

## Open

**Every counter says the mix is clean, and the thump is still there.** The run
after the full-buffer scan landed produced **no `audio:` line at all** - no
dropped buffers, no steps over a quarter of full scale anywhere in any buffer,
no late callbacks, no refused voices, nothing clipped, and no worst-step large
enough to report on its own. Reproduced on speakers and on a Bluetooth headset,
so it is not one output path.

**The step detector is the wrong instrument for a thump, and that is why.** A
click is a discontinuity between two adjacent samples and `Health::scan` finds
those. A *bass drum* is not: a full-scale 50 Hz sine moves **0.0065** between
samples at 48 kHz, which is a fifth of the *median* step in a busy engine bed
(0.055) and well under its 99.9th percentile (0.287). An audible low-frequency
transient can sit in the mix with every counter reading zero, which is exactly
what is happening.

So the next instrument is the waveform itself. `--tap-audio FILE` records what
the callback hands cpal - not what the mixer would have produced, which is what
`--dump-audio` gives and which is a different thing, because that one forces
the null backend by construction. It is verified end to end on a generated
tone: 4 s recorded, peak exactly half scale, maximum sample step 474 against a
theoretical 472, no gaps.

```sh
just play --race --tap-audio /tmp/oag-tap.wav --tap-seconds 90
```

Recording starts when the stream opens, so `--race` matters - it skips the front
end and puts the recording on the track. The file is written from the frame loop
the moment it is full, so a run killed at the terminal still leaves it behind.

## What the file will settle

- **The thump is in the recording**: it is ours, and it is visible as a
  waveform and a spectrum rather than as a counter. That localises it to the
  mixer or to what the mixer was told to play, and the timestamps line up
  against whatever else the terminal said.
- **The thump is not in the recording**: our samples are correct and something
  downstream of `cpal`'s callback produces it - the ALSA plugin, PipeWire's
  graph, or the resampler between 48 kHz and whatever the endpoint runs at.
  That is a different investigation and none of the code in `crates/audio` can
  fix it.

## Two hypotheses tested and refuted, so nobody tests them twice

**The engine's loop is not the seam.** The player's own guess was that
`~ENGINE` loops and the wrap is the thump - it is 1.211 s long and held for the
whole race, which fits "regular, every few seconds, even if I hit nothing"
exactly. Rendered four loop periods through a real `Mixer`, the step across the
wrap is **0.002 of full scale** against a median step of 0.055 in the same bed.
The sample was authored to loop over its whole decoded buffer and it does.

**And the PS-ADPCM loop flags are not usable as read.** Every block carries a
flag byte at `+1` which `decode_adpcm` walks past; `pulse-psp-eu.chd`'s
`~ENGINE` is 1,907 blocks flagged `0, 6, 2 x 1903, 3, 255` - a lead-in, a
loop-start mark, the body, a loop-end mark, and a trailing block whose flag is
not one of the eight defined values. Honouring those marks - loop 28..53,368
rather than 0..53,396 - was implemented, measured, and **reverted**: it takes
the seam from 0.002 to **0.176**, sixty times worse. Either the marks are one
block off from the conventional reading or the encoder set them decoratively;
what is certain is that the waveform's own seam is at 0 and the flags do not
point at it. `.COLLISIONS` is flagged the same way (`0 x 434, 1, 7`) and its
terminator block decodes to silence, so it costs nothing there either.

## Also still open

`Mixer::render` takes `gain` and `channel_gain` once for the whole buffer, by
its own admission - "a tick is short enough that stepping it here would model an
interpolator we have not read". A tick was short enough at 512 frames. **At
1,920 frames it is 40 ms**, and a craft passing the camera changes both across
it. Ramping them per sample would be closer to the original, not further from
it. Untested, and not the thump on current evidence - the worst-step figure
would have caught a gain staircase of any size - but it is real and it got
worse when the buffer grew.

## Next Steps

1. Record a race: `just play --race --tap-audio /tmp/oag-tap.wav`. Hear at least
   two thumps before the 90 s is up, and note roughly when.
2. Look at the file. `audacity /tmp/oag-tap.wav`, or hand it to whoever can run
   a spectrogram - a bass thump is obvious in one and invisible in a counter.
3. If it is in the file, the timestamps are the lead: what else was happening at
   that second, and which voice started or stopped near it.
4. If it is not, stop looking in `crates/audio` - the mix is provably what it
   should be, and the next place is the ALSA plugin and PipeWire's own graph.
5. Still not done: the counters belong on the performance overlay
   (`crates/game/src/perf.rs`, `draw_list`) rather than only in the log.
