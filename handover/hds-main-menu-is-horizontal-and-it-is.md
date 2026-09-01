# HD's main menu is horizontal, and it is drawn that way

2026-08-19. `MainMenu_Definition.xml` carries one `<HorizMenu name="Mode">` at `align="left" x="160" y="125" color="0xff705070"` and no `<Menu>` at all, so this build's column of rows was a layout HD's disc authors nowhere. **Confidence 92**, on five archives (`DATA00`, `DATA02`, `DATA03`, `DATA05`, `DATA06`; the sound-only `DATA01` and `DATA04` carry no copy) writing those three numbers identically, and `crates/game/tests/hd_menu_ground_truth.rs` (4 tests, disc-backed) asserts every copy rather than one. **The ADR-0022 gate was run before the field was added, not after**, and it is the part worth carrying: `None` on both PSP titles is a *measurement*, because every blob of `Data.wad`/`FE.wad`/`FEData.wad` on `pulse-psp-usa.chd` (1,411 files), `Data.wad` on `pulse-psp-eu.chd` (1,138), `WADSP.WAD` on `pulse-ps2-eu.chd` (193) and all three on `pure-psp-eu.chd` (1,241) was extracted and searched, and `HorizMenu` appears in **none** of them - against 8 of `DATA06`'s 29 front-end files. A string search finds a shortened element because the PSP dialect writes full names into each file's own `<code>` dictionary; the control column (files containing `Menu` at all: 30/27/43/46) is what says the search can see the vocabulary. **Not swept**: the PS2 pressing's `WADS2.WAD`, `PRERACE.WAD`, `PS2MUSIC.WAD`. **What is ours and says so**: which pages get a strip (`strip::suits` - every entry is navigation, because a strip has nowhere to anchor a value column), the gap between entries (`STRIP_GAP`, the widget states none), the selected colour (`selected` is a measured field and no HD capture exists), and up/down still stepping a strip beside left/right. **On the shipped `menu.toml` the rule fires on the root page alone** - `options` is the counterpart to HD's `Additional`, which HD *does* draw as a strip, but a `choice` hangs off ours - and that is stated rather than implied, because the rule is shaped by HD's navigation-versus-`Settings` split and not declared by it. **No carousel**: the widget states one anchor and lists its entries, so all of them are drawn from it; whether the original centres the selection needs a capture. **What this did not need, and the trap it avoids**: nothing here follows an include - the numbers are a title-package constant per ADR-0022 and the test reads the file directly, the same shape `menu_layout_ground_truth.rs` has for the PSP titles. **One thing cost a run**: the disc-backed sweep walked `Archives::data` and `Archives::extra` and found four copies, because `DATA02` is HD's `fe`; a sweep that misses an archive looks exactly like a disc with fewer copies. `crates/game/src/menu/strip.rs`, [hd-frontend.md](../docs/formats/hd-frontend.md#the-main-menu-is-horizontal-and-it-is-drawn-that-way-now), [menus.md](../docs/architecture/menus.md).

## Open

- The `selected` colour is a measured field with no HD capture to verify it
- Whether the original centres the selection (no carousel) is unverified

## Next Steps

- Get a capture to check whether the original centres the selection and to verify the `selected` colour

## Resolved

- 2026-09-01: swept the PS2 pressing's remaining three archives.
  `WADS2.WAD` (7,200 files, extracted via `oag-wad`): 33 hold `Menu`, **0** hold
  `HorizMenu`. `PRERACE.WAD` and `PS2MUSIC.WAD` are not `fexml` archives at all -
  both are `oag_formats::ps2_music`-shaped raw-PCM containers (`{hash, size,
  offset}` directory entries, confirmed against `PRERACE.WAD`'s own header: 32
  entries, first payload offset lands exactly at `4 + 32*12 = 388`) - so neither
  can hold an XML widget by format; a raw byte scan still confirms 0 hits for
  `Menu` or `HorizMenu` in either. `None` for Pulse's main menu now covers every
  archive on every Pulse pressing. The user separately confirmed from playing
  the games: Pulse has no horizontal main menu, only HD and Omega do - consistent
  with this measurement. Doc tables updated:
  `crates/title/src/menu.rs` and `docs/formats/hd-frontend.md`.
