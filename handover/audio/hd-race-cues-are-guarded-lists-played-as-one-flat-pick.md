---
status: ready-to-wire
titles: [hd]
updated: 2026-11-27
summary: "Open: held cues behind 0x04/0x05, ~SHIP_ONFIRE, cockpit view, front-end bend, cues with unread triggers"
size: M (45)
difficulty: medium (55)
---

# HD's race cues are guarded lists, played as one flat pick

**2026-11-16 (lane `hd-impact`):** the collision family (`.COLLISIONS`, `.GRIND`, `.COLLSHIELD`,
`.GRINDSHIELD`) is wired from its own law; its leftovers are in
[hd-collisions-flatten-a-severity-tree.md](hd-collisions-flatten-a-severity-tree.md). Mentions of
the collision severity as open below are the state before.

2026-11-15, lane `hd-sound-audit` measured it; lane `hd-cue-walk` wired rows 1, 3 and 4 (below). The maintainer
plays HD/Fury in RPCS3 and hears HD's sound as wrong in places, `SPEEDUPPAD` first. The
measurement, the ranked table and every address are on
[cue-variables.md](../../docs/ghidra/functions/ps3-hdfury-eu/cue-variables.md). In one line:
HD's command lists read four **cue variables** (perspective, severity, surface) and HD's operand
bytes sit one place over from Pulse's, and this engine walks HD's race cues with neither, so the
walk stops and the cue plays as one waveform drawn from everything the list can reach.

Related threads: [hd-collisions-flatten-a-severity-tree.md](hd-collisions-flatten-a-severity-tree.md)
(the severity rule's remaining half), [hd-2048-rear-occlusion.md](hd-2048-rear-occlusion.md).
The thread "a cue can play other cues" is closed by the page: its last open item, which variable
is the severity and which the surface, is answered there (local variables 0 and 1, not the global
table). The Zone class-step cues
(`ZONEBAR_TRANS`, `ZONEADVANCE`, `MR_*`) are another lane's and are not touched here.

## Landed (lane `hd-cue-walk`)

- ~~HD walk~~: `WalkModel::cue_variables` (`oag_formats::sblk::timeline`), selected by `SequenceTick::walks_cue_variables`
  (the PS3 tick), reads `0x19`, `0x1b`, `0x22`, `0x25` with HD's operand bytes, writes `0x1e`/`0x20` into the handler's
  four variables (a child starts with a copy), walks `0x25` as an alternate group, ends on `0x18`, and re-pitches the
  voices already keyed on a late `0x1b`. A variable outside the handler's four is unread. Unit tests in
  `crates/formats/src/sblk/timeline/tests.rs`.
- ~~Per-perspective timelines~~: `CueTables::variables(player)` gives the perspective; `Banks` builds the other craft's
  timelines and the player's own (`player_timelines`, read by `voices_for`), both or neither.
- ~~Row 1, `SPEEDUPPAD`~~: the player's pad is the body and two rear whooshes (220 and 140 degrees, 10 ticks apart, one
  bend draw), another craft's the body and one whoosh. `sfx_timeline_ground_truth::hd_s_race_cues_play_the_lists_the_bank_authors`.
- ~~Rows 3 and 4~~: every one-shot hud, ship and weapons cue but `.COLLISIONS` plays its layered voices
  (same test, `hd_s_explosion_and_launch_cues_*`); `ABSORB` is silent for another craft, as authored.
- ~~Row 6~~ for the race cues (`PLASMAHITWALL`, `QUAKELAUNCH`): the bend reads 100 %.
- ~~Row 8~~ for the race cues: a rear-half grain pans as `sin(angle)` (`RearPan::Fold`, `oag_audio::spatial::pan_of_angle_folded`,
  the rear branch of `Scream_PanVolumePair` without its phase inversion, **chosen**; read on Pulse's executable, HD's own
  table is unread). Census of the loader report (51 cue lines of `--race --dry-run`): cues playing a timeline 16 before, 38
  after; flat picks 35 before, 13 after (race cues 34 before, 12 after: `.COLLISIONS` and the 11 held cues; `cont_elim`
  is speech).
- A combination space past 64 (explosions draw 5 to 17 independent groups) is sampled, 64 timelines from a seed
  (`Bank::cue_timelines_sampled`), and a play picks one at random: the original redraws every group on each play, so
  this is a sample of its draw, not the whole space.

## Open

1. **`.COLLISIONS`** (row 2): variable 1 picks ship or wall, variable 0 the severity child, then 3 to 6 voices. The walk
   handles `0x08` as a child with the variables (unit-tested), but the race model leaves `0x08` unwalked, so the cue keeps
   its flat pick: what the game computes the severity from is unread (`FUN_000d8bd0`, `0x000d8bd0`). Read it, then wire the
   surface and severity, then `.COLLSHIELD`, `.GRIND` and `.GRINDSHIELD` beside it.
2. **The other held cues** (row 5): ~~`~BLOWUP`, `~ROCKLOCK`~~ (2026-11-27), ~~`~AUTOPILOT`, `~PLASMATVL`,
   `~ROCKETTVL`, `~SHURIKENTRAVEL`, `~QUAKETRAVEL`~~ run their lists, started and stopped where HD does
   (cue-variables.md "Held cues the game starts and stops"). **None was heard against the original.** Left:
   - `~SHIP_ONFIRE`: the runner accepts it (two loops, no bend), but `slot_held` keeps its one waveform and its
     start and stop on HD are unread.
   - `0x04` blocks `~SHIELD`, `~LEACHATTACH`, `~MISSILETVL`, `~SHIP_DESTROYED`; `0x05` blocks `~magstrip01`. Read
     `0x04`'s targets (`0x006285f0` and the per-tick update).
   - 2048: its `weapons.bnk` ships the same lists (`~AUTOPILOT` with `0x30` for `0x19`); checked, applies in shape, not
     wired: the loader asks its hashed bank by spelled name and finds none of them, and the Vita handlers are unread.
   - Omega: checked, differs (Wwise events).
3. **The cockpit view**: the player's perspective is 1 (chase) for a cue on the player's craft; the original makes it 0
   in the cockpit view (`Ship_UpdateEngineCrossfade`, `craft + 0x5f42`). The sound crate does not see the camera.
   **Chosen**, as are `ABSORB`, `SHURIKEN` and the player's `CANNON`, which read variable 2 and have no live capture of
   their own (`CueTables::variables`). Run `scripts/rpcs3-hd-cue-handlers.py --sweep 0,1,2,...,10` on a weapons-on state.
4. **Front-end cues** (`accept`, `reject`, `navUp`, `navDown`): still read with Pulse's `0x1b` byte (0 % bend) and the
   centred rear voice; the front end's walk was left alone.
5. **Not ready, each needs a read first**: the cues the original starts and we have no cue for (`.COLLSHIELD`,
   `.GRINDSHIELD`, `.GRIND`, `WEAPONPICKUP`, `MINERADAR`, `~BOMBRADAR`, `BOMBLAUNCH`, `BOMBEXPL`, `MINEEXPL`,
   `~SHIP_ONFIRE`, `CANNON_VO`, `mines`, `FINAL_LAP`, `RACE_COMPLETE`: triggers unread), the ADSR words, the eight-float
   block, and which weapons bank Zone, Detonator and Eliminator load.

## Next Steps

1. Read `FUN_000d8bd0` for the collision severity thresholds, then wire `.COLLISIONS`' variables 0 and 1 (walk `0x08`
   as a child under the race model) and add `.COLLSHIELD`, `.GRIND` and `.GRINDSHIELD`.
2. ~~Held cues: the runner on HD's layout, `~BLOWUP`, `~ROCKLOCK`, `0x1b`, `~AUTOPILOT` and the travel loops.~~ Done.
   Next on held cues: listen to them against RPCS3 (a Talon's Junction race with weapons on).
3. Read `0x04`'s targets and the ADSR mapping (`0x006323a0`) so the lists that carry them walk (the key-off,
   `0x0062fd98`, is read).
4. Regression gate for all of it: the loader report of `oag-game <hd iso> --race --dry-run` with `RUST_LOG=debug` (68
   `sfx:` lines, 46 of them "plays its timeline" on 2026-11-27), and the Pulse and Pure cue census tests unchanged.
