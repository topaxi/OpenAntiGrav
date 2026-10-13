---
categories: [frontend, tooling]
---

# Graphics settings resolve from what the player wants and the situation they are in

2026-10-13. A design idea from the maintainer, written down on request. Nothing
is built, no ADR exists, and nothing here is measured. Priorities are the
maintainer's: **1. Steam Deck on a TV, 2. Android power saving, 3. everything
else.**

## The idea

Most games keep one store of graphics settings. That breaks when the situation
changes under it: split screen, power saving, a bigger display. Separate three
things:

1. **What the player wants** (persisted). A few preferences that rarely change:
   smooth over pretty, save battery, match the original, a frame-rate target,
   plus any value the player pinned by hand.
2. **The situation** (detected every session, never persisted). The output
   display's size and refresh rate, power-saving or battery state, thermal
   headroom, view count (split screen), and device class.
3. **What gets used** (worked out from 1 and 2 every time either changes).
   Render scale, LOD and draw distance, bloom, motion blur, shadows, MSAA, frame
   cap.

Resolution works like a CSS cascade, in order: the player's base settings, then
situation rules ("views > 2", "power saving", "output above 1080p"), then values
the player pinned by hand, which always win.

## Why the cases do not conflict

The situations cost different things, so each rule touches different settings:

| Situation | What grows | What to change | What to leave |
| --- | --- | --- | --- |
| Steam Deck on a TV | Output pixels only; scene cost is the same | Hold the render resolution, upscale to the output (FSR is in the tree) | Effects, LOD |
| Android power saving | Time and energy per frame matter more | Frame cap, render scale, effects, in proportion | Nothing in particular |
| Split screen | Draw calls (8 views: 6,507 draws against 465 for one), not pixels | Draw distance, LOD, effects per view | Resolution; each view already has fewer pixels |

## What already exists

- **A per-situation layer is already there.** `[render_profiles.<title>
  (<platform>)]` in `crates/game/src/settings.rs` keys `render_scale`,
  `target_fps` and `minimum_resolution` by (title, original platform). The
  cascade generalises that key from "which title" to "which title in which
  situation".
- **Dynamic resolution** (`docs/rendering/dynamic-resolution.md`, ADR-0037,
  ADR-0040, ADR-0042) is a closed loop that reacts to a frame's cost after the
  fact. It is off by default. The cascade would decide in advance; dynamic
  resolution would then absorb what remains. They complement each other.
- **Split screen already does it by hand.** It switches off Pulse bloom, motion
  blur and jitter under a split (`docs/rendering/split-screen.md`). That is a
  situation rule hard-coded in one place.
- **Upscaling** is separate from render resolution already (ADR-0012, ADR-0045),
  which is what the Deck-on-a-TV case needs.

## Priority 1: Steam Deck connected to a TV

The case: the Deck panel is 1280x800; docked to a TV the output becomes 1080p or
4K. Rendering at the output resolution multiplies the scene's pixel cost on the
same APU.

Rule to build first: **when the output grows past what the device was tuned
for, keep the render resolution and upscale**, rather than following the output
size. Questions to answer:

- How is "docked" detected? Output size change on the surface is the
  obvious signal; whether the Deck reports anything more specific is unread.
- What is the reference resolution: the internal panel's, or the last one the
  player was happy with?
- Does the HUD follow the output (it composites at presentation resolution,
  ADR-0036) while the scene stays at render resolution? It should already.
- The maintainer plays vsync on the Deck (RADV). Test there; the "Steam Deck:
  all or nothing" rule applies to agents testing on it.

## Priority 2: Android power saving

The case: the OS lowers resolution in power-saving mode, which already helps.
We can do more: lower the frame cap, drop effects, shorten draw distance.

Questions to answer:

- Which Android API reports power-saving and thermal state
  (`PowerManager.isPowerSaveMode`, the thermal status listener), and how
  `oag-mobile` (`crates/mobile/src/android.rs`) reaches it from the native side.
- Is the OS's own resolution drop visible to us as a smaller surface, and does
  the cascade double-count it?
- The test device is the maintainer's Galaxy S24 (Exynos 2400). A member stops at
  an APK and steps; the maintainer tests.

## Priority 3: everything beyond

- Split screen's hard-coded rule moved into the cascade, plus per-view LOD and
  draw distance against the 8-up frame budget (unmeasured).
- High-refresh panels: a frame-rate target that follows the display.
- Thermal throttling on any device, and iOS.
- The `original` setting value (offered only on titles whose original had the
  effect) as one of the player's preferences, not a situation.

## Open design questions

- **Does a situation rule ever override a value the player pinned?** Default
  lean: no, but show it ("Shadows: High, reduced while split") so nothing changes
  silently.
- Where the rules live: code, or data in a settings file a player or mod can
  edit.
- How the settings menu presents worked-out values versus chosen ones.

## Next Steps

- Priority 1 first: detect a docked Deck (output size change), hold the render
  resolution and upscale. Measure the frame time docked and undocked before
  choosing thresholds.
- Write an ADR once the cascade's shape survives the first two cases.
