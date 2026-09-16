# The Cannon, the Quake and the LeachBeam: all three fire bodies read

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** the three fire handlers `plasma.md` (2026-09-02) named only from the
dispatch table and a bit map - "do not assume anything about what they do" -
are now read end to end, each down to its spawn, its per-tick behaviour and
(where one exists) its damage/slowdown application. One correction falls out
immediately: **the Cannon candidate `0x088537ac` this thread's own 2026-09-07
entry named is wrong.** It decompiles as `Ai_Construct`
(`0x088536bc`-`0x08853a7f`, already named in [pickups.md](../../../gameplay/pickups.md)
for the *Autopilot's* own reveal), not a weapon handler at all - a coincidence
of address, not a candidate. See [History](#history) for the arithmetic that
produced it and the one that replaces it.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x088577ac` | `Weapon_FireCannon` | 85 |
| `0x0883f424` | `Cannon_UpdateReload` | 85 |
| `0x088648ec` | `Cannon_Init` | 80 |
| `0x0886c600` | `Weapon_FireQuake` | 88 (was 82) |
| `0x08874b14` | `Quake_Init` | 82 |
| `0x0891d268` | `Quake_Update` | 85 |
| `0x0891c028` | `Quake_SampleSpan` | 76 |
| `0x0891c7e0` | `Quake_ProximityToCraft_q` | 60 |
| `0x0891c82c` | `Quake_SpanIntensityAt_q` | 55 |
| `0x08874a30` | `Quake_UpdateSpans` | 85 (new 2026-09-08) |
| `0x0891cab8` | `Quake_UpdateSpan` | 80 (new 2026-09-08) |
| `0x0891b714` | `Quake_ArmSpan` | 82 (new 2026-09-08) |
| `0x0891b7a4` | `Quake_RetireSpan` | 85 (new 2026-09-08) |
| `0x0891b7f4` | `Quake_PropagateForward` | 78 (new 2026-09-08) |
| `0x0891b940` | `Quake_PropagateBackward` | 78 (new 2026-09-08) |
| `0x0891ba98` | `Quake_Construct` | 85 (new 2026-09-08) |
| `0x0891bcc8` | `Quake_Destruct` | 82 (new 2026-09-08) |
| `0x0891bf98` | `Quake_StopAllSpans` | 85 (new 2026-09-08) |
| `0x08866658` | `Weapon_FireLeachBeam` | 88 (was 82) |
| `0x08873d3c` | `LeachBeam_InitLocked` | 82 |
| `0x08872da8` | `LeachBeam_InitUnlocked` | 80 |
| `0x08866b08` | `LeachBeam_UpdatePool` | 85 (was `FUN_08866b08` at 60-78, see [bad-memory-access-halt.md](bad-memory-access-halt.md)) |
| `0x08866804` | `LeachBeam_Drain` | 78 |
| `0x08872edc` | `LeachBeam_DrainRate` | 88 (new 2026-09-08) |
| `0x08872f18` | `LeachBeam_RepairRate` | 88 (new 2026-09-08) |
| `0x08873fa0` | `LeachBeam_Advance` | 85 (new 2026-09-08) |
| `0x08873020` | `LeachBeam_PulseStrength` | 82 (new 2026-09-08) |
| `0x088732f8` | `LeachBeam_MarkPulse` | 82 (new 2026-09-08) |
| `0x08873090` | `LeachBeam_MarkDisconnected` | 80 (new 2026-09-08) |
| `0x088730a4` | `LeachBeam_LingerExpired` | 80 (new 2026-09-08) |
| `0x08873068` | `LeachBeam_UnlockedExpired` | 78 (new 2026-09-08) |
| `0x0883f228` | `Ship_ApplyPendingWeaponRepair` | 85 (new 2026-09-08) |
| `0x08853a80` | `Ai_Update` | 85 (new 2026-09-08) |
| `0x0885077c` | `WeaponAi_Construct` | 85 (new 2026-09-08) |
| `0x08834b14` | `AiManager_Update` | 80 (new 2026-09-08) |
| `0x088651d8` | `Cannon_Construct` | 85 (new 2026-09-08) |
| `0x088573b0` | `CannonPool_Construct` | 88 (new 2026-09-08) |
| `0x08864b00` | `Cannon_LoadTextures` | 85 (new 2026-09-08) |
| `0x08b3bf80` | `g_cannon_bolt_texture` (data) | 85 (new 2026-09-08) |
| `0x08b3bf84` | `g_cannon_muzzle_flash_texture` (data) | 85 (new 2026-09-08) |
| `0x0886593c` | `Cannon_UpdateRound` | 82 (new 2026-09-09) |
| `0x088582b0` | `CannonPool_Update` | 88 (new 2026-09-09) |
| `0x088579a8` | `Cannon_TestCraftHit_q` | 60 (new 2026-09-09) |
| `0x08857f2c` | `Cannon_MarkCraftHit` | 85 (new 2026-09-09) |
| `0x08857e90` | `Cannon_ApplyCraftDamage` | 88 (new 2026-09-09) |
| `0x08864af4` | `Cannon_BaseSpeedKmh` | 92 (new 2026-09-09) |
| `0x0886545c` | `Cannon_DrawRound` | 88 (new 2026-09-09) |
| `0x08864cd0` | `Cannon_BuildBoltList` | 85 (new 2026-09-09) |
| `0x08864dc4` | `Cannon_BuildMuzzleFlashList` | 85 (new 2026-09-09) |

Read [weapon-fire.md](weapon-fire.md) first for the two traps every reading below
depends on: `entity+0x1b8` is the fire-request word, and every `jal` operand and
`func_0x000NNNNN` the decompiler prints in this binary is **image-base-relative**
(`real = 0x08804000 + operand`) - it is what produced this page's own correction.

## The Cannon: `0x088537ac` refuted, `0x088577ac` confirmed, and it does not use the bit system at all

### The address correction, from a clean decompile

`Weapons_DispatchFire` (`0x08861814`) was decompiled whole for this page. Its
bit-`0x4000` arm reads, verbatim:

```c
if ((uVar5 & 0x4000) != 0) {
    func_0x000537ac(*(undefined4 *)(param_2 + 0x58),iVar9,uVar6);
    uVar5 = *(uint *)(iVar9 + 0x1b8);
}
```

`0x08804000 + 0x000537ac = 0x088577ac`, **not** `0x088537ac` - a `0x4000`
arithmetic slip in `plasma.md`'s sixteen-bit table, the same magnitude as the
bit being read, which is presumably how it happened. `0x088537ac` decompiles
as a real function, but the wrong one: it falls inside `Ai_Construct`
(`get_function_by_address` gives `Entry: 088536bc, Body: 088536bc - 08853a7f`),
which builds an AI driver record and has nothing to do with weapons.
`0x088577ac` decompiles as a real weapon handler with the shape described below.
**Confidence 90** on the address itself - a direct decompile and a checked
addition, cross-checked against `weapon-fire.md`'s own independent (and
correct) citation of `FUN_088577ac` for this same bit, predating `plasma.md`'s
slip.

### `Weapon_FireCannon` (`0x088577ac`) is a round-robin burst spawn, not a single shot

```c
void Weapon_FireCannon(CannonPool *pool, Craft *craft, int craft_index) {
    craft->fire_flags &= ~0x4000;               // craft + 0x1b8, cleared unconditionally
    uint live = pool->live;                      // pool + 0x158
    pool->flags |= 2;
    if (live < 0x3c) {                           // cap 60
        Round *r = pool->slot[live];             // pool + 0x68 + live*4
        bool left = (craft->shots & 1) != 0;     // craft + 0x154, low bit
        Entity *emitter = left ? craft->entity->barrel[1]   // craft->entity + 0x68
                               : craft->entity->barrel[0];  // craft->entity + 0x64
        pool->live = live + 1;
        float speed_kmh = craft->entity->body->speed * 3.6;   // entity+0x794+0x398, |velocity|
        r->flags = 0; r->flags = 1;
        r->owner = craft_index;                  // + 0x40
        r->id    = ++g_next_projectile_id;        // + 0x44
        Cannon_Init(speed_kmh, r, emitter, craft->entity->stats_a /* +0x60 */,
                    !left, craft->entity->stats_b /* +0x50 */);
    }
}
```

**Two muzzles, alternating.** `craft+0x154`'s low bit picks between two emitter
anchors on the craft's own entity (`+0x64`/`+0x68`) - a twin-barrel cannon,
consistent with HD/Fury's own `CannonManager` (`ps3-hdfury-eu/weapons.md`)
being a real class in this lineage, though that binary settles nothing about
this one's mechanics.

**Every round inherits the firing craft's own current speed**, read off its
rigid body (`entity+0x794+0x398`, the same `|velocity|` field `contact-response.md`
already reads for the per-frame contact loop) and converted to km/h - the same
"carry the shooter's own speed" shape the Shuriken's throw already has.

**The pool cap (60) is a live-count guard, not a magazine size** - same shape as
`Weapon_FirePlasma`'s `pool->live < 0x10`. `Cannon_Init` (`0x088648ec`) is the
actual constructor: it copies the emitter matrix as the round's pose, computes
`speed = speed_kmh_arg + Cannon_BaseSpeedKmh()` - a flat `500.0`, see "`func_0x00060af4`
decompiled" below, not per class - builds velocity as `forward * speed`, and
copies the *firing craft's own* cached track-locator record
(`entity+0xad8..+0xae4`, the same fields
[Quake_Init](#quake_init-0x08874b14-locates-the-firing-craft-on-the-track) reads)
onto the round before playing a fire cue. Confidence **80** - direct decompile.

### What actually fires it: the fire button, held, driving a reload countdown

> **RESOLVED 2026-09-07, and the banner that stood here is gone with it.** The
> maintainer's play report - the Cannon shoots **either by holding the fire
> button or by tapping it repeatedly** - is correct, and the hypothesis raised
> against it is confirmed. `ship+0x94+0x78` is the craft's **live control
> record**, `+0x16` is the **fire button held** boolean, and the countdown
> below advances only on frames where it is set. Every other fact on this page
> survives untouched: bit `0x2000` really is dispatched by nothing, and the
> Cannon really does not route through the fire-request word. See
> [The gate is the fire button](#the-gate-is-the-fire-button-held-not-a-track-pad-flag)
> for the read.

**Bit `0x2000` (the Cannon's own request bit per `Weapon_RequestFire`'s
id-to-bit map) is dispatched by nothing, and that is expected, not broken.**
The Cannon does not fire through the fire-request-word system the other twelve
weapons share at all. `Cannon_UpdateReload` (`0x0883f424`) does:

```c
void Cannon_UpdateReload(float dt, Entity *ship) {
    if (ship->craft->controls->fire_held == 0) return;   // *(*(ship+0x94)+0x78) + 0x16
    if (ShipState(ship) != 1) return;                   // must be racing
    Craft *craft = ship->craft;                          // ship + 0x4c
    if (craft->held != 3) return;                        // craft + 0x1bc - held-weapon id
    craft->reload -= dt;                                  // craft + 0x158
    if (craft->reload < 0.0) {
        craft->reload += ActiveCannonStats()->rate;       // + 0x78
        craft->shots  -= 1;                                // craft + 0x154
        craft->fire_flags |= 0x4000;                       // arms Weapon_FireCannon
    }
    if (craft->shots == 0) {
        <ammo bookkeeping>
        craft->held = -1;
    }
}
```

Called every frame per craft from `FUN_0883f540`, itself part of the main
per-craft update chain (it also drives the Missile/LeachBeam lock scan and a
craft's own scene-node cache refresh), gated on the fire button being **held**
and on the craft's *held weapon id being 3* - not on any bit of `craft+0x1b8`.
So the sequence is: the player presses fire, `Weapon_RequestFire` sets bit
`0x2000` (which nothing reads) and leaves `craft+0x1bc == 3` in place; and
separately, for every frame the button stays down, `Cannon_UpdateReload`'s own
countdown - refilled from the Cannon's own authored `rate` - periodically arms
bit `0x4000`, which *is* dispatched, to `Weapon_FireCannon` above. Holding the
button therefore gives auto-repeat at the authored rate; tapping it gives a few
frames of countdown per tap, which is the slower way to the same thing. That is
exactly the behaviour reported from play. **Three independent facts tie id 3 to
the Cannon**: this function is gated on it and reads/writes exactly the
`rounds` (`craft+0x154`) and `rate` (`ActiveCannonStats+0x78`) attributes
`mine.md` and `weapon-fire.md` already identified as authored by the Cannon's
`<Stats>` alone; `craft+0x154`'s low bit is *also* what
`Weapon_FireCannon` reads to alternate barrels, a cross-check from the spawn
side; and it is the only weapon whose fire mechanism does not route through
`craft+0x1b8` at all, matching "the Cannon's own request bit is dispatched by
nothing" being true and *irrelevant* rather than a sign of missing work.

**A trap for whoever builds this**: `Weapon_FireCannon`'s `pool->live` lives at
`world+0x58 + 0x158` (the *subsystem*), while `Cannon_UpdateReload`'s reload
timer lives at `craft + 0x158` (the *per-craft weapon record*) - same offset,
different base, no relation. Worth naming next to the `craft+0x1ac`/`+0x1ac`
trap the handover thread already records for the Mine.

**Buildable now**: the whole fire-rate-and-burst mechanism is read at
instruction level. The base speed (`func_0x00060af4`) and what a round does
on hitting a craft or wall are both read too, as of 2026-09-09 - see
"`Cannon_UpdateRound` read" below.

### The gate is the fire button, held, not a track pad flag

**Read 2026-09-07 with the Ghidra bridge on `psp-pulse-usa`. Confidence 90.**
This section replaces the "self-firing" conclusion an earlier revision drew
from reading `ship+0x94+0x78` as a *track weapon-pad* flag. It was the input
pad all along, and the maintainer's play report is what sent anyone looking.

At instruction level, `0x0883f424`-`0x0883f448`:

```
lw   a1, 0x94(a0)      ; a1 = *(entity+0x94)   -> the craft object
lw   a0, 0x78(a1)      ; a0 = *(craft+0x78)    -> the craft's live control record
lbu  a0, 0x16(a0)      ; a0 = record[0x16]
beq  a0, zero, ...     ; the whole body is skipped when it is zero
```

**The record's base is named by the binary itself.** `PlayerInput_Construct`
(`0x0883c74c`) ends with

```c
FUN_0893c804(&DAT_08b317b8, s_player_input_08a7b684, this + 0x44, tag);
```

- it registers **`this + 0x44`** in the named-resource registry `DAT_08b317b8`
  under the string literal **`player_input`** at `0x08a7b684`, whose only two
  references in the image are this one `lui`/`addiu` pair. So the record base is
  `controller + 0x44`, and its name is the original author's, not ours.

**`PlayerInput_Update` (`0x0883c870`) fills it from `g_input`.** It has no
direct callers - it is reached through the vtable slot at `0x08aca094` - and it
is the class's per-frame update:

| Record | Controller | Written from |
| --- | --- | --- |
| `+0x00` | `+0x44` | analog X, `0.25` deadzone, scaled to +/-100 |
| `+0x04` | `+0x48` | `Input_IsHeld(Options_ButtonForAction(0 = OPT_CTRL_ACC))`, as `0`/`100` |
| `+0x08` | `+0x4c` | left airbrake (`OPT_CTRL_LAB`, or the novice combo) |
| `+0x0c` | `+0x50` | right airbrake (`OPT_CTRL_RAB`) |
| `+0x10` | `+0x54` | analog Y, same deadzone and gain |
| `+0x15` | `+0x59` | `Input_IsPressed(Options_ButtonForAction(1 = OPT_CTRL_FIRE))` - **fire, rising edge** |
| `+0x16` | `+0x5a` | `Input_IsHeld(Options_ButtonForAction(1 = OPT_CTRL_FIRE))` - **fire, held** |
| `+0x17` | `+0x5b` | `Input_IsPressed(Options_ButtonForAction(2 = OPT_CTRL_ABS))` - absorb |
| `+0x18` | `+0x5c` | `Input_IsHeld(Options_ButtonForAction(3 = OPT_CTRL_LBACK))` - look back |
| `+0x20` | `+0x64`/`+0x66` | `g_input+0x48` pressed and `g_input+0x44` released masks, as `u16` |
| `+0x24` | `+0x68`/`+0x6a` | `g_input+0x3c` held and `g_input+0x40` held-last masks, as `u16` |

The action numbering is [`input-bindings.md`](input-bindings.md)'s, read there
off the options page's own localisation keys; `Input_IsHeld` (`0x0894f158`)
tests `g_input+0x3c` and `Input_GetAxis` (`0x0894f198`) returns
`g_input+0xec`/`+0xf0`, both against the mask layout [`input.md`](input.md)
already documents.

**Six independent legs**, which is why this is 90 and not a hypothesis:

1. **The literal name.** `player_input` is registered at `controller+0x44`,
   which fixes the record base beyond argument.
2. **`Ship_UpdateCraft` (`0x08849618`) copies exactly `0x28` bytes** out of
   `*(craft+0x3c)` into `craft+0x44..+0x6c` when the autopilot blend is live -
   `0x44 + 0x28 = 0x6c` is precisely the span `PlayerInput_Update` writes and
   not one byte more.
3. **`craft+0x78` *is* that record.** The same function's first statement is
   `*(craft+0x78) = *(craft+0x3c)`, re-pointed at the blend buffer only when
   `craft+0x1d4 != 0`.
4. **A sibling consumer on the identical chain reads `+0x15`.**
   `Ship_FireHeldWeapon` (`0x08844ae8`) gates on
   `*(*(entity+0x94)+0x78) + 0x15` and *that* is what calls
   `Weapon_RequestFire`. `+0x15` and `+0x16` are the press and the hold of one
   button, read by two functions called back to back out of `FUN_0883f540`.
5. **[`camera.md`](camera.md)'s independently recovered steer.** That page reads
   the *first float* of the same `*(*(craft+0x94)+0x78)` as `steer * 0.01`;
   record `+0x00` is the +/-100 analog value, so the product is +/-1.0. Its
   standing "confidence 0 on what this field means" is closed by this.
6. **`input-bindings.md`'s independently recovered masks.** That page reads
   `*(ship+0x78) + 0x20`/`+0x24` as button masks indexed by
   `Options_ButtonForAction`; they are `controller+0x64` and `+0x68`, copied
   straight off `g_input`.

### `rate` is authored per second and stored as its reciprocal

**Read 2026-09-07, and it is the second half of why the weapon read as
unfireable.** `WeaponStats_ParseCannon` (`0x0880c774`) is five attribute arms,
and the `rate` one is not a plain store:

```c
rounds            -> *(int   *)(stats + 0x70) = (int)value;
absorb            -> *(float *)(stats + 0x74) = value;
rate              -> *(float *)(stats + 0x78) = 1.0 / value;   // <- the reciprocal
damage_per_bullet -> *(float *)(stats + 0x7c) = value;
slowdown_time     -> *(float *)(stats + 0x80) = value;
```

**The `rate` arm matches an unlabelled `DAT_08a78ae4`**, where the other four
match named string symbols, so it is read off the bytes rather than by
elimination: `read_memory 0x08a78ad8 48` gives
`75 73 00 00 | 72 6F 75 6E 64 73 00 00 | 72 61 74 65 00 00 00 00 | 64 61 6D 61 67 65 5F 70 65 72 5F 62 75 6C 6C 65 74 00`
- `"rounds"` at `0x08a78adc`, **`"rate"` at `0x08a78ae4`**, `"damage_per_bullet"`
at `0x08a78aec`, each 4-byte aligned.

`stats + 0x78` is exactly the field `Cannon_UpdateReload` adds to `craft+0x158`
above, so the file's `rate="20"` is **twenty rounds per second** and the
countdown reloads with `0.05` seconds. Both shipped tables author `rate="20"`
beside `rounds="30"`: a second and a half of continuous fire, not the one round
every twenty seconds a literal reading gives. That literal reading is what
`oag_tables::weapons::CannonStats::rate` carried until this pass, and it
shipped - a player holding fire for a whole race would have seen about twenty
rounds leave, which is indistinguishable from a broken weapon.

This also settles the offsets `weapon-stats.md` recorded as unchecked, and it
is the one place on this page where the *parser* rather than the *handler* was
the missing read.

### The AI's fire-held byte is never written by anything, and its object is not zero-filled

**Read 2026-09-08, and it replaces a section that said "only a human can fire a
Cannon, and that is measured".** The maintainer reports from play that **AI
craft do fire the Cannon, at the player and at each other**, so that conclusion
was wrong. Its evidence was not wrong; the *search* was blind by construction,
and this section is what the corrected search found.

`FUN_0883f540` wraps the whole per-craft weapon block in a null check on the
record, and AI craft do pass it:

```c
if (*(int *)(*(int *)(entity + 0x94) + 0x78) != 0) {
    Ship_FireHeldWeapon(entity);
    Cannon_UpdateReload(dt, entity);
    FUN_08844ec4(entity);
}
```

#### Why `field 0x16` could never have found a producer

`scripts/psp-relocate.py field 0x16` searches for accesses at **struct offset
`0x16`**. No owner of a control record ever addresses it that way, because every
owner *embeds* the record at its own offset and writes through that:

| Owner | Record embedded at | "fire held" byte is at |
| --- | --- | --- |
| `PlayerInput` (the human pad) | `controller + 0x44` | `controller + 0x5a` |
| `Ai` (an opponent, or the player's autopilot) | **`Ai + 0x08`** | **`Ai + 0x1e`** |

So "one byte read at `+0x16`, zero byte stores" was never evidence about
producers at all - only the *consumer* `Cannon_UpdateReload` holds a bare
record pointer. The `controller+0x44` half was already on this page (see
[The gate is the fire button](#the-gate-is-the-fire-button-held-not-a-track-pad-flag));
the `Ai + 0x08` half is new and is what the previous pass was missing.

#### The AI's control record is `Ai + 0x08` (confidence 90)

Four independent legs:

1. **The binary names it.** `Ai_Construct` (`0x088536bc`) registers
   `param_1 + 2` - that is `Ai + 0x08` - in the same named-resource registry
   `DAT_08b317b8` that `PlayerInput_Construct` uses, under either the formatted
   literal `"AI input %d"` (`0x08a7bf08`) or, when the ship is the human
   player's, the stack-built literal `"autopilot input"`
   (`local_50 = 0x6f747561 'auto'`, `0x6f6c6970 'pilo'`, `0x6e692074 't in'`,
   `0x747570 'put'`). Which of the two is chosen by
   `*(bool *)(Ai + 0xa8) = (entity + 0x368 == 0)`.
2. **`Ai_Update` (`0x08853a80`) writes the record's own fields, at
   `PlayerInput_Update`'s exact layout**: `param_2[2]` = steer (`Ai+0x08` =
   record `+0x00`), `param_2[3]` = throttle (`+0x04`), `param_2[4]`/`param_2[5]`
   = left/right airbrake (`+0x08`/`+0x0c`), `param_2[6] = 0` = pitch (`+0x10`).
   It is a per-frame update, not a one-shot: `AiManager_Update` (`0x08834b14`)
   loops the whole `Ai` list (`manager+0x44`, count `manager+0x40`) calling it,
   then loops the `WeaponAi` list (`manager+0x78`, count `manager+0x74`) through
   their vtable.
3. **`Ai_Construct` seeds `param_1[3] = 0x42c80000`** - `100.0f`, record `+0x04`,
   on the same `+/-100` scale `PlayerInput_Update` writes the accelerator at. An
   AI craft is constructed with the throttle already down.
4. **The weapon half resolves the same name.** `FUN_0883e820` (`0x0883e820`)
   formats `"AI input %d"` (`0x08a7b944`) and passes it through `FUN_08834d58`
   to `WeaponAi_Construct` (`0x0885077c`), whose `FUN_08850494` looks it up into
   `WeaponAi + 0x10` - the pointer `WeaponAi_Update` and
   `WeaponAi_DecideFireOrAbsorb` write `+0x15`/`+0x17` through. Matching that,
   `Ai_Construct` byte-zeroes `Ai + 0x1d` (= record `+0x15`) and `Ai + 0x1f`
   (= record `+0x17`) and nothing else in that span.

#### Nothing writes `Ai + 0x1e`, at any width (confidence 85)

- `field 0x1e` over the game range (`< 0x08a00000`) finds six `sb` sites and
  none is an `Ai`: four are the network object `FUN_08962088` returns
  (`0x0883ce50` in `FUN_0883ce3c`, `0x0886127c` and `0x08861294` in
  `FUN_08861228`, `0x088617a4` in `FUN_088616cc`), one is in the front end
  (`0x088243d8`, `FUN_08824268`) and one is `Weapon_FireQuake`'s own instance
  flag (`0x0886c738`). No site in the `Ai` module (`0x08853xxx`) or the
  `WeaponAi` module (`0x08850xxx`-`0x08852xxx`).
- `field 0x1c` finds **no `sw` in either module**. That is the wide-store trap
  this page itself warned about: a single `sw ?,0x1c(Ai)` would have written
  record `+0x14`..`+0x17` at once. There isn't one. `field 0x1d` and
  `field 0x1f` show only `Ai_Construct`'s two zeroing stores.
- `Ai_Update` **clears record `+0x14`** (`Ai + 0x1c`) on every frame it races,
  and still never touches `+0x16`.
- The only 40-byte record copy in the image is `Ship_UpdateCraft`'s autopilot
  blend (`sw` at record `+0x14`/`+0x18`/`+0x1c`, `0x088496f8` onward) and its
  *source* is another `Ai` record with the same hole.

#### The `Ai` object is allocated without a zero-fill

`FUN_08834c5c` does `FUN_08946ce4(0x890)` - which is
`FUN_08946e20` - `FUN_08946e40(0x890, 0, 0)`, a plain heap allocation - and
calls `Ai_Construct` straight after. Thirty lines away in `Craft_Construct_q`
(`0x08840c74`) the craft object is allocated the same way and then **explicitly
memset**: `FUN_08946e40(0x360, ...)` followed immediately by
`FUN_08946db8(ptr, 0, 0x360)`. This allocator does not zero; callers that want
zero ask for it, and the `Ai` allocation does not.

#### What that means

**The AI's fire-held byte is uninitialised heap.** Where it comes up non-zero,
`Cannon_UpdateReload`'s countdown advances on *every* frame that AI holds a
Cannon, so the craft empties its `rounds="30"` at the authored `rate="20"` per
second as soon as it picks the weapon up - about a second and a half of
continuous fire, with **no fire decision involved at all**. The WeaponAi's own
decision writes `+0x15`, which reaches `Weapon_RequestFire`, which sets bit
`0x2000`, which nothing reads.

Two supporting facts make this the only surviving reading rather than a
convenient one:

- **`Weapon_FireCannon` has exactly one caller** - `Weapons_DispatchFire`
  (`0x08861814`), gated on bit `0x4000` of `craft+0x1b8`. (`get_function_callers`
  became usable on this program with the 2026-09-07 PSP relocation patch; the
  previous pass predates it.)
- **Bit `0x4000` of `craft+0x1b8` has exactly one producer image-wide** -
  `0x0883f4d4 ori r5,r5,0x4000` / `0x0883f4d8 sw r5,0x1b8(r4)`, inside
  `Cannon_UpdateReload`. That is from a scan correlating *every* `sw`/`sh` at
  `+0x1b8` in the game range with its value producer, covering `ori`/`andi` with
  any immediate carrying bit `0x4000` and R-type `or`/`and`, not just the exact
  immediate `0x4000`. Every other `+0x1b8` store is another weapon's `ori` or an
  `and` that clears. Correspondingly `masked 0x1b8 0x2000` returns **zero**
  masked reads, so the Cannon's own request bit really is dispatched by nothing.

So the route to an AI cannon round is forced through that byte, and the byte has
no writer. A race's allocation sequence is deterministic, so whatever the block
holds is reproducible run to run and per grid slot - which is what reads to a
player as systematic AI cannon fire.

**This makes a prediction worth checking from play**: AI cannon fire should look
*automatic and continuous on pickup*, not aimed or intermittent. If it is aimed
and intermittent, this reading is wrong and something else sets that byte.

**Implementation status.** `oag_game::race::weapons` fires AI cannons on the
recovered gate above, but the *value* of the byte is undefined behaviour and so
cannot be measured: treating it as non-zero for every AI craft is **chosen, not
measured, and carries no confidence score**. The mechanism is recovered; the
value it terminates in is not a fact about the disc.

### What draws a Cannon round: two textures and three quads, and not a particle effect at all

**Read 2026-09-08, confidence 85, and it refutes the premise the visuals thread
was working from.** The round was assumed to be a `Data\Psys\*.POB` effect whose
trigger had not been found, with `Ship Muzzle` (`0x3e2`) and `cannon_flash`
(`0x3eb`) as the two candidate classes. Neither is it. The Cannon names its own
assets in the executable, in plain strings:

| String | Address | What loads it |
| --- | --- | --- |
| `Data\Weapons\pulse_muzzleflash.vex` | `0x08a7c85c` | `Cannon_Construct` (`0x088651d8`), via `Vex_LoadModel` into the round's own node at `instance+0xc0` |
| `Data\Weapons\Textures\Cannon_bolt.mip` | `0x08a7c804` | `Cannon_LoadTextures` (`0x08864b00`), into `g_cannon_bolt_texture` (`0x08b3bf80`) |
| `Data\Weapons\Textures\Cannon_muzzle_flash.mip` | `0x08a7c82c` | `Cannon_LoadTextures`, into `g_cannon_muzzle_flash_texture` (`0x08b3bf84`) |
| `Data\Psys\WO_CANNON_SPARKS.POB` | `0x08a7c484` | referenced from `FUN_0886593c` (`0x0886593c`) - the **impact**, not the round |
| `CANNON` | `0x08a7c7f0` | the fire cue, played at the end of `Cannon_Init` |

`Cannon_Construct` (`0x088651d8`) is the per-round constructor: 60 of them are
allocated by `CannonPool_Construct` (`0x088573b0`) at `pool + 0x68 + i*4` -
exactly the slot array
[`Weapon_FireCannon`](#weapon_firecannon-0x088577ac-is-a-round-robin-burst-spawn-not-a-single-shot)
indexes - and its `pool + 0x158` live count is zeroed in the same function, which
is the second cross-check on both offsets.

Each round then builds **two GU display lists in its own constructor**, under a
semaphore, and replays them per frame:

```c
Gu_Start(1, instance + 0x240, 0x200);  FUN_08864cd0(instance);   // two 4-vertex strips
Gu_Start(1, instance + 0x440, 0x1c0);  FUN_08864dc4(instance);   // one 4-vertex strip
```

Both set the same state - fog off, texture transform reset, depth write on with
func 6, `Gu_BlendFunc(0, 2, 10, 0, 0xffffff)` - and both draw primitive `4`
(triangle strip) with vertex format `0x19f`. `FUN_08864cd0` draws from
`instance+0x100` and `instance+0x160`; `FUN_08864dc4` draws from
`instance+0x1c0`. Those three vertex blocks are the `0x3f800000` runs
`Cannon_Construct` seeds at `+0x104`/`+0x118`/`+0x11c`, `+0x164`/`+0x178`/`+0x17c`
and `+0x1c4`/`+0x1d8`/`+0x1dc`.

**Which list is the bolt and which is the muzzle flash is not read**, so neither
function is renamed. The ordering (`Cannon_LoadTextures` loads bolt then flash;
the constructor builds `FUN_08864cd0`'s list then `FUN_08864dc4`'s) makes
"`0x08864cd0` is the bolt, `0x08864dc4` is the flash" the obvious guess and it is
written down here as a guess - **confidence 40, do not rename on it**. Neither
draw function binds a texture in the decompile; both call
`FUN_0891e988(g_display, 0xfdb2, 0x3e9, 0)` with the same two constants
`Vex_LoadModel` is passed, so the binding is somewhere those constants reach.
`0x3e9` is **not** a vex class id here - `0x3e9` in the class table is
`soundcone` (see [vex.md](../../../formats/vex.md)), so the pair reads as a tag,
not a class.

#### All three resolve in `Data.wad`, and the directory layout is the trap

Confirmed 2026-09-08 by hashing each executable-side name and matching it
against `PSP_GAME/USRDIR/Data.wad`'s 1142 hash-keyed entries - the archive
carries no recovered names for any of them, so a name is only "present" once its
hash lands on an entry. All three hit on **both** PSP pressings, one entry each:

| Asset | Path | Name hash | Entry | Bytes |
| --- | --- | --- | --- | --- |
| Round/muzzle-flash mesh | `Data\Weapons\pulse_muzzleflash.vex` | `b94a2a6c` | 1048 | 6,608 |
| Bolt texture | `Data\Weapons\Textures\Cannon_bolt.mip` | `ee06a033` | 1057 | 2,064 |
| Flash texture | `Data\Weapons\Textures\Cannon_muzzle_flash.mip` | `1762ad77` | 1058 | 5,136 |

**The layout is worth keeping, because the obvious guess is wrong.** The mesh is
in `Data\Weapons\`, but the textures are one level deeper in
`Data\Weapons\Textures\`. `Data\Weapons\Cannon_bolt.mip` and
`Data\Effects\Cannon_bolt.mip` both hash to entries that do not exist. The same
split very likely holds for the other weapons' assets.

Two corroborations: the two texture entries are **adjacent** (1057, 1058), which
is what a pair loaded back to back by `Cannon_LoadTextures` should look like -
and their order in the archive is an independent cross-check on which global is
which, should anyone read the two display lists; and the mesh sits well away from
them at 1048, matching its separate load path through `Vex_LoadModel`.

To re-run or extend the method - single backslashes either way, through `cargo
run` directly or through `just wad`/`just unpack` (both now hand `*ARGS`
through unmangled; see `justfile`'s own `positional-arguments` comment):

```sh
cargo run -q -p oag-tools --bin oag-wad -- hash 'Data\Weapons\Textures\Cannon_bolt.mip'
cargo run -q -p oag-tools --bin oag-wad -- list 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad'
```

#### What this project draws, and what it deliberately does not

`oag_game::race::CANNON_MODEL_ENTRY` is the mesh above, loaded the same way the
Rocket's, Mine's and Bomb's are, and `Race::cannon_model_matrices` puts one
matrix on each live round. **The mesh is named `muzzleflash` and is the bolt**:
`the_discs_cannon_round_carries_its_own_model` measures it at 20 vertices, 18
triangles, spanning `1.200 x 1.200 x 3.599` - a dart, longest along the +Z the
matrix aims down the velocity, which is not the shape of a flash at a barrel.

**The two display lists are still not drawn** - see "Which display list is the
bolt and which the flash: settled" below for why the *identification* is no
longer the open question - because drawing them for real needs a
textured-billboard path this pass's own file ownership does not extend to.
That is still an honest absence per CLAUDE.md, and it is bounded: the round
itself is visible, and its impact now throws a spark.

### 2026-09-09: `Cannon_UpdateRound` read - the hit path, `WO_CANNON_SPARKS`'s real trigger, and the base speed corrected

**Read with the Ghidra bridge on both `psp-pulse-usa` and `psp-pulse-eu`,
cross-checked with `diff_functions` at every step - every USA/EU pair below
came back exact-body-equal (0 added, 0 removed instructions) except for
relocated immediate operands, the same signature
[`exact-hash-transfer.md`](../psp-pulse-eu/exact-hash-transfer.md) already
established as this project's strongest match category.** This closes the
thread's three remaining "Open" items from 2026-09-08.

#### `Cannon_UpdateRound` (`0x0886593c`, EU `0x08865798`) is the per-round per-tick update, and it is what calls `Psys_Spawn_q`

Confidence 82 (571 instructions, VFPU-dense; the world-collision branch and
the spark spawn are read at instruction level, the "else" reflect/bounce
branch only at the shape of its arithmetic - a `v' = v - 2(v·n)n` mirror
about the surface normal, which is why this stops short of the 85+ this
page's smaller reads carry). Called once per live round from
`CannonPool_Update` (`0x088582b0`, EU `0x0885813c`, confidence 88): it
raycasts the round from its previous position to its new one against the
track's own collision mesh through `FUN_0883198c` - the same query
`Rocket_Update`, `Missile_Update`, `Plasma_Update`, `Shuriken_Update` and
`Camera_UpdatePlayerView` all call, confirmed by `get_function_xrefs` on that
address - and branches on the returned hit-type code:

- **`0x7f`**: no hit. The round keeps flying.
- **`0` or `4`**: a world/track hit. The round's own `+0x3c` flags gain
  `0x14` (general-hit `0x4`, wall `0x10`), and `Psys_Spawn_q` is called with
  the string at `0x08a7c890` (USA) / `0x08a7c0e0` (EU) - `WO_CANNON_SPARKS`,
  confirmed by direct string read on both pressings - and a basis built from
  the hit normal (a `vcrsp_t` cross product chain identical in shape to the
  camera-facing basis builds elsewhere on this page). **This is the recovered
  trigger**: a Cannon round throws `WO_CANNON_SPARKS` exactly when it hits
  track geometry, oriented to what it hit, at the point it hit it.
- **anything else**: a glancing hit. The round's velocity is reflected about
  the surface normal and its position is nudged off the surface by `3.0` units
  along it - a bounce, not a stop, and no effect is spawned for it.

`+0xc8` is a second, independent timer this same function advances by `dt`
every tick (distinct from `+0x48`, which `Cannon_DrawRound` reads for the
round's own two textured quads - see below) - not consumed here, only
carried forward.

#### The craft hit is a wholly separate path, and it never calls `Psys_Spawn_q` at all

**This is the missing half `weapon-fire.md`'s 2026-09-07 entry flagged as
"no `Cannon_HitCraft`-style function was located this pass" - it exists, it
is three functions, and none of them touch the particle system.**
`CannonPool_Update` runs a second per-round pass after `Cannon_UpdateRound`:
`Cannon_TestCraftHit_q` (`0x088579a8`, EU `0x08857834`, confidence 60 - a
314-instruction VFPU cylinder-distance sweep against every live craft, read
for its shape and its `6.0`-unit threshold but not down to every register)
finds the round within range of a craft's own capsule and calls
`Cannon_MarkCraftHit` (`0x08857f2c`, EU `0x08857db8`, confidence 85), which
ORs the round's `+0x3c` flags with `0x24` - general-hit `0x4` again, plus
`0x20`, the **ship** bit, distinct from the wall's `0x10` above - and, only
if the round had not already recorded a hit this tick, calls
`Cannon_ApplyCraftDamage` (`0x08857e90`, EU `0x08857d1c`, confidence 88).
That function is three stores: the struck craft's own damage accumulator
(`+0x120`) gains `ActiveCannonStats()->damage_per_bullet` (`stats+0x7c`, per
`WeaponStats_ParseCannon`'s own offsets above), a second accumulator
(`+0x130`) gains `stats+0x80` (`slowdown_time`), and `+0x138`/`+0x13c` record
a hit-source tag (`3`) and the attacker's own craft index. **Neither this nor
either of the two functions that reach it calls `Psys_Spawn_q`.** So on the
PSP, a craft hit applies damage and (through `CannonPool_Update`'s own
despawn pass, which reads the `0x10`/`0x20` split to choose `CANNONEXPLSHIP`
over `CANNONEXPLWALL`) plays a sound, but throws no spark at all - only a
wall hit does.

This is exactly what `oag_gameplay::projectile::cannon::direct_hit` already
did before this pass read the handler: apply `damage_per_bullet` and
`slowdown_time` unconditionally on a craft hit. The handler confirms the
shape rather than changing it.

#### Which display list is the bolt and which the flash: settled, confidence 88

**Not from the archive's entry-adjacency cross-check this page proposed on
2026-09-08 and never spent - from something stronger.** `Cannon_DrawRound`
(`0x0886545c`, EU `0x088652b8`, confidence 88) is the per-round per-frame
draw call `Cannon_Construct`'s two list-builders were always going to be
*replayed* from, and it binds a texture immediately before each
`Gu_CallList`:

```c
Gfx_BindTexture(DAT_08b3bf80);              // g_cannon_bolt_texture
... rebuilds the two quads at +0x100/+0x160 every frame ...
Gu_CallList(param_1 + 0x240);               // Cannon_BuildBoltList's own list

if (*(float *)(param_1 + 200) < 0.1) {      // +0xc8: age since spawn, seconds
    Gfx_BindTexture(DAT_08b3bf84);           // g_cannon_muzzle_flash_texture
    ... rebuilds the one quad at +0x1c0, random size and RGB each time ...
    Gu_CallList(param_1 + 0x440);            // Cannon_BuildMuzzleFlashList's own list
}
```

So **`Cannon_BuildBoltList` (`0x08864cd0`, EU `0x08864b2c`) is the bolt** -
drawn every frame a round is alive, as a streak between its previous and
current position - and **`Cannon_BuildMuzzleFlashList` (`0x08864dc4`, EU
`0x08864c20`) is the muzzle flash** - drawn only for the round's first tenth
of a second, sized and coloured at random each time
(`Psys_RandIntRange(0x96,0xff)` per channel). This confirms the "obvious
guess" the 2026-09-08 entry logged at confidence 40 and refused to lean on;
it does not overturn it. Both list-builder functions themselves (`0x08864cd0`/
`0x08864dc4`, 61 and 55 instructions) are unchanged from that entry's read -
`Gu` state setup and a `Vex`-style texture tag, no literal texture bind
inside either - the binding this section reads is entirely in the caller.

#### `func_0x00060af4` decompiled: it is not per-class, it is a flat `500.0`

`Cannon_BaseSpeedKmh` (`0x08864af4`, EU `0x08864950`) is three instructions
on both pressings - `lui a0,0x43fa`; `mtc1 a0,f0`; `jr ra` - and never reads
its own argument. `Cannon_Init` calls it as `FUN_08864af4(param_2)` and adds
the result to `speed_kmh`, so the earlier "per-class base speed" framing this
page and `oag_gameplay::projectile::cannon::BASE_SPEED_KMH`'s doc comment
both carried was wrong about the shape, not the existence: it is a real,
disc-authored constant, just the same one for every craft. `BASE_SPEED_KMH`
is now `500.0`, not the `400.0` chosen placeholder.

## The Quake: a travelling point on the track's own spline, and nothing that touches a mesh

### `Weapon_FireQuake` (`0x0886c600`) - one instance, gated on a busy flag

```c
void Weapon_FireQuake(QuakePool *pool, Craft *craft, int craft_index) {
    Instance *q = pool->instance;                 // pool + 0x60 - a SINGLE instance, no array
    if (q->active != 0) return;                    // + 0x48 - only one quake in flight, ever
    QuakeStats *s = ActiveQuakeStats();
    craft->held = -1;
    craft->fire_flags &= ~8;
    craft->cached_damage        = s->damage;        // craft + 0x18c  <- stats + 0x60
    craft->cached_radius        = s->radius;         // craft + 0x190  <- stats + 0x64
    craft->cached_slowdown_time = s->slowdown_time;  // craft + 0x194  <- stats + 0x68
    q->flags = 0; q->flags = 1;
    q->owner = craft_index;                          // + 0x40
    q->id    = ++g_next_projectile_id;                // + 0x44
    Quake_Init(q, craft_index, craft->entity, &craft->fwd /* craft + 0x40 */);
    <telemetry only in >=14-player sessions>
}
```

**The Quake copies its own stats onto the *firing craft*, not onto the
instance** - `craft+0x18c/+0x190/+0x194` - exactly the shape `pickups.md` and
this thread's Open list already flag as the Repulser's own, unique-looking
behaviour ("the field is a state the craft is in"). It is not unique: this is
the **second** weapon that does it. Confidence **88** on the handler as a
whole - a full, unambiguous decompile with no VFPU trap in the path read.

### `Quake_Init` (`0x08874b14`) locates the firing craft on the track

This is the function that decides what "the Quake" *is* as a piece of state,
and it never touches a mesh, a vertex buffer or any collision geometry - it
only resolves the firing craft's own already-cached position on the track's
spline (the `entity+0xad8..+0xae4`/`+0xaf0`/`+0xb30` fields
[engine.md](engine.md#0xb10-is-splineptdown-and-the-whole-record-is-a-located-spline-sample)
already identifies as `SplinePt` records `AiTrack_LocatePosition` fills) into a
travelling wave's initial state:

1. Reads the craft's current segment reference(s) and its arc-position float
   (`entity+0xb30`, a field inside the first `SplinePt` record).
2. Checks a small global list (`_DAT_...598a8`, entries keyed by segment id
   plus a `[t_start, t_end]` window) to see whether the launch point straddles
   two segments - a **track-gap/junction continuity check**, not a trigger for
   anything visual: when it finds a match it flags `q->active = 1` and nudges
   the parametric `t` across the gap by a fixed epoch (`_DAT_...78cac / segment
   length`), so a quake launched right at a gap does not get stuck astride it.
3. Dots the craft's own forward vector (the fourth argument) against the
   segment's own tangent to pick a **direction sign** (`+1`/`-1`) for which way
   along the spline the wave should travel from the launch point.
4. Stores the segment reference(s), the parametric `t`, the direction sign and
   a "span" kind (one segment or two) into a handful of small module-scope
   globals - and nothing else. No draw call, no vertex write, no mesh handle
   anywhere in this function.

Confidence **82**: every field read is one this project has already located
and named from an independent reading (`engine.md`'s `SplinePt`), and the
shape (locate-on-spline, pick a direction, stash a scalar position) has no
plausible alternative reading given what it touches. Not chased: the exact
symbol identity of the module globals it writes (their real, un-rebased-looking
addresses were not resolved to real Ghidra data symbols this pass), and -
critically - **the function that advances that stored position every
subsequent frame was not found.** See [Open](#open-1).

### The "wave branch" inside `FUN_088418e0`, located precisely

`engine.md` already read that "`+0x60`/`+0x68`" (Quake's `damage`/`slowdown_time`)
are read by "`FUN_088418e0`'s wave branch" without giving an address. It is
`0x08841e60`-`0x08842064`, and it is the *damage and slowdown application*, run
for **every racing craft, every frame** (not gated on distance in this
function - see below):

```c
// inside FUN_088418e0, once per craft per frame, s2 = this craft's entity
if (craft_is_shooter(s2)) goto skip;                 // s2->0x360 == quake owner index
if (s2->craft->fire_flags & 0x10) goto skip;         // shielded
if (s2->0x860 & 0x40) goto apply;                    // "quake reached me" latch - see below
goto skip;
apply:
    entity->pending_slowdown += QuakeStats->slowdown_time;  // entity+0x130 += stats+0x68
    entity->pending_kind      = 5;                            // entity+0x138 - Quake's own id
    entity->pending_attacker  = <shooter craft index>;         // entity+0x13c
    Ship_Damage(entity, QuakeStats->damage, 2);                // stats+0x60, mode 2
```

`entity+0x130`/`+0x138`/`+0x13c` is the **same shared pending-hit channel** the
Missile and the Mine/Bomb blast already use (`engine.md`'s nine-writer table),
consumed the same way by `Ship_AddSlowdown`. `Ship_Damage` is already a named
function (`0x088439ac`) - confirmed by `get_function_by_address`, not
inferred - and `engine.md` already recorded that it compares its `weapon_kind`
argument against the literal `7`, which is `LeachBeam_Drain`'s own tag (below),
not the Quake's `5`; the two tags share one consumer function but distinct
values. Confidence **85** for this block: `entity+0x138 = 5` and the
`damage`/`slowdown_time` reads land exactly where `WeaponStats_ParseQuake`
(`0x0880c60c`) stores them, cross-checking `engine.md`'s independent reading of
the parser.

**What this settles for question 4**: the Quake's effect on a craft is applied
through the identical generic "pending hit" mechanism every other weapon here
uses - a scalar damage number and a scalar slowdown-seconds number, credited
once a latch bit is set. Nothing in the whole chain (`Weapon_FireQuake` ->
`Quake_Init` -> this branch) reads or writes a vertex, a mesh handle, a track
collision triangle, or any per-segment geometry override. **The Quake is a
travelling impulse along the track's own path (a segment + parametric-`t`
position plus a direction sign), not a deformation of the track**, and nothing
found here needs new geometry-mutation machinery to build - the missing piece
is purely the wave's own per-frame position update (below).

### `Quake_Update` (`0x0891d268`) is the missing per-frame advance, found 2026-09-07

This is the function `Quake_Init` leaves for. It was not reachable by a direct
`jal` scan - `psp-relocate.py callers` returns zero for it, same as for the
already-documented `FUN_088418e0` "wave branch" above - but it is reachable as
data: `psp-relocate.py xrefs 0x0891d268` finds exactly one plain 32-bit
reference, at `0x08ad1930`, sitting inside a table of eight-byte
(function-pointer, padding) entries alongside several other small handlers and
one null slot. `FUN_088418e0` has the identical one-reference, table-resident
signature at its own table. Neither this page nor a live run confirms *which*
driver walks that table or how often - the read below is call-site evidence
that `Quake_Update` is dispatched indirectly the same way `FUN_088418e0` is,
not a tick count observed running. **A live PPSSPP breakpoint on `0x0891d268`
with `--give Quake`, per `docs/reverse-engineering/ppsspp-debugger.md`, would
settle it outright and was not run this pass.**

What it does, read at the instruction level (immune to the `lv.q`/`sv.q` trap
below, since every operation here is scalar `lwc1`/`swc1`/`add.s`/`div.s`, one
instruction at a time - see `0x0891d340`-`0x0891d384`):

```
0x08b3bfb4 (_DAT_0006281c, "time since launch") += dt
0x08b3bfa8 (_DAT_00062810, the wave's own t)
    += (0x08b3bfb0 (_DAT_00062818, ±270.0) * dt) / 0x08b3bfac (_DAT_00062814)
0x08b3bfa8  = fmod(0x08b3bfa8 + 1.0, 1.0)     // wrap into [0, 1)
```

`0x08b3bfac` (`_DAT_00062814`) is set exactly once, in `Quake_Init`, from
`func_0x00118a4c()` called with no arguments and never re-read after launch -
this page reads that as "the wave's speed is normalized against the *launch*
segment's own length, held fixed for the wave's whole life," but
`func_0x00118a4c` itself (loaded address `0x0891ca4c`) was not decompiled this
pass, so **do not take "segment length" as confirmed units** - it is a
plausible reading of an unread callee, not a measurement. The magnitude at
`0x08a7cca8` (`_DAT_00278ca8`), read directly as **270.0**, is what `Quake_Init`
copies `±` into `0x08b3bfb0` (the sign coming from the dot product against the
firing craft's forward vector) - a fixed engine constant, not one of
`WeaponStats_ParseQuake`'s four attributes (`damage`, `radius`,
`slowdown_time`, `absorb`), so the Quake's travel speed is not author-tunable
per this reading. **The same 270.0 does double duty**: besides the `t`-rate
divide above, `Quake_Update` also computes `ABS(0x08b3bfb0) * dt` at
`0x0891d39c`-`0x0891d3c8` as a plain world-distance increment, spent against
Euclidean segment lengths (`vsub_q`+`vdot_t`+`vsqrt_s` between consecutive
`SplinePt` samples) by the segment-cursor walk below - which is one real
constraint on what `0x08b3bfac` can be (a length in the same units 270.0 is a
rate in), even though its exact identity is still unread. Confidence **85**
for the advance formula itself (instruction-level, every global cross-checked
against `Quake_Init`'s own writes); confidence **55** for calling
`0x08b3bfac` a segment length specifically, which is why it carries no name
here.

The rest of the function (`0x0891d3cc` onward) walks a segment-index cursor
per span (one or two, per `_DAT_002bb598`) against the distance travelled this
frame, advancing to the next/previous track segment when the wave's progress
exceeds the current one's length and handling the two-span (track-gap) case
`Quake_Init` set up - the mechanism `Quake_Init`'s own comment already
predicted ("a short, dedicated advance-the-quake-along-the-spline...loop").

### `Quake_SampleSpan` (`0x0891c028`) turns `t` into two points across the track

Called from `Quake_Update` alone (`psp-relocate.py callers` returns exactly
one site, `0x0891d7f0`) - this is private machinery of the Quake, not a
general track sampler. Given a span index, it walks the wave's current
segment/cursor state and interpolates two points from the segment's own
`SplinePt` record - the same struct `engine.md` names, read here at the
offsets that page already assigns to the left/right track edges - one at each
edge of the track, at the wave's current arc position. It returns those two
points, a progress fraction, and a validity bool (false once the span has run
off either end of its track-gap window). Confidence **76**: the scalar shape
(two edge samples, a lerp, a validity gate) is unambiguous; several of the
vector ops inside it (`vsub_q`/`vscl_q`/`vdot_t`) were read from
`decompile_function`'s text rather than independently confirmed instruction by
instruction the way the advance formula above was, so a `lv.q`/`sv.q`
misattribution (see [workflow.md](../../workflow.md)) inside it is not fully
excluded.

### What `Quake_Update` builds from those two points: `WO_QUAKE`, not a mesh

`Quake_Update` uses the two edge points from `Quake_SampleSpan` to build a
transform: position is their midpoint, and the basis comes from their
normalized separation crossed with a fixed reference vector
(`0x0891dad8`-`0x0891dbc0`). **In the same block**, a call into
`AiTrack_LocatePosition` (`0x0887ce78`, already named) is made with the
midpoint slot as one argument (`0x0891da70`-`0x0891da90`) - but it runs
*before* the basis is built and none of the cross-product/normalize
instructions that build the basis consume its result, so **"orientation
refined by the track" is not what this reading supports**; the call's purpose
here is not established. And, the first time a given wave instance's node id
is zero, calls:

```
Psys_Spawn_q(new_node, "WO_QUAKE", 'QUAK' /* 0x4b415551 */, transform, 1, 0);
```

Read directly: the name argument is a static pointer to `0x08a88580`, and
`inspect_memory_content` at that address returns the ASCII bytes **`WO_QUAKE\0`**
verbatim - not inferred from the fourcc, an independent string read. `WO_QUAKE`
is already in `docs/formats/pob.md`'s 35-name authored-effect list, alongside
every other weapon's own effect (`WO_PLASMA_HEAD`, `WO_SHURIKEN_BOUNCE`, etc.),
and the calling shape - `Psys_Spawn_q(node, name, fourcc, transform, ...)` -
matches `plasma.md`'s `WO_PLASMA_HEAD`/`'PLHE'` and `shuriken.md`'s
`WO_SHURIKEN_BOUNCE`/`'SHBO'` exactly. **This settles "the Quake's own
effect/trigger is unread" from this page's own Open list below**: the trigger
is `Quake_Update`, firing once per wave instance, and the effect is the disc's
own `WO_QUAKE`, not anything invented for this project.

The same branch also builds a second, `0x70`-byte object attached to the same
transform, sets its `+0x38` field to `600.0`, and passes it to
`func_0x001352b0` - already named **`Sound_Play`** (`0x089392b0`) - as
`Sound_Play(1.0, obj, _DAT_002bddf8, 0, _DAT_00284554, ...)`. Read as: the wave
carries its own positional sound cue with a `600.0`-unit falloff, travelling
with it the same way the particle effect does. Every frame after creation
(the `if (existing_node_id != 0)` path, not re-entering `Psys_Spawn_q`), the
transform is recomputed from the current two edge points and a scale value
(edge-to-edge distance `/ 50.0`) is applied through two more calls,
`func_0x000f043c`/`func_0x000f04d8` (loaded `0x088f443c`/`0x088f44d8` -
`0x088f443c` falls in a gap between two analyzed functions and is
**unanalyzed**, not merely unnamed; `0x088f44d8` is analyzed but unnamed). A
further call, `func_0x000ec0c0` (`0x088f00c0`, analyzed, unnamed), passes one
edge point and the literal `4` to an unidentified handler - possibly a
camera-shake or screen-effect trigger; not chased this pass.

**What this answers for the maintainer's play observation:** the wave is
neither raw vertex displacement of the track mesh nor a shader-side
displacement - it is the disc's own `WO_QUAKE` particle effect plus a
travelling positional sound, both re-positioned (to the midpoint of the two
current track-edge samples) and re-scaled (to the track's own width at that
point, via the `/ 50.0` term) every frame to follow the wave along the spline.
**That is very plausibly what reads as "a concrete wave" to a player** - an
effect that tracks the road's own width and travels its own spline looks like
it belongs to the road, without a single byte of the road's own mesh
changing. (Whether it also tracks the track's *banking* is unestablished -
see the `AiTrack_LocatePosition` correction above; the `SplinePt` fields
`Quake_SampleSpan` reads at `pauVar16[3]`/`pauVar16[4]`, which `engine.md`
already assigns, are where that would come from if it does.) This is a
plausibility argument for reconciling the play observation, not a
frame-by-frame visual comparison against the original - nobody has looked at
what `WO_QUAKE.POB` itself draws.

**Shape 3 (shader-side displacement) is not positively excluded by anything
read in this function**, but the PSP's GE has no programmable vertex stage to
put a position-keyed displacement in, which is a hardware constraint against
that shape existing at all on this platform, independent of what this page
did or didn't find in software.

### The latch setter, found 2026-09-07: it was inside the already-cited function all along

**`entity+0x860 & 0x40`'s setter is not a separate, unlocated function - it sits
a dozen lines above the damage branch this page already quoted, inside the
same `FUN_088418e0` block (`0x08841e60`-`0x08842064`) that block's own prose
had already summarised as "if (s2->0x860 & 0x40) goto apply".** Read at
instruction level this pass:

```c
// still inside FUN_088418e0, s2 = this craft's entity, once per craft per frame
fVar_smoothed = s2->0x870;                              // a per-craft smoothed intensity
fVar_raw      = Quake_ProximityToCraft_q(s2->0x360);     // 0x0891c7e0, s2's own craft index
s2->0x870 = fVar_smoothed + (fVar_raw - fVar_smoothed) * 8.0 * dt;   // exponential smoothing
...
if (s2->0x870 <= 0.1 || s2->0x360 == quake_owner) {
    s2->0x860 &= ~0x40;                       // out of range, or this craft is the shooter: clear
} else if (owner_craft == 0 || !(owner_craft->fire_flags & 0x10)) {   // not shielded
    if ((s2->0x860 & 0x40) == 0) {            // rising edge only
        owner_craft->pending_slowdown += QuakeStats.slowdown_time;    // entity+0x130
        owner_craft->pending_attacker  = quake_owner;                  // entity+0x13c
        owner_craft->pending_kind      = 5;                            // entity+0x138
        Ship_Damage(QuakeStats.damage, s2, 2, 5, 0);                   // 0x088439ac
        Sound_Play(1.0, owner_craft_cue_slot, _DAT_002bddf8, 0, <hit-cue-bank>, 0);  // 0x089392b0
    }
    s2->0x860 |= 0x40;                        // set/hold the latch
} else {
    s2->0x860 &= ~0x40;                       // shielded: clear
}
```

So the latch is edge-triggered off a **smoothed proximity value**, not a bare
in-range test: `Quake_ProximityToCraft_q` (`0x0891c7e0`, confidence 60) looks
the craft up by index and delegates to `Quake_SpanIntensityAt_q` (`0x0891c82c`,
confidence 55), which matches the craft's own current segment against the
same small span table `Quake_Init`/`Quake_Update` maintain (`_DAT_...598a8`)
and returns a per-span stored value when the craft's own parametric position
falls inside a narrow window around the span's own recorded point, else `0`.
That raw 0-or-something value is smoothed toward at a fixed `8.0/s` rate and
compared against a flat `0.1` threshold - a debounce, so a craft on the
threshold's edge does not chatter the hit on and off pixel to pixel.
`Quake_SpanIntensityAt_q` also confirms a **coarse 200-unit world-distance
prefilter** ahead of the finer span match (`ABS(wave_t - craft_t) *
_DAT_00062814 <= 200.0`) - independent evidence that whatever `_DAT_00062814`
is, it is a length in the same units 200.0 is, consistent with (not proving)
"segment length".

**What this resolves for the page's own Open list**: the hit and the
travelling visual *are* wireable to each other, through a proximity test - the
Quake's authored `radius` (`<Stats absorb damage radius slowdown_time>`,
`docs/formats/weapon-stats.md`) has no other read consumer anywhere in this
chain, which is the strongest evidence yet that `radius` **is** this
mechanism's own gate, authored rather than the `200.0`/`0.1` engine constants
above. Confidence **78** for the mechanism as a whole (block read at
instruction level, cross-checked against `Quake_Init`'s and `Ship_Damage`'s
already-established fields); the two new callees are lower (60 and 55) because
their own field semantics (`+0x5c`, `+0x64`, `+0x6c`, `+0x40` on the span
table's own 0x80-byte records) are read from shape alone, not confirmed
independently.

**A second, distinct sound cue.** The hit plays its own `Sound_Play` call,
separate from the travelling wave's positional loop (`_DAT_00284554` in
`Quake_Update`) - this one reads `_DAT_00277740` for its bank slot. Neither
DAT literal was tracked down to a real cue name this pass; both are read as
"a sound plays here", not "this sound plays here".

> **The hit cue is resolved, 2026-09-08: it is `QUAKEHIT`.** Decompiling
> `FUN_088418e0` whole for the LeachBeam pass below prints the argument as
> `PTR_s_QUAKEHIT_08a7b740`, a string pointer rather than a bare DAT - so this
> one *is* "this sound plays here" now, at confidence 85. The travelling wave's
> own cue in `Quake_Update` is still only "a sound plays here"; the two were
> never the same literal.

### The three remaining helpers, read - and one correction

- **`func_0x00118a4c` (`0x0891ca4c`) is *not* a segment-length getter, and the
  page's own confidence-55 guess is now doubted rather than confirmed.**
  Decompiled in full: it is a generic lazy-singleton resource loader -
  `if (cache == null) { cache = load(0x58020, <a string literal>, ...); }
  return *cache;` - shared plumbing with no Quake-specific shape at all, and
  `Quake_Update` itself calls it a *second* time this pass revealed, feeding
  its return straight into `AiTrack_LocatePosition` as an argument, which a
  segment length has no business being. **The string literal argument could
  not be resolved to confirm what resource this loads**: reading it at the
  address this database's own `lui`/`addiu` pair appears to encode returned
  HUD/front-end text (`"Custom"`, `"%s Bar"`, `"Info->Ship"`), which is either
  the wrong resource entirely or - far more likely, given the zero-PSP-relocation
  defect measured this same day at confidence 92 (see `docs/ghidra/workflow.md`
  and the toolchain notes on `get_xrefs_to`) - a data address this database has
  not relocated and therefore cannot be trusted at all. Not fixable until the
  reimport lands; recorded as "unread, and the previous hypothesis about it is
  now weaker" as the honest state rather than silently keeping a confidence-55
  guess that this pass's own new evidence argues against.
- **`func_0x000f043c`/`func_0x000f04d8` (`0x088f443c`/`0x088f44d8`) confirmed
  as a get/set pair on a scene node's transform, one of which carries the
  scale.** Read in place inside `Quake_Update` this pass, past what
  `decompile_function`'s text alone gave before: `f043c(node, buffer)` then a
  direct assignment of `edge_distance / 50.0` into a fixed slot of the same
  `buffer`, then `f04d8(node, buffer)` - a read-modify-write on the node's own
  state block, and the literal `/ 50.0` line is what this page already read
  from `decompile_function`'s output, now seen inside its own load-bearing
  read/write pair rather than a floating computation. Their true generic
  name is not guessed at - both look like shared scene-node plumbing rather
  than anything Quake-owns, so neither is renamed; see
  [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md)'s "below 50,
  do not rename" rule, applied here to the *name* rather than the *shape*,
  which this pass is confident of.
- **`func_0x000ec0c0` (`0x088f00c0`) decompiled in full**: a generic dispatcher
  taking a shared global handle, a small integer "kind" (`4` for the Quake's
  own call site) and a position, storing both into the handle's own state and
  then calling `kind_table[kind]()` through a twelve-entry function-pointer
  table at a fixed base. The shape (state-then-dispatch) is confirmed; *which*
  of the twelve handlers kind 4 is, and therefore whether this page's
  "possibly a camera-shake or screen-effect trigger" guess is right, is not -
  the table's own entries were not walked this pass. Not renamed, for the same
  reason as the pair above.
- **Orientation remains unestablished, now cross-checked rather than merely
  read once.** `Quake_Update`'s own `AiTrack_LocatePosition` call was
  independently re-derived this pass rather than only quoted from
  `decompile_function`'s text: its output record is written and then never
  read by anything that feeds the cross-product basis built two dozen lines
  later, confirming (not merely repeating) "orientation refined by the track
  is not what this reading supports". Nothing new narrows it.

### 2026-09-08: the lifetime, found - 5.0 seconds from launch, and the Quake does deform the track

This section answers the page's own longest-standing Quake question and
**corrects one of its conclusions**. Both come from the half of the weapon
neither `Quake_Init` nor `Quake_Update` reaches: a second apparatus, keyed on
the *road spans* the wave passes over.

#### The road-span records, and the ripple that is the Quake

`DAT_08b33040` is a **pointer** to a track-side table, not a scalar. Settled at
instruction level rather than from the decompiler, which renders it three
different ways in three functions and had this page one careless step from a
fourth address slip:

```
0891cb30  lui   a0, 0x8b3
0891cb34  lw    a0, 0x3040(a0)     ; a0 = *(0x08b33040) - a POINTER load
0891cb38  lw    a1, 0x4(a0)        ; a1 = table->0x04
0891cb3c  addiu a1, a1, 0x1
0891cb40  sw    a1, 0x4(a0)        ; table->0x04 += 1
```

So `table->0x00` is a record count, `table->0x0c` the record base, and
`table->0x04` a counter. `DAT_08b33044`, which `Quake_Init` writes, is a
*separate* global that happens to sit next to the pointer - it is not
`table->0x04`, and reading the two as one is exactly the kind of slip this page
has already carried twice. Each record is `0x80` bytes: `+0x40` its own length,
`+0x52` a segment id, `+0x54`/`+0x58` a `[t_start, t_end]` window, `+0x30` and
`+0x34` neighbour indices in each direction, `+0x5c`/`+0x60` the ripple's
leading and trailing edge, **`+0x74` its age**, `+0x78` its direction (`±270.0`,
the same constant `Quake_Update` spends), and `+0x7c`..`+0x7f` four state bytes
of which `+0x7e` is "armed" and `+0x7d` is "retire me".

#### `Quake_UpdateSpans` (`0x08874a30`) is the per-frame driver, and it clears the busy byte

```c
void Quake_UpdateSpans(float dt, Quake *q) {
    FUN_0885eca4(q);
    table->live = 0;                                  // table->0x04
    for (i = 0; i < table->count; i++)
        if (record[i].armed) Quake_UpdateSpan(dt, &record[i], 1);  // each bumps table->live
    if (table->live == 0) {                            // nothing rippled this frame
        q->0x3c &= ~8;  q->0x2c &= ~4;
        DAT_08b317a8 = 0;                              // the flag Weapon_FireQuake sets to 2
        q->0x48 = 0;                                   // <- the busy byte, cleared
    }
}
```

**This is what re-opens `Weapon_FireQuake`'s own gate.** That handler returns
early on `*(char *)(pool->instance + 0x48) != 0`; `Quake_Init` sets that byte to
`1` when it arms the launch span, and nothing else in the whole apparatus writes
it. So a Quake becomes fireable again on the first frame no road span is
rippling, and not before. It also explains `Quake_Update`'s own head gate
(`0 < DAT_08abf594 && table->live != 0`): once the ripple is empty, the wave's
travelling position stops advancing, `DAT_08abf598` goes to `0`, and
`Quake_SampleSpan` returns invalid, which is what tears down the `WO_QUAKE`
effect and the `QUAKETRAVEL` emitter.

#### The 5.0 seconds, at instruction level

`Quake_UpdateSpan` (`0x0891cab8`), first fifteen instructions:

```
0891cacc  lbu    a1, 0x7f(s1)          ; record->0x7f - "skip one frame", set by propagation
0891cae8  bne    a1, zero, 0x0891cb20  ; taken -> clear it and return
0891caf8  lwc1   f12, 0x74(s1)         ; record->0x74 - the age
0891cafc  lui    a0, 0x40a0            ; 0x40A00000 == 5.0f
0891cb00  add.s  f12, f12, f20         ; age + dt
0891cb08  c.le.s f12, f13              ; (age + dt) <= 5.0 ?
0891cb10  bc1f   0x0891cb28            ; no ->
0891cb28  li     a0, 0x1
0891cb2c  sb     a0, 0x7d(s1)          ; record->0x7d = 1, the retire byte
...
0891cb68  add.s  f14, f14, f20         ; age += dt
0891cb6c  swc1   f14, 0x74(s1)
```

`lui a0, 0x40a0` is `5.0f` exactly. `+0x7d` is the same byte `Quake_RetireSpan`
(`0x0891b7a4`) sets immediately before it clears `+0x7e`, and with it set
`Quake_UpdateSpan` zeroes the trailing edge, skips both propagation calls and
lets the span go.

#### Why 5.0 is the *wave's* lifetime and not each span's

Because the age is **inherited, not restarted**. `Quake_ArmSpan`
(`0x0891b714`) writes its second argument straight into the new span's `+0x74`:

```c
bool Quake_ArmSpan(float dir, float age, float t, Span *s) {
    bool armed = s->0x7e == 0;
    if (armed) {
        s->0x7c = 1; s->0x7d = 0; s->0x7e = 1; s->0x7f = 0;
        s->0x78 = dir;  s->0x74 = age;              // <- inherited
        s->0x64 = 0; s->0x68 = 0; s->0x5c = t; s->0x60 = t;
        Quake_UpdateSpan(0.0, s, table->live < 0x19);
    }
    return armed;
}
```

and both propagators pass the *parent's* own `+0x74`:
`Quake_PropagateForward` (`0x0891b7f4`) walks `+0x30`'s two neighbour indices
and `Quake_PropagateBackward` (`0x0891b940`) walks `+0x34`'s, each calling
`Quake_ArmSpan(parent->0x78, parent->0x74, t, neighbour)`. `Quake_Init` starts
the chain with a literal `0`. **So every span the ripple ever reaches carries
one clock begun at launch, and the whole apparatus crosses 5.0 s together.**

Confidence **85**: the literal is instruction-level, the retire byte is
cross-checked against the one function that retires a span, and the inheritance
is a plain scalar copy read in three separate decompiles that agree. What is
*not* established is whether a frame's worth of `dt` slop at the boundary
matters, and whether `Quake_UpdateSpan`'s two early-return paths (the wave front
running off a span's far end, forcing the neighbour and retiring this one) can
extend the chain past the bound - they cannot, because they run under the same
`+0x7d` short-circuit, but that was reasoned rather than traced.

#### The correction: the Quake *is* a track deformation

This page states, under the wave branch inside `FUN_088418e0`, that "the Quake
is a travelling impulse along the track's own path ..., not a deformation of the
track". **That conclusion is right about the three functions it was drawn from
and wrong about the weapon.** `Quake_UpdateSpan`'s body, past the ageing above,
rewrites the road's own packed `short3` vertex positions in place - a `vcos_q`
profile over a window between the ripple's leading and trailing edges, clamped
by `vmax_q`/`vmin_q`, converted back through `vf2in_q`/`vi2s_q` and stored to
the mesh. The deformation was simply not in `Weapon_FireQuake`, `Quake_Init` or
the damage branch, which is where this page looked.

The narrower claim survives intact and is the one the port relies on: **the
damage and slowdown are a scalar impulse through the shared pending-hit
channel, with no geometry involved**, and nothing in the damage path needs the
deformation. What changes is that "does the Quake deform the track" is now
answered **yes**, at confidence 85, and this engine draws none of it.

#### The object's own two ends

`Quake_Construct` (`0x0891ba98`) increments `DAT_08abf594`, memsets every wave
global to zero and installs the vtable at `0x08ad190c` - which is the table
`Quake_Update`'s one data reference (`0x08ad1930`) sits inside, settling that
`Quake_Update` is a virtual method of this object rather than a table-driven
handler of unknown ownership. `Quake_Destruct` (`0x0891bcc8`) decrements it,
calls `Quake_StopAllSpans` (`0x0891bf98`) - which retires every armed span,
zeroes `table->0x04` and clears the span count - and tears down both sound
emitters. `Quake_Destruct` has no direct caller (`get_xrefs_to` returns none),
so it is reached through the vtable: object teardown at race end, not wave
expiry.

### 2026-09-08: the LeachBeam's texture, located - and it is not the ribbon's

`Data\Tex\Weapons\leachbeam_surface.mip` (string at `0x08a8839c`), loaded by
`Texture_LoadEffectSurfaces` (`0x0890cc1c`) into `DAT_08af2804` through the same
`FUN_089277ac` loader `Cannon_LoadTextures` uses for its two `.mip`s.

**Both resolve in `Data.wad` on the USA pressing**, checked rather than assumed:

```sh
cargo run -q -p oag-tools --bin oag-wad -- cat \
    'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' \
    'Data\Tex\Weapons\leachbeam_surface.mip'
```

returns 2,064 bytes opening `20 00 20 00 08 00 01 00` - a 32x32, 8-bit
paletted `.mip`, which is exactly `16 + 32*32 + 256*4`. Its sibling
`Data\Tex\Weapons\absorb_surface.mip` (string at `0x08a88378`, read whole out
of memory rather than trusted from the decompiler's truncated
`s_Data_Tex_Weapons_absorb_surface__`) is the same 2,064 bytes and the same
header.

**The directory is the finding.** The Cannon's textures live under
`Data\Weapons\Textures\`; this one lives under `Data\Tex\Weapons\`. Hashing
candidate names against `Data.wad` under the Cannon's layout - the technique
that found the Cannon's own two - would have returned nothing for every
plausible LeachBeam name, and read as "the texture is not on the disc". A
`(?i)leach` string search over `.rodata` found it in one call. **Prefer the
string search to name-hashing when the loading function has not been read**:
the string is what the original actually passes, a hashed guess is a guess.

**And it is not the beam's ribbon.** `DAT_08af2804` has two readers,
`FUN_0890d828` (`0x0890d828`) and `FUN_0890e140` (`0x0890e140`), and the first
walks a craft's own mesh chain and passes the texture to `FUN_0890e304` per
sub-mesh, gated on a scalar at `craft+0x74 -> +0x4c` being positive. That is a
**surface overlay on the drained craft's hull**, the sibling of
`Data\Tex\Weapons\absorb_surface.*` (`DAT_08af2800`), which the shield's own
absorb pass uses in exactly the same shape. The ribbon chain
`LeachBeam_InitLocked` zero-fills the UV columns of is textured by something
else, and that something else is still unlocated. Confidence **85** on the
identification and on which thing it textures; the ribbon's own texture is
**open**.

Neither is drawn by this engine, and both are honest absences rather than
stand-ins.

### Open

- **The LeachBeam ribbon's own texture is still unlocated**, and
  `Data\Tex\Weapons\leachbeam_surface.mip` is not it (above). The next step is
  the ribbon's own draw call out of `LeachBeam_Advance` (`0x08873fa0`) rather
  than another string search - the two `Data\Tex\Weapons\` strings are the only
  LeachBeam-flavoured texture paths in `.rodata`, so the ribbon's texture is
  either shared with something else or named for what it looks like rather than
  for the weapon.
- **`WO_QUAKE.POB` itself was not inspected.** The trigger and its transform
  are recovered; whether the effect it draws looks like the maintainer's
  "concrete wave" description is a separate, unchecked question. If it parses
  and plays visually wrong for this reading, that is evidence against the
  reconciliation above, not against the trigger recovery itself.
- **`WO_QUAKE_DETONATOR_TRAILS`** (`docs/formats/pob.md:576`) **is a second,
  unlocated Quake effect name** - distinct from `WO_QUAKE` above, plausibly an
  impact/detonation burst rather than the travelling wave. Not chased.
- **The two hit-cue and node-plumbing DAT literals above**, and the kind-4
  table entry `func_0x000ec0c0` dispatches to - none resolved this pass, and
  the string-literal trap makes any of them a bad use of time before the
  reimport lands.
- ~~The base speed `Cannon_Init` reads (`0x00060af4`) is unread, and so is
  whatever a Cannon round's collision does on a hit.~~ Both read 2026-09-09 -
  see "`Cannon_UpdateRound` read" under the Cannon section above.

## The LeachBeam: a resolved link to a pre-locked target, drained every tick it holds

### `Weapon_FireLeachBeam` (`0x08866658`) fires world-wide, not per-craft

```c
void Weapon_FireLeachBeam(LeachBeamPool *pool, Craft *craft, int craft_index) {
    pool->flags |= 2;
    craft->held = -1;
    craft->fire_flags &= ~0x8000;
    if (pool->live != 0) return;          // pool + 0x68 - ONE beam in the WHOLE RACE at a time
    Instance *b = pool->instance;          // pool + 0x64
    b->flags = 0; b->flags = 1;
    b->owner = craft_index;                // + 0x40
    b->id    = ++g_next_projectile_id;      // + 0x44
    Entity *shooter = craft->entity;         // craft + 0xf0
    if (craft->lock_target == -1) {          // craft + 0x16c
        LeachBeam_InitUnlocked(b, shooter->pose, craft_index, shooter->emitter, shooter);
    } else {
        LeachBeam_InitLocked(b, shooter->pose, craft_index,
                              craft->lock_target,      // craft + 0x16c - target craft index
                              craft->lock_node,          // craft + 0x168 - target's own scene node
                              shooter->rate_or_cue,       // shooter + 0x50
                              shooter);
    }
    <telemetry only in >=14-player sessions>
    pool->live += 1;
}
```

**Only one LeachBeam can be in flight in the entire race at once** - `pool+0x68`
is a world cursor, not a per-craft cooldown, a stricter gate than any other
weapon here. Confidence **88** - direct decompile, no VFPU trap on this path.

### Victim selection is the Missile's own lock-on, reused whole

`craft+0x16c`/`craft+0x168` are filled **before** this handler ever runs, by
the same `Ship_AcquireLock` scan `missile.md` already documents in full: a
longitudinal cone ahead of the craft, nearest-along-forward wins, using the
LeachBeam's own `lock_min_dist`/`lock_max_dist` at `<Stats>+0x114`/`+0x118`
(`missile.md`'s own reading - "the two weapons that author `lock_max_dist`/
`lock_min_dist` are the Missile and the LeachBeam"). **Nothing new needs
building for target selection** - it is the Missile's lock, unmodified, already
buildable from `missile.md` alone.

### The two constructors: an unlocked beam fizzles, a locked one becomes a real link

`LeachBeam_InitUnlocked` (`0x08872da8`, `kind = 2`) copies the shooter's own
emitter matrix as the beam's pose and sets its "target" field to the
**shooter's own entity** - there is no external target at all. Its pool update
(below) gives kind-2 instances no distance or damage logic whatsoever: they
just run out a short expiry and retire. **Firing without a lock plays the cue
and does nothing else.**

`LeachBeam_InitLocked` (`0x08873d3c`, `kind = 1`) is the real weapon:

- `instance+0x48 = target_craft_index` (from `craft+0x16c`).
- `instance+0xa0 = target_scene_node` (from `craft+0x168`) - **this is the field
  `bad-memory-access-halt.md` found null and crashed PPSSPP on**, when that
  page's raw fire-bit test skipped the lock entirely. A legitimately fired
  locked beam always has this filled straight from the lock, closing that
  page's own "what fills `instance->0xa0`" Open item at confidence **85**: it
  is the target's own resolved scene node, copied verbatim from the lock's own
  `craft+0x168`, not computed or re-resolved here.
- Allocates a positional-audio-style handle (`instance+0x4c`) and zero-fills a
  **32-entry chain of `0x30`-byte transforms** (`instance+0x3c0`..) alternating
  between two initialisation patterns - this reads as the beam's own drawn
  geometry, a segmented link between the two craft rather than a single static
  bolt, consistent with a "beam" needing more than one quad to draw convincingly.

### `LeachBeam_UpdatePool` (`0x08866b08`) - the function `bad-memory-access-halt.md` read half of

That page named this function's ownership "confidence 78 weapon-instance pool
machinery, confidence 60 on anything narrower" - reading it whole this pass
settles the narrower question: **it is the LeachBeam's own pool**, not a
shared or Missile-owned one. Every field it touches (`+0x48` target index,
`+0xa0` target node, `+0x5c` owner-craft-entity, `+0x54` kind 1/2) matches
exactly what the two constructors above just wrote, with no other weapon's
constructor touching any of them. Confidence **85**, up from that page's 60.

For a live kind-1 instance not yet flagged disconnected (`instance+0x3c & 0x40`
clear):

1. Measures the floor-clamped distance between the beam's own resolved world
   position and the **target's** resolved world position, via
   `FUN_08872f54` - the exact distance function `bad-memory-access-halt.md`
   already read in full (`max(|posA - posB|, floor)`).
2. Compares it against **`ActiveLeachBeamStats+0x11c`** - a fourth LeachBeam
   attribute, immediately adjacent to `lock_min_dist`/`lock_max_dist`
   (`+0x114`/`+0x118`) and not previously identified; call it the beam's own
   maximum reach. Past it, or if the target is invulnerable
   (`target+0x860 & 0x1000`), shielded (`target->entity+0x1b8 & 0x10`), or
   either craft's race-state check fails, the link is marked disconnected
   (`instance+0x3c |= 0x40`) rather than destroyed outright - it can sit
   disconnected for a few ticks (a linger window) before the pool actually
   retires it (`instance+0x3c & 4`).
3. **While connected**, every tick calls `func_0x0006f020(instance)` for a
   rate float, writes it into a per-craft readout slot (presumably a HUD/audio
   amplitude on the shooter), and - whenever that rate is positive - calls
   `LeachBeam_Drain` (below).

### `LeachBeam_Drain` (`0x08866804`) - the transfer, both directions

```c
void LeachBeam_Drain(Pool *pool, int instance_index, int owner_idx, int target_idx) {
    Entity *owner  = pool->craft_entities[owner_idx];    // pool + owner*4 + 0x44
    Entity *target = pool->craft_entities[target_idx];
    if (owner->fire_flags & 0x10) return;                 // shielded - gates BOTH halves
    if (Ship_IsValid(target)) {
        float amount = <func_0x0006eedc>(instance);
        target->pending_leach   += amount;                 // target + 0x120
        target->pending_kind     = 7;                        // target + 0x138 - LeachBeam's own id
        target->pending_source   = <ActiveLeachBeamStats+0x110>;   // target + 0x134
        target->pending_attacker = owner_idx;                // target + 0x13c
    }
    if (Ship_IsValid(owner)) {
        float amount = <func_0x0006ef18>(pool->instance[instance_index]);
        owner->pending_kind     = 7;                          // owner + 0x138
        owner->pending_gain    += amount;                      // owner + 0x128
        owner->pending_attacker = <instance's own owner field>; // owner + 0x13c
    }
}
```

**This is the literal "leach"**: a victim accumulator (`entity+0x120`, distinct
from the shared `+0x130` pending-hit slot every other weapon uses) and a
*shooter* accumulator (`entity+0x128`) are both credited from the same
transfer, every tick the link survives the range/shield/state gate above -
draining is continuous, not a single hit, and both halves stop the instant
either craft is shielded. `entity+0x138 = 7` confirms LeachBeam's own weapon-
type tag in the same field the Quake writes `5` into and `engine.md` already
found `Ship_Damage` comparing against `7` - two independent paths converging on
the same tag value. Confidence **78**: the transfer's *shape* (two accumulators,
symmetric credit, shield-gated) is read at instruction level; the two rate
functions (`func_0x0006eedc`, `func_0x0006ef18`) and what actually consumes
`+0x120`/`+0x128` into visible health/shield state were not decompiled this
pass.

### 2026-09-08: the whole weapon read, and an arithmetic slip in the section above

**Every item this section's own Open list carried is closed below**, and the
first thing that had to be fixed to close any of them is an address.

#### The two rate functions were rebased wrong, the same way the Cannon's was

The section above names them `func_0x0006eedc` = `0x0886eedc` and
`func_0x0006ef18` = `0x0886ef18`. **Both are wrong**: `0x08804000 + 0x0006eedc`
is **`0x08872edc`**, not `0x0886eedc`. The stated addresses decompile inside
`FUN_0886ee88`, which is not a LeachBeam function at all - it is a radial
impulse over every craft, adding `(1 - d/stats+0x1c) * stats+0x20` along the
separation into `entity+0x110`, very plausibly the **Repulser's** own push and
recorded here only as a lead for whoever opens that weapon.

This is the second `0x4000`-magnitude slip on this page (the first produced
`0x088537ac` for the Cannon, corrected at the top). The corrected addresses land
immediately beside `FUN_08872f54` - the distance helper this section already
cites - and `LeachBeam_InitUnlocked` (`0x08872da8`), which is the corroboration.

#### `WeaponStats_ParseLeachBeam` (`0x0880d328`), decompiled whole

One `Xml_AttributeNameIs` arm per attribute, each storing to a distinct offset.
This **closes "`ActiveLeachBeamStats+0x11c`'s attribute name is unmeasured"**:

| offset | attribute | Race table | Eliminator table |
| --- | --- | --- | --- |
| `+0x104` | `damage` | `0.1` | `0.5` |
| `+0x108` | `absorb` | `10` | `15` |
| `+0x10c` | `repair` | `0.1` | `0.3` |
| `+0x110` | `slowShipFactor` | `0.8` | `0.5` |
| `+0x114` | `lock_min_dist` | `10` | `10` |
| `+0x118` | `lock_max_dist` | `200` | `200` |
| `+0x11c` | `range` | `250` | `250` |
| `+0x120` | `active_time` | `3` | `10` |
| `+0x124` | `energy_multiplier` | `50.0` | `50.0` |

Values are this repository's own parser reading `pulse-psp-usa.chd`, which is
why the whole block is now `oag_tables::weapons::LeachBeamStats`. Confidence
**90** on the offset map - a direct decompile with no VFPU on the path, each arm
unambiguous.

**Two corrections to the prose above fall out.** `+0x11c` is `range`, so "call it
the beam's own maximum reach" was right about the meaning. And
`LeachBeam_Drain`'s `target+0x134 = <ActiveLeachBeamStats+0x110>`, which the
section above reads as `pending_source`, is **`slowShipFactor`** - not a source
id at all. See below for where it lands.

#### The two rate functions, read

```c
float LeachBeam_DrainRate(Instance *b) {          // 0x08872edc
    float mult = 1.0f;
    Stats *s = ActiveStats();
    if (b->first_drain /* +0x58 */) { mult = s->energy_multiplier; b->first_drain = 0; }
    return s->damage * mult;
}
float LeachBeam_RepairRate(Instance *b) {         // 0x08872f18
    float mult = 1.0f;
    Stats *s = ActiveStats();
    if (b->first_repair /* +0x59 */) { mult = s->energy_multiplier; b->first_repair = 0; }
    return s->repair * mult;
}
```

Both flags are set to `1` by `LeachBeam_InitLocked` and consumed once each, so
the **first** draining tick moves `0.1 * 50 = 5.0` energy each way and every tick
after moves `0.1`. Neither function scales by `dt`, and neither does
`LeachBeam_Drain` - the original's transfer is per *frame*, and therefore
frame-rate dependent. Confidence **88**.

#### Both accumulator consumers, found - the page's largest open item

`Ship_ApplyPendingWeaponDamage` (`0x0883f13c`, already named on
`shield-pickup.md`) is `entity+0x120`'s consumer:

```c
if (craft->pending_damage /* +0x120 */ > 0.0f && Ship_State(entity) == 1) {
    if (!(craft->flags /* +0x1b8 */ & 0x10)) {                 // no shield pickup up
        Ship_Damage(craft->pending_damage, entity, 2, craft->kind /* +0x138 */, craft->+0x124);
        if (craft->kind == 7)                                   // the LeachBeam's own tag
            handling /* craft+0x94 */ ->+0x31c = craft->+0x134;  // = slowShipFactor
    } else {
        ShipShield_Hit(...);
    }
    craft->pending_damage = 0.0f;
}
```

`Ship_ApplyPendingWeaponRepair` (`0x0883f228`, named here) is `entity+0x128`'s,
and it is four lines:

```c
if (craft->pending_repair /* +0x128 */ > 0.0f) {
    Ship_AddShield(craft->pending_repair, entity);   // 0x0883ddc8, already named
    craft->pending_repair = 0.0f;
}
```

`Ship_AddShield` is `Ship_SetShield(Ship_Shield(entity) + amount)`. So **the
victim's energy leaves through the ordinary `Ship_Damage` path a fired Shield
pickup swallows, and the shooter's arrives straight in its own pool** - no
separate mechanic on either side. Both are called from `FUN_0883f540`
(`0x0883f540`), the per-craft weapon update, in the order `FUN_0883efb4` ->
`Ship_ApplyPendingWeaponDamage` -> `Ship_ApplyPendingWeaponRepair` ->
`Ship_ApplyCollisionImpulse`. Confidence **85**.

**`slowShipFactor` is why this block authors no `slowdown_time`, and the absence
is design.** `craft+0x31c` is not a slowdown at all: `engine.md`'s
`Ship_UpdateEngine` reading has it as a **one-shot thrust scale** -
`if (craft+0x31c < 1.0) { T *= craft+0x31c; craft+0x31c = 1.0 }` - re-armed by
the block above on every tick the beam drains. A craft under a beam is throttled
to 80 % (Race) or 50 % (Eliminator) of its thrust for exactly as long as the link
holds, rather than charged a fixed number of seconds on impact the way every
other weapon is. **Ported 2026-09-16**: `oag_gameplay::projectile::leach_beam::Beam::drain`
arms `Ship::pending_thrust_scale` under the same three gates (positive amount,
racing, no Shield pickup), the composition root hands it to
`oag_physics::Environment::thrust_scale` at the next step, and
`oag_physics::engine::engine` applies it after the doubling on both branches -
`crates/game/src/race/tests/weapons.rs`,
`a_leach_beams_victim_is_throttled_by_the_authored_factor`, races two grids
and measures the difference.

#### The lifetime, and the beam's own geometry: `LeachBeam_Advance` (`0x08873fa0`)

The section above lists `func_0x0006ffa0` as unread; correctly rebased it is
`0x08873fa0`, the per-tick advance. It counts `instance+0x134` (**the age**) up
by `dt` and **returns `age < ActiveStats->active_time`** - and that return is the
flag `LeachBeam_UpdatePool` uses to decide whether to keep the link. So the
beam's lifetime is the authored `active_time`. Confidence **88**.

**This also closes "the 32-entry `0x30`-byte transform chain's exact use".** The
same function builds the beam as a **segmented ribbon** from the shooter's node
to the target's, `ceil((6.0 / range) * min(distance, range) * 6.0)` segments
long, each vertex displaced sideways on two mutually perpendicular axes by
`sin((i + 1) * step)` times a per-index random amplitude drawn at construction -
a lightning arc, not a straight bolt. It matches the alternating `(0,0)/(0,1)`
and `(1,0)/(1,1)` pairs `LeachBeam_InitLocked` zero-fills the chain with, which
are a strip's UV columns. Confidence **80**; the draw call itself was not
followed.

#### The drain is pulsed, and the four small helpers are read

```c
float LeachBeam_PulseStrength(Instance *b) {      // 0x08873020
    float t = b->age - b->last_pulse /* +0x138, initialised to -1.0 */;
    return (0.0f <= t && t <= 1.0f) ? t * 2.0f : 0.0f;
}
void LeachBeam_MarkPulse(Instance *b) {           // 0x088732f8
    if (b->age - b->last_pulse > 1.0f) b->last_pulse = b->age;
}
void LeachBeam_MarkDisconnected(Instance *b) {    // 0x08873090
    b->disconnected /* +0x144 */ = 1;  b->disconnected_at /* +0x148 */ = b->age;
}
bool LeachBeam_LingerExpired(Instance *b) {       // 0x088730a4
    return b->disconnected_at + 0.5f < b->age;
}
bool LeachBeam_UnlockedExpired(Instance *b) {     // 0x08873068
    return 0.75f < b->age;
}
```

`LeachBeam_UpdatePool` calls `LeachBeam_Drain` only while `PulseStrength > 0`.
`LeachBeam_MarkPulse` runs inside `LeachBeam_Advance`'s **pulse block**, which
re-enters each time the ribbon's scroll cursor (`+0xa8`) wraps to zero - and that
same block plays the `LEACHENERGY` cue and re-spawns `WO_LEACHBEAM_ENERGY` at the
target. So the beam pulses about once a second and drains for the second that
follows each pulse, which over a three-second beam is all but a scattering of
ticks. Confidence **80** on "effectively continuous"; the exact tick pattern
depends on the ribbon's own segment count, which varies with the two craft's
separation.

The two linger constants are engine literals, not attributes: **0.5 s** between a
disconnect and the retire, and **0.75 s** for an unlocked (`kind = 2`) beam,
which does nothing whatsoever before it expires.

#### Assets and cues - checked, not assumed

Three cues, each read as a string-pointer argument in a decompile rather than
inferred from a plausible name:

- **`LEACH`** - `LeachBeam_InitLocked`, one-shot on the shooter's emitter.
- **`_LEACHATTACH`** - `LeachBeam_InitLocked`, held, on a `SoundEmitter_Init`
  emitter the constructor allocates at `+0x4c` with a `600.0` falloff, carried by
  the beam itself.
- **`LEACHENERGY`** - `LeachBeam_Advance`'s pulse block, falloff `300.0`.

Two authored effects, both spawned through the ordinary `Psys_Spawn_q` shape:

- **`WO_LEACHBEAM_CHARGING`**, fourcc `0x43424c53` (`'SLBC'`) - spawned by
  `FUN_0883f540` on the *holder's* own node whenever `craft+0x1bc == 10` (the
  LeachBeam's held-weapon id) and `Ship_State == 1`, and despawned the moment
  either stops being true. **A player sees this while merely carrying the
  pickup**, before firing anything.
- **`WO_LEACHBEAM_ENERGY`**, fourcc `0x4542454c` (`'LEBE'`) - spawned at the
  *target's* node by the pulse block, roughly once a second.

`FUN_0883f540` also confirms outright that **`Ship_AcquireLock` runs for held
weapon ids `1` and `10` and no others** - the Missile and the LeachBeam - which
is independent confirmation of "victim selection is the Missile's own lock-on,
reused whole" above.

### Open

- **Which draw call consumes the ribbon.** The chain's contents and its segment
  count are read; what submits them is not.
- **`FUN_08872e64`** (the per-instance teardown `LeachBeam_UpdatePool` calls on
  retire) and **`FUN_08862d4c`** (the `Ship_IsValid`-shaped predicate
  `LeachBeam_Drain` gates both halves on) are unread past their call sites and
  are not in `names.tsv`.
- **The per-craft readout slot** `LeachBeam_UpdatePool` writes `PulseStrength`
  into - `**(float **)(pool + owner * 4 + 0x44)`, i.e. offset zero of the owner's
  own entity - is not identified. A HUD or audio amplitude is the obvious guess
  and it is only a guess.

## What is buildable now and what still is not

- **The Cannon is buildable, end to end.** Its whole fire-rate mechanism - the
  reload countdown gated on `craft+0x1bc == 3`, the round/rate attributes, the
  twin-muzzle alternation, the craft-speed-inherited round, the round-robin
  spawn pool - is read at instruction level, and so, as of 2026-09-09, is its
  base speed (`Cannon_BaseSpeedKmh`, a flat `500.0`) and its own hit/collision
  behaviour: a world hit throws `WO_CANNON_SPARKS` and stops the round, a
  craft hit applies `damage_per_bullet`/`slowdown_time` and stops it without a
  spark, and anything else bounces. See "`Cannon_UpdateRound` read" above for
  all three.
- **The Quake is buildable for its hit/damage/slowdown half**, which reuses the
  Missile's and Mine/Bomb's own shared pending-hit channel outright - nothing
  new to build there beyond wiring the Quake's own `damage`/`slowdown_time` and
  the self-exclusion/shield checks this page reads. **The travelling half is
  now buildable too, found 2026-09-07**: `Quake_Update` (`0x0891d268`) advances
  the wave's spline `t` every tick at a fixed engine speed (`270.0`, not
  authored) and drives `Quake_SampleSpan` (`0x0891c028`) to place and scale the
  disc's own `WO_QUAKE` particle effect and a travelling positional sound along
  it - see the new section above. **The per-craft latch that flags "this craft
  is currently under the wave" (`entity+0x860 & 0x40`'s setter) is now found
  too, 2026-09-07** - see "The latch setter, found 2026-09-07" above - so the
  hit-timing half and the travelling-visual half are wireable to each other,
  through the Quake's own authored `radius`. **It does not need
  track deformation of any kind** - the visual is an authored effect
  re-transformed every frame, not a mesh or vertex write, which is a stronger
  version of the same conclusion this page reached before the per-frame update
  was found.
- ~~**The LeachBeam is buildable for target selection and the connect/disconnect
  gate** … **but not for the actual drain amount**~~. **The LeachBeam is
  buildable in full, 2026-09-08, and is built** - see the section above. The
  drain amount is the authored `damage`/`repair` per tick with a one-shot
  `energy_multiplier`; both accumulators' consumers are found
  (`Ship_ApplyPendingWeaponDamage` and `Ship_ApplyPendingWeaponRepair`); the
  lifetime is the authored `active_time`; and the whole nine-attribute
  `<Stats>` block is decoded as `oag_tables::weapons::LeachBeamStats`. The one
  half that waited - `slowShipFactor`, the one-shot thrust scale at
  `craft+0x31c` - is wired since 2026-09-16; see "`slowShipFactor` is why this
  block authors no `slowdown_time`" above for the port.

## History

This page supersedes the three addresses `weapons-eight-of-thirteen-the-plasma-and-the.md`'s
2026-09-07 entry named as unread: `0x088537ac` (refuted, see above),
`0x0886c600` and `0x08866658` (both now read in full). See that thread for the
narrower, single-session account of how this pass came about.
