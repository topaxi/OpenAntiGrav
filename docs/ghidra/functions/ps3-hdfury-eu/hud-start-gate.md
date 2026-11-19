# HD's whole-HUD gate at the start: Zone draws no HUD until GO

**Binary:** `ps3-hdfury-eu` `EBOOT.elf` (BCES-00664). Read statically and on RPCS3, 2026-11-19, lane
`zone-hud-gate`. Every function here is below `0x32d5e0`, where Ghidra's TOC (`0x008ad4d8`) is the right
one; each TOC slot quoted was resolved with `scripts/ps3-toc.py resolve <function> <disp>`.

**The law (confidence 90).** The HUD object `*0x0098767c` draws only while its byte `+0x115` is
non-zero. A Zone race clears that byte on the grid and sets it when the race is released, so a Zone race
shows **no HUD at all**, neither on the grid nor through the countdown. The whole Zone HUD appears with
the release. A single race never clears it, so its HUD is on screen through the countdown.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0098767c` (data) | `g_Hud`: the race's HUD object (vtable `0x862f30`, the one `Hud_LoadDefinition` stores) | 90 |
| `0x00084e08` | `Hud_PrepareDraw`: HUD vtable `+0x14`; queues the HUD's draw-list entries and returns `hud+0x115` | 72 |
| `0x00076708` | `SPZone_Update`: Zone race manager vtable `+0xc`, a jump table on the phase `+0x1970` | 82 |
| `0x00076548` | `SPZone_UpdateRacing`: Zone's phase-2 case; writes `g_Hud+0x115 = 1` every tick | 85 |
| `0x00091c90` | `Hud_HideRaceGroups`: clears bit `0x4` of every bound group parent | 80 |
| `0x0008fd70` | `Hud_ShowPositionGroups`: `PositionsParent`, `TotalTimeParent`, `ZoneParent` on or off | 85 |
| `0x00090160` | `Hud_ShowLapTimeGroups`: `LapTimesParent`, `LapBestParent`, `ScoreFrame` on or off | 85 |
| `0x0008fbf0` | `Hud_ShowStatusGroups`: `SpeedClassParent` .. `PickupBackground` on or off | 80 |
| `0x00090278` | `Hud_UpdateCountdownWidgets`: the HUD's own countdown nodes `hud+0x2a8/+0x2ac`, keyed on `*(hud+0x40)+0x68/+0x69` | 60 |
| `0x0007d8e0` | `DemoRaceManager_Construct` (`Demo_RaceManager.cpp`) | 85 |
| `0x0007d398` | `DemoRaceManager_Update`: clears `g_Hud+0x34 & ~6` every tick, then the phase switch | 80 |

## Zone's phase switch (static)

`SPZone_Update` (`0x00076708`, slot `+0xc` of the vtable `SPZone_RaceManager`'s constructors store) does
`cmplwi r0,4` on `+0x1970`, then jumps through the inline table at `0x0007679c`. The table resolves to
these cases:

| Phase | Case | What it does to the HUD |
| --- | --- | --- |
| 0 | `0x000767b0` | `RaceManager` phase-0 work (`0x0005e2d8`), then `stb 0, 0x115(*0x0098767c)`. TOC `-0x632c` resolves to slot `0x008a71ac` -> `0x0098767c` |
| 1 | `0x000768e8` | `RaceManager_UpdateCountdown` (`0x00057690`) only. Nothing writes `+0x115` |
| 2 | `0x000768d4` | `SPZone_UpdateRacing` (`0x00076548`): `stb 1, 0x115(*0x0098767c)` on both of its paths (`0x000765ac`, `0x00076620`), same slot |
| 3, 4 | `0x000768c0`, `0x000768ac` | finish and results; not read |

`Hud_LoadDefinition` initialises `+0x115` to 1 (`li r27,1` at `0x00097ddc`, `stb r27,0x115(r31)` at
`0x00097ea8`). The byte also gates the now-playing caption: `0x0005c8f8` writes the track name only while
`g_Hud+0x115` is set.

## Read live (RPCS3, 2026-11-19)

`scripts/rpcs3-hd-hud-gate.py` cold-boots the game, walks Racebox into a race, and samples through
`/proc/<pid>/mem` with no pause. It reads the manager `*0x0095ae78` (`+0x1970` phase, `+0x1974` sub-state,
`+0x1958`/`+0x1968`) and the whole 0x6d0-byte `g_Hud`. It photographs the frame on every change. Runs
were at about a fifth of real time, under load 32.

**Zone, Vineta K (`GetMode()==6`):**

| Phase / sub | `g_Hud+0x34` | `+0x115` | `+0x164` (s since GO) | Frame |
| --- | --- | --- | --- | --- |
| 0 / 1, 2 | `0x52000` (bit `0x4` clear, widgets unbound) | 0 | 0 | grid, blank board, no HUD |
| 0 / 4 | `0x5a006` (visible bits set, widgets bound) | 0 | 0 | grid, no HUD |
| 1 / 0 (countdown) | `0x5a006` | 0 | 0 | gantry shows the `3` glyph, **no HUD**: the pair's original frame |
| 2 / 0 (released) | `0x5e006` | **1** | 0.133 | gantry `GO` glyph, no HUD yet |
| 2 / 0 | `0x5e006` | 1 | 0.767 | **whole Zone HUD drawn**: list, header, gauge |
| 3 | `0x5e000` | 1 | | race over |

The HUD's own visible bit (`+0x34 & 4`) and every group parent's bit are already set at phase 0 sub 4,
long before the HUD appears, so neither is the gate.

The HUD first appears between two samples, 0.133 s and 0.767 s after the release. Sampling was too
coarse to tell a cut from a short fade. `+0x115` is a byte, not a ramp, and it flips at the release.

**Single race, Talon's Junction (`GetMode()==3`), with a poke.** Here `+0x115` reads 1 from the grid on,
and the HUD (lap, position, shield, timer, speed) is drawn through the countdown. Writing 0 to
`g_Hud+0x115` mid-countdown made the whole HUD vanish in the next frame. It stayed gone after GO, because
nothing in that mode writes the byte back. That is the causal check: the byte alone decides whether the
HUD draws.

Frames and samples were in the lane's scratch directory and are not kept. To reproduce, run the script
twice: `--out DIR` for the Zone timeline, then `--out DIR --poke --any-mode --rights 0` for the poke. The
Racebox remembers its race type between boots, so read `GetMode()` in the log before trusting the mode.

## Other modes (static, not wired)

Other functions store 0 to a HUD's `+0x115`. These are not mapped to modes:

- `0x000631f0` (calls `RaceManager_UpdateCountdown`, in `SPArcade_RaceManager.cpp`'s address range)
- `0x0006bda0`/`0x0006be50` (every player's HUD)
- `0x0003f138`/`0x0003f190` (multiplayer range)
- `0x00066c60`, `0x000759a8`/`0x00075a18`

`0x0003f6d8`, `0x0006d958` and `0x00067c00` (`Detonator_UpdateRace`) store 1. Which game mode owns each
function is not read. The live single race shows that at least one non-Zone mode keeps its HUD through
the countdown, so the port holds only Zone.

## Negatives on the way (do not re-read)

- `RaceManager_UpdateCountdown`'s bytes `+0x68`/`+0x69` belong to `*(g_Hud+0x40)`. Their one HUD reader
  is `Hud_UpdateCountdownWidgets`, which shows or fades the HUD's own countdown nodes. It does not gate
  the whole HUD.
- `Hud_Update`'s phase-0 branch calls `Hud_HideRaceGroups`, but the same tick falls through to the show
  path (`Hud_ShowPositionGroups(hud, 1)` and the others). The hide never survives a tick.
- `0x000cf970`, which `RaceManager_SetPhase(1)` calls per player, stops and frees four sound handles
  (`+0x5f00`..`+0x5f0c`). It does not reset the HUD. [countdown-voice.md](countdown-voice.md) is
  corrected.
- `0x0007d4a0` and `0x0007d8e0` both store the demo vtable `0x862de0`. They are the attract demo's
  constructors, not a race state's enter.
- Ghidra's data xrefs to `0x0098767c` from `Scene_PrepareFrame` and from the `0x00082xxx` score helpers
  are TOC misattributions. Trust direct `bl` xrefs and `ps3-toc.py resolve`.

## Omega and 2048

- **Omega: checked, applies, wired.** `SPZoneRaceManager_Construct` (`0x012b4960`) stores vtable
  `0x019148a8`. Its slot 3, `0x012b3400`, switches on the phase:
  - cases 0 and 1 write `*(DAT_019ffc90 + 0x189) = 0`;
  - case 2 writes 1.

  `+0x189` sits 3 bytes before `+0x18c`, the player index the same function compares (`HUD->`/`HUD1->`).
  That is how `+0x115` sits before `+0x118` on HD, so it is the same field at 8-byte-pointer offsets.
  Static only, two sites agree: 80. `oag_omega::race::DEFAULTS.hud_waits_for_go` is `["zone"]`, measured.
- **2048: checked, not read.** 2048 ships `SPZone_RaceManager.cpp` and `HUD1->`. Its Zone constructor
  `0x81186584` stores vtable `0x8151173c`. Slot 3 (`0x8118720e`, Thumb) has no Ghidra function, so the
  phase switch was not read. To read it: create a function at `0x8118720e`, then find the byte store of 1
  into the HUD global in its phase-2 case. Wired as inherited from Omega.

## What the port does

`oag_title::RaceDefaults::hud_waits_for_go` names the modes held until the release:

- HD and Omega: `["zone"]`, measured;
- 2048: `["zone"]`, inherited from Omega;
- Pulse and Pure: empty, chosen.

`Race::hud_shown` returns false while `oag_race::RaceState::thrust_gated(tick)` holds for such a mode.
The check is independent of the flyby, so a `--race --camera-pose` capture with no intro is gated too.

**One exemption.** A capture that imposes a HUD state (`--hud-state`, `CaptureHud`) depicts a frame with
that HUD on screen, so the gate leaves it alone.

- Every Omega and 2048 Zone pair is a mid-race frame posed at tick 0 with an imposed HUD state. Their
  renders do not move: `omega/zone-capital-reach-2048-z5` differs from main's render by 1 pixel,
  `2048-vita/zone-altima-z5` by 0.
- `omega/zone-vineta-k-z5` and `2048-vita/zone-dlc-vineta-k-z5` differ only by main's drift since their
  last render, not by this gate.
- The HD countdown pair must drop its `--hud-state` (`docs/tools/compare-page.md`, "HD Zone race-start pair").
Tests:

- `tests::viewpoint::a_mode_held_until_go_draws_no_hud_before_the_release` pins hidden at tick 0 and
  `COUNTDOWN_TICKS - 1`, shown at `COUNTDOWN_TICKS`, and shown at tick 0 with an imposed HUD state;
- `tests::viewpoint::hd_and_omega_hold_zone_and_only_zone` pins the title data.

The pair `hd/zone-vineta-k-z5` moved 30.02 -> 28.77 mean grey (RMSE 0.172 -> 0.165). The zone-list and
gauge regions moved 6.76 -> 5.49. What remains under them is scenery (the white buildings), not HUD.
