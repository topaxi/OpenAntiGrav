# The weapon-absorb effect plays in no title, and its trigger is already recovered

2026-09-17. The user, who plays the originals, reported from play: "the
weapon absorb animation/effect also does not animate/play in any title (the
sfx plays though)." Confirmed against the source: `Cue::Absorb` fires, and
nothing draws.

What is already read, so this is a build, not an RE pass:

- **The particle burst.** `Ship_PlayAbsorbFeedback` (`0x08840640`,
  [shield.md](../../docs/ghidra/functions/psp-pulse-usa/shield.md), "Ported"
  paragraph) plays `ABSORB` once and then calls
  `ShipCollisionFx_Trigger(1.0, node, 2, 0)` - kind 2 is `WO_WEAPON_ABSORB`
  ([contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md))
  - over up to **ten** of the craft's fx nodes, staggered `i * 0.1` s
  (`DAT_08abf564`). Its four callers are all read: the Eliminator lap refill,
  the absorb handler's tail (`0x088455b8`), and two network arms. That page's
  own "What is not verified" says it plainly: "the absorb-spark burst is not
  drawn ... it is the stagger and the per-node anchoring that wait, not the
  reading." `crates/game/tests/psys_inventory_ground_truth.rs`'s
  `WO_WEAPON_ABSORB` reason ("nothing has been read that ties this effect to
  that path") is **stale** and should be retired in the same change that
  wires it.
- **The hull overlay.** `Data\Tex\Weapons\absorb_surface.mip` (string at
  `0x08a88378`, loaded by `Texture_LoadEffectSurfaces` `0x0890cc1c` into
  `DAT_08af2800`) is drawn over the craft's own hull sub-meshes by
  `FUN_0890d828` -> `FUN_0890e304`, gated on `craft+0x74 -> +0x4c > 0`, the
  same path the LeachBeam's `leachbeam_surface.mip` takes - see
  [cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
  "2026-09-08: the LeachBeam's texture, located - and it is not the ribbon's".
  Both resolve in `Data.wad` (2,064 bytes, 32x32 8-bit). HD ships
  `/data/tex/weapons/absorb_surface.gtf` too (DATA02). The overlay draw
  itself (`FUN_0890e304`: blend, UV/texture matrix, fade) is being read by
  the LeachBeam ribbon lane of 2026-09-17; check that page before re-reading.

## Open

- The ten staggered bursts: which ten "fx nodes" - the same `Fx` locator set
  `WO_SHIP_FXNODE_EXPLO` plays at, unread on the PSP; HD's
  `Ship_DispatchCollisionFx_q` picks nearest-of-ten locators
  ([ship-collision-fx.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md)),
  which is the strongest hint that the ten are the hull's own locator nodes.
  This engine has a located set for HD (`Locators.vex`) and the PSP
  `Ship.vex` node walk.
- The overlay's per-frame animation: `craft+0x74 -> +0x4c` is a scalar that
  something counts down; find its writer (probably the absorb handler) and
  its rate.
- Which title the user saw it missing on first; the fix is one path for all
  three since all three ship the same two assets.

## Next Steps

1. Wire `WO_WEAPON_ABSORB` on the absorb path (`Race::spend_pickup`'s absorb
   arm and the Eliminator refill) as ten staggered `psys::Stage::play`
   instances anchored on the craft's fx locators - fall back to the craft
   position for the anchors only if the locator set is genuinely unread, and
   say so in the loader report. Retire the stale inventory reason.
2. Build the hull overlay from the LeachBeam lane's `FUN_0890e304` reading
   (or read it if that lane did not reach it), driven by the recovered scalar.
3. Screenshot both at player size, several frames, on Pulse PSP and HD.
