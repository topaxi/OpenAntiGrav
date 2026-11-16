# Omega's craft laws: the box mass, the wall point rate and the shield slot (2026-11-11, craft-laws)

Three laws `oag_omega::race::CRAFT_LAWS` carried as inherited or held, read in the base
`eboot.bin` (Ghidra `/omega/eboot-ps4-omega-eu.bin`, x86-64, image base `0x01000000`).
**Static reading only**: Omega has no memory readout, so every score is capped at 84
([rubric](../../../reverse-engineering/confidence-rubric.md)). The 1.07 patch `eboot.bin` was
not searched. HD's side, with its live captures, is on
[craft-inertia.md](../ps3-hdfury-eu/craft-inertia.md) and
[wall-response.md](../ps3-hdfury-eu/wall-response.md); 2048's read of the same three is on
[vita-2048-eu-v104/craft-laws.md](../vita-2048-eu-v104/craft-laws.md).

**Verdict.** All three are HD's: the tensor is the mass-`1.0` box (not Pulse's `0.9`), the wall
resolver crosses the world lever arm with the stored body-local rate unrotated, and
`Ship_SetShield` clamps to the `<Misc>` slot the skill indexes.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x0131fe60` | `Body_ResolveContact` | 84 | [below](#the-wall-point-rate-is-body-local) |
| `0x012f3360` | `Ship_SetShield` | 84 | [below](#the-shield-slot) |
| `0x012fa370` | `Ship_RestoreShield` | 72 | calls `Ship_SetShield` with the slot's own value for controller kinds `0` and `2` (`ship+0x68fc`), then posts `GAMEPLAY_HEALTH` `Normal` for kind `0` |
| `0x0193fc54` | `g_ShipWallCofR` (data) | 84 | written by `HandlingStats_ParseXml` on `shipWallCofR` (`0x012dafa6`), read where a contact's restitution is filled |
| `0x01f999e0` | `g_skill_level` (data) | 82 | `g_GameState+0xf0` (`g_GameState` at `0x01f998f0`): indexed by `Ship_SetShield` and by `AI_ResolveSkillScale` (`0x012c9650`) beside the class at `0x01f999d8` |

Not renamed here: the two ship constructors `FUN_01309e90` and `FUN_0130e540` (other pages cite
them by address). `0x0131ca50` keeps its row from
[mp-time-trial-collision.md](mp-time-trial-collision.md) (`Collision_ProcessPairs`, 74); this
read shows the same function also holds the contact loop and the inlined pair resolver
(`0x0131f5a4` on), so it is HD's `Physics_StepWorld` with the pair pass inlined, and that
page's owner should take the rename.

## The box mass is 1.0

Omega's compiler inlined `Body_SetMass` and `Body_SetBoxInertia` into the ship constructors and
folded the arithmetic. `FUN_01309e90`, on the body it has just allocated:

    0x0130af82  mov   dword [rax+0x90],  0x3f800000   ; mass 1.0
    0x0130af93  mov   dword [rax+0x94],  0x3f800000   ; 1 / mass
    0x0130af9d  mov   dword [rax+0x4a4], 0x3f800000
    0x0130afa7  vmovss xmm2, [0x01801c30]             ; 0.0576923
    0x0130afc6  vmovups [rax+0x130], xmm2             ; inverse inertia, row x
    0x0130afce  vmovups xmm2, [0x017e9840]            ; (0, 0.0416667, 0, 0)
    0x0130afd6  vmovups [rax+0x140], xmm2
    0x0130afde  vmovups xmm2, [0x017e9850]            ; (0, 0, 0.0576923, 0)
    0x0130afe6  vmovups [rax+0x150], xmm2
    0x0130affe  vmovss xmm1, [0x01801c34]             ; 17.3333
    0x0130b006  vmovups [rax+0xb0], xmm1              ; inertia, row x
    0x0130b00e  vmovups xmm1, [0x017e9860]            ; (0, 24, 0, 0)
    0x0130b01e  vmovups xmm1, [0x017e9870]            ; (0, 0, 17.3333, 0)

and the same stores at `0x0130f5e2`..`0x0130f676` in `FUN_0130e540`. `(17.3333, 24, 17.3333)`
is `m (b^2 + c^2) / 12` for the `(12, 8, 12)` box at `m = 1.0` exactly (`208 / 12`,
`288 / 12`), and the three inverses are its reciprocals; mass `0.9` would store `21.6` on the
yaw row. These are the constants HD reads live at `body+0x10`/`+0x24`/`+0x38`. The `0.1`
angular impulse scale follows at `0x0130b212` (`body+0x4b4 = 0x3dcccccd`).

A third body builder, `FUN_012cc190`, stores mass `0.1` and is not a ship.

**Confidence 84**: an exact arithmetic identity on six stored constants, in two constructors,
matching 2048's unfolded call and HD's live tensor. Not run live.

## The wall point rate is body-local

Each narrowphase test appends a `0x40`-byte record at `world + 0x880 + 0x40 * n`
(`world+0x8880` the count): point `+0x00`, unit normal `+0x10`, the two colliders
`+0x20`/`+0x28`, the other side's material `+0x30`, depth `+0x34`, friction `+0x38`,
restitution `+0x3c`. The contact loop at the end of `0x0131ca50` (`0x0131f5a4`) sends a record
with one owning body to `Body_ResolveContact` (`0x0131f839`, `0x0131f846`) and resolves a
record with two owners inline.

`Body_ResolveContact` (`0x0131fe60`), `rdi` the contact, `rsi` the body:

```text
r      = contact.point - body+0x40                          # 0x0131fe84
v_p    = body+0xa0 + cross(r, body+0x1b0)                   # 0x0131fe64..0x0131feb7, no rotation of +0x1b0
vn     = dot(v_p, contact.normal)
if vn > 0 return                                            # 0x0131fee5 vucomiss, ja
invMass = body+0x4a8 ? body+0x4a4 : body+0x94
D      = invMass + dot(cross(M130 * cross(r, n), r), n)
if D == 0 return
j      = (-1.0 - contact+0x3c) * vn / D                     # -1.0 at 0x01801e18
p      = j * n;  if contact+0x38 > 0 and v_t != 0: p -= contact+0x38 * v_t
v(+0xa0) += p * invMass                                     # Body_ApplyImpulseAtPoint, inlined
L(+0x210) -= M50 * cross(r, p) * body+0x4b4
...
s      = -clamp(dot(body+0x10, p), -20.0, 20.0)             # 0x01320239 vminss, 0x0132024e vmaxss; 0x01801e1c
torque(+0x1e0) += body+0x20 * 5.0 * s                       # 5.0 at 0x017e9ff0
```

HD's `0x000f7770` term for term, with 2048's body layout moved up by `0x08` to `0x10`.

**The frame of `body+0x1b0`, from a second call site.** The inlined pair resolver reads the
same field for each of its two bodies and rotates it through the basis rows before the cross
product: `omega.x * row(+0x10) + omega.y * row(+0x20) + omega.z * row(+0x30)`, then
`cross(.., r) + body+0xa0`. The craft-craft resolver turns the stored rate into world axes and
the wall resolver does not, so the stored rate is body-local and the wall point velocity is
`v + (R^T omega) x r`, as HD measured live over sixteen wall hits.

**Confidence 82** for the body-local point rate (the unrotated cross read directly, the pair
resolver as the second site, 2048 showing the same contrast, HD's live measurement of the same
code; the integrator was not decoded and nothing ran). **84** for the one-sided gate and the
steer-away constants `20`, `-20`, `5`.

## The shield slot

`HandlingStats_ParseXml` (`0x012d8e30`) stores `<Misc shield>` to `stats[0x26]`, `[0x27]` and
`[0x28]` together (floats, so `+0x98`, `+0x9c`, `+0xa0`), then `easyshield` to `+0x98`,
`mediumshield` to `+0x9c`, `hardshield` to `+0xa0`: 2048's layout.

`Ship_SetShield` (`0x012f3360`), `xmm0` the amount, `rdi` the ship:

    0x012f3380  mov    rax, [rbx+0x6518]
    0x012f3387  movsxd rcx, dword [0x01f999e0]     ; g_skill_level
    0x012f338e  cmp    rcx, -2
    0x012f3392  mov    rax, [rax+0xc8]             ; the craft's stats block
    0x012f3399  lea    rcx, [rax+rcx*4+0x98]       ; the slot of the skill
    0x012f33a1  lea    rax, [rax+0x9c]             ; the medium slot
    0x012f33a8  cmovnz rax, rcx
    0x012f33b3  vminss xmm0, xmm0, [rax]           ; amount = min(amount, slot)

One global for every craft, no controller branch before the clamp. `Ship_RestoreShield`
(`0x012fa370`) and the low-health check in `FUN_012f3150` repeat the same slot expression,
`-2` branch included.

**A skill of `-2` reads the medium slot.** Neither HD's nor 2048's reader has that branch.
What writes `-2` was not traced (the global has six writers); it does not change the `0`, `1`,
`2` mapping.

**Confidence 84**: parser and three readers agree on the offsets, and the index is the one
`AI_ResolveSkillScale` uses. Not run live.

## 2048

**Ported.** 2048 v1.04 carries all three
([vita-2048-eu-v104/craft-laws.md](../vita-2048-eu-v104/craft-laws.md)):
`Body_SetBoxInertia(body, 1.0, 12, 8, 12)` as a call with immediates (`0x811b975e`,
`0x811c8a7c`), the same resolver pair (`0x811d3eaa` unrotated, `0x811d38f6` rotated) and
`Ship_SetShield` (`0x811bb524`) on `stats + 0x98 + level * 4` without the `-2` branch.

## Wired

`oag_omega::race::CRAFT_LAWS`: `inertia_mass` `measured(1.0)`, `shield_by_skill`
`measured(true)`; `wall_point_rate_body_local`, `wall_one_sided` and `wall_steer_away` become
`measured` at the values they had. Pinned by
`launch_hover::tests::omega_and_2048_build_hds_tensor_not_pulses` and
`shield_difficulty_ground_truth::omegas_novice_field_starts_on_each_teams_easyshield`.

## Open

- What stores `-2` to `g_skill_level`, and so which races read the medium slot whatever the
  difficulty.
- The 1.07 patch executable was not checked for any of the three.
