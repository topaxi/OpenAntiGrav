# The Autopilot pickup

Weapon id **6**, fire-request bit **`0x1000`**, and the last of the three
"weapons that damage nobody" whose handler had not been read.

[`shield-pickup.md`](shield-pickup.md) recovered the Turbo's and the Shield's and
recorded the Autopilot's handler address on the way past; this page reads it.
The three are the same shape - a fire handler that arms a timer from
`<Weapon><Stats time>`, and an update that counts it down - which is what makes
the Autopilot's differences the interesting part, and there are three of them.

**Static reading of `psp-pulse-usa`'s `BOOT.BIN`. Nothing here is
runtime-verified.**

## Read this before trusting an address

Same trap as [`positional-audio.md`](positional-audio.md): this import carries
**unrelocated address constants**, so a `jal` prints as `0x0003a9b0` and the real
address is that plus the image base `0x08804000`. Every xref tool in the bridge
is silent for the same reason; the call sites below were found with
`search_instructions`, not `get_xrefs_to`.

## The cast

| Address | Name | Confidence |
| --- | --- | --- |
| `0x088613bc` | `Autopilot_Fire` | 88 |
| `0x08861404` | `Autopilot_Update` | 85 |

`WeaponStats_ParseAutopilot` (`0x0880cb2c`, 88) was already named by
[`shield-pickup.md`](shield-pickup.md) and gives `time` at `stats+0x94`.

## `Autopilot_Fire` arms it

```c
// dispatched on request bit 0x1000, from Weapons_DispatchFire
void Autopilot_Fire(World *w, Craft *craft)
{
    craft->fire_flags  |= 0x800;                    // craft+0x1b8, the running bit
    craft->auto_timer   = weapon_stats[+0x94];      // craft+0x148, <Weapon type="Autopilot"><Stats time>
    craft->held         = -1;                       // craft+0x1bc, the pickup is spent
    craft->fire_flags  &= ~0x1000;                  // the request is consumed
}
```

Four stores and nothing else. Confidence **88**: it is the Shield's and the
Turbo's body with three constants changed, and the one constant that could have
been wrong - `stats+0x94` - is the offset `WeaponStats_ParseAutopilot` was
independently read to write.

**The running bit is `0x800` and the request bit is `0x1000`**, which continues
the pattern the other two set: `Shield_Fire` is `0x10` running on `0x20`
requested, `Turbo_Fire` is `0x200` on `0x400`. Each running bit is its request
bit halved.

**`Autopilot_Fire` itself plays nothing** - `Shield_Fire`'s last act is
`Shield_Activate`, which opens `~SHIELD` and the announcer; this handler has
no equivalent call. **Correction, 2026-09-25: the opener is a sibling
function, not "not recovered".** The "what is not recovered" section below
used to claim `~AUTOPILOT` and `autopilot_eng` had no located opener at all;
that was true of `Autopilot_Fire` alone. `Ship_FireHeldWeapon` (`0x08844ae8`,
already named and cited by
[`missile.md`](missile.md#weapon_requestfire-and-the-weapon-id-to-bit-map))
opens both, in its own switch on the held weapon id - see
["`Ship_FireHeldWeapon` opens both cues"](#ship_fireheldweapon-opens-both-cues)
below.

## `Autopilot_Update` counts it down, and warns at one second

```c
// dispatched on running bit 0x800, once per tick, with the frame's dt
void Autopilot_Update(float dt, World *w, Craft *craft)
{
    craft->auto_timer -= dt;                                  // craft+0x148
    if (craft->auto_timer < 1.0f && craft->auto_prev > 1.0f)  // craft+0x144
        Sound_PlayDry(craft->entity, speech_bank, "disengaging", 0x400, 0);
    if (craft->auto_timer <= 0.0f) {
        craft->fire_flags &= ~0x800;
        if (entity->0x58 != 0) { Scream_StopSound(...); entity->0x58 = 0; }
    }
    craft->auto_prev = craft->auto_timer;
}
```

Three things worth naming.

**`craft+0x144` exists only to make the warning an edge.** The test is
`now < 1.0 && previous > 1.0`, and `+0x144` is written unconditionally at the
bottom of every call. A level test would say "disengaging" sixty times a second
for the last second.

**The warning is played dry, not positionally.** `Sound_PlayDry` here is
`FUN_0883e9b0` -> `FUN_0893a768`, the path that takes **no emitter** and is
handed volume `0x400` - the maximum - with pan zero. That is the same path
[`positional-audio.md`](positional-audio.md) identifies as the local player's
own, and it is the right one: an announcer line is in the player's ear, not out
in the world. The cue string is at `0x08a7c6cc`, reached through the pointer at
`0x08a7c6d8`, and it is literally `"disengaging"` - `Data\Sound\speech.bnk` cue
2, two waveforms, 0.80 s.

**A held voice is released at expiry and never opened here.** `entity+0x58` is
one word past the `entity+0x54` that `Shield_Activate` writes `~SHIELD`'s handle
into, so the autopilot has a held loop and this is where it stops. What opens it
is not on this page; see below.

## Absorbing cancels it (was "Firing cancels it")

**Corrected 2026-10-26 (`autopilot-re`): this function is the absorb path, not
the fire path, and the law is "absorb cancels it".** `0x08844ec4` is
`Ship_AbsorbHeldPickup` (named on
[cannon-quake-leachbeam.md](cannon-quake-leachbeam.md), 85): it gates on the
control record's byte `+0x17`, absorb, where `Ship_FireHeldWeapon` gates on
`+0x15`, fire. The paragraph below keeps its original reading and its 85 for
the record; the measured law, with the live run, is in
[the 2026-10-26 section](#absorb-cancels-it-fire-does-not-and-no-corner-holds-it-2026-10-26-autopilot-re).

`FUN_08844ec4`, the player's fire path, opens with:

```asm
08844fe8  lw    a0, 0x1b8(a2)      ; the craft's fire flags
08844fec  andi  a0, a0, 0x800      ; is the autopilot running?
08844ff8  bne   a0, zero, 0x0884502c
...
0884502c  mtc1  zero, f12
08845034  swc1  f12, 0x148(a2)     ; auto_timer = 0.0
```

and only reaches its held-weapon jump table (14 entries at `0x08a2bcc8`, indexed
by `craft->held + 1`) when the bit is clear. So **pressing fire while the
autopilot is running zeroes its timer instead of firing anything**, and
`Autopilot_Update` tears it down on the next tick through its own `<= 0` arm
rather than through a second copy of the teardown. Confidence **85**: the
control flow is unambiguous; what is inferred is only that this function is the
player's fire path rather than some other caller's.

## The HUD prints the remaining time

`PlayerStatus_Update` (`0x0883b3b8`) tests the same `0x800` bit and, when it is set, formats
`craft+0x148` through `"%1.2f"` (`0x08a7b5ac`) into the `SubWeapon` text, and keeps the
Autopilot's icon up when nothing else is held. The compare against `1.0` that follows feeds
`Craft_SetAutopilotBlend` (the controls' blend, below), **not a restyle of the text** - this
paragraph guessed "presumably to restyle" until 2026-10-26. Seen live: the original prints
`5.00` under the hexagon, ours the same glyphs in the same place. Full reading, with the
Cannon's `%d` that shares the line: [sub-weapon.md](sub-weapon.md).

## Which craft: a sixth use of `racer+0x368`

[`pads.md`](pads.md) records the hypothesis that `racer+0x368` separates the
local player from everyone else, and
[`positional-audio.md`](positional-audio.md) took it from 45 to 60 on a fourth
and fifth use. This pass found a **sixth**, and it is the most legible one yet:
`FUN_0883e064` - a craft-explosion spawner in the same cluster - branches on
`craft+0x368 == 0` twice in four lines. On the zero side it takes the **dry,
full-volume** sound path (`FUN_0893a768`, no emitter, volume `0x400`, pan zero)
while the other side gets the positional `Sound_Play`; and on the zero side
alone it calls `FUN_08878750(0.3, 0.4, DAT_08ab10b0, 3)`.

**`FUN_08878750` is unnamed and its argument is the active camera.** It writes an
amplitude at `+0x11c`, a duration at `+0xe4` with its reciprocal at `+0xe0`, a
decay keyframe run it builds by walking a count at `+0x10c`, and a random phase
`rand(0.2, 0.8)` at `+0x120`, all into the camera object `DAT_08ab10b0`. That is
the shape of a **decaying camera oscillation**, which is to say a shake - an
inference from the field set rather than a reading, so nothing is renamed. It is
also a lead for HANDOVER's "the original's camera/HUD shake on impact is
untraced" row, reached sideways.

The point for `+0x368` is what the branch *means*: you play a sound dry and
shake the camera for the player and for nobody else. That is not a sixth
ambiguous use, it is the flag doing the one thing only a "local player" flag
can do. **Still not renamed**: no write site has been found anywhere, and the
naming threshold is about evidence rather than about how convinced the reader
is. Recorded at **65**.

## `Ship_FireHeldWeapon` opens both cues

**2026-09-25, resolving the conflict [`missile.md`](missile.md) and this page
used to carry in opposite directions.** `missile.md`'s own "`Weapon_RequestFire`,
and the weapon-id to bit map" section already named `Ship_FireHeldWeapon`
(`0x08844ae8`) as the opener of `ROCKET`, `QUAKELAUNCH` and `_AUTOPILOT` plus
`autopilot_eng`, off ids 0/2/6 of a switch on the held weapon id; this page's
own "Not determined" section (below, now corrected) independently claimed
neither cue had a located opener at all. `missile.md` was right. Decompiling
`Ship_FireHeldWeapon` directly (`program=BOOT.BIN`, confidence 88 - a direct
read with every string a literal pointer, held back from `missile.md`'s own 92
only because the struct identity note below is inferred, not read) gives:

```c
void Ship_FireHeldWeapon(Entity *param_1)  // 0x08844ae8, local player only - see below
{
    // ... fire-button edge, Ship_State == 1, held != -1, and not
    // (held == 6 && running) gate all elided; Weapon_RequestFire(...) already ran ...
    if (*(int *)(param_1 + 0x368) == 0) {           // the local-player field every
        switch (*(int *)(*(int *)(param_1 + 0x4c) + 0x1bc)) {   // other page in this tree reads at this offset
        case 0:  // Rocket
            Sound_Play(1.0f, *(void **)(param_1 + 0x50), DAT_08ac1df8 /* weapons.bnk */,
                       0, "ROCKET", 0);
            break;
        case 2:  // Quake
            Sound_Play(1.0f, *(void **)(param_1 + 0x50), DAT_08ac1df8 /* weapons.bnk */,
                       0, "QUAKELAUNCH", 0);
            break;
        case 6:  // Autopilot
            param_1->0x58 = FUN_0883e9b0(param_1, DAT_08ac1dec /* hud.bnk */,
                                          "~AUTOPILOT", 0x400, 0);
            FUN_0883e9b0(param_1, DAT_08ac1dfc /* speech.bnk */, "autopilot_eng", 0x400, 0);
            break;
        // ids 1, 4, 5, 7-12: no sound here, just a per-weapon fire-attempt counter
        }
    }
}
```

Settles every open question the two bullets below used to carry:

- **The gate is `entity+0x368 == 0`** - the same "local player" field
  [`pads.md`](pads.md), [`zone-mode.md`](zone-mode.md) and
  [`shield.md`](shield.md) each independently establish at that exact offset -
  so this whole switch, sounds included, never runs for an opponent. Matches
  this port's own `Race::spend_pickup`, which is likewise only ever called for
  `player` (`crates/raceplay/src/tick.rs`).
- **`~AUTOPILOT` goes through `FUN_0883e9b0`**, the same dry, no-position
  helper `Cue::Blowup` and `Cue::Disengaging`
  (`crates/sound/src/sfx/cue.rs`) already cite for the player's own ear
  rather than a craft's emitter - and its return is stored at `param_1+0x58`,
  matching this page's own note two sections up that `Autopilot_Update`
  "stops a held handle at `entity+0x58`". The two now agree on the same
  field from opposite ends: this is that opener. And `Autopilot_Update`'s own
  `<= 0.0f` arm - the timer's expiry, not the `disengaging` warning edge one
  second earlier - is where that same handle is released, so the two sides of
  the loop's own lifetime are both read, not assumed.
- **`autopilot_eng`'s call discards its return** - a one-shot voice line, the
  same shape `Cue::Disengaging` (its own "about to let go" partner) already
  is, not a held loop.
- **`ROCKET` and `QUAKELAUNCH` go through a plain positional `Sound_Play`**
  with `*(param_1+0x50)` as an explicit emitter argument, the same call shape
  [`shield-pickup.md`](shield-pickup.md) reads for `~SHIELD`
  (`Sound_PlayLooping(1.0, entity->0x50, DAT_08ac1df8, 0, "~SHIELD", entity + 0x54)`)
  - a positional cue, not a dry one.
- **The bank pointers cross-check independently.** `DAT_08ac1df8` (weapons.bnk),
  `DAT_08ac1dec` (hud.bnk) and `DAT_08ac1dfc` (speech.bnk) are the same three
  globals [`shield-pickup.md`](shield-pickup.md) and [`pads.md`](pads.md) each
  already read at those addresses for cues already confirmed on those banks -
  they agree here too, on cues this page separately confirms load from exactly
  those banks (`~AUTOPILOT` on `hud.bnk`, `autopilot_eng` on `speech.bnk`,
  above).
- **No underscore/tilde trap here, despite first appearances.** The bridge's
  own decompile rendered the string argument as a symbol,
  `PTR_s__AUTOPILOT_08a7b6af_1_08a7b6bc`, which reads like the `_LEACHATTACH`
  / `~LEACHATTACH` split `cue.rs`'s own `Cue::LeachAttach` doc comment warns
  about. It is not: `read_memory` at `0x08a7b6ac` (`program=BOOT.BIN`) shows
  the bytes directly - `7e 41 55 54 4f 50 49 4c 4f 54 00 00`, `"~AUTOPILOT\0"`,
  tilde included, at `0x08a7b6b0`. Ghidra substitutes `_` for a
  non-identifier byte (the `~`) when it auto-names a label, and the label's
  own `_1` suffix is "one byte past `08a7b6af`", which is exactly
  `08a7b6b0`. So the pseudocode above already had it right, and confirmed
  independently against `pulse-psp-usa.chd`'s own bank: `oag-wad sounds`
  lists `~AUTOPILOT cue 0 cmds 0..4 3 waveform(s), 1 looping, 3.04s total`
  and `autopilot_eng cue 0 cmds 0..2 2 waveform(s), 0 looping, 3.14s total`.

**Not settled by this read**: whether `param_1` here is the same "entity"
struct every `+0x368`/`+0x860`/`+0x85c` reference elsewhere in this tree
names directly, or a distinct "player controller" wrapper that merely mirrors
those fields at the same offsets and holds a genuine pointer-to-craft at
`+0x4c` (the read that would explain the extra dereference this function
takes and every flat `craft->1bc` elsewhere does not). Nothing here turns on
which is true - the sound-opening behaviour is read either way - so this is
left as a loose end rather than chased further.

## What the running bit does to the craft (2026-10-19, `autopilot-rail`)

Read at instruction level in `PlayerStatus_Update` (`0x0883b3b8`, the local
player's per-frame status), then measured live. The maintainer's question was
whether the original's autopilot "snaps harder to the spline" than ours, since
ours still touches walls. **It does not.** The rail is the opponents' rail with
no player gain, and the original's autopilot touches walls too. What the running
bit adds is pace, and pace stiffens the rail through its speed gate.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08848664` | `Craft_SetAutopilotBlend` (already named, [race-finish.md](race-finish.md)) | 90 |
| `0x0884866c` | `Craft_SetUncappedThrust` | 88 |

`Craft_SetUncappedThrust(craft, on)` is seven instructions: `on & 0xff` set
ORs `0x8` into `craft+0x1c0`, clear ANDs it out (`0x0884866c..0x0884868c`).
The only masked read of `craft+0x1c0` against `0x8` in the image is
`Ship_UpdateEngine`'s at `0x0884c7d0` (`psp-relocate.py masked 0x1c0 0x8`,
one hit), where it makes the acceleration cap `1e10`
([engine.md](engine.md#engine)). Against `0x4` the only one is
`0x0884c874`, the `T *= craft+0x2a0` multiplier.

`PlayerStatus_Update`, for the racing player (`entity+0x912 == 0`, ship state
not `2`):

```c
if ((pickup->fire_flags & 0x800) == 0) {           // not running
    if (!(entity->0x860 & 8) && !(entity->0x860 & 0x10)) {
        Craft_SetAutopilotBlend(craft, 0.0);
        Craft_SetUncappedThrust(craft, 0);
        craft->0x2a0 = 0; craft->flags_1c0 &= ~4;
    }
} else {                                           // running
    // HUD: "%1.2f" of the timer
    Craft_SetAutopilotBlend(craft, timer > 1.0 ? 1.0 : timer);   // 0x0883be54..0x0883be98
    Craft_SetUncappedThrust(craft, 1);                           // 0x0883bea4
    craft->flags_1c0 |= 4;  craft->0x2a0 = 1.2f;                 // 0x0883beb0..0x0883bec0
}
```

and for a finished player or ship state `2`: blend `1.0` and uncapped, with
`0x4` left as it was (`0x0883bdd0..0x0883bdf8`). `entity+0x6c != 0` also
forces blend `1.0` and uncapped; what sets that byte is not read.

**The handover.** `Ship_UpdateCraft` only tests the blend against zero
(`0x08849690..0x0884969c`): non-zero copies the driver's whole record over the
pad's (`0x088496b8..0x08849768`). `FUN_088418e0` also only zero-tests it
(`0x088419bc`): it picks which of two per-profile distance counters the frame's
distance goes into. No handling reader uses the magnitude: the controls switch on
zero against non-zero. One load is unchecked: `Hud_UpdateEnergyBar` loads
`0x1d4(s0)` (`0x0881c8d0`), and its base was not traced, so a HUD element may
read the fade. The original hands over the controls in one tick, both ways, and
ours already matches. Confidence 85 for the controls.

**Nothing that reads the running bit touches the walls.** `psp-relocate.py
masked 0x1b8 0x800` finds five readers, the complete set for a `lw`/`andi`
pair:
- `0x0883be0c`: the branch above;
- `0x0883f0c0`, in `FUN_0883efb4`: the first call of the per-craft weapon
  update `FUN_0883f540`, which skips `0x08862bb4` while the bit is set;
- `0x08844b44`: `Ship_FireHeldWeapon`'s cue;
- `0x08844fe8`: the fire path's cancel;
- `0x08861c3c`: `Weapons_DispatchFire`'s call to `Autopilot_Update`.

None reaches the contact ring or the wall response. Confidence 85.

### Measured live

PPSSPP v1.20.4, `pulse-psp-usa`, Single Race, the player's pickup armed 60
frames after GO with the timer raised (the `psp-postrace.py` method), so the
original's own player driver flies the whole race.
`scripts/psp-autopilot-rail.py` stops once a frame in `Ai_SnapToLane` at
`0x08854340`, after `Ai_LanePoint` has written the lane point to `sp+0x30`, and
logs the lane point, the body pose, the rail's blend and the contact ring
(`body+0x170`, count `body+0x370`). A ring record whose kind word is `1` comes
from `Body_RecordPairContact` (craft against craft), and `0` from
`Body_RecordContact` (the world).
[contact-response.md](contact-response.md) has both. Lateral offset is
`dot(pos - lane_point, body left)` while the rail's blend is above zero.

| Run | Frames | `craft+0x1c0` | Wall-contact frames (episodes) | Pair records | Mean speed | Mean blend | Lateral p50 / p95 / p99 / max |
| --- | ---: | --- | --- | ---: | ---: | ---: | --- |
| Talon's Junction White (`16_Track`), VENOM, whole race | 6,583 | `0xd` every frame | **0** (0) | 129, all in the first 315 frames | 139.6 | 0.87 | 0.22 / 1.01 / 1.67 / 5.24 |
| de Konstruct Black (`05_Track`), VENOM, laps 1-2 and part of 3 | 3,981 | `0xd` | **16** (3, at 0, 16 and 453 frames after arming) | 278 | 132.1 | 0.82 | 0.19 / 1.29 / 8.34 / 20.11 |
| Fort Gale Black (`14_Track`), PHANTOM (speed class `3`), whole race | 7,698 | `0xd` | **0** (0) | 121 | 211.0 | 0.88 | 0.52 / 2.44 / 5.42 / 10.46 |

The circuit was identified each time by the start position against
`oag-trace track` (6 and 11 units off a spline row, heading with the forward
file). Mean speed and blend skip the first 300 frames.

**The live answer: no hard snap, and the original scrapes too.** It touches
de Konstruct Black's walls about as often as ours does. Lateral offset stays
around a fifth of a unit, as for an opponent. The `0xd` confirms both engine
bits on every frame of the pickup.

**A read-order trap, for whoever logs the pitch next.** At `0x08854340` the
driver's pitch output `driver+0x18` reads `0.0` on every airborne frame. That
is because `Ai_ComputeControls` writes the record earlier in the same
`Ai_Update` and `Ai_ComputePitch` (`0x08853148`) runs after `Ai_SnapToLane`.
At `Ai_ComputePitch`'s return (`0x088533b4`, conditional on the player's
driver), 231 of 231 airborne frames held a non-zero pitch. The controller does
fly the player's pitch.

### The other titles (2026-10-19, `hd-autopilot-law`)

Every title's executable carries this branch: HD's `PlayerStatus_Update`
(`0x000cb928`) at `craft+0x264`/`craft+0x2f0`, read live on RPCS3 (`0xd`, `1.2`);
Pure, Omega and 2048 read off their code. Addresses and the Ubermall comparison:
[ps3-hdfury-eu/autopilot.md](../ps3-hdfury-eu/autopilot.md).

## Not determined

**What flies the craft is now read** (2026-10-19): the player's own driver, the
opponents' `Ai_Update` with lane `0`, throttle `100` and the rail kept through
contact ([ai-controls.md](ai-controls.md#the-players-own-driver-the-autopilot)).

- **Where the control source is actually swapped. Found 2026-10-11**
  ([ai-controls.md](ai-controls.md#who-drives-whom-the-control-record)):
  `Ship_UpdateCraft` reads the craft's controls through `craft+0x78`, which is
  `craft+0x3c` (the pad record) unless the blend `craft+0x1d4` is non-zero, when
  it copies `craft+0x40`'s record, the `"autopilot input"` below, over it. The
  blend's setter is now read (2026-10-19, above): `PlayerStatus_Update`. The original note:
  `Ai_Construct` (`0x088536bc`)
  names the local player's input `"autopilot input"`, which
  [`weapon-stats.md`](../../../formats/weapon-stats.md) already records, so the
  player's craft carries a driver object for exactly this. Which code reads bit
  `0x800` and switches to it was not found - `FUN_0883efb4` and
  `Ship_FireHeldWeapon` both test the bit and neither is it.
- **The duration itself is not a constant here.** It is
  `<Weapon type="Autopilot"><Stats time>` off the disc, which
  `oag_tables::weapons` already parses; no code literal stands in for it.
- **`Weapons_DispatchFire`'s own structure.** It tests a long run of bits on
  `craft+0x1b8`; this page assumes the `0x800` test at `0x08861c40` is what calls
  `Autopilot_Update`, by analogy with the Shield's pair, and did not verify it.
- **Nothing here is runtime-verified.** One PPSSPP breakpoint on
  `Autopilot_Fire` with the pickup collected would settle the duration and the
  teardown in a single run. *(2026-10-26: the countdown, the teardown and the
  cancel are now watched live, in the section below.)*

## Absorb cancels it, fire does not, and no corner holds it (2026-10-26, `autopilot-re`)

The maintainer, 2026-10-26: "autopilot powerup should be able to be cancelled (I
think via the absorb button? unsure, needs recovery) and there's also some logic
in the original that prevents it from automatically stopping at sharp turns."
The first half is right. The second is not, on Pulse, and not on HD or Pure as
read.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0883f540` | `Ship_UpdateWeapons` | 80 |
| `0x0883efb4` | `Ship_TestWeaponPads` | 65 |

`Ship_UpdateWeapons` is every craft's per-tick weapon step: `Ship_TestWeaponPads`,
the pending weapon damage and repair, the collision impulse, the lock for held
ids `1` and `10`, then, when the craft has a control record
(`*(craft+0x94)+0x78 != 0`), `Ship_FireHeldWeapon`, `Cannon_UpdateReload` and
`Ship_AbsorbHeldPickup` in that order. Its one caller is `Ship_UpdateEntity`
(`0x0884230c`), and only when the race settings (`0x08b30f90`, `s4` from
`0x08841918`) have the byte `+0x18` set or the game mode is one of a short list
(`0x08842270..0x088422fc`). That byte is the race's weapons switch: in a Weapons
OFF race neither button reaches a pickup path at all, which the first live run
below ran into.

### The law

1. **Absorb cancels a running Autopilot**, in every mode, with or without a
   pickup held. `Ship_AbsorbHeldPickup` tests the absorb byte and
   `Ship_State == 1`, then (outside Eliminator) goes straight to
   `craft+0x1b8 & 0x800` (`0x08844fe8`) and, when it is set, stores `0.0` to the
   timer `+0x148` (`0x08845034`) and skips the whole absorb switch, so a held
   pickup is neither paid out nor spent. `Autopilot_Update` tears it down through
   its own `<= 0` arm on the next dispatch. In Eliminator (game mode `8` or
   `0x12`) a held pickup is first requested as a fire (the Shield there) and the
   same cancel follows, which is the edge `crates/raceplay/src/eliminator.rs`
   records. **Confidence 95**: static, and the live run below.
2. **The player's absorb reaches that test under the Autopilot, and fire does
   not.** `Ship_UpdateCraft` copies the pad record's absorb byte into the
   driver's record: `craft+0x78` holds `*(craft+0x3c)`, the pad record, from
   `0x08849638`; `lbu a0,0x17(a0)` at `0x0884967c`; `sb a0,0x17(a2)` into
   `a2 = *(craft+0x40)` at `0x08849684`. With the blend non-zero it then copies
   the pad record into `craft+0x44`, copies the driver's whole record over it
   (`0x088496b8..0x08849768`, ten words) and points `craft+0x78` there. The fire
   byte `+0x15` that reaches `Ship_FireHeldWeapon` is the driver's own.
   **Confidence 92**: static, and the live run below.
3. **A held Autopilot cannot be fired into a running one.**
   `Ship_FireHeldWeapon` refuses `held == 6 && (flags & 0x800)` (`0x08844b44`)
   and leaves the pickup held. **Confidence 90**, static and live.
4. **No pickup is collected while it runs.** `Ship_TestWeaponPads` asks
   `WeaponPads_TestCraft` only with an empty slot, and calls the grant request
   `FUN_08862bb4` (a store of `1` to `record+0x1c4`; not renamed, "grant
   pending" is inferred from this one use) only when `craft+0x1b8 & 0x800` is
   clear (`0x0883f0c0`). `Autopilot_Fire` spends the slot (`held = -1`), so the
   slot stays empty for the whole stint and fire has nothing to fire.
   **Confidence 80**, static only.
5. **Nothing holds it through a corner.** The timer `+0x148` has three writers
   on a pickup record (`psp-relocate.py field 0x148`, every `sw`/`swc1`):
   `Autopilot_Fire` (`0x088613ec`), `Autopilot_Update` (`0x0886142c`) and the
   absorb cancel (`0x08845034`); the other hits are the body's velocity
   (`body+0x140..0x148`) and unrelated structures. `Weapons_DispatchFire` calls
   `Autopilot_Update` with the frame's `dt` whenever the bit is set
   (`0x08861c40..0x08861c5c`, `f20` from its own `f12`), with no test of track
   state; the byte `0x08ab0fec` at its top is a one-time init latch. The release
   in `PlayerStatus_Update` has two guards, `entity+0x860 & 0x8` and `& 0x10`
   (`0x0883beec..0x0883bf18`), and **neither bit has a setter**: every `sw` to
   `+0x860` (55 sites) was read with up to 96 bytes before it, `masked 0x860
   0x8`/`0x10` find the release as the only reader, and only `Craft_Construct_q`
   clears them. The byte `entity+0x6c`, which forces blend `1.0` when set, has
   byte stores only in `Craft_Construct_q` (a clear), `Ai_Construct` and
   `FUN_08854e7c` (the driver's own `+0x6c`), `FUN_0887eba8` (not read) and an
   unrelated global's setter. A word store to it is not excluded statically, so
   it was watched live instead. **Confidence 90** for "no corner hold", static
   and live.

### Measured live

PPSSPP v1.20.4, `pulse-psp-usa`, the saved Single Race
`race-talons-venom-alongside` (Talon's Junction White, VENOM, Weapons OFF, the
player's Autopilot already running at `1e9`). `scripts/psp-autopilot-cancel.py`
stops once a frame on `Weapons_DispatchFire` and logs the pickup record, the
blend, the engine bits, the record's two button bytes, `entity+0x860`,
`entity+0x6c` and `entity+0x912`. `--weapons-on` writes the race's weapons byte
`0x08b30fa8 = 1` after the load. Without it the absorb press below did nothing
(the absorb byte read `1`, the timer stayed at `1e9`), which is
`Ship_UpdateEntity`'s gate above.

| Phase (weapons byte set) | Input | What the original did |
| --- | --- | --- |
| absorb, empty slot | CIRCLE, 4 frames | absorb byte `1` on frame 22; timer `1e9 -> 0.0` the same frame; frame 23 bit `0x800` clear, timer `-0.017`, engine bits `0xd -> 0x1`, blend `0.0` |
| fire, empty slot | SQUARE, 4 frames | the record's fire byte stayed `0`; nothing changed for 30 frames |
| fire, held Autopilot (`held = 6` written) | SQUARE, 4 frames | held stayed `6`, the timer stayed at `1e9`, nothing changed |
| expiry, racing | timer set to `0.25 s`, re-armed on the frame it ran out | 152 of 152 expiries released: blend `0.0` and engine bits `0x1` on the expiry frame, the pad record selected on the next. The sharpest averaged 4.86 degrees of heading a frame over the six frames before (about 290 degrees a second), the next 3.76, 2.48 and 2.42 |
| expiry, finished | the same, after the finish | 108 of 108 kept blend `1.0` and bits `0xd`: `PlayerStatus_Update`'s finished-racer arm, the after-the-finish flight, not a hold |

`entity+0x860 & 0x18` and `entity+0x6c` were never non-zero across the 4,596
frames. **The timer is not extended either:** the 152 racing expiries came every
17 or 18 frames (a `0.25 s` stint, the expiry frame and the two logged unarmed
frames), through the sharpest corner as on the straights, so a corner neither
delays the expiry nor gates the release. The countdown runs on through a corner and releases mid-corner. What can
read as a hold is only that the controls hand back in one tick with the craft
already at the Autopilot's speed and line.

### Ours

**Wired 2026-10-26 (`autopilot-wire`).** Until then `Race::spend_pickup` returned on an
empty slot before it reached its cancel, and its cancel sat on SQUARE. In the
original's flow the slot is always empty under a running Autopilot, so in play ours
cancelled on **neither** button, which is what the maintainer saw, and a pad granted
mid-stint. Measured on that tree with a scratch test: armed through the pickup, then
CIRCLE and SQUARE with the slot empty, `flown_for_the_player` stayed `true` after both.

Now a press under a running Autopilot is `Race::spend_pickup_under_autopilot`'s
(`crates/raceplay/src/field.rs`): an absorb zeroes the timer (before the empty-slot
test, so with nothing held too), pays nothing and spends nothing, and the controls
are the person's again on the next tick; a fire press is ignored, which also keeps a
held Autopilot from being fired into a running one. In an Eliminator a held weapon is
first requested as the Shield (`Race::eliminator_absorb`) and the cancel follows.
`Race::test_weapon_pads` stamps the pad as before and grants nothing to a craft whose
timer runs. Both are per-title data on `oag_title::weapons::Weapons`
(`autopilot_absorb_cancels`, `autopilot_pad_grants`, each `Sourced`): Pulse measured,
HD measured (static) for the cancel, Pure `false` (measured absence, static), 2048 and
Omega inherited from HD, and the pad gate Pulse's everywhere but Pulse itself
(inherited: HD's and Pure's pad grants are unread). The fire suppression and the
refusal of a held Autopilot are not per-title: they follow from the driver flying the
craft, and no title's executable was read to differ.

**One thing is inferred rather than measured: the `disengaging` call on a cancel.**
`Autopilot_Update` above runs on the next dispatch with the timer at `-dt` and
`craft+0x144` still holding last frame's value, so with more than a second left its
`now < 1.0 && previous > 1.0` edge is crossed and the announcer speaks. Ours raises
`Cue::Disengaging` from the cancel when more than a second was left (confidence 85,
static; the live run above logged state, not cues, and it has not been heard).
Pinned by `cancelling_with_time_left_says_disengaging_and_the_last_second_does_not`.

Disc-backed trace, `crates/raceplay/tests/autopilot_cancel_ground_truth.rs` on Pulse
(Pure's reads the same up to the last two rows, where its timer runs on):

```text
tick  333  arm      buttons 0x00a0  timer   4.983  flown true   slot None
tick  335  fire     buttons 0x00a0  timer   4.950  flown true   slot Some(Rocket)
tick  337  refire   buttons 0x00a0  timer   4.917  flown true   slot Some(Autopilot)
tick  339  absorb   buttons 0x0030  timer   0.000  flown false  slot None
tick  340  after    buttons 0x0020  timer   0.000  flown false  slot None
```

### The other titles

- **HD/Fury: the same cancel.** `Ship_UpdatePickup_q` (`0x000e9160`) tests the
  control record's byte `+0x1b` and the state word `ship+0x5fa4 == 1`, then
  `slot+0x200 & 0x800` (`0x000e92a0..0x000e92b0`), and stores `0` to the timer
  `slot+0x158` (`0x000e92b8`). `0x000f1958` copies the pad record's `+0x1b` into
  the driver's (`lbz r0,0x1b(r6)` / `stb r0,0x1b(r5)` at `0x000f1a08..0x000f1a10`)
  right before its blend test, the shape of Pulse's pass-through, so `+0x1b` is
  HD's absorb byte. `Autopilot_Update` (`0x00129988`) has no hold; at expiry,
  when `entity+0x628c == 0`, it also calls `FUN_00676a38` and then `FUN_006778e8`
  (not read). The release guards
  `ship+0x6a64 & 0x8`/`0x10` have no `ori`/`rlwimi` setter in the eight
  instructions before any of the 60 `stw` to `+0x6a64`, and nothing stores a
  byte at `+0x6a67`. Static only, **confidence 80**.
- **Pure: no cancel, no hold.** `Autopilot_Fire` (`0x0884c770`) and
  `Autopilot_Update` (`0x0884c794`) keep the running flag as a byte (`+0x154`)
  and the timer at `+0x15c`, see
  [psp-pure-usa/autopilot.md](../psp-pure-usa/autopilot.md). Static only,
  **confidence 70** for "absorb does not cancel in Pure".
- **2048: not checked.** The `disengaging` cue is passed at
  `0x8120ed52..0x8120ed5e` of the v1.04 executable, inside a function Ghidra has
  not delimited; its update was not read. Omega not checked. Ours runs HD's law
  on both, `Origin::InheritedFrom`, by the rule that an unmeasured 2048 rule takes
  HD's through Omega (checked, applies, wired as inherited; their own update and
  cancel stay unread).
