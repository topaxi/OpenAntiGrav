# The replay file (`.oagr`)

**Status: built (stage 2 of [ADR-0087](../architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md), 2026-11-07).**
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
  GHST  optional: the ghost lap's pose track
  END_  u64: FNV-1a over every byte before this chunk
```

**A chunk the reader does not know is skipped**, and the `u16` version has not moved: it is bumped
only when a chunk that already exists changes meaning. `EVNT` is written only when there is an
event, so a race with none is the file it was before the chunk existed, apart from the header line.

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
| `[options]` | the launch settings: `scheme`, `difficulty`, `autopilot`, `pilot_assist`, and the original driver's request `ai = "original"`, `ai_level`, `ai_skill` (`oag_raceplay::Launch`; the session flies a front-end race's opponents with it, so a rebuild must). **Read back** by a rebuild; an absent key is the default, an unknown value is refused by name |
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
proves (two people, Pulse); multi-window
races (`seat_picks`); Race Remix (`craft_source`); DLC and
foreign tracks (`dlc`, `foreign_track`); campaign events (`grid_teams`); capture aids (`pose`,
`field`, `hud`); network races (a client records nothing) and modded races (`start_recording`
returns early). None of these is in `[setup]` yet. [The viewer](../ui/replays.md) lists a mode outside the table as MODE NOT
VERIFIED and tries it: Zone, Eliminator and Head to Head with one person each reproduced 752
ticks on Pulse through WATCH.

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
| `controller` | 3 | `0` the opponent driver, `1` a person at this machine | `Race::set_controller`, called by `oag_game::split_screen::follow_devices` when a pad leaves or returns to a seat; applied idempotently. ADR-0087's fork hands a seat nobody took to the driver the same way |

A mid-race Pilot Assist change used to stop the recording (`Race::stop_recording`, removed); it is an
event now, and the file reproduces: the pause-menu test sees `recorded_events` rise and the recording
go on, and the ground truth (`*_records_a_mid_race_pilot_assist_change_and_reproduces`) records a
change and a weapon, reads the file back, re-drives it to the last tick with the events applied, and
shows that a copy with the Pilot Assist event stripped, and a copy with the weapon stripped, each
desync. **Pilot Assist's level is not in the state hash**, so only the hashes after the change can
tell. A cut file (a ghost lap) keeps the events of the ticks it keeps.

## Checked against the other title of the pair

The file knows no title; the rebuild reads each title's own source. 2048 and Omega are both in the
table above with their own tick-zero rows. **ported**: Omega and 2048 use the same `[setup]`,
`Launch` and events as the other three titles.

## Open

Recording every race and the `auto/` ring, the `Timeline` and the REPLAYS page and viewer are built
([replays](../ui/replays.md)). Later work: the `POSE` chunk, `[fork]` written.
