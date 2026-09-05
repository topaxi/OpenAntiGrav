# A thrown Shuriken now leaves zero blades on the disc-backed test

`crates/game/tests/shuriken_ground_truth.rs::a_thrown_blade_bounces_off_a_real_circuit_and_dies_on_its_fuse`
is deterministically red as of 2026-09-05, reproduced identically across two full
`just test-data` runs (one contended at load 22-29, one on a quiet machine) taken
minutes apart - not a flake, not an environment gap (the test needs only a disc
image, which is present):

```
assertion `left == right` failed: one press threw 0 blades
  left: 0
 right: 1
```

## What changed underneath it

`0c78c477` ("feat(race): make Wipeout Pure's VECTOR speed class selectable",
landed 2026-09-05, the same day) changed
`oag_gameplay::projectile::shuriken::launch` from returning `(Vec3, Vec3)`
unconditionally to `Option<(Vec3, Vec3)>`, and changed its `class` parameter
from an `oag_formats::handling::SpeedClass` enum to a `&str` name, resolved
through the new `ShurikenStats::speed_for_named` (name-keyed lookup) instead of
the old `speed_for` (enum-indexed, infallible). The caller in
`crates/game/src/race/weapons.rs`'s `Weapon::Shuriken` arm of
`Race::spend_pickup` now does:

```rust
let Some((position, velocity)) = oag_gameplay::projectile::shuriken::launch(
    &physics, &dimensions, &stats, &self.class, &mut self.world.rng,
) else {
    return;
};
```

silently dropping the throw if `launch` returns `None`. The same commit changed
the test fixture's own class from `SpeedClass::Venom` to `"VENOM".to_string()`
(a 3-line diff in the test file itself) - so the test was already updated for the
signature change and still fails, meaning the regression is in the lookup or
somewhere else in the same arm, not a stale test expecting the old API.

The commit message's own claim is that the new `None` path is "neither...
reachable on measured data" for Pulse - "Pure authors one class-independent
weapon speed" is the only case it expected to matter. This test runs on real
Pulse `VENOM` data and hits the silent early return anyway, which means either
that claim is wrong for the Shuriken specifically, or the `None` is coming from
a different early return in the same match arm and is unrelated to the class
lookup at all.

## What I checked and what I didn't

Checked: `SpeedClass::from_name("VENOM")` (`crates/formats/src/handling.rs`)
matches case-insensitively against `"VENOM"`/`"FLASH"`/`"RAPIER"`/`"PHANTOM"` and
should return `Some(SpeedClass::Venom)` for the exact string the test passes -
so on paper the name lookup ought to succeed, which makes it a weaker suspect
than the commit's own reasoning would suggest, not a stronger one.

**Not checked** - this is the actual next step, and it is short: the
`Weapon::Shuriken` arm in `crates/game/src/race/weapons.rs` has several early
returns before a blade is drawn, in order:

1. `spend_pickup`'s own top-level check - `fire` (Square edge) or `absorb`
   (Circle edge) pressed at all.
2. `weapons.shuriken()` returning `None` (the disc's `<Shuriken>` block not
   found in this race's `WeaponStats`).
3. `self.world.projectiles.live() >= MAX_PROJECTILES` (128 slots - very unlikely
   to be full this early, but not ruled out; the fixture leaves opponents active
   and does not isolate the field the way `stall_rescue_ground_truth::solo()`
   does).
4. `shuriken::launch(...)`'s own `?` on `stats.speed_for_named(class)`.

Any of 2-4 returning early looks identical from the test's point of view: zero
blades thrown, pickup not spent (worth checking too - the test's very next
assertion, `race.ship_pickup() == None`, is never reached because the length
assertion panics first, so it's unknown from this run whether the pickup was
even spent). **Add one `eprintln!`/`dbg!` at each of the four sites** (or a
single debug print of `weapons.shuriken().is_some()`,
`self.world.projectiles.live()`, and `stats.speed_for_named(&self.class)`
right before the `launch` call) and rerun just this test:

```sh
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
    -E 'binary(shuriken_ground_truth)'
```

That should take under ten minutes to identify which of the four is firing, at
which point the actual fix (or the actual RE correction, if `speed_for_named`
turns out to be wrong for a case the commit didn't consider) becomes obvious.

## Open

- Which of the four early-return sites in the `Weapon::Shuriken` arm is
  triggering. Not yet determined.
- If it *is* `speed_for_named` returning `None` for `"VENOM"` despite the
  case-insensitive match looking correct on paper, `self.class` might not
  actually hold `"VENOM"` at the point `launch` is called - worth printing
  `self.class` itself, not just re-deriving it should equal `"VENOM"` from the
  test setup.
- Whether `every_pure_pack_decrypts_and_parses_as_a_wad`-style pack tests or any
  other caller of `speed_for_named`/`class_named`/`table_for` (the three
  name-keyed lookups `0c78c477` introduced) has the same silent-`None` exposure
  on Pulse/HD data that the commit message asserts doesn't exist. Not swept.

## Next Steps

1. Add the debug prints above, rerun the single test, identify the firing
   branch.
2. If it's `speed_for_named`: trace why, with `self.class`'s actual runtime
   value printed alongside the lookup result - don't assume the string matches
   what the test set without checking it survived `race::load`/`Race::start`
   unchanged.
3. If it's `weapons.shuriken()` or the projectile pool: that points at a
   different bug entirely (an archive/parse regression, or an opponent filling
   the pool during the now-longer `WARM_UP_TICKS = COUNTDOWN_TICKS + 120`
   warm-up) - re-scope the thread once known.
4. Do not "fix" by making `launch` fall back to a default speed on `None` -
   `0c78c477`'s whole point was replacing a silent substitution with an honest
   `None`; if the lookup is wrong, fix the lookup, not the honesty of the
   `Option`.
