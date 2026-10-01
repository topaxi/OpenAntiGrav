# PS2 Pulse's shield: the model and the tint are fixed against a GS dump; what is left

2026-10-01. The user reported on 2026-09-17 that the PS2 shield "renders solid and
not animated". Three layers, found in order:

1. **No texture** (2026-09-24): the shell's `Texture` node carries no pixels on PS2.
   `livery::shield::shield_model` skins it from the preceding archive entry.
2. **Wrong blend class** (2026-09-24, superseded): the shell *as then loaded*
   (`shipshield.vex`, class-less batches) drew opaque. A load-time reclassification
   (`blend_additively`) moved it to additive. **That function is gone**: layer 3.
3. **Wrong model, and a tint that is never applied** (2026-10-01, GS dump): the PS2
   executable builds `<Team>\extrashield.vex` (literal prefix `extra` at `0x002a7920`),
   not `shipshield.vex`. `extrashield.vex` carries its own additive class and its own
   `pulse_shield_extra_ADD` texture (128x64, 15 colours), and the original leaves its
   vertices at the file's colours through the fade-up, the flicker and the fade-out.
   `shield_entry_names` takes the platform; `PS2_PULSE_PALETTE` has `tints_shell =
   false`. Evidence and the GS registers:
   `docs/ghidra/functions/ps2-pulse-eu/shield-pickup.md`. The method:
   `docs/reverse-engineering/pcsx2-debugger.md` "GS dumps". Pinned by
   `crates/game/tests/ps2_shield_ground_truth.rs` and
   `oag_render::shield::tests::the_ps2_shell_is_never_tinted_and_the_other_palettes_are`.

The first pass's open items are answered: the "lattice against rings" gap was the
wrong file; the "66-triangle band drawn twice" was `shipshield.vex`'s opaque and
cutout copies of one batch, which the original does not draw; "not animated" is the
authored `u` scroll (`0 -> 251/256` over 0.9833 s, read off the GS at `1.014 /s`) plus
the swell.

## PSP half (2026-10-01, `pulse-hull-bloom`)

The PSP's `ShipShield_Construct` formats the `FE_TeamModel` registry value, not the
literal `ship`, into `%s\%sshield.vex`, so a PSP **Concept** race (hull `extra.vex`)
raises `<Team>\extrashield.vex` - on the disc for all eight teams - for the player and
every opponent. Built (`race::shield_entry_names` takes the player's hull stem) and pinned
by a unit test and a disc test. Open there: no frame of the original's Concept shield was
taken (the Concept model is behind a loyalty unlock this profile lacks), and the registry
value of a Concept race was not read live. `extrawreck.vex` exists too, by the same
registry value; the wreck loader does not use it yet. See `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`,
"The PSP's shell follows the Concept model".

## Open

- **A pixel-matched frame, and a brightness gap that is not explained.** `oag-game --pose`
  puts the craft at the savestate's `(-385.617, 4.004, 127.095)`, but the chase camera does
  not match (hull 38 % of the frame's width in the original, 23 % in ours; neither published
  eye reproduces it). Shell-only frames (frame minus its own no-shield control, inside the
  dome, three clocks): median added blue **67** in the original (20 / 80 / 101) against **44**
  in ours (42 / 44 / 47, and 21 to 69 across the scroll phase). The pixel counts differ by
  the camera, the brightness does not obviously. The registers all agree, so the term is
  unfound: candidates are the scroll clock's phase at those frames (the original's reaches 101
  where ours' sweep tops out at 69), sRGB against linear blending of the additive, and
  something outside the draw (a post pass). Numbers and method in
  `docs/ghidra/functions/ps2-pulse-eu/shield-pickup.md`, "Not yet compared".
- **The cockpit sphere** (`vr_shield_cockpit.vex`) on PS2 was not drawn from the chase
  camera and so not measured; `oag-game` keeps the tint on it. Whether it is tinted
  needs an internal-camera dump.
- **`FUN_001df718`** (the model colour setter `ShipShield_Update` calls): where its
  argument goes if not to the vertices. Unread.
- **The hit flash** (`ShipShield_Hit`, `0x00169af8`) and the shield **running out by
  its timer** on PS2: the fade-out was driven by writing `Deactivate`'s fields.
- **Teams other than Assegai** were checked for the model's shape (all twelve decode,
  `u <= 0.5`, `v <= 1`, additive) and not by a dump.

## Next Steps

1. Read the PS2 camera's real eye (the published eyes are not it) so a posed frame
   lines up, then redo the shell-only difference at pinned scroll phases on both sides.
   If the original still adds more, test sRGB against linear blending of the additive pass.
2. Take one dump from the internal camera for the sphere.
3. Decompile `FUN_001df718` (headless, `program=SCES_547.48`).
