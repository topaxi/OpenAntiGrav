# Input bindings: the action table and the two control schemes

Opened to answer one question left standing by
[`engine.md`](engine.md#the-sideshift-is-a-force-and-its-direction-is-read-rather-than-guessed):
which button fires a sideshift. That section named `FUN_08836828` (a
control-scheme test) and `FUN_088366bc` (an action-id-to-button-id map) as
unread, and treated the answer as unavailable.

Both are three-line functions, and reading them opens the whole module. The
answer is that the original does not have *a* sideshift binding: it has **two
control schemes with different bindings and different trigger gestures**, and
which one is live is a profile setting.

Everything here is static analysis of `BOOT.BIN` (PSP, `SCUS`-side image
`pulse-psp-usa.chd`). Nothing on this page has a runtime leg yet; where that
matters it is said so.

## The action table

The game binds **eight abstract actions**, not buttons. Two parallel tables of
eight words each live in `.bss` at `0x08ab0c90` and `0x08ab0cb0`, `0x20` apart,
with a scheme byte immediately after them at `0x08ab0cd0`.

| Action | String the options page shows | Scheme |
| ---: | --- | --- |
| 0 | `OPT_CTRL_ACC` | both |
| 1 | `OPT_CTRL_FIRE` | both |
| 2 | `OPT_CTRL_ABS` | both |
| 3 | `OPT_CTRL_LBACK` | both |
| 4 | `OPT_CTRL_AIRBRAKES` | novice only |
| 5 | `OPT_CTRL_LAB` | veteran only |
| 6 | `OPT_CTRL_RAB` | veteran only |
| 7 | `OPT_CTRL_SS` | novice only |

**The names are read, not inferred.** `FUN_0889b468` (`0x0889b468`) is the
options page that rebuilds the control-scheme rows, and it pairs each action
index with its localisation key literally:

```c
if (Options_ControlSchemeFlag() == 0) {          // novice
    row(4, localise("OPT_CTRL_AIRBRAKES"));
    row(7, localise("OPT_CTRL_SS"));
} else {                                         // veteran / custom
    row(5, localise("OPT_CTRL_LAB"));
    row(6, localise("OPT_CTRL_RAB"));
}
row(0, localise("OPT_CTRL_ACC"));
row(1, localise("OPT_CTRL_FIRE"));
row(2, localise("OPT_CTRL_ABS"));
row(3, localise("OPT_CTRL_LBACK"));
```

`LAB`/`RAB` are the left and right airbrake, `ABS` is absorb, `LBACK` is look
back, and **`SS` is the sideshift** - which is what settles the question this
page was opened for. Every one of those ten strings was read out of the binary
(`0x08a7e850`..`0x08a7e8d8`); none is a guess at an abbreviation.

Note what the split says about the schemes, because it is the whole design:
**novice gives you one button for both airbrakes and a dedicated sideshift
button; veteran gives you the two airbrakes separately and no sideshift button
at all.** Veteran's sideshift has to come from a gesture, and it does - see
below.

Confidence **92**. One store per row, ten string literals, and the two branches
are mutually exclusive on the same flag the trigger code reads.

## The default mapping is `[5, 7, 4, 6, 9, 8, 9, 8]`

`Options_LoadDefaultControlMapping` (`0x0883672c`) writes seventeen words: the
eight-word novice table, the eight-word veteran table, and the scheme byte. The
values are immediates in its own body, so they are readable without running
anything:

```
0x08ab0c90 (novice) : 5  7  4  6  9  8  9  8
0x08ab0cb0 (veteran): 5  7  4  6  9  8  9  8
0x08ab0cd0 (scheme) : 1
```

Both halves are identical, which means the scheme flag does not change *what a
button does* - it changes *which actions exist* and *how the sideshift is
triggered*.

Against the button indices `Input_BuildState` (`0x0894eec4`) uses - see
[`input.md`](input.md), and `crates/gameplay/src/input.rs`'s `button` module,
which mirrors it - the table reads:

| Action | Id | Button |
| ---: | ---: | --- |
| 0 accelerate | 5 | `CROSS` |
| 1 fire | 7 | `SQUARE` |
| 2 absorb | 4 | `CIRCLE` |
| 3 look back | 6 | `TRIANGLE` |
| 4 both airbrakes | 9 | `R` |
| 5 left airbrake | 8 | `L` |
| 6 right airbrake | 9 | `R` |
| 7 sideshift | 8 | `L` |

Cross is thrust, which `crates/gameplay/src/controls.rs` already asserts on
other grounds, and it is the row that makes the whole table self-checking: a
mapping that put accelerate anywhere else would be wrong on a fact already
known.

**Every action a sideshift needs resolves to `L` or `R`**, both of which exist
on our abstract button layer. That is the fact that unblocks porting the
trigger; there is no PSP button here that our `InputSnapshot` cannot carry.

Confidence **90**. The immediates are in the function body; the button-id
column leans on `input.md`'s index list, which is itself at 90.

## The five functions

### `Options_ButtonForAction` (`0x088366bc`)

| | |
| --- | --- |
| **Address** | `0x088366bc` |
| **Confidence** | **90** |

```c
u32 Options_ButtonForAction(int action) {
    return Options_ControlSchemeFlag() == 0
        ? novice_table[action]      // 0x08ab0c90
        : veteran_table[action];    // 0x08ab0cb0
}
```

Callers use the result as `1 << id` against a button mask, which is what fixes
the return as a button *index* rather than a mask.

### `Options_ControlSchemeFlag` (`0x08836828`)

| | |
| --- | --- |
| **Address** | `0x08836828` |
| **Confidence** | **88** |

Returns the byte at `0x08ab0cd0`. `0` is novice, non-zero is veteran. It is a
getter and nothing else - worth saying, because `engine.md` carried it as an
unread "control-scheme test" that might have been arbitrarily complicated.

### `Options_SetControlScheme` (`0x08836714`)

| | |
| --- | --- |
| **Address** | `0x08836714` |
| **Confidence** | **88** |

`scheme_flag = (arg == 1)`. Called with `0` on the novice path and `1`
otherwise.

### `Options_LoadDefaultControlMapping` (`0x0883672c`)

| | |
| --- | --- |
| **Address** | `0x0883672c` |
| **Confidence** | **88** |

Writes the seventeen words above over both tables and the scheme byte. The
copy runs eight iterations of two words with the seventeenth stored after the
loop, which is why the scheme byte at `0x08ab0cd0` sits in the same blob and
gets set to `1` here as a side effect - the callers that want novice override
it immediately afterwards.

### `Options_LoadControlMapping` (`0x08836a48`)

| | |
| --- | --- |
| **Address** | `0x08836a48` |
| **Confidence** | **85** |

The loader. Reads two profile settings:

- **`ControlMapping2`**, `0x44` bytes - the saved custom mapping. If the
  setting is absent it is created and seeded with the default blob.
- **`Control_Type`**, a string, compared case-insensitively against `custom`
  and then `novice`.

```c
if (Control_Type == "custom")      copy ControlMapping2 over both tables
                                   (the 17th word lands on the scheme byte)
else if (Control_Type == "novice") load defaults; scheme = 0
else                               load defaults; scheme = 1
```

`FUN_0889b468` above compares against `veteran`, `novice` and `custom`, so
`veteran` is the third value and the `else` branch here is it.

**The shipped default is veteran**, at confidence **75** rather than 85: the
`else` branch sets `1`, and `Options_LoadDefaultControlMapping`'s own blob also
carries `1`, so two independent paths agree - but the value `Control_Type`
holds on a profile that has never visited the options page was not read, and a
settings-defaults table that presets it to `novice` would overturn this without
contradicting anything above. **Measure it before leaning on it**: break in
`Options_LoadControlMapping` under PPSSPP on a fresh profile and read
`0x08ab0cd0`.

## What each scheme does with the sideshift

Both branches live at the tail of `Ship_UpdateSideshiftInput_q` (`0x08846a54`)
and are gated together on two things: `craft+0x1c0 & 2` clear, and the
**lockout timer `entity+0x8ac` at or below zero**.

### Novice: hold `OPT_CTRL_SS` and flick the stick

Read at `0x088474xx`. `engine.md` already has this branch in full and it is
unchanged by this page: the action-7 button must be **held** (tested against
the mask at `*(craft+0x78) + 0x24`), an armed latch `entity+0x860 & 0x400` is
set whenever the steering axis is inside `+/-10`, and the axis then crossing
`> +10` fires right (`entity+0x8a8`) or `< -10` fires left (`entity+0x8a4`),
clearing the latch either way.

What this page adds is that the held button is **`L`**, and that the branch is
**novice-only** - so the gesture is: hold the left shoulder, flick the stick.

### Veteran: double-tap the airbrake, which is not a "repeat lockout"

Read at `0x088473xx`. `engine.md:1515-1517` describes this branch as "the
buttons bound to actions `5` and `6`, **with a repeat lockout**". That reading
is **wrong and is corrected here**: it is a *double-tap*, and the two floats it
touches are a tap window, not a lockout.

```c
u32 left  = Options_ButtonForAction(5);
u32 right = Options_ButtonForAction(6);

if (pressed(left) && entity+0x8a4 <= 0.0f) {
    if (entity+0x89c > 0.0f) entity+0x8a4 = 0.2f;   // second tap: fire left
    else                     entity+0x89c = 0.25f;  // first tap: open window
}
if (pressed(right) && entity+0x8a8 <= 0.0f) {
    if (entity+0x8a0 > 0.0f) entity+0x8a8 = 0.2f;   // second tap: fire right
    else                     entity+0x8a0 = 0.25f;
}
```

`0x3e800000` is `0.25` and `0x3e4ccccd` is `0.2`. `entity+0x89c` and
`entity+0x8a0` are the left and right **tap windows**; both are counted down by
`dt` earlier in the same function, alongside the two sideshift timers. So the
gesture is **double-tap the left airbrake within 0.25 s to shift left**, and
the same on the right - which is the manoeuvre veteran needs, given it has no
`OPT_CTRL_SS` button.

The mask read here is `*(craft+0x78) + 0x20`, not `+0x24`. The novice branch
tests `+0x24` for a *held* button and this one needs an *edge* or a held
airbrake would fire a shift every frame, so **`+0x20` is the pressed/edge mask
and `+0x24` is the held mask**. Confidence **80** on that pair - it is an
argument from what the two branches must mean rather than from the mask
producer, which was not re-read here.

### The lockout is real, and it is common to both

`entity+0x8ac` is set to `1.0` at the end of the function on any tick where
either sideshift timer is positive, is counted down by `dt`, and gates the
whole trigger block. **One second between sideshifts**, whichever scheme is
live. This is what `engine.md` was reaching for with "repeat lockout", but it
sits above both branches rather than inside the button one.

Confidence **85** for this section: one function, read end to end, with every
literal identified.

### Confirmed at runtime: the veteran double-tap, captured off a real PPSSPP session

Every claim in the veteran section above was static analysis until 2026-08-27,
when `verification/scenarios/sideshift-double-tap.inputs` was captured for the
first time off a real, booted `pulse-psp-usa.chd` (see
[`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md) for the
capture recipe) with the five entity floats above recorded per tick (added to
`scripts/psp_trace_fields.py`'s `ENTITY_FIELDS` in the same pass -
`ss_tap_window_l`/`_r` at `+0x89c`/`+0x8a0`, `ss_shift_l`/`_r` at
`+0x8a4`/`+0x8a8`, `ss_lockout` at `+0x8ac`). Against a committed control
(`sideshift-double-tap-control.inputs`, the same script with the two `l`
tokens dropped, which never touches any of the five columns over its own 285
ticks - a clean negative), the capture with the taps reads exactly what the
static read predicted, to the tick:

- `ss_tap_window_l` arms at `0.25` on the first tap (tick 181) and counts down
  by `dt` every tick after.
- `ss_shift_l` fires at `0.2` on the second tap (tick 185, three ticks later
  as scripted, well inside the `0.25 s`/15-tick window) and counts down the
  same way.
- `ss_lockout` sets to `1.0` on the same tick `ss_shift_l` fires - not on the
  first tap - and decays to zero over the following ~60 ticks, i.e. **1.0 s**
  at 60 Hz.
- `ss_tap_window_r`/`ss_shift_r` stay at `0` throughout, as they should for a
  gesture tapped left.

Confidence **95** for the three constants and which offset each belongs to -
independently confirmed at runtime, not just read out of the decompiler - with
the 5-point left/right assignment now doubly checked (static reading plus a
live capture that fires exactly the branch a left tap should).

**What this does not settle: the lateral-displacement claim.** The scripted
run-up (`180 cross`, no steering) grazes a wall on Talon's Junction around
tick 60-90 - the same collision the standing-start capture already documents
(`ppsspp-debugger.md`'s reference-scenario section) - and the capture and its
control, despite an identical script up to tick 180, are already `1.17` units
apart in position by the time the first tap fires (tick 181), against `0.035`
units apart at tick 0. A wall contact amplifies the residual pose spread
between any two captures (documented at `~0.03` units / `1-2°` even from a
pinned `--start-heading` start), so a large chunk of the runs' eventual `42`
units of separation at tick 284 is that amplification, not the sideshift. The
static reading's "11.7 units of leftward displacement" figure came from
`oag-trace drive` runs through our own simulation, which does not hit this
wall on the same script (`travelled 427.986` unit(s), never below `grounded`)
- so it was never itself a runtime measurement of the original, and this
capture does not supply one either. A clean displacement comparison needs a
run-up that stays off the wall, which this scenario's `180 cross` does not do
on the real hardware; the timer columns above are the runtime leg that does
not depend on getting that right.

## The tap-history path is the barrel roll

`engine.md` closes its sideshift section by recording a third path it
deliberately did not read - "a three-entry tap history
(`entity+0x88c`/`+0x890`/`+0x894`) fed by d-pad bits and by the axis crossing
`+/-90`, matching the patterns `2,1,2` and `1,2,1`" - and says it "is **not**
read far enough to say what it is". It is read far enough now, and it is the
**barrel roll**.

The chain, from the same function:

1. **The history records a direction, not a button.** `1` is written when the
   `LEFT` d-pad bit is pressed *or* the steering axis crosses below `-90`; `2`
   when `RIGHT` is pressed or the axis crosses above `+90`. Each entry shifts
   the previous two down, but **only if `entity+0x884` is below `0.6`** -
   `+0x884` accumulates `dt` and is zeroed on every entry, so it is a
   **0.6 s inter-tap timeout** and the history is a gesture buffer.
2. **The patterns are the two three-tap alternations.** `2,1,2` is
   right-left-right and `1,2,1` is left-right-left.
3. **It costs shield energy.** On a match the code calls `FUN_08840770(entity)`
   for a cost and `Ship_Shield` (`0x0883e68c`) for the current shield, and
   **arms nothing at all unless `cost < shield`**; when it does arm it charges
   the cost with `Ship_SetShield` (`0x0883e6f4`). A manoeuvre you pay energy
   for, and cannot perform on a low shield.
4. **What it arms is a signed, self-completing roll phase.** `2,1,2` sets
   `entity+0x860 & 0x100` and `1,2,1` sets `& 0x80`; those two flags ramp
   `entity+0x87c` toward `+1.0` and `-1.0` respectively. When they clear,
   `+0x87c` does not reverse - it **continues to the nearer end**, `0.5` being
   the split: past half way it runs on to `1.0`, short of it, it falls back to
   `0.0`. That is an animation phase that finishes what it started, not a held
   axis.
5. **It only pays out if it is complete on landing.** The head of the function
   fires on the *airborne-to-grounded* transition, and only when
   `|entity+0x87c| > 0.5`: it sets a timer `entity+0x898`, plays a cue, and
   increments a counter at `settings+0x238`. While `+0x898` is positive the
   craft flag `craft+0x1c0 |= 0x400` is held.

Energy cost, a three-tap left-right-left alternation, a roll phase that
completes rather than reverses, and a reward paid only if the roll finished
before the craft touched down. Confidence **80**: every link is read at
instruction level, and the reading has no runtime leg and no measurement of
what `+0x87c` drives visually - `DAT_08b36bf0`, the ramp rate, is `.bss` and
could not be read statically.

**This is not implemented and is not in scope here.** It is recorded because
`engine.md` explicitly parked it as unknown, and because it is a headline
mechanic the reimplementation does not have. Two things a follow-up would need:
`FUN_08840770`'s cost (unread), and what consumes `craft+0x1c0 & 0x400` -
presumably the ship's visual roll, which would make `+0x87c` an angle rather
than a phase.

## Applied renames

| Address | Name | Conf |
| --- | --- | ---: |
| `0x088366bc` | `Options_ButtonForAction` | 90 |
| `0x08836828` | `Options_ControlSchemeFlag` | 88 |
| `0x08836714` | `Options_SetControlScheme` | 88 |
| `0x0883672c` | `Options_LoadDefaultControlMapping` | 88 |
| `0x08836a48` | `Options_LoadControlMapping` | 85 |
| `0x0889b468` | `Options_BuildControlSchemeRows` | 82 |

`FUN_088367d8` (the custom-mapping writer) and `FUN_08840770` (the barrel
roll's energy cost) are left unnamed: both were seen only through a caller.

## For reimplementation

- The sideshift trigger is **two** gestures, not one, and which is live is a
  setting. A port that implements only one should say which.
- Novice: hold `L`, flick the stick past `+/-10` on the `+/-100` axis, having
  been inside `+/-10` first.
- Veteran: double-tap `L` or `R` within `0.25 s`.
- Either way, `0.2 s` of force (already ported, see
  `crates/physics/src/airbrake.rs`) and then `1.0 s` before another will fire.
- The armed latch, the two tap windows and the lockout are all **per-craft
  state in the original**, so they belong in `oag_physics::ship::ShipState`
  beside `sideshift_timers`, not in the input layer. `InputSnapshot` needs
  nothing new: `L` and `R` are already on it.

## Cross-platform

Not checked against `SCES_547.48` (PS2). The PS2 pad has no direct `L`/`R`
equivalent to the PSP's single shoulder pair, so the action table is expected
to differ; the scheme *split* may not.
