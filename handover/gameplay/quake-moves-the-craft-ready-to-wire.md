# What a Quake does to the craft it reaches: ready to wire

**Status: ready to wire** (2026-10-26, `quake-craft-re`). The RE is done and measured
on PPSSPP. Evidence, addresses and confidence:
[cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
section "2026-10-26: what a Quake does to the craft it reaches". Nothing here needs new RE.

Maintainer's report, playing ours: "when getting hit by quake, the craft does not react to
it, it should get hit (physics) as well as 'ride' the quake wave".

## What the original does, and what that means for the report

1. **No physics kick (confidence 88).** The hit writes no velocity, rotation, impulse,
   stun timer or airborne flag. Its whole motion effect is the shared slowdown: thrust
   cut, no lateral grip, and the hover target lowered by `min(timer, 4.0)`. Ours already
   runs all of that (`oag_physics::slowdown`, the Quake credits `pending_slowdown`).
   **Do not add a kick.**
2. **The body does not ride the road (92, measured 2026-09-24) and the hull is not drawn
   lifted (95, measured 2026-10-26).** The road lift is render-only.
3. **The cockpit and rear-view cameras ride it (92 / 85, measured).** Each craft carries a
   smoothed wave amplitude `h` (`entity+0x870`). The internal rig adds it to its eye
   height along the ship's up; the backward rig adds it to its eye's world `y`. **The
   chase rigs do not.** In the chase view the original shows no ride either, so the
   report's "ride" is real only in the cockpit view, or it comes from HD (HD's film shows
   the camera rising with the road, and HD ships Quake pad rumbles; see the evidence
   page's "Cross-title").
4. **The same `h` is the hit gate.** The original hits on the rising edge of `h > 0.1`,
   not on our flat `radius` test, so the hit lands where and when the player sees the
   wave reach the craft.

## The gaps, ranked by what a player notices

1. **Cockpit view does not lift with the wave.** Ours: the eye is rigid. Wire it (step 2
   below).
2. **Hit timing and reach differ.** Ours hits anywhere within `QuakeStats::radius` of the
   wave's progress. The original hits once the craft has been within `w(age)/2` of the
   bump's centre long enough for `h` to pass 0.1, usually on the first frame. Late in the
   wave's life (amplitude under about 0.75) the hit lands a frame or more late, and
   under 0.1 it never lands. Wire it (step 3).
3. **Mines tripped by the wave** use the raw (unsmoothed) amplitude test `> 0.1` at the
   mine's own track cursor (`mine.md`, 2026-09-16). Ours uses `radius` there too. Same
   raw function as step 1, so wire it in the same change (step 4).
4. **No rear-view camera exists in ours** (`<BackwardCamera>`), so its lift is **not
   applicable** until one does. When it lands, it adds `h` to the eye's world `y` and
   builds the aim from the lifted eye (`Ship_UpdateBackwardCamera`, `0x0883f7b8`).
5. **`CockpitHitFx_Arm_q`** (`0x088eeaf8`, a cockpit-only overlay on any weapon hit to the
   human) is not in ours. Its picture is unread (shield.md, confidence 60). Not
   Quake-specific; leave it for its own thread.

Already in ours and matching: damage, slowdown, `QUAKEHIT`, hit sparks, the weapon-hit
camera shake (`Race::arm_weapon_hit_shake`, reached through `throw_hit_sparks` on the
Quake's landed hits), the screen flash, the road lift. No AI code reads the latch or
`h` (one masked read in the binary, the latch's own), and the PSP has no rumble.

## The law, implementable

Per craft, every tick, for every active craft including the shooter, whether or not a
wave is in flight:

```text
raw = 0
if a wave is in flight:
    d = signed distance along the course from the craft to the bump centre,
        the bump centre being wave.progress + direction * LAUNCH_LEAD (15.0)
    if |d| < half_width(a) * 0.5:      raw = amplitude(a)
    where a = the age Quake_UpdateSpan stores amplitude at: the wave's age plus one dt
          (the "age + dt" lead the evidence page records; ours draws the road with
          the same lag, `oag_render::ripple::Wave`, "lagged")
h += (raw - h) * 8.0 * dt
```

`amplitude` and `half_width` are the existing `oag_render::ripple::amplitude`/`half_width`
(12.0 peak at 0.3 s, linear to 0 at 5.0 s; half-width 25 growing 10 per second).
The original matches the craft against the wave per road span, and the spans are
distance-exact (`quake_ground_truth`), so "distance along the course" is the
first-order form. The 200-unit prefilter can never bind (`w/2 <= 37.5`), so leave it out.
**Chosen, not measured:** treating a craft on a split's other branch by course distance;
the original compares span segments. Label that in the code.

Hit latch, replacing the `radius` test:

```text
if slot == owner or h <= 0.1:  latch = false
elif shielded:                 latch = false
else:
    if !latch: land the hit (pending_slowdown += slowdown_time, damage, as today)
    latch = true
```

Camera: `oag_render::camera::internal::eye` becomes
`position + forward * length + up * (height + h)`, and `look_at_point` keeps building from
that eye (the original's aim is `eye + fwd * 10 + up * pitch`, so it rises with it).
The original reads last frame's `h` (its rigs run before the Quake block); this port
reads `h` after the tick, which is the same value one tick later. **Chosen, not
measured:** whether to reproduce the one-frame lag; reading the post-tick value is the
simpler default and differs by one tick.

## Files and functions

1. **`crates/weapons/src/projectile/quake.rs`**: move `amplitude`, `half_width`,
   `PEAK_AMPLITUDE`, `RISE_SECONDS`, `HALF_WIDTH_AT_LAUNCH`, `HALF_WIDTH_GROWTH` and
   `LAUNCH_LEAD` here from `crates/render/src/ripple.rs` (the simulation now needs them,
   and the simulation may not depend on `oag-render`). Update the call sites in
   `oag_render::ripple::Wave::new` and `crates/raceplay/src/weapons/visuals/quake.rs`;
   `oag-render` gains `oag-weapons` as a dependency (rule 1 forbids only the reverse;
   run `just check-deps`). No `pub use` shim. Add `Wave::raw_lift(progress, length, dt)
   -> f32` (the raw value above) and rewrite `Wave::apply_hits` to take the per-slot
   `h` and gate on it. Delete `radius` from `Wave` once nothing reads it; keep
   `QuakeStats::radius` decoded (its doc must then say the original's hit path never
   reads it, which is the 2026-09-07 note's own finding).
2. **The per-craft value**: `quake_lift: f32` on `oag_gameplay::world::Ship` (beside
   `pending_slowdown`), reached through a new `Craft::quake_lift_mut` in
   `crates/weapons/src/craft.rs`. It must live on the ship, not the wave: it keeps
   decaying after `Race::advance_quake` drops an expired wave. Zero it in
   `crates/gameplay/src/spawn.rs` where `pending_slowdown` is zeroed (the original
   zeroes it in the constructor; a respawn reset is **chosen, not measured**).
3. **`crates/raceplay/src/weapons/single_instance.rs` `Race::advance_quake`**: update
   every ship's `quake_lift` every tick, including ticks with no wave (raw 0), then run
   the latch. Today the function returns early with no wave; the decay must still run,
   so split the update out and call it from the tick before `advance_quake`'s early
   return.
4. **The mine trip** in the same function: replace
   `progress_delta(...) <= wave.radius` with `raw_lift(mine progress) > 0.1`.
5. **`crates/render/src/camera/internal.rs`**: `eye`/`look_at_point`/`view` take the
   lift (an `f32` argument, as `roll_phase` already is); `crates/raceplay/src/camera.rs`
   `tick_view_unshaken` passes the player ship's `quake_lift`.
6. **`docs/gameplay/weapon-lineage.md`**: move the Quake's hit-gate and camera cells to
   measured, with the evidence link, in the same change.

## Determinism

`quake_lift` is simulation state: it gates the hit, so it is hashed, and it is plain
`f32` add/mul with no transcendental (`amplitude` and `half_width` are linear). No SIMD,
no `mul_add`. **Replacing the `radius` gate moves the golden hashes** of any run with a
Quake in it: regenerate them in a separate commit that names the behaviour change, never
in the same commit. Adding the field changes the hashed layout even with no Quake, so
expect every golden hash to move; that commit says so.

## Red-to-green tests

1. `crates/weapons/src/projectile/quake/tests.rs`:
   - the lift decays by exactly `1 - 8 * dt` per tick with no wave (from 1.0, one tick at
     60 Hz gives `0.8666667`);
   - a craft at the bump centre at age 1.0 gets `raw = amplitude(1.0 + dt)`; one at
     `0.51 * half_width` from it gets 0 (the `* 0.5` is the law; at `radius` today it is
     hit);
   - the hit lands on the first tick `h > 0.1` and not before, once; a craft whose `h`
     falls to `0.1` clears its latch and is hit again on the next rise;
   - the shooter's `h` still integrates, its latch never sets.
2. `crates/render/src/camera/internal.rs` tests: `eye` with lift `h` sits `h` further
   along `up` than with 0; `look_at_point` moves by the same vector.
3. A raceplay test (`crates/raceplay/src/tests/`): fire a Quake from slot 1 at a slot 0
   craft 100 units ahead; assert slot 0's `quake_lift` rises, the hit lands once, and the
   lift is still non-zero on the tick after the wave expires.
4. Each test must fail with the law removed: drop the `* 0.5` and test 1b fails; replace
   the decay with an instant drop and test 1a fails.

Regression gate: quote `just gate-affected` and `just test-data-affected` before and
after; the selection will be the full suite (a simulation crate changes).

## Open

- **HD's per-craft camera path** was not traced; HD's film shows the camera rising with
  the road, and HD ships `Quake_Fired/Impact/Incoming_Rumble.xml`. Whether HD's camera
  lift is the same smoothed flat amplitude is unread. Read recipe: in the EU `EBOOT.elf`,
  find the `QUAKEHIT` cue's caller (string at `0x00782678`) and look beside it for a
  per-craft `+= (x - y) * 8.0 * dt`; then find that field's camera readers. Until then
  HD inherits this Pulse law per the lineage rule.
- **Pure** was not checked for the camera lift; Pure has no road lift, so likely none.
