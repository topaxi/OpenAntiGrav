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

**Where the thump comes from is still not known**, and the fault is now in the
instrument as much as anywhere: `Health::scan` checked only the *seam between*
two buffers until now, so it reported zero through a race that was audibly
thumping. A voice ends wherever its own playhead runs out, which with a
1,920-frame buffer is at the seam roughly one time in two thousand. It now
scans every frame of every buffer and reports the **worst step it saw and the
frame it saw it at**, whether or not anything crossed the threshold:

```
audio: 0 dropped, 0 jump(s) in the mix (worst 0.187 at frame 913),
       0 late callback(s) (worst 0.0 ms over), 0 voice(s) refused,
       0 sample(s) clipped - of 5310 buffer(s) of 1920 frame(s) so far
```

That number is the whole diagnosis waiting to happen. A busy race genuinely
moves 0.287 between samples on its own, so anything near or above that is the
mix stepping, and the frame offset says whether it is at a buffer boundary
(gain or pan, which `Mixer::render` applies once per buffer and not per sample)
or in the middle of one (a voice starting or ending).

**One candidate is untested and now more likely than it was**: `Mixer::render`
takes `gain` and `channel_gain` once for the whole buffer, by its own admission
- "the original moves a pan over a ramp inside SCREAM rather than per sample,
and a tick is short enough that stepping it here would model an interpolator we
have not read". A tick was short enough at 512 frames. **At 1,920 frames it is
40 ms**, and a craft passing the camera changes both across it. Ramping them
per sample is a small change and would be closer to the original, not further
from it - but it is a hypothesis, and two have already been wrong this session,
so the worst-step line comes first.

## Next Steps

1. Play a race and read the `worst N at frame M` figure on the `audio:` line.
   That is the measurement everything else waits on.
2. **Worst near or above 0.3, at a frame that is not 0**: a voice is starting or
   ending discontinuously. The frame offset times 1/48,000 says when inside the
   buffer, and the release fade already covers a *stopped* voice, so look at
   what starts.
3. **Worst near or above 0.3, at frame 0 or near it**: the per-buffer gain and
   pan staircase above. Ramp them per sample.
4. **Worst comfortably under 0.3 and still thumping**: our samples are clean and
   the fault is downstream of this process. The next instrument is a tap in the
   callback that writes exactly what cpal was handed to a WAV, which
   `--dump-audio` cannot do because it forces the null backend.
5. Still not done: the counters belong on the performance overlay
   (`crates/game/src/perf.rs`, `draw_list`) rather than only in the log.
