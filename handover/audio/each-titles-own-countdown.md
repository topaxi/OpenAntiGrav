---
status: measure
titles: [hd, 2048, pure, omega]
updated: 2026-11-02
summary: "2048 start cues corrected and playing; HD spacing inherited; Omega start events found (<cue>__Weapons_NGP), wiring in omega-wwise-sfx-path"
---

# Each title's own countdown: what is left

2026-10-19, lanes `race-countdown-audio` and `countdown-audio-2`. Census and evidence:
[psp-audio.md](../../docs/formats/psp-audio.md) ("The countdown census, every title"), the per-title pages
[Pure](../../docs/ghidra/functions/psp-pure-usa/countdown-voice.md),
[HD](../../docs/ghidra/functions/ps3-hdfury-eu/countdown-voice.md) and
[2048](../../docs/ghidra/functions/vita-2048-eu-v104/countdown-voice.md).

**Landed:** Pure's `ready`/`go` measured on PSP USA (`oag_title::CountdownVoice::origin`); HD plays `ready`,
`HBEAT` and `go` from located triggers (the start beat is new); 2048 plays `ready`, `HBEAT` and `go` in a named
mode and `321_NGP` and `Go_NGP` in an event; **2026-11-02 (`countdown-voice`)**: 2048's hashed name lookup returned the hash bucket, not the cue, so its
start played other cues (`ready` was `rockets`'s cue); corrected to the record's `+0xc` field, the way
`Scream_FindCueByName` does, and 2048's named start is now HD's shape (`ready` into `321_GO`'s 3.00 s pair, the
three-group `HBEAT`, `go` cue 3). Omega checked: the bare names are no event, but the executable posts
`<cue>[_PC]__<bank>`, and `_321_NGP__Weapons_NGP` / `Go_NGP__Weapons_NGP` are in `English(US)/Weapons_NGP.bnk`
(2026-11-02, `omega-wwise-sfx-re`, [wwise-sound.md](../../docs/ghidra/functions/ps4-omega-eu/wwise-sound.md)).
Ours still plays nothing there.

## Open

- **HD's ready-to-go spacing is inherited from Pulse (180 ticks), not measured.** The capture counted audio
  ticks, which run while the game thread is parked; the spacing is the countdown timer `RaceManager+0x1958`,
  unread. The `Sound_PlayNamedCue` run timed out, and the race manager's phase did not read (the singleton slot at
  `0x008a6814` did not resolve). `INIT_GO` and `launch.bnk`'s `LAUNCHSEQ` start no cue of the start and are
  unplayed.
- **2048's campaign start plays `321_NGP` and `Go_NGP`** (`event-countdown-2048`): the rear-edge pan
  (`pan_of_angle_to_rear_edge`, 90 degrees only, asked for by the countdown cues alone) was wired and
  `COUNTDOWN_VOICE.event` is `StartCue::Plays`. No LFO is needed (the earlier reading was `PLASMA`'s). Multiplayer
  events (CRC ids too) play the named pair here. A Vita3K cue-start log would measure the ready-to-go spacing for
  both starts.
- **Every other 2048 hashed lookup was on the bucket too** (the crossfade engine, the circuits' emitters, and the
  perfect-lap noise of 2026-10-05, probably): re-listen-free check by WAV render after the correction, and
  reconsider the "2048's own SFX stay name-table-only" scoping once each trigger is read
  ([2048-xfx.md](../../docs/formats/2048-xfx.md)).
- **Pure**: Zone and Eliminator banks and the EU pressing were not run; Pure's update runs at 30 Hz, so `go`
  against the release has 2 frame resolution.
- **Omega**: the start posts `_321_NGP__Weapons_NGP` (243270081) and `Go_NGP__Weapons_NGP` (744083536) on every
  non-demo mode; both resolve to embedded ATRAC9 media in `English(US)/Weapons_NGP.bnk` (13 and 6 candidates,
  with action delays 400/1,300/2,150 ms). Wiring needs a Wwise sfx path in `oag-sound`: see
  `handover/audio/omega-wwise-sfx-path.md`.
- Not heard by a human; checks are WAV renders and cue logs.

## Next Steps

1. Read `RaceManager+0x1958` (the countdown length) and where it is set; or rerun
   `scripts/hd-countdown-cues.py --which named` with the phase read fixed (1 hour).
2. Vita3K cue-start log on 2048 for both starts (half a day).
