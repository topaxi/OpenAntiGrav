# What Pulse never authors, and the log lines that used to say it was missing

2026-10-06. A race on Pulse (PSP or PS2) used to log a WARN for every effect, texture
and cue its loader asked for and did not find. Each line is either a **cause fixed**
or an absence **proven by design**; this page holds the evidence for the second kind
and the fixes for the first, so no line was demoted without a reason. The census
tools are the ones this project already has: `oag-wad list`/`hash`/`extract`,
`oag-unpack list`, and `grep -a` over the executables.

## Seven `Data\Psys` names Pulse does not author, and the eighth that only the PS2 does

`oag_title::engine_effects` is the engine's shared name list. It claimed to be
"as Pulse's executable names them", and eight of its entries are not. Pulse's
`Data.wad` (PSP) and `WADS2.WAD` (PS2) hold 35 and 41 `SYSP` blobs, every one named by
its own hash ([pob.md](pob.md#recovering-all-35-names-by-their-own-hash)), and none of
them is any of these.

| Name | Where it comes from | PSP | PS2 |
| --- | --- | --- | --- |
| `WO_PLASMA_LIGHTNING_EXPAND` | HD's plasma detonation (`WeaponExplosions_Start`) | absent | absent |
| `WO_PLASMA_LIGHTNING_COLLAPSE` | HD, its other half, 1.3 s later | absent | absent |
| `WO_TRAIL_HITSHIP`, `_RED` | HD's `Trail_HitShipEffect`; `oag_fx::exhaust::hd` | absent (HD's is lent with `graphics.trail_hits` on) | absent (same) |
| `WO_LEACHBEAM_ABSORB` | HD's `LeachBall_Advance` | absent | absent |
| `WO_MAGSTRIP_SPARKS`, `_ZONE` | 2048's ship constructor (`GameMode_IsHdLineage` false) | absent | absent |
| `WO_SHIP_ENGINEFLARE` | the PS2's own authored flare | absent | **present on the disc, never requested** (`engine-trail.md`, "The engine flare is one quad") |

How "absent" was established, and how "never requested" was:

1. **By hash, in every archive of both discs.** The eight names hash to `498f3499`,
   `4bb1faff`, `636b4809`, `fc0c5fc0`, `734a2af7`, `7af37ef8`, `51fa2612` and
   `0afe819a`. Against the entry directory of every WAD on both discs (`Data.wad`,
   `FE.wad`, `FEData.wad`, `BEData.wad`, and `WADS2.WAD`, `WADSP.WAD`, `PRERACE.WAD`,
   `PS2MUSIC.WAD`) exactly one hit exists: `0afe819a` in `WADS2.WAD` (the PS2 engine
   flare). `PRERACE.WAD` and `PS2MUSIC.WAD` print no `entries` summary line from
   `oag-wad list`, so their zero is a directory grep, not an extracted census.
2. **By content, so "under another name or directory" is ruled out.** `Data.wad`
   (1,138 entries), `WADS2.WAD` (7,200) and `WADSP.WAD` (193) were extracted and every
   blob searched for `HITSHIP|LIGHTNING|LEACHBEAM_ABSORB|MAGSTRIP|ENGINEFLARE`. The
   only hits are three large non-effect blobs (the string `LIGHTNING` in two, the cue
   name `PLASMAHITSHIP` in the weapon sound bank), one
   `.POB` whose emitter reuses an `EngineFlare` *texture directory*
   (`WO_MISSILE_BOUNCE`'s art, not an effect), and the PS2's own
   `WO_SHIP_ENGINEFLARE.POB`.
3. **By request.** Neither `BOOT.BIN` (PSP, USA and EU) nor `SCES_547.48` (PS2)
   contains any of the eight strings, nor any of the eight hashes as a 32-bit value
   in either byte order. None of the eight appears there. This is the weaker
   leg on its own: [pob.md](pob.md) records that a continuous effect such as the
   engine flare is absent from its executable's string table as well and is
   triggered anyway. It is sufficient here because legs 1 and 2 show the file does
   not exist, so no request could be satisfied; leg 3 only says nothing in the
   executable even asks.
4. **Where each came from instead.** HD plays the lightning pair, the trail pair
   and the leach burst
   ([weapons.md](../ghidra/functions/ps3-hdfury-eu/weapons.md),
   [engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md)); 2048 plays the
   magstrip pair (`crates/2048/src/lib.rs`, `crates/game/tests/magstrip_wire_2048_ground_truth.rs`).
   Pulse draws its plasma ending from `WO_PLASMA_FLASH`
   ([plasma.md](../ghidra/functions/psp-pulse-usa/plasma.md)) and has no magstrip wake
   particle effect (its `MagEffect1/2.vex` models are a different mechanism).

**The fix is Pulse's table, not the list.** `crates/pulse/src/effects.rs` chains
`Effects::without` for the seven, and scopes the PS2's engine flare with the new
`EffectSpec::on` (`Platforms::Only(&[Platform::Ps2])`); the loader asks
`Effects::names_on(platform)`. `crates/raceplay/tests/psys_inventory_ground_truth.rs`
(`pulse_psp_asks_for_nothing_its_archive_lacks`,
`pulse_ps2_asks_for_nothing_its_archive_lacks`) fails by name if a name Pulse's table
requests is not in that platform's archive, so re-adding one cannot go unnoticed.
The PSP draws its engine flare procedurally from the `Engine Flare` locator, which
`engine_effects::ENGINE_FLARE_EFFECT` already said and nothing contradicts.

**Left alone, and open:** `Effects::engine()` still carries all of them for Pure, HD,
2048 and Omega. Separating "the engine's names" from "HD's and 2048's" there is
cross-title work in `crates/hd` and `crates/2048`'s lanes. Pure's executable names
none of the seven either (checked, same grep) and its `names()` therefore still asks
for them; that is Pure's table to correct, and was not touched.

## The cannon bolt, muzzle flash and ghost static on the PS2 were a name-lookup gap

`Data\Weapons\Textures\Cannon_bolt.mip`, `Cannon_muzzle_flash.mip` and
`Data\Tex\staticglow.mip` were reported absent on the PS2. `SCES_547.48` itself contains all three `.mip` strings, so the PS2 requests them
and finds them under `.pct`. They are on the disc as
`.pct` (`077abc83`, `fe1eb1c7`, `fd15c258`), the rewrite
[`ps2_texture_name`](../../crates/formats/src/wad.rs) already documents. The two
loaders read through `Archives::read_name` and so never applied it. They now go
through `assets::exhaust_texture`, which reads through `oag_pulse::read_image` and
decodes either format; the PS2 now reports `32x32 .pct, 8 bpp`. Checked, applies:
HD and the Vita/PS4 titles name `.gtf`/`.gxt` and go through `decode_gtf`, unchanged.

## `talonsj~SETREG_01` and `_02` are control cues, silent by design

Both run only opcodes `0x14` and `0x1e`. `0x14` is a bare no-op (`Scream_OpNop14`,
confidence 82, [sound.md](../ghidra/functions/psp-pulse-usa/sound.md#opcode-0x14-is-a-no-op-corroborated-on-hd-2026-09-08))
and `0x1e` is `Scream_OpSetRegister` (confidence 88, same page), neither of which
starts a waveform. `oag_formats::sblk`'s timeline walk models both as pass-through
([timeline.rs](../../crates/formats/src/sblk/timeline.rs)); a cue whose walk is complete,
starts no grain and passed at least one such opcode is now reported as
`control only: ... authors no sound` at debug, not `play nothing` at WARN. A cue with
an opcode the walk does not read still takes the old WARN. The same reclassification
applies to every title that reaches `load_cue_record` (HD, 2048, Omega), by the same
walk. Run once each (HD `talons_junction`, 2048 `altima`, Omega `tech_de_ra`) and
diffed against the reference: no `play nothing` line moved on any of them (none of
their cues is control-only on those circuits), so this lane changed no 2048 or Omega
log line.

## The blob shadow is only a stand-in when the tier is on

`blob` (`graphics.shadows = blob`, default `off`) is this project's own tier, and on
every title but HD it draws a generated falloff because the disc ships no silhouette:
`blob` `0x3e0` and `textureBlob` `0x3df` are authored zero times in all 415 Pulse
`.vex` files ([shadows.md](../rendering/shadows.md)), and 72 candidate
`Data\Ships\<team>\{textures\,}ambient_shadow.{gtf,mip,pct,tga}` names (six teams,
three directory spellings, four extensions) hash to nothing in any Pulse WAD. The
original draws no blob on Pulse, so there is no disc image to wire; Pulse's own shadow
is the `original` tier's `Dynamic Shadow Occluder` hull, already built.

What changed is *when the log says so*. The load used to warn eight times in every
race that a falloff would be used, although nothing draws it unless the player
selects `blob`. `oag_render::shadow::Silhouette` now records `generated`, the load
logs the census at debug, and `Scene::shadow_geometry` warns once, the first frame
the `blob` tier is on and a generated silhouette is in use. That is the moment the
stand-in becomes visible to a player, so it is the moment it is named. The other titles take the same path, each with its census:

- **HD: proven absent for the two slots.** HD ships nine `ambient_shadow.gtf`
  (`ag_systems`, `assegai`, `egx`, `feisar`, `goteki`, `piranha`, `qirex`,
  `triakis`, `zone`; `scripts/psarc.py list` over all seven archives). The two
  slots that fell back on the reference run are `Icaras` and `Auricom`, which have
  none under any directory. Checked, differs: HD is the one title whose disc ships
  the silhouette, for nine teams.
- **2048: absent for every race team.** The only `Ambient_Shadow` in the Vita
  archives is `data/art/published/hdships/Zone/Textures/Ambient_Shadow.gxt`, a
  `.gxt` and for the Zone craft alone; the loader asks `.gtf` names for
  `feisar2048`, `Piranha2048` and the rest. Checked, applies, not wired: the Zone
  craft's `.gxt` silhouette could be read through `oag_texture`'s GXT decoder.
- **Omega: absent for every race team.** 46,005 names over the nine `.psarc` files
  of the base and patch trees; the only `Ambient_Shadow` are
  `hdships/zone/Textures/Ambient_Shadow.gnf` and `HDShips/Zone_VR/...`, the Zone
  craft alone, as `.gnf`. Same status as 2048.

## Billboard slots: closed for model slots (2026-10-06, billboards lane)

Rendered with `oag-view --mesh`: `Data\Billboards\Pulse_Adverts\goteki\GOTEKI_LANDSCAPE_01.vex` is a
self-contained scene of flat panels plus **logo and lettering as geometry**, with only
generic textures, like the start gantry's lettering
([start-gantry.md](../rendering/start-gantry.md)). The earlier reading here - "a model
that needs a transform, the constructor writes the identity matrix" - is retired: **the
model is drawn through its own camera into a 128 x 128 texture and the circuit's
`billboardN.tga` quad shows that texture**, read off a live PPSSPP GE dump
([billboards.md](../ghidra/functions/psp-pulse-usa/billboards.md), 2026-10-06 section).
Pulse PSP and PS2 now draw it (`oag_raceplay::adverts`), and the load report's
`N billboard-slot placeholder draw(s) suppressed` WARN no longer fires on a circuit whose
slots all name a model (Talon's Junction). It still fires, honestly, on a circuit with a
**colour** slot and a quad for it (`05_Track`: three), because which advert a colour picks
is a pool walk this project has not read. HD is not wired.

## Trail-hit burst budget (2026-11-11)

`graphics.trail_hits` lends HD's `WO_TRAIL_HITSHIP` to Pulse and Pure (above). A burst lives
about 30 ticks, so a craft inside a rival's trail holds about 30 `oag_fx::psys::Stage`
instances when it fires every tick (HD's measured rate), and the stage's 32 instances were
all trail hits: a play with no free slot recycles the instance with the fewest live
particles, which took detonations, absorb bursts and pickups. A maintainer report ("other
effects stop playing when trail hits happen") started this.

What changed, in `oag-fx` and `oag-raceplay`:

1. **Trail hits are expendable** (`Stage::play_expendable`): they take a free instance or
   recycle the oldest other trail hit, never another effect's. Any other play, and `attach`'s
   reserve of six, count them as free. Red-to-green test:
   `tests::trail_hits::a_detonation_started_while_trail_hits_stream_still_plays_to_completion`.
2. **The lent burst fires every other tick** per owner and intruder pair, on the first tick
   inside and every second one after (a maintainer ruling, "30-50 % fewer", on 2026-11-11).
   A burst the title authors (HD, and 2048 and Omega with the setting on) keeps the measured
   every-tick rate.
3. **`MAX_INSTANCES` is 64**, from a measurement: `oag-game --race --mode single_race --seed
   N --ticks 20000` on Pulse PSP EU, trail hits on (HD mounted as the lender), `OAG_MODS` an
   empty directory, the stage temporarily at 256 instances with a peak counter in
   `Stage::advance`, seeds 1 to 11. Peak instances that are not trail hits: 20, 27, 29, 23,
   22, 18, 21, 16, 23, 22, 27 (the worst, 29, is one wrecked craft: 9 attached
   `WO_SHIP_DEATH_SPARKS` and 9 `WO_SHIP_FXNODE_EXPLO`, over collision sparks, a quake and a
   bomb ring). Peak including trail hits at the every-other rate: 46, 53, 48, 46, 47, 71, 93,
   43, 87, 112, 52. A first set of runs with the default mods loaded (`oag.energy-turbo`,
   `oag.wall-penalty`) peaked at 56 non-trail instances in one seed: two wrecked craft (19
   attached of each wreck effect) over a rocket's flare, detonations and a leech beam. **The
   stock figure is 29; 64 is sized for two wrecks plus the six-instance attach reserve
   (about 55), the modded run's 56 being the observed instance of that.** The player's craft
   held the throttle and did not steer; the opponents flew. Memory, from `size_of`: 26,784 B
   an instance (1.7 MB for 64, 0.86 MB for 32), and three particle vertex buffers (additive,
   alpha-over, underlay) of `MAX_INSTANCES * 256 * 6` 100-byte vertices: 28.1 MiB for 64,
   14.1 MiB for 32.

Staged look: `--trail-sparks --give rocket --hold cross --press square` at 760 and 820 ticks
draws the fireball with the lent sparks on the craft at 32 and at 64 instances alike. The
staged scene holds one craft's burst, about 15 instances, so it does not exercise the
eviction; the red-to-green test above does.

Not measured: a rocket volley from several craft at once (the `--give rocket` aid fires
about 127 in flight, which no race reaches), and Pure. Checked against HD, 2048 and Omega:
the stage and the expendable rule are title-blind, and their burst rate is unchanged.

## Trail-hit burst colour

With `graphics.trail_hits` on, the lent `WO_TRAIL_HITSHIP` is redrawn in the hue of the ribbon
the craft was struck in, at the burst's own brightness (`oag_fx::psys::Tint`,
`Race::trail_hit_tint`). **A derivation, not a measurement.** HD picks its burst by the ribbon's
owner: `WO_TRAIL_HITSHIP` in a blue ribbon, `_RED` in a red one
(`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`, the call-site table: the variant byte is the owner's `+0x7d2c`).
Pulse and Pure author no burst, so the lent one would otherwise be HD's regardless of their
ribbon; this applies HD's rule to it.

**The rule.**

1. **The hue is the ribbon's visible colour**: the sum of the layers the platform draws, since the
   three are drawn additively and the burst sits on the hull inside the ribbon
   (`oag_fx::exhaust::TrailLaw::ribbon_colour`, computed from `LAYER_COLOUR`, nothing typed in).
   PSP (every layer): (1.43, 0.86, 1.81), luma 1.05, chroma direction (0.44, -0.22, 0.87), violet.
   PS2 (`PS2_FIRST_LAYER`, the deep blue layer is not submitted): (1.43, 0.82, 1.31), luma 0.99,
   chroma direction (0.78, -0.29, 0.56), pink-white. The outer layer alone, (0, 0.03, 0.50), is
   direction (-0.13, -0.06, 0.99), blue; it is not used because it is the rim of the ribbon, not
   what the core shows.
2. **Chroma swap, luminance kept.** Per vertex, `Y = 0.2126 R + 0.7152 G + 0.0722 B`, chroma
   `c = rgb - Y`; the vertex becomes `Y + |c| * unit(target chroma)`. Luminance is unchanged by
   construction, a white or grey vertex stays as it was (the white-hot core is not coloured), a
   saturated one keeps its strength of colour. A channel that would go negative shrinks the chroma
   rather than being clamped (a negative in an additive blend subtracts light). A plain multiply
   by the ribbon colour would only darken the core.
3. **Per instance**, set by `Stage::play_carried_expendable` and cleared when the slot is
   claimed again, applied where the stage gathers vertices. Render side only; no simulation
   state, determinism untouched.
4. **Only a lent burst.** `EffectHandles::is_lent(TrailHitship)` gates it: HD, and 2048 and Omega
   (which play their own with the setting on), draw the effect as authored. The Pulse and Pure
   ribbon is one colour law for every craft, so the owner does not change the hue; the PSP and
   PS2 laws differ.

**Evidence, and the limit of it.** Tests: `psys::tint::tests` (white stays white, blue takes the
target hue at equal luminance, no negative channel), `psys::carried::tests` (a tint recolours
its own instance only and a recycled slot forgets it), `tests::trail_hits` (a lent burst is the
ribbon's hue at the authored luminance; the PS2 law gives another hue; an authored burst is
drawn in the same colours as its effect draws it) and, on the discs,
`trail_hits_lent_ground_truth` (every coloured vertex of HD's real burst on a Pulse race points
within 0.999 of the ribbon's chroma direction).

**The change is small on the PSP, because the burst's vertex colours already point the
ribbon's way.** Measured on the discs (`trail_hits_lent_ground_truth`'s scene, a craft tucked
eight units into a rival's ribbon, the tint switched off for the measurement): of 792 vertices
with any chroma, the mean chroma direction is (0.42, -0.21, 0.87) against the PSP ribbon's
(0.44, -0.22, 0.87), and 756 sit within 0.99 of it; 36 do not (not inspected further). HD's palette ends (0.79, 0.43, 1.0), direction (0.46, -0.22, 0.86)
(`engine-trail.md`, "The two emitters"). The burst is violet at the vertex, and only the
display pipeline (additive stacking of hundreds of soft quads, bloom, clipping) makes its rim
read cyan. So on the PSP the tint moves the outliers and the rim, and the PS2's pink-white
ribbon is where it shows.

Frames, Pulse PSP EU, seed 7, `--trail-sparks`, tick 200, 1440x816, HD as lender; the burst is
74,597 pixels (frame with the setting off against on); a pixel counts when a channel moves by
more than 8 levels:

| pressing | pixels moved by the tint | share of the burst |
| --- | --- | --- |
| Pulse PSP | 4,617 | 6 % |
| Pure PSP | 4,156 | 6 % |
| Pulse PS2 | 24,433 | 33 % |
| Pulse PSP, tint forced red (probe) | 60,370 | 81 % |

On the PSP the moved pixels' mean goes from (144, 207, 246) to (171, 202, 244): cyan-white to
lilac-white. On the PS2 the cyan-white rim and sparks turn pink-violet, to match its pink-white
ribbon. The red probe recolours most of the burst (the rest is clipped to white), so no draw
path escapes the tint; it is a probe, not shipped.

**Open: which hue is "the ribbon".** The summed layers are a derivation from the authored
colours. In a PSP frame at 700 ticks the plume around the hull reads blue overall (chroma
direction (-0.14, -0.06, 0.99), the outer layer's own (-0.13, -0.06, 0.99)), though that sample
includes the hull and the bloom, and the strip below the hull reads (0.16, -0.15, 0.98). If a
player judges the ribbon blue, the outer layer is the other candidate; it is a one-line change in
`TrailLaw::ribbon_colour`. Checked against HD, 2048 and Omega: not applicable, they author the
burst and are never lent it; Pure draws Pulse's ribbon law (`oag_pure::race`), not measured
separately.
