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

- **HD's cockpit lean is its own and ours runs Pulse's on it.** `ship+0x6a44`/`+0x6a48`:
  stick times `0.015 * <Tilt internalAmount>`, `+/-0.5` from the sideshift timers
  `ship+0x6ac8`/`+0x6acc` (Pulse `+/-10`), follower `<Tilt internalSpeed>`, smooth `6.0`
  (Pulse `0.01`, `5.4`, `4.0`). Live: settles at `2.097` under a full stick
  (`0.99867 * 0.015 * 1.4`), so `oag_physics::controls::update_camera_lean` is wrong for HD by
  a factor of two. Same shape of port as `SteerLean`: a per-title gain set from `<Tilt>`.
- **HD's HUD sways with the steer.** `Hud_Update` (`0x0009e710`-`0x0009e754`) offsets two
  widgets' `+0x40`/`+0x44` by the drawn lean's follower `ship+0x6a58` and `ship+0x6a5c` (written
  only by the constructors). Not ported, not measured.
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
- **`oag-game --input-script` drops the stick axis on a race.** A script line
  `200 cross stick_x=0.7` left `steer` at `0` for 120 ticks (`--trace-out`), where `right`
  steered: the race capture path takes the script's buttons and not its analog axes, so any
  "same script on both sides" comparison that uses `stick_x` silently runs ours with the stick
  centred. Look at `oag_game::race_capture::tick` and `HeldButtons::advance`.
- **The stick shape.** Ours feeds the lean the unshaped stick; the original's record goes
  through a `0.2` deadzone and an S-curve (camera.md), so a partial stick leans less there.
  The maintainer's decision keeps our linear deadzone, so this is a known difference, not a bug.

## Next Steps

1. Port HD's cockpit lean (first item): a `CameraLean` gain set beside `SteerLean`, filled
   from `<Tilt internalAmount internalSpeed>`, smooth `6.0`; pin with the HD capture's
   `cam_lean` column (`scripts/rpcs3-hd-air-hold.py --script` logs it).
2. Measure the HUD sway on RPCS3 (stick held, read the two widgets' `+0x40`/`+0x44`).
