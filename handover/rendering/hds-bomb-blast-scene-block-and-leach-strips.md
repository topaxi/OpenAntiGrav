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

- **`HD_bomb_halo` is drawn (2026-10-08, `hd-bomb-halo`).** `BOMB_HALO` (bit 21), the Fresnel shell's
  `shade.wesl` branch, the vertex program's `+ Speed` folded into `v` at load, the pool and the per-bomb
  matrix (`halo_scale(age)`, period 0.5 s). `hd_bomb.vex`'s own sixth chunk earns the same bit, which is
  the film's steady inner ring. Open: the film's white disc at the sawtooth reset (about 0.1 s every
  0.5 s) is not reproduced - the ramp's row 0 is opaque white and may be what makes it, but the shell's
  `v` range was not measured against that row; a halo-only film frame at the reset is the next check.
  The ramp's row selection assumes a repeat sampler in `v` (the film's ring needs it; the material's own
  sampler state is unread).
- **The white core's draw is unconditional on the executable** (2026-10-08): `NormalBombBlast_Draw`
  (`0x001512f8`) calls the set-matrix on `+0x2e4`, `+0x2e8` and `+0x2ec` in a row with no eye or age test
  (only the viewport picks which of two matrices). So the film's marbled frames with the eye inside are
  not "the core is skipped"; the difference is in the model's flags (`0x00151538` ORs 4 into the first
  four) or render state (depth write, cull) that this draw does not touch. Open, not changed.
- **The owner's trip window** is Pulse's 0.5 s (see the capture page), not 0.35 s; our trip excludes the owner
  for ever. Simulation, queued.
- **The matched detonation, done as far as a pinned camera goes** (`weapons.md`, `hd-bomb-match`
  section). Open: the original's owner is flung to about 125 km/h within 0.2 s and the camera
  follows, ours is shoved to 50 km/h and the sim carries no further impulse, so frames after age
  0.5 s need the owner's own motion (the camera pose is fixed per run).

## 2026-10-07 (`hd-weapon-fx`): LeachBeam strips, one more read

`0x00153218` is not a draw: it is the beam's reset (`strip_init(+0x6420); strip_init(+0x50); balls
(+0xc820); +0xc938 = 0`). The strip class initialises through `0x002a4d58(this, 300, 3)`: 300 samples
times 3 (a three-fin tube like the engine trail's, 72-byte records, buffers allocated in the SPU-visible
heap, `+0x70/+0x74/+0x80/+0x84` double-buffered), so the geometry is **SPU-extruded** like the engine
trail, and a faithful strip needs the same treatment `engine-trail.md` had: dump the buffers of a live beam
and fit. A beam needs a LeachBeam held and a rival in range, which this lane did not obtain
(states 3 and 10 gave nothing at the grid). Not started past that read.

## 2026-10-08 (`hd-leach-beam`): the strips are a PPU ribbon, read, not wired

Correction to the 2026-10-07 note above: the strips are **not** SPU-extruded.
`0x002a4d58` is `RibbonBuilder_Alloc`, the PPU builder the Rocket's smoke
uses, and the strip object drives it itself (`0x001164e0`), with model index 2
(`leachbeam_triangle`). Not a ThickLine pool. Only the `+0x6420` strip is
drawn; fade, reveal, material and the two-sided facing are on
[leach-beam-strips.md](../../docs/ghidra/functions/ps3-hdfury-eu/leach-beam-strips.md).
What stops the wiring is the **path**: the samples are the shooter's and the
target's `arc_anchor_point` nodes with a walk over the target's own sample
history between them (confidence 50), and a straight line would be an invention.
Open: a live dump of a held LeachBeam (state 3 via `rpcs3-hd-weapon.py`) reading
`+0x4c`, the count at `+0x6420+0x144` and the samples; `0x00116c48`'s selection
rule; the stack record's half-width at `+0xe8`; whether the `+0x50` strip is
ever drawn.

## 2026-10-08 (`hd-leach-path`): the path is measured live; the draw waits on the material

State **10** is the LeachBeam (state 3 never fires). Path = target's anchor trail
(3.4-unit ring per craft), walked back to the shooter's progress and bent onto the
shooter's anchor by cumulative length; nodes, half-width 1.0, `u` and the reveal law
emulated; material fragment/vertex programs read. See leach-beam-strips.md, "hd-leach-path".
Open: the second-texture/`time`/blend pipeline (unit assignment, sampler wrap, blend of
this draw), an anchor trail per craft in `oag-raceplay`, the HD state timeline (ball 0.4 s,
reveal 0.3 s, held) over our Pulse-lineage beam, the wobble `0x001157d0`.

## Next Steps

1. Build the strip from leach-beam-strips.md's measured law (anchor trail, walk, bend,
   nodes), after reading the draw's blend and texture units from a film or the draw call.
2. Add the ghost-static axis to `Title` so HD stops asking.
