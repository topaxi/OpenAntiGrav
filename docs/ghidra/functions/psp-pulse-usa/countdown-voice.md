# The start-of-race voice: `ready` and `go`, and what plays them

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`. Measured 2026-09-29
on PPSSPP v1.20.4, `pulse-psp-usa.chd`.

**Status:** the trigger is read and the ticks are measured. A race start plays
**two cues and nothing else**, `"ready"` and `"go"`, 180 ticks apart, by name,
through the dry (no emitter) play path. There is no beep, no per-digit cue and no
other sound started between them.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x088274b4` | `RaceMode_UpdateCountdown` | 80 |
| `0x0893a768` | `Sound_PlayNamedInSlot` | 75 |
| `0x08829e6c` | `RaceMode_UpdateIntro` | 65 |

`RaceMode_SetState` (`0x08827350`, already named,
[zone-mode.md](zone-mode.md#the-ten-second-step)) is the other half of the
trigger and gains a finding here, not a new name.

## What is measured

`scripts/psp-countdown-cues.py` breaks on `Scream_StartSound` (`0x0898f864`), the
one function every cue start passes through - `Scream_PlaySoundByName`, a
positional emitter and a command list's own `PlayChild` all reach it - and at
each hit, with the CPU stopped so the read is free, logs the PSP cycle counter,
the bank (`a0`) and the cue index (`a1`). Tick numbers are anchored on the same
axis as the rest of the countdown docs: the first `Ship_UpdateCraft` entry at
which the craft's `throttleState` reads non-zero is **272**
([race-modes](../../../gameplay/race-modes.md#the-countdown-is-measured)).
Cross held throughout, restart via the pause menu each time.

| run | `ready` | `go` | throttle first non-zero | bank |
| --- | --- | --- | --- | --- |
| Time Trial, Talon's Junction | 90.02 | 270.02 | 272.00 | `SPEECH` cues 11, 12 |
| Time Trial, again | 90.02 | 270.02 | 272.00 | `SPEECH` 11, 12 |
| single race, eight craft | 90.05 | 270.06 | 272.00 | `SPEECH` 11, 12 |
| Eliminator (coarser anchor, +-1) | 90.79 | 270.79 | - | `elim_vo` 19, 20 |

- **`ready` to `go` is 180.0 frames** in all four, and `RaceMode_SetState` is
  entered with state `1` and then state `2` exactly 180.0 frames apart (a
  breakpoint on it, Time Trial).
- **`go` lands between the `Ship_UpdateCraft` entries numbered 270 and 271**,
  and `throttleState` is written by the craft update in between, which is the
  first one the state change no longer gates. So `go` is voiced on the **last
  gated tick**, and that ordering is causal: the state change that lifts the gate
  is what plays it. In `World::tick` terms that is `COUNTDOWN_TICKS - 1` = 271,
  and `ready` is 91.
- **Between the two, nothing.** Every cue start from the first `InGame` tick to
  400 frames past the release, in Time Trial: the track's own ambience (`gentrak`,
  the circuit bank) and `SHIP` 0 in the first tick, `SPEECH` 11 at 90, `SPEECH` 12
  at 270, then ambience and `HUD` cues well after the release. The gantry's
  `3`, `2`, `1` at 132, 178 and 222 start no cue; the spoken digits are inside
  `ready`'s own timeline.
- **`ready` is a timeline.** In `speech.bnk` it is six waveforms on authored
  delays (3.73 s summed). The first `Scream_KeyOnVoice` after the cue start is
  22.5 ticks later (rel. 113.27 against the cue at 90.79) and the next speech-
  shaped one 72.7 ticks after it (163.53); this port's own render of the same
  timeline has its first audible frame at 23.3 ticks and its second run at 72.1
  (`crates/game/tests/countdown_voice_ground_truth.rs`). The other key-ons in the
  window (a per-voice log, `--bp 0x0899456c`) belong to the ambience and are not
  attributed here.

## The bank is the mode's speech bank

The names `"ready"` and `"go"` are looked up in the bank slot `DAT_08ac1dfc`
holds, and `World_LoadTrack` (`0x08883fa0`) sets that slot in one of three
branches:

| `g_game_mode` | branch | bank | `ready` | `go` |
| --- | --- | --- | --- | --- |
| 6 (Zone) | opens `ZONE_ENV`, `ship_zone`, then `speech_zone` | `speech_zone.bnk` (`zone_vo`) | cue 15, 2.48 s | cue 16, 0.80 s |
| 8 (Eliminator) | opens `ship`, `weapons`, then `speech_elim` | `speech_elim.bnk` (`elim_vo`) | cue 19, 3.00 s | cue 20, 0.80 s |
| anything else | opens `ship`, `weapons`, then `speech` | `speech.bnk` (`SPEECH`) | cue 11, 3.73 s | cue 12, 0.80 s |

Each branch first tries the localised template (`speech_%s.bnk`,
`speech_zone_%s.bnk`) and falls back to the plain name if that file is absent; the
plain ones are what the discs carry. **Confirmed live for the two non-Zone
branches**: the cue-start log names `SPEECH` on Time Trial and single race and
`elim_vo` on Eliminator. Zone is from the loader alone (confidence 75): it was
not captured live. A forced-Zone attempt (writing `g_game_mode = 6` at
`Race_CreateModeObject` on a normal circuit) hung on the loading screen and is
not evidence either way.

**This corrects `psp-audio.md`'s "Extra cues: -" for Pulse's `speech_zone.bnk`**:
it carries `ready` and `go` (`oag-wad sounds`, cues 15 and 16), as it does on
Pure and HD.

## The trigger

```c
/* RaceMode_SetState (0x08827350), state 1 */
FUN_089385b4(g_music_player_ptr, 0, 1);
Sound_PlayNamedInSlot(DAT_08ac1e10, DAT_08ac1dfc, "ready", 0x400, 0, 0, 0);
```

`RaceMode_SetState(mode, 1)` is called from `RaceMode_UpdateIntro`
(`0x08829e6c`) - the return address at the live hit was `0x0882a46c`, inside it -
in its substate 4, when the per-mode wait finishes and `g_game_mode < 0xe` (every
single-machine mode; `>= 0xe` is multiplayer and takes a different exit).
`RaceMode_UpdateIntro` is the nested five-substate machine
[zone-mode.md](zone-mode.md#zone_updatestates-own-five-states-and-state-0s-nested-countdown)
describes for Zone; all seven modes reach it, Zone through the thin wrapper at
`0x0882f31c`. 65: the substate roles are read at branch level, only the call that
matters here was watched live.

```c
/* RaceMode_UpdateCountdown (0x088274b4), the state-1 update */
if ((int)f(max(0, total(+0x7bc) - elapsed(+0x7c0))) == 0) {   /* f = FUN_0897e158, whole seconds left */
    Race_StartRacing(mode);
    RaceMode_SetState(mode, 2);            /* live ra 0x08827674 */
    Sound_PlayNamedInSlot(DAT_08ac1e10, DAT_08ac1dfc, "go", 0x400, 0, 0, 0);
    ...
}
```

`RaceMode_UpdateCountdown` is called from each mode's state-1 handler, with the
substate `+0x7cc` still 0 (`0x0882c5a8`, `0x0882cdfc`, `0x0882d55c`, `0x0882ddb0`,
`0x0882ea0c`, **`0x0882f338` - Zone's**, `0x08833548`). That Zone reaches the same
function is what puts the same `go` on it; confidence 70 for Zone's ticks, since
they are inferred from the shared code rather than captured. 80 for the function
itself: a decompile whose two calls are each confirmed by a live `RaceMode_SetState`
hit and a cue start 180 frames apart.

`Sound_PlayNamedInSlot` (`0x0893a768`) is
`(manager, slot, name, volume, pan, ...)`: it reads the bank pointer out of
`manager + slot * 4 + 0x124`, calls `FUN_0898dac0` and then
`Scream_PlaySoundByName(bank, 0, name, ...)`. It is the dry, no-emitter play -
the same path `Disengaging` takes through `FUN_0883e9b0` - and it has some forty callers
(the `Ship_*`, `Weapon_*`, HUD and menu voice lines among them). 75: one
decompile, its shape confirmed by its callers' arguments rather than a second
read of its body.

## Corrections

- `zone-mode.md`'s "plays a sound cue at the exact tick it crosses 40" in
  substate 1 is `FUN_0893a958(0.02f, manager)` at `+0x1a04 == 0x28`. **No cue
  starts there**: the log above has no cue start on that tick. It is a
  volume/fade call on the music player's side, and still unread.
- The gantry timeline's derived start (tick 92,
  [start-gantry.md](../../../rendering/start-gantry.md)) and `ready` (tick 90 on
  the same axis, 91 in `World::tick`) are one to two ticks apart. Both are the
  countdown state being entered, so they are probably the same event seen through
  a screenshot's presentation lag; not measured further.

## Not determined

- **Zone live.** The mode is not menu-reachable on a fresh profile (greyed in the
  race-type list) and the forced route hung.
- **Pure, HD, 2048.** Their `speech_zone.bnk` carries `ready` and `go`, and HD's
  a longer `321_GO`; nothing says when or from where those titles play them, so
  `RaceDefaults::countdown_voice` is `None` for all three and they play nothing.
- **The PS2 pressing** is the same title package and is lent the PSP's ticks.
- What `FUN_089385b4(g_music_player_ptr, 0, 1)` does beside the `ready` cue (it
  is in the same branch) - the race music's start, by its argument shape, unread.
