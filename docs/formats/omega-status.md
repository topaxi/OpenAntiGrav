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
the widget's own `.gtf`-spelled key (`crates/game/src/sprite.rs::Image::decode_gnf`
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
`crates/game/src/race/hud.rs`, reached only once a race has already started
loading).

## The menu backdrop (2026-09-30, `omega-menu-backdrop`)

Omega's menus now sit on the HD-style `<BackgroundAnim>` scene, which is the only backdrop
Omega can have: no `.points2` clouds ship in any of the nine archives. The scene is
`FrontEndScene_HD_ATG.vex` with its PS4 `.rcsmodel` (byte-identical in `data00` and `data08`);
the `*_VR` scenes are not drawn. The `.vex` carries the motion and the camera (60 s loop, 2,274
translation keys on `camera1`), the `.rcsmodel` the geometry, and the two bind by shape name -
`oag_render::mesh::rcs::psp2::build_with_vex`. The picture is the scene drawn on white and filtered by
`FEBackgroundAnim_fp` into a grey line drawing; everything is read in
[`menu-backdrop-scene.md`](../ghidra/functions/ps3-hdfury-eu/menu-backdrop-scene.md).
`--menu-page main` (settled `Main Menu` row: edge 0.6, fill 0.2, width 0.5, no blur) and any other
page (the `default` row: 0.3, 0.1, 1.5, blur 3) draw it, and `--anim-seconds` picks the moment. **The
boot screens do not** - they are outside `Top FE Screen` in the skin, so the original has none there
either. **Not validated against the original**: no PS4 emulator exists and HD's one capture of its HD
style shows a flat white page. Chosen, not measured: the blur kernel, the linear sampling, and which
page takes which row. The `--menu-page grid-select`/`cell-select` stills draw it too (they take the
page's picture); the live campaign stage and the end-of-race screens sit under the
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
  campaign still and live stage dispatched `title == HD` to `oag_ui::campaign::hd`
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
- **The menu block art** (`MenuSkin::blocks`) - deliberately left `None` (trying HD's
  `MENU_BLOCKS` on Omega on 2026-09-30 put HD's geometry squarely on Omega's own `file2.gtf`
  outline, but the fill swatch HD's loader samples reads alpha 0.004 on Omega's copy - cause
  unread - so the rows came out as outlines with no fill and the white text still
  invisible).
  **Its consequence, fixed 2026-09-30:** with no block behind them, Omega's option rows were
  white (`FEGlobals->TextColor`) on a white page (`HD_BG`). Where a row's text cannot be
  read on the page the frame clears to (`menu::rows::text_is_lost_on_page`, lightness within
  a quarter), each row and value now sits on a box in the frame's own authored `HD_Grey`
  (`HD_Blue` selected), the way the strip's tabs are filled when there is no block art. The
  box's size and the lightness threshold are **chosen, not measured**; the colours are
  authored. Pulse, Pure and HD captures of `options`/`graphics` are byte-identical before
  and after (their text is light on a dark page).
  HD's own `MenuBlocks` numbers are read out of `EBOOT.elf` at named
  addresses, nothing authored in any XML, and nobody has disassembled
  Omega's PS4 executable; pointing at HD's table would assert an unmeasured
  equivalence between two different binaries. The menu strip and list
  positions (`MenuStrip`/`MenuList`) **are** authored and independently
  re-derived off Omega's own `mainmenu_definition.xml`/`additional_definition.xml`.
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
  `Data\Ships\<Team>\FE\Logo.gtf` (`oag_ui::picker::hd::logo_src`), the stat
  blocks do not draw, and `hdships\<Team>\screen.xml` has no slideshow chain.
  A cell therefore races whichever team the RACE page holds (`settings.race.team`;
  `Session::apply_race_team` copies it in): slot 0 was `hdships\Assegai` on the walk,
  the first entry of the catalogue, which is where `settle` falls when the stored
  id is not offered (the catalogue lists `AG_Systems`; `DEFAULTS.team` spells it
  `ag_systems`). **Chosen, not measured.**
- **The EndRace screens are skipped, not wired.** `EndRace_Definition.xml` is at
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
   `.final` one only when it is absent (`oag_render::mesh::rcs::sibling_name_cooked`),
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
(`race::TextureSink`, `oag_render::mesh_render::TextureSinkScope`,
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
  (`omega-catchup`, 2026-09-30). The atlas is each material's `lightmap`
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
- **`.EnvSettings` is read through 2048's reader** and its sun (`[4.00, 2.33,
  0.82]` over an ambient of `1.0`) is not checked against PS4's schema. The
  patch's copy carries no HDR/bloom block, so the read bloom chain is off.
- **Sound is read, not played** (`omega-pvs-sound`, 2026-09-30). The banks are
  **Audiokinetic Wwise**, bank generator version 118, not the PSP's `SBlk`, so
  `sfx: <name> not loaded: unsupported bank version 1145588546` is still what a
  race prints: nothing wires Wwise events to this project's cues.
  [`wwise.md`](wwise.md) has the container, the event-to-media chain (0
  unresolved media over `data00` and `data08`) and the `.wem`: **every one is
  Sony's ATRAC9**, mono and stereo decode in process and agree with FFmpeg to
  one LSB up to polarity, four-, six- and eight-channel files do not decode yet.
  The ten `.bnk` in `data02`, which an extension-based census had counted
  as Wwise, are the old `SBlk` container. One Tech De Ra cue plays through the mixer to a WAV.
  Still silent: `Data\Psys\*.POB` particle effects are not in this archive set
  under those names; the blob shadow is this project's generated falloff, not the
  disc's.
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

## See also

- [`omega-frontend.md`](omega-frontend.md) - the front-end census this page's
  own findings extend: the GUI XML, the boot chain, the colour-skin switch,
  the 2048 campaign branch.
- [`psarc.md`](psarc.md#the-ps4-omega-collection-family) - the container-level
  block-data-location trap, and `data/README.md` for the extraction commands.
- [`hd-frontend.md`](hd-frontend.md) - the file this crate's own numbers are
  compared against, field by field.
