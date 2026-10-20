# The collision-cage effect pool (`cage_collision_curved.vex`)

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`. Names are applied from [names.tsv](names.tsv). Written 2026-10-02
from static reading of `boot.dis` plus a live PPSSPP v1.20.4 session on
`03_Track` (a `collisionCageEnabled` circuit). The decode of VFPU code came from
PPSSPP's own `memory.disasm`, not `objdump` (which reads `lv.q`/`sv.q` as Octeon
`bbit`).

## What is established

| Claim | Evidence | Confidence |
| --- | --- | --- |
| `0x0883170c` (`CageEffect_Construct`) is the only code that references the vex path: `lui 0x27 / addiu 0x6b98` at `0x0883170c+0x64` builds the string `Data\visual_effects\cage_collision_curved.vex`, passed to the model loader `jal 0x08943330` with flags `1, 0`; it then sets bit 2 of model+0x14 and builds four 0x90-byte scene objects in a ring at `this+0x734..0x740`, cursor at `this+0x744` | `boot.dis` `2d70c..2d888`; live: singleton at `[0x08ab0c88]` has four slot pointers at `+0x734`, each a scene object with vtable `0x08a6ccfc`, the vex name pointer `0x08a7abe0` and an identity 4x4 at `+0x50` (parked at the origin, nothing placed) | 75 |
| It is built once per race load: one caller, `0x08829640` inside `RaceManager_Construct`, and the singleton pointer moved from `0x09a0a3d0` to `0x09a0c9a0` across a `psp-drive.py restart` | caller `jal 0x2d70c` at `0x25640`; two live reads | 80 |
| `g_cage_effect` (`0x08ab0c88`) holds that singleton; `0x08ab0c84` the loaded model (`0x08cf69f0`); `0x08ab0c8c` a byte "cage usable" flag; `0x08ab0c80` a float set to the tick `dt` (0.016689) by the setter `0x08831980`, called from `0x08829938` | live read, flag 1 and the float on `03_Track` | 80 |
| `0x08832b0c` (`CageEffect_Spawn`) is the spawn: for each of the four slots it calls `0x0883318c` (slot age) and reuses a slot whose age is in (0.1, 0.25) and which is within 7.0 of the new contact point, otherwise takes the ring slot `[this+0x744]`; either way it calls `0x08832fb0(slot, contact frame, a2, a3)` to pose and start it, and advances the cursor modulo 4 | static, `boot.dis` plus `memory.disasm` of `0x08832b0c..0x08832d40`. **Never observed to fire live** (breakpoint at its entry silent through a wall scrape and four off-track `place` teleports) | 58 |
| `0x08831aa4` is a per-craft query, reached through the wrapper `0x0883191c` from `0x08842384` (inside the per-craft update, 5.9 KB past `Craft_Construct`) every tick while the flag `0x08ab0c8c` is set; it casts a 500-unit query (`AiTrack_UpdateCursor`, `0x0887e464`) from craft+0xb0, returns **2 when the hit is further than 200.0 (`0x4348`)**, 127 for "nothing", and on a nearer hit tests the segment against the track data and, within 15.0 (`0x8a7abd8`), calls `CageEffect_Spawn` then `Body_ResolveContact` | static; live: returns 127 on every one of ~200 sampled ticks on `03_Track`, including a wall scrape | 55 |
| The caller treats a return of 2 as event 3: `jal 0x08844100(craft, 3)` at `0x08842390`. This is the likely home of the 200-unit airborne rule the leaving-the-track page lists as measured but unread | static only | 50 |
| The sphere in `yaw.ppdmp` (prims 627/628, world translation about `(-124.6, 2.3, 480.4)`, a non-uniform scale of about 40/40/81) is a **posed instance of this model**, not the placed-at-origin pool state | prim census of `yaw.ppdmp`; the pool instances here sit at the identity | 65 |

`0x0883198c` (`Collision_SweepSegment`, named elsewhere) calls `0x08832134` through
the wrapper `0x08831948` as its fallback; `0x08832134` also fires every frame, from
`Camera_UpdatePlayerView`'s sweep (`ra=0x08831974`). It was not decoded.

## What is not established

- **What puts the sphere in the player's frame.** `CageEffect_Spawn` is the only
  path seen that poses an instance, and it never fired. The trigger as read is a
  segment hit on the cage geometry inside 15 units; no pose reached it.
- Whether the spawn is ever reached on a circuit without the attribute. The
  attribute is read at `0x088c40a8` into the definition record at `+0x16f`; no
  reader of that byte was found, and `CageEffect_Construct` is unconditional (the
  singleton exists on `03_Track`, which has the attribute; a circuit without it
  was not booted).
- The pose and fade written by `0x08832fb0`, and the scale seen in the dump.

## Next addresses

`0x08832fb0` (slot pose and start), `0x0883318c` (age), `0x08832134` (the sweep's
fallback), the consumer of definition `+0x16f`, and a PPSSPP execution breakpoint
on `0x08832b0c` while a craft is driven into the track's leaky edge on a circuit
in the yaw dump (`07_Track`).

## The cage is a contact source, live 2026-10-20

**Corrects the table above** in two places. `FUN_08831aa4` (now
`Cage_ResolveCraft`) does fire on a racing craft, and it is not "a segment hit on
the cage geometry": it is an **invisible elliptical tube along the AI track data**
that keeps every craft inside it, a second contact source beside the hull
narrowphase. The 127 returns on `03_Track` were a craft inside the tube. Found by
the lane `wall-contact-miss`, chasing the wall hit our hull does not find on
`talons-junction-clean-lap.csv` between ticks 1583 and 1584
([contact-response.md](contact-response.md#one-wall-hit-our-hull-does-not-find-clean-lap-tick-1584)).

### How it was found

PPSSPP v1.20.4, software renderer, muted, `pulse-psp-usa.chd`, Time Trial,
Talon's Junction, VENOM. `scripts/psp-contact-at.py` stops at `Ship_UpdateCraft`
(`0x08849618`) for the player, writes one row of a capture into the body (rows,
position, velocity, `+0x150`, `+0x160`), then stops after
`Collision_StepNarrowphase` returns (`0x0885001c`) and reads the world's contact
array.

- At tick 1583's state the narrowphase makes **no contact**, at the same box
  centre our step reaches (`(-768.68, 17.17, 91.99)`, ours `(-768.68, 17.16,
  92.00)`), yet the next `Ship_UpdateCraft` reads the velocity redirected to
  `(-82.6, 3.6, -54.8)` (recorded at tick 1584: `(-83.4, 4.0, -53.4)`).
- Stopping in between: the velocity is unchanged at `Body_StepWorld`'s return
  (`0x08850270`) and already redirected at its next entry (`0x0884f70c`), and the
  only call to `Body_ApplyImpulseAtPoint` in between comes from
  `Body_ResolveContact` (`ra 0x0884eeb0`), whose only callers are
  `Body_StepWorld` and `FUN_08831aa4`.
- A stop at the cage's own `Body_ResolveContact` call (`0x08831ff0`, condition
  `s0 == body`: `a1` is set in the delay slot) read the contact: point
  `(-770.206, 15.632, 87.516)`, normal `(-0.441, 0.317, 0.840)`, depth `0.646`,
  friction `0.000`, and the section slot it was built from (`*0x08ae4cd8`).
  Rebuilding that slot from the disc's control point reproduces it to `0.0005`,
  and testing the live collider's own eight corners against it reproduces the
  contact to three decimals.

The teleport's orientation write does not stick (the collider's rows at the
narrowphase were about 30 degrees off the written basis), so per-tick numbers
from the probe validate the construction and the test on live inputs, not tick
1583's own corner. Tick 1583 itself is reproduced offline from the recorded pose,
below.

### The law

`Cage_ResolveCraft(cage, collider, body, cursor)`, through `Cage_UpdateCraft`
(`0x0883191c`), from the ship entity update `FUN_088418e0` at `0x08842384`, after
`Body_StepWorld`, while `g_cage_enabled` (`0x08ab0c8c`) is set, for an entity
whose `+0x368` is `0` (the human) or `2` (every AI craft,
[shield.md](shield.md)) and whose `+0x8c` is not `6`:

1. Locate the box collider's centre (`collider+0xb0`, the pose
   `Body_SyncBoxCollider` left after `Body_Integrate`) with
   `AiTrack_UpdateCursor(500.0, ..)` from the entity's cursor (`entity+0xad8`).
   The cursor's point is a **control point record** (lifted position, tangent,
   down, lateral, the eight bytes at `+0x68`), not an interpolated sample. None:
   return `127`. Further than `200.0` from the centre: return `2`, no contact.
2. `AiTrack_LocateOnSiblingPath` (`0x0887d4d4`): the other successor of the
   path's entry junction, else another predecessor of its exit junction; if one
   exists, the nearest point on it to the located point's own position.
3. `Cage_BuildSection` (`0x08832464`) for the point (and the sibling), into a
   ring of four `0x90`-byte slots. Four extremes, each one byte `b` of `+0x68..+0x6f`
   turned into `s * |s| * 250`, `s = b / 127` (`0.007874016`, `fabsf` at
   `0x0897e140`):

   | Extreme | Function | `lateral` | `down` |
   | --- | --- | --- | --- |
   | left | `Cage_SectionLeft` `0x08831104` | `o0 - 25` | `o1 - 5.5 + 3.0` |
   | right | `Cage_SectionRight` `0x08831288` | `o2 + 25` | `o3 - 5.5 + 3.0` |
   | top | `Cage_SectionTop` `0x0883140c` | `o4` | `o5 - 30 + 3.0` |
   | bottom | `Cage_SectionBottom` `0x0883158c` | `o6` | `o7 + 30 + 3.0` |

   all from the lifted position. Then a frame: `L = unit(right - left)`,
   `V = unit(L x tangent)`, `T = unit(V x L)` (the free axis).
4. For each of the box's **eight corners** (`Collider_BoxSamplePoints`, entries
   0-7; not the flank pair), `Cage_TestPoint` (`0x088327b8`): with `l, v` the
   corner's `L` and `V` coordinates, the vertical centre is the left extreme's
   `V`, the half above or below picks the top or the bottom extreme (its `L` is
   the lateral centre and its `V` the vertical reach), and the side picks the left
   or right extreme's `L`. `x, y` normalised, outside when `x^2 + y^2 > 1`. The
   boundary point is the radial projection `(x, y) / r` back in world space at the
   corner's own `T`, and the normal points inward along the ellipse's gradient.
5. Outside the point's section and, at a fork, outside the sibling's (inside
   either is no contact): the nearer of the two boundary points. Depth is
   `|boundary - corner|`; over `15.0` (`0x08a7abd8`) returns `2` at once. Else
   `CageEffect_Spawn` (`0x08832b0c`) at the boundary point, then
   `Body_ResolveContact` on a contact `{point: boundary, normal, depth,
   friction: 0}`. A contact whose normal's world `y` is above `0.5` (the craft
   under the tube's floor) makes the return `2`, else `4`.
6. The caller sets ship state 3 (the reset) on a return of `2`.

`g_cage_enabled` is written `1` unconditionally at `0x08886ef8` (in
`FUN_08886af4`, called from `Race_CreateModeObject`) and cleared by
`Cage_ResolveCraft` itself when `AiTrack_CageEnabled` (`0x0887d36c`, `*(track +
4) + 0x18 & 1`) reads `0`: bit 0 of the `WO Track` header word `+0x18`, which is
`1` on all 40 Pulse tracks and `0` on all 16 Pure ones
([track.md](../../../formats/track.md#the-file-layout)). The
`collisionCageEnabled` definition attribute still has no reader.

Confidence **88** for the law: decompiled end to end, the slot and the test
reproduced on the live inputs to three decimals, and the step-then-cage
reproduction below. **80** for the reset returns (read, one of them, the
200-unit check, unexercised live).

### Offline: the clean lap's tick 1583 and Fort Gale's tick 1341

`oag_physics::step` from the recorded pose, then the cage
(`crates/trace/tests/collision_cage_ground_truth.rs`):

| Capture, tick | our hull | cage contacts | `\|v - next\|` without -> with | angular residual |
| --- | ---: | ---: | --- | ---: |
| `talons-junction-clean-lap`, 1583 | 0 | 2 | `38.43 -> 0.18` | `0.093` |
| `fortgale-autopilot-ramp-3` (untracked), 1341 | 0 | 1 | `29.72 -> 0.12` | `0.169` |
| same, 1342 | 0 | 1 | `5.07 -> 0.13` | `0.066` |
| same, 1343 | 0 | 1 | `2.71 -> 0.13` | `0.030` |

The recorded clean-lap `dv` is purely along the cage normal (its length and its
normal component are both `39.29`), which is the friction of `0`. Across the
three tracked Talon's Junction captures and two of this lane's own (a weave and
a pulse-steer run, 4,200 ticks), the cage fires on tick 1583 alone and never
moves a step away from the record.

**The hull was never the miss.** At the original's own narrowphase pose (the
collider centre, rows and ten sample points read at `0x0885001c` each tick,
`scripts/psp-autopilot-contacts.py`), the documented hull law on our collision
mesh reproduces **818 of 818** of the original's contacts on the weave run and
**1,466 of 1,466** on the pulse-steer run, at the same sample, normal and depth,
with 2 and 0 extras (two at depth `0.000`). The capture also fixes the sample
order: the body's row 0 is its left, so the original's sample 0 is lower left
front, and ours (`oag_physics::wall::hull_sample_points`) lists the mirror image
first. The two captures are kept (gitignored) at `data/reference/wall-contact-miss/`
(`talons-weave1`, `talons-pulse1`: the CSV and the `.contacts.jsonl`); flatten with
`scripts/psp-contacts-flat.py` and run
`collision_cage_ground_truth::the_originals_contacts_against_our_mesh` with
`OAG_SWEEP=1 OAG_CONTACTS_FLAT=<file>`.

**And the hull is on time.** `wall_contact_ground_truth.rs` records our first
contact 3-5 ticks after the original's. Stepped from each recorded row instead
(`collision_cage_ground_truth`'s sweep with `OAG_CAGE_TICKS`), our hull makes a
contact on exactly the ticks the original's `speed / |v|` marks one, onset included:
standing start ticks 183-190, time-trial lap 166-176. The lateness is that test's
method (it tests the row pose, before the frame's integration), not the hull.

### Ported

`oag_physics::cage` (the section, the test, the corners in the original's order,
the resolve through `Body_ResolveContact`'s port with friction 0, the three reset
conditions as `CageResponse::reset`), `oag_gameplay::cage` (the locate: nearest
control point over every path, which is where the cursor's three-nearest scan and
walk settle; the sibling), and `oag_raceplay` calls it after every craft's step,
player and field, on a track whose header flag is set and whose points carry the
bytes. The return of `2` is `oag_raceplay`'s `RespawnCause::Cage`, through the
original's reset relocation (`Race::reset`). The race's own `Reset`-volume test now
reads the step's pose before the cage moves the craft, as the original's `Reset`
contact comes out of the narrowphase first. **The `entity+0x8c == 6` skip is ported for
a wreck (2026-10-20)**: `FUN_088418e0` loads `s1 = 6` at `0x0884193c`, and the cage block
reads `lw a0, 0x8c(s2)` at `0x08842360` and branches past the `Cage_UpdateCraft` call on
`beq a0, s1` at `0x08842364` (static; `s1` is callee-saved, no write to it was looked for
between). Outside the Eliminator a wreck reaches state 6 1.5 s after it finishes exploding
(measured live 2026-10-02, `shield.md`), so `oag_gameplay::cage::pass` skips a craft for
which `oag_physics::damage::in_state_6` holds. On de Konstruct Black (12 forward cells by
nine perturbation scales, `cage_effect_sweep`) it changes 5 of 108 races and moves
destroyed 59 to 60: a wreck's pose decides nothing else. Still not ported: the finished
player, which also reads `6`, and `CageEffect_Spawn`'s visual (the trigger is now known: one per cage
contact, at the boundary point). Our own track format carries the cage bytes but
not the header flag, so a track file runs no cage.

**Other titles.** Pure: off by its own data (header word `0`), and Pure's
binary is not read. HD: `04_chenghou_project` alone sets the flag and authors the
bytes on all 839 points, the other fifteen circuits neither
(`crates/hd/tests/hd_title_ground_truth.rs`); our race runs Pulse's cage there,
**inherited, not measured**: HD's code is not read. 2048 and Omega (version
`0x107`, 96-byte points): not checkable from data, as the points end before
`+0x68`, so no cage; whether their code has one is not read.

### What the cage does to our AI (2026-10-20, `ai-gate-robustness`)

De Konstruct Black's forward field (four classes, seeds 1-3, seven Aces) over the nine
scales of `oag-raceplay`'s `tests/perturb` (108 races each way), cage off against on
(`cage_effect_sweep`, `OAG_SWEEP=1`):

| | cage off | cage on |
| --- | ---: | ---: |
| destroyed per board of 12 cells, mean (sd) | 5.3 (2.9) | 6.6 (2.5) |
| deaths where wall charge was the larger share | 39 | 39 |
| deaths where weapons (the rest) were the larger share | 9 | 20 |
| weapon damage taken, all craft | 18,336 | 20,715 |
| wall charge, all craft | 20,308 | 21,099 |
| `Airborne` rescues | 755 | 37 |
| `ResetZone` rescues | 581 | 948 |
| `Cage` resets | 0 | 208 |

So the cage does not kill through the walls: a cage contact costs no shield (friction 0)
and wall-dominant deaths are unchanged. It keeps craft in the tube that would have beached
and been put back by the `Airborne` rescue, replacing those with reset-zone rescues and
cage resets; the extra deaths are weapon deaths (7 of the 60 cage-on deaths came within
10 s of a cage reset). Both the cage and the reset relocation are the original's; whether
the original's field takes the same extra weapon deaths is not measured.

### The denominator on a rolled craft (contact-response.md's open question)

Fort Gale 1341 is a cage contact on a craft rolled to `up.y 0.65`. The cage's
angular share against the recorded one (the record minus our step without the
cage): world-axis denominator (as ported) ratio `0.821`, cosine `0.990`;
body-frame denominator ratio `0.910`, cosine `0.990`; at 1342 `0.896` against
`0.993`. It **leans body-frame**, one capture, three ticks, with the free-flight
residual on these captures `0.02-0.04` rad/s against a `0.06` difference.
Confidence **55**, recorded and not ported.

### Names applied (2026-10-20)

| Address | Kind | Name | Conf | Evidence |
| --- | --- | --- | ---: | --- |
| `0x08831aa4` | function | `Cage_ResolveCraft` | 88 | decompiled; its `Body_ResolveContact` call caught live with the contact the law predicts |
| `0x0883191c` | function | `Cage_UpdateCraft` | 85 | three instructions: `Cage_ResolveCraft(g_cage_effect, ..)`; one caller, `0x08842384` |
| `0x08832464` | function | `Cage_BuildSection` | 88 | decompiled; live slot rebuilt from the disc to `0.0005` |
| `0x088327b8` | function | `Cage_TestPoint` | 88 | decompiled; live contact reproduced from the live slot and corners |
| `0x08831104` | function | `Cage_SectionLeft` | 85 | decompiled, part of the live slot match |
| `0x08831288` | function | `Cage_SectionRight` | 85 | the same |
| `0x0883140c` | function | `Cage_SectionTop` | 85 | the same |
| `0x0883158c` | function | `Cage_SectionBottom` | 85 | the same |
| `0x0887d36c` | function | `AiTrack_CageEnabled` | 72 | one instruction pair, `*(track + 4) + 0x18 & 1`; `track + 4` as the header is inferred, the census (Pulse 1, Pure 0) agrees |
| `0x0897e140` | function | `fabsf` | 95 | clears the sign bit of `f12` |
| `0x08ab0c8c` | data | `g_cage_enabled` | 85 | set at `0x08886ef8`, cleared in `Cage_ResolveCraft`, read at `0x08842370` |
| `0x08a7abd8` | data | `g_cage_max_depth` | 90 | `0x41700000`, `15.0`, read by `Cage_ResolveCraft` |
