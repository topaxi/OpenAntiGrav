---
status: blocked-maintainer
titles: [hd]
updated: 2026-11-10
summary: "HD's 1080p width controller is read and every pair names its width; pin references at 1920 or reproduce it is the maintainer's choice"
---

# HD's 1080p dynamic width: located, the pairs name it, a choice is open

The law, the evidence and the frame estimator are on
`docs/ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md`; the per-pair widths are in
`docs/tools/compare-page.md` ("a pair names the width"). In short: the scene is drawn
`1920 - 20 * level` wide, level 0 to 31; the end of `Render_FinishFrame` (`0x003e3320`) steps it
once a frame, narrower when the RSX's timed frame is 16112 us or more or the flip missed its
vblank, wider under 15556 us; off (1920) in the menus and Photo Mode. Both inputs are wall-clock,
so RPCS3 on a loaded host sits at 1300: nine of the thirteen stored race originals are 1300, one
is 1400, three are 1920.

## Open

- **How references and ours treat it (maintainer's choice).** Two faithful options, not exclusive:
  1. **Pin the reference captures at 1920, for comparison only (recommended first).** What a pair
     compares is then the scene, not the host's load on the day. Clear the byte at `0x00c81da4`
     after the race has loaded (tested live: level 0 on 1,047 of 1,047 samples, ring only 1920),
     or patch `0x003e33c0` from `0x881f0324` to `0x38000000` in `patch.yml` beside the hull-shake
     patch (the patch itself is untested). Cost: the nine 1300 pairs and sol-2 are re-captured, so
     their scores move for a reason that is not ours; record `original.scene_width` as pinned.
     About an hour with a working menu walk.
  2. **Reproduce the controller as an `original` value of a setting.** Per-title `Title` data: 32
     levels of 20 px, the two thresholds, one step a frame, off in Photo Mode and the menus, fed
     by our own GPU frame time. It is what a PS3 does, but its visible effect on a PS3 is
     unmeasured (nobody here has watched a console leave 1920), and on a host that holds 60 Hz it
     would never engage. Our dynamic resolution (`oag-present`) already covers the purpose; this
     would be a second, horizontal-only mode. Size: a day (confidence 40). Not recommended before
     a real console's behaviour is known.
- **A PS3's own behaviour.** How often a console leaves 1920 and on which circuits. Needs
  hardware or a capture of one; RPCS3's timing is the host's.
- **A new pair end to end.** `rpcs3-hd-pair.py` writes `original.scene_width` (run on a stored
  capture with `--json`) and `rpcs3-drive.py capture` records `ship.scene_width` (seen live), but
  no single new pair was captured: the camera pick returned null three times in the attract race
  and the menu walk's presses were dropped on this lane's instance.

## Next Steps

1. Decide between the options above. For option 1: add the word to the patch file
   `scripts/rpcs3-hd-no-hull-shake.py` writes, boot, check `python3 scripts/hd_scene_width.py`
   reads 1920 on a race frame, then re-capture the ten narrow pairs.
2. Small, any lane: the five target arrays beside the colour targets by role, and whether
   `Debug.Enable Frame Buffer Throttling` reaches a race set-up (both listed as open on the page).
