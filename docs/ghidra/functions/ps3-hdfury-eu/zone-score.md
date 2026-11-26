# HD's Zone score hangs on a string of `3`s

`EBOOT.elf` (WipEout HD/Fury, PS3, `BCES-00664`), segment 0 maps file offset `0` to vaddr `0x00010000`. Static reading
(2026-11-26), corroborated by a frame of the running original, so the score is capped at 84
([rubric](../../../reverse-engineering/confidence-rubric.md)).

**The question.** `Zone_HUD.xml` authors `Score` as `align="left"` with no x, a child of `ScoreFrame`
(`OffsetX="960"`), yet the original's frames read it centred
([hd-hud.md](../../../formats/hd-hud.md), "The Zone score is centred").

## `Hud_UpdateZoneWidgets` at `0x0008ce10` (confidence 80)

Called once, from `Hud_Update` (`0x0009fc48`). Beside lighting the `ZonePlus` rows and writing the class name, it
writes the score (`*(*(hud + 0x40) + 0x2c)`, `"%d"`) into the widget at `hud+0x270` (bound by `Hud_BindWidgets` as
`Score`). Before the text is set:

1. **A template, not the score, is measured**: as many `3`s as the score has digits (`n = n * 10 + 3` per digit; a
   score of 0 measures `3`).
2. `FUN_00200b98` loads the `Score` widget's own font, alignment and **scale** (`+0xb8` x, `+0xbc` y) into the font
   globals at `0x00ae9080`; `Font_MeasureString` ([navigation-legend.md](navigation-legend.md), "What measures a word")
   multiplies every advance by the x scale (`+4` of that block), so the width is measured at `ScaleX` (1.3).
3. `Score`'s x (`+0xb0`) becomes **`ScoreFrame`'s x (vtable `+0x6c`) plus `-0.5` times that width** (the `-0.5` is the
   TOC float `0xbf000000`). The widget at `hud+0x274` is `HUD->ScoreFrame`: `Hud_BindWidgets` builds that path
   (`0x0009dff4`-`0x0009d6d8`). `ScoreFrame` authors `x="0"` under its `OffsetX`, so the value hangs centred on x 960.
4. The write is skipped when `hud+0xac` or `hud+0xad` is set (which setting sets them is unread). The retro skins
   (`2097_hud`, `wo3_hud`) author `Score` right-aligned.

The widget stays left-aligned: the original rewrites the anchor and leaves the alignment alone.

## Why a template matters on this face

`PulseHud.fnt`'s digits are not one width: `1` has advance 22, every other digit 52. At scale 1.3, `3333` measures
259.5, so a four-digit value starts at **830.25** whatever it reads; centring `1076` on its own width (221.5) would
start it 19 units to the right, and Omega's `5911` (181.8) 39.

## The frames

The running original (`data/reference/hd-capture/flare-owner/zone/race-running.png`, HUD size 96 %) draws `1076` from
x 835 to 1046 at 1920x1080. Ours, same HUD size, before: 962 to 1169 (left-anchored at the centre); with the rule: 837
to 1045. Omega's PS4 frame (pair `zone-vineta-k-z5`) draws `5911` from the same left edge.

## What is wired

`oag_title::ZoneScore::CentredOnThrees` on HD's and Omega's `HudArt`, applied by `oag_hud`'s `zone_score` module with
the `HUD` face's `3` (`oag_hud::TemplateDigit`) and `oag_ui::font::cells_ink_width` (the `Font_MeasureString` law,
pens rounded up). Tests: `crates/hud/src/zone_score/tests.rs` and, on the disc,
`crates/raceplay/tests/hd_hud_ground_truth.rs::a_zone_race_draws_the_ladder_and_numbers_it_from_the_current_zone`.

**Split screen.** `splitscreenzone_hud/zone_hud.xml` authors the same left-aligned `Score` (`OffsetX="1550"`). Ours does
not load it: a split Zone race draws the whole-screen `zone_hud.xml` into each view
(`oag_raceplay::hud`'s `split_assets`), where the move applies as in a single view. Whoever wires the split Zone layout
must leave its `Score` authored if the two flag bytes turn out to be the split-view flags (unread).

## Other titles

**Omega: ported** (confidence 80). The same law sits in the HUD update holding `0x01265b2d`
(`/omega/eboot-ps4-omega-eu.bin`): the `n * 10 + 3` template (`0x01265b10`-`0x01265b31`), `"%d"` of it measured after
the widget's font setup, and `Score`'s x (`+0x114`) written as the measure times `-0.5` (`0x01800bf4`) plus the
frame widget's x (`0x01265c9a`-`0x01265cb2`), behind the same two flag bytes. Wired on Omega's HD-mode art. Its
2048-mode layout (`2048_hud`) is **not checked** and stays as authored.

**2048: the same template**, around `ScoreTxt`'s x instead of a frame's
([zone-speed-class-text.md](../vita-2048-eu-v104/zone-speed-class-text.md), "The score"). Ours still centres on the
value's own width there (`ZoneScore::CentredOnValue`, chosen); whether 2048's digits share a width is unchecked.

**Pulse, Pure: not read**, drawn as authored.
