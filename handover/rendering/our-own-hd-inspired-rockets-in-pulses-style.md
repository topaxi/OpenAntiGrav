---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-09
summary: "Option, default off: our own HD/Fury-inspired rocket effect on Pulse and Pure, in Pulse's design language, no disc bytes"
categories: [rendering]
---

# Our own HD-inspired rockets, in Pulse's style

2026-11-09, drive lead, a lane the maintainer asked for. The maintainer says Pure's rockets look quite
bad today and Pulse's are quite simple.

## Open

- **Our own effect and data (maintainer):** nothing is loaded from any disc. It is HD/Fury-inspired but
  made in Pulse's design fidelity, style and visual language: `.pob` palette, sprite language, additive
  glow, colour range. It must not look like a pasted-in PS3 effect.
- HD's rocket is the reference for which elements to have and how they read in motion: projectile,
  exhaust flame, smoke trail, launch flash, impact.
- Visual only: rocket gameplay, timing and the simulation do not change.
- Code to start from: `crates/fx/src/rocket_smoke.rs`, `crates/fx/src/flash.rs`, `crates/fx/src/psys/`,
  `crates/fx/src/fat_line.rs`; see also [weapon-lineage.md](../../docs/gameplay/weapon-lineage.md).

## Next Steps

1. Look at HD's, Pulse's and Pure's rockets side by side, in captures over several frames, and write down
   which elements to build.
2. Build them behind a graphics setting (default off), as procedural WESL or tracked assets of ours.
3. A `menu.toml` row with `string_id` and `english.toml` text, dropped on HD, 2048 and Omega;
   `just audit-leakage` stays clean.
4. Acceptance: off is byte-identical on Pulse and Pure, pinned by a test. On has captures of launch,
   flight and impact on both titles, beside HD's rocket and the Pulse original, judged as a player would.
