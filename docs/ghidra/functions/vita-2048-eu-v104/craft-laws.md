# 2048's craft laws: the box mass, the wall point rate and the shield slot (2026-11-11, craft-laws)

Three laws `oag_2048::race::CRAFT_LAWS` carried as inherited or held, read in the v1.04
executable (Ghidra `/2048/eboot-vita-2048-eu-v104.elf`, ARM Thumb-2, image base `0x81000000`).
**Static reading only, no Vita3K run**, so every score here is capped at 84
([rubric](../../../reverse-engineering/confidence-rubric.md)). HD's side, with its live
captures, is on [craft-inertia.md](../ps3-hdfury-eu/craft-inertia.md) and
[wall-response.md](../ps3-hdfury-eu/wall-response.md); Omega's read of the same three is on
[ps4-omega-eu/craft-laws.md](../ps4-omega-eu/craft-laws.md).

**Verdict.** All three are HD's: the box is built with mass `1.0` (not Pulse's `0.9`), the wall
resolver crosses the world lever arm with the stored body-local rate unrotated, and
`Ship_SetShield` clamps to the `<Misc>` slot the level indexes. A campaign event's level is `1`,
the medium slot.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x811d23dc` | `Body_SetMass` | 82 | stores `s0` to `body+0x88` and its reciprocal to `+0x8c` and `+0x414`; called with `1.0` just before the box builder in both ship constructors |
| `0x811d35a8` | `Body_SetBoxInertia` | 84 | [below](#the-box-mass-is-10); HD's `0x000f5aa0` operation for operation |
| `0x811d32c8` | `Body_SetOrientation` | 65 | normalises two axes, completes the basis into `body+0x08`/`+0x18`/`+0x28` and writes its inverse to `+0x48`/`+0x58`/`+0x68`; arguments not traced to a caller's meaning |
| `0x811d3164` | `Body_ApplyImpulseAtPoint` | 80 | `v(+0x90) += p * invMass`; `L(+0x200) -= M48 * cross(r, p) * body+0x424`; `omega(+0x1a0) = M160 * L`; HD's `0x000f6348` |
| `0x811d30b0` | `Body_AddTorque` | 72 | the resolver's tail hands it the steer-away vector; HD's `0x000f5848` by position, body not read |
| `0x811d3eaa` | `Body_ResolveContact` | 84 | [below](#the-wall-point-rate-is-body-local) |
| `0x811d38f6` | `Body_ResolveContactPair` | 70 | the two-owner branch of the contact loop; only its point-velocity head was read |
| `0x811d41c4` | `Physics_StepWorld` | 78 | integrates each body (`0x811d2742`), then walks the contact array through `Collision_GetContact` into the two resolvers |
| `0x8118b884` | `Collision_GetContact` | 84 | `index < world+0x7440 ? world + 0x440 + index * 0x38 : 0` |
| `0x811a19fc` | `Collision_PairRestitution` | 84 | a switch over two material ids returning floats of one table; `0x8151fc34` for a craft (`0`) against ids `4`, `5`, `6` |
| `0x811ac8b6` | `Handling_ParseMisc` | 84 | [below](#the-shield-slot) |
| `0x811bb524` | `Ship_SetShield` | 84 | [below](#the-shield-slot) |
| `0x811c4296` | `Ship_RestoreShield` | 70 | calls `Ship_SetShield` with the slot's own value, for controller kinds `0` and `2` (`ship+0x58dc`) |
| `0x8151fc34` | `g_ShipWallCofR` (data) | 84 | written by `Handling_ReadGlobalSettings` on `shipWallCofR` (`0x811ad9aa`), read only by `Collision_PairRestitution` |
| `0x8153fd20` | `g_skill_level` (data) | 84 | `g_GameState+0xe0`: written by `GameModeBase_Apply` from `event+0x1a4` ([ai-controls.md](ai-controls.md)), indexed by `Ship_SetShield` and `AI_ResolveSkillScale` |

Not renamed: the two ship constructors `FUN_811b8806` and `FUN_811c82dc` (other pages of this
directory already cite them by address) and the integrator `FUN_811d2742` (HD's
`Body_Integrate` by position in the step loop; its matrix arithmetic was not decoded).

## The box mass is 1.0

Both ship constructors run the same sequence on the body at `ship[r6]`:

    0x811b973a  vmov.f32 s17, #1.0
    0x811b9742  vmov.f32 s0, s17
    0x811b9746  bl   0x811d23dc            ; Body_SetMass(body, 1.0)
    0x811b974a  vmov.f32 s1, #12.0         ; 0x41400000
    0x811b9752  vmov.f32 s0, s17           ; 1.0
    0x811b9756  vmov.f32 s2, #8.0          ; 0x41000000
    0x811b975a  vmov.f32 s3, s1            ; 12.0
    0x811b975e  bl   0x811d35a8            ; Body_SetBoxInertia(body, 1.0, 12, 8, 12)

and again at `0x811c8a58`..`0x811c8a7c` in `FUN_811c82dc`, register for register. It is HD's
`Ship_Construct` call (`0x000debc8`), a code literal and not the class's `<Physical mass>`.

`Body_SetBoxInertia` squares the three extents, sums them in pairs, multiplies each sum by
`s0`, divides `12.0` by each (`vdiv.f32`, `0x811d35e4`..`0x811d35f8`) into the diagonal of the
rows at `body+0x120`/`+0x130`/`+0x140`, and multiplies each product by `0x3daaaaab` (`1/12`,
`0x811d35c8`) into the diagonal at `body+0xa0`/`+0xb0`/`+0xc0`. So `I = m (b^2 + c^2) / 12`,
which at mass `1.0` is `(17.333, 24, 17.333)`; `0.9` would give a yaw entry of `21.6`.

The body's layout is HD's with the blocks reordered: HD's `+0x10`/`+0x50`/`+0x90`/`+0xd0`
matrices are here `+0xa0`/`+0xe0`/`+0x120`/`+0x160`.

**Confidence 84**: an immediate at two call sites and the builder's arithmetic, read
unambiguously; Omega's constructors hold the folded result of the same call. Not run live.

## The wall point rate is body-local

`Physics_StepWorld` (`0x811d41c4`) ends in the contact loop: a record with one owning body goes
to `Body_ResolveContact(world, contact, body)`, one with two owners (neither with bit `1` of
`+0x42c`) to `Body_ResolveContactPair`. A record is `0x38` bytes: point `+0x00`, unit normal
`+0x10`, the two colliders `+0x20`/`+0x24`, the other side's material `+0x28`, depth `+0x2c`,
friction `+0x30`, restitution `+0x34`.

`Body_ResolveContact` (`0x811d3eaa`), `r1` the contact, `r2` the body:

```text
r      = contact.point - body+0x38                         # 0x811d3eb2..0x811d3ec6
v_p    = body+0x90 + cross(r, body+0x1a0)                  # 0x811d3ebe..0x811d3f2a, no rotation of +0x1a0
vn     = dot(v_p, contact.normal)
if !(vn <= 0) return                                       # 0x811d3f7c vcle.f32 #0, cbnz
invMass = body+0x418 ? body+0x414 : body+0x8c
D      = invMass + dot(cross(M120 * cross(r, n), r), n)
if D == 0 return
j      = -(contact+0x34 + 1.0) * vn / D                    # 0x811d3f9a..0x811d3fa2
p      = j * n;  if contact+0x30 > 0 and v_t != 0: p -= contact+0x30 * v_t
Body_ApplyImpulseAtPoint(body, contact.point, p, 1)        # 0x811d3164
s      = -clamp(dot(body+0x08, p), -20.0, 20.0)            # the right row
Body_AddTorque(body, body+0x18 * s * 5.0)                  # the up row; 0x811d415e
```

That is HD's `0x000f7770` term for term: the one-sided gate, the `-(1 + e)` impulse, the
friction, and the `5 * clamp(.., -20, 20)` steer-away torque Pulse does not have.

**The frame of `body+0x1a0`, from a second call site.** `Body_ResolveContactPair`
(`0x811d38f6`) reads the same field for each of its two bodies and, before the cross product,
multiplies it through the basis rows:

    0x811d3960  vmul.f32 q10, q4,  d14[0]   ; row +0x08 * omega.x
    0x811d396c  vmla.f32 q10, q6,  d15[0]   ; row +0x28 * omega.z
    0x811d3980  vmla.f32 q10, q14, d14[1]   ; row +0x18 * omega.y

so the craft-craft resolver turns the stored rate from the body's axes into world axes, and the
wall resolver does not. The stored rate is therefore body-local, and the wall resolver's point
velocity is `v + (R^T omega) x r`, the mixed-frame expression HD's sixteen live wall hits
measured. `Body_ApplyImpulseAtPoint` agrees: it rotates the world angular impulse `cross(r, p)`
through the inverse basis (`+0x48`, written by `Body_SetOrientation`) before subtracting it
from `+0x200`.

**Confidence 82** for the body-local point rate: the unrotated cross product read directly,
the pair resolver's rotation as the second site, Omega's two resolvers showing the same
contrast, and HD's live measurement of the same code. Short of 84 because the integrator that
defines `+0x1a0` was not decoded and nothing was run on Vita3K. **84** for the one-sided gate
and the steer-away constants.

**The earlier hold was this engine's AI, not 2048.** `wall_point_rate_body_local` was
`chosen(false)` because with the law on (and the `0.9` mass)
`vita_2048_original_ai_ground_truth::the_field_finishes_tower` left the field a lap down at
300 s. With the measured mass `1.0` the same test finishes with the law on: `2048 - Event 1`
in 277.6 s, all seven opponents on lap 4, no respawns, wall-contact ticks `0 0 12 0 0 0 91`.
The hold is lifted.

## The shield slot

`Handling_ParseMisc` (`0x811ac8b6`) reads `<Misc>`: `shield` to `stats+0x98`, `+0x9c` and
`+0xa0` together, then `easyshield` to `+0x98`, `mediumshield` to `+0x9c`, `hardshield` to
`+0xa0`. Pulse's layout (`0x84`/`0x88`/`0x8c`, [shield.md](../psp-pulse-usa/shield.md)) rebased.

`Ship_SetShield` (`0x811bb524`), `s0` the amount, `r0` the ship:

    0x811bb534  ldr   r1, [r4, #0x5640]
    0x811bb53a  ldr.w r2, [r1, #0x80]            ; the craft's stats block
    0x811bb542  ldr.w r0, [0x8153fc40 + 0xe0]    ; g_skill_level
    0x811bb546  add.w r2, r2, r0, lsl #2
    0x811bb564  vldr  s1, [r2, #0x98]            ; the slot
    0x811bb56c  vcmpe s0, s1 / vmovgt s16, s1    ; amount = min(amount, slot)
    0x811bb58e  vst1  [ship + 0x5634]            ; the shield

One global for every craft: there is no controller branch before the clamp, so the player and
the opponents read the same slot. The same index feeds the HUD's percentage
(`amount / slot * 100`, `0x811bb594`) and `Ship_RestoreShield`.

`g_skill_level` is `g_GameState+0xe0`. `GameModeBase_Apply` (`0x812b0d52`) writes it from
`event+0x1a4` at `0x812b0df4`, whatever the event's kind, and that field is `1` in every
`SP.xml` event (the constructor's default, read live on [ai-controls.md](ai-controls.md)). So a
campaign event starts every craft on `mediumshield`, which no shipped file authors: plain
`shield`. The HD-lineage modes write the global from their own `SkillLevel` grid
(`FUN_81077462`), where `0` reaches `easyshield`.

**Confidence 84**: writer and reader in one binary with consistent offsets, the same index in
`AI_ResolveSkillScale`, and Pulse's, HD's and Omega's readers agreeing. Not run live.

## Omega

**Ported.** Omega's `eboot.bin` carries all three with its own addresses
([ps4-omega-eu/craft-laws.md](../ps4-omega-eu/craft-laws.md)): the folded mass-`1.0` tensor in
both ship constructors, the same unrotated rate in `Body_ResolveContact` (`0x0131fe60`) against
the rotated one in the inlined pair resolver, and `Ship_SetShield` (`0x012f3360`) on
`stats + 0x98 + skill * 4`. One difference: Omega's shield read falls back to the medium slot
when the skill global is `-2`, a value 2048's reader has no branch for.

## Wired

`oag_2048::race::CRAFT_LAWS`: `inertia_mass` `measured(1.0)`, `wall_point_rate_body_local`
`measured(true)`, `shield_by_skill` `measured(true)`; `wall_one_sided` and `wall_steer_away`
become `measured` at the values they had. `oag_raceplay::load_event` sets
`Options::skill_level` to the title's `EventAi::level`, so an event's field starts on the
medium slot whatever the race box's difficulty says. Pinned by
`launch_hover::tests::omega_and_2048_build_hds_tensor_not_pulses`,
`launch_hover::tests::the_wall_point_rate_frame_follows_the_title` and
`shield_difficulty_ground_truth::{a_2048_novice_field_starts_on_each_teams_easyshield,
a_2048_event_starts_on_its_own_level_not_the_difficulty}`.

## Open

- Nothing here was run on Vita3K. A read of `body+0xa0`/`+0xb0`/`+0xc0` on a running race
  (`scripts/vita3k-mem.py`) would take the mass to 90.
- The integrator `FUN_811d2742` rewrites `+0xe0` and `+0x160` every step; its arithmetic was
  not decoded.
