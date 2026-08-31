# ADR-0032: Render ahead into a ring, and let the callback only copy

## Status

Accepted. Supersedes [ADR-0031](0031-wait-briefly-for-the-mixer-lock.md)
**entirely** - its decision was about how the audio callback should take the
mixer lock, and the callback no longer takes it. Supersedes
[ADR-0018](0018-audio-mixer-architecture.md)'s **`Output` shape only**: the
paragraph that has `Output` own "the `cpal` stream, or none at all" with the
callback draining `Mixer` directly. Everything else of ADR-0018 stands - the
crate's place on the non-simulation side, `cpal` under our own mixer, the
state/device split, the null backend, the clockless mixer, cues as a per-tick
output, and ADSR/reverb deferred - as does
[ADR-0027](0027-three-mix-buses.md)'s bus layout.

## Context

The audio callback rendered the mixer itself, so the only audio queued anywhere
was whatever the device asked for in one callback. Two measurements taken while
chasing a reported thump say that is not enough and, worse, not ours to fix by
asking.

**The client does not choose the buffer on a modern desktop.** cpal's PipeWire
backend turns a `BufferSize::Fixed(n)` request into a `node.latency = n/rate`
property (`cpal-0.18.1/src/host/pipewire/device.rs:182`), and `node.latency`
asks PipeWire for a *ceiling* on latency rather than a floor - a client cannot
make the graph run a bigger quantum by asking for one. Measured on the
reporting machine with `cargo run -p oag-audio --example device-report`,
requests of 40 ms, 85 ms and 200 ms all came back as the graph's own quantum:
256 frames, 5.3 ms, and 1,024 frames in an earlier run, because the number
moves with whatever else is on the graph. The ALSA host honoured the same
request exactly, because its plugin buffers on the client's behalf - so the
project's own switch to PipeWire, which is right for other reasons, halved and
then quartered the queue it had just been given.

**And the stalls it has to cover are far larger than that.** An HD race on the
same machine, with the frame-time logging added beside the audio counters:
`frame: 49.5 ms` and `frame: 48.4 ms` next to `1 late callback(s) (worst
25.1 ms over)`, with the mix itself provably clean - nothing dropped, nothing
discontinuous, nothing refused, nothing clipped. The process stalls and the
audio thread is late behind it; a 5.3 ms queue has no chance of covering
50 ms.

ADR-0031's bounded spin was the right answer to the question it asked - it made
the callback wait out a briefly-held mixer lock rather than dropping a buffer -
and its counter has read zero in every run since. It does nothing about a queue
that is too shallow, because that is a different failure.

## Decision

**A dedicated thread renders the mixer ahead of time into a lock-free ring, and
the `cpal` callback does nothing but copy out of it.** `output::render::Ahead`
owns the thread; `rtrb` owns the ring.

**The depth is this project's number, and it is a ceiling.** The thread holds
the ring at the target rather than filling it to the brim, so the depth a
caller names is the most a cue can be delayed rather than the least.
`MIN_BUFFER` is 60 ms and the caller may ask for more - the composition root asks for two and a half frames of the
configured frame cap. It is the same on every host, it is exactly how long a
stall the queue covers, and it is exactly how late a cue is heard. Nothing is
asked of the device at all any more: whatever it wants per callback is now
nobody's problem.

**The audio thread never takes a lock.** The frame loop and the render thread
contend for `Mutex<Mixer>`, and the render thread can afford to block where the
callback could not. The spin, the budget and the poisoned-lock case all go.

**The thread polls rather than parks.** It fills while the ring has room for a
whole 512-frame chunk and sleeps 5 ms otherwise. Waking it from the callback
would put a futex wake on the audio thread; two hundred wake-ups a second of a
thread that usually finds nothing to do is the cheaper of the two.

**A ring that runs dry is still declicked and still counted.** `Health`'s
`dropped` keeps its name and changes its meaning - it was a buffer the callback
gave up on, it is now a buffer the ring could not fill - and the ramps either
side of the gap are the ones ADR-0031 introduced.

## Alternatives considered

**Asking the device for more, harder.** Measured and refused; see Context. It
also had the wrong shape: a number the process asks a server for and cannot
check is not a guarantee.

**Keeping ALSA, which honoured the request.** Rejected. It works by putting
PipeWire's compatibility plugin between this process and the graph that
actually plays, and the plugin's buffering is not something to build a
guarantee on either. Reaching the default sound server directly is right on its
own terms.

**A lock-free command queue with the mixer owned by the audio thread.** The
shape ADR-0031 named as the eventual answer. Rejected in favour of this because
it is strictly harder and solves less: `position`, `is_playing` and
`active_voices` are read back by `race_music`, the movie path and the tests, so
it needs a state snapshot published the other way - and it would still leave the
queue as deep as the device's own buffer. Rendering ahead takes the lock off the
audio thread *and* sets the depth, with an ordinary `Mutex` on both sides.

**Hand-rolling the ring.** The workspace sets `unsafe_code = "deny"` and a
single-producer single-consumer ring is exactly the thing not to write twice.
`rtrb` is small, does one thing, and is the crate this problem has.

## Consequences

**Up to 60 ms of latency, deliberately and honestly.** A cue is heard when the
ring drains to it, so the depth is the delay. The thread holds the ring *at*
the target rather than filling it, so the figure is a ceiling: about 49 to
60 ms at 48 kHz, one render chunk of spread. Against what it replaced that is
roughly 40 ms added - the device buffer this used to rely on was 5 to 21 ms on
PipeWire, and cue emission already cost up to a tick on top of either. Before
this the number was whatever the sound server happened to be doing; now it is a
decision, which is the part that matters.

**It does not cover the worst stall measured.** Frames of 80 ms outrun 60 ms of
queue, and raising the depth to cover them would put the delay on a collision
somewhere a player can feel. **The remaining honest fix is the stall**, and the
`frame: N ms` line exists to point at it.

**One more thread, and one more dependency.** The thread is one per `Output`,
which is one per run. `rtrb` is 0.4 and unlikely to move.

**The frame loop now contends with the render thread.** A `with_mixer` call can
wait for a 512-frame render, tens of microseconds. That is a cost paid on the
thread that can afford it, which is the whole trade.

**The mixer is no longer rendered in the callback, so a mixer panic is a silent
run rather than a crash.** The render thread returns on a poisoned lock instead
of spinning; the ring drains, the callback ramps into silence, and `dropped`
climbs. Audible, counted, and not a hang.
