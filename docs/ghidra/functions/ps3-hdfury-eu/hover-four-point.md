# HD's four-point hover, and the craft terms read beside it (2026-10-09, hd-pitch-airbrake)

Read with capstone past Ghidra's AltiVec truncation (a throwaway disassembler, not kept;
TOC `r2 = 0x008ad4d8`, so `-0x4358(r2)` is `0x008a9180`) and checked against the per-frame craft
dumps of `scripts/rpcs3-trace.py` (the hd-handling lane's game-time-keyed captures, not kept in
the repository; the command is in each `verification/scenarios/hd-*.inputs` header): Racebox
Time Trial, Talon's Junction, Venom, Feisar concept1. The measured comparison is
[hd-handling-ground-truth.md](../../../physics/hd-handling-ground-truth.md); the craft's fields
are on [physics.md](physics.md#the-handling-update-read-against-a-live-trace-2026-10-08-hd-handling).

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x000ede88` | `Craft_HoverFourPoint` | 88 | static read below; per-probe heights `craft+0x364..+0x370` and their mean `craft+0x354` read live; the same `0.15` and `0.25` as Pulse PS2's `Ship_HoverFourCorner`; this engine reproduces HD's pitch once it flies the law |
| `0x000edb50` | `Craft_UpdateSteering` | 85 | static; its ramp `craft+0x314` read live at `+8.3` a frame (`<Turning gain="500">`) |
| `0x000eda30` | `Craft_ApplyAngularDamping` | 75 | static; constants match Pulse's law |
| `0x000eff78` | `Craft_ApplyLateralGrip` | 78 | static; Pulse's law term for term |
| `0x000f0700` | `Craft_UpdateBrake` | 72 | static; gated on both airbrakes |

## `Craft_HoverFourPoint` at `0x000ede88`

`Craft_IntegrateHull` (`0x000ef450`) copies the body's basis and position into the craft
(`craft+0x90..0xc0` rows, `+0x200` up, `+0x220` forward, `+0x240` position), places four hull
points `craft+0x120..0x150` from the body-local offsets at `craft+0xe0..0x110`, builds each query
point `craft+0x160..0x190 = hull point - up * craft+0x344` (the hover target), and calls
`Collision_MarchSegment` (`0x000364c0`) once per probe, **with no speed test**: the four calls at
`0x000ef6c8`, `0x000ef7d0`, `0x000ef8d8`, `0x000ef9f0` run every frame the craft runs. Hit bytes go
to `*(craft+0x2b4) + i`, hit records to `*(craft+0x2b8) + 0x60 * i` (`+0x00` point, `+0x10` normal,
`+0x20` triangle id, `+0x24` a per-hit value, `+0x28` a class). Two more marches follow (records
4 and 5).

`Craft_HoverFourPoint` loops over the four (`0x000edfd0`-`0x000ee000`):

    h       = dot(hull_i - hit.point, up)                     -> craft+0x364 + 4i
    escape  : h < 1.0 and hit+0x28 == 1 (mesh tag) and maglock != 1.0 and class != 2
              -> body moved by up * (1 - h)                    (0x000f5ca0)
    contact : surface class 1 or 3
      grounded (craft+0x304) += 0.25                         (0x008a9180)
      normal_sum (craft+0x1e0) += hit.normal
      vn      = dot(v_body + basis * (offset_i x omega), hit.normal)
      d       = clamp(-0.1 * vn, -1, 2)                        (0x008a911c, 0x008a9110, 0x008a9194)
      reb     = class rebound (1.0 in the grid state or under flag 0x40),
                the landing blend below 0.2 s                  (0x008a9178, -5.0, 0.5)
      load    = K * (0.75 * grounded_prev + 0.25) * mass       (K through 0x008a9174)
      spring  = (reb * d * load + load) * (gravity * (target - h)) * 0.15 * (1 - maglock)
                                                               (0.15 at 0x008a917c)
      force   = spring * hit.normal  at  hull_i - craft+0xd0 + body position   (0x000f6588)

`gravity` is the class block's `+0x64 + +0x6c` (`normal_gravity + track_gravity`). After the loop
(`0x000ee02c`-`0x000ee188`): `craft+0x354` (summed heights) and the normal sum are scaled by `0.25`
unless exactly one probe touched; the alignment torque is `cross(up, normal) * -400` with the right
axis `craft+0x210` projected out (`0x008a9184`, world accumulator `0x000f5848`); bank-to-yaw is
`30 * craft+0x214 * (1 - maglock)` once the craft has left the grid state (`0x008a9198`,
`0x000ee6bc`); the downforce is `-track_gravity * mass * grounded * (1 - maglock)` along the normal
(`0x000ee108`). A trailing 10-unit ray (`0x000ee510`, only when the second argument is set) pushes
by `(3 - d) * 0.5 * up` for a hit between 3 and 10 units; not ported.

**What differs from Pulse's racing hover** (`Ship_HoverTwoPoint`, `oag_physics::hover`):

| | Pulse | HD |
| --- | --- | --- |
| probes | two, `(0, -1.125, +/-4.5)` | four, `(+/-1.5, -1.125, +/-4.5)` |
| share per spring | `0.3` | `0.15` |
| rear probe above 50 units/s | derived from the front hit, slope gain `6.0` | cast |
| spring direction | craft up | hit normal |
| grounded per contact | `0.5` | `0.25` |
| contact normal | mean, normalised | sum times `0.25` |

Everything else (the damper, the landing blend, the load factor, `K`, the alignment gain and its
projection, bank-to-yaw `30`, the downforce) is the same law.

**Why HD pitches shallower.** Vertical stiffness is equal (`4 * 0.15 == 2 * 0.3`) and so is the
pitch stiffness the springs give on paper (`4 * 0.15 * 4.5^2 == 2 * 0.3 * 4.5^2`). Pulse's derived
rear hit is the difference: it places the rear surface `6 * sin(pitch)` along up from the front
where the probes are `9` apart, so the two-point craft at speed keeps two thirds of the geometric
pitch stiffness, and HD, casting every probe, keeps all of it. The live dumps agree: HD's resting
mean height is `2.85` on a `4.125` target (`1.27` compressed, ours `1.237`), so `K` and the load are
the same, and during the pitch holds the front-rear height difference `craft+0x364 - +0x368` is
`0.64`-`0.84` either way, a symmetric `4.3`-`4.7` deg relative to the floor.

Wired as `oag_title::hover_rig::HoverRig` on HD's `RaceDefaults` (`oag_hd::race::HOVER_RIG`), carried
into `oag_physics::hover::Rig`; titles with `None` fly Pulse's two-point law. Pinned by
`oag_physics::hover::rig::tests` (the derived rear hit leaves `2/3` of the cast pitch stiffness;
four springs at `0.15` carry what two at `0.3` carry).

## The escapes sum (2026-10-20, hd-hover-residual)

Confidence **90** (static loop read plus a live position residual; under player input on two classes since 2026-10-20, below).

The loop at `0x000edfd0`-`0x000ee000` takes each probe whose hit byte is set to `0x000ee218`,
where `h = dot(craft+0x120 + 0x10 * i - hit.point, up)` (`0x000ee220`-`0x000ee250`, stored to
`craft+0x364 + 4i`). The hull points `craft+0x120..0x150` are written by `Craft_IntegrateHull`
before the loop and nothing in it rewrites them. Under `1.0` (`0x000ee25c`, `blt 0x000ee628`):

- `hit+0x28 != 1` (not a mesh hit, which no probe hit is; see below) or `craft+0x2c0 == 1.0`
  (full magnet lock): no escape,
  on to the contact test at `0x000ee268`;
- the hit triangle's material type (`+0x74`, through `*(craft+0x35c)`) is `2`: the probe is
  skipped outright, no escape and no spring (`beq 0x000edfec` at `0x000ee668`);
- otherwise the body moves by `up * (1 - h)` (`bl 0x000f5ca0` at `0x000ee6ac`), then on to the
  contact test (`b 0x000ee268`) and the next probe.

So every penetrating probe moves the body by its own `1 - h`, all off heights fixed before the
loop: the moves **sum**. Ours took the largest alone (a choice, for Pulse's two probes).

**Live.** One `oag_physics::step` per row of an RPCS3 capture of the original's Autopilot on
Ubermall PHANTOM (Feisar, rows 400-1260, `crates/raceplay/tests/hd_air_pose_walk.rs` with
`OAG_HD_DIAG=1`, `OAG_HD_AIR=data/reference/hd-hover-residual/ubermall-phantom-autopilot.csv
OAG_HD_ROWS=400..1260 OAG_HD_BIAS=1 OAG_HD_AUTOPILOT=1`; the walk refuses Autopilot rows without
the last since 2026-10-20): 87 of 447 rows escape. Our position residual along up on those rows fit
`dp_up = -0.954 * (sum - max) + 0.015`, correlation `-0.996`, where `sum - max` is what summing
adds. Ported as `oag_title::hover_rig::HoverRig::sum_escapes` (HD measured `true`), carried into
`oag_physics::hover::Rig::sum_escapes`:

| Escape rows (87) | dp_up median | p5 | p95 | mean abs |
| --- | ---: | ---: | ---: | ---: |
| largest only (before) | `+0.0118` | `-0.3412` | `+0.0241` | `0.0480` |
| summed (after) | `+0.0148` | `+0.0096` | `+0.0346` | `0.0177` |

A `+0.015` bias stays on escape rows (non-escape rows: `+0.0017`); not attributed.

**Re-measured under player input (2026-10-20, hd-air-player-input).** The run above was flown
by the original's Autopilot, whose rail writes the craft's velocity. Fourteen placed
Time Trial captures with the throttle held and no Autopilot
(`data/reference/hd-air-player-input/`, method in
[airborne-ground-truth.md](../../../physics/airborne-ground-truth.md#wipeout-hd-under-player-input-2026-10-20-hd-air-player-input))
give 131 escape rows (PHANTOM Feisar and Venom Harimau on Ubermall). Largest-only fits
`dp_up = -0.955 * (sum - max) + 0.034`, correlation `-0.874`; summed, the median is `+0.025`
(p5 `+0.006`, p95 `+0.093`, mean abs `0.042` against `0.227`). The sum stands; **confidence
90**. The `+0.02` escape-row bias is there too. The material
type `2` skip and the magnet-lock gate are not ported (no row of this capture reaches them).
Pulse and Pure keep the largest-only choice: whether Pulse's loop re-reads its heights between
escapes is unread. **Omega and 2048: not checkable in this lane.** Both fly Pulse's two-point rig
(`hover_rig: None`) until their own springs are read; the sum rides on the rig, so it waits for that: the loop the law lives in is not read on
Omega yet, and 2048's hover is not located. Omega's `Craft_UpdateSurfaceProbes` (`0x0131b510`) carries HD's probe layout forward, so
the same loop is the expected reading there.

**The escape does not wait for contact (2026-10-20, `pulse-air-pitch`).** The translate at
`0x000ee6ac` comes before the contact test at `0x000ee268`, so a probe under `1.0` moves the body
on a frame where no probe ends up in contact. Ours dropped every escape on such a frame until
Pulse's crest showed the original lifting off one
([airborne-ground-truth.md](../../../physics/airborne-ground-truth.md#the-1595-crest-residual-is-a-hover-escape-off-a-wall-2026-10-20-pulse-air-pitch));
HD now takes it too (read statically here, not seen live on HD).

## `hit+0x28` is the march's mesh-shape tag (2026-10-21, `deviations-to-original`)

Confidence **88** that `+0x28` is a constant mesh-shape tag (one writer, `1` on 247 of 247 live
hits); **80** that HD's escape therefore fires off a wall, which also rests on class `0` being the
wall class (the numbering below) and was not seen live.

**Static.** `Collision_MarchSegment` (`0x000364c0`) copies its result into the caller's record
at `0x00036cbc`-`0x00036d84`: `+0x00` from its local `sp+0x170`, `+0x20` from `sp+0x1a0` (the
triangle id), `+0x24` from `sp+0x1a4`, and **`+0x28` from `sp+0x1a8`** (`lwz r0,0x1a8(r1)` at
`0x00036d1c`, `stw r0,0x28(r11)` at `0x00036d44`). The one store to `sp+0x1a8` in the function is
`li r0,0x1` / `stw r0,0x1a8(r1)` at `0x00036bb4`-`0x00036bb8`, on the path where a triangle test
(`bl 0x0007a900`) found a nearer hit. That path is reached only for an object whose shape type
`+0xc` is `1` (`0x0003677c`-`0x00036784`); a type-`3` object branches to `0x00036c80` and
`bl 0x000336b0`, which never touches the local record. So `+0x28` is a constant `1` on every hit
the march returns, the same shape tag Pulse's `Collision_RaycastMesh` writes into `craft+0x208`
([collision.md](../psp-pulse-usa/collision.md#raycasts)), and not a floor class.

The class the hover tests separately is the hit object's `+0x74`, resolved through
`*(craft+0x35c)` and the record's `+0x20`: contact on `1` or `3` (`0x000ee290`-`0x000ee2a0`),
the escape skipped on `2` (`0x000ee660`-`0x000ee668`). The march itself skips class-`2` objects
unless its byte argument at `sp+0x600` is set (`0x000367c8`-`0x000367d4`). That is Pulse's
`Surface` numbering, `1` Floor, `3` MagFloor, `2` Reset (excluded from ordinary raycasts), `0`
Wall; the numbering is read off the two uses, not off HD's loader (confidence 80).

**Live**, RPCS3, `hd-timetrial-talons-venom-start` (Racebox Time Trial, Venom, Feisar), PPU
interpreter, cross held and the stick swung left and right every 1.5-2 s, Autopilot never armed
(`scripts/rpcs3-hd-escape-gate.py`):

| Breakpoint | Stops | `+0x28` | class `+0x74` | shape `+0xc` |
| --- | ---: | --- | --- | --- |
| `0x000ee628`, the escape gate (`h < 1.0`) | 13 | `1` on all | `1` on all | `1` on all |
| `0x000ee294`, the contact class test (`--contacts`) | 234 | `1` on all | `1` on all | `1` on all |

Every hit was a floor: the run never put a wall or a magstrip under a probe, so the live leg
shows the tag constant across 247 hits and the static read carries the claim that a wall's
would read the same. Magnet lock read `0.0` on all 13 escape stops.

**Ported.** HD escapes off a `Wall` as off a `Floor`, and off a `MagFloor` unless the magnet lock
is full (`oag_title::hover_rig::MagFloorEscape::BelowFullLock`, carried into
`oag_physics::hover::Rig::mag_floor_escape`). It escaped off `Floor` alone before, a choice made
while the field was unread. The material-type-2 skip needs no port: our raycasts already leave
out `Reset`. Gates, before and after: the Pulse lone-craft gate twelve of twelve clean both
times (Pulse's rig is unchanged); HD's twelve `lone_opponent_on_*` pass with no wall tick before
or after, two first laps moving `0.02` s (The Amphiseum `53.85 -> 53.83`, Talon's Junction
`46.70 -> 46.68`); all 247 `hd_` tests in `oag-raceplay` pass. **Omega and 2048: unaffected**:
both fly Pulse's two-point rig (`hover_rig: None`), whose gate was already Pulse's.

## The grounded vertical residual was the harness (2026-10-20, hd-hover-residual)

The `0.95` units/s per tick a grounded row landed lower than the original (2026-10-19) was the
pose walk seeding every row from a craft ticked once **on the grid**, where HD caps the hover
target at `0.75 * 3.0x` ([hover-target.md](hover-target.md)): every probe reached `2.28` instead
of `4.125`, so 191 of 423 grounded rows found no floor and the rest sprang from the wrong target.
The capture itself is past the release from its row 20 (no cap fits rows 20-300 at `0.000`; a
ramp from row 0 at `-0.53`). With the cap released:

| Rows 400-1260, Ubermall PHANTOM | grounded (423) dv_up median | airborne (24) |
| --- | ---: | ---: |
| grid cap seeded (2026-10-19) | `-0.950` | `0.000` |
| cap released | `-0.010` (IQR `-0.03..+0.01`) | `0.000` |
| cap released, `OAG_HD_GRAV=ground` | `-0.020` | `+0.380` |

No hover law was missing. The two class-gravity readings do not separate on grounded rows (inside
the IQR); the airborne rows keep the airborne term. No wall or floor contact reached the resolver
on any of the 447 rows, so HD's floor restitution is not exercised here. `craft+0x358`, the
trailing 10-unit ray (`0x000ee510`) and the escape's class gates are not needed to close the
median, and stay unported.

## The terms the one-airbrake gap is not in

Read to find why one airbrake turns HD about 10% less than this engine. Closed 2026-10-09: the gap
is HD's inertia mass, and `Craft_UpdateSteering`'s ramp is clamped where Pulse's is not; both on
[craft-inertia.md](craft-inertia.md). Each term below is otherwise Pulse's law:

- **`Craft_UpdateSteering` (`0x000edb50`)**: ramps `craft+0x314` toward the control's `+0x00` at
  the class block's `+0x38` up / `+0x40` down, then `torque.y = ramp * block+0x3c (Turning amount)`
  into the body's local torque accumulator (`0x000f5dd8`, `body+0x170`). The same accumulator the
  airbrake's yaw reaches.
- **`Craft_ApplyAngularDamping` (`0x000eda30`)**: local torque
  `(-block+0x78 * w.x, -5 * w.y, c * w.z)` of `body+0x1c0`, `c = -5` in the grid state and `-2`
  after (`0x008a9150`, `0x008a9154`).
- **`Craft_ApplyLateralGrip` (`0x000eff78`)**: returns while `craft+0x2d8 > 0` (counting it down);
  `k = max(L, R) * (0.01 - slidegrip) - 1`, local `x += k * dot(v, right) * (grip_ground * grounded
  + grip_air * (1 - grounded))` into `body+0x160`, grips times `1.5` under flag `0x40`
  (`0x008a9204`); also accumulates `max(0, 0.5 * |dot(v, right)| - 3)` into `craft+0x2e4`.
- **`Craft_UpdateAirbrakes` (`0x000ee730`)**: the ramps advance by `dt * gain` and clamp to
  `0..100` only (no clamp to the target), and the yaw is
  `(speed * R * turn - speed * L * turn) * 0.001` into lane y of a zero vector (`0x000ee95c`): as
  on [physics.md](physics.md#craft_updateairbrakes-at-0x000ee730).
- **`Craft_UpdateBrake` (`0x000f0700`)**: runs only with **both** airbrakes above zero, so not in a
  one-sided turn.
- **The hover rig**: flying HD's four-point rig moves the airbrake response by under 2%.

Per frame, HD's body yaw rate (`body+0x1a0`) rises by `0.0078`, `0.0138`, `0.0209`, `0.0265`
rad/s as the right airbrake ramps `13.5`, `26.7`, `40.2`, `53.5` at `86` units/s, which is 3-12%
under `speed * R * 9 * 0.001 / 21.6` (`I_yy`) less `5 * w` damping: HD's `I_yy` is `24`, not
`21.6` ([craft-inertia.md](craft-inertia.md)).

## Omega and 2048

- **Omega**: checked, applies, not wired. `Craft_UpdateSurfaceProbes` (`0x0131b510`) marches four
  hull segments `+0x180..+0x1b0` to `+0x1c0..+0x1f0` with no speed test, hit bytes at
  `+0x324..+0x327` and `0x60`-byte records from `+0x330`, then three more: HD's layout carried
  forward. Its spring's share and probe offsets are not read; Omega's race keeps `hover_rig: None`
  until they are.
- **2048**: not checked in this pass (the Vita executable is open in Ghidra; its hover was not
  located).

## Zone: the same law plus the trailing ray, and the handling file's one class (2026-10-20, zone-hover)

Static read, for the cross-title check of Pulse's Zone hover
([zone-hover.md](../psp-pulse-usa/zone-hover.md)).

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x000f18b8` | `Craft_UpdateHover` | 82 | clears bit 0 of `craft+0x264`, then calls `Craft_HoverFourPoint` with `r4 = 0`, or with `r4 = 1` when the debug byte (`-0x4328(r2)`) is clear and the game mode (`*(-0x43ac(r2)) + 0xe0`) is under `0x16` with its bit set in `0x206040`; the shape of Pulse's `Ship_UpdateHover` |
| `0x000c8330` | `Handling_ParseFile` | 72 | `Team_LoadHandlingStats`'s only parse call; walks `<Stats>`'s children through one name dispatch (`Handling_ReadGlobalClasses` among them) and, for `<Class>`, sets the current class index and walks its children through the same block readers |

**HD's Zone hover is its racing hover plus the trailing ray.** The mask `0x206040` is modes 6, 13,
14 and 21; 6 is Zone, 13 Zone Battle and 14 Detonator (read live, [billboards.md](billboards.md)), 21
not identified. `Craft_HoverFourPoint`'s second argument gates only the trailing 10-unit ray
(`0x000ee510`, `(3 - d) * 0.5` for a hit between 3 and 10, above): HD keeps its four-point probes,
its `0.15` spring with load factor and rebound, its projected alignment and its scaled downforce in
Zone. The ray's gain is `0.5` where Pulse's is `0.25`; its axis was not re-read here. Not wired
by this lane; read whole, verified live and wired on 2026-10-20 (the `hd-zone-handling` section
below).

**Which class HD's classless Zone file fills.** `Team_LoadHandlingStats` reads
`Data\Ships\Zone\handlingstats.xml` for the same mask ([opponent-handling.md](opponent-handling.md)),
and that file authors every block (`Physical`, `Antigrav`, `Engine`, `Brakes`, `Turning`,
`Airbrake`, `Tilt`, `Pitch`) directly under `<Stats>`. `Handling_ParseFile` zeroes the current class
index (`*(-0x4e64(r2)) + 0x10`) before the walk, the block readers write at
`stats + index * 0x80 + offset` (`0x000c6310` is one: `+0xc0..+0xd4`), and only a `<Class>` element
moves the index. So a classless file fills **class 0 alone**; classes 1 to 3 keep whatever the
stats block held before. Confidence **70** for that (decompile of the parser and one block reader).
Which class index a Zone race then reads, and so whether a Zone craft flies these numbers in every
class, is **not determined**: the next read is the class index `Craft_Update` uses for the stats
block in mode 6, or an RPCS3 read of `engine->stats` in a Zone race. (Read live on 2026-10-20: class 0,
the Zone file's numbers; the `hd-zone-handling` section below.)

## Zone, read live: the ray, the class, the grid and the brakes (2026-10-20, hd-zone-handling)

Static reads plus one RPCS3 capture: a cold boot straight into Racebox Zone (`TTY.log`:
`BackendRoot has g_GameState.GetMode()==6`), Vineta K, the player's craft entry (`ship+0x5fac`,
`0x600` bytes), its class block (`*(entry+0x7c)`) and team block (`*(entry+0x78)`) read through
`/proc/<pid>/mem` at 10 Hz, no pause: 21 flyby samples, 81 through the countdown, 401 over 50 s of
flight. Cold boot first is deliberate: `Team_LoadHandlingStats` loads a team's file once
(`team+0xb8`), so a racing mode run first in the session would leave the racing file in place.

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x000c6550` | `Handling_ReadAntigrav` | 85 | the `<Antigrav>` block reader: one string dispatch over six TOC names (`-0x4dd8` `rebound` -> class `+0x04`, `-0x4dd4` `landing_rebound` -> `+0x08`, `-0x4dd0` `rebound_jump_time` -> `+0x0c`, `-0x4dcc` -> `+0x00`, `-0x4dc8` -> `+0x10`, `-0x4dc4` -> `+0x14`), each stored at `block + index * 0x80 + 0xa0 + offset` with `index = *(*(-0x4e64(r2)) + 0x10)`; live, the class block is the team block plus `0xa0` and `+0x00`/`+0x04`/`+0x10`/`+0x14` hold the file's `ride_height`/`rebound`/`grip_ground`/`grip_air` |

**The trailing ray, read whole (confidence 85).** `r18` is `Craft_HoverFourPoint`'s second
argument (`or r18,r4,r4` at `0x000edf44`); `r25`, `r27` and `r31` are all the craft. After the probe
loop both paths reach the ray: with a contact through the epilogue (`0x000ee188`), with none through
`0x000ee4e8`. Ghidra's listing is misaligned at `0x000ee52c` and `0x000ee5e0`-`0x000ee5e8`; the words
were decoded from the raw bytes (`7c004c0e` `lvewx v0,0,r9`; `7c204c0e` `lvewx v1,0,r9`, `10200a8c`
`vspltw`, `d0010100` `stfs f0,0x100(r1)`).

    0x000ee510  r9 = TOC-0x4350 -> 10.0 (0x008a9188), splatted
    0x000ee548  end = craft+0x240 (centre) - craft+0x200 (up) * 10
    0x000ee560  Collision_MarchSegment(craft+0x35c, centre, end, hit at r1+0xa0, body+0x4cc, 1, 2)
                (the four probe marches pass r8 = 0; what r8 = 1 changes is not read)
    0x000ee594  d = dot(centre - hit.point, hit.normal)
    0x000ee5b0  skip unless 3.0 < d < 10.0   (3.0 at 0x008a919c, 10.0 at 0x008a9188)
    0x000ee5f4  move = hit.normal * (3.0 - d) * 0.5   (0.5 at 0x008a918c, TOC-0x434c)
    0x000ee614  Body_Translate(body, move)   (the probe escapes call it at 0x000ee6ac)

Pulse's `Ship_HoverFourCorner` snap term for term, gain `0.5` against `0.25`. **Live**: the mean probe
height (`craft+0x364..0x370`) reads `2.077` through the flyby, median `2.041` (`2.007`-`2.077`)
through the countdown and median `2.117` (tenth to ninetieth percentile `2.019`-`2.207`) in flight,
on a `4.125` target; the racing rig rests at `2.85` (above). With the probes `1.125` below the centre
that puts the centre at `3.17`-`3.24` above the floor: held just above `3.0` by a half-gap pull every
frame against a spring that would rest it at `4.0`. The breakpoint at `0x000ee614` was not set (the
height is the measurement); confidence 85 rests on the instructions and the height agreeing.
**Ported**: `oag_hd::race::ZONE_HOVER_RIG`, `oag_physics::hover::Rig::snap` now the gain.

**The brakes (confidence 82, static).** `Craft_Update` (`0x000f1958`): at `0x000f1e78` the debug
byte is tested, then the mode at `0x000f1e84`, and `0x000f27c8`-`0x000f27e0` (`lis 0x20; ori 0x6040;
sld; and`) branches past `Craft_UpdateBrake` (`bl 0x000f0700` at `0x000f28c0`) for a mode in the
mask. Pulse's Zone skips its brakes the same way. Ported as `ZONE_HOVER_RIG.brakes_off`.

**The grid (confidence 90).** `0x000f1ca0` branches to `0x000f2390`, which sends modes `6`, `0xd`
and `0xe` (not 21) straight to the scale at `0x000f1ce8`, past both the grid clamp
(`0x000f1cb4`-`0x000f1cc4`) and the raced ramp (`0x000f23bc`). Live, mode 6: `entry+0x344` reads
`4.125` on every sample of the grid state (`entry+0x2f8 = 0`, flags `3`) and after it, and
`entry+0x348` holds `4.0` unread. So a Zone craft waits on the grid at the ray's height, not the
lowered hover ([hover-target.md](hover-target.md)). Ported as
`oag_title::launch_hover::LaunchHover::in_zone` (HD `false`).

**The class (confidence 90, live).** `craft+0x7c` is `craft+0x78 + 0xa0`: class 0. It holds the
Zone file's numbers: `+0x24` engine gain `900`, `+0x28` amount `0.3`, `+0x2c` falloff `500`, `+0x30`
accelcap `20`, `+0x10` grip_ground `16`, `+0x5c` sideshift `550`, `+0x58` slidegrip `0.01`,
`+0x50` turn `10`, `+0x60..+0x6c` `<Physical>` `1, 5, 120, 100`. Class 1 (`+0x80`) is empty but for
`+0x0c = 2.0`, `+0x58 = 0.01` and `+0x78 = 1.5`. The team block is the Zone file as well:
`+0x78` airbrake graphics `0.5236` (30 deg), `+0x80` down speed `50`, `+0x84..+0x90` Misc
`5, 13, 3.5, 100`, `+0x9c` weight_distribution `0.0` (unauthored). The file authors no
`landing_rebound` or `rebound_jump_time`; class `+0x08` and `+0x0c` read `0.0` and `2.0`, the values
class 1's untouched block holds. So **a Zone craft flies the Zone file in every class**, which
closes the question the section above left open. **Ported**: `oag_hd::race::ZONE_HANDLING`
(`oag_title::zone_handling`), read by `oag_tables::handling::parse_classless`.

**Bank-to-yaw.** Zone runs this same function, so its epilogue's `30` (`0x008a9198`) is the Zone gain
too, not Pulse's four-corner `50`. **Ported**: `CraftLaws::bank_to_yaw_zone`, HD measured `30`.

**Wall friction under the mask: not measured.** `Collision_PairFriction` (`0x000be068`) returns
`*(table + 0x38)` for a craft-wall pair (`(0 or 1, 3)`) when the net byte `*0x008a83fc` is clear and
the mode is in the mask (`0x000be1a4`, `0x000be1fc`), `*(table + 0x2c)` otherwise, `table` the
pointer at `0x008a83ec`. The capture above hit no wall it could isolate and set no breakpoint. Next:
an interpreter boot to mode 6, a `Z0` on the return of `0x000be068` during a wall scrape, read `f1`.

**The mask's other sites in the craft code.** A search for `ori rX,rY,0x6040` (any register; the
mask is built `lis 0x20` then `ori 0x6040`) finds 25 word-aligned sites between `0x000d0000` and
`0x000f9000` plus `0x001e7110`. Read in this lane: `0x000f18f8` (`Craft_UpdateHover`, the selector
above), `0x000f2740` (`Craft_Update` calls `Craft_HoverFourPoint(craft, 1)` itself under the mask,
`bl 0x000ede88` at `0x000f275c`: a second route to the same ray), `0x000f27d0` (the brakes, above),
`0x001e7110` (`Team_LoadHandlingStats`, the Zone file, [opponent-handling.md](opponent-handling.md)).
Open, address only: `0x000efec0` in `Craft_UpdateThrottle` (the Zone engine; this engine's
auto-speed law was not compared); `0x000eed84` in `FUN_000eeb70`, which sets `ship+0x6a64 |=
0x04000000` under the mask (the flag's reader is unread); `0x000e7d00` (`Ship_ApplyDamage_q`),
`0x000e8d58` (`FUN_000e8b38`), `0x000e3f04` (`FUN_000e3d18`), and `0x000e2260`, `0x000e2bf8`,
`0x000e504c`, `0x000e50cc`, `0x000ec7e0`, `0x000ec988` (code Ghidra holds no function for); the rest
are in the ship constructors, the engine sound and the model reload. Nothing beyond the four read
sites is wired.

### The other titles (hd-zone-handling)

- **Omega**: checked, applies, not wired. `Data/art/published/hdships/zone/handlingstats.xml` is HD's
  file value for value (`ZoneMode`, no `<Class>`, accelcap `20`, `Antigrav` without
  `landing_rebound`). Omega's executable keeps the mask as an `imm32` (`mov edx,0x206040; bt` at
  `0x0150bb28` in `FUN_0150b8f0`, and in `FUN_015314b0` and `FUN_0157f360`); a compiler that folds
  the test another way would not show up in that search, so three is a floor. Its hover selector was
  not located. Omega's race flies Pulse's two-point rig (`hover_rig: None`), so HD's ray
  on it would be a hybrid nobody measured: not wired. Its Zone bank-to-yaw is inherited from HD (`30`).
- **2048**: not checkable here. The v1.04 `eboot.elf` holds no `0x206040` literal (an ARM build would
  split it into `movw`/`movt`; not searched). The base package's ships are PFS-encrypted; the HD pack
  (`2048-PCSF00007-dlc1.vpk`) ships `data/art/published/hdships/Zone/handlingstats.xml`, HD's file.
  Zone bank-to-yaw inherited from HD (`30`).
