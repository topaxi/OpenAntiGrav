# The weapon AI: what an opponent does with the pickup it is holding

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** the decision is read end to end. This is the consumer of
`Data\XML\WeaponAIstats.xml` that [ai-stats.md](ai-stats.md) spent two passes
failing to find, and it is what
[`docs/gameplay/ai.md`](../../../gameplay/ai.md) and
`crates/raceplay/src/field.rs` have both recorded as "nothing about it is
recovered".

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08851550` | `WeaponAi_Update` | 85 |
| `0x088518b4` | `WeaponAi_DecideFireOrAbsorb` | 85 |
| `0x08ab0eac` | `g_weapon_ai_rate` | 80 |
| `0x08ab0ec0` | `g_weapon_ai_rate_eliminator` | 80 |
| `0x08850edc` | `WeaponAi_FindTargetInPath` | 85 |
| `0x08850d70` | `WeaponAi_ScanTraffic` | 85 |
| `0x088508d4` | `WeaponAi_ScoreForwardWeapon` | 85 |
| `0x088509e4` | `WeaponAi_BiasAbsorbByShield` | 85 |
| `0x08850d0c` | `WeaponAi_HeldPastPatience` | 80 |
| `0x08850d34` | `WeaponAi_SomethingCloseBehind` | 80 |
| `0x08850c6c` | `WeaponAi_ScoreTurbo` | 85 |
| `0x08850cc8` | `WeaponAi_ScoreShield` | 85 |
| `0x08850ac4` | `WeaponAi_ScoreBomb` | 85 |
| `0x08850b7c` | `WeaponAi_ScoreMine` | 85 |
| `0x088504f4` | `WeaponAi_RowForWeapon` | 85 |
| `0x0885058c` | `WeaponAi_LayInPlane` | 85 |
| `0x088506c4` | `WeaponAi_LayPointInPlane` | 85 |
| `0x0885a0e4` | `Missile_ClassSpeed` | 80 |
| `0x0885d22c` | `Rocket_ClassSpeed` | 80 |
| `0x0883d3c0` | `Math_FoldSigned` | 85 |

The last six, and the corrections marked 2026-10-03 below, are from
[the instruction-level pass of 2026-10-03](#the-fire-half-at-instruction-level-read-2026-10-03),
which is also what `oag_ai::weapon_ai` ports.

**How it was found matters** and is on
[ppsspp-debugger.md](../../../reverse-engineering/ppsspp-debugger.md#but-the-log-is-the-prize-not-just-a-hazard):
three static sweeps missed it because it computes the field address into a
register first, and a watchpoint *log* - which carries the program counter -
caught it in one run.

## The context object

`WeaponAi_Update(dt, self)` is called per craft per frame. `self`'s fields, as
far as they are used here:

| Offset | What |
| --- | --- |
| `+0x00` | the entity; `+0xae4` on it must be non-zero |
| `+0x04` | the weapon record - `+0x1bc` on it is the held weapon id, `-1` for empty |
| `+0x10` | the craft the request bytes are written to |
| `+0x18` | **the `WeaponAIstats` record** |
| `+0x20` | the held weapon id, from `FUN_088504f4` |
| `+0x1c` | "is holding something", set from `record[0x1bc] != -1` |
| `+0x2c` | a `0..1` scalar both branches gate on (`> 0.8`, `> 0.2`) - unread |
| `+0x30` | seconds since this craft last acted on a pickup |
| `+0x34`, `+0x38` | two skill indices, clamped to `0..4` |
| `+0x50`, `+0x51` | this weapon works on a target **ahead** / **behind** |
| `+0x54` | a cooldown in seconds, set to `3.0` on one branch |
| `+0x58` | set for the forward-firing projectile weapons |
| `+0x60` | a **signed** along-track gap |

The two request bytes it writes are `craft+0x15` (**fire**) and `craft+0x17`
(**absorb**).

Added 2026-10-03, from `WeaponAi_Construct` (`0x0885077c`) and the writer scan
in [the fire-half pass](#the-fire-half-at-instruction-level-read-2026-10-03):
`+0x3c`/`+0x40` the nearest gap ahead/behind, `+0x44` and `+0x4c` never
written, `+0x52` "no craft in the shot's path", `+0x5c` a dead field the scan
zeroes. The object is `0x68` bytes from the non-zeroing `operator new`.

## It decides on every call after the first quarter-second - corrected 2026-10-03

**Superseded heading**: this section was "It decides four times a second, not
every frame". The 2026-10-03 pass found the second instruction-level fact that
settles which way the paragraphs below lean - see
[the decision cadence](#the-decision-cadence-every-call-confidence-65) - and the
port runs every tick. The original text is kept for its evidence.

`WeaponAi_Update` accumulates `dt` into `+0x30` and does nothing until it
reaches **0.25**. So an opponent reconsiders its pickup four times a second -
**if `WeaponAi_Update` itself is called once a frame.** That premise is not
verified, and the disassembly raises a real question about it.

**`+0x30` has no reset anywhere in this function.** Checked at instruction
level, not just in the decompile (`08851648`-`08851660`: one `lwc1`/`add.s`/
`swc1` triple, no second write to that offset anywhere in the listing). If
`WeaponAi_Update` runs at 60 Hz, `+0x30` crosses `0.25` about a quarter-second
into the race and then **stays above it forever** - the `0.25 <=` gate would
open once and never close again, and the decision block would run *every
tick* for the rest of the race, not four times a second. That contradicts the
rate tables' own magnitudes (`0.05` as the *highest* non-Eliminator entry only
makes sense against a handful of rolls a second, not sixty), so the more
likely explanation is that `WeaponAi_Update` itself is called at a throttled
cadence by an as-yet-unlocated caller and the `0.25` check is either
redundant or a leftover from a version that was ticked more often. **Neither
reading is confirmed**: `get_xrefs_to` and `get_function_callers` both return
nothing for `WeaponAi_Update`, the same register-computed-address trap this
page's own "How it was found matters" section names for the two functions
this page is about - a watchpoint found the reader, not a static call site,
and nothing has looked for the *caller's own* cadence the same way.

`scripts/psp-relocate.py xrefs 0x08851550` (which matches on the relocated
value rather than Ghidra's own index, and so is not fooled by the same
register-address trap) finds exactly **one** reference: a `word` at
`0x08aca29c` holding this address - a stored function pointer, not a `jal`.
So the call is indirect, through a table entry, which is consistent with the
"a watchpoint found it, not a static sweep" story and explains why neither
Ghidra tool nor this script's own call-site search sees a caller: nothing
`jal`s this address directly, ever. `scripts/psp-relocate.py xrefs
0x08aca29c` finds nothing pointing at that table slot either, so what walks
the table - and how often - is still unlocated. A port
should keep the four-times-a-second framing (it is what the rate-table
magnitudes support) and say so is a choice, not a re-confirmed reading, until
someone catches the actual call frequency live.

Then it reads the held weapon id and switches on it, setting the direction flags
before calling the decision. **The switch is the strongest corroboration of the
weapon-id map** on [ai-stats.md](ai-stats.md), because the flags it sets match
what each weapon physically does:

| Weapon ids | `+0x50` ahead | `+0x51` behind | `+0x58` |
| --- | --- | --- | --- |
| 0 Rocket, 1 Missile, 5 Cannon, 7 Plasma, 10 LeachBeam, 11 Repulser, 12 Shuriken | yes | - | yes |
| 2 Quake | yes | - | - |
| 3 Turbo, 4 Shield | yes | yes | - |
| **8 Bomb, 9 Mines** | - | **yes** | - |

**The Bomb and the Mine are the only two flagged "behind" alone** - which is
exactly what you drop behind you - and every forward-firing weapon is flagged
"ahead". An id map that were wrong would not produce that.

Weapon id **6, the Autopilot, falls to `default`** and gets no direction flags at
all, which fits a pickup that has no target.

## The decision

`WeaponAi_DecideFireOrAbsorb(self)`, with the VFPU noise and the clamps stripped:

```c
rate = (mode == 8 || mode == 0x12) ? g_weapon_ai_rate_eliminator: g_weapon_ai_rate;      // mode is DAT_08b31048

// An early out that fires immediately and clears the cooldown.
if (self->0x58 && self->0x54 != 0.0 && !self->0x52 && rand01() < rate[4]) {
    craft[0x15] = 1; craft[0x17] = 0; self->0x54 = 0.0;
    return;
}

// Is there a target in the direction this weapon works?
in_range = (self->0x50 && 0.0 < self->0x60 && self->0x60 < 150.0)
        || (self->0x51 && self->0x60 < 0.0 && self->0x60 > -200.0);

stats = self->0x18 + self->0x20 * 0xc;          // the WeaponAIstats row
use   = in_range ? stats[0x08]                  // useAgainstPlayer: stats[0x0c];                 // useAgainstAI
if (mode == 2) use *= 2.0;
if (mode == 8) use *= 5.0;                      // Eliminator

fire_p   = rate[self->0x38] * use;
absorb_p = rate[self->0x34] * stats[0x10];      // absorb

craft[0x15] = craft[0x17] = 0;
if (rand01() >= fire_p || self->0x2c <= 0.8) {
    if (rand01() < absorb_p && self->0x2c > 0.2) craft[0x17] = 1;   // absorb
} else if (!self->0x58 || !self->0x52) {
    craft[0x15] = 1;                                                // fire
} else {
    craft[0x15] = 0; self->0x54 = 3.0;                              // wait 3 s
}
```

`rand01()` is `FUN_089731c4() * 4.656613e-10`, i.e. a 31-bit draw scaled by
`2^-31`.

**Both products are computed the long way** - `1/((1/a)*(1/b))` rather than
`a*b`. Reproduced verbatim if this is ever ported: the two are not the same
function in `f32`.

## The two rate tables

Five `f32` each, indexed by a skill level clamped to `0..4`, read out of the
image:

| Skill | `g_weapon_ai_rate` (`0x08ab0eac`) | `g_weapon_ai_rate_eliminator` (`0x08ab0ec0`) |
| --- | --- | --- |
| 0 | **0** | 0.001 |
| 1 | 0.0005 | 0.002 |
| 2 | 0.002 | 0.01 |
| 3 | 0.008 | 0.02 |
| 4 | 0.05 | 0.1 |

These are per-*decision* probabilities, and a decision happens four times a
second. Skill 0 in the ordinary table is **exactly zero** - an opponent at the
easiest setting never fires at all.

**Worth noting for `oag-ai`**: this project invented `TRIGGER_RATE = 0.05` for
its own opponents, and the original's hardest non-Eliminator rate is **0.05**.
That is a coincidence rather than a recovery - ours is per *tick* at 60 Hz and
this is per quarter-second - but it is a pleasing sanity check on the order of
magnitude.

## The conflict about `useAgainstPlayer` is resolved, and the names are right

[ai-stats.md](ai-stats.md) recorded an apparent contradiction: the consumer picks
`+0x08` versus `+0x0c` on a **range test**, not on whether the target is the
player, so the parser's attribute names looked wrong.

They are not. `+0x60` is a *signed along-track gap to one particular craft*, and
the test asks whether that craft is inside the arc and range this weapon works
in. Read together with the attribute names the reading is coherent: **`+0x60` is
the gap to the player**, so "the player is a viable target for this weapon right
now" selects `useAgainstPlayer` and everything else selects `useAgainstAI`.

Confidence **75** on that last step, and it is the one thing on this page that is
inference rather than reading: what writes `+0x60` has not been found. If it
turns out to be the nearest craft of any kind, then the shipped attribute *names*
are misleading and the behaviour is "in range" versus "not", which would be worth
knowing before anyone ports it.

## A developer path that is still in the shipped build

```c
if (self->0x10 != 0 && (sceKernelGetGPI() & 0x10)) {
    craft[0x15] = craft[0x17] = 0;
    if (rand() % 0x50 == 0)      craft[0x15] = 1;
    else if (rand() % 0x50 == 1) craft[0x17] = 1;
}
```

`sceKernelGetGPI` reads the devkit's general-purpose input pins, which are zero
on retail hardware and in PPSSPP. With bit `0x10` set the whole AI is bypassed
and every craft fires or absorbs at random. Dead on any real machine; recorded
because it is a second, simpler code path a future capture could mistake for the
real one.

## What is still open

- **`FUN_088504f4`**, which returns the held weapon id - not read, so not named.
- ~~**`self+0x2c`**, the `0..1` scalar both branches gate on...~~ **Read
  2026-09-16 - see [the six fields below](#the-six-fields-a-port-needs-read-2026-09-16).
  Not a fraction; a seconds-held counter.**
- ~~**What writes `self+0x60`**...~~ **Read 2026-09-16, and it settles the
  `useAgainstPlayer` question at the same section below: it is the gap to the
  player specifically, not to the nearest craft.**
- ~~**The two skill indices at `+0x34` and `+0x38`**, and what sets them...~~
  **Read 2026-09-16, same section: recomputed from live race state every
  decision, not a static difficulty setting.**
- **Mode `0x12`**, which shares the Eliminator rate table. `DAT_08b31048` is 3 in
  a Single Race and 8 in Eliminator (both measured live); `2` gets a `2.0`
  multiplier and is unidentified. **Still open** - not chased in the
  2026-09-16 pass either.
- **`DAT_08ab07e3`**, gating a `mode == 8` (Eliminator) branch in
  `WeaponAi_DecideFireOrAbsorb` that zeroes `+0x34` and forces a minimum
  fire chance when `+0x38` clamps to `0`. Reads like a network/authority
  flag ("is this the local sim") but not read past that. **Still open**;
  what the branch does is now read at instruction level, see
  [below](#mode-8-what-the-original-does-with-a-held-weapon-read-2026-10-03).
- ~~**`self+0x44`**, an integer field ... not enough to say what it
  counts.~~ **Read 2026-10-03: never written.** Nothing stores to it - see
  [the three fields](#0x44-0x4c-and-0x5c-two-never-written-one-dead). Uninitialised heap,
  the Cannon's fire-held byte's shape. Confidence 70.
- ~~**`self+0x5c`**, a float Turbo's own setup function compares against
  `300.0` ...~~ **Read 2026-10-03: a dead field.** `WeaponAi_Construct` writes
  `10.0`, `WeaponAi_ScanTraffic` writes `0.0` before every setup and nothing
  else writes it, so Turbo's `+0x38` is always `3`. Confidence 80.
- ~~**`self+0x4c`** ... Reads like a count of something nearby~~ **Read
  2026-10-03: never written**, as `+0x44`. Confidence 70.

## The six fields a port needs, read 2026-09-16

Everything below closes an item this page previously listed as unread, off a
direct decompile of the writers - `FUN_08850edc` (called first, unconditionally,
at the top of every `WeaponAi_Update` tick) and `FUN_08850d70` (called once per
decision, right before the per-weapon dispatch switch), plus the eight
per-weapon setup functions the switch calls. Confidence **85** throughout
unless noted - decompiled directly, not inferred, and every offset cross-checks
against how `WeaponAi_DecideFireOrAbsorb` itself consumes it.

### `+0x2c`: seconds held, uncapped - not a `0..1` fraction

This page's own speculation ("reads like a shield or energy fraction") is
wrong. `WeaponAi_Update` resets it to `0.0` the instant `+0x1c` ("is holding
something") goes false, and while holding, adds the tick's `dt` to it **every
time the 0.25 s accumulator fires** - so it grows without bound for as long as
a craft sits on a pickup. **Corrected 2026-10-03:** it gains the frame's
`dt` per decision, not `0.25` - the page said "four steps of roughly `0.25` a
second", which the listing does not support (`param_1 = +0x2c + param_1`, with
`param_1` the frame time). Read with the never-reset `+0x30`, that makes it
real held seconds; see [the decision cadence](#the-decision-cadence-every-call-confidence-65). The `> 0.8`
and `> 0.2` comparisons in the decision are therefore **hold-times in
seconds**, not thresholds on a normalised scale: an opponent will not even
roll to fire until it has held the weapon for about 0.8 s, and not roll to
absorb until about 0.2 s. A pickup taken and immediately re-decided on the
very next 0.25 s tick cannot yet do either.

### `+0x60`: the gap to the player, confirmed - not inferred

`FUN_08850d70` walks every other live craft (`DAT_08b34420`, stride `0x370`,
count `DAT_08b35fa0`) and calls `FUN_0883dbf4(self_entity, other_entity, &ok)`
for an along-track signed gap. For **every** craft it also tracks the nearest
one ahead (`+0x3c`, minimum positive gap) and behind (`+0x40`, minimum
`abs()` of a negative gap) - two fields not on this page's table before now,
new at confidence 85. But it does one more comparison per craft:

```c
if (other_entity == DAT_08b34418) self->0x60 = gap;   // DAT_08b34418: the player, by pointer identity
```

`DAT_08b34418` is a fixed singleton compared by pointer, which is a stronger
claim than "probably the local player" - it is *a specific one entity*, and
the local craft is the one entity in a race that is unique in exactly that
way. This closes the confidence-75 inference this page carried before ("if it
turns out to be the nearest craft of any kind... the shipped attribute names
are misleading"): **it is not the nearest craft, it is the player, by
identity**, so `useAgainstPlayer`/`useAgainstAI` are correctly named and the
in-range test (`self->0x50 && 0 < self->0x60 < 150`, `self->0x51 && -200 <
self->0x60 < 0`) is asking "is the player within this weapon's working arc,"
not "is anybody." Raised to confidence **90** for the field, kept at 85 for
`FUN_0883dbf4`'s own precise semantics (read enough to place it, not walked
instruction by instruction).

### `+0x34`/`+0x38`: recomputed every decision, not a difficulty setting

Both reset to `0` at the top of `WeaponAi_Update`'s decision branch, then
rebuilt from scratch before `WeaponAi_DecideFireOrAbsorb` reads them - so
**this project does not need a "difficulty rung to skill index" mapping at
all**: nothing here is a static per-race setting, it is a live score. Every
per-weapon setup function calls `FUN_088509e4` first, which reads
`Ship_Shield(self)` and bumps `+0x34` (the absorb index) by the craft's own
shield-health tier:

| Own shield | Bump to `+0x34` |
| --- | --- |
| `> 80` | none |
| `60..80` | `+1` |
| `40..60` | `+2` |
| `20..40` | `+3` |
| `< 20` | `+4` |

**A damaged opponent is more likely to absorb any weapon it holds**, which
reads as a survival instinct and needs no separate label - the numbers say it
outright. Then the per-weapon setup runs, in two shapes:

**`FUN_088508d4`** - Rocket, Missile, Quake, Cannon, LeachBeam, Repulser,
Shuriken, Plasma (everything with `+0x58` "forward-firing" set, plus Quake)
- **accumulates** onto both indices:

```c
if (self->0x2c > 15.0)                    self->0x34 += 1;   // held 15s+: itchier to absorb too
if (gap_ahead == 0 || gap_ahead > 300)    self->0x34 += 1;   // nothing ahead worth shooting
if (gap_ahead == 0 || gap_ahead >= 100) {
    if (gap_behind_flag)                  self->0x38 += 2;   // self->0x44, see "still open" above
} else                                    self->0x38 += 3;   // something close ahead: fire more readily
if (weapon == Quake && gap_behind_flag > 2 && coinflip())
                                           self->0x38 += <coinflip result>;   // FUN_088502b4, a 0.5 threshold on rand01()
if (gap_behind != 0 && gap_behind < 100)  self->0x38 += -1;  // something close behind: hold the shot instead
```

**Turbo (`FUN_08850c6c`), Shield (`FUN_08850cc8`), Bomb and Mine
(`FUN_08850ac4`/`FUN_08850b7c`, byte-identical)** - **assign** `+0x38`
outright rather than accumulate, and never touch `+0x34` at all:

```c
// Turbo
self->0x38 = (self->0x5c != 0 && self->0x5c < 300) ? 2 : 3;
// Shield
if (self->0x4c != 0 || gap_behind_flag) self->0x38 = 2;   // else left at 0
// Bomb / Mine
self->0x38 = 1;
if (self->0x4c == 1)                          self->0x38 += 1;
if (self->0x4c > 1 || (gap_behind != 0 && gap_behind < 100)) self->0x38 += 2;
if (gap_ahead != 0 && gap_ahead < 300)        self->0x38 += 1;
```

**Consequence worth porting deliberately: Turbo's `+0x34` never leaves `0`
outside the shield bump**, so a healthy opponent's `absorb_p = rate[0] *
absorb = 0.0 * 1.0 = 0.0` in every non-Eliminator mode - a Turbo is
*structurally* never absorbed unless the craft is already hurt. That is a
measured fact, not this project's own policy.

**Autopilot matches none of the eight setup functions.** `WeaponAi_Update`'s
own `switch` has no `case 6`, so it falls to `default:`, which sets
`+0x34 = 3` directly and calls no setup function at all - `+0x38` stays `0`
(the reset value) and no direction flags are set, matching this page's
existing read that Autopilot gets no aim. New at confidence 85.

### `+0x52`: is there a craft in this shot's own predicted path

`FUN_08850edc`, called first every tick before the accumulator check even
runs. It predicts the firing craft's own shot along a straight line at a
per-weapon speed, then for every other live craft extrapolates *that craft's
own* straight-line motion and finds the closest approach between the two
predicted paths. If the closest approach lands within `0 < t < 20` seconds and
the miss distance is under a margin, it writes `self->0x52 = 0` and returns
immediately - a target found. If the loop finishes without any craft
satisfying that, `self->0x52 = 1` - no target.

**Two corrections, 2026-10-03, at instruction level**
([below](#the-predicted-path-test-instruction-by-instruction)): the speed is
the Missile's for id **1 only** and the Rocket's for id 0 and everything else
(this page said "`FUN_0885a0e4` for ids 0-1"), and the margin is
`0.15 * |shot velocity * t| + 4.0` - the distance **the shot** has flown to
the closest approach, a cone of about 8.5 degrees - not the other craft's
speed.

**The sign is confirmed by which branch it feeds, not just by this reading in
isolation.** `WeaponAi_DecideFireOrAbsorb`'s fire branch is
`if (!self->0x58 || self->0x52 == 0) fire = 1; else { fire = 0; self->0x54 =
3.0; }` - "not an aimed weapon, or a target is confirmed in its path: fire.
Aimed and nothing there: hold and wait 3 s." Only the "target confirmed"
reading makes that branch make sense; a "hazard/avoid" reading would have a
craft firing *because* something was in the way, which contradicts the
early-out at the top of `WeaponAi_Update` using the identical field the same
way (`self->0x52 == 0` is one of the four early-out disjuncts, alongside
"cooldown pending" and "not forward-firing" - "if there's nowhere obviously
wrong to shoot, run the full roll; if I'm still waiting and a rare roll
succeeds, just fire").

### `+0x54`: confirmed as a real countdown, not a latch

`WeaponAi_Update` decrements it by `dt` unconditionally at the very top of
every tick, clamped to `0` (never negative) - so it is a genuine cooldown
that expires on its own, and `WeaponAi_DecideFireOrAbsorb`'s "no target"
branch is what arms it, at a fixed `3.0` s. Confidence 90 (the decrement is
four lines, unambiguous).

### What this means for a difficulty knob

**There isn't one to invent.** Both skill indices are rebuilt from race
geometry, this craft's own shield, and the weapon id every 0.25 s; the rate
tables' five entries are indexed by a live score, not a fixed per-race
setting. A port needs no "chosen, not measured" mapping here at all. The fire
half is ported as `oag_ai::weapon_ai`, predicted-path test included - see
[the 2026-10-03 pass](#the-fire-half-at-instruction-level-read-2026-10-03).

## Mode 8: what the original does with a held weapon, read 2026-10-03

Read off the disassembly of the extracted `BOOT.BIN` (image base `0x08804000`)
for the `pulse-eliminator-pace` lane, to answer "does the original's AI ever
throw away a weapon it cannot use in an Eliminator". Confidence **80**: a
direct instruction read, not observed live.

**The flag gates both Eliminator terms.** At `0x08851a8c` the byte
`DAT_08ab07e3` is tested with `ori $7, $zero, 0` in the branch's delay slot,
so `$7 = (DAT_08ab07e3 == 0 && g_game_mode == 8)`. When that holds, `+0x34` is
stored `0` and, if `+0x38` is `0`, `+0x38` becomes `1`. The `use *= 2.0` (mode
2) and `use *= 5.0` (mode 8) multipliers at `0x08851b10..5c` are gated on the
same byte the same way. Every Eliminator reading on this page assumes the byte
is `0` in a single-player race.

**So, in an Eliminator:**

- **Absorb** is `rate_eliminator[0] * absorb` = `0.001 * 1.0` per decision,
  whatever the craft's own shield, after `0.2` s held. Four decisions a second
  make that about once in four minutes of holding.
- **What an absorb does** is not energy. `Ship_AbsorbHeldPickup`
  (`0x08844ec4`) reads the controller record's absorb byte (`+0x17`, which
  this function sets for an opponent) and, in mode 8 or `0x12` with a weapon
  held, turns the weapon into the mode's one-second Shield - see
  [race-modes.md](../../../gameplay/race-modes.md#eliminator), "No energy from
  an absorb".
- **A Mine or a Bomb is laid with no target.** Neither sets `+0x58` (not an
  aimed weapon), so the fire branch never waits on `+0x52`. Fire chance per
  decision is `rate_eliminator[+0x38] * use * 5`, after `0.8` s held, with
  `+0x38` built by `FUN_08850ac4`/`FUN_08850b7c` (above): `1`, `+2` with a craft
  inside 100 behind, `+1` with one inside 300 ahead (and the `+0x4c` term, under
  50, not read). With nobody near and `useAgainstAI` `1.2`
  (`WeaponAIstats.xml`'s Bomb and Mines rows), `0.002 * 1.2 * 5 = 0.012` a
  decision, about one drop in twenty seconds.
- **A Cannon or a Leech Beam** is aimed (`+0x58`), so with no target in its
  predicted path it waits three seconds (`+0x54`) and is otherwise kept.

**Measured on this build, and not ported.** Both halves above (untargeted
drops and the `0.001` absorb), and a chosen 10 s absorb on top, were swept over
240 seeds of a parked-player Eliminator: no shift in the time to five kills
outside its 95 % interval, and the field's kill rate unchanged. See
[race-modes.md](../../../gameplay/race-modes.md#eliminator).


## The fire half at instruction level, read 2026-10-03

Read for the `pulse-eliminator-fire` lane off a fresh headless Ghidra import of
`BOOT.BIN` with the Allegrex module (analysed at base `0`, every address here
rebased by `0x08804000`), decompile and listing side by side. Not observed
live. This is the reading `oag_ai::weapon_ai` ports.

### The predicted-path test, instruction by instruction

`WeaponAi_FindTargetInPath` (`0x08850edc`), confidence **85**:

1. **The shot's speed** (`0x08850ef4..f6c`): `slti a1,a0,2`, `blez a0`, then
   `li a1,7`. Id **1** calls `Missile_ClassSpeed` (`0x0885a0e4`), id **7**
   calls `Plasma_ClassSpeed` (`0x0885c650`), and id **0 and every other id**
   call `Rocket_ClassSpeed` (`0x0885d22c`). Each is the same shape: the
   WeaponStats block `*(&DAT_08b32420 + DAT_08b32428 * 4)` at the class index
   `DAT_08b31040`, reading `+0x34..+0x40` (the Missile's four class speeds,
   [missile.md](missile.md)), `+0xac..+0xb8` (the Plasma's) and `+0x08..+0x14`
   (the Rocket's, the offsets `Rocket_SpeedForClass` reads). The result goes
   straight into `vscl.q` at `0x08850fbc`: **no `/ 3.6`**, which the launchers
   apply ([weapon-stats.md](../../../formats/weapon-stats.md)).
2. **The frame** is the shooter's node `*(entity + 0x794)`: `+0x10` the up row,
   `+0x20` the nose, `+0x30` the position ([camera.md](camera.md)). The shot's
   velocity is `WeaponAi_LayInPlane(up, nose * speed)` (`0x08850fe0`).
3. **The candidates** come from `Race_CountLiveCrafts` (`0x08826dc8`, called at
   `0x08850ff8` on `*(0x08b317b4)`): not destroyed (`+0x860 & 0x1000`) and
   `Ship_State != 7`. The shooter is skipped by entity pointer (`0x088510fc`).
   **The player is one candidate among the rest**, not singled out.
4. **Per craft**: its position laid into the shooter's plane by
   `WeaponAi_LayPointInPlane` (`0x08851138`), its velocity (`body + 0x140`) by
   `WeaponAi_LayInPlane` (`0x08851150`); `d = P - own`, `dv = V - shot`, each
   laid in again (`0x088511d0`, `0x088511e8`).
5. `t = -(d . dv) / (dv . dv)` (`neg.s` `0x08851240`, `div.s` `0x08851244`).
   `c.le.s t, 0` skips (`0x08851248`); `c.lt.s t, 20.0` false skips
   (`0x08851258`, `20.0` = `0x41a00000` loaded at `0x0885109c`). A `NaN`
   fails both and is skipped.
6. `miss = |d + dv t|` (`vsqrt.s` at `0x088512cc`) and
   `flown = |(own + shot t) - own|` (`0x088512f0..0x0885137c`).
7. `miss < flown * 0.15 + 4.0` (`mul.s` by `f24` = `0x3e19999a` at
   `0x08851388`, `add.s` `f26` = `0x40800000`, `c.lt.s` at `0x08851390`)
   stores `0` at `+0x52` and returns (`0x08851448`). The loop falling through
   stores `1`.

`WeaponAi_LayInPlane` (`0x0885058c`) is `v - (v . n) n`, rescaled to `|v|`
unless its own length is under `1e-8`. `WeaponAi_LayPointInPlane`
(`0x088506c4`) is `p0 + LayInPlane(n, p - p0)`. Both confidence 85: two short
bodies, read whole.

**So the corridor is a cone of about 8.5 degrees off the nose plus four units,
with a 20 s horizon and no other range limit**, measured against where the
target will be rather than where it is, and with no wall test: a craft round a
corner is a target if the straight line passes it.

### What else the decision reads, corrected

- **`WeaponAi_RowForWeapon`** (`0x088504f4`, confidence 85): a switch from the
  craft's held id (`craft + 0x1bc`) to the `WeaponAIstats` row the rest of the
  AI indexes by: `3 -> 5`, `4 -> 3`, `5 -> 4`, `8 -> 9`, `9 -> 8`, the rest
  unchanged, `-1` otherwise. That is [ai-stats.md](ai-stats.md)'s "fourth id
  space" (Turbo, Shield and Cannon rotated) plus the Bomb and Mine swapped. The
  ids in this page's tables are rows, not held ids.
- **The three-second window is not a wait.** The full roll runs on every
  decision whatever `+0x54` says. While it is open, an aimed weapon with a
  craft in its path gets one more chance first, at the table's top rate
  (`rate[4]`, `0.1` in an Eliminator), bypassing the hold time and the odds.
  A roll that succeeds with nothing in the path re-arms it at `3.0`.
- **`WeaponAi_ScanTraffic`'s gaps** are written only when `0x0883dbf4`'s
  out-flag is set (a test on the two craft's track sections, `0x0887d970`, not
  read further), and that gap is folded by
  `Math_FoldSigned` (`0x0883d3c0`, confidence 85: two loops, read whole) into
  half a lap either way.

### `+0x44`, `+0x4c` and `+0x5c`: two never written, one dead

`WeaponAi_Construct` (`0x0885077c`) writes `+0x00`, `+0x04`..`+0x18` (through
its helpers), `+0x24`..`+0x30`, `+0x3c = 100.0`, `+0x40 = 50.0`, `+0x52 = 0`,
`+0x54 = 0`, `+0x5c = 10.0`, `+0x60 = 50.0` and `+0x64`. **Not `+0x44` and not
`+0x4c`.** Its caller `0x08834d58` allocates the `0x68` bytes with
`operator new` (`0x08946ce4`), which does not zero
([memory.md](memory.md), [cannon-quake-leachbeam.md](cannon-quake-leachbeam.md)'s
"allocated without a zero-fill" for the Cannon's fire-held byte, the same
shape).

Two store scans, with their scope said:

- Every `sw`/`swc1`/`sb`/`sh`/`sv.s` at `0x44`, `0x4c` or `0x5c` off a register
  other than `sp`, in `0x0884c000..0x08853000`: three. `0x08850da8 swc1
  f20,0x5c(s0)` is `WeaponAi_ScanTraffic` writing `0.0`, `0x088507b0` is the
  constructor's `10.0`, and `0x0884d114 sw zero,0x44(a0)` lies outside every
  function and outside the object's methods.
- Image-wide there are 558 such stores. The eight in functions that also touch
  a byte at `+0x50..+0x58` are none of the WeaponAi's (`0x08844100`,
  `0x08873d3c`, `0x088eeaf8`, `0x0890e998`, `0x08912b80`, `0x08919cf8`,
  `0x0898f864`, `0x089c54dc`). **A filter, not a proof**: the object's vtable
  (`0x08aca284`..) holds its destructor (`0x08850890`) and `WeaponAi_Update`,
  and neither writes them.

So:

- **`+0x44` and `+0x4c` are never written: uninitialised heap.** Confidence
  70. What a real PSP holds there is not known. `+0x44` is the
  `WeaponAi_ScoreForwardWeapon` term that adds `2` to the fire index with
  nothing inside 100 ahead (and the Quake's coin flip above `2`); in an
  Eliminator it is the difference between index 1 and 2, `0.002` against
  `0.01`. `+0x4c` feeds the Shield, Bomb and Mine setups.
- **`+0x5c` is dead.** The scan writes `0.0` before every setup, so Turbo's
  `+0x38` is always `3` after the first decision. Confidence 80.

### The decision cadence: every call, confidence 65

Two instruction-level facts agree:

1. `+0x30` gains `dt` and is never reset (`0x08851648..60`, its only store), so
   the `0.25 <=` gate opens a quarter-second after construction and stays open.
2. `+0x2c` gains the frame's `dt` per decision. At four decisions a second the
   `0.2`, `0.8` and `15` s thresholds would be reached after about 3, 12 and
   225 s of real holding.

So the decision runs on **every call** after the first quarter-second,
confidence 80. That the call is once a frame rests on the stored vtable slot
(`0x08aca29c`, its walker not located) and on racing presenting at about
59.94 Hz ([ADR-0007](../../../architecture/adr/0007-fixed-timestep-vs-original.md)),
so **65** combined. A live hit count on `0x088518b4` against frames in a
Single Race on PPSSPP would settle it.

### What is ported, and what is chosen

`oag_ai::weapon_ai` ports the fire half for the Rocket, Missile, Plasma,
Shuriken and LeachBeam (aimed) and the Quake (not aimed), in every mode, on the
odds out of `WeaponAIstats.xml` (`oag_tables::weapons::ai`, through
`oag_title::weapons::Weapons::ai`). Everything above is read; these are
**chosen, not measured**:

- one decision a tick from the first tick, the clocks in 60 Hz ticks (`0.8` s
  is 48);
- `+0x44` and `+0x4c` taken as `0`;
- a craft not in `CraftState::Racing` does not fire (the original's guard,
  `entity + 0xae4`, is not read);
- the gaps from `Standing::distance`, folded by the course length, over every
  active craft;
- two `oag_ai` noise streams for the two draws;
- the Cannon (its fire byte reaches nothing), Mine, Bomb, Turbo and the absorb
  half stay on this project's own rules.

**Measured** (parked player, `16_Track`, seeds 1 to 240, Venom): time to five
kills median **108 s -> 75 s** (17 to 232 s -> 17 to 154 s), field kill rate
9.08 -> 13.63 a minute, craft-ticks holding any weapon 0.555 -> 0.299. A forward
weapon is held about 1.9 s before it goes, against 10 to 12 s. With one
decision every 15 ticks instead (the four-a-second reading): 93 s. See
[race-modes.md](../../../gameplay/race-modes.md#eliminator).
