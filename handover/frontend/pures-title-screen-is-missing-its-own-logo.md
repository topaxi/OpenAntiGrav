# Pure's `Title Screen`/`FE Screen` runtime mechanism is read; the reveal timeline is not

Comparing `oag-game`'s own render against the real PPSSPP capture: the big "WipEout"/"pure" logo (`TitleFrame`, `x=0 y=76 width=480 height=128`, `StartEnabled="false"`) never gets a `src` in `Skin.xml` at all - it is set programmatically on the original, the same way Pulse's own dangling globals are, and finding it needs either Ghidra (where `TitleFrame`'s texture gets assigned) or a full image-content scan of `Data.wad`/`FE.wad`, not a name guess - one guess (`Data\FE\Images\Logo.mip`, hash `be900df4`) did resolve to a real entry but at 2064 bytes is far too small to be a 480x128 texture, so it was not pursued.

`Image` now captures `U`/`V`/`TxtrWidth`/`TxtrHeight` (`crates/game/src/screen.rs`) and `draw_backdrops` samples the right sub-rect of a shared texture instead of always the whole thing (`crates/game/src/frontend/draw.rs`). `collect_widgets` now recurses into `<Animation>` the way it already did `<Viewport>`, so the widgets it wraps reach `screen.images`/`screen.texts` instead of being silently dropped. And `Screen::fills` is now `Vec<Fill>` - a colour-only `Image` carries its own `x`/`y`/`width`/`height` rather than being coerced into a full-screen backdrop, and its `color` is resolved through `FEGlobals->` the same way `image_from_node` and `screenclear` already do. Together those turned `Title Screen`'s drawn image count on `pure-psp-usa.chd` from 1 to 5 (the "Press start button", "Japanese chars" and "Double arrows" patches of `FETextures_startscreen.mip`, each sampling its own `U`/`V` sub-rect).

**`Title Screen`'s frame lines draw, and `FrameLineColor` turned out to be *authored*, not undeclared - corrected 2026-09-10.** This bullet used to read "genuinely undeclared in Pure's own `Skin.xml` (seventeen usages, zero declarations - confirmed by dumping the whole expanded file)", and a whole PPSSPP session went into sampling it off a captured frame at exact `2x` native resolution: `RGB(179, 215, 226)`, added to `FALLBACK_GLOBALS` as `0xFFB3D7E2`, confidence 60. **The file dumped was the front-end root, and Pure's front end is skinnable.** `Data\Plugins\PI001\Definition.xml` activates a second `PI_Skin` at `Data\Skins\Default`, whose own `Skin.xml` declares 41 globals including `FrameLineColor` (`0xFE99C9D8`) and the three this thread had already sampled - `TitleColor` `0xFFED4796`, `DesignColor` `0xFF5FDBF6`, `TextColor` `0xFF11ACD0`, identical on both pressings. Three of the four sampled values were wrong, `TextColor` badly so (`0x88D6E8` sampled against `0x11ACD0` authored). `oag_game::boot` now reads every activated style skin and `FALLBACK_GLOBALS` is empty; `title_screens_frame_lines_keep_their_own_rects_and_share_one_colour` in `pure_boot_ground_truth.rs` still pins the rect list against both discs, now against the disc's own colour. **The lesson generalises: zero declarations in a title's front-end root does not mean undeclared on the disc when that title activates a style skin.** See `docs/formats/race-setup.md`.

Also visible in that same capture, and worth carrying forward rather than acted on here: the "wipEout"/"pure" wordmark - `TitleFrame`'s own missing texture - **is genuinely drawn on the real disc**, orange "WipEout" over a white-outlined "pure", filling roughly the screen's own width. That is a real reference frame for whoever chases `TitleFrame`'s texture next (Ghidra, or the image-content scan this thread already named), not a resolution of it - no texture was identified this session, so the open item below stands as it did.

Separately: `ArrowSelect` (`U="53" V="0"` on `FETextures.mip`, present on both Pulse's and Pure's own `Skin.xml`) is not read by name anywhere in `crates/game/src/menu*.rs` or `crates/game/src/frontend*.rs` - `crates/game/src/frontend/draw.rs` even documents deliberately *not* drawing it where it is authored, on `Intro Screen` behind the language picker. So there is currently nothing in this build that draws `ArrowSelect` at all, U/V or not - the "is it visibly wrong" question this thread asked has no picture to check yet, on either title.

`LoadXML` includes (`Demo_Definition.xml` among them, whose own two `Animation`-wrapped colour-only `Image`s were the evidence that `Fill` needed a rect before it got one) are collected into `Screens::load_xml` and counted in the boot report, but nothing in `crates/game` actually loads or merges them. That was its own thread until 2026-08-30, closed once follow-includes was decided against for HD; see [`hd-frontend.md`](../../docs/formats/hd-frontend.md#where-hd-and-pulse-differ) for what it found (`Src`/`SrcRel` dual-authoring on HD's `Skin.xml`) before it closed.

**2026-08-25: a second `src`-less "set programmatically" widget found, on `Main Menu` rather than `Title Screen` - `FE Screen`'s own `BackgroundController->BackgroundImage`.** A player-reported bug ("Pure's menu has no background"), not this thread's own next step, is what found it: `Screens::collect_widgets` did not recurse into `<BackgroundController>` at all, so the whole of `FE Screen`'s frame - rule lines, scroll arrows, `ArrowSelect`, the squiggle-text strip, and `BackgroundImage` itself - was silently dropped along with `Main Menu`'s backdrop. Fixed (`crates/game/src/screen.rs`), and `oag_pure::FRONT_END::menu_frame` now names `FE Screen`, closing this thread's own `ArrowSelect` open item along the way: it is drawn now, at `U="53" V="0"` on `FETextures.mip`, tinted `MenuHighLightArrowColor`. `BackgroundImage` itself stays the same class of gap `TitleFrame` is - no `src`, so nothing here invents a texture - but its *colour* is now measured off a real `Main Menu` capture (solid white) and recorded as `oag_title::MenuSkin::background`. See `docs/ui/menus-original.md`'s own section on it.

**2026-09-07: `TitleFrame`'s texture found, by the content scan this thread named rather than Ghidra.** Every entry across `FE.wad` (157), `FEData.wad` (240) and `Data.wad` (832) was extracted and tried against `oag_texture::texture::Texture::parse` - 361 decode as `.mip` total (27/143/191) - and filtered to width >= 200px: mostly `256x128`/`256x256` track-select art, one `480x272` full-screen track render ruled out by eye, and exactly three `512x128` textures, all `Data.wad`-only (absent from both other archives, checked by hash). Two of the three (entries 535/536, byte-identical) are a blue-over-orange colourway; the third, entry 537 (hash `3af18d90`), decodes to **orange "wipEout" over a white-outlined "pure"** - an exact match to the reference frame above. `TitleFrame`'s own `TxtrWidth="480"` against the texture's `512`-wide physical size is corroborating rather than incidental: the rightmost 32 columns are opaque white padding with no ink, and the widget's authored sample width crops exactly that. Byte-identical on both pressings. Confidence 85 - short of 90 because the runtime mechanism that assigns this hash to this widget specifically is still unread. Full method, table and every candidate ruled out: `docs/formats/pure-status.md`'s own "The Title screen wordmark, `TitleFrame`" section.

Wired through a new `fallback_images` axis - `oag_title::BootProfile::fallback_images`, `oag_pure::frontend::FALLBACK_IMAGES`, consulted in `Screens::from_xml_with_fallbacks` (`crates/game/src/screen.rs`) the same way `fallback_globals` fills an undeclared colour, but keyed by the widget's own `name` attribute rather than a `Variable`. The `src` value takes `BootStep::movie`'s own `hash:`-prefixed spelling (`hash:3af18d90`) rather than a guessed path - `crates/game/src/boot/images.rs` reuses `boot::movies::EntryRef`'s parser and routes a hash spec straight to `Archives::read_hash`, ahead of `oag_pulse::read_image`'s PS2 name-rewrite, which a hash spec would defeat rather than answer. Ground-truth-pinned on both discs: `title_screens_own_wordmark_gets_the_measured_texture` in `pure_boot_ground_truth.rs`. Screenshotted with `--screen "Title Screen" --presented` and checked by eye against the reference frame - the logo now draws, orange wordmark over the white-outlined "pure", at its own rect above "PRESS START".

**2026-09-08: the scan run again, sized for `BackgroundImage`'s full-screen shape rather than `TitleFrame`'s `512x128` - one hit, one clean negative.** `FE Screen->BackgroundController` wraps two `src`-less images, not one: `BackgroundImage` (full-screen, `width="512" height="272" TxtrWidth="480" TxtrHeight="272"`) and `BackgroundTopRightImage` (`x="252" y="3" width="256" height="32"`, the corner graphic). Filtering the same 361 decoded textures to `480x272`/`512x256`/`512x512` (full-screen candidates, PSP-padded) and to `256x32` (the corner graphic's own size) turned up five candidates total. A fresh `Main Menu` capture was needed to check them against - PPSSPP v1.20.4 under Xvfb (`:97`), `--xres 960 --yres 544` for exact `2x`, driven the whole way from a cold boot through PPSSPP's websocket debugger (`input.buttons.press`, not a keyboard binding, so no PPSSPP keymap needed guessing): `Language Selection` -> `Developer Publisher Screen` -> `MemoryStickWarning` -> `FMV Intro` (skipped with start) -> `Title Screen` (start) -> `Profile` -> `New` -> `Set Name` -> `Set Tag` -> `Save Profile` (yes, through the PSP firmware's own save dialog) -> `Main Menu`.

`BackgroundTopRightImage` matched: entry 27 of `Data.wad` (hash `7ba78aca`, `256x32`, `Data.wad`-only, byte-identical on both pressings) decodes to the "ワイプアウト" katakana wordmark beside the swoosh/arrow logo - cropping the real capture to the widget's own rect reproduces it exactly. Wired through `FALLBACK_IMAGES` the same way `TitleFrame` is, confidence 85, pinned by a new test (`fe_screens_own_corner_logo_gets_the_measured_texture`, same file). `BackgroundImage` did not match anything: the three full-screen candidates (a track-background render already ruled out for `TitleFrame`'s own scan, a blue speed-line loading-screen-shaped texture, and a `512x512` card bearing "A-G RACING"/a barcode/a world map) all show content nowhere on the real `Main Menu`, whose background is flat white to the pixel (`255,255,255`, sampled directly) - which is exactly what `MenuSkin::background` already supplies. Nothing added to `FALLBACK_IMAGES` for it; a genuine negative, not an unlooked-for gap any more. Full candidate table: `docs/formats/pure-status.md`'s "`FE Screen`'s corner logo, `BackgroundTopRightImage`" section.

**2026-09-23: the runtime mechanism for all three is read, one real bug found and fixed, one hard-coded hash replaced with the real mechanism, and the reveal timeline's own struct mapped (its consumer still isn't).**
Full evidence: `docs/ghidra/functions/psp-pure-eu/title-screen.md`; the
corrections to each finding above: `docs/formats/pure-status.md`'s own
"2026-09-23" paragraphs under "The Title screen wordmark", "`FE Screen`'s
corner logo" and the new "`<Animation><Key TextureWidth="...">` reveal"
section.

- **`TitleFrame` was a real, live bug, not just an unread mechanism**: the EU
  pressing's own executable loads a different, region-suffixed texture name
  than the USA one (`FMV_last_frame_EU.mip` against `_US.mip`), and this
  build's `FALLBACK_IMAGES` hard-coded the USA hash regardless of which disc
  was open - so the EU disc was drawing the USA pressing's own wordmark
  colourway. Confirmed both ways: hashing each literal name lands exactly on
  two hashes this project already had from its own content scan, and a live
  PPSSPP breakpoint plus a settled-screen capture on `pure-psp-eu.chd` shows
  the EU-only blue-over-orange colourway. Fixed: `oag_assets::Layout` gained
  a `serial` field (the second exception its own doc predicted, after
  `ColourScale`), `oag_pure::frontend::title_frame_src` picks the right hash
  from it, and `oag_game::boot::load_shell` is the one caller that appends it
  - `BootProfile.fallback_images` itself is untouched, and unchanged for
  every other title.
- **`BackgroundTopRightImage`/`BackgroundImage` both resolve through a
  declared `FEGlobals->` global**, not a hash at all -
  `BackgroundController_UpdateImages` reads `BackgroundTopRightTexture`
  (`Data\Skins\Default\Images\default_texture.mip`, hashing to the value this
  project already had) and `BackgroundTexture` (the **empty string**, which
  the code treats as an explicit "no texture" branch) every frame.
  `FALLBACK_IMAGES` now resolves both globals directly rather than
  hard-coding the one hash that happened to be right.
- **The reveal timeline's own struct is mapped, its consumer is not.**
  `<Animation><Key>` is a shared 0x20-byte struct (`Time`/`X`/`Y`/
  `TextureWidth`/`TextureHeight`/`ScaleX`/`Scaley`) - the same one `hud.rs`'s
  `X`/`Y`-as-travel reading already uses, for a different pair of fields.
  Every `<Key>` on `Title Screen` brackets `TextureWidth` between
  `-<width>` and `0`, confirmed across all thirteen `<Animation>`s - a
  genuine reveal-authoring convention, not a guess. **Not implemented**:
  which function reads a `Key`'s interpolated `TextureWidth` and what it does
  to the render is unread - the `+0x3c` table this pass found is a
  parse-time "which function reads this XML tag" table, not the object's own
  runtime behaviour vtable (offset `+0x00`, not located). `Screens::collect_widgets`
  still discards the timeline and draws each widget at its final state,
  which is the honest choice while the render mapping stays unread rather
  than a plausible-looking guess.

## Open

- Which function is `Animation`'s real leading vtable's `Update`/`Draw` slot,
  and what it does with an interpolated `TextureWidth` - crop vs scale,
  which edge anchors. See `docs/ghidra/functions/psp-pure-eu/title-screen.md`'s
  own "Next steps".
- `Data.wad` entry 536 (`Data\FE\Images\FMV_last_frame_JAP.mip` by hash) is
  unwired - no Japanese-region Pure disc is in this project's corpus.
- Whether some skin *other than* `Data\Skins\Default` (none seen activated on
  either disc this project holds) declares a non-empty `BackgroundTexture` is
  unread - only the one active skin was.

## Next Steps

- Find `Animation`'s real object vtable (offset `+0x00` in the C++ ABI
  sense): a breakpoint on whatever calls `Animation_ParseValuesOrKey`'s own
  caller (the generic "walk an element's children by tag" loop, not yet
  identified) would find where the parsed `Key`/`Animation` objects go next,
  which is more likely to reach `Update`/`Draw` than the `+0x3c` parse table
  this pass found instead.
- Once read: implement the reveal in `oag_ui::screen`, verified against a
  PPSSPP capture of `Title Screen` mid-reveal (not just its settled state) -
  breakpoint-driven per-frame reads, not timed screenshots racing a 0.7s
  window; a first attempt at timed screenshots this pass landed on frames
  already fully revealed, which is why this is called out rather than
  assumed easy.
