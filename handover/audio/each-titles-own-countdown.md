---
status: measure
titles: [hd, 2048, pure]
updated: 2026-10-19
summary: "HD's ready-to-go spacing is inherited, not measured (RaceManager+0x1958 unread); 2048 needs a Vita3K cue log"
---

# Each title's own countdown: what is left

2026-10-19, lanes `race-countdown-audio` and `countdown-audio-2`. Census and evidence:
[psp-audio.md](../../docs/formats/psp-audio.md) ("The countdown census, every title"), the per-title pages
[Pure](../../docs/ghidra/functions/psp-pure-usa/countdown-voice.md),
[HD](../../docs/ghidra/functions/ps3-hdfury-eu/countdown-voice.md) and
[2048](../../docs/ghidra/functions/vita-2048-eu-v104/countdown-voice.md).

**Landed:** Pure's `ready`/`go` measured on PSP USA (`oag_title::CountdownVoice::origin`); HD plays `ready`,
`HBEAT` and `go` from located triggers (the start beat is new); 2048 plays `ready`, `HBEAT` and `go` in a named
mode and `Go_NGP` in a campaign event; Omega checked (Wwise events, no `< 0x17` split).

## Open

- **HD's ready-to-go spacing is inherited from Pulse (180 ticks), not measured.** The capture counted audio
  ticks, which run while the game thread is parked; the spacing is the countdown timer `RaceManager+0x1958`,
  unread. The `Sound_PlayNamedCue` run timed out, and the race manager's phase did not read (the singleton slot at
  `0x008a6814` did not resolve). `INIT_GO` and `launch.bnk`'s `LAUNCHSEQ` start no cue of the start and are
  unplayed.
- **2048's `321_NGP`** (an event's `ready`) is authored and not played: two looping voices keyed off 3.75 s
  later under a +-180 degree pan sweep at 1 Hz. It needs a key-off schedule and a pan modulator in the mixer and
  a rear-half pan, and its start offset against `go` measured (it outlives a 3.0 s spacing by 1.4 s, so the
  spacing is probably not Pulse's there). A Vita3K cue-start log would settle the spacing for both of 2048's
  starts. Multiplayer events (CRC ids too) play the named pair here.
- **Pure**: Zone and Eliminator banks and the EU pressing were not run; Pure's update runs at 30 Hz, so `go`
  against the release has 2 frame resolution.
- **Omega**: `_321_NGP` and `Go_NGP` are Wwise events posted on every non-demo mode; nothing maps Wwise events to
  cues.
- Not heard by a human; checks are WAV renders and cue logs.

## Next Steps

1. Read `RaceManager+0x1958` (the countdown length) and where it is set; or rerun
   `scripts/hd-countdown-cues.py --which named` with the phase read fixed (1 hour).
2. Vita3K cue-start log on 2048 for both starts (half a day), then the mixer key-off and pan modulator for
   `321_NGP`.
