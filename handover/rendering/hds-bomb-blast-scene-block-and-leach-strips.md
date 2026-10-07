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

## Next Steps

1. Read what `0x00153218`'s strips are drawn with, then build them beside the
   other three ribbons (`docs/rendering/trail-ribbon.md`).
2. Add the ghost-static axis to `Title` so HD stops asking.
