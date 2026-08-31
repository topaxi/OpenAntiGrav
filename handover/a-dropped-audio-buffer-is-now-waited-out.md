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

**What the glitch actually sounds like**, asked for after the first round of
guessing and worth more than either of the theories below: *"a bit like a bad
bass drum, it also aborts a current sfx (for example speed pad boost sound
abruptly stops)"*.

That is a very exact description of what the **pre-fix** callback did, and it
is worth reading before anything else here. PipeWire's quantum on this machine
is 1024 frames at 48 kHz - **21 ms**. One `try_lock` miss under the old code
zero-filled that whole buffer, so what came out was: a step discontinuity into
silence (a low thump), 21 ms of nothing in *every* voice at once, then a step
back out (another thump). On a short cue like the pad boost, 21 ms is enough of
it to sound like the sound was cut off rather than interrupted. Two thumps
21 ms apart is about 47 Hz, which is exactly "a bad bass drum".

**So the first thing to establish is which binary was played.** The run that
produced "glitches, no logs" may have been the main checkout rather than this
branch, in which case there was no counter to print and no ramp to hear, and
the description above is the bug this branch already fixes rather than evidence
against it. `cd ../oag-audio-glitch && just play` is the command; nothing in
the main checkout has any of this.

If the glitch survives *this* branch, the rest of this section applies.

**The other candidate, and it is not ours.** Played again with the change in,
the hiccups still happened and **neither** diagnostic line appeared - no
`audio: dropped N buffer(s) to mixer-lock contention`, no
`audio: output stream error`. Both were checked as reachable: `report_dropouts`
is called from `Session::frame`, `warn!` passes the default filter, and cpal's
ALSA worker does call the error callback on `ErrorKind::Xrun`
(`cpal-0.18.1/src/host/alsa/mod.rs:1000-1010`). Silence from both means neither
mechanism fired.

What the machine says instead (2026-08-31, `nobby`, Arch):

```
$ ps -Lo tid,cls,rtprio,ni,comm -p $(pgrep -x pipewire)
    TID CLS RTPRIO  NI COMMAND
   2292  TS      -   0 pipewire
   2317  TS      -   0 module-rt
   2332  TS      -   0 data-loop.0     <- the audio graph thread
$ ulimit -r
0
$ pacman -Qq realtime-privileges rtkit
error: package 'realtime-privileges' was not found
error: package 'rtkit' was not found
```

**PipeWire's own `data-loop` runs SCHED_OTHER at nice 0.** `module-rt` is
loaded and has nothing to grant it: no rtkit daemon, no `realtime` group, an
rtprio limit of 0. The graph quantum is 1024 frames at 48 kHz - 21 ms - and a
GPU-bound game preempting a normal-priority thread past that deadline is a
glitch PipeWire absorbs internally. With `default` routed through the
PipeWire ALSA plugin (`/etc/alsa/conf.d/50-pipewire.conf`), that underrun never
becomes an `EPIPE` reaching the client, so cpal cannot see it and nothing in
this process can log it. That is exactly "glitches, no logs".

If that is right, **the fix is on the machine, not in this repo**, and the same
glitch should be reproducible in any application under the same load.

`crates/audio`'s own two improvements stand on their merits either way - a
single `try_lock` was throwing away winnable collisions and zeroes were a
click - but neither is the cause of what was reported, and this thread should
not be closed as though they were.

## Next Steps

1. **Play this branch**, not the main checkout: `cd ../oag-audio-glitch && just play`.
   If the thump is gone or much quieter, the ramp is working and the counter
   says how often it still happens.
2. **Measure the graph.** `pw-top` has an `ERR` column that counts xruns per node. Run
   `pw-top -b -n 60 > /tmp/pwtop.log` alongside a race and read the `ERR`
   column for the output sink. Non-zero and climbing while the hiccups happen
   settles it.
3. **Then grant the privileges and measure again**, which is the actual repair:
   `sudo pacman -S realtime-privileges && sudo gpasswd -a "$USER" realtime`,
   then log out and back in. PipeWire's `module-rt` takes the direct
   `pthread_setschedparam` route once the rtprio limit allows it; `ps -Lo
   tid,cls,rtprio -p $(pgrep -x pipewire)` should then show `FF` on
   `data-loop.0`.
4. Only if the hiccups survive a real-time graph is there anything left here to
   fix, and at that point the counter from step 1 of the old plan is the one to
   read.

**cpal's `realtime` feature was tried and reverted, deliberately** - do not
re-add it without reading this. On Linux, `audio_thread_priority` promotes a
thread *only* through rtkit over D-Bus (`rt_linux.rs:92`, `rtkit_set_realtime`),
and cpal declares the crate `default-features = false`, so plain
`features = ["realtime"]` compiles to the blanket no-op fallback
(`audio_thread_priority-0.35.1/src/lib.rs:100-120`) and does nothing at all.
The feature that works is `realtime-dbus`, which pulls libdbus - a new C build
dependency for the whole workspace on Linux - and which still does nothing on a
machine with no rtkit, which is the machine that has the symptom. It is worth
revisiting once the limits.d route above is in place and there is evidence that
*our* worker thread, rather than PipeWire's, is the late one.
