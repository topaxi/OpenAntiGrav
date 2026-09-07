# The gantry hexagon is TurboIcon, legitimate - the grant-timing bug is fixed; the advert-board blur is not

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

### The exact fix - landed 2026-09-07

`Race::grant_free_turbo` (`crates/game/src/race/weapons.rs`) was called from
two places: `Race::start`, unconditionally at tick 0 (the bug), and
`oag_race::Outcome::lap_completed` inside `Race::tick` for every later lap
(fine). Fixed by deleting the `Race::start` call and calling
`grant_free_turbo()` once from `Race::tick`, at the tick
`oag_race::state::RaceState::thrust_gated` first reads `false`
(`self.world.tick == oag_race::state::COUNTDOWN_TICKS`) - see
`crates/game/src/race/tick.rs`.

`a_fresh_time_trial_or_speed_lap_already_holds_lap_ones_turbo` (renamed
`..._holds_no_turbo_until_the_countdown_releases`) and two other tests' doc
comments that assumed the tick-0 grant were flipped alongside it -
`crates/game/src/race/tests/weapons.rs` and
`crates/game/tests/race_ground_truth.rs`. `docs/gameplay/pickups.md` and
`weapons-eight-of-thirteen-the-plasma-and-the.md` (its own countdown-gate
note) were updated to match.

**Verified**: `just` (full gate) and `OAG_REQUIRE_GAME_DATA=1 just test-data`
(full disc-backed suite) both pass. Screenshots at `--race --mode
time_trial --size 960x540` confirm it visually: no `TurboIcon` hexagon
during the countdown (`--ticks 150`), present once released (`--ticks 400`,
logged `holding Turbo`) - the "judge as a player would" check this brief
asked for.

**Left for whoever picks up
`weapons-eight-of-thirteen-the-plasma-and-the.md` next**: whether a blanket
countdown weapon gate is now warranted elsewhere, now that the one case
that argued against it (this bug) is understood and fixed.

## Next Steps

- The advert-board blur: unstarted. The same maintainer comparison flagged
  our left-side advert boards rendering blurred where the original's are
  legible - may be a mip/filtering or texture-resolution difference, not
  chased this session or the one before it.

## Open

- The advert-board blur, entirely unstarted.
