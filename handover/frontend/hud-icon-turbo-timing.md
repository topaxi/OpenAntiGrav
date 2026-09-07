# The gantry hexagon is TurboIcon, legitimate - what's open is *when* the original grants it

2026-09-07. A maintainer-requested side-by-side
(`~/.claude/projects/-home-topaxi-projects-OpenAntiGrav/scratch/gantry-compare.md`,
`gantry-compare/side-by-side-countdown.png`) flagged a green hexagon over
our start gantry with no counterpart in the original, in every Time Trial
gantry screenshot taken that day.

## What is settled

**Not spurious, not invented - it is `PickupBackground` + `TurboIcon`,
drawn as authored.** Traced to source, not inferred:

- `Race::grant_free_turbo` (`crates/game/src/race/weapons.rs`) hands Time
  Trial/Speed Lap a free Turbo, called once from `Race::start` before lap 1's
  own edge exists, and again on `lap_completed`. Confidence 85 already
  recorded in `docs/gameplay/pickups.md`: `MSC_EVENT_TT`/`_SL`'s own text and
  `TimeTrial_HUD.xml` authoring exactly one weapon icon (`TurboIcon`) where
  Arcade authors thirteen and Zone none.
- `crates/game/src/hud/draw.rs`'s `draw_list` draws `readout.pickup` (which is
  `Some(Turbo)` from tick 0 in Time Trial) unconditionally, with no countdown
  gate anywhere in that file.
- The widget's authored position is screen top-centre (`x=240`, half the
  480-wide reference; `crates/hd/src/hud.rs` independently documents the same
  widget as "top centre" in HD's dialect) - the same region the countdown
  board occupies. The overlap in the screenshot is two top-centre widgets
  sharing space, not a stray draw or a placement bug in the ordinary sense.

So the identification is closed. **Do not delete this draw** - it is real
disc-authored content for a weapon the craft genuinely holds.

## Resolved 2026-09-07: the original grants the free Turbo at release, not at the start line

**Confirmed directly by the maintainer on `pulse-psp-eu`** (our own render
target, not just the USA disc this thread's earlier capture used): the free
Turbo appears only **after** the countdown releases the craft. It is not held
through the countdown at all - so the original never has anything in the
pickup slot for the HUD to draw during those 272 ticks, and the icon
(identification above still stands - it is `TurboIcon`, drawn correctly given
what it's handed) simply has nothing to show at that point in the real game.

**So: the HUD is not the bug.** `crates/game/src/hud/draw.rs` is drawing
`readout.pickup` exactly as it should; the bug is that `readout.pickup` is
`Some(Turbo)` during the countdown at all. **Do not touch the icon's
position, opacity or draw gate** - all three are behaving correctly given the
state they're handed.

### The exact fix, not yet landed (ran out of time before shutdown)

`Race::grant_free_turbo` (`crates/game/src/race/weapons.rs`) is called from
two places:

1. `Race::start` (`crates/game/src/race/start.rs:381`, `race.grant_free_turbo();`)
   - **this call is the bug**. It fires at tick 0, unconditionally, for a
   fresh Time Trial/Speed Lap.
2. `oag_race::Outcome::lap_completed` inside `Race::tick`
   (`crates/game/src/race/tick.rs:324`) - this one is fine; it fires well
   after the countdown on every later lap.

**Fix: delete the `Race::start` call, and instead call `grant_free_turbo()`
once at the release edge** - the same tick
`oag_race::state::RaceState::thrust_gated` first reads `false`, which is
exactly `self.world.tick == oag_race::state::COUNTDOWN_TICKS` (272, measured
in `crates/race/src/state.rs`). The natural spot is `Race::tick`, near the top
where `thrust_gated` is already read (`crates/game/src/race/tick.rs` around
line 30-35):

```rust
if self.world.tick == oag_race::state::COUNTDOWN_TICKS {
    self.grant_free_turbo();
}
```

`grant_free_turbo` already no-ops outside `TimeTrial`/`SpeedLap` and when the
slot isn't empty, so this is safe to call unconditionally at that one tick -
no new gate needed beyond the tick equality, and it only ever fires once
since `world.tick` is monotonic.

**A test has to move with it**:
`crates/game/src/race/tests/weapons.rs`'s
`a_fresh_time_trial_or_speed_lap_already_holds_lap_ones_turbo` currently
asserts the opposite of the fix - that a freshly constructed race already
holds the Turbo at tick 0. That assertion, and its doc comment ("`Race::start`
now grants it directly... begins already holding a Turbo"), both need to flip
to "no turbo at tick 0, `Some(Turbo)` once `world.tick` reaches
`COUNTDOWN_TICKS`". `only_the_two_solo_modes_are_given_a_free_turbo_and_never_two_at_once`
(same file) calls `grant_free_turbo()` directly and should be unaffected.

**Also check** `race_with_weapon_pads` and any other test helper that reads
`race.ship_pickup()` right after construction and expects `Some(Turbo)` for
`TimeTrial`/`SpeedLap` - a quick `rg 'ship_pickup\(\)' crates/game/src/race/tests`
after the change will find every assertion that needs to flip.

**Not attempted this session**: the change itself, because the full `just`
gate (fmt, clippy, `cargo nextest run --workspace`, doc/dep/determinism/size/
name/handover checks) did not fit in the ~10 minutes left before the
maintainer's shutdown, and landing it without running the gate risks a
half-fixed state worse than leaving the (now well-understood) bug in place.
The trace above - both call sites, the exact constant, the exact test to
flip - is the complete handoff.

## Next Steps

- Make the exact code change above (delete the `Race::start` call, add the
  tick-272 call in `Race::tick`), flip
  `a_fresh_time_trial_or_speed_lap_already_holds_lap_ones_turbo` and its doc
  comment, grep for any other test asserting the old tick-0 behaviour, then
  run the full `just` gate (`OAG_REQUIRE_GAME_DATA=1 just test-data` too,
  since this touches simulation state read by ground-truth tests).
- Screenshot before/after at `--size 960x540` during the countdown to
  confirm the icon is gone in that window, and still appears once released -
  the "judge as a player would" check this brief asked for, not yet done.
- Once landed, go back to `weapons-eight-of-thirteen-the-plasma-and-the.md`'s
  new note above and decide whether a blanket countdown weapon gate is now
  warranted, now that the one case that argued against it is understood to
  be our own bug rather than the original's behaviour.
- Separately, unstarted: the same comparison flagged our left-side advert
  boards rendering blurred where the original's are legible - may be a
  mip/filtering or texture-resolution difference, not chased this session.

## Open

- The grant-timing fix above: traced completely, not yet landed.
- The advert-board blur, entirely unstarted.
