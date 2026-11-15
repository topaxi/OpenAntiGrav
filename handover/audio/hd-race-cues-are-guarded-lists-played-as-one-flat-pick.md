---
status: ready-to-wire
titles: [hd]
updated: 2026-11-15
summary: "33 HD race cues are variable-guarded multi-voice lists played as one flat-picked waveform; wire an HD walk model, SPEEDUPPAD first"
size: L (55)
difficulty: medium (60)
---

# HD's race cues are guarded lists, played as one flat pick

2026-11-15, lane `hd-sound-audit` (a measure lane; nothing in `crates/` changed). The maintainer
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

## Open

Ranked by what a player hears (rows of the page's audit table):

1. **`SPEEDUPPAD`**: the original keys a body, then a whoosh behind-left and behind-right 10 ticks
   apart under one +/-2 semitone bend draw (the player's own pad, variables all zero); another
   craft's pad is the body plus one of the two whooshes (variable 2 = 2). Ours: one of the three
   waveforms at random. Ready to wire.
2. **`.COLLISIONS`**: variable 1 picks ship or wall, variable 0 the severity child (wall S/M/L,
   ship M/L/L: the small ship-to-ship set is unreachable), then 3 to 6 voices.
   Ours: 1 of 112. The variables are identified; **what the game computes the severity from is
   not** (`FUN_000d8bd0`, `0x000d8bd0`, unread). Wire the surface now, the severity when read.
3. **Explosion and hit cues** (`ROCKEXPL*`, `MISSILEEXP*`, `SHURIKENEXPL`, `QUAKEHIT`,
   `PLASMAHITSHIP`, `REPULSORHIT`, `SHURIKENHIT`, `CANNONEXPL*`): layered, 2 to 17 voices, children
   chosen by `0x19` and the random goto `0x25`. Ours: one layer. Ready to wire (no variable needed:
   a projectile's emitter sets none).
4. **Launch cues** (`ROCKET`, `MISSILE`, `CANNON`, `MINELAUNCH`, `TURBO`, `SHURIKEN`, `LEACHENERGY`,
   `ABSORB`, `REPULSOR`): stereo pairs and short sequences switched on variable 2. Ready to wire.
5. **Held cues** (`~SHIELD`, `~AUTOPILOT`, `~BLOWUP`, `~ROCKLOCK`, the travel loops,
   `~LEACHATTACH`, `~magstrip01`): lists that loop, re-key and wait. Ours: one waveform. Needs the
   repeating runner on HD's tick and layout (`layers::program` is PSP-only).
6. **The random bend is read as 0 % on every HD cue**, also the ones that already play as a
   timeline (`PLASMAHITWALL`, `QUAKELAUNCH`, `accept`, `reject`). One operand byte.
7. Not ready, each needs a read first: the cues the original starts and we have no cue for
   (`.COLLSHIELD`, `.GRINDSHIELD`, `.GRIND`, `WEAPONPICKUP`, `MINERADAR`, `~BOMBRADAR`,
   `BOMBLAUNCH`, `BOMBEXPL`, `MINEEXPL`, `~SHIP_ONFIRE`, `CANNON_VO`, `mines`, `FINAL_LAP`,
   `RACE_COMPLETE`: triggers unread), the `0x04` modulators, the ADSR words, the eight-float
   block, and which weapons bank Zone, Detonator and Eliminator load.

## Next Steps

1. **An HD walk in `oag_formats::sblk::timeline`** (`WalkModel`), switched on for
   `SequenceTick::Ps3` race cues only (2048 runs the Vita handlers; Pulse and Pure must not move):
   - operand bytes HD's way for `0x19` (count `a`, stride `b`: `hd_alternates` already does it,
     the front end is the only caller that asks), `0x1b` (percent `a`), `0x22` (variable `a`,
     mode `b`, threshold `c`);
   - `locals: [i8; 4]`, read by a guard whose variable is not negative, written by `0x1e` and
     `0x20`, inherited by a `0x05` child;
   - `0x25` as an alternate group over marker ids `a..=b` (2048's `0x2e` is the same shape),
     `0x1c` as a fixed bend, `0x18` as the end of the cue, `0x1b` after a key-on (the bend is
     the handler's, it reaches voices already keyed), `0x26` as a wait for the cue's own voices.
   Test that fails if the law is dropped (HD disc, add it to the pass that already walks
   `weapons.bnk`): `SPEEDUPPAD` with locals `[0,0,0,0]` is three grains at ticks 0, 0, 10 and
   angles 0, 220, 140 with one bend of 100 %; with `[0,0,2,0]` two grains, the second one of two;
   with `[0,0,3,0]` one. `PLASMAHITWALL`'s bend reads 100. `.COLLISIONS` with `[0,1,0,0]` reaches
   only `c_CShipWall`'s first block.
2. **`Banks` builds a cue's timelines per perspective** and `voices_for` picks: `[0,0,0,0]` for a
   cue the original plays unplaced from the player's seat (`SPEEDUPPAD`, `TURBO`: measured),
   `[_,_,1,_]` for one on the player's craft emitter (measured: `MINELAUNCH`, `QUAKELAUNCH`,
   `~SHIELD`, `LEACH`, the shield crackle; the cockpit view makes it 0), `[_,_,2,_]` for another
   craft. A cue whose path is not in the page's live table: run
   `scripts/rpcs3-hd-cue-handlers.py --sweep 0,1,2,3,4,5,6,7,8,9,10` on a weapons-on race state
   first, do not guess.
3. **The rear half of the pan law** (`oag_audio::spatial::pan_of_angle`): 9 cues carry a rear
   angle and 14 a voice at 90 degrees, and one such grain sends the whole cue back to the flat
   pick (`layers.rs:209`). A stereo fold is a choice, label it chosen; see
   [hd-2048-rear-occlusion.md](hd-2048-rear-occlusion.md) for the rear cue already wired.
4. **Held cues**: `oag_formats::sblk::runner` on HD's layout and tick with gotos and alternates,
   then `~BLOWUP` and `~ROCKLOCK` (variable 0 selects the lock tone's fixed bend) through it.
5. Regression gate for all of it: the loader report of `oag-game <hd iso> --race --dry-run` with
   `RUST_LOG=debug` (71 `sfx:` lines today, 17 of them "plays its timeline"), and the Pulse and
   Pure cue census tests unchanged.
6. Read `FUN_000d8bd0` for the collision severity thresholds, then wire `.COLLISIONS`' variable 0
   and add `.COLLSHIELD`, `.GRIND` and `.GRINDSHIELD` beside it.
