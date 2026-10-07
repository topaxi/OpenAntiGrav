# HD's `PlayerInput_Update`: the same stick law as Omega

2026-10-07, stick-curve lane. Static reading (decompile + disassembly); the live
RPCS3 sweep is not done (see "Open"), so confidence is capped at 84 and the
sub-claims below are scored on their own. Omega's page has the full law:
[`../ps4-omega-eu/player-input.md`](../ps4-omega-eu/player-input.md).

## Where

Constructor `FUN_000cd338` (names `"player input %d"`, `PlayerInput.cpp`
`0x00781c30`) installs vtable `0x00863490`. Slot 3 is OPD `0x00874478`, whose
code address is `0x000cd548`: `PlayerInput_Update`, confidence 80 (same class,
same record layout, same constants as Omega's `0x012e8fb0`).

## The law (confidence 80)

The deadzone pair is chosen by the per-player control-scheme word
(`options + 0x70`, `iVar8`):

- scheme 0 (plain stick): `lfs f27,-0x4ac0(r2)` / `lfs f25,-0x4abc(r2)`
  (`0x000cdee8`) = **+0.1 / -0.1**, constants at `0x008a8a18` / `0x008a8a1c`
  (`0x3dcccccd`, `0xbdcccccd`; TOC `0x008ad4d8`).
- schemes 1-2 (motion): `-0x4a3c` / `-0x4a38` = **+0.2 / -0.2**
  (`0x008a8a9c` / `0x008a8aa0`).

Then, with `hi` the positive bound and `scale = 1.0 / (1.0 - hi) * 100`
computed once into `pcVar24[8]` behind a static-init flag (`*pcVar24 == 0`,
the `-0x1295` constant is 1.0, `-0x128d` is 100.0 at `0x008a8aa4`):

```
if v <= -hi or hi <= v:   out = (v -+ hi) * scale      // steering -> this+0x4c, pitch -> this+0x5c
else:                     out = 0
```

For scheme 0 that is `(|v| - 0.1) * 111.11` onto +/-100, exactly Omega. The
d-pad override writes `-100.0` / `0x42c80000` (+100.0) into the same two
fields, confirming the +/-100 domain.

**Lineage difference:** HD derives the gain from the active deadzone and latches
it on first use (so a scheme change mid-session keeps the first gain; whether
HD ever runs two schemes in one boot is not checked), where Omega has one
folded constant (111.111115) and uses 0.1 everywhere. For the plain stick,
which is what the table compares, they agree.

## Settings

`search_strings "Sensitiv"`: `"Thrust Sensitivity"`, `"Airbrake Sensitivity"`,
`"Motion Sensitivity"` (`0x00793560..0x00793590`). No stick setting, same as
Omega.

## Open

- **Byte-to-float stage not read.** The update reads its axes through
  `FUN_006778b8` / `FUN_0033d650` as `*(float *)(pad + 0x524 + index*4)`
  (steer is index `0x13`, pitch `0x14`, i.e. `pad + 0x570/0x574`), filled by the
  pad class update `FUN_0033e460` (`this+0x524..`). Whether HD zeroes a byte
  window before PlayerInput like Omega's `Pad` layer, and whether it divides by
  128 or 127.5, is the question a live sweep answers. Exact plan: patch port 0's
  left-X byte right after `cellPadGetData(0, ...)` in `FUN_005b9570`, sweep
  `0, 64, 114, 115, 128, 140..145, 255`, and read `PlayerInput+0x4c` at the
  return of `0x000cd548`. Not done this pass.
- Not verified at runtime: confidence stays under 85.
