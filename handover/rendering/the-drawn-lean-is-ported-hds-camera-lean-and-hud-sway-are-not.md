# The drawn lean is ported; HD's cockpit lean and HUD sway are not

Since 2026-10-21 (`ship-lean`) every title's hull leans into a steer: Pulse's
`Ship_UpdateSteerLeans` drawn lean (`entity+0x854`, confidence 95) and HD's twin
(`ship+0x6a54`, `<Tilt shipAmount shipSpeed>` with a `10.0` smooth, confidence 90), composed
into the barrel roll's own angle, `lean * 0.5 + roll * 6.28`. Evidence:
[camera.md, "The drawn lean"](../../docs/ghidra/functions/psp-pulse-usa/camera.md#the-drawn-lean-entity0x854-read-and-measured-2026-10-21),
[ps3-hdfury-eu/ship-lean.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md),
[psp-pure-usa/craft-physics.md](../../docs/ghidra/functions/psp-pure-usa/craft-physics.md).
Per title: Pulse measured, Pure checked (same literals, static), HD measured, Omega and 2048
inherited from HD (both parse `<Tilt>`; consumer not located).

## Open

- ~~**HD's cockpit lean is its own and ours runs Pulse's on it.**~~ Ported 2026-10-21
  (`hd-cockpit-lean`): `oag_physics::CameraLean`, pinned by an RPCS3 capture; HD's step bound is
  one-sided (a centred stick lets go unbounded). The `+/-0.5` sideshift terms are read, not
  measured (the capture's taps never raised the timers):
  [ship-lean.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md).
- **HD's HUD sways with the steer, law not closed.** `Hud_Update` slides the "HUD" and "InGame"
  containers by `(-15 * ship+0x6a58, 15 * ship+0x6a5c)`. `ship+0x6a5c` has no `stfs` writer and is
  live-dynamic; the two nodes were not found in RAM. Need the `+0x6a5c` writer (vector store,
  `stfsx` or a block copy) and the node's on-screen shift.
- **Pulse's first-filter `+/-10.0` terms** (`entity+0x8a4`/`+0x8a8`) are the offsets
  `psp_trace_fields.py` records as `ss_shift_l`/`ss_shift_r`, so likely the sideshift kicking
  the cockpit lean; HD's twin reads its sideshift timers at the same place. Not captured.
- **Omega's and 2048's consumers** of `<Tilt shipAmount/shipSpeed>` were not located (Omega's
  parser is `FUN_012d8e30`; its `-0.005` literals' readers are other code; the compiler may have
  folded `-l * 0.005 + r * 0.005`). Both inherit HD's law.
- **`& 2` on Pulse's `entity+0x860`** negates the stick term of both filters; no writer was
  identified and it read clear in the capture. Treated as clear (chosen).
- ~~**HD's lean sign is inherited, not measured.**~~ Measured 2026-10-21: HD's node matrix
  under a held stick matches Pulse's form, pinned by
  `hds_lean_turns_the_hull_the_way_its_captured_display_matrix_does`
  ([ship-lean.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md)).
- ~~**`oag-game --input-script` drops the stick axis on a race.**~~ Fixed 2026-10-21:
  `HeldButtons::advance` carries the script's analog axes into the snapshot.
- **The stick shape.** Ours feeds the lean the unshaped stick; the original's record goes
  through a `0.2` deadzone and an S-curve (camera.md), so a partial stick leans less there.
  The maintainer's decision keeps our linear deadzone, so this is a known difference, not a bug.

## Next Steps

1. Find the writer of HD's `ship+0x6a5c` (not an `stfs`) and the "HUD"/"InGame" node in RAM; then
   port the sway into `oag-hud` as per-title data, constant `15.0`.
2. Take a sideshift capture on HD that raises `ship+0x6ac8`/`+0x6acc` (a single tap of the
   sideshift button on the scheme the pad script drives) and fit the `0.5` term.
3. Check whether Pulse's `FUN_0883fab4` has the same one-sided bound (its capture says the `0.3`
   return holds).
