# `TeamSelectionScreen_Update`: the loyalty block

2026-11-13, lane `hd-shipselect-loyalty`. Static reading (Ghidra bridge, `/hdfury/EBOOT-ps3-hdfury-eu.elf`), no live run. No rename: the
functions below stay at their addresses (the update is `TeamSelectionScreen_Update`, `0x00254348`, in
[team-selection-model.md](team-selection-model.md)).

## What the update does with the loyalty total (confidence 80)

After the model fit, the update calls `Loyalty_FindTeamRecord` (`0x0001b678`, [endrace-loyalty.md](endrace-loyalty.md)) on the
highlighted team and reads `record + 8`, the same total the end-of-race ticker prints. With no record it passes `0.0`.

| Step | Evidence | Reading |
| --- | --- | --- |
| Stat-bar setter `0x002503c0` called with `(total, max, ...)`, stat index 4 | the call at the label `LAB_002558e0` | `max` is the float at the data block's `+0x4844`, `0x008b1d1c` = `0x47c35000` = **100000.0** |
| Bar position `(total / max) * DAT_008b1b70` | the setter's first lines | `DAT_008b1b70` = `0x44000000` = **512.0**, the width of the `Slide_4` block |
| Label text for index 4 | `param_4 < 4` multiplies by 10 (the four ratings are tenths); index 4 is not | the **raw total**, formatted by the string at `0x00799218`, **`%03d`** |
| Label x | the setter's last line | the nobble's x minus `DAT_008b1b78` (2.0) |

So a fresh profile prints `000` (the measured empty state, now explained as `%03d` of 0), a total of 7 prints `007`, a total of 50000
prints `50000`, and the bar is `total / 100000` of the block. Our stat-bar draw keeps its own nobble and label offsets (measured on
`000`).

## The hearts (read, not wired)

After the bar, the update loops over up to eight thresholds at `screen + variant * 0x20 + 0xbd4 .. 0xbf0` (the highlighted variant's row).
For each threshold the total has **not** reached, the next of five `UnlockNobble` widgets (`screen + 0xac0`) is moved to
`block x + threshold * 0.005120` (`+0x4850`, `0x3ba7c5ac` = 512 / 100000) and its flag `+0x34 |= 4` (shown); the rest are hidden
(`&= ~4`). So a heart marks a variant unlocked by loyalty that this team has not yet earned, at its threshold's place on the bar.
Where the thresholds (`+0xbd4`) are filled from is unread (probably the variant's `<Unlock>` loyalty rows); confidence 50 on that.

## Other titles

Omega: **ported by construction** (same `picker::hd` path as HD's front end; `omega_reads_its_own_team_selection` and the
Slide_0..4 template census read the block; no Omega frame with a total was captured, and Omega's own executable's update was not read).
Fury is the same disc and executable as HD: **ported**. 2048: **not checked** (its team screen was not censused for a `Slide_4`).
