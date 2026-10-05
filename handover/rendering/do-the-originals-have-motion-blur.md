# Do the originals have motion blur? Never checked, and the docs contradict themselves

2026-10-05, queued by the maintainer and deferred to a new session. No work
started. Our motion blur ([motion-blur.md](../../docs/rendering/motion-blur.md))
is documented as **invented**, resting on the line "Wipeout Pulse has no motion
blur". That line carries no evidence, and other pages in this tree say otherwise.

**Maintainer decision, same day:** if an original has motion blur, the motion
blur setting (`oag_display::display::MotionBlur`, `[graphics] motion_blur`,
`crates/game/src/settings.rs`) gains an `original` value that plays that
title's recovered law. It is offered only on the titles that have it, like the
shadow setting's existing `original` tier
([shadows.md](../../docs/rendering/shadows.md)). A saved `original` on a title
without blur falls back to off with a load-report line. The existing
low/medium/high values stay.

## Open

Leads, from strings and docs only, none verified:

- **Pulse PSP**: [magfloor-fx.md](../../docs/ghidra/functions/psp-pulse-usa/magfloor-fx.md)
  (around line 314) says "the original's capture carries the PSP bloom and its
  own motion blur". The BOOT.BIN strings include `blurX`/`blurY`/`CalcBlur`
  (possibly bloom's). PSP titles commonly fake blur by blending the previous
  frame, so check that and rule out PPSSPP or LCD ghosting as the source.
- **Pulse PS2**: [camera.md](../../docs/ghidra/functions/ps2-pulse-eu/camera.md)
  (around line 412) has an unconsumed camera roll angle at `+0x124`, with
  "motion-blur direction" as one guess.
- **HD/Fury**: [hd-hud.md](../../docs/formats/hd-hud.md) (around line 763)
  describes an RPCS3 capture at 529 km/h as "heavy motion blur". The EBOOT has
  DoF blur programs (`FunkLayerDofBlur*_fp`), bloom blur and photo mode's
  `photomodeRadialBlur`, but no pass named MotionBlur. What blurs that capture
  is unknown: a speed radial blur, DoF, or frame blending.
- **Omega**: [lightmap-prelit.md](../../docs/ghidra/functions/ps4-omega-eu/lightmap-prelit.md)
  (around line 182) shows a settings registrar naming a `MotionBlur` group
  beside Vignette, DepthOfField and Water. It most likely has one; its keys and
  consumer are unread.
- **2048**: `ImageMotionBlur` and a front-end `Motion Blur Ship Button`, which
  look like UI. Check the `.gxp` program names (`scripts/vita-gxp.py`).
- **Pure**: the `ImageMotionBlur` string only. That string ships in every title
  from Pure on and is probably a front-end widget type. Confirm that and exclude
  it rather than count it as evidence.

## Next Steps

Suggested shape: one `oag-re` lane on opus. Use `mblur-originals` as the lane
name if it is driven.

1. **Static, per title** (about 2 hours a title): find the race-time blur pass
   or prove its absence. Ghidra programs as named in `docs/ghidra/`. Stay in the
   post-processing, blur, frame-blend and render-target-feedback functions.
2. **Live, where static reading is ambiguous**: a matched pair of frames at
   speed versus at rest in the original, with the blur measured as a streak
   length or blend weight. Use PPSSPP's software renderer for the PSP titles
   (the reference), RPCS3 for HD and Vita3K for 2048.
3. **Write the result per title** as recovered or absent, with confidence and
   evidence, in [motion-blur.md](../../docs/rendering/motion-blur.md). Fix the
   unsupported "Pulse has no motion blur" line either way. Add a row to
   [status.md](../../docs/overview/status.md) and apply the 2048 to Omega check.
4. **Wire `original`** only for a title whose law is recovered at confidence 70
   or more: strength against speed, direction, the layers it skips, whether the
   HUD is excluded. Below 70, record the law and do not wire it. The new option
   needs its `string_id` and English text (`just check-strings`). Verify as a
   player: our `original` against the original's capture, at speed and at rest.
