---
status: re-first
titles: [hd]
updated: 2026-10-21
summary: "HD HUD y sway needs the body+0x1b0 and ship+0x6b00 writers read; sideshift terms read not measured"
---

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

**Direction corrected 2026-10-21 (`lean-sign-fix`):** the lean and the barrel roll drew
mirrored (right stick raised the right wingtip) because the captured row 0 is the hull's
left and the sign test mapped it onto our right. `ROLL_DIRECTION` is `+1.0`; a right stick banks
right on every title. Open: a PPSSPP/RPCS3 photograph of the hull under a held stick (the fix rests
on the recorded rows and `Basis::LeftUpForward`, not on a new capture).

## Open

- ~~**HD's cockpit lean is its own and ours runs Pulse's on it.**~~ Ported 2026-10-21
  (`hd-cockpit-lean`): `oag_physics::CameraLean`, pinned by an RPCS3 capture; HD's step bound is
  one-sided (a centred stick lets go unbounded). The `+/-0.5` sideshift terms are read, not
  measured (the capture's taps never raised the timers):
  [ship-lean.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md).
- ~~**HD's HUD sways with the steer.**~~ x ported 2026-10-21 (`hd-hud-sway`): `Hud_Update` slides
  the `"HUD"` screen, which every race layout is, by `-15 * ship+0x6a58` (RPCS3: 13.3 px at a full
  stick, 1280x720, every region alike), `HudArt::sway` (HD measured, Omega and 2048 inherited). The
  `+0x6a5c` writer is found (`Ship_UpdateCameraRigs`, the second rig's spring lag along the craft's
  up axis) and the y term is **drawn as zero**: [ship-lean.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md).
  Still open: **`body+0x1b0`** (its writer and meaning; it feeds the anchor and is what makes the
  y bumps, up to +/-20 px) and `ship+0x6b00`, then the `"InGame"` screens (pause menu), which are
  not drawn through `oag-hud`. ~~**The whole-HUD scale** (`0.966`)~~ Ported 2026-10-21
  (`hd-hud-leftovers`): it is HD's `Safe Area Setting` (the saves hold 96), `HudArt::safe_area`,
  `[graphics] hud_safe_area`, default 90 (the disc's), pairs pin 96
  ([hd-hud.md](../../docs/formats/hd-hud.md#the-whole-hud-is-drawn-at-the-safe-area-setting-hd-hud-leftovers-2026-10-21)).
  Still open there: the setting's consumer is unread (poking 90 into the global did not move the HUD).
  The y term was looked at again and does not fit a wiring lane: it needs the `body+0x1b0` writer
  and a port of the second rig's spring; the original's lap hexagon sits about 15 px (display) higher
  than ours on `hd-race/talons-pack`, which is that term.
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

1. Read the writer of `body+0x1b0` and the `ship+0x6b00` vector, then port the y term of the sway
   (`15 * ship+0x6a5c`, the far rig's up-axis lag) beside the x term in `oag_hud::sway`.
2. Take a sideshift capture on HD that raises `ship+0x6ac8`/`+0x6acc` (a single tap of the
   sideshift button on the scheme the pad script drives) and fit the `0.5` term.
3. Check whether Pulse's `FUN_0883fab4` has the same one-sided bound (its capture says the `0.3`
   return holds).
4. **Pose carry (2026-10-21, `pose-lean-boost`):** Pulse PSP pairs now carry the drawn lean (`--pose-lean` imports the pose row's `steer_lean` and `roll_phase`; `talons-boost` craft crop RMSE 0.292 to 0.179 with the boost, 0.292 to 0.276 for the lean alone). **HD is open**: `rpcs3-hd-pair.py` has no pose CSV (the player is slot 0 of `field.csv`) and `rpcs3-drive.py capture --field` does not read `ship+0x6a54`/`+0x6a90` (`rpcs3-hd-lean-display.py` does); it needs those two reads in the capture, two columns on the player's `field.csv` row and `--pose-lean` taking them, then a re-make of `hd-race/talons-pack` from its state (`rerender-cold` found a restored HD race state lands on a different moment, so the stored pair may not be re-makeable). The follower `ship+0x6a58` (HUD sway) is not read by any capture either.
