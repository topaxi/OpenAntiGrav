# HD's LeachBeam strips are unread; the ghost static WARN names a Pulse asset

2026-10-06, hd-weapons; the Bomb half closed 2026-10-07 (`hd-weapon-blasts`).
The weapon-drawable scene work landed (`docs/rendering/hd-unlit-programs.md`,
"Weapon scene blocks"), and so did the Bomb's detonation
(`docs/ghidra/functions/ps3-hdfury-eu/weapons.md`, 2026-10-07).

## Open

- **LeachBeam body.** HD's beam is two `0x63d0`-byte strip objects at
  `LeachBeam+0x50`/`+0x6420` (`0x00115770`, constructors `0x001153c0`,
  `0x00115cc0`; 300 samples of `0x50` bytes, colour `0xffffff00`), built by
  `0x00153218`, from `leachbeam_triangle` + `hd_leachbeam.rcsmaterial` +
  `hd_leechbeam_glow.gtf`. Their draw is unread. HD no longer asks for Pulse's
  `pulse_leechbeam1_ADD.mip` (the WARN is gone; the report says the strips are
  unbuilt). **2026-10-07: not started** - the Bomb took the lane. The
  scratch interpreter that read the Bomb (`hd-weapon-blasts`'s
  `data/scratch/.../{ppcdis,emu,runblast,spec}.py`, and `bomb_blast::hd::tests`
  for the shape of the check) is the way past Ghidra's AltiVec truncation.
- **Ghost static.** `search_strings` in `/hdfury/EBOOT-ps3-hdfury-eu.elf` finds
  no `staticglow` and no `static.gtf`; `GhostShip.cpp` exists. So nothing in
  the executable binds the texture and the WARN is by design, but it still
  names a Pulse asset on HD: gate it with a `Title` field.
- **The Bomb's Repulser field and mag floor** (the brief's trailing items):
  *Repulser field* - HD hands out the Repulser, but its field model and law
  are unread (`WeaponModels::repulser_field` is `None` on HD); open.
  *Mag floor* - **not applicable**: `EBOOT.elf` carries no `MagEffect` string
  and HD draws `MagstripWake` instead. The Pulse pair's `Scene::off` note is
  moot on HD: `write_fog` is gated on `Drawable::is_ps3_shaded`.

## 2026-10-07 (`hd-weapon-ref`): the first original-side reference

Recipe in `docs/reverse-engineering/rpcs3-capture.md`, "Giving the player a weapon" (`scripts/rpcs3-hd-weapon.py`),
findings in `weapons.md` last section. Open from it:

- **`HD_bomb_halo` is not drawn; its law now agrees with the film (2026-10-07, `hd-bomb-match`).** The
  period was read on the HUD clock; in recorder time it is 0.496 s against the law's 0.5 s (conf 80 for
  the period). What is missing is the **fragment program**, not the law: `hd_bomb_halo.rcsmaterial`'s
  lit-race block `@0x19c0` is a Fresnel shell, `rim = sat(1 - N.V)`, a 4x16 ramp texture
  (`pulse_bombflash_glow`) sampled at the vertex program's `(TC0.w, TC1.w)`, alpha
  `5 ta^2 rim^(10 - 10 ta)` (`ta` the ramp's alpha), then fogged; it needs a `Shape` in
  `mesh/rcs/rim_glow.rs`, a `shade.wesl` branch and the vertex program's UV read, as `BOMB_FIRE` had.
  The pool, the law (`halo_scale(age)`), the per-bomb matrix and the draw call are written and
  tested against the generic lit program, which draws nothing visible: saved as
  `data/scratch/hd-bomb-match/halo-wiring-unfinished.patch` (also carries two debug lines to drop).
  The steady outer ring in the film is probably the sawtooth's end of cycle plus `hd_bomb.vex`'s own sixth
  chunk (same material), unconfirmed.
- **The owner's trip window** is Pulse's 0.5 s (see the capture page), not 0.35 s; our trip excludes the owner
  for ever. Simulation, queued.
- **The matched detonation, done as far as a pinned camera goes** (`weapons.md`, `hd-bomb-match`
  section). Open: the original's owner is flung to about 125 km/h within 0.2 s and the camera
  follows, ours is shoved to 50 km/h and the sim carries no further impulse, so frames after age
  0.5 s need the owner's own motion (the camera pose is fixed per run).

## Next Steps

1. Read what `0x00153218`'s strips are drawn with, then build them beside the
   other three ribbons (`docs/rendering/trail-ribbon.md`).
2. Add the ghost-static axis to `Title` so HD stops asking.
