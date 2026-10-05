# Streaming decode for audio would break seek, and nothing forces the change yet

2026-08-25. The race-launch and track-boundary hitches
(`crates/sound/src/race_music.rs`) are fixed by moving decode to a
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

## 2026-09-03: the memory number is measured, and it is not the whole question

`cargo run --release -p oag-game --example audio_memory_probe` (new,
committed alongside this update) decodes the longest track on each disc
through the exact path `RaceMusicWorker` uses and reports both the
`Sound` buffer size and the measured process RSS delta:

| Disc | Longest track | `Sound` buffer | Measured RSS delta |
| --- | --- | --- | --- |
| Pure PSP (USA/EU) | 326.1 s | 54.87 MiB | +51 to +55 MiB |
| HD/Fury PS3 | 325.1 s | 59.52 MiB | +48.49 MiB |
| Pulse PS2 (from `docs/formats/ps2-audio.md`, not reprobed - already well under the other two) | 204.3 s | 34.37 MiB | not measured |

Worst case in the corpus today is one `Sound` under 60 MiB, two live at once
(the prefetch window) puts genuine peak around 120 MiB. **On a desktop
target that is a rounding error - it is not a settled answer for every
target this project has.** Handhelds are an explicit future target
(unspecified which ones yet), and a 120 MiB peak for one piece of race music
is a very different number on a memory-constrained handheld than on a
desktop; this measurement does not close that door, it just states what the
number actually is so a future session isn't guessing. **A previous pass
through this thread closed it on exactly this point, reading "not a problem
on desktop" as "not a problem" - wrong, caught by the user, see the next
section for the bigger miss in the same close.**

## 2026-09-03: the actual freeze - a still-live synchronous decode, not fixed by the worker

**This is the part the earlier close of this thread got wrong in a more
basic way: it read "streaming vs. whole-buffer" as the only question this
thread was ever asking, and missed that a third hitch - distinct from the
two `RaceMusicWorker` already fixed - was never closed.** The 2026-08-25
text above is careful to say "neither of the **two** bugs this thread's
earlier work fixed needed streaming" - race launch and the natural
track-boundary. It does not say every track-change path is off the
frame/settings thread, and one is not:

- **`Audio::set_music_source`** (`crates/sound/src/lib.rs:954`), the
  `audio.music_source` settings row's handler
  (`crates/game/src/main/session/apply.rs:224`) - a player toggling the
  MUSIC SOURCE row, in the menu or **mid-race**, calls this **synchronously**
  on whatever thread applies a settings change. `apply.rs`'s own comment
  already states the cost it accepts: "measured at 2.0 s for the PS2's 36
  MiB of PCM and 0.4 s for the PSP's cached decode" (`apply.rs:211`). That is
  a genuine freeze on a user action, not a hypothetical: `fetch`/
  `fetch_indexed` do a disc read plus, for ATRAC3+, an out-of-process
  `ffmpeg` decode, on the same call stack that applies the setting.
- **`Audio::advance_race_track`**'s fallback path
  (`crates/sound/src/race_music.rs:338`) - called from `Audio::tick`,
  inside the fixed-timestep loop, every time the race voice stops. The
  prefetch (`maybe_prefetch_next_race_track`) covers the *natural* end of a
  track, but the method's own doc already names the gap: "falls back to the
  old synchronous fetch only when there is no prefetch to trust - none
  started in time". **A skip that lands before the prefetch has finished -
  or immediately after a track starts, before one has even begun - hits this
  fallback and blocks the tick loop for the same 0.4-2.8 s.** There is no
  user-facing "skip to next track" control today (grepped: none), so this
  path is currently only reachable by the natural-end race described in its
  own comment, not by a player action - but `set_music_source` above *is* a
  player action, and it is the "tracks change... by the user" freeze this
  thread was actually asked about.

Neither of these needs progressive/streaming decode to fix - both are the
same shape as the two bugs `RaceMusicWorker` already solved: get one
synchronous `load_track`/`fetch`/`fetch_indexed` call off the thread that
cannot afford to block, keep the buffer whole. The pattern to copy is
`RaceMusicWorker` itself, not a new mechanism.

## 2026-09-03: `set_music_source`'s freeze is fixed

The actual player-facing freeze (previous section) is closed.
`set_menu_music_source`/`set_race_music_source` (moved to `race_music.rs`,
beside `MusicFetchWorker` - renamed from `RaceMusicWorker`, now generic over
a `label` so a debugger still says which caller is waiting on it) still
apply an already-read release on the spot - that path is pure memory, no
I/O - but a fresh one now spawns a `MusicFetchWorker` and returns
immediately. `Audio::tick` polls the pending switch
(`Audio::source_switch`) and applies it once it lands, reading the live
playhead **at that moment** rather than when the fetch was first asked for,
since a worker can now take several ticks to land while the old track keeps
sounding. A second request before the first lands drops the stale one,
including a press back to the platform already playing - new race-shaped
territory the old synchronous row could never reach at all, since every
press used to block until it finished.

Verified two ways:

- Three synthetic unit tests in `crates/sound/src/tests/source_switch.rs`
  (fake disc paths, no game data needed): the call never blocks, a failed
  fetch leaves state untouched, and a press back to the current release
  drops a switch still in flight.
- `crates/game/examples/audio_source_switch_probe.rs`, against real discs
  (`pulse-psp-usa.chd`/`pulse-ps2-eu.chd`) and a real `ffmpeg` decode: the
  `set_music_source` call itself measures **0.0 ms**, and the playhead keeps
  advancing through the whole 3-second window the old synchronous version
  would have spent blocked on the PS2's cold 2.0 s decode.

`just` clean; full workspace test 2,831 passed.

## Open

- Whether progressive decode is ever worth it depends on a constraint that
  is now partly measured: a fully-materialized `Sound` for the longest track
  on disc costs 55-60 MiB (measured above), which is not a problem on any
  desktop this project has targeted so far, and is an open question on the
  handheld targets the project has not yet chosen. Revisit once a specific
  handheld's memory budget is known - not before, and not as a blocker to
  anything above, which streaming would not have fixed any faster than a
  worker thread did.
- If progressive decode is later worth doing anyway, decide seek's fate
  explicitly first - keep it and progressively fill (no memory win, `Sound`
  stays a `Vec`) or drop it to checkpoint-based seek (memory win, breaks
  `race_position` scrubbing and pause/resume as currently built) - rather
  than let a half-considered ring buffer answer it by accident.
- `Audio::start_music`'s own first-boot fetch is the same shape of
  synchronous decode `set_music_source` used to be, unfixed - likely lower
  priority since it runs once per boot rather than on a repeatable player
  action, and is plausibly already masked by a loading screen, but that is
  an assumption nobody has checked against a real boot.

## Next Steps

1. Cover `advance_race_track`'s fallback path the same way if a skip
   control is ever added - right now it is unreachable by a player, so this
   is scoping for later, not a bug to fix today.
2. Do not build progressive/streaming decode without a stated handheld
   memory budget to measure against - the desktop number above already
   answers "is this a problem today" (no).
3. Check whether `Audio::start_music`'s first-boot decode is actually
   masked by a loading screen or genuinely visible as a stall - if the
   latter, it is the same fix shape as `set_music_source`'s, not a new one.
