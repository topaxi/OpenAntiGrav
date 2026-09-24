# PS2 Pulse's shield drew solid; it is textured and additive now, and unverified against a capture

2026-09-17. Reported from play by the user: on the PS2 Pulse source the
Shield pickup's shell "renders solid and not animated", where the PSP source
draws it right (the additive shell that swells on impact and breathes -
`oag_render::shield`, recovered on
[shield-pickup.md](../../docs/ghidra/functions/psp-pulse-usa/shield-pickup.md)).

## Diagnosed and fixed, 2026-09-24

Reproduced with `oag-game data/images/pulse-ps2-eu.chd --race --no-audio
--give shield --hold cross --press square --ticks 420 --screenshot ...`
against the same on `pulse-psp-eu.chd`: the PS2 shell was an opaque pale-blue
dome hiding the craft. `oag-view ... --mesh 'Data\Ships\Feisar\shipshield.vex'
--draws` on both discs showed two causes, both on the PS2 path alone:

1. **No texture.** The PS2 shell and `vr_shield_cockpit.vex` each declare one
   `Texture` node and embed nothing, so both bound the white 1x1.
   `livery::shield::shield_model` now takes the plume's `ps2_skin` on the same gate;
   every shell and the sphere report `1 of 1` decoded. See
   [ps2-texture.md](../../docs/formats/ps2-texture.md).
2. **No blend class.** The PS2 shell's batches (`0x1031`, `0x18b1`, `0x10b2`)
   carry no `0x0700` bit where the PSP shell's (`0x1232`) carry `0x200`, so
   they drew opaque. With the shell opaque, the alpha breath could not show,
   which accounts for "not animated". `livery::shield::blend_additively`, gated on
   the PS2 platform, moves them to the additive class. Evidence, confidence
   70: `ShipShield_Construct` (`0x00169168`) builds the shell through the
   same constructor, sort word `0x7d000000` and arguments as the plume, whose
   blending a PCSX2 capture settled. See
   [batch-draw-state.md](../../docs/ghidra/functions/ps2-pulse-eu/batch-draw-state.md#the-shield-shell-reaches-the-same-unfollowed-draw-2026-09-24).

`crates/game/tests/ps2_shield_ground_truth.rs` pins both halves on the real
disc.

## Open

- **No capture of the original PS2 shield has been compared.** The blend is
  an inference from the plume, not a reading of the shell's own draw. The
  shell's colour, brightness and whether it is additive or alpha-over are all
  unchecked against PCSX2.
- **The "not animated" half is explained, not measured.** After the fix, two
  frames a second apart show the shell, but the camera moves between them, so
  no pixel diff isolates the breath. A stationary capture (a `--pose`, or a
  shield fired and the craft held still) would.
- **One 66-triangle band may draw twice.** `--draws` lists a list-0 cutout
  batch and an opaque batch with the same triangle count, colour and centre.
  Added together, that band would read about twice as bright as the body.
  Whether the two share geometry is unread. Don't drop one to hide it.

## Next Steps

1. Take a PCSX2 capture of a raised shield on the PS2 disc and compare it
   against the same `--give shield` frame here, same team and circuit.
2. Follow `0x0029a3a0`'s draw chain (the `obj+0x38` second-base thunks) on
   `SCES_547.48`. It would turn both the plume's `draw_additive` and the
   shell's `blend_additively` into a decode at once.
