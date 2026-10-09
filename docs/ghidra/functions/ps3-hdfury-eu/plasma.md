# The Plasma, per tick: `PlasmaManager_Update` and what it calls

2026-09-16. Functions in `EBOOT.elf` (WipEout HD / Fury, PS3, `BCES-00664`,
EU). Picks up where [`weapons.md`](weapons.md) left `PlasmaManager_Construct`
(`0x001457b8`) and the `Plasma.cpp` constructor pair: the manager's vtable
(`PTR_DAT_008aadd4` -> `0x00864698`, slot 3 -> OPD `0x00877360` ->
`0x00146e68`) is the per-tick walker, and the walker holds the state
machine. This page was read **second**, against
[`ps4-omega-eu/plasma.md`](../ps4-omega-eu/plasma.md), whose x86 decompile
is the cleaner of the two; every number below is the PPC's own, resolved
through this function family's TOC (`0x008ad4d8`, `exact` per
`scripts/ps3-toc.py`, so the `DAT_` addresses Ghidra shows are the right
ones here - see [memory.md](memory.md) for why that has to be said).

**The names here are applied**, from [names.tsv](names.tsv).

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x00146e68` | `PlasmaManager_Update` | 88 | two passes over the 16-slot pool at `+0x84`, live count `+0xc4` |
| `0x0011fc88` | `Plasma_Update` | 85 | `(dt, plasma)`: the floor follower, a separate function here |
| `0x00120fe8` | `Plasma_Init` | 88 | arms the held bolt, 1.0 s charge, `WO_PLASMA_HEAD`/`_CHARGING` |
| `0x0011f810` | `Plasma_Launch` | 82 | `(plasma, node_matrix, craft_velocity)`; called at charge expiry |
| `0x00146838` | `Plasma_CheckShipHit` | 85 | `(manager, slot)`: the +-6.0 corridor test, damage, blast |
| `0x00121878` | `Plasma_PostUpdate` | 80 | the 10.0 s reap, the charge-glow ease, the visual |
| `0x0011f6c0` | `Plasma_Detonate` | 85 | `(plasma, &position)`: releases the psys, starts the explosion per viewport |
| `0x00127cd0` | `WeaponExplosions_Start` | 88 | `WO_PLASMA_LIGHTNING_EXPAND`, track fit, shows the models |
| `0x00127948` | `WeaponExplosions_Update` | 88 | vtable slot 3 of `0x00863ef8`: `age += dt`, done at 3.5 s |
| `0x00127770` | `WeaponExplosions_Collapse` | 85 | once at 1.3 s: `WO_PLASMA_LIGHTNING_COLLAPSE`, hides the models |
| `0x00126dc0` | `WeaponExplosions_Reset` | 88 | zeroes age, seeds the three ramps |
| `0x001270b8` | `WeaponExplosions_Draw` | 85 | advances the ramps, scales the three models, `UV_offset` |
| `0x0002ca00` | `WeaponStats_ParsePlasma` | 90 | the `Plasma` `<Stats>` reader - the same name `psp-pulse-usa`'s `0x0880cc2c` carries |

Unnamed: `0x00126d80` (a static initialiser that only writes the second and
third model lifetimes, `1.3`, into `0x008c18ac`/`0x008c18b0` - confidence
below 70 for a name), `0x00146470` (the blast-radius impulse loop, called
from the ship hit, not decompiled), `0x00145ef8` (the mine/bomb clearance
pass, not decompiled), `0x0011eba0`/`0x0011ebb0` (the pass-two "may I
recycle" query and the teardown, not decompiled).

## `PlasmaManager_Update` (`0x00146e68`)

Confidence **88**. `dt` arrives in `f1`. Slot layout differs from the PS4
but the shape is the same: flags `+0x40` (`8` live, `1` armed, `2` net-held,
`4` destroyed), charging `+0x50`, charge `+0x54`, age `+0x58`, destroy
pending `+0x12c`.

```c
for each live slot p:
    if (p->charging == 0) {
        Plasma_Update(dt, p);                              // 0x0011fc88
        if (p->flags & 1) {
            Plasma_CheckShipHit(mgr, i);                   // 0x00146838
            FUN_00145ef8(mgr, i);                          // mines/bombs in the path
        }
    } else {
        p->charge -= dt;
        if (p->charge <= 0.0f && !(p->flags & 2)) {        // 0.0 at TOC-0x26b8 = 0x008aae20
            node = p->craft->+0x5f34; orthonormalise node->matrix (+0x38);
            Plasma_Launch(p, matrix, ship_entry + 0xe0);   // 0x0011f810
            if (g_GameState.mode >= 0x10) net message 0x40 (PlasmaNetFire), 0x50 bytes
        }
    }
    Plasma_PostUpdate(p);                                  // 0x00121878
    if (p->destroy) Plasma_Detonate(p, &p->position);      // 0x0011f6c0, + 0xa0
pass two: for each slot with flags & 4, if FUN_0011eba0(p): swap-remove, FUN_0011ebb0(p), flags = 0
```

`Plasma_PostUpdate` (`0x00121878`) holds the reap:

```c
if (10.0f <= p->age || (p->flags & 4)) p->destroy = 1;    // DAT_008aa010 = 0x41200000
else {
    if (p->charging) p->glow += (p->glow_target - p->glow) * p->glow_rate;   // + 0x190 <- (+0x194, +0x198)
    ...visual placement (0x00121418) unless the craft is hidden
}
```

**Hardcoded 10.0 s, no authored `timetodie`** - and the Plasma `<Stats>`
block on this binary has no such attribute to read (below). Same as Pulse,
same as Omega.

## `Plasma_Init` (`0x00120fe8`): the wind-up is 1.0 s, hardcoded

Confidence **88**. `*(param_1 + 0x54) = DAT_008a9f44._4_4_` - that is the
word at `0x008a9f48` = `0x3f800000` = **1.0** - with `+0x50 = 1`
(charging), `+0x58 = 0` (age), `+0x40 |= 8`, `+0x110 = -(craft + 0x7850)`
(the carried normal starts as minus the craft's up), and `+0x124 =
*(0x008c1868 + 4)` = `0xbf99999a` = **-1.2**, the same seed Omega's
`Plasma_Init` writes as a literal. Two particle systems are attached at
`p + 0xb0`: `'PLHE'` (`WO_PLASMA_HEAD`) and `'PLCG'` (`WO_PLASMA_CHARGING`),
both owned by the race-wide `RaceManager` instance as
[weapons.md](weapons.md) already noted for the explosions. Rumble
`FUN_000a3528(0.07, 1.0, ...)` (`0x008aa000 = 0x3d8f5c29`, `0x008a9f48`)
if the craft is local, and two light descriptors are registered from
`p + 0x1a0` (`FUN_00677c78`, descriptors at `*0x008a9f40 + 4` and `+ 0x74`).

**`charge_time` is authored and unread, third binary.**
`WeaponStats_ParsePlasma` (`0x0002ca00`) stores it at stats `+0xbc`. A
program-wide `search_instructions` for `lfs` with operand `0xbc(r` returns
68 hits and **none in any function attributed to `Plasma.cpp` or
`PlasmaManager.cpp`** (`0x0011eba0`..`0x00121878`, `0x00145ef8`..
`0x00147020`); the same search scoped to `Plasma_CheckShipHit` for
`0xc0(r` finds the `damage` read at `0x00146d6c`, which is the
calibration. Pulse's authored `3` and this binary's own value are both
three times a wind-up that is compiled in.

## `Plasma_Update` (`0x0011fc88`): the floor follower

Confidence **85**. The PPC decompile is a wall of Altivec permutes, so the
constants were read out of the TOC rather than the pseudocode:

| Role | Where | Value |
| --- | --- | ---: |
| probe length along the carried normal | `0x008a9fc0` | **6.0** (Omega 6.0, Pulse 12.0) |
| fall, world Y, per second | `0x008a9fe4` | **-50.0** |
| ride height above the hit point | `0x008a9fd8` | **4.0** (Omega 4.0) |
| km/h -> m/s | `0x008a9fdc` | **`0x3e8e2eb2` = 0.2777**, the same odd literal as Omega's walker |
| launch-ramp window | `0x008a9f48` | 1.0 s |
| `+0x124` ramp: `min(0, x + dt * rate)` | `0x008a9fe8`, `0x008a9f20` | rate 3.0, ceiling 0.0 |

Control flow, read off the branch structure:

```c
p->age += dt;  p->prev = p->position;  next = p->position + p->velocity * dt;
kind = Collide(track, next, next - p->surface * 6.0f, &out);      // 0x0007be58
if (kind < 6 && kind in {0, 4, 5})  p->destroy = 1;                // a wall
else if (kind == 0x7f)               fall: velocity.y -= dt * 50.0f     // open air (Pulse's code, Pulse's role)
else                                 ride: surface = out.normal; next = hit + normal * 4.0f; renormalise
speed = class speed (g_GameState + 0xd4 selects stats +0xcc..+0xd8 - FOUR classes, no SuperPhantom)
if (p->age < 1.0f) speed = (1.0f - p->age) * p->launch_kmh + p->age * speed;   // + 0x4c
p->velocity = dir * (speed * 0.2777f);
kind = Collide(track, p->prev, next, &out);                        // the travel segment
if (kind < 6 && ((1 << kind) & 0x31))  p->destroy = 1;             // kinds 0, 4, 5 again
if (p->destroy) Event_Push(0x33, next);                            // FUN_00677048
p->ramp = min(0.0f, p->ramp + dt * 3.0f);                          // + 0x124
```

The wall/open-air enumeration is **Pulse's** (`0`/`4` wall, `0x7f` none -
[psp-pulse-usa/plasma.md](../psp-pulse-usa/plasma.md)) with `5` added;
Omega renumbered it (`4, 5, 6, 8` wall, `0xe` none). The gravity axis is
world-vertical on both HD ports and along the carried normal on the PSP.

`Plasma_Launch` (`0x0011f810`, confidence 82 - read from its constant loads
and its call site rather than a full decompile): `lfs` from the TOC pulls
`3.6` (`0x008a9fb8`), `1.0`, `0.0`, the launch light's `24.0` radius and
`(2.2, 1.6)` colour (`0x008a9fbc`, `0x008a9fac`, `0x008a9fb0`) and the
`0.06`/`0.3` rumble pair (`0x008a9fa4`, `0x008a9f80`) - the same five
things Omega's `Plasma_Launch` does with the same numbers.

## `Plasma_CheckShipHit` (`0x00146838`): the corridor and the damage

Confidence **85**. For every craft except the firer (`p + 0x44`), the craft
position is projected onto the segment `prev -> position`; a hit needs the
projection inside the segment and the perpendicular distance strictly
between `0x008aae24 = -6.0` and `0x008aae28 = 6.0`. Then:

```c
p->destroy = 1;
stop the travel emitter; Sound_Play(1.0f, emitter, ..., "PLASMAHITSHIP");   // 0x007852e8 - Pulse's cue name, not Omega's NGP_
if (victim is remote) Event_Push(0x33, hit);
online-stat bump for modes 0x10/0x11/0x14 (a counter at +0x8c of 0x0004d7e0's object)
if (g_GameState.mode < 0x10) {                                     // local play
    if (firer == local player) victim->hit_by_local = 1;           // + 0x124
    victim->last_attacker = firer;  victim->last_weapon = 2;       // + 0x13c, + 0x138
    victim->hits         += 1.0f;                                  // + 0x12c, 0x008aad8c
    victim->damage_taken += stats->damage;                         // + 0x120 += stats + 0xc0
    victim->slowdown     += stats->slowdown_time;                  // + 0x130 += stats + 0xe4
    FUN_00146470(mgr, hit, firer);                                 // the blast-radius impulse, unread
}
else net message 0x18 to the victim's owner
```

**No `weapon_damage_multiplier` here.** Omega multiplies `stats->damage` by
the firer's handling `+0x4c4`; this binary adds `stats->damage` raw. Whether
HD's handling XML has the attribute at all was not checked - the PS4 code
is the only place it was seen read.

`WeaponStats_ParsePlasma`'s block, in `strcasecmp` order, each name
resolved through the function's TOC:

| Attribute | stats offset |
| --- | ---: |
| `charge_time` | `+0xbc` (unread) |
| `damage` | `+0xc0` |
| `blastradius` | `+0xc4` |
| `blastforce` | `+0xc8` |
| `slowdown_time` | `+0xe4` |
| `venomspeed` / `flashspeed` / `rapierspeed` / `phantomspeed` | `+0xcc` / `+0xd0` / `+0xd4` / `+0xd8` |
| `launchspeed` | `+0xdc` |
| `absorb` | `+0xe0` |

Eleven attributes against Omega's fourteen: no `superphantomspeed`, no
`screen_shake_amount`/`_time`. The four-way class switch in `Plasma_Update`
matches.

## `Plasma_Detonate` (`0x0011f6c0`) and the explosion

Confidence **85**. Releases the head and charging psys (`+0x5c`, `+0x60`),
sets flags `(& ~8) | 4`, calls the teardown `FUN_0011ebb0`, then for each
viewport (`g_GameState + 0xe4` of them) the first that passes
`FUN_002d64d0(-1, pos, pos, v)` gets `WeaponExplosions_Start(p->explosion,
pos)` (`+0x140`) and the loop stops. A bolt no viewport can see never
starts its explosion, and pass two recycles the slot as soon as
`FUN_0011eba0` says so.

`WeaponExplosions_Start` (`0x00127cd0`): `+0x191 = 1` (active), spawns
`'PLED'` (`WO_PLASMA_LIGHTNING_EXPAND`) at the position, fits a basis to the
track under it (`FUN_000a97f0`) into `+0x90..+0xc0` and per viewport into
`+0xd0 + v * 0x40`, shows the three models (`+0x54`, `+0x58`, `+0x5c`:
`HD_plasma_ring`, `HD_plasma_sphere`, `HD_plasma_halo` in that order - the
`.vex` names resolved from `WeaponExplosions_Construct`'s TOC slots
`-0x31ec`, `-0x31e8`, `-0x31e4`), calls `WeaponExplosions_Collapse` once
(a no-op at age 0), registers **two** light descriptors (`+0x30` and
`+0xa0` of the block `WeaponExplosions_Construct` fills at `*0x008aa260`),
and sets `+0x34 |= 6`.

`HD_plasma_ball` is not among them: on this binary as on the PS4 it is the
bolt's own head, not part of the explosion.

What animates the three models:

| | ring | sphere | halo |
| --- | ---: | ---: | ---: |
| ramp `(cur, target, rate)` at | `+0x60` | `+0x6c` | `+0x78` |
| seeded from | `*0x00994070 + 0`, `0x008c18b4`, `0x008aa264` | `+4`, `0x008c18b8`, `0x008aa268` | `+8`, `0x008c18bc`, `0x008aa26c` |
| target | **100.0** | **7.1** | **7.0** |
| rate | 0.01 | 0.3 | 0.2 |
| advanced while `age <=` | 1.7 s (`0x008c18a8`) | 1.3 s (`0x008c18ac`, written by `0x00126d80`) | 1.3 s (`0x008c18b0`, same) |
| basis | per-viewport | shared (`+0x90`) | per-viewport |

`cur += (target - cur) * rate` per `WeaponExplosions_Draw` call, the model's
basis scaled by `cur`, then `FUN_002c1b30(age, model)` - the `UV_offset`
constant `WeaponExplosions_Construct` looks up by CRC on each model.
`WeaponExplosions_Update` adds `dt` to `+0x50` and retires the object when
`age >= 0x008aa2b4 = 0x40600000 = 3.5 s`; `WeaponExplosions_Collapse` fires
once at `0x008aa25c = 1.3 s`, spawning `'PLCE'` and hiding all three.

**One hedge on the second and third windows.** On disk `0x008c18ac` and
`0x008c18b0` hold `0.0`; the `1.3` is written at runtime by `0x00126d80`,
and that function is gated (`if (param_1 != 1) return; if (param_2 !=
0xffff) return;`) on two arguments whose meaning was not read. If the gate
never fires, the sphere and halo never scale at all. The reading stands on
corroboration rather than on the gate: Omega's `0x01372600` is a plain
`void (void)` static initialiser that writes the identical `1.7, 1.3, 1.3`
triple unconditionally, and two independently-compiled binaries agreeing on
the triple is what makes `1.3` the right number here.

The rates and lifetimes are Omega's exactly; the targets are not (Omega
50 / 2.0 / 3.0). Given that both titles ship the same three `.vex` names,
the likeliest reading is that the PS4's `rcsmodel` re-exports carry a
different base scale - unverified, and the reason this row is in the
comparison table rather than a claim about the models.

## What is not verified

- `Plasma_Launch`'s body beyond its constants and call site.
- The blast-radius loop `FUN_00146470` (Omega's, read in full, applies a
  `blastradius`/`blastforce` falloff impulse to every craft; this one was
  not opened).
- What `FUN_0011eba0` checks before a slot is recycled (Omega: the
  explosion's `active` byte).
- Whether the HD handling XML carries `weapon_damage_multiplier` at all.
- What `0x00126d80`'s `(1, 0xffff)` gate means, i.e. when the two `1.3 s`
  windows actually get written (see the hedge above).

## 2026-09-17: implementation pass - the Collapse/Draw split, and both `.pob` internal names confirmed

Landing the bolt and the explosion in `oag-game` (`crates/raceplay/src/blast_models.rs`,
`crates/title/src/weapons.rs`) forced two checks past what this page's
`WeaponExplosions_Start`/`_Collapse`/`_Draw` section already established.

**`WeaponExplosions_Collapse` (`0x00127770`) and `WeaponExplosions_Draw`
(`0x001270b8`) were re-decompiled to settle an apparent contradiction**: this
page's own table says the ring is "advanced while `age <= 1.7 s`" but
Collapse "hides all three" at `1.3 s`. Both are true and answer different
questions. Collapse's own body clears bit `0x4` at offset `+0x34` on all
three model nodes (`&= 0xfffffffb`) - a node-visibility flag, distinct from
anything `Draw` reads: `Draw`'s three blocks are gated purely on `age`
against each model's own window (`PTR_DAT_008aa258[0/1/2]` = 1.7/1.3/1.3 s)
and never touch `+0x34` at all, and the transform-write helper `Draw` calls
(`_opd_FUN_00327500`) touches a different bit range (`0x0f000000`, a "dirty"
nibble) on the same offset. So the ring's own ease keeps computing between
1.3 s and 1.7 s, and nothing shows it: Collapse's node-hide fires first and
`Draw`'s per-model window is not what draws or hides anything on screen.
**Implemented as**: all three models draw with the recovered ease from `0 s`
and go invisible (`hd_ease` forced to `[0.0; 3]` for the draw alone, object
state unaffected) from `1.3 s` to the `3.5 s` object reap - i.e. HD's own
node-hide, not the per-model window. Confidence 85 stands, now corroborated
rather than merely read.

**`WO_PLASMA_LIGHTNING_EXPAND`/`_COLLAPSE`, confirmed against the disc's own
`.pob` internal name field, not just the fourcc.** `scripts/psarc.py cat` on
`DATA02`'s `wo_plasma_lightning_expand.pob`/`_collapse.pob` and a raw `xxd`
of the header shows the ASCII name field reading `WO_PLASMA_LIGHTNING_EXPAND`
and `WO_PLASMA_LIGHTNING_COLLAPSE` byte for byte - both load and resolve
their emitters live (`oag-game`'s own loader report: 1 emitter each).

**`0x00121418` (`Plasma_PostUpdate`'s visual-placement call, "not
decompiled" in the table above) was opened but not resolved to a name.**
Confidence stays below 70 by this project's own rubric, so it is not
renamed. It branches on a byte at `param_1+0x50`: when clear, a bare
placement (`_opd_FUN_00327500(node, param_1+0x150, 0)`); when set, a full
Gram-Schmidt basis build off two vectors read from `param_1+0x160`/`+0x170`
(normalize, cross, cross again - the same shape `Race::projectile_model_matrices`
already builds off velocity and a reference), consistent with a
velocity-plus-carried-normal basis but not enough to say which field is
which without more work. **The engine implementation uses velocity alone**
(`Race::plasma_ball_model_matrices`, reusing the Rocket's own helper) -
chosen, not measured, and said so in that function's own doc comment.

**Observed, not yet explained: the picture is oversized.** `HD_plasma_ring`'s
own target scale (100.0) produces a sphere that fills most of the frame at
normal chase-camera distance - screenshot at
`/home/topaxi/.cache/oag/drive/reports/hd-weapon-models-screenshots/t200-detonation.png`
(kept alongside this thread's report). The number itself is read at
confidence 88 and re-confirmed this session; the loader report shows every
one of the three models resolving a real material through the lit race pass
(not the "no material" flat-shading gap the shield shell has), so the size
is not a missing-material artifact. The likeliest explanation is still the
one already on this page - a base-scale mismatch between what this session
assumed (`cur` as a literal uniform-scale multiplier on the model's own
authored unit) and what the original composes it against - but nothing this
session did settles which. Left as an open finding rather than a silent
correction, per `CLAUDE.md`'s rule against inventing a fix with no evidence
behind it.

**Gated off the same day, before merge.** At `t200`/`t400` the chase camera
sits *inside* the 100-scale sphere for a real span of its 3.5 s life, which
reads as the whole frame going solid grey then tinted purple - worse than
the billboard fallback it replaces, and a regression by CLAUDE.md's own
"draw nothing and say so" rule. `blast_models::HD_BLAST_MODELS_DRAWN`
(`false`) now gates the ring/sphere/halo trio's *draw* off while every other
Plasma line here keeps running unaffected: the bolt's own head, both
`WO_PLASMA_LIGHTNING_EXPAND`/`_COLLAPSE` triggers, and this trio's own
age/ease tracking (`Race::advance_plasma_blast_models`) - so the next reader
only has to flip the constant once the scale composition above is
understood. The load report now says so explicitly on an HD source: "HD
plasma explosion models loaded, not drawn: the recovered scale ease
produces a screen-filling sphere - see plasma.md's 2026-09-17 'the picture
is oversized' section."

## 2026-09-23: the scale reading holds; the three models are simply that big

The open question above - "a base-scale mismatch between `cur` as a literal
uniform scale and what the original composes it against" - was put to the
models and to `WeaponExplosions_Draw` directly. **Nothing composes against
`cur`: it is a uniform scale on an orthonormal basis, onto a model whose
authored radius is several metres.** Read on the `ps3-hdfury-eu` database
the bridge serves as `/hdfury/EBOOT-ps3-hdfury-eu.elf` (language
`PowerPC:BE:64:A2ALT-32addr`, 26,100 functions - the pre-`lvlx` import, so
every vector sequence below was checked against `llvm-objdump
--triple=powerpc64` rather than taken from the decompile).

### The models' own extents

Measured by `crates/render/examples/hd_weapon_extents.rs`, which builds each
`.vex`/`.rcsmodel` pair through `mesh::rcs::build` - the exact path
`oag_game`'s weapon loader takes - and walks the `.vex` node tree with
`oag_vex::vex::world_transforms_at` at eight times from 0 to 3 s:

| model | shape | model-space radius | node transforms | material, authored blend | texture |
| --- | --- | ---: | --- | --- | --- |
| `HD_plasma_ring` | flat disc in XY, `z = 0.0156` | 8.82 | identity at every sampled time | `hd_plasmaring_glow`, `SrcAlpha, One` (additive) | `plasma_ring.gtf` 256x256, mean RGB 8.9 of 255 |
| `HD_plasma_sphere` | sphere | 4.06 | identity at every sampled time | `plasmasphere_glow`, `SrcAlpha, OneMinusSrcAlpha`, second sampler `noise.gtf` | `plasma_1024x1024.gtf` |
| `HD_plasma_halo` | flat disc in XY | 8.98 | identity at every sampled time | `hd_plasmahalo_glow`, `SrcAlpha, One` (additive) | `plasma_halo.gtf` 256x512, mean RGB 25.6 |

**The ring is not ~14x smaller than the other two**, which is what a
composition error behind the `100.0` would have needed; its radius is the
halo's to within 2%. And no node in any of the three carries a scale, static
or keyed, so there is no authored factor for `cur` to multiply against
either. Confidence **95** on the extents (a direct parse, the same builder
the race draws with).

### What `cur` feeds: a uniform scale, by the select mask

`WeaponExplosions_Draw` (`0x001270b8`), per model: splat `cur` to all four
lanes, `vmaddfp` it into the basis' three rows, then `vsel` each scaled row
against the unscaled one under the 16-byte mask `*PTR_DAT_008aa280` =
`*0x00769cd0` = `{0, 0, 0, 0xffffffff}` - so X, Y and Z of every row take the
scaled value and only W keeps the original. The translation row (`+0x30`) is
copied unscaled. The result goes to `_opd_FUN_00327500(node, &matrix, 0)`,
which is a plain 64-byte copy into `node + 0x80` plus a dirty-bit update on
`node + 0x34` and its children - no normalisation, no second factor. Then
`_opd_FUN_002c1b30(age, node)`, which walks the node tree calling a virtual
(slot `+0x40`) on the three `Anim Transform`-family classes
(`PTR_PTR_008b3984`/`88`/`8c`) - an anim-time set, which the static identity
transforms above make a no-op for the geometry. **Confidence 90** that the
world scale of each model is exactly `cur` times its authored geometry.

`cur` starts at `0`: `WeaponExplosions_Reset` (`0x00126dc0`) seeds it from
`0x00994070`, which is `.bss` and has no static writer (its only other
reference, from `0x00433090`, is a read).

### The basis: a billboard for the two discs, the track for the sphere

`WeaponExplosions_Start` (`0x00127cd0`), past the `FUN_000a97f0` call the
decompiler cuts off at (Ghidra marks it non-returning; the disassembly from
`0x00128028` on continues): per viewport it copies the track-fitted basis
into `+0xd0 + v * 0x40`, then **replaces its third row with the negated,
normalised per-viewport vector** it pulled out of that viewport's camera
matrix (`PTR_DAT_008aa2bc + v * 0x40`) at the top of the function, removes
that direction from the second row and renormalises it (Gram-Schmidt), and
rebuilds the first as their cross product (`0x001280a0`..`0x001281ec`). So
the ring and halo, which `Draw` places with the per-viewport basis, lie in
the plane facing that viewport's camera, "up" kept as close to the track's
up as that allows. The sphere uses the shared, unmodified track basis at
`+0x90`. Confidence **80** on the camera vector being the view direction
rather than the eye-to-blast direction: which column of the camera matrix
the first loop extracts was not traced element by element.

### What that composes to, on screen

`1` world unit is one metre (`oag_raceplay::spline`'s own note, from the
HUD's `speed * 3.6` km/h factor). At a 60 Hz `Draw` rate - **assumed, not
read**: `Draw` is called from `0x00127468`, which is slot 5 of the
explosion's vtable `0x00863ef8`, and what schedules slot 5 was not followed -
`cur = target * (1 - (1 - rate)^n)` gives:

| age | ring radius | sphere radius | halo radius |
| ---: | ---: | ---: | ---: |
| 0.1 s | 52 m | 25 m | 46 m |
| 0.5 s | 230 m | 29 m | 63 m |
| 1.3 s (hidden) | 479 m | 29 m | 63 m |

These are large, and they are what the disc authors: the sphere is a
29-metre bubble the chase camera can easily be inside, and both discs are
screen-facing additive layers (the ring's texture nearly black, so it adds
little) whose size mostly decides how far across the screen their texture
is spread. **The same disc authors the Missile's
explosion at a comparable size with no code involved** -
`HD_missile_explosion.vex`'s `Anim Transform` keys scale its three main mesh
nodes from 1 to 18 over its first second (radius 6.4 -> 116 m), and its
fourth, pre-scaled `0.0977`, to 44 - so a 100-metre-class detonation is the
title's own authoring convention, not an arithmetic slip in one reading.

**So the 2026-09-17 "oversized" picture is not a scale error**, and the
reading that `cur` might compose against a base scale is retired. What the
engine was drawing differently is below.

### What the engine drew differently: the back faces and the basis

`Material_ApplyRenderState` (`0x005d8f68`, see
[material-state.md](material-state.md)) writes `state >> 4 & 1` to RSX
method `0x183c`, which RPCS3's own `rpcs3/Emu/RSX/gcm_enums.h` names
`NV4097_SET_CULL_FACE_ENABLE` (and `0xa74`, the other register that page
left unidentified, `NV4097_SET_DEPTH_TEST_ENABLE`). The cull face is
`GL_BACK` outside the ship-shadow pass
([ship-sun-occlusion.md](ship-sun-occlusion.md)). All three explosion
materials are state `0x39`: blended, **culled**. Confidence **90** - a
decompiled register write, named from the emulator's own header.

And the geometry is built for culling, measured by `hd_weapon_extents` (each
triangle's `(b - a) x (c - a)` against its vertices' authored normals, all
agreeing, and against the direction from the model's origin):

| model | shells |
| --- | --- |
| `HD_plasma_sphere` | two meshes of 760 triangles, one wound outward and one inward |
| `HD_plasma_ring` | two discs of 256 triangles, one facing `+Z`, one `-Z` |
| `HD_plasma_halo` | **one** disc of 320 triangles, every normal `-Z` |

This engine drew every HD `.rcsmodel` draw unculled (`mesh::rcs` sets
`DrawCall::culled = false` throughout), so the sphere drew all four layers
along any line of sight instead of two, and from inside it both shells
instead of the inner one. `oag_raceplay::load::weapon_models::cull_as_authored`
now culls the three explosion models as authored.

**This is the authored state, not a demonstrated cure for the 2026-09-17
frame.** That solid-grey frame was not reproduced on this tree, culled or
not: a matched pair under continuous fire (a new bolt every charge, several
blasts overlapping - `rapid-fire/unculled/` against `rapid-fire/culled/`,
same ticks) differs only in detail, and both fill the frame with purple. That
is several 29-metre blasts on top of one another, which a player holding one
pickup does not see. The 09-17 frame was taken with the reflected basis below
and before this pass, and what it was is left there.

**The halo settles the discs' orientation.** A single-sided `-Z` disc under
a culling material can only ever be seen with `-Z` towards the viewer, so
the basis's `Z` must point away from the camera - which is what `Start`'s
`vsubfp` from zero reads as, if the camera vector it negates is the view
direction. That corroborates the confidence-80 reading above from the data
side. `oag_game`'s shared `billboard_matrix` points `Z` at the camera **and
is a reflection** (`right = forward x up`, `up = right x forward`, determinant
`-1`), which under culling would hide exactly the faces the original draws;
HD's discs now take `blast_models::facing_away`, a right-handed basis with
`X = Y x Z` like `Start`'s own cross at `0x001281b4`..`0x001281cc`.

**Checked by pixels, not by eye.** With the ring and sphere forced to scale 0
(a temporary edit, reverted), the halo alone changes 125,422 pixels at tick
190 and 93,471 at 215 against all three at 0 - and **exactly** the same
pixels, to the same summed difference, culled and unculled. So under this
basis every halo triangle the engine rasterises is a front face: the `-Z`
sign and the engine's counter-clockwise front face agree with the original's
data. Had either been wrong the culled halo would have vanished.

Screenshots of the result, several frames across one blast's life from the
chase camera: a single shot from the grid, detonating 45 m ahead, and a single
shot at speed that the craft flies through at about 0.17 s. Kept (not in the
tree) under `~/.cache/oag/drive/reports/hd-weapon-detonations/`
(`final-grid/`, `final-flythrough/`, `rapid-fire/`, `halo-check/`, a
`contact.png` in each). A purple, translucent dome round a white core that
grows and goes at 1.3 s; flying through it tints the upper frame for a few
frames and no more. To regenerate them, write two input scripts (the
`scripts/input_script.py` format):

```text
# grid.inputs - one shot from the grid
100 none
2 square
1000 none

# one-shot.inputs - one shot at speed
400 none
2 square
1000 none
```

and for each tick `T` of interest (`170 180 190 200 215 230 245 260` for the
first, `462 466 470 474 480 490 510 534` for the second):

```sh
./target/debug/oag-game data/images/hdfury-ps3-eu-dec.iso --race \
    --mode single_race --autopilot --give plasma \
    --input-script grid.inputs --ticks T --screenshot out/tT.png
```

Deterministic, so one run per frame. The continuous-fire pair used a script
of `1 square` / `1 none` repeated, at `330 360 ... 540`.

**Still not played, and each a visible difference from the original:**

1. `UV_offset`. `WeaponExplosions_Construct` (`0x00128ab0`) looks up the
   shader constant named `UV_offset` (string at `0x00783d00`, through the TOC
   slot `0x008aa304`) on each of the three models and binds it
   (`FUN_00677018`) to `_opd_FUN_002c11c8(model) = node + 0xc0` of the
   model's first node of class `PTR_PTR_008b3988`; `Draw` then sets anim time
   `age` on the tree (`_opd_FUN_002c1b30`, a walk calling slot `+0x40` on the
   three anim node classes). What `node + 0xc0` holds on that class was not
   read. None of the three materials carries an Edge curve of its own
   (`mesh::rcs::curve_track`'s sweep found `weapons/reticule_missile` alone
   among weapons), so the scroll, if it is one, is the node's.
2. The sphere's second sampler (`noise.gtf`), which `mesh::rcs` loads and
   does not draw ("role unread").
3. The sphere's basis: the original gives it the track fit `FUN_000a97f0`
   returns, this engine the camera-facing one. Round, so only its texture
   seam moves.

### How to falsify this

A live `Z0` breakpoint (interpreter only, see
[rpcs3-debugger.md](../../../reverse-engineering/rpcs3-debugger.md)) on
`Draw`'s three `bl 0x327500` sites - `0x0012722c`, `0x00127310`,
`0x001273f4` - reading `+0x50` (age), `+0x60`/`+0x6c`/`+0x78` (`cur`) and the
row lengths of the matrix at `r4`. The prediction is row length = `cur` =
`target * (1 - (1 - rate)^n)` with `n` the number of `Draw` calls since
`Start`; a live row length that differs from `cur`, or a `cur` that grows at
half this rate (a 30 Hz slot 5), would overturn the table above.

## 2026-09-25: `UV_offset`'s binding mechanism, and the sphere basis's found/not-found branch

Reading the two items this page's 2026-09-23 section left open for the
Plasma explosion's `UV_offset` and the sphere's track-fitted basis. Read on
the bridge (`program=EBOOT.elf`, read-only, `analysis_status` idle first),
decompile cross-checked against the raw disassembly at every point the
decompiler produced Altivec noise - the same caution this page's earlier
sections already took. **Not run this pass**: the live RPCS3 `Z0` check the
previous section proposed; both findings below stand on static reads alone.

### `UV_offset`: bound by pointer, not by value, to a generic Anim Transform node's own live output

`WeaponExplosions_Construct` (`0x00128ab0`) calls two small helpers to make
the binding, and both are now named from [names.tsv](names.tsv):

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x002c11c8` | `AnimNode_FindTransformValueField` | 80 | `(model)`: if the model's *own* node isn't already class `PTR_PTR_008b3988`, walks its children (`_opd_FUN_006b2108` matches by class) for the first one that is; returns **that node's address plus `0xc0`**, unconditionally - even the "not found" path returns `0 + 0xc0`, i.e. `0xc0`, which the caller never checks for validity |
| `0x002c1b30` | `AnimNode_UpdateTransformTree` | 82 | `(age, node)`: if `node`'s vtable is one of the three Anim Transform classes (`PTR_PTR_008b3984`/`88`/`8c`), calls **virtual slot `+0x40`** on it with no visible extra args beyond the implicit `this`/`age` pair already in registers; then recurses every child via a *different* walker, `_opd_FUN_002c1778` |
| `0x00677018` | `Material_BindInstanceParamPointer` | 85 | two-line thunk: sets up its own TOC then tail-calls the already-named `Material_SetInstanceParamPointer` - confirms the bind is **by pointer**, not a value copy |

`Construct`'s own body (decompiled and matched against `WeaponExplosions_cpp`
string context) does, per model, in order: resolve the model's `UV_offset`
constant slot via `FUN_00677008` (name-hash lookup, `~FUN_00676ff8(name)`
already established on this page for the psys tag), and if that lookup
succeeds, call `AnimNode_FindTransformValueField(model)` then
`Material_BindInstanceParamPointer(..., ~name_hash, field_ptr)` - i.e. the
shader constant named `UV_offset` is pointed **directly** at `node + 0xc0` of
the model's first Anim-Transform-class node, read fresh by the RSX every
draw, never copied into the material's own storage.

**Confirmed to be the same mechanism, not merely similar, by a second,
independent call site.** `MissileManager_Construct` (`0x00154cf0`) makes the
identical two calls **twice** against the **same** node
(`param_1[0x61]`, `HD_missile_explosion`'s own model): once with
`~FUN_00676ff8("UV_offset")`, once with `~FUN_00676ff8("Shockwave_scalar")` -
both binding to the *same* `node + 0xc0` pointer. Two shader parameters with
different names aliasing the identical four bytes only makes sense if that
field is a single generic "current value" the node's own Anim Transform
class produces each tick, read by whichever material asks for it under
whichever name its own `.cgfx` program uses - not a UV-specific field at all.
This raises confidence in the *general* reading (0-80 for what the field
literally is) beyond what either call site alone would support.

**What `node + 0xc0` holds and what law drives it was not resolved to
specific numbers this pass.** `AnimNode_UpdateTransformTree`'s virtual slot
`+0x40` resolves (checked via the class's own vtable at `0x008b3988` -> OPD
`0x00875268` -> `0x001047b8`) to a keyframe evaluator: a loop bounded at 19
iterations (`0x13`), reading a frame index that wraps at 112 (`0x70`) out of
the node's own `+0x310`/`+0x314` fields and calling a per-key blend
(`_opd_FUN_0010c958`) - the same shape this page's own 2026-09-23 section
already described for `HD_missile_explosion`'s 1x-to-18x keyed scale ramp,
generalised to whatever property a given node instance carries. **None of
that function's own field offsets (`+0x30`, `+0x40`, `+0x50`, `+0x70`,
`+0x80`..`+0x88`) is `0xc0`**, so it evaluates into a shared/global scratch
object (`PTR_DAT_008a9758`) rather than into the node directly by this read;
how the evaluated value reaches the node's own `+0xc0` field - and therefore
what its authored keys and units are for each of the ring/sphere/halo models
- needs the **data** side: the Anim Transform track's own keyframes in each
`.vex`, the same way `HD_missile_explosion`'s scale keys were read directly
off the file rather than off the code. `hd_weapon_extents`'s own measurement
(`identity at every sampled time`) was checking the models' *geometry*
transform nodes, not this Anim-Transform-class node, so it says nothing
about whether this field moves. **Confidence on "what the value is used for
by the shader" stays at the 80 above; confidence on "what its law is" is
below 50 and nothing is guessed here** - CLAUDE.md's line against
hand-transcribing a table applies exactly here: the next step is parsing the
node's own keys, not authoring a plausible scroll rate.

`Shockwave_scalar` (the thread's other open name) is now placed: it is
Missile's own second binding to the same mechanism, not a Plasma parameter
at all - none of the three Plasma explosion models' own name-hash lookups in
`WeaponExplosions_Construct` resolve a second name; only `UV_offset` does,
per model, once.

### The sphere's basis: a found/not-found branch on the same track query, not a separate function

`WeaponExplosions_Start` (`0x00127cd0`) seeds the shared basis at
`param_1 + 0x90 .. + 0xc0` (four 16-byte rows: `+0x90/+0xa0/+0xb0` from a
constant at `0x008aa29c`, which **is** a plain 4x4 identity matrix - read
directly at the address the TOC slot points to, `0x00769d70`:
`[1,0,0,0][0,1,0,0][0,0,1,0][0,0,0,1]`, confidence 95 - and `+0xc0`
overwritten with the detonation position from `param_1 + 0x40`) **before**
calling `FUN_000a97f0`, then branches on that
call's own boolean return (`r3`, masked to a byte, `bne` on nonzero at
`0x00128030`-`0x00128034`):

- **Not found (`r3 == 0`, the fallthrough at `0x00128038`)**: the function
  goes straight into the per-viewport loop this page's 2026-09-23 section
  already described (`+0xd0 + v * 0x40`, camera vector negated and
  Gram-Schmidt'd against the seeded rows) - `+0x90/+0xa0/+0xb0` are never
  touched again and keep the seeded constant.
- **Found (`r3 != 0`, the branch to `0x001282b8`)**: before joining the
  *same* per-viewport loop (it falls into it via an unconditional `b
  0x00128038` at the end, `0x001283fc`), the code does **one extra
  Gram-Schmidt-shaped construction** - normalise, cross, cross again, the
  identical instruction pattern the per-viewport loop itself uses - seeded
  from a 16-byte vector at the caller's own stack (`r1 + 0x140`, negated
  from zero first) rather than from a camera matrix, and writes the result
  into `param_1 + 0xa0` and `+0xb0` only (`+0x90` is read, not rewritten).

That difference is exactly what "the sphere takes the track fit, the ring
and halo take the camera-facing override" (this page's 2026-09-23 section)
predicts: the *shared* rows are camera-independent only when the track query
found a point, and get an extra orthogonalisation pass keyed off whatever
`r1 + 0x140` is when it did.

**What feeds `r1 + 0x140` was not pinned down, and the confidence on this
whole branch reading is 60, below this project's rename threshold - nothing
above gets a new name for `FUN_000a97f0` itself.** The call site passes
`r3 = *(RaceManager + 0xbc)` (a pointer, read from the singleton, not a
per-call position) and `r4 = r1 + 0x120`, with `r8 = 0` (`in_r8` in the
decompile, meaning `FUN_000a97f0`'s own "seed/write-back through r8" path is
dead code at this call site - it always starts from zero and never copies
its find back out through that argument). Both `_opd_FUN_000a8b20` and
`_opd_FUN_000a8198`, which it calls in sequence, disassemble as a
node-indexed tree walk (child pointers at `+8`/`+0xc` of 0x20/0x18-byte
records, squared-distance comparisons via `vmaddfp`/`vrsqrtefp` against a
vector in a hidden vector-register argument) consistent with a nearest-point
search over some spatial structure hung off `RaceManager + 0xbc` - plausibly
the track collision mesh the way `Plasma_Update`'s own `Collide` calls read
it, but that identification is a hypothesis, not a read: nothing here
confirms `RaceManager + 0xbc` is the track rather than, say, a per-race
weapon-explosion pool with its own spatial index. **Where `r1 + 0x140` gets
its sixteen bytes from was not traced further** - it sits outside the 16-byte
buffer at `r1 + 0x120` that is the only address actually passed into
`FUN_000a97f0`'s own frame, so the two are not obviously the same value, and
resolving that needs either a slower instruction-by-instruction trace of
`FUN_000a97f0`'s own prologue (its `param_2`/`in_r8` handling suggests the
buffer might be threaded through a call this pass didn't follow into
`_opd_FUN_000a8198`) or the live `Z0` check the 2026-09-23 section already
proposed, on this call site instead. **Left exactly as it was**: this
engine's sphere keeps the camera-facing basis, chosen not measured, and nothing
above changes that gate.

## 2026-09-25: the sphere's `noise.gtf` is read, and it rides `UV_offset`

`plasmasphere_glow.rcsmaterial` (`HD_plasma_sphere`'s own), the lit race
pass's variant `@0x2690` (`scripts/ps3-microcode.py fp-file`), declares three
samplers: `Texture1` (`0x3bdc0403`, `plasma_1024x1024.gtf`) on unit 1,
`0x4bb6f08c` (`noise.gtf`) on **unit 2**, and `paraboloidReflectionTex`
(`0x9edd3243`, engine-bound, no `.gtf` in the model) on unit 0. Its seven
parameters are the sun's direction and colour, `fogColour`,
`globalAlphaScaler`, `constantAmbientColour`, `0x7480de6d` (a `float4`, no
preimage, not authored by the model) and **`UV_offset`** - `0x8f2fe704` is
`~crc32("UV_offset")`, and the model authors `0.52704` for it, which the
engine then overwrites by pointer (the 2026-09-25 section above).

The noise tap, instruction by instruction:

```text
@0x0e  MOV R1.z, TC0.w                      u
@0x11  MOV R0.z, TC1.w                      v
@0x16  MOV R2.x, {UV_offset}
@0x30  MUL R1.w, R0.z, 15                   15 v
@0x34  MAD R1.zw, R2.x, 10, R1              (u + 10 UV_offset, 15 v + 10 UV_offset)
@0x36  TEX H3.xyz, R1.zwzz unit2            noise.rgb
@0x5d  MUL R3.w, R1.x, R3                   a sun term: pow(N.H-like, 32) * saturate(N.L)
@0x5f  MUL H6.xyz, R3.w, {sunColour}
@0x65  MUL H3.xyz, H3, H6
@0x66  MUL H2.xyz, H3, 20                   20 * noise * sun * spec
@0x6c  MAD H1.xyz, H1, (1.93, 1.2, 8), H2   added into the colour, then the fog lerp
```

So the noise is sampled at a coordinate that **is** `UV_offset` scrolled
across a 1 x 15 tiling, and it modulates a specular sun highlight twenty-fold
before that is added to the sphere's colour; the same `UV_offset` also shifts
the unit-1 plasma texture's first tap (`@0x1a`, `v + 0.01 UV_offset`) and
scales the alpha (`@0x68`). Confidence 85 for the listing.

**Left unwired, on purpose.** Even with `UV_offset` fed from the node's own
Anim Transform track at `age`, the program still needs the sun's direction
and colour, the constant ambient, `0x7480de6d` (which nothing in the model
authors and whose engine source is unread) and a paraboloid reflection probe
this renderer does not have - it is a lit, reflective program, not an unlit
one, so the path `docs/rendering/hd-unlit-programs.md` added does not cover it
and drawing the noise alone would be an invention of its combine.

## 2026-10-05: `UV_offset` is the blast's age, not a keyed track, and it is played on the ring and halo

**`hd-weapons`.** The 2026-09-25 section above left `UV_offset`'s law "a data
read: parse the Anim Transform track at that node". Parsed, and it is not one.
Corrects that section's "keyframe evaluator" reading, which followed the wrong
vtable (the correction [gantry-clock.md](gantry-clock.md) already recorded).

**The data side, negative.** `crates/render/examples/hd_weapon_anim_keys.rs`
dumps every `0x3c0` node of the trio (`keys.txt` has
the run). Each of `HD_plasma_ring`, `_sphere` and `_halo` authors exactly one
`Anim Transform` node, `Root`, with a single scale key `(256, 256, 256)` (1.0)
and a single translation key at frame 36000 with a `1.8e-43` quantum, no
rotation. Nothing in those keys moves, so no scroll can come from them.
(`HD_missile_explosion`'s `sphere`/`bloom`/`rays`/`shockwave` nodes do carry
scale keys: 25 keys 1 -> 18 over 60 frames, the shockwave 25 -> 11264 /256.)

**The code side, which settles it.** `AnimNode_GetTime` (`0x002c0d78`) returns
`*(float *)(node + 0xc0)` of the first node whose type is `*0x008b3988`, and
`AnimNode_FindTransformValueField` (`0x002c11c8`) returns the address of that
same field - [gantry-clock.md](gantry-clock.md) reads the class as
`MeshImporter`, whose slot `+0x40` is `MeshImporter_SetTime` (`stfs f1,
0xc0(r3)`), and `AnimNode_UpdateTransformTree` (`0x002c1b30`) calls that slot
on the tree. `WeaponExplosions_Draw` calls it with the blast's `age`. So the
`UV_offset` every one of the three materials is bound to **is the blast's age
in seconds**, set each draw, unbounded (no `fmod`). `Shockwave_scalar` on the
Missile's own explosion is the same field, i.e. that object's age too.
Confidence 80: every link is read, no live watchpoint, and the age passed by
`Draw` is read off the decompile rather than a breakpoint.

**Which programs read it, and how.** `hd_plasmaring_glow` (race block
`@0x1900`) and `hd_plasmahalo_glow` (`@0x1960`) declare `UV_offset`; the
sphere's `plasmasphere_glow` does too, but through the lit program above.
`scripts/ps3-microcode.py fp-file`, `t = UV_offset`, `(u, v) = TC3.xy`:

```text
ring   n  = tex(u, v + 0.01 t).a
       uv = (u, v) + 0.15 n + 0.1 t            (both axes, each term)
       rgb = tex(uv).rgb * TC0 ;  alpha = TC3.z * (r + g + b)  (then fog, globalAlphaScaler)
halo   n  = tex(u, v + 0.4 t).a
       uv = (2u + 0.1 t, v + 0.04 t + 0.1 n + 0.1 t)
       rgb = tex(uv).rgb * TC0 ;  alpha = (r + g + b) * globalAlphaScaler.y / 3 + .x
```

(The halo's `ADD R0.xy, R1, R0.zwzz` adds `R0.z = u` to `u`, so its first
coordinate doubles; read from the listing, not a typo.)

**Played.** Two slot bits (`slots::CLOCK_SCROLL_RING`, `_HALO`, matched by
`mesh::rcs::rim_glow`'s fingerprint) and a per-drawable `model_clock` in the
mesh uniform (a pad, as `sun_occlusion_layer` was) that `mesh.wgsl` reads in
place of the scene's `time` where those bits are set; the blast writes its age
into it. **Only where the colour tap samples** moves; the programs' own colour
and alpha combine is not reproduced, the engine's path stands in for it.

**The sphere stays unwired, and says so.** Its `noise.gtf` tap only reaches
the colour through the sun's specular term (`20 * noise * sun * spec`), and the
sun direction and colour, the ambient, `0x7480de6d` and the reflection probe
are all unwired. Its `UV_offset` therefore has nothing to move: the sphere
draws exactly as before, a lit plain-textured ball without the noise highlight.
(Its plasma tap's `v + 0.01 UV_offset` and alpha scale are not played alone:
the alpha operand is unread.)

**What a player sees.** The dome is the ring and halo; their texture now
flows. `--give plasma` grid shot, ticks 185/200/215/230 (HD, chase camera):
the streak pattern in the purple dome shifts frame to frame instead of sitting
fixed on the shell (537,340 pixels differ at tick 185, 41,672 at 200, 152,234
at 215, 58,312 at 230; 0 at 175, before the blast). Subtle by construction:
0.1 per second over a 1.3 s window is 0.13 of the texture. Pulse's frame of
the same shot is pixel-identical before and after (ticks 200 and 230).
