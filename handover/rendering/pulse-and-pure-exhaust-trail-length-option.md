---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-09
summary: "Option, default off: lengthen Pulse's and Pure's exhaust trail to HD/Fury's length, keeping head width, fade and taper shape"
categories: [rendering]
---

# An option to lengthen Pulse's and Pure's exhaust trail to HD/Fury's

2026-11-09, drive lead, a lane the maintainer asked for. It is an enhancement of ours, a departure from
the original, and off by default.

## Open

- Pulse's and Pure's engine trail is `TRAIL_SAMPLES = 10` (`crates/fx/src/exhaust.rs`, from `ring+0x04`
  at preset 2). The taper rate derives from it. HD/Fury's trail is longer.
- HD's trail length has to come from what is already measured or wired for HD
  ([trail-ribbon.md](../../docs/rendering/trail-ribbon.md),
  [hd-ship-materials.md](../../docs/rendering/hd-ship-materials.md)). If it is not on record, say so
  rather than guess.

## Next Steps

1. Find HD's trail length on record.
2. A graphics setting (default off) scales the trail length, either in samples or in sample spacing.
   It keeps the head width, the fade law and the taper's shape. Use fixed arrays and no per-frame
   allocation.
3. A `menu.toml` row next to TRAIL ORDER, with `string_id` and `english.toml` text (`just check-strings`).
4. Acceptance: off is byte-identical on Pulse and Pure, pinned by a test. On is shown in a capture.
