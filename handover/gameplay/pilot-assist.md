# Pilot Assist: what is left after the three levels

Pilot Assist is built title-agnostic in `oag_physics::pilot_assist` with 2048's three levels
(Off, Normal, Extreme) on every title: the disc's numbers where it authors them, 2048's as
labelled stand-ins ("chosen, not measured") where it does not. Switch: `[controls]
pilot_assist`, Normal by default on Android only. HD and 2048 draw the indicator (Extreme only).
Behaviour, the per-title table and the measured runs:
[`docs/physics/pilot-assist.md`](../../docs/physics/pilot-assist.md); evidence:
[`ps3-hdfury-eu/pilot-assist.md`](../../docs/ghidra/functions/ps3-hdfury-eu/pilot-assist.md),
[`vita-2048-eu-v104/pilot-assist.md`](../../docs/ghidra/functions/vita-2048-eu-v104/pilot-assist.md).

## Open

- **Pulse and Pure draw no indicator.** Their art waits on the asset-layer lane
  (`handover/tooling/an-own-asset-layer-behind-the-disc.md`: a fallback layer of our art, first
  use the Pilot Assist HUD on Pulse and Pure). Nothing is authored here; no art was made.
- **Normal's two undocumented details**: the multiplier at `(*(craft+0x8c))+0x10` is taken to
  be `notInUseStrength` and `craft+0x5a8` to be speed, neither tied to the code (confidence
  65). Normal is weak in the demo (HD 13 contacts against Off's 16); a trial with the multiplier
  at 1.0 did not change that, so the weakness is the `<Assist>` numbers. Reading
  `PilotAssist_UpdateNormal` against `PilotAssist_SetStrength`'s caller in
  `/2048/eboot-vita-2048-eu-v104.elf` would settle both. 1-2 hours.
- **The row's value labels** are raw tokens (`off`, `normal`, `extreme`): the menu has no
  per-title value strings, so 2048's own "Super" shows only in its `OptionsPilot` list.
- **A mid-race switch** (HD's pause-menu row) would have to become a replayed input; today the
  setting applies from the next race only.
- **Cost on mobile**: `oag_raceplay::pilot_assist::candidates` scans the spline twice per probe,
  four full scans a tick while the assist runs (Android's default is Normal, so most of a lap).
  One pass tracking the best sample and the best on another path halves it; a windowed search
  around the player's last index would remove most of it.
- **`pilot_assist_disable` volumes** (`PilotAssistDisable_Importer.cpp`, 2048 and Omega): who reads
  them, and the tip's "Extreme kicks in automatically in the trickiest parts".
- **HD's pause-menu row** (`Pilot Assist` between Continue and Game Options) is not in this
  build's pause menu; the CONTROLS row is the only switch.
- **Omega's HUD indicator** is not wired (`oag_omega::hud`'s `assist: None`); check whether its
  layouts embed `HUD_assist_indicator.xml`.
- **2048 live**: nothing of 2048's assist was run on Vita3K.

## Next Steps

1. Fold the two corridor scans into one pass, then measure the tick on the S24. 1 hour.
2. Read 2048's `PilotAssist_SetStrength` caller and the per-ship block offsets to confirm
   Normal's multiplier. 1-2 hours.
