---
status: re-first
titles: [omega]
size: M (55)
difficulty: medium (50)
updated: 2026-11-12
summary: "Omega start and 10 of 43 race cues play their Wwise events; 14 posted cues wait on switch-container play, 12 held loops on loop flags"
---

# Omega race sound effects through Wwise

2026-11-02, lane `omega-wwise-sfx-re` (measure only). Evidence:
[wwise-sound.md](../../docs/ghidra/functions/ps4-omega-eu/wwise-sound.md) (executable) and
[wwise.md](../../docs/formats/wwise.md) ("Event names and the stub banks"). Ground truth:
`crates/formats/tests/wwise_ground_truth.rs`, `posted_events_resolve`.

**Landed (2026-11-02, same lane, at the maintainer's request):** the start. `oag_sound::sfx` reads a `BKHD`
bank's event into a timeline (`crates/sound/src/sfx/wwise_start.rs`; steps 1-4 below for one bank), and Omega's
`COUNTDOWN_VOICE` names `_321_NGP__Weapons_NGP` and `Go_NGP__Weapons_NGP` on every mode. Every other Omega race
cue still goes through the `SBlk` loader and is silent. The start is audible: `countdown_voice_ground_truth::omega_time_trial_counts_in_from_wwise_events` renders it speech-only and asserts energy on each authored onset (400, 1,300, 2,150 ms after Ready, and Go), silence before Ready. The disc tests before it checked the events' names and timings, not sound.

## What a wiring needs, in order

1. **Banks per race.** Load from `Data/audio/sound/` (base) or `data/audio/sound/` (patch), trying
   `English(US)/<name>.bnk` first and then `<name>.bnk` (what the executable's IO does, watched live):
   `frontend`, `Speech_NGP`, `generaltrack`, `speech_fe`, `Ship_NGP`, `Weapons_NGP`, `crowd`, `Speech_NGP_Grid`,
   plus the track's `TrackStartup.xml` `<LoadSoundBank Filename=...>`, replaced by `env0_det` in Detonator and by
   `env0_zone` in Zone and Nitro Battle (single and multiplayer). Build one `oag_formats::wwise::Library` over
   them; events in one bank play media embedded in another (140 such sounds) or a loose `<id>.wem`.
2. **Cue to event.** `name_hash(cue + ["_PC" if the emitter is the player's] + "__" + bank)`, with a leading `~`
   turned into `_`. The bank is the slot's name: `Weapons_NGP` for weapons and the countdown, `Ship_NGP` for the
   ship, `Speech_NGP` for speech. Legacy 8-character bank names in data (`track.final.audio`, a node's
   `bank~cue`) go through the alias table on the evidence page first.
3. **Follow relays.** Action kind `0x2103` plays its target event; every `_PC` event and the weapon events
   (`NGP_MissileFire__Weapons_NGP` -> `WPN_MISSILE_Fire`) are relays, and `Library::resolve_event` follows only
   `Play`. Extend the library (the ground-truth check's `reached_media` is the shape).
4. **Delays.** A `Play` action's property 15 is a delay in ms (the countdown's six sounds at 400, 1,300, 2,150);
   property 16 a fade on the `0x0102` action (3,100 ms, fade 50). Fitted, confidence 70; read before trusting.
5. **Decode.** Embedded and loose media are ATRAC9 `.wem`, mono and stereo decode in process
   (`oag_music::wem::decode`).

## Open (read before wiring a cue beyond the countdown)

- ~~**Container rules.**~~ Done: `oag_formats::wwise::container` reads random/sequence and switch
  containers (wwise.md, "Sound containers"); `wwise_start::load` draws one child of each random container
  from a passed-in `Rng`. Still open: **switch containers are refused** at play (their group value is the
  game's), one path-positioned `Ship_NGP` ranseq is refused by name, and the pick is made at load, not per
  post (`Banks::load_wwise_event` picks once; per-post alternatives would be one `Timeline` per draw, which
  `Banks::voices` already draws among with its never-repeat rule).
- **Action kinds** `0x1e03` (two in the countdown, on sounds, delays 2,000 and 500), `0x0102` (on an
  actor-mixer), `0x1204` (`SetState`, in `Go_NGP`): bodies unread.
- **Positioning.** The countdown plays on the speech manager's group, unpositioned in the code read. Each cue
  request carries 23999.0, 0.0005 and 0.4 at `+0x0c`, `+0x10`, `+0x1c` (23999.0 matches `AudioConfig.xml`'s
  `DistanceFilter near_cutoff`, a hypothesis); each sound's 3D and bus parameters are in the unread part of the
  node's base parameters. Pan the countdown as 2D until read.
- **Pass-bys and one NPC engine layer pick by the built-in Azimuth and Distance** (a switch or an RTPC; the engine computes them from the
  listener): [audio-3d.md](../../docs/ghidra/functions/ps4-omega-eu/audio-3d.md), "Front and back are authored". Their playback needs these.
- **The patch's countdown group** (race manager `+0x1aff0`) has an unread player byte; harmless, the `_PC` twin
  relays to the same event.
- **1,450 dangling `Play` targets** ([wwise.md](../../docs/formats/wwise.md#where-the-references-go)) are not
  in the stub banks; where they went is unread. None is on the countdown's or the missile's path.

## Done: the per-cue table (2026-11-12, lane `omega-wwise-cue-table`)

[wwise-cues.md](../../docs/ghidra/functions/ps4-omega-eu/wwise-cues.md) reads the cue string and slot the
1.07 executable posts for each of this port's 43 race cues (26 posted) and the patch's slot globals.
`oag_omega::race::WWISE_CUES` carries the table (`oag_title::WwiseCues` on `SoundBanks::wwise`);
`Banks::load_wwise_cues` loads `<cue>__<slot>` and `<cue>_PC__<slot>` (the player's, played from slot 0
through `Banks::voices_for`). **10 of 43 play** for every emitter (Message, Absorb, MineLaunch, Plasma,
Rocket, Missile, CannonHitShip, LeachFail, LeachEnergy, Repulsor) plus `NGP_LEACH` for the player alone.
Pinned by `omega_countdown_ground_truth::a_race_plays_its_posted_cues_from_their_wwise_events`.

## Open after the table

- **Switch containers block 14 posted cues** (SpeedupPad, Turbo, ShieldActive, Engaging, Disengaging, both
  Plasma hits, both Rocket hits, MissileHitWall, Cannon, CannonHitWall, QuakeHit, ShurikenLaunch). Reading
  each switch group's game-side value (ship class, surface, player) is the next unlock; the loader report
  names each target id.
- **Held loops (12 cues)**: Omega's `_` cues are kept handles; `wwise_start` reads no loop flags. Read the
  sound's loop property, then wire them through the held-voice paths (`travel`, `magstrip`, shield).
- **`QUAKELAUNCH`'s media `405228712`** fails `oag_music`'s ATRAC9 decode at frame 0 ("invalid reuse band params").
- **`NGP_LEACH__Weapons_NGP` and `NGP_LEACHENERGY_PC__Weapons_NGP` start no sound this loader reads**
  (a relay into another bank, a dangling target or state actions alone: unread). The player's LeachEnergy
  is silent rather than the non-player event.
- **Unread choices**: `.COLLISIONS` (`_COLLISIONS_1`/`_COLLSHIELD`/`_GRIND*`, `FUN_0130b100`), the missile's
  ship hit (no `MISSILEEXPSHIP` post), the speed pad's per-team `Pad_<team>` layer on `Ship_NGP`.
- **2048** shares the cue strings (7 of 8 probed in its executable) but plays `SBlk`; not wired.

## Next Steps

1. ~~Countdown~~ done (`the_start_plays_its_wwise_events_as_timelines`).
2. ~~Container rules~~ done; the missile launch loads (`omega_countdown_ground_truth::the_missile_launch_plays_one_child_of_each_random_layer`):
   a layer container of five layers, four random containers and a sound, so five media per post, not one.
   ~~Next: the per-cue table~~ done (above). Next: switch-container play, then the held loops' loop flags.
   Per-post draws (above) before a cue is audible as random.
3. Live check of the countdown post (an hour): run the wipeout_pc port past GO and look for no
   `Failed Posting Audio Event` line (patch `0x017caf30` prints one on an unknown event).
