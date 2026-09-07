# A craft hit by a weapon slows down; the port is in, the runtime check is not

2026-09-06. Reported from play: a craft hit by a mine, rocket or missile did not
slow down. It was **missing, not mistuned** - nothing armed the timer. The law
is recovered end to end and written up on
[engine.md](../../docs/ghidra/functions/psp-pulse-usa/engine.md); `oag_formats::weapons`
decodes `slowdown_time` on all six decoded blocks as of the same day, with a
ground-truth test that asserts every weapon's figure sits at or under the global
cap on both shipped tables.

**Steps 1-4 landed the same day.** A Plasma off the disc now slows a craft for
79 ticks on Talon's Junction in Venom, to 58.6 units/s against an identically
seeded control run's 122.4, and it climbs back to 104.1 once the timer expires.
What is left open is step 5 - the PPSSPP capture - and the RE loose ends below,
none of which the port depends on.

## What the port turned out to be, and the correction that matters

**Three of the four effects were already implemented**, under the name
`ShipState::leap_timer`. That field *is* `craft+0x2e0`: the engine's early
return, the lateral-grip skip and the hover target's `min(timer, 4.0)`
subtraction were all written against it long before anyone knew what armed it,
and its own doc admitted the guess ("a leap, a respawn and a race start are all
plausible"). So this thread's original "nothing is implemented, `crates/physics`
and `crates/gameplay` are untouched" was wrong in the more useful direction:
the port was a rename plus the two ends nobody had, not four effects from
scratch. Anyone reading a similar row should check for an existing field under a
hypothesised name before writing a second one - two gates on one original field
is the failure that was one commit away here.

What was genuinely missing, and is now in:

| Piece | Where |
| --- | --- |
| `Ship_AddSlowdown` (`0x08848690`), the clamp | `oag_physics::slowdown::add` |
| The timer, renamed off its hypothesis | `ShipState::slowdown_timer` |
| The decay, moved and unclamped | `oag_physics::forces::evaluate` |
| The pending slot (`entity+0x130`) | `oag_gameplay::world::Ship::pending_slowdown` |
| The credit | `oag_gameplay::projectile::blast` |
| The drain and its shield gate | `oag_gameplay::slowdown::drain` |
| The once-a-tick call | `oag_game::race::Race::tick` |

**The decay is deliberately not clamped to zero**, unlike
`airbrake::advance_sideshift`'s three timers. That function justifies its clamp
with "every reader gates on `> 0.0`, so the residue changes nothing
observable"; the premise is false here, because `Ship_AddSlowdown` adds into the
field *before* clamping, so the one-`dt` residue is worth exactly that much less
slowdown on the next hit. It is gated rather than merely unclamped - an ungated
`t -= dt` would drift without bound and make an old craft immune.

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

## Where it went

- **`oag_gameplay`** owns the pending slot, `Ship::pending_slowdown` - one `f32`
  per craft, no queue - and `slowdown::drain`, the single consumer with the
  shield gate on it.
- **`oag_physics`** owns the timer and its four effects.
  `slowdown::add` is the clamp; the engine early return, the lateral-grip skip
  and the hover subtraction were already there against the same field under its
  old name, and the decay now runs where `Ship_UpdateCraft` runs it.
- **`oag_formats`** was already done. `WeaponStats::slowdown_limit` and each
  decoded block's `slowdown_time` are read off the disc and needed nothing.
- **`oag_game`** calls the drain once a tick from `Race::tick`, over the whole
  field, ahead of every craft step.

## Open

- **No runtime capture, and this is the one that is still worth doing.** Every
  claim on `engine.md` is static; nothing has been watched in PPSSPP with a
  craft actually taking a hit. What the port now gives that it did not before
  is something to compare *against*: a trace column on `craft+0x2e0` through a
  real impact would confirm the decay rate and the saturation behaviour in one
  run, and `scripts/psp-trace.py` already carries the field as `timer_2e0`.
  See Next Steps.
- **The credit is wired to the blast and to nothing else.** Every weapon that
  reaches `projectile::blast` credits its `slowdown_time` - Rocket, Missile,
  Plasma, Mine, Bomb, Shuriken - which is the six blocks this engine decodes.
  The original has nine writers of `entity+0x130`; the other three are on paths
  this engine does not have (the Quake wave among them). Nothing is missing
  that has a Rust caller to be missing from.
- **The `4.0` clamp on the hover-target subtraction is unreachable on the
  shipped disc** and is ported anyway, in `hover::SLOWDOWN_ADJUST_MAX` - the
  only writer clamps to `slowdown_limit` first and the shipped limit is far
  below it, so `min(timer, 4.0)` is the identity for every value the field can
  hold. A test that exercises it would be testing nothing the disc can reach,
  so there is none. Whether it is dead code, a guard against a limit a later
  title raises, or the fossil of a second writer is not determined.
- **The hover-target subtraction takes seconds off a height with no conversion.**
  Recorded verbatim on `engine.md` rather than reconciled. It is the one part of
  the law that reads like a bug in the original, and it should be ported as
  written rather than "fixed" into a rate.
- **`entity+0x138`**, set alongside every credit to the pending slot, is
  unnamed. A weapon-type id is the obvious reading and it does not match the
  class-name pool's order. Nothing in the law depends on it, but a HUD or a
  kill-attribution feature would.
- **Five of the nine writers of the pending slot are unidentified**, and the
  four that are identified are not all the same quality of evidence: the
  Rocket, the Missile and the Quake wave are read here off the verified
  `0x08b32420` table, while the Mine/Bomb blast is **inherited from
  [mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md)** and rests on a
  reading of a *different* table that this work's own `Ship_Damage` evidence
  reopens (see the last bullet). Play says mines do slow a craft, which is
  corroboration of a different kind. The roster does not change the law; it
  changes which weapons visibly slow a victim once the mechanic is wired.
- **The Quake's own path also charges damage**, from the same `<WeaponStats>`
  block, so wiring the Quake means wiring both together. No Quake exists here.
- **A contradiction this work reopened and did not settle.**
  [mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md) reads the pointer
  table at `0x0885bff0` as one entry per weapon type; `Ship_Damage`
  (`0x088439ac`) indexes that same table with `entity+0x13c` - which the
  missile's own bookkeeping fills with an *attacker index* - and reads `+0x364`
  and `+0x8d8` off the result, both craft fields.
  [contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md)
  had already floated "keyed by craft". Nothing in the slowdown law rests on it
  (that table is `0x08b32420`, measured three ways), but
  `Weapon_PostBlastImpulse`'s stats identification does, and that is the
  Mine's whole blast.
- **PS2 is unchecked.** The four PSP tables that route through this parser -
  Pulse USA and EU (race and Eliminator each) and Pure USA and EU - all author
  `slowdown_time` on every decoded block, checked directly off the discs, which
  is why the parser requires it rather than defaulting. `docs/formats/README.md`
  lists the format as PSP-only and nothing calls the parser with a PS2 blob, so
  the PS2 build was not read.

## Next Steps

Steps 1-4 landed on 2026-09-06. What is left:

1. **Capture `timer_2e0` in PPSSPP through a real hit** and compare it against
   the port rather than against the disassembly alone. `scripts/psp-trace.py`
   already carries the field. Two things the capture settles that nothing else
   can: the decay really is `dt` per craft update rather than per anything else,
   and the saturation behaviour on a second hit inside the window. Both are
   currently 90-confidence readings of fifteen and four instructions
   respectively, and `crates/game/tests/weapon_slowdown_ground_truth.rs` asserts
   only that the port agrees with *itself*.
2. Settle `entity+0x138` and the five unidentified `+0x130` writers if a HUD or
   a kill-attribution feature ever needs them. Nothing in the law does.
3. Settle the `0x0885bff0` contradiction, which `Weapon_PostBlastImpulse`'s
   stats identification - and so the Mine's whole blast - does rest on.
4. Read the PS2 build's table, if anything ever calls the parser with a PS2
   blob.
