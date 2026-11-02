---
status: ready-to-wire
titles: [omega]
size: M (55)
difficulty: medium (50)
updated: 2026-11-02
summary: "Omega race sfx need a Wwise path in oag-sound; read: bank list, <cue>[_PC]__<bank> law, relays; unread: container rules, positioning"
---

# Omega race sound effects through Wwise

2026-11-02, lane `omega-wwise-sfx-re` (measure only). Evidence:
[wwise-sound.md](../../docs/ghidra/functions/ps4-omega-eu/wwise-sound.md) (executable) and
[wwise.md](../../docs/formats/wwise.md) ("Event names and the stub banks"). Ground truth:
`crates/formats/tests/wwise_ground_truth.rs`, `posted_events_resolve`.

Omega races play the song and nothing else: every cue goes through `oag_sound::sfx`, which reads `SBlk` banks by
name, and Omega's banks are Wwise. The read side now names what a Wwise sfx path needs.

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

- **Container rules.** Random/sequence, switch and layer containers decide which of a target's children sound:
  the countdown reaches 13 media candidates (VO "three", "two", "one", UI layers, ship layers, the legacy
  `Weapons_NGP__321_stereo` and `beat1`) and the library lists all of them. Playing all at once is wrong for a
  random container. `oag_formats::wwise::hirc` reads no playlist or switch key for sound containers.
- **Action kinds** `0x1e03` (two in the countdown, on sounds, delays 2,000 and 500), `0x0102` (on an
  actor-mixer), `0x1204` (`SetState`, in `Go_NGP`): bodies unread.
- **Positioning.** The countdown plays on the speech manager's group, unpositioned in the code read. Each cue
  request carries 23999.0, 0.0005 and 0.4 at `+0x0c`, `+0x10`, `+0x1c` (23999.0 matches `AudioConfig.xml`'s
  `DistanceFilter near_cutoff`, a hypothesis); each sound's 3D and bus parameters are in the unread part of the
  node's base parameters. Pan the countdown as 2D until read.
- **The patch's countdown group** (race manager `+0x1aff0`) has an unread player byte; harmless, the `_PC` twin
  relays to the same event.
- **1,450 dangling `Play` targets** ([wwise.md](../../docs/formats/wwise.md#where-the-references-go)) are not
  in the stub banks; where they went is unread. None is on the countdown's or the missile's path.

## Next Steps

1. Countdown first (a day): banks 1, law 2, relays 3, delays 4, with `Placement` 2D. Acceptance: a WAV render of
   an Omega start that holds VO "three, two, one" at the action delays, and a test that fails if the suffix law
   or the relay follow is dropped.
2. Container rules for random/sequence and switch in `oag_formats::wwise::hirc` (half a day to a day), then the
   missile launch.
3. Live check of the countdown post (an hour): run the wipeout_pc port past GO and look for no
   `Failed Posting Audio Event` line (patch `0x017caf30` prints one on an unknown event).
