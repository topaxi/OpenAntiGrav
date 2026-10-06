# HD's weapon drawables are never written a scene block

2026-10-06, logwarn-hd. Closed the Plasma head
(`docs/rendering/hd-unlit-programs.md`, "Gates"); the same defect is still on
every other weapon drawable.

## What was measured

- `Scene::write_ship_scenes` writes the hull's scene block (fog, light rig,
  **eye**) into `self.ships`, the wrecks and the shine passes. The pools in
  `Scene::rockets`, `mines`, `bombs`, `cannon_rounds`, the plasma blast trio,
  the Bomb blast pair and the LeachBall are written a matrix per frame
  (`write_weapon_models`) and no scene block, so each holds `Scene::off`: fog
  off, the stand-in light, and `Fog::camera = (0, 0, 0)`.
- On the Plasma head that put the eye at the world's origin, and `rim = 1 -
  N.V` painted the disc white. It now writes the hull's scene block and draws a
  dark core inside a blown-out rim (`hd_plasma_ball_ground_truth.rs`).
- The earlier thread's reading, that HD's cull keeps the faces turned away from
  the eye, was wrong: the winding and the vertex normals agree
  (`hd_unlit_probe.rs --built`), and flipping the cull moved 2,379 pixels
  without un-whitening the disc.

## Open

- The LeachBall's `RIM_GLOW` (`a = (0.9 (1 - rim^5))^5`, brightest face-on)
  reads the same eye, so its brightness and falloff are drawn from the wrong
  vector. It was judged "reads as a small cyan energy ball", which a wrong eye
  would also give.
- The Rocket, Mine, Bomb, Cannon round and the blast models are shaded without
  the circuit's rig or fog (`frame.rs` says the original lights rockets with it).
  Whether their specular term, which reads the eye too, was ever judged against
  the original is unrecorded.
- Which of them should get the hull's scene block, or the circuit's own with the
  Zone half off, is a per-weapon question against each program, not a blanket
  write: switching it on for all of them changes every HD rocket's look.

## Next Steps

1. Write the hull scene block to the LeachBall's drawable and compare a frame
   before and after at player size (`--give leach`-style input script on Talon's
   Junction).
2. Do the same for the Rocket and the Cannon round, one at a time, each against
   its fragment program in `docs/ghidra/functions/ps3-hdfury-eu/material-state.md`.
3. Pulse's weapon drawables are not touched by this: their programs are the GE's
   fixed pipeline, so check that the stand-in light there is intended before
   anything shared moves.
