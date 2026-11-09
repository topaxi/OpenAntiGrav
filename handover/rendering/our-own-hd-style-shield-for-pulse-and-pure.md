---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-09
summary: "Option, default off: our own HD/Fury-style shield effect on Pulse and Pure, authored by us, no bytes from any disc"
categories: [rendering]
---

# Our own HD-style shield effect for Pulse and Pure

2026-11-09, drive lead, a lane the maintainer asked for.

## Open

- **Our own effect and data (maintainer):** nothing is loaded from the HD, 2048 or Omega discs. We
  author our own shader and parameters, procedurally in WESL or as a tracked asset under `assets/`, made
  to look like HD's shield. It works with only a Pulse or Pure disc. This is a sanctioned exception to
  "never invent what the assets author", because it is an opt-in extra of ours, not a stand-in for
  authored data.
- HD's measured shield is the visual reference only:
  [weapon-absorb-pairs.md](../../docs/rendering/weapon-absorb-pairs.md),
  `crates/raceplay/tests/hd_shield_colour_ground_truth.rs`, `crates/render/src/shield.rs`.
- The menu row is shaped like `graphics.wingtip_contrails`, but with no cross-disc source. It is dropped
  on HD, 2048 and Omega, which draw their own shield.

## Next Steps

1. Build the effect beside the existing shield in `crates/render/src/shield.rs`, selected by a graphics
   setting (default off).
2. A `menu.toml` row with `string_id` and `english.toml` text (`just check-strings`); `just audit-leakage`
   stays clean.
3. Acceptance: off is byte-identical on Pulse and Pure, pinned by a test. On is shown in a capture beside
   an HD shield frame.
