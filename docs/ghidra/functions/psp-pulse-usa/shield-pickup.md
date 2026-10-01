# The Shield pickup: what arms it, what it absorbs, and what it draws

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** the arm, the countdown, the consumers of its flag and the whole of
its visual object - including its colours - are read end to end, statically. No
runtime leg yet, so nothing here exceeds **88** per the
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
        Shield_Deactivate(craft->entity)   ;         // 0x0883e5c0
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

**Both are now played, and the banks corroborate the reading.** `~SHIELD`
resolves in `weapons.bnk` to **two waveforms, both carrying the descriptor's
loop flag**; `shieldactive` resolves in **`speech.bnk`**, not `weapons.bnk`,
which is what says it is a voice line rather than an effect. The loop flag is a
field the name table knows nothing about, so the `~` convention read above is
confirmed from the data side rather than restated. `oag_game::audio::sfx` holds
the looping voice for exactly as long as `shield_pickup_timer` runs and fires
the announcer once on the same activation. See
[psp-audio.md](../../../formats/psp-audio.md#a-cue-owns-a-run-of-the-command-table).

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

### 3. The contact loop, all four gates read

`FUN_088418e0` tests the bit at `0x08841fcc`, `0x088420e8`, `0x0884255c` and
`0x08842684`, and **all four are read** - which matters, because the claim that
rests on them is the one this whole page is load-bearing for: *a shielded craft
still bounces off a wall exactly as an unshielded one does.* Not one of the four
touches a velocity, an impulse or a position.

| Gate | Skipped when shielded |
| --- | --- |
| `0x08841fcc` | A once-latched block (`entity->0x860 & 0x40`): adds a stats value to `craft+0x130`, writes `craft+0x138 = 5`, calls `Ship_Damage(stats[0x60], entity, 2, kind, 0)` and plays a cue. **Not read further** and left open. |
| `0x088420e8` | Four instructions reading `craft+0x130`, the accumulator the block above writes. |
| `0x0884255c` | **The collision sparks.** The proximity test on `DAT_08ab10b0 + 0x70` and the call to `Ship_DispatchCollisionFx` (`0x0883de90`) - reaction #1 on [contact-response.md](contact-response.md). A shielded craft throws no hull sparks at all. |
| `0x08842684` | **The damage/flash switch**, reaction #2 against #3: `Ship_Damage(|p| * 0.035, ...)` plus `entity->0x860 |= 0x20 \| 0x400000` on one side, `if (shield->active) ShipShield_Hit(shield)` on the other. |

The third is a finding in its own right and is not on
[contact-response.md](contact-response.md): **sparks are the hull being hurt,
and a shielded hull is not being hurt**, so the shell's own bulge is the whole of
what a shielded contact shows. Confidence **84**.

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
texture as the per-team shell. **No code path assembles that name** on this
binary - the format string always supplies a directory *and* a prefix - so on
Pulse it reads as a stray copy and is not the thing to draw.

### The same asset across the three titles, checked on all three

Each name hashed or listed against that disc's own archive directory:

| Title | Per-team shell | Shared `Data\Weapons\shield.vex` |
| --- | --- | --- |
| **Pulse** (PSP) | `Data\Ships\<Team>\shipshield.vex`, all eight teams | present, unreferenced |
| **Pure** (PSP) | **absent** - and no `shipboost.vex` either | present |
| **HD/Fury** (PS3) | `/data/ships/<team>/shipshield.vex` + `.rcsmodel`, all eight teams plus Zone and Detonator | absent |

So the effect is the same one across the lineage and only the *packaging*
moves: Pure has the shared model and no per-team one, HD has the per-team one
with its geometry in the sibling `.rcsmodel` the PS3 build splits every model
into, and Pulse has both. **HD also authors a material per team**
(`/data/materials/ships/<team>_shield.rcsmaterial`), which nothing in this
project reads yet - so an HD shell draws its real geometry with no material at
all, which is brighter and flatter than the disc's. The loader report says so
on every HD boot rather than leaving it to a screenshot.

That table is why the entry name is a per-title axis
(`oag_pulse::race::ships::SHIELD` and `oag_pulse::race::SHARED_SHIELD`) rather
than a constant: it is not one name with a fallback, it is three packagings of
one effect.

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

n     = sinf(obj->time);                                // obj + 0x78, seconds
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

`ShipShield_Deactivate` (`0x0885e1c0`), which `Shield_Deactivate` calls, is the
other end:

```c
obj->rgba_target = (0, 0, 0, 0);        // obj + 0x50, all four channels
obj->swell_target = DAT_08ab0f1c;       // obj + 0x68, an unresolved constant
obj->fading = 1;                        // obj + 0x75
hide shell; hide cockpit;               // clear bit 2 of each model's +0x2c
```

**The two hides are per-frame bookkeeping, not a stop**, and reading them as a
stop would delete the fade: the update sets the drawn model's flag again on the
very next frame, every frame, so the shell keeps drawing until the
`alpha <= 0.1` test at the bottom of the update takes it down. What ends a
shield is the fade, not the deactivate.

`ShipShield_Hit` (`0x0885eb04`) is five stores: a hit colour into `obj+0x40`
and **`1.1` into `obj+0x64`**. So an absorbed hit re-flashes the colour and
pops the swell to `1.1`, which the `0.2`-per-step approach pulls back to `1.0`
over about a fifth of a second - the shield visibly bulges where it took the
hit and settles. `ShipShield_Activate` (`0x0885de4c`) sets the same fields from
an activation colour, writes `obj->0x60 = 0.15`, `obj->0x68 = 1.0`,
`obj->0x6c = 0.2`, and sets `obj->active = 1`.

The literals `0.15`, `0.2`, `1.0`, `1.1`, `0.012`, `0.25`, `0.75` and `1.8` are
instruction immediates, read directly. Confidence **84**.

### The camera branch is a second consumer of `craft+0x6d`

`crate::display::CameraView::draws_own_ship` in this project reads the same
byte and recorded it at confidence **70**, as "inference from the correlation
rather than from a consumer" - the original sets it to 1 for `OPT_INT` and 0 for
both external views, and nothing had been found that *used* it.

This is that consumer, and it agrees. Choosing between a hull-shaped shell and a
sphere scaled to clear the hull only makes sense as "the camera is inside this
craft", and it is an independent site rather than the same correlation read
twice. Raised to **82** on that page in the same change.

**The `1.8` is not arbitrary either**: `vr_shield_cockpit.vex` is authored at
radius `3.99` against an Assegai hull of `6.97`, so `1.8` carries it to `7.2` -
just past the hull, which is exactly what a shell has to do to enclose a camera
sitting inside one.

**The flicker is `sinf`, and this nearly went down as unreadable.** The update
calls one function with the object's accumulated time and spends the result on
both the alpha and the scale, and the decompiler prints it as an unresolved
`func_0x0017a300` - which is `0x0897e300`, and decompiles as the C library's
`sinf`. So the argument is radians against a clock in seconds and the shell
breathes on a **`2*pi`-second** cycle, between half and all of its colour's
alpha. The first implementation pass here filled that slot with an invented
~2.7 Hz shimmer, which is a visibly different effect; recorded because the trap
is general - an unresolved `func_0x...` in this image is not evidence that
anything is unreadable, only that the call was not relocated.

## The colours, and the two things that hid them

The first pass on this page recorded the three colour vec4s as unresolvable and
had the reimplementation draw the shell white. Both halves are now settled, and
**how** they hid is the generally useful part - the same two traps sit under
every unresolved constant in this image.

### A data reference relocates against the *other* segment

This is an `ET_SCE_RELEXEC` PRX with two `LOAD` segments: text+rodata+data at
vaddr `0`, and `.cplinit`/`.linkonce.d`/`.ctors`/**`.bss`** at vaddr `0x2d5798`.
Its relocations live in `SHT_PRXRELOC` (`0x700000a0`) sections of ordinary
`Elf32_Rel` shape, and `r_info` packs three fields rather than one: the type in
bits 0-7, the segment holding the *site* (`OFS_BASE`) in bits 8-15, and **the
segment holding the *target* (`ADDR_BASE`) in bits 16-23**.

Ghidra applies none of them, so a `lui`/`addiu` pair prints its raw addend and
the reader has to supply the base. Adding the image base `0x08804000` - which is
right for every `jal`, and right for the string pointers this page already used
- puts `0x627a8` at `0x088667c8`, **inside `.text`**, where the bytes decode as
instructions. That reads exactly like "there is no such constant", and it is why
the first pass gave up.

Read from the table instead, the `ShipShield_Hit` pair at raw `0x5ab04`/`0x5ab08`
is `R_MIPS_HI16`/`R_MIPS_LO16` with `OFS_BASE=0` and **`ADDR_BASE=1`**. So the
base is `0x2d5798 + 0x08804000 = 0x08ad9798`, and the three addends land at
`0x08b3bf40`, `0x08b3bf50` and `0x08b3bf60` - in `.bss`. That also settles
[contact-response.md](contact-response.md)'s `DAT_08b3bf50`, which is the same
address reached by the same arithmetic.

**The rule, for the next constant:** a `jal` target is `addend + 0x08804000`; a
`lui`/`addiu` data reference is `addend + 0x08804000` when `ADDR_BASE` is `0`
and `addend + 0x08ad9798` when it is `1`, and the only way to know which is to
read `r_info`. A quick test that needs no parser: if the segment-0 reading lands
below `0x08a76a3c` it is pointing into `.text` and is almost certainly wrong.

### Their one writer is a static initialiser no call reaches

`.bss` is zeroed at load, so the values have to be written at runtime - and
nothing `jal`s the function that writes them. `FUN_0885eb54`, sitting
immediately after `ShipShield_Hit`, stores all four vec4s from two registers
(`f12 = 0.0`, `f13 = 1.0`) and returns; a whole-image scan for both a `jal` and
a stored pointer to it finds **nothing**.

It is not dead. It is entry 15 of **`.cplinit`**, the C++ static-initialiser
list at `0x2d5798`, as the pair `(0x0885eb54, 0)` - found by scanning the raw
image for the word rather than by following a reference. The same scan run
against `ShipShield_Hit` finds its two known callers, which is what says the
method works.

### The values

| Addend | Address | Value | Role |
| --- | --- | --- | --- |
| `0x627c8` | `0x08b3bf60` | `(0, 0, 0, 0)` | what `ShipShield_Activate` sets the **current** colour to |
| `0x627a8` | `0x08b3bf40` | `(1, 1, 1, 1)` | what it sets the **target** to |
| `0x627b8` | `0x08b3bf50` | `(0, 1, 1, 1)` | what `ShipShield_Hit` sets the current colour to |
| `0x627d8` | `0x08b3bf70` | `(0, 0, 0, 0)` | written by the initialiser, read by nothing on this page |

And two more from `.data`, which were never in doubt once the right base was
used: `ShipShield_Activate` sets the swell to `0.7` (`0x08ab0f18`) and
`ShipShield_Deactivate` retargets it to `1.2` (`0x08ab0f1c`).

So the whole animation, which is a good deal more than "a shell appears":

- **Activation** starts the shell at transparent black and `0.7` of its size,
  and lerps it to white and `1.0`. It **fades up and grows into place**.
- **An absorbed hit** sets the colour to `(0, 1, 1, 1)` and the swell to `1.1`.
  The red channel alone drops out, so against the mesh's own authored
  `(0.55, 0.50, 0.91, 0.50)` the flash is a **hue shift to cyan** rather than a
  brightening - the alpha is untouched. Both settle back over about a quarter of
  a second.
- **Expiry** targets transparent black *and* `1.2`, so the shell **blows outward
  as it fades** rather than shrinking away.

White being the target is what makes the settled shell exactly what the artists
painted: the draw multiplies these against the model's own vertex colours, and
white is that multiply's identity. Confidence **84**, the same as the rest of
the object - the arithmetic is unambiguous and there is still no runtime leg.

## Names recovered

All in `names.tsv` in this change.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08861568` | `Shield_Fire` | 88 |
| `0x08861630` | `Shield_Update` | 88 |
| `0x088614c4` | `Turbo_Fire` | 88 |
| `0x0883e544` | `Shield_Activate` | 84 |
| `0x0883e5c0` | `Shield_Deactivate` | 80 |
| `0x0885db38` | `ShipShield_Construct` | 85 |
| `0x0885de4c` | `ShipShield_Activate` | 84 |
| `0x0885e1c0` | `ShipShield_Deactivate` | 84 |
| `0x0885e254` | `ShipShield_Update` | 84 |
| `0x0885eb04` | `ShipShield_Hit` | 80 |
| `0x0885eb54` | `ShipShield_InitColours` | 82 |
| `0x0883f13c` | `Ship_ApplyPendingWeaponDamage` | 82 |

`0x08861534`, the Turbo's
countdown, is disassembled and understood but has no function defined in the
database and no page section of its own, so it gets no row.

## 2026-10-01: the shell watched on PPSSPP, side by side with ours

The first runtime leg for this page. Same method as `rocket-visuals.md`'s
2026-10-01 section: `scripts/psp-weapon-pair.py shield` ORs bit `0x20` into the
player's weapon record 120 frames after GO (speed 106.2, Time Trial, Talon's
Junction, Venom/Assegai), the emulator window is photographed at 480x272 up to
240 frames later (two runs, 16 and 17 frames each), and ours is
`oag-game --race --mode time_trial --give shield` with the same
`weapon-after-go.inputs`, at the tick `fire + k - 2` (two frames of display lag).
Frames: `data/scratch/pulse-weapons/shield-orig-tt1`, `-tt2`,
`shield-ours-tt480` (not committed).

What matches, at player size:

- A shell that **fades in and settles**, violet-magenta at first and
  **blue** by about fire+70, with the cap-and-bands look of the authored mesh,
  persisting to fire+240 at least (the pickup's `time` is not read here).
- The shell's size against the craft: about 1.6-1.7 x the craft's width on both
  sides (the original's looks larger only because its craft is about 1.4 x larger
  on screen; the camera framing is not this page's).

What differs, named and not fixed:

1. *Onset.* **Resolved 2026-10-01, second pass: see "The onset, measured live" below.** In the original nothing is visible at fire+12, a faint outline at
   +20 and a clear shell at +28. Ours shows a faint shell at +12 and a clear one
   at +20: **about 6-8 frames earlier**. The recovered fade (`rgba += (target -
   rgba) * 0.15` per 60 Hz step, 90 % in 14 steps) cannot account for an
   original delay of 20+ frames, so something before `ShipShield_Activate`
   delays it in the original (the arm function's own timer, or the `active` flag
   being set late). Not read. This is the same size as the delay ours already
   carries from somewhere; which part of it is the original's is open.
2. *Banding and brightness.* The original's concentric cyan bands are sharper
   and its shell reads as brighter and more opaque at fire+50 to +150 than ours.
   Not measured in numbers (a colour comparison needs the same camera framing
   first). Candidate: the sinusoidal alpha `(n * 0.25 + 0.75)` or the model's
   vertex colours.

## The onset, measured live (2026-10-01, second pass)

`psp-weapon-pair.py shield --probe shield` breaks on `ShipShield_Update`
(`0x0885e254`) for 200 frames after the fire and logs the whole object and the `dt` in
`f12` each frame. Time Trial, Talon's Junction, Venom, fire at speed 106.2.

- **Nothing between `Shield_Fire` and the first drawn frame delays the shell.** The
  object is `active = 1` and its clock `0` on the first update (fire+1), the colour is
  `(0, 0, 0, 0)` with target `(1, 1, 1, 1)` (all four equal), the swell starts at
  `0.7` with target `1.0`, the rates are `0.15` and `0.2`. The model at `obj+0x80` has its
  draw bit (`flags & 4`) set from the second frame and the one at `+0x7c` is hidden, in
  the chase camera: `craft+0x6d == 0` takes the update's first branch, which draws
  `+0x80`. Confidence 90 (read live, constant for 60 frames).
- **The fade, swell and clock match `ShipShield::advance` frame for frame when it is fed
  the same `dt` values** (`crates/render/src/shield/tests.rs`, 30 live frames, tolerance
  `1e-5`).
- **The apparent delay is the frame timer.** The `dt` argument is a measured frame time,
  mean 16.682 ms (59.94 Hz) with jitter of 0.1 to 0.6 ms (min 16.061, max 17.260 over 200
  frames) - **measured on PPSSPP**. `(int)(dt / 0.016666668)` is **0 on 47 of those 200
  frames** (`dt` under `1/60`), and a frame with no substep advances neither lerp: the
  original took 153 substeps in 200 frames, 0.765 per frame, so by fire+20 it had done 14
  steps against ours 20 (colour 0.897 against 0.961) - the "six to eight frames" the first
  pass saw. The mechanism (a variable `dt` truncated by `(int)`) is the executable's own,
  read and then reproduced to `1e-5` from the logged `dt`s; **how large the jitter is on a
  real PSP is not measured**, so the rate on hardware is unknown and may be nearer ours' or
  this. Confidence 85 that the onset difference is this and nothing else (the live object
  matches frame for frame; no second delay is visible in it). A fixed 60 Hz engine does not
  reproduce it and nothing here chooses a rate for it: `ShipShield::advance` keeps
  `(dt / SUBSTEP) as i32` as it was.
- **The shell's brightness depends on a second clock phase.** Two original runs 2 s
  apart differ in how clear the shell reads at the same frame (the shell's own texture
  scrolls on the animation clock), so a frame-for-frame brightness comparison needs the
  clock pinned on both sides; none was done. The original's shell also reads brighter
  than ours at settled state (about three times the mean pixel change from the control in
  the craft's box, with the caveat above). Unexplained: candidate is how `mesh+0x6c`
  (written by `Image_SetVertexColours`, `0x089122b4`) reaches the draw - read on
  `mesh-draw.md` as an ambient colour on batches with normals and no vertex colours - against
  how ours multiplies the authored vertex colours.

## What is not verified

- **Almost no runtime leg.** The 2026-10-01 section above watched the shell once
  on PPSSPP, as a picture; no number on this page was measured live. Every claim
  is still a static read, which is why nothing exceeds 88.
- **The block behind the contact loop's first shield gate**
  (`0x08841fe0`-`0x08842088`), which does more than post damage. The other
  three gates are read; see the table above.
- **HD's `<team>_shield.rcsmaterial`**, which is what an HD shell should be
  drawn with.
- **The id-to-bit conflict** with [missile.md](missile.md) and
  [ai-stats.md](ai-stats.md).
- **`FUN_0883efb4`**, the third member of `FUN_0883f13c`'s family, which also
  tests the flag and is unread.

## History

- 2026-08-19, third pass: **the colours resolved**, and with them the fade-up,
  the grow-in, the cyan hit flash and the expand-on-expiry - none of which the
  first two passes knew about. Two things had hidden them: a data reference
  relocates against segment 1 (`ADDR_BASE=1` in the PRX relocation table, which
  Ghidra does not apply), and their one writer is a `.cplinit` static
  initialiser that no call reaches. Both are written up above as general traps,
  because neither is specific to the shield.
- 2026-08-19, same day, second pass: all four contact-loop gates read - the
  third suppresses the collision sparks, which is a finding of its own - and the
  flicker settled as `sinf` rather than left as an unread function. The asset
  checked across Pure and HD/Fury, which is what turned the entry name into a
  per-title axis.
- 2026-08-19: page created. The Shield's fire bit settled at `0x20` against the
  three parsers' stat offsets, the consumers of flag `0x10` read, the two
  authored models located and confirmed present on the disc, and the visual
  object's animation read in full.
