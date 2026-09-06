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
whole trigger block. This is what `engine.md` was reaching for with "repeat
lockout", but it sits above both branches rather than inside the button one.

**Corrected by the runtime capture below: the gap between sideshifts is closer
to 1.2 s than 1.0 s.** `entity+0x8ac` being *set* to `1.0` every tick the shift
timer runs, rather than once at the trigger, means the lockout does not start
counting down until the ~0.2 s shift force itself has expired - so the two
timers add rather than overlap. A previous pass of this page read "set to
`1.0`" as "one second between sideshifts" and that is the part the capture
corrects, not the mechanism itself.

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
  first tap - and **stays pinned at exactly `1.0` for as long as `ss_shift_l`
  is positive** (ticks 185-196, the same 12 ticks the shift force runs), only
  starting to count down by `dt` once `ss_shift_l` goes non-positive at tick
  197. It reaches zero around tick 256. So the lockout's own `1.0 s` is real,
  but it is not the whole gap: the shift force's ~`0.2 s` runs *first*, with
  the lockout re-armed to `1.0` on every one of those ticks rather than
  counting down alongside it, so the two add rather than overlap - **~71
  ticks, ~1.2 s, from the fire to the next sideshift being legal**, not the
  `1.0 s` a flat reading of "set to `1.0`" suggests.
- `ss_tap_window_r`/`ss_shift_r` stay at `0` throughout, confirming a left tap
  never touches the right pair - not that the right pair themselves fire at
  `0.25`/`0.2` on a right tap, which this capture never sent one to check.

Confidence **95** for the three constants (`0.25`, `0.2`, `1.0`) and the
left-hand offsets they were read off (`0x89c`/`0x8a4`/`0x8ac`) - independently
confirmed at runtime, not just read out of the decompiler. Confidence **85**,
unchanged, for the right-hand offsets (`0x8a0`/`0x8a8`) and for the *shape* of
the lockout's 1.2 s total: read correctly from the code, confirmed not to
fire on a left tap, but the right pair's own values and a repeat of this
capture are what would raise it.

### The countdown does not clamp at zero, and the port's does - deliberately left that way

The capture also shows two of the five columns freezing at a tiny **negative**
value instead of `0.0` once they expire: `ss_shift_l` at `-0.000149006` from
tick 197 onward, `ss_tap_window_l` at `-0.0004569963` from tick 196 onward,
each held bit-exact for the rest of the run. A clamped countdown cannot
produce that at all, let alone two different residues.

Read directly from `Ship_UpdateSideshiftInput_q`'s decompile (`0x08846a54`),
the countdown block is

```c
if (0.0 < fVar6)  { *(float *)(param_2 + 0x89c) = fVar6  - param_1; }  // tap window, left
if (0.0 < fVar9)  { *(float *)(param_2 + 0x8a0) = fVar9  - param_1; }  // tap window, right
if (0.0 < fVar8)  { *(float *)(param_2 + 0x8a4) = fVar8  - param_1; }  // shift force, left
if (0.0 < fVar10) { *(float *)(param_2 + 0x8a8) = fVar10 - param_1; }  // shift force, right
if (0.0 < fVar11) { *(float *)(param_2 + 0x8ac) = fVar11 - param_1; }  // lockout
```

`if (timer > 0.0f) timer -= dt;` - gated, not clamped. The tick that crosses
zero still subtracts a full `dt`, landing slightly negative, and every tick
after that the gate itself is false, so the field is never touched again.
That is exactly what the capture shows. Confidence **95**: the instruction
was read directly, not inferred from the data (the data only pointed at the
hypothesis first).

`crates/physics/src/airbrake.rs`'s `advance_sideshift` instead does
`(*timer - dt).max(0.0)` on all five, unconditionally, every tick - it
converges to exactly `0.0` rather than freezing on a residue. **Deliberately
not changed to match**, for two reasons: every consumer of these fields in
the whole tree, original and port alike, gates on `> 0.0`/`<= 0.0`, which a
residue of `-1.5e-4` and a clamped `0.0` answer identically - there is no
behaviour this affects. And `ShipState::hash_state`
(`crates/physics/src/probe.rs`) hashes `sideshift_timers`, `shift_tap_windows`
and `shift_lockout` into committed golden-test hashes; matching the residue
would change those constants for a purely cosmetic difference, which is a
real cost for zero behavioural gain. If a bit-exact comparison of these
columns against a real capture is ever built, this paragraph is why a `-1.5e-4`
reading is not a bug.

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

### A shorter run-up avoids the graze, but finds a different confound

2026-08-30, `verification/scenarios/sideshift-double-tap-clean.inputs` (and its
`-control`), same reference scenario, run-up cut from `180 cross` to `130
cross` so the first tap fires well clear of the documented 60-90 graze window.
It works, in the narrow sense that neither trace shows a differing
lateral-velocity spike before the tap - both share one common jolt (`~6.6-6.75`
units/s on the right-axis column, tick 108 in the capture and tick 103 in the
control, five ticks apart but otherwise near-identical) that reads as a shared
track feature, not a divergent wall hit.

**What it finds instead: the "straight" run-up is not straight relative to the
track, and the control is not a clean do-nothing baseline past about tick
150.** Both traces drift at a steady, near-identical `~-4.5` to `-5` units/s on
the `right_x/y/z` columns from as early as tick 125 onward, with zero steering
input - this is `craft+0x170`, which the field name says `right` but the basis
section above says is row 0, **the ship's own left axis**, so a negative
reading here is rightward drift. It is present in both runs before the tap and
is too large and too repeatable between two independently-booted captures to
be pose noise; it reads as the track's own camber on this stretch, not
measurement error.

Left running, that drift is exactly what puts the **control** into a wall:
from tick ~148 its `pos_z` freezes (`-203.77` at tick 148, `-203.13` at tick
232 - 0.6 units of lateral motion over 84 ticks) while `pos_x` keeps climbing
normally, and speed decays from 41 toward 26 and still falling - the same
grind-along-a-wall signature `ppsspp-debugger.md`'s reference-scenario section
already documents for this exact configuration, settling toward the ~21-22
units/s regime described there. The **capture** does not: `pos_z` keeps moving
freely through the same window (`-204.02` to `-196.12`) and speed climbs
cleanly past 100 units/s by tick 232. The sideshift's own force window
(`ss_shift_l` positive, ticks 135-148) is where the two trajectories part:
the capture's right-axis velocity swings from `-4.55` before the tap up
through `+8.92` mid-force and settles near `0` to `+0.6`, while the control's
carries on decaying smoothly through `-3.9` to `-2.3` over the same span. The
sideshift is what reverses the shared drift enough to clear the wall the
control rides into.

**So this run confirms the sideshift is a real, signed, mid-run velocity
event of the right sign (leftward, matching a left tap) and roughly the right
duration** - but it still does not produce a clean displacement figure in the
original's units. Past ~tick 148 the two runs are not laterally-offset copies
of the same trajectory, they are in different physical regimes (free versus
wall-pinned), so a position difference at any later tick mixes "the
sideshift's own kick" with "which run happened to dodge the wall" and cannot
be read as one number. The one window where both runs are still free (tick
135-148, 13 ticks) is too short for the position delta to clear the pre-existing
phase noise (a few tenths of a unit either way). Confidence **80** for the
directional/velocity finding (independently measured, signed correctly against
the decompiled force's polarity); the absolute "11.7 units" figure remains
unconfirmed against real hardware, now for a different and better-understood
reason than the original capture's outright wall graze.

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
before the craft touched down. Confidence **80** when first written, because
every link was read at instruction level but the reading had no runtime leg and
no measurement of what `+0x87c` drives visually - `DAT_08b36bf0`, the ramp rate,
is `.bss` and was believed unreadable statically. **The subsection below raises
this to 90 and supersedes that last clause**: the global is `.bss`, but its
writer is not, and every number turned out to be authored XML on the disc.

**This is still not implemented.** It is recorded because `engine.md`
explicitly parked it as unknown, and because it is a headline mechanic the
reimplementation does not have.

### The cost, the ramp rate and the roll's effect are read, and every number is authored

Corrects the paragraph above: none of the three needed a live read, and the
guess that `craft+0x1c0 & 0x400` drives the *visual* roll is **wrong**.

**All four barrel-roll tunables are `<Global><Special>` XML attributes.**
`Xml_ReadGlobalSettings` (`0x0883a970`) parses `Data\XML\HandlingStats.xml` and
stores five floats into one contiguous `.bss` run, each keyed by an attribute
name in `.rodata`:

| Attribute (string address) | Global | Name |
| --- | --- | --- |
| `turbo_jump` (`0x08a7b3cc`) | `0x08b36be8` | `g_turbo_jump` |
| `speedpad_jump` (`0x08a7b3d8`) | `0x08b36bec` | `g_speedpad_jump` |
| `roll_speed` (`0x08a7b3e8`) | `0x08b36bf0` | `g_roll_speed` |
| `roll_cost` (`0x08a7b3f4`) | `0x08b36bf4` | `g_roll_cost` |
| `roll_turbotime` (`0x08a7b400`) | `0x08b36bf8` | `g_roll_turbotime` |

Both shipped PSP pressings author the same line, read with
`oag-wad cat <image>:PSP_GAME/USRDIR/Data.wad 'Data\XML\HandlingStats.xml'`:

```xml
<Special roll_cost="8" roll_speed="1.5" roll_turbotime="0.5"
         speedpad_jump="0.1" turbo_jump="0.1"/>
```

`speedpad_jump="0.1"` is the control. That value was already recorded on
[engine.md](engine.md) from an independent live read of `0x08b36bec`, so the
same line's other four attributes come from a source this project has already
validated, and `g_speedpad_jump`'s address confirms the `.bss` run's alignment.

**`Ship_BarrelRollCost` (`0x08840770`) returns 8% of the ship's shield
capacity.** Ten instructions, a leaf:

```
lw    a0, 0x94(a0)             ; a0 = craft->0x94
lw    a0, 0x6c(a0)             ; a0 = [craft->0x94 + 0x6c], the stats base
lw    a1, 0xb4(a1)             ; a1 = g_skill_level (0x08b31044)
sll   a1, a1, 0x2
addu  a0, a0, a1
lui   a1, 0x3c23 / ori 0xd70a  ; 0x3c23d70a is 0.01f exactly
lwc1  f12, [g_roll_cost]       ; 8
lwc1  f0,  0x84(a0)            ; stats_base + 0x84 + g_skill_level*4
mul.s f12, f12, 0.01
jr    ra
mul.s f0,  f12, f0
```

`roll_cost` is a **percentage** - the `0.01` literal is what makes it one - and
`stats_base + 0x84 + index*4` is `<Misc>`'s three difficulty slots
(`easyshield`/`mediumshield`/`hardshield`, offsets `0x84`/`0x88`/`0x8c` per
[handling-stats.md](../../../formats/handling-stats.md)).

The index needs no separate identification, because
**`Ship_SetShield` (`0x0883e6f4`) computes the identical expression** as the
ceiling it clamps every shield write to:

```c
fVar3 = *(float *)(*(int *)(*(int *)(param_2 + 0x94) + 0x6c)
                   + _DAT_000578ac * 4 + 0x84);
if (param_1 <= fVar3) { fVar3 = param_1; }
```

Same chain, same `+0x84 + index*4`, same global - `_DAT_000578ac` is the naive
form of `0x08b31044`. So whatever the index selects, the roll costs
**`roll_cost`% of exactly the number that bounds the shield pool**, which is
`oag_physics::params::Dimensions::shield` in this project (already resolved for
the race's skill level by `oag_formats::handling::Misc::shield_for`). The same
function's HUD call divides by the same expression to get a percentage.

`g_skill_level` (`0x08b31044`) is `0x08b30f90 + 0xb4`, one slot along from the
speed-class index at `+0xb0` and the race mode at `+0xb8` that
[pads.md](pads.md) places in the same global block. It was **already named**, on
[shield.md](shield.md) at confidence 84, and this pass reached the same name
from the other end without knowing that: written by `Race_ReadSetupOptions`
(`0x08896b84`) at `0x088970cc`/`0x088970d4`, read at 25 sites,
`Ship_SetShield` among them. Two independent derivations agreeing is worth
recording; the row stays on `shield.md`.

**`craft+0x1c0 & 0x400` is a handling modifier, not a visual one.** A complete
instruction-level scan of the whole `.text` for `lw rX, 0x1c0(rY)` followed
within six instructions by `andi rX, rX, 0x400` returns **exactly three** sites,
and all three are ship dynamics:

| Site | Function | What the bit does |
| --- | --- | --- |
| `0x08848ba8` -> `0x08848bb4` | `Ship_ApplyLateralGrip` (`0x08848b78`) | both grip terms, `stats+0x10` and `stats+0x14`, are multiplied by **1.5** |
| `0x0884a708` -> `0x0884a70c` | `Ship_HoverTwoPoint` (`0x0884a658`) | a hover scalar is forced from `stats+0x4` to a literal **1.0** |
| `0x0884c8b0` -> `0x0884c8b4` | `Ship_UpdateEngine` (`0x0884c5c8`) | enables the uncapped turbo add |

The scan is the searchable form because `get_xrefs_to` cannot help here at all -
see [workflow.md](../../workflow.md). `scripts/psp-relocate.py masked 0x1c0 0x400`
reproduces the three; `andi 0x400` alone returns 151 sites.

The hover reading is from the delay slots: `f20` is set to `0x3f800000` (`1.0f`)
at `0x0884a6bc`, the default `f28` is `lwc1 f28, 0x4(a0)` at `0x0884a704`, and
`0x0884a720` does `mov.s f28, f20` only on the bit's arm. The same override
already applies when `craft+0x2a4 == 0`.

`Ship_UpdateEngine`'s use was already recorded from the other direction in
`crates/physics/src/ship.rs` at confidence 75, as one of two bits (`0x200` or
`0x400`) that turn the same term on. `roll_turbotime="0.5"` now supplies that
arm's duration.

So a barrel roll is **four** effects, not one: it costs 8% of shield up front,
gives 50% more lateral grip and a neutralised hover scalar while the phase
runs, and pays a 0.5 s turbo on landing if `|craft+0x87c| > 0.5`. And the ramp
rate being `roll_speed = 1.5` means `+0x87c` reaches `1.0` in **0.667 s**, with
the half-way split the section above describes crossed at 0.333 s.

Confidence **90** overall, up from 80: every value is authored data read off
both pressings rather than a measurement, the cost expression is shared verbatim
with an already-named function, and the `0x400` consumer list is a complete scan
rather than an xref that could have missed one.

Nothing in the paragraphs above says what *draws* the roll; the `0x400` bit
simply is not how it gets there. **The subsection below answers that**: `+0x87c`
is eased into a second field and that field rotates the ship's own display
matrix about its nose.

### The roll is drawn: `+0x87c` eases into `+0x880`, which rolls the ship about its nose

Searched 2026-09-06, and the answer is **yes, there is a render consumer** - two
of them, both reading one intermediate. Confidence **88**.

**Nothing here used `get_xrefs_to`, and an empty result from it would have proved
nothing.** Ghidra applies no relocation to any PSP database (see
[workflow.md](../../workflow.md)), so the searchable form is the same
whole-`.text` operand scan the `craft+0x1c0 & 0x400` section above used:
`scripts/psp-relocate.py field 0x87c` and `field 0x880`, plus a second sweep for
the encodings `field` cannot see - `addiu rD, rBase, <off>` followed by an
indirect access, and the VFPU `lv.s`/`sv.s`/`lv.q`/`sv.q` forms. Both sweeps
together are what makes the reader sets below complete rather than merely
unrefuted.

#### Stage 1: the ease, `0x08841eb8` - `0x08841f88`

`FUN_088418e0` reads `craft+0x87c` eight times in one branch nest and stores the
result to **`craft+0x880`**. The constants are all `lui` immediates at the site
(`0x4000` = `2.0`, `0xbf00` = `-0.5`, `0xc000` = `-2.0`, `0xbf80` = `-1.0`, with
`f20` = `0.5`, `f22` = `1.0`, `f26` = `0.0` live across the block):

```c
p = craft[0x87c];
if      (p == 0.0f)   e = 0.0f;
else if (p >  0.0f)   e = (p <= 0.5f) ?  2.0f*p*p         : 1.0f - 2.0f*(1.0f-p)*(1.0f-p);
else                  e = (p >= -0.5f) ? -2.0f*p*p        : -1.0f + 2.0f*(-1.0f-p)*(-1.0f-p);
craft[0x880] = e;                       /* swc1 f12, 0x880(s2) at 0x08841f88 */
```

That is the symmetric quadratic ease-in-out, odd in `p` (`e(-p) == -e(p)`),
continuous at `+/-0.5` where it takes `+/-0.5`. So the *phase* ramps linearly at
`roll_speed` and the *picture* accelerates out of level and decelerates back into
it - the roll does not start or stop with a jerk.

`craft+0x880` has **exactly one writer and exactly two readers** in the whole
binary. The `field` scan returns 14 sites at that offset; nine are stack slots
(`r29` base), two are a byte access (`sb`/`lbu`) on an unrelated structure and two
are `lw` on another; the three float accesses on a craft base are the write above
and the two reads below. The `addiu`/VFPU sweep returns 29 further sites, 22 of
them `sp`-relative VFPU spills; **the exclusion criterion for the remaining seven
is the identity of the containing function, not its base register's provenance** -
all seven sit outside the craft-update code cluster (`0x0883e...`-`0x0884d...`),
in functions that handle no craft, so none of them adds a consumer. `field 0x87c`
is on the stronger footing: its two sweep hits are one `sp`-relative and one in
`FUN_088ed9ec`, on an unrelated structure.

#### Stage 2a: the ship's own display matrix, `0x08842140` - `0x08842264`

Same function, ~90 instructions later:

```c
angle = craft[0x854] * 0.5f + craft[0x880] * 6.28f;      /* 0x40c8f5c3 */
/* wrap to (-pi, pi]: *2^24/pi (0x4aa2f983), trunc.w.s, (x << 7) >> 7, *pi/2^24 (0x34490fdb) */
c = cos(angle); s = sin(angle);   /* vcst.s 2/PI, vmul.s, vcos.s / vsin.s */

R = [ c  s  0  0 ]      /* up to transpose - see below */
    [-s  c  0  0 ]
    [ 0  0  1  0 ]
    [ 0  0  0  1 ]

M  = vmmul.q(R, *(craft + 0x794));        /* the rigid body's 4x4 */
M' = vmscl.q(M, [0x002ace1c]);            /* a scalar applied to rows 0..2 only */
dst = FUN_08945220(craft[0x8b0]);         /* the node's matrix */
dst[0..2] = M'[0..2];  dst[3] = M[3];     /* translation row taken unscaled */
FUN_08945284(craft, dst, 0);
```

**The rotated-about axis is index 2 and that is *not* subject to the transpose
ambiguity.** The two literal `1.0`s in `R` are written by `vone.s S122` and
`vone.s S133` - both on the diagonal, and a diagonal element is the same under
either row-major or column-major reading of the VFPU register names. Only the
*sign* of the `s`/`-s` pair flips with the reading, so which axis is held fixed is
determined and which way the ship rolls is not.

Index 2 is the nose. [camera.md](camera.md) establishes "`row2` is the ship's
forward axis, positive toward the nose" at confidence 92, measured on the running
game across three views, and lists `craft+0x374` as the pointer to the body's
forward row. `FUN_088418e0`'s own first ten instructions agree independently:

```c
body = craft[0x794];
craft[0x37c] = body;          /* side    <- body + 0x00 */
craft[0x378] = body + 0x10;   /* up      <- body + 0x10 */
craft[0x374] = body + 0x20;   /* forward <- body + 0x20 */
```

So a completed roll turns the drawn ship through `6.28` radians about its own
nose - a full revolution to within `0.16` degrees, `2*pi` being `6.28319`. The
`6.28` is a code literal, not authored data.

**`craft+0x854 * 0.5` is a second, independent contribution to the same angle**
and is *not* part of the barrel roll. It is the steering lean computed by
`FUN_0883fab4`, which [camera.md](camera.md) scores at confidence 0 for what it
physically means. Nothing in a reimplementation should transcribe it; the roll
term stands alone.

#### Stage 2b: the internal camera's up vector

`0x08845d7c`, in `FUN_088455ec`, the internal rig's update - already on
[camera.md](camera.md) as `up = Rot(craft[0x880] * 6.28) * up`, "a roll about the
view axis", recorded there before anything connected `craft+0x880` to the barrel
roll. Two consumers reached independently, reading one field at one scale, is the
corroboration that carries most of this section's confidence.

#### That the drawing code runs is proven, not assumed

`psp-relocate.py callers 0x088418e0` returns **zero** `jal` sites, which on its
own would leave this in the same position as
[ps2-pulse-eu/camera.md](../ps2-pulse-eu/camera.md)'s dead vtable. It is not:
`xrefs 0x088418e0` finds one reference, a **data** word at `0x08aca104`, inside a
`{this-adjustment, fn-pointer}` vtable. Its neighbouring slots, read through the
same relocation replay, are `0x089444d0`, `0x0894479c`, `0x089447a4`,
**`0x088418e0`**, `0x089447f4`, `0x0883e418`, `0x0894484c`, `0x0894485c`,
`0x08944864`, `0x089449a4`, `0x08944a30` - a live class in which most slots hold a
base implementation from the `0x08944xxx` run and two, this one among them, are
overrides. `FUN_088418e0` is reached through that pointer, which is why no `jal`
names it.

**That the vtable is installed, and that this slot is the one that gets called,
are both established from the binary alone.** The table's base is loaded by a
`lui`/`addiu` pair at two sites and **stored into `object+0x38`** at both -
`sw a0, 0x38(s0)` at `0x0883d6a0` and `sw a1, 0x38(s0)` at `0x08840d80`, the
constructor pattern. And `FUN_088418e0` **itself** dispatches through the same
slot, on a sub-object, at `0x088427cc`-`0x088427f4`:

```
lw    a0, 0x78(s2)        ; child = craft->0x78, skipped if null
lwc1  f12, 0x344(sp)      ; dt
lw    a1, 0x38(a0)        ; vtbl = child->0x38
addiu a1, a1, 0x20
lh    a2, 0x0(a1)         ; this-adjustment,  vtbl + 0x20
lw    a1, 0x4(a1)         ; function pointer, vtbl + 0x24
jalr  a1
addu  a0, a0, a2          ; this += adjustment
```

`0x08aca0e0 + 0x24` is `0x08aca104`, this function's own slot, and the
`{lh adjust, lw fnptr}` pair at `+0x20`/`+0x24` is exactly the 8-byte entry layout
the table dump shows. So slot `+0x24` is a virtual per-frame update taking `dt` in
`f12`, `FUN_088418e0` is a class's override of it, and the function's own
signature (`float, int`) matches the call it makes. Nothing about that argument
routes through another page.

A weaker second line of evidence exists and is recorded with its own caveat rather
than leaned on: `FUN_088455ec` has exactly one caller in the binary - the `jal` at
`0x08841b6c`, inside `FUN_088418e0` - and [camera.md](camera.md) settled
`fov = 60 + 0.075 * dot(fwd, vel)` live at confidence 94, a law whose arithmetic
matches `FUN_088455ec`'s `craft+0x790` store. **That is not proof**: camera.md's
own correction section leaves open whether the internal rig's update runs while an
external view is selected, and `g_camera_fov_degrees` (`0x08b34310`) is a global
that page treats as distinct from `craft+0x790`. The vtable argument above does
not depend on either point.

#### The other writers of `+0x87c`, for completeness

Besides `Ship_UpdateSideshiftInput_q`'s ramp, three sites write the phase and none
of them reads it:

| Site | What it does |
| --- | --- |
| `0x0883d5b4` | a six-instruction leaf: `table[i]->0x87c = 0.0f` and nothing else |
| `0x088442e8` | clears `+0x860 & 0x100`, stores `+0.0` to `+0x87c`, zeroes the tap history `+0x88c`/`+0x890` |
| `0x08847a54` | clears `+0x860 & 0x100`, stores `f20` to `+0x87c`, zeroes `+0x88c`/`+0x890` |

All three are roll-cancel paths. `f20`'s value at `0x08847a54` was not traced.

#### Deliberately not renamed

`FUN_088418e0` stays `FUN_`. It advances per-craft timers, runs contact reaction
over the body's contact array, updates the internal camera rig **and** builds and
submits the display matrix; no `Subsystem_VerbNoun` covers that, and camera.md
already declined to name it for the same reason. Naming it would be a guess that
stops the next reader looking.

#### What is not determined

- **The sign of the roll.** The `s`/`-s` placement is the one thing the transpose
  ambiguity does reach, so whether a `1,2,1` gesture rolls the ship left or right
  on screen is not settled by this static read. A capture settles it in one run.
- **`vmmul.q`'s operand order.** Whether the composed matrix is `R * body` or
  `body * R` was not established, and the two differ when the body matrix is not
  orthonormal.
- **`[0x002ace1c]`**, the scalar `vmscl.q` applies to rows 0-2. Unidentified;
  camera.md's own `g_craft_scale` discussion is the place to look first.



| Address | Name | Conf |
| --- | --- | ---: |
| `0x088366bc` | `Options_ButtonForAction` | 90 |
| `0x08836828` | `Options_ControlSchemeFlag` | 88 |
| `0x08836714` | `Options_SetControlScheme` | 88 |
| `0x0883672c` | `Options_LoadDefaultControlMapping` | 88 |
| `0x08836a48` | `Options_LoadControlMapping` | 85 |
| `0x0889b468` | `Options_BuildControlSchemeRows` | 82 |
| `0x08840770` | `Ship_BarrelRollCost` | 90 |
| `0x08b36be8` | `g_turbo_jump` | 82 |
| `0x08b36bf0` | `g_roll_speed` | 90 |
| `0x08b36bf4` | `g_roll_cost` | 90 |
| `0x08b36bf8` | `g_roll_turbotime` | 88 |
| `0x08b31044` | `g_skill_level` | 85 |

`FUN_088367d8` (the custom-mapping writer) is left unnamed: it was seen only
through a caller.

`g_turbo_jump` is 82 rather than 90 because only its *name* is read - the
attribute-to-global mapping in `Xml_ReadGlobalSettings` is as direct as the
other four's, but unlike them nothing has been traced that reads it, so what
`turbo_jump` does is unknown. `g_roll_turbotime` is 88 for a milder version of
the same: its consumer is `Ship_UpdateEngine`'s `0x400` arm by elimination
rather than by a read of the store into `craft+0x898`.

## For reimplementation

- The sideshift trigger is **two** gestures, not one, and which is live is a
  setting. A port that implements only one should say which.
- Novice: hold `L`, flick the stick past `+/-10` on the `+/-100` axis, having
  been inside `+/-10` first.
- Veteran: double-tap `L` or `R` within `0.25 s`.
- Either way, `0.2 s` of force, then a `1.0 s` lockout that does not start
  counting down until the force expires - **~1.2 s total from one sideshift
  firing to the next being legal**, confirmed at runtime above. Already ported
  correctly: `crates/physics/src/airbrake.rs`'s `advance_sideshift` re-arms
  `shift_lockout` to `SIDESHIFT_LOCKOUT` on every tick either
  `sideshift_timers` entry is positive rather than once at the trigger, and
  `airbrake/tests.rs` already asserts the lockout clears at
  `(SIDESHIFT_DURATION + SIDESHIFT_LOCKOUT) / DT` ticks - the additive total
  this capture now confirms against the original rather than only the port's
  own reading of the decompiler.
- The armed latch, the two tap windows and the lockout are all **per-craft
  state in the original**, so they belong in `oag_physics::ship::ShipState`
  beside `sideshift_timers`, not in the input layer. `InputSnapshot` needs
  nothing new: `L` and `R` are already on it.

## Cross-platform

Not checked against `SCES_547.48` (PS2). The PS2 pad has no direct `L`/`R`
equivalent to the PSP's single shoulder pair, so the action table is expected
to differ; the scheme *split* may not.
