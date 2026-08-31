# The device queue is sized off the *configured* frame cap, not the achieved one

Asked after the HD thump landed: *"why are audio skips tied to the frame?
should this be put in a different thread? dropping to a slower device with
let's say 30fps lowers the error tolerance quite a bit"*.

## What is not tied to the frame

**Mixing already runs on its own thread and always has.** `cpal` calls
`Mixer::render` from its own callback, and `crates/game/src/main/session/frame.rs`
says so where the tick loop calls `audio.tick()`: "there is deliberately no
per-frame counterpart: with a device attached `cpal` drains the mixer from its
own callback thread". `Output::render_tick` returns zero the moment a stream
exists, so nothing pulls samples from the frame loop.

**Cue emission is per simulation tick, not per frame** - inside the fixed 60 Hz
loop, which is what makes a headless capture and a window produce the same
sound at the same tick ([ADR-0018](../docs/architecture/adr/0018-audio-mixer-architecture.md)).
A slow frame runs several ticks back to back rather than fewer.

**What is frame-paced is diagnostics only**: `Output::report_health` and
`Output::flush_tap`, both at the top of `Session::frame`. `report_health`
throttles in frames, so at 30 Hz it prints every four seconds instead of two.
Nothing about the samples depends on either.

## What genuinely was tied to the frame

**The device queue's size.** It was a flat 40 ms, chosen as "two and a half
frames at 60 Hz" - and the number that matters is not milliseconds but *how
long a stall it can cover*, and a stall is a frame. At the 30 Hz the menus
offer (`perf::FrameLimit::OFFERED`) the same 40 ms is one and a fifth frames,
so a single missed frame outruns the queue.

It now asks for `2.5 x` the configured frame period, floored at
`oag_audio::MIN_BUFFER` (40 ms, which is where the measurement that chose it
was made). Unlimited and every cap above 60 Hz take the floor unchanged; a
30 Hz cap asks for 83 ms.

## Open

**The cap is not the achieved rate.** A cpal stream's buffer is fixed when the
stream is built, so a machine that asks for 240 and delivers 30 gets the floor
and the tolerance the question was about. Covering that means rebuilding the
stream when the measured frame time persistently exceeds the queue - `Meter`
already measures it, and `Session` already holds the `Output` - which is a
disruptive thing to do mid-race and has not been attempted. **Nothing measures
whether it would help**: no run has yet produced a `frame: N ms` line beside an
underrun, which is the evidence that would say the two are the same event.

**The mutex is the remaining coupling, and it is not a frame one.** The frame
thread and the audio thread share `Mutex<Mixer>`; the callback waits out a
briefly-held lock and counts the buffers it still loses
([ADR-0031](../docs/architecture/adr/0031-wait-briefly-for-the-mixer-lock.md)),
and that counter has read zero in every run since. The lock-free command queue
that would remove it entirely is scoped in ADR-0031's alternatives and is
blocked on the same thing it was then: `position`, `is_playing` and
`active_voices` are read back by `race_music`, the movie path and the tests, so
it needs a published state snapshot coming the other way rather than a queue
going one.

## Next Steps

1. If a player on a slow machine still hears drops, read the `audio:` line
   first - `late callback(s)` with a `frame: N ms` line beside it is the
   process stalling and the queue being too small for it; `late` on its own is
   the audio path.
2. Only then consider rebuilding the stream against `Meter`'s measured frame
   time. It is the one change that would close the achieved-rate gap, and it
   should be evidence-led rather than added on the argument alone.
3. An audio-latency row in the settings would let a player make the trade
   themselves and is cheaper than either. Not done.
