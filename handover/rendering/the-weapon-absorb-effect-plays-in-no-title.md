# The weapon-absorb effect: burst on Pulse, Pure and HD, HD's absorb shell drawn, Pulse's hull overlay drawn

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
- **The hull overlay, Pulse only: drawn** (`oag_render::hull_overlay::DRAWN
  = true`, later on 2026-09-23). A real absorb on PPSSPP, instrumented with
  logged watchpoints, showed the stamp store, `HullOverlay_AbsorbFade` and
  `HullOverlay_Submit` firing together for exactly one second. The matrix
  handed to `Gu_SetMatrix(3)` was read live and confirms `u = 10x`,
  `v = -10z - 1.5p`. The earlier "never evaluated" probe was void: PPSSPP
  fires only the last of two execution breakpoints, and the HUD calls the
  gate every frame. The same run corrected the port in two ways:
  - it covers every list-0 hull mesh of the first level of detail, not only
    `shipShape`;
  - it writes the bloom glow mask whole, as the original's stencil
    `REPLACE 0xff` does.
  - **Corrected 2026-10-01 (`pulse-hull-bloom`): the mask write is now limited to
    the glow batch.** The original's shadow pass puts the whole mask back to 4
    after the overlay's ordinary batches, so the white blob this item described is
    PPSSPP's OpenGL backend and not the software renderer's, nor (by the GE words) the
    PSP's. `hull_overlay::stamps_mask`; evidence in `docs/rendering/glow-mask.md`, "The hull
    overlay's mask is wiped". The frames in `~/.cache/oag/drive/reports/pulse-absorb-probe/`
    are of the white-blob kind.

  Evidence is in
  [cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
  "2026-09-23 (later)". A lap refill does not light the hull; the
  original's stamp belongs to the absorb handler alone
  (`Ship_AbsorbHeldPickup`).

Screenshots from the 2026-09-23 lane are in
`~/.cache/oag/drive/reports/weapon-absorb/`. The original's own absorb
(PPSSPP, Talon's Junction, Assegai, stationary on the grid) is at
`~/.cache/oag/drive/reports/pulse-absorb-probe/original-ppsspp-01..14.png`,
0.11-1.09 s into the window. Ours at the same moments is next to it:
`glow-t*.png` has bloom off and `bloom-t*.png` has it on, and
`compare-sequence-*.png` shows the original above ours.

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
- **Superseded 2026-10-01 (`pulse-hull-bloom`): this item's target, the white blob, is what PPSSPP's OpenGL backend draws; the software renderer's completed frames (the reference here) bloom modestly, because the shadow pass wipes the overlay's mask - see `docs/rendering/glow-mask.md`, "The hull overlay's mask is wiped". Do not tune the hull back toward the blob.** Pulse: ours reads weaker than the original's absorb, much less so since
  2026-09-23 (later still). Three causes found and fixed, all measured on
  PPSSPP (see [scene-light.md](../../docs/ghidra/functions/psp-pulse-usa/scene-light.md)
  and [glow-mask.md](../../docs/rendering/glow-mask.md)):
  - the hull is lit by the circuit's own `AmbientLight`/`DirectionalLight`
    through the GE, not by our stand-in and an invented grey, so its blue is
    PPSSPP's now (`mesh_render::HullLights`);
  - the bloom is on by default for Pulse PSP: the original runs it every
    frame, and the mask it reads is stamped as the GE stencil stamps it
    (`GlowMask::Stamped`), which also takes the plume out of it;
  - the `_GLOW` decals draw at all (`GEQUAL` on the `0x10` cutouts).
  Frames: `~/.cache/oag/drive/reports/pulse-bloom/compare-absorb-bloom.png`
  (original above, ours below). **The LOD gap is closed (2026-09-23).**
  Under the old `lod = "both"` default the peak whitened only the upper
  hull, the overlay's mask had holes where the hull's `lodShape` sat in
  front of `shipShape`, and a dark ring ran round the rear hull; drawing
  tier 0 alone gave the original's white blob
  (`compare-absorb-bloom-lod-single.png`, `lod-both-vs-single-absorb.png`).
  The original draws only tier 0 up close and switches tiers per frame
  (`LodGroup_SelectChild`, `docs/formats/vex.md`), which main now does, so
  the player's hull stays on tier 0. See
  [glow-mask.md](../../docs/rendering/glow-mask.md), "What is not reproduced".
- **Pulse overlay residuals, labelled in the module:**
  - `LessEqual` stands in for `EQUAL`;
  - the colour test runs on the texel rather than the modulated fragment;
  - `self_illuminatedShape`/`glowingShape` take the fade where the original
    uses their own vertex colour;
  - both of `glowingShape`'s list-0 batches are overlaid, where the
    original was seen to submit one;
  - the airbrakes project from model space rather than their own local
    coordinates.
- **Pulse HUD: the energy bar's absorb flash - built 2026-09-25.**
  `Hud_UpdateEnergyBar` calls `HullOverlay_AbsorbWindowActive` on the player
  every frame and reads it twice: it suppresses the forced-red branch
  entirely (`iVar1`, now identified - it was the open question in
  `docs/ghidra/functions/psp-pulse-usa/shield.md`'s own "still open" note),
  and it enters the same `hud+0x1dc`/`* 8.0` blink loop the low-shield icon
  uses regardless of the red suppression. Confidence 84/82/80 for the call,
  the red suppression and the blink respectively (this page's own decompile
  ceiling). See shield.md's "`Hud_UpdateEnergyBar`: the absorb flash
  (2026-09-25)" for the full decompile. **Still open**: the widget's own
  `+0xf4` field is written a flat `0xff`/`0x00` in lock-step with the
  window, on the `ShieldBar` widget itself rather than a separately-named
  one, but *what it drives* was not chased past that write despite a
  targeted `search_instructions` pass (confidence 60) - the port stands in
  a flat white copy of the bar's own cropped fill, painted under it. The
  fourteen `pulse-absorb-probe` frames now settle that **a white layer
  genuinely exists**: the off-phase pixels read near-`255,255,255`
  uniformly across the fill, and `ShieldBarBg`'s own authored colour
  (`HudColour3A`, `0x60B5D7C8` - pale, 38% alpha) is nowhere near white, so
  "alpha-to-zero exposing the background" is ruled out by the XML itself,
  not just by the pixels. Confidence 82. Still open only at the byte
  level: which draw call paints the white layer; see shield.md's own frame
  note, which also records that the blink's *phase* does not read as
  zeroed at the absorb - consistent with the freeze-not-reset accumulator
  the port already implements, not with a per-absorb reset.
- **Pulse burst look:** in the original it reads as short crackles on the
  hull, and ours reads as long streaks. Not investigated.
- **AI absorbs light the hull too.** Seen live: an AI craft absorbed on its
  own, and `HullOverlay_Submit` ran for its one-second window. Our
  `absorb_overlay` already draws per slot, so no change is needed, unless an
  AI absorb path in ours skips `play_absorb_feedback`.
- **Overlay choice, labelled in the module:** `LessEqual` in place of
  `EQUAL`. The per-mesh gate is read: only a mesh named `...ship...` takes
  the overlay, which is `shipShape` on every Pulse team.
- **The LeachBeam's `leachbeam_surface.mip` overlay: fade source read
  statically, not wired.** It is the firing craft's weapon record `+0`,
  which `LeachBeam_UpdatePool` writes each tick with
  `LeachBeam_PulseStrength`, gated on `DAT_08b317ac == 2` ("a beam is
  live"). Confidence 75: the pool's `+0x44` slots are identified by the
  destructor, not by their writer. Not seen live: firing a LeachBeam through
  `psp-fire-weapon.py` halts PPSSPP. See cannon-quake-leachbeam.md,
  "2026-09-23 (later)".
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
2. ~~Pulse hull LOD: read which of `LodGroup`'s children the original
   draws for the player's hull, and when.~~ **Done 2026-09-23:**
   `LodGroup_SelectChild` switches per frame at the authored distance (30
   units on a craft), the player's hull never reaches it, and main switches
   the same way - `docs/formats/vex.md`. Re-check the absorb peak at player
   size with bloom on to confirm the white blob in a normal race.
3. ~~Pulse HUD: the energy bar's absorb flash (`Hud_UpdateEnergyBar`,
   `+0xf4 = 0xff`, 8 Hz blink off `+0x1dc`).~~ **Done 2026-09-25**: forced-red
   suppression and the blink are measured and built. Pixel-sampled against
   the `pulse-absorb-probe` frames: the off-phase reads a uniform white
   across the whole fill with no gradient or partial patch, and `ShieldBarBg`'s
   own authored colour rules out an alpha-reveal of the background as the
   mechanism (see shield.md's "the fourteen `pulse-absorb-probe` frames"
   note) - a white layer genuinely exists, which is what the port draws.
   Still open only at the byte level: which draw call paints it.
4. Wire the LeachBeam overlay onto the firing craft, faded by its beam's
   pulse strength. Before that, confirm the record `+0` write live. It
   needs a way to fire a LeachBeam that does not halt PPSSPP, or an AI
   LeachBeam caught with a write watch on every record's `+0`.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-23: burst on every title; HD's shell and Pulse's hull overlay drawn, both confirmed live; Pulse's hull lighting, glow mask and bloom now measured and ported; the LOD gap is closed (tier 0 up close, switched per frame); 2026-09-25: Pulse's HUD energy bar now flashes white and blinks through an absorb (forced-red suppression and the blink accumulator both ported and measured, the `+0xf4` flash layer's own draw target still not chased); open: the LeachBeam overlay, HD's cockpit shell
