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
**not** carried forward at all: the circuit files are a mix of plain `.vex`
and an unread `.final.*` family. Front-end images are Sony's PS4 `.gnf`
container, which [`gnf.md`](gnf.md) now decodes for most of the sprite sheet
(219 of 289 front-end/campaign `.gnf` files draw). Racing is out of scope
for this crate; see [`omega-frontend.md`](omega-frontend.md) for the frontend
census this page builds on.

## What reads unchanged

| Layer | Reads? | Notes |
| --- | --- | --- |
| [PSARC](psarc.md) container | **yes** | Nine archives across the base package and the patch; version and flags not diffed against HD's own PS3 copies this lane. |
| Front-end XML (`Screens::from_xml`) | **yes** | `skin.xml`, `mainmenu_definition.xml`, `cellmode_definition.xml`, `additional_definition.xml` all parse with the same `oag_ui::screen::Screens` reader HD and Pulse/Pure use - confirmed both by `crates/game/examples/omega_frontend_probe.rs` (docs-only sweep) and by this lane's own boot (`Data\Plugins\Frontend\Gui\Skin.xml: 27 screens, 74 globals, 24 LoadXML includes`). |
| `FEGlobals` menu layout | **yes, bit-identical to HD's** | Ten of ten authored globals match HD's own `skin.xml` to the digit, re-derived directly against `data09.psarc` in `crates/omega/src/frontend.rs` rather than copied - see that module and its ground-truth test. |
| Boot chain shape | **declared, not measured** | Same eight redirects in the same order as HD's `DATA00`/`DATA05`/`DATA06` family, no dead `LogoFMV`. `Provenance::Declared`: no PS4 emulator exists in this project's toolchain to upgrade it the way HD's RPCS3 capture did. |
| Campaign schema (`CellMode_Definition.xml`) | **yes, HD's own schema** | `Grid Selection`/`Cell Selection` as `FlyerSelection`/`CellSelection` screens, plain UTF-8 (not Pulse's dictionary-shortened copy). Nineteen grids where HD has sixteen (`oag_omega::campaign::GRID_COUNT`), confirmed by direct listing. Twelve of nineteen parse this lane; the rest fail per-row the same tolerant way a bad HD grid already does (`docs/formats/hd-frontend.md`'s note on `grid_04.xml`'s own broken tag). |
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

**Circuits are a mix of plain `.vex` and an unread `.final.*` family, per
environment, not title-wide.** `Data\environments\talons_junction\` ships
`TrackStartup.xml`, `track.final.audio` and `track.final.rcsskeleton` in
place of a `track.vex`; `Data\environments\tech_de_ra\`, `zone_4\` and the
four `environments2048\*` circuits (`sol`, `square`, `mall`, `subway`,
`cathedral`) do ship a plain `track.vex`. Which environments are which was
not surveyed exhaustively - `tech_de_ra\track.vex` is confirmed present and
is `oag_omega::race::DEFAULTS.track`'s value, chosen for existing rather than
measured as any kind of default. **It is confirmed to have no readable `WO
Track` node** - a clean negative result, not a gap: `oag_render::track`'s own
reader refused it outright (`Data\environments\tech_de_ra\track.vex has no
WO Track node`), hit directly by the headless-capture crash below. Whether
that is this one file, or every plain-`.vex` circuit on this title, was not
surveyed past this one instance.

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

## What a menu-page capture does today

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
- **The menu block art** (`MenuSkin::blocks`) - deliberately left `None`.
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

## Racing: out of scope, and why nothing crashes

No CLI path in this lane starts a race. `oag_title::RaceDefaults`'s otherwise
-mandatory fields are real, archive-confirmed placeholders documented as
chosen-not-measured (`crates/omega/src/race.rs`); the interactive session's
own generic recovery (`Session::finish_race_loading`) reports a failed race
load by name and returns to the menu rather than crashing, for every title,
not something added for Omega.

**One crash this lane did find, and left alone.** `--screenshot --press
start,cross` (any title) skips the language picker and drives straight
through to a `Launch Game` hand-off; the headless capture path
(`crates/game/capture.rs`) always races once the front end finishes, with no
interactive menu to stop at - so on Omega it attempted
`oag_omega::race::DEFAULTS.track` (`tech_de_ra\track.vex`) and the process
exited with `Error: Data\environments\tech_de_ra\track.vex has no WO Track
node` rather than reporting and stopping. This is a property of the headless
capture path shared by every title, not something Omega's own data
introduced - the brief for this lane says not to chase it, and it was not.

## See also

- [`omega-frontend.md`](omega-frontend.md) - the front-end census this page's
  own findings extend: the GUI XML, the boot chain, the colour-skin switch,
  the 2048 campaign branch.
- [`psarc.md`](psarc.md#the-ps4-omega-collection-family) - the container-level
  block-data-location trap, and `data/README.md` for the extraction commands.
- [`hd-frontend.md`](hd-frontend.md) - the file this crate's own numbers are
  compared against, field by field.
