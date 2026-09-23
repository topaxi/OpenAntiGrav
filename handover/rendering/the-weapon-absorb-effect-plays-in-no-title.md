# The weapon-absorb effect: burst drawn on Pulse, Pure and HD, hull overlay drawn on Pulse

2026-09-17. The user, who plays the originals, reported: "the weapon absorb
animation/effect also does not animate/play in any title (the sfx plays
though)."

**2026-09-23: built, with two halves still missing on HD.**

- **The burst.** `oag_game::race::absorb` fires `WO_WEAPON_ABSORB` on the
  title's own stagger, and it rides the locators. Both absorb paths call
  `Race::play_absorb_feedback`: `spend_pickup`'s absorb arm and the
  Eliminator lap refill.
  - Pulse: every `Ship Collision Fx` node, up to ten, `i * 0.1` s apart.
    `Ship_GatherCollisionFxNodes` and the `+0xb8` start delay are read in
    [shield.md](../../docs/ghidra/functions/psp-pulse-usa/shield.md).
  - Pure: the same, up to eight.
  - HD: six `absorb` (`0x3ee`) locators from `Locators.vex`, fired as three
    mirrored pairs 0.2 s apart. See
    [absorb-feedback.md](../../docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md).
  - `crates/game/tests/absorb_ground_truth.rs` pins the stagger on Pulse and
    on HD.
- **The hull overlay, Pulse only.** `oag_render::hull_overlay` draws
  `absorb_surface.mip` projected top-down over the hull for one second after
  a pickup absorb, on a `0 -> 1 -> 0` grey-alpha pulse, additive. The writer,
  projection and constants are in
  [cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
  "2026-09-23". A lap refill does not light the hull; the original's stamp
  is the absorb handler's alone.

Screenshots from the 2026-09-23 lane are in
`~/.cache/oag/drive/reports/weapon-absorb/`. For Pulse, compare
`pulse-overlay-t430-crop.png` with `pulse-control-t430-crop.png`.

## Open

- **HD's burst is authored faint.** HD's own `wo_weapon_absorb.pob` has
  0.06 units/tick streaks, 5-tick lives and alpha that peaks at 120 on a
  0.37-alpha palette, and it plays exactly that. HD's prominent absorb
  picture is almost certainly its **overlay**: each team's
  `absorbeffect.vex`/`.rcsmodel` shell with a `*_hd_absorbinternal` material,
  driven by the `AbsorbFader`/`AbsorbScroller` render parameters, and
  plausibly timed by `craft+0x7a5c = 1.0`, which `Ship_PlayAbsorbFeedback`
  writes. None of that is read. On HD the user's report is therefore only
  half answered.
- **Nothing has been compared against the original's own absorb.** A PPSSPP
  capture of a Pulse absorb is the check that matters. Two things need it:
  the burst's look at speed (world-space streaks trail the craft) and the
  overlay's `vmmul` operand order. Break at `Gu_SetMatrix(3, sp+0xd0)`,
  `0x0890e754`, and read the twelve words. Run PPSSPP silent (`[Sound]
  Enable = False`).
- **Overlay choices, labelled in the module:** `LessEqual` in place of
  `EQUAL`; every hull mesh, because the per-mesh `+0x79` gate's writer is
  unread; deflected airbrake flaps keep their stowed overlay.
- **The LeachBeam's `leachbeam_surface.mip` overlay** would ride the same
  module, but its fade source (`**(float**)(state+0x4c)`) is unread, so it
  is not wired.
- **The PS2 port**: the burst plays through Pulse's schedule. The overlay is
  not built, because PS2 hull batches are VIF and have no GE input
  coordinates to project, and its own draw is unread.
- **Sound gate difference**: Pulse plays `ABSORB` only when the hull has
  collision-fx nodes (`+0xca8 != 0`). The port raises the cue regardless.

## Next Steps

1. HD overlay: read how `absorbeffect.rcsmodel` is drawn, and who writes
   `AbsorbFader`/`AbsorbScroller`. Start from `craft+0x7a5c`'s readers in
   `/hdfury/EBOOT-ps3-hdfury-eu.elf`.
2. Take a PPSSPP capture of a Pulse absorb (`scripts/psp-drive.py`, `--give`
   routes) and compare it frame by frame with
   `just play pulse-psp-usa --race --give mine --input-script <script>`.
