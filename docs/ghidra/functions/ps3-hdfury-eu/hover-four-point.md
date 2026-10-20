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
    escape  : h < 1.0 and hit+0x28 == 1 and maglock != 1.0 and class != 2
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

Confidence **88** (static loop read plus a live position residual; one circuit, one class).

The loop at `0x000edfd0`-`0x000ee000` takes each probe whose hit byte is set to `0x000ee218`,
where `h = dot(craft+0x120 + 0x10 * i - hit.point, up)` (`0x000ee220`-`0x000ee250`, stored to
`craft+0x364 + 4i`). The hull points `craft+0x120..0x150` are written by `Craft_IntegrateHull`
before the loop and nothing in it rewrites them. Under `1.0` (`0x000ee25c`, `blt 0x000ee628`):

- `hit+0x28 != 1` (not the floor class) or `craft+0x2c0 == 1.0` (full magnet lock): no escape,
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
OAG_HD_ROWS=400..1260 OAG_HD_BIAS=1`): 87 of 447 rows escape. Our position residual along up on those rows fit
`dp_up = -0.954 * (sum - max) + 0.015`, correlation `-0.996`, where `sum - max` is what summing
adds. Ported as `oag_title::hover_rig::HoverRig::sum_escapes` (HD measured `true`), carried into
`oag_physics::hover::Rig::sum_escapes`:

| Escape rows (87) | dp_up median | p5 | p95 | mean abs |
| --- | ---: | ---: | ---: | ---: |
| largest only (before) | `+0.0118` | `-0.3412` | `+0.0241` | `0.0480` |
| summed (after) | `+0.0148` | `+0.0096` | `+0.0346` | `0.0177` |

A `+0.015` bias stays on escape rows (non-escape rows: `+0.0017`); not attributed. The material
type `2` skip and the magnet-lock gate are not ported (no row of this capture reaches them).
Pulse and Pure keep the largest-only choice: whether Pulse's loop re-reads its heights between
escapes is unread. **Omega and 2048: not checkable in this lane.** Both fly Pulse's two-point rig
(`hover_rig: None`) until their own springs are read; the sum rides on the rig, so it waits for
that. Omega's `Craft_UpdateSurfaceProbes` (`0x0131b510`) carries HD's probe layout forward, so
the same loop is the expected reading there.

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
