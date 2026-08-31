# A dropped audio buffer is now waited out and ramped, and nobody has heard it yet

Reported from play on 2026-08-31: a music/sound hiccup under load, alongside
`[WARN ] [calloop] Received an event for non-existence source: TokenInner { id:
3, version: 10, sub_id: 0 }` in the terminal.

The two are **not** cause and effect. The calloop line is winit's Wayland
backend removing and re-inserting a key-repeat timer source on every key press
(`winit-0.30.13/src/platform_impl/linux/wayland/seat/keyboard/mod.rs:167`, and
the `remove` twenty lines above it); slot 3 is the first slab slot after the
three sources winit inserts permanently at startup - the Wayland queue, the
user-event channel and the awakener - so `id: 3, version: 10` is that timer on
its tenth reuse. Under load a dispatched batch still carries an event for the
token that was just removed, and calloop warns. The dropped event is a stale
key-repeat tick for a key this game tracks itself through `Controls::set_key`,
so nothing is lost. It is filtered to `error` in `init_logging` (both
`crates/game/src/main.rs` and `crates/view/src/logging.rs`); `RUST_LOG=warn,calloop=warn`
puts it back.

The hiccup is the mixer-lock contention [ADR-0018](../docs/architecture/adr/0018-audio-mixer-architecture.md)
predicted in so many words. [ADR-0031](../docs/architecture/adr/0031-wait-briefly-for-the-mixer-lock.md)
supersedes that consequence: the callback now waits out a held lock (spin
alternating with `yield_now`, budget a quarter of the buffer's own playing time
capped at 500 µs), and ramps down/up over 64 frames around a buffer it still
loses instead of cutting to zeroes.

## Open

**Nothing here is confirmed against the machine that has the symptom.** This
session cannot open a GUI window (it is the standing constraint on every
windowed check in this repo), so the whole change is verified by unit test and
by reading, and the report that prompted it was a player's ear. What is
untested: whether the drops the player heard were ever mixer-lock drops at all.

The discriminator is now in the code and costs one run:

- `Output::dropped_buffers` counts buffers the callback gave up on;
  `Output::report_dropouts`, called once per frame from `Session::frame`, logs
  the first at `warn` and later growth at `debug` (throttled to 300 frames).
- A **cpal/driver xrun** is the other candidate and logs separately, at `error`,
  through the existing `audio: output stream error: {err}` arm in
  `crates/audio/src/output.rs`. That line passes the default filter already.

So: race until the hiccup is heard, then read which of the two lines appeared.
If it is `output stream error` and no dropout count, this change fixed nothing
audible and the next place to look is the device period and cpal's own
buffer-size request, not the lock.

Second unknown: the waiting budget takes latency out of the callback's
headroom. If a device that was previously fine starts reporting
`output stream error` after this change, that is the trade biting and
`SPIN_FRACTION` is the knob.

## Next Steps

1. Play a race on the machine that hiccups, with the terminal visible. Note
   which of the two log lines appears.
2. If it is the dropout counter: the count says how much is left. The follow-up
   ADR-0031 names is the real one - a lock-free command queue with the mixer
   owned by the audio thread - and it is a redesign of `Output`'s API, because
   `position`, `is_playing` and `active_voices` are read back by `race_music`,
   the movie path and the tests.
3. If it is `output stream error`: nothing above applies, and the question is
   the device configuration `Output::open` accepts verbatim from
   `default_output_config` - it never asks for a buffer size at all.
4. Either way, the counter belongs on the performance overlay
   (`crates/game/src/perf.rs`, `draw_list`) rather than only in the log. A
   player who is not watching a terminal learns nothing today.
