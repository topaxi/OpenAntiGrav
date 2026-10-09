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
`crates/physics/src/damage.rs` and `crates/raceplay/src/weapons.rs`. Both are
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

**A corroborating data point, 2026-10-03, which does not resolve it.**
`Weapon_RequestFire`'s jump table (`.rodata` at link address `0x00278710`,
thirteen entries indexed by the held id) sends index 5 to `ori $4, $6, 0x20` -
this page's Shield bit. And the Eliminator absorb (`Ship_AbsorbHeldPickup`,
mode 8 or `0x12`) writes `5` into the held id through `0x088612e8` and then
calls `Weapon_RequestFire`, so the one caller that picks an id on purpose picks
5 to raise a Shield. Id 5 is the Cannon in `WeaponAiStats_Load`'s order. See
[race-modes.md](../../../gameplay/race-modes.md#eliminator).

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
confirmed from the data side rather than restated. `oag_sound::sfx` holds
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
which `oag_mesh::mesh_render::blend::ADDITIVE_BLEND` already has a pipeline
for.

`Data\Weapons\shield.vex` is on the disc too, the same mesh and the same
texture as the per-team shell. **No code path assembles that name** on this
binary - the format string always supplies a directory *and* a prefix - so on
Pulse it reads as a stray copy and is not the thing to draw.

### The PSP's shell follows the Concept model (2026-10-01, `pulse-hull-bloom`)

`ShipShield_Construct` does **not** use the literal `"ship"` the section above
reads as the format's prefix, except as a fallback. Read again in full
(headless, `program=/pulse/BOOT-psp-pulse-usa.BIN`):

```c
uVar1 = Libc_HashString("FE_TeamModel");
iVar2 = Registry_Lookup(&g_named_registry, 0, uVar1);
iVar3 = 0x8a7c134;                       // the literal "ship"
if (iVar2 != 0) iVar3 = iVar2;           // the registry's value wins
...
FUN_089724b4(buf, 0x80, "%s\\%sshield.vex",
             *(craft->team + 0x94), iVar3);   // team directory, model stem
```

`Ship_LoadModel` (`0x08843258`) reads the **same** registry value for the
player's hull - `puVar11 = Registry_Lookup(FE_TeamModel)` for the player's
craft kind, `"%s\%s.vex"` in every mode but Zone and the Eliminator - and
for the wreck, `"%s\%swreck.vex"`. The Concept hull is `extra.vex`, which only
that registry value can name, so a Concept race has `FE_TeamModel = "extra"`
and the shell is **`Data\Ships\<Team>\extrashield.vex`**. The constructor
takes the *constructed craft's* team directory with the *player's* stem, so
every opponent raises its own team's `extrashield.vex` in that race too.
Confidence **85**: both decompiles agree and the Concept hull's name has no other
source; the live registry value of a Concept race was **not** read (the
Concept model is behind a loyalty unlock this profile does not have).

**The PSP disc carries it**, for all eight teams, in both regions
(`crates/game/examples/pulse_shield_probe.rs`): `extrashield.vex` is a
different file from `shipshield.vex` on every team (9,536 to 9,776 bytes), and it
references `pulse_shield_extra_ADD.tga` for six teams and `pulse_shield_test_ADD.tga`
for Feisar and Triakis. **`extrawreck.vex` is on the disc too** (19,536 to
22,432 bytes against `shipwreck.vex`'s 18,000 to 18,816), so by the same
registry value a Concept craft's wreck is not `shipwreck.vex`; this port does not
do that yet.

**Ours**: `oag_livery::entry::shield_entry_names` takes the player's hull stem
(`Options::hull_variant`) and names every slot's shell from it on the PSP
(`extra` -> `extrashield`, none or `Ship` -> `shipshield`); the PS2 build still
passes the literal `extra`. Pinned by
`race::tests::load::the_psp_shield_shell_follows_the_players_hull_stem_and_the_ps2_one_does_not`
and, on the disc, `crates/game/tests/psp_shield_model_ground_truth.rs`. One frame
of ours (Concept hull, `--give shield --variant extra`, tick 40 and 70) against
the same without the variant: the shell changes from the purple `grid`
lattice to the pale-blue `extra` texture, scrolling. **No frame of the original's
Concept shield was taken**, so its look is read, not compared.

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

`crates/raceplay/src/weapons.rs` said `Data\Ships\<Team>\<Team>shield.vex`.
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
Frames: a scratch directory, not kept, `-tt2`,
`shield-ours-tt480` (not committed).

What matches, at player size:

- A shell that **fades in and settles**, violet-magenta at first and
  **blue** by about fire+70, with the cap-and-bands look of the authored mesh,
  persisting to fire+240 at least (the pickup's `time` is not read here).
- The shell's size against the craft: about 1.6-1.7 x the craft's width on both
  sides (the original's looks larger only because its craft is about 1.4 x larger
  on screen: its profile flew `OPT_CLOSE` and ours `far`, resolved 2026-10-01 in
  [camera.md](camera.md#the-default-view-is-opt_close-measured-2026-10-01)).

What differs, named and not fixed:

1. *Onset.* **Resolved 2026-10-01, second pass: see "The onset, measured live" below.** In the original nothing is visible at fire+12, a faint outline at
   +20 and a clear shell at +28. Ours shows a faint shell at +12 and a clear one
   at +20: **about 6-8 frames earlier**. The recovered fade (`rgba += (target -
   rgba) * 0.15` per 60 Hz step, 90 % in 14 steps) cannot account for an
   original delay of 20+ frames, so something before `ShipShield_Activate`
   delays it in the original (the arm function's own timer, or the `active` flag
   being set late). Not read. This is the same size as the delay ours already
   carries from somewhere; which part of it is the original's is open.
2. *Banding and brightness.* **Resolved 2026-10-01, third pass: see "The shell's own GE state" below** (a linearly decoded texture and a texture that did not scroll). The original's concentric cyan bands are sharper
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

## 2026-10-01, third pass: the shell's own GE state, and why ours looked dim

Method: `scripts/psp-ge-dump.py`-style `gpu.record.dump` frames of a raised
shield on Talon's Junction (Time Trial, Venom, Assegai, craft stationary on the
grid, PPSSPP v1.20.4 with the **software renderer**, so EDRAM is the real
framebuffer), at three shield clocks, plus the displayed frame read out of EDRAM
at `0x04000000`/`0x04088000` and the same frame again after hiding the shell in
place (`obj+0x74 = 0` and bit 2 of both models' `+0x2c` cleared, about 0.7 s of
game time, restored afterwards). The difference of the two frames is the shell's
own contribution, camera and craft unchanged. Harness: a scratch directory, not kept
(`shield_cap.py`, `ge_prims.py`, `emu_shell.py`); raw frames stay under `data/`.

### What the shell's two draws set (read off the dumps, three clocks agree)

The shell is two strips - 109 vertices (107 triangles) and 50 (48), the two
`--draws` batches of `shipshield.vex` - drawn after the hull:

| GE state | Value | Meaning here |
| --- | --- | --- |
| `VTYPE` | `0x13d` | `u8` UV, `RGBA8888` colour, `s8` normal, `s16` position |
| lighting | on, all four lights off, `MATERIALUPDATE = 7`, material alpha `0xff` | the vertex colour **replaces** the material's ambient, diffuse and specular; no light adds anything, so the lit colour is `scene ambient x vertex colour` |
| `AMBIENTCOLOR` / `AMBIENTALPHA` (`0x5c`/`0x5d`) | `(0xfe, 0xfe, 0xfe)` and `0xdc`, `0xfe`, `0xe0` at the three dumps | **this is where `ShipShield_Update`'s `set_model_colour` lands**: rgb is `rgba.rgb`, alpha is `rgba.a x (0.75 + 0.25 sin t)`. The alphas match `sin` of the shield's own clock at all three (peak `0xfe` at `t = pi/2`; `0x83` at `t = 4.37`) |
| `TEXFUNC` | `0x100`: modulate, texture alpha used | colour and alpha both multiply the texel |
| blend | `0xa2`: `SRC_ALPHA` and fixed white (`FIXB = 0xffffff`), add | `src.rgb x src.a + dst` - `mesh_render::ADDITIVE_BLEND` exactly |
| alpha test | `GREATER 0`; colour test `NOTEQUAL 0` | |
| depth | test `GREATER` (the PSP's `Less`), write off | |
| stencil | **off** | so the shell does **not** stamp the glow mask (confirms the `0x1232` reading) |
| cull | **off** | both faces add |
| fog | on, `FOG1 = 1850`, `FOG2 = 1/1428` | inert: the shell is about 30 units from the eye |
| `TEXLEVEL` / `TEXLODSLOPE` | mode 2 (slope), bias `0x0a`; `1/256` | level 0 at this depth; ours picks level 0 too (`--texture-detail maximum` changes no pixel) |
| `TEXOFFSET` `u` | `0.4737`, `0.1031`, `0.6455` at clocks `135.18`, `136.79`, `138.30` | the authored track of `shipshield.vex` (frames 1 to 59, `0` to `251/256`, 59-frame loop), sampled at those clocks to within the dump's timing error |

**The candidate this thread carried is closed**: the colour does not reach the
draw through `mesh+0x6c` as some other term. It is the GE's scene ambient colour
with the model's own vertex colours as the material, which is *exactly* what
`Drawable::tint` already does (vertex colour x `ShipShield::colour`). Authored
vertex colours arrive untouched (`(255,255,255,255)` and `(27,0,209,0)` on
alternating rings - the `rgba 0.55, 0.50, 0.91, 0.50` average `--draws` prints).
Confidence **92** (three dumps, every field constant across them where the law
says it is, varying where it says it varies).

### The two defects, and what each was worth

1. **The texture was decoded linearly.** `pulse_shield_test_ADD.tga` carries
   flags `0xe5` - bit 0, "already swizzled" - and `vex::textures` read bit 0 on
   version 4 and below only. In the dump the shell's level-0 bytes in RAM are
   **byte-identical to the file's**, and so are levels 1 to 3; `TEXMODE` bit 0
   (swizzle) is set on every draw of the frame; and the unflagged hull textures'
   bytes in RAM are the file's bytes *reordered* (the loader swizzles them -
   `Texture_BindEmbeddedData`/`Texture_SwizzleForGe`, see
   [psp-texture.md](../../../formats/psp-texture.md)). So the GE reads the shell's
   texture swizzled and ours read it as noise. Read linearly it is a random
   speckle with two bright bars; read as the GE reads it, a structured band
   pattern. Fixed in `vex::textures` (every version, each level unswizzled at
   its own stride; `shield_texture_swizzle_ground_truth.rs`).
2. **The texture did not scroll.** `Drawable::write_anims` was never called for
   the shells, so the authored `u` track (a full texture width in 0.98 s) stood
   at offset 0. Fixed in `race/scene/frame.rs`
   (`shield_shell_scroll_ground_truth.rs`: 8,486 pixels move between two clocks
   with the write, 276 without).

### The comparison, pinned clock, matched frame

Same craft, circuit and tick on both sides; the clock pinned (`--anim-seconds`
set to the `g_ingame+0x40` read at the pause, shield `time` equal to the object's
`+0x78`); each side differenced against its own no-shell frame, over the shell's
box, per channel (correlation / least-squares scale `k`, original = `k` x ours):

| Frame | Before (linear texture, no scroll) | Texture fixed, still no scroll | Both fixes, clock as read | Both fixes, clock one frame (0.015 s) earlier |
| --- | --- | --- | --- | --- |
| shield clock 0.65 s | R .36 G .48 B .51 (k 1.2 / 1.2 / .92) | R .92 G .91 B .93 (k .90 / .86 / .88) | same | not needed |
| shield clock 1.75 s | R .22 G .24 B .37 | R .13 G .18 B .29 | R .62 G .60 B .72 | **R .92 G .90 B .91 (k .90 / .86 / .91)** |

The last column is the display lag: the frame in EDRAM was drawn one frame before
the state read at the pause, and the texture's scroll is fast enough (a width in
0.98 s) to show a one-frame shift. A frame-by-frame sweep of the offset gives one
peak (0.92) at -0.015 s and nothing above 0.75 elsewhere in +-0.06 s.

**"Ours is about a third as bright" was a phase artefact.** At a pinned clock
the shell's blue energy was within 10 % either way even before the fix
(blue, shell box: original 594k against ours 605k at 0.65 s, 666k against 677k at 1.75 s), and a
dump-driven rasteriser of the original's two draws with the *unswizzled*
texture reproduces the original's frame's blue sum to 0.1 % (668,952 against
668,372; 639,069 against 639,404), against 20 % off with the linear one. The
earlier measure compared different texture phases (a scroll the engine did not
have) with a noisy pixel-change metric. Confidence **90** that the settled
brightness and banding gap is these two defects.

**The per-level rule was checked on every flagged node**, version 4 and 6, across
`Data.wad`, `FEData.wad`, `BEData.wad` and `FE.wad` (88 nodes, 236 levels below the
base): each decoded level is compared against a 2x box-downsample of the decoded level
above it, and against the linear reading of the same bytes. The unswizzled level matches
at least as well on **236 of 236** (mean absolute RGB difference 3.5 against 8.1). The 13
version-4 nodes (Pulse's Zone shipwrecks, flags `0x61`) and Pure's 1,806 already took
the swizzled branch before 2026-10-01, with level 0 unswizzled as one block and no
authored levels (the renderer synthesises their chain, as `frame-audit.md` records for
Pure). **That is kept exactly**: `vex::textures` hands on authored levels only from
version 5, because handing Pure's on would switch it onto the slope level rule, which
nothing measured. A unit test pins both (`a_flagged_texture_is_unswizzled_and_only_version_six_keeps_its_levels`).

Base-level coherence of every flagged node, decoded reading against the linear one
(neighbour difference of palette luminance): Pulse USA and EU, 88 nodes, **69 smoother
decoded, 19 identical (32-wide or flat), 0 smoother linear**; Pure USA and EU, 1,806
nodes (the reading it has had since 2026-08-12), 1,543 smoother decoded, 248 identical,
**15 smoother linear** - all `col_banners*_ADD_GLOW`/`AAdc_BaseTexture`-style 128x32
or 64x64 textures (e.g. `col_banners2_ADD_GLOW` 22.9 against 7.0). Pure's reading is
unchanged here and those 15 are an observation for whoever audits Pure's textures, not
a finding. Nothing else reads this `.vex` texture block: PS2 `.vex` scenes carry no
texels and HD's are big-endian and skipped (`embedded` is false for both).

Residual, reported and not tuned toward: ours reads 10 to 14 % brighter in the
fit (k about .9), the hull occludes the shell in ours over the craft's own box,
and the emulator frame lags the pause by one frame. No term was adjusted.

Frames, dumps and the numbers: a scratch directory, not kept (`off2`, `off3`,
`ours6`; `report.md`).

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
