---
categories: [gameplay]
---

# A struck craft throws its own hull sparks; they read within 10 % of the original's

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

- **The brightness gap is closed to within 10 % (2026-09-30).** Measured per
  locator against the original with the same forced picks, the same camera view
  and the shake and bloom taken out of both: original / ours `0.89-0.93` on
  every one of the six locators, `0.81-0.95` per hit over six hits six frames
  apart. Three causes, in [hull-sparks.md](../../docs/rendering/hull-sparks.md):
  the earlier comparison crossed `OPT_CLOSE` and `OPT_FAR`; a template's first
  draw was a tick late; the template sprite is a stretched, rolling quad
  (`+0xf0` aspect `1.5`/`1.7`, the keyed roll channel), which the parser had
  set aside as constant. The earlier "1.6x / 2.6x" numbers are retracted.
  Ours now reads 7-11 % *brighter* (up to 20 % on the sixth overlapping hit);
  not decoded.
- **An emitter's own particles are not rotated or stretched.** 45 of the 76 PSP
  emitters draw as class 3 (`ParticleSystem_DrawRotatedSprite`), 23 author a
  roll (`WO_SHIP_COLL_SPARK_DAMAGE`'s smoke root: random `0..0.105` rad per
  tick). The law is read and measured on templates only; an emitter's flag bits
  live at a different word and what feeds its derived `+0xf0` is unread. First
  step: log `particle+0x5c`/`+0x64` of a live emitter particle.
- **A particle's age at its first draw.** The original samples a particle before
  it ages, and a template is drawn at age 0; ours now does that for templates
  only. An emitter's particle is emitted and aged in one tick in `psys`, so it
  draws one tick *older* than the original would if the same law holds. Not
  measured for emitters.
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

- Emitter rotation: capture a live emitter particle's `+0x5c` roll and `+0x64`
  aspect (the smoke root of `WO_SHIP_COLL_SPARK_DAMAGE`, a Rocket's
  `SMOKERING`), read how the emitter's flag word becomes the derived
  `res+0x884`, then play the roll on the 23 emitters that author one.
- Re-shoot the Missile, Shuriken, absorb and ship-explosion looks against the
  original now that their templates are stretched and rolled: only the
  collision sparks were compared per locator.
- The 7-11 % surplus: compare a single locator's sprite alone (the original's
  `ParticleSystem_DrawParticle` skipped for everything but it) against ours
  at a frame where nothing overlaps.
- Read the `+0x2b0` constant block's consumer on a multi-cell template (none in
  the corpus), and the templates on child emitters
  (`Effect::skipped_templates`; zero on the collision sparks).
- HD: find its `Ship_Damage` (the `uWeaponDamageReceived` telemetry string is
  a lead), then the `WO_DAMAGE_*` consumer.
- The original fired **two** Plasma bolts from one write of the fire bit (two
  `PlasmaBlast_Construct`s a unit apart, twin `PLASMA_GLOW`s): whether a
  pickup fires a pair or the injected bit dispatched twice is unread.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-30: the gap was 28 unparsed sprite templates (`shazam`, `glow`, ...) - now parsed and played; then matched per locator against the original (same picks, same camera view) and closed to within 10 %: a one-tick-late first draw and a square quad where the template is a stretched, rolling one (`+0xf0` aspect, roll channel); camera shake armed on a weapon hit. 2026-09-24: `Ship_Damage`'s weapon branch throws `WO_SHIP_COLL_SPARK_DAMAGE` (LeachBeam: its own variant) on one or two random hull locators per landed hit, severity 2.4, 0.8 s per locator; built as `race::hit_sparks`, Pulse only. The streak strips and atlas advance landed and did not close the bloom gap (rays became orange wedge heads); next is a matched struck/unstruck capture off the light strip. HD's Cannon sparks are read, not built.
