# Pilot Assist: what is left after HD's law

HD's Pilot Assist is recovered (read live on RPCS3), built title-agnostic in
`oag_physics::pilot_assist`, fed from each title's own `<PilotAssist>` table, switched by
`[controls] pilot_assist` (on by default on Android only) and drawn on HD's HUD. HD, 2048
(Extreme's law) and Omega race with it. Behaviour and numbers:
[`docs/physics/pilot-assist.md`](../../docs/physics/pilot-assist.md); evidence:
[`ps3-hdfury-eu/pilot-assist.md`](../../docs/ghidra/functions/ps3-hdfury-eu/pilot-assist.md),
[`vita-2048-eu-v104/pilot-assist.md`](../../docs/ghidra/functions/vita-2048-eu-v104/pilot-assist.md).

## Open

- **Pulse and Pure** author no table and no code. The maintainer asked for the assist on every
  title; which law and numbers they inherit (HD's, as chosen values?) waits on a ruling. Until
  then the CONTROLS row is shown on every title and does nothing on these two.
- **2048's Normal** (its default): the same law on the ship's `<Assist>` block, strength ramped
  by `<SteerAssist>`. Not built. Which `<Assist>` attribute sits at which block offset
  (`+0x84`..`+0xa0`, and `(*(craft+0x8c))+0x10`) needs the per-ship reader.
- **2048's `OptionsPilot` list** (Off/Normal/Super) is drawn and not tied to the switch.
- **`pilot_assist_disable` volumes** (`PilotAssistDisable_Importer.cpp`, 2048 and Omega): who reads
  them, and the tip's "Extreme kicks in automatically in the trickiest parts".
- **HD's pause-menu row** (`Pilot Assist` between Continue and Game Options) is not in this
  build's pause menu; the CONTROLS row is the only switch.
- **Omega's HUD indicator** is not wired (`oag_omega::hud`'s `assist: None`); check whether its
  layouts embed `HUD_assist_indicator.xml`.
- **2048 live**: nothing of 2048's assist was run on Vita3K.

## Next Steps

1. Get the ruling on Pulse/Pure, then either hide the row on titles with no table or give them
   the chosen law. 30 minutes either way.
2. Tie 2048's `OptionsPilot` list to the switch: Off -> off, Super -> on. Normal waits on step 3.
   1 hour with pointer support.
3. Read 2048's per-ship `<Assist>` reader (search `notInUseStrength`'s xrefs in
   `/2048/eboot-vita-2048-eu-v104.elf`) and build Normal as a second `Params` source. 2-3 hours.
