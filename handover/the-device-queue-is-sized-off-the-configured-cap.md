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

## And then the measurement that undercut it

**On the PipeWire host the request is ignored entirely.** cpal's PipeWire
backend turns `BufferSize::Fixed(n)` into a `node.latency = n/rate` property
(`cpal-0.18.1/src/host/pipewire/device.rs:182`), and `node.latency` asks
PipeWire for a *ceiling* on latency, not a floor - a client cannot make the
graph run a bigger quantum by asking for one. Measured on this machine with
`--example device-report`:

| asked for | got |
| --- | --- |
| 40 ms | 256 frames (5.3 ms) |
| 85 ms | 256 frames (5.3 ms) |
| 200 ms | 256 frames (5.3 ms) |

...and 1,024 frames (21.3 ms) in an earlier run, because the number is the
**graph's** quantum and it moves with whatever else is on the graph. Under the
ALSA host the same request produced exactly 1,920 frames, because the plugin
buffers on the client's behalf.

So `requested_buffer_size` and the frame-cap sizing above are a no-op on the
host this project now uses by default. They are correct and they are also
inert; nothing about them is worth removing, and nothing about them helps.

**Against frame stalls of 40 to 80 ms**, measured in an HD race on the same
machine with the `frame:` logging that landed beside them, a graph quantum of
5.3 to 21.3 ms has no chance. This is the confirmation the previous section
asked for and did not have: `frame: 49.5 ms` and `frame: 48.4 ms` beside
`1 late callback(s) (worst 25.1 ms over)`, with the mix itself clean - 0
dropped, 0 jumps, worst step 0.064.

## Open

**The queue depth is not ours to choose, and on PipeWire it never will be.**
Asking the device is the only lever `Output` has, and the measurement above
says the device declines. The only way to hold more audio than the graph's
quantum is to hold it **ourselves**: a ring buffer that a dedicated thread
renders into ahead of time, with the `cpal` callback doing nothing but copying
out of it. That would put the tolerance under this project's control rather
than PipeWire's, and it would take the mixer lock off the audio thread
entirely - which is the other half of
[ADR-0031](../docs/architecture/adr/0031-wait-briefly-for-the-mixer-lock.md)'s
unfinished business. It is a change to the shape ADR-0018 sets out and wants an
ADR of its own.

**The other reading of the same log is that the frame stalls are the bug.**
40 to 80 ms frames in an HD race is 12 to 24 fps in the spikes, and audio
lateness is a symptom of it rather than a fault of its own. Fixing the stall
fixes both, and improves the thing the player is actually looking at.

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

1. Decide which of the two the next piece of work is: a render-ahead ring under
   our own control, or the frame stalls that make the queue's size matter. They
   are not alternatives in the long run, only in the order.
2. `frame: 183.3 ms` fires on the frame that loads a race, which `Session`'s
   own `stalled` flag is supposed to exclude from the meter and evidently does
   not exclude from this log. Cosmetic, and it makes the log harder to read
   than it needs to be.
3. An audio-latency row in the settings would let a player make the trade
   themselves, and on PipeWire it would have nothing to set. Worth it only
   after item 1.
