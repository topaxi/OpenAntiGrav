# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); open,
in-flight work lives in [`handover/`](handover/), one file per thread; this
file is the index plus what a fresh reader would otherwise have to
rediscover before touching either.

**Keep it that way.** A finding belongs on a docs page with its evidence and
its confidence score; an open thread belongs in `handover/`; this file points
at both rather than restating them. Rewrite a section in place rather than
appending a dated pass to it - `git log -- HANDOVER.md` is the history, this
file does not need to carry its own.

## Read this first

- **The disc images are in place** - all six, in `data/images/`:
  `pulse-psp-usa.chd`, `pulse-psp-eu.chd`, `pulse-ps2-eu.chd`,
  `pure-psp-usa.chd`, `pure-psp-eu.chd` and `hdfury-ps3-eu.iso`. Hashes match
  [`source-images.md`](docs/reverse-engineering/source-images.md), so
  `just test-data` works. Earlier sessions have believed them lost and spent the
  session unable to verify anything - check before concluding a ground-truth
  test "cannot run".
- **`rg` and `fd` cannot tell you that.** Both respect `.gitignore` and `data/`
  is gitignored, so both return nothing with exit code 0 whether the directory
  is full or empty - no error, no warning, and it reads exactly like "there is
  nothing here". Use `--no-ignore`, or `/bin/ls` / `find` / `grep`. Note plain
  `ls` may be shell-aliased (`ls -la` failing with `ls:1: command not found:
  -la` reads like a broken argument and is not), and a spawned agent has a
  second, independent reason to see `data/` as empty: being gitignored, it does
  not travel into a worktree or a restricted sandbox. **In a worktree, fix that
  with `just link-data`** before concluding anything - it symlinks every `data/`
  subdirectory from the main checkout, and until it is run the DLC and trace
  suites skip silently (or, under `OAG_REQUIRE_GAME_DATA=1`, fail in a heap
  naming files that were in the main checkout all along). Added 2026-08-26 after
  exactly that happened: five `dlc_ground_truth` failures that were a worktree
  with no `data/dlc` in it and nothing else.
- **Derived evidence under `data/` is not durable; the scenarios and scripts
  that regenerate it are.** A 2026-07-29 pass found three reference traces gone,
  leaving `force-balance-ground-truth.md`, `contact-response.md` and three
  citations inside `wall.rs` pointing at files that were not there. Anything
  under `data/traces/`, `data/shots/` or `data/cache/` can be absent; the
  committed `verification/scenarios/*.inputs` are what reproduce it.
  **Two are absent right now (2026-08-26):** `pad0-boost.csv`, which the three
  `chase_camera_ground_truth` tests read, and
  `talons-junction-time-trial-lap-omega.csv`, which `maglock_ground_truth` reads.
  All four skip cleanly and only fail under `OAG_REQUIRE_GAME_DATA=1`, so a
  sweep with that variable set reads as four reds that are not.
  `stall_rescue_ground_truth::a_craft_that_stops_on_the_disc_is_put_back` is a
  **real** red as of the same date - it fails identically on `origin/main` with
  "nothing recovered the craft", is not a missing-data skip, and is undiagnosed.
- **Check `git status` before assuming the tree is clean.** A whole milestone's
  work once sat uncommitted for a day.
- **Gate status:** last measured green at **2,470 tests (2026-08-26)** in 3.3s,
  with `fmt`, `clippy`, `check-docs`, `check-deps`, `check-determinism`,
  `check-size`, `check-names` and `check-handover` all clean (480 skipped -
  the `#[ignore]`d disc-backed ones). Re-measure rather than trusting the number here -
  `git stash && just test` is how the drift was caught last time.
- **The whole disc-backed sweep is 2,488 of 2,494 in about 7:40**, measured
  2026-08-18 with `cargo nextest run --workspace --run-ignored all
  --no-fail-fast`. **Five of the six that fail are the `data/traces/` absence
  below**; the sixth is
  `ps2_source_ground_truth::an_uncapped_transcode_still_reports_a_total_to_divide_by`,
  and it is a **flake under load, not a failure**: it transcodes a PS2 `.PSS`
  through ffmpeg, takes 342s and returns no frames when the whole sweep is
  competing for the machine, and passes in 124s run on its own. Re-run it alone
  before chasing it. Use
  `--no-fail-fast`, because two of the trace ones sort early enough to stop the
  run at 335 of 2,494 and that reads like a much worse tree than it is.
- **`just test-data` takes about 3:17, not 11:27.** If it takes eleven minutes
  you are on a tree from before the build-profile fix: `[profile.dev.package."*"]`
  never matched a workspace member, so our own decoders and the sim compiled at
  `opt-level = 0`, and the suite additionally ran its multi-minute tests last.
  Both are fixed in `Cargo.toml` and `.config/nextest.toml`; the measurement and
  the reasoning are in
  [workspace-layout.md](docs/architecture/workspace-layout.md#build-profiles).
  **Do not "simplify" either file back:** the per-crate `opt-level` lines look
  redundant next to the `"*"` glob and are not, and the nextest priorities look
  cosmetic and are worth half the win. **The four `oag-trace` failures this bullet used to warn
  about are gone**: the disc-backed `oag-game` and `oag-formats` suites are
  **1,269 of 1,269, 0 skipped** under `--run-ignored all` (re-measured
  2026-08-17 after the HD HUD and `.gtf` passes; `oag-game` alone read 739, then
  656, then 652). **`oag-trace`'s five `data/traces/` failures are fixed
  (2026-08-19), and it is skips now, not reds**: `wall_contact_ground_truth` and
  `yaw_authority_ground_truth` used to fail with `No such file or directory` on
  `data/traces/talons-junction-standing-start.csv` and `-steer-left.csv`/`-steer-right.csv`
  whenever `data/traces/` did not exist, because - unlike every other
  ground-truth test in the tree - they read those files with a bare `expect`
  instead of the usual skip-if-absent guard. Both files now gate on a
  `capture()`/`capture_path()` helper matching the pattern `image()` already
  uses elsewhere (`OAG_REQUIRE_GAME_DATA` forces a hard failure, otherwise a
  missing capture prints the `verification/scenarios/*.inputs` command that
  regenerates it and the test returns early). Verified both ways: all five skip
  cleanly with `data/traces/` absent, and fail loudly under
  `OAG_REQUIRE_GAME_DATA=1`. **Two of them are flaky rather than solid, and both are in
  `ps2_source_ground_truth`**:
  `an_uncapped_transcode_still_reports_a_total_to_divide_by` failed once and
  passed on the re-run, panicking in *ffprobe duration parsing* ("invalid float
  literal"); `a_transcode_reports_its_frames_as_it_encodes_them` did the same on
  2026-08-16 - one failure in a full `--run-ignored all` sweep, then a pass
  alone and a pass in a second full sweep of the identical tree. Neither has
  been diagnosed. **Suspect the transcode's environment, and re-run before
  believing either**; a single red in that file is not a signal about the code
  under test. **2026-08-17 adds the shape of it, still not the cause: the
  failure tracks machine *load*, not a cold cache.** The same test failed again
  in a full sweep - panicking at `ps2_source_ground_truth.rs:657` on
  `movie.frames` being `None`, i.e. `.expect("a picture")` - and then passed
  **cache-cold and alone in 117s** after `/tmp/oag-ps2-progress-ground-truth`
  was deleted outright, so an absent cache is not the trigger. The transcode
  self-parallelises (242s user over 117s wall), which is why it is the test that
  suffers when 16 cores are already full: it took 489s contended and 135s
  scheduled first. Reproducing it means reproducing the contention, not the
  cache state. Note also that `refresh: true` is load-bearing there - the
  assertion is about what a *transcode* reports, so it re-encodes 950 frames
  every run whatever is cached, and the "(once; cached after this)" line it
  prints is misleading about itself. Separately, and still true, **one failure is the
  machine rather than the code**:
  `oag-disc::ground_truth listing_matches_chdman_and_the_reference_iso` shells
  out to `chdman extractdvd` into `/tmp/oag-ground-truth.iso`, `/tmp` here is a
  6.8 G tmpfs, and the extracted PSP ISO does not fit in what is free. It fails
  at "Extracting, 29.4% complete... Error writing to file; check disk space",
  which `nextest` reports as `chdman extractdvd failed` two frames up and reads
  like a broken invocation. Free `/tmp` or point the test elsewhere; do not go
  looking for a disc bug.
- **Rendering fidelity gaps are mostly not bugs.** Bloom, colour grading and
  every authored light class (`AmbientLight`/`DirectionalLight`/etc.) are still
  unchecked M6 roadmap items - the flat, dark look that produces is a known,
  documented gap, not something to guess-fix from a screenshot.
- **And a second one was, in the other direction: the disc authors the light
  rig and nothing read it.** `mesh.wgsl`'s two invented light directions were a
  stand-in and its own header said so; Wipeout HD states a sun direction, a sun
  colour and a constant ambient in a plain-text `track.envsettings` beside every
  circuit. Read and drawn on 2026-08-18 -
  [`envsettings.md`](docs/formats/envsettings.md). **The trap in the file**:
  `"Lighting.Sky colour"=128 128 128 0` is a byte quadruple where every other
  colour is normalised, and the only thing that says so is the missing decimal
  point; and `Sun direction` is **not** a unit vector on most circuits, with
  four degenerate. **The hold**: its magnitudes are authored for a linear HDR
  pipeline with a tonemapper and this one has neither, so the hue is drawn and
  the scale is not. That the file authors magnitudes above 1.0 at all is
  evidence that ADR-0020's gamma-authoritative model is wrong *for HD
  specifically* - recorded on the page, not acted on, because superseding an ADR
  needs a reference frame from the running original.
- **But one of them was, and the shape of it is worth carrying.** On 2026-08-18
  a screenshot of HD's Talon's Junction showed a quarter of the circuit smeared
  into horizontal streaks. Two of `.rcsmodel`'s readings were *solving* for
  something the file states: a chunk's `+0x58` points at a **vertex-attribute
  declaration** giving the stride, and per attribute a `~crc32` name, an RSX
  type and a byte offset. The texture coordinate was being read from the last
  four bytes of a vertex, which on the commonest layout is `lightmapUV`.
  `docs/formats/rcsmodel.md`. **The trap that cost the first sweep**: `+0x58` is
  the index count on a `LAYOUT_INLINE` chunk, so a disc-wide sweep that does not
  gate on the `+0x06` layout byte comes back as convincing noise - 606
  declarations with a stride of zero, type codes spread over all sixteen values.
  **The lesson worth more than the fix**: the retired reading had a
  texel-density discriminator that really did separate two groups, and a sweep
  of every header byte that really did find no field separating them. Both were
  correct and both were about the wrong thing - the sweep was run against labels
  the discriminator produced, so it could not find the field one word away from
  the bytes it swept. When a heuristic stands in for a missing field, check the
  field is missing.

## Where the project stands

### The 2026-08-18 review's findings

[`docs/reviews/2026-08-18-code-review.md`](docs/reviews/2026-08-18-code-review.md)
lists the findings and the headline fixes. The do-now batch, the next-session
batch and the P2/P3 sweep are all landed, one commit each, full `just` green
throughout; what was not done is below.

**Not done, and why**: S4/S5 (a per-asset `(Archives, &Title)` split and a
title-probe table - racebox groundwork the review itself grades L/M and puts
last); S6 (a naming axis on `Title`, waiting on the 2048 corpus that would
give it shape, per ADR-0022's own rule); I8 (splitting this file - M-sized,
orthogonal to a findings pass); U3 (physical- vs logical-key bindings -
investigated and rejected: AZERTY needs physical scancodes throughout, and a
hybrid mapping some keys physically and some logically would be worse than
either pure scheme); U4 (a `Vec` resize plus an `Arc<Sound>` drop on the
audio thread - needs a return channel to fix honestly).

**One trap worth keeping**: G3's "validate the chain's first step" check also
fired on `--reel`, whose state is off-path by construction (the dev/pub reel
has no screen in the set), and silently turned the flag into a no-op.
`boot_ground_truth::the_reel_leg_still_runs_its_frame_holds` is the only
thing keeping that path from rotting, and it is `#[ignore]`d, so `just`
stayed green throughout. A validity check applied to a state the *operator*
named rather than a chain led to is a check against the wrong question.

### M4 and M5

M4 (shield/energy, Zone's ending, the grid, weapon pads, the weapon table,
pickups, projectiles, the race-level determinism hash, weapon-fire dispatch,
`Mode::SingleRace`) and M5 (six AI stages, per-craft liveries, respawns,
mistake injection, per-opponent lap times) are both landed. Each has its own
docs page with the evidence - **do not requote from this file**:

- Shield/energy pool, contact damage, Zone's ending:
  [shield.md](docs/ghidra/functions/psp-pulse-usa/shield.md),
  [zone-mode.md](docs/ghidra/functions/psp-pulse-usa/zone-mode.md).
- The grid, weapon-fire dispatch, pickups, projectiles, the Missile (the
  first weapon whose whole behaviour - lock, guidance, speed ramp, bounce -
  is recovered rather than half-invented):
  [grid.md](docs/ghidra/functions/psp-pulse-usa/grid.md),
  [weapon-fire.md](docs/ghidra/functions/psp-pulse-usa/weapon-fire.md),
  [pickups.md](docs/gameplay/pickups.md),
  [missile.md](docs/ghidra/functions/psp-pulse-usa/missile.md).
- A race ends and shows a scoreboard, off a captured snapshot rather than a
  live tick (`Race::tick` itself is unchanged):
  [race-modes.md](docs/gameplay/race-modes.md#what-happens-when-a-race-ends).
- The player is recovered when they leave the circuit
  (`Race::lost_off_the_track`, `PLAYER_RESCUE_HALF_WIDTHS`/`_TICKS`,
  deliberately not extended to a human-held slot 0):
  [ai.md](docs/gameplay/ai.md#and-nothing-recovered-the-player-either).
- Six AI stages, liveries, respawns, mistake injection, per-craft lap times -
  the AI no longer blocks M5, and **this paragraph is why a summary here ages
  faster than the row it summarises**: re-read the AI rows under Open threads
  before trusting this line. [ai.md](docs/gameplay/ai.md).
- Wipeout HD reaches the menus and starts a race, off a *declared* rather
  than measured boot chain, with four independent format bugs fixed on the
  way (`.fnt` byte order, no `.gtf` decode path, a Latin-1 language file, the
  480x272-authored menu grid at HD's 800 offset):
  [hd-frontend.md](docs/formats/hd-frontend.md),
  `crates/game/tests/hd_boot_ground_truth.rs` (13/13).
- Rocket visuals (`Data\Weapons\Rocket.vex`, not a billboard, with two
  separately authored explosions), floor-following flight and the km/h fix:
  [rocket-visuals.md](docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md).
- Race sound effects on all three titles, off the discs' own `.bnk` banks:
  six cues on Pulse and Pure, four of the six on HD.
  [psp-audio.md](docs/formats/psp-audio.md#a-cue-owns-a-run-of-the-command-table).

**What M5 still wants**: Zone's explosion. Positional audio and the Autopilot
pickup both landed 2026-08-24; each has a row under Open threads carrying what
is still unread about it.

The force law, angular-momentum model, contact response and mag-lock
attitude hold are all recovered at instruction level and implemented, and
every fitted constant that once stood in for a recovered one has been
retired (`YAW_DRIVE_CALIBRATION`, `ALIGNMENT_INERTIA`, the `<Misc>`-derived
box inertia). Evidence: [docs/physics/](docs/physics/README.md) (both
ground-truth pages and
[angular-velocity-column.md](docs/physics/angular-velocity-column.md)) and
[docs/ghidra/functions/](docs/ghidra/functions/README.md) (`engine.md`,
`rigid-body.md`, `contact-response.md`).

**Read lap fidelity off the *seeded* comparison, never open-loop.** The
original integrates a variable frame duration
([ADR-0007](docs/architecture/adr/0007-fixed-timestep-vs-original.md)), so
two runs of the original itself, from the same pinned start pose, are 100
units apart by tick 495 - a long single-seeded comparison measures the
emulator's scheduler, not the force law. The reference lap
(`data/traces/talons-junction-clean-lap.csv`) is 96.3% wall-free, first
touching a wall at tick 256, and *inside* that clean window single-seeded
position error is already 44.1 units by tick 240: the force law, not contact
geometry, is now what limits comparison length. Full numbers, the
standing-start scenario's independent cross-check (launch acceleration 32.69
against a recorded 32.52), and why `grounded` cannot be read as a headline
(`1.0` on all 3,146 of the original's own ticks) are on
[oag-trace.md](docs/tools/oag-trace.md#reseeding-and-why-a-long-comparison-needs-it)
and
[force-balance-ground-truth.md](docs/physics/force-balance-ground-truth.md).
**Always pass `--script-lead 2` against a `data/traces/` capture** - the
emulator's three-frame input latency means a script's first two states never
reach it, and replaying them anyway reads as a two-tick physics wedge;
`oag-trace.md` has the mechanism. A save-state fixed start was investigated
and not adopted (two loads of the same state disagreed by 0.031 units/1.41
degrees against `--start-heading`'s 0.0022/0.0001, which is what closed the
roadmap checkbox instead) -
[ppsspp-debugger.md](docs/reverse-engineering/ppsspp-debugger.md#save-states-and-the-input-recording-api-that-may-replace-them)
keeps the mechanism documented for a need heading-pinning cannot reach.

## Open threads

Each is a real, named next step, one file per thread under [`handover/`](handover/). Task numbers in a title are the ones the agent passes used, kept because commits and docs cite them. **When a thread's work lands, delete its file and this line** - the same rule this file always followed for a row, now for a file.

- [Invented UI text has no translation, and the disc's own strings have no override path either](handover/invented-ui-text-has-no-translation-and-the.md)
- [HD's sprite flare reads oversized against the original, and the tuning file's other 19 rows are read](handover/hds-sprite-flare-reads-oversized-and-the-tuning.md)
- [HD's glow draws, and the coordinate it samples on is the one thing not read](handover/hds-glow-draws-and-the-coordinate-it-samples.md)
- [2048's four Vita eboots are decrypted and imported; loose ends from getting there](handover/2048s-vita-eboots-are-imported-re-not-started.md)
- [2048's seven boot-manager names could be cross-verified against `ps3-hdfury-eu`, once that side has the same singletons named](handover/vita-2048-boot-managers-await-ps3-side-names.md)
- [2048's PSARC and title plumbing are wiring-ready; two binary formats changed underneath a track](handover/2048s-track-vex-parses-but-two-binary-formats-changed.md)
- [Streaming decode for audio would break seek, and nothing forces the change yet](handover/streaming-decode-for-audio-would-break-seek-and.md)
- [HD's cues end loud and were cut dead, and its `.COLLISIONS` is a flattened tree](handover/hd-collisions-flatten-a-severity-tree.md)
- [The audio queue is ours now, and the frame stalls are what is left](handover/the-device-queue-is-sized-off-the-configured-cap.md)
- [A circuit's billboard slots are a 9-entry array on the engine side, and slot 7 is not what it says it is](handover/a-circuits-billboard-slots-are-a-9-entry.md)
- [A parser cannot fail on a field it does not know about, so coverage is now measured](handover/a-parser-cannot-fail-on-a-field-it.md)
- [A chunk header is 0x20 bytes and then a *surface* record, and the byte after the layout says which space it is in](handover/a-chunk-header-is-0x20-bytes-and-then.md)
- [HD ships a per-chunk PVS, and drawing every chunk was the whole "meshes that should not be there"](handover/hd-ships-a-per-chunk-pvs-and-drawing.md)
- [HD's loading screen is recovered from its own constructor; three threads left](handover/hds-loading-screen-is-recovered-from-its-own.md)
- [The race mix saturates; the per-voice SAS volume would settle whether it should](handover/the-race-mix-saturates-the-per-voice-sas.md)
- [The Autopilot pickup is built off its own two handlers; three of its parts are still ours](handover/the-autopilot-pickup-is-built-off-its-own.md)
- [Positional audio is recovered whole, and the pan is a table the disc computes from `cos` and `sin`](handover/positional-audio-is-recovered-whole-and-the-pan.md)
- [HD's exhaust is measured from the running game now: the trail is a 54-sample three-fin tube, the flame breathes with the throttle, and the plume scales rather than blinks](handover/hds-exhaust-is-measured-from-the-running-game.md)
- [HD's engine trail is one of four ribbons, and a craft flying through one sparks](handover/hds-engine-trail-is-one-of-four-ribbons.md)
- [HD's 81 engine shader parameters are read from their own initialiser, and `time` is entry 0](handover/hds-81-engine-shader-parameters-are-read-from.md)
- [The PS2 boost plume draws, and the PS2 file is not the PSP file wearing the same name](handover/the-ps2-boost-plume-draws-and-the-ps2.md)
- [HD's frame was too bright and too bloomy; the bloom chain was not what was wrong](handover/hds-frame-was-too-bright-and-too-bloomy.md)
- [HD needs a per-material shader path, and two general rules for finding one have been refuted](handover/hd-needs-a-per-material-shader-path-and.md)
- [A PS3 title now boots, drives and screenshots from a script - and its savestates are not worth adopting](handover/a-ps3-title-now-boots-drives-and-screenshots.md)
- [Reversed grids were putting craft off the track; fixed by walking the spline instead of extrapolating a straight line](handover/reversed-grids-were-putting-craft-off-the-track.md)
- [HD's sky, fog and lighting draw from the disc's own statements - three holds remain, each named where it lives](handover/hds-sky-fog-and-lighting-draw-from-the.md)
- [A cue can play other cues, and that is what `.COLLISIONS` does on HD](handover/a-cue-can-play-other-cues-and-that.md)
- [Alternate selection never repeats the previous pick, and `Banks::pick` does not know that](handover/alternate-selection-never-repeats-the-previous-pick.md)
- [HD's `.bnk` sound banks read, and HD makes five of the six sounds](handover/hds-bnk-sound-banks-read-and-hd-makes.md)
- [HD/Fury's particle effects parse, and a "field that disagrees" turned out to be a wrong check](handover/hd-furys-particle-effects-parse-and-a-field.md)
- [Pulse's draw order is recovered, and it is a layer sort rather than a depth sort](handover/pulses-draw-order-is-recovered-and-it-is.md)
- [HD's see-through surfaces: the blend-equation fix landed, and the texture-coordinate/second-texture residue this file tracked is now resolved elsewhere](handover/hds-see-through-surfaces-draw-with-the-files.md)
- [Talon's Junction's "missing floor" is a glass floor drawn with the wrong texture: rendered, not absent](handover/talons-junctions-missing-floor-is-a-glass-floor.md)
- [An RPCS3 capture harness: a reference frame paired with the camera that drew it](handover/an-rpcs3-capture-harness-a-reference-frame-paired.md)
- [`Material::state`'s upper bits: bit 7 is measured, mode 2 is now fully decoded](handover/material-states-upper-bits-bit-7-is-measured-mode.md)
- [Wipeout HD's front end boots, off a chain that says it is only declared](handover/wipeout-hds-front-end-boots-off-a-chain.md)
- [HD's menus are drawn inside its own frame, and the `HD_*` palette turned out to be the FE style](handover/hds-menus-are-drawn-inside-its-own-frame.md)
- [HD's strip widget draws bare text; the real menu also draws a tab shape, an underline, and a background scene](handover/hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md)
- [Wipeout HD's video decodes, and the logo reel is the Studio Liverpool ident](handover/wipeout-hds-video-decodes-and-the-logo-reel.md)
- [Wipeout HD's soundtrack plays; finding it fixed a disc-wide PSARC bug](handover/wipeout-hds-soundtrack-plays-finding-it-fixed-a.md)
- [Pure's soundtrack plays, and both titles' track names are recovered - Pulse's are deliberately unwired](handover/pures-soundtrack-plays-and-both-titles-track-names.md)
- [HD/Fury's HUD draws, and three of Pulse's constants applied to every title were why it did not](handover/hd-furys-hud-draws-and-three-of-pulses.md)
- [Wipeout HD / Fury's HUD reads, and the reader it shares with Pulse was wrong in three ways Pulse's own data could never have shown](handover/wipeout-hd-furys-hud-reads-and-the-reader.md)
- [`.gtf` reads, so HD's textures are pixels](handover/gtf-reads-so-hds-textures-are-pixels.md)
- [Wipeout Pure races, on Pulse's physics, and what is still absent is a list of unrecovered class ids](handover/wipeout-pure-races-on-pulses-physics-and-what.md)
- [Pure's particle effects decode and play, and two of five names do not resolve](handover/pures-particle-effects-decode-and-play-and-two.md)
- [Per-team hulls are drawn; which team flies which slot is not recovered, and the paints are not read](handover/per-team-hulls-are-drawn-which-team-flies.md)
- [The particle effects are played from the disc, and most of them have no recovered trigger](handover/the-particle-effects-are-played-from-the-disc.md)
- [The PS2's engine flare plays and reads as nothing on screen](handover/the-ps2s-engine-flare-plays-and-reads-as.md)
- [The AI's six-stage plan (line-follower to a field of pilots) is complete; what remains unbuilt is the residual](handover/the-ais-six-stage-plan-line-follower-to.md)
- [The AI was never the problem: four contact-path bugs, and what is left after them](handover/the-ai-was-never-the-problem-four-contact.md)
- [A global reached through `$gp` has an instruction displacement unrelated to its address](handover/a-global-reached-through-gp-has-an-instruction.md)
- [`just apply-names` reported success for renames the bridge did not make](handover/just-apply-names-reported-success-for-renames-the.md)
- [DLC packs are mounted; two things inside them are not read](handover/dlc-packs-are-mounted-two-things-inside-them.md)
- [Two recovered chase-camera behaviours are ported; `headtilt` is not](handover/two-recovered-chase-camera-behaviours-are-ported-headtilt.md)
- [The menus draw the disc's layout; the footer's ticker and prompts are still unbuilt](handover/the-menus-draw-the-discs-layout-the-chrome.md)
- [PS2 front-end layout is hardcoded to 480x272](handover/ps2-front-end-layout-is-hardcoded-to-480x272.md)
- [The HUD layout ground truth is PSP-only, and the PS2 layouts are unchecked](handover/the-hud-layout-ground-truth-is-psp-only.md)
- [`12_Track`'s sky draws one white face](handover/12-tracks-sky-draws-one-white-face.md)
- [Audio: race SFX plays on all three titles](handover/audio-race-sfx-plays-on-all-three-titles.md)
- [`oag-trace plan` has never been replayed into the emulator](handover/oag-trace-plan-has-never-been-replayed-into.md)
- [M6 authored lighting: no hardware light slot found enabled, and no `DirectionalLight` consumer found anywhere](handover/m6-authored-lighting-no-hardware-light-slot-found.md)
- [Ghidra target of record is `psp-pulse-eu`; EU cross-verification is owed](handover/ghidra-target-of-record-is-psp-pulse-eu.md)
- [`/psp-pure-usa`, `/psp-pure-eu` and `/psp-pure-eu-reimport` were missing from the project on 2026-08-10, despite the 2026-08-09 note below describing them as already imported and saved](handover/psp-pure-usa-psp-pure-eu-and-psp.md)
- [The EU/Pure imports have never been diffed](handover/the-eu-pure-imports-have-never-been-diffed.md)
- [Ghidra names owed from the camera/matrix corroboration](handover/ghidra-names-owed-from-the-camera-matrix-corroboration.md)
- [The ghost-ship renderer is read and written down nowhere else](handover/the-ghost-ship-renderer-is-read-and-written.md)
- [Magstrip leftovers, none load-bearing](handover/magstrip-leftovers-none-load-bearing.md)
- [Frame comparison: three residuals](handover/frame-comparison-three-residuals.md)
- [Weapons: seven of thirteen, and two doc pages had two of them swapped](handover/weapons-seven-of-thirteen-and-two-pages-were.md)
- [Projectiles follow the floor, and the km/h fix - both now landed](handover/projectiles-follow-the-floor-and-the-km-h.md)
- [The Missile fires without a lock, and the lock-on reticle and its tone are in](handover/the-missile-fires-without-a-lock-now-and.md)
- [Pure and HD lock on too, and it cost two axes rather than any new recovery](handover/pure-and-hd-lock-on-too-and-two-axes.md)
- [The hull sparks are approximated in three named ways](handover/the-hull-sparks-are-approximated-in-three-named.md)
- [The original's camera/HUD shake on impact is untraced](handover/the-originals-camera-hud-shake-on-impact-is.md)
- [PVS culling: three ceilings, one of them invented](handover/pvs-culling-three-ceilings-one-of-them-invented.md)
- [The HUD's four remaining items](handover/the-huds-four-remaining-items.md)
- [Race modes: the start line is measured on one circuit only](handover/race-modes-the-start-line-is-measured-on.md)
- [Zone ends now; what is missing is the explosion, not the rule](handover/zone-ends-now-what-is-missing-is-the.md)
- [Zone flies its own environment now; the menu is the part still not mode-aware](handover/zone-flies-its-own-environment-now-the-menu.md)
- [Zone races run on all three titles now, and the craft axis closed the way the circuit one did](handover/zone-races-run-on-all-three-titles-now.md)
- [2048's Zone grade escalates on the disc's own ladder now; HD's own trigger is what remains](handover/2048-and-hd-ship-an-unread-effectsettings-table.md)
- [HD's per-stage Zone textures are grounded now: which file feeds which sampler is read, what the shader does with them is not](handover/hd-zone-stage-textures-are-grounded.md)
- [HD's Zone ladder draws and its speed-class sound now plays; the class-change blend stays unwired on three unrecovered inputs](handover/hds-zone-ladder-draws-and-the-zone-to.md)
- [The AI is authored XML, and its units are the only thing blocking a port of the original's numbers](handover/the-ai-is-authored-xml-and-its-units.md)
- [The HUD shows a place, measured against the original; the shield bar's colour is the new open half](handover/the-hud-shows-a-place-measured-against-the.md)
- [The field drove in single file, and the fix is a per-craft personality off the disc's own AI corridor](handover/the-field-drove-in-single-file-and-the.md)
- [Nothing airborne has ever been captured](handover/nothing-airborne-has-ever-been-captured.md)
- [The hover probe takes only the deepest hit](handover/the-hover-probe-takes-only-the-deepest-hit.md)
- [Is there a fifth handling class?](handover/is-there-a-fifth-handling-class.md)
- [The lap capture's own "the HUD said `Lap 2 of 3`" claim is now in tension with the finding that closed this row, and nobody has reconciled them](handover/the-lap-captures-own-the-hud-said-lap.md)
- [The 13 % roll-stiffness gap](handover/the-13-roll-stiffness-gap.md)
- [Craft-to-craft collision is implemented; the stun is still not armed](handover/craft-to-craft-collision-is-implemented-the-stun.md)
- [The barrel roll is identified and unimplemented](handover/the-barrel-roll-is-identified-and-unimplemented.md)
- [Sideshift has no runtime leg](handover/sideshift-has-no-runtime-leg.md)
- [Task #31 residual: the unguarded `slice(..)` in the race's draw path](handover/task-31-residual-the-unguarded-slice-in-the.md)
- [Menus: rebinding is the one thing that does not work](handover/menus-rebinding-is-the-one-thing-that-does.md)
- [MONITOR has only ever run on a one-screen machine](handover/monitor-has-only-ever-run-on-a-one.md)
- [FSR 1's default is open](handover/fsr-1s-default-is-open.md)
- [Front-end gaps behind `Image`](handover/front-end-gaps-behind-image.md)
- [Pure's dev/pub hold duration, and how the original picks a regional cut](handover/pures-dev-pub-hold-duration-and-how-the.md)
- [`Movie::entry_name` hardcodes `_US`, so a European Pure disc shows the American card](handover/movie-entry-name-hardcodes-us-so-a-european.md)
- [`--until` cannot reach a late movie frame on a machine with an audio device](handover/until-cannot-reach-a-late-movie-frame-on.md)
- [Where Pulse's language picker belongs is unevidenced](handover/where-pulses-language-picker-belongs-is-unevidenced.md)
- [Pure's string tables are not read, and its front-end font is absent](handover/pures-string-tables-are-not-read-and-its.md)
- [Pure's `Title Screen` is missing its own logo wordmark - not chased further](handover/pures-title-screen-is-missing-its-own-logo.md)
- [A *slot-resolved* record's own field layout](handover/a-slot-resolved-records-own-field-layout.md)
- [Pure's DLC trailer key and TEST.bin, still unknown](handover/pure-dlc-trailer-key-and-testbin-unknowns.md)
- [A model built from several small pieces sharing one atlas](handover/a-model-built-from-several-small-pieces-sharing.md)
- [Which movie cut plays, and what plays the three 260-frame reels](handover/which-movie-cut-plays-and-what-plays-the.md)
- [The PS2 PAL/NTSC selector's ultimate trigger](handover/the-ps2-pal-ntsc-selectors-ultimate-trigger.md)
- [Two PS2 identifications, one of which is wrong](handover/two-ps2-identifications-one-of-which-is-wrong.md)
- [The SAP clamp globals](handover/the-sap-clamp-globals.md)
- [SteamOS's own glibc version](handover/steamoss-own-glibc-version.md)
- [The loading screen draws now; the wave's decode is still not runtime-verified](handover/the-loading-screen-has-no-caller.md)
- [`Anim Transform` `0x3c0` is read and played, and closing it fixed a placement defect](handover/anim-transform-0x3c0-is-read-and-played-and.md)
- [The PS3 Ghidra path works; the cspec-only fork is done, `lvlx` is what's left](handover/the-ps3-ghidra-path-works-two-improvements-to.md)
- [Wipeout HD Fury's executable is 26,100 functions with 26 of them named, and no gameplay behaviour yet](handover/wipeout-hd-furys-executable-is-26100-functions-with.md)
- [The alpha-test cutout reference is recovered as three values, not one, and the per-batch selector is not](handover/the-alpha-test-cutout-reference-is-recovered-as.md)

## Pending maintainer decision: shipped design data in tracked docs

[`handling-stats.md`](docs/formats/handling-stats.md)'s own rule, per
[ADR-0006](docs/architecture/adr/0006-no-copyrighted-content.md) - "a field name
is a description of the format, a tuning table is the content itself" - is
breached in places by earlier passes. `angular-velocity-column.md`'s shipped
`Turning.amount` has been redacted, but it still quotes the Assegai `Misc` hull
dimensions in its box-tensor refutation, and `force-balance-ground-truth.md`
carries XML-side values from the force-law derivations (`accelcap`, an
`amount`-derived figure, `ride_height`, `Physical.mass`), some of them
load-bearing in the arithmetic.

**Whether to sweep those historically is a maintainer decision, not an
agent's**, which is why it is flagged rather than done. Git history retains
everything regardless, so a sweep would clean the distributable snapshot, not
the past.

**One new instance was resolved by the maintainer rather than flagged,
2026-08-17.** The `Weapon Pad` ready-state colour cycle - a 6-entry `[r,g,b]`
keyframe table recovered from `WeaponPad_UpdateRefreshTimer`
(`docs/ghidra/functions/psp-pulse-usa/pads.md`) - is exactly this kind of
tuning table. Asked directly, the maintainer chose to commit it
(`oag_render::weapon_pad::READY_KEYFRAMES`) rather than defer or build a
runtime BOOT.BIN reader, so it is tracked source now. Recorded here as the
precedent for the next one of these that comes up: ask, do not assume either
answer.

## Scope decisions that look like gaps

| Decision | Why |
| --- | --- |
| **The Memory Stick and profile system** | One automatic save slot, the way a modern PC game does it. Most of the disc's boot chain between `Language Selection` and the menus is deliberately out of scope. |
| **PSP over PS2 where they diverge** | User directive, now policy in [goals.md](docs/overview/goals.md#scope). PS2 stays the corroboration leg for confidence, not the target. |
| **The collision stun is not armed by track contact** | The original's is gated on a pending impulse the contact path never writes. |
| **Sweep and prune is not reimplemented** | The original's packing clamps world space rather than rebasing it, so it never drops a genuinely overlapping pair; ours does the same job without the packing. |
| **The four-corner hover variant** | Not implemented. Its selector is known (`DAT_08ab07e3 == 0 && DAT_08b31048 == 6`, which also disables the brakes and the weapons) and it is not the racing configuration. |
| **Pure asset work** | No longer deferred as a whole. [ADR-0022](docs/architecture/adr/0022-title-packages.md) opened the format and asset layers on the strength of [`pure-status.md`](docs/formats/pure-status.md)'s measurements; ADR-0009 item 2 still defers Pure *simulation* work behind M4's exit. |
| **PS3 content** | Still paused (since 2026-08-09), on the toolchain rather than on the research - see [`data/README.md`](data/README.md#the-ps3-image-is-encrypted-and-nothing-here-decrypts-it-yet). None of `PS3Dec`/`scetool` is installed and there is no RPCS3 install to borrow a decrypted copy from. |
| **Vita content** | Not paused: all four `eboot.elf` (base + patch v1.04, both regions) are decrypted end to end and imported into Ghidra as of 2026-08-26. The layer the prior entry missed - `eboot.bin` past PFS decryption is still NpDrm-encrypted under its SELF wrapper, a plaintext header and phdrs say nothing about the segments - is solved by `scripts/vita-self-decrypt.py`, which needs only the title's klicensee (itself pure-software recoverable from a zRIF, no F00D/hardware - `scripts/zrif-to-klicensee.py`). See [toolchain.md#vita](docs/reverse-engineering/toolchain.md#vita). Default target: `/vita-2048-eu-v104/eboot.elf` (EU, patch v1.04); naming and RE work itself are the open thread now. |
| _(shared architectural fact)_ | **HD/Fury and 2048 both ship PSARC, not WAD**, and 2048 arrives as a PKG rather than a disc filesystem. |
| **A "CPU renderer" switch** | There is nothing to switch. wgpu ships no software rasteriser (`Backend::Noop` draws nothing), so CPU rendering exists only via a system Vulkan implementation such as lavapipe, selected outside the process. |
| **RENDERER applies on the next launch** | The device is made once at boot and everything hangs off it. Live switching means tearing down the surface, the pipelines and every GPU resource. |
| **PS2 `.PSS`/`.IPF` through GStreamer** | [ADR-0017](docs/architecture/adr/0017-gstreamer-native-video.md). |
| **`just play`'s default is not `oag-game`'s default** | `native-video`'s Cargo feature is off by default, so `cargo build`/`cargo test`/CI stay GStreamer-free; `just play` turns it on. |
| **GStreamer for ATRAC3+** | **Do not retry this.** Extending the `native-video` path to audio was the intended route and it cannot work - see [ADR-0019](docs/architecture/adr/0019-atrac3plus-out-of-process.md). |
| **Motion blur is built, and the jitter in its tile lookup is load-bearing** | 2026-08-31, replacing a row that said it was designed and had no code: it shipped with the velocity buffer ([ADR-0030](docs/architecture/adr/0030-velocity-buffer-motion-blur.md)), and both questions that row held open are answered - `Rg16Float` *is* multisample-renderable at 4x here, pinned as a tripwire by `rg16float_is_multisample_renderable_at_4x_on_this_adapter`, and the ADR owed was written. **The trap now points the other way.** `fs_reconstruct` offsets its tile lookup by up to half a tile, which reads as noise for its own sake and is not. Take it out and an axis-aligned lattice of squares comes back, 8 % of the viewport height across - 87 px at 1080p, anchored to the window rather than the world, so the scene flows through a stationary grid. Reported from play in a turn, measured at a 2.33x phase-0 peak and fixed to 1.48x; the numbers, the method and the two suspects ruled out instead of assumed are in [motion-blur.md](docs/rendering/motion-blur.md). The squares are that large because the reach cap doubles as the tile edge, so a generous cap makes coarse tiles - the same coupling that made the square tile-max starve the GPU. Still open there and *not* a suspect for this: `SOFT_Z` compares nonlinear 0..1 depths against a constant, which is dimensionally wrong and measured as contributing nothing to the lattice. |
| **The disc chooser is not a `menu::Menu`** | 2026-08-18. `oag_game::launcher` has rows, a cursor and the same abstract buttons, and it is its own module anyway: it runs before any archive is open, so it has no title, therefore no `MenuSkin` and no disc font. Making it a menu would mean choosing a skin in order to choose which disc the skin comes from. It draws with `font::Atlas::build` and `sprite::Sheet::default`, the disc-free pair the perf overlay already uses. The test for anything else moving out of `menu.rs` is whether a title can be behind it - and everything else can. [menus.md](docs/architecture/menus.md#the-one-screen-that-is-not-a-menu) |
| **The chooser has no headless capture route** | `--until` matches front-end state-machine names read off the disc's own `Skin.xml` (`capture.rs`), and this screen has none - it exists before a disc does. `--menu-page` is for our menu tree. So `--launcher` is *refused* with `--screenshot`/`--dry-run`/`--race` rather than half-working, and the screen is covered by unit tests plus `crates/game/tests/launcher_ground_truth.rs` instead. **`every_line_fits_on_screen` in `launcher/tests.rs` is the one that earns its keep**: it measures each drawn string in the 5x7 face against the 480-unit grid, and it exists because the first version's footer ran off the right edge where no test could see it. |
| **`chdman` is not installed in this sandbox; PPSSPP does not need it anyway** | 2026-08-19. `just extract-iso` (and `psp-drive.py preflight`'s own suggested recipe) calls `chdman extractdvd`, and in this sandbox that fails immediately with `command not found` - no `chdman` binary anywhere on the machine, confirmed by a full `find /`. Cost a detour into "live verification is blocked, commit the doc-only half and leave the port uncommitted" before the actual fix was noticed: `ppsspp-debugger.md` already says **PPSSPP v1.20.4 boots a `.chd` directly**, extraction is optional on that version, and the installed `PPSSPPSDL`/`PPSSPPHeadless`/`Xvfb` are all present and work. Pointing the SDL launch straight at `data/images/pulse-psp-usa.chd` (skip the `chdman` line in the docs' own recipe entirely) reached a live, driveable Single Race normally. Check `which PPSSPPSDL PPSSPPHeadless Xvfb` before assuming a live PSP session is unreachable here - the CHD-vs-ISO step is what's actually missing, not the emulator or the display. |
| **`body+0x50` is live-verified as a craft's world position; `rigid-body.md`'s own `body+0x40..0x70` inertia-tensor table says that range is something else - and a follow-up check sharpened the conflict rather than closing it** | 2026-08-19, found while live-verifying `Weapon_PostBlastImpulse_q`'s port - see the stun row above and [contact-response.md](docs/ghidra/functions/psp-pulse-usa/contact-response.md#weapon_postblastimpulse-0x0886794c-confidence-82). A breakpoint read `body+0x50` off a real craft's confirmed `RigidBody` pointer mid-tick and got a live, track-scale, per-craft value that tracked the craft's actual position - not tensor-shaped data. `rigid-body.md` independently reads `body+0x40..0x70` as one confidence-88 inverse-inertia-tensor block, backed by `Body_Integrate` visibly consuming it every sub-step via `vtfm3.t`/`vscl.t`. **Checked further, not just flagged**: `Body_SetBoxInertia`'s diagonal patch-back writes land at `+0x40`/`+0x54`/`+0x68`, stride `0x14` - exactly what a row-major 4x4 with `0x10`-byte rows at `+0x40`/`+0x50`/`+0x60`/`+0x70` predicts for its own diagonal, which *confirms* the constructor genuinely treats `+0x50` as row 1's base rather than resolving the conflict in the other direction. That makes the runtime finding sharper: an axis-aligned box's row-1 off-diagonal terms should read `0.0` forever after construction, and they visibly do not. Both readings rest on real instruction traces; neither has been withdrawn, and they still cannot both be exactly right about `+0x50` specifically. Flagged with a cross-reference in both docs, reconciled in neither - next step is tracing exactly which quad-words `Body_Integrate`'s `vtfm3.t` addresses (a `.t` transform is 3x3; which three rows from a `+0x40` base is unread) and finding what writes `body+0x50` every tick. A second, smaller thread from the same session, explicitly **not** a finding: `body+0x30` was read at the start line pre-countdown (not a controlled comparison against the mid-race `+0x50` sample) and clustered near `(0, 0.7-0.75, 0)` for all eight craft - as consistent with a parked grid in some local frame as with anything else; `rigid-body.md` already has `Body_Translate` accumulating into `+0x30` the way a real position field would. Needs a same-breakpoint mid-race re-read before it means anything. |
| Everything else milestone-scoped | Weapons, AI, netcode, the shell: [roadmap](docs/overview/roadmap.md), not this file. |

## Working rules that were learned expensively

**Most of them now live in `docs/`**, because they are properties of the method
rather than of any pass:

- [Reverse engineering](docs/reverse-engineering/methodology.md#rules-learned-the-expensive-way) -
  decode before reporting, xref counts bound authored instances only, sibling
  pages go stale, never write a fitted constant in as a recovered one, and the
  rest.
- [Reading a capture](docs/reverse-engineering/verification-protocol.md#reading-a-capture-rules-that-each-cost-a-session) -
  control columns first, unclamped ramps, clean-capture detection, ask the
  tangent not the index, Left-Up-Forward, and threshold-versus-frame stability.
- [GhidraMCP bridge quirks](docs/reverse-engineering/toolchain.md#search_instructionss-mnemonic-filter-is-exact-match-not-substring) -
  `search_instructions`'s `mnemonic` filter is exact-match despite its own
  description, so a sweep filtered on one mnemonic silently misses every
  underscore-prefixed delay-slot form of it; `run_script_inline`'s current
  failure mode.
- [Measuring a renderer change](docs/rendering/README.md#measuring-a-renderer-change) -
  aggregates rank broken changes higher, brightness thresholds measure the
  circuit, measure a screenshot before citing it.

What stays here is what is about working in *this* repository, with other
writers in it at the same time:

- **Editing this file or a `handover/` thread**: verify each cut against the
  page or evidence it cites before cutting, not after - a fitted number
  drifting from its source is the usual way this goes wrong. Compress or
  delete a thread that is closed or stale, never one still carrying open
  work. Never merge numbers into a new total that does not already appear
  merged on the page they came from.
- **`git stash push -- <pathspec>` has the same failure, one level earlier:
  it captures a listed file's *entire* working-tree diff, not the hunks one
  writer actually made.** 2026-08-25. A session isolating its own edits into a
  worktree via `git stash push -u -- <its files>` named `menu.rs` among them;
  another session was concurrently mid-refactor in that same file, uncommitted.
  The stash swept both diffs off disk in one move, and popping it in the
  worktree, then discarding the mixed result down to just the first session's
  own hunk, left the second session's work recoverable only from the stash
  commit's blob (`git show <sha>:<path>`) - not restored on disk anywhere, and
  the main checkout was left with that file's *other* uncommitted edits
  (test/data files referencing the now-stashed API) pointing at a version of
  it that no longer matched. **Before stashing a file by pathspec while
  another session might be touching it, diff it first** (`git diff -- <path>`)
  and check whether the whole diff is actually yours; if not, this needs a
  worktree isolating *that session's* edits instead, or a message to them
  before touching the file at all - not a stash-and-hope.
- **Do not run `just` or `cargo fmt --all` while anything else is editing.** A
  gate run taken mid-edit reports failures that do not exist, and `cargo fmt
  --all` rewrites files another writer holds open. Scope to `-p <crate>`.
- **A doc two parties are editing wants committing promptly**, not left dirty:
  concurrent uncommitted edits are indistinguishable from data loss to whoever
  looks second.
- **Splitting a large file (`crates/game/src/main.rs`, 4,808 to 430 lines
  across 18 modules, 2026-08-16) is safest computed as one map from the
  original applied in a single pass, not carved interactively** - each
  edit shifts every line number below it, so line-number-anchored edits
  on later slices corrupt them. Also: `impl` blocks that were
  private-to-file become `pub(crate)` once the pieces are siblings rather
  than parent/child, not `pub`.
- **If two sessions do run at once, the value is in their *disagreement*, not
  their throughput - and one thing destroys it.** On 2026-08-09 two sessions
  worked this tree simultaneously; every number in that day's four commits was
  reproduced by whoever did **not** produce it, and each caught refuted
  citations the other's sweep had missed, in both directions. What made that
  work is that neither could see the other's intermediate state, so **a wrong
  premise had to survive contact with someone who had not formed it** - which
  is precisely what six days of single-session work could not provide. Two
  sessions that agree on method up front and split strictly by file reproduce
  each other's blind spots at twice the cost. So: split by *claim* and re-derive
  each other's numbers, do not split by directory and trust the reports. And
  note the same day's counter-example - a spawned subagent went idle four times
  without ever producing a report, and its one decisive check had to be run by
  hand. **Parallelism bought nothing that day; independent re-derivation bought
  everything.** Do not read that session as an argument for more agents.

## Traps that are live

**A stale or invalid PPSSPP memory watchpoint address is not harmless - it can stop emulation outright, and two debugger connections racing the same execution breakpoint is how one gets computed.** 2026-08-31, chasing [the M6 lighting thread](handover/m6-authored-lighting-no-hardware-light-slot-found.md)'s live capture. `psp-drive.py restart`'s own internal `Ship_UpdateCraft` breakpoint and a second, independent connection's own breakpoint at the same address raced each other (PPSSPP execution breakpoints are global CPU state, shared across every client), producing a garbage field read (`craft+0x1CC` came back `0`) that got armed as a watchpoint on address `0x30`. It sat at 0 hits, `enabled: false`, looking exactly as harmless as a working-but-quiet watchpoint should - then `E[MEMMAP] Bad memory access detected! 00000030 ... Stopping emulation` killed the session minutes later, identically across two independent boots. Full account, including the fix (take an address a running script already printed instead of re-deriving it with a second breakpoint):
[`ppsspp-debugger.md`](docs/reverse-engineering/ppsspp-debugger.md#a-stale-or-invalid-watchpoint-address-is-not-harmless---it-can-stop-emulation-outright).

**RPCS3 with no PS3 firmware installed fails in three different, non-obvious ways depending on how you ask, and a naive liveness check lies about all of them.** 2026-08-30, first `rpcs3-drive.py browse` run in this checkout. A fresh `~/.config/rpcs3/dev_flash` is empty (firmware is a separate, user-supplied install - a disc's own bundled updater under `data/extracted/<title>/PS3_UPDATE/PS3UPDAT.PUP`, where one is present, is a legitimate source, but installing it is the user's call, not a session default). Symptoms: a normal boot prints `Firmware is missing` and exits; `rpcs3 --no-gui --installfw <path>` refuses outright (`Cannot perform installation in no-gui mode!`); `rpcs3 --installfw <path>` with no `--no-gui` needs a real GUI pass and does nothing headless. **`pgrep -x rpcs3` never matches** - the actual process `comm` is `AppRun.wrapped` (the AppImage wrapper), so any liveness or exit check built on it is silently wrong in both directions; check `/proc/<pid>` or grep `AppRun.wrapped` instead, or better, read `TTY.log` growth the way `Session.wait_for_screen` already does. Once firmware is in place, RPCS3's own first-run **"Welcome" dialog is a native Qt window that overlays every screenshot** (nothing under it is visible, including circuit-select text) and nothing the gamepad sends reaches it - fix is `infoBoxEnabledWelcome=false` in `~/.config/rpcs3/GuiConfigs/CurrentSettings.ini`, not a button. **A manual `rpcs3 ...` launch that sets `DISPLAY=127.0.0.1:77` in one Bash call and launches in a later, separate call drops the export** - shell env state does not persist across this harness's tool calls - and the emulator opens a real window on the user's desktop instead of Xvfb, interrupting their session. `scripts/rpcs3-drive.py`'s `Session` sets `DISPLAY`/`QT_QPA_PLATFORM` in-process via `emulator_env()` and is not exposed to this; prefer it over a manual `rpcs3` invocation for exactly this reason. Separately, `EpilepsyWarning`'s live path needs a `cross` press or a boot stalls there for its whole timeout - see [the boot-chain thread](handover/wipeout-hds-front-end-boots-off-a-chain.md) and `Session.wait_for_screen_pressing`.

**A future bit-exact comparison of the sideshift timer columns will see a tiny negative residue where a clamped port reads exactly `0.0` - that is the original, not a bug.** 2026-08-27, capturing `sideshift-double-tap.inputs` off real PPSSPP for the first time. `Ship_UpdateSideshiftInput_q`'s own countdown (`0x08846a54`, decompiled via ghidra-mcp) is `if (timer > 0.0f) timer -= dt;` - gated, not clamped - so the tick that crosses zero still subtracts a full `dt` and lands slightly negative (`ss_shift_l` froze at `-0.000149006`, `ss_tap_window_l` at `-0.0004569963`, each held bit-exact for the rest of the run), then the field is never touched again. `crates/physics/src/airbrake.rs`'s `advance_sideshift` clamps with `.max(0.0)` instead and converges to exactly `0.0`. Left that way deliberately - every reader in the tree gates on `> 0.0`/`<= 0.0`, so the residue is behaviourally invisible, and the three fields feed `ShipState::hash_state`'s committed golden hashes, so matching it costs a hash regeneration for a cosmetic difference. Full evidence and the exact decompiled block:
[`input-bindings.md`](docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-countdown-does-not-clamp-at-zero-and-the-ports-does---deliberately-left-that-way).
If a bit-exact trace comparison is ever built and wants to match this too, it is a small, mechanical change (drop the three `.max(0.0)` calls in `advance_sideshift`, regenerate the affected golden hashes) - not a data gap.

**A fragment `<LoadXML>` never resolving is not always a missing file - HD's
own disc disables one by misspelling the tag.** 2026-08-26, diffing the three
copies of `speedlap_hud.xml`
([the HUD thread](handover/wipeout-hd-furys-hud-reads-and-the-reader.md)).
`DATA05`'s copy references `HUD_lap_ghost.xml` through `<aLoadXML>`/
`</aLoadXML>` instead of `<LoadXML>`/`</LoadXML>` - one added letter per tag,
which is also exactly `DATA05`'s 2-byte size difference from the otherwise
byte-identical `DATA00`/`DATA06` copies. `LoadXML_Item`'s reader keys on the
literal tag name, so this is not a fragment that fails to resolve and logs a
miss - it is a fragment the reader never looks for at all, deliberately
authored to be skipped without deleting the reference. If a HUD or front-end
composition is ever missing a feature that a sibling copy of the same root
has, check for a misspelled tag before assuming a parser or precedence bug.

**Reaching an arbitrary circuit through the front end is gated by Race Campaign
progression, not by anything `psp-drive.py` or the debugger can skip.** 2026-08-26,
chasing [nothing-airborne-has-ever-been-captured](handover/nothing-airborne-has-ever-been-captured.md).
RaceBox's Custom Race → Track Select is a **fixed** wrapping list of three -
confirmed live, up/down only, `right`/`left` genuinely do nothing there, matching
`docs/reverse-engineering/ppsspp-debugger.md`'s existing note - and the three are
exactly the `PI_Track` entries in `Data\Plugins\PI001\Definition.xml` carrying no
`<Unlock>` element at all: `16_Track` (Talon's Junction), `03_Track` (Moa Therma),
`18_Track` (Metropia reversed). Every other circuit's `PI_Track` node carries
`<Unlock Grid="N">`, and a fresh profile has only Grid 1 of 16 open. **The XML
attribute answers "how many grids" before a single button gets pressed** - no
need to grind blind: `13_Track` needs `Grid6` (five grids of real races first),
where `10_Track` needs `Grid0` (the currently-open grid, so just its own Race
Campaign cell). Check the target's own `<Unlock Grid="N">` before planning a
capture around it, and expect an `N` above 0-1 to mean the capture is not this
session's task.

**A device test with a *uniform* depth buffer cannot exercise any
depth-weighted term - it proves the pass runs, not that the weighting is
right.** 2026-08-26, found reviewing the motion blur chain. `post::motion_blur`
gathers its taps with McGuire's three weights, two of which are gated on
`soft_depth_compare`. Its device test cleared depth to a flat `0.5`, so both
gates returned `1` for every tap in the frame and the two cone terms absorbed
the whole weight - the test was green, and it was structurally blind to the
half of the filter that classifies surfaces. What hid behind that was a real
bug: the third, *ungated* term was written against the tile neighbourhood's
dominant reach instead of the centre pixel's own, so a still surface in front
of a fast one took full-weight taps of the background and read 199 of 255 at
its silhouette. Twenty lines of a second test with two depths caught it
immediately.

Generalise it, because the next weighted gather will have the same shape:
**whatever a filter classifies by, the test has to vary.** Uniform depth, a
single velocity, one material - each turns an assertion about a *decision* into
an assertion about an average. The cheap trick that made the second test easy
is worth keeping too: the pass binds depth as `Float { filterable: false }` and
`textureLoad`s it, so an `R32Float` texture with `COPY_DST` works and takes a
plain `write_texture` - no depth-writing pipeline, and none of `Depth32Float`'s
"you may not copy into me".

The same review turned the rule on the fix and it bit twice more. Splitting
tile-max into two axes needed a frame size the tile edge does **not** divide,
because a power-of-two frame makes every tile square and full - and it needed
motion that varies in *both* axes, because a velocity that is constant down
each column lets any row of the horizontal pass answer for any other. Sizing
the split's intermediate target wrong on purpose left two green tests before
a third one, varying both, caught it. **A test that does not vary what the
code branches on is a test of the average, not of the decision.**

**A reach cap that doubles as a tile size makes a square reduction starve the
GPU.** Same review. `post::motion_blur`'s tile-max reduced each `K x K` square
in one pass, `K` being the blur's own pixel cap - 8 % of viewport height, so 87
at 1080p. That is 7,569 serial `textureLoad`s inside each of only 299
fragments: the right amount of arithmetic (every pixel once) at almost no
occupancy, and it measured 1.5 ms of the chain's 2.6 ms on an Intel Arc iGPU.
Separable - horizontal to `ceil(w / K) x h`, then vertical onto the tile grid -
is the same arithmetic in 24,840 fragments and took the whole chain to 1.2 ms.
The tell is the *fragment count*, not the loop: a fullscreen pass whose target
is much smaller than its source is where this hides.

The reproducible measurement is the table in
[`docs/rendering/motion-blur.md`](docs/rendering/motion-blur.md) - 2.6 ms
square, 1.2 ms separable, 1.1 ms with the reduction stubbed out entirely - not
the stub-the-loop isolation that first pointed at it.

**A worktree sharing `CARGO_TARGET_DIR` with the main checkout compiles against
the *other* tree's uncommitted code.** 2026-08-18, and it cost a confusing hour.
Setting `CARGO_TARGET_DIR=<repo>/target` from `.claude/worktrees/<name>` looks
like a free win - the dependency builds are already there and a workspace build
drops from minutes to seconds - and it works right up until the two trees'
sources differ. Then `just test` in the worktree fails with
`missing field 'lightmaps' in initializer of oag_render::mesh::Model` against a
`mesh.rs` that has no such field, because the field is in the *main* checkout's
work in progress. Nothing in the error names the other directory.

It also produces a subtler failure first: a run that behaves as though an edit
had not landed, then behaves correctly a few commands later. Do not share the
directory. A worktree's own `target/` is gitignored already (`/target/` in
`.gitignore`), and the first build is a few minutes once.

The other half of the same setup is worth knowing: **`data/` does not travel
into a worktree**, being gitignored, so `data/images/` looks empty there.
Symlink `data/images` (and `data/cache`) at the main checkout's, and do **not**
`rm -rf data` to do it - `data/README.md` is tracked.

**A grid of eight identical craft is what an *empty* roster looks like, not what
a broken one looks like - so nothing failed and nothing said so.** 2026-08-18.
Wipeout HD raced eight Assegais because two independent faults each produced an
empty team list, and `livery::teams_for_slots` did the honest thing with one:
every slot gets the player's team. Both are fixed. The general lesson: a
fallback that produces a legible picture is the hardest kind to notice - both
sites now report the *reason* rather than a count of zero, since the report
line used to read `0 team(s)` for two unrelated causes in a row and hid the
second behind the first. Full account, including that HD ships its plugin
definition **five times** across seven archives with the copies disagreeing
(`DATA00` declares twelve teams/28 circuits, `DATA02` eight/eight, precedence
serves `DATA00`):
[hd-status.md](docs/formats/hd-status.md#it-ships-five-times-and-the-copies-disagree).
The two faults, briefly: (1) `race::load`/`boot::definitions` read Pulse's
numbered plugin path (`Data\Plugins\PI001\Definition.xml`) whatever title was
open, where HD names its own (`Data\Plugins\Frontend\Definition.xml`) - fixed
via `oag_title::Title::plugin_definition`, the fourth axis moved into the
title package for this reason. (2) `fexml::expand` refuses a file with no
`<code>` dictionary, and HD's definition is plain `<?xml`; **any new
`read_name` of an XML entry should be `fexml::text`, not `fexml::expand`,
unless the file is known to be shortened** - the trap was already written
down for the PS2 in-race HUD and still caught this caller.

**A name-variant sweep is only as good as its extension list, and one whole
class of PS2 names was missing because `.pct` was not in it.** 2026-08-18.
`Data\Tex\engineFlare\Engine_noise.mip` is on the PS2 disc; the lookup was
wrong, not the disc. The PS2 texture resolver rewrites `.MIP` and `.TGA` to
`.PCT` before it hashes, so **none** of the 18 `.mip` or 32 `.tga` literals in
`SCES_547.48` resolve as spelled and 47 of 50 resolve rewritten, while every
`.pob`, `.vex`, `.bnk` and `.xml` name resolves on both discs untouched.
Mechanism: [texture-names.md](docs/ghidra/functions/ps2-pulse-eu/texture-names.md),
implemented as `oag_pulse::ps2_texture_name`. **The general lesson: when a
whole class of names misses and every other class hits, the answer is a
transformation the loader applies, not more spellings** - the sweep that
missed this tried 60 candidates across path shape and case, and any remaining
PS2 asset gap phrased as "not in the archive set" deserves the rewrite tried
first. [loading-screen.md](docs/ghidra/functions/ps2-pulse-eu/loading-screen.md)
had the `.mip`-to-`.pct` rewrite written down for the loading screen since
2026-08-07, read at the time as a quirk of that one screen. Two more instances
were found and fixed this way on 2026-08-18 - the two exhaust textures and the
loading screen's glow strip (`Data\Defaults\Loading\LoadingPulseOverlay.mip`,
`WADS2.WAD` entry 138, which had been falling back to
`GlowStrip::placeholder` every boot). The sweep behind that covered
`read_name` call sites in `crates/game/src` and `crates/render/src` only; its
two remaining texture-path hits (`livery.rs`, `race/load.rs`) are `mesh::rcs`
callbacks and so PS3-only. **No sweep outside those two crates has been
done**, so treat this as two instances found, not the complete list.

**Fixed in place, 2026-08-28 - the workaround below is no longer needed for
`ps3-hdfury-eu`.** The maintainer ran `scripts/import-ps3-eboot.sh --ps3-cspec`
(Ghidra closed first, as the script requires); verified directly afterward:
language reads `PowerPC:BE:64:A2ALT-32addr-PS3`, still 26,100 functions, real
imports, and `scripts/apply-ghidra-names.py` re-applied all 114 of this
binary's `names.tsv` rows clean. `AssignPs3R2FromOpd.java` now runs as part
of the normal import (`-postScript`), so every function's TOC-relative load
should resolve correctly without `ps3-toc.py` cross-checks going forward -
though any *pre-existing* decompile read or claim made before this date and
not yet re-verified should still be treated with the same suspicion the rest
of this section describes, since it may have been made under the old,
TOC-defective database. The history below is kept for the lesson, not as a
live workaround to repeat.

**The Ghidra bridge cannot run scripts, so the PS3 TOC defect has to be worked
around rather than fixed in place.** 2026-08-18. `GHIDRA_MCP_ALLOW_SCRIPTS` is
unset on this bridge, so both `run_ghidra_script` and `run_script_inline` refuse
with "Script execution disabled" - which means **`AssignPs3R2FromOpd.java`
cannot be applied to the live `ps3-hdfury-eu` database through MCP at all.** The
next session will reach for it, because
[memory.md](docs/ghidra/functions/ps3-hdfury-eu/memory.md) correctly names it as
the fix. The two routes that do work are the full
`scripts/import-ps3-eboot.sh` re-import (needs Ghidra closed, and is heavy), or
**`scripts/ps3-toc.py`, which resolves each function's own TOC against the file
with no Ghidra at all** - added 2026-08-18 for exactly this. Its self-check is
`scripts/ps3-toc.py resolve 0x003914b0 -0x6634`, which must print
`Small is  %3.2f MB (%d bytes)` and not `forward`. **Its `map` mode is validated
against `Collision.cpp`**, whose two attributed addresses are already in
`names.tsv` from independent work. The failure mode it avoids is not a visible
error: in module-B functions Ghidra renders TOC-loaded *floats* as string
pointers, so arithmetic decompiles as `(float)PTR_s_Emp__d_008a754c` and reads
like a nonsense variable name rather than a wrong base.

**The same defect has a second failure mode, and this one doesn't look
wrong.** 2026-08-25. Hunting for where HD's HUD resolves a texture reference's
extension to `.gtf` (see
[the HUD thread](handover/wipeout-hd-furys-hud-reads-and-the-reader.md)),
`get_xrefs_to` on the literal string `SrcRel` reported a real-looking hit:
`FUN_00550eb4`, decompiling (under Ghidra's one global TOC) as a function
that opens with a call site literally named `PTR_s_LoadXML_Item_cpp_...` and
checks `strncmp(param_2, "Values", 0xb)` - a thematically perfect match for
the LoadXML fragment loader this thread was chasing. `scripts/ps3-toc.py toc
0x00550eb4` says its real TOC is `0x008bd3c4`, not the global
`0x008ad4d8` - so every one of those names is Ghidra decompiling module-B
code against module-A's table, and it happened to land on another *real,
coherent* table (LoadXML_Item's actual attribute-name strings live a few
words further into the same data region) rather than garbage. The `0xb`
in that `strncmp` is the tell once you know to look: `"Values"` is 6
characters, but `"Set-Cookie:"` is exactly 11 - the function is
`CCookie.cpp`, confirmed by reading its real TOC's neighbouring strings
(`Set-Cookie:`, `expires`, `domain`, `path`, `secure`) directly with
`inspect_memory_content` rather than trusting Ghidra's resolution. **The
general lesson: `get_xrefs_to` and decompiled symbol names for any
TOC-relative load are only as trustworthy as the target function's TOC
matches the global one** - check first with `scripts/ps3-toc.py toc <addr>`
(`0x008ad4d8` means trust it, anything else means don't), and prefer
`scripts/ps3-toc.py map` (grep the `.cpp` list) plus raw
`inspect_memory_content` reads over `search_strings`/`get_xrefs_to` when
hunting a specific mechanism in a function whose TOC hasn't been checked.

**Even with the TOC defect fixed, a function that calls through an unknown
pointer can still make Ghidra hide a real TOC-relative global behind a fake
local variable.** 2026-08-28, past the `--ps3-cspec` reimport above, on
`Environment_LoadStageTextures` (`0x003d6dc8`,
[zone-effectsettings-loader.md](docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)).
Every ELFv1 indirect call (`lwz r2,0x4(rX)` loading a *different* object's TOC,
then `std r2` / `ld r2` round-tripping the caller's own TOC to the stack
around the call) breaks Ghidra's constant propagation even when it is
provably faithful - so a `-0x58xx(r2)` load *after* such a call decompiles as
an opaque `iStack_NNNN` read, indistinguishable at a glance from a genuine
stack variable or a missing parameter. The tell is `analyze_dataflow`
(backward, from a use of the mystery variable): it terminates at "function
input" rather than a real definition, which is what a value equal to entry
`r2` looks like once the direct chain of assignments is too indirect to
fold. Once recognised, the fix is the same as ever - resolve by hand against
the function's own already-known TOC - but the *recognition* step is new:
don't assume a Ghidra-invented local is really local just because the TOC
defect is fixed; check whether it's actually `r2` in disguise first.
This is binary-wide, not specific to this one string or this one function.

**A related but separate trap, hit later the same session: proximity to a
known-good table is not evidence of relatedness, TOC issue or not.** The
same HUD-thread session chased `get_xrefs_to` hits clustered around
`008ad9e0`, one word away from `LoadXML_Item`'s real attribute-name pointer
table at `008ad9f0` - a dozen-plus hits, in the right file's address range,
looked like exactly the read-the-table code being searched for. It wasn't:
`008ad9e0` is a generic pooled-object free-list head that many unrelated
classes' destructors happen to share, and every one of those "hits" was a
destructor releasing an unrelated smart-pointer field. **Read the actual data at the address in question** - the `.rodata` string
run itself (`0078bf00`, holding the real literal names) rather than the
nearby pointer table that references it (`008ad9f0`, one word from the
free-list head that produced the false lead) - rather than trusting that a
cluster of xrefs near a landmark means the landmark. This one doesn't need a
TOC check to avoid: both addresses were module-A/global-TOC and the xrefs
were technically real; the inference "near my table, therefore about my
table" was simply wrong.

**Deferred from the 2026-08-18 renderer sweep**, so nobody re-derives that these
are open: the **draw path** (inline command-buffer writes on PS3, so no import
census or call graph finds it - it needs a search for RSX method constants);
**`SortRoot.cpp`'s four constructors** (`0x002dbe08`, `0x002dbe50`, `0x002dbe98`,
`0x002dbf40`) - **spent as a render lead, 2026-08-25**: `0x002dbe50` turned out
to have six real callers after all (base cannot be told from complete *and
none is named* was true only for the other three), and chasing them one level
further landed in `DetonatorBomb.cpp` - a bomb weapon's constructor, now named
([`detonator-bomb.md`](docs/ghidra/functions/ps3-hdfury-eu/detonator-bomb.md)) -
tied to `RaceManager_GetInstance()`. `SortRoot`'s only located use is gameplay
code two hops from the render layer, not the "draw-order root" the table entry
guessed; treat that guess as refuted, not merely unconfirmed. See
[renderer.md](docs/ghidra/functions/ps3-hdfury-eu/renderer.md#what-was-deliberately-not-read)
and [the handover thread](handover/hds-see-through-surfaces-draw-with-the-files.md) -
the render draw-order question needs a fresh lead (RSX method constants in
the render layer's own address range), not more of this one; and -
**now closed** - where the engine-owned shader
programs' microcode lives: 124 `SHO` blocks linked into `EBOOT.elf` against
exactly 124 registered names, same container as a `.rcsmaterial`. Two things
stay open there and are on the page. **Only 62 of the 124 name-to-blob pairings
are measured**; the rest match by count alone, because the other 62 names
register by a route a whole-image direct-branch scan does not find - and the
plausible explanation is disproved, not untested: `0x005cd700`, the registry's
second constructor, has **zero branches to it anywhere in the image**. And **no
microcode has been disassembled**, so nothing says what any program does.

**A model reports the right triangle count and still renders nothing - check
the alpha test, not the class table.** 2026-08-17. Wipeout Pure's `Speedup
Pad`/`Weapon Pad` class ids were recovered 2026-08-13, `mesh::build_pads` is
title-agnostic through `vex::classes::V4`, and `Data\Environments\01_Vineta_K\track.vex`
decoded 14 `Speedup Pad` nodes, 952 triangles, correctly - every load report
line said so. **The pad still drew zero pixels**, because `mesh.wgsl`'s
`ALPHA_TEST_THRESHOLD` was an invented `0.5` and the pad's own glow texture
(`speedup_GLOW_KEY.tga`) tops out at alpha 58/255 (~0.23): every fragment of
every cutout batch discarded. A geometry decode that reports a plausible
triangle count is not evidence a model draws anything - `oag-view --pads
--screenshot` (or the new `crates/render/tests/pad_alpha_test_ground_truth.rs`)
is the check that would have caught this, and neither ran until the picture
was asked for directly. Fixed by lowering `ALPHA_TEST_THRESHOLD` to `1/255`
(`0`, one of three real GE references `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`
recovered for this branch) - see `docs/formats/vex.md`'s materials section.

**A one-seed race comparison cannot resolve two adjacent difficulties, and the
test that did it read as a regression on an improvement.** 2026-08-17.
`every_difficulty_is_quicker_than_the_one_below_it` ran one 3,600-tick race per
level and compared the leader's distance. Measured: novice 6,704, skilled 7,199,
elite 7,766, ace 7,810 - the first two gaps are 495 and 567 and **the last is
44**, half a percent, on a system of seven drivers trading places. A change that
made *every* level faster (the stall rescue freeing a wedged craft: elite 8,078,
ace 8,075) duly failed it by three units in eight thousand. Averaged over five
seeds the ordering is real and comfortable - 6,477 / 7,095 / 7,637 / 7,751 -
while elite still beats ace on two seeds of the five. **The test now averages and
prints the spread**, and moved to `crates/game/tests/difficulty_ground_truth.rs`
(which took `race_ground_truth.rs` from 2,464 to 2,387, ratcheted in
`check-file-size.py`). The general form is worth keeping: **an ordering that
holds on the mean and not on one run is the measurement being noisy, not the
setting being broken** - print the spread before touching the thing under test.

**A synthetic race fixture now has a horizon, and it is `STALL_TICKS` long.**
Same day. `race_with_a_grid` builds craft on `Handling::ZERO` with an empty
collision world, so no force law runs and the craft hold full throttle without
moving - which is exactly the condition the stall rescue catches. Any synthetic
race test that ticks past 120 now recovers all seven opponents and zeroes the
throttle on the tick it does. `the_opponents_are_driven_rather_than_parked` runs
`STALL_TICKS / 2` for that reason. A fixture that wants to run longer has to give
its craft handling that moves them.



**Subsystem traps have moved to their pages**:
[surveying a disc](docs/formats/wad.md#traps-when-surveying-a-disc),
[the Ghidra database versus `names.tsv`](docs/ghidra/workflow.md#the-database-can-disagree-with-namestsv-and-the-docs-win),
[PPSSPP session hygiene](docs/reverse-engineering/ppsspp-debugger.md#session-hygiene),
and the `ffmpeg`/`ffprobe`/GStreamer traps on
[frontend-boot.md](docs/architecture/frontend-boot.md). What is left is this
machine and this checkout.

- `data/` could be on `ecryptfs`. **`cp --reflink` does not work** (copying the images
  is a real multi-GB copy), and `ls`/`rg` intermittently fail with "permission
  denied" or "no such file" on directories that plainly exist. `fd`, `find` and
  absolute paths work.
- **Backticks in a justfile are command substitution at parse time**, even
  inside an `@echo` string - an echoed example command once ran a full
  trace-compare on every `just` invocation, `just --list` included.
- **`XDG_CONFIG_HOME` is how you point `oag-game` at a scratch `settings.toml`**
  for A/B config captures without touching `~/.config/oag/`.
- **A niri screenshot piped through `wl-paste` can return the PREVIOUS clipboard
  image.** One lap "verification" shot came out as a menu from an earlier
  session. `psp-trace.py` now detects this and writes `.stale.png` rather than
  citing it, but nothing else does. `niri` screenshots land in
  `~/Pictures/Screenshots/`.
- **`/tmp` here is quota-limited well below the project's own disk.** Extracting
  a large WAD into it once broke the shell entirely - every command failed,
  `echo` included, until the partial extraction was deleted. Scan through
  `oag_assets::Archive` in memory instead.
- **Shipped `section` data is not tidy, and a strict parser refuses real
  tracks.** ~2 % of PVS mask bits name a section the file does not declare, one
  id is authored three times over on four tracks, and 116 PS2 control points
  name a section their file lacks. All benign; see
  [`track.md`](docs/formats/track.md).
- **No capture will ever show the BRIGHTNESS or GAMMA settings.** They apply at
  the blit, which `--screenshot` and the race capture never reach, so a picture
  that looks ungraded is not a bug. The real check is
  `upscale::tests::the_grade_moves_the_picture_in_the_direction_the_setting_names`,
  which builds the actual pipeline on a headless target and reads the pixel back
  (mutation-checked by swapping brightness and the exponent in the shader). It
  **skips with a note where there is no adapter**, so a green CI run does not
  mean it ran.
- **Judge line width at native 480x272 only.** Anything upscaled makes a
  one-pixel authored line a magnification artefact argument rather than a
  fidelity one.
- **`--features native-video` makes the boot fifty times slower, and `just play`
  turns it on** (Linux only, `justfile:25`). Measured on `pulse-psp-eu.chd`,
  caches warm: **4.9 s** of boot with it (intro 2.7, backdrop 2.2) against
  **0.08 s** without - `GstDecoder::open` decodes every frame into memory
  before returning (~288 MiB for the two reels) where the AV1 cache path
  decodes on demand.
  [ADR-0017](docs/architecture/adr/0017-gstreamer-native-video.md) chose that
  deliberately, costed at 270 frames/53 MiB for the backdrop (the intro is
  1200 frames). The boot prints its own timings now, so this is re-measurable
  rather than folklore. Making the GStreamer path lazy would remove the wait
  rather than hide it, and is not done.
- **Two things convert into `data/cache/movies` and they must never overlap**
  - the boot's own two reels and the `--prefetch` worker both shell out to
  `ffmpeg`, and two processes writing one cache file corrupts it rather than
  racing safely. `Session::start_prefetch` is the only thing serialising them
  now that movies run on `boot::MediaWorker`. `--refresh-video` (2026-08-12)
  makes the worker stop skipping cached movies, so combined with `--prefetch`
  it re-converts the boot's own two reels too - wasteful but still ordered.
- **The loading screen's bar vanished on every ordinary boot, and the gate
  wasn't the cause.** `0 / 0 converted` beside a full `100%` bar was real: the
  boot's media phase reported no counts at all, and the display bundled bar,
  counts and percentage into one row keyed on `total > 0`. Fixed 2026-08-12 by
  giving the phase real counts (`boot::MediaPlan::loads`,
  `boot::MediaWorker::progress`) rather than loosening the gate - **a display
  gate keyed on "is there data" reads "nobody reported" and "nothing to
  report" as the same thing.** The bar is now continuous (each load fills its
  own slice from its own step, so a minute-long transcode crosses 40%->60%
  rather than holding at 40%); the invariant to preserve is monotonicity.
  [frontend boot](docs/architecture/frontend-boot.md#loading-is-not-transcoding).
- **`--prefetch` could not fill the movie cache on a `native-video` build, and
  said it had.** Found 2026-08-12. The GStreamer path writes no cache file -
  it decodes into memory with a `{key}-gst` pseudo-path - so
  `prefetch::convert`'s `report_picture` saw a picture and called it
  converted, while the planning pass found nothing cached and relisted the
  same 22 movies every run. Fixed by making `prefetch::convert` set
  `movie::Decode::prefer_cache` unconditionally (filling the cache is the
  worker's whole job, not a preference); confirmed empirically, the cache
  went from 9 to 21 `.ivf` files in three minutes. **The general trap: a
  success/failure check that asks "did this produce a picture?" cannot see
  that the *side effect* the caller wanted did not happen.**
- **An existing AV1 cache file now beats the platform decoder with no flag,
  and ADR-0017's default still stands** - compatible because 0017 is only
  about the case with nothing cached, where GStreamer still wins;
  `open_psmf` now asks `movie::cached` first. Measured on `pulse-psp-eu.chd`,
  both reels: 4.80 s of media phase through GStreamer, 36.08 s once to build
  the cache (intro 30.33, backdrop 5.74), 0.06 s on every run after.
  `--prefer-av1-cache` builds it for the boot's reels, `--prefetch` for all
  22, `--refresh-video` implies it. ADRs are immutable, so nothing edited
  0017; making the *uncached* default the cache would need a new one.

**The Ghidra bridge's `switch_program` can silently no-op, and its own reply
says success when it does.** 2026-08-19. Four of this project's binaries share
the bridge's display name `BOOT.BIN` (`psp-pulse-usa`, `psp-pulse-eu`,
`psp-pure-usa`, `psp-pure-eu`), and `rename_function_by_address` /
`create_function` both ignore the `program` field on the request and act on
whichever program the bridge currently considers *truly* active - unlike
`get_function_by_address`, which resolves `program` correctly on every call.
`switch_program` is supposed to be the fix, but confirmed live: it returns
`{"success":true}` while leaving the true active program completely
unchanged, for any of the three non-`psp-pulse-usa` `BOOT.BIN`s. Tried
switching away to a known-reachable program first in case it was
call-order-dependent - it wasn't; those three are unreachable for writes
through this bridge session regardless of sequence. `ps2-pulse-eu` and
`ps3-hdfury-eu` are unaffected (uniquely named, genuinely switchable).

The failure mode is not a loud error, it is silent cross-binary corruption:
a `just apply-names` run for, say, `psp-pulse-eu` reports success while
writing `psp-pulse-eu`'s documented names onto `psp-pulse-usa`'s addresses
instead, at whatever address happens to also be a function start in the
wrong binary. This had been happening for a while - live `psp-pulse-usa` on
2026-08-19 carried 234 stray names (91 mismatched, 143 unsanctioned) before
cleanup, every one traceable to a name legitimately documented at a nearby
address in `psp-pulse-eu` or `ps2-pulse-eu`. `scripts/apply-ghidra-names.py`'s
`Bridge.switch_program` (and `scripts/audit-ghidra-names.py`'s `--prune`, via
`ensure_active`) now distrust the reply and re-read the bridge's true active
program afterward with no `program` parameter, aborting loudly instead of
writing blind - so `just apply-names` now correctly refuses `psp-pulse-eu`,
`psp-pure-usa` and `psp-pure-eu` outright rather than corrupting whichever
program was truly active. **Those three binaries cannot be named through this
bridge at all right now** - that is a `bridge-mcp-ghidra` limitation, not
something fixable from either script. `psp-pulse-usa` was cleaned (audits
clean as of 2026-08-19); `psp-pulse-eu`/`psp-pure-usa`/`psp-pure-eu` still have
no way to apply their `names.tsv` live until the bridge can disambiguate
same-named programs, or until each is loaded in a Ghidra project without a
`BOOT.BIN` name collision.

**`psp-pulse-usa`'s Ghidra DB stores every `jal` target pre-relocation, so
`get_xrefs_to`/`get_function_callers` return "none" for almost all of them.**
Found 2026-08-11, re-derived independently 2026-08-19, and it cost most of a
session before the mechanism was understood. Ghidra's *static* disassembly
bytes for a `jal` hold the unrelocated `R_MIPS_26` field - decoding them with
the ordinary MIPS rule lands on a small, bogus sub-`0x08000000` address, not
the real target; the *running* program's bytes at the same address are
already relocation-patched and decode correctly. The fix: add this program's
own image base (`0x08804000`, from `get_current_program_info`) to the
low-26-bits-shifted-left-2 value read from Ghidra's static bytes -
`target = image_base + ((word & 0x03FFFFFF) << 2)`, no `(addr+4) &
0xF0000000` term. Confirmed on every `jal` checked so far, including all six
inside `Body_ResolveContactPair`. **Practical consequence**: when an xref
tool says "no callers", it is not evidence of that - search
`search_instructions` on the zero-padded *relative* operand
(`jal 0x000645cc`, not `jal 0x0886a5cc`) instead of trusting the "none"
result. Full derivation, and confirmation this is not database corruption:
[contact-response.md](docs/ghidra/functions/psp-pulse-usa/contact-response.md#its-call-target-needed-the-image-base-added-by-hand---ghidras-own-bytes-are-pre-relocation).
**Settled 2026-08-27: it is not regional, it is every one.** A different
session had independently concluded (2026-08-17, since retracted - see
[weapon-fire.md](docs/ghidra/functions/psp-pulse-usa/weapon-fire.md#read-this-first-the-disassemblys-addresses-are-image-base-relative))
that two address ranges were exempt; `search_instructions` across the whole
program found zero `jal` anywhere - not just those two ranges - that
displays an already-relocated (`0x08`-prefixed) target, out of 524,728
instructions scanned, and the specific counter-example instruction that
claim cited does not exist in the database. Apply the workaround
unconditionally to a static `jal` reading here; there is no exempt region.
Whether this is a general PSP-ELF-relocation trait or specific to how this
project's importer handled `BOOT.BIN`, and whether it is related to the
decompiler failing on several functions in the same file, are both still
open - no evidence ties either to the mechanism above.

**It is not just `jal`: `lui`/`lw` HI16/LO16 pairs and raw 32-bit data words
carry the same pre-relocation values, and the fix generalises.** Found
2026-08-27 while chasing the three unidentified globals on
[zone-mode.md](docs/ghidra/functions/psp-pulse-usa/zone-mode.md#not-determined)'s
case-4 arm: `disassemble_bytes` on `lui a0,0x2b` / `lw a0,-0x37c8(a0)` at
`0x088444b0` gives a naive address of `0x002ac838` - small, and in no mapped
section - and `get_xrefs_to` on that naive value duly finds nothing, same
failure shape as a `jal`. Adding the image base the same way the `jal` fix
does (`0x08804000 + 0x002ac838 = 0x08ab0838`) reproduces the address the
2026-08-24 pass already had by hand, and does the same for the other two
globals on that page (`0x08ab2120`, `0x08ab10e8`) against their own naive
values. A 32-bit data word is the same story one level removed: the pointer
stored at that now-correct `0x08ab0838` slot reads `88 59 27 00` (LE
`0x00275988`), which resolves to nothing on its own but to the `"PickupBackground"`
string table entry once the same `+0x08804000` is applied
(`0x08804000 + 0x00275988 = 0x08a79988`, confirmed by `inspect_memory_content`
landing on readable text). **Same fix, three different encodings** - a
26-bit jump target, a 16+16 hi/lo pair, and a plain 32-bit word - so the
practical rule widens: any address-shaped value read off this program's
static bytes (not already surfaced via a working xref) is a pre-relocation
candidate, not just call targets. `get_xrefs_to`/`get_xrefs_from` on the
naively-read address will report nothing either way, which reads exactly
like "no reference" and is not.

**`psp-pulse-eu` has the same `jal` wart, not just `psp-pulse-usa`.** Found
2026-08-31 tracing `World_LoadTrack_q` (`0x088835f0`, EU) for
[the M6 lighting thread](handover/m6-authored-lighting-no-hardware-light-slot-found.md):
its call into `World_CollectMarkerLists` (`0x0887a1c4`) decompiles as
`func_0x000761c4(...)`, and `get_xrefs_to`/`get_function_callers` on
`0x0887a1c4` both return empty despite a live PPSSPP capture already having
confirmed the call executes. `0x000761c4 + 0x08804000 = 0x0887a1c4` exactly -
same fix, second binary. This nudges the still-open "general PSP-ELF trait
vs. this project's importer" question above toward the importer, since both
binaries went through the same procedure; not settled by one data point, but
worth weighing next time it comes up. Full account:
[`lighting.md`](docs/ghidra/functions/psp-pulse-eu/lighting.md#a-fourth-pass-world_loadtracks-own-body-is-now-disassembly-verified-to-hold-no-consumer-and-its-name-is-doubtful).

## Verification status: what to lean on

**The disc chooser was driven end to end on this machine** (2026-08-18), which
is worth writing down because there is no automated way to do it. `just launch`
listed all seven images under `data/images/` with the title, platform and serial
read off each disc; `hdfury-ps3-eu.iso` came up red and unselectable beside the
decrypted twin it shares a serial with; and two Down presses plus Return booted
`pure-psp-eu.chd` through the ordinary loading screen. Whole survey of seven
images: 0.85 s.

**How to repeat it, and the two dead ends.** The input has to go in over
**X11** - `xdotool` cannot reach the window under Wayland at all, so
`unset WAYLAND_DISPLAY` before the run is what makes a scripted press land, and
`xdotool key --window <id>` does not work either; focus the window and use plain
`xdotool key`. **Xvfb is not a way round having a real display here**: the run
starts and the survey prints, but Mesa reports *"vulkan: No DRI3 support
detected - required for presentation"* and nothing is ever drawn or ticked, so
no press is acted on. A headless *capture* is no help either - see the entry in
"Scope decisions" about the chooser having no `--until`.

**The cross-platform gate now runs four layers, not one** (2026-08-15). CI's
`determinism` job runs `oag-core`, `oag-physics`, `oag-gameplay` and the new
`oag-ai` scenario on Linux, Windows and macOS, in release *and* debug. The AI
one exists because the physics gate drives a **scripted** input, so no
controller decision was ever hashed: `oag_ai::Line::curvature` called the
platform's `acos` from the day it was written and nothing failed, because
nothing cross-platform ever ran a driver. It calls `oag_core::math::acos` (our
own libm) now. Measured on this machine before the swap: the two implementations
differ on 8.6 % of the domain by up to one ULP. Full account, including what is
still uncovered, on
[determinism.md](docs/architecture/determinism.md#each-gate-covers-the-layer-below-it-and-one-thing-more).


**Measured against the original running**, all with full method and numbers on
their own pages - lean on these, don't requote them: the forward force law end
to end, lateral grip (0.9985 of the disc value), the yaw accumulator term by
term (99.94 %), the inertia tensor on all three axes, the pitch step response,
the contact friction coefficient, the mag-lock probe, the authored `Start
Position` frame, and the chase camera (RMS 0.008 world units over 150 ticks) -
[`docs/physics/`](docs/physics/README.md),
`crates/game/tests/chase_camera_ground_truth.rs`. The whole-lap result has its
limit in the same breath as its number: over the clean window (first ~170
ticks) position tracks to 6.54 units worst / 2.41 mean, but nothing after tick
256 measures our physics against the current capture, and `grounded` is a
constant column, so agreeing with it says only that our ship also never left
the ground.
[oag-trace.md](docs/tools/oag-trace.md#reseeding-and-why-a-long-comparison-needs-it)
carries both numbers.

**Instruction-level reading with no runtime leg yet.** Mag-lock's blend weights
and its `|h - d| > 5.0` fallback; the swept collision path; everything airborne;
both sideshift gestures. Single-sided wall rejection is measured against the
*data* rather than against the original running: **2,076 of 2,084 wall triangles
on `16_Track` (99.6 %)** are wound toward the nearest point of the track's own
spline, which is what makes reproducing the original's
`dot(boxCentre - sample, n) > 0` rejection faithful rather than merely literal.

**The determinism gate reaches the simulation** (`dd53c1f`).
`oag_physics::probe` steps the same `oag_physics::step` the race loop steps, over
a scripted input and a synthetic corridor built in the file, so it runs in CI on
all three OSes rather than needing a disc image. Two mechanisms keep the hashed
field list from rotting and **both are needed**: exhaustive destructuring makes a
new `ShipState`/`Body` field a compile error, and a perturbation test proves each
field actually reaches the hash rather than merely being bound - that second test
caught a real hole on its first run, since `time_since_landing` defaults to `1.0`
and perturbing it *to* `1.0` was a vacuous no-op. What it does not cover is
stated in its module docs: maglock, reset, and the swept tunnelling path.
`probe::environment` now crosses a speed pad twice, once on each side of the
`<Special speedpad_jump>` tilt threshold, because until it did, step 15 of 15 was
the one force term the gate did not cover at all - and a new branch inside an
uncovered term lands with the hashes *not* moving, which is the worst outcome
available.

**Two standing rules.** When the determinism test fails, find the bug - never
update the reference constants to make it pass. And never raise a confidence
score on the strength of the simulation agreeing with itself.
