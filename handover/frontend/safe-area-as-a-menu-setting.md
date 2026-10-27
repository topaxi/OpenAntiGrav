---
status: re-first
titles: [all]
updated: 2026-10-27
size: L
difficulty: medium
summary: "Offer the safe area (screen size) as a menu setting: HD's own Screen Size slider first, find Pulse PS2's, a shared invented screen elsewhere"
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

## Open

- Whether Pulse PS2 (or Pulse PSP, Pure) has a screen-position or screen-size option, and what it changes
  (PS2 games often offer a picture-position nudge instead of a scale). Unread.
- Whether 2048 (Vita, fixed screen) or Omega have anything comparable. Probably not; confirm, never assume.
- How HD's live Game Options screen draws the slider (it is authored XML, so ours may already be able to
  draw the row once wired).
- For titles without an original option: what the invented screen looks like. It belongs in the fallback
  asset layer, labelled ours, and should be shared across titles rather than drawn once per title.

## Next Steps

1. Wire HD's own `Screen Size` slider row on its Game Options page from the disc XML, writing
   `hud_safe_area` live (front end and HUD together), with pointer support.
2. Search Pulse PS2's executable and front-end tables for a screen-adjust option (strings such as
   "screen", "position", "centre"); record found or not found per title on the title's frontend page.
3. If a title has none, add one shared invented "Screen size" page (slider 80-100, step 2, default the
   title's own value or 100 where the original is 1:1, corner markers), labelled as ours.
4. Phones and notches (maintainer, 2026-10-27): the platform's safe-area insets (Android display cutout,
   iOS safe area) are the setting's **default** on a phone, not a floor. A player may still choose a value
   beyond them ("if someone wants to go below it, so be it").
