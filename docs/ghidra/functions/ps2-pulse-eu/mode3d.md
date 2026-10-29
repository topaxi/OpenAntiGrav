# `Mode3D` on the PS2: the same camera as the PSP, and a switch for the `Origin` shift

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base `0x00100000`.
Read 2026-10-29 (`pulse-ps2-ship-pose`) to settle where Ship Select puts the hull.
[`race-box-screens.md`](../psp-pulse-usa/race-box-screens.md#mode3ds-own-fixed-camera-and-the-child-models-own-pose-2026-09-28)
has the PSP class; every structural finding there holds here, offsets aside.

## Names recovered

| Address | Name | Conf | What it is |
| --- | --- | ---: | --- |
| `0x00199550` | `Mode3D_ReadValues` | 85 | attribute reader: `OriginX` `+0xa4`, `OriginY` `+0xa8`, `nearZ` `+0xac`, `farZ` `+0xb0`, `mirror` `+0xa0` (`Xml_AttributeAsBool`), `mode` `+0x98` (`orthographic`), `FirstPass` `+0x9c` |
| `0x00199718` | `Mode3D_EnterView` | 75 | the widget-tree draw callback: pushes the matrices, projection with fov `0x3f91361e` = 1.1346 rad (65 deg), aspect `0x3fb6db6e` = 1.4286 (`-` when `mirror`) |
| `0x001999c8` | `Mode3D_DrawQueued` | 75 | the later, queued callback: fov `1.0`, the same aspect `1.4286`, `nearZ`/`farZ`, then the `Origin` shift below |
| `0x00199940` | `Mode3D_LeaveView` | 70 | pops the two matrix stacks `Mode3D_EnterView` pushed |
| `0x001993a8` | `Mode3D_SetOriginMapping` | 75 | writes `g_mode3d_origin_scale`/`offset` (`0x0027f0f8..0x0027f104`) and the enable flag `g_mode3d_origin_enabled` (`0x0027f0f4`); enable `0` also zeroes the live shift `0x00285088/8c` |
| `0x0027f0f4` | `g_mode3d_origin_enabled` | 75 | data: the switch |

The two projection callbacks mirror the PSP's `Mode3D_EnterView`/`Mode3D_DrawQueued` pair
(fov 1.1346 first, 1.0 at the queued draw), which is why `Mode3D_DrawQueued` carries the same
name: confidence 75 and not higher because neither PS2 callback was caught live.

## The aspect is the grid's own, 640/448

`Mode3D_DrawQueued` calls the perspective builder `FUN_001e9420(1.0, 1.4286, nearZ, farZ, 1.0, ctx)`.
`0x3fb6db6e` is `640/448`, the PS2 front end's grid, exactly as the PSP's `1.7647059` is `480/272`.
The PS2 draws that grid stretched to 16:9, so ours must use the grid's ratio, not the glass's
(`oag_game::preview::mode3d_view_projection`: was `display_aspect`, now `size.0 / size.1`; PSP moves
by 0.7 percent).

## The `Origin` shift is switched, and Ship Select has it off

`Mode3D_DrawQueued` ends:

```
if (g_mode3d_origin_enabled == 0) { shift = (0, 0) }
else { shift.x = OriginX * g_mode3d_origin_scale.x + g_mode3d_origin_offset.x   (0x00285088)
       shift.y = OriginY * g_mode3d_origin_scale.y + g_mode3d_origin_offset.y } (0x0028508c)
```

`FUN_0021b160` adds the shift to the GS viewport offset. Writers of the enable flag (all call
`Mode3D_SetOriginMapping`):

- `FUN_001c11f8` (the circuit screen's controller: it formats `%s\FE\%s.vex` with `forward` or
  `reverse` and loads that circuit model) and `FUN_001c10a8` (the `Split ScreenV/H` controller) pass
  **enable 1** with the scale `0x0027f0c0/c4` = `0.8`/`1.1428571` (`Video_SetRefreshMode`'s 50 Hz
  pixel aspect: 640 grid columns to 512 GS pixels, 448 rows to 512) and offset `0, 0`.
- `FUN_001831a0` (per-screen set-up, for the screen names it compares against `0x2aaea0` and the `Launch Game` prefix) and
  `FUN_001be2b0` (the Tournament/`CSplit` controller) pass **enable 0**.

Which controller loads `Team Selection` is not traced. The measurement settles the craft:

PCSX2 savestate of Ship Select (Assegai, Classic; reached Racebox, Track Select, Ship Select), Xvfb,
GS screenshot, 12 sway phases of ours against the frame, box (60,150)-(640,560) at 1280x720:

| ours | best mean abs grey diff to the original | mean grey (original 19.81) |
| --- | ---: | ---: |
| before: PS2 craft on the 12 s orbit | 24.36 (t 1.0) | 14.65 |
| after: authored `x -6 y 2 z -15`, `RotX 0.4 RotY 0.7`, fov 1.0, aspect 1.4286, **no `Origin`** | **8.16** (t 3.0) | 18.87 |

Hull centroid at 640x360: original 148 to 164 over a sway cycle, ours without the `Origin` 147 to
158; the PSP-signed `Origin 120` puts it at about 72 (the left edge), the circuit-signed one at
about 274 (on the stat panel). So **the PS2's craft `Origin` is not applied** (confidence 70: one
screen reached by one path; the switch is in the code, its setter for this screen is not traced).
The circuit keeps its `Origin`, with the sign the 2026-09-28 live check measured (positive
`OriginX` moves the outline right on the PS2, left on the PSP).

## Checked against Omega and the PSP

PSP: its craft `Origin 120` matches the PSP capture with the shift applied (`pulse-picker-clock`),
so this is a PS2 difference, **checked, differs**. Pure and Omega/2048 do not use this class
(HD's front end has its own `Mode3D` code): not applicable.
