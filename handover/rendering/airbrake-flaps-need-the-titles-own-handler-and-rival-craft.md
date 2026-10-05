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
