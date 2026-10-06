# Airbrake flaps: the swing sign is checked, not read, and only the player's craft moves

## Open

- Every title swings its flaps about the hinge frame's local X, Pulse's recovered
  axis. HD, 2048 and Omega's own `Airbrake` handler is unread, so the direction
  (raise and flare outward) is asserted by `hd_airbrake_flaps_ground_truth` and
  `psp2_airbrake_flaps_ground_truth`, not measured. Chosen, not measured.
- Auricom's HD flap swings out sideways (world `y` delta 0.000) and `qirex2048/2`'s
  flaps sit at the nose and do not flare; both are authored frames, neither
  confirmed against the original running.
- Rival craft keep stowed flaps on every title (Pulse included): only
  `ships.first()` is deflected.
- HD's default craft hides its flaps under a blown-out hull glow in the close
  camera; the default chase camera shows them.

## Next Steps

1. Read HD's `Airbrake` class handler (class `0x3c5`, `/hdfury/EBOOT-ps3-hdfury-eu.elf`) for the axis and sign.
2. Deflect rival craft from their own sim level, if Pulse's original does.

## Closed 2026-10-06: HD flaps drew at the model origin (the "Zone flaps do not move" report)

Cause: every HD hull (all twelve teams and the shared Zone hull) hangs its
flaps under an `Anim Transform` hinge, so the flap vertices are baked in hinge
space and carry a node slot; `Scene::write_frame` never wrote a craft's node
animation table, which starts all-identity, so the flap drew at the hull's
origin, stowed or raised. On the large coloured team hulls that read as a blade
popping up mid-hull; on Zone's small white hull it read as nothing moving.
Fix: `write_node_anims` for each craft beside `write_anims` in
`crates/raceplay/src/scene/frame.rs`. Sim side was never wrong: a Zone race's
`airbrake_flaps()` is non-zero on HD, Pure, Pulse and 2048.
Test: `crates/game/tests/zone_airbrake_flaps_ground_truth.rs` (HD single race
fails without the fix; Zone only guards the gross case), and Zone's hull added
to `hd_airbrake_flaps_ground_truth`. 2048 and Omega fly the shared `hdships\Zone` hull in
Zone (`ZoneCraft::OwnShipAt`, since 2026-10-06), which carries the same HD flap hinges the HD Zone hull does;
2048 had no such defect (no `Anim Transform` hinge). Omega racing: not checkable.
Still open: the shine pass's own drawable (`oag-render`'s `shine`) writes no
node table either; it is a Pulse mechanism, not checked on HD.
