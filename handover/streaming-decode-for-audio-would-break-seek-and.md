# Streaming decode for audio would break seek, and nothing forces the change yet

2026-08-25. The race-launch and track-boundary hitches
(`crates/game/src/audio/race_music.rs`) are fixed by moving decode to a
worker thread, not by shrinking the buffer - `RaceMusicWorker` decodes a
whole track and hands `Audio` a complete `oag_audio::mixer::Sound`
(`{samples: Vec<i16>, channels, sample_rate}`), same as before, just off
the main thread. Asked whether audio should stream/decode progressively
instead, for any media type the engine supports, not only race music.

The engine already has a streaming precedent: `crates/game/src/movie.rs`'s
`Feed` decodes into a bounded `Ring` on a worker thread, consumed by the
render thread through a `Mutex`+`Condvar` (`Shared`). That pattern does not
transfer to audio. `Feed`'s consumer is the render thread, which can absorb
a lock stall of a few hundred microseconds with nothing audible. Audio's
consumer is `cpal`'s real-time callback, which must not block, lock, or
allocate - any stall there is a click, not a dropped frame. Copying `Feed`'s
`Mutex`-guarded ring into the mixer's render path would put a lock on the
one thread in the engine that cannot afford one.

A lock-free version (bounded SPSC ring, `Sound`/`Voice` reading a partially
filled buffer) is buildable, but it does not do what it would be built for:

- It does not reduce peak memory unless the ring is bounded well below a
  full track, and a bounded ring means no arbitrary seek past the ring's
  horizon.
- Race music's own PCM is ADPCM-sourced and decoded once into `i16`; seek
  today means indexing straight into the finished buffer. A bounded ring
  makes seek mean re-decoding from a checkpoint instead, and `race_position`,
  pause/resume, and `set_race_music_source` are all built on the current,
  free seek.
- `oag_audio::mixer::Sound` is shared by SFX, movie audio, menu music, and
  the WAV-dump path, not just race music - this would be a change to the
  shared real-time core, not a race-specific one.

Point two is the one that actually rules it out, not effort: a real feature
(seek) would regress. Neither of the two bugs this thread's earlier work
fixed needed streaming - both were fully solved by getting decode off the
frame/callback-adjacent thread while keeping the buffer whole.

## Open

- Whether progressive decode is ever worth it depends on a constraint not
  yet stated: how much peak memory a fully-materialized `Sound` costs for
  the longest track on disc, and whether that number is actually a problem
  anywhere it has been measured. Nobody has measured it.
- If it is later worth doing anyway, decide seek's fate explicitly first -
  keep it and progressively fill (no memory win, `Sound` stays a `Vec`) or
  drop it to checkpoint-based seek (memory win, breaks `race_position`
  scrubbing and pause/resume as currently built) - rather than let a
  half-considered ring buffer answer it by accident.

## Next Steps

- Do not start this without a concrete number showing the current
  fully-materialized `Sound` is actually a memory problem in practice - the
  two hitches that motivated looking at this are already fixed without it.
- If it proceeds, scope it as a change to `oag_audio::mixer::Sound`/`Voice`
  directly (not a race-music-local worker), since every audio path shares
  that type, and settle the progressive-fill-vs-bounded-ring/seek question
  in that crate's own design doc before writing code.
