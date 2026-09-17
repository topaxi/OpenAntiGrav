# 2048's front end is read and not wired

2026-09-17, branch `lane/2048-frontend`. Full write-up in
[2048-frontend.md](../../docs/formats/2048-frontend.md); the reference frames
are under `data/reference/2048-frontend/` (gitignored, README there names
each one) and the capture recipe extends
[vita3k-capture.md](../../docs/reverse-engineering/vita3k-capture.md) with
this pass's own display numbers (weston `wayland-oag95`, Xwayland `:95`).

**The one to know**: 2048 ships one front end, in one archive
(`data/plugins/frontend/NEWGUI/`, base `data.psarc` only - the patch touches
a subset, the DLC touches none), and it is not Wipeout HD's `FEGlobals`/
`MenuSkin` idiom retargeted. It is a genuinely different presentation layer:
touch-icon grids (`GameModeChoice`, `Home`) over a persistent 3D campaign map
(`FE3DCanvas`), no `<FEGlobals>` block anywhere, no `<Menu>`/`<HorizMenu>`
widget anywhere. `oag_title::MenuSkin` was built for the vocabulary the other
three titles share and disagree on numbers within; 2048 does not author that
vocabulary at all. That is why `oag_2048::TITLE.front_end` is still `None`
after this pass, on purpose, and not from a lack of evidence.

The boot chain (`Boot Connect` -> `Boot Studio Logo` -> `Boot Intro Movie` ->
`Load Save Bootup` -> `TitleScreen` -> `GameModeChoice`) is declared in the
XML and was watched once on Vita3K, in that order, matching content exactly
(the Studio Liverpool card's own 4-second delay, the `PRESS ANY BUTTON TO
START` text, the exact EU legal footer). A `LOADING...` screen and an
attract-mode demo race - a real frame of `2048-hud.md`'s own
`DemoRaceManager_Construct` finding - sit between `TitleScreen` and
`GameModeChoice`, undeclared in any `NEWGUI` file.

This pass also found and fixed a wrong claim on two other pages:
`2048-hud.md` and `2048-status.md` both said the HUD's raw `IG_HUD_*`
fallback text was because 2048 names no language plugin or HUD font. It
names both - `english/Definition.xml` declares a `2048HUD` font role at
`Data\XML\2048_hud\font\2048_hud.fnt` - the actual cause is `front_end` being
`None`, which starves `crates/game/src/race/hud.rs`/`race/load.rs`'s
`language_plugins` lookup. Both pages now say so.

## Open

- **`oag_title::MenuSkin` has no shape for a touch-icon front end.** This is
  the real blocker, not an unread disc. Filling `menu_x`/`space`/etc. with
  placeholder numbers to satisfy the type would be inventing a menu shape
  2048 does not draw - exactly what `CLAUDE.md`'s "never invent" section
  forbids. Two shapes for a fix, neither taken here because a type decision
  this size is bigger than a first sweep should make unasked:
  - Make `FrontEnd::menu` `Option<&'static MenuSkin>` and add a second,
    optional axis (`touch_menu` or similar) for the icon-grid/3D-canvas
    idiom, filled from `GameModeChoice`/`Home`/`FE3DCanvas`'s own numbers
    (all read and quoted in `2048-frontend.md`).
  - Or decide the touch idiom is out of `MenuSkin`'s scope entirely and give
    `FrontEnd` a `menu: Option<...>` with 2048 the first `None`, since
    nothing downstream currently draws a touch grid anyway.
  Whichever is chosen, `2048-frontend.md`'s "front end is a touch-icon grid"
  section has every number (`GameModeChoice`'s four `TouchButton`s at
  `x`/`y`/`140x140`, `Home`'s five, the `FE3DCanvas`'s ~50 `CanvasLabel`
  hotspots, `Team_Definition.xml`'s 3D ship-model origin) already quoted and
  ready to fill whichever type lands.
- **The Game Mode grid has six tiles at runtime; the XML declares four.**
  `HD CAMPAIGN`/`FURY CAMPAIGN` (unlocked by `dlc1.psarc`/`dlc2.psarc` being
  mounted, plausibly) correspond to no `TouchButton` in `NEWGUI/Definition.xml`
  at all. Not traced into the executable - `2048-frontend.md`'s own
  "what the executable sweep did not reach" names why (see below) and this
  is the first thing a decompile of `GameModeChoice_Screen`'s construction
  should answer.
- **The executable sweep is string-only, not decompiled.** The live Ghidra
  session has `/ps3-hdfury-eu/EBOOT.elf` open under another lane's active
  work; `switch_program` was correctly avoided, but the bridge also refuses a
  `program=` that is not open (`get_function_by_address` against
  `/vita-2048-eu-v104/eboot.elf` returns `Program not found`), so no new
  address was recovered this pass, only `strings` corroboration of names
  `game-boot.md` already carries. **`analyzeHeadless` against a second,
  independent Ghidra project is the concrete next step** - it does not touch
  the live GUI session at all. `docs/reverse-engineering/toolchain.md#vita`
  has the existing import recipe; the six-tile grid and the boot-mode
  selector (`Boot Connect`/`Launch 2048`/`RaceBox`/`Main Menu`/`MPStress`,
  all confirmed present as literal strings, clustered beside a
  `-mpscreen` command-line flag) are the two things worth pointing it at
  first.
- **`frontend.bnk` is located, its cues are not.** `data/audio/sound/frontend.bnk`
  exists exactly where `SoundManager_Construct`'s decompiled bank list
  (`game-boot.md`) says it should. `oag_title::Music::front_end` needs a
  *cue* name inside the bank, not the bank's filename, and `oag-wad sounds`
  reads a WAD-hosted bank where this one is PSARC-hosted - extracting it
  first (or teaching the CLI a PSARC-backed path) is the gap.
- **The loading screen is real and unauthored.** A percentage-driven
  `LOADING...` bar over the `WIPEOUT 2048` mark runs on every transition seen
  this pass. No `NEWGUI` file declares it - it reads as engine chrome rather
  than FE data, but that was not confirmed against the executable. If it
  *is* engine-native, `oag_title::Loading` (built for a disc-authored screen)
  is the wrong axis for it entirely, on the same "no shared vocabulary"
  grounds as `MenuSkin` above.
- **`CheckPoint_HUD.xml`-style dangling references were not swept for in the
  front end.** `2048-hud.md` already flags one HUD XML the executable wants
  and the base package does not ship; this pass did not repeat that search
  for `NEWGUI` screen names against the patch/DLC manifests beyond the
  front-end-shaped paths already listed.

## Next Steps

1. Resolve the `MenuSkin` type question above (ask, don't guess) and, once
   resolved, fill `FrontEnd::menu` (or its replacement) from
   `2048-frontend.md`'s already-quoted numbers.
2. Run `analyzeHeadless` on `/vita-2048-eu-v104/eboot.elf` (or the USA
   pressing, `1,197` functions currently, far less analyzed) in a project of
   its own, and decompile `GameModeChoice_Screen`'s construction to settle
   the six-vs-four tile question and recover real addresses for
   `names.tsv`.
3. Extract `frontend.bnk` and read its cue table (`oag_formats::sblk::Bank::parse`)
   to name `Music::front_end`.
4. Once `front_end` is fillable, re-run
   `crates/game/tests/vita_2048_hud_ground_truth.rs` and a fresh
   `just play 2048 --race` boot report - the HUD font fix
   (`2048-hud.md`'s correction above) is a real, free improvement that falls
   out of wiring `front_end` at all, independent of the `MenuSkin` question.
