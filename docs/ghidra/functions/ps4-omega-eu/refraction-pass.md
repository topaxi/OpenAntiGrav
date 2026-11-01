# The refraction pass on Omega: which track meshes leave the main view

2026-11-01, `omega-glass-re`. `eboot.bin` (WipEout: Omega Collection, PS4, EU),
Ghidra program `/omega/eboot-ps4-omega-eu.bin` (the **base** package's eboot;
see [patch-v107-addresses.md](patch-v107-addresses.md) for the build a race
runs). **Static evidence only**: shadPS4 gives no memory readout, so the
[rubric](../../../reverse-engineering/confidence-rubric.md) caps every score
here below 85.

This is the Omega half of
[the 2048 refraction pass](../vita-2048-eu-v104/refraction-pass.md). The law
is the Vita's, instruction for instruction: a mesh object of the environment
track model whose name contains `wohdmeshrefract` gets flag `0x20`, and the
main view's pass (kind 0) rejects it.

## The law

**Confidence: 80** (static. The loader, the list build and the filter each
read at instruction level. The filter's tests and the kind stores match the
Vita's at every step. The render target of the kind-1 list was not traced.)

A mesh object whose name contains `wohdmeshrefract` (case-insensitive
substring) is **not drawn in the main view outside Zone mode**. In Zone it is. Pass kind 1
draws only meshes flagged `0x30` (reflect or refract). By the Vita's reading,
that is the refraction target.

Applied to every `track*.rcsmodel` under `environments/` in the base
(`data00`..`data04`) and patch (`data05`, `data07`..`data09`) packages:

| model | archive | tagged | of |
| --- | --- | --- | --- |
| `01_vineta_k/track.final.rcsmodel` (the one the race loads) | `data01` | **212** | 2,878 |
| `01_vineta_k/track_reversed.final.rcsmodel` | `data01` | 212 | |
| `10_sebenco_climb/track.final.rcsmodel` | `data02` | **1** (`ice_tunnel_wohdmeshrefract2Shape`) | |
| `10_sebenco_climb/track_reversed.final.rcsmodel` | `data02` | 1 | |

Every other circuit carries no tag. **Sebenco Climb is new against the Vita**:
2048's own copy carries no tag, but Omega's re-cooked model has one. The rule
is generic in both the loader and the filter, so it moves Sebenco too.

## `Environment_Load` - `0x015fb4c0`

The counterpart of the Vita's `Environment_Load` (`0x8102f6d0`). It also loads
`\ZoneMode2048.effectSettings` and the crowd rigs.

- **Instance records.** It allocates one record per mesh object into
  `g_track_mesh_instances` (`0x01f8f6a0`, from `_Znam`), clearing `+0xa0`.
  It stores each record's address into the mesh object at `+0x10`: the
  decompile reads `*(mesh + 0x10) = g_track_mesh_instances + i * stride`.
  That is the PS4 counterpart of the Vita's mesh `+0x0c`.
- **Tag loop.** It runs the case-insensitive substring loop (`| 0x20` on both
  sides) over the mesh name and ORs bits into record `+0xa0`:
  - `wohdsecondaryfog` (`0x0182ba21`) gives `0x1000`.
  - `speedup_pad` gives `0x4`.
  - `wohdmeshreflect` (`0x0182ba54`) gives `0x10`.
  - `wohdmeshrefract` (`0x0182ba64`) gives `0x20`.
- **Draw lists.** It builds each list with `DrawList_BuildForTarget`
  (`0x01789300`). The context is `{vtable 0x01926470, DAT_01f8f640, kind
  @+0x10 (dword), sub-index @+0x14}`. Outside Zone (`0x01f90551 & 1 == 0`),
  the kinds come from the stores:

```text
0160426d LEA R14,[0x1926470]            ; context vtable
01604296 MOV qword [RSP+0x600],0        ; kind 0, sub 0  -> list 0x01f8f1c0
016042b1 MOV dword [RSP+0x604],1        ; kind 0, sub 1  -> 0x01f8f1f0
016042d5 MOV dword [RSP+0x604],2        ; kind 0, sub 2  -> 0x01f8f220
016042f9 MOV dword [RSP+0x604],4        ; kind 0, sub 4  -> 0x01f8f280
0160431d MOV dword [RSP+0x604],6        ; kind 0, sub 6  -> 0x01f8f2e0
01604367 MOV qword [RSP+0x3040],2       ; kind 2, sub 0  -> 0x01f8f5e0
016043eb MOV dword [RSP+0x3440],1       ; kind 1, sub 0  -> 0x01f8f310 (g_refraction_draw_list)
```

The kind-0 sub-indices 0, 1, 2, 4, 6 are the Vita's. List `0x01f8f1c0` is read
after a visibility cull in `FUN_015d90b0` (`015daf0d`) and in `FUN_015e3520`
(`015e354d`). That is the main track draw. List `0x01f8f310` is drawn in
`FUN_015c63a0` (`015d0497`, `015d04ea`).

In Zone mode the lists use kind `3` or `5` (`(0x01f90552 & 1) * 2 + 3`), at
sub-indices 0, 1, 2, 3, 4 and 6. Without `0x01f90552` there is also a kind-4
set. Kind 2 is a separate list, `0x01f8f5e0`, outside Zone.

## `DrawList_BuildForTarget` - `0x01789300`

It walks mesh objects (`+0x38` count, `+0x40` table) and their submeshes
(count `+0x20`). For each submesh it calls a wrapper (vtable `0x0192dec8`,
context at wrapper `+0x10`):

```text
01789339 MOV [RBP-0xf8],RCX     ; RCX = 0x192dec8, wrapper vtable
01789347 MOV [RBP-0xe8],RDX     ; the context
0178948e CALL qword [RAX+0x10]  ; wrapper slot +0x10 = 0x017898e0
01789491 TEST EAX,EAX
01789493 JZ 0x01789533          ; zero: no entry, leave this mesh's submesh loop
```

The wrapper at `0x017898e0` loads the context (`MOV RDI,[RDI+0x10]`) and
calls its slot `+0x10` (`0178990b`), which is the filter below. A non-zero
return is a CRC32 key over the program pairing (table `0x0183f4b0`, 12
bytes), and the walker looks it up.

## `TrackPass_FilterSubmesh` - `0x01605760`

This is slot `+0x10` of context vtable `0x01926470`. The other slots are
`0x01605740` (an empty function) and `0x01605750` (deleting destructor).

```text
0160578e MOV RDX,[RAX+0x10]     ; mesh object +0x10 -> instance record
016057a1 MOV R14D,[RDX+0xa0]    ; its flags
016057ed MOV RBX,[RDI+0x10]     ; context kind (low dword), sub-index (high)
016057f7 CMP EBX,5 ; JNZ 01605810
01605810 TEST R14B,0xe ; SETNZ CL
01605817 TEST EBX,EBX ; JZ 0160584c
0160581b CMP EBX,1   ; JZ 0160583f
0160583f XOR EAX,EAX
01605841 TEST R14B,0x30 ; JNZ 01605858
01605847 JMP 016059ad           ; kind 1: only reflect|refract meshes, else return 0
0160584c XOR EAX,EAX
0160584e TEST R14B,0x20
01605852 JNZ 016059ad           ; kind 0: a refract mesh returns 0, not drawn
```

The `0x20` test is reachable only from kind 0, and the `0x30` test only from
kind 1. Kinds 2 to 5 never test `0x20`: kind 2 takes `[0x01f8f760]`, and
kinds 3, 4 and 5 take their own program tables. **So in Zone mode the
original keeps these meshes in its main view.** The Vita's filter agrees:
its Zone kinds 2 to 4 never reach the `0x20` test either (`813b96e6..813b96f4`
branch kinds 0 and 1, or flags `0x6`/`0xe`, to `813b983e`, and only kinds 1 and
0 are tested there). So `oag_raceplay::load::geometry::track_model` skips them
outside Zone only, on both titles.

## The 1.07 patch eboot

The race runs the patch build, which is recompiled. Byte search
(`data/extracted/ps4/omega-eu-patch/uroot/eboot.bin`) finds the same body, with
the two tests laid out in the opposite branch sense:

```text
file 0x637beb XOR EAX,EAX ; 0x637bed TEST R14B,0x20 ; JE  0x637c04 ; JMP 0x637d5d (return 0)
file 0x637bf8 XOR EAX,EAX ; 0x637bfa TEST R14B,0x30 ; JE  0x637d5d (return 0)
```

They are preceded by the same `CMP EBX,5`, `TEST R14B,0x6`,
`AND EAX,0x800; SHR EAX,0xb` and `TEST R14B,0xe` sequence. The base eboot's
copy is at file `0x60a92f` (Ghidra `0x0160583f`). The patch also carries the
`wohdmeshrefract` string (file `0x895388`).

## Wired

`oag_mesh::mesh::rcs::psp2::refraction_only::split_refraction_target` now runs
on Omega as on 2048: the `is_ps4` guard in `track_model` is gone.
`crates/raceplay/tests/psp2_refraction_only_ground_truth.rs` pins the census
above and checks that the loaded Omega Vineta K draws none of the 212 outside
Zone, and all of them in Zone. With the guard back it fails with
`refraction-only meshes drawn: 212 of 212`. With the skip applied in Zone it
fails with `Zone drops 212 refraction-only meshes`.

Frames are from `oag-game data/extracted/ps4 --race --no-audio --size 960x540
--track 'Data\environments\01_vineta_k\track.vex' --autopilot --ticks N
--screenshot`, before and after:

- **Vineta K, tick 900.** The teal-and-chrome sheet that hangs across the
  tunnel roof goes. It is the same `trackouter`-family sheet the 2048 fix
  removed. Road and walls are unchanged.
- **Vineta K, tick 3600.** A small change at the tunnel mouth.
- **Vineta K, tick 1800.** Pixel-identical.
- **Sebenco Climb, tick 1800.** The icy rock sheet above the track
  (`ice_tunnel_wohdmeshrefract2Shape`) goes, and the sky behind shows. No
  other Sebenco frame of ticks 600 to 3600 moves.

The original draws them only into the glass's refraction texture. **This project now draws that
target** (2026-11-01, `psp2-refraction-target`): the left-out meshes are drawn into a 960x540
target (half of 1920x1080, **inherited** from the Vita's halving, the Omega target's size is
untraced), and the three `mt_`/`cl_tunnelrefraction` materials of Vineta K sample it. The
rest is on [the 2048 page](../vita-2048-eu-v104/refraction-pass.md), "The target is drawn
here": **ported** for the split, the camera and the glass law (inherited, Omega's GCN is unread).
**Checked, applies, not wired: `rt_icetunnel`** (Sebenco Climb, one submesh) declares the same
engine-bound `screenSpaceRefractionTex`, but its GCN pixel program is unread, so Sebenco's rock
sheet is drawn into the target and nothing samples it: tick 1800's black above the track stays.

## Evidence still owed

- The render target of list `0x01f8f310`. `FUN_015c63a0` draws it after
  `FUN_016304d0(5, ...)`, a pass-bucket begin. Map pass 5 to its target to
  confirm it is the half-size refraction target.
- A live read (the native port's `/proc/<pid>/mem`, see
  [patch-v107-addresses.md](patch-v107-addresses.md)) of a refract record's
  `+0xa0` and the kind-0 list's length would take this past 85.
