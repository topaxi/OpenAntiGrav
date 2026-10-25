# HD's start-of-race voice: `ready`, `HBEAT` and `go`

**Binary:** `ps3-hdfury-eu` `EBOOT.elf` (BCES-00664). Read statically and captured live on RPCS3
2026-10-19, lane `countdown-audio-2`. Every string and TOC slot below is resolved with
`scripts/ps3-toc.py` against the function's own TOC (`0x008ad4d8` for module A), never read off
Ghidra's decompile.

**Status:** the start plays **three cues**: `ready` (queued) and `HBEAT` (the bank-cue player) at the start
of the countdown, `go` (queued) when the race is released, and nothing else of the start. The triggers are
read in code and the order is captured. **The ready-to-go spacing is not measured** (see "The capture").

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00056368` | `RaceManager_SetPhase`, already named ([gantry-clock.md](gantry-clock.md)), its jump table now walked | 75 (name), 85 (the phase 1 reading below) |
| `0x00057690` | `RaceManager_UpdateCountdown`: the countdown's end, which releases the craft, enters phase 2 and queues `go` | 82 |
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

## `go`: queued right after phase 2 is entered

`RaceManager_UpdateCountdown` (`0x00057690`, `_opd_FUN_00057690` in Ghidra; `scripts/ps3-toc.py attrib` ties the
string `'go'` at `0x0077cce0` to it alone). When the countdown timer
(`*(this + 0x1958) - *(this + 0x1968)`) runs out it:

1. sets every live craft's state with `Craft_SetState(craft, 1)` (`0x000f0630`, the release, for each of the
   `*(0x008a67f8)` players whose `+0x628c` is `0` or `2`) and `0x000e1e20(craft, 1)`;
2. calls `RaceManager_SetPhase(this, 2)` at `0x00057970`;
3. **unless** the byte at `0x009384e1` is zero **and** `g_GameState.mode` is `2`, calls
   `Sound_QueueAnnouncerCue(*0x00b6e300, *0x00b6cc48, "go", 2)` at `0x000579a8`.

So `go` is voiced in the call that releases the race, as on Pulse ([the Pulse page](../psp-pulse-usa/countdown-voice.md)):
the port's `go` at the last gated tick is the same moment. Confidence 85 (disassembly, exact TOC). Phase 2
itself plays nothing, which is why a search of `RaceManager_SetPhase` alone never found it.

## The capture (RPCS3, 2026-10-19)

`scripts/hd-countdown-cues.py` boots HD on RPCS3's interpreter (the only decoder `Z0` breakpoints fire under),
walks to a Fury race on Talon's Junction, restarts it to the START RACE prompt, arms one breakpoint, presses
START RACE with cross held and logs every hit with the cue name (`r5`) and the SCREAM master tick
(`0x013bc500`, 240 a second of **audio** time).

| Entry armed | Hits, in order (ticks after the first) |
| --- | --- |
| `Sound_QueueAnnouncerCue` (`0x00310bd8`) | `ready` 0, `go` +737 (3.07 s), then in-race lines (`rockets`, `NEW_LAP_REC`, `mines`, `FINAL_LAP`) |
| `Sound_PlayBankCue` (`0x002ffa58`), which every queued line reaches | `ready` 0, `HBEAT` +8 (33 ms), `go` +981 (4.09 s), `c_AfterburnerIg` +1224, `SPEEDUPPAD`, `.COLLISIONS`, `.GRIND`... |

- **Between `ready` and `go` the only start cue is `HBEAT`**: no `321_GO`, `INIT_GO`, `LAUNCHSEQ` or `LAUNCHBEEP`
  starts as a cue, on either entry. `LAUNCHSEQ` (`launch.bnk`) is not part of the start.
- **The two `go` intervals differ (3.07 s and 4.09 s) because the audio clock is not game time**: the game
  thread is parked at each hit while the audio thread keeps counting, so the number orders hits and bounds
  nothing. The spacing is the countdown timer `+0x1958`, an authored value this page did not read. The port keeps
  Pulse's 180 ticks (3.0 s), **inherited, not measured**.
- A third run (`Sound_PlayNamedCue`, `0x0062c500`) hit a stub timeout and gave nothing.
- The race manager's phase was not read in the capture (the singleton slot did not resolve): the order comes
  from the entries, the phase from the code above.

## What the cues are

`speech.bnk` (HD EU), read in a one-off pass:

- `ready` is a lone `0x08` into the child `321_GO` ([sound.md](sound.md), `Scream_DoGrainBranch`, 85: it
  replaces the voice's own playback state with the child). `321_GO` is a goto to a marker at its tail, skipping
  three stages on delays 125, 205 and 205 ticks, and keys the 3.0 s stereo pair there.
- `HBEAT` (cue 27) is **three groups of voices on delays of 125, 205 and 205 ticks of the 240 Hz tick** (the
  stages `321_GO` skips, the same three delays): at 0.52 s, 1.375 s and 2.23 s after it starts, 3, 3 and 4
  voices at volume 127, ending on a `0x1a` random wait nothing follows. Played with `ready` they are the three
  beats of the count.
- `go` is a goto over its own four-voice stage to a tail pair (volume 127, note -62).

## What the port plays (HD)

| | Before | Now |
| --- | --- | --- |
| `ready` | the authored 3.0 s tail of `321_GO` at tick 91 (part 1) | unchanged |
| `HBEAT` | not played | the three groups at ticks 91 + 31, + 82 and + 134 (0.52, 1.375 and 2.23 s), `Cue::StartBeat` |
| `go` | tick 271 | unchanged |

`crates/raceplay/tests/countdown_voice_hd_ground_truth.rs` pins `HBEAT`'s ten voices and three onsets. The
WAV render of the start differs from before by exactly the beats (first difference at 2.04 s, tick 122);
Pulse's and Pure's renders are byte-identical.

## Still not found

- `INIT_GO` (`speech.bnk` cue 4, four key-ons) and `LAUNCHSEQ`/`LAUNCHBEEP2/3` (`launch.bnk`): not started by
  any cue of the start, and no code or data names them. Not played.
- The countdown's own length (`+0x1958`).
