# Weapon blasts draw their own sprites and wash the screen; every flash kind with a caller is wired

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
     packed sheet, atlas cell included (`oag_fx::psys::sprite`).
  2. **The sprite reader had the wrong offsets.** The header's pixel and palette
     words are relative to the resource base: they are fixup sites. Fixed in
     `oag_pob::texture`, locked by `every_psp_texture_pointer_is_a_fixup_site`.
  3. **The Quake's `/ 50` went into severity.** It belongs in the extent
     co-factor. Severity made every fireball `width / 50` too big and stacked
     them all on one point. `WO_QUAKE` is a line emitter (shape 1); lines and
     sphere extents are now placed as read (`oag_fx::psys::spawn`).

Evidence and addresses:

- [particle-system.md](../../docs/ghidra/functions/psp-pulse-usa/particle-system.md), "A particle is its sprite times its colour, and the Quake stretches its emitter".
- [pob.md](../../docs/formats/pob.md), "Correction: the two texture offsets are from the resource base".
- [rocket-visuals.md](../../docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md), "The craft-hit blast against the original".

Matched original frames live in a scratch directory, not kept (gitignored):

- Quake: `quake-compare-1.png`. Top row PPSSPP `run3` frames 12/16/20/30, middle row ours before, bottom row ours after.
- Rocket: `rocket-compare-1.png`. Top row PPSSPP craft hit at 51 units (`ppsspp-rocket/report.md`), middle before, bottom after.

**The original's Rocket peak is white at its core.** A white disc at k=8-12 is correct; the textured orange fireball that follows it was what was missing.

**2026-09-24, second pass (psys-draw lane): all three draw-side pieces
landed, Pulse PSP only.** Evidence:
[particle-system.md](../../docs/ghidra/functions/psp-pulse-usa/particle-system.md),
"Streaks, atlas advance and the screen flash".

- **`ScreenFlash_Start`'s consumer, `ScreenFlash_Update` (`0x088ef3a4`), is
  read and drawn** (`oag_fx::flash`, confidence 88).
  - It is additive, a linear lerp of the two keys, and alpha times
    `clamp(1 - (d - near) / (far - near))` from the eye, re-measured every
    frame. A pending flash replaces the running one only when stronger.
  - The PPSSPP Rocket frames match it frame by frame: red `153 (1 - t)`,
    green over red `1 - t`, blue untouched, `t` stepping `(1/60) / 0.5`.
  - Kind 0 fires on a craft-hit Rocket, and kind 4 every tick a Quake runs.
- **Streaks sample their own sprite** (`oag_fx::psys::streak`, 88). Class
  6 is a wedge: `DrawStreak`'s third vertex sits by the particle,
  `0x08916ba0`. Class 7 is the capped bar, as read before.
- **The atlas frame advances** (`oag_fx::psys::frames`, 85). The frame
  rate is the unparsed `+0x778` channel, now
  `oag_pob::Emitter::frame_rate`.

Frames are in a scratch directory, not kept (gitignored):

- `rocket-main-vs-flash.png`: original, then main, then this lane.
- `rocket-main-vs-frames.png`: the frame advance alone.
- `quake-main-vs-flash.png`: the Quake tint.
- `struck-main-vs-streak.png`: the streaks.
- `ps2-hd-pure-final.png`: PS2, HD and Pure. Each is pixel-identical to main
  at t305/t312.

## Open

- **The Rocket wash now reads as the original's** (`rocket-main-vs-flash.png`).
  What is left on the Rocket:
  - Ours detonates a few ticks early against the capture's frame count.
  - The fireball is a little smaller than the original's 85x110 PSP px at
    k=25.
    (2026-10-08: that is the **craft-hit** blast `WO_ROCKET_EXPLO`; a Time Trial pair has none before tick ~49, so it is not measurable there. A Single Race pair puts the grid craft ahead on ours and gone on the original.)
  - No matched per-frame particle count was taken.
- **The frame advance changes the fireball only subtly.** At t42 the Rocket
  fire shows more of its 4x4 texture and reads redder
  (`rocket-main-vs-frames.png`). It also animates the Shuriken, Plasma head
  and ship explosions, none of which were re-shot.
- **The other flash kinds: wired 2026-09-30.** All fifteen callers of
  `ScreenFlash_Start` are read in
  [screen-flash-callers.md](../../docs/ghidra/functions/psp-pulse-usa/screen-flash-callers.md)
  and every kind with a caller is started off its own trigger (Missile,
  Shuriken, Plasma, Bomb, Mine detonations; craft explosions; the player's
  reset), checked live on PPSSPP for the kind, the `ra` and the timing. Plasma's
  wash matches the original within 5 counts a channel over 38 frames. Still
  unwired: kinds 9 and 10 (the intro
  fly-through this port lacks), kinds 5 and 11 (no caller anywhere).
- **The Shuriken's end plays `WO_SHURIKEN_EXPIRE`** (`ShurikenPool_Update`
  `0x0886ff38`, teardown `0x08870c78`), fuse or craft hit, with the kind-0 wash.
  Its `SHURIKENEXPL` cue is not played.
- **The craft's explosion particles are not played** (`WO_SHIP_FXNODE_EXPLO`,
  `WO_SHIP_DEATH_SPARKS`, `WO_SHIP_EXPLOSION`), nor `Camera_ArmShake`, nor the
  destroy camera (`Camera_SetMode(5)`, measured 168.7 units from the wreck).
  Only the washes are.
- **A close track hit (`WO_ROCKET_EXPLO_TRACK`) washes nothing**: `Rocket_Update`
  calls no `ScreenFlash_Start`. The capture this thread asked for is therefore a
  check that nothing washes, not a measurement to make.
- **Flash details not reproduced:**
  - the `g_display+0x5dec` gate on the draw is unread;
  - the Quake's `A` edge is taken as `left`. That is chosen: which of
    `Quake_SampleSpan`'s points is `A` is not read.
- **Streaks are built in world space.** The original pushes both ends to one
  view depth: the farther for class 6, the mean for class 7. Chosen, not
  measured. The 1-unit near-plane cull of either end is not applied either.
- **Unimplemented on the particle side**:
  - billboard roll;
  - extents for shapes 2, 3, 6 and 8;
  - whatever `+0x9ac`'s random flag bits feed. It is not the frame count,
    as `pob.rs` had it.
- **The extent law is Pulse-PSP-only, by the lead's choice (2026-09-24).** It
  is read off Pulse's PSP `BOOT.BIN` alone, so every other source (PS2 Pulse,
  HD, and Pure, which is a different executable) keeps main's behaviour:
  `psys::Effect::without_extents` at load puts every emitter back at its
  anchor, and the Quake feeds its `/ 50` in as severity again. Checked
  against main (a84003c7): PS2 Quake t305/t312 and HD Quake, and PS2/HD
  engine flares at t240, all pixel-identical
  (`ps2-hd-gated-vs-main.png`, `after2/`). So
  PS2's Quake still blows out white exactly as on main - its `.pob`s embed
  no sprite. Sprite sampling and the resource-base offsets are format facts
  and stay on everywhere they apply.
  **To lift the gate, read the same four places in the other executables**
  (and, for this lane's pieces, `ParticleSystem_DrawStreak`,
  `ParticleSystem_UpdateParticles`' frame step and `ScreenFlash_Update`):
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

- **Effects that gained sprite templates 2026-09-30** (Plasma, Mine, Missile, Quake, Repulser, Shuriken, absorb, ship explosions; see the hull-sparks thread): Plasma, Mine and the Quake were re-shot against the original and read right, the others were not.

- Play the craft explosion's own effects and shake off the triggers in
  `screen-flash-callers.md`, and port the destroy camera: the player's state-5
  wash is left out until then (its falloff is measured against that camera).
- Read `Repulser_SpawnWaves` (`0x08876300`) with the Repulser itself when it is
  built; it starts kind 2.
- The Missile and Shuriken washes were placed, not seen: both detonated past
  `far` in the capture. A close one would show kind 0 beside the Rocket's.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-30: every flash kind with a caller is wired and checked live. 2026-09-24: particles draw their own `GU_TFX_MODULATE`d sprites, the sprite offsets are base-relative, and the Quake's `/ 50` is its extent co-factor. Second pass, Pulse PSP only: `ScreenFlash_Update` is read and drawn (`oag_fx::flash`, matched to PPSSPP frame by frame), streaks sample their sprite as `DrawStreak`'s wedge and `DrawCappedStreak`'s bar, and the atlas frame advances. Open: the Rocket and Quake looks against the original.
