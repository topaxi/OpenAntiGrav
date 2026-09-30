---
categories: [gameplay]
---

# A struck craft throws its own hull sparks; they still read weaker than the original's

2026-09-24. The user saw it in the original: a craft taking Cannon rounds
shows sparks and smoke. Ours showed nothing on the struck craft.

**The trigger is recovered, measured and built.** `Ship_Damage`
(`0x088439ac`), on its weapon branch, throws `WO_SHIP_COLL_SPARK_DAMAGE` on
one or two random `Ship Collision Fx` locators of the victim:

- severity 2.4;
- at most once per locator per 0.8 s;
- `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` instead for a LeachBeam hit.

The weapon's own path spawns nothing on a craft. PPSSPP stops at
`ra 0x08844050` on every posted hit, and the smoke is per hit, not a
shield state.

Evidence:

- [shield.md](../../docs/ghidra/functions/psp-pulse-usa/shield.md), "`Ship_Damage`'s weapon branch throws the hit sparks".
- [contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md): corrects the old "death burst only" reading.
- Code: `oag_game::race::hit_sparks` and `oag_gameplay::projectile::WeaponHit`.

Frames are in `data/scratch/hit-sparks/` (gitignored):

- `cannon-orig-vs-ours.png` and `cannon-orig-vs-ours-zoom.png`. The top row is PPSSPP k=0..80, the bottom row ours at ticks 300..380.
- On both sides the hit is posted into the player's pending-damage channel every 6 frames. It is not a fired Cannon.
- On PPSSPP the post goes through `craft+0x120`/`+0x138 = 3`. Ours uses a scratch capture hook calling `cannon::direct_hit` then `throw_hit_sparks`. The hook was never committed; its text is in `scratch-struck-hook.patch`.
- `cap-lowshield/` is the no-hit, 8-energy control.

## Open

- **Closed 2026-09-30 to 1.6x on the hull and 2.6x on the lower half, from
  6.5x and 9x.** The gap was two sprite layers nobody parsed. A live
  `ParticleSystem_DrawParticle` log after a hit named `shazam` (white, half-size
  9.36, six ticks) and `glow` (orange, 0.75 to 4.8, 40 ticks) at the two struck
  locators, in records `WO_SHIP_COLL_SPARK_DAMAGE.POB` carries and
  `pob::emitters` never reached: **sprite templates**, one particle per record
  the moment an emitter instance exists (`ParticleSystem_InitInstance`
  `0x088f58a4`, list at `+0x9a8`). Now parsed (`oag_vex::pob::initial`) and
  played (`oag_render::psys::template`), Pulse PSP only; layout and corpus in
  [pob.md](../../docs/formats/pob.md), "The sprite templates". 28 of them on 20
  PSP effects were never drawn: the Plasma's 36-unit `PLASMA_GLOW`, the Mine's
  `ring` and `BANG`, the Missile's `glow` and `booga`, the Quake's `shazzam`,
  the Shuriken's, the ship explosions' and the absorb's `glow`. Their looks moved
  with this; Plasma, Mine, the collision sparks and the Quake were re-shot against
  the original (below), the others were not.
- **Measured** (struck minus unstruck mean luminance, the player stationary on
  the grid after GO, a Cannon-tagged hit every 6 frames from frame 1; original
  on PPSSPP, ours from a scratch hook `direct_hit` + `throw_hit_sparks`):

  | region, frames 2-16 after the first hit | original | ours before | ours now |
  | --- | ---: | ---: | ---: |
  | hull box, first hit | +100 | +14 | +103 |
  | hull box, mean over the burst | +79 | +12 | +53 |
  | lower half, first hit | +55 | +3 | +41 |
  | lower half, mean over the burst | +45 | +5 | +17 |

  Frames: `data/scratch/pulse-impact-visuals/ours/sstruck-montage.png` (ours),
  `frames/hit-orig-struck.png` (original). **What is left**, none decoded:
  - the later hits: the original's second and third hit (one locator each, the
    others in the 0.8 s gate) wash as hard as the first; ours ~2.6x weaker.
    The locator picks are random on both sides (`ShipCollisionFx_Trigger` calls
    logged live: hit 1 locators 3,4; hit 2 locator 2; hit 3 locator 1, and so
    on), so a per-locator comparison needs the same picks;
  - the glow mask is **ruled out** (EDRAM alpha read on the software renderer
    at frames 6, 8, 10 stamps nothing under the sparks);
  - `Camera_ArmShake` on a weapon hit is now armed (`Ship_Damage`'s
    `Camera_ArmShake(0.6, 0.6, camera, 1)`, player only), which moves no
    luminance but the view.
  - The smoke is emitted and matches the file, as before.
- **Wall sparks and hit sparks keep separate cooldowns.** The original's
  0.8 s gate lives on the locator's `ShipCollisionFx` instance, so the two
  share it. Ours keeps two separate gates, and wall sparks remain the
  player's alone. Chosen.
- **`FUN_0883e37c`**, the display-mask gate in front of the spawn, is unread.
  Ours draws hit sparks on every craft.
- **`CockpitHitFx_Arm_q` (`0x088eeaf8`) is not drawn.** It is a cockpit-view
  (`craft+0x6d`) overlay armed on each weapon hit to the player: 0.6 s linear
  decay, jittered in eighths. What it draws is unread. `Camera_ArmShake` on
  weapon hits (`Ship_Damage`, same block) is not ported either.
- **HD:**
  - The Cannon sparks from the weapon side. `Cannon_ApplyCraftHit`
    (`0x0010f730`) calls `Ship_DispatchCollisionFx(..., 1)`, which spawns
    `WO_SHIP_SPARK_DAMAGE_WEAPON` at the nearest locator; see
    [ship-collision-fx.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md).
  - Not built. HD's own `Ship_Damage` is unread, and
    `WO_SHIP_SPARK_DAMAGE_WEAPON` has not been checked against the disc
    inventory.
  - HD also names `WO_DAMAGE_MILD`/`_MODERATE`/`_CRITICAL`, a likely
    shield-state smoke that Pulse lacks. Its consumer is unread.
- **Pure** is unasked and unwired: its `Ship_Damage` is unread.

## Next Steps

- Capture the original and ours with the same locator picks (force them in a
  scratch hook, and post on PPSSPP through the one-locator path
  `ShipCollisionFx_Trigger`'s `a0` names) and compare per locator; the
  remaining 2.6x on later hits is the first place to look.
- Re-shoot the Missile, Shuriken, absorb and ship-explosion looks against the
  original now that their templates play; only Plasma, Mine, the collision
  sparks and the Quake were compared.
- Read the templates' unread pieces: the two constant channels at `+0xf0` and
  `+0x2b0`, the roll channel (unplayed), and templates on child emitters
  (`Effect::skipped_templates`; zero on the collision sparks).
- HD: find its `Ship_Damage` (the `uWeaponDamageReceived` telemetry string is
  a lead), then the `WO_DAMAGE_*` consumer.
- The original fired **two** Plasma bolts from one write of the fire bit (two
  `PlasmaBlast_Construct`s a unit apart, twin `PLASMA_GLOW`s): whether a
  pickup fires a pair or the injected bit dispatched twice is unread.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-30: the gap was 28 unparsed sprite templates (`shazam`, `glow`, ...) - now parsed and played, hull box 1.6x weaker from 6.5x; camera shake armed on a weapon hit. 2026-09-24: `Ship_Damage`'s weapon branch throws `WO_SHIP_COLL_SPARK_DAMAGE` (LeachBeam: its own variant) on one or two random hull locators per landed hit, severity 2.4, 0.8 s per locator; built as `race::hit_sparks`, Pulse only. The streak strips and atlas advance landed and did not close the bloom gap (rays became orange wedge heads); next is a matched struck/unstruck capture off the light strip. HD's Cannon sparks are read, not built.
