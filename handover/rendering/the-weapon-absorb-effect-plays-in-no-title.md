# The weapon-absorb effect: burst on Pulse, Pure and HD, HD's absorb shell drawn, Pulse's hull overlay built but off

2026-09-17. The user, who plays the originals, reported: "the weapon absorb
animation/effect also does not animate/play in any title (the sfx plays
though)."

**2026-09-23: built. HD's absorb shell followed later the same day; its cockpit twin is still missing.**

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
- **The hull overlay, Pulse only: built and switched off**
  (`oag_render::hull_overlay::DRAWN = false`). When on, it draws
  `absorb_surface.mip` projected top-down over the hull for one second after
  a pickup absorb, on a `0 -> 1 -> 0` grey-alpha pulse, additive. It is off
  because a controlled PPSSPP probe never saw the original evaluate the
  overlay's gate in play; see Open. The writer,
  projection and constants are in
  [cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
  "2026-09-23". A lap refill does not light the hull; the original's stamp
  is the absorb handler's alone.

Screenshots from the 2026-09-23 lane are in
`~/.cache/oag/drive/reports/weapon-absorb/`. For Pulse, compare
`pulse-overlay-t430-crop.png` with `pulse-control-t430-crop.png`.

## Open

- **HD's absorb shell: drawn since 2026-09-23 (later).** Every team's
  `AbsorbEffect.vex`/`.rcsmodel` is drawn over the hull with
  `hd_absorbinternal`'s own fragment program, its vertex alpha scaled by the
  fade at `craft+0x7a50`. That fade relaxes a tenth of the way to `1.0` per
  frame while the one-second timer at `+0x7a5c` runs, then back to `0`,
  hidden at `0.01`. It was read on the EBOOT (`ShipAbsorbShell_Load`/`Show`/
  `Step`/`Update`) and confirmed live on RPCS3 by writing the timer. See
  [absorb-feedback.md](../../docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md),
  "The absorb shell". Frames: `~/.cache/oag/drive/reports/hd-absorb-overlay/`
  (`hd-t*.png` against `hd-control-t*.png`, and the original's
  `original-rpcs3-*.png`). **The brief's `AbsorbFader`/`AbsorbScroller` lead
  was wrong**: no file on the disc declares either, and the shell reads
  `ShieldColour`.
  Still open on HD:
  - the cockpit shell (`vr_absorbinternal_cockpit`, placed at `(0, 0, 7)`,
    measured live) is not drawn;
  - the reference frames are the pre-race flyby, not a real absorb in play,
    and not the same team as ours;
  - ours reads bluer than the original's white-lavender. Exposure and bloom
    are the first suspects, and nothing has been measured.
- **Nothing has been compared against the original's own absorb.** A PPSSPP
  capture of a Pulse absorb is the check that matters. It would settle three
  things: the burst's look at speed (world-space streaks trail the craft),
  the overlay's `vmmul` operand order, and **whether the overlay draws in
  play at all**. A 2026-09-23 controlled probe on a silent private
  instance collected 60 hits of the per-craft update and **0** of
  `HullOverlay_AbsorbWindowActive`. Stamping `craft+0x878` did not reach
  `Gu_SetMatrix(3)` at `0x0890e754`. That is why `DRAWN` is false. The probe
  that decides it: a real absorb, with the stamp store `0x088455ac` as the
  control and `0x0883e904` as the test. See
  [cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
  "2026-09-23". Two traps it hit, recorded here for whoever retries:
  - `a0` at `0x08849618` (the object `psp-drive.py` calls the craft) is not
    the entity whose held weapon is `*(entity+0x4c)+0x1bc`; its own `+0x1bc`
    is a float. The eight entities `FUN_088418e0` updates sat
    `0x2a000`-`0x35000` apart, and the last of them was `0xfc0` below that
    object, which is the unconfirmed guess for the player's. The probe hung
    before it could check.
  - `import -window root` on the Xvfb display captured black frames.
- **Overlay choice, labelled in the module:** `LessEqual` in place of
  `EQUAL`. The per-mesh gate is read: only a mesh named `...ship...` takes
  the overlay, which is `shipShape` on every Pulse team.
- **The LeachBeam's `leachbeam_surface.mip` overlay** would ride the same
  module, but its fade source (`**(float**)(state+0x4c)`) is unread, so it
  is not wired.
- **The PS2 port**: the burst plays through Pulse's schedule. The overlay is
  not built, because PS2 hull batches are VIF and have no GE input
  coordinates to project, and its own draw is unread.
- **Sound gate difference**: Pulse plays `ABSORB` only when the hull has
  collision-fx nodes (`+0xca8 != 0`). The port raises the cue regardless.

## Next Steps

1. HD: a real in-play absorb on RPCS3 (give the player a pickup and press
   circle), with the chase camera, to compare against
   `just play hd --race --give mine --input-script <absorb.inputs>`. The
   method (private `XDG_CONFIG_HOME`/`XDG_CACHE_HOME`, own GDB port, the
   craft array at `0x0098d7c0`) is in absorb-feedback.md, "Live on RPCS3".
2. Take a PPSSPP capture of a Pulse absorb (`scripts/psp-drive.py`, `--give`
   routes) and compare it frame by frame with
   `just play pulse-psp-usa --race --give mine --input-script <script>`.
