# The refraction pass: which track meshes leave the main view

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. Recovered 2026-11-01 (`psp2-glass-re`) while chasing the
extra meshes the maintainer sees in the 2048 and Omega Vineta K tunnels. Wipeout
HD keeps the same split in a file field, chunk render flag `0x10`
([ps3-hdfury-eu visibility](../ps3-hdfury-eu/visibility.md)). 2048 has no such
field: **the flag is derived at load from the mesh object's name.**

## The law

**Confidence: 82** (static, instruction level at every load-bearing step: the
loader, the filter and the target agree. No Vita3K capture yet.)

A mesh object of the **environment track model** whose name contains
`wohdmeshrefract` (case-insensitive substring) is **not drawn in the main
view**. It is drawn only into the half-resolution target that the glass
samples as `screenSpaceRefractionTex`. A name containing `wohdmeshreflect`
goes into that target too, but stays in the main view.

Applied to 2048 DLC1 `Vineta_K/track.rcsmodel` (1,833 mesh objects):

| tag | meshes | instance flag |
| --- | --- | --- |
| `wohdtrack` | 101 | `0x1` |
| `wohdsecondaryfog` | 237 | `0x1000` |
| `wohdmeshreflect` | 0 | `0x10` |
| `wohdmeshrefract` | **238** (left out of the main view) | `0x20` |
| `tunnel_understruts*` (no tag) | 172, **none tagged** | none |

The refract set is `metalsphincter*`, `newesttunnel*`, `trackouter_*`,
`newtunnel2pt2v*`, `tunnelentrance*`, `NEW_ROCK_arch*`, the pistonheads and
towers, and similar pieces. **The understruts are not in it.** So, unlike HD's
chunk 1574, the original 2048 draws the tunnel's dark strut band in its main
view. No mesh carries both `wohdtrack` and `wohdmeshrefract`, so the road is
never left out.

**Blast radius.** Over all 80 `track*.rcsmodel` under `environments/` in the
EU package's five archives (base, patch `data1`/`data2`, `dlc1`, `dlc2`), only
Vineta K carries the tag: forward 238 (`dlc1`; 238 of 1,833 in the base copy),
reversed 242 (243 in the base copy). Every other circuit has none, so the rule
moves no other circuit.

**What the 12 `trackouter_*` pieces are.** Their boxes hug the tunnels
(`trackouter_*4` spans x -877..-657, y -158..-135, z -61..395, around HD's
tunnel pose A at (-841, -144, 203)). They are the tunnel's outer skin, seen
from inside as a dark curved sheet across the roof. Frames before and after
the skip, at pose A and at autopilot ticks 900, 1200, 1800 and 3600 of
`oag-game data/extracted/vita/PCSF00007 --race --track
'Data\art\published\DLC1\environments\Vineta_K\track.vex' --autopilot
--ticks N --screenshot`: the sheet goes, and road and walls are unchanged in
every frame.

## `Environment_Load` - `0x8102f6d0` (named on [zone-environment-fallback.md](zone-environment-fallback.md))

After it allocates one 0x70-byte instance record per mesh object
(`g_track_mesh_instances` at `0x8151c574`, count = model header `+0x28`, mesh
table `+0x2c`), it writes the record's address into mesh object `+0x0c`, then
substring-matches mesh object `+0x10` (the shape's name) against a run of
strings at `0x81422694..0x81422734`, with each byte `| 0x20` on both sides. Each
hit ORs a bit into record `+0x60`:

```text
0x81422694 wohdtrack        -> 0x1     (81030494..8103049a region, LAB_8103049e)
0x814226a0 shellgeometry    -> 0x800   (81030490)
0x814226b0 wohdsecondaryfog -> 0x1000  (81030482)
0x814226c4 weapon_pad       -> 0x2     (810304ac)
0x81422708 speedup_pad      -> 0x4     (81030474)
0x81422714 wohdmeshreflect  -> 0x10    (81030466: orrs r1,r1,#0x10)
0x81422724 wohdmeshrefract  -> 0x20    (8103043e: orrs r1,r1,#0x20)
"billboard" in a material   -> 0x8
```

It then builds the draw lists with `DrawList_BuildForTarget` (`0x812edd14`),
using a context `{vtable 0x81509550, ..., kind @+8, sub-index @+0xc}`. Kinds
were read from the stores, not the decompiler's slot names:

- Lists `0x816a3b98`, `+0x24`, `+0x48`, `+0x90`, `+0xd8`: kind `r7 = 0` stored
  at `sp+0x310` (`81030c76: strd r7,r7,[sp,#0x310]`), sub-indices 0,1,2,4,6.
  The target is `Render_GetMainTarget` (`0x810362b2` -> `0x816ae340`).
- List `0x816a3c94` (`g_refraction_draw_list`): kind `r9 = 1`
  (`810306e8: mov.w r9,#0x1`, stored at `81030d60: strd r9,r7,[sp,#0x320]`).
  The target is `Render_GetRefractionTarget` (`0x810363ae` -> `0x816ae750`).

In Zone mode (`0x816a4d80` set) the lists use kinds 2, 3 and 4 instead. **None
of them rejects `0x20`** (read 2026-11-01, `omega-glass-re`). At
`813b96e6..813b96f4`, kinds 0 and 1, or a mesh with flags `0x6`/`0xe`, go to
`813b983e`. Only kind 1 (`0x30`) and kind 0 (`0x20`) are tested there, and
every other kind draws. So a Zone race keeps these meshes in its main view, and
this project skips them outside Zone only.

## `TrackPass_FilterSubmesh` - `0x813b9670`

Slot `+8` of context vtable `0x81509550`. `DrawList_WalkSubmeshes`
(`0x812f3fec`) iterates every mesh object (`+0x28`/`+0x2c`) and every submesh
(`+0x14`/`+0x1c`). For each one it calls wrapper vtable `0x81517298` slot `+8`
(`0x813f045d`), which forwards to this function. A zero return leaves the
submesh out of the list. A non-zero return is the program pairing the walker
looks up. Instruction-level:

```text
813b9696 ldr r11,[r4,#0xc]      ; mesh object +0x0c -> instance record
813b969c ldr r10,[r11,#0x60]    ; its flags
813b96aa ldr r7,[r0,#0x8]       ; context kind
813b96c8 str lr,[sp,#0x4]       ; lr = 0: the reject return value
813b983e cmp r7,#1 ; bne 813b9890
813b9842 ands r2,r10,#0x30 ; beq 813b989a   ; kind 1: only reflect|refract meshes
813b9890 cmp r7,#0 ; bne 813b9848
813b9894 ands r2,r10,#0x20 ; beq 813b9848   ; kind 0: refract meshes ...
813b989a ldr r0,[sp,#0x4]                   ; ... return 0, not drawn
```

## The target is the refraction texture

In `FUN_81035b10` (render-target setup), target `0x816ae750` is created at
`(width >> 1, height >> 1)` of the main surface, with colour surface
`0x816ae770`. Texture `0x816ae7d8` is made from that surface
(`FUN_812f7592(&0x816ae7d8, &0x816ae770)`) and returned by
`Render_GetRefractionTexture` (`0x810363a4`). The per-frame renderer
`FUN_8101bc7a` binds it to material slot `0x27` (`screenSpaceReflectionTex`) and
slot `0x28` (`screenSpaceRefractionTex`): `FUN_812ee0a4(ctx, 0x28,
FUN_810363a4())`. The slot names come from the registration table in
`FUN_8101d068`. HD's glass target is 640x360, half of 1280x720, so the halving
matches HD.

## Omega: ported

`eboot.bin`'s `Environment_Load` (`0x015fb4c0`) runs the same case-insensitive
substring loop over the mesh name (PS4 mesh object `+0x18`) against
`wohdsecondaryfog` (`0x0182ba21`), `wohdmeshreflect` (`0x0182ba54`) and
`wohdmeshrefract` (`0x0182ba64`). It ORs `0x1000`, `0x10` and `0x20` into the
per-mesh record at `+0xa0`: the same bits as the Vita.

Its `TrackPass_FilterSubmesh` (`0x01605760`, slot `+0x10` of context vtable
`0x01926470`) makes the same two tests:
- kind 0 rejects `0x20` (`0160584e TEST R14B,0x20`);
- kind 1 requires `0x30` (`01605841 TEST R14B,0x30`).

The 1.07 patch eboot carries the same body. Confidence 80. The evidence and
the census are on [the Omega page](../ps4-omega-eu/refraction-pass.md): 212
meshes on `01_vineta_k/track.final.rcsmodel` of 2,878, and one on Sebenco
Climb, which the Vita's copy lacks. The skip runs on both titles, outside Zone
only: Omega's Zone kinds (3 to 5) never test `0x20` either.

## Evidence still owed

A Vita3K capture of the refraction target's draws at the tunnel, tied to
submeshes by vertex-buffer offset as HD's RPCS3 census did, would take this
from 82 to 90+.
