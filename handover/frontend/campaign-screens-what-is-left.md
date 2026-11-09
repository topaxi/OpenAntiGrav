---
status: measure
titles: [pulse, hd, omega]
updated: 2026-11-09
summary: "Campaign screens draw and launch; left: HD Results button glyph, Line8 and Zone/Elim records, Grid1 purpose, ticker speed, Cell Help scroll"
---

# The campaign screens: what is left

The Race Campaign's `Grid Selection`, `Cell Selection` and `Cell Help` draw and launch on Pulse, HD/Fury and Omega, with
medals, the footer legend, the ticker, the cursor memory and the Selector bloom. Everything that landed, with its evidence
and confidence, is on `docs/ui/campaign-screens.md` (start at its "Open" section) and
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`. This file keeps only what is still open.

## Open

- **HD (and Omega) end-of-race button glyph.** `oag_ui_screens::endrace::hd`'s `text_draw` has no `face_role` check, so
  `ControlTextConfirmButton` (`font="buttons"`) on `Results`/`Rewards`/`Menu` draws nothing, though the `Buttons` atlas now
  loads for the campaign footer. Evidence: `docs/formats/hd-endrace-screens.md` (the `ControlTextConfirmButton` row) and the
  comment in `crates/ui-screens/src/endrace/hd.rs`. Ready to wire: route the widget through
  `campaign::footer::face_role` and `Renderer::set_buttons_atlas`. Omega draws HD's end-of-race screens, so it follows.
- **`Line8` and the `Zone`/`Elimination` saved records.** `Line8` authors the same `OffsetX="260" OffsetY="180"` as `Target0`
  and nothing traced says what the original shows there. `oag_game::records::Record` keeps no raw zone or kill count, so
  `Line5` answers `None` for those modes. `docs/ui/campaign-screens.md`, "The tip ticker, the Confirm/Back legend, `Cell Help` and a podium".
- **What `Grid1` is for** (the second, `startenabled="false"` `GridController` on `Grid Selection`). Confidence 50. Four bursts
  from 50 ms after a page turn show no crossfade, which leans to a pre-settled buffer swapped at once. Skipping it is safe
  (confidence 60). `docs/ui/campaign-screens.md`, "Two `GridController`s, and only one is drawn".
- **The ticker's scroll speed** is `oag_ui::anim::MARQUEE_SPEED` (60 px/s), chosen, not measured. Settle it with two PPSSPP
  frames a known interval apart on a long tip.
- **`Cell Help`'s `Viewport`/`Animation` scroll timeline** (`LimitVerticalScroll="10"`) is not scripted, on purpose. The static
  overlay was seen on PPSSPP and matches.
- **A new difficulty-tagged medal, played end to end.** `Race_RecordResult`'s `record+8`/`record+9` write is unit-tested
  against the decompile (`crates/game/src/records/tests.rs`, `crates/ui-screens/src/campaign/draw/tests.rs`); a full Confirm,
  race, finish, re-entry loop producing a fresh `Gold (Medium)` line was not driven live.
- **Pulse PS2** on these screens is audited for the hex textures, the selector offset and the ticker clip only.
- **A fresh save's empty hex fill** was never reached on PPSSPP (the profile stayed at zero medals).

## Next Steps

1. Wire the `Buttons` face into `endrace::hd`'s `text_draw` (the one ready-to-wire item here; an afternoon with the tests).
2. Two PPSSPP frames of the ticker a known time apart to settle its speed (half an hour).
3. The rest wait on a reason to open them; none changes what a player sees today.
