# 2048's scenery clock: the time every animated scenery node keeps

Where 2048 keeps the time its animated scenery runs on, found in a running Vita3K so a comparison pair can read it at the grab and ours
can be put on it (`--world-clock-offset`), as Pulse's, Pure's and HD's pairs do. EU v1.04 (`PCSF00007`), the HD Campaign's first node
(DLC1 Vineta K, Race), read through `/proc/<pid>/mem` of Vita3K (guest address plus `0x400000000`), 2026-11-18, lane `clock-titles`.
No code was read in Ghidra for this page: **no function is named and `names.tsv` has no row for it** (see "Not found").

## What was found

1. **Two equal floats `0x40` apart in about a thousand nodes.** Two dumps of the guest's read-write memory (330 MiB, 207 spans) 5.16 s of
   wall time apart (`scripts/vita-2048-world-clock.py`), grouped by the value a word held in the first dump and the value it moved to in
   the second, gave one group with a value above 1 that advanced by the wall time: 28.8295 to 33.8687 s (delta 5.039 against 5.16 s of wall
   time, the guest running 2 % slow), 2,035 words, in pairs `0x40` apart, the pairs `0x850` to `0x8d0` apart. A node read from 0x80 before the
   first copy has two identity matrices (`+0x00`, `+0x40`), then `+0x80` the time (the first copy), `+0xc0` the time again (the second), and
   pointers and a third identity matrix after (`+0x140` and on). So a scenery node keeps its running time twice, 64 bytes apart.
   **Confidence 85** that this is the animation time the scenery runs on: one value, shared by about a thousand objects, advancing at the
   guest's frame time.
2. **It restarts at a Retry.** The race's nodes are freed and a new scene's are built: the freed group keeps its last time in memory and
   still agrees with itself (58.31 s in two boots, bit for bit, then 68.6 s: the previous races' last times), while the new scene's group
   starts again. A reader that does not know this reads a frozen clock.
3. **Its value at the grid.** In the HD Campaign's Retry the new group read 24.12 s at the mode card and 25.57 s at the first HUD frame, and
   advanced 5.0 s over 30 grabs 0.17 s apart (the guest runs at wall rate through the countdown): 28.17 s at the grab of the pair.

## What the pair does with it

`scripts/vita-2048-pair.py --walk` scans the guest before the Retry for every value that heads at least 200 such pairs (`scan_scenery_clock`,
2 to 10 s; the old race's clocks, one of them live, become the stale set), then at each grab of the countdown burst re-reads 24 cached nodes
of the group that is not in that set (milliseconds; a scan again while the Retry's mode card still holds the countdown and no group agrees).
It records `original.world_clock_s`, the series (`original.world_clock.series_s`), and `original.world_clock_offset_s` (the race clock reads
zero for the whole countdown, so the offset is the clock itself), renders ours with `--world-clock-offset = world clock - ticks / 60` and
stores it as `ours.world_clock_offset_s`, which `compare-rerender-ours.py` keeps.

**Measured, one pair** (`dlc-vineta-k`, HD Campaign Race, Feisar HD, grid before GO, two boots that read, one that completed a pair):
world clock 28.17 s at the grab, ours at tick 178 and offset 25.20 s. The scenery element that moves with ours' clock in this view is a
small dark flying vehicle crossing the sky above the skyline. A sweep of ours' offset (0 to 50 s, 21 to 27.5 s in 0.5 s steps) puts it at
about the original's place (left of the tower, level with the top of the harimau board) at an offset of 22.5 to 23.0 s (mean abs grey
difference in the box `380,25 to 470,112`: 7.20 and 7.36 against 7.72 with the vehicle out of frame and 8.0 at the read offset 25.2 s), that
is 2.2 to 2.7 s before the world clock read. **The falsifier the lane set (the best placement at an offset other than the read one) fired,
by 2.5 s.** So the read clock is a node clock that tracks the guest's time, but the vehicle is not placed by it to better than about
2.5 s; whether that is the vehicle's own node starting later (its time is the clock less a start offset), the presented-frame lag of Vita3K
(not measured here, the pair's lag is `0`), or ours' path differing from the original's is not determined. **Confidence 60** that the
scenery clock read here is the clock that places the vehicle; the pair was not adopted.

## Not found

- **The writer.** Vita3K has no memory watchpoint, so no instruction that stores the time was caught. The nodes' class and its
  advance function (HD's `MeshImporter_AdvanceTime`, `ps3-hdfury-eu/gantry-clock.md`, keeps `+0xc0` and a stamp at `+0x15c`) were not
  located in Thumb-2 by a read of the executable; the layout above differs from HD's (the two copies are `0x40` apart, HD's `0x9c`), so
  HD's offsets are not assumed. A global clock object (HD reads `clock + 0x44`) was not found either: the words close to the group's value
  outside it number in the thousands (every timer near the same age).
- **Omega.** Checked, not assumed (`docs/tools/compare-page.md`, "The world clock and random craft motion, per title"): Omega's nodes
  are HD's class (the `MeshImporter` equivalent in the PS4 executable was not read) and the native port's memory is readable, so the same
  scan can be run there; not done in this lane.
