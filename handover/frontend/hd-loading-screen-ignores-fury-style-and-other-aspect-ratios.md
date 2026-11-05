---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-11-05
summary: "HD's loading screen shows white HD feature art under the Fury style, and its images do not fit on another screen size or ratio"
size: S (60)
difficulty: easy (50)
---

# HD's loading screen ignores the Fury style and other aspect ratios

Two reports from play. Under the Fury front-end style the loading screen's feature images are the
HD ones, on white. And the images are not scaled into their place when the screen has another size
or ratio. Omega's front end is HD's, so check both there.

## Open

- **Style.** `oag_hd::loading::FEATURES` holds both decks and `FeatureStyle` picks by name;
  `oag_title::loading::Loading::features` says the first entry, `STYLE_HD`, is what a source with no
  saved choice gets. `display.front_end_style` starts empty (`args::style_of` makes that `None`).
  The report was on the default, with no style picked in OPTIONS, so the first reading to prove is
  that an empty setting gets the HD deck while the menus lead with Fury. Check the second too: the
  choice is resolved once at boot (`Session::front_end_styles`, `loading_assets`), so a style changed
  in OPTIONS may not reach the deck until a restart.
- Also check the globals and the mode icon follow the same choice, not only the feature pictures.
- **Scale.** `LoadingScreen_HD.toml` draws one 1920x1080 grid under the Safe Area, picked at a 16:9
  default aspect, with no per-aspect `[[layout]]` (`docs/formats/screen-layout.md`). The panels and
  the text may scale while the pictures do not, or the reverse: measure where each lands at 1280x800,
  2560x1080 and a phone's ratio, using `--loading-screen`. The panel is the feature image rect, 160 to
  950 by 155 to 655 in that grid. How the original fills another ratio is not read; a 16:10 Deck or a
  phone is ours to fit, so label the fit chosen, not measured.

## Next Steps

1. Reproduce both with `--loading-screen` stills: empty setting, setting `FURY`, a live switch.
2. Fix the style so the deck follows what the menus show, with a test that goes red without it.
3. Make pictures land inside their panels at every ratio; a test on the placed rects at 16:10 and 21:9.
4. Update `docs/formats/hd-loading.md`.
5. Follow-up, after the fixes above: Omega draws no loading screen, reported from play, and its
   layout is expected to be close to HD's. Read what Omega's own disc ships before reusing HD's file,
   check it against 2048 per CLAUDE.md, wire it as `Title` data, then delete this thread.
