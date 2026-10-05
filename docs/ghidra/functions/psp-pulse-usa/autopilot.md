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

## Firing cancels it

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

`FUN_0883b3b8` tests the same `0x800` bit and, when it is set, formats
`craft+0x148` through `"%1.2f"` (`0x08a7b5ac`), then compares the timer against
`1.0` and branches - a second consumer of the same one-second boundary the
warning uses, presumably to restyle the text. Not chased further; recorded so the
next reader has the address.

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

## Not determined

- **Where the control source is actually swapped.** `Ai_Construct` (`0x088536bc`)
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
  teardown in a single run.
