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

## Open

**Where the ~96 ms stall comes from is not established.** The player's own
observation is that the thump follows a boost pad or a collision *by about a
second*, and that two boosts in sequence give two thumps in sequence - a
delayed, reliable correlation with a cue rather than with anything obviously
scheduling-shaped. Two readings, and nothing has separated them:

- **The whole process stalls** and the audio thread is collateral. A frame that
  takes 90 ms and an underrun are the same event from two threads.
- **Only the audio path stalls** - cpal's ALSA worker runs at ordinary priority
  and PipeWire's `data-loop.0` on this machine is SCHED_OTHER with no rtkit and
  no `realtime` group (`ulimit -r` is 0). This was retired earlier when heavier
  games turned out to be clean, and the underrun evidence puts it back on the
  list rather than at the top of it.

`Session::frame` now logs `frame: N ms` for any frame over 40 ms, which
discriminates them: a `frame:` line beside every `output stream error` is the
first reading, `output stream error` on its own is the second.

**Nothing here is confirmed by ear yet** - this session cannot open a window,
so both the bigger buffer and the release fade are unheard.

## Next Steps

1. Play a race and watch the terminal. Three lines matter: `frame: N ms`,
   `audio: output stream error`, and the `audio: ... late callback(s)` line.
2. If `frame:` lines appear beside the underruns, this stops being an audio
   problem and becomes "what takes 90 ms a second after a boost pad" -
   `just play-mangohud` for the frametime graph, and the particle effects
   (`oag_render::psys`) are the first place to look, since a boost and a
   collision both start one.
3. If there are no `frame:` lines, the stall is the audio path alone, and the
   next thing to try is the real-time privileges after all:
   `sudo pacman -S realtime-privileges && sudo gpasswd -a "$USER" realtime`,
   then log out. `ps -Lo tid,cls,rtprio -p $(pgrep -x pipewire)` should show
   `FF` on `data-loop.0` afterwards.
4. `cargo run -p oag-audio --example device-report --release -- 20` is the same
   counters with no game around them - if it is late with nothing rendering,
   the machine is the answer.
5. Not done either way: the five counters belong on the performance overlay
   (`crates/game/src/perf.rs`, `draw_list`) rather than only in the log, and
   `Mixer::starved`/`clipped` have been waiting for that since they were
   written.
