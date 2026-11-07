# The replay timeline

**Status: built (stage 3 of [ADR-0087](adr/0087-a-replay-and-photo-mode-share-one-timeline.md), 2026-11-07).**
Code: [`oag_raceplay::timeline`](../../crates/raceplay/src/timeline.rs). Tests:
[`src/tests/timeline.rs`](../../crates/raceplay/src/tests/timeline.rs) (a fixture, in CI) and
[`tests/replay_timeline_ground_truth.rs`](../../crates/raceplay/tests/replay_timeline_ground_truth.rs)
(every title's data). The ADR holds the decisions and why; this page is what was built and what it
was measured to do. Nothing draws from it yet: the viewer, the photo rewind and the take-over are
stages 5 to 7.

## The shape

A `Timeline` owns a **probe**, a `Race` of its own, and moves it with `restore_sim` and
`tick_sim_only`. It builds without the `view` feature, as do `oag_raceplay::replay::rebuild`,
`Launch` and `Race::apply_event`; the half of `replay` that watches a live race and draws a ghost
does not. It holds no reference to a live race and takes one only as `&Race`.

| Call | What it does |
| --- | --- |
| `Timeline::open(setup, replay)` | Starts the probe, refuses a tick-zero mismatch, starts the opening pass on a worker (`oag_thread::Task`). `replay::load_recorded(&header, source)` gives the `Setup` |
| `Timeline::live(setup, &race)`, `keep(&race)`, `visit(&race)` | Over the race being driven: a keyframe when one falls due, straight after each tick; `visit` moves the probe to the live tick and opens every tick up to it |
| `pass()`, `prefix()` | How the pass stands, and the last tick the probe may stand at |
| `seek(tick)`, `step()` | Move the probe. `seek` returns the tick reached; with no recording (a modded race) that is the keyframe at or before the target |

**A keyframe is a `RaceSim`.** A clone is the mutable half plus a reference count per constant field
(`sim.rs`, "Why it is `Clone`"), so decision 5's "owns the shared constant half" needs no handle of
its own. A probe restored from a keyframe drops the copy `Race::start` made and shares the half.

**What a tick number means.** Keyframe `T`, and a probe at `T`, is the state after `T` ticks, before
the events due at `T`: where a straight pass stands between ticks `T - 1` and `T`. Moving forward
applies a tick's events, then runs it, so an event on a keyframe's own tick and one among the
resimulated ticks land between the same two ticks as on the recorded run. The one exception is a
`visit` to a live race that took an event on the tick it stands at: the probe starts with it
applied, and only a seek back to that tick stands before it.

**The prefix.** A keyframe is published when a stored hash has vouched for it, so nothing past the
last agreeing hash is ever held. `Pass::Desynced` ends the prefix at that hash, `Pass::Refused` at
an event this build does not apply, `Pass::Lost` if the worker dies. A `Desync` at tick zero is
`OpenError::Desync` from `open`, with no pass. The ticks after the last stored hash have nothing to
disagree with and join the prefix when the pass ends.

**The ceiling.** `KEYFRAME_CEILING`, 16 MiB, against `RaceSim::snapshot_bytes`, an estimate that read
exactly what a counting allocator read for one snapshot on Pulse and 2048 (27,156 and 31,972 bytes).
Past it the interval doubles and every keyframe off the new interval goes, the first excepted.
`open_within` takes another ceiling.

**The plans.** The pass calls `Race::ensure_speed_plans` before it cuts keyframe 0, so that keyframe
carries them, and a live timeline's first keyframe is handed them once the first tick has learned
them (`RaceSim::share_plans_of`). So a probe never learns plans on the caller's thread, and the
ADR's "Bad" entry about a first tick on a file-opened probe does not apply. The price is that
`prefix()` is `None` until the plans are in: the probe stands at tick 0 and cannot play for the
"first keyframe" column below, not "at once" as decision 3 has it.

## What a seek was shown to do

A Single Race of eight craft, VENOM, flown by the pad of `replay_ground_truth`, so the input moves
nearly every tick; run to the finish and 300 ticks past it, at least 6,700 ticks, read back from the
file's bytes and rebuilt from its header alone. The stored hashes cannot judge a seek (they fall on
keyframe ticks, where a seek resimulates nothing), so the recording run kept its own hash of every
tick.

| Title | Run (ticks) | Finish | Hash targets | Text targets | Seeks during the pass |
| --- | ---: | ---: | ---: | ---: | ---: |
| Pulse | 8,088 | 7,788 | 41 of 41 | 4 of 4 | all agreed |
| Pure | 6,700 | 4,234 | 41 of 41 | 4 of 4 | all agreed |
| HD | 8,257 | 7,957 | 41 of 41 | 4 of 4 | all agreed |
| 2048 | 14,410 | 14,110 | 41 of 41 | 4 of 4 | all agreed |
| Omega | 7,880 | 7,580 | 41 of 41 | 4 of 4 | all agreed |

- The targets sit 1, 17, 30, 43 and 59 ticks past a keyframe, in an order that goes back as often as
  forward, past the finish and before it. So the probe's view half, which keeps a results board and
  a wreck's tick from a later seek, is shown not to reach `RaceSim`.
- The run gives the player a Rocket at tick 10, and changes Pilot Assist on keyframe tick 540 and at
  tick 577. Two of the text targets are the ticks after those changes, because no hash reads the
  level. Dropping the event application fails at tick 541 on Pulse.
- **A visit:** the same inputs and events through a live `Race::tick`, a visit at tick 1,500 with
  four seeks, then 300 more ticks: the live race's hash equals the recorded one at every tick, and
  its text is unchanged across the visit, on all five titles. The fixture does the same with a race
  that never had a visit.
- **The ceiling:** on Pulse, a ceiling a third of the bytes doubles the interval twice, and a seek
  239 ticks from its keyframe reproduces.
- **Split screen:** `head_to_head_people_ground_truth`'s setup with a pad a seat, 7,000 ticks, both
  seats past lap 2: every target reproduces, once the seats are started from the header. **Without
  that it disagrees at tick 0:** `oag_game::split_screen::apply` writes `World::controllers` after
  `Race::start`, the controllers are hashed, and `rebuild` left them at slot 0. `start_recorded` now
  seats `Header::slots`. Pulse only.

**Found by the text, not by a hash:** `change_pilot_assist` writes the level to all eight seats and
records the person-flown ones, and `Race::apply_event` applied each event to its own seat, so a seek
read `[Normal, Off, Off, ...]` against the live race's eight `Normal`. Nothing reads a seat nobody
flies, so no hash could show it. The applier now does what the writer does.

## Cost

`Timeline::open` to the pass ending, release, desktop (Ryzen 9 7900), one title a process, 18,000
ticks, 3 events. Load is the 1-minute average at the end of the run. The sweep:

```sh
OAG_SWEEP=1 OAG_SWEEP_TITLE=<title> OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay \
    --cargo-profile release --run-ignored all --no-capture \
    -E 'test(/replay_timeline_ground_truth::the_opening_pass_costs/)'
```

| Title | `open` | First keyframe (the plans) | Whole pass | Times real time | Load | Decision 3: pass + first tick |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Pulse | 4 ms | 0.20 s | 0.88 s | 340 | 11.6 | 1.03 + 0.12 s (load 7) |
| Pure | 6 ms | 0.92 s | 2.20 s | 136 | 11.4 | 1.36 + 0.34 s (load 7) |
| HD | 13 ms | 0.90 s | 2.58 s | 116 | 10.9 | 3.87 + 0.84 s (load 13) |
| 2048 | 23 ms | 1.10 s | 2.78 s | 108 | 10.6 | 6.38 + 1.59 s (load 19) |
| Omega | 14 ms | 0.52 s | 1.50 s | 200 | 9.4 | 3.20 + 0.77 s (load 28) |

The whole-pass column includes the plans and is on a pad-flown race; decision 3's pass was the
Autopilot's, took a whole clone every 60 ticks and read eight poses a tick, so the two are not one
measurement. On this load every title is inside the 100-times-real-time mark decision 3 sets for the
Deck. **The Deck itself stays owed.** Seeks on the dev profile, load about 10 to 20, median 1 to 7 ms, worst
2 to 22 ms.

## The pose track

Stage 4 of the ADR: the `POSE` chunk records every craft on the grid beside the inputs, for the day
a build no longer reproduces the run (POSE VIEW, decision 4). The `Timeline` does not read it: a
seek resimulates, and a pose track is never a state. **Where it sits:** the codec and the reader in
`oag-replay` (`track.rs`, which knows no race), the capture in `oag-raceplay`'s `Recording`
(`replay/recording.rs`, the `view` half), next to the recorder and outside `RaceSim`, so nothing in
it reaches the state hash. Its layout, its cost on all five titles and the respawn it was shown on
are on [the replay file's page](../formats/replay-file.md) ("The pose track"). The numbers that
bear on this page:

- At Venom, 156 to 176 bytes a craft a second (Phantom: 197 to 263), so a Single Race of eight craft for five minutes is 0.37 to
  0.42 MB: 4.5 percent over the 149 to 168 decision 4 chose, the difference being the key index the
  sweep did not store and a refinement that now holds the stated bound.
- **Capture is a segment at a time.** Refinement works on one 6-tick segment alone, so the recorder
  closes a segment as it ends and keeps keys (20 bytes each, about 300 bytes a craft a second), not a
  pose a tick. Eight craft over a race to the finish hold 270 to 650 KiB; a track stops at an hour
  (`MAX_TICKS`). A race that ends by finish, quit or restart has nothing left to encode:
  `Race::recorded` closes the open segment in a copy (0.05 to 0.19 ms) and the chunk's encoding takes
  1.0 to 5.0 ms (5.0 on 2048's 14,410 ticks, dev profile at load 20), so no worker thread is used.
- A recording that writes no track costs nothing: a race with no `start_recording` does no work.

## The other title of the pair

**Ported.** 2048 and Omega each ran every row above on their own data; nothing here is per title.

## Open

- **The probe's view half after a seek back.** `results` and `wreck_ended_tick` are written once and
  a restore leaves them, so a viewer that reads the probe's board after seeking before the finish
  sees the later one. They do not reach the simulation. Stage 5 owns what the viewer shows.
- **Unrecorded writers.** `oag_game::split_screen::follow_devices` writes `World::controllers`
  outside the tick when a pad leaves a seat. It has no event kind, so such a recording desyncs at
  the next hash. The `controller` kind is reserved; nothing writes it.
- **Modes.** Single Race on every title and Head to Head with two people on Pulse. Time Trial and
  Speed Lap reach the same pass through `replay_ground_truth`'s `Verifier` run, not through a
  `Timeline`. The modes the [replay file](../formats/replay-file.md) does not claim stay unclaimed.
- **An event can apply twice.** A seek that walks on from a `visit` to a tick that took an event
  applies it again. Harmless for `pilot_assist` and `give`; the `controller` kind will need a look.
- **The lock on the page's thread.** `Keyframes::push` can reallocate the list while holding it,
  milliseconds for a list of 25 KB entries. Boxing each keyframe fixes that; stage 5 can do it.
- **Playback through `Race::tick`.** `step` is quiet. Decision 6's drawn 1x playback and
  `Sfx::rebuild` are stage 5.
