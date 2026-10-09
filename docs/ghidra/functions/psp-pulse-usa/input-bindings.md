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
**novice gives you one airbrake button that picks its side off the steering
(both when the stick is centred; see
[below](#novice-r-picks-its-airbrake-off-the-steering)) and a dedicated
sideshift button; veteran gives you the two airbrakes separately and no
sideshift button at all.** "One button for both airbrakes", which this page
said until 2026-10-07, was read off the label alone and is wrong. Veteran's sideshift has to come from a gesture, and it does - see
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
| 4 airbrake, side by steering | 9 | `R` |
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
        ? novice_table[action]      // 0x08ab0c90: veteran_table[action];    // 0x08ab0cb0
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

## Novice: `R` picks its airbrake off the steering

Recovered 2026-10-07. The label `OPT_CTRL_AIRBRAKES` says nothing about what
the button does to the craft, and the port had read it as "both airbrakes".
The runtime effect lives in `PlayerInput_Update` (`0x0883c870`), the
`player_input` record's per-frame fill (record layout on
[cannon-quake-leachbeam.md](cannon-quake-leachbeam.md): record `+0x00` steer,
`+0x08` left airbrake, `+0x0c` right airbrake, `controller+0x44` onwards).
Its scheme branch, at the call to `Options_ControlSchemeFlag` (`0x0883cb6c`)
and `Options_ButtonForAction(4)` (`0x0883cb80`):

```c
if (Options_ControlSchemeFlag() == 0) {            // novice
    rec->airbrake_left  = 0;                        // +0x4c, every frame
    rec->airbrake_right = 0;                        // +0x50, every frame
    if (Input_IsHeld(g_input, Options_ButtonForAction(4), 0)) {
        if      (rec->steer < -10.0f) rec->airbrake_left  = 100.0f;
        else if (rec->steer >  10.0f) rec->airbrake_right = 100.0f;
        else { rec->airbrake_left = 100.0f; rec->airbrake_right = 100.0f; }
    }
} else {                                            // veteran
    rec->airbrake_left  = Input_IsHeld(.., Options_ButtonForAction(5)) ? 100 : 0;
    rec->airbrake_right = Input_IsHeld(.., Options_ButtonForAction(6)) ? 100 : 0;
}
```

`rec->steer` is the value already written earlier in the same call, so the
test sees the shaped steer, not the raw stick:

1. `x = Input_GetAxis(g_input, 0)`, on `-1..=1`.
2. Inside the deadzone, `|x| < 0.2`, steer is `0`. The deadzone is the float
   at `0x08a7b698`, bytes `cdcc4c3e` = `0x3e4ccccd` = **`0.2`** (not the
   `0.25` the record table on `cannon-quake-leachbeam.md` carried; corrected
   there).
3. Outside it, `v = (|x| - 0.2) * g`, with `g = 100 / (1 - 0.2) = 125`,
   computed once into `0x08ae4cec`. Named `g_stick_deadzone` (`0x08a7b698`)
   and `g_stick_gain` (`0x08ae4cec`), confidence 92 and 90: both read live in
   the capture below, and the gain is also derived in this function's own
   first block from the deadzone.
4. Response curve: `v < 50` gives `v / 2`, otherwise `1.5 v - 50`; signed by
   `x`. So `+/-100` at full lock, and continuous at `v = 50` (both `25`).
5. The d-pad then overrides: `LEFT` held writes `-100.0`, `RIGHT` `+100.0`
   (`0xc2c80000`/`0x42c80000`), after the stick and before the airbrake test.

**The law, inputs to outputs.**

| Input | Output |
| --- | --- |
| action-4 button (`R`) not held | both airbrakes `0`, whatever the stick |
| held, steer `< -10` | left `100`, right `0` |
| held, steer `> +10` | left `0`, right `100` |
| held, `-10 <= steer <= +10` (centred included) | both `100`, which is the brake |
| d-pad left / right with the button | as full stick: left alone / right alone |

- **Threshold:** `10` on the `+/-100` steer record, strict. On the PSP nub,
  after the deadzone and the curve, that is `v = 20`, a raw `|x|` of
  **`0.36`**.
- **All or nothing:** the button is digital and the store is `100.0`; nothing
  scales with deflection past the threshold.
- **No memory:** both fields are zeroed every novice frame, so a centred stick
  is both, never "the last side".
- **Gesture independence:** the novice sideshift (`L` held plus a flick) is a
  different action and branch; `L` drives no airbrake in novice.

**Measured 2026-10-07**, PPSSPP v1.20.4, `pulse-psp-usa`, software renderer,
Time Trial on Talon's Junction (Venom, the reference scenario). The scheme
byte `0x08ab0cd0` was written `0` (novice) through the debugger instead of
walking the options page; equivalent, because the two default tables are
identical ([above](#the-default-mapping-is-5-7-4-6-9-8-9-8)), and restored
to `1` afterwards. Execution breakpoint at `PlayerInput_Update`'s entry,
`a0` the controller, record read at `a0 + 0x44` on the sixth hit after each
input change, `R` (`rtrigger`) and cross held:

| Stick sent | `Input_GetAxis` | steer `+0x00` | left `+0x08` | right `+0x0c` |
| --- | ---: | ---: | ---: | ---: |
| full left | `-1.0` | `-100.0` | `100` | `0` |
| `-0.5` | `-0.375` | `-10.9375` | `100` | `0` |
| `-0.4` | `-0.248` | `-3.0029` | `100` | `100` |
| `-0.36`, `-0.3` | `-0.199`, `-0.121` | `0.0` | `100` | `100` |
| centred | `0.0` | `0.0` | `100` | `100` |
| `+0.3`, `+0.4` | `0.121`, `0.248` | `0.0`, `3.0029` | `100` | `100` |
| `+0.5` | `0.375` | `10.9375` | `0` | `100` |
| `+0.8` | `0.746` | `52.3926` | `0` | `100` |
| full right | `0.990` | `98.1689` | `0` | `100` |
| d-pad left | `-1.0` | `-100.0` | `100` | `0` |
| d-pad right | `1.0` | `100.0` | `0` | `100` |
| full left, no `R` | `-1.0` | `-100.0` | `0` | `0` |
| centred, no `R` | `0.0` | `0.0` | `0` | `0` |

The "stick sent" column is PPSSPP's input, which its own analog mapping
reshapes before the game sees it; the `Input_GetAxis` column is what the game
read. Every steer value matches the curve above to the printed digit
(`0.375 -> 10.9375`, `0.746 -> 52.39`, `0.248 -> 3.00`), which confirms the
`0.2` deadzone and the gain of `125` live (`0x08a7b698` read `0.2` and
`0x08ae4cec` read `125.0` in the same session). The threshold is bracketed at
runtime between steer `3.0` (both) and `10.94` (one side); its exact value,
`10.0`, is the literal in the comparison.

Confidence **95**: an instruction-level read, every row of it reproduced by a
live capture, and the bracket around the threshold measured on both sides.
Capture script and raw rows: a throwaway script, not kept and
`novice-run1.json` (derived data, not committed).

**The port** (`oag_gameplay::controls::novice_airbrakes`) applies the
threshold to `InputSnapshot::stick_x`, which stands for the record's steer
over `100` (it is fed to the steering ramp as `stick_x * 100`), so the
comparison is `0.1`. The port's stick shaping is `oag_input::pad`'s own, a
`0.15` rescaled deadzone with no curve, not the original's `0.2` and S-curve;
that mismatch affects every player's steering and is outside this finding.
An analogue trigger's travel is carried onto the chosen side unscaled:
**chosen, not measured**, since the original's button is digital.

**Lineage, 2048 (Vita), checked, not resolved.** `eboot-vita-2048-eu-v104`
ships the same `OPT_CTRL_AIRBRAKES` key (`0x8142dd30`, `0x81459f68`) beside
`OPT_CTRL_LAB`/`OPT_CTRL_RAB` and an `auto_airbrakes` setting (`0x8148fca0`).
`Backend/Ships/PlayerInput.cpp`'s constructor `FUN_811b2f72` sets the vtable
`0x81511c88`, whose fourth slot `FUN_811b3072` (4.6 KiB, created as a function
in this pass) is the per-frame fill. It holds no `+/-10` steer test. Its
schemes `2` and `3` (`0x811b35xx`) do write the two airbrake floats
(`+0x50`/`+0x54`) from a steer-shaped value scaled by `1/0.7` (`1.4285715`)
and `1/0.9` (`1.1111112`), which reads like "steering past 70 % pulls the
airbrake on that side, analogue". The decompiler's VFP flag lifting garbles
the comparisons, so this stays a hypothesis at confidence **40**: no rename,
not ported. HD/Fury and Omega: **not checked**, the HD executable belongs to
another lane this pass.

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
the race's skill level by `oag_tables::handling::Misc::shield_for`). The same
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

### The tap history is the player's pad, and it is cleared on the ground

Read 2026-09-06, whole-function, because a port that let AI craft reach this
gesture was destroying an opponent on `07_Track`. Two findings, one of which is
the missing gate the section above says "nothing here checks".

**The whole tap leg is skipped while the craft is in contact with the track.**
`craft+0x1c0 & 1` is the contact bit ([engine.md](engine.md), which reads it as
"no probe touched anything this tick" at `0x0884870c`). At `0x08846bd0`:

```text
08846bc4  lw    a1, 0x1c0(a0)          ; a0 = craft+0x94, the ship struct
08846bc8  andi  a1, a1, 0x1            ; the contact bit
08846bcc  andi  a1, a1, 0xff
08846bd0  bne   a1, zero, 0x08847018   ; in contact -> jump past every tap
```

and the landing site zeroes the history outright:

```text
08847018  sw    zero, 0x88c(s0)        ; the three tap slots
0884701c  sw    zero, 0x890(s0)
08847030  sw    zero, 0x894(s0)        ; (delay slot)
```

So a grounded craft does not merely fail to arm - it **cannot hold a partial
gesture at all**, and an alternation cannot span a takeoff. The inter-tap timer
`+0x884` is advanced before the branch (`0x08846bbc`) and so keeps running
either way. Confidence **88**: the branch and the three stores are unambiguous,
the contact bit's identity is already established on `engine.md` from a
different function, and what is missing is only a runtime leg.

The other side of the same `if` is the release event that
`crates/physics/src/barrel_roll.rs` had until now recorded as its own guess:

| Transition | Test | What runs |
| --- | --- | --- |
| grounded -> airborne | `!(craft+0x1c0 & 1)` and `craft+0x860 & 0x200` | `0x08846ab4`-`0x08846ac8` clear both arm bits `0x80`/`0x100` |
| airborne -> grounded | `craft+0x1c0 & 1` and `!(craft+0x860 & 0x200)` | if armed and `|craft+0x87c| > 0.5`, set the payout timer `+0x898`; then clear both arm bits |

**The payout is once per arm (fixed in the port 2026-10-03).** The landing's
`|craft+0x87c| > 0.5` test sits inside the "armed" test, and the landing clears
both arm bits whether or not it paid. `+0x87c` itself is not reset there: it runs
on to `+/-1.0` and stays until the next completed alternation writes `0.0`. The
port had gated on the phase alone, so every later landing paid again (maintainer
report from play: on a wavy track, "insane boosts"); `ShipState::roll_armed` is
now the arm-bit pair. The grounded-to-airborne clear is omitted because an armed
roll cannot exist on the ground (the tap history is zeroed there), so it would
clear nothing. Static read, confidence 85; no live leg.

`craft+0x860 & 0x200` is this function's own copy of last call's contact bit,
written at `0x08847220`-`0x08847240` at the tail. So the release is the
**landing**, which is what the port already had, and it now rests on a reading
rather than on the payout's gate being the simplest explanation.

One more difference the port did not have: `swc1 f22, 0x87c(s0)` at `0x08846e5c`
and `0x08846f54` writes `0.0` to the roll phase on a completed
alternation **while unarmed** (see "One roll per flight" below), on the far side of
the `cost < shield` test, so a refused gesture levels the ship exactly as an
accepted one does.

**And the gesture is the human player's alone.** Two independent legs, both from
this function, confidence **85**:

1. **The tap leg early-outs on a null pointer.** `0x08846be0` is
   `beq a3, zero, 0x08846c94` on `ship+0x78`, which skips both d-pad reads, both
   axis comparisons and the previous-sample store. The same block's button bits
   are read at `ship+0x78 + 0x20`/`+0x24` and indexed by
   [`Options_ButtonForAction`](#options_buttonforaction-0x088366bc) - the
   *player's* configured mapping - so `+0x78` is a pad block and there is one
   set of bindings, not eight.
2. **The axis edge detector's previous sample is a single global.** The `lui`
   at `0x08846c08` plus the `lwc1`/`swc1` at `-0x4aa8` resolve to
   **`0x08ae4cf0`**, in segment 1 (`.data`/`.bss`, which loads at `0x08ad9798`),
   and `scripts/psp-relocate.py xrefs 0x08ae4cf0` returns exactly the two
   references inside this function. A per-process scalar cannot hold "which side
   of `+/-90` was I on last tick" for eight craft at once, so at most one craft
   ever runs this leg.

**A complete scan for arm sites finds no third one.** Every `sw rX, 0x860(rY)`
in `.text` whose stored register is written by an `ori rX, ?, 0x80` or
`ori rX, ?, 0x100` within the preceding eight instructions, over the relocated
image:

| Site | What it is |
| --- | --- |
| `0x08846d94` (`ori 0x100` at `0x08846d8c`) | the `2,1,2` arm, in the tap block above |
| `0x08846eb8` (`ori 0x80` at `0x08846eb0`) | the `1,2,1` arm, in the tap block above |
| `0x0883d580` / `0x0883d59c` | a **remote** applier: `lbu a3, 0x4(a2)` picks the direction out of an 8-byte packet and `table[id]` picks the craft - the same 8-byte packet `Ship_UpdateSideshiftInput_q` *sends* at `0x08846ddc`/`0x08846f04` when the local craft arms, gated on `DAT_088357f8+0xb8 > 0xd` |
| `0x088442bc` / `0x088442e0` | **dead**, and they look live: each is `li a0, 0` then `beql a0, zero`, always taken, so the `ori 0x80`/`ori 0x100` sitting in the fall-through is never reached and what actually runs is the `andi ~0x80`/`~0x100` clear these two pages already record at `0x088442e8` |

So the only thing that arms a barrel roll on this disc is a human tapping it
out, or a network peer reporting that one did.

Neither leg is a live read of `ship+0x78` on an opponent, which is what would
take this past 90; both were produced with `scripts/psp-relocate.py`
(`callers`, `resolve`, `xrefs`, `field`) and `search`-style operand scans, never
with `get_xrefs_to`, which returns nothing on any PSP database here
([workflow.md](../../workflow.md)).

**This project deviates from leg 2 deliberately.** On a maintainer's ruling of
2026-09-06 - "AI may barrel roll, if they have enough shield energy" - the port
leaves the gesture reachable by every craft rather than gating it on a pad. The
grounded gate above is what makes that affordable, and it is a port; the AI's
access to the gesture is not.

### One roll per flight: an armed craft skips the pattern match (2026-10-07, confidence 95)

Reported from play: "a barrel roll can be triggered during a barrel roll; in the
original this is not possible". Answers, each with its evidence:

| Question | Answer | Evidence |
| --- | --- | --- |
| Refused while one is in progress? | **Yes.** `0x08846d14`-`0x08846d30` test `craft+0x860 & 0x100` then `& 0x80` and branch to `0x08846f58` when either is set, skipping both pattern compares, `Ship_BarrelRollCost`, the charge and the phase-levelling stores. | static, plus the live run below (95) |
| A flag or a timer? | The two arm bits themselves. No timer, no cooldown, no height or airtime test: the only inputs to the gate are `+0x860` and the contact bit. | complete read of the function (90) |
| Per-flight limit? | **One roll per flight.** The bits are not cleared when the phase finishes (`+0x87c` parks at `+-1.0` with the bit still set); they clear only on the grounded/airborne transitions (`0x08846ab4`, `0x08846b98`) and two cancel paths (`Ship_SetState` `0x088442e8`, `Ship_UpdateRespawn` `0x08847a54`, both reset). A second roll after the first *completes*, still airborne, is refused too. | live run (95) |
| Does landing reset it? | **Yes.** A flight after a landing arms again, either direction. | live run (95) |
| Shield cost or reward change with repeats? | No. `Ship_BarrelRollCost` (`0x08840770`) is `g_roll_cost * 0.01 * stats[skill]` with no repeat counter. A refused match charges nothing. | decompile (90) |
| Do taps still count while armed? | Yes: the history shift (`0x08846c94`-`0x08846d10`) runs before the gate, so the history keeps updating; only the match is skipped. | live run (95) |
| The original never clears the history on a match. | Checked, harmless: it is zeroed on every grounded tick (`0x08847018`), and an armed craft cannot match. This port clears on a match. | read (85) |

**Live run** (PPSSPP v1.20.4, software renderer, `pulse-psp-usa.chd`, Time Trial on
Talon's Junction, `scripts/psp-roll-gate.py`): breakpoint at `0x08846a54` each tick
for the player entity, `craft+0x1c0` bit 0 forced to script a flight, real d-pad
presses, entity fields read each tick. The craft sat on the start line, so the
flight is scripted by the contact bit; the gate does not depend on real height.

| Tick | Input | `+0x860 & 0x380` | `+0x87c` | History `+0x88c/890/894` | arm counter `settings+0x23c` |
| --- | --- | --- | --- | --- | --- |
| 5, 20, 35 | right, left, right | `0` -> `0x100` by 38 | ramps 0.025 a tick from 38 | `2,1,2` | 0 -> 1 |
| 50, 60, 70 | left, right, left (mid-roll) | `0x100` | **keeps ramping, not levelled** | `1,2,1` at 53, `2,1,2` at 63, `1,2,1` at 73 | stays 1 |
| 78 | none | `0x100` | reaches `1.0` and parks | | 1 |
| 100, 110, 120 | right, left, right (after completion) | `0x100` | `1.0` | `2,1,2`, `1,2,1`, `2,1,2` | stays 1 |
| 140-141 | contact forced on | `0x100` -> `0x200` -> `0` | `1.0` | zeroed | payout timer `0.483`, payout counter `settings+0x238` 0 -> 1 |
| 155, 170, 185 | left, right, left (next flight) | `0x80` from 188 | ramps to `-1.0` | `1,2,1` | stays 1 (see below) |

The second arm did not move `settings+0x23c`. That store (`0x08846e50`) sits behind
`+0x368 == 0` and a call to `0x08809b38`; what the counter counts was not resolved,
so it is recorded, not explained.

The port had no such gate: `advance_gesture` matched and re-armed on every
alternation, flipping `roll_target` and levelling the phase mid-roll. Ported
2026-10-07 as `ShipState::roll_armed` gating the match and the direct request.

Lineage: HD and 2048 not checked (no cheap capture path this session); Omega not
checked for the same reason.

### The roll is drawn: `+0x87c` eases into `+0x880`, which rolls the ship about its nose

Searched 2026-09-06, and the answer is **yes, there is a render consumer** - two
of them, both reading one intermediate. Confidence **88**.

**Correction, 2026-09-08: every `craft[...]` below is the entity, not the
craft `Ship_UpdateCraft` takes in `a0`.** This is the same trap the shield
field's own history and `fov_additive`/`fov_intercept` (both in
`scripts/psp_trace_fields.py`) already caught once each for a different
offset and for `FUN_088455ec`'s offsets respectively - `FUN_088418e0` is
reached through `craft->0x78`'s child dispatch (the "Deliberately not
renamed" section below quotes the exact instructions), so its own local that
this page's pseudocode calls `craft` is that child, i.e. the entity
`ENTITY_POINTER = craft+0x1c4` names elsewhere. Confirmed live 2026-09-08, at
the rollsign capture below: `entity+0x794` read back exactly the body pointer
`craft+0x1cc` (`Ship_UpdateCraft`'s own `BODY_POINTER`) already held, while
`craft+0x794` itself - Ship_UpdateCraft's own `a0` base - read `0`. The
pseudocode blocks below keep the decompiler's own `craft[...]` spelling
unchanged, matching `camera.md`'s own precedent for the same trap; read every
occurrence as the entity.

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
determined and which way the ship rolls was not, until the capture below settled
it.

**`vmmul.q`'s operand order is `R * body`, settled by the same capture.** `D0`
(the captured display matrix's own row 0) came out a combination of `body`'s
row 0 and row 1 *only*, and `D2` came out `scale * body row 2` unchanged -
exactly `R * body`'s shape, since the other order (`body * R`) would mix
`body`'s **columns** instead. The caveat the thread carried is still real -
the two orders agree while `body` is orthonormal, which it is at every sample
here - but the reading that was actually disassembled is the reading that
matches.

**`[0x002ace1c]` is `0.75`, and it is `g_craft_scale`.** The captured `M'`
read exactly `0.7500` at every sample, including the `angle = 0` baseline
where hand-checking is trivial (`M' = 0.75 * body`, `M = body`, both
confirmed against the live body basis). `0.75` is the `3/4` factor
[camera.md](camera.md#the-34-factor-is-g_craft_scale-a-code-literal-and-it-is-the-scale-this-project-already-knew)
already identified as `g_craft_scale` (`0x08ab0e1c`) at confidence 85 for a
different call site; this is the same scale applied a second, independent
place, and it closes this section's own open item on the strength of an exact
match rather than a coincidence of the two addresses' similar names.

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

#### Settled on the running game, 2026-09-08: the sign, confidence 92

The rollsign capture, PPSSPP v1.20.4 under Xvfb, `pulse-psp-usa.chd`, Time
Trial on Talon's Junction, the player's own craft stationary on the start
line. Method: break execution at `0x08842140` (Stage 2a's own first
instruction), filter to the player's own entity by reading `s2` (the
function's own entity-in-register, confirmed against `cpu.getAllRegs`) since
every craft's per-tick pass through this code hits the same address, write a
test value into `entity+0x880` and `0.0` into `entity+0x854` (the steering
lean, zeroed to isolate the roll term) before resuming, then on the *next*
hit for the same entity read back the previous tick's own display matrix -
`*(*(entity+0x8b0)+0x3c)+0x40`, self-validated because its own row 3
reproduces the rigid body's position exactly - alongside the body basis at
`entity+0x794`. This is the same memory-read technique the sideshift capture
used (`docs/reverse-engineering/ppsspp-debugger.md`), applied to a value
written under debugger control rather than one produced by a scripted
gesture: a ground d-pad `[2,1,2]`/`[1,2,1]` gesture was tried first and
produced no roll at all (`entity+0x87c` stayed exactly `0.0` through 20
polls at `grounded=1.0`), consistent with this port's own airborne-only
arming gate (`crates/physics/src/barrel_roll.rs`) - so the direct write is
what actually produced the data below, and the ground-gesture attempt is
recorded as the honest negative that motivated it rather than omitted.

Six samples, `entity+0x880` at `0.0`, `+/-0.2`, `+/-0.35`, and `0.0` again to
close the sweep. Reading `D0`/`D1` (the captured matrix's rows 0/1) against
the body's row 0/1 (`right`/`up`):

| `entity+0x880` | `angle = value * 6.28` | `dot(D0,right)` | predicted `scale*cos` | `dot(D0,up)` | predicted `+scale*sin` |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `0.000` | `0.0000` | `0.7500` | `0.7500` | `-0.0000` | `0.0000` |
| `+0.200` | `1.2560` | `0.2322` | `0.2321` | `0.7131` | `0.7132` |
| `-0.200` | `-1.2560` | `0.2322` | `0.2321` | `-0.7131` | `-0.7132` |
| `+0.350` | `2.1980` | `-0.4402` | `-0.4409` | `0.6073` | `0.6068` |
| `-0.350` | `-2.1980` | `-0.4402` | `-0.4409` | `-0.6073` | `-0.6068` |

Every sample agrees with the **literal, un-transposed** reading of `R` -
`D0 = scale*cos(angle)*right + scale*sin(angle)*up` - to 3-4 significant
figures, `scale` reading exactly `0.75` throughout (see above). `dot(D1,right)`
and `dot(D1,up)` mirror this (`-scale*sin(angle)` and `scale*cos(angle)`
respectively) at every sample and are omitted from the table for space.

**What this means for the ship's own roll:** for a positive `entity+0x880`
(same sign as `roll_phase`, since `ease` is odd and monotone), the physics-
right axis gains a positive `up` component - **the right wingtip rises**.
Since `right`, `up` and `forward` are a right-handed triple here
(`cross(right, up)` reproduces `forward` to four figures at every sample,
confirming `camera.md`'s own reading of the row layout) and the captured
`up` row is itself near world-`+Y` (`(-0.046, 0.996, 0.076)` at the baseline
sample, consistent with a level start line), "rises" is a plain world-height
statement and needs no further handedness reasoning.

`crates/render/src/roll.rs`'s `ROLL_DIRECTION` carries this finding, flipped
from the `+1.0` it shipped as (chosen, not measured) to `-1.0` -
`crates/raceplay/src/drawable.rs`'s
`roll_direction_matches_the_original_captured_display_matrix` test reproduces
this exact body and phase through `oag-render`'s own `orientation * roll`
composition (rather than hand algebra) to carry the sign across the two
engines' different forward-axis conventions
(`oag_physics::ship::Body::forward` is local `-Z`; the row read here is `+`)
and pins the constant against it. A `[1, 2, 1]` gesture (tap LEFT first,
which ramps `roll_phase` toward `-1.0`) turns out to drop the **right** wing,
not the left the constant shipped assuming.

Confidence **92**: a runtime trace of the exact quantity (six samples across
a baseline and four nonzero angles, all agreeing to 3-4 figures, plus a
self-validating pointer chain), one binary, one emulator - the same evidence
shape `camera.md`'s fov law scored 94 with nineteen samples; short of that
ceiling for fewer points. The internal camera's own site
(`FUN_088455ec`'s `up = Rot(...) * up`) was **not** independently captured
this pass; `oag_render::camera::internal` inherits the same
`ROLL_DIRECTION`, labelled as an inference rather than a second measurement
- see its own doc comment.

#### Deliberately not renamed

`FUN_088418e0` stays `FUN_`. It advances per-craft timers, runs contact reaction
over the body's contact array, updates the internal camera rig **and** builds and
submits the display matrix; no `Subsystem_VerbNoun` covers that, and camera.md
already declined to name it for the same reason. Naming it would be a guess that
stops the next reader looking.

#### What is not determined

Three of the four items this section used to list are closed by the
rollsign capture above: the sign, `vmmul.q`'s operand order and
`[0x002ace1c]`. What is left:

- **The internal camera's own site** (`FUN_088455ec`'s `up = Rot(...) * up`)
  was not independently captured - `oag_render::camera::internal` inherits
  the ship's measured sign on the argument that the VFPU register-naming
  ambiguity is one binary-wide fact, not a per-call-site choice, but that is
  an inference, not a second measurement. A capture identical in shape to
  the one above, breaking inside `FUN_088455ec` instead of `FUN_088418e0`,
  would close it.



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
- Novice's airbrake button `R` picks its side off the steer record: left
  alone below `-10`, right alone above `+10`, both in between. See
  [above](#novice-r-picks-its-airbrake-off-the-steering); ported in
  `oag_gameplay::controls::novice_airbrakes`.
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
