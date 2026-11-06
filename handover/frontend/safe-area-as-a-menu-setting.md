---
status: parked
titles: [all]
updated: 2026-11-07
size: L (50)
difficulty: medium (60)
summary: "SCREEN SIZE row landed on all five titles; open: corner markers, phone insets, Pure options XML unlocated"
---

# The safe area as a menu setting

2026-10-27, maintainer request (not taken up by the drive that noted it). HD's front end and HUD now draw
inside HD's `Safe Area Setting` (0.96 at the compare saves, default 90; maintainer: keep the original 90 %,
"especially for phones and notebooks with rounded edges"). Today a player can change it only in the
settings file (`[graphics] hud_safe_area`, `crates/game/src/settings.rs`); there is no menu row. Console
games usually offer this on a dedicated screen-adjust page, often with corner markers to line up against
the TV's edge.

What is already known:

- **HD authors the option.** `additional_definition.xml` (DATA06's copy, the live Game Options screen)
  has `<Slider name="Safe Area Setting" ... default="90%">`, `minSlide="80" maxSlide="100" incSlide="2"`,
  labelled `Screen Size` (`OPT_SCREEN_SIZE`). Evidence and the RPCS3 read of the save's value 96:
  `docs/formats/hd-hud.md` ("The whole HUD is drawn at the Safe Area Setting"). The consumer in the
  executable is unread (confidence 70 that the HUD scale is value / 100).
- **Ours.** `oag_title::hud::SafeArea` on `HudArt::safe_area` (HD measured, every other title `None`);
  the front end uses the same value through `Renderer::set_inset` (`docs/formats/hd-frontend.md`).
- **Omega** draws its front end 1:1 and authors no slider (checked, differs).
- **Fury** ships with HD: check whether its options page carries the same slider.

## Landed (2026-11-07, `screen-size`)

- Every title offers `SCREEN SIZE`: the DISPLAY page row `graphics.hud_safe_area` (`assets/ui/menu.toml`, values from
  `HudArt::safe_area` through `ValueSource::SafeAreas`), applied live to the menus' inset, taken by a race's HUD at race
  load. HD keeps the disc's slider (default 90); Pulse, Pure, 2048 and Omega carry `SafeArea::SHARED` (default 100,
  **chosen**). 2048's options are the disc's touch hub, so `OptionsCamera` has a second control there and the touch
  front end takes the inset. Evidence, the per-title search and frames: `docs/formats/hd-hud.md`, "SCREEN SIZE on every
  title". Fury is HD's disc and carries the same slider (checked).
- Tests: `crates/hud/tests/safe_area_ground_truth.rs`, `crates/game/tests/menu_screen_size.rs`,
  `drive_ground_truth::the_screen_size_row_saves_hud_safe_area`,
  `vita_2048_boot_ground_truth::options_camera_carries_the_screen_size_control`.

## Open

- **Corner markers** (optional): a page that draws the safe edge to line up against the TV's. Not built.
- **Phones and notches** (step 4, out of scope here): the platform insets as the setting's default on a phone.
- **Pure's own options XML** was not located by name (`PI001\GUI\Additional_Definition.xml` is absent), so "Pure
  authors no option" rests on its `Skin.xml`, `Selection_Definition.xml` and `BOOT.BIN`.
- **The value is one global**: a 96 set on HD is 96 on Pulse too (maintainer's decision); the default is per title.
- **The boot screens** (logos, language picker, loading's neighbours on a non-touch title) are not inset; their
  original is unmeasured at a safe area. A race's HUD is not live because no page opens over a running race.
- **HD's own Game Options screen** (`additional_definition.xml`, `DATA06`) is not drawn by this build: the live page is
  our `menu.toml`, so HD shows our label (English words identical to `OPT_SCREEN_SIZE`), not the disc's string.
- **Split screen** HUDs are not scaled (`draw_views`, unmeasured).

## Next Steps

1. Corner markers on a screen-adjust page, if wanted.
2. Phones and notches: platform insets as the default (step 4 of the original thread).
3. Locate Pure's options XML and record found or not found on its frontend page.
