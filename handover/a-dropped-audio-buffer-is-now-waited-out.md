# The race thumps when a sound stops, and three of the four candidates are measured out

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

## Open

**Whether the release fade actually silences the thump is unverified.** This
session cannot open a GUI window, so every measurement above is offline or
idle, and the ear that reported the fault is the only instrument that can close
it.

If it does not, the instrument to read is now in the code.
`Output::report_health` runs once per frame from `Session::frame` and logs one
line every ~120 frames whenever any of five counters moved:

```
audio: 0 dropped, 4 jump(s) in the mix, 0 late callback(s) (worst 0.0 ms over),
       0 voice(s) refused, 0 sample(s) clipped - of 5310 buffer(s) of 512 frame(s) so far
```

Each column names a different fault and they do not overlap - see
`crates/audio/src/output/health.rs`:

- **dropped** - the mixer lock was still held when the callback's waiting
  budget ran out ([ADR-0031](../docs/architecture/adr/0031-wait-briefly-for-the-mixer-lock.md)).
- **jumps** - the callback rendered normally and the samples still step
  discontinuously from the previous buffer's last frame. **A click in our own
  mix**, which is what a surviving thump should look like.
- **late** - more wall-clock time passed between two callbacks than the buffer
  between them was worth. The device path or the scheduler, not this crate.
- **refused** - `Mixer::play` had no free slot. The user's own guess ("maybe
  there's not enough parallel sounds") lands here, and it would be heard as a
  *missing* sound rather than a thump.
- **clipped** - the voice sum saturating.

## Next Steps

1. Play a race on this branch and listen. If the thump is gone, delete this
   thread and the branch is done.
2. If it survives, read the `audio:` line. Which column moved is the whole
   diagnosis: jumps means our mix, late means the device path, refused means
   the voice pool, clipped means the gain staging.
3. `cargo run -p oag-audio --example device-report --release -- 20` is the
   same counters with no game around them, for separating a fault in the race
   from a fault in the device path.
4. Not done, and the obvious follow-up either way: the five counters belong on
   the performance overlay (`crates/game/src/perf.rs`, `draw_list`) rather than
   only in the log, and `Mixer::starved`/`clipped` have been waiting for that
   since they were written.
