---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-09
summary: "HD wingtip locators run two smoke ribbons while a speed pad or turbo acts; trigger read, ribbon push law unread, ours draws none"
---

# HD's wingtip vortices: recovered trigger and data, no ribbon drawn

Found 2026-11-09 (lane `hd-nose-glow-control`) while checking whether the original's white nose glow is its own effect
(it is not). A player reports white contrails at the Zone ship's wing tips; this is them, probably. Evidence:
[`wingtip-vortices.md`](../../docs/ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md) (the update, read in full, and
the live node fields) and [`hd-wingtip-contrails.md`](../../docs/rendering/hd-wingtip-contrails.md) (frames: six with the
speed-pad state up show a thin pale streak from each wing tip, six without show none).

## Open

- **Data and trigger, named.**
  - Two `wingtip_left`/`wingtip_right` locators (class `0x3ec`) on every hull's `Locators.vex`; the same in 2048 and
    Omega at the same coordinates; Pulse has none.
  - `ShipWingtipVortices_Update` (`0x002db130`) refills a 1.3 s timer while `craft-entry+0x2e8` (the speed pad's
    remaining force time) is above zero or Turbo flag `0x20`/`0x40` is set. It lays a ribbon of pool `+0x74`
    (`smoke_trails_frame1_ALPHA`: widths `0.05` to `1.5`, life `0.7` s, three chains, 100 nodes, helix radius `0.8`)
    with a strength of the timer's fraction times a speed ramp (zero below 50 units/s, `0.3` from 100).
- **Unread, and the reason this is not ready to wire:** what `0x002ac208` (the push) and the pool's element class
  `0x002ac070` do with the strength and the side byte (`node+0xe8`, `1` on the left node), and the helix handedness.
  Read recipe: disassemble both with capstone over `EBOOT.elf` (`scripts/ps3-toc.py` resolves the TOC; the `FatLine`
  base is read in `missile-ribbon.md` and implemented as `crates/fx/src/fat_line.rs`), then confirm live: the node's
  `+0xe0` handle is the pool element, its fields move with the timer.
- **Not shown:** a natural speed-pad or Turbo crossing (the field was forced); the Zone ship's pair; the same update on
  Omega (find it from the importer string `ShipWingtipVortices_Importer.cpp` in `eboot.bin`, search key the constants
  `50`, `100`, `0.3`, `0.006`, `1.3`) and on 2048 (encrypted `eboot.bin`).
- **Ours** draws none of it: no `wingtip` locator is read (`oag_vex::vex::class_names` names it, nothing consumes it).

## Next Steps

1. RE lane: read `0x002ac208` and `0x002ac070`, name them, finish the law (an hour or two with the Missile's `FatLine`
   page open).
2. Measure lane: a Time Trial pad crossing on Talon's Junction and a Zone run over a pad, frames at 0.1 s steps from the
   crossing, ours rendered at the same pose; then wire the two ribbons on the craft's wingtip locators.
