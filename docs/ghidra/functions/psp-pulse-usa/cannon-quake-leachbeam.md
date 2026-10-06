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
| `0x088734d0` | `LeachBeam_KeepInTrack` | 92 (new 2026-09-23, re-read 2026-09-30, measured 2026-10-01) |
| `0x08873328` | `LeachBeam_ReaimChain` | 92 (new 2026-09-23, re-read 2026-09-30, measured 2026-10-01) |
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

**Implementation status.** `oag_raceplay::weapons` fires AI cannons on the
recovered gate above, but the *value* of the byte is undefined behaviour and so
cannot be measured: treating it as non-zero for every AI craft is **chosen, not
measured, and carries no confidence score**. The mechanism is recovered; the
value it terminates in is not a fact about the disc.

**2026-09-23: `CANNON`, `CANNONEXPLWALL` and `CANNONEXPLSHIP` wired.**
`Cue::Cannon`, `Cue::CannonHitWall` and `Cue::CannonHitShip` are in
`crates/sound/src/sfx/cue.rs`. `Cue::Cannon` fires from
`advance_one_cannon` only when a round actually spawns, on a **chosen**
emitter - `CANNON`'s own call site names no argument. The wall/craft split
reads `Impact::struck`, the same field the Rocket's own pair already routes
on; a round that outlives its own flight time is silent by construction, the
same reasoning `rocket-visuals.md`'s "Audio, in passing" note gives for the
Rocket's timeout. **`CANNONEXPLSHIP` itself is a child reference, not an
empty cue**: read against a real disc, its one command is opcode `0x05`
indexing `CANNONEXPLWALL` directly - see
`crates/formats/src/sblk/child.rs`'s `cue_tree_sounds` doc comment - so a
craft hit plays the wall cue's own nine waveforms rather than a set of its
own.

**2026-09-25: the timeout reap is genuinely silent here too, confirmed the
same way as the Rocket's.** `CannonPool_Update` (`0x088582b0`, confidence 88
- read whole via `decompile_function` against `psp-pulse-usa`'s `BOOT.BIN`)
has the same outcome, though the gate is shaped slightly differently from the
Rocket's: the despawn pass enters its teardown block on
`if (1.0 < age || (flags & 4) != 0)` - the age test (the Cannon's own,
shorter, flight timeout) is an **`||` on the gate itself**, not a bit the age
check sets first the way `RocketPool_Update` does. Once inside, it is the
same shape as the Rocket: `Sound_Play` only ever branches on `flags & 0x10`
(`CANNONEXPLWALL`) versus `flags & 0x20` (`CANNONEXPLSHIP`) - no third arm.
A round that ages out with neither flag set (no hit recorded) still enters
the teardown block on the `1.0 < age` half of the `||`, but falls through
both `Sound_Play` branches and plays nothing. Confirms, rather than just
mirrors by reasoning, the "silent by construction" note above and the
equivalent Rocket finding in `rocket-visuals.md`'s "What a rocket hit spends"
section.

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

`oag_raceplay::CANNON_MODEL_ENTRY` is the mesh above, loaded the same way the
Rocket's, Mine's and Bomb's are, and `Race::cannon_model_matrices` puts one
matrix on each live round. **The mesh is named `muzzleflash` and is the bolt**:
`the_discs_cannon_round_carries_its_own_model` measures it at 20 vertices, 18
triangles, spanning `1.200 x 1.200 x 3.599` - a dart, longest along the +Z the
matrix aims down the velocity, which is not the shape of a flash at a barrel.

~~**The two display lists are still not drawn**~~ - **both are drawn as of
2026-09-17**: see "The two hand-built quads' geometry, read - and drawn"
further down this page for the vertex-level recovery and
`oag_fx::weapon_quads` for the pipeline. "Which display list is the
bolt and which the flash: settled" below is what made the geometry read
possible to attribute correctly once it was found.

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

This is exactly what `oag_weapons::projectile::cannon::direct_hit` already
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
page and `oag_weapons::projectile::cannon::BASE_SPEED_KMH`'s doc comment
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
(`0x0891dad8`-`0x0891dbc0`). ~~**In the same block**, a call into
`AiTrack_LocatePosition` (`0x0887ce78`, already named) is made with the
midpoint slot as one argument (`0x0891da70`-`0x0891da90`) - but it runs
*before* the basis is built and none of the cross-product/normalize
instructions that build the basis consume its result, so **"orientation
refined by the track" is not what this reading supports**; the call's purpose
here is not established.~~ **Wrong, corrected 2026-10-06** (last section of this page): the call fills the record row 1 is read from. And, the first time a given wave instance's node id
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

**2026-09-24: what the `/ 50` scales, and what `WO_QUAKE.POB` draws.** The
scale goes into slot 1 of the seven-word block `0x088f443c` reads and
`ParticleSystem_SetScaleParams` (`0x088f44d8`) writes back - instance
`+0x2c`, the **emitter-extent co-factor**, not the severity that scales size
and speed. All three `WO_QUAKE` emitters are shape 1, a line emitter
(`ParticleSystem_EmitLine`, `0x088fcfec`) of extent `50` along the frame's
`X`, and the basis's first row is `normalize(B - A)` (`0x0891da08`), edge to
edge. So the fire spreads across the road at its authored size, which is
what the PPSSPP crest shows. One correction to the paragraph above: the
basis's second row is built from `sp+0xd0`, a word of the struct passed to
`AiTrack_LocatePosition` as `a1` - so the call's output *is* consumed if it
writes that struct; which of its fields lands there is not read, and our
renderer keeps world up for that row (chosen, not measured). The same branch's
`func_0x000ec0c0` is `ScreenFlash_Start` with kind `4`, an orange tint of
0.4 s. Evidence for all of it: [particle-system.md](particle-system.md),
"A particle is its sprite times its colour, and the Quake stretches its
emitter".

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
>
> **2026-09-23: wired.** `Cue::QuakeHit` is in
> `crates/sound/src/sfx/cue.rs`, fired on the rising edge of
> `oag_weapons::projectile::quake::Wave::hit`, snapshotted before
> `Race::advance_quake`'s own `apply_hits` call and compared after - and
> placed on the **struck** craft, settling this section's own naming
> ambiguity (`owner_craft_cue_slot`) in favour of the victim, the same craft
> every pending-damage field around it belongs to. **2026-09-25:
> `QUAKELAUNCH` wired too**, once `Ship_FireHeldWeapon` (`0x08844ae8`) was
> decompiled whole for the missile.md/autopilot.md conflict it settled - see
> `autopilot.md`'s own "`Ship_FireHeldWeapon` opens both cues" section for
> the switch body. It fires on the fire press itself, positional, local
> player only, ahead of `Race::spend_pickup`'s own Quake busy check.
> `~QUAKETRAVEL` alone stays unwired: its own cue-name literal is the one
> just above this note - explicitly never resolved to a disc string, twice
> over - so a same-named cue existing in the bank is not trusted as it.

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
- ~~**Orientation remains unestablished, now cross-checked rather than merely
  read once.** `Quake_Update`'s own `AiTrack_LocatePosition` call was
  independently re-derived this pass rather than only quoted from
  `decompile_function`'s text: its output record is written and then never
  read by anything that feeds the cross-product basis built two dozen lines
  later, confirming (not merely repeating) "orientation refined by the track
  is not what this reading supports". Nothing new narrows it.~~ **Wrong, corrected 2026-10-06** (last section of this page): the record feeds row 1.

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
(Superseded 2026-09-24: read in full, measured live and drawn - see "the
ripple itself" below.)

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

### 2026-09-24: the ripple itself, read at instruction level - a raised-cosine bump along the road's up axis

This closes the open half of the 2026-09-08 correction above: *what*
`Quake_UpdateSpan` (`0x0891cab8`) writes into the road, *which* vertices it
owns, and whether the craft rides it. Read with the Allegrex module, whose
VFPU prefix decode (`vpfxs [X,Y,X,Y]`, `vpfxs [-2,-2,-2,-2]`) is what makes
the profile legible; the decompiler's text renders the same block as
`vpfxs(5,5,5,5)` and is no help. Full disassembly and the scripts behind every
number below are scratch, not committed.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0891b600` | `Quake_SpanAmplitude` | 90 |
| `0x0891b654` | `Quake_SpanHalfWidth` | 90 |
| `0x0891beac` | `QuakeNode_Load` | 85 |
| `0x0891b67c` | `Quake_FixupSpan` | 88 |
| `0x0891bda8` | `QuakeNode_Create` | 75 |
| `0x08b33040` | `g_quake_span_table` (data) | 90 |
| `0x08a88540` | `g_quake_peak_amplitude` (data) | 88 |
| `0x08a7ccac` | `g_quake_launch_lead` (data) | 80 |

#### The span table is a `.vex` node: class `0x3c7`, `Quake`

`g_quake_span_table` is written in exactly one place, `QuakeNode_Load`
(`0x0891beac`, the only function with a `WRITE` xref to it). That function
is slot 6 of the vtable at `0x08ad1960` (`psp-relocate.py xrefs 0x0891beac`
finds the one word, at `0x08ad1988`), beside `Quake_Destruct` and the factory
`QuakeNode_Create` (`0x0891bda8`, which allocates the `0x50`-byte object,
attaches it to its parent node and calls `Quake_Construct`). It takes a
loader cursor and a node, points the global at the cursor, advances the
cursor by the node's own data size, and runs `Quake_FixupSpan` over every
record. So the table is not built at runtime from anything - **it is the
payload of the track file's own `Quake` node** (`.vex` class `0x3c7`, already
in [vex.md](../../../formats/vex.md)'s effects row), one per circuit, and it
can be read straight off the disc:

```text
payload +0x00  u32  record count           (318 on 01_Track)
        +0x04  u32  live count, runtime    (0 on disc)
        +0x08  u32  1 on disc
        +0x0c  u32  record base, runtime   (0 on disc; fixed up to +0x10)
        +0x10  records, 0x80 bytes each

record  +0x00  f32[4]  A: the span's "down" axis at its start, mesh-local
        +0x10  f32[4]  B: the same at its end
        +0x20  f32[2]  forward neighbours' origins, in this span's distance
        +0x28  f32[2]  backward neighbours' gaps (see below)
        +0x30  u16[2]  forward neighbour indices, 0xffff = none
        +0x34  u16[2]  backward neighbour indices
        +0x38  u16     fixed-up flag, 0 on disc
        +0x3a  u16     vertex stride
        +0x3c  u16     parameter stride (4 on every record measured)
        +0x3e  u16     this record's own index
        +0x40  f32     length, world units
        +0x44  i32     vertex positions, self-relative -> pointer at load
        +0x48  i32     per-vertex parameters, self-relative -> pointer
        +0x4c  i32     the GE batch header, self-relative -> pointer
        +0x50  u16     vertex count
        +0x52  i16     segment: the AiTrack path index
        +0x54  f32     t_start, normalised along that path
        +0x58  f32     t_end
        +0x5c..+0x7f   runtime state, zero on disc (fields as 2026-09-08)
```

`Quake_FixupSpan` (`0x0891b67c`) is the whole of the load-time work: guarded
by `+0x38`, it turns the three self-relative offsets into pointers
(`*(p) += p`) and replaces every per-vertex parameter whose bit pattern is
`0xffffffff` with `1.0e8` (`lui 0x4cbe; ori 0xbc20`) - a sentinel that clamps
that vertex out of every bump, so it never moves.

**Measured across every Pulse circuit file on the disc** (the twelve
`track.vex` and twelve `track_reversed.vex`, 9,226 records, 1,306,295
vertices):

- **Every record is exactly one whole GE batch.** `+0x4c` lands on a batch
  header, `+0x44` on the position field of that batch's first vertex (offset
  8 at stride 14, offset 14 at stride 20 - the GE's position-last order), and
  `+0x50` equals the batch's own vertex count, on 9,226 of 9,226.
- **Every batch is render geometry.** 9,148 sit in `Mesh` (`0x125`) node
  payloads; the other 78, all on `16_Track`/`16_Track` reversed, sit in
  `Speedup Pad` (`0x3bd`) and `Weapon Pad` (`0x3be`) payloads, which
  [pads.md](../../../formats/pads.md) already shows *are* mesh payloads. None
  lands in any collision node.
- **`len / (t_end - t_start)` is one constant per segment**, to 0.001 in
  2,500: the path's own length. 01_Track's two paths read 2581.8 and 1583.3.
- **The neighbour links are distance-exact.** A forward neighbour's origin is
  at `+0x20[k]` in this span's own distance; a backward neighbour ends at
  `len + +0x28[k]`. Checked against `t_start * path length` on every
  same-path link: worst residual 0.01 units, on all 24 files.
- **One connected graph per file**, with a single forward and backward link
  per record except the one pair at each split on 05, 07 and 14.
- 106,877 of the 1,306,295 vertex parameters are the `-1` sentinel.

Confidence **92** for the layout (a file walk and a loader read agree on every
field that has both), **95** for "one record, one batch".

#### What `Quake_UpdateSpan` writes

Past the ageing already read on 2026-09-08, the first half is scalar:

```text
prev_pos = pos;  prev_amp = amp;  prev_w = w          // +0x60, +0x68, +0x70
age += dt
pos += dir * dt / len                                  // +0x5c, span fraction
amp  = Quake_SpanAmplitude(age + dt)                   // +0x64
w    = Quake_SpanHalfWidth(age + dt)                   // +0x6c
if retiring:   amp = 0
if first frame: prev_amp = 0
```

The two helpers are leaf functions, read whole:

```text
Quake_SpanAmplitude(t):  peak = *(0x08a88540) = 12.0
    t < 0.3  ->  peak * t / 0.3                        // rise
    else     ->  peak - peak * (t - 0.3) / (5.0 - 0.3) // linear fall, 0 at 5.0
Quake_SpanHalfWidth(t):  t * 50.0 / 5.0 + 25.0         // 25 -> 75 over the life
```

`0x08a88540` is confirmed by relocation, not by reading the `lui`/`lwc1`
pair at face value: `psp-relocate.py resolve 0x0891b60c 0x0891b61c` gives
`0x08a88540` for both. The four bytes there are `0x41400000`, `12.0f`, and the
next twelve spell `~QUAKETRAVEL`. That string sitting beside the constant is
a lead on the travelling cue this page twice failed to resolve from
`Quake_Update`'s side, **not** a resolution of it: whether `Quake_Update`'s
own cue argument points here was not checked.

Then the vertex block (`0x0891cddc`-`0x0891d1f4`), which runs only while the
bump overlaps the span (`-m < pos < m + 1` for either the new or the previous
position, `m` the larger half-width as a span fraction). The setup:

```text
C700 = A                                   lv.q  0x00(s1)
C710 = B - A                               lv.q  0x10(s1); vsub.q
C600 = [pos, prev_pos, pos, prev_pos]      vpfxs [X,Y,X,Y]
C610 = 2 * [len/w, len/prev_w, ...]        vpfxs [X,Y,X,Y]; vadd.q C610,C610,C610
C620 = [amp, prev_amp, ...] * 0.5 * 32767 / batch_scale
C630 = [1, 1, 1, 1]                        vpfxs [1,1,1,1]
C720 = [-2, ...],  C730 = [2, ...]         vpfxs [-2,...], vpfxs [2,...]
```

and per vertex (unrolled by four when the parameter stride is 4):

```text
p      = the vertex's parameter                         lv.s; vpfxs [X,X,Y,Y]
x      = clamp((p - [pos, prev_pos]) * C610, -2, 2)     vsub/vmul/vmax/vmin
h      = (cos(x * pi/2) + 1) * C620                     vcos.q; vadd.q; vmul.q
dir    = A + (B - A) * p                                vpfxt [X]; vmul; vadd
vertex = vertex - round(dir * h_new) + round(dir * h_prev)   vf2in/vi2f; vsub; vadd
```

`vcos` is the VFPU's quarter-turn cosine, `cos(x * pi/2)`, so over
`x in [-2, 2]` the factor `(cos + 1) / 2` runs 0 -> 1 -> 0. Put back into
world units, **the height a vertex is moved by is**

```text
H(d) = amp(age) * (1 + cos(pi * d / w(age))) / 2   for |d| < w(age), else 0
d    = p * len - pos * len    (distance along the span from the bump's centre)
```

**a raised-cosine (Hann) bump, peak 12.0 units at 0.3 s, full width `2w`
growing from 50 to 150 units, centred on the wave and moving with it at
270 units a second.** The displacement is along `-lerp(A, B, p)`: A and B are
the track's own "down" axis at the span's two ends (exactly `(0, -1, 0)` on
level road, and [track.md](../../../formats/track.md) pins `+y` as up), so the
road rises, along its own surface normal where it banks. **Not every span is
road.** Barrier and bank spans author a "down" that is nearly horizontal -
`(0.43, 0.13, -0.89)` on `01_Track` - and a handful on `02_Track` and
`16_Track` author one pointing up, so the original moves those sideways or
down exactly as their data says; the direction is always taken in the batch's
own space, before its node's transform. (An earlier draft of this paragraph
guessed the node transform flips those; `level_road_rises_in_world_space` in
`crates/game/tests/quake_ripple_ground_truth.rs` shows it does not - only
spans authored level are held to rising.)

The write is **incremental and exact**: each frame subtracts last frame's
displacement and adds this frame's, each rounded to whole `short` units before
the subtraction, so the two cancel bit for bit. A vertex therefore always sits
at its authored position plus this frame's bump, and returns exactly to its
authored position when the span retires (amplitude forced to 0). The
coordinate conversion `32767 / batch_scale` is the inverse of the decoder's
`s16 / 32768 * scale`; the one-part-in-32768 difference is the original's.

Confidence **88** for the profile (every constant is an immediate or a
relocated load, every VFPU prefix decoded by the module, and the arithmetic
closes: the new and old terms are computed by identical instructions, which is
what makes the incremental write exact). Confidence **85** that A/B are the
span-end "down" axes: that reading is the data's (level spans hold
`(0, -1, 0)`), not a consumer's.

**`age + dt`, not `age`.** Both helpers are handed the age *after* this
frame's increment plus one more `dt` - a one-frame lead. Negligible at 60 Hz
(0.4 units of amplitude at the steepest point), recorded because it is what
the code does.

#### How a bump reaches the neighbours: `Quake_Init`'s arm and the two propagators

`Quake_Init` locates the firing craft's path-normalised position
(`(progress - first.progress) / (last.progress - first.progress)`, read off
the path's own control points), takes the **first** record in table order
whose segment matches and whose window contains it, and arms it with
`pos = (t - t_start) / (t_end - t_start) +/- 15.0 / len` -
`g_quake_launch_lead` (`0x08a7ccac`, `0x41700000`, the literal this page's
older reading called "a fixed epoch") is a 15-unit lead in the direction of
travel. `0x08a7cca8` beside it is the `270.0` already recorded.

`Quake_PropagateForward` arms a forward neighbour once the bump's leading
edge (`pos * len > +0x20[k] - w`) reaches the neighbour's origin, and hands it
`pos = (pos * len - +0x20[k]) / neighbour.len`; `Quake_PropagateBackward`
does the same toward a backward neighbour's end, with
`pos = (pos * len - len - +0x28[k] + neighbour.len) / neighbour.len`. Both
use `Quake_SpanHalfWidth(age)`, and both set the neighbour's skip byte `+0x7f`
when its index is higher than the parent's and the wave is past its first
frame, so the update loop does not advance it twice in one frame. Confidence
**85**, all instruction-level.

Because every link is distance-exact (above), the whole machine is, to first
order, **one bump at one distance along the track, drawn into every span it
overlaps.** A Python port of the four functions, run against the disc's own
tables (01_Track and 16_Track, several launch points, both directions),
agrees with that stateless reading in which spans ripple, and differs only by
a per-span phase skew of whole frames: a span advanced once too often or
once too few by the update order lands 4.5 units (one frame at 270/s) ahead
or behind, occasionally 9-22 units on 16_Track. **Whether the original shows
that skew as a visible step between road batches is unmeasured** - the port
is of the reading, not of a capture.

#### The craft does not ride it

Nothing the floor query reads is touched:

1. `Quake_UpdateSpan` stores only to its own record and to the `short3`
   positions behind `+0x44` (`sh` at `0x0891d09c`-`0x0891d0d8` and
   `0x0891d1c4`-`0x0891d1d0`; there is no other store in the function).
2. Every `+0x44` on the disc, 9,226 of them, lands inside a `Mesh` or pad
   payload's GE batch - render geometry. None lands in a `Floor Collision`
   (`0x3b9`), `Wall Collision` (`0x3ba`), `Mag Floor Collision` (`0x3e6`) or
   `Reset Collision` (`0x3cd`) payload.
3. The collision the hover probes cast against is a separate indexed soup of
   `f32` positions, parsed by `CollisionNode_ParseChunks` from those nodes'
   own payloads ([collision.md](collision.md): "not the render mesh"). A
   routine that writes `s16` into a GE batch cannot reach it.
4. The one piece of gameplay built from a rippling batch, a pad's trigger
   volume, is the pad mesh's bounding box copied **at bind**
   ([pads.md](../../../formats/pads.md)), not re-read from its vertices, so a
   Quake rolling over a `16_Track` pad does not move its trigger.

**Render-only, confidence 85.** What would lower it: a second consumer of the
batch vertices that this page has not found. What would raise it: a PPSSPP
read of a craft's floor height while the bump passes under it.

The damage path is unchanged by any of this and still does not read geometry
- but it is worth knowing that `Quake_SpanIntensityAt_q` reads the same
`+0x5c`/`+0x64`/`+0x6c` this section names (position, amplitude, half-width),
so the original's hit test is very probably "how high is the bump under this
craft", compared against `0.1` after smoothing. This project's hit uses the
authored `radius` instead; matching the original there would move the golden
hashes and is left as its own change.

#### Measured live on PPSSPP, 2026-09-24

Everything above was then measured against the running game. PPSSPP v1.20.4,
UCUS98712, a private silent instance; single race, Venom, Talon's Junction
(`16_Track`). The Quake was fired from the player's own weapon record by
setting bit `0x8` of `+0x1b8` while stepping (`scripts/psp-fire-weapon.py
quake`, which gained the entry in this pass). A breakpoint on
`Quake_UpdateSpans` read the whole table at `*(g_quake_span_table)` every
frame for 290 frames, and compared every armed record's batch against the
disc's own shorts.

| Quantity | Read statically | Measured |
| --- | --- | --- |
| every vertex of every armed span, every frame | `v0 - sum round(-lerp(A,B,p) * H * 32767/scale)` | **0 shorts of error** |
| amplitude `+0x64` | `12 t/0.3`, then `12 (1 - (t - 0.3)/4.7)`, `t = age + dt` | max error 0.00000 over 4,523 samples |
| half-width `+0x6c` | `25 + 10 t` | max error 0.00000 |
| advance of `+0x5c` | 270 units a second | 269.996 to 270.004 |
| peak rise in world `+y` | the amplitude | 11.941 against 11.956, 10.251 against 10.296 |
| lifetime | 5.0 s | last armed frame at age 4.788; `Quake_UpdateSpans` stops being called once `live == 0` |

Two things the static read did not say:

- **A batch can belong to two or three span records, and they add.** At a
  path junction one GE batch is owned by one record per path - on `16_Track`
  records 223/224, 228/229, 230/231, 349/350, 351/352 and 354/355; 18 such
  batches on `01_Track`, 42 on `05_Track`, 54 on `07_Track`. Every owner
  writes its own incremental displacement into the same vertices, so the live
  vertex is its authored position less the **sum** over its armed owners.
  Comparing per record gave 220-short "errors" on exactly those records;
  summing gives zero.
- **`Quake_Init` arms about 20 records before the first update**, all at age
  0 and amplitude 0: the launch span's own `Quake_ArmSpan` call updates it
  with `dt = 0`, which propagates, recursively, to everything the launch-width
  bump already overlaps. 115 records were armed over the wave's life; the live
  count ran 10 to 41 a frame.

**The craft does not ride it, measured.** With the player settled on the grid,
an AI's Quake was fired from 250 units behind it. The road vertex 0.31 units
under the craft rose to **+10.80** at the wave's peak; the craft's body did
not rise at all, and dipped by up to 1.24 units only from the frame the hit
landed (energy 100 % to 88 %), which is the hit, not a floor. At its peak the
drawn road stood about 7 units above the craft's own body, and the frame shows
the road sheet passing *through* the craft. Confidence **92** for render-only:
the static argument above and a live read agree.

**What a player sees** (the run's own frames, at the size a player sees them):
a strong orange-white tint over the whole screen for the first eighth of a
second, while the road just ahead starts to lift; then one smooth, full-width
hump that rolls **forward only**, 50 to 90 units ahead, the kerbs and barriers
bending up with it, fire and dark debris (`WO_QUAKE`) on its crest, tall
enough near its peak to hide the road behind it. By a second it has passed the
start gantry and shrinks with distance; the tint has gone. The screen tint is
`Quake_Update`'s kind-4 `ScreenFlash_Start`, re-started every frame the wave
runs - identified 2026-09-24, see
[particle-system.md](particle-system.md#the-screen-flashs-consumer-read-and-measured-2026-09-24).

Profile confidence **95** (instruction-level read and a zero-error live
match). The per-span one-frame skew the Python port predicted is still
unexamined: the live comparison used each record's own position, which would
absorb it.

#### What this project draws, 2026-09-24

The ripple is drawn, render-only, and the simulation still never learns a
renderer exists:

- `oag_vex::quake` decodes the `Quake` node's table, and
  `crates/vex/tests/quake_ground_truth.rs` holds all 9,226 records of the 24
  circuit files to "one record, one whole batch" and to distance-exact links.
- `oag_mesh::mesh::batch_placements` says which vertices of a built model
  each batch became; `oag_render::ripple` draws the bump into them, **summing
  the owners of a shared batch** as measured above, and restoring a batch to
  its authored vertices the frame the bump leaves it.
- `oag_raceplay::SpanPlaces` maps every vertex onto the course through its
  path's own `t` - the control points' authored `progress`, now read into
  `oag_vex::track::SplinePoint::progress` - so that neighbouring spans meet
  without a step. `crates/game/tests/quake_ripple_ground_truth.rs` holds every
  road vertex to a median 0.37 and p99 under 5.1 units of where it really is
  on the course, on all 24 files, and every seam in the table - driven paths
  and split branches alike - to 0.00 at p99 and 2.34 units at worst (at a
  path junction). The table's zero-length spans (`t_start == t_end`) carry
  link offsets against nothing and are left out of the seam check.
- The bump follows the simulation's own `Wave`: its `progress` plus the
  original's 15-unit launch lead, its `age` one tick late, as both helpers are
  handed. Nothing hashed changed; `race_ground_truth`'s lone-craft run is
  identical before and after.

Chosen, not measured, and labelled so in the code: the branch path of a split
(05, 07, 14) is placed as one function of its own `t`, pinned to the fork and
the merge by the table's links, since the course has no distance for a branch
it does not walk; and the original's update-order phase skew between spans is
not reproduced.

The screen tint is drawn since 2026-09-24 on Pulse's PSP source
(`oag_fx::flash`).

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
`HullOverlay_DrawLeachBeam` (`0x0890d828`, renamed from `FUN_0890d828`) and
`HullOverlay_SubmitLeachBeamBatched` (`0x0890e140`), and the first walks a
craft's own mesh chain and passes the texture to `HullOverlay_Submit`
(`0x0890e304`, renamed from `FUN_0890e304`) per sub-mesh, gated on a scalar
read through a shared state object's `+0x4c` field being positive. That is a
**surface overlay on the drained craft's hull**, the sibling of
`Data\Tex\Weapons\absorb_surface.*` (`DAT_08af2800`), which the shield's own
absorb pass uses in exactly the same shape - see "2026-09-17: the hull
overlay pair, read in full" below for the complete draw path, both textures.
Confidence **85** on the identification and on which thing it textures.

~~The ribbon chain `LeachBeam_InitLocked` zero-fills the UV columns of is
textured by something else, and that something else is still unlocated. The
ribbon's own texture is **open**.~~ **Wrong, corrected 2026-09-17**: the
ribbon's own texture is found, and it is not either of the two
`Data\Tex\Weapons\` strings above - see "2026-09-17: the LeachBeam ribbon's
own texture, and the draw call that proves it" under the LeachBeam section
below. ~~The two `Data\Tex\Weapons\` strings are the only LeachBeam-flavoured
texture paths in `.rodata`~~ was also wrong: a third string,
`Data\Weapons\Textures\pulse_leechbeam1_ADD.mip`, sits under the Cannon's
`Data\Weapons\Textures\` directory rather than `Data\Tex\Weapons\`, which is
why the `(?i)leach` search here (spelled "leach") missed it - the disc spells
it "lee**ch**".

~~Neither hull overlay is drawn by this engine~~ (the absorb one is, as of 2026-09-23 - see below), and both are honest absences
rather than stand-ins.

### 2026-09-17: the hull overlay pair, read in full - both textures, one draw routine

**Prompted by a from-play report**: the weapon-absorb effect does not appear
to play in any title here, only its `ABSORB` sound cue does. Reading the
draw path in full, for both `leachbeam_surface.mip` and `absorb_surface.mip`
together, since they share every function but the one texture constant.

**Four thin wrappers, one shared draw call, two dispatch sites.** The two
readers named above are each one of a **pair**:

| Wrapper | Texture | Shape |
| --- | --- | --- |
| `HullOverlay_DrawLeachBeam` (`0x0890d828`) | `DAT_08af2804` (`leachbeam_surface.mip`) | walks one entity's own sub-mesh chain |
| `HullOverlay_DrawAbsorb` (`0x0890d744`, renamed from `FUN_0890d744`) | `DAT_08af2800` (`absorb_surface.mip`) | walks one entity's own sub-mesh chain |
| `HullOverlay_SubmitLeachBeamBatched` (`0x0890e140`) | `DAT_08af2804` | one pre-resolved (entity, mask) pair from a shared render batch |
| `HullOverlay_SubmitAbsorbBatched` (`0x0890e120`, renamed from `FUN_0890e120`) | `DAT_08af2800` | one pre-resolved (entity, mask) pair from a shared render batch |

Confirmed at instruction level, not inferred from symmetry: the two
"batched" wrappers decompile to a bare `HullOverlay_Submit()` tail call with
no visible arguments, but their disassembly is `lui a3, 0x8af` /
`lw a3, 0x2804(a3)` (LeachBeam) and `lw a3, 0x2800(a3)` (Absorb) immediately
before the `jal` - each one injects only the texture into `a3`, leaving
`a0`/`a1`/`a2` (entity, param2, mask) and the float fade in `$f12` exactly as
its own caller already set them up. All four ultimately call
`HullOverlay_Submit` (`0x0890e304`), which is the entire drawn picture for
both overlays - confidence **88** on the four-wrapper structure, a direct
decompile and disassembly read with no VFPU trap on any of the four paths.

**Two dispatch sites, same two gates, same shared state object.** The
per-entity pair (`HullOverlay_DrawLeachBeam`/`HullOverlay_DrawAbsorb`) is
called from the tail of the generic per-entity mesh-draw function at
`0x0890f288` (not renamed - shared mesh machinery far outside this weapon's
scope), and the per-batch pair
(`HullOverlay_SubmitLeachBeamBatched`/`HullOverlay_SubmitAbsorbBatched`) from
the tail of `Mesh_DrawBatchSet` (`0x0893074c`, already named). Both dispatch
sites read from the **same kind of pointer** - `entity+0x74` at the
per-entity site, a batch-scoped `+0xc0` field at the per-batch site - which
this page calls the **shared overlay state** for want of a better name; its
own type, size and writer are not identified this pass (see Open, below).
Both sites gate identically:

- **LeachBeam**: `DAT_08b317ac == 2` (a small-integer global, meaning
  unidentified - plausibly a race-mode or race-state selector, since it is
  compared for equality against a literal rather than tested as a flag bit)
  **and** `**(float**)(state + 0x4c) > 0.0` - a **double** pointer
  dereference: `state+0x4c` is itself a pointer field, read again to reach
  the actual float. That double indirection is exactly the shape a "points
  at the live instance's own float, or null when there is none" field would
  have, and the natural candidate is the active `LeachBeam` instance's own
  age or `LeachBeam_PulseStrength` - not confirmed by a write site this
  pass. **Found 2026-09-23 (later)**: it is the firing craft's weapon record
  `+0`, written with `LeachBeam_PulseStrength`, and `DAT_08b317ac == 2` means
  "a beam is live". See "The LeachBeam overlay's fade source" below.
- **Absorb**: `HullOverlay_AbsorbWindowActive(state)` (`0x0883e904`, renamed
  from `FUN_0883e904`) **and** `HullOverlay_AbsorbFade(state) > 0.0`
  (`0x0883e950`, renamed from `FUN_0883e950`). Both read the identical pair
  of fields on the same `state` pointer - `state+0x830` minus `state+0x878`
  - against the identical bounds (`read_memory` on both `DAT_08a7b6a8`/
  `_DAT_08a7b6ac`: `0.0` and `1.0`), so the two functions are the same
  window test in a bool and a float shape:

  ```c
  bool HullOverlay_AbsorbWindowActive(State *state) {
      float elapsed = state->f0x830 - state->f0x878;
      return 0.0 <= elapsed && elapsed <= 1.0;
  }
  float HullOverlay_AbsorbFade(State *state) {
      float elapsed = state->f0x830 - state->f0x878;
      if (elapsed < 0.0 || elapsed > 1.0) return 0.0;
      return ((elapsed - 0.0) / (1.0 - 0.0)) * 2.0;   // = elapsed * 2.0
  }
  ```

  Confidence **88**: both are short, direct decompiles with matching
  literals read straight from `.rodata`, not derived from each other.
  `state+0x830`/`+0x878` read as an "age" and a "started-at" timestamp pair
  on the same object the LeachBeam's own pointer lives on, though which
  object that is remains open.

**The absorb fade's whole shape is now measured: a one-second, symmetric
triangle pulse.** `HullOverlay_AbsorbFade` returns `elapsed * 2.0`, a value
that runs `0..2` as `elapsed` runs `0..1` second. `HullOverlay_Submit` then
folds anything past the midpoint back down (`fVar11 = param_1; if (1.0 <=
param_1) fVar11 = 2.0 - param_1;`), so the alpha this produces is `0` at
`elapsed = 0`, ramps linearly to `1` at `elapsed = 0.5` s, and ramps linearly
back to `0` at `elapsed = 1.0` s - **half a second in, half a second out,
gone by one second**, whatever `state+0x878` marks as the start. Nothing
about the LeachBeam overlay's own fade *shape* was found this pass beyond
"some float, read through a pointer, greater than zero" - `HullOverlay_Submit`
applies the identical `param_1 >= 1.0 -> 2.0 - param_1` fold to whatever
value it is handed, so **if** the LeachBeam's own source value also runs
`0..2` over its lifetime the same triangle shape would apply to it too, but
that is inferred from the shared draw code rather than read from a LeachBeam-
side writer.

**`HullOverlay_Submit` (`0x0890e304`) itself, decompiled whole - the entire
picture for both overlays:**

1. **Colour is a flat grey-alpha tint, not a fixed colour with a fading
   alpha.** All four `Gu_Color` channels - R, G, B *and* A - are computed
   from the *same* expression, `fade * 255.0` (`fVar11` above, clamped by the
   caller's own `0.0 <` gate on one side and the `2.0 - param_1` fold on the
   other). So the overlay is literally `(a, a, a, a)`: it tints the texture
   grey as it fades in and brightens toward white as it nears full fade,
   with alpha rising in lock-step - not a white surface whose *only* fading
   channel is alpha. Confidence **85**, a direct read of four identical
   arithmetic sequences feeding one `Gu_Color` call.
2. **Depth test and fog are per-sub-mesh, not fixed for the whole overlay.**
   `Gu_DepthMask`/`Gu_DepthFunc` branch on bit `1` of the sub-mesh's own
   flags word (`*param_4`, the mask argument threaded through from the
   caller): ~~set, depth write on and `LEQUAL`; clear, depth write off and
   `LESS`~~ **corrected 2026-09-23**: `sceGuDepthMask(1)` *disables* writes
   and the function codes are the PSP's own (`2` `EQUAL`, `6` `GREATER`), so
   bit `1` clear - a batch in list A only - draws with `EQUAL` and writes on,
   exactly over the surface the hull drew, and bit `1` set - a list-B batch -
   with the engine's ordinary test (`6`) and writes off. See "2026-09-23"
   below. `Fog_Disable` unless the sub-mesh's own material flags
   (`*(byte*)(entity->something+0xc) & 8`) ask for it, in which case
   `0x0891eac0` (not read this pass) is called instead. Confidence **80** -
   read at instruction level, but what the two states mean for a given
   sub-mesh (which ones set which bit) was not traced back to the model
   data.
3. **~~Rotating~~ - corrected 2026-09-23, see below: a fixed quarter turn.
   The texture is bound in `GU_TEXTURE_MATRIX` UV mode, not the mesh's own
   baked UVs**, confirmed independently by `mesh-draw.md`'s own existing
   `Gu_TexMapMode` table (`0x0890e584`/`0x0890e76c`, both inside this
   function - mode `(1,0,0)` on entry, restored to `(0,0,0)` on exit). A
   4x4 projection matrix is built per draw from `cos`/`sin` of a **live,
   per-frame VFPU time register** (`vcst_s(5)`) at `pi/2` steps, matrix-
   multiplied against a fixed basis whose one cell is `fade * -1.5` - so the
   projection's own motion is coupled to the same fade value driving the
   colour, not a separate clock. Bound as texture matrix slot `3`
   (`Gu_SetMatrix(3, &matrix)`) for the one draw, restored after. This is a
   **rotating, self-projected surface treatment**, not a static decal -
   confidence **75**: the matrix construction and its inputs are a direct
   read, but the exact visual result (how fast it turns, what the basis
   matrix's other cells represent) was not derived by hand or checked
   against a captured frame.
4. **No `Gu_BlendFunc` call appears in `HullOverlay_Submit` itself - its
   blend state is set one call earlier, and it decodes clean.**
   `Gfx_BuildBatchStateList` (`0x0891f890`, already named) is called once
   per draw with a literal flag word, `0x282`, and its own state-index enum
   is already pinned project-wide by `mesh-draw.md`'s "PSP SDK" table (`0`
   `GU_ALPHA_TEST`, `3` `GU_STENCIL_TEST`, `4` `GU_BLEND`, `5`
   `GU_CULL_FACE`, `0x11` `GU_COLOR_TEST`), which this reading reuses rather
   than re-derives. Walking `Gfx_BuildBatchStateList` with `param_2 = 0x282`
   and its own `param_3 = 0` (both literal at this call site):
   `GU_CULL_FACE` **on**; `Gu_TexWrap(0, 0)` = **repeat** both axes (sensible
   for a UV matrix that can push the projected coordinate outside `0..1`);
   `GU_BLEND` **on**, `Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0,
   0xffffff)` - **the identical additive blend `exhaust::BLEND` and the
   LeachBeam ribbon's own `LeachBeam_SubmitStrip` both already use**, not a
   different equation; depth write **on**, depth test `LEQUAL`;
   `GU_ALPHA_TEST` on, `GREATER` against `0` (discards fully-transparent
   texels only); `GU_COLOR_TEST` on, discarding pure-black fragments (`ref =
   0`) - the same idiom `mesh-draw.md`'s own worked example names; and
   `GU_STENCIL_TEST` on, `ALWAYS`-pass with a `REPLACE` op - the same
   stencil shape `LeachBeam_SubmitStrip` uses to feed the bloom glow mask,
   so **both hull overlays write the glow mask too**, on top of everything
   else. Confidence **85**: every branch actually taken is a direct
   `0x282 & mask` evaluation against the SDK's own documented constants, not
   a guess; the untaken branches (a title or draw call passing a different
   flag word) were not chased.

~~**Still neither overlay is drawn by this engine**~~ (the absorb one is drawn as of 2026-09-23 - see below), but the read is now
complete enough to build from: geometry reuse (the model's own hull
sub-meshes, gated per sub-mesh by a bitmask), the grey-alpha tint, the
rotating `GU_TEXTURE_MATRIX` UV projection, the additive blend (recovered,
not chosen - identical to the ribbon's own), the stencil/glow-mask write,
and the absorb half's full 0.5 s-in/0.5 s-out triangle timing. What is
still open is narrower than it was: the shared state object's own identity
and writer, and the LeachBeam side's specific fade source (see Open,
below) - neither blocks a first build of the **absorb** overlay, which can
be triggered off this engine's own existing shield-hit event
(`Race::advance_leach_beam`'s own `shell.hit()` call is one such site, and
the shield pickup's own absorb path is the more general one) rather than
the unidentified `state+0x830`/`+0x878` pair. That trigger substitution
would be **chosen, not measured** - the same split every other module in
this codebase draws between a recovered draw and a chosen wire-up (see
`oag_fx::exhaust`'s own module doc comment for the shape).

### 2026-09-23: the absorb overlay's writer, its projection, and two corrections

Read for the weapon-absorb build (`oag_fx::hull_overlay`,
`oag_raceplay::absorb`).

- **The writer.** `+0x830` is the craft's own clock: `FUN_088418e0`, the
  per-craft update, opens with `lwc1 f13,0x830(a0); add.s f13,f13,f12;
  swc1 f13,0x830(a0)` (`0x088418e4..04`), with `f12` the frame's `dt`.
  `+0x878` has two writers, `Craft_Construct_q` (initialising it) and the
  absorb handler `FUN_08844ec4` at `0x088455ac..b4`: `lwc1 f12,0x830(s0)`
  then `swc1 f12,0x878(s0)` in the delay slot of the branch that skips
  `Ship_PlayAbsorbFeedback`. The overlay therefore starts on every
  **pickup** absorb, and `Ship_RefillLapShield`, which calls the feedback
  without that store, lights no hull. The "shared state object" above is
  the craft: the per-entity dispatch at `0x0890f288` reads it from
  `mesh+0x74` and gates on `mesh+0x79 != 0`, a per-mesh byte whose writer
  was not read. Confidence **88** for the stamp and the clock (direct
  instruction reads).
- **The projection is a fixed quarter turn, not a rotation.** The matrix
  code is `lv.s S000 <- pi/2; vcst.s S002, 2/PI; vmul.s S003, S002, S000;
  vcos.s S010, S003; vsin.s S012, S003`. `2/PI * pi/2 = 1`, and the VFPU's
  `vsin`/`vcos` take quarter turns, so this is `cos 90 = 0`, `sin 90 = 1`
  every frame. "A live, per-frame VFPU time register" above was wrong:
  `vcst` index 5 is the constant `2/PI`. The rows `(1,0,0,0)`,
  `(0,cos,sin,0)`, `(0,-sin,cos,0)`, `(0,0,0,1)` are multiplied (`vmmul.q
  E000, E100, E200`) by a second matrix with rows `(10,0,0,0)`, `(0,10,0,0)`,
  `(0,0,0,1)`, `(0,-1.5p,1,1)`. `10` is `DAT_08abf4a4` (`00002041`, one
  reader, no writer), `p` is the unfolded fade this function receives, and
  the last column is zeroed before `Gu_SetMatrix(3, ...)` (`0x0890e754`).
  Taking the `E`-operand product as `A * B` on the memory rows, a vertex
  `(x, y, z)` projects to `(s, t, q) = (10x, -10z - 1.5p, 1)`. The other
  operand order gives `q = 10y - 1.5p`, a perspective divide that means
  nothing for a hull, which is why this reading is taken. ~~**The operand order
  is an inference, not a measurement**~~ - **measured the same day**: the
  matrix handed to `Gu_SetMatrix` at `0x0890e754` during a real absorb gives
  exactly this projection; see "2026-09-23 (later)" below. The
  projection source is `GU_POSITION`. `Gu_TexProjMapMode`'s only caller,
  `FUN_0890dc64`, passes `0`, and the GE init zeroes the same context word,
  so `(x, y, z)` are the vertex's GE input coordinates: the `.vex`'s own
  `s16 / 32768`, before its batch scale. Result: a top-down planar
  projection, 10 repeats per input unit, sliding 3 repeats along the
  craft's length over the second. Confidence ~~**70**~~ **90** for the
  projection as a whole (measured live, below), **85** for the constants.
- **The primary colour reaches the fragment on the hull proper.**
  `HullOverlay_Submit` disables lighting (`Gu_Disable(10)`), which on the GE
  takes a vertex's own colour where the vertex format has one. Assegai's
  `shipShape`, airbrake and canopy batches are vertex type `0x0121` with no
  colour bits, so they take `Gu_Color`'s `(a, a, a, a)`. Only
  `self_illuminatedShape` and `glowingShape` (type `0x013d`, colour bits
  `7`) would take their own vertex colour. Measured with a throwaway batch
  dump of `Data\Ships\Assegai\Ship.vex`. Every Assegai mesh batch
  shares one scale, `37.088448`.

- **The per-mesh gate is the mesh's name** - **but only on the per-entity
  path, and the race does not take it**; see "2026-09-23 (later)" below: in
  play the overlay covers every mesh in the hull's batch set, measured.
  `Mesh_InitFromPayload` sets
  `mesh+0x79` (`0x0890ec30..4c`) from a match of the mesh's own name against
  the literal `"ship"` at `0x08a883c4`, via `FUN_089737ac`. The neighbouring
  literals `"track"`/`"TRACK"`/`"Track"` set `+0x78` the same way. It sets
  `mesh+0x74` (`0x0890ec28`) to the first node in the mesh's list at `+8`
  whose type token is `FUN_08a6bfc0`'s. That is the craft-entity class token
  `Race_CreatePlayer`, `Race_SpawnAiRacer` and `Craft_Construct_q` use. So
  on Assegai only `shipShape` can take the overlay: not the airbrakes, the
  canopy, or the glow and LOD meshes. Confidence **75**: `FUN_089737ac` is
  read as a substring test from its call shape, not decompiled.
- ~~**A controlled live probe finds the gate never evaluated in play.**~~
  **Voided** - that run armed two execution breakpoints at once, and PPSSPP
  v1.20.4 only fires the most recently added one (see
  [ppsspp-debugger.md](../../../reverse-engineering/ppsspp-debugger.md#only-the-most-recently-added-execution-breakpoint-fires)).
  The HUD alone calls the gate every frame, so a live race can never log
  zero hits of it. See "2026-09-23 (later)" below. The original text: On
  PPSSPP v1.20.4, in a single race on this project's own silent instance
  (debugger `47831`), one run armed breakpoints on the per-craft update
  `FUN_088418e0` (the control) and on `HullOverlay_AbsorbWindowActive`
  together. It collected 60 hits, all on the update (7-8 per craft across
  the eight craft) and none on the gate. Both overlay draw paths call that
  gate for any mesh that could take the overlay. A second run, without a
  control, saw neither `0x0890f288` nor `Mesh_DrawBatchSet` hit in 60 s. After writing `craft+0x878 = +0x830 - 0.5` on the first
  craft updated (the store read `-10.0` before the write, which is
  `Craft_Construct_q`'s initial value), `HullOverlay_Submit`'s
  `Gu_SetMatrix(3, ...)` at `0x0890e754` was not reached within 10 s. Both
  draw paths look like record-time functions whose lists are replayed
  without re-entering them. That does not rescue the overlay: a list
  recorded at load, with the stamp at `-10`, holds no overlay and a frozen
  `Gu_Color`, so it could not animate. **Nothing shows the original drawing
  this overlay in play**, and the `vmmul` order above is still unmeasured.
  The probe that would change that: give all eight entities a weapon
  (`*(entity+0x4c)+0x1bc = 1`; only the player absorbs on circle), then
  break on the stamp store `0x088455ac` as the control and on `0x0883e904`.
  If the stamp hits and the gate does not, the overlay is dead code in play.
  If both hit, turn it on.

**Built** ~~**and off by default**~~ - **on since "2026-09-23 (later)"
below** (`oag_fx::hull_overlay::DRAWN = true`), as
`oag_fx::hull_overlay`. The port and what it chose are in
that module's own doc comment. The depth test is `LessEqual` with no write,
standing in for `EQUAL`. ~~Every hull mesh is overlaid, since the `+0x79`
byte is unread~~ ~~- read the same day (above): only a mesh named
`...ship...` takes it, and the port follows that~~ - **measured later the
same day**: the race path overlays every list-0 mesh of the first level of
detail, and the port follows that (`hull_overlay::overlaid_meshes`). ~~All
five of the hull's lists go through the additive pipeline~~ - the overlay
now draws through its own blend, `hull_overlay::BLEND`, which writes the
glow mask the way the stencil does. The colour-bearing glow meshes take the
tint like the rest.
The LeachBeam half (`leachbeam_surface.mip`) would ride the same module,
but its fade source (`**(float**)(state+0x4c)`) is unread, so it is **not
wired**.

### 2026-09-23 (later): a real absorb draws the overlay in play

A second probe, on a real absorb, settles it: **the original draws the
absorb hull overlay in play, for exactly the one-second window.** The same
run measured the texture matrix, and it found that the in-race draw path
does not use the name gate above.

**Setup.** PPSSPP v1.20.4, `UCUS98712`, on a private silent instance
(Xvfb `:133`, debugger `47833`, `[Sound] Enable = False`,
`SDL_AUDIODRIVER=dummy`). Single race, Venom, Talon's Junction, Assegai, the
profile's own choices (`psp-drive.py menu --single-race`). The player sat
stationary on the grid after the countdown, with the chase camera.
- **The player's craft** is `*(g_race_manager + 0x2c0)`. That is the object
  `Hud_UpdateEnergyBar` hands to the gate every frame. It is not
  `Ship_UpdateCraft`'s `a0`, the trap the first probe hit. Its weapon record
  is `*(craft + 0x4c)`.
- **The absorb.** The weapon was granted by writing `0` into the record's
  `+0x1bc` (a memory write while stepping), and circle was then pressed
  through `input.buttons.press`. The absorb handler
  `Ship_AbsorbHeldPickup` (`0x08844ec4`, renamed from `FUN_08844ec4` in this
  pass; confidence 85: it reads the controller's absorb byte, adds the held
  pickup's shield value, clears the slot, stamps `+0x878` and calls
  `Ship_PlayAbsorbFeedback`, all read in its decompile and seen live) ran on
  that real input: the slot went back to `-1` and the stamp was written.
  **Its Eliminator path** (`g_game_mode` 8 or `0x12`, read 2026-10-03 off the
  disassembly, confidence 80, not live) pays no energy: it rewrites the held id
  to `5` (`0x088612e8`), calls `Weapon_RequestFire` (case 5 sets bit `0x20`,
  the Shield), skips `Ship_AddShield` in every arm, clears the slot and skips
  the feedback, so the press raises the mode's one-second Shield. See
  [race-modes.md](../../../gameplay/race-modes.md#eliminator).
- **The organic control.** An AI craft absorbed on its own during the
  baseline (nothing was cheated for it), and the same draw-side signal
  appeared for it, also for 0.97 s.

**Instrument: logged memory watchpoints only** (`enabled: False, log:
True`, which does not halt). Execution breakpoints were not used for the
timing, because two cannot coexist.
- **Read and write on `player + 0x878`.** The write is the control: it logged
  one `Write32` at `PC 0x088455b4`, which is the stamp store. Reads at
  `0x0883e908` are `HullOverlay_AbsorbWindowActive`. **That PC cannot tell a
  draw from the HUD**, because `Hud_UpdateEnergyBar` (`0x0881c740`) calls it
  every frame. They logged continuously from the moment of arming, which
  proves the instrument and the address. Reads at `0x0883e954` are
  `HullOverlay_AbsorbFade`, whose only callers are the two draw paths. They
  are the test.
- **Read on `0x08abf4a4`**, the `10`. Its only reader is `HullOverlay_Submit`
  at `0x0890e670`, just before `Gu_SetMatrix(3)`. It is a second test
  signal, with no halt.

**Result**, in the log from the absorb onward:

| Signal | PC | Count | Span |
| --- | --- | --- | --- |
| stamp store on `player+0x878` | `0x088455b4` | 1 | `30:23.215` |
| `AbsorbFade` reads `player+0x878` | `0x0883e954` | 120 | `30:23.230`-`30:24.215` |
| `Submit` reads the `10` | `0x0890e670` | 649 | `30:23.247`-`30:24.215` (11 a frame) |
| gate from the HUD (+ draws) | `0x0883e908` | 844 | continuous |

Nothing was read at `0x0883e954` or `0x0890e670` in the baseline before the
absorb, except the AI's own window. Both stop within a frame of the second
elapsing. **So the stamp, the Fade gate and the Submit draw fire together,
for one second, and nothing else starts or stops them.** Confidence **95**.

**The texture matrix, measured.** One halting breakpoint was armed alone at
`0x0890e754`, and the absorb was triggered with circle held and released
(`input.buttons.send`), not `press`. It was read at the first frame of the
window (`p = 0.0334`):

```text
A (sp+0x50): (1,0,0,0) (0,-0,1,0) (0,-1,-0,0) (0,0,0,1)     // the quarter turn
B (sp+0x90): (10,0,0,0) (0,10,0,0) (0,0,0,1) (0,-0.050171,1,1)   // -1.5p = -0.0502
handed to Gu_SetMatrix(3) (sp+0xd0):
             (10,0,0,-) (0,0,0,-) (0,-10,0,-) (0,-0.050171,1,-)
```

Read as the GE's column vectors, that is `s = 10x`, `t = -10z - 1.5p`,
`q = 1`: **the projection this page inferred above, now measured.**
Confidence **90**.

**The in-race path is `Mesh_DrawBatchSet`, not the per-entity one.** A lone
breakpoint on `HullOverlay_Submit`'s entry during the player's absorb gave
`ra = 0x0890e134` on every hit. That is `HullOverlay_SubmitAbsorbBatched`,
called from `Mesh_DrawBatchSet` (`0x0893074c`). That function gates only on
its batch set's `+0xc0` (the craft), the window and the fade. It then
submits **every entry of the set** (`+0x74` entries of `0x28` bytes at
`+0x78`, the mesh at `+4`, the batch at `+8`). It never reads the per-mesh
`+0x79` name byte. The mesh objects of eleven submits in one frame, read
back through `*(mesh+0x48) + 0x10`:

| Mesh object | Name | Submits a frame | `+0x79` |
| --- | --- | --- | --- |
| `09b907d0` | `shipShape` | 5 | 1 |
| `09b92820` | `Airbrake_RightShape` | 2 | 0 |
| `09b92ac0` | `Airbrake_LeftShape` | 2 | 0 |
| `09b92e20` | `self_illuminatedShape` | 1 | 0 |
| `09b92fd0` | `glowingShape` | 1, later in the frame | 0 |

The batch counts match the file: `shipShape` has five list-0 batches, each
airbrake two, `self_illuminatedShape` one. `glowingShape` has two list-0
batches, and one of them is overlaid. Which one was not established; the
late submit suggests a second batch set. **Not overlaid:** `canopyShape`
(its one batch is in list 1) and `lodShape` (under the `LodGroup`'s second
child). **So on the race path every drawn list-0 hull mesh takes the
overlay, not only the one named for the ship.** Confidence **85** for
Assegai, which was measured directly. The rule for other teams is inferred
from it. `mesh+0x79` still gates the per-entity path at `0x0890f288`, which
this race never reached.

**The overlay stamps the glow mask at full strength.**
`Gfx_BuildBatchStateList(0x282)` takes its `param_2 & 0xc0` branch (`0x80`).
It calls `0x08811914` with `(1, 0xff, 0xff)`, which emits GE command `0xDC`
(stencil test: `ALWAYS`, ref `0xff`). It calls `0x08811948` with `(0, 0, 2)`,
which emits `0xDD` (stencil op `KEEP, KEEP, REPLACE`). The command bytes
were read at instruction level. With `Gu_PixelMask(0)` opening alpha, every
overlay fragment that passes the alpha test (`GREATER 0`) and the colour
test (not black) writes **`0xff`** into the bloom's glow mask. That value
does not depend on the fade. The original's frames show it: at the peak the
hull blooms into a white blob wider than its own silhouette (see the
screenshots below). Confidence **85**.

**Corrected 2026-10-01 (`pulse-hull-bloom`): the stamp is real and almost
all of it is overwritten before the bloom reads it.** A GE list of a real
absorb shows these ten overlay batches drawn *before* the shadow pass, whose
full-screen quad (`REPLACE`, reference 4, on every stencil outcome) puts the
whole mask back to `4`. Completed frames read on the software renderer hold
the hull at `4` through the window, and the hull does not bloom into a white
blob; PPSSPP's OpenGL backend draws that blob, and the frames below came from
a run that did not record its backend. The only overlay draw after the reset
is the glow batch's own (75 vertices on Assegai). The `255` read out of EDRAM
on 2026-09-23 was a mid-frame halt. Evidence, the table of prims and the
backend difference: [glow-mask.md](../../../rendering/glow-mask.md), "The hull
overlay's mask is wiped".

**Frames.** PPSSPP's own screenshot key (`g` in this profile's
`controls.ini`, sent with `xdotool` to the window after `windowmove 0 0`)
saved fourteen frames 0.11-1.09 s into the window. They are
`original-ppsspp-01.png`..`-14.png` under
`~/.cache/oag/drive/reports/pulse-absorb-probe/`, and none are in the repo.
The peak frame (`-07`, about 0.56 s) is near-white across the whole hull,
including the airbrakes. The HUD energy bar is white instead of cyan.

**Still not measured.** The depth test the port uses (`LessEqual` with no
write, for the original's `EQUAL` with writes on) remains chosen: a frame
cannot tell the two apart on an unmoving coplanar redraw.

**The port, against these frames.** `oag_fx::hull_overlay` was changed
to match what this run measured. Two changes:
- the mesh set is every list-0 mesh of the first level of detail;
- the overlay has its own blend, which writes the full glow mask the stencil
  writes.

Ours was captured on the same grid, team and circuit (`--race --mode
single_race --give mine`, one circle on tick 400). Frames are `glow-t*.png`
(bloom off, the default) and `bloom-t*.png` (`[graphics] bloom = true`),
and `compare-sequence-*.png` puts them under the original's. **Ours reads
much weaker than the original.** With bloom off, which is
`Graphics::bloom`'s uncalibrated default, the glow mask reaches nothing,
and only the `a^2`-weighted texture shows: a blue-white sheen near the peak.
With bloom on, the hull glows at the edges but does not blow out. Our base
hull is also a darker navy than PPSSPP's vivid blue, and the bloom's bright
pass is `rgb * mask`, so a darker hull blooms less. Neither cause is in
this module, so nothing here was tuned toward the picture.

**The LeachBeam overlay's fade source (static, confidence 75).**
`Mesh_DrawBatchSet` gates the LeachBeam half on `DAT_08b317ac == 2` and on
`**(float **)(craft + 0x4c) > 0`. `craft+0x4c` is the craft's weapon record
(`world + 0x70 + i * 0x1f0`, the same record whose `+0x1bc` is the held
weapon), so the value is that record's `+0`. Two functions touch it:
- `Weapon_FireLeachBeam` (`0x08866658`) sets `DAT_08b317ac = 2`.
  `LeachBeam_UpdatePool` clears it when a beam retires. So `2` means "a beam
  is live", not a race mode.
- `LeachBeam_UpdatePool` writes `LeachBeam_PulseStrength(beam)` every tick
  through `**(float **)(pool + 0x44 + beam[+0x40] * 4)`. `beam+0x40` is the
  firing craft's index (`Weapon_FireLeachBeam`'s third argument). Only the
  pool's destructor `FUN_088664e0` shows what those `+0x44` slots are: it
  decrements a count at `+0x1e0` of each one. A 0x1f0-byte record with a
  count at `+0x1e0` is the weapon record, but that is inferred; the slots'
  writer was not read.

**So the LeachBeam overlay lights the firing craft's hull, faded by its own
beam's pulse strength.** A 45 s write watch on every record's `+0` logged
nothing, because no LeachBeam fired in that window. One record's `+0` read
`0.5006` throughout, with `DAT_08b317ac == 0`. That fits a pulse strength
left over from an earlier beam: `LeachBeam_UpdatePool` stops writing when
the beam retires, and nothing clears the slot. The `== 2` gate is what keeps
a stale value from drawing. That is a zero without
the scenario behind it. Firing one through `psp-fire-weapon.py` halts
PPSSPP, so the live check is still open. Not wired.

### Open

- ~~**The LeachBeam ribbon's own texture is still unlocated**~~ - found
  2026-09-17, see the LeachBeam section below.
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
- ~~**The hull overlay pair's shared state object is unidentified**~~ - for
  the absorb half it is the craft entity itself (`+0x830` its clock,
  `+0x878` the absorb stamp); see "2026-09-23" above. The LeachBeam half's
  `+0x4c` pointer is still unread - see
  "2026-09-17: the hull overlay pair, read in full" above. Its type, size
  and writer are open; only two of its fields (`+0x4c`, `+0x830`/`+0x878`)
  are read, both through their consumers rather than a constructor.
- ~~**The LeachBeam overlay's own fade source is unidentified.**~~ Read
  2026-09-23 (later), statically: the firing craft's weapon record `+0`,
  written each tick by `LeachBeam_UpdatePool` with
  `LeachBeam_PulseStrength`. See "The LeachBeam overlay's fade source" in
  the "2026-09-23 (later)" section. Not yet seen live, and not wired.
- ~~`Gfx_BuildBatchStateList`'s `0x282` flag word is undecoded~~ - decoded
  2026-09-17, same pass: `GU_BLEND` on, additive (`exhaust::BLEND`'s own
  equation), `GU_CULL_FACE` on, depth write on, `GU_ALPHA_TEST`/
  `GU_COLOR_TEST` both cutout-shaped, `GU_STENCIL_TEST` on and feeding the
  glow mask - see "2026-09-17: the hull overlay pair, read in full" above.

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

`craft+0x16c`/`craft+0x168` are filled **before** this handler ever runs - and
**not by `Ship_AcquireLock`; corrected 2026-10-01** (`pulse-cull`, disassembly and a
live fault). `Weapon_RequestFire` (`0x08862d9c`), case `10`, sets bit `0x8000` and
stores its own third and fourth arguments into `+0x168` (the target's world matrix
pointer) and `+0x16c` (its craft index). `Ship_FireHeldWeapon` (`0x08844ae8`)
chooses them: **`(0, -1)` - the unlocked arm - unless `entity+0x85c != -1` and
`entity+0x860 & 1`**, the lock flag the reticle's own update sets on the tick its
brackets arrive (`HudSight_UpdateLeachBeam`, `lock-sight.md`). So a LeachBeam fired
before the arrowheads close (under `0.34` s of the target on screen) takes the
fizzle arm in the original, **and a target found by the window alone does not lock
it**; writing the held id into the weapon record by hand leaves `+0x168`/`+0x16c`
stale and the locked arm dereferences a null matrix (the halt
`scripts/psp-fire-weapon.py` records). The window scan itself is
`Ship_AcquireLock`, the same scan `missile.md` already documents in full: a
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

### 2026-10-02: the race-state half of the disconnect gate, read whole

`LeachBeam_UpdatePool` (`0x08866b08`) decompiled in full. A live kind-1 link
sets `instance+0x3c |= 0x40` (`LeachBeam_MarkDisconnected`) when ANY of:
`FUN_08872f54 > range` (owner craft node at `+0x5c`->`+0x30` matrix `+0x30`
minus target node `+0xa0` matrix `+0x30`, floored at `instance+0x13c`);
`target+0x860 & 0x1000` (set by state 5); `target->entity+0x1b8 & 0x10`;
the **owner entity's `+0x120` pending-leach > 0**; `Ship_State(target) != 1`;
`Ship_State(owner) != 1`; or `LeachBeam_Advance` returned 0 (lifetime). The mark
is one-way: a disconnected link only lingers (`+0.5 s`) and retires, it never
re-forms. So a kill (state 4 then 5) and the whole respawn wait (states 6/8)
disconnect the beam, and a respawned target is not drained. Static read of the
decompile, confidence 85; not watched live on PPSSPP. Ours lacked both
`Ship_State` tests (it only tested slot occupancy) and now has them
(`Beam::link_broken`). The owner `+0x120` arm is not ported: only the beam
writes that accumulator and there is one beam per race, so it cannot fire.

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
other weapon is. **Ported 2026-09-16**: `oag_weapons::projectile::leach_beam::Beam::drain`
arms `Ship::pending_thrust_scale` under the same three gates (positive amount,
racing, no Shield pickup), the composition root hands it to
`oag_physics::Environment::thrust_scale` at the next step, and
`oag_physics::engine::engine` applies it after the doubling on both branches -
`crates/raceplay/src/tests/weapons.rs`,
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

**2026-09-23: `LEACH` and `~LEACHATTACH` wired.**
`Cue::Leach` and `Cue::LeachAttach` are in `crates/sound/src/sfx/cue.rs`.
`Cue::Leach` fires from both places a beam can be fired locked -
`Race::spend_pickup`'s own LeachBeam arm and `Race::fire_opponent_leach_beam`
- since this section names `LeachBeam_InitLocked` alone, not the unlocked
fizzle case. `Cue::LeachAttach` is held for as long as a **locked** `Beam`
instance exists, through its own disconnect linger, matching "carried by the
beam itself" above rather than `Beam::connected`'s narrower window; its
position (the owner/target midpoint) is chosen, since this section does not
say what the beam's own scene node tracks.

**2026-09-25: `LEACHENERGY` wired too**, off the same edge this note used to
call "read and discarded". `oag_fx::beam::Ribbon::advance` (see the
2026-09-23 ribbon re-read just below) returns a `bool` for the cursor's own
wrap to zero - the pulse block - and
`crate::race::weapons::visuals::Race::advance_leach_beam_ribbon` now pushes a
`Cue::LeachEnergy` `CueEvent::at_point` on that same edge, at the ribbon's own
`energy_point` (the pulse block's `WO_LEACHBEAM_ENERGY` re-spawn target, not
`LeachAttach`'s owner/target midpoint). Verified against a real disc rather
than assumed: `LEACHENERGY` resolves on both `pulse-psp-usa.chd`'s own
`weapons.bnk` and, unexpectedly, on Wipeout HD's own `weapons.bnk` too (6
waveforms) - see `sfx_ground_truth.rs`'s
`wipeout_hd_loads_every_cue_but_one_and_reports_the_miss`.

### 2026-09-17: the LeachBeam ribbon's own texture, and the draw call that proves it

**The ribbon's texture is `Data\Weapons\Textures\pulse_leechbeam1_ADD.mip`**
(string at `0x08a7cc60`), found by widening the earlier `(?i)leach` search: the
disc spells the weapon "lee**ch**" in this one path, under the Cannon's own
`Data\Weapons\Textures\` directory rather than the LeachBeam hull overlays'
`Data\Tex\Weapons\` - which is exactly why the original search, and its own
claim that only two `Data\Tex\Weapons\` strings existed, both missed it.
Confirmed on disc, both pressings:

```sh
cargo run -q -p oag-tools --bin oag-wad -- cat \
    'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' \
    'Data\Weapons\Textures\pulse_leechbeam1_ADD.mip'
```

returns 5,136 bytes opening `40 00 40 00 08 00 01 03` - a 64x64, 8-bit
paletted, **swizzled** `.mip` (`+0x07` bit 0 set), exactly
`16 + 1024 (palette) + 4096 (pixels)`. `oag_texture::texture::Texture::parse`
decodes it clean on both pressings; the picture is a horizontal band of
white/blue static - an additive electric-arc strip, not a solid fill or a
gradient, matching the `_ADD` suffix and a beam ribbon by eye before any
Ghidra evidence is checked at all.

**The evidence chain, address by address:**

1. `search_strings` finds exactly one xref to the string, from
   `LeachBeam_LoadTexture` (`0x088730d4`, renamed from `FUN_088730d4`) - the
   same `FUN_089277ac` loader `Cannon_LoadTextures` and
   `Texture_LoadEffectSurfaces` both use, writing the loaded handle into
   `DAT_08b3bfa4` (left unrenamed - `rename_data`'s Hungarian-prefix linter
   rejects this project's own `g_snake_case` convention outright and
   `set_global` hits the same wall; not worth fighting further for one data
   label).
2. `LeachBeam_LoadTexture` is called from `LeachBeam_Construct` (`0x08872aa0`,
   renamed from `FUN_08872aa0`) - a node constructor that installs a vtable at
   `instance+0x38` and zero-fills a run of fields, the exact "tagged-object
   constructor idiom" `HANDOVER.md`'s "Traps that are live" warns is shared
   boilerplate across every node class on this engine and proves nothing
   alone. What clears that bar here is **class-specific evidence**: this
   constructor is the one and only caller of `LeachBeam_LoadTexture`, which
   loads a string with "leech" and "beam" both in it - not a generic
   allocation pattern. Confidence **80**, capped below the two functions
   below because the constructor's own field writes past the vtable and the
   texture load were not read line by line.
3. `LeachBeam_Construct` is called from `FUN_088662a8`, which allocates the
   **single** beam `Instance` at `pool+0x64` (`FUN_088662a8`'s own
   `*(int *)(param_1 + 100) = iVar4` - `100` decimal is `0x64`) - the exact
   field `Weapon_FireLeachBeam`'s `pool->instance` reads. This is the
   LeachBeam's own pool/weapon-system constructor, one call per craft, name
   left alone: it is shared `WEAPON_SYSTEM_%d` node machinery near the Cannon
   member's own function set, not read further this pass.
4. `DAT_08b3bfa4` has exactly one other reader: `LeachBeam_BuildStrip`
   (`0x088739b0`, renamed from `FUN_088739b0`), which opens with
   `Gfx_BindTexture(DAT_08b3bfa4)`, then walks the beam's own `0x30`-byte-
   stride vertex chain building GE vertices, then calls
   `LeachBeam_SubmitStrip` (`0x088731c4`, renamed from `FUN_088731c4`), which
   issues the actual `Gu_DrawArray`. **Neither function is called by a `jal`** -
   `get_xrefs_to` on `0x088739b0` finds nothing, which looked like dead code
   until `read_memory` on `DAT_08acb048` (the vtable `LeachBeam_Construct`
   installs at `instance+0x38`) showed `0x088739b0` sitting at vtable slot
   `0x44`, alongside `0x08872c78` and `0x08872b68` at neighbouring slots -
   it is a **virtual draw method**, dispatched through the node system, which
   is the whole reason no direct call site exists. Confidence **85** on both
   functions: the vtable slot is a direct memory read, not an inference, and
   the field agreement with `LeachBeam_Advance` below is total.

**`LeachBeam_BuildStrip` (`0x088739b0`) is the ribbon's draw call, and it
closes the page's oldest open item.** It reads:

- `param_1+0x140` - the segment count `LeachBeam_Advance` computes
  (`ceil((6.0/range)*min(distance,range)*6.0)`, already recovered below).
- `param_1+0x170 + i*0x10` - **not** "a 32-entry chain of `0x30`-byte
  transforms" as read before; that is the GE vertex array's own base
  (`+0x3c0`). `+0x170` is a **separate**, plain `Vec4` array of the chain's
  raw world-space points, written earlier in the same tick by
  `LeachBeam_Advance` and consumed here.
- `param_1+0xe4` - the strip half-width, a **recovered constant**:
  `LeachBeam_InitLocked` sets it from `DAT_08a7cc04`, read directly as
  `0x3f800000` = **1.0** world unit (`read_memory`, both pressings share one
  binary so one read suffices). Confidence **90**.
- `param_1+0xe8` - the base colour, `0xffffffff` (opaque white) from
  `LeachBeam_InitLocked`; only its alpha byte is overwritten per vertex below,
  so the ribbon is unlit, uncoloured white modulated solely by alpha and the
  texture's own colour. Confidence **88**.
- `param_1+0x134`/`+0x144`/`+0x148` - age, the disconnected flag and
  disconnected-at, the same three fields `Beam::advance` already carries.

**The colour write is the disconnect fade, read at instruction level**: while
`+0x144` (disconnected) is clear, every interior vertex gets alpha `0xff`
(`iVar6 = -0x1000000`, i.e. `0xff000000`, ORed with `+0xe8`'s RGB). Once
disconnected, alpha becomes
`(1.0 - (age - disconnected_at) * 2.0) * 255`, i.e. a **linear fade to zero
over exactly 0.5 seconds** - [`DISCONNECT_LINGER_SECONDS`] in
`oag_weapons::projectile::leach_beam`, already ported and unchanged by this
finding. Confidence **88**. **The two chain endpoints (`i == 0` and
`i == segment_count - 1`) are forced to `0xffffff`** - RGB white, alpha
**zero** - regardless of the connected/disconnected branch, so the ribbon is
invisible at both attachment points under the additive blend and only the
middle of the arc ever shows. Confidence **85**, a direct read of the two
branches that override the computed `iVar6`.

**Corrected 2026-09-23 - see "2026-09-23: the ribbon re-read" below: both
strips share one chain displaced along both axes, and they widen along the
camera's `x` and `y`, not along the displacement.** The original text:
**The geometry is two crossed strips, not one flat ribbon.** `LeachBeam_Advance`
(`0x08873fa0`) writes each chain point twice - once at `param_2 + i*0x10`
(consumed by the `i*0x30` vertex pair, displaced along one axis) and once
folded into the same array at the mirrored index `segment_count + i + 1`
(displaced along a second, perpendicular axis) - and `LeachBeam_SubmitStrip`
draws `(segment_count*2+2)*2` vertices as **one** `GU_TRIANGLE_STRIP`
(`Gu_DrawArray(4, 0x19f, ...)`, primitive `4` = triangle strip). Two
perpendicular ribbons sharing a spine, crossed through each other like an
"X" in cross-section, is a standard cheap volumetric-beam trick and reads as
deliberate rather than as two unrelated draws. Confidence **80**: the vertex
count and indexing match exactly, but which two axes (view-space billboard
vs. two fixed axes derived from the craft nodes' own orientation) was not
resolved past "the CPU pre-transforms the points and the GE matrices are set
to identity right before the draw" (`Gu_SetMatrix(1, identity)`,
`Gu_SetMatrix(2, identity)` in `LeachBeam_BuildStrip`), which reads as
**world-space geometry, not a camera-facing quad** - the transform applied is
the node's own place in the scene graph, not the camera. This engine has no
equivalent per-craft node basis to draw the two axes from (`Ship` carries no
local orientation frame the way a `.vex` node chain does), so the build below
chooses its own pair of axes perpendicular to the owner-target line - labelled
**chosen, not measured** for that reason alone, with the crossed-strip
*structure* itself recovered.

**The amplitude table, read in full from `LeachBeam_InitLocked` (`0x08873d3c`).**
`instance+0xb4` is a **12-entry `f32` array**, each drawn independently once
at construction via `Psys_RandFloatRange(0.0, 2.0)` - `0x40000000` is `2.0f`.
`LeachBeam_Advance` re-rolls exactly **one** bucket per pulse
(`uVar21 = ceil((segment_count-1)/DAT_08a7cc00)+1; if (uVar21<0xc) amplitude[uVar21] = Psys_RandFloatRange(0,2.0)`),
where `DAT_08a7cc00` is `0x40400000` = **3.0** (`read_memory`, both
pressings) - so **one amplitude value covers three consecutive chain
segments**, not one value per segment as the pre-2026-09-17 reading of this
page assumed. Confidence **88** on the range and the bucket span, both direct
reads of literals and a decompiled loop; the exact re-roll *cadence* is not
reproduced - see below.

**What is buildable and what stays chosen**, for
`crates/fx/src/beam.rs` (**superseded 2026-09-23**: nothing in the
geometry is chosen any more, see below):

- Recovered and built: the segment-count formula, the half-width (`1.0`),
  the base colour (opaque white), the disconnect fade (`0.5` s linear, already
  the ported [`DISCONNECT_LINGER_SECONDS`]), the endpoint-alpha-zero taper,
  the amplitude range (`[0.0, 2.0]`) and bucket span (`3` segments), the
  additive blend (identical to [`oag_fx::exhaust::BLEND`] -
  `Gu_BlendFunc(0, 2, 10, 0, 0xffffff)` is the same `GU_ADD`/`GU_SRC_ALPHA`/
  `GU_FIX(1.0)` call `exhaust::BLEND`'s own doc comment already cites, so this
  reuses that constant rather than re-deriving it), and the crossed-double-
  strip structure.
- Chosen, no confidence score: the exact pair of perpendicular axes the two
  strips displace along (this engine has no per-craft node basis to read
  them from), and the amplitude re-roll cadence (recovered as "about once a
  second, tied to the ribbon's own scroll cursor" - the same cursor
  `oag_weapons::projectile::leach_beam`'s own doc comment already declines to
  model, for the reason given there: reproducing it needs render geometry the
  simulation crate must not carry). The build re-rolls each bucket on a fixed
  one-second render-side timer instead, seeded from a `RaceView`-owned `Rng`
  the same way [`RaceView::exhaust_rng`] is - never `world.rng`.
- **Not built, and not read this pass**: `Gu_Enable(3)` +
  `Gu_StencilOp(0,0,2)` + the `g_bloom` call inside `LeachBeam_SubmitStrip`
  strongly suggest the ribbon feeds the bloom glow mask the same way
  `Trail_BuildStateList` does for the exhaust ribbon (`exhaust.rs`'s own doc
  comment on `TRAIL_BLEND`/`TRAIL_GLOW_GAIN`) - flagged for whoever next
  touches this, not chased further here.

### Open

- ~~**Which draw call consumes the ribbon.**~~ Found 2026-09-17, see above.
- **`FUN_08872e64`** (the per-instance teardown `LeachBeam_UpdatePool` calls on
  retire) and **`FUN_08862d4c`** (the `Ship_IsValid`-shaped predicate
  `LeachBeam_Drain` gates both halves on) are unread past their call sites and
  are not in `names.tsv`.
- **The per-craft readout slot** `LeachBeam_UpdatePool` writes `PulseStrength`
  into - `**(float **)(pool + owner * 4 + 0x44)`, i.e. offset zero of the owner's
  own entity - is not identified. A HUD or audio amplitude is the obvious guess
  and it is only a guess.
- ~~**Which two axes the crossed strips displace along**, and **whether the
  ribbon feeds the bloom glow mask**~~ - both closed 2026-09-23, see "the
  ribbon re-read" below: the shooter's node rows, and yes (`0x28`).
- **`FUN_08872c78` and `0x08872b68`**, the LeachBeam node's two other vtable
  slots found alongside `LeachBeam_BuildStrip` in `DAT_08acb048` - not
  inspected; plausibly construct/destruct or an update method.

### 2026-09-23: the ribbon re-read, and measured in play

**Prompted by a from-play report** that of every weapon effect the
LeachBeam was the furthest from the original. A capture settled it: the
original draws a crisp, jagged lightning arc and lights the firing craft's
whole hull, pulsing; this build drew a soft, nearly straight smear and no
hull light at all. Re-reading `LeachBeam_Advance`, `LeachBeam_BuildStrip`,
`LeachBeam_InitLocked` and `LeachBeam_SubmitStrip` whole showed that three
readings in the 2026-09-17 section above were wrong. The corrections, and the
functions they come from:

**1. The strips widen along the camera, not along the displacement.**
`LeachBeam_BuildStrip` transforms each chain point (`instance+0x170 +
i*0x10`) through the matrix on top of the stack (`Math_TransformVec4` against
`param_2 + depth*0x40 + 0x1410`), then sets the view and model matrices to
identity (`Gu_SetMatrix(1/2, &DAT_08a907a0)`). Pair `i` gets `(x - w, y, z)`
and `(x + w, y, z)`; pair `segments + 1 + i` gets `(x, y - w, z)` and
`(x, y + w, z)`, with `w = instance+0xe4 = 1.0`. So the two strips are the
same chain widened along **view-space `x` and `y`**: a cross section that
faces the camera, the same trick the Cannon's bolt uses. Confidence **88**:
the six stores per pair are a direct read.

**2. Every chain point is pushed along both axes, and the push travels.**
`LeachBeam_Advance` builds two axes from the shooter's own node matrix
(`*(node+0x30)`, rows `+0x00` and `+0x10`): `A = normalize(step x -up)` and
`B = normalize(step x right)` (`vcrsp.t`, with the usual zero-length guard).
For chain point `i + 1`, `i` in `0..segments`:

```c
a  = (i + cursor) % segments;                 // cursor = instance+0xa8
b  = (i + (int)(counter * 0.5)) % segments;   // counter = instance+0x130
k  = (2*pi / segments) * ((6.0 / range) * min(distance, range));
p  = origin + (i + 1) * step
   + A * sin((a + 1) * k) * amp[ceil(a / 3.0)]
   + B * sin((b + 1) * k) * amp[ceil(b / 3.0)] * 0.5;
```

`k` works out near `2*pi / 6` whatever the length, so the arc kinks about
once every six segments. The scroll phase `instance+0x1180` plays no part in
the geometry. `counter` counts ticks modulo `2 * segments` and `cursor` is
`counter % segments`, so the pattern moves one segment a tick toward the
shooter. `ceil(a / 3.0)` reaches `12` for a full-length beam, one entry past
the twelve-entry table, so the original reads `instance+0xe4`, the
half-width `1.0`. Confidence **85**.

**3. The texture repeats per segment and scrolls.** `LeachBeam_InitLocked`
zero-fills each `0x30`-byte pair and writes `u = (pair & 1)` on both
vertices, `v = 0` on the first and `v = 1` on the second. So `u` flips at
every chain point along the beam and `v` runs across it.
`LeachBeam_SubmitStrip` sets `Gu_TexWrap(0, 0)` (repeat) and
`Gu_TexOffset(instance+0x1180, instance+0x1184)`: the phase advances
`rate * dt * 2` a tick, so the band slides two repeats a second. Confidence
**88**.

**Smaller corrections.**

- The alpha taper zeroes points `0` and `segments - 1`, not the last point.
  After its loop, `LeachBeam_BuildStrip` overwrites pair `segments` with the
  first strip's last vertex and the second strip's first vertex, a
  zero-area bridge. So the first strip stops at point `segments - 1`, and
  only the second reaches the target, at full alpha. Confidence **85**.
- Depth writes are **off**: `Gu_DepthMask(1)` masks them, per the hull
  overlay's own 2026-09-23 correction. Culling and lighting are disabled.
- **The ribbon writes the glow mask.** `Bloom_SetGlowMaskWritable(g_bloom,
  1)`, `Gu_PixelMask(0)`, then `Gu_StencilFunc(GU_ALWAYS, DAT_08ab1078,
  0xff)` and `Gu_StencilOp(KEEP, KEEP, REPLACE)`. `DAT_08ab1078` reads
  `0x28`, so every ribbon fragment stamps `40` into the mask. The alpha test
  is off, so transparent texels stamp too. Confidence **80**.
- **The one-bucket re-roll is a fixed bucket, and only for short beams**:
  `ceil((segments - 1) / 3.0) + 1`, applied only when it is under `12`. A
  beam of 34 or more segments re-rolls nothing. Confidence **85**.
- **The origin offset is not the player's.** `LeachBeam_Advance` moves the
  origin `-3.0` (`DAT_08ab1080`) along the shooter's up when the node's byte
  `+0x6d` is set. Measured live: that byte reads `0` on the player's craft
  (`InitLocked`'s seventh argument, `t2`), so the player's beam starts at
  the craft's origin. Which craft set it was not read.

**`WO_LEACHBEAM_ENERGY` travels.** Inside the pulse block,
`Psys_Spawn_q(..., instance+0xf0, ...)` spawns the effect on the matrix at
`instance+0xf0`. That matrix is then copied from the target's. In the same
tick's chain loop, `LeachBeam_Advance` writes the undisplaced point
`origin + (segments - 1 - cursor) * step` into its translation row
(`instance+0x120`). So the effect appears one segment short of the target
and walks back to the shooter one segment a tick. The previous instance's
handle (`instance+0xec`) goes to `FUN_088f3298` first, which **kills** it:
read 2026-09-30, see [particle-system.md](particle-system.md), "Releasing an
instance by handle". Confidence **82**.

**The two names, re-read from disassembly on 2026-09-30.** The first pass
read them from single decompiles, and the decompile of `ReaimChain` hides its
first argument (a float in `f12`), so the row for it here was wrong. Read
again instruction by instruction:

| Address | Name | Confidence | What it does |
| --- | --- | --- | --- |
| `0x088734d0` | `LeachBeam_KeepInTrack` | 92 | `AiTrack_LocatePosition(100.0, track, &frame, point, 0, -1, 0)` fills a `SplinePt`-shaped record on the stack (`+0x00` pos, `+0x20` down, `+0x30` lateral, `+0x44`/`+0x48` half-widths), zero-initialised from `DAT_08a90a20`, which reads sixteen zero bytes. Three plane tests follow, in order, against that one record. |
| `0x08873328` | `LeachBeam_ReaimChain` | 92 | Moves the point and rewrites the step. Arguments: `a0` out step, `a1` the beam instance (target at `*(inst+0xa0) + 0x30`), `a2` direction vector, `a3` the point (in place), `t0` chain points still to place, `f12` the scale. |

**The three tests** (`0x088735d4`, `0x08873724`, `0x088738d0` each leave the
failed test's own dot product in `f12`, which is the scale `ReaimChain`
receives; it is negative by construction):

1. **Below the road:** `(pos - point) . down < 0`. Direction `-down`. The
   move is an exact projection onto the road plane through `pos`.
2. **Past the right wall:** `(pos + lateral * (half_width_right - 2.0) -
   point) . lateral < 0`. Direction `normalize(pos - point)`, straight at the
   centre-line point, **not** along `lateral`.
3. **Past the left wall:** the mirror, with `-lateral` and
   `half_width_left - 2.0`.

There is no ceiling test. The record is located once; each test reads the
point the test before it may have moved, and a later re-aim overwrites the
step. `*(point + 0xc)` is set to `1.0` after each. The `2.0` is the literal
at `0x08873648`/`0x08873818`; `normalize` divides by `MaxFloat` for a zero
length (`vcst.s MaxFloat`).

**What `ReaimChain` computes** (`0x0887334c`-`0x08873460`), with `s = f12`
and `dir = *a2`:

- the point moves to `point - s * dir`;
- the step becomes `(target - (point + s * dir)) / remaining`, measured from
  the point **displaced the other way**, and left undivided when `remaining`
  is zero (`0x0887346c`).

So a chain re-aimed once ends `2 * s * dir` off the target, not on it. This
is what the code does, and the port reproduces it rather than correcting it.
The earlier row's "`(target - point) / remaining`" was the decompiler's
reading of the same instructions with the sign of `s` lost.

**`LeachBeam_Advance`'s loop around them** (`0x08873fa0`): per chain point
`base += step`, then `KeepInTrack(instance, &base, &step, segments - i - 1)`
updates `base` and `step` **in place**, then the two-axis displacement is
added to the kept `base` and stored. The two displacement axes, the
angular step and the amplitude span are computed once before the loop from
the unbent step, so only the base walk bends. The value written into
`WO_LEACHBEAM_ENERGY`'s matrix (`instance+0x120`) is the kept base, so the
effect follows the bend.

**Why the record layout is 80 and not higher.** The layout is `SplinePt`'s
([track.md](../../../formats/track.md)) field for field: `+0x20` is the
`down` axis (the floor test's sign only comes out as "below the road" with
it), `+0x30` the lateral axis, and `+0x44`/`+0x48` the left and right
half-widths, which the test pairs with `-lateral` and `+lateral`
respectively, matching `oag_game`'s own `centre + lateral * half_width_right`.
What is not read is `AiTrack_LocatePosition`'s search: `100.0` is taken as a
radius, and whether it interpolates along a segment or returns a control
point is not established. When it finds nothing the zeroed record makes
every test compute `0`, which is not `< 0`, so the chain stays straight -
that part is read.

**Built** in `oag_fx::beam::tube` (2026-09-30), fed by
`oag_raceplay::Spline::tube_frame`; a disc-backed test on Talon's
Junction's own spline (`leach_tube_ground_truth`) finds the worst chord on
the circuit, `9.22` units out of the tube straight and `0.01` bent. Measured
against PPSSPP on 2026-10-01, below.

#### 2026-10-01: the tube against PPSSPP

**Question and falsifier, written before the capture.** Does a failed plane
test move the chain exactly as the port's literal reading of
`LeachBeam_ReaimChain` says (the point to `point - s * dir`, the step measured
from `point + s * dir`), or as the "corrected" `(target - point') / remaining`?
The two predict the next chain point and the next step `2 * s * dir / remaining`
apart (the corrected reading keeps the point at `point - s * dir` and measures
the step from there), so one chain whose tests fail tells them apart. (The first
draft of this paragraph said "`2 * s * dir` apart", and the first analysis moved
the corrected reading's point the wrong way, inflating its misses by about
`remaining + 1`; both are fixed here and the table below is the recomputed one.) It would come
out the other way if the logged next point matched the corrected law. A run
where no plane test fails could not decide anything, so each run is counted
only for its failing transitions.

**Method.** PPSSPP v1.20.4, Pulse (UCUS98712), a Talon's Junction time trial
(`psp-drive.py restart`, so every craft is in race state `1`), private muted
instance. The player is pinned on a spline point with `place_body`'s writes (the
same rigid-body write `psp-drive.py place` makes) for six ticks, a fake target
node (an identity matrix with the chosen translation, in free RAM) is written
into the player's weapon record at `+0x168`, `+0x16c` and `+0x1bc = 10`, and the
fire bit `0x8000` is set at a `Weapons_DispatchFire` hit. A breakpoint on the
return from `AiTrack_LocatePosition` inside `LeachBeam_KeepInTrack`
(`0x08873568`) then logs, for every chain point, the point (`s2`), the step
(`s1`), the points still to place (`s0`), the target node's translation and the
located record at `sp+0x10` (24 floats). The checks used: `rec.pos` within 20
units of the query, `|down|` and `|lateral|` near `1`, and `s0` counting down
from `segments - 1` to `0` - all held, so the register mapping is right.
Consecutive calls of one chain give the before and after of each call: the next
call's point is the kept point plus the kept step. The scripts are under
`data/scratch/pulse-capture/` (`bent.py`, `an.py`, `an4.py`); captures
`bent2`..`bent8` there.

**The law is the port's literal one.** Six runs, 585 consecutive transitions:

| Run | Chord | Segments | Transitions with a failing test | Literal law, worst miss | Corrected law, worst miss |
| --- | --- | --- | --- | --- | --- |
| bent2 | spline 2634 to 2718 | 13 | 55 (48 right wall, 7 floor) | `9e-5` | `1.68` |
| bent3 | same, start moved 0.7 along the chord | 13 | 55 | `7e-5` | `1.66` |
| bent5 | same, moved 1.4 | 13 | 55 | `7e-5` | `1.63` |
| bent8 | same, repeated | 13 | 70 (60 right, 10 floor) | `7e-5` | `1.68` |
| bent4 | spline 2619 to 2700 | 13 | 46 (right wall) | `8e-5` | `1.29` |
| bent6 | 2634 to a target 160 units past the left edge of 2718 | 36 | 79 (46 left, 12 right, 15 floor, 6 floor then left) | `8e-5` | `81.3` |

(Units, miss of the next chain point; the next step misses by the same amount
under the corrected law and by at most `3e-5` under the literal one.) All three plane
tests fire, floor then wall in one call included, and the original's sign and
order are the port's: the below-the-road test, the right wall, the left wall,
each reading the point the one before moved. **The hook is real**: a re-aimed
chain does end off the target (the four 13-segment chains end `0.44` to `1.24`
units from it, not on it). Confidence **92**: a runtime trace of the
arithmetic, six runs, every branch; short of the top band because no second
binary has been read for it. `KeepInTrack` and `ReaimChain` are raised from 80
to 92. No captured table is committed: the records are the original's own runtime
track data, which `legal.md` keeps out of the tree, so the numbers stay on this
page and the captures under `data/scratch/pulse-capture/` (`bent2`..`bent8`).
`tube::tests::the_step_is_measured_from_the_point_displaced_the_other_way` pins
the law on a synthetic road.

**The locator interpolates, and the port does not.** The located frame is not a
row of our exported spline and not a discrete choice: with the query moved
`0.65` and `1.29` units along the chord (bent3, bent5) and the same nearest row,
`rec.pos` moved `0.42`/`0.85`, the left half width `0.22`/`0.45` and `lateral`
`0.006`/`0.012`, linearly. So `AiTrack_LocatePosition` interpolates along the
segment, as the page's earlier open question guessed it might. What it
interpolates between is not recovered: lerping our rows leaves a `0.19` residual
at the start of the chain (mostly lateral, falling to `0.01`), at spacings of
one to eight rows alike, and the half widths differ from the nearest row by up
to `0.3`. Its position-error budget is small: running our locator (the nearest
row, within `100.0`) and the literal law over the original's own start and
target reproduces the original's chain to **`0.25`-`0.54` units at worst**
(`0.17`-`0.31` mean) in the four 13-segment runs, while the chain itself leaves
the straight line by `14.4`-`21.7`. That is below anything visible, so nothing
in `oag_raceplay::Spline::tube_frame` is changed here (not this lane's file);
the record is in the handover thread.

**The search radius was not exercised.** The `100.0` could only show as an
all-zero record for a query more than 100 from any spline point, and the tube
keeps every chain point near the road: the farthest query seen from its
located `pos` is `19.4` units over 585 calls, and no all-zero record appeared.
A shooter pinned 130 units along the lateral axis of spline point 2634
was tried: that point turned out to lie within 20 units of another stretch of
the circuit (the corner folds back on itself), so it never left the radius
either, and pinned over six ticks the craft was re-seated about 22 units away
by the fire. Written inside the dispatch hit instead, the body stayed where it
was put, but the first chain still started from the previous pose. Open: it
needs a point more than 100 from every spline sample, and a shooter that stays
there for a frame.

**What can disconnect a fired beam, read on the way.** The first fires
disconnected on their first update (`instance+0x3c & 0x40`).
`LeachBeam_UpdatePool`'s per-tick test (`0x08866c80`-`0x08866d28`, read
instruction by instruction) sets the disconnect when any of: the floor-clamped
distance between the beam and the target's resolved position exceeds
`ActiveLeachBeamStats+0x11c` (`250`, measured live as `81.1` in one fire); the
target's `entity+0x860 & 0x1000`; the target's weapon record `+0x1b8 & 0x10`
(shielded); the owner's weapon record `+0x120` is above `f22` (`0.0`); or
`FUN_0883e64c` (which returns `entity+0x8c`, or a lookup through `+0x364` when
`entity+0x860 & 0x800`) is not `1` for the target or for the owner. Two of
these were seen holding during the session: the owner's `+0x120` read `3.0`
after a failed shot (and `0.0` otherwise), and after the earlier race had
finished the entities read `+0x8c = 2` (AI) and `6` (player), where all eight
read `1` after `psp-drive.py restart`. Which test fired was isolated for
neither. Five of the six captures above still disconnect on the first update
(`instance+0x144 = 1`, `+0x148 = 0.0167`), and the chain is then built for the `0.5` s linger, which is what was logged; that
cause was not run down.

Frames, native 480x272, three consecutive logged moments of one beam
(bent8, `beamA_40`, `beamA_66`, `beamA_105` under `data/scratch/pulse-capture/`):
the arc leaves the craft, runs along the outside of the corner, kinks at the
two re-aim points and ends on the target, jagged but kept inside the tube. They
are of the original only: our build cannot be posed against a fake node, so
these are illustrative and not a matched comparison.

#### Measured in play (PPSSPP v1.20.4, UCUS98712)

Private muted instance, single race / Venom / Talon's Junction / Assegai,
player eighth on the grid. `+0x1bc = 10` was written on the player's weapon
record during the countdown. The player sat still and fired `square` on the
tick `entity+0x860` bit 0 set (lock confirmed). Logged, non-halting
watchpoints, armed just after one `LeachBeam_InitLocked` breakpoint hit:

| Watch | Hits | Reading |
| --- | --- | --- |
| `0x08b3bfa4` read (ribbon texture, `LeachBeam_BuildStrip` `0x08873ab4`) | 166 in 2.70 s | 60 Hz |
| `instance+0x130` write (`0x088748f4`/`0x08874908`) | 2 x 163 | the counter steps every frame |
| `instance+0xec` write (`0x088743f8`/`0x088744b8`) | 7 x 2 | the ENERGY re-spawn, every cursor wrap |
| `instance+0x138` write (`LeachBeam_MarkPulse`, `0x0887331c`) | 2, 1.17 s apart | the pulse strength re-arms at most once a second |
| `instance+0x144` write (`0x08873098`) | 1 | disconnect, 0.5 s before the last frame |
| shooter weapon record `+0` write (`0x08866eac`, `LeachBeam_UpdatePool`) | 162 | every frame, **through the linger** |
| `0x08af2804` read (`HullOverlay_SubmitLeachBeamBatched`, `0x0890e150`) | 1,617 | the LeachBeam hull overlay draws in play, on the firing craft, for the whole beam |

This closes the "LeachBeam overlay's fade source ... not yet seen live"
item above: the overlay is drawn, and its fade is the pulse strength.
Confidence **90** for the overlay drawing (a read of its only texture from
its only reader), **85** for the cadences.

**Built** in `oag_fx::beam` (the ribbon, the ENERGY walk) and
`oag_raceplay::scene::absorb_overlay` (the LeachBeam half of the hull
overlay pair).

### 2026-09-17: the two hand-built quads' geometry, read - and drawn

**Closes this page's own "the two display lists are still not drawn" line
from 2026-09-08.** Read with the Ghidra bridge on `psp-pulse-usa`:
`Cannon_DrawRound` (`0x0886545c`) itself, plus three functions it or its
callers reach that were previously unread: `Cannon_RotateFlashCorner`
(`0x08864ea0`, confidence 82 - single caller `Cannon_DrawRound`, a plain 2D
rotate-about-`center` applied to one `(x, y)` corner) and
`Cannon_BuildRoundBasis` (`0x08864f54`, confidence 75 - four callers, all
Cannon: `Cannon_DrawRound`, `Cannon_Init`, `Cannon_UpdateRound` and
`FUN_0886481c`; reads the round's Vex node's own world matrix and, only
when a per-round flag at `+0xe4` is set, recomputes the translation row by
an offset along the node's local axes - the mechanism is read, the
*purpose* of that conditional offset is not, so this stops at 75 rather
than the 85+ this page's fully-closed reads carry). `Cannon_BaseSpeedKmh`,
`Cannon_Construct`, `Cannon_Init`, `Cannon_UpdateRound` and
`Weapon_FireCannon` were re-read for the constants below rather than newly
decompiled - all already named on this page.

#### The bolt: two crossed camera-facing ribbons, not two literal 3D quads

`Cannon_BuildBoltList` draws two triangle strips (`instance+0x100` and
`+0x160`) from vertex blocks `Cannon_DrawRound` rewrites every frame.
Reading the stores at instruction level: both blocks share the same two
world points - the round's own position last tick (`prev`, from
`instance+0x80`, transformed) and a point 20% of the way from this tick's
position (`curr`, `instance+0xa0`) back toward `prev`
(`near = curr + 0.2 * (prev - curr)`, `DAT_08ab1064 = 0x3e4ccccd = 0.2`) -
and only the offset added to each point's `x`/`y` differs: block one adds
`(-W, +W)`/`(+W, -W)` (the two corners at one point lie on the diagonal
`(1, -1)`), block two adds `(-W, -W)`/`(+W, +W)` (diagonal `(1, 1)`),
`W = *(instance+0xd4) = DAT_08ab1060 = 0x3eb33333 = 0.35`, reseeded to that
literal by both `Cannon_Construct` and `Cannon_UpdateRound` and never
touched anywhere else - a flat constant, not a per-round roll. The two
diagonals are perpendicular, so the pair is a camera-facing cross section
through the streak's own axis - the standard cheap-volumetric-streak trick -
rather than two coplanar or two literally-3D-crossed quads. Confidence 85.

**The near 20% of the previous-to-current segment is left uncovered** -
the streak runs from `near` to `prev`, not from `curr` to `prev` - which
reads as leaving room for the round's own dart mesh (`pulse_muzzleflash.vex`)
at its nose, though that specific reason is inferred rather than read.

**Colour is fixed, not read from anywhere per-frame**: `instance+0xd8` is
seeded to `0xffffffff` (opaque white) once in `Cannon_Construct` and no
other write to it was found in `Cannon_DrawRound`, `Cannon_UpdateRound` or
`Cannon_Init`. Confidence 88.

**The original builds this in a hand-transformed, near-clip-space frame**:
it transforms `prev`/`curr` through the current top-of-matrix-stack
transform, resets that matrix to an identity constant
(`DAT_08a907a0`), and adds the constant width straight to the transformed
`x`/`y` - a fixed-function trick for a billboard whose on-screen size
tracks perspective without a per-vertex camera basis. `oag_render` has no
such hand-transform stage, so `oag_fx::weapon_quads::geometry` builds
the equivalent offset from the camera's own `right`/`up` vectors instead,
the same port `exhaust::sprite` already makes - see that module's own doc
comment for the full equivalence argument.

#### The muzzle flash: one quad, randomly sized and rotated, fixed white with random alpha

`Cannon_BuildMuzzleFlashList` draws one strip (`instance+0x1c0`), centred
on the round's own position (`Cannon_BuildRoundBasis`'s translation row,
transformed) with half-size `*(instance+0xcc) * 3.0`
(`FLASH_SIZE_SCALE = 3.0`, confidence 88 - the literal at the draw site).

`Cannon_UpdateRound` rerolls three values **every tick** the round's age is
under `0.1` seconds (`FLASH_WINDOW_SECONDS`, the same `+0xc8 < 0.1` gate
`Cannon_DrawRound` reads to decide whether to draw the flash at all):

- `instance+0xcc` (the half-size before the `* 3.0` above) from
  `Psys_RandFloatRange(DAT_08ab105c, DAT_08ab1058)` =
  `Psys_RandFloatRange(0.65, 1.3)` - `FLASH_SIZE_RANGE`.
- `instance+0xd0`, a rotation angle from `Psys_RandFloatRange(0, 2*pi)`,
  consumed by `Cannon_RotateFlashCorner` to rotate each of the flash
  quad's four corners about its own centre - a rotated square from one
  axis-symmetric texture, not four differently-shaped corners.
- The colour word, from `Psys_RandIntRange(0x96, 0xff)` combined as
  `roll * 0x1000000 + 0xffffff` - **the low three bytes are hard-coded
  `0xff` (opaque white in each of R/G/B), only the top byte (alpha) is the
  rolled value.** This corrects the 2026-09-08 entry's "randomly sized and
  coloured" framing on this page and in `HANDOVER.md`/the visuals thread:
  the flash is a fixed white with a randomly rerolled **alpha**, not a
  random RGB colour. Confidence 88 (the arithmetic is read directly).

All three literals - `0.65`, `1.3`, `0.35`, `0.2` - are read at
`0x08ab105c`/`0x08ab1058`/`0x08ab1060`/`0x08ab1064` respectively, by direct
memory read on `psp-pulse-usa`, not decompiled from an instruction operand.

#### What this project draws now

`oag_fx::weapon_quads::Pipeline` draws both, textured from
`Data\Weapons\Textures\Cannon_bolt.mip` (`CANNON_BOLT_TEXTURE_ENTRY`, entry
1057) and `Cannon_muzzle_flash.mip` (`CANNON_MUZZLE_FLASH_TEXTURE_ENTRY`,
entry 1058), with the same additive blend as the exhaust flare's own
(`Gu_BlendFunc(0, 2, 10, 0, 0xffffff)` at both `Cannon_BuildBoltList` and
`Cannon_BuildMuzzleFlashList`, an exact match for
`ExhaustFlare_BuildDisplayList`'s recovered `sceGuBlendFunc(GU_ADD,
GU_SRC_ALPHA, GU_FIX, 0, 0xffffff)` - see `crate::exhaust::BLEND`'s own doc
comment). **Depth write is on** (`Gu_DepthMask(1)`, read directly at both
list builders) where the exhaust flare's is off - kept as measured rather
than matched to the flare, and noted as unusual in
`oag_fx::weapon_quads`'s own module doc comment. The flash's per-tick
roll is generated render-side (`oag_fx::weapon_quads::random`), seeded
from the round's own pool slot and the simulation tick rather than from
`world.rng`, so drawing it never advances the simulation's own seeded
stream or moves a committed determinism hash.

**Not read and not built**: the small extra particle-like spawn
`Cannon_UpdateRound` also drives inside the same `age < 0.1` window
(`Psys_RandFloatRange(0, 2*pi)` into `instance+0xd0` doubles as an input to
a second computation feeding `FUN_08945284`, the same function
`Cannon_Init` also calls once at spawn) - this reads as a separate small
particle/spark object at the muzzle, not part of either of the two textured
quads, and is out of this pass's scope. Left as an open item below.

## What is buildable now and what still is not

- **The Cannon is buildable, end to end.** Its whole fire-rate mechanism - the
  reload countdown gated on `craft+0x1bc == 3`, the round/rate attributes, the
  twin-muzzle alternation, the craft-speed-inherited round, the round-robin
  spawn pool - is read at instruction level, and so, as of 2026-09-09, is its
  base speed (`Cannon_BaseSpeedKmh`, a flat `500.0`) and its own hit/collision
  behaviour: a world hit throws `WO_CANNON_SPARKS` and stops the round, a
  craft hit applies `damage_per_bullet`/`slowdown_time` and stops it without a
  spark, and anything else bounces. See "`Cannon_UpdateRound` read" above for
  all three. **Its visuals are complete as of 2026-09-17**: the dart mesh,
  the bolt streak and the muzzle flash all draw - see "The two hand-built
  quads' geometry, read - and drawn" above. Only the muzzle's small extra
  particle spawn (`FUN_08945284`) is unread and undrawn.
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
- **The ribbon's own body is buildable too, as of 2026-09-17** - its texture
  (`Data\Weapons\Textures\pulse_leechbeam1_ADD.mip`), its draw call
  (`LeachBeam_BuildStrip`/`LeachBeam_SubmitStrip`), its width, its base
  colour, its disconnect fade and its endpoint taper are all recovered; see
  "2026-09-17: the LeachBeam ribbon's own texture" above and
  `crates/fx/src/beam.rs`. Only the two displacement axes and the
  amplitude re-roll cadence are chosen rather than measured.

## History

This page supersedes the three addresses `weapons-eight-of-thirteen-the-plasma-and-the.md`'s
2026-09-07 entry named as unread: `0x088537ac` (refuted, see above),
`0x0886c600` and `0x08866658` (both now read in full). See that thread for the
narrower, single-session account of how this pass came about.

### 2026-09-24: the round's own matrix, measured live

One round fired from the player's own record on PPSSPP (`pulse-psp-usa.chd`,
Single Race, fire word bit `0x4000` - **not** `0x2000`, which no handler
consumed; `mine.md`'s id table lists the Cannon under `0x2000`, and this
session's live run says the handler this page names clears `0x4000`).
`Cannon_Init` (`0x088648ec`) copies the muzzle anchor into `round+0x50..+0x8c`
and sets the velocity to row 2 times the speed. `0.05 s` and `0.10 s` later:

```text
round+0x50  row0 (-0.0617,  0.0,    -0.9981)   up x f
            row1 (-0.0116,  0.9999,  0.0007)   up
            row2 ( 0.9980,  0.0116, -0.0617)   f = velocity / |velocity|
            det  +1, unscaled
velocity    (696.2, 8.1, -43.0)
```

So the round's own per-tick basis is `(up x f, up, f)`, a rotation - the
shape `Race::projectile_model_matrices` builds, and the Rocket's own. **What
this does not settle is the draw**: the scene node at `round+0xc0` held a
stale, non-orthonormal matrix at a position the round had already left, and
the node hand-off inside `Cannon_UpdateRound` (`0x088661bc`) never fired in
forty seconds of flight. The round's model is therefore drawn from something
other than that node - most likely the per-round draw `FUN_0886545c` using
`round+0x50` directly - which is **unread**. The orientation this engine
draws is the measured round basis; that the original draws the model with
it is an inference, confidence 70.

## 2026-09-30: `~QUAKETRAVEL`, `LEACHFAIL` and the Cannon round's lifetime (pulse-weapon-audio lane)

- **`~QUAKETRAVEL` is triggered by `Quake_Update` (`0x0891d268`), confidence 85.**
  Per road span (two slots): when `Quake_SampleSpan` first reports the span
  active it spawns `WO_QUAKE`, allocates a `SoundEmitter` (`FUN_08946ce4(0x70)`
  + `SoundEmitter_Init`) whose `+0x50` points at the span's own matrix and whose
  radius `+0x38` is `0x44160000` = **600.0**, and plays `~QUAKETRAVEL` (pointer
  cell `0x08a88554`, string `0x08a88544`, the `lw` at `0x0891d954`), keeping the
  handle in a per-span table at `0x08abf5ac`. When the span goes inactive it
  releases the effect, stops the voice (`FUN_089393b8`) and frees the emitter.
  The earlier "never resolved to a disc string" was a Ghidra string defined one
  byte early (`@A~QUAKETRAVEL` at `0x08a88542`), which hid the pointer cell.
  Wired as one held voice at the wave's road midpoint (`Cue::QuakeTravel`).
- **`LEACHFAIL` is `LeachBeam_InitUnlocked`'s cue (`0x08872da8`), confidence
  85**: its last call is `Sound_Play(1.0, param_4, ..., "LEACHFAIL", 0)` (pointer
  cell `0x08a7cc20`, string `0x08a7cc14`), on the emitter `Weapon_FireLeachBeam`
  passes it - `shooter->emitter`, the one `LEACH` uses. An unlocked fire reaches
  it; a locked one plays `LEACH` instead. Wired as `Cue::LeachFail`, the
  player's unlocked arm only (an opponent only fires with a lock).
- **A Cannon round that hits nothing is reaped at `1.0 < age`, confidence 90.**
  `Cannon_Init` (`0x088648ec`) zeroes `round+0x48`, `Cannon_UpdateRound`
  (`0x0886593c`) adds `dt` to it, and `CannonPool_Update` (`0x088582b0`) tests
  `1.0 < round+0x48` first in its teardown gate. This port let a round fly to the
  shared 10 s cap, about 9 s too long; it now takes the 1.0 s branch, silently.

### 2026-10-06: the `WO_QUAKE` frame is established (weapon-visuals lane)

**This corrects the two "orientation remains unestablished" notes above**, which
said the `AiTrack_LocatePosition` result feeds nothing. It does, and the earlier
reading missed it because the struct is read back through a stack slot, not the
call's return. Re-read at instruction level, `Quake_Update` (`0x0891d268`),
confidence 85:

- Row 3 (position) is `(A + B) * 0.5`, the two `Quake_SampleSpan` points
  (`0x0891d970`..`0x0891d9c8`).
- `u = normalize(B - A)` goes to `sp+0xa0`; the output record at `sp+0xb0` is passed
  to `AiTrack_LocatePosition(track, &record, &midpoint, 100.0)` (`0x0891da90`).
  `FUN_0887cf88` -> `FUN_0887c7e8` fills it with a cubic B-spline blend of four
  `SplinePt` records, so it is `SplinePt`-shaped: `+0x20` is the interpolated `down`.
- `Y' = vneg(record+0x20)` (`0x0891daac`), i.e. the track's **up** at the midpoint.
- Row 2 = `normalize(u x Y')`, row 1 = `Y'` re-orthogonalised against row 2 (a no-op
  here), row 0 = `row1 x row2` (`0x0891dae8`..`0x0891dbb0`).

That is the frame `oag_fx::psys::spawn::frame_x`/`across.cross(up)` already builds from
`(across, up)`; the engine passed world up. It now passes `-sample.down`.
The frame is handed to `Psys_Spawn_q` as a flag-1 pointer (`0x0891d88c`, `t0 = 1`).
Test: `quake_orientation_ground_truth::the_quake_frame_leans_with_the_banked_track`.
Cross-title: **not checkable** - `WO_QUAKE` frames on HD/2048/Omega were not read, and
the extent law is Pulse-only, so those sources keep their previous frame.
