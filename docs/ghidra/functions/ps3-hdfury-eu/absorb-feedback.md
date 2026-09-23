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
  on every call, which reads as the timer for HD's own absorb overlay (see
  Open). That is an inference from the store, not a read of its consumer.

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

`oag_game::race::absorb::HD_ABSORB_BURST` (`MirroredPairs { stagger: 0.2 }`)
plays one attached `WO_WEAPON_ABSORB` per slot, on the pairs above. The
`absorb` locators come from `Locators.vex` through
`oag_vex::vex::CLASS_ABSORB` and `livery::absorb`. A hull with fewer than six
plays nothing, which is the original's own gate. Detonator and Zone therefore
play no burst.

## Open

- **HD's absorb overlay.** Every team also ships `absorbeffect.vex` with its
  `.rcsmodel` (a single mesh named like `Feisar_LeachEffectShape`) and a
  `*_hd_absorbinternal` material. `AbsorbFader`/`AbsorbScroller` are among
  the render parameters [`renderer.md`](renderer.md) lists. This is the PS3
  counterpart of Pulse's `absorb_surface.mip` hull overlay, and none of it is
  read: not the model's draw, not the two parameters' writers, and not whether
  `craft+0x7a5c` drives them.
- **The visibility gate**: per-viewport, and `FUN_002d4c20` was not read.
- **The remote-craft branch**, `craft+0x628c != 0`: it plays the sound and no
  burst. Nothing here reaches a remote craft.
