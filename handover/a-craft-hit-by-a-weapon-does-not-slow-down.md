# A craft hit by a weapon does not slow down; the law is recovered, the physics half is not written

2026-09-06. Reported from play: a craft hit by a mine, rocket or missile does not
slow down. It is **missing, not mistuned** - the whole slowdown mechanic is
unimplemented in this engine. Both halves of the data have always been on the
disc (`slowdown_time` per weapon, one `<Global slowdown_limit>`), and what was
missing until now was the *law*. The law is recovered end to end and written up
on
[engine.md](../docs/ghidra/functions/psp-pulse-usa/engine.md); `oag_formats::weapons`
decodes `slowdown_time` on all six decoded blocks as of the same day, with a
ground-truth test that asserts every weapon's figure sits at or under the global
cap on both shipped tables. **This row is the implementation half, deliberately
not written in the session that recovered it**: `crates/physics` was occupied by
another thread, and the RE/implementation split is what the project's workflow
asks for.

## The law, in the shape a port needs

```text
weapon impact         victim.pending_slowdown += weapon.slowdown_time
   |
once per tick         if victim.pending_slowdown > 0 {
   |                      if !victim.shielded { add_slowdown(pending) }
   |                      victim.pending_slowdown = 0
   |                  }
   v
add_slowdown(t)       timer = min(timer + t, slowdown_limit)
   |
   +--> engine        timer > 0  ->  thrust = 0 and throttle_state = 0, this tick
   +--> hover         hover_target_height -= min(timer, 4.0)
   +--> grip          timer > 0  ->  the lateral-grip term is skipped entirely
   +--> decay         timer > 0  ->  timer -= dt      (no clamp to zero)
```

Every line above is read off the PSP executable at confidence 88-92; the
per-claim scores and the disassembly they come from are on `engine.md` under
"The slowdown mechanic, recovered end to end". `Ship_AddSlowdown` (`0x08848690`)
is the clamp, fifteen instructions with exactly one caller.

**`slowdown_limit` is a ceiling on seconds of slowdown outstanding.** Not a
speed floor and not a total-slow budget: each new impact refills the timer up to
that ceiling again, so sustained fire keeps a craft slowed indefinitely but never
raises the timer above the cap at any one moment. The Plasma is the weapon that
matters for testing here - it authors a `slowdown_time` **equal to** the global
cap on both shipped tables, so a single Plasma hit saturates the mechanic and a
second one inside the window adds nothing.

**Four ordering facts a port will get wrong by default.**

1. The drain and the decay are two different steps in the same tick, in the
   original's own order: the pending slot is drained into the timer by the
   craft's entity update, and the timer is decremented at the *end* of
   `Ship_UpdateCraft`, after the hover target has already been computed from it.
   A hit therefore costs its full first tick.
2. **The shield gate consumes the pending slot anyway.** A shielded craft takes
   no slowdown, and the pending figure is zeroed rather than banked - so a hit
   landed one tick before a shield expires is simply lost.
3. The engine effect is not a multiplier on thrust, it is an early return: no
   thrust *and* no lift, with the throttle state reset to zero, so the throttle
   has to ramp back up from nothing afterwards. That ramp is the reason the
   slowdown reads longer in play than the timer's own duration.
4. The lateral-grip skip is the same one the collision-stun timer
   (`craft+0x290`) already causes, and `oag_physics` models that one -
   whatever wires this should share that path rather than add a second gate.

## Where it goes

- **`oag_gameplay`** owns the pending slot. It is per-craft state, written by an
  impact and drained by the tick - `World`'s plain-data rule, one `f32` per
  craft, no queue.
- **`oag_physics`** owns the timer and its three effects. The engine early
  return and the lateral-grip skip both already exist for `craft+0x290`; this is
  a second timer feeding the same two gates plus the hover-target subtraction.
- **`oag_formats`** is done. `WeaponStats::slowdown_limit` and each decoded
  block's `slowdown_time` are read off the disc and need nothing further.

## Open

- **Nothing is implemented.** `crates/physics` and `crates/gameplay` are
  untouched by this work; the mechanic is inert.
- **The `4.0` clamp on the hover-target subtraction is unreachable on the
  shipped disc**, because the only writer clamps to `slowdown_limit` first and
  the shipped limit is far below it. Port it anyway - it is what the executable
  does - but a test that exercises it is testing nothing the disc can reach.
  Whether it is dead code, a guard against a limit a later title raises, or the
  fossil of a second writer is not determined.
- **The hover-target subtraction takes seconds off a height with no conversion.**
  Recorded verbatim on `engine.md` rather than reconciled. It is the one part of
  the law that reads like a bug in the original, and it should be ported as
  written rather than "fixed" into a rate.
- **`entity+0x138`**, set alongside every credit to the pending slot, is
  unnamed. A weapon-type id is the obvious reading and it does not match the
  class-name pool's order. Nothing in the law depends on it, but a HUD or a
  kill-attribution feature would.
- **Five of the nine writers of the pending slot are unidentified** - four are
  (the Mine/Bomb blast, the Rocket, the Missile and the Quake wave). The roster
  does not change the law; it changes which weapons visibly slow a victim once
  the mechanic is wired.
- **The Quake's own path also charges damage**, from the same `<WeaponStats>`
  block, so wiring the Quake means wiring both together. No Quake exists here.
- **No runtime capture.** Every claim on `engine.md` is static; nothing has been
  watched in PPSSPP with a craft actually taking a hit. A trace column on
  `craft+0x2e0` would confirm the decay rate and the saturation behaviour in one
  run, and `scripts/psp-trace.py` already carries the field as `timer_2e0`.

## Next Steps

1. Add the pending slot to `World` in `crates/gameplay` and credit it from the
   existing blast/impact paths in `oag_gameplay::projectile` - one `f32` add per
   hit, using the `slowdown_time` the parser now returns.
2. Add the timer to `crates/physics` beside the collision-stun timer, drain the
   pending slot into it once a tick with the shield gate and the clamp to
   `WeaponStats::slowdown_limit`, and decay it by `dt` at the end of the step.
3. Wire the three effects onto the timer: the engine early return and the
   lateral-grip skip through the same gates `craft+0x290` already uses, and the
   hover-target subtraction as written, `4.0` clamp included.
4. Test it against the disc, not against a literal: fire a Plasma (the weapon
   that saturates the cap) at a craft and assert the victim's speed falls and
   recovers, and that a second hit inside the window does not extend the timer
   past the cap. `crates/game/tests` is where a whole-race assertion of that
   shape belongs.
5. Only then consider a PPSSPP capture of `timer_2e0` through a real hit, to
   confirm the decay rate against the port rather than against the disassembly
   alone.
