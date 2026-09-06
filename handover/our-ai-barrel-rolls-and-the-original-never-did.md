# Our AI barrel-rolls on purpose, and the original's never did

**Queued 2026-09-06 by maintainer directive**, verbatim: *"our AI shall do
barrel rolls, 'invented', ACE level shall do barrel rolls as long as there's
energy budget, lower AI tiers shall do less barrel rolls."*

This is a **deliberate, authorized deviation from the original** - the second
this mechanic carries, and much the larger of the two. It is not a fidelity
gap to be closed later; it is a game-design choice the maintainer made with the
original's behaviour already known.

## What the original does, so the deviation is legible

**The original's AI never barrel-rolls at all**, recovered 2026-09-06 at
confidence 85 - see
[input-bindings.md](../docs/ghidra/functions/psp-pulse-usa/input-bindings.md),
"The tap history is the player's pad, and it is cleared on the ground". Three
independent legs:

- the whole tap leg early-outs on `ship+0x78 == 0` (`0x08846be0`),
- the axis edge detector's previous-sample store is a **single global** at
  `0x08ae4cf0`, referenced from that one function and nowhere else, so only one
  craft in the field can ever drive the axis leg,
- a complete arm-site scan finds no other caller.

Our build currently matches that: after the grounded gate landed, an AI arms
**0 rolls on all twelve circuits**. It reaches the mechanism - the airborne
windows are ample, up to 76 ticks (1.27 s) on `10_Track` and 251 on `01_Track` -
but it holds its line through a jump rather than flailing the stick, so the
accidental alternation that used to arm rolls no longer happens.

**So this feature cannot be built by leaving the AI to stumble into the
gesture.** It needs a deliberate decision, which is exactly why it is an
invention rather than a port.

## Shape to build

Gate order, outermost first. The first two are recovered and **must not be
weakened** - the deviation goes on top of them, never through them:

1. **Airborne** - authored, confidence 88 (`craft+0x1c0 & 1` clears the tap
   history). Applies to every driver including the player. Keep.
2. **`cost < shield`** - authored, confidence 90. `roll_cost` is 8% of capacity.
   Keep.
3. **Energy budget** - `AI_ROLL_SHIELD_FLOOR`, currently `0.20`, AI-only,
   invented and today **dormant** (identical per-circuit figures at `0.20` and
   `0.00`). This directive is what gives it a job: Ace rolls "as long as there's
   energy budget", so this constant becomes the budget.
4. **Difficulty propensity** - new, invented. `Ace` rolls whenever 1-3 allow;
   `Elite`, `Skilled` and `Novice` progressively less often.

`oag_ai::Difficulty` is `Novice`, `Skilled`, `Elite`, `Ace`
(`crates/ai/src/difficulty.rs:42`).

## Decided 2026-09-06 by the maintainer

**All three mechanisms, and all three configurable from the pilot TOML.**
Verbatim: *"each of these variables sound good to me, let's go with
probability, raised shield floor and minimum airborne time, make them
configurable via our AI racer toml files as well."*

So three new pilot axes, alongside `commitment`, `patience`, `trigger` and the
rest in `assets/ai/example-pilot.toml`:

| Axis | What it does |
| --- | --- |
| a roll **probability** | how readily it commits, per airborne window |
| a roll **shield floor** | the energy budget below which it will not spend |
| a **minimum airborne time** | how long a jump has to be before it is worth it |

### Two constraints that will each cause a subtle bug if missed

**1. A new pilot axis appends to the END of the draw order, for ever - it never
goes between.** [`docs/gameplay/ai.md`](../docs/gameplay/ai.md) is explicit
(around line 652): every pilot consumes the same draws in the same order, so
inserting an axis re-rolls every later axis for every existing pilot. Three new
axes means three new draws, appended, in a fixed order chosen once. Getting this
wrong silently changes the character of every pilot already written, including
the four built-ins.

**2. Difficulty degrades; it never boosts.** `crates/ai/src/difficulty.rs`'s own
module doc: the top level is the measured tuning and *every level below takes
something away*, because "a baseline tuned for a novice and then multiplied
upward has no measurement behind its top end, so the hardest setting is the
least tested one, which is exactly backwards." So **Ace is the pilot's value
unmodified**, and `Elite`/`Skilled`/`Novice` degrade it - lower probability,
*raised* floor, *longer* minimum airborne time. Never the reverse.

That composes cleanly with the maintainer's directive, which already says Ace
rolls whenever the budget allows and lower tiers roll less.

### Following the file's existing conventions

`assets/ai/example-pilot.toml` sets rules the new axes inherit for free, and
each is worth honouring rather than reinventing:

- **Every axis is a range `[low, high]`**, and each craft draws its own value
  inside it - four craft on one pilot are four drivers, not one driver four
  times.
- **Anything omitted falls back to `balanced`**, so a pilot that says nothing
  about rolling still gets sensible behaviour.
- **Anything misspelled is an error naming the key**, never a setting that
  silently does nothing.
- None of these numbers is the game's - they are this project's own, per
  [ADR-0006](../docs/architecture/adr/0006-no-copyrighted-content.md). That is
  already true of the whole file and is doubly true here, where the behaviour
  itself is invented.

`AI_ROLL_SHIELD_FLOOR`'s existing `0.20` is the natural default for the floor
axis, which retires it as a bare constant.

## Open

- **Whether the AI calls `arm` directly or synthesises tap input.** Direct is
  honest about being a decision; synthesising taps would route an invented
  intent through a recovered input path and make the two hard to tell apart
  later. Direct is the recommendation, still not a ruling.
- Whether a rolling AI should also get the landing payout's turbo. It falls out
  of the existing mechanism for free if `arm` is reached the normal way, and
  nothing suggests suppressing it - but it has not been thought about.
- The three axes' default ranges are unchosen. They are play-feel numbers and
  want a play-test, not a derivation.

## Next Steps

1. ~~Decide the "does less" mechanism with the maintainer.~~ **Done
   2026-09-06** - all three, configurable per pilot. See above.
2. Implement behind the four gates above, drawing any randomness from
   `oag_core::Rng` at a fixed sequence of draws, the way `crates/ai`'s existing
   personality already does (`crates/ai/src/lib.rs:71`). Never OS entropy.
3. **Expect the committed determinism hashes to move**, in `crates/physics` and
   `crates/gameplay` both: AI craft arming rolls changes sim state directly.
   Isolate the cause, move them in their own commit, record the reasoning in
   each file's History note. **Never edit a constant to make a test pass.**
4. Re-run the disc-wide measurement (lone Ace, 18k ticks, every forward
   circuit) and report arms and shield spent per circuit per tier. The figures
   to beat: after the grounded gate, `14_Track` finishes on 41.4 shield, `09`
   on 59.2, `10` on 57.0, and `07`/`16` lap clean. A tier that rolls itself
   back down to single-digit shield has been tuned wrong.
5. Record the deviation in `HANDOVER.md` as well as in the source - a reader
   comparing against the original must not have to find it by surprise.
