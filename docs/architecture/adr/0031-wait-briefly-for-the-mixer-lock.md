# ADR-0031: Wait briefly for the mixer lock, and declick the buffer still lost

## Status

Accepted. Supersedes [ADR-0018](0018-audio-mixer-architecture.md)'s **"the
audio callback can emit a frame of silence" consequence only** - the paragraph
that reads "It takes the mixer lock with `try_lock` and fills silence on
contention". ADR-0018's decision is untouched: the callback still never blocks
on the lock, and the state/device split, the null backend, the clockless mixer
and cues as a per-tick output all stand, as does
[ADR-0027](0027-three-mix-buses.md)'s bus layout above them.

## Context

A player reported a music hiccup on a loaded machine. ADR-0018 predicted it in
so many words - "under heavy lock contention this is audible" - and the
mechanism is the one it named: `crates/audio/src/output.rs` took the mixer with
a single `try_lock`, and filled the whole buffer with zeroes the moment that
first attempt failed.

Two things about that were worse than the consequence ADR-0018 accepted.

**One attempt is not a wait.** The control side takes the lock to start, stop,
retune or query a voice and for nothing else; those critical sections are
microseconds. A callback that gives up on the first attempt throws away every
collision a wait of a few hundred microseconds would have won outright, and
keeps only the ones it cannot win anyway. The duty cycle is small, so the
collisions are rare - and each of the rare ones cost a whole buffer.

**Zeroes are not silence.** Filling a buffer with zeroes puts a step
discontinuity into the signal wherever the waveform happened to be, and again
where the next buffer resumes. A step is a click, and a click is far more
audible than the millisecond of missing music it replaces. The mixer's playback
position does not advance across a miss - `render` never ran - so both edges are
discontinuities of amplitude alone, which is the kind a ramp removes completely.

Separately and unrelatedly, the same report carried
`[calloop] Received an event for non-existence source`. That is winit's Wayland
backend removing and re-inserting a key-repeat timer on every key press, and
calloop dispatching a batch that still held the removed token. It is upstream
noise, it costs nothing - this game tracks key state itself through
`Controls::set_key` - and it is a marker of the same main-thread stall rather
than a cause of the hiccup. It is filtered to `error` in `init_logging` and is
not otherwise this ADR's business.

## Decision

**The callback waits, without blocking.** `acquire` tries once before reading
any clock, so the uncontended path costs exactly what it did. On contention it
alternates a short spin with `yield_now` until a budget expires. The budget is a
quarter of the buffer's own playing time, capped at 500 µs - derived from the
frame count handed to the callback, because a 64-frame PipeWire quantum is
1.45 ms and a 2,048-frame ALSA period is 46 ms.

**Spin *and* yield, not either.** A pure spin is right for the microsecond
collision and wrong for the one that matters: on a loaded machine the thread
holding the lock is the thread that just got preempted, and refusing to hand
back the core is precisely what stops it finishing. A pure yield gives up the
microsecond case for nothing. The loop does both.

**A poisoned lock gives up at once.** It never becomes unpoisoned, so waiting on
it is waiting the full budget once per buffer, forever.

**A buffer that is still lost is ramped, not cut.** The callback remembers the
last stereo frame it emitted; a give-up ramps down from it over 64 frames
(~1.5 ms at 44.1 kHz) and the first buffer after a gap ramps back up over the
same. A buffer shorter than the ramp still reaches the end of it.

**Dropped buffers are counted and reported.** `Output::dropped_buffers` reads
an atomic the callback bumps; `Output::report_dropouts` is called once per frame
from `Session::frame` and logs the first occurrence at `warn` and later growth
at `debug`, throttled to one line per 300 frames. The counter, not the code,
is what says whether a given machine still drops anything - the precedent is
`Mixer::starved`, and the reason it is read from the frame loop rather than the
callback is that formatting a message allocates, which on the audio thread is a
dropout of its own.

## Alternatives considered

**A plain `lock()` in the callback.** The classic priority inversion: the audio
thread would block on a main thread that is preempted, miss the device deadline,
and turn a gap this code can shape into an underrun in the driver that it
cannot. Rejected, and it is the thing ADR-0018 was right about.

**A lock-free command queue, with the mixer owned by the audio thread.** The
correct end state, and what a rewrite would do. Rejected *for now* because the
control side does not only write: `position`, `is_playing` and `active_voices`
are read back by `race_music`, the movie path and the tests, so a command queue
needs a published state snapshot coming the other way. That is a redesign of the
`Output` API rather than a fix to a callback, and it should be an ADR that
proposes the whole shape rather than a change smuggled in under a bug report.

**`parking_lot::Mutex::try_lock_for`.** Does exactly what `acquire` does and is
better tested. Rejected because it is a new workspace dependency for twenty
lines, and those twenty lines are ones this project wants to be able to explain.

**Repeating the previous buffer instead of ramping.** Standard concealment for a
lost packet, and it would fill the gap rather than leaving one. Rejected because
it needs the previous buffer kept, it invents a period that was not in the
signal, and at these budgets the gap should be rare enough that concealing it
better than "a ramp to silence" is optimising a path that ought not to be taken.

## Consequences

**A collision now costs latency instead of a buffer, up to the budget.** In the
worst case the callback spends a quarter of its period waiting and has three
quarters left to render and convert in. On a device already close to its
deadline that is a real reduction in headroom, and if it ever bites the symptom
will be an `output stream error` from cpal rather than a counted drop - a
different line in the log, which is why both exist.

**Drops are rarer but not gone.** Nothing here helps the case where the lock
holder is preempted for longer than the budget, which is exactly the heavily
loaded machine that prompted this. It is quieter when it happens, and it is now
counted, and that is the whole of the improvement in that case.

**The ramp is not free of artefacts either.** A 1.5 ms fade is a fast one; on
sustained low-frequency content it is audible as a blip rather than a click.
That is the trade this takes deliberately - a longer ramp would eat more of a
short buffer, and a buffer shorter than the ramp is a real case.

**The callback is testable now, and only because it was split.** `render_stereo`
is a free function so a machine with no sound card can hold the lock from
another thread and assert what comes out. The device path itself is still
untestable here, and the tests say so.

**The report is a log line, not an overlay.** A player who is not watching a
terminal still hears the hiccup and learns nothing. Putting the counter on the
performance overlay is the obvious follow-up and is deliberately not done here.
