# HD/Fury's HUD draws, and three of Pulse's constants applied to every title were why it did not

2026-08-25, branch `worktree-hd-hud-align`; all of it on [hd-hud.md](../../docs/formats/hd-hud.md#the-hud-draws-and-each-sprite-out-of-its-own-texture). **The one to know**: the sheet held only `Layout::atlas()`'s *first* texture, so 45 of the arcade HUD's 138 sprites sampled `HUD_Components.gtf` at coordinates meant for another image - the right rectangle out of the wrong picture, which reads as art rather than as an error, and which the existing source-rectangle check passes either way. `oag_title::HudArt` is the new axis: texture extension, always-on widgets, pickup-backdrop colour. **Compared against a frame of the running original** (`just rpcs3-race`, speed lap) for the first time - one state of one mode, which is why the always-on set is fifteen names and not fifty. **Four things that frame shows and this build does not**: `DamageBar`'s and the lap arcs' runtime tints, `ShieldBarText`'s `100%`-in-red where the original reads `100` in grey, and the per-lap time rows.

**2026-09-07 re-check.** All four are still genuinely open - confirmed against current code (`crates/hud/src/draw.rs`, `crates/hd/src/hud.rs`), not just against the doc - and the doc's own reasoning for not fixing them holds up under a harder check than it had before. Full evidence in [hd-hud.md#what-is-not-done](../../docs/formats/hd-hud.md#what-is-not-done); the short version:

- **`DamageBarBg`/lap-arc tints and `DamageBar`'s swap-in state**: two throwaway probes (`crates/game/examples/hd_hud_shield_census.rs`, `hd_hud_tint_census.rs`) walked every raw fragment file behind all eighteen composed layouts, then every one of those fragments was re-pulled **archive by archive** (`scripts/psarc.py cat`) to rule out the copy-precedence trap this same doc page already documents for other paths. `DamageBarBg`, `LapBar0`-`6` and `PosBar0`-`7` carry **no `color=` attribute in any copy on the disc**, and none of the four `FEConst`/`FEGlobals` names any HUD layout declares is yellow or the frame's saturated blue. `DamageBar` itself *is* authored a colour, identically across every copy - red in every mode but Zone, blue only in `zone_hud.xml` - which rules out "we're just drawing the wrong mode's `DamageBar`" as the explanation for the speed-lap frame's blue. One archive-only widget turned up along the way: `DATA06`'s copy of `HUD_damage_indicator.xml` alone adds a `DamageBarShieldBg` layer coloured opaque green, excluded by precedence (this path resolves to `DATA02`) and not blue anyway - a loose end, not the answer. Separately: `arcade_hud.xml` composes **two** widgets named `DamageBar` (the red one, and an unrelated colourless one from `HUD_pickups.xml`) - `draw_list`'s "draw the first of a repeated name" rule means wiring `DamageBar` by name needs a check first. This needs the executable. **Nobody held the Ghidra bridge this pass**, so it stays unread rather than guessed at - next pass, take the bridge for this specifically.
- **`ShieldBarText`**: confirmed HD's own layout genuinely resolves it to translucent red (`[1.0, 0.0, 0.0, 0.58]`), against Pulse's white - so the red is not a resolution bug in this build, it is what the disc says. No widget authors a `%`-suffix companion the way `SpeedBarTextKMH` does for `SpeedBarText`'s unit, so neither half has a layout-derived fix. One frame at 100% shield cannot tell "always grey" from "grey only when not critical" (the more likely rule, given `ShieldBar`'s own bar already has a measured critical-threshold tint) apart - guessing which would be exactly the invention this project's rules forbid. Left alone rather than fixed on a guess.
- **Per-lap time rows (`Lap1Image`-`Lap4Image`)**: not a HUD-side gap. `oag_race::RaceState`/`Standing` track a running `best_lap_ticks` only, no per-lap split history, so `Readout` has nothing to carry even if the widgets were wired. Populating that history reaches into `crates/race` and `crates/raceplay/src/*`, outside this thread's declared HUD-only lane.

Net: no guessed fix was implemented for any of the four, on purpose - each would have required inventing a value or a rule with no data or executable evidence behind it, which is exactly what this project's "never invent" rule and this doc's own prior reasoning already ruled out. The doc page carries the full evidence trail so the next pass does not have to re-derive it.

**2026-09-07, the per-lap rows landed.** `oag_race::RaceState::lap_splits` and
`Standing::lap_splits` (`[Option<u32>; MAX_RECORDED_LAPS]`, `MAX_RECORDED_LAPS
= 4` - read off the disc's own four rows, not chosen) now carry the history,
and `oag_hud::lap_splits` wires `Lap1Image`-`Lap4Image` and their
`Lap{n}Text`/`Lap{n}Time` children, gated on which laps have a recorded
split. Full writeup: [hd-hud.md#the-per-lap-history-draws-off-a-new-
racestatestanding-field](../../docs/formats/hd-hud.md#the-per-lap-history-draws-off-a-new-racestatestanding-field).
Along the way: 2048 ships the identical fragment but does not reach it from
any played race (only the demo-only bare-root `TimeTrial_HUD.xml` loads it);
Pulse and Pure author no equivalent at all - both checked directly, not
inferred. Three of the four original items remain open below.

**2026-09-13, three matched-state frames landed** (`data/reference/hd-capture/
talons-matched/{00,01,03}.png`: grid/full shield, mid-race, mid-race damaged -
all Feisar `concept1` on Talon's Junction, a Fury campaign single race rather
than speed lap). Full writeup: [hd-hud.md#three-matched-state-frames-replace-
the-one-this-page-was-written-from](../../docs/formats/hd-hud.md#three-matched-state-frames-replace-the-one-this-page-was-written-from).
Two of the four original items moved, one sharpened, one is still exactly
where it was:

- **Two widgets this build drew that no frame ever shows, now fixed**:
  `PositionTxt2` (a second, unused `POS` caption) and `PickupDamageTxt`/
  `PickupAbsorbTxt` (`DAMAGE`/`Absorb`, which drew unconditionally instead of
  only after a hit). Both were the same bug - `text_for`'s idstring catch-all
  has no gate - fixed by naming both explicitly. `ShieldBarText`'s `%` is also
  fixed: all three frames read a bare number, never `100%`, so `oag_title::
  HudArt::shield_percent` is a new per-title axis rather than a hardcoded
  suffix.
- **`ShieldBarText`'s colour is not settled, but the shape of the open
  question changed.** The three frames rule out "always red" and "red only
  when not full" outright - `03.png`'s 98% still draws blue, not red - and
  narrow the live question to "does it ever draw red at all," rather than
  "grey above a threshold, red below it." Pixel-sampled directly: a
  consistent, near-opaque blue (~`#2C6BE7`) at both 100% and 98%. A same-frame
  alpha-blend control (back-solving `DAMAGE`/`15`'s authored `0x94FF0000`
  recovers pure red, validating the method; the identical back-solve on
  `Absorb`'s authored `0x941664FF` undershoots R/G in a consistent direction)
  points at a specific candidate rather than a mere hue family: **the runtime
  most likely substitutes the disc's own `0x1664FF`** ("HD blue," authored
  twice elsewhere in this exact composition) for `ShieldBarText`, though the
  substitution rule itself - which widget, which state, why - is still
  unread, so this stops short of confirmation. **Not implemented** - a
  specific candidate is not a confirmed one, and this project's rule against
  tuning a colour to a screenshot applies exactly here.
- **`DamageBarBg`/lap-arc tints: narrowed, not closed.** `PosBar0`-`7`'s lit
  count now measurably tracks *place* (0 lit at 8th, 1 lit at 7th, in both
  `01.png` and `03.png`) rather than being a fixed decoration - but the
  segments are baked pure white in `HUD_Components.gtf` (confirmed directly,
  `cargo run -p oag-game --example hd_hud_bar_pixels`), so implementing the
  count alone would draw white segments over an already-white ring and change
  no pixel. Still needs the executable for the colour. `LapBar0`-`6` stayed
  exactly where it was: all three new frames are lap 1 of 3, so the lit count
  (3, constant) cannot separate "encodes total laps" from "encodes current
  lap." **Still nobody has held the Ghidra bridge for this question
  specifically** - three passes now with real frame evidence in hand and no
  attempt at the executable.
- **Per-lap time rows**: untouched by this pass. All three new frames are lap
  1, so none of them can settle the still-open per-lap-rows question below.

**2026-09-23, the executable pass - three of the four tint questions closed.**
Full writeup: [hd-hud.md#the-executable-writes-the-shield-the-number-and-the-arcs](../../docs/formats/hd-hud.md#the-executable-writes-the-shield-the-number-and-the-arcs);
addresses and constants on [hud-readouts.md](../../docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md).
Found from the widget names: `Hud_BindWidgets` stores `DamageBar`/`DamageBarBg`/
`ShieldBarText`/`LapBar*`/`PosBar*` at fixed HUD offsets, and a byte-offset search
leaves one reader of each set - `Hud_UpdateShieldReadout` (`0x000866c8`),
`Hud_UpdateLapCounter` (`0x00096ef0`), `Hud_UpdatePositionCounter` (`0x00096088`),
all dispatched every tick by `Hud_Update` (`0x0009e3d0`). What they write, now
drawn by `oag_hud::runtime` off `oag_title::HudArt::runtime`:

- `DamageBar` is the fill, cropped from the top to the shield fraction, opaque
  `0x1664FF` (white in Eliminator/Detonator modes). `ShieldBarText` takes the same
  colour, truncated digits, no `%`. `DamageBarBg` is white, flashing `0xFFFF0000`
  at <= 20 % or for a second after a hit, 8 phases/s; the number blinks with it.
- The arcs are only shown/hidden: `LapBar_k` while `k >= laps - lap`,
  `PosBar_k` while `k <= 8 - place`. **Their yellow is baked in the atlas** - the
  old "pure white" probe read the `.gtf` upside down; `hd_hud_bar_pixels` is fixed.
  The frames agree once re-read: 8th shows one `PosBar`, 7th two, lap 1/3 five.
- Duplicate `DamageBar` in `arcade_hud.xml`: the fill is taken as the one sharing
  `DamageBarBg`'s source rectangle - an inference (the name resolver
  `0x0067e6d0` was not read); `hd_hud_runtime_ground_truth.rs` pins that exactly
  one matches in each hexagon layout.

**2026-10-02, the low-shield pass (branch `hd-hud-flash`).** The flash was compared
against the running original for the first time (private RPCS3, the shield
poked through the GDB stub, the HUD recorded at 30 fps; evidence and numbers on
[hud-readouts.md](../../docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md#what-the-running-original-does-at-low-shield)).
Rate (3.91-4.03 cycles a second, six stretches in two boots), the `<= 20` threshold, the red bracket, the
solid fill, the blinking number and the 1 s post-hit window all agreed. Three
things were fixed: the window now arms on a **whole-percent** drop (60.9 to
60.2 arms nothing, 60.2 to 59.9 does - read off `hud+0x110`), the flash phase
is the original's own accumulator, and the absorb half of the third condition is
wired (the fill and the number blink, the bracket stays white). `ship+0x6a80` is
the absorb stamp - 3 of 3 stamps coincided with the absorb timer starting, 0 of 12
other pickup clears did; `ship+0x6958` is the LeachBeam manager's flag.
`oag-game --force-shield TICK:PERCENT` puts both sides in the same state.

## Open

- **`ship+0x6958`** (the flash's other input) is set by
  `LeachBeamManager_Update_q` (`0x0013d7a8`) on `*(beam+0xc94c)`; which of the
  two craft that is (the firing one or the beamed one) is unread, so the
  flash on a live beam does not fire. Read `LeachBeam`'s `+0xc93c`/`+0xc94c`/`+0x44`
  writers, or watch it live: it needs two craft on a beam, i.e. an AI that fires one.
- **The player's own absorb was not seen on the original** (no pickup on the
  line in 35 s of thrust); the AI's three were. A steered run onto a weapon pad,
  then a button tap, would give the player's `ship+0x6a80` and the real blink.
- **`0x0067e6d0`'s traversal order** (which `DamageBar` it binds) is unread.
- **A look at one frame each found the top plate and the pickup slot's hexagon
  outline drawn heavier than the original's** (different scenes - this build was
  rendered at its default circuit, the capture is Talon's - so unmeasured): at 100 % the original's outline is a
  light translucent grey and its top plate shows the scene through it; this build's
  outline is near-black and its plate opaque (`ring.png`,
  original left, this build right, same crop). Not touched - it is not the flash,
  and `hd-hud.md` records the outline as measured "in its authored grey" - but the
  two pictures disagree, so check the blend and the baked alpha of that atlas region.
- The per-lap rows still draw on no reference frame at all - none of the three
  captures completes a lap. Confidence 70 stands; see `hd-hud.md`'s per-lap
  section.

## Next Steps

- Get a capture with two or more laps completed on a mode that shows
  `Lap1Image`-`Lap4Image` (time trial, default skin) to check the digit-per-row
  reading and the "invisible until completed" gate against a real frame. The
  same capture checks `LapBar` lighting one more segment per lap. (Not attempted
  this pass: the original has no autopilot, and three laps by hand is minutes of
  steering through a pad.)
- Resolve `beam+0xc94c` and wire `ship+0x6958` into
  `Race::advance_shield_flash_whole`'s blink gate once it is read.
- Whoever next holds `crates/2048`'s HUD reading: `HUD_lap_times.xml` ships in the `2048_hud` skin's archive but no played-race root loads it (`SpeedLap_TimeTrial_HUD.xml` loads `HUD_lap_counters.xml`/`HUD_target_time_total.xml` instead) - confirm that stays true once `oag_2048::hud::ALWAYS_ON` (currently empty) gets filled in, rather than assuming this cluster is reachable there. Separately: 2048's *unplayed* `wo3_hud`/`2097_hud`/bare-root skins do author `PickupDamageTxt`/`PickupAbsorbTxt`/`PositionTxt2` (checked directly, 2026-09-13) - moot today since nothing composes those skins, but worth knowing before assuming this thread's `None` fix needs revisiting there.
