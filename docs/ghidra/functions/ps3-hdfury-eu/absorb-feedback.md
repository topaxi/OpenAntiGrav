# HD's weapon-absorb burst: six `absorb` locators, three mirrored pairs

2026-09-23. Prompted by a report from someone who plays the originals: the
weapon-absorb effect "does not animate/play in any title (the sfx plays
though)". Pulse's absorb burst was already read
([`psp-pulse-usa/shield.md`](../psp-pulse-usa/shield.md),
`Ship_PlayAbsorbFeedback`). This page reads HD's, and it is **not** Pulse's
loop on HD's locators. It uses a different node class and a different
stagger, and it fires in pairs.

## The chain

```
Ship_PlayAbsorbFeedback      @ 0x000d9398   (was .opd.FUN_000d9398)
  -> ShipAbsorbNode_SpawnBurst @ 0x002d8d10 (was .opd.FUN_002d8d10), six times
Ship_GatherAbsorbNodes       @ 0x000d2f58   (was .opd.FUN_000d2f58) fills the six slots
ShipAbsorbNode_RegisterClass @ 0x002d8bb8   (was .opd.FUN_002d8bb8) registers class 0x3ee
ShipAbsorbNode_Construct     @ 0x002d8e58   (was .opd.FUN_002d8e58) stamps the type token
```

How it was found: `search_strings "(?i)absorb"` on
`/hdfury/EBOOT-ps3-hdfury-eu.elf` lists `WO_WEAPON_ABSORB` at `0x007a1940`,
directly after `ShipAbsorbNode_Importer.cpp`. The only word holding that
address is the TOC slot `0x008b3e8c`, which is displacement `+0x69b4` from
the default TOC `0x008ad4d8`. `search_instructions "0x69b4(r2)"` finds one
positive-displacement load, and that load is in `ShipAbsorbNode_SpawnBurst`.

## `ShipAbsorbNode_RegisterClass` and `ShipAbsorbNode_Construct`: class `0x3ee` is the absorb node

The static registration calls the class registrar with id **`0x3ee`**, sets the
importer name `ShipAbsorbNode_Importer.h`, and installs the type pointer from
TOC `0x008b3e40`, which is `0x00874598`. The constructor writes the same
`0x00874598` into the node's `+8`. That is the field the collector below
compares against. `0x3ee` is also the id
[`vex-classes.md`](vex-classes.md) names `absorb`, so this is where that
class goes.

Confidence **88**. Both are short, direct decompiles, and one literal ties
them together.

## `Ship_GatherAbsorbNodes` (`0x000d2f58`): up to six, depth-first, under the ship's root

The collector clears the six words `craft+0x7a0c..+0x7a20`. It then takes the
root at `craft+0x6af0` and walks its subtree with `FUN_00683cf8`, keeping
every node whose `+8` equals the absorb type pointer (`PTR_PTR_008a8b4c`,
also `0x00874598`), up to six of them. The count goes in `craft+0x7a2c`. This
is the same shape as Pulse's `Ship_GatherCollisionFxNodes`
([`psp-pulse-usa/shield.md`](../psp-pulse-usa/shield.md)): the same
collector with a different class and a cap of six instead of ten.

**The disc agrees.** Across all 39 `Locators.vex` on the HD disc, 37 carry
exactly six class-`0x3ee` nodes, named `Absorb_1` to `Absorb_6`. The other
two, Detonator's (`DATA00`) and Zone's (`DATA02`), carry none. Each set is
mirrored left to right: on Feisar, `Absorb_1`/`Absorb_6` sit at the wingtips
(`x = +-2.687`), `Absorb_2`/`Absorb_5` just inboard, and `Absorb_3`/`Absorb_4`
nearest the centreline. The fixed pairing below depends on that symmetry.
Three hulls (Auricom, Harimau, Mirage) store the six out of name order.
Collection follows file order, not names, so the pairs are by slot. Measured
with a throwaway node lister over every `Locators.vex` extracted by
`just psarc`. `crates/game/tests/absorb_ground_truth.rs` pins Feisar's count
through the loader.

Confidence **85**.

## `Ship_PlayAbsorbFeedback` (`0x000d9398`): three mirrored pairs, 0.2 s apart

In outline:

```c
visible = any viewport's craft passes the per-viewport mask test;
if (craft->0x628c == 0 && RaceManager()->0x1970 == 2) {
    play "ABSORB" (gated further on a network flag);
    if (visible && craft->count_0x7a2c && slot[0] && slot[5]) {
        g_delay = 0;            ShipAbsorbNode_SpawnBurst(slot[0]);
        g_delay = 0;            ShipAbsorbNode_SpawnBurst(slot[5]);
        if (slot[1] && slot[4]) {
            g_delay = s;        ShipAbsorbNode_SpawnBurst(slot[1]);
            g_delay = s;        ShipAbsorbNode_SpawnBurst(slot[4]);
            if (slot[2] && slot[3]) {
                g_delay = s + s; ShipAbsorbNode_SpawnBurst(slot[2]);
                g_delay = s + s; ShipAbsorbNode_SpawnBurst(slot[3]);
            }
        }
    }
} else if (visible) {
    play "ABSORB" at the craft's own emitter;   // the other branch: a remote craft, read as such
}
craft->0x7a5c = settings->0x58;
```

- `g_delay` is the global at `PTR_DAT_008a8e68` (`0x00ad8f10`). It plays the
  same role as Pulse's `DAT_08abf564`: a one-shot start delay that the
  particle attach consumes.
- `s` is `settings->0x54`, where `settings` is `PTR_DAT_008a8c6c` =
  `0x008c15e0`. `read_memory` at `0x008c1634` gives `3e4ccccd` = **`0.2`**.
  So the pairs start at `0.0`, `0.2` and `0.4` s: the wingtips first, then
  each pair nearer the centreline.
- `settings->0x58` at `0x008c1638` is `1.0`. It is written to `craft+0x7a5c`
  on every call. That is the absorb shell's timer; its consumers are read in
  "The absorb shell" below.

Confidence **80** for the pair structure and the gates, which is a direct
decompile. **70** for the delay semantics: the global is written before each
spawn and zeroed around the outer pair, exactly as Pulse's is, but its
consumer inside `ShipAbsorbNode_SpawnBurst`'s callee was not read on this
binary. **65** for `0.2` being the runtime value: it is `.data`, and no writer
to that settings block was looked for.

## `ShipAbsorbNode_SpawnBurst` (`0x002d8d10`): the system parented to the node

This allocates a `0x180`-byte particle node, links it under the absorb node
(`FUN_003238f8(node, psys)`), and starts it by hash `0x4f534241` (`ABSO`). That
hash is the one `ShipCollisionFx_Trigger` uses for the same effect on Pulse,
and `WO_WEAPON_ABSORB` is the string beside it in `.rodata`. Because the
system is a child of the locator, the burst rides the hull. Confidence **80**.

## Ported

**Four callers**, the same set Pulse's twin has (read on this binary with
`get_function_callers`):

- `0x000d9d60`: `FUN_000d9b18` of a stats-table value, indexed by
  `g_GameState+0xdc`, times `DAT_008a8db4`, then the feedback. It has the
  shape of Pulse's `Ship_RefillLapShield` (a skill-indexed maximum times
  `0.2`), so it reads as the lap-refill twin, which is why the port fires HD's
  burst from the Eliminator refill too. `FUN_000d9b18` and the constant were
  not read; confidence **65** for the identification.
- `0x000d9cf8`: `FUN_000d9b18()`, the feedback, then a store of `5` into
  the craft's `+0x5edc` object's `+0x204`. Not placed.
- `0x000d9688`: a network message handler on `'B'` and `'Q'`, as on Pulse.
- `0x000e9160`: not read. By elimination it is the pickup absorb handler.

`oag_game::race::absorb::HD_ABSORB_BURST` (`MirroredPairs { stagger: 0.2 }`)
plays one `WO_WEAPON_ABSORB` per slot on the pairs above, through
`psys::Stage::play_riding`, which follows the locator while it emits. The
`absorb` locators come from `Locators.vex` through
`oag_vex::vex::CLASS_ABSORB` and `livery::absorb`. A hull with fewer than six
plays nothing, which is the original's own gate. Detonator and Zone therefore
play no burst.

## The absorb shell: `AbsorbEffect.vex`, faded by `ShieldColour` off `craft+0x7a5c`

2026-09-23, later. The burst above is authored faint on HD. The prominent
absorb picture is a second model per team, drawn over the hull. It is **not**
driven by `AbsorbFader`/`AbsorbScroller`; see the negative result below.

```
ShipAbsorbShell_Load     @ 0x000dba30  (was .opd.FUN_000dba30) loads both shells
ShipAbsorbShell_Show     @ 0x000db6f8  (was .opd.FUN_000db6f8) shows one, hides the other
ShipAbsorbShell_Step     @ 0x000cef50  (was .opd.FUN_000cef50) relax + timer decay
ShipAbsorbShell_Update_q @ 0x000db990  (was .opd.FUN_000db990) target + visibility
```

### `ShipAbsorbShell_Load` (`0x000dba30`): two nodes, one parameter

Called from `FUN_000ddd58` (`0x000df788`) and `FUN_000dfd90` (`0x000e17c4`).
It clears `craft+0x7a40`/`+0x7a44`, hashes the string at TOC `0x008a8ed4`
(`0x00782148`, **`ShieldColour`**), and then builds two scene nodes the same
way: allocate `0x2080` bytes, link the node under the craft
(`FUN_003238f8(craft, node)`), load the file (`FUN_002c1ec8`, then
`FUN_002c0890`), clear the node's bit `4` at `+0x34` (hidden), and bind the
model's `ShieldColour` parameter to `&craft+0x7a50`
(`FUN_00677008`/`FUN_00677018(..., ~hash, craft+0x7a50)`).

| Node | Path | Format string |
| --- | --- | --- |
| `craft+0x7a40` | `Data\Weapons\vr_absorbinternal_cockpit.vex` | `0x00782158` `%s\vr_absorbinternal_cockpit.vex` with `0x00782180` `Data\Weapons` |
| `craft+0x7a44` | `<team dir>\AbsorbEffect.vex` | `0x00782190` `%s\AbsorbEffect.vex` with `*(craft+0x6298)+0x1b4` |

Between the two loads it sets `+0x7a58 = 0.15` (TOC `0x008a8ee0`, the value
itself) and zeroes `+0x7a50`, `+0x7a54` and `+0x7a5c`. The strings are read
with a throwaway ELF reader over the decrypted `EBOOT.elf`. Confidence **85**.

### The four floats at `craft+0x7a50`

| Offset | Role | Written by |
| --- | --- | --- |
| `+0x7a50` | the fade, the value `ShieldColour` reads | `ShipAbsorbShell_Step` |
| `+0x7a54` | its target | `ShipAbsorbShell_Update_q`, and the inline copy in `FUN_000e41b0` |
| `+0x7a58` | the rate, the share of the gap closed per step | the same |
| `+0x7a5c` | the timer, seconds | `Ship_PlayAbsorbFeedback` (`1.0`, above), `ShipAbsorbShell_Step` |

### `ShipAbsorbShell_Step` (`0x000cef50`): relax, then count down

```c
n = (int)(dt * 59.999996f);        // DAT_008a8ab8 = 0x426fffff, fctiwz truncates
for (i = 0; i < n; i++) fade += (target - fade) * rate;
timer -= dt;
```

Its OPD has no reference anywhere in the image (a word scan for `0x008744b8`
finds none). The live copy is **inlined in `FUN_000eadb8`** at
`0x000ebff0..0x000ec05c`, identical instruction for instruction, with `f26` as
`dt`, cross-checked with `llvm-objdump`. At exactly `dt = 1/60` the `f32`
product rounds to `1.0`, so a 60 Hz tick is one step. Confidence **80**.

### `ShipAbsorbShell_Update_q` (`0x000db990`) and the head of `FUN_000e41b0`

The same body twice: `ShipAbsorbShell_Update_q`'s OPD is also unreferenced,
and `FUN_000e41b0` (whose OPD sits in the vtable word at `0x008635a4`) opens
with it inline.

```c
if (timer > 0.0) {                   // DAT_008a8b00 = 0.0
    rate   = 0.1;                    // DAT_008a8abc
    target = settings->0x6c;         // 0x008c164c = 1.0
    ShipAbsorbShell_Show(craft);
} else {
    target = 0.0;
    if (fade <= 0.01)                // DAT_008a8afc
        hide both shells;            // +0x34 &= ~4 on +0x7a40 and +0x7a44
    else
        ShipAbsorbShell_Show(craft);
}
```

So the rate is `0.1` whenever it matters. The initial `0.15` is only ever
used while target and fade are both zero. Confidence **70**; the `_q` is for
the name, since the stand-alone function is dead and the reading is off its
inline twin.

### `ShipAbsorbShell_Show` (`0x000db6f8`): the external shell or the cockpit one

Ghidra's decompile of this function stops at its first `bl 0x00327500`, so
this is read off `llvm-objdump -d --mcpu=pwr6 --start-address=0xdb6f8
--stop-address=0xdb990` instead. `FUN_00327500(node, matrix, 0)` installs a
node's local matrix.

- **`g_PhysicsHalfStep` (`0x008a8c24`) clear and `craft+0x5f42 == 0`**: hide
  the cockpit shell, give the external shell the identity matrix
  (`*(0x008a8c88)` = `0x00769d70`, four identity rows), show it (`|= 4`).
- **`craft+0x5f42 != 0`**: give the cockpit shell the identity with row 3
  replaced by `vsel(identity row 3, splat(7.0), mask)`, show it, and hide the
  external one. The mask is `*(0x008a8c20) + 0x20`, and `*(0x008a8c20)` is
  `0x00c47730`, in `.bss`, so no static read gives which lane takes the
  `7.0`.
- **`g_PhysicsHalfStep` set**: both paths run, and the net result is the
  external shell shown at identity and the cockpit one hidden.

`FUN_000e41b0` shows the same flag's meaning: with `craft+0x5f42` set it hides
`craft+0x6adc` and `craft+0x6af4` and shows the cockpit shell, which reads as
"the camera is inside the hull". Confidence **75** for the external branch,
**55** for the cockpit one's placement.

### The material: `hd_absorbinternal.rcsmaterial`

Every team's `absorbeffect.rcsmodel` in `DATA02`/`DATA03`/`DATA06` names one
mesh (Feisar's is `Feisar_LeachEffectShape`, 12,977 triangles) and one
material, `data/weapons/materials/hd_absorbinternal.rcsmaterial`, texture
`data/weapons/textures/hd_absorbinternal.gtf`, factor pair
`0x0302`/`0x0001` (`SrcAlpha`/`One`), and **one** parameter: `0xaaf53119`,
authored `1.0`. `0xaaf53119` is `~crc32("ShieldColour")`. The chunk declares
`position`, `normal`, `VertexColour1` and `Uv2`. Detonator's `DATA00`
`absorbeffect` is an opaque `nitro_emissive_outlines` with no `ShieldColour`.

Disassembled with `scripts/ps3-microcode.py`:

```text
vertex block #1
   5  MUL R0.w, v[2].wwww, c[208].xxxx   <- VertexColour1.w * ShieldColour (c464)
  10  MOV o[TC2], R0                     <- (world xyz, that product)
   0  MOV o[TC0].w, v[3].xxxx            <- Uv2
   1  MOV o[TC1].w, v[3].yyyy

fragment block #1
@0x00  MOV R0.y, f[TC1].wwww
@0x01  MOV R0.z, {time}
@0x03  MOV R0.x, f[TC0].wwww
@0x04  TEX R0.w, R0 unit0                <- the texture's ALPHA
@0x05  MAD R1.xy, R0.wwww, {0.5}, R0     <- uv + 0.5 * alpha
@0x07  MAD R1.xy, R0.zzzz, {5}, R1       <- + 5 * time, both axes
@0x09  MOV H0.w, f[TC2]                  <- the output alpha
@0x0a  TEX H0.xyz, R1 unit0 END          <- the output colour
```

`0.5` and `5` are literals in the code, not patched parameters; only `time`
(`0x906b67ba`) is patched. The fogged blocks (#2 to #9) add
`fogColour`/`globalAlphaScaler` around the same arithmetic. The texture's RGB
is a horizontal blue band and its alpha is cloud noise, so the picture is a
blue band sweeping over the shell, broken up by the noise. Confidence **90**
for the program, which is read instruction by instruction.

### Negative result: `AbsorbFader`/`AbsorbScroller` drive nothing on this disc

`FUN_003ea368` (and its one-entity twin `FUN_003eb890`) is where those names
live. For each entry of a `0x1b0`-byte record table at `0x00c86880`, it
redraws the model at `entry+0xec` under the `RigidBody` class hash, and
patches four material parameters by hash before each chunk: `AbsorbFader` =
`entry+0x10c > 0`, `AbsorbScroller` = `1 - entry+0x10c`, `LeachFader` and
`LeachScroller` likewise off `entry+0x110` (`0x003eaeb4..0x003eb190`).
But **no `.rcsmodel` or `.rcsmaterial` on the disc declares any of the four
hashes**: `0x15f4afa4`, `0x264f29d1`, `0x71c91ea7` and `0x81bf4404` occur in
0 of the 2,275 files, swept byte for byte over every archive. `ShieldColour`
occurs in 92 of them: every `absorbeffect`, every `shipshield` and the three
`vr_*_cockpit` models. So on this disc the fader and scroller patch finds
nothing to write, and `FUN_003ea368` stays unnamed. Confidence **85** for the
negative.

### Ported

`oag_render::absorb_shell` reproduces the four floats and their tick.
`oag_game::livery::absorb::shell` loads each team's `AbsorbEffect` pair and
refuses any whose materials do not all declare `ShieldColour` and blend
`SrcAlpha`/`One`. `mesh.wgsl`'s `absorb_shading` path is fragment block #1.
The fade reaches it as the vertex alpha. Every `play_absorb_feedback` restarts
the timer, the Eliminator's lap refill included, because the store sits
outside all of `Ship_PlayAbsorbFeedback`'s branches.

## Open

- **The cockpit shell** (`vr_absorbinternal_cockpit`) is not drawn: its
  `7.0` offset lane comes from the `.bss` mask above. The player's external
  shell is not drawn while the camera is inside the hull either, which is
  what the original does.
- **Who fills `FUN_003ea368`'s record table**, and what `entry+0xec` is, is
  unread. It does not matter for any picture while no material declares the
  four parameters.
- **The visibility gate**: per-viewport, and `FUN_002d4c20` was not read.
- **The remote-craft branch**, `craft+0x628c != 0`: it plays the sound and no
  burst. Nothing here reaches a remote craft.
