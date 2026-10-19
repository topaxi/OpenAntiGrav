# Pure's start-of-race voice: `ready` and `go`

**Binary:** `pure-psp-usa` `BOOT.BIN`, image base `0x08804000`. Measured 2026-10-19 on
PPSSPP v1.20.4 (software renderer, muted), `pure-psp-usa.chd`, Time Trial on Modesto Heights,
Feisar. **Pressing measured: USA only.** The EU pressing is not run.

**Status:** measured. A Pure race start plays **two countdown cues**, `ready` then `go`,
180.0 frames apart, from the `SPEECH` bank (`speech.bnk`), and no other countdown voice. This is
Pulse's law ([countdown-voice.md](../psp-pulse-usa/countdown-voice.md)) on a title whose own data
now shows it, so `oag_pure::race::COUNTDOWN_VOICE` carries `Origin::Measured`.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0883defc` | `Scream_StartSound` | 88 |

## Finding `Scream_StartSound`

`Scream_PlaySoundByName` (`0x08835a90`, [shield-sound.md](shield-sound.md)) resolves a name and
calls a dispatcher; the dispatcher (`0x0883be6c` into `0x0883be9c`) pushes note `0x3c` and calls
`0x0883defc`. Its decompile is Pulse's `Scream_StartSound` line for line: the bounds test
`index < *(i16*)(bank + 0x16)`, the cue record at `*(bank + 0x1c) + index * 12`, the
`handler + 0x34 = 0x3c` default note, and the four-iteration modulator loop at `+0x5c`
(`Scream_StartSound`'s own, [sound.md](../psp-pulse-usa/sound.md)). It was then checked by running:
a breakpoint on it hits with `a0` an `SBlk` bank and `a1` an in-range cue index on every cue start
of a race (track ambience, `SHIP`, `GENTRAK`, `SPEECH`). Confidence 88: structural match plus the
runtime check, no second capture method.

## What is measured

`scripts/psp-pure-countdown-cues.py` (the Pure counterpart of `scripts/psp-countdown-cues.py`,
whose addresses are Pulse's) breaks on `0x0883defc`, logs the bank name and cue index at each
hit, and after `go` dumps the craft on every `Ship_UpdateEngine` (`0x0892f1f0`) hit. Cross held,
restart via the pause menu or RACE AGAIN.

| run | `ready` (frames from the first cue start) | `go` | `ready` to `go` |
| --- | --- | --- | --- |
| pause menu RESTART | 424.1 | 604.1 | 180.0 |
| RACE AGAIN | 98.1 | 278.1 | 180.0 |
| RACE AGAIN, script | 0.0 (first logged) | 180.0 | 180.0 |

- **Cues:** `SPEECH` cue 12 is `ready` (1.25 s) and cue 13 is `go` (1.12 s) in the disc's own bank
  (`just wad sounds`). `SPEECH` is the only bank that starts a voice between the first cue of a
  restart and the first update after `go`. The other starts in the log are track ambience
  (`MODESTO` cue 12 is `~CRAFT`, the passing craft), `SHIP` and `GENTRAK`, and they are not
  countdown cues.
- **Spacing:** 180.0 frames at 59.94 frames a second, in every run: 3.0 s, Pulse's 180 ticks.
- **`go` is at the release.** Pure's game update runs at 30 a second (two frames), so the
  resolution is 2 frames. The craft's launch gate (`craft+0x260` and `+0x264` reading `100.0`,
  `+0x270` reading `0.0`) flips (`0.0`, `0.0`, `100.0`) in the first `Ship_UpdateEngine` update
  after `go` starts, 2 to 4 frames later; `craft+0x290` starts a 100 to 0 ramp of about 60 frames
  from the same update. That is Pulse's "`go` voiced on the last gated tick", to the update.
  The three field names are not recovered, so the gate is read off values, not off code.
  Confidence 70 for the `go` anchor, 85 for the cues and the 180 frame spacing.
- **No beep between** `ready` and `go`: the log has no other cue start in the 180 frames.

## What this settles for the port

Pure keeps Pulse's ticks (`ready` at `World::tick` 91, `go` at 271) and `speech.bnk`, and the
Title data says so as `Origin::Measured` instead of lent. Render with
`cargo run -p oag-game --example countdown_census -- data/images/pure-psp-usa.chd out.wav`:
`ready` plays 1.24 s from tick 91.1 and `go` 1.12 s from tick 271.1, the bank's own lengths. (The
census table's earlier "0.51 s" and "0.43 s" for Pure were wrong.)

**Not measured on Pure:** Zone and Eliminator (the banks are the title's own names, unrun), the
EU pressing, and whether Pure's 30 Hz update shifts `go` by one update against the 60 Hz tick.

## Checked against the other titles

HD, 2048 and Omega keep Pulse's timing as `Origin::Chosen`: their start is read on their own pages
(`docs/formats/psp-audio.md`, "The countdown census, every title"). Pure's measurement does not
transfer to them (different engine builds, different banks). **Checked, differs.**
