# Code review, 2026-08-18: the whole workspace, focused on the generation split

A full-workspace review pass, run as five parallel read-only reviews (generation
split and its consumers, formats/disc/assets, simulation and determinism,
render/view/input/audio, infrastructure and docs) with every headline finding
re-verified against the source by the orchestrating session before it was
graded. The occasion is a design question as much as a defect hunt: a possible
future **global racebox** in which models, tracks, physics tuning and craft
rosters mix across titles and generations (a Pure ship on an HD track, possibly
online), which means full per-title separation is not the end state. The
[racebox assessment](#the-racebox-question) below is graded against that goal.

Nothing in this review was fixed on the way past; it is findings only.

## Grading rubric

Every finding carries five grades:

| Grade | Values | Meaning |
| --- | --- | --- |
| **Impact** | High / Medium / Low | What it costs if left as is: High breaks a stated invariant or a player-visible behaviour, Medium degrades one path or misleads a contributor, Low is polish. |
| **Priority** | P0 - P3 | P0 fix now (cheap and load-bearing), P1 next working session on that area, P2 when the area is next open, P3 opportunistic. Priority is impact times cheapness, so a trivial High fix outranks an expensive one. |
| **Confidence** | 0 - 100 | Per the [confidence rubric](../reverse-engineering/confidence-rubric.md)'s spirit: 95 means re-verified against the source by a second reader in this review, 80 - 85 means one reviewer verified it with exact code in hand, 70 means a design judgement rather than a checkable fact. |
| **Faithfulness** | yes / no / n.a. | Whether the finding is about divergence from the original game's observed behaviour (yes), an engineering defect with no original to compare (no), or not applicable (docs/infra). |
| **Effort** | S / M / L | S is a line or a guard, M is a bounded refactor in one crate, L is cross-crate design work. |

IDs: **D** determinism/simulation, **S** generation split, **F**
formats/disc/assets, **R** render, **U** input/audio/view, **G** game shell,
**I** infrastructure and docs.

## Headline findings

1. **D1** - the rocket fan's spread goes through the platform's `sin_cos`, into
   hashed state, and no gate can see it. The one confirmed determinism hole in
   the workspace.
2. **S1** - `oag-hd` was never added to `GAMEPLAY_CRATES` in
   `scripts/check-dependency-rules.py`, so dependency rule 1 silently does not
   cover one of the three title packages. One-line fix.
3. **D2** - Zone mode's damage rules contradict the measured evidence in
   `oag-race`: the engine's Zone likely takes double the original's contact
   damage.
4. **S2** - HD's own HUD tables have zero non-test consumers; every title is
   served Pulse's HUD layout names and survives on path-normalisation luck.
5. **F1** - a crafted PS2 texture entry panics the parser; the PSMT4 guard was
   never mirrored for PSMT8.

## The racebox question

**Short answer: the architecture is closer to racebox-ready than the grep
counts suggest, and the deliberate refusals (no `trait Game`, no `enum Title`)
are the right shape for it, not obstacles.** What blocks a mixed-title session
is concentrated in `oag-game`'s session plumbing, not in the split itself.

What already works in the racebox's favour, verified in this review:

- **`oag_title::Title` is a `&'static` struct of tables selected once**
  ([ADR-0022](../architecture/adr/0022-title-packages.md),
  [ADR-0023](../architecture/adr/0023-boot-sequence-as-title-data.md)). A
  racebox does not need behaviour dispatch; it needs several titles' *data*
  live at once, and tables deliver that where a `trait Game` would not.
- **No singletons anywhere.** No `OnceLock`, `lazy_static`, `static mut` or
  `thread_local` in any non-test code. `Archives::open(source, title)` is
  instance-based, so two titles' archives can already coexist in one process.
- **Format decoding is chosen by each file's own version word and sniffed byte
  order, never by the disc** ([ADR-0004](../architecture/adr/0004-asset-pipeline.md),
  ADR-0022 item 1). A mixed session hands any blob to `oag-formats` and it
  decodes without being told whose it is. This is the single most
  racebox-enabling property in the codebase.
- **The simulation is title-free.** `oag-physics`/`oag-race`/`oag-ai`/
  `oag-gameplay` have no title-crate dependency; per-ship tuning is injected
  from the player's own disc through one documented seam
  (`oag_gameplay::handling_for`). Mixing a Pure ship's handling with an HD
  track is, at the data level, already expressible.
- **The DLC-pack mount is a working prototype of the racebox's shape**:
  foreign content mounted behind a base source, manifests concatenated. It is
  gated to Pulse packs today, deliberately, but the mechanism is the one a
  mixed session needs.
- **Determinism discipline (lockstep's prerequisite for online) is strong**,
  with exactly one confirmed hole (D1) and one checker scope gap (I6).

What actually blocks it, in dependency order:

1. **The session shape is one-source-one-title.** `Opened` carries exactly one
   `&'static Title` and one `Archives`; `race::Options.source` is a single
   string; `race::load` reads track, handling, liveries, HUD and roster from
   that one mount. A racebox needs the load path split into per-asset
   `(Archives, &Title)` pairs (track-source vs craft-source at minimum). This
   is the L-sized design work (S4).
2. **`open_source` is an implicit, hardcoded title registry** (S5): a probe
   cascade of HD-then-Pulse-then-Pure spread over `crates/game/src/title.rs`.
   A fourth title, or a session that opens two, edits this chokepoint. A small
   explicit registry (a `&'static [TitleProbe]` table, still no trait) fixes
   it in the codebase's own idiom.
3. **Cross-title asset naming currently rests on normalisation coincidence**
   (S6): consumers spell Pulse entry names and HD happens to fold onto them
   via PSARC case-normalisation. There is no "how does this title name a ship
   model / HUD layout" axis on `Title`. The corpus shares a lineage so the bet
   keeps paying, but 2048 (a different engine generation) is where it likely
   stops. The fix is the ADR-0022 pattern applied again: promote the axis when
   the divergent corpus exists, and the racebox is exactly the forcing
   function ADR-0022 item 4 waits for.
4. **Verification has no cross-title story** (S8): `oag-trace` is Pulse-only,
   so a mixed-generation physics claim cannot be checked against any original.
   For a racebox this is arguably fine (there *is* no original for a Pure ship
   on an HD track: faithfulness applies per-component, not to the mix), but it
   should be a stated decision, not an accident.
5. **The physics constants are Pulse's, in-engine, deliberately** (ADR-0022
   stage 6, gated on M4's exit). Every number already has a named seam and a
   classification, so the eventual move into title packages is mapped and
   mechanical. Do not move them early for the racebox's sake; the racebox
   inherits the move when M4 closes.

One naming note: **Racebox is the original games' own term** - Pulse ships the
mode and HD's front end still carries a `FE_RACEBOX` redirect (see
[hd-frontend.md](../formats/hd-frontend.md)). A cross-title racebox is this
project's own mode built on that vocabulary, which also means its menu tree is
project-authored (like the existing menus) and faithfulness pressure applies
only to the components, not the mode itself.

## Findings

### Determinism and simulation

| ID | Finding | Impact | Priority | Conf | Faithful | Effort |
| --- | --- | --- | --- | --- | --- | --- |
| D1 | `Quat::from_axis_angle` at `crates/weapons/src/projectile.rs:868` (and `spawn.rs:92`) reaches the platform's `sin_cos`: workspace glam is `["std", "scalar-math"]` with no `libm` feature, so the rocket fan's spread directions - which land in `Projectile::velocity`, hashed state - differ across OSes by ULPs. Three gates all miss it: `check-transcendentals.py` matches textual method names only, the gameplay determinism test spawns rockets with hand-written velocities and never calls `launch()`, and the physics probe fires no weapon. First rocket fired with nonzero `<Rocket spread>` on two OSes desyncs the world hash. | High | P0 | 95 | no | S |
| D2 | `damage_rules(Mode::Zone)` (`crates/gameplay/src/world.rs:132`) says `weapons: true` while `Mode::weapons_enabled(Zone)` (`crates/race/src/mode.rs:223`) is `false` with measured front-end evidence (explicit `<Weapons>Off</Weapons>`, 2026-08-10). The weapons flag halves *all* damage when off, contact included, so the engine's Zone takes roughly 2x the original's contact damage; survival times and scores diverge. The `mode.rs` evidence is the stronger of the two contradicting claims. | Medium | P1 | 95 | yes | S |
| D3 | `hash::write_ship` (`crates/gameplay/src/hash.rs:110-136`) destructures every struct exhaustively except `oag_ai::Driver`, whose 7 fields are read by name. A field added to `Driver` compiles clean and silently drops out of the hash - the exact drift the module's own doc promises is a compile error. Latent, not live: all current fields are covered, but `Driver` is the struct that grew fastest last week. | Medium | P2 | 85 | no | S |
| D4 | `crates/physics/src/probe.rs:406-409` claims no probe script crosses a pad, so `pad_timer`/`pad_direction` hash as fixed bytes; false since `PAD_PLAIN`/`PAD_TILTED` landed (`environment()` crosses pads at ticks 1600/1900). Misleads the next reference regeneration. | Low | P2 | 85 | no | S |
| D5 | Doc drift: `Mode::has_opponents` doc says "`false` for every mode" then returns `true` for SingleRace; `crates/core/src/tick.rs:8-16` still calls 60 Hz "not yet a measurement" while `docs/architecture/determinism.md` records fourteen 1/60 sub-step sites as evidence. | Low | P3 | 85 | no | S |
| D6 | `crates/core/src/tick.rs:115` truncates `(accumulated_nanos / dt) as u32`; needs ~828 days of accumulated time in one call to matter, and the 8-tick cap bounds the behaviour either way. Informational. | Low | P3 | 80 | no | S |

Verified clean in the same pass, worth recording: no f64, `mul_add`, hash-map
iteration, wall clock, OS entropy, SIMD or threads anywhere in the five
simulation crates' non-test code; the `World` snapshot claim is true (every
field transitively `Copy`, no heap anywhere in it); `TickClock` has no
spiral-of-death or accumulator defect; NaN paths are consistently guarded; lap
counting's wrap-plus-two-half-gate design survived attempts to construct a
miscount.

### The generation split

| ID | Finding | Impact | Priority | Conf | Faithful | Effort |
| --- | --- | --- | --- | --- | --- | --- |
| S1 | `GAMEPLAY_CRATES` in `scripts/check-dependency-rules.py:36-45` lists `oag-pulse`, `oag-pure` and `oag-title` but not `oag-hd`, so dependency rule 1 does not cover one title package: `crates/hd` could grow a `wgpu` or `oag-render` dependency and pass `just check-deps`. The script's own comment states the invariant ("a title package joins in the same change that creates it"); `oag-pure` joined, `oag-hd` did not. | High | P0 | 95 | n.a. | S |
| S2 | Every title is served Pulse's HUD: `hud_layout` (`crates/game/src/race/hud.rs:25-31`) returns `oag_pulse::hud::layouts::*` for all modes and titles, and `crates/game/src/hud.rs` is a `pub use oag_pulse::hud`. HD's own tables (`crates/hd/src/hud.rs`: layouts, skins, ROOTS, TEXTURES) have zero non-test consumers; HD works only because PSARC normalisation folds `Data\XML\Arcade_HUD.xml` onto `/data/xml/arcade_hud.xml`. HD's SPEED_LAP, DETONATOR and DUEL layouts and the wo3/2097 retro skins are unreachable. | Medium | P1 | 95 | yes | M |
| S3 | `crates/game/src/main/prepare.rs:115` falls back to `oag_pulse::race::DEFAULT_TEAM` for every title although the per-title axis exists (`Title::race.team`) and the track default beside it already uses `title.race.track`. Works on HD by case-folding luck only. | Medium | P1 | 95 | no | S |
| S4 | The session shape is one-source-one-title: `Opened` (`crates/game/src/title.rs:16-31`) carries one `&'static Title` and one `Archives`; `race::Options.source` is a single string; `race::load` reads everything from that one mount. This is the structural blocker for any cross-title racebox. The DLC-pack mount (foreign content behind a base source) is the existing mechanism to generalise from. | High (racebox) | P2 | 90 | n.a. | L |
| S5 | `open_source` (`crates/game/src/title.rs:51-76`) is a hardcoded three-title probe cascade (PSARC means HD, else Pulse, else Pure via Pulse's deny-list) acting as the de-facto title registry; `Opened` cannot express a second title, and every new title edits this function plus the frontend state-name unions in `crates/game/src/frontend.rs` (lines 86-96, 139-162, 868-880). | Medium | P2 | 90 | n.a. | M |
| S6 | Cross-title asset addressing rests on name-normalisation coincidence: consumers spell Pulse entry names (`race.rs:102` ship spellings, `race/assets.rs:188`, `boot.rs:1364` and `race/hud.rs:90` routing through `oag_pulse::read_image`, `boot/movies.rs:279` `LOOSE_MOVIES`) and other titles resolve only because PSARC/WAD normalisation happens to fold onto them. No `Title` axis says how a title names a ship model or HUD layout. The bet likely fails at 2048. | Medium | P2 | 90 | n.a. | M |
| S7 | `oag-render` compiles Pulse tables in: `loading.rs:71` re-exports `oag_pulse::loading` (the loading wave is Pulse's for any source) and `mesh.rs:43` re-exports `ANIMATED_TEXTURES` (mostly retired to a cross-check, but `is_blink_light_texture` still keys ship paths off Pulse texture names). Allowed by rule 1's direction; a racebox needs these per-session rather than compile-time. | Low | P3 | 95 | yes | M |
| S8 | `oag-trace` is Pulse-only (`crates/trace/src/main.rs:1139,1159,1554` call `pulse::open` directly), so the M3 verification harness cannot capture or compare any other title. Cross-generation physics claims are structurally unverifiable today. | Medium | P2 | 90 | n.a. | M |
| S9 | `PITCH_STAND_IN` (`crates/gameplay/src/handling.rs:220`) is one title's tuning block hand-carried into shared simulation code, substituted when a title authors no `<pitch>`. Well-documented and loudly reported at load; noted because a racebox multiplies the pattern ("whose numbers fill the gap?") and this is the template answer: substitute loudly, name the donor title. | Low | P3 | 90 | yes | S |
| S10 | HD's archive-overlap ordering decides content by fiat (`crates/title/src/lib.rs:265-284`: the plugin definition exists in 5 of 7 archives declaring 12 vs 8 teams; first-listed wins by documented choice, not measurement). A racebox mounting several titles' archives into one search order inherits and amplifies exactly this ambiguity class via `holder_of`'s first-hit rule. Known open thread; graded here for its racebox multiplier. | Low | P3 | 90 | yes | M |
| S11 | `crates/game/src/prefetch.rs:457` hardcodes `oag_pulse::TITLE`, so the whole prefetch/convert pipeline is Pulse-only by construction. | Low | P3 | 90 | n.a. | S |

Positives verified in the same pass: `oag-assets` reaches Pulse only as a
dev-dependency for ground-truth tests (the mechanism/table split is real);
duplication across `crates/pulse`/`pure`/`hd` is minimal and deliberate (thin
`open()` wrappers plus const tables; the crates are data, not logic); and the
`Title`/`Archives` design has no global state to unwind.

### Formats, disc and assets

| ID | Finding | Impact | Priority | Conf | Faithful | Effort |
| --- | --- | --- | --- | --- | --- | --- |
| F1 | `crates/texture/src/ps2_texture.rs:396` indexes `texels[psmt8_offset(x, y, width)]` with no minimum-size guard; PSMT4 has one at line 383 (`width < 128 \|\| height < 128`) but PSMT8's permutation is only valid for power-of-two widths >= 16, so a crafted 8x8 PSMT8 entry with self-consistent GIFtags panics the parser (verified arithmetic: index 77 into a 64-byte buffer). Reachable from `Archives::read_font` and any archive browse of a hostile image. | Medium | P1 | 95 | no | S |
| F2 | `crates/formats/src/psarc.rs:457-459`: `Vec::with_capacity(size)` uses the entry's u40 declared length (up to ~1 TiB) before `entry_blocks` validates the block range; a crafted PSARC commits the allocation first. Mitigated in practice by the assets layer calling `entry_range` first, but `read_entry`'s own contract does not require that. | Medium | P2 | 85 | no | S |
| F3 | `crates/disc/src/source.rs:72`: `len.div_ceil(SECTOR_SIZE) as u32` truncates; hostile ISO 9660 multi-extent joins can sum to 8 TiB, div_ceil hits 2^32, the cast yields 0 and the read returns an empty Vec with `Ok` - silent wrong data instead of an error. | Low | P2 | 85 | no | S |
| F4 | `crates/disc/src/image.rs:154`: `entry.lba + u32::try_from(offset / sector_size).unwrap_or(u32::MAX)` overflows u32 for the same hostile entries (debug panic, release wrap to a wrong sector). Same root cause as F3; one checked add fixes both. | Low | P2 | 85 | no | S |
| F5 | `crates/disc/src/chd_source.rs:358`: a CHD track line with a missing or unparseable `FRAMES:` field defaults to 0, so `sector_count = 0` and every read fails `SectorOutOfRange { total: 0 }` - bricked with a confusing error instead of falling back to `header.unit_count()` as the no-metadata path does. | Low | P2 | 85 | no | S |
| F6 | `read_sector` contract enforced inconsistently: `raw_source.rs:49` only `debug_assert`s the buffer length (release silently reads across sector boundaries) while `chd_source.rs:326` panics. The trait is public; misuse gives silently wrong data on one impl and a panic on the other. | Low | P3 | 85 | no | S |
| F7 | `crates/texture/src/gtf.rs:480-484,523-527`: every `decode::level` `None` is reported as `Error::Swizzled`, including the too-short-texel-slice case, so a truncation is diagnosed as "swizzled, not implemented" - a misleading error in a codebase that prizes honest ones. | Low | P3 | 85 | no | S |
| F8 | Info: dead bounds check in `vex.rs:1302-1308` (loop condition already guarantees it); PSARC block-width probe (`psarc.rs:544-556`) could tighten `needed` using each entry's block span for free; `crates/assets/src/psarc.rs:122-128` allocates a normalised String per stored path per lookup, O(paths) allocations per read across HD's seven archives at load time. | Low | P3 | 80 | no | S |

Positives: validate-before-allocate is applied consciously in ~30 parsers with
the hostile case named each time (F2 is one of only two slips found);
byte-order handling for HD is sniffed per file from each format's own magic so
no parser takes a platform parameter; malformed-input tests exist for every
sampled parser, with the gaps lining up exactly with F1/F3.

### Render

| ID | Finding | Impact | Priority | Conf | Faithful | Effort |
| --- | --- | --- | --- | --- | --- | --- |
| R1 | `crates/fx/src/psys.rs:749-762`: the emission loop never terminates for an `EmitterSpec` with `interval_ticks == (0, 0)`. No disc asset can produce one (`Effect::parse` clamps to >= 1) but `EmitterSpec` is a pub struct with pub fields inviting direct construction; a hand-built looping emitter hangs the frame loop. A min-1 clamp or debug_assert at the use site closes it. | Medium | P2 | 85 | no | S |
| R2 | `crates/mesh/src/capture.rs:277,286,314`: out-of-range texture slots clamp to the *last* texture bind rather than slot 0, the documented white fallback - an inconsistent Model silently paints with an arbitrary texture instead of honestly white. | Low | P3 | 85 | no | S |
| R3 | `crates/mesh/src/mesh_render/uniforms.rs:372`: `FOG_SIZE` is actually `size_of::<Scene>()` (Fog + Light, 96 bytes); the name and the "fog" labels mislead anyone extending `Scene`. Layout itself verified correct against `mesh.wgsl`. | Low | P3 | 85 | no | S |

Verified clean: every WGSL/Rust struct pair matches byte for byte (Uniforms,
Fog, Light, Scene, TexAnims, NodeAnims, the post-process constant blocks, the
shared 68-byte `GpuVertex`); parse-boundary index validation makes the
renderer's direct indexing safe; no wall clock in `crates/render` (the orbit
viewer's `Instant` is scoped and documented); `MAX_TRAILS` duplication is
pinned by a `const` assert so drift fails the build - do not re-flag it.

### Input, audio, viewer

| ID | Finding | Impact | Priority | Conf | Faithful | Effort |
| --- | --- | --- | --- | --- | --- | --- |
| U1 | `crates/audio/src/output.rs:89-107`: `open()` builds an f32 stream from `default_output_config()` without checking `sample_format()` and has no i16/u16 fallback; on a device whose default format is not f32 (bare ALSA hw is common), the session runs permanently silent despite a working card. | Medium | P1 | 85 | no | M |
| U2 | `crates/view/src/orbit.rs:114-158`: the viewer has no `Focused(false)` handler clearing held keys; a key held across alt-tab orbits forever. `oag-game` already fixes this class (`release_all` at `crates/game/src/main/app.rs:346`); the viewer never got it. | Medium | P2 | 85 | no | S |
| U3 | `crates/input/src/keys.rs:31-42`: bindings map logical `Key::Character`, so the layout moves the controls (AZERTY scatters the WASD cluster; QWERTZ moves CIRCLE's "z") and no rebinding UI exists to compensate (rebinding is a known non-working menu item). Physical scancodes are the game convention. | Medium | P2 | 95 | no | S |
| U4 | Real-time-thread hygiene: `output.rs:102-110` resizes a Vec inside the audio callback and `mixer.rs:472-474` drops an `Arc<Sound>` on the audio thread (deallocation of sample data); worst case an audible dropout. | Low | P3 | 85 | no | S |
| U5 | `crates/input/src/lib.rs:175-198`: a key tap fully contained between two 60 Hz snapshots is lost - no press-latching between `begin_frame` calls. | Low | P3 | 85 | no | S |
| U6 | `crates/audio/src/mixer.rs:281-301`: pitch 0.0 is accepted (`pitch.max(0.0)`), the voice never advances, emits a constant DC offset and occupies its slot forever. | Low | P3 | 85 | no | S |
| U7 | `crates/audio/src/mixer.rs:484-489`: `render_tick` truncates `sample_rate / tick_hz`, so a non-divisible rate on the `--dump-audio` path drifts the WAV behind the simulation by up to tick_hz-1 frames per second. | Low | P3 | 85 | no | S |
| U8 | `crates/view/src/orbit.rs:227-229`: stale "keyboard-only" comment justifies `set_cursor_visible(false)` while the same file implements drag and wheel input at lines 376-436, hiding the pointer the user drags with. | Low | P3 | 85 | no | S |
| U9 | `crates/input/src/lib.rs:30-33,91-103`: `Keyboard`'s embedded `Input` is dead state when used inside `Controls` (which merges `held_mask()` and never calls `Keyboard::snapshot`); two edge-computing `Input`s in one struct invites reading the stale one. Footgun, not a bug today. | Low | P3 | 80 | no | S |

Positives: every input producer path ends in `InputSnapshot::sanitised()`
(NaN-to-zero, clamped axes) with the policy in device-free tested functions;
degraded modes are first-class (`Output::null`, `open_or_null`, `Pad::none`)
with no unwrap on any device-init path; the viewer's `--draws`/`--only`
diagnostics operationalise the "draw nothing and say so" rule.

### Game shell

(Findings from the dedicated game-crate pass; the shared-path Pulse couplings
are graded under S2/S3/S5/S6 above. That pass also confirmed the `--team` doc
at `crates/game/src/main/cli.rs:365-373` explicitly promises per-title
resolution "the same way --track does" - so S3 is a landed-halfway refactor,
not a missing feature: the track half landed, the team half did not, and
`oag_title::RaceDefaults::team` has no reader.)

| ID | Finding | Impact | Priority | Conf | Faithful | Effort |
| --- | --- | --- | --- | --- | --- | --- |
| G1 | `crates/game/src/main/session/frame.rs:253-270`: the menu arm can swap `Stage::Menu` to `Stage::Race` inside the fixed-tick loop without the `break` the results-table arm (lines 167-179) uses for exactly this reason; remaining steps of that frame tick the fresh race with the menu-confirm key still held, reading the X press as thrust. Bounded by the catch-up cap (usually 0-1 leftover steps). | Low | P2 | 85 | no | S |
| G2 | `crates/game/src/icon.rs:42,49` claims no player input reaches it, but `--icon-size` does (`main.rs:117`) with no non-zero validation, so `--write-icon out.png --icon-size 0` panics via `expect` instead of a CLI error. | Low | P3 | 85 | no | S |
| G3 | `crates/game/src/boot.rs:504-524`: the boot-chain walk validates presence and drivability for every step except the first - `walked` starts with `profile.start()` unconditionally, so a pressing lacking the boot screen (demo/trial disc) walks an unbacked state any later step would have been skipped-and-reported for. | Low | P3 | 85 | no | S |
| G4 | The title-generic boot path builds language-plugin paths through `oag_pulse::names::language_definition` and `oag_pulse::read_image` for all titles (`crates/game/src/boot.rs:1427,1364`); the convention is genuinely shared today (HD resolves through it), but it lives in the Pulse crate rather than `oag-title`, and a future title whose plugins live elsewhere silently loads zero languages (`load_languages` continues on every miss). Same class as S6. | Low | P3 | 85 | n.a. | S |
| G5 | Cosmetic/doc rot: `main/args.rs:128` fabricates the loading label "Data.wad {INTRO_MOVIE}" from Pulse constants for any source; `main/headless.rs:185-233` has `run_race`'s doc fused onto `write_trace` leaving `run_race` (line 316) undocumented; `main/args.rs:153-156` carries a dangling paragraph about the moved `RACE_KEYS` constant. | Low | P3 | 85 | no | S |

Verified clean: missing assets degrade rather than panic everywhere on the
boot path (every movie error becomes a report line, a panicked media worker
yields empty media, an unbootable disc keeps the chooser up); the headless
claim is real (boot assembly and the whole front-end state machine are
GPU-free and exercised under `--no-video`); no reverse render-to-sim leakage
(one wall-clock read per frame feeds `TickClock`, sim ticks fixed 60 Hz, the
dt handed to menus is the clock's constant).

### Infrastructure and docs drift

| ID | Finding | Impact | Priority | Conf | Faithful | Effort |
| --- | --- | --- | --- | --- | --- | --- |
| I1 | `CLAUDE.md`'s crate table omits `oag-audio` (which exists, with real cpal code) and still lists it under "later crates added when their milestone opens"; `docs/architecture/workspace-layout.md` already lists it as existing. An agent following CLAUDE.md would refuse to touch it or try to re-create it. | Medium | P1 | 95 | n.a. | S |
| I2 | `scripts/check-transcendentals.py:43` covers only the five simulation crates; values parsed by `oag-formats` (handling, splines) feed the sim at load time and the crate already contains a live platform `log2` (`entropy.rs:36`, currently harmless). The CLAUDE.md claim "no platform transcendental reaches simulation code" is broader than what the script enforces - and D1 shows the textual matching also misses glam wrappers. Extend the crate set and teach it `from_axis_angle`/`from_rotation_*`/`angle_between`/`slerp`. | Medium | P1 | 90 | n.a. | S |
| I3 | Stale format status rows: `docs/formats/README.md:40` says the section PVS payload is "not implemented" (`crates/vex/src/pvs.rs` implements it) and row 46 says weapon stats are "schema read, nothing implemented" (`crates/tables/src/weapons.rs` parses Turbo/Shield/Autopilot in full plus absorb and Pickupodds). The status column also uses values outside its own legend. | Medium | P1 | 95 | n.a. | S |
| I4 | `HANDOVER.md`'s open thread for `Gu_TexScale`/`Gu_TexOffset` ("unported") is stale: `docs/rendering/scenery-animation.md` records it complete (922/922 materials, 2026-08-18) and HANDOVER's own trap row agrees. Violates its stated "a row whose work landed is deleted" rule. | Low | P2 | 90 | n.a. | S |
| I5 | `docs/README.md:61` still says "Milestone **M0 (Foundation)** ... nothing about the game's runtime behaviour has been established yet", several milestones and one booted HD front end ago. The first page a new contributor reads is the most wrong one. | Medium | P1 | 95 | n.a. | S |
| I6 | Small staleness set: CLAUDE.md says the size baseline holds 27 files (it holds 23, gate passes); `check-dependency-rules.py:24` says "oag-audio does not exist yet"; `crates/trace/src/main.rs:40` doc points at `oag_game::race::DEFAULT_TRACK` which lives at `oag_pulse::race::DEFAULT_TRACK`. | Low | P2 | 95 | n.a. | S |
| I7 | `oag-wad` numeric selector >= entry count falls through to name-hash lookup, erroring "no entry named 9999 (hash ...)" instead of "index out of range". Cosmetic. | Low | P3 | 85 | n.a. | S |
| I8 | `HANDOVER.md` is 308 KB / 1,371 lines with single table rows over 2 KB; it has outgrown "read this before starting work". Worth splitting the open-thread table by subsystem or archiving resolved history, on the same reasoning as the file-size ratchet. | Low | P2 | 90 | n.a. | M |

Verified clean, worth recording: the trace capture/compare logic is sound
(tick-offset misalignment detection, NaN can never pass tolerance, absent
columns are "not compared" rather than zero); confidence discipline holds
(zero sub-50 rows in any `names.tsv`; sub-70 rows store bare names by design,
the `_q` suffix is derived at apply time); `check-leakage.py` and the
file-size ratchet enforce exactly what CLAUDE.md claims.

## What is done well

Recorded so the next reviewer does not spend their budget re-proving it:

- **Hash and determinism engineering beyond the letter of the rules**: own
  libm `acos` wrapper, cosine-not-angle conventions in the AI, integer-only
  RNG with an independently implemented reference and a bijectivity test,
  integer-nanosecond clock with its truncation drift measured and pinned,
  exhaustive-destructure-as-compile-error hashing with discriminant bytes for
  every `Option`.
- **Validate-before-allocate and refuse-rather-than-guess** applied
  consciously across ~30 parsers, with the hostile case named in a comment
  each time, and malformed-input tests beside them.
- **The mechanism/table split is real**: `oag-assets` reads any title through
  `&Title`, reaches Pulse only in dev-dependency ground-truth tests, and the
  title crates are data with almost no duplicated logic.
- **Provenance rigor**: blend equations, cull winding, fog curves and physics
  constants each carry a Ghidra address or an explicit "ours, because" - this
  review could mostly check the code against its own cited evidence, which is
  rare.
- **WGSL/Rust layout mirroring is actively managed**, with padding documented,
  vec3 traps designed out, and lockstep files named.

## Recommended order

Do now (each under an hour, most under ten minutes):

1. **S1** - add `oag-hd` to `GAMEPLAY_CRATES` (one line, restores a stated
   invariant).
2. **D1** - route `from_axis_angle` through a deterministic `sin_cos` (libm,
   as `oag_core::math::acos` already does), add rocket-spread coverage to the
   gameplay determinism test, and extend `check-transcendentals.py` per I2.
3. **D2** - reconcile `damage_rules(Zone)` with `weapons_enabled(Zone)`; the
   measured evidence sides with weapons off.
4. **F1** - mirror the PSMT4 guard for PSMT8.
5. **I5 + I1 + I3** - the three doc-drift items a new contributor hits first
   (docs/README status, CLAUDE.md crate table, format status rows).

Next session on each area: S2/S3 (serve HD its own HUD tables, use
`Title::race.team`), U1 (sample-format fallback), I2, D3.

Racebox groundwork, in order and without new abstractions: an explicit title
probe table replacing the `open_source` cascade (S5), then the per-asset
`(Archives, &Title)` split in `race::load` (S4), then promoting the naming
axis onto `Title` when a second divergent corpus (2048) forces its shape (S6) -
each of which is useful for the single-title engine on its own.
