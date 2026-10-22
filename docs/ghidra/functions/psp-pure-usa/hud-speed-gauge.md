# Pure's speed and shield gauge: three models, scrubbed by a fraction

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra `/pure/BOOT-psp-pure-usa.BIN`.
Read 2026-10-22 (`pure-fidelity` lane). Disassembly through the bridge's `disassemble_function`;
Ghidra's decompiler did not complete on these functions in this session.

Pure's HUD authors the gauge as `<Mode3D>` models, not `<Image>` bars
([pure-status.md](../../../formats/pure-status.md#the-speed-gauge-is-three-models-2026-10-22)). This page is
what the executable does with them.

## Functions

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x0881eabc` | `Hud_UpdateSpeedBar` | 85 | The speed gauge and its digits, called from the `Hud` update `0x0881e6bc`. |
| `0x0881f13c` | `Hud_UpdateEnergyBar` | 80 | The shield gauge: eases a shown percent toward the craft's, scrubs the `Shield3D` model by it. |

## Binding

`FUN_08821838` (called from the same update) resolves widgets by name through the lookup
`0x000b2fdc` (file-relative, `0x088b6fdc` relocated) and stores each handle in the `Hud`: `Speed3D` (`0x08a464f4`) in `hud+0x154`,
`Shield3D` in `+0x158`, `Overlay3D` in `+0x15c`, `SpeedL` (`0x08a465e4`) in `+0x17c`, `SpeedR` in
`+0x180`. The `<aText name="SpeedUnits">` is the disabled spelling, so the layout has no unit label
and the `km/h` is the outline model's own texture.

## `Hud_UpdateSpeedBar` (`0x0881eabc`)

- `speed = *(hud->status)` as a float; `if (hud+0x154) FUN_000c6418(speed / 700.0f)` - the
  divisor is `lui 0x442f` = `700.0`, read straight off the instruction. `0x000c6418` (relocated
  `0x088ca418`) loads the widget's model at `+0x9c` and calls `0x0005ff40` -> `0x0005f3c4`
  (`0x088633c4`), which walks the model's node tree calling a per-class virtual with the float.
  **That it scrubs the model's animation clock by the fraction is inferred** (confidence 70): the
  two fill models carry one texture-offset track each, `u` from 0 at frame 1 to `-5.0` at frame 60,
  and with a clamped edge that is exactly a fill growing from the left (below).
- Digits: `speed < 5.0` reads as `0`; the text is `sprintf("%.0f", speed)` (`0x08a46314`);
  `strlen < 3` puts the whole string in `SpeedR` (`+0x180`) and an empty string in `SpeedL`
  (`+0x17c`), otherwise `SpeedL` takes all but the last two characters and `SpeedR` the last two.
  Confidence 85 (disassembly read in full).

## `Hud_UpdateEnergyBar` (`0x0881f13c`)

`hud+0x78` is a shown percent. Each tick it moves toward the craft's shield (`*(status+4)`) by
`dt * 100.0 * 1.4` (`0x42c8`, `0x3fb33333`), clamped so it never passes it, and
`FUN_000c6418(shown / 100.0f)` scrubs `Shield3D`. So the bar chases the true value at 140 percent a
second. Our readout shows the true value directly: **the chase is not ported**.

## The models, from the disc

`Data\HUD\Bar_1.vex` (speed, 20 vertices), `Bar_3.vex` (shield, 12), `Bar_outline_1.vex` (a
256x32 quad). The fills are untextured-looking strips with a vertex-colour grade (red, light cyan to
white); their 32x16 texture is a column pair, opaque on the left 16 texels and clear on the right,
and their `u` runs 0.5 to 5.5 across the mesh. With the track's `-5.0` offset at the end, every
fragment's `u` is at or below 0.5 and (clamped) reads the opaque half: full. At the start of the
track `u` is 0.5 or more and reads the clear half: empty. In between the edge sits at that fraction
of the mesh's width. The same arithmetic is what `oag_hud` does with a crop.

## Placement

The orthographic block puts `y` up from the bottom: the origin is `(394, 272 - 20)`. Rasterised at
one unit per pixel the three canvases land at `(309, 239)` 169x14 (speed), `(309, 255)` 166x9
(shield) and `(266, 236)` 256x32 (outline, of which the visible art is the yellow box at
`(309, 239)-(477, 252)` and the cyan box at `(309, 255)-(474, 263)`), which is the box measured on
all four `data/reference/compare-page/pure-psp/` pairs to the pixel.

## Other titles

Pulse: **checked, differs.** Its `Hud_UpdateSpeedBar` (`0x0881a3e0`) crops a sprite by the same
`speed / 700`; the divisor is the same law, the drawing is not (`<Image>` bars, no models).
HD, Omega, 2048: **not checkable here**, they do not share Pure's HUD engine; their bars are
`<Image>` and `<CurveBar>` widgets (`oag_hud::draw::bar_fraction`), with HD's own 700 recovered
(`Hud_UpdateSpeedReadout`, `0x00086e88`).
