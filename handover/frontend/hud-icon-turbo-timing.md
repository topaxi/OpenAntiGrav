# The gantry hexagon is TurboIcon, legitimate - the grant-timing bug is fixed; the advert-board blur was texture level selection, fixed 2026-09-30

2026-09-07. A maintainer-requested side-by-side
(`gantry-compare.md`,
`gantry-compare/side-by-side-countdown.png`) flagged a green hexagon over
our start gantry with no counterpart in the original, in every Time Trial
gantry screenshot taken that day.

## What is settled

**Not spurious, not invented - it is `PickupBackground` + `TurboIcon`,
drawn as authored.** Traced to source, not inferred:

- `Race::grant_free_turbo` (`crates/raceplay/src/weapons.rs`) hands Time
  Trial/Speed Lap a free Turbo, called once from `Race::start` before lap 1's
  own edge exists, and again on `lap_completed`. Confidence 85 already
  recorded in `docs/gameplay/pickups.md`: `MSC_EVENT_TT`/`_SL`'s own text and
  `TimeTrial_HUD.xml` authoring exactly one weapon icon (`TurboIcon`) where
  Arcade authors thirteen and Zone none.
- `crates/hud/src/draw.rs`'s `draw_list` draws `readout.pickup` (which is
  `Some(Turbo)` from tick 0 in Time Trial) unconditionally, with no countdown
  gate anywhere in that file.
- The widget's authored position is screen top-centre (`x=240`, half the
  480-wide reference; `crates/hd/src/hud.rs` independently documents the same
  widget as "top centre" in HD's dialect) - the same region the countdown
  board occupies. The overlap in the screenshot is two top-centre widgets
  sharing space, not a stray draw or a placement bug in the ordinary sense.

So the identification is closed. **Do not delete this draw** - it is real
disc-authored content for a weapon the craft genuinely holds.

## Resolved 2026-09-07: the original grants the free Turbo at release, not at the start line

**Confirmed directly by the maintainer on `pulse-psp-eu`** (our own render
target, not just the USA disc this thread's earlier capture used): the free
Turbo appears only **after** the countdown releases the craft. It is not held
through the countdown at all - so the original never has anything in the
pickup slot for the HUD to draw during those 272 ticks, and the icon
(identification above still stands - it is `TurboIcon`, drawn correctly given
what it's handed) simply has nothing to show at that point in the real game.

**So: the HUD is not the bug.** `crates/hud/src/draw.rs` is drawing
`readout.pickup` exactly as it should; the bug is that `readout.pickup` is
`Some(Turbo)` during the countdown at all. **Do not touch the icon's
position, opacity or draw gate** - all three are behaving correctly given the
state they're handed.

### The exact fix - landed 2026-09-07

`Race::grant_free_turbo` (`crates/raceplay/src/weapons.rs`) was called from
two places: `Race::start`, unconditionally at tick 0 (the bug), and
`oag_race::Outcome::lap_completed` inside `Race::tick` for every later lap
(fine). Fixed by deleting the `Race::start` call and calling
`grant_free_turbo()` once from `Race::tick`, at the tick
`oag_race::state::RaceState::thrust_gated` first reads `false`
(`self.world.tick == oag_race::state::COUNTDOWN_TICKS`) - see
`crates/raceplay/src/tick.rs`.

`a_fresh_time_trial_or_speed_lap_already_holds_lap_ones_turbo` (renamed
`..._holds_no_turbo_until_the_countdown_releases`) and two other tests' doc
comments that assumed the tick-0 grant were flipped alongside it -
`crates/raceplay/src/tests/weapons.rs` and
`crates/game/tests/race_ground_truth.rs`. `docs/gameplay/pickups.md` and
`weapons-eight-of-thirteen-the-plasma-and-the.md` (its own countdown-gate
note) were updated to match.

**Verified**: `just` (full gate) and `OAG_REQUIRE_GAME_DATA=1 just test-data`
(full disc-backed suite) both pass. Screenshots at `--race --mode
time_trial --size 960x540` confirm it visually: no `TurboIcon` hexagon
during the countdown (`--ticks 150`), present once released (`--ticks 400`,
logged `holding Turbo`) - the "judge as a player would" check this brief
asked for.

**Left for whoever picks up
`weapons-eight-of-thirteen-the-plasma-and-the.md` next**: whether a blanket
countdown weapon gate is now warranted elsewhere, now that the one case
that argued against it (this bug) is understood and fixed.

## The advert-board blur: resolved 2026-09-30 - the original samples the base level (see the end of this section)

2026-09-08. Identified the exact board and texture, ruled out two
`docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`-documented mechanisms, and
landed a real (but confirmed non-fixing) gap the same doc already flagged.

**The board is `hub_banner_GLOW.tga`** (128x128, 8bpp indexed, authored path
reads `Data\Environments\12_Track\Textures\...` even though it draws on
`16_Track` - a shared asset), on mesh node 1125 at track-space `(-63, -47,
-209)`, right beside the start line. Confirmed by `oag-view --only
hub_banner_GLOW` against `16_Track/track.vex` - the isolated draw matches the
small red/black board visible top-left in every gantry-area capture. Repro:
`cargo run -p oag-game --release -- data/images/pulse-psp-eu.chd --race
--mode time_trial --track 'Data\Environments\16_Track\track.vex' --size
960x540 --screenshot out.png --ticks 130` (`--until` takes a state name to
wait for, not a tick count - `--ticks` is the plain counter; the board is
visible from the grid at tick 0 regardless, which is enough to reproduce
this).

**Two mechanisms `mesh-draw.md` already covers were re-checked against this
specific texture and both are still negatives:**

- **Filter mode**: `Gu_TexFilter` (`0x088113f0`) is a single global
  `(min=7 GU_LINEAR_MIPMAP_LINEAR, mag=1 GU_LINEAR)` for the whole game, no
  per-material override exists anywhere to emit one, and `oag_render`'s
  shared albedo sampler already uses `Linear`/`Linear`/`Linear` at
  `Anisotropy::X16` by default - same state, not a candidate.
- **Mip chain depth**: `Texture::mip_count` (the PSP `.vex` texture object's
  own `+0x05` byte) was parsed by `oag_vex::vex::textures` but silently
  dropped by every consumer - `mesh_render`'s uploader always synthesised a
  full box-filtered chain down to 1x1 regardless of what the asset declared.
  **Wired through** (`ModelTexture::mip_count: Option<u32>`, threaded from
  `mesh.rs`'s vex-embedded-texture path, capping `mesh_render::texture`'s
  synthesised chain when known): `hub_banner_GLOW.tga` declares `mip_count =
  5` (128x128 down to 8x8) against a synthesised chain that ran to 1x1.
  **Measured, not assumed, and it is inert here too**: capping at the
  authored depth changed 24 of 518,400 pixels between two otherwise-identical
  captures, mean difference 4.7e-5 - the same "measured inert" shape
  `mesh-draw.md` already found for the exhaust textures on 2026-08-10. Landed
  anyway as its own fix (`docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`'s
  own "worth fixing on its own account" note asked for exactly this), but it
  does not explain the blur.

**What is not yet checked**: `Gu_TexLevelMode` (`0x088114b0`, `TEXLEVEL`),
the other half of `mesh-draw.md`'s finding. `Texture_BuildBindList` emits
either mode `2` with a *computed* LOD bias or mode `0` with none, gated on
the texture object's `+0x06 & 0x18` bits - which of the two applies to
`hub_banner_GLOW.tga` specifically is unread, and `oag_render`'s sampler sets
no LOD bias at all (`SamplerDescriptor::default()`'s zero). A systematic bias
difference here - rather than a per-texture chain-depth one - would explain a
blur that mip-depth capping did not touch. Reading `Texture_BuildBindList`'s
bias computation and this texture's own `+0x06` byte needs a Ghidra project
open on `psp-pulse-usa`; not attempted this session.

Also unchecked: whether bloom or another post-process pass amplifies this
additive-blend (`ADD`/`GLOW`-named) texture's inherent softness beyond what
the original's own compositing does - not traced this session either.

**Resolved 2026-09-30 by the frame audit** ([frame-audit.md](../../docs/rendering/frame-audit.md)):
the blur was `TEXLEVEL` after all, but not as a bias. Matched frames at 480x272
show the original resolving every scenery texture at its base level, while ours
walked a box-filtered chain (trees, mountains and road read 15-30 % softer by
Laplacian energy). A PSP `.vex` texture now reaches the GPU with the levels the
disc authors, picked by the game's rule (`Texels::Chain`,
`mesh_render::PSP_TEXLOD_SLOPE`): slope mode with a `1/256` slope
(`Gu_TexLodSlope`, `0x08811694`), a depth-driven law that stays at level 0 inside
about 128 units. Post-processing was ruled out too: bloom on and off left the
trees identical. Nothing in this thread is open.

## Next Steps

None.

## Open

Nothing.
