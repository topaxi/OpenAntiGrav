# Omega's front end is HD's `PI001` plugin, carried forward - and load order is unresolved

2026-09-21. A player-level sweep ("Omega's menu looks like HD/Fury's"),
answered with a byte-level diff:
[`docs/formats/omega-frontend.md`](../../docs/formats/omega-frontend.md).
`Data\Plugins\PI001\GUI\skin.xml`'s `FEGlobals` block is bit-identical to
HD's own headline table (ten of ten authored globals match to the digit),
the boot chain is declared identically (no dead `LogoFMV`, same eight
screens), and this project's existing `oag_ui::screen::Screens` reader
parses Omega's `skin.xml`/`mainmenu_definition.xml`/`cellmode_definition.xml`
with zero code changes
([`crates/game/examples/omega_frontend_probe.rs`](../../crates/game/examples/omega_frontend_probe.rs)).
2048's campaign is neither purely re-authored into HD's vocabulary nor kept
as 2048's own: the screen *shape* (`Campaign Selection` -> `Grid Selection
2048` -> `Cell Selection 2048`) is HD's `FlyerSelection`/`CellSelection`
types extended by one branch, but the map *content* one level in
(`Campaign2048_Definition.xml`, newly authored, `<LoadXML>`'d as a sibling of
`CellMode_Definition.xml`) is 2048's own `FE3DCanvas`/`CanvasLabel`/
`linkedevent` vocabulary - unreadable past its own corrupted prefix in the
original extraction this thread was opened against, **now fully readable**
after `lane/omega-psarc` (2026-09-27) fixed the extraction-tool bug behind
that corruption; see the "Open"/"Next Steps" updates below.

Prior art this sits beside without duplicating:
[`omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md`](../tooling/omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md)
already census'd the patch's four archives at the file-listing level; this
thread opened the front-end files that census only named and diffed their
content against `hd-frontend.md`.

## Open

- **Maintainer observation, 2026-10-06 (authoritative): the Omega front end
  is in a state HD/Fury's was in earlier** ("the menu items do not render
  correctly and they do not animate yet"). **Main menu and settings rows
  closed 2026-10-06 (`omega-frontend`)**, see `docs/formats/omega-status.md`,
  "The menu block art": Omega's executable-drawn `.gnf` art keeps HD's
  bottom-up row order, so the block nine-patch was never drawn (and the strip
  took the unanimated measured-tab path). Blocks, eased focus and the
  underline mark now draw on HD's table, **chosen, not measured** for Omega;
  reference is HD on this engine, no PS4 emulator. What is still different
  from HD, drawn from `--menu-page grid-select`/`cell-select` side by side: no flyer card, five hexagons
  where HD draws six (a different authored cluster, unchecked), circuit
  names as ids, the three medal icons in the wrong order and no `TARGET
  (NOVICE)` line. Also unverified: the strip's pointer hit regions on the
  new geometry, the `EndRace Menu` option blocks (the old "draws no blocks"
  cause may be this one; `--menu-page endrace-menu` still says the source has
  no EndRace screens), and 971 Omega `.gnf` with no HD file of the stem, whose
  row order is unchecked.

- **Load order between the patch's own four archives is not settled, but
  both files this bullet used to call unreadable now read clean everywhere,
  and one of them narrows the question to two candidates instead of three.**
  `lane/omega-psarc` (2026-09-27) found and fixed the extraction-tool bug
  that was corrupting both copies - see the resolved bullet below - and a
  corrected re-extraction also
  surfaced a **third** copy of both files that the old, corrupted manifest
  never even listed as a candidate: `data08.psarc` carries its own
  `cellmode_definition.xml` and `campaign2048_definition.xml`, previously
  invisible entirely. `campaign2048_definition.xml` is now **byte-identical**
  between `data08` and `data09` (both 567 lines, whole-file `diff` clean),
  so between those two it no longer matters which one loads.
  `cellmode_definition.xml` is where the real difference sits:
  `data08`/`data09`'s copies are byte-identical to each other (1,265 lines)
  but **genuinely differ from `data07`'s** (also 1,265 lines, not
  corruption) - `data07`'s uses a scalar `orthoScale` and no
  `IsHidden`/`IsCallbackChk` attributes; `data08`/`data09`'s use vector
  `orthoScaleX/Y/Z` and add both flags, a newer authoring revision. So the
  open question is now binary (`data07`'s older revision vs. `data08`/`data09`'s
  newer, identical one), not three-way, and `data09` - highest-numbered, per
  a numeric-order-wins patch-layering guess - remains the same circumstantial
  favourite `hd-frontend.md`'s own history warns against trusting before a
  runtime capture (its `DATA00` looked like the newest layer for the same
  reasons, and "looking like it is not evidence" was the finding right up
  until RPCS3 settled it with a `TTY.log` load-order printf). No PS4 emulator
  exists in this project's toolchain to do the equivalent capture.
- **`vita_legacy.xml`'s role is a hypothesis, not a finding** (confidence
  50). It is confirmed live - `skin.xml`'s own `<LoadXML>` list includes it
  unconditionally - and it is real 2048-shaped content (`FE3DCanvas`,
  `GameModeChoice`-targeted redirects, a `TouchScroll` widget named
  `overshell`), but it is not reachable from `Main Menu`'s own redirect, so
  "companion overlay or debug surface, not the primary campaign flow" is a
  guess from adjacency, not a traced path.
- **`StudioLiverpool_fury.bik` does not appear in any of the nine archives'
  path listings** - whether Omega unified the Fury/base movie cuts, dropped
  one, or renamed it is not checked.
- **`Controls_Definition_PS4.xml` vs `_ps3.xml`** both read clean and both
  exist in `data09`; only the PS4 one's `<LoadXML>` site was found. Not
  diffed field-by-field.

## Next Steps

- ~~An `oag-omega` title crate is the real unlock for any of this landing as
  code~~ - landed 2026-09-21 (`crates/omega`,
  [`omega-status.md`](../../docs/formats/omega-status.md)). `just play omega`
  boots and stops on `Language Selection`; `--menu-page` draws this build's
  own main menu and the campaign grid/cell screens (nineteen grids, twelve
  parse). What that opened rather than closed:
  - ~~A `.gnf` reader is the single highest-value follow-on now~~ - landed
    2026-09-27 (`oag_texture::gnf`, [`gnf.md`](../../docs/formats/gnf.md)):
    container, BC7 and the micro-tile address formula all decode, guarded by
    an `Error::CorruptBlocks` refusal for a base level with genuinely
    missing PSARC-level content (219 of 289 front-end/campaign `.gnf` files
    draw clean). Wired into `crates/hud/src/sprite.rs`,
    `crates/game/src/boot/sprites.rs::gnf_sibling` and
    `crates/game/src/campaign.rs::load_omega`. What that opened rather than
    closed: this lane's own two-archive extraction is missing five of the
    campaign's own hex texture names outright (`Hexagon_HD_OUTLINE.gtf` and
    four siblings - no `.gnf` sibling either, not an unread name) and most
    of the front end's small *boot-time* image set (`saveIcons.gnf` is
    present but corrupt, `line.gnf` fails its own magic check) -
    `omega-status.md`'s "What did not draw" section names each one. **Update
    2026-09-27, and every one of these recovers:** this "missing"/"corrupt"
    content was this project's own extraction, not the package - see the
    resolved bullet two below. Checked by name against the corrected
    re-extraction: all five hex
    texture names (`hexagon_hd_outline.gnf`, `hexagon_hd.gnf`,
    `hexlock_hd.gnf`, `hexagon_hd_thick_out.gnf`,
    `nonselectable_arrow_hd.gnf`) now have real entries in `data00.psarc`
    (duplicated in the patch's `data08.psarc` - both invisible before, since
    the old, corrupted manifest never surfaced them as candidates at all,
    not just their content) and decode clean through this project's own
    unmodified `Texture::decode`; so do `saveicons.gnf` (257x129),
    `line.gnf` (8x8), `hexmedal_hd.gnf` (976x549) and `vr_headset.gnf`
    (447x370, `data08.psarc` - previously "no entry at all", also a manifest
    casualty). `StudioLiverpool.bik` still does not appear under that name
    in any of the nine archives even in the corrected extraction - genuinely
    absent, not a reading artefact.
  - ~~`crates/game/src/campaign.rs::load_omega` reads its hex textures by the
    literal HD path rather than through the front end's own
    `gnf_sibling_report`~~ - landed in the same change: it now falls back to
    `crate::boot::sprites::gnf_sibling` (renamed, and reused rather than
    front-end-only) the same way the sprite sheet does.
  - Racing stays fully out of scope: `oag_omega::race::DEFAULTS` is
    real-but-unread placeholders, and whether `tech_de_ra\track.vex` (the
    chosen default) even parses as a `WO Track` node was not checked past a
    headless-capture crash that lane found and left alone (`omega-status.md`'s
    "Racing: out of scope" section) - a title-wide capture-path gap, not
    Omega-specific.
- If a PS4 emulator is ever integrated into this project's tooling (`oag-trace`
  currently covers PCSX2/PPSSPP/RPCS3 only), a boot capture would settle load
  order and the boot chain's `Provenance` in the same pass RPCS3 did for HD -
  see [ADR-0025](../../docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md).
- ~~Chasing `campaign2048_definition.xml`'s corrupted prefix is worth
  revisiting if `oag_formats::psarc`'s block-data-location understanding
  advances for other reasons~~ - landed 2026-09-27, exactly this way and not
  Omega-specific: `lane/omega-psarc` root-caused and fixed the extraction
  tool bug behind the whole "garbage"/"all-zero" population
  (`docs/formats/gnf.md`'s "Root cause" section), and
  `campaign2048_definition.xml` reads clean from byte zero in the corrected
  extraction - see "Open" above for its content (`<Screen type="Campaign2048"
  name="Cell Selection 2048">`, `FE3DCanvas`/`CanvasLabel` widgets intact).
  Turning that into a real campaign-map render is `campaign-map`/2048-lane
  work, not started here (PSARC/GNF only, per this lane's own boundary).

- **2026-09-30, `omega-campaign-launch`: a confirmed campaign cell starts its
  race** (see `docs/formats/omega-status.md`, "Campaign: confirming a cell
  starts its race"). What is left, in the order a player meets it:
  1. **`Team Selection` draws an empty frame on Omega**, so
     `FRONT_END.team_select` stays `None` and a cell races the RACE page's team
     (`settings.race.team`: `Assegai` on the walk, the catalogue's first entry).
     Logos are at `hdships\<Team>\FE\Logo.gnf` (HD's reader asks
     `Data\Ships\<Team>\FE\Logo.gtf`), the stat blocks do not draw,
     `screen.xml` has no slideshow chain. Fix the logo path off the title's
     ship dir, find why the blocks do not draw, then set `team_select`.
  2. **The EndRace screens stay unwired**: `EndRace Menu` draws no option
     blocks through HD's loader, so `RETURN TO GRID` is unreachable and a
     finished race returns to `Main Menu`. Set `endrace_entry` and dispatch
     Omega in `oag_game::endrace::load`, `session::endrace` and
     `capture::endrace_page` (all three test `title.name == oag_hd::TITLE.name`)
     once the blocks draw.
  3. **Omega has no `Campaign Selection` and no flyer cards**
     (`load_omega`: `selection_layout`, `grid_layout_fury`, `flyers` all
     `None`), and `Grid Selection` draws on a white page. Only the
     cards and the selection screen are HD-only; the grid, cell and launch
     code is shared.
  4. Grids 16-18 have no `RequiredPoints` attribute and are skipped; their
     cells never reach the screen.
  5. **2026-09-30, `omega-frontend-fixes`: the "hexagons do not start a race" report was the mouse.**
     Hit regions were the 128x64 sprite texture, not the 72x62 hexagon, so a hover landed on a
     neighbour; fixed and held by `campaign_pointer_ground_truth` (Omega and HD). Also closed: the
     invisible click target on Omega's `Grid Selection`, and a refused `NitroBattle`/`Detonator`
     cell now says so on screen. **Still unwalked:** a fresh profile has exactly two playable cells
     (`grid0_3_1`, `grid0_3_2`, the only `Locked="false"` ones on an unlocked grid); everything else
     opens by medal, so finishing a cell race and seeing a neighbour unlock on Omega has not been
     walked (with the EndRace screens unwired, the built-in results table is what ends the race).
     The maintainer's own `records.toml` holds no Omega medal, so nobody has yet.

## From the HANDOVER.md index (moved 2026-09-25)

swept 2026-09-21 from a player observation ("the Omega menu looks like HD/Fury's"); `FEGlobals` matches HD's to the digit and this project's own screen reader parses it unchanged, but which of the patch's four archives the runtime actually loads is the one open question HD's own six-copy version needed an RPCS3 capture to close
