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
`linkedevent` vocabulary, unreadable past its own corrupted prefix in this
extraction.

Prior art this sits beside without duplicating:
[`omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md`](../tooling/omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md)
already census'd the patch's four archives at the file-listing level; this
thread opened the front-end files that census only named and diffed their
content against `hd-frontend.md`.

## Open

- **Load order between the patch's own four archives is not settled.**
  `cellmode_definition.xml` exists in both `data07.psarc` (garbage - real
  text recovered from byte 35,404 of 69,013, and it disagrees with the other
  copy past the corruption) and `data09.psarc` (clean from byte zero, and the
  one with the complete plugin set). `data09` is the circumstantial favourite
  - most complete, cleanest-reading, highest-numbered - but that is exactly
  the shape of evidence `hd-frontend.md`'s own history warns against trusting
  before a runtime capture (its `DATA00` looked like the newest layer for the
  same reasons, and "looking like it is not evidence" was the finding right
  up until RPCS3 settled it with a `TTY.log` load-order printf). No PS4
  emulator exists in this project's toolchain to do the equivalent capture.
- **`campaign2048_definition.xml`'s own opening `<Screen>`/`<FE3DCanvas>`
  tags are unreadable** - real content starts at byte 18,269 of 29,919 in
  this extraction, past the file's own corrupted, all-zero prefix. What
  screen name it declares, and what widget wraps the recovered
  `<CanvasLabel>` fragment, is unknown.
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
    draw clean). Wired into `crates/game/src/sprite.rs`,
    `crates/game/src/boot/sprites.rs::gnf_sibling` and
    `crates/game/src/campaign.rs::load_omega`. What that opened rather than
    closed: this lane's own two-archive extraction is missing five of the
    campaign's own hex texture names outright (`Hexagon_HD_OUTLINE.gtf` and
    four siblings - no `.gnf` sibling either, not an unread name) and most
    of the front end's small *boot-time* image set (`saveIcons.gnf` is
    present but corrupt, `line.gnf` fails its own magic check) -
    `omega-status.md`'s "What did not draw" section names each one; whether
    a more complete disc dump has them is unchecked.
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
- Chasing `campaign2048_definition.xml`'s corrupted prefix is worth revisiting
  if `oag_formats::psarc`'s block-data-location understanding advances for
  other reasons (`docs/formats/psarc.md`'s own open "garbage" population) -
  a generic fix there would likely recover this file's opening tags too,
  rather than needing Omega-specific work.

## From the HANDOVER.md index (moved 2026-09-25)

swept 2026-09-21 from a player observation ("the Omega menu looks like HD/Fury's"); `FEGlobals` matches HD's to the digit and this project's own screen reader parses it unchanged, but which of the patch's four archives the runtime actually loads is the one open question HD's own six-copy version needed an RPCS3 capture to close
