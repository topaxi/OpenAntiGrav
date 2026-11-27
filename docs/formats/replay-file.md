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
| top level | the key (`title`, `track`, `mode`, `class`, `team`; the records key, so lower case), `seed` (**the 64 bits as an `i64`**: a TOML integer is signed and a seed is drawn over all of `u64`; a seed above `i64::MAX` written as it stood made the header unreadable), `tick_rate`, `build` (informational, never compared), `hash_interval`, `slots`, `ticks` (the length, written with the file so the REPLAYS list reads it from `HEAD`) |
| `[finish]` | how the player's craft finished, when the race ended with a result: `place`, `ticks` (written with `ticks`) |
| `[options]` | the launch settings: `scheme`, `difficulty`, `autopilot`, `pilot_assist`, and the original driver's request `ai = "original"`, `ai_level`, `ai_skill` (`oag_raceplay::Launch`; the session flies a front-end race's opponents with it, so a rebuild must), `skill_level`, the rung a campaign cell raced at (`0` Easy to `2` Hard), which picks every craft's shield slot (`docs/formats/handling-stats.md`, "Which difficulty picks the shield slot"; absent: the rung `difficulty` stands for, `oag_ai::Difficulty::level`), and `campaign_skill`, a campaign cell's resolved skill scale (a float), written only for a campaign race, whichever driver flies it. It tunes the field (`oag_ai::Difficulty::tune_at_scale`) and the finished player's thrust, both simulation state, so `RaceSim::apply_launch` repeats it before the first tick (`RaceSim::apply_campaign_skill`); it is its own key because a campaign cell is tuned whichever driver flies it, and a 2048 event's driver skill tunes nothing. **Read back** by a rebuild; an absent key is the default, an unknown value is refused by name |
| `[setup]` | the resolved launch request (`oag_replay::header::Setup`): mode, class, track, team, hull variant, skin, opponent teams, opponents, people, views, lap/weapon/Eliminator overrides, add-on campaign, Zone model. Written by `Race::start_recording` from the request the race was loaded with, so no caller can forget it |
| `[[mods]]` | the simulation modules the race ran, in the order they ran (`oag_replay::header::ModUse`): `id`, `version`, `module` (path in the mod), `sha256` (of the module's bytes), `abi`, `revision`, `state_bytes`, `picks` (the player's `sim` picks by row). Absent for a vanilla race. A rebuild runs exactly these and `apply_recorded` refuses a missing module, another build, another layout or other picks by name ([wasm-extensions](../architecture/wasm-extensions/determinism-and-state.md#replays)) |
| `[[active_mods]]` | every mod that was on for the race, in load order: `id`, `version`. Absent for a race with none. See "Active mods" |
| `[ghost]` | the lap a file's ghost is; never present in the file of a race that ran a simulation module |
| `[fork]` | read, never yet written (stage 7): `tick`, `offered_at`, `seats`, `parent_build`, `parent_hash` |

## Active mods

`oag_replay::Header::active_mods` lists every mod that was on for the race, data-only ones
included (a mod that only carries `fallback/<title>/` entries has no module, so `[[mods]]` never
named it). The built-in extras are the build's own and are left out. A mod is listed when its
`titles` admit the track's or the craft's title, a PPSSPP texture pack only on a PSP source, and a
`network-races = false` mod in no LAN or online race: what the race's archives could mount.
Replays, ghost files and screenshot streams all carry it, because `Race::start_recording` writes
it. The list travels the way the enabled mods' entries do (ADR-0100): the composition root
registers it at start and on every MODS page switch (`oag_game::mods::ForLoads`,
`oag_source::layers::register_enabled`), and the load takes its title's share.

**It decides nothing about validity** (the stored hashes do, ADR-0055). A rebuild
(`Options::apply_replay`, so the viewer, WATCH, a dropped or named file and `--from-png`) compares
the list with the mods on now, and `oag_replay::ActiveMod::differences` names each difference,
mod first:

- `missing <id> <version>`: in the recording, not on now;
- `extra <id> <version>`: on now, not in the recording;
- `<id> is <here> here and <recorded> in the recording`;
- `load order is <here> here and <recorded> in the recording`.

Where it shows:

| Case | What is said |
| --- | --- |
| always, before the race builds | `warn`: `replay: mods differ from the recording: ...` |
| the load or the rebuild fails | the log chain starts with that line (`oag_raceplay::ModsDiffer`); a dropped screenshot's notice says the line alone; the REPLAYS row says the differences alone |
| the race loads and plays | the notice band says the line as the viewer opens, and in place of a screenshot's "other hash" notice |
| `--png-info` | one `# mod on: <id> <version>` line each |

An online room's recording is rebuilt as a network race (`apply_replay` sets `Options::network`),
so a single-player mod it never mounted is not reported as extra.

Measured on Pulse PSP EU with `oag.detonator-pulse` 1.0.0: a Detonator race driven from the menus
wrote the mod into its header; switched off on the MODS page and watched in the same session, the
row said `missing oag.detonator-pulse 1.0.0` and the load failed on the mod's missing layout files,
not on a hash (`drive_ground_truth::mod_fallback_switch`,
`detonator_ground_truth::a_pulse_detonator_replay_names_the_mod_it_is_missing`). A Time Trial
recorded with the mod on still loads with it off, and plays under the notice.

**Not covered**: a player's `overrides/<title>/` directory and a `--menu` file's stale switches are
not in the list. Records say nothing about mods
([asset layers](../architecture/asset-layers.md), "Records and ghosts"). **Checked against the
other titles**: title-agnostic code; **ported** to all five by construction, run on Pulse only.

## Rebuilding a race from a header

`oag_raceplay::rebuild(&header, source)` loads the race `[setup]` describes from `source` (the
reader's own disc image or package for the header's title; it is not in the file), starts it,
applies `[options]` through `RaceSim::apply_launch`, the same call the composition root makes when it
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
tick-zero hash refuses a rebuild that does not match): Zone, Eliminator, Tournament on any title but Pulse and beyond the files named below; Head to Head
and split-screen beyond the one setup [the timeline's page](../architecture/replay-timeline.md)
proves (two people, Pulse); multi-window
races (`seat_picks`); Race Remix (`craft_source`); DLC and
foreign tracks (`dlc`, `foreign_track`); campaign events (`grid_teams`); capture aids (`pose`,
`field`, `hud`); network races beyond the section below. A modded race records, and its `[[mods]]` names the modules. None of these is in `[setup]` yet. [The viewer](../ui/replays.md) lists a mode outside the table as MODE NOT
VERIFIED and tries it: Zone, Eliminator and Head to Head with one person each reproduced 752
ticks on Pulse through WATCH.

**Tournament** is a mode a header names, and `[setup] mode` is read through `Mode::from_recorded_name`,
not `Mode::from_name`: `from_name` is the settings file's and `--mode`'s reader and does not list
Tournament (a campaign's tournament leg is launched by `Session::launch_campaign_cell`, never
chosen), while a replay header records the mode its race ran, so a file must be able to name it. A
campaign recording rebuilds only with its `[options] campaign_skill`; without it the field flies at
another tuning and the file stops matching within seconds. Proved on Pulse only, not in the table
above: `replay_viewer_ground_truth::a_campaign_race_recorded_from_the_menus_reproduces` (the campaign's
first cell, a Single Race against the original driver, recorded from the menus: it records the key,
reproduces every stored hash, and desyncs without it). **Old recordings are not supported across a
simulation or state-hash change** (policy, 2026-11-17): a physics law, a tuning or a new hashed field
desyncs them, at the first stored hash or, when the tick-zero hash moves, at open: a file with a `POSE` chunk then opens in POSE VIEW from tick 0 (the verified prefix is empty), one without is refused naming the missing pose track. No test pins how an
outdated file fails; replay coverage is a recording the same build makes inside the test. 2048's event
campaign (`Launch::Event`, `grid_teams`) is still refused by the tick-zero hash.

The four titles' 7,200-tick Time Trial recordings are re-driven through the rebuild, not through a
hand-made setup (`*_replays_reproduce...`).

## Events (`EVNT`)

ADR-0055's rule is that anything that changes the race outside `Race::tick` must become an input
or be recorded. An `Event` is `(tick, seat, kind, value)`, applied **before** tick `tick` runs, by
whoever re-drives the file (`Verifier::events_due`, then `RaceSim::apply_event`). A seek applies them
the same way, between the same two ticks ([the timeline](../architecture/replay-timeline.md)).

| Kind | Code | Value | Writer |
| --- | --- | --- | --- |
| `pilot_assist` | 1 | the level's index in `Level::ALL` (off 0, normal 1, extreme 2) | `Race::change_pilot_assist`, called by the pause menu (`Session::apply_pause_setting`); one event per person-flown seat, each applied as the writer does, to every seat |
| `give` | 2 | the weapon's index in `Weapon::ALL`, 14 for a mod's `Extension` pickup | `Race::give_weapon`, the flag `--give`, from the session and from a capture alike (writes only an empty pickup slot, and records only a write) |
| `controller` | 3 | `0` the opponent driver, `1` a person at this machine | `Race::set_controller`, called by `oag_game::split_screen::follow_devices` when a pad leaves or returns to a seat; applied idempotently. ADR-0087's fork hands a seat nobody took to the driver the same way |

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
- **A pose** is what `RaceSim::craft_pose` reads: the position, and the orientation with the barrel roll
  and the steering lean folded in, exactly as the hull is drawn (`SimRace::player_pose` is the same
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
  a normalised lerp for the rotation (ADR-0087 decision 4 says slerp; `oag-replay` is in `check-determinism`'s scan, which flags `slerp` and `acos`, and the two gave the same bytes and bound), measured against the keys as a reader rebuilds them, so quantisation is inside the bound).
  A segment is refined alone, so the recorder does it as each closes (`PoseRecorder`): the keys equal
  an offline pass over the whole race (`track::tests::streaming_equals_offline`, byte for byte) and a
  recording holds keys, never a pose a tick.
- **Playback** (`PoseTrack::pose(craft, at: f64)`): lerp (position) and normalised lerp (rotation, the short way round) between the keys around `at`
  ticks, the ends held. Across a jump of more than 16 units in one tick (a respawn) the earlier pose
  is held up to the later tick, so a frame between the two never draws the hull halfway along it. The
  16 is **chosen, not measured**, on the steps seen: the 99.99th percentile of a craft's step is 3.2
  to 4.8 units a tick at Venom (the largest 6.6) and 4.8 to 7.7 at Phantom (the largest 5.8, bar a
  1,104-unit respawn on 2048; the first 6,000 ticks of the Autopilot race on each title,
  `OAG_SWEEP_CLASS=PHANTOM`), and the respawns measured below move 147 to 538 units.
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
| Pure | 149.0 | 155.7 | 0.0163 / 0.0465 / 0.0496 / 0.0500 | 0.069 / 0.241 / 0.373 / 0.496 |
| HD/Fury | 168.3 | 175.5 | 0.0137 / 0.0464 / 0.0495 / 0.0500 | 0.069 / 0.382 / 0.489 / 0.499 |
| 2048 | 162.9 | 169.6 | 0.0159 / 0.0454 / 0.0494 / 0.0500 | 0.079 / 0.309 / 0.444 / 0.499 |
| Omega | 166.6 | 174.0 | 0.0170 / 0.0471 / 0.0496 / 0.0500 | 0.069 / 0.280 / 0.444 / 0.499 |

The file costs **6.7 to 7.4 bytes a craft a second (about 4.5 percent) more** than the row decision 4
chose, and the bound now holds. Two reasons, both measured on these runs: the sweep's encoder did
not store which ticks are keys (a reader cannot decode a refined track without it: a five-bit mask a
segment is 6.2 B a craft a second, a varint tick step a key would be 16.6 to 19.4), and it refined
against the unquantised keys, which left its worst error at 0.057 to 0.059 unit and 0.54 to 0.62
degree (the ADR's own row); refining against the keys as read adds a few keys and holds 0.0500 and
0.499. A pad-flown Single Race to the finish (`pose_track_ground_truth::bounds`, over
`replay_timeline_ground_truth`'s recording) gives 145.5 (Pure, 6,700 ticks), 166.6 (2048, 14,410),
167.2 (Omega, 7,880), 171.0 (Pulse, 8,088) and 171.5 (HD, 8,257), worst error 0.0500 unit and 0.499
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
0.5 degree at every tick of the run, and at the ticks around the jump (-2 to +30), worst position error
Pulse 0.039 unit, Pure 0.048, HD 0.031, 2048 0.035, Omega 0.041 (worst rotation 0.097 to 0.224 degree); the two ticks either side of the jump are both keys,
and a frame between them is the earlier pose. The refinement's handling of a teleport is shown, not argued. The 2048 Phantom Autopilot race above
also contains one it did not provoke (a step of 1,104 units in the first 6,000 ticks) and holds the
same bound.

**Reading it.** An unknown chunk is skipped: `file::tests::a_build_that_does_not_know_the_pose_chunk_skips_it`
renames the tag, redoes the checksum and reads the same recording without a track, which still
reproduces to the last tick. A damaged chunk is refused by byte (`Truncated`): a cut payload, a
trailing byte, a mask bit past the last tick, a track longer than the recording.

**Cost at the end of a race**, dev profile, load average about 20 (so an upper bound): a push a tick
1.5 to 6.1 microseconds for eight craft, `Race::recorded`'s copy of the track 0.05 to 0.19 ms and
the chunk's encoding 1.0 to 5.0 ms (5.0 on 2048's 14,410 ticks). No worker thread is needed. The capture holds
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

## An online race's recording

A client of an online room records only the ticks the server confirmed: a predicted tick is rewound
by every correction, so the recorder is paused for it (`Race::pause_recording`, `Race::tick_confirmed`).
The file's inputs, hashes and `POSE` track are therefore the server's run, and a hit the client
predicted as a miss is in it at the server's tick. Three additions make that file watchable and
comparable:

- **The room is in `[options]`** (`oag_raceplay::replay::Room`, keys `room_people`, `room_assists`,
  `room_barrel_rolls`, `room_zone_target`; `oag_net::race::replay_options` writes them with the
  launch keys). A room's people are `Controller::Remote`, which the state hash reads, and the
  rebuild seats `slots` as local; `RaceSim::apply_room` is the one call `prepare` and the rebuild both
  make. Without the keys the rebuild disagrees at tick 0.
- **`Replay::fingerprint`** (`oag_replay::fingerprint`) hashes what the run is made of: title, circuit,
  mode, class, seed, tick rate, slots, `[options]`, mods, the tick-zero hash, every stored hash, every
  seat's inputs and the events; not the team, build, finish, ghost or pose track. Two clients of one
  race are equal over the ticks both confirmed (`fingerprint_through`). The client keeps it
  locally at the end of the race (`NetPlay::replay_fingerprint`); sending the server's own is a wire
  change (`PROTOCOL` 2).

Proved by `net_ground_truth::a_predicting_clients_recording_is_the_confirmed_run_and_re_drives`
(Pulse; corrections happen, the viewer's own rebuild reaches the recorded tick-zero hash, a full-tick
re-drive of the confirmed inputs draws the pose track the client kept, the two clients' fingerprints
are equal) and `fingerprint::tests`. Not yet recorded: a seat's retirement (`oag_net::race::retire`)
is not an `EVNT` entry, so a recording that spans one does not re-drive past it. **Checked against
the other titles**: the room, the fingerprint and the recording are title-agnostic code; only Pulse
is exercised.

## Checked against the other title of the pair

The file knows no title; the rebuild reads each title's own source. 2048 and Omega are both in the
table above with their own tick-zero rows. **ported**: Omega and 2048 use the same `[setup]`,
`Launch` and events as the other three titles. The pose track is **ported** the same way: each title
was recorded and measured on its own data (the tables above), and the respawn ran on each.

## Compression, measured

2026-11-17. Nothing in a `.oagr` is compressed today (the screenshot chunk is deflated). This
section measures what a general compressor would save, per chunk, on all five titles at Venom and
Phantom, and recommends. The dependency decides the answer, so it comes first.

**Which encoder.** `ruzstd` 0.9.1 (the workspace's zstd, a pure-Rust crate used for `.zim`
pictures) **encodes only `Uncompressed` and `Fastest`**; `Default` (zstd 3), `Better` (7) and `Best`
(11) are marked `UNIMPLEMENTED` in its source. The zstd levels below (3, 9, 19) therefore need the
C `zstd` crate, which the mod loader's `zip` already links, native-only
(`cargo tree -i zstd -e normal`: `oag-mod`, `oag-source` and everything above them, including
`oag-net`'s dependents). `oag-replay` is not native-only. `oag-replay` already depends on `flate2`
(zlib-rs, every target; `shot.rs` deflates with it), so **deflate costs no new dependency**. The
workspace's LZMA crate (`lzma-rs-perf-exp`, for Unity bundles) was not measured as an encoder; xz is
shown only as the ceiling.

**Method.** Recordings: for each title and class, (a) the cost sweep's race, the player's craft on
Autopilot with idle input, 18,000 ticks, eight craft; (b) the pad-flown Single Race of
`replay_timeline_ground_truth` (an AI driver emitting pad values, with an assist change and a
given weapon, see `write_between_ticks`), for 18,000 ticks; (c) the same stopped 300 ticks after the
finish (at least 6,700 ticks). Written by the scratch sweep
`replay_timeline_ground_truth::dump_the_recordings_for_a_compressor`:

```sh
OAG_SWEEP=1 OAG_SWEEP_TITLE=<title> OAG_SWEEP_CLASS=<VENOM|PHANTOM> OAG_SWEEP_OUT=<dir> \
  OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --cargo-profile quick \
  --run-ignored all --no-capture -E 'test(/dump_the_recordings/)'   # OAG_SWEEP_FINISH=1 for (c)
```

Each file was split by its chunk framing (tag, u32 length, payload); `INPT` is every seat's
payloads joined. Compressors: the `zstd` (levels 1, 3, 9, 19), `gzip -9` and `xz -9` command-line
tools, one process per chunk; percent saved is `100 * (1 - compressed / raw)`. `ruzstd`'s
`Fastest` was run in-process over the same chunks. **Times** are best of 3 (CLI) or 5 (ruzstd)
and the CLI ones include process start-up (about 2 ms), so they are upper bounds; the machine was
loaded (load average 2 to 23 across the runs). Pulse Venom Autopilot `POSE` reproduces ADR-0087's
general-compressor rows (gzip -9 10.4 percent against its 9, xz -9 28.7 against 28).

**What the recordings are.** Neither carries a person's input. The pad race is an AI driver
(`replay_ground_truth::Pad`; Pure's is the race's own driver) emitting stick and button values; a
human's stick is probably noisier, so its `INPT` compresses worse than the bot's, not better. The
Autopilot race has no human seat: its `INPT` is 8 bytes. (a) and (b) run 18,000 ticks whatever the
finish, which on Pure Phantom (finish 4,187) and 2048 Phantom (5,309) is mostly input after the
finish; (c) removes that. Finish ticks, Venom / Phantom: Pulse 7,889 / 7,774, Pure 8,563 / 4,187,
HD 8,083 / 5,960, 2048 14,097 / 5,309, Omega 7,643 / 8,778.

**Percent saved, `POSE`** (the Autopilot race, (a); the pad-flown race, (b), is within 2 points of it):

| Title, class | raw bytes | zstd 3 | zstd 19 | gzip -9 | xz -9 | ruzstd Fastest |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Pulse Venom | 418,288 | 8.5 | 16.5 | 10.4 | 28.7 | 3.4 |
| Pulse Phantom | 525,745 | 9.4 | 17.0 | 11.3 | 28.6 | 4.4 |
| Pure Venom | 356,070 | 10.3 | 16.9 | 12.5 | 29.0 | |
| Pure Phantom | 454,367 | 11.1 | 19.4 | 12.9 | 32.6 | |
| HD Venom | 420,285 | 8.3 | 15.7 | 10.7 | 27.0 | |
| HD Phantom | 552,499 | 7.5 | 15.0 | 9.5 | 26.5 | |
| 2048 Venom | 406,411 | 9.2 | 13.2 | 10.5 | 25.5 | |
| 2048 Phantom | 537,886 | 9.3 | 13.2 | 10.5 | 23.5 | |
| Omega Venom | 417,473 | 8.4 | 15.2 | 10.4 | 27.4 | |
| Omega Phantom | 575,659 | 6.7 | 12.8 | 8.6 | 23.5 | |

`ruzstd` Fastest over `POSE`: 1.6 to 7.4 percent on every file (mean 3.5). zstd 9 sits within 1
point of zstd 3 on `POSE` and `INPT`.

**Percent saved, `INPT`** (pad-flown race stopped after the finish, (c); one seat):

| Title | Venom bytes | zstd 3 | zstd 19 | gzip -9 | xz -9 | Phantom bytes | zstd 3 | zstd 19 | gzip -9 | xz -9 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Pulse | 55,063 | 26.5 | 42.9 | 32.2 | 43.9 | 83,913 | 47.0 | 54.4 | 48.1 | 56.8 |
| Pure | 64,187 | 29.0 | 43.6 | 34.9 | 45.5 | 38,560 | 39.2 | 50.5 | 43.8 | 51.7 |
| HD | 57,169 | 26.1 | 44.2 | 32.8 | 45.8 | 68,109 | 45.9 | 54.4 | 48.0 | 57.2 |
| 2048 | 88,723 | 24.8 | 44.2 | 32.1 | 43.8 | 44,233 | 33.9 | 48.1 | 38.6 | 48.4 |
| Omega | 51,131 | 23.3 | 42.2 | 30.7 | 43.6 | 86,939 | 47.1 | 55.1 | 48.1 | 57.2 |

That is 6.2 to 7.2 bytes a tick at Venom (ADR-0055: 6.3 to 6.9), and the compressor finds
the repeats of a held stick. `ruzstd` Fastest: 4 to 11 percent at Venom, 17 to 35 at Phantom.

**Percent saved, the whole file** (pad-flown race, finish + 300 ticks, (c); the Autopilot race
(a) in brackets, whose `INPT` is empty):

| Title, class | raw bytes | zstd 3 | zstd 19 | gzip -9 | xz -9 | ruzstd Fastest |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Pulse Venom | 311,016 | 9.6 (7.3) | 17.4 (15.0) | 12.7 (9.7) | 29.9 (28.6) | 3.3 |
| Pulse Phantom | 382,073 | 14.7 (8.8) | 20.8 (16.1) | 16.4 (10.7) | 29.8 (28.2) | 9.1 |
| Pure Venom | 310,256 | 11.4 (8.7) | 18.1 (15.2) | 14.3 (11.3) | 31.2 (28.0) | 4.2 |
| Pure Phantom | 252,586 | 12.4 (10.2) | 19.0 (18.3) | 15.2 (12.2) | 29.9 (31.9) | 6.3 |
| HD Venom | 320,913 | 9.4 (7.3) | 17.4 (14.3) | 13.0 (9.9) | 27.3 (27.0) | 3.3 |
| HD Phantom | 323,649 | 13.7 (5.9) | 18.8 (14.2) | 15.6 (9.0) | 28.7 (26.3) | 8.3 |
| 2048 Venom | 536,615 | 10.7 (7.9) | 17.2 (11.3) | 12.8 (9.2) | 27.9 (26.2) | 2.3 |
| 2048 Phantom | 310,687 | 10.7 (8.4) | 15.1 (12.1) | 12.9 (9.9) | 21.9 (23.5) | 4.2 |
| Omega Venom | 295,543 | 9.5 (7.4) | 17.4 (13.7) | 12.6 (9.6) | 30.2 (27.2) | 2.8 |
| Omega Phantom | 423,733 | 14.5 (6.0) | 19.8 (12.2) | 16.0 (8.3) | 28.3 (22.8) | 8.4 |

**The other chunks.** `HASH` (2,410 bytes at 18,000 ticks, one 8-byte hash a second) does not
compress: zstd grows it by 0.5 percent, gzip by 1, xz by 2.9. `EVNT` (15 bytes in these runs) and
`HEAD` (about 570 bytes of TOML, 33 to 37 percent with zstd or gzip, 22 with xz) are too small
to matter; a compressor grows an `EVNT` of that size by 87 percent or more.

**Speed**, the whole file (c), 10 files, median / worst, milliseconds, encode then decode: zstd 1
2.2 / 2.4 then 1.3 / 1.5; zstd 3 2.3 / 3.3 then 1.3 / 1.6; zstd 9 5.4 / 6.7 then 1.3 / 1.6; zstd 19
51 / 74 then 1.6 / 2.1; gzip -9 18 / 60 then 1.7 / 6.2; xz -9 82 / 126 then 6.9 / 12.5. `ruzstd`
Fastest: encode 5 to 9 ms, decode 0.2 to 0.6 ms in-process. Even xz at the worst is
a one-off at the end of a race (the chunk's own encoding is 1 to 5 ms, "Cost at the end of a
race"), so speed does not decide anything; the saved bytes and the dependency do.

**Recommendation.** The keep-or-drop line used here, 10 percent of the file and a dependency that
builds on every target the replay crate does, is **chosen, not measured**.

1. **Do not wrap the file in `ruzstd`.** Its encoder reaches 2 to 9 percent of the whole file, a
   third to two thirds of what the same format saves at
   level 3. There is no pure-Rust encoder at any useful level.
2. **Do not compress `HASH`, `EVNT` or `HEAD`.** They grow or are under 600 bytes.
3. **`POSE` is worth little by itself**: 7 to 11 percent at zstd 3, 13 to 19 at zstd 19, 9 to 13
   with deflate -9. The track is already delta-coded and refined (ADR-0087 decision 4); only xz
   (21 to 33 percent) finds structure in it, and no encoder for it is wired.
4. **`INPT` is the chunk that earns it**: 23 to 29 percent at Venom and 34 to 47 at Phantom with zstd
   3, deflate -9 31 to 35 and 38 to 48. A human's stick would compress less (above).
5. **If anything is wired, deflate -9 over the whole payload after `HEAD`** (`flate2` is already
   an `oag-replay` dependency, works on every target, and saves 12.6 to 16.4 percent of the whole
   file on a finish-length race, 8 to 12 percent on an Autopilot one). Its cost is the
   format, not measured: the container changes, so a new `FORMAT_VERSION` or a new chunk (a design
   note, not a finding).
6. **zstd 19 through the C crate** saves 15 to 21 percent (2 to 5 points more than deflate) and
   is native-only like `oag-mod`. It is the option to revisit if the materialized replay (this
   thread) roughly doubles the stored size and a native-only container is acceptable. The
   dependency choice is the maintainer's.

A five-minute race is 250 to 540 KB uncompressed, so the best of these saves 30 to 95 KB a file.
Whether that justifies a format bump is the maintainer's call; the measurement does not make the
case on its own.

## Open

Recording every race and the `auto/` ring, the `Timeline`, the REPLAYS page and the viewer are built
([replays](../ui/replays.md)); POSE VIEW plays the track past the last second a build reproduces.
Later work: `[fork]` written. A track holds hulls only; nothing else a craft does is in it.
