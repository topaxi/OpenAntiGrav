# The reset relocation: a ring of grounded positions, and the launch

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`. Read 2026-10-13 (lane `jump-13-flight`) in Ghidra with the
Allegrex module, every claim below at instruction level unless marked. Then
**verified live** the same day by lane `pulse-13-capture` on PPSSPP, which
read the ring every tick through nine resets ("Verified live" below). Names
applied from [names.tsv](names.tsv).

This answers where a craft put through `Ship_SetState(3)` comes back, which
[shield.md](shield.md#state-3-is-the-reset-and-four-things-enter-it-2026-09-29)
read as "the corridor midpoint, five up, launched at `+0x78c * 0.25 + 50`" and
left the source of the position open. The position is **not where the craft
fell**: it is the oldest of the craft's last seven grounded positions.

## The functions

| Address | Name | What it does | Confidence |
| --- | --- | --- | --- |
| `0x0883d960` | `Ship_PushResetCrumb` | Copies a 4x4 pose into slot `craft+0x780` of an 8-slot ring at `craft+0x380 + i * 0x40`, advances `+0x780` mod 8, and when it lands on `+0x784` advances that too | 90 |
| `0x0883d9ec` | `Ship_LayResetCrumb` | Unless `craft+0xb51 & 1`, compares the pose's position (`+0x30..+0x38`) with the newest crumb's (`craft+0x370 + head * 0x40`, slot 7 when the head is 0) and pushes when any axis differs by more than `20.0` | 90 |
| `0x0883ef30` | `Ship_TakeOldestResetCrumb` | Copies the crumb at `craft+0x784` out (w lanes forced to `0,0,0,1`) and sets `+0x780 = +0x784 + 1` mod 8 | 90 |
| `0x0883ff6c` | `Ship_UpdateReset` | State 3's update ([shield.md](shield.md)): relocates once from the oldest crumb and launches | 90 |
| `0x0883db20` | `Ship_PlaceAndLaunch` | `FUN_0884d9bc(body, pose)` copies the pose into the body; then `body+0x140 = pose.row2 * speed` | 92 |

`0x0884d9bc` copies a 4x4 matrix from its second argument to its first. Ghidra
shows it with one parameter. The call site passes the pose in `a1`, untouched
from `Ship_PlaceAndLaunch`'s own `a1`.

### The ring is closed: every accessor was found

`search_instructions` over the whole image for `0x780(` and `0x784(` finds the
head and the oldest read or written only in `0x0883d960`, `0x0883d9ec`,
`0x0883ef30` and `Craft_Construct_q` (zeroes both, `0x08841154`/`0x08841158`).
The rest are stack offsets. `0x0883d960` has two callers:
- `0x0883d9ec`;
- `Race_PlaceGrid` (`0x08827dc8`), which seeds the ring with the grid pose.

`0x0883ef30` has two callers:
- `0x0883ff6c`;
- `Ship_UpdateRespawn` (`0x08847914`, state 8, the Eliminator's return).

Seven crumbs survive, not eight: the push that brings the head onto the oldest
drops the oldest.

### When a crumb is laid, and the speed latched

`FUN_088418e0`, the per-craft update, at `0x08841c00`:

```text
08841c00  lw     a0,0x94(s2)        ; the physics craft
08841c04  lwc1   f12,0x2b0(a0)      ; the 0/0.5/1 grounded fraction
08841c08  c.eq.s f12,f22            ; f22 = 1.0 (lui 0x3f80 at 0x08841908)
08841c10  bc1f   0x08841c30         ; not fully grounded: skip both
08841c1c  jal    0x0883d9ec         ; lay a crumb (a1 = the craft's pose)
08841c28  lwc1   f12,0x398(a0)      ; a0 = craft+0x794, the body: |velocity|
08841c2c  swc1   f12,0x78c(s2)      ; latched
```

`body+0x398` is `|velocity|` ([rigid-body.md](rigid-body.md), confidence 92).
So `craft+0x78c` is **the speed on the last fully grounded frame**. It is latched
on every such frame, including the ones where the flag forbids a crumb. This
settles shield.md's open reading of `+0x78c` (confidence 50 there): it is the
launch speed's input, not a hover height.

`0xb51` is `0xaf0 + 0x61`. `craft+0xaf0` is the `SplinePt` record
`AiTrack_LocatePosition` keeps for the craft, and `+0x61` is `flags`
([track.md](../../../formats/track.md#locating-a-ship-on-the-track)). **So the disc decides where no crumb may
be laid.** A census over every Pulse circuit (`the_flag_bit_zero_census` in
`crates/game/tests/novice_jump_ground_truth.rs`) finds bit 0 on 8 of 24 track
files. Seven of them bracket a run of racing-line samples with no surface
under them:

| circuit | bit 0 (line indices) | no surface (line indices) |
| --- | --- | --- |
| `13_Track` | 3036-3083, 0-119 | 19-49 |
| `29_Track` (13 reversed) | 3028-3079, 0-115 | 19-48 |
| `10_Track` | 20-211 | 135-206 |
| `14_Track` | 652-715 | 684-765 |
| `25_Track` | 476-691, 3272-3383 | 524-755 |
| `26_Track` | 3204-3387, 0-7 | 0-67 |
| `30_Track` | 3612-3647, 0-27 | 3-37 |
| `09_Track` | 3996-4015, 0-75, 1308-1419 | 967-1021 (not bracketed) |

`01`, `02`, `05`, `06`, `07`, `17`, `18` and `22_Track` have unsupported runs and
no bit 0. Those are drops onto road below, not pits. Confidence **80** that bit
0 means "lay no reset crumb here": the gate is read, and the authored runs sit
on the pits. Nothing else reading it was looked for.

### The relocation, `0x0883ff6c`

`+0x79c` is zeroed on entry to state 3, so the `< 0.3` branch runs on the first
update and the state returns the same frame:

```text
crumb = Ship_TakeOldestResetCrumb(craft)              ; 0x0883ffc4
crumb.y += 5.0                                        ; lui 0x40a0 at 0x0883ffd4
AiTrack_LocatePosition(500.0, track, craft+0xaf0, &crumb, craft+0xad8, craft+0x8f0, 0)
mid  = rec.pos + rec.lateral * (rec[+0x48] - rec[+0x44]) * 0.5   ; lui 0x3f00
pos  = mid + (0, 5, 0)
aim  = rec.pos + rec.tangent * 40                     ; lui 0x4220 at 0x08840160, f22
aim  = locate(aim).pos + locate(aim).tangent * 40     ; 0x088401f8, then the vscl at 0x08840210
aim  = locate(aim).pos                                ; 0x0884028c
fwd  = normalize(aim - mid)
up   = -rec.down, orthonormalised against fwd         ; lui 0xbf80 at 0x08840304
Ship_PlaceAndLaunch(craft+0x78c * 0.25 + 50, craft, {right, up, fwd, pos})   ; 0x08840448
```

- **Two 40-unit hops, three locates.** shield.md said "forty units down the
  tangent". The disassembly has three `jal 0x0887ce78` and one `40.0`, held
  in `f22` and used by two `vscl.q`. The second hop scales the tangent of the
  record the first locate wrote: `s4 = sp + 0x60` is the record's `+0x10`.
- **The midpoint is the track's width midpoint, not the AI corridor's.**
  `+0x44`/`+0x48` are `half_width_left`/`half_width_right`. The AI bounds are
  `+0x4c`/`+0x50`.
- **The record's position is the lifted one**, three units above the surface
  line ([track.md](../../../formats/track.md)), and the pose adds five more in
  world `y`.

Confidence **84** for the whole relocation: decompile and disassembly agree,
and every input is a field with its own read evidence. **The one capture fits**
([leaving-the-track.md](../../../gameplay/leaving-the-track.md)): the player's
craft reset on `01_Track` came back moving at 53.5 u/s. That is `50 + 14 / 4`,
and 14 u/s is a plausible last grounded speed for a craft placed at 20 u/s that
coasted for 2.2 s. It came back at sample 50 on the lower floor, which is not
where the trial placed it (sample 10). That fits "oldest crumb" only if earlier
trials in the same session had left crumbs on the lower floor, which was not
recorded. **Not runtime-verified**, so capped below 85. One breakpoint on
`0x0883ff6c` reading `craft+0x780`/`+0x784`/`+0x78c` and the crumbs would
settle it.

**The launch speed reproduces the capture from an independent latch**
(2026-10-13). Ours was re-run on the same placement: `shove_trial_trace` with
`OAG_SHOVE_TRIAL=01_Track:10:L:0:20 OAG_COAST=1 OAG_TRIAL_TICKS=420`. It
latched 14.3 u/s on the craft's last fully grounded frame and launched at
**53.6 u/s**, against the original's 53.5. So the speed law is at **88**: one
capture agrees with it to 0.1 u/s. The position cannot be compared: our
sweep drives 600 ticks before placing the craft, and the original's ring
history before its trial is unknown. Ours came back at sample 940 and the
original at 50.

## Verified live (2026-10-13, lane `pulse-13-capture`)

PPSSPP, Pulse PSP USA, `13_Track`, VENOM. The ring was read on the entity
(`craft+0x1c4`) every tick: crumbs at `+0x380 + i * 0x40 + 0x30`, head
`+0x780`, oldest `+0x784`, latch `+0x78c`, flag byte `+0xb51`. The full tables
are in [ai.md](../../../gameplay/ai.md), "What the original does after a
fall". Every prediction of the reading above held:

- **Nine resets, nine on the oldest crumb.** Four were placed or teleported;
  five were natural drive-in falls in a Time Trial (a pure-pursuit driver at
  speed caps of 60 to 100).
- **The speed is exact.** `50 + 0.25 * latch` to 0.1 u/s on all nine. Each
  natural fall's latch sits within 2 u/s of its speed at the lip (84.0/83.4,
  71.9/70.6, 59.1/60.2, 79.6/80.4, 100.0/98.1). That fits the reading that the
  latch is taken on every fully grounded frame, until the craft leaves the
  ramp. The capture's notes read it as the speed at the ring's last push; the
  instructions settle it the other way, since the store at `0x08841c2c` sits
  in the grounded block, outside `Ship_LayResetCrumb` and its flag gate.
- **On `13_Track` the ring freezes where the disc's no-crumb flag begins**,
  row 3036, the same row this project's census finds (line index 3036). Every
  natural fall reappeared at row 2949, (-388.6, 7.3, 240.8): the oldest crumb
  (-388.1, 3.3, 240.4) moved to the midpoint and 5 up.
- **The ring holds seven valid crumbs.** Before each reset the head is 4 and
  the oldest 5: slot 4 is the stale one the next push writes, slot 5 is row
  2949.
- **The reset takes one frame** (0.97 to 1.03 frames between consecutive
  `Ship_Update` hits). The heading is along the line.
- **The AI resets in the race captures fit the same law.** They come back at
  rows 2938-2953 at 72.6-76.1 u/s at skill level 2.

So the ring, the gate and the launch are runtime-verified on one binary:
**90** for the ring functions and **92** for the launch, each short of 95 by
the second binary the rubric asks for.

**This project's port reproduces it.** `the_parked_field_is_put_back_where_the_original_puts_it`
(`crates/game/tests/novice_jump_ground_truth.rs`, disc-backed) asserts that
every reset of the parked-player field lands at line rows 2936-2955 at
`50 + 0.25 * latch` to 1e-3 u/s. Measured: rows 2939 and 2950, 73.6 u/s from a
94.6 latch.

**Not verified:** the width midpoint against the AI corridor (both put the
craft on the line here, where the line is the centre), and the two 40-unit
hops (the heading only shows the line direction).

## Ported, and what was not

`crates/raceplay/src/breadcrumbs.rs`, for the two triggers the original has
(`RespawnCause::ResetZone`, `RespawnCause::Airborne`), on 2026-10-13. Measured on
`13_Track`, VENOM, seed 1, the original driver's field with the player parked,
9,000 ticks:

| | respawns | where put back |
| --- | --- | --- |
| before (at rest, nearest sample to the fall) | 389 (one fall per slot, then a loop) | line index 38-46, over the gap |
| after | 2 | line index 2939 and 2950, about 160 samples before the gap, at ~73 u/s |

Departures, each recorded in the module:
- The locate is the nearest spline sample, not the B-spline projection.
- A crumb is laid after this project's step from that tick's groundedness. The
  original's order between `Ship_UpdateCraft` and `FUN_088418e0` was not traced.
- `place_at` zeroes the angular velocity, which `Ship_PlaceAndLaunch` leaves
  alone.

Not ported:
- The player's shield charge (`Ship_SetState(3)`, shield.md).
- The Eliminator's state-8 return, which reads the same ring.
- This project's invented dwells (`LostCircuit`, `Beneath`, `Stalled`,
  `OffTrack`). They stand in for the original's distance trigger
  (`0x08841cec`), which enters the same state, but their rescues are measured
  against the old pose.

**2048 and Omega: not checkable here.** This is a Pulse executable reading,
and neither title's reset was opened in this pass.
