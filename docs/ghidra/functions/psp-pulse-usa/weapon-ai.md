# The weapon AI: what an opponent does with the pickup it is holding

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** the decision is read end to end. This is the consumer of
`Data\XML\WeaponAIstats.xml` that [ai-stats.md](ai-stats.md) spent two passes
failing to find, and it is what
[`docs/gameplay/ai.md`](../../../gameplay/ai.md) and
`crates/game/src/race/field.rs` have both recorded as "nothing about it is
recovered".

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08851550` | `WeaponAi_Update` | 85 |
| `0x088518b4` | `WeaponAi_DecideFireOrAbsorb` | 85 |
| `0x08ab0eac` | `g_weapon_ai_rate` | 80 |
| `0x08ab0ec0` | `g_weapon_ai_rate_eliminator` | 80 |

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

## It decides four times a second, not every frame

`WeaponAi_Update` accumulates `dt` into `+0x30` and does nothing until it
reaches **0.25**. So an opponent reconsiders its pickup four times a second.

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
rate = (mode == 8 || mode == 0x12) ? g_weapon_ai_rate_eliminator
                                   : g_weapon_ai_rate;      // mode is DAT_08b31048

// An early out that fires immediately and clears the cooldown.
if (self->0x58 && self->0x54 != 0.0 && !self->0x52 && rand01() < rate[4]) {
    craft[0x15] = 1; craft[0x17] = 0; self->0x54 = 0.0;
    return;
}

// Is there a target in the direction this weapon works?
in_range = (self->0x50 && 0.0 < self->0x60 && self->0x60 < 150.0)
        || (self->0x51 && self->0x60 < 0.0 && self->0x60 > -200.0);

stats = self->0x18 + self->0x20 * 0xc;          // the WeaponAIstats row
use   = in_range ? stats[0x08]                  // useAgainstPlayer
                 : stats[0x0c];                 // useAgainstAI
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
- **`self+0x2c`**, the `0..1` scalar both branches gate on. A low value pushes
  toward absorbing, which reads like a shield or energy fraction, but nothing has
  been read that says so.
- **What writes `self+0x60`**, which is what decides the question above.
- **The two skill indices at `+0x34` and `+0x38`**, and what sets them - they are
  separately clamped, so absorbing and firing can be tuned to different
  difficulties.
- **Mode `0x12`**, which shares the Eliminator rate table. `DAT_08b31048` is 3 in
  a Single Race and 8 in Eliminator (both measured live); `2` gets a `2.0`
  multiplier and is unidentified.
