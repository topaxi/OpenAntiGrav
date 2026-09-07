---
categories: [audio, rendering]
---

# The audio queue is ours now, and the frame stalls are what is left

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
sound at the same tick ([ADR-0018](../../docs/architecture/adr/0018-audio-mixer-architecture.md)).
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

## What landed

[ADR-0032](../../docs/architecture/adr/0032-render-ahead-into-a-ring.md): a
dedicated thread renders the mixer ahead into an `rtrb` ring and the `cpal`
callback does nothing but copy out of it. The depth is `oag_audio::MIN_BUFFER`
(60 ms) or the caller's own target, whichever is larger - the same number on
every host, and exactly how long a stall it covers. Nothing is asked of the
device any more. ADR-0031's spin is gone with it; its declick ramps are not.

## Open

**It does not cover the worst stall measured.** Frames of 80 ms outrun 60 ms of
queue. Raising the depth is a one-line change and a bad one - the depth is also
the delay on a collision cue - so **the remaining honest fix is the stall
itself**, which is a renderer question and not an audio one. `frame: N ms` is
in the log to point at it.

**The configured cap is still not the achieved rate.** The ring is sized when
the stream is built, so a machine set to 240 that delivers 30 gets the floor.
Resizing it live is now *possible* in a way it was not when the number belonged
to the sound server - the ring is ours - but it means rebuilding the stream or
holding the ring behind an indirection, and nothing has shown it is needed.

**Unmeasured on the machine that has the fault.** This session cannot open a
window; the ring is verified by a headless test that spawns the real thread
against a real mixer, and by five seconds of a tone through a real device with
nothing dropped. Whether it silences the lateness in an HD race is unheard.

## Next Steps

1. Race HD and read the `audio:` line. `0 dropped` with `late callback(s)` now
   means the device was late and the ring covered it, which is the whole point;
   `dropped` climbing means 60 ms was not enough and the stall is the thing to
   chase.
2. Then the stalls: 40 to 80 ms frames in an HD race is 12 to 24 fps in the
   spikes. `just play-mangohud` for the frametime graph.
3. `frame: 183.3 ms` fires on the frame that loads a race, which `Session`'s
   own `stalled` flag is supposed to exclude from the meter and evidently does
   not exclude from this log. Cosmetic, and it makes the log harder to read.
