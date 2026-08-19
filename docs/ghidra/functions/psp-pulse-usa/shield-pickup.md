# The Shield pickup: what arms it, what it absorbs, and what it draws

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** the arm, the countdown, the consumers of its flag and the whole of
its visual object are read end to end, statically. No runtime leg yet, so
nothing here exceeds **88** per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md).

This page is about the **pickup** - the thing a Weapon Pad hands out and
`SQUARE` fires. It is a different subsystem from the energy pool, which
[`shield.md`](shield.md) covers; the two share only a name. Read that page for
`entity+0x88`, `Ship_Damage` and the `0.035` contact coefficient.

It was opened because the reimplementation had a Shield that made a craft
immune to everything and showed nothing at all, both marked "ours" in
`crates/physics/src/damage.rs` and `crates/game/src/race/weapons.rs`. Both are
recovered here, and one of the two guesses turns out to have been right.

## The Shield is fire bit `0x20`, not `0x400`

`Weapons_DispatchFire` (`0x08861814`) dispatches sixteen bits of `craft+0x1b8`,
and [weapon-fire.md](weapon-fire.md) reads that dispatch. Which bit is the
Shield was **not** settled there: the weapon-id-to-bit map on
[missile.md](missile.md) gives `4 Shield -> 0x400`, and the `0x400` handler
turns out to be the Turbo.

**The discriminator is the stat offset, and it is decisive.**
[weapon-stats.md](../../../formats/weapon-stats.md) records three
byte-identical parsers writing three adjacent eight-byte slots. Decompiled here
to fix which attribute lands where:

| Parser | Address | `time` | `absorb` |
| --- | --- | --- | --- |
| `WeaponStats_ParseTurbo` | `0x0880c92c` | `+0x84` | `+0x88` |
| `WeaponStats_ParseShield` | `0x0880ca2c` | **`+0x8c`** | `+0x90` |
| `WeaponStats_ParseAutopilot` | `0x0880cb2c` | `+0x94` | `+0x98` |

The two key strings are read out of `.rodata` rather than assumed - `time` at
`0x08a78b00`, `absorb` at `0x08a78aac` - and each parser's `if`/`else` tests
`time` first.

Now the two handlers, and each reads exactly one of those offsets:

- `FUN_088614c4`, dispatched on bit **`0x400`**, loads `stats[+0x84]` - the
  **Turbo's** `time` - into `craft+0x14c` and sets running bit `0x200`. That
  agrees with `docs/gameplay/pickups.md`, which already recorded `0x200` as the
  Turbo's engine gate, reached from the opposite direction.
- `FUN_08861568`, dispatched on bit **`0x20`**, loads `stats[+0x8c]` - the
  **Shield's** `time` - into `craft+0x188` and sets running bit `0x10`.

Confidence **88**: a parser and a consumer recovered independently, agreeing on
an offset, with a third pair - the Turbo - checking the same argument against a
fact this project already held from elsewhere.

**This contradicts the id-to-bit map** at
[missile.md](missile.md#weapon_requestfire-and-the-weapon-id-to-bit-map), which
makes `0x400` id 4, and [ai-stats.md](ai-stats.md), which calls id 4 the Shield
and id 5 the Cannon. Both readings are left standing and the conflict recorded
rather than resolved: that page already flags three loose ends in the same
switch, and the offset evidence here does not say which of them is the mistake.

### Two addressing traps on the way, both worth writing down

**`weapon-fire.md`'s five small-handler addresses are wrong by `0x4000`.** The
decompiler prints an unresolved call as `func_0x000NNNNN`, and the real address
is that plus the **image base `0x08804000`**, not `0x08800000`. That page got
the resolved names right (`func_0x0006a104` -> `Weapon_FireRocket` at
`0x0886e104`, and `func_0x0003f9ac` -> `Ship_Damage` at `0x088439ac`, both
checked) and the unresolved ones wrong, so its `FUN_0885d3bc`, `0x0885d404`,
`0x0885d534` and `0x0885d630` all land inside `Rocket_Update`
(`0x0885d2a8`-`0x0885db37`) and decompile as that function. The five real
handlers are `0x088613bc`, `0x08861404`, `0x088614c4`, `0x08861534` and
`0x08861630`. Corrected on that page in the same change as this one.

**Ghidra has no cross-reference to any string in this image**, because the
`lui`/`addiu` pairs are not relocated in the loaded program. `get_xrefs_to` on
a string address returns nothing and a byte search for that address finds
nothing, which reads exactly like "unused". What does work: take the
unrelocated value, split it the way the assembler would, and search for the
`addiu` immediate. `%s\%sshield.vex` at `0x08a7c178` is reached by
`addiu ..., -0x7e88` at `0x0885dce0`, and that single search found the whole
visual subsystem below.

## `FUN_08861568` arms it, `FUN_08861630` counts it down

```c
// bit 0x20 - the fire request
void Shield_Fire(World *w, Craft *craft)
{
    if ((craft->fire_flags & 0x10) == 0) {          // not already running
        craft->fire_flags |= 0x10;                  // running
        craft->shield_timer = weapon_stats[0x8c];   // <Weapon type="Shield"><Stats time>
        craft->held = -1;                           // craft + 0x1bc
        Shield_Activate(craft->entity);             // 0x0883e544 - cues and visual
        if (g_game_mode >= 0xe) net_broadcast(...); // multiplayer only
    } else {
        craft->fire_flags &= ~0x20;                 // drop the request, keep the shield
    }
}
```

```c
// bit 0x10 - the running shield, once a frame
void Shield_Update(float dt, World *w, Craft *craft)
{
    craft->shield_timer -= dt;                      // stored unconditionally
    if (craft->shield_timer <= 0.0) {
        craft->fire_flags &= ~0x10;
        Shield_Deactivate_q(craft->entity);         // 0x0883e5c0
        if (g_game_mode >= 0xe) net_broadcast(...);
    }
}
```

Three behaviours fall out, and the first is not obvious:

- **A shield cannot be refreshed while one is running.** The second press is
  swallowed by the `else` arm, which clears only the *request* bit. The pickup
  is still gone - `WeaponPickup_Grant` cleared `craft+0x1bc` when it handed the
  Shield over - so firing a Shield into a running Shield wastes it.
- **The arm takes two frames to settle.** The `if` arm does not clear its own
  `0x20`, so the next frame's dispatch re-enters, finds `0x10` already set and
  clears it there. Every other handler on the dispatch clears its own bit.
- **The timer is stored before the test**, exactly as the Turbo's is, so a
  `time` of `t` protects for one tick more than `ceil(t / dt)`.
  `crates/physics/src/damage.rs`'s `advance_shield_pickup` already has that
  shape, arrived at for its own reason, and it is the original's.

`Shield_Activate` (`0x0883e544`) plays two cues and arms the visual:

```c
if (DAT_08ac1dfc != 0) Sound_Play(entity, DAT_08ac1dfc, "shieldactive", 0x400, 0);
ShipShield_Activate(entity->0x8c8);
Sound_PlayLooping(1.0, entity->0x50, DAT_08ac1df8, 0, "~SHIELD", entity + 0x54);
```

`"shieldactive"` (`0x08a7b7a4`) and `"~SHIELD"` (`0x08a7b770`) are reached
through pointer globals at `0x08a7b7b4` and `0x08a7b778`, whose contents were
read out of the image directly rather than inferred from adjacency. The `~`
prefix is this binary's own convention for a looping cue - `~AIRBRAKE_MONO`,
`~ROCKETTVL`, `~MISSILETVL`, `~BLOWUP` all carry it - so `~SHIELD` runs for the
duration and `shieldactive` is the one-shot on activation.

## What the flag gates: two drains and a contact loop, and no fourth thing

`craft+0x1b8 & 0x10` is tested in exactly nine places in the whole image, found
by scanning every `andi ..., 0x10` across all 523,246 instructions and
rejecting the ones whose base is not `0x1b8` of a craft. Three are the
subsystem itself - the dispatch and `FUN_08861568`'s own re-arm guard. The
other six are the whole of the shield's effect.

### 1. Weapon damage is absorbed whole, and replaced by a flash

`FUN_0883f13c` drains a **posted** damage amount once a frame:

```c
if (craft && craft->pending_damage > 0.0 && Ship_State(entity) == 1) {
    if ((craft->fire_flags & 0x10) == 0) {
        Ship_Damage(craft->pending_damage, entity, 2 /* weapon */,
                    craft->0x138 /* kind */, craft->0x124 /* by_rival */);
        if (craft->0x138 == 7) entity->0x94->0x31c = craft->0x134;
    } else if (ShipShield(entity->0x8c8)->active) {
        ShipShield_Hit(entity->0x8c8);              // 0x0885eb04
    }
    craft->pending_damage = 0.0;                    // consumed either way
}
```

`craft+0x120` is the pending amount, `+0x124` the by-rival flag, `+0x138` the
weapon kind. So a shielded craft takes **no weapon damage at all**, the amount
is discarded rather than deferred, and the hit is not silent: it arms the
shield's hit flash instead. Confidence **82**.

This is the leg that says the reimplementation's blanket refusal was right for
weapons: `crates/physics/src/damage.rs` refused every source while marking the
refusal "ours", and for weapons the original refuses too.

### 2. The posted collision impulse and its stun are suppressed

`Ship_ApplyCollisionImpulse` (`0x0883f274`), flag test at `0x0883f2e4`:

```c
if (craft->pending_impulse != 0) {                  // craft + 0x110, a vec3
    if ((craft->fire_flags & 0x10) == 0) {
        len  = |pending_impulse|;
        axis = *(entity + 0x794);
        craft->pending_impulse = axis * (dot(axis, pending_impulse) >= 0 ? len : -len);
        Body_ApplyImpulseAtPoint(axis, point, &craft->pending_impulse);
        entity->0x290 += 0.5;                       // the stun
    }
    craft->pending_impulse = 0;                     // cleared either way
}
```

That closes an open item on [contact-response.md](contact-response.md), which
had narrowed this flag to "bit `0x10` at `*(entity+0x4c) + 0x1b8`, what sets it
is still unread". **The Shield sets it, and nothing else in the image does.**

**And it does not touch wall contact.** `Body_ResolveContact` applies the track
impulse unconditionally, and nothing on the track path posts `craft+0x110` -
which that page establishes independently, from a trace where `stun_timer`
reads `0.0` on all 3,146 ticks of a lap. So a shielded craft still bounces off
a wall exactly as an unshielded one does. Only the rival/weapon knockback and
its half-second stun go away.

### 3. The contact loop's hull damage becomes the same flash

`FUN_088418e0` tests the bit at `0x08841fcc`, `0x088420e8`, `0x08842560` and
`0x08842684`. [contact-response.md](contact-response.md) already documents the
outcome from the other side, as its reaction #3: the shield branch is taken
*instead of* the hull-damage call, and it is `FUN_0885eb04(shieldEntity)` - the
same `ShipShield_Hit` as above.

The block behind the first of the four gates is larger than a damage call: it
also adds a stats value to `craft+0x130`, writes `craft+0x138 = 5`, latches
`entity->0x860 |= 0x40` so it fires once, and plays a cue. What that whole
block is has **not** been read; it is recorded here as skipped-when-shielded
and left open.

### And there is no fourth

No projectile, blast or rival-damage function reads the flag. `Ship_Damage`
itself does not test it - its gate is the craft state and
`entity->0x860 & 0x1000`, per [shield.md](shield.md) - and
`Projectiles_Update_q`'s `& 0x10` at `0x08869a68` is on a projectile's own
flags at `+0x3c`, not on a craft's word. Absorption happens at the two drain
points above and nowhere else. Confidence **80**: a whole-image scan is good
evidence for an absence, but it keys on one instruction form and would miss a
test written some other way.

## The visual is two authored models, and both are on the disc

`FUN_0885db38` builds the shield's visual object and names both models:

```c
team_model = config("FE_TeamModel") ?: "ship";                  // literal default
snprintf(buf, 128, "%s\\vr_shield_cockpit.vex", "Data\\Weapons");
obj->cockpit = load_model(buf);                                 // obj + 0x7c
snprintf(buf, 128, "%s\\%sshield.vex", craft->team->dir, team_model);
obj->shell   = load_model(buf);                                 // obj + 0x80
obj->active = 0; obj->fading = 0; obj->time = 0;
```

So the two names are `Data\Weapons\vr_shield_cockpit.vex` and
`Data\Ships\<Team>\shipshield.vex`. **Both are on the USA disc**, confirmed by
hashing the assembled names against `Data.wad`'s directory - the shell for all
eight playable teams, the cockpit once. `oag-view --nodes` reads them:

| Entry | Tree |
| --- | --- |
| `Data\Ships\<Team>\shipshield.vex` | one `Mesh` (`polySurfaceShape6`, 3,616 bytes) + `Texture Data\Weapons\Textures\pulse_shield_test_ADD.tga` |
| `Data\Weapons\vr_shield_cockpit.vex` | `Anim Transform pSphere3` -> `Mesh pSphereShape3` (4,176 bytes) + `Texture Data\Weapons\Textures\noise1_ADD.tga` |

Both textures carry the `_ADD` suffix the artists use for an additive material,
which `oag_render::mesh_render::blend::ADDITIVE_BLEND` already has a pipeline
for.

`Data\Weapons\shield.vex` is on the disc too, the same mesh and the same
texture as the per-team shell. **No code path assembles that name** - the
format string always supplies a directory *and* a prefix - so it reads as a
stray copy and is not the thing to draw.

`crates/game/src/race/weapons.rs` said `Data\Ships\<Team>\<Team>shield.vex`.
That name hashes to nothing in the archive, and it is the reason the feature
was recorded as unbuildable.

## `ShipShield_Update` (`0x0885e254`) is the whole animation

```c
if (!obj->active) return;                               // obj + 0x74
steps = (int)(dt / 0.016666668);                        // fixed 60 Hz substeps

for (i = 0; i < steps; i++)                             // colour, obj+0x40, rgba 0..1
    obj->rgba += (obj->rgba_target - obj->rgba) * 0.15; // target obj+0x50, rate obj+0x60

for (i = 0; i < steps; i++)                             // swell, obj+0x64
    obj->swell += (1.0 - obj->swell) * 0.2;             // target obj+0x68, rate obj+0x6c

n     = noise(obj->time);                               // obj + 0x78
alpha = obj->rgba.a * (n * 0.25 + 0.75);
scale = obj->swell + 0.012 + n * 0.012;

if (craft->0x6d == 0) {                                 // external camera
    draw shell   at scale;         hide cockpit;
} else {                                                // cockpit camera
    draw cockpit at scale * 1.8;   hide shell;
}
set_model_colour(model, pack_abgr(alpha, rgba.b, rgba.g, rgba.r));

obj->time += dt;
if (obj->fading && obj->rgba.a <= 0.1) {                // obj + 0x75
    obj->active = 0; obj->fading = 0; hide both;
}
```

`ShipShield_Hit` (`0x0885eb04`) is five stores: a hit colour into `obj+0x40`
and **`1.1` into `obj+0x64`**. So an absorbed hit re-flashes the colour and
pops the swell to `1.1`, which the `0.2`-per-step approach pulls back to `1.0`
over about a fifth of a second - the shield visibly bulges where it took the
hit and settles. `ShipShield_Activate` (`0x0885de4c`) sets the same fields from
an activation colour, writes `obj->0x60 = 0.15`, `obj->0x68 = 1.0`,
`obj->0x6c = 0.2`, and sets `obj->active = 1`.

The literals `0.15`, `0.2`, `1.0`, `1.1`, `0.012`, `0.25`, `0.75` and `1.8` are
instruction immediates, read directly. Confidence **84**.

**The three colour vec4s are not resolved.** They are `lui 0x6`/`addiu`
constant-pool loads - `0x627a8` (target), `0x627b8` (hit) and `0x627c8`
(activation), unrelocated - and the relocation model that works for text and
for string pointers puts all three inside `.text`, where the bytes decode as
instructions. So the shield's **tint** is unknown; its geometry, its texture,
its timing and its shape are not. Reproducing the tint by eye is exactly the
invention `CLAUDE.md` forbids, so the reimplementation draws the model's own
authored colour and records the gap here. Whoever fixes the relocation model
closes this in one read.

## Names recovered

All in `names.tsv` in this change.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08861568` | `Shield_Fire` | 88 |
| `0x08861630` | `Shield_Update` | 88 |
| `0x088614c4` | `Turbo_Fire` | 88 |
| `0x0883e544` | `Shield_Activate` | 84 |
| `0x0883e5c0` | `Shield_Deactivate_q` | 68 |
| `0x0885db38` | `ShipShield_Construct` | 85 |
| `0x0885de4c` | `ShipShield_Activate` | 84 |
| `0x0885e254` | `ShipShield_Update` | 84 |
| `0x0885eb04` | `ShipShield_Hit` | 80 |
| `0x0883f13c` | `Ship_ApplyPendingWeaponDamage` | 82 |

`Shield_Deactivate_q` keeps its `_q`: it is named from its one call site - the
countdown's expiry arm - and its body is not read. `0x08861534`, the Turbo's
countdown, is disassembled and understood but has no function defined in the
database and no page section of its own, so it gets no row.

## What is not verified

- **No runtime leg.** Nothing on this page has been watched in PPSSPP. Every
  claim is a static read, which is why nothing exceeds 88.
- **The three colour constants**, above.
- **The block behind the contact loop's first shield gate**
  (`0x08841fe0`-`0x08842088`), which does more than post damage.
- **The id-to-bit conflict** with [missile.md](missile.md) and
  [ai-stats.md](ai-stats.md).
- **`FUN_0883efb4`**, the third member of `FUN_0883f13c`'s family, which also
  tests the flag and is unread.

## History

- 2026-08-19: page created. The Shield's fire bit settled at `0x20` against the
  three parsers' stat offsets, the consumers of flag `0x10` read, the two
  authored models located and confirmed present on the disc, and the visual
  object's animation read in full.
