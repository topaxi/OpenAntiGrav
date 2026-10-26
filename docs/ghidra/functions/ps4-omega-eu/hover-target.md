# Omega's grid hover clamp: HD's `Craft_Update` block, field for field

2026-10-26, lane `omega-grid-height`. Binary `eboot.bin` (Omega EU, image base `0x01000000`, Ghidra program
`/omega/eboot-ps4-omega-eu.bin`). The question: Omega's craft sat visibly higher above the floor than the original's on
the grid (both compare pairs), and HD holds its craft low there
([hover-target.md](../ps3-hdfury-eu/hover-target.md)). Read statically, no live capture (no PS4 emulator route reads
craft memory). Unnamed: the function stays `FUN_01316960` (the craft update, by this block alone; not renamed, below 50
confidence in the name).

| Omega `FUN_01316960` | HD `Craft_Update` | Meaning |
| --- | --- | --- |
| `entry+0x654` | `+0x344` | the hover target |
| `entry+0x658` | `+0x348` | the clamp: jitter in the grid state, a seconds timer once raced |
| `entry+0x65c` | `+0x34c` | the re-roll countdown |
| `entry+0x2c4` bit 1 | `+0x264` bit 1 | the grid state |

The fields sit `0x310` higher than HD's, and the code is the same:

- `0x01316e0a`-`0x01316e23`, grid state (bit set): `+0x654 = +0x658` (the clamp replaces the base), countdown `+0x65c -= dt`.
- `0x01316e3d`-`0x01316eb2`, countdown expired: `+0x608 = 3.0` (`0x40400000`), `+0x65c = rand8 * 0.002` (`0x01801ccc`),
  `+0x658 = 3.0 + rand8 * 0.0003` (`0x01801cd0`); `rand8` is the low byte of `0x017a59f0`'s result.
- `0x01316df3`-`0x01316dfc`, raced (bit clear): `+0x658 += dt`, one unit a second.

Confidence 80 for the law (instruction for instruction the same as HD's, the min against the race base and the `0.75`
scale were not located in Omega's copy of the block). The clamp number is HD's mean, `3.03825`, **chosen**; the release
rate `1.0` is read off the instruction (**measured**, static). Whether Omega skips the clamp on Zone modes is **not
read**: `in_zone` is inherited from HD.

## Effect (2026-10-26)

`oag_omega::race::LAUNCH_HOVER` wires it. Both Omega compare pairs, ours re-rendered at tick 100, mean grey difference
against the original: Vineta K 56.23 to 46.69, Anulpha Pass 75.90 to 61.50 (edge correlation 0.08 to 0.18 and 0.07 to
0.23). The floor and the grid pads now sit where the original's do; before, the track dropped away under the craft.
Craft nose tip row is unchanged (the camera follows the craft), the floor moved.

## Lineage

- 2048: **ported as inherited** (`oag_2048::race::LAUNCH_HOVER`, release rate and Zone flag `InheritedFrom("Wipeout Omega
  Collection")`). Not read on 2048's own Vita executable; the title shares the `Backend/...` tree with HD and Omega.
- HD: unchanged, one HD pair re-rendered byte-identical before and after.
