---
categories: [rendering, frontend]
---

# Bloom should be a level, not a boolean: a design for the maintainer to decide (nothing built)

2026-10-02, `bloom-setting` lane. The maintainer, playing Pulse: bloom "was VERY
strong", and asked whether it should be per title or platform and whether it
should be a level instead of on or off. **This is a proposal. Nothing below is
implemented, and every number in it that is not in the Measured section is
chosen, not measured.**

## Measured first

Ours was 4.4-5.7 times the original over a matched racing frame and 1.4 times
at rest, and that is fixed on this branch (per-tap byte truncation in
`bloom.wgsl`; `docs/ghidra/functions/psp-pulse-usa/bloom.md`, "Is ours stronger
than the original?"). Whole-frame mean luma the bloom adds, racing straight:
original 1.0-1.3, ours before 5.6-5.7, ours after 1.55-1.63; at rest on the grid
original 8.35, ours before 11.8, ours after 8.3. It was a wash over the whole
picture, not the exhaust, and it is resolution independent already. Ours is
still about 1.2-1.6x the original on a racing straight (1.3x around the craft),
so a `Low` level has something real to offer now. The boost pad's glow is the
original's own look (same size in both), so a player who found the pad flash
"very strong" is describing the original, and a level, not a fix, is the answer
there. The glow-mask value of the pad is a texture byte, not the stamp.

## What runs today, per title

| Title | Bloom path | Gated by `graphics.bloom` | Original reference |
| --- | --- | --- | --- |
| Pulse PSP | `post::bloom`, four passes, constants from `BOOT.BIN` | yes | always runs (121 of 121 frames) |
| Pulse PS2 | none: the glow mask is not measured there (`scene.rs`: `measured_mask`) | n/a | unread |
| Pure | none, same reason | n/a | unread |
| HD / Fury | `post::hd_bloom`, five stages, per-circuit `HDR and Bloom` values | yes, as `Glow::Drawn` or `Suppressed` (the chain still runs; only the glow term goes) | read, not measured as a player option |
| Omega | none: the patch carries no `HDR and Bloom` block | n/a | n/a |
| 2048 | none | n/a | unread |

The original exposes no bloom option in any title. A level is therefore ours by
definition, and the only faithful value is the title's own.

## Proposal

1. **One global setting in `Graphics`, not one per title.** Per title and
   platform storage already exists (`RenderProfile`, one per title and
   platform, for render-cost knobs), so a per-title bloom level is a field move
   plus a `migrate` step, not new machinery; it is not recommended only because
   a look preference is the player's, not the title's.
   **Recommendation, in detail:** `Graphics::bloom` becomes an enum
   `Bloom { Off, Low, Original, High }` beside `MotionBlur { Off, Low, Medium,
   High }` in `oag-display`. Its meaning is relative to whichever path the
   running title has: `Original` is "that title's recovered values, unscaled",
   so it is correct on Pulse PSP and HD without a table of titles. A per-title
   override is not worth building: the setting only matters on two paths, and a
   player who wants a different look on HD can change it when they race there.
2. **A level scales one number, and no threshold.** Neither path has a
   brightness threshold (the bright pass is `rgb * mask`), so a threshold level
   would be inventing a different effect. On the PSP path the one free scalar
   is the composite's `GU_FIX` factor, `0xaf`; on HD it is the glow term's
   `alpha_contribution`. Chosen, not measured: `Low` is 0.5x, `Original` 1.0x,
   `High` 1.5x. `Off` takes the existing no-pass path, and `Low` and `High`
   cost the same as `Original`.
3. **`Original` is the default**, as `true` is today, so a fresh profile
   matches the capture the project compares against.
4. **Migration.** Accept both spellings on read (an untagged deserialiser):
   `bloom = true` reads as `Original`, `bloom = false` as `Off`, a string reads
   as a level, and the next save writes the string. Do it as a step in
   `settings.rs`'s `migrate(&mut toml::Table)` chain, where the other key
   migrations live. A build older than this change **cannot** read the string:
   `settings::load` treats a malformed value as an error ("a typo should be
   visible"), so a downgrade fails to start with the key named. Say so in the
   change, or keep writing a `bool` alongside for one release. The menu row is a cycle row like motion blur, with its `string_id` and
   English text in the same change (`just check-strings`).
5. **Plumbing.** `bloom_enabled: bool` threads through `Scene::new`,
   `hd_chain::build`, `capture.rs`, `headless.rs` and `stage.rs`; it becomes the
   enum, and `Bloom` keeps the composite strength in a constants buffer it
   writes through the queue the way it already does for the bright pass.

## Alternatives not recommended

- **Per title or platform storage.** More rows, and the same two code paths.
- **A free slider.** The recovered strength is a byte; a slider invites tuning
  by eye away from the one value that is measured.
- **A threshold or radius level.** Radius is the original's 240 x 136 buffer
  and is why the look is resolution independent; changing it per level breaks
  that.

## Open

- Why ours is still 1.2-1.6x on a racing straight (candidates: the flare and
  plume's drawn size and mask ramp at 85 km/h; not varied here).
- A matched boost-pad on/off pair in the original (1 of 4 placed approaches
  triggered the pad in this lane).
- HD's own strength at matched poses: this lane measured Pulse PSP only.
- Whether the maintainer wants `Low`/`High` as multipliers (above) or
  `Original`/`Low`/`Medium`/`High` with `Medium` as the faithful value.

## Next Steps

1. Maintainer picks the level names and the multipliers.
2. Explain the residual 1.2-1.6x before adding `High`.
3. Implement points 1 to 5 above; add the `bloom = true` migration test.
