# A circuit places its own particle effects: `ParticleSystem` nodes in `track.vex`

2026-10-02. **Pulse's circuits author their ambient effects as scene nodes,
and the original spawns every one of them when the circuit loads.** A
`ParticleSystem` node (class `0x3c4`) in a `track.vex` carries a `Name` string
attribute naming a `Data\Psys\<Name>.POB` and a 64-byte payload that is its
local matrix. `PsysNode_Init` (`0x089156a0`) loads the effect and starts one
instance on the node; `PsysNode_Update` (`0x08915cdc`) keeps it on the node's
world matrix every frame. Confirmed live on PPSSPP (below).

This corrects two earlier readings:

- [`pob.md`](../../../formats/pob.md) called `0x08a6bd18`, the value
  `ParticleSystem`'s registration installs, "a trivial self-address-returning
  thunk ... the same dead end". It is the class's identity tag, the mechanism
  [`weatherpos.md`](weatherpos.md) found for `weatherPos`, and the class it
  tags is live.
- The environmental effects were believed to need `weatherPos` (`0x3da`) to
  place them. Two of them, `WO_BLUE_WELDER` and `WO_MODESTO_STEAM_A`, are
  placed by `ParticleSystem` nodes instead. `WO_MODESTO_STEAM_A` is a Pulse
  effect, placed on circuits five and seven, not a Pure circuit's.

## What the disc places

Every `ParticleSystem` node on the PSP USA disc's `Data.wad`
(`crates/vex/examples/psys_node_census.rs`), by file:

| File | Effect | Nodes | Parent chain |
| --- | --- | ---: | --- |
| `01_Track\track.vex`, `track_reversed.vex` | `WO_BLUE_WELDER` | 3 each | `Transform` < `world` |
| `01_Track\zone_track*.vex` | `WO_BLUE_WELDER` (`psystem_cathead2`) | 1 each | `_10_Mid_Section_C` < `world` |
| `05_Track\track.vex` | `WO_BLUE_WELDER` | 6 | `Transform` chains |
| `05_Track\track_reversed.vex` | `WO_BLUE_WELDER` | 7 | `Transform` chains |
| `05_Track\track*.vex` | `WO_MODESTO_STEAM_A` | 2 each | `Anim Transform` < `Transform` < `world` |
| `05_Track\zone_track*.vex` | `WO_MODESTO_STEAM_A` | 2 each | as above |
| `07_Track\track*.vex` | `WO_MODESTO_STEAM_A` | 18 each | `Transform` chains |

65 nodes, no other effect named, and no node parented to an
`animationTrigger`. `WO_RAIN`, `WO_RAIN_LENS` and `WO_SNOW` are named by no
`ParticleSystem` node, so `weatherPos` stays their likely owner (open, see
[`weatherpos.md`](weatherpos.md)). Confidence **95** on the census: it is a
read of the files, pinned by `crates/vex/tests/placed_psys_ground_truth.rs`.

## The functions

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x089164c0` | `PsysNode_RegisterClass` | 92 | `Vex_RegisterClass(&DAT_08b63510, 0x3c4)`, then stores `0x08a6ba64()` and finally `0x08a6bd18()` into the descriptor's `+4`, vtable `0x08ad1884` at `+0x38`. The same two-write shape as `weatherPos`'s and `Mesh_Register`'s. |
| `0x08a6bd18` | `PsysNode_Tag` | 88 | Returns its own address (`lui`/`jr`/`addiu`). Installed by the registration above, stamped into a new node's `+4` by `0x089153b0` and `PsysNode_Construct_q`, and called by the 30-odd weapon and collision sites that create a particle node at runtime (`Missile_Init`, `ShipCollisionFx_Trigger`, ...). This is the third `Node_SetAnimTimeTree` branch [`anim-transform.md`](anim-transform.md) left unidentified. |
| `0x089156a0` | `PsysNode_Init` | 90 | The vtable's `+0x7c` slot (`0x08ad1884 + 0x7c`), the slot the generic per-node spawner `FUN_08908f98` calls ([`weatherpos.md`](weatherpos.md)). Body below. Confirmed live. |
| `0x0891559c` | `PsysNode_Construct` | 65 | The vtable's `+0x74` slot: allocates `0xd0` bytes, attaches it to a parent node, stamps `PsysNode_Tag`, stores `param_3` at `+0x4c`. The same shape as `weatherPos`'s `FUN_0892c404`. Who calls it at runtime is not read, hence `_q`. |
| `0x0892e384` | `AnimationTrigger_RegisterClass` | 90 | `Vex_RegisterClass(&DAT_08b65d00, 0x3dc)`, tag `0x08a727cc`; `0x3dc` is `animationTrigger` in [`vex.md`](../../../formats/vex.md#node-types). |
| `0x08a727cc` | `AnimationTrigger_Tag` | 85 | Its identity tag, the value `PsysNode_Init` compares a parent against. |
| `0x08945284` | `Node_SetLocalMatrix` | 75 | Copies sixteen words from its second argument into the node's matrix block (`node+0x3c` -> `+0x40..+0x7c`), sets the dirty bits in `node+0x2c`. Called by `PsysNode_Init` with `*(node+0x50)`. |

### `PsysNode_Init` (`0x089156a0`)

```text
Node_SetLocalMatrix(node, *(node+0x50))       ; the 64-byte payload
name = attribute "Name" of the node header *(node+0x48), strcasecmp  ; 0x08a884d0
if none: name = ""                            ; 0x08a884d8
sprintf(buf, "Data\\Psys\\%s.POB", name)       ; 0x08a884dc, call at 0x0891575c
node+0xb4 = ParticleManager_LoadResource(mgr, buf)
(an optional "RenderOrder" lookup, 0x08a884f0, whose result is not used)
node+0x60 = 0; node+0xb0 = 1
if *(*(node+8) + 4) != AnimationTrigger_Tag():  ; the parent's class
    inst = FUN_088f3174(mgr, 0, node+0xb4, 'vex ')
    FUN_088f58a4(inst, node+0xb4, *(node+0x50), 0, 0)
    node+0x6c = inst+0x14                     ; the instance handle
```

No severity is written, so the instance plays at the default. Confidence **90**:
a branch-clear decompile whose call at `0x0891575c` was then breakpointed live.

### `PsysNode_Update` (`0x08915cdc`), read again

Already named on [`shield.md`](shield.md) for its `+0xb8` start delay. The rest
of it, read this pass: while the delay is spent and `+0xb0 & 1`, it composes the
node's world matrix (from `+0xbc`'s node plus an offset at `+0xa0`, or its own
local matrix), negates row 1 when `+0x60 & 2`, looks the instance up by `+0x6c`
and hands it the matrix (`FUN_088f4498`) and one `ParticleSystem_Update`. **When
the lookup fails - the instance has finished - the node queues itself for
destruction.** So a looping effect lives for the race and a burst plays once.
Confidence **80**.

## Confirmed live (PPSSPP 1.20.4, software renderer, 2026-10-02)

Basilico Black, a Time Trial reached with the dev-unlock byte
([`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md)),
breakpoint at `0x0891575c` (the `sprintf`) armed from Team Selection on,
reading `a2` (the name), `s0` (the node), `*(s0+0x50)` (the matrix) and the
parent's tag:

| Hit | Name | Parent tag | Local matrix row 3 |
| ---: | --- | --- | --- |
| 0 | `WO_BLUE_WELDER` | `0x08a6ba64` | `(-597.43, 23.91, 829.56)` |
| 1 | `WO_BLUE_WELDER` | `0x08a6ba64` | `(-756.33, 30.76, 741.42)` |
| 2 | `WO_BLUE_WELDER` | `0x08a6ba64` | `(0, 8.58, 0)` |

Three hits, then none for 120 s: exactly `01_Track`'s three nodes, every matrix
word identical to the disc's payload, and no parent an `animationTrigger`, so all
three spawn. Circuits five and seven were not captured live; their nodes go
through the same function and are read statically.

## What a player sees, and how ours compares

Frames on Basilico Black at matched poses (`psp-trace.py --camera` rows fed to
`oag-game --pose-from`), under `data/scratch/pulse-psys/` (gitignored). The
original's welder at `(-756, 31, 741)` reads as a **white core inside a large
blue-violet halo**, with a few small blue sparks falling, re-flashing every few
frames on the tunnel's left wall. Ours puts its sparks in the same place but
draws them as long blue streaks with **no halo**. The halo is the effect's
`GLOW` template; ours parses that template's size channel as 19,662 keys, almost
all `0.0068`, so it draws at about 0.2 units. That is a misread in the `.pob`
interpreter, open below, not a placement difference.

## Open

- **`GLOW`'s size channel.** 19,662 keys on one channel is not authored data.
  Read `WO_BLUE_WELDER.POB`'s `GLOW` template record by hand and find where its
  key count comes from.
- **Culling.** Whether the draw slot (`0x08915fd0`) skips a node outside the
  visible sections is unread; ours draws every placed effect.
- **The emitter frame's rotation about `+Y`.** The original hands the instance
  the whole world matrix; ours carries a position and an `up`.
- **`PsysNode_Construct_q`'s runtime callers.**
- **The other titles.** Pure, HD and the PS2 port were not checked for the same
  placement; ours places effects on Pulse PSP only.
