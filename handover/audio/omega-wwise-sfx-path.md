---
status: re-first
titles: [omega]
size: M (55)
difficulty: medium (50)
updated: 2026-11-19
summary: "Omega: 24 of 43 race cues play Wwise events (9 held; GAME_VR, TeamName read); 6 wait on switch groups, 6 on ATRAC9 decode, the engine"
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

- ~~**Switch group `2068003018`**~~ done (2026-11-19, lane `omega-silent-cues`): it is the state
  `GAME_VR`, set to `No` by `SoundManager_Construct` on a flat screen (confidence 85; wwise-cues.md, "The
  `GAME_VR` state"). `WwiseCues::states` holds it; `wwise_start` plays a switch container on a listed group
  and refuses any other. Plays now: PlasmaHitShip, MissileHitWall and the four travel cues (15 -> 21).
- ~~**`VoiceReg__TeamName`**~~ done (2026-11-19): `SetSwitch` (`0x0177fe40`) group 3 by the ship's team;
  AG Systems' event sets `Team_AG_Systems`. ShieldActive, Engaging, Disengaging play the player's team's
  line, their media loose `.wem` (`WwiseCues::media_dirs`); 21 -> 24.
- **Switch containers still block 6 posted cues**, by group (wwise-cues.md, "The groups still refused"):
  `GAME_MODE` (Cannon, CannonHitWall: no `Set_State__`/`Set_Switch__` post names it; find who sets it,
  or show nothing does and play its `None` case), `VoiceReg__HUDTargetingState` (the held LockOn,
  `SetSwitch` index 5: read its callers), and three groups whose names are not among the executable's
  strings (SpeedupPad `3202674372`, Turbo `3167274579`, ShurikenLaunch `814711502`).
- **Chosen in the team switch**: every emitter plays the player's team's case (one timeline per cue);
  a split-screen second person hears the first's team voice.
- **ATRAC9 decode now blocks 6 cues past `GAME_VR`** (PlasmaHitWall, RocketHitWall, RocketHitShip,
  QuakeHit, the death, and QuakeLaunch from before): `oag_music::wem::decode` rejects frame 0 of media
  `144136475`, `483130367`, `573010160`, `845662642`, `504708181`, `405228712`. One decoder fix may unlock
  five cues at once; a lane of its own (oag-music).
- ~~**Held loops (12 cues)**~~ 5 done (2026-11-12, lane `omega-wwise-held-loops`): property 58 is a sound's
  loop count and a continuous container loops its pick ([wwise.md](../../docs/formats/wwise.md#loops)); a
  held post stops with no fade (`FUN_0109c140(id, 0, 4)`, `StopPlayingID`'s shape). Shield, Autopilot,
  Magstrip, ShurikenTravel and LeachAttach (no loop authored: plays once, held) play through the existing
  held paths (`Banks::held_voices`), pinned by `omega_held_loops_ground_truth` (red on main, green after)
  and `a_race_plays_its_posted_cues_from_their_wwise_events`. Evidence: wwise-cues.md, "Held loops".
- ~~**Six held cues stop at switch group `2068003018`**~~ four travel cues play through `GAME_VR`; the death
  stops at the ATRAC9 decoder, the lock-on at `VoiceReg__HUDTargetingState`. Rocket's flat branch authors no
  loop, and Plasma's and Missile's loops are cut by a `Stop` in their own event (laid out as media): check
  against the original by ear-free means (a WAV of a held travel cue) before calling them right.
- **The engine** is `~Ship<n>_<band>` (`FUN_0130b8c0`: team to `Ship1`..`Ship5`, bands `LowAcc_%d`,
  `HighDec_G_%d`, ..., `CONTROLLERID_*` game parameters), not `xpress`/`xrelease`. Unread: how the band
  and `%d` are chosen, and the RTPC curves. A lane of its own.
- **Held-loop simplifications, chosen**: travel loops play the non-player event (the owner is not passed),
  the magstrip plays the non-player event from every craft, a continuous random container repeats its
  first draw, `smpl` loop points are not read (the whole media loops), the `0x0103` stop-on-repost and the
  `0x1303`/`0x1402`/`0x0803` game-parameter and pitch actions in the held events are reported, not played.
- **`QUAKELAUNCH`'s media `405228712`** fails `oag_music`'s ATRAC9 decode at frame 0 ("invalid reuse band params").
- **`NGP_LEACH__Weapons_NGP` and `NGP_LEACHENERGY_PC__Weapons_NGP` start no sound this loader reads**
  (a relay into another bank, a dangling target or state actions alone: unread). The player's LeachEnergy
  is silent rather than the non-player event.
- **Unread choices**: `.COLLISIONS` (`_COLLISIONS_1`/`_COLLSHIELD`/`_GRIND*`, `FUN_0130b100`), the missile's
  ship hit (no `MISSILEEXPSHIP` post), the speed pad's per-team `Pad_<team>` layer on `Ship_NGP`.
- **2048** shares the cue strings (7 of 8 probed in its executable) but plays `SBlk`; not wired. It has no
  `GAME_VR` or `Set_State` string: checked, differs.

## Next Steps

1. ~~Countdown~~ done (`the_start_plays_its_wwise_events_as_timelines`).
2. ~~Container rules~~ done; the missile launch loads (`omega_countdown_ground_truth::the_missile_launch_plays_one_child_of_each_random_layer`):
   a layer container of five layers, four random containers and a sound, so five media per post, not one.
   ~~Next: the per-cue table~~ done (above). ~~Held loops' loop flags~~ done (5 of 12). ~~Switch-container
   play, `GAME_VR` first~~ done (9 of 12 held), ~~`TeamName`~~ done (24 of 43). Next: the ATRAC9 frames
   (6 cues), `GAME_MODE`, `HUDTargetingState`, then the engine's band cues.
   Per-post draws (above) before a cue is audible as random.
3. Live check of the countdown post (an hour): run the wipeout_pc port past GO and look for no
   `Failed Posting Audio Event` line (patch `0x017caf30` prints one on an unknown event).
