# Wipeout: Omega Collection status

What `oag-omega` and the rest of this project's tooling read off the PS4
package pair unchanged, what changed, and what stays unread. Measured against
`data/extracted/ps4/omega-eu` (base) and `omega-eu-patch` (mandatory day-one
patch) - see [`source-images.md`](../reverse-engineering/source-images.md#omega-ps4-eupkg--omega-ps4-eu-patchpkg---wipeout-omega-collection-ps4).

**The short version.** Omega's front end is [HD's own `PI001` plugin carried
forward](omega-frontend.md), not a rewrite, and everything that follows from
that reads with **no parser change at all**: `skin.xml`'s `FEGlobals`, the
boot chain, `mainmenu_definition.xml`'s `<HorizMenu>`, `CellMode_Definition.xml`'s
`Grid Selection`/`Cell Selection` schema. What is genuinely new is the
*plumbing* - `oag-omega` as a title package, a nine-archive candidate list
where the patch's own front-end-complete archive has to win a name collision
with the base package's older, incomplete copies - and one thing that is
**not** carried forward unchanged: the circuit files ship every one of HD's
files under new spellings (`track.final.rcsmodel`, a 19-byte collision node, a
64-bit `.rcsmodel`) that this project's 2048 readers turn out to read after
three small changes - see "Racing" below.
Front-end images are Sony's PS4 `.gnf`
container, which [`gnf.md`](gnf.md) now decodes for most of the sprite sheet
(219 of 289 front-end/campaign `.gnf` files draw). **A race starts** - see
"Racing" below - and [`omega-frontend.md`](omega-frontend.md) is the frontend
census this page builds on.

## What reads unchanged

| Layer | Reads? | Notes |
| --- | --- | --- |
| [PSARC](psarc.md) container | **yes** | Nine archives across the base package and the patch; version and flags not diffed against HD's own PS3 copies this lane. |
| Front-end XML (`Screens::from_xml`) | **yes** | `skin.xml`, `mainmenu_definition.xml`, `cellmode_definition.xml`, `additional_definition.xml` all parse with the same `oag_ui::screen::Screens` reader HD and Pulse/Pure use - confirmed both by `crates/game/examples/omega_frontend_probe.rs` (docs-only sweep) and by this lane's own boot (`Data\Plugins\Frontend\Gui\Skin.xml: 27 screens, 74 globals, 24 LoadXML includes`). |
| `FEGlobals` menu layout | **yes, bit-identical to HD's** | Ten of ten authored globals match HD's own `skin.xml` to the digit, re-derived directly against `data09.psarc` in `crates/omega/src/frontend.rs` rather than copied - see that module and its ground-truth test. |
| Boot chain shape | **declared, not measured** | Same eight redirects in the same order as HD's `DATA00`/`DATA05`/`DATA06` family, no dead `LogoFMV`. `Provenance::Declared`: no PS4 emulator exists in this project's toolchain to upgrade it the way HD's RPCS3 capture did. |
| Campaign schema (`CellMode_Definition.xml`) | **yes, HD's own schema** | `Grid Selection`/`Cell Selection` as `FlyerSelection`/`CellSelection` screens, plain UTF-8 (not Pulse's dictionary-shortened copy). Nineteen grids where HD has sixteen (`oag_omega::campaign::GRID_COUNT`), confirmed by direct listing. Sixteen of nineteen parse (2026-09-30; twelve when this row was written) - grids 16-18 carry no `RequiredPoints` attribute and are skipped with a logged warning; the rest fail per-row the same tolerant way a bad HD grid already does (`docs/formats/hd-frontend.md`'s note on `grid_04.xml`'s own broken tag). |
| Plugin definitions | **yes, but split three ways, like 2048's** | `Data\Plugins\teams\`, `tracks\` and `music\` each ship their own `Definition.xml`, not HD's single `Data\Plugins\Frontend\Definition.xml`. New finding, not in `omega-frontend.md`, which only opened the GUI files. |

## What changed, and what state each is in

**The archive layout is new.** Omega ships as a base `.pkg` (five archives,
`data00`-`data04`) plus a mandatory day-one patch (four more, `data05`,
`data07`, `data08`, `data09`) - and the patch **replaces** the front end
wholesale rather than supplementing it: `skin.xml`, `mainmenu_definition.xml`,
`racebox_definition.xml` and `cellmode_definition.xml` exist only in the
patch's `data09.psarc`. `oag_omega::TITLE` makes `data09` the bulk (`data`)
candidate specifically so it is searched before the other eight - chosen to
win the collision, not measured to be what a PS4 actually loads first. See
`crates/omega/src/lib.rs`'s own `DATA_CANDIDATES` doc and
[`omega-frontend.md`](omega-frontend.md#load-order-is-not-settled).

**The ship directory moved to 2048's scheme, not HD's.** `Data\art\published\hdships\<team>\`,
lowercase ids (`ag_systems` confirmed present), not HD's `Data\Ships\<Team>\`.
Confirmed by direct listing on `data00.psarc`; `crates/omega/src/race.rs`'s
own `SHIP_DIR` constant records it. Sound banks moved too:
`Data\audio\sound\<name>.bnk`, not HD's `Data\Sound\<name>.bnk` - `shipHD.bnk`,
`weapons.bnk` and `speech.bnk` are confirmed present under the new path.

**Front-end images are `.gnf`, a PS4 texture container `gnf.md` now decodes
for most of the sprite sheet.** The XML still names `.gtf` (HD's spelling,
never re-authored), and the shipped file is `.gnf` under the same stem -
`saveIcons.gnf`, `line.gnf`, `Title_Arrow_HD.gnf` all confirmed present this
way. `crates/game/src/boot/sprites.rs::gnf_sibling` checks the `.gnf`
sibling directly: when it decodes, its bytes are pushed into the sheet under
the widget's own `.gtf`-spelled key (`crates/hud/src/sprite.rs::Image::decode_gnf`
is what actually rasterises them); when it does not, the boot report names
the exact reason rather than a bare "not found" (`image
Data\FE\Images\saveIcons.gtf: found as Data\FE\Images\saveIcons.gnf (158976
bytes) - PS4 GNF container, 342 block(s) in the base level have no valid BC7
mode bit - refusing rather than decoding around missing bytes, see
docs/formats/gnf.md, drawing nothing`). On this lane's own two-archive
extraction, the front end's own **boot-time** image set is small (8 names)
and mostly either genuinely absent from these archives or corrupt -
`Title_Arrow_HD.gtf` (32x32) is the one that decodes and draws, visible as
the nav-legend arrow in `just play omega --menu-page main`'s own screenshot.
The front end's full sprite sheet fares far better: `gnf_frontend_census`
(`crates/texture/examples/gnf_frontend_census.rs`) measures 219 of 289
front-end/campaign `.gnf` files decoding clean, 53 refused for a corrupt
base level, 17 in a format this module does not decode at all (`Bc4`, font
surfaces) - see `gnf.md`'s own "Tiling" section for the evidence that closed
the tiling question and the `CorruptBlocks` refusal's own reasoning. What
still draws with no images at boot specifically is a data-availability fact
about this two-archive extraction (most boot-time names are absent or
corrupt here), not a decoder gap.

**Fonts are unread too, on the same terms.** `Data\FE\Fonts\helv.fnt`/
`helvb.fnt` (HD's own spellings) do not resolve on Omega under either
name checked this lane; whether Omega ships a font under a different name or
a different container was not searched.

**Circuits: every one ships a `track.vex` with a readable `WO Track`, and its
baked outputs under a `.final` infix.** Across the whole package pair, **39 of
39** `track.vex`/`track_reversed.vex` files carry a `WO Track` node that
`oag_vex::track::parse` reads unmodified with `encoded_len()` equal to the
payload's length to the byte (`tech_de_ra`: 2 paths, 2 junctions, 835 points;
`environments2048\altima`: 6 paths, 4 junctions, 2,029 points, 195,104 bytes,
the same numbers 2048's own file has, at the same 96-byte point
[`track.md`](track.md) measured there). Beside each sit `track.final.rcsmodel`,
`track.final.rcsskeleton`, `track.final.rcsanimclip`, `track.final.pvs`,
`track.final.audio`, `track.final.checkpoint` and `track_col.col`; the four
`zone_N` circuits use the plain `track.rcsmodel`-style spelling instead, and
**no directory ships both**. This page's earlier revision said the opposite
(`talons_junction` "ships `.final.*` in place of a `track.vex`", `tech_de_ra`'s
`track.vex` "confirmed to have no readable `WO Track` node - a clean negative
result") and **both claims were the extraction bug**, below: on the short-read
copy `tech_de_ra\track.vex` has 278 nodes and no `WO Track`, on the corrected
one 458 nodes and one (60,151 of its 133,888 bytes differ, 110,298 of them
zero against 50,172).

**The in-race HUD has no per-mode composition this lane found.** No
`arcade_hud.xml`/`timetrial_hud.xml`/`speedlap_hud.xml`/`zone_hud.xml` at
HD's root paths; instead `Data\xml\` carries loose fragments
(`hud_damage_indicator.xml`, `hud_pickups.xml`, `hud_proximity.xml`,
`hud_sights.xml`, `hud_ready_go.xml`, `hud_elim_lap_counters.xml`,
`hud_elim_positions.xml`, `hud_detonatorweapons.xml`) plus `2048_hud\`/
`2097_hud\`/`wo3_hud\`/`splitscreen_hud\`/`splitscreenzone_hud\`/`duel_hud\`
subtrees. Working out which fragments compose which mode is real
reverse-engineering, not attempted here - `crates/omega/src/hud.rs` fills
`Title::hud`/`hud_art` with a single real, unread placeholder entry, verified
provably inert for this lane (the only reader is
`crates/raceplay/src/hud.rs`, reached only once a race has already started
loading).

## The menu backdrop (2026-09-30, `omega-menu-backdrop`)

Omega's menus now sit on the HD-style `<BackgroundAnim>` scene, which is the only backdrop
Omega can have: no `.points2` clouds ship in any of the nine archives. The scene is
`FrontEndScene_HD_ATG.vex` with its PS4 `.rcsmodel` (byte-identical in `data00` and `data08`);
the `*_VR` scenes are not drawn. The `.vex` carries the motion and the camera (60 s loop, 2,274
translation keys on `camera1`), the `.rcsmodel` the geometry, and the two bind by shape name -
`oag_mesh::mesh::rcs::psp2::build_with_vex`. The picture is the scene drawn on white and filtered by
`FEBackgroundAnim_fp` into a grey line drawing; everything is read in
[`menu-backdrop-scene.md`](../ghidra/functions/ps3-hdfury-eu/menu-backdrop-scene.md).
`--menu-page main` (settled `Main Menu` row: edge 0.6, fill 0.2, width 0.5, no blur) and any other
page (the `default` row: 0.3, 0.1, 1.5, blur 3) draw it, and `--anim-seconds` picks the moment. **The
boot screens do not** - they are outside `Top FE Screen` in the skin, so the original has none there
either. **Not validated against the original**: no PS4 emulator exists and HD's one capture of its HD
style shows a flat white page. Chosen, not measured: the blur kernel, the linear sampling, and which
page takes which row. The `--menu-page grid-select`/`cell-select` stills draw it too (they take the
page's picture). **The live campaign stage draws it too, and did before this was written**
(`menu_stage.rs` hands the stage's `shown` picture, the movie or else the style's
`Live::picture`, to every campaign list builder, and `Live::tick` runs per frame): walked windowed under
Xvfb + lavapipe on `main` `4def1665` plus the texture-limit change, language -> `Main Menu` -> `RACE CAMPAIGN`
-> `Grid Selection` -> `Cell Selection` by mouse, 2026-10-02, and the scene draws behind all three
(`data/scratch/omega-talon-crash/walk_1.png` to `walk_3.png`). The end-of-race screens sit under the
same widget and do not yet. Omega authors no track or ship picker.

## What a menu-page capture does today

**Invocation (watched working, 2026-09-30).** The source is a positional path,
the extraction root holding both packages:

```sh
cargo run -p oag-game -- data/extracted/ps4 --no-audio --menu-page main --screenshot out.png
# or, after a build: ./target/debug/oag-game data/extracted/ps4 --no-audio --menu-page main --screenshot out.png
# the Language Selection boot screen:
./target/debug/oag-game data/extracted/ps4 --no-audio --until "Language Selection" --hold start --screenshot out.png
```

`--menu-page` takes `main`, `grid-select` or `cell-select`. `just play omega ...`
maps the keyword `omega` onto `data/extracted/ps4` and forwards the rest; it
builds `--release`, and was not re-run here. A bare `oag-game --menu-page main`
with no path, or with `omega` passed as if it were one, reads no source and
fails with the missing `<SOURCE>` usage error.

**Scale (2026-09-30, `omega-fe-retouch`).** Omega's source used to fall through
`Space::of` to the PSP's 480x272 grid ("front-end grid 480x272" in the boot
report), so HD-authored coordinates were drawn on a grid four times too small
and the disc's own fonts came out huge. Now `Space::OMEGA` (1920x1080, grid
confidence 85: the legal line at `x=960 y=972` and two `line.gtf` rules at
`x=160 width=1600`, asserted by `omega_font_scale_ground_truth`). Omega's faces
are also HD's at twice the pixel size (`helv` line height 64 against 33, `helvb`
89 against 44, `PS_BUTTONS` 109 against 54; median glyph width, height and
advance ratio 2.0 on every shared glyph), so its atlases are read at
`Space::font_texel_scale` 0.5 - **an inference, confidence 60**: the ratio is
measured, that the PS4 build draws at half size is not (no PS4 text path has
been read). The main menu now lays out as HD's does.

**Screens (2026-09-30, `omega-fe-screens`).** Four defects closed, three of them
shared with HD:

- *Language Selection rows overlapped* (HD too). The picker's `<Menu>` authors
  x, y, scale, colour and alignment and **no pitch**, so the step was always an
  inference, and the table it inferred from (`font_line_height`) is Pulse's: 13
  units against HD's 33 and Omega's 32 (`helv`, 64 at 0.5). `oag_title::BootProfile::picker_from_loaded_faces`
  (true for HD and Omega only) now steps by one line of the loaded `Default`
  face; the face height is the disc's, the rule "one line of the widget's font"
  is the inference. The selected row's band is drawn the full row and 300 wide,
  **chosen, not measured**. No reference exists: HD's picker was never caught on
  RPCS3 (`hd-frontend.md`, the redirect fires in milliseconds) and no PS4
  emulator exists here. Pulse, Pure and 2048 still use the table, byte-identical.
- *The confirm glyph was a box* (HD too). `ControlTextConfirmButton` authors
  `font="menu"`, HD's language plugins declare no `menu` role, and `FE_CONFIRM_BUTTON`
  is a single Buttons-face codepoint. The picker now draws it (and the back
  prompt) through the `Buttons` slot, the mechanism the campaign footer already
  uses. Which face the original picks is **chosen, not measured**.
- *`cell-select`/`grid-select` raw ids ("Event line", "rc laps line"), `%d`, the
  footer overprint and "more points needed" over "010"* were one cause: the
  campaign still and live stage dispatched `title == HD` to `oag_ui_screens::campaign::hd`
  and sent Omega to Pulse's draw list, which has no arm for HD's widget names.
  `oag_game::campaign::draws_hd_campaign` covers both titles. Omega's `Cell
  Selection` also authors a `RecordsButton` (`FE_RECORDS`) at exactly
  `DifficultyButton`'s `x=944 y=994`; it is suppressed (no leaderboard behind it,
  as HD's endrace drops `RecordsCycle`) - **chosen, not measured**.
- `--race` still shows no HUD at tick 120 on Omega; untouched.

Still visible on Omega: circuit names read as ids (`01_Track`: the German
`entries.xml` names only the four Zone circuits, `25_Track`..`28_Track`, the rest
live elsewhere or under other ids); the three medal icons under the target row
are in the wrong order (`Hexmedal_HD` is 976x549 on Omega against HD's 1024x768,
and the crop is HD's); no flyer and a white backdrop on `grid-select` (`flyers:
None`); `Event 01/08` is an English literal on HD too; the Endrace screens still
dispatch `title == HD` alone. The picker's `Svenska` x4 and `Portugus` are **the
disc's own**: `definition.xml` really says `ID="Japanese" String="Svenska"` (and
Korean, TraditionalChinese), and `ID="Portuguese" String="Portugus"`. `P??????`
(Russian) is Cyrillic missing from the `Default` face's atlas.

`just play omega` boots the front end, reads `data09.psarc`'s ten `FEGlobals`,
the boot chain and the string table, and stops on **Language Selection** - the
disc's own real first screen - because German (this build's compiled-in
default) is not offered: every `english`/`german`/... `Definition.xml` this
lane found is one of the all-zero copies `omega-frontend.md`'s own census
already flagged for `english`, so the picker shows exactly as it would for a
real player with an unsupported language.

`--menu-page main`/`grid-select`/`cell-select` draw this build's own
reimplemented menus - **not** the disc's own `Main Menu`/`Grid Selection`/
`Cell Selection` screens, which are never parsed for an HD-idiom front end at
all: `crates/game/src/boot/includes.rs`'s `LoadXML`-follow mechanism only runs
for a title whose `FrontEnd::touch` is `Some` (2048 alone), by design, since
following every include on HD would pull in art this build never samples.
`main` draws the RACE CAMPAIGN row on white, with `Title_Arrow_HD.gtf`'s
decoded `.gnf` sibling as the nav-legend arrow (no block art - see "What did
not draw" below); `grid-select` shows `EVENT 01/08` and lays out the twelve
grids that parsed, with `Subtitle_Arrow_HD.gtf`/`Padlock.gtf` also decoding
through their own `.gnf` siblings; `cell-select` draws the header with no
cell content (needs a grid selected first, not exercised this lane).
Screenshots: `data/scratch/drive-2026-09-27/omega-gnf/omega-{boot,main,grid,cell}.png`.

### What did not draw, by name

- **Most of the front end's own *boot-time* sprite set** (8 names reached
  before `Language Selection`) - `saveIcons.gnf` is present but its base
  level carries 342 corrupt blocks (`Error::CorruptBlocks`, refused by
  name); `line.gnf` is present but fails the `GNF ` magic check outright
  (an all-zero PSARC entry, the same population `psarc.md`'s "Block data
  location" section documents); `vr_headset.gtf` and the four
  `NewImages/Demo/*.gtf` names have no entry at all on this base+patch
  extraction, `.gnf` sibling included. Only `Title_Arrow_HD.gtf` (32x32)
  decodes and draws. The front end's *full* sprite sheet fares much better
  off this same boot's screens - see `gnf.md`'s own census (219 of 289
  decode) - this boot-time set is just small and unlucky.
- **`Data/FE/Images/StudioLiverpool.bik`**, `Studio Logo`'s own reel - not
  found under that name in any of the nine archives, unlike every other
  front-end asset this lane looked for. Whether it is elsewhere under a
  different name, or genuinely absent from this build of the front end, was
  not chased.
- ~~**The menu block art** (`MenuSkin::blocks`) - deliberately left `None`~~ **drawn, 2026-10-06
  (`omega-frontend`).** The 2026-09-30 reading "the fill swatch reads alpha 0.004 on Omega's copy,
  cause unread" had a cause: **Omega's `file2.gnf`, `cursor.gnf` (and `file`, `corner`, `corner2`,
  `square`, `line`, `unlocked_corner`) decode to HD's `.gtf` pictures in HD's own file row order,
  bottom-up**, where every image a screen names was re-authored top-down. A `.gnf` carries no
  flag, and the sheet leaves a `.gnf` unreversed (right for the 276 re-authored ones), so the
  nine-patch came out mirrored and the swatch texel `(4.5, 58.5)` of the file was read at the
  wrong row. Census (`data/scratch/omega-frontend/orient-census.txt`, Omega `.gnf` against HD
  `.gtf` of the same stem and size, deduplicated across the base and patch archives): the
  re-authored ones match HD's rows reversed (135), the eight above match HD's rows as stored,
  22 are symmetric (no verdict), 41 differ in size, and 971 have no HD file of the stem to
  compare (so they are unverified, drawn unreversed as before). The census compares each Omega `.gnf`'s decoded rows with HD's `.gtf` of the same stem,
  as stored and reversed (a throwaway probe, not committed). Fix: `oag_title::FrontEnd::bottom_up_gnf`
  (the eight stems, empty on every other title) makes `boot::sprites::load` reverse them as it
  does a `.gtf`; that includes `line` (the two rules `skin.xml` stretches across the frame) and
  `unlocked_corner` (the selection and manual screens), which now match HD's sheet; `oag_omega::frontend::MENU_BLOCKS` is
  HD's table, **chosen, not measured** for Omega - every number is read from HD's `EBOOT.elf`
  and nobody has read Omega's `eboot.bin`; what supports it is that the nine-patch decodes to
  HD's picture to within BC7 error (`omega_menu_blocks_ground_truth.rs`, texel for texel
  within 8/255 of HD's sheet on this engine, and a focus move eases the strip) and that the boot report reads
  `menu blocks: HD style, fill swatch alpha 0.427 then 1.000, underline decoded, arrows decoded`.
  **Reference: HD's front end on this engine, and Omega's own shipped data. No PS4 emulator
  exists, so nothing here is compared with the original console.** The strip is now bordered
  blocks with a chamfered landing, the selected one eased wider (`Menu::focus_of`, one sixth a
  tick) with the underline mark sliding and blinking; before, the blocks-less strip took the
  measured-tab path whose tabs are fixed to their text - the "do not animate" in the report.
  Settings rows take the List blocks (520/280 to 340) and step arrows the same way. Not yet
  verified live: the pointer hit regions on the new block geometry are HD's own code path
  (`regions`), covered by `campaign_pointer_ground_truth` only for the campaign.
  **Removed with it:** the 2026-09-30 white-row-on-white-page boxes
  (`rows::text_is_lost_on_page`, `row_box`); Omega no longer reaches the bare-text row path,
  and no other title had a light page under light rows, so it was dead.
  **2048 cross-check:** `checked, applies, not wired` - 2048 declares no `MenuBlocks` and
  draws a touch front end, so there is no block art to reverse; whether its package ships
  `file2.gxt` in HD's row order was not opened.

- **Most of the campaign hex textures** (`Hexagon_HD_OUTLINE.gtf`,
  `Hexagon_HD.gtf`, `Hexlock_HD.gtf`, `Hexagon_HD_THICK_OUT.gtf`,
  `NonSelectable_Arrow_HD.gtf`) - `crates/game/src/campaign.rs::load_omega`
  now falls back to `crate::boot::sprites::gnf_sibling` the same way the
  front end's own sheet does, but none of these five has a `.gnf` sibling
  either on this base+patch extraction - genuinely absent content, not an
  unread name. `Hexmedal_HD.gnf` **is** present (1,402,112 bytes) but fails
  the `GNF ` magic check, the same all-zero population as `line.gnf` above.
  `Subtitle_Arrow_HD.gtf` and `Padlock.gtf` (`OTHER_TEXTURES`) both decode
  through their `.gnf` siblings and draw - see the `grid-select`/
  `cell-select` screenshots.

### Why so much of this is missing: an extraction-tool bug, not this build's data

`lane/omega-psarc`, 2026-09-27. Every "not found"/"all-zero"/"corrupt
blocks" name above (`saveIcons.gnf`, `line.gnf`, `Hexmedal_HD.gnf`, the five
campaign hex textures, most of the boot-time sprite set) was checked against
one open question this project's own extraction of
`omega-ps4-eu{,-patch}.pkg` left unresolved: is the content genuinely absent
from the package, or did this project's own copy of it come out wrong? It is
the latter, confidence 90 - see
[`gnf.md`'s "Root cause"](gnf.md#root-cause-confidence-90-a-short-streamread-in-the-extraction-tool-not-this-projects-reader)
for the full mechanism. In short: `PkgTool.Core`'s (`LibOrbisPkg`'s) PFSC
sector decompressor calls `DeflateStream.Read` once per 64 KiB sector and
never checks whether that call actually filled the buffer - most of the time
it does not, and the untouched remainder reads back as zero bytes, which is
what this page's "all-zero"/"corrupt" checks are seeing. **Not this
project's PSARC/GNF readers** - both are confirmed correct against the raw
archive bytes and against a from-source repro of the extraction tool. A
one-line loop fix in that tool's `PFSCReader.ReadSector`, verified by
rebuilding the tool and re-reading `Harimau_c1_Livery.gnf` directly out of
the real `.pkg`, turns a previously-refused (`Error::CorruptBlocks { count:
49899 }`) livery texture into a fully legible PNG through this project's own
unmodified decoder.

**Checked against a corrected re-extraction
(first written under `data/scratch/drive-2026-09-27/omega-psarc/extracted-fixed/`, now `data/extracted/ps4/{omega-eu,omega-eu-patch}`), 2026-09-27: every named
"missing"/"corrupt" texture above recovers, and none of it was genuinely
absent.** `saveIcons.gnf`, `line.gnf`, `Hexmedal_HD.gnf` and all five
campaign hex texture names (`Hexagon_HD_OUTLINE`, `Hexagon_HD`, `Hexlock_HD`,
`Hexagon_HD_THICK_OUT`, `NonSelectable_Arrow_HD`) all have real `.gnf`
entries in the corrected `data00.psarc` (duplicated in the patch's
`data08.psarc`) and decode clean through this project's own unmodified
`oag_texture::gnf::Texture::decode` - the old, corrupted manifest had not
just corrupted their pixel payloads but hidden most of them from the
directory listing entirely, which is why "no `.gnf` sibling either" read as
absence rather than corruption. `vr_headset.gnf` (447x370, `data08.psarc`)
recovers the same way, previously "no entry at all" for the same manifest
reason. `StudioLiverpool.bik` remains genuinely absent under that name even
in the corrected extraction, across all nine archives - not a reading
artefact. See `data/scratch/drive-2026-09-27/omega-psarc.md` for the exact
decode results and the full census.

## Campaign: confirming a cell starts its race, 2026-09-30

**Correction, 2026-09-30 (`omega-frontend-fixes`): the mouse walk below proved less than it
said, and a player could not start the second unlocked cell with a mouse.** It clicked
`grid0_3_1`, the first target in the list, which wins every overlap; `grid0_3_2` (also
`Locked="false"`, Speed Lap on `08_Track`) was never hovered. HD's and Omega's hex
sprites are 128x64 power-of-two textures holding a 72x62 hexagon in the top-left corner
(alpha box measured on `Hexagon_HD.mip`: x 1..72, y 1..62), and the pointer target was the
whole texture, so every hit region was 128 wide, 111 tall and 28 units right of the art.
Hovering `grid0_3_2` selected its padlocked neighbour `grid0_2_2` (the panel read
`Zeitrennen / 08_Track`) and a confirm did nothing; the log said only `grid0_2_2 is
locked`. The keyboard and pad path was never broken (`Return` and `Down`, `Return` both
reach a race, walked from the language picker). Fixed: the target is the box of the
sprite's opaque pixels (`Sheet::opaque_extent`, `oag_game::campaign::hit`), held by
`crates/game/tests/campaign_pointer_ground_truth.rs` on every Omega and HD grid (it fails
on the old targets). **HD had the same bug.** Two more things a mouse-only player met:
`Grid Selection` has no flyer card on Omega, so its click target was the invisible
`Flyer Pad Lock` rect - with no card drawn a click on the page's content band now confirms
the tier (chosen, not measured); and a refused cell (25 of 167: `NitroBattle`, `Detonator`)
only logged, it now shows `OAG_CAMPAIGN_MODE_UNSUPPORTED` above the footer. The walk below
otherwise stands.

`omega-campaign-launch`, walked windowed under Xvfb by mouse: `RACE CAMPAIGN`,
`Grid Selection` (a click on the flyer's authored `Flyer Pad Lock` rect),
`Cell Selection` (a hover, then a second click on the hex), then a running race.
Two cells, two modes, two circuits: grid 0's `Race` on `01_Track` (Vineta K,
Venom, 3 laps, 8 craft) and its `Speed Lap` on `08_Track` (7 laps, solo).
`Session::launch_campaign_cell` needed **no Omega arm**: the cell's circuit id
resolves through the teams/track plugin catalogue (`Data\Plugins\teams\Definition.xml`,
38 circuits, the archive-filtered list every title's shell builds), and the
mode, class and laps mapping is HD's. What was added is a test: the mapping is
now `oag_game::campaign::launch::plan_cell`, a pure function the session applies,
and `crates/game/tests/omega_campaign_launch_ground_truth.rs` holds it to the
disc: of the 167 cells in the 16 grids that parse, **142 launch onto a circuit
the catalogue resolves and 25 (`NitroBattle`, `Detonator`) are refused for
their mode** - never a circuit id that resolves to nothing. Grids 16-18 lack
`RequiredPoints` and are skipped by the grid reader (logged).

- **The cell's class reaches Omega's handling.** `--race --class venom` and
  `--class flash` on Omega log `GravityMul 0.85 for the venom class` against
  `1 for the flash class` on the same team, so the class is not cosmetic here
  even though `DEFAULTS.speed_classes` is `None`. Both walked cells are Venom;
  a Flash or Rapier cell was not walked.
- **AI count is not a launch option.** `AICount` is 7 on `Race`/`Elimination`/
  `Tournament` and 1 on `Head2Head`; the test asserts every Omega cell's value
  equals `Mode::opponent_count()`, so the field the mode already races with is
  the authored one. Nothing reads `AICount` for any title.
- **Team Selection is skipped, not wired.** `Team_Selection_Definition.xml` is
  in `data09.psarc` at HD's path and naming it in `oag_omega::frontend::FRONT_END`
  makes the screen read, but it draws an empty frame: the logos are at
  `Data\art\published\hdships\<Team>\FE\Logo.gnf` where HD's reader asks
  `Data\Ships\<Team>\FE\Logo.gtf` (`oag_ui_screens::picker::hd::logo_src`), the stat
  blocks do not draw, and `hdships\<Team>\screen.xml` has no slideshow chain.
  A cell therefore races whichever team the RACE page holds (`settings.race.team`;
  `Session::apply_race_team` copies it in): slot 0 was `hdships\Assegai` on the walk,
  the first entry of the catalogue, which is where `settle` falls when the stored
  id is not offered (the catalogue lists `AG_Systems`; `DEFAULTS.team` spells it
  `ag_systems`). **Chosen, not measured.**
- **Omega also ships 2048's end-race file** (checked 2026-10-06, `checked, applies, not wired`): `data09.psarc`'s `data/plugins/frontend/gui/vita/vita_EndRace_Definition.xml` is byte-identical (22,098 bytes) to 2048 v1.04's `NEWGUI/EndRace_Definition.xml`, beside Omega's own HD-lineage `endrace_definition.xml`; nothing in Omega's front-end XML names the `vita/` file. See [endrace-2048.md](../ui/endrace-2048.md#omega).
- **The EndRace screens are skipped, not wired.** (Census 2026-10-06: the three copies of `EndRace_Definition.xml` author Results, Menu and Podium and no Rewards - `hd-endrace-screens.md`.) `EndRace_Definition.xml` is at
  HD's path too, and dispatching Omega through HD's loader draws `EndRace
  Results`, but `--menu-page endrace-menu` then draws no option blocks where
  HD's draws three, so the way back (`RETURN TO GRID`) is unreachable. Without
  the screens a finished race (autopilot, 3 laps of `01_Track`, about 7,000
  ticks) shows the built-in results table and the next confirm returns to
  `Main Menu`, **not** `Cell Selection` as on HD.
- **Omega has no `Campaign Selection` screen and no flyer cards in this build** (`load_omega` passes `selection_layout:
  None, grid_layout_fury: None, flyers: None`; `RACE CAMPAIGN` opens `Grid
  Selection` directly, `Event 01/16`); the flyer-card work drew them for HD only.
- **Not Omega's to fix here, seen on the walk:** the HUD is absent in the race
  (`HUD_Components.gtf`/`hdHUD.mip` are not in the archives), 793 of the
  circuit's materials are unresolved (white surfaces on `08_Track`), and the
  menu backdrop is white.

## Racing: a race starts, on this title's own data

**Try:** `oag-game data/extracted/ps4 --race
--hold cross --no-audio --screenshot out.png` (any directory holding both
packages' `uroot/dataNN.psarc`; they are found by name). `just play omega`
reads the same directory. `data/extracted/ps4` has held the corrected
extraction since 2026-09-29; the short-read copy is `data/extracted/ps4.bak`,
and a race does not get this far on it. Census figures on this page that name
a count were measured on whichever copy the paragraph says.

`omega-race`, 2026-09-29. On the corrected extraction a race loads the real
spline, the real collision, the real craft and the real circuit geometry with
its textures, the craft rides the circuit (`grounded 1.0`, 27 units/s after 300
ticks with accelerate held, 58 units along the spline by tick 900, where it
stops against the first barrier - nothing steers it), and the frame is legible: an AG Systems hull on Tech De Ra's start gantry with the
circuit's road, barriers and banners behind it
(`data/scratch/omega-race/j300.png`). Nothing here is a stand-in for an asset
the disc authors except what "What does not draw right" lists.

### What loads, from what, through which reader

| Layer | Omega file | Reader | Measured |
| --- | --- | --- | --- |
| Spline | `track.vex`, `WO Track` | `oag_vex::track`, **unchanged** | 39 of 39 files, `encoded_len` = payload length |
| Start position, pads | `track.vex` nodes | unchanged | `tech_de_ra`: 15 speedup and 11 weapon pad volumes |
| Collision | `track_col.col` | `oag_vex::kdcol`, **19-byte node** | 38 of 38 decode; `tech_de_ra` 12,894 vertices, 20,777 triangles |
| Circuit geometry | `track.final.rcsmodel` | `oag_rcs::rcsmodel::psp2`, **third pointer gap** | `tech_de_ra` 3,186 submeshes, 2,379,040 triangles, 0 unpaired |
| Node table | same file | `psp2::nodes`, **8-byte pointers** ([`2048-animation.md`](2048-animation.md#the-models-node-table)) | `tech_de_ra`: 1,702 nodes, all with a written matrix; 2,659 mesh objects; 1,684 node-bound submeshes placed by their bind matrix |
| Skeleton, clip | `track.final.rcsskeleton`, `.rcsanimclip` | `oag_rcs::rcsskeleton`, `rcsanimclip`, **8-byte offsets** | `tech_de_ra`: 168 nodes, 64 animated tracks, 250 s loop, 150 submeshes moving |
| Materials | same file | `psp2::material::read_ps4`, **by the header's own table** | 461 materials; 453 resolve a texture, 3,155 of 3,186 draws textured |
| Textures | `.gnf` | `oag_texture::gnf`, unchanged | as [`gnf.md`](gnf.md) |
| Craft hull | `hdships\<team>\Ship.vex` + `ship.rcsmodel` | `psp2` | `ag_systems` 12 submeshes, 22,666 triangles, 4 of 4 materials textured |
| Handling | `hdships\<team>\handlingstats.xml` | `oag_tables::handling`, unchanged | `ag_systems`: team "AG Systems", class `venom` |

The three changes, each with what it rests on:

1. **A collision node is 19 bytes, not 24 - confidence 85.** All 38
   `track_col.col` files state stride 19, and in every one the next `"----"`
   sits at exactly `0x14 + 19 * N`. `environments2048\altima` is 2048's own file
   with the node array re-encoded: past it, 415,286 bytes (leaf indices, bounds,
   triangle soup, final tag) are **byte-identical**, and the file is smaller by
   exactly `5 * 27,199`. The packed node is the wide one with the constant
   `0x000b` half-word dropped and the axis narrowed to a byte; against 2048's
   nodes on altima the children, axis, count and leaf start agree on all
   27,199, the split on 22,582 exactly and on 4,617 to one ulp. All 38 files'
   leaf runs tile their leaf array. `oag_vex::kdcol::NodeLayout::Packed`;
   [`2048-collision.md`](2048-collision.md).
2. **A circuit's model is `track.final.rcsmodel` - confidence 95, a counted
   fact.** 22 circuits use the `.final` spelling for the model, skeleton, clip,
   `.pvs`, audio and checkpoint files, four (`zone_1`..`zone_4`) use the plain
   one, and none ships both. The game tries the plain sibling first and the
   `.final` one only when it is absent (`oag_mesh::mesh::rcs::sibling_name_cooked`),
   so no other title's lookup changes.
3. **A PS4 `.rcsmodel` is 2048's container with 64-bit pointers.**
   - *Same container, told apart by header word `+0x04` - confidence 95:* 0 on
     all 953 of 2048's base package, `0x100` on all 1,272 of Omega's (entries,
     a few paths repeat across archives)
     (`crates/rcs/examples/omega_rcsmodel_census.rs`).
   - *Submesh record: vertex pointer 32 bytes past the index pointer, not 28 -
     confidence 90.* Twelve records on `ag_systems\ship.rcsmodel`: three in the
     Vita's 184-byte shape (a mirrored twin, 710 triangles, all a reader
     without the new gap found) and nine in the new one (the body alone is
     10,486 triangles). Every pair clears the reader's arithmetic and the
     position and 3-signed-byte normal decode at the Vita's offsets to unit
     normals (mean 0.994). Cross-title: Omega's Feisar and Qirex hulls decode to
     17 / 22,527 triangles / 5 materials and 15 / 29,643 / 4, **identical** to
     2048's ports of them (vertex counts differ - Omega re-welded them). All
     the 1,272 Omega `.rcsmodel` entries all decode, with 90 unpaired pointers in total
     (82 in the crowd rigs, 2 on `sol`, 6 in the front-end scenes). The Vita
     corpus is unaffected by construction: a gap is only consulted for a site
     no known gap paired, and no Vita site goes unpaired.
   - *Materials, by the header's own table - confidence 95 (was 75, by name
     pointers alone, until 2026-09-30).* The `.rcsmaterial` paths sit in the
     clear and a material is a 64-bit word pointing at one; the file states its
     count at CPU `+0x70` and its table of header pointers at `+0x78`
     ([below](#a-ps4-submesh-points-at-its-own-vertex-declaration-and-its-materials-are-the-headers-own-table)).
     A submesh names its material 0x28 bytes before its record, and all
     239,108 submeshes in the package name one inside the table. On
     `ag_systems` `GlassShape` gets `glass_texture` and `FlashyFlashyShape` gets
     `emissive_bloom`.
   - *A sampler entry is 0x28 bytes, the name hash first and the texture
     pointer 0x18 in - confidence 85.* On `tech_de_ra` the hashes resolve to
     `lightmap` (291 entries, every one an `-lmap.gnf`), `Texture1`,
     `DiffuseTexture`, `Diffuse`, `Normal` and so on. **This is what picks the
     diffuse:** the road, `track_surface_displacement2out`, lists
     `track_de_ra_displacement_df2.gnf` first and read in address order the
     first frame drew the road as a white sheet; ranked by sampler role its
     `Diffuse` (`track_de_ra.gnf`) comes first.

### A PS4 submesh points at its own vertex declaration, and its materials are the header's own table

Two readings that looked settled were wrong for part of the corpus, both found
by asking why a lightmap control group was not exact.

- **A submesh record holds a 64-bit pointer to its own declaration's header at
  `+0x28` from the record start - confidence 95, a counted fact.** It resolves,
  with the stride the declaration states equal to the buffer-derived one, on all
  226,381 submeshes of the five base archives
  (`crates/rcs/tests/omega_declaration_ground_truth.rs`). The Vita's `u32` at the
  same offset resolves for 14.2 % and is left alone. Keyed by stride, one
  declaration stood for every chunk of that stride, and `tech_de_ra` has 43
  layouts over 8 strides: 285 of its 3,186 submeshes read the diffuse coordinate
  out of a tangent or colour set, and the reversed circuit decoded 218,672
  vertices to non-finite floats. Both are gone. A PS4 file no other reading of
  which is measured falls back to the stride's declaration.
- **The declaration's `lightmapUV` and the material's `lightmap` sampler agree
  on every submesh** - 50,042 have both, the rest neither, **0 in either
  off-diagonal** - and all 17.97 M decoded lightmap coordinates lie in `0..=1`.
  Two independent reads agreeing everywhere is what says the pointer offset is
  right; it is also what found the second bug.
- **A PS4 material table is the header's, not the address order of the name
  pointers - confidence 95.** The count is a 64-bit word at CPU `+0x70` and the
  address of `count` header pointers at `+0x78`; a header's `.rcsmaterial` name
  pointer is 8 bytes in. Every file's read materials are exactly that count.
  Taking every name site in address order gave the same list where none lies
  outside the table, and shifted every later material by one where one does
  (`05_ubermall`'s `diffuse_normal_specular.rcsmaterial` at `0x1a248`: 594
  sites, 593 entries; thirteen circuit models) - a submesh drew with its neighbour's
  material. `tech_de_ra`, `talons_junction` and the craft were not affected.
- The atlas decodes as BC7 at 2048x2048 with an alpha channel whose mean is 32 to
  77 of 255. What the alpha is for is not read.

**The mount order is chosen, not measured.** 10,921 of the 27,077 distinct
paths are named by more than one archive (the patch repacks most of the base)
and where the copies differ the patch's is the larger, newer one - 29 of 40
sampled race-relevant collisions differ, and every `.EnvSettings` is ~4.8 KB in
the base and ~5.6 KB in the patch. `oag_omega::EXTRA_CANDIDATES` therefore
searches `data08`, `data07`, `data05` before `data00`-`data04`. Nobody watched
a PS4 mount them.

### The material uniform and sampler table (2026-10-05, `omega-uv-scroll`)

A PS4 material header carries the same instance table as the Vita's
([2048-material-params.md](2048-material-params.md)), with 64-bit pointers and
one improvement: **a uniform states its own component count**. Read by
`oag_rcs::rcsmodel::psp2::material::ps4_params`; confidence 92 on the layout.

```text
header  +0x3c u32  count of 32-bit floats   +0x40 u64  offset of the float pool
        +0x48 u32  entry count              +0x50 u64  offset of the entries
        +0x34 +0x38 +0x44 +0x4c +0x54       zero on every material
entry (0x28 bytes)
        +0x00 u32  name hash, ~crc32(name)  +0x04 u32  kind: 1 uniform, 0x12 sampler
uniform +0x10 u64  offset of the value      +0x18 u32  components << 16 (1 to 4)
        +0x1c u32  pool, 0x1000 the float pool (no half pool on PS4)
sampler +0x18 u64  offset of the .gnf path, 0 for a sampler nothing is bound to
        +0x20 u32  sampler state (mostly 0x522800; meaning not read)
```

**What the numbers say**, `crates/rcs/tests/omega_material_param_ground_truth.rs`
over all nine archives (five base, `data05`, `data07`, `data08`; `data09` ships no model),
every `.rcsmodel` that has materials:

- **0 entries of a third kind, 0 dropped, 0 stray header words.** Every entry
  is a uniform or a sampler and the reader returns every one; a sampler is a
  `.gnf` path or a null pointer (unbound: 18,746 of them).
- **The float pool tiles exactly on all 34,423 materials**: the uniforms'
  stated component counts add up to the pool's count with none missing and none
  overlapping. A wrong stride or offset would not do this; it is the invariant
  that replaces the Vita pass's name check, because PS4 ships no shader that
  names its inputs.
- **The names are 2048's, not HD's.** Of the `uv_anim`/`uvanim`/`scroll`
  material families (35 names over `data00` to `data02`), the glow layer's
  `Emissive_UV_Offset`, `Emissive_UV_Scale`, `GlowTint` and `time`/`TimeScaler`
  resolve through the 2048 table (`mt_uvanim_diffuse_emissive*`,
  `uvanim_diffuse_emissive*`, `fc09`/`fc10_effects_vscroll_lambertalpha_emissive`)
  and the glow uniforms read the widths their names say (1, 1 and 3) on every
  one. HD's names (`V_Offset`, `VSpeed`, `speed`) appear only on
  `scrollingalpha`, `basic_uv_scroll` and the `uvanim_diffuse_emissive*_bloom`
  extras, which no consumer reads.
- **317 materials are glow layers** (both `Emissive_UV_*` uniforms plus bound
  emissive and diffuse samplers: 16 in `data00`, 128 in `data01`, 153 in
  `data02`, 20 in `data04`) and **19 are plain `speed_multipliaer` scrolls**
  (2 and 17).

**Consumed by the existing 2048 glow plan, unchanged**:
`oag_mesh::mesh::rcs::psp2::glow::plan` reads `Material::params` and
`samplers` by name hash, so filling them on PS4 is the whole wiring. A Vineta K
race reports `28 glow layer(s) and 0 plain scroll(s) off the materials' own
uniforms (rates chosen, not measured)`. Rates are the 2048 rule: `TimeScaler`
where authored (**chosen, not measured**), else `1.0`, the engine clock HD's own
records play, since 2026-10-06 (`psp2-scroll`; the authored `time` is no longer
read, see [2048-material-params.md](2048-material-params.md#what-scrolls-and-what-is-acted-on)).
**Ported 2026-10-06:** the HD-named vertex scrolls (`basic_uv_scroll`
`USpeed`/`VSpeed`, `cf_uvanim_emssive*`, `emissive_bloom`/`emissive_lights`) are
admitted by name and authored rate hashes, labelled inherited from HD in the load
report (Omega's GCN is unread, so no program confirms `time` there); 11 materials
on Anulpha Pass, up to 15 on Amphiseum, 0 on any `environments2048` circuit
(`crates/render/tests/psp2_glow_ground_truth.rs`).
Frames: `data/scratch/omega-uv-scroll/after_vineta_{120,180,240,300}.png`
against `before_vineta_300.png`. Before, the panels the glow layer lights are
dull grey; after, they glow. The only pixels that differ from main's build at
tick 300 are those surfaces (438 pixels over a threshold of 24, all inside one
band of the frame). On that panel the texture is near-uniform, so the scroll is
subtle at this size.

### What one race costs in memory

2026-09-30, `lane/omega-memory`. Tech De Ra forward, `--race --hold cross
--ticks 300`, a **debug** build, Linux, peak RSS from `getrusage` (the
`--screenshot` run; the same binary that took 375 s and 9.6 GiB).

| | RGBA8 decode (before) | BC7 blocks (after) |
| --- | --- | --- |
| **Peak RSS** | **9,605 MiB** | **3,360 MiB** |
| Wall clock to the still | 375 s | 7 s |
| Track albedo, CPU (461 textures) | 6,129 MiB | 2,043 MiB |
| Track lightmaps, CPU (51 atlases) | 709 MiB | 236 MiB |
| Sky (6), CPU | 384 MiB | 128 MiB |
| Craft (3 teams), CPU, decoded once per team | 513 MiB each | 171 MiB each |
| Track GPU, summed uploads incl. mips | 8,172 + 945 MiB | 2,043 + 236 MiB |
| Craft GPU, 8 craft | 684 MiB x 8 | 171 MiB x 3 distinct (20 of 551 uploads reused) |
| Track vertices + indices, CPU and GPU each | 188 + 27 MiB | unchanged |
| Archive buffers held | none (`RssFile` 19 MiB) | none |

Every texture in the circuit is BC7 (`Thin_1DThin`) and 4 bytes a texel decoded
against 1 as the disc ships it, so the decode was the whole cost: the textures
**are** 2,279 MiB of blocks and 6,838 MiB decoded, and the source-format size
is what the upload now is. The biggest are 18 x 8192-square and 21 x
4096-square files - the craft liveries are 8192-square (85 MiB with the chain)
despite the `_1024` in their names.

What was tested against each lead suspect: **(a) confirmed and fixed** - BC7
passes through with the disc's own chain (`gnf.md`, "Mip chains"). **(b)
confirmed and fixed** - the lightmaps are BC7 too, 709 to 236 MiB. **(c)
confirmed** - a `Drawable` kept its `Model`, so every decoded texture lived for
the race; `Model::release_texels` now drops them once uploaded, which frees
2.3 GiB of blocks after the upload. **(d) refuted** - `RssFile` is 19 MiB; the
PSARC readers seek. **(e) refuted as a large item** - the whole circuit's
geometry is 215 MiB per copy, against 2 GiB of textures; a 2.38M-triangle
circuit's parse costs about 380 MiB transient.

**What the peak was, and the change that removed it** (`omega-texture-stream`,
2026-10-02). Every model's textures were decoded before the scene built any of
them, so the peak (3,360 MiB) was all of them at once; after the uploads RSS
fell to 2,585 MiB and stayed there because glibc keeps freed pages. A load
given a device now uploads each texture as it is decoded
(`race::TextureSink`, `oag_mesh::mesh_render::TextureSinkScope`,
`Texels::Uploaded`; the mechanism is in
[`race-load-transition.md`](../architecture/race-load-transition.md)), so a
`Model` never holds the blocks and glibc never has them to keep.

Same method, same circuit (Tech De Ra forward, `--race --hold cross --ticks
300 --screenshot`, debug build, peak from `getrusage`, steady RSS read at the
last log line; the two builds were run back to back on one machine):

| | before (main `503eb268`) | after |
| --- | ---: | ---: |
| **Peak RSS** | **3,417 MiB** | **792 MiB** |
| RSS at the still, after every upload | 2,650 MiB | 560 MiB |
| `CPU kept` in the loader's texture line (track, sky, craft) | 2,043 + 236 / 128 / 171 each | 0 |
| `GPU uploaded` in the same line | unchanged | unchanged (2,043 + 236, 128, 171 each) |
| Still, `cmp` against the before still | | byte-identical |

The shared RCS path moved too, and draws the same pixels:

| `--race --hold cross --ticks 300 --screenshot`, default circuit | peak before | peak after | still |
| --- | ---: | ---: | --- |
| Wipeout HD Fury (`hdfury-ps3-eu-dec.iso`, Dion) | 769 MiB | 667 MiB | byte-identical |
| Wipeout 2048 (Vita `PCSF00007`, Altima) | 701 MiB | 349 MiB | byte-identical |

What this does not cover, so the numbers are not read wider than they are:

- **Only a caller whose scene is built from the device it loaded through opens
  the sink**: the menus' `LAUNCH RACE` (the frame loop's own device) and the
  headless `--race --screenshot` (which now opens its device before the load).
  The windowed `--race` loads before its window exists, and `--trace-out`,
  `--dry-run` and the 2048 front-end capture build no scene from that device;
  those keep every texture on the CPU as before.
- **The GPU side is not in RSS on this machine** (an AMD Radeon RX 7800 XT, RADV, discrete). On a
  software or integrated adapter the uploaded bytes live in the process, and the
  total there is the GPU column, which this does not shrink.
- **HD's `sky.gtf` is still decoded on the CPU** (24 MiB kept in the loader
  line): the sky cube path builds its textures without going through the
  `.rcsmodel` decode sites the sink is wired into.
- The flush rule (a submit and a non-blocking poll every 64 MiB uploaded) is
  **chosen, not measured**; the peak above is with it, and no run without it
  was taken.

**Textures wider than the device allows.** 2026-10-02, `omega-talon-crash`. Two
circuits ship a 16,384-wide texture, against wgpu's default
`max_texture_dimension_2d` of 8,192: Talon's Junction's
`texturesps4/ds_floor_cs.gnf` and Modesto Heights'
`texturesps4/track_floor_diffuse_new.gnf` (both 16,384 by 4,096). On `main` a race
on either panicked in `create_texture` ("Dimension X value 16384 exceeds the
limit of 8192", reproduced on both). Every other circuit's largest side is 8,192
(the craft liveries and the sky among them), including the 10 `environments2048/*`.
Two changes, one commit:

1. **The device asks for the adapter's own texture limits**
   (`oag_mesh::mesh_render::required_limits`: wgpu's defaults with only the
   1D/2D/3D dimension limits raised, `Limits::using_resolution`). RADV and
   lavapipe both report 16,384, so on this machine both circuits go up whole.
   It also raises the clamp `oag_present::upscale::target_size` reads from the same
   limit; `Scale::RANGE` tops out at 200 %, so only a display wider than 4,096
   pixels at that scale could ever have hit 8,192.
2. **A texture still over the limit uploads from the first mip level that fits**
   (`texture::first_fitting_level`, one rule shared by the `Chain`, `Blocks`,
   decoded-blocks and synthesised-RGBA8 arms, so a 16,384-by-4,096 chain on an
   8,192 device goes up from level 1, 8,192 by 2,048). **Chosen, not measured**:
   nothing is known of how the original treated an oversize texture. A texture
   with no fitting level (a single-level one over the limit) is not uploaded and
   its slot is bound as a texture that never decoded is: the same path as a `.gnf`
   the decoder refuses (`a_gnf_the_decoder_refuses_draws_nothing`: the slot is
   `None` and the build binds its 1x1 "undecoded" white, which on a surface that
   carries real art is a plain white surface, see the Talon's Junction check
   below). That is the codebase's existing convention for a texture it cannot
   hold, not a new mechanism, and no Omega texture reaches it. Both cases are
   named in the loader line: `N texture(s) wider than this device's
   max_texture_dimension_2d of L uploaded from the first mip level that fits -
   chosen, not measured: <labels>` and, for the refusal, a warning. The census's
   `GPU uploaded` figure counts what went up, not what the disc holds
   (`texture::plan` mirrors `texture::upload`).

Forcing an 8,192 device (a sink on `DeviceDescriptor::default()`) over every
circuit that loads - 15 `environments/*` forwards, 12 reversed (`zone_*` ship no
reversed entry, `zone_3` loads to no triangles on `main` too: 27 loads, the same
27 as the census below) and the 10 `environments2048/*` forwards: Talon's Junction
and Modesto Heights, forwards and reversed, are the only ones that trim, one
texture each, from level 1; no texture is left undrawn anywhere. Checked on a
real race, `--race --hold cross --ticks 120 --screenshot`, on the RX 7800 XT
(16,384): all 27 loads reach the still without a panic (and the 10
`environments2048` ones). **What `ds_floor_cs.gnf` is**, found by making `upload`
refuse it locally (not committed) and differencing the stills: the circuit's
**road surface**. With it refused, the road in the `--ticks 300` frame is plain
white where it is textured otherwise (3.3 % of the frame differs, all of it road;
`talon_t300.png` against `talon_nofloor_t300.png` and `talon_diff_t300.png` under
`data/scratch/omega-talon-crash/`), so Talon's Junction draws its floor with the
whole 16,384 texture on this machine, and the white road is also what a refusal
would look like. Also viewed: `talon_t700.png`, `talon_lvp_t30.png` (lavapipe).
Tech De Ra's still, HD Dion's and 2048 Altima's are byte-identical before and
after. Tests: `texture::tests` (the rule, the descriptor's limit, an over-limit
chain / RGBA8 / BC7 on a 1,024 device, a refusal) and
`texture_stream_ground_truth`'s two Talon's Junction tests (an 8,192 device, the
renderer's own).

**Single-level `.gnf` textures per circuit.** The `.rcsmodel` loader report now
carries one clause, `; N .gnf texture(s) kept as BC7 blocks, M decoded to RGBA8
with a synthesised chain (S single-level, O off the block grid, R refused as
blocks)`, from `ModelTexture::from_gnf_form`. Over the 27 `environments/*` loads
that succeed (15 circuits, forwards and reversed where one ships; `zone_3`
loads to no triangles) and the 10 `environments2048/*` forwards, the textures the
track model names are all BC7 chains: **0 single-level and 0 off the block grid
on every circuit**, and 1 refused as blocks that still decodes on `amphiseum`
and on `modesto_heights`, forward and reversed (the report counts it but does not
name the file).
The archives do hold 846 single-level `.gnf` entries (a scratch census over
the base and patch archives **without deduplicating a path present in both**, so
an upper bound on distinct files; it also could not parse a large share of the
`.gnf` names it was handed, not looked into here), all of which decode: 99 under
`environments2048/mall`, 71 `tower`, 59 `shared`, 135 `art/published`, and the
rest front-end and particle art - but
no circuit's track model names one. Whether the 200 to 690 unresolved draws each
`environments2048` circuit reports (identical on `main`) are materials that
name them was not checked.

**A picture difference, and it is the mips.** The still differs from the RGBA8
one by a mean 0.88 of 255, 0.7% of pixels over 16, at high-contrast edges:
the renderer now samples the disc's authored chain instead of a box-filtered
one, and the GPU's BC7 decoder does the base level. The authored chain is what
the PS4 sampled.

Not Omega, but found here: **Pulse and HD peak at 8.4 and 8.2 GiB, and it is
not their assets** - it is 128 full `Drawable`s per weapon model
(`MAX_PROJECTILES`), 1,152 on Pulse and 896 on HD, each about 7 MiB. Omega
builds no weapon pools. See the handover thread.

### What does not draw right, or at all

- **Node-bound scenery is placed by the model's bind matrices, and nothing
  else places it** (`omega-nodes`, 2026-09-29). The PS4 node table reads
  (Vita layout, 8-byte pointers; one invariant per header pointer over all
  124,709 mesh objects, [`2048-animation.md`](2048-animation.md#the-models-node-table)),
  and a race now draws `tech_de_ra`'s 1,684 node-bound submeshes at their
  nodes. Before, all of them drew in node space, a clump inside the box
  `(-87, -25, -46)..(25, 25, 23)` around the world origin - visible from the
  start looking toward -z as a grey blob of legs and pods beside the AG
  banner (17,446 pixels differ, all inside that blob); now the camera droid
  `CamBot_New2` hovers over the stands at `(-73, -1, 89)`, the crowd entities
  stand at their nodes' translations (placed box `(-1275, -79, -1097)..(1402,
  566, 508)`), and the craft's airbrake flaps are back on their hinges at
  `(±2.16, -0.05, -4.47)` instead of under the cockpit
  (`data/scratch/omega-nodes-2/omega-{before,after2}-clump.png`,
  `omega-{before,after1}-t300.png`). **The box at the bottom of
  `final_300_close.png` is not node-bound and did not move**: it is static
  circuit geometry (`node None`, road level under the start position; the 56-
  and 228-vertex `wohdtrack_0021Shape`/`SFLine_003Shape` boxes contain the
  point), drawn identically with and without the table - the earlier note that
  a node-bound prop sat under the camera was an inference that the render does
  not bear out. Nothing within 25 units of the craft at tick 300 is node-bound
  except the droid. A node with no written matrix and no skeleton entry is not drawn
  and is counted (`Report::unplaced`): 3,220 submeshes on `data00` and 15,035
  on `data04` while the skeleton did not decode, all in 2048's `trackZone`
  (Zone mode) models and a few props on `cathedral`/`mall`/`tower`. With the
  skeleton read the census finds none: the skeleton names every one (below).
- **Skeleton and clip decode, and the scenery moves** (`omega-catchup`,
  2026-09-30). The PS4's are the Vita's files with 8-byte offsets
  ([`2048-animation.md`](2048-animation.md#rcsskeleton)); `tech_de_ra` is 168
  skeleton nodes and 64 animated tracks over a 250 s loop, 150 submeshes move,
  and the 1,534 model nodes the skeleton does not name stand at their own bind
  matrix. The camera droid `CamBot_New2` behind the start line is upright and
  facing the grid at tick 0 and has dropped and turned away by tick 1800
  (`data/scratch/omega-catchup/droid-crop.png`, from `droid-t0.png` and
  `droid-t1800.png`, the same camera pose). The 12,310 mesh objects on nodes with no matrix of their own across Omega's base
  archives (2048's Zone models and a few props) are all on nodes its skeleton
  names, so the census leaves none unplaced; **`Report::unplaced` itself was not
  re-measured through the render path on such a model** (no Omega race loads a
  Zone model). What is
  *not* checked: the rotors' spin and the crowd against a reference (nothing
  to compare with), and the shader's node-table ceiling on the Zone models,
  which no race here loads.
- **Lightmaps are bound, and their combination is chosen, not measured**
  (`omega-catchup`, 2026-09-30; **the combination is now read, see the next
  bullet**). The atlas is each material's `lightmap`
  sampler (a 2048x2048 BC7 `-lmap.gnf`, 49 distinct on `tech_de_ra` reversed)
  and its coordinate is `lightmapUV` in the submesh's own vertex declaration
  ([the declaration pointer](#a-ps4-submesh-points-at-its-own-vertex-declaration-and-its-materials-are-the-headers-own-table)).
  The shader path is Wipeout HD's, unchanged: the prelit curve on the atlas
  colour and the atlas alpha as the sun mask. Nothing measured says that is this
  title's combination. **Two authored things it does not read:** the patch's
  `.EnvSettings` carries `"Lighting.Nova prelit scale bias power"=2.5 0.2 2`
  (2048's Vita files carry `1 0 1 0`; the formula behind three numbers is not
  recovered, so the scene keeps its 1/1 defaults), and a `Tonemap.*` block
  (`Exposure minimum/maximum/response/time`, luminance and source-colour
  coefficients) where the reader looks for HD's `HDR and Bloom.*`, so the earlier
  "no HDR/bloom block" line means "none under HD's names". The atlas alpha is 39
  to 55 % zero on the three atlases sampled, so the sun mask is not the constant
  it is on HD's circuits, and what it means here is a question for the shader.
  Frames (forward circuit, tick 600, hold accelerate, debug build):
  `data/scratch/omega-catchup/fwd-t600-pre.png` (no lightmap, 11.98 % clipped
  white, mean luminance 0.619) against `fwd-t600-lm1.png` (8.32 %, 0.563), with
  HD's Tech De Ra at tick 300 (`hd-tdr-t300.png`, 4.78 %, 0.321) as the only
  comparison there is. The reversed frame at tick 300 goes the other way
  (0.99 % to 7.04 %), so the aggregate is not a verdict; the walls now carry
  baked shading where they were flat. **The purple crystalline kerbs are not
  a defect of this title's reader**: HD's own render of the same circuit has
  them. Load time is about 19 s longer in a debug build.
- **The lightmap's combination is read off the pixel shaders, and wired for
  Omega alone** (`omega-lightmap`, 2026-10-02). `Lighting.Nova prelit scale bias
  power` is three scalars, `(scale, bias, power)`, and the circuit pixel shaders
  compute **`scale * pow(lightmap.rgb, power) + bias`** on the *raw* atlas
  (`v_log_f32`, `v_mul_f32` by the power, `v_exp_f32`, `v_mad_f32` by scale and
  bias; no clamp). All 1,028 `*-lmap.gnf` atlases are BC7 **UNORM** (the same
  descriptor parse reads number format 9 on albedo and 0 on normal maps), so the
  HD renderer's `pow(lightmap, 2.2)` was a second decode. **No constant ambient
  joins a lightmapped surface**: of 4,664 unique pixel shaders that declare the
  triple, all are lightmapped and none declares `constantAmbientColour`. The atlas
  alpha is the direct sun's mask (`liveLighting0diffuse * lightmap.a * N.L`), the
  same role as on HD, so the 39 to 55 % zero is a baked sun shadow. Evidence,
  addresses and census:
  [`ps4-omega-eu/lightmap-prelit.md`](../ghidra/functions/ps4-omega-eu/lightmap-prelit.md).
  A file that omits the key keeps the executable's static default
  `(1.4, 0.2, 1.5)`; 88 of the title's 97 files author it, 20 distinct triples,
  one with a negative bias.

**Airbrake flaps swing** (2026-10-05, `airbrake-flaps`). Same builder as 2048 (`psp2::build`); the twelve `hdships` hulls in `data01.psarc` all carry both flaps.
The swing is the same `Flap::deflect` Pulse uses (`hinge * Rx(angle) * hinge^-1`,
about the hinge frame's local X), scaled by the title's own `<AirbrakeGraphics>`
`amount` and rates through `RaceView::airbrake_flaps`. **Which way it turns is
checked, not read:** the title's own `Airbrake` handler is unread, so
`psp2_airbrake_flaps_ground_truth` asserts the physical claim Pulse's recovered axis makes (a positive
deflection raises the flap and flares it outward, both sides) on every Omega hull in `data01.psarc`. Only
the player's craft swings, as on Pulse; a rival's flaps stay stowed.
Frames: `data/scratch/airbrake-flaps/` (`omega_*.png, kc2_omega.png`).

  **Wired** in `oag-render` as `Light::with_nova_prelit` and `mesh.wgsl`'s
  `nova` branch, **only for a PS4 container** (`GeometryKind::Ps4`, by
  `psp2::is_ps4`): 2048 authors the key as `1 0 1 0` and its Vita shader is
  unread, so 2048 does not take it. The branch is gated on the lightmap bit, so
  the bias never lights an unlit draw. Tests: `omega_nova_prelit` (the quad
  reads `2.5 t^2 + 0.2`, no ambient leaks in, an unlit draw is unchanged, HD's
  curve is unchanged with the flag off; it fails if the bias is dropped) and, on
  the real disc, `omega_lightmap_ground_truth`.

  Frames (`oag-game --no-audio --race --hold cross`, debug, 1440x816,
  `data/scratch/omega-lightmap/shots/`, before from `main` `679bd898`):

  | frame | clipped white before -> after | mean luminance |
  | --- | --- | --- |
  | Tech De Ra forward, tick 300 | 8.13 % -> 6.72 % | 0.566 -> 0.529 |
  | Tech De Ra forward, tick 600 | 12.24 % -> 10.47 % | 0.585 -> 0.598 |
  | Tech De Ra reversed, tick 300 | 5.47 % -> 3.88 % | 0.522 -> 0.491 |
  | Anulpha Pass forward, tick 300 | 2.75 % -> 2.86 % | 0.293 -> 0.315 |
  | Anulpha Pass reversed, tick 300 | 4.27 % -> 4.46 % | 0.343 -> 0.364 |
  | HD Tech De Ra, tick 300 | 5.16 % -> 5.16 % | **byte-identical** |
  | 2048 default circuit, tick 300 | 4.97 % -> 4.97 % | **byte-identical** |

  **Sweep of all twelve forward circuits** (tick 30): every one binds its
  lightmaps with **0 unresolved** and reports its own triple
  (`omega-lightmap` load logs), so no lightmapped draw falls back to the bias
  without an atlas; the outlier is Sebenco Climb's `16 * lightmap^12 + 0.96`,
  which renders brighter (0.375 -> 0.461 mean luminance, 0.43 % -> 1.65 %
  clipped) and plausible. The Zone rig rebuild (`ZoneGrade::light`) now carries
  the Omega flag through, though Omega ships no Zone palette today
  (`zone_palette: None`), so no Omega Zone race reaches it.

  Read by eye (`*-before-after.png`): Tech De Ra's blown-white upper walls now
  carry panel shading, and Anulpha Pass's walls and floor take a deeper baked
  shadow, which is what dropping the 1.0 constant ambient and the second sRGB
  decode do. **The clipped share does not rise, against the expectation that a
  2.5x scale would clip more**; it is not a verdict either, because the
  original writes fp16 and tonemaps afterwards.

  **Not wired, recovered:** the shadow-light factor on the prelit term
  (`0.75 + 0.25 * sat(N.L_shadow) + ...`, general in presence across the 4,664
  shaders and fixed in form in only 1,780), left at 1 (**chosen, not
  measured**); and the vertex-colour variant of the same curve (`NOVAColor`),
  which needs role bits the PS4 material container does not yet provide.

  **The `Tonemap.*` block is applied** (`omega-tonemap`, 2026-10-05). Its consumer
  is `ToneMap_ApplyEnvSettings` (`0x01620980`), which reads the block through the
  environment object (the earlier absolute-address search could not see that), and
  the law is the executable's own, read off GCN microcode and checked by emulation:
  a Rec.601 mean luma of the frame, averaged over `round(time * 60)` frames,
  `LAvg` stepped toward it by at most `response / 60` a frame, an exposure
  `clamp((a + b LAvg) / LAvg, min, max)`, and a **cubic Hermite curve** from `(0, 0)`
  to `(t1, 1)` with end slopes set by the two angles, applied **per colour channel**
  inside the 4x MSAA resolve. Bloom is added after it. Evidence, scores and what is
  not read: [`ps4-omega-eu/tonemap.md`](../ghidra/functions/ps4-omega-eu/tonemap.md).
  The answer to "does Omega render differently by console" is: only in where the
  curve runs (the 4x MSAA resolve on a base PS4; a compute resolve with the same
  coefficients under the Pro's checkerboard mode, whose sample setup is not read); HDR video out switches to the
  `TonemapHDR.*` twins and a curve ending at 40.0 instead of 1.0.

  **Wired** as `oag_post::omega_tonemap`: an Omega race (a PS4 circuit
  whose file has the block) draws into the linear `Rgba16Float` scene target and
  the chain runs the law, then the `pow(1/2.2)` display encode every linear target
  here ends on (**chosen, not measured**: the scanout format is unread; the HDR
  twin's 0..40 range is why the curve's output is taken as linear light). Also
  chosen: per pixel rather than per MSAA sample, the first frame starts settled,
  no readback latency, brightness at the middle setting. Tests:
  `the_law_reproduces_the_original_coefficient_shader` (numbers produced by the
  original shader's own instructions on an emulated lane),
  `the_chain_draws_the_law_on_a_real_device`, and on the disc
  `omega_lightmap_ground_truth`. Frames (`oag-game --no-audio --race --hold cross`,
  debug, 1440x816, `data/scratch/omega-tonemap/shots/`, before from `main`):

  | frame | clipped white before -> after | mean luminance |
  | --- | --- | --- |
  | Tech De Ra forward, tick 300 | 5.97 % -> 0.01 % | 0.529 -> 0.412 |
  | Tech De Ra forward, tick 600 | 9.31 % -> 0.14 % | 0.598 -> 0.453 |
  | Altima (2048 heritage), tick 300 | 7.58 % -> 0.09 % | 0.713 -> 0.566 |
  | Altima (2048 heritage), tick 600 | 13.51 % -> 3.08 % | 0.585 -> 0.501 |
  | HD Tech De Ra, tick 300 | 5.26 % -> 5.26 % | **byte-identical** |
  | Pulse PSP, tick 300 | 4.35 % -> 4.35 % | **byte-identical** |

  Read by eye: Tech De Ra's blown ship hull, banner and upper walls carry their
  detail. **Altima's road is a flat colour in both frames**: before, it clipped
  to white; after, it is a flat pink with no texture. That is a defect the clip
  was hiding (a road surface lit far past 1.0, or drawn without its texture), not
  the curve, and it is open.

- **`track.final.pvs` is read and culls** (`omega-pvs-sound`, 2026-09-30). It is
  [HD's `track.pvs`](hd-pvs.md) in the 2048 lineage's dialect: little-endian,
  header word 2 is `1`, the bitmap is `ceil(chunks / 8)` bytes, and a chunk is a
  **`.rcsmodel` mesh object** (`scene.meshes`, 2,659 on `tech_de_ra`) and not a
  submesh (3,186). All **44** shipped `.pvs` files (34 `track.final.pvs` and 10
  zone-mode `trackzone.pvs`, over `data00`, `data01`, `data02`, `data04`) parse, declare exactly their model's mesh-object count, and fill
  their table to the byte (`psp2_pvs_ground_truth`); the four `zone_N` circuits
  ship a `.pvsxml` and no `.pvs`, so they draw every chunk and the load report
  says so. A cell sees on average 30 to 78 percent of a `track.final.pvs`
  circuit's chunks (16 to 51 percent for `trackzone`; HD's is about 33), so
  the reduction is smaller than HD's on the open circuits.
  **The name follows the model that was found**: the loader used to ask for
  `track.pvs` / `track_reversed.pvs` (derived from the `.vex`), which the archive
  does not have, so *neither* direction of any circuit was ever read.
  Measured at the same tick with `--pvs false` and `--pvs true`
  (`data/scratch/omega-pvs-sound`, debug build, 1440x816, `tech_de_ra`, hold
  accelerate; `false` is frustum culling alone, which is on by default - the whole
  scene is 3,198 draws and 2,379,040 triangles):

  | frame | draws `false` / `true` | triangles `false` / `true` | pixels that differ |
  | --- | --- | --- | --- |
  | forward, tick 300 | 1,271 / 757 | 939,301 / 619,822 | 2,384 (0.20%) |
  | forward, tick 600 | 1,283 / 759 | 952,672 / 620,986 | 2,107 (0.18%) |
  | reversed, tick 300 | 943 / 708 | 718,371 / 618,388 | **0** |
  | reversed, tick 600 | 944 / 707 | 725,503 / 625,349 | **0** |
  | 2048 (Vita) Altima, tick 300 | 616 / 447 | 145,779 / 98,032 | **0** |

  The forward frames differ in one 57 x 77 pixel block on the right-hand wall
  (`fwd-t300-crop.png`): with the PVS off a dark slab lies across the pipe wall,
  with it on the wall is continuous. **It is one object**, found by logging the
  draws the PVS rejects and the frustum passes and projecting them onto that
  block: mesh object 2412, `tracksurface:wohdtrack_0022Shape`, two triangles of
  `track_surface_displacement2out` in world space, **set in none of the
  circuit's 882 cells** (56 of its 2,659 mesh objects are set in none). That
  the original never draws it is an inference (confidence 75): no Omega
  executable has been read, and HD's PS3 loader was read to use one cell's
  bitmap as the frame's visible set, so a chunk no cell sets is one that engine
  never draws. Nothing else the PVS-off frame draws differs
  (`omega_pvs_placement_ground_truth`).
  **The cell padding is HD's** (`CHUNK_PAD`, `CHUNK_TRUST_RADIUS`, chosen for
  HD's 12-unit cell spacing; Omega's are about 5.6 apart) - **chosen, not
  measured** for this title. A debug build still spends minutes decoding the
  453 textures; culling does not shorten that.
- **Sky turn and fog are ported from 2048** (2026-10-06): the six-face sky
  cube is turned by `Sky rotation` and the circuit's `Lighting.Fog colour`
  draws, the curve inherited from HD ([envsettings.md](envsettings.md)).
  Omega's altima road still draws flat pink in a start frame (pre-existing,
  not part of that change).
- **`.EnvSettings` is read through 2048's reader** and its sun (`[4.00, 2.33,
  0.82]` over an ambient of `1.0`) is not checked against PS4's schema. The
  patch's copy carries no HDR/bloom block, so the read bloom chain is off.
- **Music plays** (`omega-music`, 2026-10-05): a race plays 17 of the 29
  `PI_Music` songs, the front end plays its own loop. A song is a Wwise
  `Music_Track` state: `Set_Music_Track_<location>__frontend` sets it, the
  music switch walks to one segment, and the song is that segment's 7 to 11
  stereo stems summed (unity, scaled down when it would clip - chosen, not
  measured). **Not played, by name:** location 0 (no event in the bank) and
  eleven songs that are one eight-channel file (`atrac9dec` refuses them). A
  headless `--race` WAV is the first song, `Shake It (WipEout Omega
  Instrumental Edit)`, at the music bus's 0.44 gain (correlation 0.9999999 with
  the offline mix). The front end's loop is `Game_FLOW` `Menus`, **not
  watched in a running original**. See [`wwise.md`](wwise.md#music-a-music_track-state-per-song).
- **Sound effects are read, not played** (`omega-pvs-sound`, 2026-09-30). The banks are
  **Audiokinetic Wwise**, bank generator version 118, not the PSP's `SBlk`, so
  `sfx: <name> not loaded: unsupported bank version 1145588546` is still what a
  race prints: nothing wires Wwise events to this project's cues.
  [`wwise.md`](wwise.md) has the container, the event-to-media chain (0
  unresolved media over `data00` and `data08`) and the `.wem`: **every one is
  Sony's ATRAC9**, mono and stereo decode in process and agree with FFmpeg to
  one LSB up to polarity, four-, six- and eight-channel files do not decode yet.
  The ten `.bnk` in `data02`, which an extension-based census had counted
  as Wwise, are the old `SBlk` container. One Tech De Ra cue plays through the mixer to a WAV.
  **Particles play** (`omega-particles`, 2026-10-05): Omega keeps its effects in
  `Data\particles` and `Data\particles2048` (not `Data\Psys`), with `.gnf`
  sprites; a race reads `particles2048` on an `environments2048` circuit and
  `particles` elsewhere (chosen, not measured - `docs/ghidra/functions/ps4-omega-eu/particle-paths.md`).
  The blend-class-8 distortion emitter (`shockdistort`) is read and not drawn (its program's arithmetic is read, `ghidra/functions/ps4-omega-eu/heat-haze.md`; the strength scalar and target format are not)
  (2026-10-06, `pob.md`): the explosions play their other emitters, and the effects
  Omega never authors are left out of its table. Still logged at WARN on a race: the two
  HUD atlases (`HUD_Components.gtf`, `hdHUD.mip`) are HD's names asked of an Omega
  archive that has no such entry, because Omega's HUD is unread (`crates/omega/src/hud.rs`:
  fragment XML under `Data\xml\` with no per-mode composition), and the blob shadow. Still silent: the
  blob shadow is this project's generated falloff, not the disc's.
- **Reversed circuits get their collision** (`omega-catchup`, 2026-09-30).
  `kdcol::sibling_name` used to pair only `track.vex`, so a reversed race
  loaded no collision at all. The reversed name is `track_col_reversed.col`
  (`_col` before `_reversed`), beside `track_reversed.vex`, on all twelve of
  `data01`/`data02`'s reversed circuits
  (`every_reversed_circuit_pairs_with_its_reversed_collision`). `tech_de_ra`
  reversed: 12,880 vertices, 20,757 triangles, 29,104 k-d nodes, four colliders,
  and the craft rides it (`grounded 1.0`, 27.1 units/s after 300 ticks,
  `data/scratch/omega-catchup/rev-t300.png`). **This also changes 2048, on
  purpose.** Its packages ship `track_col_reversed.col` for 12 of their
  reversed circuits (3 in the base package, 5 in `dlc1`, 4 in `dlc2`; an earlier
  version of this note said none, from a listing cut short). Of the 12
  `track_reversed.vex` a race can name, five author their collision in the
  `.vex` (Anulpha Pass, Metropia, Talon's Junction, Ubermall, Vineta K) and the
  sibling is never asked, so those are unchanged; **five had no collision at
  all and now load the authored one** (Amphiseum, Modesto Heights, Sebenco
  Climb, Sol 2, Tech De Ra - the last with the same 12,880 vertices as Omega's,
  and a 2048 race on it grounds, `grounded 1`, 23.1 units/s at tick 300); two
  (Chenghou Project, Moa Therma) do not load for an older reason (`decoded to no
  triangles`). Pulse, Pure and HD author collision in the `.vex` and are
  unaffected. The "before" for those five is read from the old `sibling_name`,
  not observed: this was not re-run on the old tree. If 2048 must stay
  byte-identical, the pairing is one branch in `kdcol::sibling_name` to gate on
  the title; it was left on because a reversed race with no ground is a defect
  and the file is the disc's own.
- **Defaults are chosen, not measured:** `tech_de_ra` and `ag_systems`. Zone,
  boost, speed classes and every per-team variant table stay `None`.

Reproducers: `crates/vex/examples/omega_track_probe.rs`, `omega_col_probe.rs`;
`crates/rcs/examples/omega_rcsmodel_probe.rs`, `omega_rcsmodel_census.rs`,
`omega_materials_probe.rs`; and the ground-truth suite
`crates/game/tests/omega_race_ground_truth.rs`, which skips (loudly, even under
`OAG_REQUIRE_GAME_DATA`) on a short-read extraction.

## Zone flies one shared hull, `hdships\Zone` (2026-10-06, `zone-craft`)

The maintainer's report ("Zone does not pick the appropriate craft") was a
placeholder: `zone_craft` was `PlayerShip`, a "not a finding" value. Omega's
ship-model loader (`Ship_LoadModelSet`, `0x01301ba0`, confidence 75) names
`Data\Art\Published\HDShips\Zone\Ship.vex` for game mode 6 without reading the
craft, now `oag_title::ZoneCraft::OwnShipAt`. A Zone race on `tech_de_ra` with
`ag_systems` loads and draws it (`data/scratch/zone-craft/omega-zone-hd.png`).
Livery key differs from 2048's (`zoneship_team` against `zoneship_zone`).
**Wired (2026-10-06, `omega-2048-craft`)**: an HD-era craft's `PI_TeamModel
name="zone"` `texturelocation` (`zoneship_quirex`; all twelve match a
`Zoneship_<Team>/Team.gnf` on disc, spelt as the disc spells them) replaces the
key on the hull's team texture request; the pixels change in the hull's patches
(`data/scratch/omega-2048-craft/ab-egx-default-vs-livery.png`). The five
2048-era teams author no zone model, keep the default `Zoneship_Team` skin and
the loader report says so (`Tigron` and `VanUber` name `livery1`, which has no Zone directory: the swap finds nothing and says so). 2048: **ported**, see
[2048-status.md](2048-status.md). Evidence:
[zone-craft.md](../ghidra/functions/ps4-omega-eu/zone-craft.md).

## 2048-era craft start a race (2026-10-06, `omega-2048-craft`)

Omega's `Data\plugins\teams\Definition.xml` lists nineteen raceable teams in
two eras: five `*2048` teams whose `PI_Team location` is
`Data\art\published\Ships\<Team>2048` (four craft each, `\1`..`\4`, with
`handlingstatslocation="Data\HandlingStats\<team>\<n>"` authored per craft) and
fourteen HD-era ones under `hdships`. The loader looked every craft up under
`hdships\`, so `--team Feisar2048\3` died on `hdships\feisar2048\3\handlingstats.xml`
(the old repro used a lowercase id, which no table recognises either way). Now
`oag_title::GuestRoster` carries a second tuning directory (`handling_dir`,
`None` for 2048's HD-derived twelve) and `oag_omega::race::ERA_2048_ROSTER`
names Omega's: Subdirectory join, the five teams, `Measured` (the definition
authors both directories). `omega_2048_era_craft_ground_truth` reads the
roster off the disc and resolves Ship.vex, ship.rcsmodel and a parsing
`handlingstats.xml` for all 34 craft ids, asserts each team's table directory
equals its declared `location` and that no 2048-era craft exists under
`hdships\`. A race on `Feisar2048\3` loads and draws
(`data/scratch/omega-2048-craft/single-2048era.png`). Side effect, accepted:
Omega now offers CRAFT TITLE "Wipeout 2048" in Race Remix when no 2048 disc is
present, and its own list drops the 2048-era teams to that row.
**2048 vs Omega: checked, differs** - 2048 keeps these craft as its own roster
(`team_variants`), Omega as a guest of its HD-era one.

### Vita 2048 against Omega's 2048-era craft: census (2026-10-06, `remix-labels`)

`crates/game/tests/remix_2048_sources_ground_truth.rs` reads all twenty craft
(five teams, four each) off `data/extracted/vita/PCSF00007` and
`data/extracted/ps4`:

- **Roster**: identical ids on both, `AG_Systems2048`, `Auricom2048`,
  `Feisar2048`, `Piranha2048`, `Qirex2048`, `\1`..`\4` each.
- **`handlingstats.xml`**: all twenty parse to equal `Stats` (every field).
  Omega kept 2048's tuning unchanged, so a race cannot tell the sources apart
  by handling.
- **Models**: `ship.rcsmodel` differs in size for every craft (for example
  `Feisar2048\3` is 476,012 bytes on the Vita disc, 489,720 on Omega; Qirex
  craft are smaller on Omega, the rest larger). Textures are `.gxt` against
  `.gnf`. Not decoded to a per-vertex or LOD comparison here.

**Race Remix offers both** (maintainer, 2026-10-06): with only Omega mounted,
CRAFT TITLE offers the plain "Wipeout 2048", backed by Omega; with only the
Vita disc, unchanged; with both, "Wipeout 2048" is the Vita disc's and
"Wipeout 2048 (Omega)" Omega's, each loading its own source.
`oag_title::GuestRoster::alongside_label` carries the second name (`Some` for
Omega's roster, `None` for 2048's HD twelve, where a real HD disc still wins).
Title names are plain values in this menu, so the label needs no `string_id`.
2048 vs Omega: **checked, differs** (models and textures), **checked, same**
(tuning).

## See also

- [`omega-frontend.md`](omega-frontend.md) - the front-end census this page's
  own findings extend: the GUI XML, the boot chain, the colour-skin switch,
  the 2048 campaign branch.
- [`psarc.md`](psarc.md#the-ps4-omega-collection-family) - the container-level
  block-data-location trap, and `data/README.md` for the extraction commands.
- [`hd-frontend.md`](hd-frontend.md) - the file this crate's own numbers are
  compared against, field by field.
