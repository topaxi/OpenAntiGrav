# HD's start-of-race voice: where `ready` is played

**Binary:** `ps3-hdfury-eu` `EBOOT.elf` (BCES-00664). Read statically 2026-10-19, lane `countdown-audio-2`.
Every string and TOC slot below is resolved with `scripts/ps3-toc.py` against the function's own TOC
(`0x008ad4d8` for `0x00056368`, module A), never read off Ghidra's decompile.

**Status:** the `ready` trigger is located and its twin `HBEAT` with it; **`go` is not found**, and the timing
is not measured. See "Open" below.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00056368` | `RaceManager_SetPhase`, already named ([gantry-clock.md](gantry-clock.md)), its jump table now walked | 75 (name), 85 (the phase 1 reading below) |
| `0x00310bd8` | `Sound_QueueAnnouncerCue`, already named ([sound.md](sound.md)) | 85 |
| `0x002ffa58` | `Sound_PlayBankCue`, already named ([menu-sounds.md](menu-sounds.md)) | 75 |

## Phase 1 plays `ready` and `HBEAT`

`RaceManager_SetPhase(this, phase)` switches on `phase` (`cmplwi r4, 4`; `lwzx` through the relative table at
`0x000563c4`, five entries). The entries land on `0x000563d8`, `0x0005673c`, `0x000566bc`, `0x000565f0` and
`0x00056598` for phases 0 to 4. The earlier page said "phase 2 starts on the release" from a capture; the code
for the other half now reads:

- **Phase 1** (`0x0005673c`) is the countdown's start. It sets two flags, calls `0x003048c0(2)`, `0x000a4d18(1)`
  and `0x000a4d28(3)` (not read), then **unless** the byte at `0x009384e1` is zero **and** `g_GameState.mode`
  (`*(0x00936fe8 + 0xe0)`) is `2`, plays two cues:
  1. `Sound_QueueAnnouncerCue(*0x00b6e300, *0x00b6cc48, "ready", 2)` at `0x000567c0`: the announcer queue,
     priority `2`, the name at `0x0077cbe0` (`'ready'`).
  2. `Sound_PlayBankCue(*0x00b6cc28, "HBEAT", 0x400, ...)` at `0x000567f8`: the dry path, the name at
     `0x0077cbe8` (`'HBEAT'`, a heartbeat cue in the same speech banks, `HBEAT` is in `speech.bnk`,
     `speech_zone.bnk`, `speech_elim.bnk`, `speech_zbattle.bnk` and `speech_det.bnk`).
  It then resets the HUD (`0x000cf970` per player) and stores the phase (`+0x1970`).
- **Phase 2** (`0x000566bc`) calls `0x003048c0(8)` and `0x000a4d18(2)` and plays **no cue**: neither `go` nor
  `LAUNCHSEQ` is started here.
- The strings `ready` and `HBEAT` sit beside `RaceManager.cpp` in the string pool, and `scripts/ps3-toc.py attrib`
  attributes both to `0x00056368` alone.

This is **the same pair 2048 plays for a named mode** ([the 2048 page](../vita-2048-eu-v104/countdown-voice.md)):
`ready` plus `HBEAT`, gated on the mode not being `2`. Checked, applies: HD's named `ready` is a speech-bank cue
played through the announcer queue at the countdown's start, which is where the port raises `Cue::Ready`.

## `go`, `LAUNCHSEQ` and the rest: not found

- No literal `go`, `321_GO`, `INIT_GO`, `LAUNCHSEQ` or `LAUNCHBEEP` is in the executable, nor in any of the
  seven `DATA0n.PSARC` archives outside the sound banks themselves (a byte search of every entry), so no
  data file names them either.
- `RaceManager_Update`'s seven announcer calls are the lap lines (`FINAL_LAP`) and the end-race lines
  (`RACE_COMPLETE`, `SESS_COMPLETE`, `TOURN_COMPLETE`); none is `go`.
- The HUD's `ReadyGo` is a model (`Data\HUD\Pulse_Ready_Go.vex`, `hud_ready_go.xml`), a picture and no sound.
- So what plays `go`, and whether `ready`'s `321_GO` child ever runs its three stages (125, 205 and 205 ticks of
  the 240 Hz tick behind a goto) rather than its tail, is **unread**. The port keeps `go` at tick 271 and
  `ready`'s tail from part 1, **chosen, not measured**.
