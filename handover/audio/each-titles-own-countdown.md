---
status: measure
titles: [hd, 2048, pure]
updated: 2026-11-02
summary: "2048 start cues corrected (name table +0xc is the cue, not the bucket); event pair silent until the rear-edge pan; HD spacing inherited"
---

# Each title's own countdown: what is left

2026-10-19, lanes `race-countdown-audio` and `countdown-audio-2`. Census and evidence:
[psp-audio.md](../../docs/formats/psp-audio.md) ("The countdown census, every title"), the per-title pages
[Pure](../../docs/ghidra/functions/psp-pure-usa/countdown-voice.md),
[HD](../../docs/ghidra/functions/ps3-hdfury-eu/countdown-voice.md) and
[2048](../../docs/ghidra/functions/vita-2048-eu-v104/countdown-voice.md).

**Landed:** Pure's `ready`/`go` measured on PSP USA (`oag_title::CountdownVoice::origin`); HD plays `ready`,
`HBEAT` and `go` from located triggers (the start beat is new); 2048 plays `ready`, `HBEAT` and `go` in a named
mode; **2026-11-02 (`countdown-voice`)**: 2048's hashed name lookup returned the hash bucket, not the cue, so its
start played other cues (`ready` was `rockets`'s cue); corrected to the record's `+0xc` field, the way
`Scream_FindCueByName` does, and 2048's named start is now HD's shape (`ready` into `321_GO`'s 3.00 s pair, the
three-group `HBEAT`, `go` cue 3). Omega checked: its events exist in no shipped bank, its start stays silent
(the lent `Go` cue only times the song).

## Open

- **HD's ready-to-go spacing is inherited from Pulse (180 ticks), not measured.** The capture counted audio
  ticks, which run while the game thread is parked; the spacing is the countdown timer `RaceManager+0x1958`,
  unread. The `Sound_PlayNamedCue` run timed out, and the race manager's phase did not read (the singleton slot at
  `0x008a6814` did not resolve). `INIT_GO` and `launch.bnk`'s `LAUNCHSEQ` start no cue of the start and are
  unplayed.
- **2048's campaign start is silent now (ready to wire).** `321_NGP` (`Weapons_NGP.bnk` cue 70) and `Go_NGP`
  (cue 68) walk completely but pan a pair to 90 and 270 degrees, and `pan_of_angle(90)` is `None` (the rear edge,
  `a' = 180`, of `Scream_PanVolumePair`'s second branch, which also negates a channel). Wire the `a' = 180` edge
  (hard right, the lone voice's sign flip is inaudible) scoped to the start cues, or model the rear half, then
  switch `oag_2048::race::COUNTDOWN_VOICE.event` to `StartCue::Plays` (1 to 2 hours; confidence 70 that it is
  that small). Before the correction the event played `Go_NGP`'s bucket cue, a wrong sound. Multiplayer events
  (CRC ids too) play the named pair here. A Vita3K cue-start log would measure the ready-to-go spacing for both starts.
- **Every other 2048 hashed lookup was on the bucket too** (the crossfade engine, the circuits' emitters, and the
  perfect-lap noise of 2026-10-05, probably): re-listen-free check by WAV render after the correction, and
  reconsider the "2048's own SFX stay name-table-only" scoping once each trigger is read
  ([2048-xfx.md](../../docs/formats/2048-xfx.md)).
- **Pure**: Zone and Eliminator banks and the EU pressing were not run; Pure's update runs at 30 Hz, so `go`
  against the release has 2 frame resolution.
- **Omega**: `_321_NGP` and `Go_NGP` are Wwise events posted on every non-demo mode, and no shipped bank defines
  them (`omega_countdown_ground_truth`); nothing to play until a bank that does ships or a mapping is chosen.
- Not heard by a human; checks are WAV renders and cue logs.

## Next Steps

1. Read `RaceManager+0x1958` (the countdown length) and where it is set; or rerun
   `scripts/hd-countdown-cues.py --which named` with the phase read fixed (1 hour).
2. Vita3K cue-start log on 2048 for both starts (half a day); the rear-edge pan for the event pair (above).
