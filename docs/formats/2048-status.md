# Wipeout 2048 status

The [ADR-0009](../architecture/adr/0009-multi-game-fanout.md) probe run a third
title on: what the existing tooling reads off *Wipeout 2048* unchanged, what it
refuses, and what `oag-2048` was filled in from. Measured against
`data/extracted/vita/PCSF00007` - the decrypted EU package, base plus both DLC -
on 2026-08-26.

**The short version.** 2048 is Wipeout HD's asset tree on a little-endian
console. The container, the plugin XML, the handling XML and the `.vex` class
table all read with no code change at all. Three *binary* formats changed
underneath it, and **all three are now read** - the spline, the collision and
the render geometry, including its normal, its diffuse texture coordinate and
its material table and its per-submesh material binding (all added
2026-08-27, after this page's own last full pass). **A race now draws
textured**: the circuit paints 2,782 of its 2,800 draws and the craft all 16
of its own. Getting there needed two independent gaps closed on the same day,
neither a follow-on to the other - the submesh-to-material index
([2048-rcsmodel.md](2048-rcsmodel.md), confidence 90, measured across all
244,889 shipped submeshes rather than read out of the executable) and the
`PVRTII4BPP` pixel format almost every 2048 texture is stored in
([gxt.md](gxt.md), confidence 92, validated against Wipeout HD's own `.gtf`
copies of 2,284 shared textures).

## What reads unchanged

| Layer | Reads? | Notes |
| --- | --- | --- |
| [PSARC](psarc.md) container | **yes** | Version 1.4, flags 1, 18,430 entries in `PSP2/data.psarc`. HD's PS3 archives are version 1.3, flags 3; nothing asserts either. |
| [`.vex`](vex.md) class table | **yes** | Version 6, little-endian, the same class IDs HD's version 6 carries. `LodGroup` is authored in one file of the 1,059 (`DLC1/environments/Moa_Therma/track_reversed.vex`, 16 groups, every one childless), so the per-frame level-of-detail switch has nothing to do here - see [`vex.md`](vex.md), "implemented - the switch runs every frame". |
| [Handling stats](handling-stats.md) | **yes** | Both the HD-derived roster's per-team files and the shared `<Global>` block at `Data\XML\handlingstats.xml`. The native roster's five teams parse too, `SUPERPHANTOM` class included. |
| [`WO Track`](track.md) spline | **yes, after a version gate** | See below. |
| [Collision](2048-collision.md) | **yes, in a container of its own** | Not the `.vex` path: a `track_col.col` beside every `track.vex`. See below. |
| [Render geometry](2048-rcsmodel.md) | **positions, triangles, normals, diffuse UV, the material table, the per-submesh material binding, and the node table** | A `.rcsmodel` sharing HD's extension and no other part of its format. Draws textured, and since 2026-09-16 its node-bound meshes draw where their node puts them. `tangent`'s type nibble and the rest of section B's object graph are still unread. |
| [Scenery animation](2048-animation.md) | **yes, from two files of its own** | `track.vex` authors no `Anim Transform`; a `.rcsskeleton` and a `.rcsanimclip` beside the model carry the hierarchy and the keys. Read, wired, and checked against Wipeout HD's evaluator on the twelve shared circuits. |
| Plugin definitions | **yes, but split three ways** | `Data\Plugins\teams\`, `tracks\` and `music\` each ship their own `Definition.xml` where every other title ships one file carrying all three node kinds. `oag_title::Title::plugin_definition` names the teams one; `oag_title::Title::track_plugin_definition` is the axis that reaches the circuits one - `None` on every other title, `Some` here. The soundtrack list needs no axis of its own: `oag_2048::MUSIC.tracks` names `Data\Plugins\music\Definition.xml` directly (see the music bullet below). |

## What changed, and what state each is in

**`WO Track` is version `0x107` and its control point shrank from 112 bytes to
96 - recovered, and the parser is version-gated for it.** Everything up to and
including `racing_line` stayed exactly where it was; only `section_id` and
`flags` moved, from `+0x60`/`+0x61` to `+0x5c`/`+0x5d`. The evidence is a
**pairing against Wipeout HD**: 2048's DLC re-ships twelve HD circuits with
identical path and per-path control-point counts, so HD's fully decoded
`0x106` record is ground truth for 2048's, 10,165 control points of it. Full
numbers and confidences are in
[track.md](track.md#wipeout-2048-authors-version-0x107-and-the-control-point-shrank-to-96-bytes).
All fourteen circuits the base package ships now parse with
`AiTrack::encoded_len()` landing on the payload length to the byte.

**`.rcsmodel` is not HD's container - unread.** `Data\art\published\environments\altima\track.rcsmodel`
opens `ed ad 5c ca` where [HD's](rcsmodel.md) opens `0x000a0000`, and is 17.4 MiB
for one circuit. `RcsModel_Load` (`0x812f15b2`) is named and the outer
three-section shape is confirmed by disassembly *and* by the arithmetic closing
on the real file - `189824 + 682966 + 16573144 = 17445934`, exactly the file
length. All three sections' interiors are unread. See
[track-and-collision-loaders.md](../ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md).

**Collision moved out of the `.vex` into `track_col.col`, a k-d tree - and is
now read, whole.** `KdTree_Load` (`0x8118d134`) and `SimpleMesh_Load`
(`0x8118fac8`) between them account for every byte of all 26 shipped files, and
`TrackCollision_MeshFromNode` (`0x8126f800`) names six of the nine surface bytes
by switching on this project's own already-recovered `.vex` collision class IDs.
`oag_vex::kdcol` reads it. Full evidence, including the independent
170,744-triangle match against Wipeout HD's named collision classes and the
winding check, is in [2048-collision.md](2048-collision.md).

## What a race does today

`just play 2048 --race` opens the package, reads the spline, places the craft on
the start line and runs the simulation. What it does **not** do, and why:

- **The craft flies.** It hovers on the surface, is `grounded` every tick, and
  collides with the barriers - `track_col.col` is decoded and its 15,986
  triangles on `altima` become four colliders. With no steering input it drives
  until it meets a wall and stops there, which is what any title does.
- **The circuit and the craft draw, textured.** Altima comes up as 413,358
  triangles over 2,800 submeshes, 2,782 of them painted from 521 of its 527
  materials, and the craft as its own hull with all 16 draws painted from 5 of
  its 6 - livery, tech panels, plastics, light strips and canopy glass, each on
  the submeshes the file's own index names. Both lit off the file's own
  authored normals. 18 of the circuit's draws name a `.gxt` that does not
  resolve or will not decode and get no texture rather than a neighbour's. The
  screenshot is `data/shots/2048_altima_textured.png`. What is still absent
  from it is unrelated: `.envsettings` does not parse for this title, so the
  race lights off a stand-in rig, unfogged and without bloom. (`track.pvs` was
  listed here as not being HD's layout, so every chunk drew. It is that
  layout's little-endian dialect and culls since 2026-09-30, see
  [hd-pvs.md](hd-pvs.md); Altima at tick 300 goes from 616 draws and 145,779
  triangles to 447 and 98,032 with the same pixels, and four DLC1
  `track_reversed.pvs` files are refused as not their model's.)
  See [2048-rcsmodel.md](2048-rcsmodel.md) and [gxt.md](gxt.md).
- **The scenery moves, and the craft's airbrakes are on its tail** (since
  2026-09-16). `track.vex` authors no `Anim Transform`; what animates a 2048
  circuit is the `.rcsskeleton`/`.rcsanimclip` pair beside its model and a
  node table inside the model itself, all three now read
  ([2048-animation.md](2048-animation.md)) and validated node by node
  against Wipeout HD's own evaluator on the twelve circuits both titles
  ship. `altima` runs 113 tracks over 165 nodes - river boats, balloons,
  wind-turbine rotors, a crane, pigeons - and the 130 meshes bound to those
  nodes, which drew a median 1,086 units from their place before the table
  was read, are placed and moving. The load report says so:
  `165 skeleton node(s), 113 animated over 113 track(s), 166.7 s loop`.
  **2026-10-05**: a Zone race can draw `trackZone.rcsmodel` (behind `race::Options::zone_model`, off by default while its road shader `fc01_dummy` is unread) through its own
  skeleton and 30 Hz clip, its Zone colours are read off the material's
  uniforms, and the glow-layer and `speed_multipliaer` scrolls play off the
  same table ([2048-material-params.md](2048-material-params.md)). Not wired:
  the start-grid animation (a pit-bot rig 110 units ahead of the grid; what
  triggers it is not read, [2048-animation.md](2048-animation.md)).
- **Resolved 2026-09-20: `oag_2048::TITLE.front_end` is `Some` -
  [ADR-0054](../architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md)
  widened `oag_title::FrontEnd::menu` to `Option` and added a second,
  touch-icon axis (`FrontEnd::touch`) rather than waiting on a `MenuSkin`
  this title was never going to author.** The boot chain (`Boot Connect` ->
  `Boot Studio Logo` -> `Boot Intro Movie` -> `Load Save Bootup` ->
  `TitleScreen`) is wired as `Provenance::Declared`, the seventeen language
  plugins are wired, and the touch-icon layout (`GameModeChoice`'s four
  buttons, `Home`'s five, the team screen's ship-model origin, the
  `FE3DCanvas` screen and atlas names) is wired as `FrontEnd::touch`.
  `FrontEnd::menu` itself stays `None`, on the same evidence as before: no
  `<FEGlobals>` block, no `<Menu>`/`<HorizMenu>` widget anywhere. The
  practical effect: `oag-game`'s menu-driven boot (`load_shell`, what
  `--screenshot` with no `--race` goes through) now refuses by a narrower,
  more accurate name - "draws no MenuSkin-shaped menu ... race on it with
  `--race` instead" - where it used to refuse for "no front end at all."
  `loading` stays `None` on the narrower gap this page already named: a
  real percentage-bar loading screen with no located plugin XML.
  **`music` is `Some` since 2026-09-25**: `frontend.bnk` turned out to
  carry no 16-byte name table (its names are FNV-1 hashes, [2048-xfx.md](2048-xfx.md#sblk-version-5-banks-names-are-hashes-95)),
  so what fills `Music::front_end` instead is
  a standalone RIFF-wrapped ATRAC9 file, `FEMusic/frontend_stereo.at9`,
  named the same way Pulse's and HD's own front-end tracks are - see
  [2048-frontend.md](2048-frontend.md)'s "frontend.bnk" bullet for the
  full account, including the new `oag_music::at9` decoder and the
  `MusicDiscs::survey`/`pick` fix that was also needed for a Vita boot to
  reach it at all.
- **The race soundtrack plays since 2026-10-05.** `data/plugins/music/Definition.xml`
  declares eleven `PI_Music` nodes (`01`..`11`, `location="data\audio\music\NN"`);
  each directory holds `music_stereo.at9` (RIFF-wrapped ATRAC9, 48 kHz stereo,
  about 301 s for `01`) and an `.fft` sidecar nothing reads. `oag_2048::MUSIC.tracks`
  is `Some(DeclaredTracks { music\Definition.xml, "music_stereo.at9" })`, so the
  same declared route Pure and HD use lists them in file order. Two gaps stopped it
  reaching a race: `Soundtrack::read` had no `Platform::Vita` arm, and
  `music::riff_seconds` accepted only 44.1 kHz (ATRAC9 is told apart by its
  subformat GUID rather than by widening the PSP rate test). The front-end loop
  did play all along (302 s, decoded from `FEMusic`), but only once the boot's
  100 s intro movie ends, so a capture shorter than that is silent under it.
  **Playlist rule - chosen, not measured:** the title-agnostic one, track 01 first,
  then in declared order, wrapping, with the position kept across races. The
  original's order, shuffle and selection rule are unread. `Artist` and `Label` are
  not shown: neither Pulse nor HD shows them. Evidence: `--race --dump-audio` WAV,
  race music RMS 0.16 in the mixer against raw decoded track level 0.36 (front end
  raw 0.22), and `vita_2048_music_ground_truth.rs`.
- **Zone's announcer is wired, off a bank path that is read rather than
  measured.** The executable's own track-construction function decompiles to
  a real dispatch between two live Zone speech banks, gated on the selected
  circuit's own pack id; a base-package circuit like `DEFAULT_TRACK` reads as
  taking `data/audio/sound/speech_zone_NGP.bnk`, and `zone_announcer` uses it
  with the same fifteen-entry ladder Wipeout HD's own `speech_zone.bnk`
  carries. **Not yet verified against real audio** - this title's
  `data.psarc` is not extracted in this tree, so unlike the other three
  titles nothing here has run `oag-wad sounds` against the actual bank - and
  the milestone table's own reading function was swept for and not found,
  despite every direct reader of the Zone speech handle being checked. See
  [zone-audio.md](../ghidra/functions/vita-2048-eu-v104/zone-audio.md).
- **The HUD draws its four always-on sprites where the original has them,
  and nothing state-gated yet.** All 25 non-split-screen HUD roots compose
  with nothing missing and nothing skipped, the played skin is known
  (`2048_hud\`), the reticle is `Sights::Concentric`, `oag_texture::gxt`
  reads the textures - and, since 2026-09-16, **the title has been run**:
  Vita3K on a headless weston/Xwayland stack
  ([vita3k-capture.md](../reverse-engineering/vita3k-capture.md)) drove the
  EU v1.04 build through three campaign races, a time trial and a Zone run,
  and `ALWAYS_ON` is the intersection of what those frames show. The same
  frames measured the 960x544 coordinate space (`Space::VITA`; before it a
  Vita source fell through to the PSP's grid and drew the HUD at twice its
  size with the right-hand column off screen). `just play 2048 --race --mode
  single_race` now shows the shield silhouette and the speed readout at the
  original's own positions; the shield fill, the pickup slot, the speed
  fills, `PilotAssist` and Zone's lit dashes are read as state-gated and not
  wired.
  **Resolved 2026-09-21: the HUD text now draws in `2048_hud.fnt`, not the
  5x7 fallback.** The cause was exactly what the 2026-09-20 entry below left
  open: `crates/raceplay/src/hud.rs`'s `hud_font` asked every source for the
  literal role name `"HUD"`/`"HUDSmall"` (`oag_ui::language::roles`), which
  is Pulse's, Pure's and HD's own spelling and not 2048's - every one of
  2048's 17 plugins names its real HUD face `2048HUD` instead, and two of the
  seventeen (`korean`, `traditionalchinese`) carry a leftover `HUD`/
  `HUDSmall` role pointing at files 2048 does not ship, which the old
  first-match search picked up regardless of the player's own language. Fixed
  by giving `oag_title::HudArt` a per-title axis -
  `hud_font_role`/`hud_small_font_role` - so `hud_font` asks each title for
  the role its own plugins actually name: `"HUD"`/`Some("HUDSmall")` for
  Pulse, Pure and HD (measured, unchanged from before), `"2048HUD"` for 2048.
  2048's own plugins name no distinct caption role at all - confirmed across
  all seventeen, a real gap rather than an oversight - so
  `hud_small_font_role` is `None` there and `hud_font` falls back to the
  value face for captions too. **Measured, not merely chosen**: composing
  every root the played skin (`oag_2048::hud::skins::PLAYED`) actually reads
  finds zero `font="HUDSmall"` widgets in any of them - every label is
  `font="HUD"`, and the visible caption/value size split (`LapTxt` at
  `scale=0.6` beside `Laps` at `scale=1.0`) comes entirely from each
  widget's own authored `scale` on that one face, not a second `.fnt` this
  pass failed to find. `font="HUDSmall"` exists in this archive, 261
  widgets' worth, only in the three skins this title's race-manager
  constructors never read. Verified with
  `just play 2048 --race --screenshot ...`: the boot report now reads `HUD
  font Data\XML\2048_hud\font\2048_hud.fnt (role "2048HUD")` and the
  screenshot shows the same clean sans-serif face the real Vita3K captures
  in [2048-hud.md](2048-hud.md) do, not blocky 5x7 glyphs. Pinned by
  `crates/game/tests/vita_2048_hud_ground_truth.rs`'s
  `the_resolved_hud_font_is_2048_huds_own_face_not_a_leftover_or_the_fallback`.
  See [2048-frontend.md](2048-frontend.md#the-language-plugins-carry-a-hud-font-role-too).
- **2026-09-20: the HUD font role was located but did not yet draw.** Wiring
  `front_end` (above) makes the seventeen language plugins reachable, and a
  `--race` capture logs `17 language(s): American ..., ...` where it used to
  log none at all - but the literal-role mismatch described above meant the
  text still drew as raw `IG_HUD_*` ids in the 5x7 fallback, and on
  `korean`'s dangling entry rather than a clean "unavailable" besides. Closing
  this needed a new axis (a per-title HUD font role name, and 2048 also names
  no distinct "small" face for `HUDSmall` to fall back to, which is a gap of
  its own rather than one this pass could fill without inventing one) - out
  of scope for [ADR-0054](../architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md),
  which only wired the front end far enough to expose this. Closed the
  following day - see the entry above.
- **2026-09-21: `--race --event "<name>"` launches a real campaign event.**
  2048's own campaign is not a racebox grid at all - it is `Data\xml\SP.xml`,
  a "mjolnir" typed-instance database, 288 instances across eight typedefs,
  five named in-file by their own `type=`/`typedefid=` pairs and four
  classified by measured field shape. `oag_tables::mjolnir` reads it,
  `oag_2048::campaign` names 2048's own facts (its five speed classes
  including `SUPERPHANTOM`, the `laps == 0` Speed Lap sentinel, the mode each
  event kind maps onto), and `race::load_event` resolves an event onto
  `race::Options` before the existing loader runs. `--event "2048 - Event 3"`
  loads Park/Flash/2 laps/`single_race` with a full AI grid; an
  `Elimination`-kind event loads too, since `oag_race::Mode::Eliminator` turns
  out to be a real, implemented mode with this title's own
  `weaponstats_Elimination_2048.xml` already wired - not the unsupported mode
  this feature was built expecting to have to refuse. See
  [2048-campaign.md](2048-campaign.md).

**Airbrake flaps swing** (2026-10-05, `airbrake-flaps`). A node-bound mesh named `Airbrake_Left`/`Airbrake_Right` is baked through its node's bind matrix, and `mesh::rcs::psp2::build` records that matrix as the hinge. `qirex2048/2` authors its flaps at the nose (z +5.5) and they rise without flaring.
The swing is the same `Flap::deflect` Pulse uses (`hinge * Rx(angle) * hinge^-1`,
about the hinge frame's local X), scaled by the title's own `<AirbrakeGraphics>`
`amount` and rates through `RaceView::airbrake_flaps`. **Which way it turns is
checked, not read:** the title's own `Airbrake` handler is unread, so
`psp2_airbrake_flaps_ground_truth` asserts the physical claim Pulse's recovered axis makes (a positive
deflection raises the flap and flares it outward, both sides) on all 20 native craft and the 12 HD-derived hulls. Only
the player's craft swings, as on Pulse; a rival's flaps stay stowed.
Frames: `data/scratch/airbrake-flaps/` (`2048_*.png, k_2048_*.png, kcs_2048.png`).

## The one axis 2048 forced into existence

`oag_title::RaceDefaults::ship_dir`. Pulse, Pure and Wipeout HD all keep their
rosters at `Data\Ships\<Team>\`, and `oag_title::race`'s own docs argued that
made it shared vocabulary rather than a title fact. 2048 disagrees: its twelve
HD-derived teams are at `Data\art\published\hdships\<Team>\`, carrying the same
file set under the same names. That is the third-title disagreement
[ADR-0022](../architecture/adr/0022-title-packages.md) asks for.

**Its own five teams needed a *second* axis besides `ship_dir`.**
`ag_systems2048`, `auricom2048`, `feisar2048`, `piranha2048` and `qirex2048`
keep their models under `Data\art\published\Ships\<team>\<1..4>\` and their
tuning under `Data\HandlingStats\<team>\<1..4>\` - two trees, where the
HD-derived roster keeps both together. So `RaceDefaults` carries `ship_dir`
*and* `handling_dir`, and the team id is two levels (`feisar2048\3`).

**The numbered level is the four craft each team flies**, and the disc names
them: `qirex2048\1\Textures_1\Qirex_Fighter_Livery.gxt`, then `_Agility_`,
`_Speed_` and `_Prototype_` in order, corroborated by the front end's own
`locked_Feisar2048_speed.gxt` and its three siblings. Feisar spells the first
`Combat` where Qirex spells it `Fighter`. Confidence 90.

**The HD-derived roster is reachable too, through a third axis.** One
`ship_dir` string cannot address both trees, so `oag_title::GuestRoster`
(`RaceDefaults::guest_roster`) is a *second* roster a title can carry beside
its own, naming a directory of its own and the twelve team ids and `_c1`/`_n1`
suffixes that resolve under it - `RaceDefaults::ships_for`/`handling_dir_for`
route an already-combined id (`Assegai_c1`) to whichever tree it belongs to.
See [ADR-0035](../architecture/adr/0035-a-craft-pick-may-fall-back-to-a-title-that-reships-the-same-roster.md).

## Particles - 2026-10-05

A 2048 race plays `Data\Particles2048` effects with their own `.gxt` sprites:
27 of the 36 wired names load (the 9 that do not are listed in
`psys_inventory_ground_truth.rs`'s `v2048` module). Census, directory choice and
what is not wired: [pob.md](pob.md), "Wipeout 2048 carries the same container".
