# The replay file (`.oagr`)

**Status: built (stages 2 and 4 of [ADR-0087](../architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md), 2026-11-07).**
Code: [`oag-replay`](../../crates/replay) (the container), [`oag-raceplay`](../../crates/raceplay)
(`replay/`: the launch request, the rebuild, the event writers), `oag_game::ghosts` (the files in
`ghosts/`). Design: [ADR-0055](../architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md)
(inputs are the truth, poses are a cache) and ADR-0087 decisions 7 to 9. **Chosen, not measured**:
this is this project's own format; nothing in it is read off an original.

## The container

```text
"OAGR"  u16 version (1)  u16 reserved (0)
chunk*: [u8; 4] tag, u32 length, payload      (all integers little-endian)
  HEAD  the Header, as TOML text, its first line the toml-schema line
  INPT  one per recorded slot: u8 slot, then the input codec's stream
  HASH  u64 initial hash, varint count, count x u64
  EVNT  optional: varint count, then per event: varint tick, u8 seat, u8 kind, varint value
  GHST  optional: the ghost lap's pose track (one craft, 28 bytes a tick)
  POSE  optional: every craft on the grid, as refined keys (below)
  END_  u64: FNV-1a over every byte before this chunk
```

**A chunk the reader does not know is skipped**, and the `u16` version has not moved: it is bumped
only when a chunk that already exists changes meaning. `EVNT` is written only when there is an
event, so a race with none is the file it was before the chunk existed, apart from the header line.
`POSE` is written only by a run that recorded one (`Race::recorded`); a ghost file cut from a run
(`take_new_best_ghost`) carries none.

## `HEAD` and its schema

`HEAD` starts with the one-line `toml-schema` header (kind `replay-header`, major 1; see
[TOML Schema](toml-schema.md)). The schema is [`schemas/v1/replay-header.tosd`](../../schemas/v1/replay-header.tosd),
deployed with the others. What the loader does with the line:

| `HEAD` | Result |
| --- | --- |
| line present, `replay-header` v1 | loads |
| no line (a file from before it existed) | loads; `oag_game::ghosts` logs it at `debug` and the next write adds the line |
| another kind, or a later major | refused by name through `oag_schema::check` (`ReadError::Schema`); a stored ghost is ignored with that message at `warn` |

The reader tolerates a key it does not know (a file a later build wrote within v1) and the schema is
closed. `crates/replay/tests/replay_header_schema.rs` runs accepted and rejected headers through
both and states each place they differ on purpose (an unknown key, a `parent_hash` that is not hex,
a missing line). `options` is declared as `type = "collection"`, `itemtype = "string"`, the way
`settings.tosd` declares its string maps; `tosd` 1.0.0-rc.2 can say an open table of strings, so the
table keeps its keys open (ADR-0087 decision 8).

| Table | What it is |
| --- | --- |
| top level | the key (`title`, `track`, `mode`, `class`, `team`), `seed`, `tick_rate`, `build` (informational, never compared), `hash_interval`, `slots` |
| `[options]` | the launch settings: `scheme`, `difficulty`, `autopilot`, `pilot_assist` (`oag_raceplay::Launch`). **Read back** by a rebuild; an absent key is the default, an unknown value is refused by name |
| `[setup]` | the resolved launch request (`oag_replay::header::Setup`): mode, class, track, team, hull variant, skin, opponent teams, opponents, people, views, lap/weapon/Eliminator overrides, add-on campaign, Zone model. Written by `Race::start_recording` from the request the race was loaded with, so no caller can forget it |
| `[ghost]` | the lap a file's ghost is |
| `[fork]` | read, never yet written (stage 7): `tick`, `offered_at`, `seats`, `parent_build`, `parent_hash` |

## Rebuilding a race from a header

`oag_raceplay::rebuild(&header, source)` loads the race `[setup]` describes from `source` (the
reader's own disc image or package for the header's title; it is not in the file), starts it,
applies `[options]` through `Race::apply_launch`, the same call the composition root makes when it
launches a race, and seats `slots` as local people, which a split screen writes after
`Race::start` and the state hash reads. **The tick-zero hash is the safety net**: a rebuild that is missing something
disagrees at tick 0 and `Verifier::new` refuses it by name; a header with no `[setup]` is refused
before any load.

Proved by `crates/raceplay/tests/replay_ground_truth.rs` (`*_rebuilds_from_its_header_at_tick_zero`):
the file's bytes are read back, the race is rebuilt from the header alone, and its tick-zero hash
agrees with the recorded one; the same file with another mode in `[setup]` disagrees at tick 0 on
every title, so the table is what the rebuild uses.

| Title | Time Trial | Speed Lap | Single Race |
| --- | --- | --- | --- |
| Pulse (PSP) | proved | proved | proved |
| Pure (PSP) | proved | proved | proved |
| HD (PS3) | proved | proved | proved |
| 2048 (Vita) | proved | proved | proved |
| Omega (PS4) | proved | proved | proved |

**Not claimed** (a recording of these may be written, and its rebuild is then not promised; the
tick-zero hash refuses a rebuild that does not match): Zone, Eliminator, Tournament; Head to Head
and split-screen beyond the one setup [the timeline's page](../architecture/replay-timeline.md)
proves (two people, Pulse); a seat a pad left mid-race (`follow_devices`, no event yet); multi-window
races (`seat_picks`); Race Remix (`craft_source`); DLC and
foreign tracks (`dlc`, `foreign_track`); campaign events (`grid_teams`); capture aids (`pose`,
`field`, `hud`); network races (a client records nothing) and modded races (`start_recording`
returns early). None of these is in `[setup]` yet.

The four titles' 7,200-tick Time Trial recordings are re-driven through the rebuild, not through a
hand-made setup (`*_replays_reproduce...`).

## Events (`EVNT`)

ADR-0055's rule is that anything that changes the race outside `Race::tick` must become an input
or be recorded. An `Event` is `(tick, seat, kind, value)`, applied **before** tick `tick` runs, by
whoever re-drives the file (`Verifier::events_due`, then `Race::apply_event`). A seek applies them
the same way, between the same two ticks ([the timeline](../architecture/replay-timeline.md)).

| Kind | Code | Value | Writer |
| --- | --- | --- | --- |
| `pilot_assist` | 1 | the level's index in `Level::ALL` (off 0, normal 1, extreme 2) | `Race::change_pilot_assist`, called by the pause menu (`Session::apply_pause_setting`); one event per person-flown seat, each applied as the writer does, to every seat |
| `give` | 2 | the weapon's index in `Weapon::ALL` | `Race::give_weapon`, the capture flag `--give` (writes only an empty pickup slot) |
| `controller` | 3 | reserved for a seat handed to the opponent driver at a fork (decision 9) | not implemented; a file carrying it reads, and `Race::apply_event` refuses it by name |

A mid-race Pilot Assist change used to stop the recording (`Race::stop_recording`, removed); it is an
event now, and the file reproduces: the pause-menu test sees `recorded_events` rise and the recording
go on, and the ground truth (`*_records_a_mid_race_pilot_assist_change_and_reproduces`) records a
change and a weapon, reads the file back, re-drives it to the last tick with the events applied, and
shows that a copy with the Pilot Assist event stripped, and a copy with the weapon stripped, each
desync. **Pilot Assist's level is not in the state hash**, so only the hashes after the change can
tell. A cut file (a ghost lap) keeps the events of the ticks it keeps.

## The pose track (`POSE`)

Every craft on the grid, recorded while the race is driven, for the day the inputs no longer
reproduce the run (ADR-0087 decision 4): a **picture** of the race on the build that recorded it,
hulls only, never a verified state. Code: `oag_replay::track` (the codec and the reader; it knows
no race), `oag_raceplay::replay::recording` (the capture).

```text
POSE  varint last tick (poses exist for ticks 0..=last)
      u8 craft count (craft i = grid slot i)
      per craft:  varint key bytes
                  mask: ceil(5 * ceil(last / 6) / 8) bytes
                  keys: per key  3 x zigzag varint (position step), u32 LE (rotation)
```

- **Tick 0** is the state before the recording's first tick, the same index as `INPT` (`inputs[k]`
  is the tick that moves `k` to `k + 1`) and `Replay::ticks()`; a track's `last` is at most that.
- **A pose** is what `Race::craft_pose` reads: the position, and the orientation with the barrel roll
  and the steering lean folded in, exactly as the hull is drawn (`Race::player_pose` is the same
  function for the player).
- **Position** is quantised to 1/64 unit and coded as the step from the previous key (the first key
  from zero) in each axis, zigzag then LEB128.
- **Rotation** is a smallest-three quaternion in four bytes: the largest component dropped (sign
  chosen so it is positive), the other three in ten bits each over +-1/sqrt 2, two bits (the top two)
  for which was dropped.
- **Which ticks are keys.** Every 6th tick from 0 and the last tick always are; they are not stored.
  An interior key is one bit in the mask: five bits a 6-tick segment, bit `j` of segment `s` meaning
  tick `6s + j + 1`, LSB first. A bit for a tick at or past `last` (the padding included) is refused.
  Keys are read in tick order, so the number of keys is fixed by `last` and the mask.
- **Refinement.** Inside each segment the tick that interpolates worst becomes a key, and again,
  until no tick between two keys is more than **0.05 unit or 0.5 degree** from the truth (lerp and
  slerp, measured against the keys as a reader rebuilds them, so quantisation is inside the bound).
  A segment is refined alone, so the recorder does it as each closes (`PoseRecorder`): the keys equal
  an offline pass over the whole race (`track::tests::streaming_equals_offline`, byte for byte) and a
  recording holds keys, never a pose a tick.
- **Playback** (`PoseTrack::pose(craft, at: f64)`): lerp and slerp between the keys around `at`
  ticks, the ends held. Across a jump of more than 16 units in one tick (a respawn) the earlier pose
  is held up to the later tick, so a frame between the two never draws the hull halfway along it. The
  16 is **chosen, not measured**, on the steps seen: the 99.99th percentile of a craft's step is 4.8
  to 5.0 units a tick at Venom and 4.8 to 7.7 at Phantom (the first 6,000 ticks of the Autopilot race
  on each title, `OAG_SWEEP_CLASS=PHANTOM`), and the respawns measured below move 147 to 538 units.
  A craft that flew faster than 16 units a tick (a class or a mode not swept) would hold for a frame
  wherever refinement left a one-tick segment; that is open. The recorded ticks are exact either way.
- **Bounds.** `MAX_TICKS` is 216,000 (one hour): the recorder takes nothing past it, and the track
  ends there. A capture holds 20 bytes a key in memory, about 300 bytes a craft a second.
- **Chosen, not measured:** the 1/64 step, the 10-bit rotation, the 6-tick base, the tolerances and
  the 16-unit jump. **Measured:** what they cost and lose, below.

**What it costs and loses**, 8 craft, the cost sweep's Autopilot race of 18,000 ticks (the race
ADR-0087 decision 4 measured; one title a process, `OAG_SWEEP=1 OAG_SWEEP_TITLE=<title>`). Bytes a
craft a second; error against the true pose at every tick, p50 / p99 / p99.9 / max:

| Title | ADR-0087's row (sweep encoder) | `POSE` as written | Position, units | Rotation, degrees |
| --- | ---: | ---: | --- | --- |
| Pulse | 167.5 | 174.8 | 0.0140 / 0.0470 / 0.0496 / 0.0500 | 0.069 / 0.319 / 0.488 / 0.499 |
| Pure | 149.0 | 155.7 | 0.0163 / 0.0465 / 0.0496 / 0.0500 | 0.069 / 0.244 / 0.373 / 0.496 |
| HD/Fury | 168.3 | 175.5 | 0.0137 / 0.0464 / 0.0495 / 0.0500 | 0.079 / 0.382 / 0.489 / 0.499 |
| 2048 | 162.9 | 169.6 | 0.0159 / 0.0454 / 0.0494 / 0.0500 | 0.079 / 0.312 / 0.444 / 0.499 |
| Omega | 166.6 | 174.0 | 0.0170 / 0.0471 / 0.0496 / 0.0500 | 0.079 / 0.280 / 0.442 / 0.499 |

The file costs **6.7 to 7.4 bytes a craft a second (about 4.5 percent) more** than the row decision 4
chose, and the bound now holds. Two reasons, both measured on these runs: the sweep's encoder did
not store which ticks are keys (a reader cannot decode a refined track without it: a five-bit mask a
segment is 6.2 B a craft a second, a varint tick step a key would be 16.6 to 19.4), and it refined
against the unquantised keys, which left its worst error at 0.057 to 0.059 unit and 0.54 to 0.62
degree (the ADR's own row); refining against the keys as read adds a few keys and holds 0.0500 and
0.499. A pad-flown Single Race to the finish (`pose_track_ground_truth::bounds`, over
`replay_timeline_ground_truth`'s recording) gives 145.5 (Pure, 6,700 ticks), 166.6 (2048, 14,410),
167.2 (Omega, 7,880), 170.9 (Pulse, 8,088) and 171.5 (HD, 8,257), worst error 0.0500 unit and 0.499
degree on all five.

**Class matters.** The table is Venom. The same Autopilot race at Phantom, first 6,000 ticks (Venom
over the same stretch: Pulse 171.3, 2048 167.5): Pulse 230.3, Pure 196.9, HD 230.1, 2048 263.1, Omega
246.3 bytes a craft a second, the error bound unchanged (0.0500 unit, 0.499 degree). A faster craft
bends the line more per tick and refinement adds keys, so a Phantom race costs 15 to 57 percent more.
The classes between, Zone and the other modes are not swept.

**A respawn** (`pose_track_ground_truth::*_pose_track_bounds_a_respawn`): the pad flies to tick 600,
the test lifts the player's craft 500 units off the circuit, and the game's own rescue puts it back
(Pulse: the collision cage, the next tick, a step of 146.8 units; Pure, HD, 2048 and Omega: the
200-unit distance rescue 30 ticks later, a step of 522 to 538 units). The track holds 0.0500 unit and
0.5 degree at every tick of the run, and at the ticks around the jump (-2 to +30): Pulse 0.039 unit,
Pure 0.048, HD 0.031, 2048 0.035, Omega 0.041; the two ticks either side of the jump are both keys,
and a frame between them is the earlier pose. The refinement's handling of a teleport is shown, not argued. The 2048 Phantom Autopilot race above
also contains one it did not provoke (a step of 1,104 units in the first 6,000 ticks) and holds the
same bound.

**Reading it.** An unknown chunk is skipped: `file::tests::a_build_that_does_not_know_the_pose_chunk_skips_it`
renames the tag, redoes the checksum and reads the same recording without a track, which still
reproduces to the last tick. A damaged chunk is refused by byte (`Truncated`): a cut payload, a
trailing byte, a mask bit past the last tick, a track longer than the recording.

**Cost at the end of a race**, dev profile, load average about 30 (so an upper bound): a push a tick
4 to 26 microseconds for eight craft, `Race::recorded`'s copy of the track 0.13 to 0.21 ms and
`to_bytes` 2.5 to 4.6 ms on 2048's 14,410 ticks. No worker thread is needed. The capture holds
270 to 650 KiB for eight craft over a race to the finish. The capture does not change the race, and
the evidence differs by title: on Pulse, Pure, HD and 2048 a recording race hashes as a plain one
at every tick of 7,200 (`replay_ground_truth::ghost_changes_nothing`, which records in the haunted
twin); on Omega, which that test does not cover, the recording run's hashes are reproduced by the
`Timeline`'s probe, which never records, at the opening pass's per-second hashes and 40 seek targets
(`replay_timeline_ground_truth::omega_seeks_reproduce_a_pad_flown_race`), and the same seeks do so on
the other four.

**A restart or a quit** drops the race, and its recording with it. `Race::recorded` is valid at any
tick (the open segment is closed in a copy), so writing the file before the drop is the session's:
`restart_parked_race` and `quit_parked_race` build or drop the race, and neither touches the track.

## Checked against the other title of the pair

The file knows no title; the rebuild reads each title's own source. 2048 and Omega are both in the
table above with their own tick-zero rows. **ported**: Omega and 2048 use the same `[setup]`,
`Launch` and events as the other three titles. The pose track is **ported** the same way: each title
was recorded and measured on its own data (the tables above), and the respawn ran on each.

## Open

Later work: recording every race and the `auto/` ring, any screen (POSE VIEW included), `[fork]`
written, the `controller` event. A track holds hulls only; nothing else a craft does is in it.
