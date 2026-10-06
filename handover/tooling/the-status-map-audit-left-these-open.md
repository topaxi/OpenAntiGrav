# The status-map audit: what it could not settle, what it found stale elsewhere, and the gaps it ranked

2026-09-23. `docs/overview/status.md` had drifted behind the code: Sideshift,
HD's sky, fog and SFX, and Pulse's and HD's bloom were all marked unstarted
while built. An audit walked every non-✅ cell and corrected about 25 of them
(merge `5ad05f6a`). This thread carries what it did not land.

## Cells it could not settle (still `⬜ ?`, deliberately)

No doc page or test covers any of these, and none was guessed:

- Front end: PS2 language selection, PS2 results, PS2 lock-on reticle, Pure
  results, Pure loading screens.
- Rendering: 2048 skybox, fog, particles and engine trails, plus several
  HD/2048 sub-cells.
- Weapons: HD/Omega Turbo and Autopilot manager classes not located; Omega
  Shield and Autopilot.
- Race modes: Pure Speed Lap, Pure and HD campaign grids, and several other
  Pure cells.

## Level calls it left to a maintainer

Two independent passes disagreed on these. The page's own rule ("verified =
built and checked against the original") picks the lower level unless noted:

- **HD Language selection**: ✅ or 🟩. The four RPCS3 boots measure the
  original, but the only test (`intro.rs`'s
  `skip_never_shown_picker_defaults_hd_to_english_with_no_input`) checks the
  build, not a comparison. Rule says 🟩.
- **Pure Language selection**: 🟨, gap "row pitch 14.95 vs measured 18"
  (`docs/formats/pure-status.md`, about lines 798-812). The pink selection
  arrow is unwired too, but its placement is unmeasured.
- **Pure Menus, Pure Ship/track, PS2 Ship/track**: 🟨 (named gaps on the
  pages: row pitch, stat bars, no PCSX2 comparison) versus 🟩 (how Pulse
  PSP's similar small gaps are scored).
- **2048 Boot movies** 🟨 (`BOOT_PROFILE` is still `Provenance::Declared`,
  no attract-mode race). **2048 Loading** 📖 rather than 🟨: nothing
  2048-specific is built (`loading: None`).
- **HD Track geometry**: ✅ is available if an RPCS3 matched-camera overlay
  counts as the screenshot diff (`docs/reverse-engineering/rpcs3-capture.md`,
  "The pick is fixed, and a rendered overlay confirms it"). The overlay is
  judged by eye and is not committed.
- **Pulse PSP Results/end-race**: a ✅ candidate. `docs/ui/endrace-screens.md`
  "Captures" matches the original's frames digit for digit, except one
  documented last-centisecond artefact.
- **2048 Ships**: 22 submeshes and all 6 materials
  (`crates/rcs/tests/psp2_rcsmodel_material_ground_truth.rs`), not the
  16/5-of-6 some pages still say.
- Section 3 has no Omega column, although `oag-omega` boots to Language
  Selection and draws its main menu and campaign grid.

## Stale text outside status.md (not fixed; out of the audit's lane)

Checked still stale at the time of writing unless marked:

- `docs/overview/roadmap.md` about 1189-1195: the M6 bloom box is unchecked
  and says nothing writes the alpha. Bloom is ported and measured
  (`crates/post/src/bloom.rs`,
  `docs/ghidra/functions/psp-pulse-usa/bloom.md`), and status.md's own M6 row
  now says so.
- `docs/formats/2048-status.md` about 86: ".envsettings does not parse for this
  title". `docs/formats/envsettings.md`'s 2048 section says it does.
- `docs/formats/2048-status.md` about 82 and `docs/formats/2048-rcsmodel.md`
  about 601: the stale 16-submesh figure.
- `docs/rendering/README.md`: "Shadow has a design and no implementation"
  (`shadows.md` says all four tiers are built), and "FSR 3.1 ... one of its
  eight passes built" (`fsr3.md` says all eight).
- `docs/rendering/shadows.md` lines 3-9: a garbled header, "`mapped` is still
  design" directly followed by "All four tiers are built".
- `docs/formats/rcsmaterial.md` about 2111 and
  `docs/rendering/scenery-animation.md` about 281/289: "the `time` scroll is
  unwired", against the same pages' "Wired 2026-08-31".
- `docs/formats/envsettings.md` See-also: "the sky cubemap that is refused".
  Every cubemap decodes.
- `docs/formats/psp-audio.md`: the heading "HD makes eight of the nine sounds"
  (the test says 13 of 14), and about line 1090 "HD ... deliberately not
  implemented".
- `docs/ui/selection-screens.md` "The flow" and
  `crates/ui/src/menu/definition.rs` about 388-398: "Pure, HD, 2048 have no
  race box". Pure has had one since `f6f82bc3`.
- `docs/ghidra/functions/ps2-pulse-eu/loading-screen.md` "Not determined":
  where `LoadingPulseOverlay.mip` lives on PS2 is answered by
  `crates/render/src/loading.rs` about line 365. The same page says
  `LoadingBackTop.pct` is `WADS2.WAD` entry 137, while the code and
  `ps2_source_ground_truth.rs` use 138, both 2,317 B. One `oag-wad` listing
  settles which.
- `docs/formats/2048-frontend.md`: "Still open ... the intro's picture and
  length", against its own table row (`intro.mp4`, 99.57 s).
- `docs/ui/endrace-screens.md` line 22: an in-page anchor
  `#wipeout-hdfury-results-menu-off-...`, where the heading slug is
  `...-resultsmenu-...`.
- `crates/hd/src/loading.rs` module doc (lines 1-90): presents the `LSAD_*`
  stills as the retail loading screen. `docs/formats/hd-loading.md` "What the
  emulator settled" says RPCS3 disproved that.
- `crates/sound/src/sfx/announcer.rs` about 20-23: says 2048 has no zone
  announcer.
- ~~`shuriken.md` about 112 against the `projectile_sprites` doc comment~~ the comment was stale; fixed 2026-10-06 when the blade's model was drawn.
- `handover/rendering/m6-authored-lighting-no-hardware-light-slot-found.md`
  still calls `DirectionalLight`'s consumer unfound; `scene-light.md`
  (2026-09-23) found it.
- **Fixed since**: `docs/gameplay/pickups.md`'s "a mine or a bomb draws
  nothing" (closed by the Bomb blast merge).

## Ranked gaps, by what a player would notice

The audit's list, lightly corrected. The ones worked this session are
marked.

1. Pulse weapon SFX. **In progress 2026-09-23**, see
   `handover/audio/pulses-weapon-cues-are-read-and-mostly-unwired.md`.
2. HD draws Pulse's fallback weapon models and effects, not its own. See
   `handover/rendering/hd-furys-weapons-draw-pulses-fallbacks-not-their-own-models.md`;
   the LeachBeam half was being worked 2026-09-23.
3. HD particle effects: all 88 `.pob` files parse, but only 2 have a recovered
   trigger. The audit's list attributed this to Pulse; its own rendering pass
   and Pulse's 35-effect count both say HD. Needs per-effect trigger RE on
   `ps3-hdfury-eu`, in batches.
4. Zone flies the player's own team stats, because no title authors a Zone
   `<Class>` block. Find where the original reads Zone's stats
   (`psp-pulse-usa` first).
5. Colour grading is unported on Pulse and HD. (Pulse's motion blur is an
   invented feature per `docs/rendering/motion-blur.md`, not a port gap.) The
   bloom work is the template.
6. Sideshift's availability gate (`craft+0x1c0 & 2`) is read and
   deliberately unported. A short RE pass on what sets that bit, then a small
   change.
7. 2048's atmosphere (skybox, fog, particles) is uninvestigated; it starts
   from zero on `vita-2048-eu-v104`.
8. HD/2048 engine sound. See
   `handover/audio/hds-engine-sound-is-a-per-team-crossfade-table.md`.

## Next Steps

1. Take the level calls above to a maintainer, or apply the page rule
   (lower level) and move on.
2. Fix the stale-text list in one docs-only pass, checking each item against
   the current tree first. Run `just check-docs` afterwards.
3. Draw implementation lanes from the ranked gaps.
4. Delete this thread and its index line once the first two are done; the
   ranked gaps each have or will get their own thread.
