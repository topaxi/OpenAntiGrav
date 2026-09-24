# Weapon blasts draw their own sprites; the screen flash is next

2026-09-24. The Quake's crest and a close Rocket blast drew as white
blowouts where the original draws orange fire. Measured, not re-tinted:

- **(c) bloom: ruled out.** Bloom off moves 1.9% of pixels and the blowout
  stays.
- **(b) blend space: ruled out by code.** Pulse already renders into an 8-bit
  `Rgba8Unorm` gamma target, window and capture alike (ADR-0020), so forcing
  one changes nothing.
- **(a) the effect was played wrong, three ways.**
  1. **Particles drew a white procedural disc, not their own sprite.** The GE
     draws a particle as sprite texel times colour (`GU_TFX_MODULATE`, set
     once for the frame). `WO_QUAKE`'s `fireballs` author a near-white colour
     ramp over an orange fire-ring sprite. The sprites are now sampled off one
     packed sheet, atlas cell included (`oag_render::psys::sprite`).
  2. **The sprite reader had the wrong offsets.** The header's pixel and palette
     words are relative to the resource base: they are fixup sites. Fixed in
     `oag_vex::pob::texture`, locked by `every_psp_texture_pointer_is_a_fixup_site`.
  3. **The Quake's `/ 50` went into severity.** It belongs in the extent
     co-factor. Severity made every fireball `width / 50` too big and stacked
     them all on one point. `WO_QUAKE` is a line emitter (shape 1); lines and
     sphere extents are now placed as read (`oag_render::psys::spawn`).

Evidence and addresses:

- [particle-system.md](../../docs/ghidra/functions/psp-pulse-usa/particle-system.md), "A particle is its sprite times its colour, and the Quake stretches its emitter".
- [pob.md](../../docs/formats/pob.md), "Correction: the two texture offsets are from the resource base".
- [rocket-visuals.md](../../docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md), "The craft-hit blast against the original".

Matched original frames live in `data/scratch/fx-brightness/` (gitignored):

- Quake: `quake-compare-1.png`. Top row PPSSPP `run3` frames 12/16/20/30, middle row ours before, bottom row ours after.
- Rocket: `rocket-compare-1.png`. Top row PPSSPP craft hit at 51 units (`ppsspp-rocket/report.md`), middle before, bottom after.

**The original's Rocket peak is white at its core.** A white disc at k=8-12 is correct; the textured orange fireball that follows it was what was missing.

## Open

- **`ScreenFlash_Start` (`0x088f00c0`) is not drawn.** It is the full-screen
  yellow wash of a craft-hit Rocket (kind 0) and the Quake's orange tint
  (kind 4). The per-kind parameters are read: duration, two distances, two
  RGBA keys. The per-frame consumer is not read: its blend, the distance
  falloff and the key interpolation. The capture's blue channel does not move
  under the yellow wash, which points at an additive blend. That is an
  inference; the blend itself is unread.
- **Remaining visual gap on the Rocket**: ours is a few frames early and its
  fireball smaller than the original's 85x110 PSP px at k=25; no matched
  per-frame particle count was taken.
- **Unimplemented on the particle side**:
  - the atlas frame advancing over life (the frame-rate channel is unparsed);
  - streak classes still use the procedural profile (`DrawStreak`'s UVs are unread);
  - billboard roll;
  - extents for shapes 2, 3, 6 and 8.
- **The extent law is Pulse-PSP-only, by the lead's choice (2026-09-24).** It
  is read off Pulse's PSP `BOOT.BIN` alone, so every other source (PS2 Pulse,
  HD, and Pure, which is a different executable) keeps main's behaviour:
  `psys::Effect::without_extents` at load puts every emitter back at its
  anchor, and the Quake feeds its `/ 50` in as severity again. Checked
  against main (a84003c7): PS2 Quake t305/t312 and HD Quake, and PS2/HD
  engine flares at t240, all pixel-identical
  (`data/scratch/fx-brightness/ps2-hd-gated-vs-main.png`, `after2/`). So
  PS2's Quake still blows out white exactly as on main - its `.pob`s embed
  no sprite. Sprite sampling and the resource-base offsets are format facts
  and stay on everywhere they apply.
  **To lift the gate, read the same four places in the other executables**:
  the shape dispatch `ParticleSystem_SpawnBurst` (`0x088f56c4` on PSP), the
  line emitter `ParticleSystem_EmitLine` (`0x088fcfec`), the sphere emitter
  `ParticleSystem_EmitSphere` (`0x088fd340`), and where `Quake_Update`
  (`0x0891d268`) stores its `/ 50` via `ParticleSystem_SetScaleParams`
  (`0x088f44d8`) into what `ParticleSystem_DeriveScaledParams`
  (`0x088f4910`) reads. Their PS2 ELF and HD `EBOOT.elf` counterparts are
  not located; find them from the `Data\Psys\%s.POB` load string and the
  `WO_QUAKE` string's xrefs.
- **HD's `.gtf` sprites are not loaded**, so HD keeps the procedural profile.
- `Quake_Update`'s basis second row reads the struct handed to
  `AiTrack_LocatePosition`; which field is unread, so the frame's Y is world up
  (chosen).

## Next Steps

- Read the consumer of `DAT_08ab2200`'s flash block: who reads `+0x1a0` and
  `+0x60` each frame. Then draw kind 0 and kind 4. Check against
  `data/scratch/fx-brightness/ppsspp-rocket/scenarioB/det-003..008`.
- Capture a close track hit (`WO_ROCKET_EXPLO_TRACK`) in PPSSPP. None was
  captured at player size (scenario A detonated 402 units away).
