# Weapon lineage

Where each weapon's behaviour, numbers, AI use, look and sound come from, per
title, and how this engine adjusts or shares the code that plays it.
[`overview/status.md`](../overview/status.md) says how *far along* a cell is;
this page says **whose law it is**. The two were conflated once: Pulse's per-tick LeachBeam drain ran on HD
with HD's damage value (1.5, against Pulse's 0.1), emptied the target's shield
in 94 to 119 ticks and wrecked the field, so opponents were made to hold the
beam for a few hours until HD's own once-per-trip drain was read
([`ps3-hdfury-eu/weapons.md`](../ghidra/functions/ps3-hdfury-eu/weapons.md),
confidence 80; [`ai.md`](ai.md), "An original opponent never fired, and now
does"). A cell that reads "inherited" here is a cell where that can happen
again.

The rule behind it (maintainer, 2026-10-19): **always measure**, and track
explicitly how a weapon's behaviour carries through titles and generations, how
it is implemented, and where the engine adjusts it per title.

## How to read a cell

Every cell opens with one source tag:

| Tag | Meaning | Score |
| --- | --- | --- |
| **M** | measured on this title: its own executable, its own data, or watched running. The page that holds the evidence is linked. | the page's own score; restated here only where it decides the cell |
| **I:X** | inherited from title X: the engine runs X's law (or X's name for a thing) and this title's own has not been read. Checked, not measured. | none |
| **C** | chosen by this project, not measured. Nothing in the original says so. | none |
| **A** | absent: the title does not have it. The evidence is linked, because "absent" needs evidence. | the page's own score |
| **?** | not investigated. It is neither inherited nor absent until someone looks; the cell says what would settle it. | none |

`unread` in the checked table below means the same as `?` here: a title with
no such thing recovered, as opposed to `absent`, a title that does not have it.

Pulse PSP and Pulse PS2 share **one** `oag_title::Title` (`oag_pulse::TITLE`:
one definition, two archive candidates), so everything this engine decides for
one it decides for the other. The PS2 column therefore says only what was
measured on the PS2 separately. PSP is the implementation target; a PS2
divergence implements the PSP.

Scores are not repeated per cell. They live on the linked page, next to the
instruction-level evidence. A tag with no link is a claim to distrust.

## Where the code adjusts a weapon per title

Four mechanisms, from most shared to most title-specific:

1. **One code path, no title input.** `oag_weapons` (pickup draw, projectiles,
   blasts, beams, slowdown) and `oag_raceplay`'s weapon glue run the same on every
   title. A weapon whose law is "inherited" runs here.
2. **A table the title names.** `oag_title::weapons::Weapons` picks the
   `WeaponStats_*.xml` (race and Eliminator) and the `WeaponAIstats.xml`
   the shared code reads its numbers from. The numbers are the title's own
   whenever the file is.
3. **A law the title selects.** `Weapons::leach_drain` is one: Pulse
   drains every tick, HD once per ball trip. `Weapons::quake_hit` is the other:
   the smoothed wave-height gate and cockpit lift, with Pure's variant holding the
   eye level (chosen) over the same gate. A new per-title law is a new field
   here, with provenance, not a `title.name` branch (ADR-0058, `just
   check-title-branching`).
4. **What a title draws.** `oag_title::weapons::WeaponModels` names each
   weapon's own body, blast, trail and strip, and `oag_title::Effects` its
   particle triggers. A field that is `None` draws nothing; it never falls back to
   another title's model.

## Checked axes

The table below is machine-checked by `scripts/check-weapon-lineage.py`
(`just check-weapon-lineage`, part of the gate). Each cell is
`` `value` - provenance ``. The script compares the value with the Rust source of
the five title crates, and the provenance word where the code carries an
`Origin` (`ghost_static`, `cockpit_hit`, the effect rows and the two Autopilot bools). It also fails when a public
field of `Weapons` or `WeaponModels` has no row here, so a new axis cannot land
unrecorded.

**Not checked**, and therefore only as good as the last person who looked: the
provenance of a `Weapons` axis other than the two Autopilot bools, which are
`Sourced` and so checked (the rest live in `//` comments, not in a field),
the `eye_origin` of `WaveHeightEyeLevel` (Pure: chosen; the row's provenance word is
the gate's `origin`), the
provenance of a `WeaponModels` field other than `ghost_static` and `cockpit_hit`, every cell of
the aspect tables further down, and the confidence scores on the linked pages.
Values: a path is the exact string in the source; `set` is `Some(..)` (or
`true`); `none` is `None` (or `false`).

| Axis | Pulse (PSP, PS2) | Pure | HD / Fury | 2048 | Omega |
| --- | --- | --- | --- | --- | --- |
| `weapons.race` | `Data\XML\WeaponStats_Race.xml` - measured | `Data\XML\weaponstats.xml` - measured | `Data\XML\WeaponStats_Race.xml` - measured | `Data\XML\weaponstats_Race_2048.xml` - chosen | `Data\XML\WeaponStats_Race.xml` - chosen |
| `weapons.elimination` | `Data\XML\WeaponStats_Elimination.xml` - measured | `none` - measured | `Data\XML\WeaponStats_Elimination.xml` - measured | `Data\XML\weaponstats_Elimination_2048.xml` - chosen | `Data\XML\WeaponStats_Elimination.xml` - chosen |
| `weapons.ai` | `Data\XML\WeaponAIstats.xml` - measured | `Data\XML\WeaponAIstats.xml` - measured | `Data\XML\WeaponAIstats.xml` - measured | `Data\XML\WeaponAIStats2048.xml` - chosen | `Data\XML\WeaponAIstats.xml` - chosen |
| `weapons.leach_drain` | `PerTick` - measured | `PerTick` - absent | `PerTrip` - measured | `PerTrip` - inherited | `PerTrip` - inherited |
| `weapons.quake_hit` | `WaveHeight` - measured | `WaveHeightEyeLevel` - inherited | `WaveHeight` - inherited | `WaveHeight` - inherited | `WaveHeight` - inherited |
| `weapons.autopilot_absorb_cancels` | `set` - measured | `none` - measured | `set` - measured | `set` - inherited | `set` - inherited |
| `weapons.autopilot_pad_grants` | `none` - measured | `none` - inherited | `none` - inherited | `none` - inherited | `none` - inherited |
| `weapon_models.rocket` | `set` - measured | `set` - measured | `set` - measured | `none` - unread | `set` - inherited |
| `weapon_models.mine` | `set` - measured | `set` - measured | `set` - measured | `none` - unread | `set` - inherited |
| `weapon_models.bomb` | `set` - measured | `set` - measured | `set` - measured | `none` - unread | `set` - inherited |
| `weapon_models.cannon` | `set` - measured | `none` - absent | `set` - measured | `none` - unread | `set` - inherited |
| `weapon_models.plasma_ball` | `none` - absent | `none` - absent | `set` - measured | `none` - unread | `none` - unread |
| `weapon_models.plasma_blast_pulse` | `set` - measured | `none` - unread | `none` - absent | `none` - unread | `none` - unread |
| `weapon_models.plasma_blast_hd` | `none` - absent | `none` - absent | `set` - measured | `none` - unread | `set` - inherited |
| `weapon_models.bomb_blast_pulse` | `set` - measured | `set` - measured | `none` - absent | `none` - unread | `none` - unread |
| `weapon_models.bomb_blast_hd` | `none` - absent | `none` - absent | `set` - measured | `none` - unread | `none` - unread |
| `weapon_models.missile_blast_hd` | `none` - absent | `none` - absent | `set` - measured | `none` - unread | `set` - inherited |
| `weapon_models.repulser_field` | `set` - measured | `none` - absent | `none` - unread | `none` - unread | `none` - unread |
| `weapon_models.mag_floor` | `set` - measured | `none` - absent | `none` - unread | `none` - unread | `none` - unread |
| `weapon_models.magstrip_wake` | `none` - absent | `none` - absent | `set` - measured | `none` - absent | `set` - inherited |
| `weapon_models.magstrip_pob` | `none` - absent | `none` - absent | `none` - absent | `set` - measured | `none` - unread |
| `weapon_models.rocket_trail` | `none` - unread | `none` - unread | `set` - measured | `none` - unread | `none` - unread |
| `weapon_models.leach_strip` | `none` - absent | `none` - absent | `set` - measured | `none` - unread | `none` - unread |
| `weapon_models.leachbeam_ball` | `none` - unread | `none` - absent | `set` - measured | `none` - unread | `none` - unread |
| `weapon_models.shuriken` | `set` - measured | `none` - absent | `none` - unread | `none` - unread | `none` - unread |
| `weapon_models.cannon_look` | `none` - measured | `none` - absent | `set` - measured | `none` - unread | `set` - inherited |
| `weapon_models.ghost_static` | `set` - measured | `set` - measured | `none` - measured | `set` - inherited | `set` - inherited |
| `weapon_models.cockpit_hit` | `set` - measured | `set` - inherited | `none` - measured | `none` - measured | `none` - unread |
| `effects.hit_spark` | `set` - measured | `none` - unread | `none` - unread | `none` - unread | `none` - unread |
| `effects.leach_hit_spark` | `set` - measured | `none` - absent | `set` - measured | `none` - unread | `none` - unread |
| `effects.weapon_spark` | `none` - unread | `none` - unread | `set` - measured | `none` - unread | `none` - unread |
| `effects.shield_absorb` | `set` - measured | `set` - measured | `set` - measured | `none` - unread | `none` - unread |
| `effects.bomb_rays` | `none` - absent | `none` - absent | `set` - measured | `none` - unread | `none` - unread |

Notes on the cells the code does not explain by itself:

- **`weapons.race` on 2048 and Omega is chosen.** 2048 ships HD's three tables
  and two suffixed `_2048` ones; the suffix is the only evidence the race reads
  the latter, and no executable read says which file the race opens. Omega
  names HD's unsuffixed file for the same reason in reverse. Both decode
  (`docs/formats/weapon-stats.md`, "Omega checks against HD").
- **`weapons.elimination` is read.** `Weapons::elimination`'s own doc comment,
  and the HD and Pure comments in their `lib.rs`, still say nothing opens it;
  that was true until 2026-09-08. `oag_raceplay`'s `load_weapons` opens it for
  `Mode::Eliminator` and falls back to the race table, with a report line, on a
  title whose field is `None`. The comments are stale (a `.rs` change, so not
  fixed here).
- **`weapons.leach_drain` on Pure is `absent`**: Pure ships no LeachBeam
  (`psp-pure-usa/weapons.md`, confidence 90), so `PerTick` is an inert value
  the field needs, not a law Pure runs.
- **`weapons.ai` is a table, not a law.** Each title ships a file (searched
  2026-10-03, so the file is measured data); the decision code that reads it is
  Pulse's on every title, inherited, see "AI use" below. 2048 and Omega are
  chosen because two byte-identical copies ship and nothing says which the race
  opens. Pure's entry is the same name's hash, the spelling Pulse's.
- **Effect rows**: `set` means the title answers that trigger itself in its
  `effects.rs` (or 2048's and Omega's `EFFECTS`). A title that shares
  `Effects::engine`'s defaults for every other trigger inherits those names
  from Pulse; the rows above are the triggers each title has to answer for itself.
- **`weapon_models.*` on Omega is inherited, played from Omega's own archives.**
  `data00` ships HD's `Data/weapons/HD_*.vex` set and the executable's ribbon-model
  table is HD's, in HD's order; every law that draws the models (placement, the cull
  rule, the cannon's look) is HD's, read off HD's executable and unconfirmed on PS4.
  **Which root a race reads is the mode's, not the circuit's**: `Rocket_Construct`
  (`0x01368610`) formats `Data/Weapons/%s` when `DAT_01f999e4 < 0x17` and
  `Data/Weapons2048/%s` otherwise, and every mode this project launches on Omega is
  below `0x17`, so one table is right for every race there is, a 2048-era circuit
  included. **Held back**: `plasma_ball`, `bomb_blast_hd`, `leachbeam_ball` and
  `leach_strip` stay `none` because what draws is wrong or invisible: their HD
  materials are shaded by fragment programs `oag_mesh::mesh::rcs::rim_glow`
  recognises by fingerprint on the PS3's RSX code (`RIM_EDGE`, `RIM_GLOW`,
  `BOMB_FIRE`), and the PS4 builder sets none of those bits. `oag-raceplay`'s
  `omega_weapon_bodies_ground_truth` pins both halves.
- **`ghost_static`**: `none` on HD is a measurement (no `staticglow` string in
  its EBOOT); 2048's and Omega's entry is Pulse's name for a file they ship as
  `.gxt` and `.gnf`.
- **`cockpit_hit`**: the noise `CockpitHitFx` draws over the cockpit view on a weapon
  hit to the player (`oag_fx::cockpit_hit`). Pulse's is measured and watched live.
  Pure's `BOOT.BIN` names the same `noisematt.mip` (and a second, `noisemattvr.mip`,
  that Pulse's does not), the discs ship it decoding to the same 64 by 64 pixels as Pulse's (USA and EU, both
  games), and Pure's `Ship_Damage` is unread, so the arm and the draw are
  Pulse's: `set - inherited`. `none` on HD and 2048 is a measurement (no `noisematt`
  string in HD's `EBOOT.elf` or 2048's decrypted `eboot.elf`); Omega's executable cannot
  be read, so it follows HD and draws none (`unread`). The Pulse PS2 pressing names
  `noisematt.mip` in `SCES_547.48` and ships it as a 64 by 64 `.pct`, which loads.

## The aspect tables

Rows are the weapons and pickup effects each title has, columns the titles.
Cells are short on purpose: the page behind the link is the evidence. Where
the Pulse PSP and PS2 columns are merged into one, PS2 equals PSP except the
named exceptions (the PS2 `extrashield.vex` shell, its engine flare as a particle
effect, and its tuning table, which is equal to the PSP's, see below).

### Implementation: the shared path and what adjusts it per title

Each row is the code every title runs, and the fields that make a title
differ. A weapon with no per-title field runs identically on every title that
has it; only its table moves.

| Weapon | Shared code | Per-title data that adjusts it |
| --- | --- | --- |
| Rocket | `oag_weapons::projectile::rocket` on the `flight` floor follower; player glue `oag_raceplay::weapons`, opponents `field/opponent_weapons.rs` | `Weapons.race`/`.elimination` (numbers); `WeaponModels.rocket`, `.rocket_trail`; `Effects` `RocketFlare`, `TrackBlast`, `CraftBlast` |
| Missile | `projectile::missile` (lock, guidance, five bounces); `oag_raceplay::weapons::reticle` | the table; `WeaponModels.missile_blast_hd`; `Effects` `MissileFlare`, `MissileExplo`, `MissileBounce` |
| Quake | `projectile::quake` (the wave, `raw_lift`, `update_lift`); the road ripple in `oag_render::ripple`; the cockpit eye in `oag_render::camera::internal` | the table; **`Weapons.quake_hit`** (`WaveHeight`, or `WaveHeightEyeLevel` where the cockpit eye stays level); `Effects.quake_look` (`QuakeLook`: up, lift, wash), `.extents`, `.sprites`; `Effects` `Quake` |
| Cannon | `projectile::cannon` | the table; `WeaponModels.cannon`, `.cannon_look` (`CannonLook`, `CannonBody`); `Effects` `CannonSparks`, `WeaponSpark` |
| Turbo | `oag_weapons::pickup`, then `oag_physics::engine` | the table only |
| Shield | `oag_weapons::pickup`, then `oag_physics::damage` | the table; `Looks` (`ShieldPalettes`, hull overlay rule) |
| Autopilot | `oag_weapons::pickup`, then `oag_ai::original::Autopilot` (the player's own original driver) through `Race::autopilot_controls` | the table; the title's original AI tables (controller row, `Laws`); this project's driver where none read |
| Plasma | `projectile::plasma` on the `flight` follower | the table; `WeaponModels.plasma_ball`, `.plasma_blast_pulse`, `.plasma_blast_hd`; `Effects` `PlasmaFlare`, `PlasmaBlast`, `PlasmaLightning*` |
| Bomb | `projectile::mine` (shared with Mine), `oag_raceplay::bomb_blast` | the table; `WeaponModels.bomb`, `.bomb_blast_pulse` (`shockwave_fades`), `.bomb_blast_hd`; `Effects` `BombSmokering`, `BombRays` |
| Mine | `projectile::mine` | the table; `WeaponModels.mine`; `Effects` `MineExplo` |
| LeachBeam | `projectile::leach_beam` (`Beam`, `Beam::per_trip`, `trip`); `oag_raceplay::leach_strip` | **`Weapons.leach_drain`** (`PerTick`, `PerTrip`); the table; `WeaponModels.leach_strip`, `.leachbeam_ball`; `Effects` `Leachbeam*`, `LeachHitSpark` |
| Repulser | `projectile::repulser` (+ `fork`); `oag_raceplay::repulser_field` | the table; `WeaponModels.repulser_field`; `Effects` `RepulserBlast`, `Repulser` |
| Shuriken | `projectile::shuriken` on the `flight` follower | the table; `WeaponModels.shuriken`; `Effects` `Shuriken*` |
| Disruptor | `projectile::disruptor`, `oag_weapons::disruption`; `oag_raceplay::weapons::disruptor` | the table (Pure ships the only one); no field of its own |
| Pickup draw | `oag_weapons::pickup` | the table's `<Pickupodds>`; `Title.weapon_pad_glow` for the pad's look |
| Absorb | `oag_physics::damage` absorb branch | the table's `absorb`; `Effects` `ShieldAbsorb` with its burst |
| Slowdown stacking | `oag_weapons::slowdown`, clamped to the table's `Global slowdown_limit` | the table's `slowdown_time` per weapon |
| Fire law | `oag_ai::weapon_ai`, `oag_raceplay::field::fire_law` (`FireLaw`) | **`Weapons.ai`** (the odds file); `FireLaw::Ours` when it is missing |
| Sound | `oag_sound::sfx::cue`, push sites in `oag_raceplay::weapons` and `field/opponent_weapons.rs` | the title's `weapons.bnk` path (`SOUND_BANKS` in its `race.rs`) |
| EMP, Light Barrier, GiftCannon | none | none: authored, nothing reads them |

### Behaviour law

| Weapon | Pulse PSP | Pulse PS2 | Pure | HD / Fury | 2048 | Omega |
| --- | --- | --- | --- | --- | --- | --- |
| Rocket | M: ground truth, [rocket_floor_trace.rs](../../crates/raceplay/tests/rocket_floor_trace.rs), [pickups.md](pickups.md) | I:Pulse PSP | I:Pulse; same code, Pure's table | I:Pulse; `Rocket_Construct` read, law unread ([weapons.md](../ghidra/functions/ps3-hdfury-eu/weapons.md)) | I:Pulse; unread | ?: constructors read ([weapons.md](../ghidra/functions/ps4-omega-eu/weapons.md)), law unread |
| Missile | M: [missile.md](../ghidra/functions/psp-pulse-usa/missile.md), [missile_ground_truth.rs](../../crates/raceplay/tests/missile_ground_truth.rs) | I:Pulse PSP | I:Pulse; same code, no Pure test | I:Pulse; `MissileManager_Construct` read | I:Pulse; unread | ?: read, not ported |
| Quake | M: the travelling instance, the road ripple, the hit gate (rising edge of the craft's smoothed wave height `> 0.1`) and the cockpit eye lift by that height, [cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md#2026-10-26-what-a-quake-does-to-the-craft-it-reaches-quake-craft-re-lane); one per race, a press while one is live keeps the pickup and stays armed (65, ours drops it) | I:Pulse PSP | M: its own wave extent, frame and wash, [quake.md](../ghidra/functions/psp-pure-usa/quake.md); road ripple open; the gate is I:Pulse, and the cockpit eye is held level, **C**: the maintainer, 2026-10-26, "in Pure this does not exist" (Pure's camera path unread) | I:Pulse; `QuakeManager_Construct` read; M: one per race, a press while one is live keeps the pickup and stays armed, as Pulse ([weapon-instance-limits.md](../ghidra/functions/ps3-hdfury-eu/weapon-instance-limits.md), 90 and 65) | I:Pulse; M: one per race, Pulse's guard ([weapon-instance-limits.md](../ghidra/functions/vita-2048-eu-v104/weapon-instance-limits.md), 80) | ?: read, not ported; M: one per race, Pulse's guard ([weapon-instance-limits.md](../ghidra/functions/ps4-omega-eu/weapon-instance-limits.md), 80) |
| Cannon | M: [cannon_ground_truth.rs](../../crates/raceplay/tests/cannon_ground_truth.rs) | I:Pulse PSP | A: not in Pure's roster ([weapon-stats.md](../formats/weapon-stats.md)) | I:Pulse; `CannonManager_Construct` read; `recharge_time` authored, unread | I:Pulse; unread | ?: read, not ported |
| Turbo | M: [pickups.md](pickups.md#what-a-fired-turbo-does); the running bit, the stun escape, the pad fade and the d-pad Up boost lift, [craft-flags.md](../ghidra/functions/psp-pulse-usa/craft-flags.md) | I:Pulse PSP | I:Pulse; the same four engine reads of `0x200` and a pad clock at `craft+0x284`, read not watched | M: the running bit is `craft+0x264 & 0x20`, mirrored from the slot's `0x200` a frame later, with Pulse's stun escape, pad fade (clock `craft+0x320` held at `0`, watched live) and d-pad Up lift (read), [craft-flags.md](../ghidra/functions/ps3-hdfury-eu/craft-flags.md#ship_updateturboflag-0x000cf7a8-and-the-pad-fade); no manager class located | I:HD; 2048's `craft+0x264` bit `0x20` has a writer (`0x811bf0b4`, v1.04), its readers unread | ?: no manager located |
| Shield | M: [shield-pickup.md](../ghidra/functions/psp-pulse-usa/shield-pickup.md) | M: same law, own model ([ps2 page](../ghidra/functions/ps2-pulse-eu/shield-pickup.md), [ps2_shield_ground_truth.rs](../../crates/raceplay/tests/ps2_shield_ground_truth.rs)) | I:Pulse | M: same law as Pulse, two colours moved ([shield.md](../ghidra/functions/ps3-hdfury-eu/shield.md)) | I:Pulse; unread | ?: HD's `Shield` table applies, law unread |
| Autopilot | M: the player's own original driver takes the craft: lane `0`, throttle `100`, the rail kept through contact ([ai-controls.md](../ghidra/functions/psp-pulse-usa/ai-controls.md#the-players-own-driver-the-autopilot), [ai.md](ai.md#the-autopilot-flies-the-originals-player-driver-2026-10-19)); the engine runs thrust x1.2 with no acceleration cap while it does (live `craft+0x1c0 = 0xd`, [ai.md](ai.md#the-autopilots-rail-is-the-opponents-and-its-pace-is-the-engines-2026-10-19)); in Zone the x1.2 multiplies the auto-speed target as well (`0x0884c874`, live `craft+0x328 = 81.6`, [zone-start.md](../ghidra/functions/psp-pulse-usa/zone-start.md#the-autopilots-speedup-reaches-the-auto-speed-too-2026-10-20)) | I:Pulse PSP | M: throttle `100` (the cruise law skipped); lane `0` I:Pulse, Pure's planner unread for the player; engine bits M: the same x1.2 and cap lift off its executable (`craft+0x258`, [hd autopilot.md](../ghidra/functions/ps3-hdfury-eu/autopilot.md)) | M: the same three branches read (thrust `0x00100d7c..0x00100d9c`, planner `0x00102554..0x00102574`, rail `table+0x08`); engine bits M: read and live on RPCS3, `craft+0x264 = 0xd`, `craft+0x2f0 = 1.2` ([hd autopilot.md](../ghidra/functions/ps3-hdfury-eu/autopilot.md)); in Zone I:Pulse (HD's Zone engine, `Craft_UpdateThrottle` `0x000efb50`, is not Pulse's shape and reads no `+0x2f0`; not compared) | checked, differs: the player's driver steers with the pilot assist's gains ([ai-controls.md](../ghidra/functions/vita-2048-eu-v104/ai-controls.md)); the opponents' controller flies it here; engine bits M: the same law off its executable (`craft+0x55c`, [hd autopilot.md](../ghidra/functions/ps3-hdfury-eu/autopilot.md)) | checked, differs: as 2048 ([ai-controls.md](../ghidra/functions/ps4-omega-eu/ai-controls.md)); engine bits M: the same law off its executable (`craft+0x604`, [hd autopilot.md](../ghidra/functions/ps3-hdfury-eu/autopilot.md)) |
| Autopilot cancel and release | M: absorb cancels, fire does nothing, no pad grant mid-stint, a held Autopilot cannot re-fire, no corner hold ([autopilot.md](../ghidra/functions/psp-pulse-usa/autopilot.md#absorb-cancels-it-fire-does-not-and-no-corner-holds-it-2026-10-26-autopilot-re)); expiry in a corner, on a steep bank or at 290 degrees a second releases on the timer like a straight, hand-back in one tick with no ease, the `disengaging` call one second before the release and on a cancel with time left, all watched live (the pasted "extended in a turn zone" claim, refuted: [autopilot.md](../ghidra/functions/psp-pulse-usa/autopilot.md#expiry-mid-corner-and-the-pasted-extended-in-a-turn-zone-claim-2026-10-26-autopilot-expiry));  ours the same (wired 2026-10-26): an absorb cancels with or without a pickup held, spends and pays out nothing and hands the craft back on the next tick; a fire press under a running Autopilot is ignored, which also keeps a held Autopilot from being fired into it; a pad grants nothing mid-stint (`weapons.autopilot_absorb_cancels`, `weapons.autopilot_pad_grants`); a cancel with more than a second left says `disengaging` (I: the next `Autopilot_Update` crosses its one-second edge from above, static, 85, not heard live) | ?: not investigated | M: no cancel (static, 70), no hold (live, 76 expiries on Chenghou Project including its sharpest corners, [autopilot.md](../ghidra/functions/psp-pure-usa/autopilot.md#expiry-mid-corner-live-2026-10-26-autopilot-expiry)); ours: no cancel, and the rest I:Pulse | M: absorb cancels, static ([autopilot.md](../ghidra/functions/ps3-hdfury-eu/autopilot.md#absorb-cancels-it-here-too-2026-10-26-autopilot-re)); ours cancels, the pad gate and the fire suppression I:Pulse; no hold and no ease: no store adds to the slot timer, the blend is only zero-tested, and 34 expiries on Chenghou Project (24 forced into its hairpins at 60 to 122 degrees a second) all released on the timer with the pad record back in two frames ([autopilot.md](../ghidra/functions/ps3-hdfury-eu/autopilot.md#expiry-mid-corner-2026-10-26-autopilot-expiry), live) | M: no hold, no ease, static (80): the update is the one-tick shape with the `disengaging` edge, nothing conditions the timer on the craft, and the blend is only zero-tested ([autopilot.md](../ghidra/functions/vita-2048-eu-v104/autopilot.md)); the cancel unread, ours as HD's | I:HD: unread; ours as HD's |
| Plasma | M: [plasma_ground_truth.rs](../../crates/raceplay/tests/plasma_ground_truth.rs); flight against the original per update and the detonation tick, 2026-10-31: [plasma_timing_ground_truth.rs](../../crates/raceplay/tests/plasma_timing_ground_truth.rs) | I:Pulse PSP | M: hardcodes the same 1 s charge ([weapons.md](../ghidra/functions/psp-pure-usa/weapons.md)) | M: blast mechanics read ([plasma.md](../ghidra/functions/ps3-hdfury-eu/plasma.md)); flight law I:Pulse, including the 2026-10-31 launch scale, first-ride gate, ride height, muzzle and steer (measured on Pulse only, `flight.rs` has no title gate) | I:Pulse; unread | M: read ([plasma.md](../ghidra/functions/ps4-omega-eu/plasma.md)), not ported |
| Bomb | M: [mine_ground_truth.rs](../../crates/raceplay/tests/mine_ground_truth.rs); `damageradius` has no consumer | I:Pulse PSP | M: fuse-less dialect, [weapons.md](../ghidra/functions/psp-pure-usa/weapons.md#pures-bomb-has-no-fuse-and-damageradius-has-no-reader) | M: own blast object read (`NormalBomb`); `number_of_shots_to_destroy` authored, unread | I:Pulse; `WeaponType` splits Mine from Bomb ([weapon-type-bits.md](../ghidra/functions/vita-2048-eu-v104/weapon-type-bits.md)) | ?: read, not ported |
| Mine | M: [mine_ground_truth.rs](../../crates/raceplay/tests/mine_ground_truth.rs) | I:Pulse PSP | I:Pulse | I:Pulse; `MineManager_Construct` read | I:Pulse; unread | ?: shares one composite with LeachBeam, no own constructor |
| LeachBeam | M: link, drain, repair, `slowShipFactor`; no disc-backed test; one per race, a press while it is up **spends** the pickup (88; ours keeps it) | I:Pulse PSP | A: ships none ([weapons.md](../ghidra/functions/psp-pure-usa/weapons.md#pure-ships-no-leachbeam-checked-2026-10-12-lane-pure-leach-quake-confidence-90), 90) | M: once per ball trip (80, [weapons.md](../ghidra/functions/ps3-hdfury-eu/weapons.md)); one per race, a press while it is up spends the pickup, as Pulse ([weapon-instance-limits.md](../ghidra/functions/ps3-hdfury-eu/weapon-instance-limits.md), 88 and 85) | I:HD via Omega; own `repair` is 30.0, law unread; M: **eight** per race, a press with eight up spends the pickup ([weapon-instance-limits.md](../ghidra/functions/vita-2048-eu-v104/weapon-instance-limits.md), 85; ours runs HD's one) | I:HD; built inline with Mine, no own constructor, the drain not read; M: **eight** per race, as 2048 ([weapon-instance-limits.md](../ghidra/functions/ps4-omega-eu/weapon-instance-limits.md), 85; ours runs HD's one) |
| Repulser | M: [repulser.md](../ghidra/functions/psp-pulse-usa/repulser.md); not runtime verified | I:Pulse PSP | A: not in Pure's roster | I:Pulse; `Repulser_Construct` read; HD's Eliminator odds weight it | I:Pulse; unread | ?: no `Repulser.cpp` tag found, absent or unnamed |
| Shuriken | M: [shuriken_ground_truth.rs](../../crates/raceplay/tests/shuriken_ground_truth.rs) | I:Pulse PSP | A: not in Pure's roster | I:Pulse; no manager located | I:Pulse; unread | ?: unread |
| Disruptor | A | A | M: whole law, grant to victim ([weapons.md](../ghidra/functions/psp-pure-usa/weapons.md)) | A | A | A |
| EMP, Light Barrier | A | A | A | M: authored (`EMP` in the Detonator table, `LightBarrier` in Fury's race table), not wired, `Weapon::from_type` drops both; read, gated to Detonator and Zone Battle ([weapons.md](../ghidra/functions/ps3-hdfury-eu/weapons.md)); not built | ?: both ship in the 2048-named tables, unread | M: `EMPManager_Construct`, `LightBarrierManager_Construct` read |
| GiftCannon | A | A | A | A | ?: authored only in `weaponstats_*_2048.xml`, nothing reads it | ?: authored in the `_2048` copies, Omega reads HD-named files |
| Pickup draw (`<Pickupodds>`) | M: [pickups.md](pickups.md), 92 | I:Pulse PSP | I:Pulse: Pure's own odds, Pulse's draw | I:Pulse draw, HD's own odds | I:Pulse draw, 2048's own odds | I:Pulse draw; a fifth `SuperPhantom` class is unread |
| Absorb (energy paid back) | M: [shield.md](../ghidra/functions/psp-pulse-usa/shield.md) | I:Pulse PSP | I:Pulse | I:Pulse | I:Pulse | ?: unread |
| Slowdown stacking | M: `Ship_AddSlowdown` clamped to `slowdown_limit`, [engine.md](../ghidra/functions/psp-pulse-usa/engine.md) | I:Pulse PSP | I:Pulse | I:Pulse | I:Pulse; its own `slowdown_time`s are half Pulse's | ?: unread |

### Tuning table

The numbers each title authors. Compared by reading Pulse's table out of its
WAD (EU PSP, and EU PS2 for the second column) and HD, 2048 and Omega's out of
their archives, 2026-10-19; the commands, the differing fields and the scores
are in [weapon-stats.md](../formats/weapon-stats.md#the-race-tables-compared-across-titles-2026-10-19).

| Weapon | Pulse PSP | Pulse PS2 | Pure | HD / Fury | 2048 | Omega |
| --- | --- | --- | --- | --- | --- | --- |
| Rocket, Missile, Plasma, Bomb, Mine, Cannon, Quake, Repulser, Shuriken, Turbo, Shield, Autopilot | M: `WeaponStats_Race.xml`, [weapon-stats.md](../formats/weapon-stats.md) | M: the same 14 blocks, equal value for value to the PSP's EU copy | M: its own 10-weapon `weaponstats.xml`, one `speed` per weapon ([pure_weapons_ground_truth.rs](../../crates/tables/tests/pure_weapons_ground_truth.rs)) | M: HD's own file. Equal to Pulse's on every field of these weapons except `Mine.slowdown_time` (0.10 against 0.16), a per-class `Quake` speed, `Cannon.recharge_time` and `Bomb.number_of_shots_to_destroy` ([hd_weapons_ground_truth.rs](../../crates/tables/tests/hd_weapons_ground_truth.rs)) | M: its own `_2048` file, chosen by suffix. `slowdown_time` halved on every weapon; adds `screen_shake_*` (unread), `superphantomspeed` (unread) | M: HD-named files decode equal to Fury's apart from `superphantomspeed`; which copy a race reads is C; the `_2048` pair also ships and is unread |
| LeachBeam | M: `repair` 0.1, `damage` 0.1, `energy_multiplier` 50, `slowShipFactor` 0.8 | M: equal to the PSP's | A | M: `repair` 2.0, `damage` 1.5, `energy_multiplier` 2.0, `slowShipFactor` 0.4. **Every figure moved**, the three race copies disagree, `DATA00` is read | M: `repair` **30.0**, `damage` 1.5, `energy_multiplier` 2.0, `slowShipFactor` 0.4 | I:HD; Omega's HD-named copy is equal |
| Pure's Disruptor | A | A | M: [pure_weapons_ground_truth.rs](../../crates/tables/tests/pure_weapons_ground_truth.rs) | A | A | A |
| LightBarrier, EMP | A | A | A | M: authored, unread | M: authored, unread | M: authored, unread |
| GiftCannon | A | A | A | A | M: authored, unread | M: authored, unread (`_2048` copy) |

### AI use and fire law

Every title's opponents fire the **same code**: `oag_ai::weapon_ai`, which is
Pulse's `WeaponAi_DecideFireOrAbsorb`, fire and absorb halves (`0x088518b4`, confidence 85,
[weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md)), on the title's
own `WeaponAIstats.xml` odds. HD's loader for that file is read (85,
[weapon-ai-stats.md](../ghidra/functions/ps3-hdfury-eu/weapon-ai-stats.md)); **no
other title's decision code is read**. A race whose title names no table, or
whose table fails to read, runs `FireLaw::Ours`, which is chosen, not measured
(`crates/raceplay/src/field/fire_law.rs`).

| Weapon | Pulse | Pure | HD / Fury | 2048 | Omega |
| --- | --- | --- | --- | --- | --- |
| Law that decides to fire | M: both halves ported, fire and absorb, once a tick (the cadence read 2026-10-19: `AiManager_Update` calls every `WeaponAi_Update` once a world step, [weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md#the-decision-cadence-every-call-confidence-85)) | I:Pulse; own odds | I:Pulse; own odds, an `AllWeapons` row and an `EliminatorAIStats` row parsed and not consumed | I:Pulse; own odds | I:Pulse; own odds |
| Rocket, Missile, Plasma, Shuriken, Repulser, LeachBeam | M: aimed, fired or absorbed on the law's rolls | I:Pulse (those it has) | I:Pulse | I:Pulse | I:Pulse |
| Quake | M: rolled, not aimed; absorbed on the same law | I:Pulse | I:Pulse | I:Pulse | I:Pulse |
| Cannon | ?: the original flags it a forward weapon, the port does not aim it (`opponent_fires`; see `Driver::holds_fire`) | A | ? | ? | ? |
| Mine, Bomb | C: laid by `Driver::wants_to_drop`, not the original's law | C | C | C | C |
| Turbo | C: fired when the plan and the corner model allow `speed + 220` over the boost (`Driver::affords_boost`); the original fires it on the roll alone, no gate ([weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md#the-turbo-has-no-gate-in-the-original)). The 220 is chosen on Pulse physics only | C: Pulse's gate, unmeasured on Pure physics | C: same | C: same | C: same |
| Shield | C: absorbed, never raised | C | C | C | C |
| Autopilot | M: never fired in a race: `WeaponAi_Update`'s default arm leaves the fire index at `0` and `g_weapon_ai_rate[0]` is `0` ([weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md)); in an Eliminator the index becomes `1` and the authored row (`useAgainstAI 1.2`) gives `0.012` a decision, its effect on an opponent unread (`Autopilot_Fire` has no player gate; the blend's setter is not found). The port (C) flies the original's player driver for the stint per class and difficulty: VECTOR, VENOM and FLASH at every level, RAPIER at Skilled and Elite, never PHANTOM or SUPERPHANTOM (`oag_title::opponent_autopilot::BY_CLASS_AND_DIFFICULTY`), measured on this engine per class and level ([ai.md](ai.md#an-opponents-autopilot-per-class-and-difficulty-2026-10-20)) | I:Pulse (never fired); C: as Pulse, measured on its own tables | I:Pulse (never fired); C: as Pulse, measured on its own tables | I:Pulse (never fired); C: as Pulse, measured on its own tables | I:Pulse (never fired); C: as Pulse, measured on its own tables |
| Disruptor | A | C: opponents never fire it (not in `opponent_fires`' forward list) | A | A | A |

The 240 s single-race census in [`ai.md`](ai.md) (opponent projectiles
fired: Pulse 244, Pure 38, HD 168, 2048 288, Omega 156) is the only place the
inherited fire law is measured **across** titles, and what it measures is that
it fires, not that the rate matches any original's.

### Visuals

| Weapon | Pulse | Pure | HD / Fury | 2048 | Omega |
| --- | --- | --- | --- | --- | --- |
| Rocket | M: `Rocket.vex`, `WO_ROCKET_FLARE`/`_EXPLO` | M: `Rocket.vex`, its own spelling | M: `hd_Rocket.vex`, ribbon trail | ?: no model named; `WO_ROCKET_FLARE` is looked up as Pulse's; checked, applies, not wired: `Data/Weapons2048/hd_Rocket.vex` ships | I:HD: `Data\Weapons\hd_Rocket.vex`, the root `Rocket_Construct` picks for a mode below `0x17` ([weapons.md](../ghidra/functions/ps4-omega-eu/weapons.md)); same mesh and decals as HD's, unlit by HD's weapon lights, no ribbon trail |
| Missile | M: `WO_MISSILE_HEAD` at both anchors; twin trails not built | I:Pulse (effect names) | M: `HD_missile_explosion.vex` on its own keys | ?: effect names only | I:HD: `HD_missile_explosion.vex`, on HD's keys; the structure draws, the white-out does not (the program bits below) |
| Quake | M: `WO_QUAKE`, road ripple, orange tint; a hit in the cockpit view also draws the `noisematt` static for 0.6 s (`CockpitHitFx`, [measured and wired](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md#wired-2026-10-26-quake-feel-wire-lane), on every weapon hit to the player) | M: own frame, lift, wash ([quake.md](../ghidra/functions/psp-pure-usa/quake.md)); a weapon hit in the cockpit view draws Pulse's `noisematt` static (I:Pulse, Pure's executable names the file) | M: spreads edge to edge by the extent law | I:Pulse (effect names) | I:Pulse |
| Cannon | M: `pulse_muzzleflash.vex` on the round, bolt and flash quads | A | M: `hd_muzzleflash.vex` at the muzzle, own textures and size range | ?: not drawn | I:HD: `hd_muzzleflash.vex` at the `cannon_flash` locator, HD's look numbers with Omega's `.gnf` textures; `CannonBullet`'s own draw is unread |
| Plasma | M: charge glow, three-model blast with a baked track, two blasts on a craft hit (bolt and craft, [plasma.md](../ghidra/functions/psp-pulse-usa/plasma.md#a-craft-hit-spawns-two-detonation-objects-and-fun_0883e37c-does-not-degate-it); HD and Omega run one) (2026-10-30 matched pair, then GE dumps, then wired: the blast's models play their tracks at the age, ease their scale per tick, tint by the dumps' ramps, the hemispheres sit in a world-fixed track frame and the halo takes the camera's orientation; the head's loader census holds but its template playback and the blast's onset flash are open, [plasma.md](../ghidra/functions/psp-pulse-usa/plasma.md#2026-10-30-the-blast-wired-from-the-dumps-pulse-plasma-blast-wire)) | ?: the three-model blast is in the executable, its key tables are unread, nothing is wired | M: `HD_plasma_ball`, ring, sphere and halo on eased scale ramps | ?: `WO_PLASMA_FLASH` is not authored there; nothing drawn | I:HD: ring, sphere and halo of the blast; the bolt's head (`HD_plasma_ball`) is held back, it draws as a barely visible ghost |
| Bomb | M: hemisphere, shockwave, smoke ring | M: same pair, no alpha fade | M: fireball, white core, bloom disc, eight rings, rays | ?: not drawn | I:HD: the laid body only; the blast set (fireball, core, bloom disc, rings, halo) is held back, its bloom ring paints the fire-ramp lookup texture over the road as a red wash |
| Mine | M: `Pulse_Mine.vex`, spin | M: `Mine.vex` | M: `HD_Mine.vex` | ?: not drawn | I:HD: `HD_Mine.vex`; the green rays are thinner than HD's |
| LeachBeam | M: ribbon, `WO_LEACHBEAM_ENERGY`, hull overlay | A | M: strip, ball (named, placement partly read), `WO_LEACHBEAM_*` | ?: `WO_LEACHBEAM_ENERGY` not authored; strip unread | ?: not drawn; the ball and the strip are held back: the ball draws as a grey bubble, the strip as a line too faint to see (Omega's own ribbon model names `hd_leechbeam.gnf`, 128x32, where HD's names `hd_leechbeam_glow`, 256x256) |
| Repulser | M: field model, `WO_REPULSER_BLAST`, matched to the PSP | A | ?: no field model recovered | ?: not drawn | ?: not drawn |
| Shuriken | M: `pulse_shuriken.vex`, `WO_SHURIKEN_*` | A | ?: no blade model recovered | ?: not drawn | ?: not drawn |
| Shield | M: three colours, shell and sphere; PS2 draws `extrashield.vex` | ?: not investigated | M: two of three colours read | I:Pulse | I:Pulse |
| HUD rounds left (`SubWeapon`, Cannon) | M: `PlayerStatus_Update` `%d` of `craft+0x154`, armed at the grant ([sub-weapon.md](../ghidra/functions/psp-pulse-usa/sub-weapon.md)); drawn under the icon | A | M: the same writer (`0x000cb928`), drawn under the icon | ?: checked, applies, not wired: the same writer, but the widget sits in the grant announcement's cluster, whose runtime visibility is unread | M: the same writer, read in Omega's own executable (`FUN_012e5470`, `%d` of `pickup+0x164`); drawn through HD's layouts |
| HUD seconds left (`SubWeapon`, Autopilot) | M: `%1.2f` of the timer, seen live (`5.00`) and matched by ours; the icon stays up while it runs | M: read (`0x08922340`); drawn, though Pure's pickup icons do not | M: the same writer; drawn | ?: the same, held back with the line above | M: the same writer, read in Omega's own executable (`%1.2f` of `pickup+0x158`); drawn |
| Hit sparks | M: `Ship_Damage` branch | ?: `Ship_Damage` unread | M: `WO_SHIP_SPARK_DAMAGE_WEAPON` | ?: unread | ?: names located (`ShipCollisionFx_Trigger` `kind 1`), the event that raises it unread |
| Disruptor | A | ?: effect blocks authored, four have no parser branch | A | A | A |

**HD's blends are encoded (2026-10-22, `hd-blend-space`).** Every HD weapon visual drawn into
the race scene (models, psys, beams, flashes, the weapon quads) now combines with the scene in
display values, as the console's shader packer makes it: the blend law is **M**
([hd-track-tone.md](../rendering/hd-track-tone.md), "HD's blends combine encoded values"),
the sources are not re-measured. The Bomb's stacked rings now burn through to white at 1.8 s
where they drew yellow (the frame test was re-derived on the new build). No original frame of
the blast exists: an RPCS3 capture of the rings at about 1.8 s is the falsifier. The fireball
program reads its texels raw as light, as `holo.wesl` did before its units were read live,
so its source may be too strong.

### Sound

Cue names are Pulse's, looked up in whichever bank the title ships
(`weapons.bnk` on HD and 2048). HD also has its own authored mix
([audio-mix.md](../ghidra/functions/ps3-hdfury-eu/audio-mix.md)). The weapon cue **sites** are Pulse's reading
applied to the other titles: confirmed on Pure for eight of nine race cues
(78 to 82, by decompilation only, `psp-pure-usa/rocket-and-collision-fx.md`,
`shield-sound.md`, `lockon-sound.md`), not located on HD, 2048 or Omega. Which
individual weapon cue a title plays, and whether it plays it at the same
simulation edge, has not been compared weapon by weapon.

| Weapon | Pulse | Pure | HD / Fury | 2048 | Omega |
| --- | --- | --- | --- | --- | --- |
| Fire, travel, hit cues | M: wired for Rocket, Missile, Quake, Cannon, Plasma, Shuriken, LeachBeam and the pickups, [cue.rs](../../crates/sound/src/sfx/cue.rs); `MINERADAR` unwired; a weapon hit also plays `.COLLISIONS` once per spark locator behind the locator's 0.8 s gate (two on a Quake hit), measured 2026-10-26 and wired the same day (`Race::throw_hit_spark`) | I:Pulse; sites confirmed for the cues listed above; the weapon-hit `.COLLISIONS` is **not wired**: it rides the hit spark's locators and gate, and Pure's `Ship_Damage` weapon branch is unread (its executable names the cue) | I:Pulse; sites not located; the weapon-hit `.COLLISIONS` is **not wired**: the executable names the cue, but the call site of HD's Cannon craft-hit path is unread | I:Pulse; sites not located; the weapon-hit `.COLLISIONS` is **not wired** (the executable names the cue twice, no call site read) | ?: not investigated |

## 2048 and Omega: the cross-check

CLAUDE.md asks that a 2048 finding be checked against Omega and the reverse.
For weapons, as of 2026-10-19:

- **Tables**: checked, applies, not wired. Omega's HD-named tables equal HD's
  field for field apart from `superphantomspeed`; its `_2048` pair equals 2048's
  own. Racing on Omega is incomplete, so the only title that reads a `_2048`
  table in a race is 2048.
- **LeachBeam**: 2048's `repair` is 30.0 against HD's and Omega's 2.0. The
  per-trip law is HD's, carried to both by inheritance, so the 30.0 meets a law
  nobody has read on 2048.
- **Models (2026-10-26, `omega-from-hd-wire`)**: ported to Omega (above), **checked,
  applies, not wired** on 2048. 2048's `data.psarc` ships the same HD-named body
  set, but only under `Data/Weapons2048/`, and the bytes are its own: of 22 names
  present in Omega's `Data/weapons`, Omega's `Data/weapons2048` and 2048's
  `Data/Weapons2048`, none is byte-equal between any two of the three sets (the
  rocket's `.rcsmodel` is 12,265 bytes in Omega's HD set, 5,033 in its `Weapons2048`
  and 4,793 in 2048's), so 2048 cannot borrow Omega's table and needs its own paths,
  read off its own executable. Both are the Vita/PS4 container, which
  `oag-raceplay`'s weapon loader now builds (`mesh::rcs::psp2`). Omega's textures:
  `leachbeam_triangle.rcsmodel` names `hd_leechbeam.gnf` where HD's names
  `hd_leechbeam_glow` (**checked, differs**; 2048 ships the same
  `hd_leechbeam.gxt`).
- **Effects**: unread on both. Omega's executable carries the names
  `WO_SHIP_SPARK_DAMAGE_WEAPON` (`ShipCollisionFx_Trigger`, `0x01723ae0`, `kind 1`),
  `WO_DAMAGE_MILD`/`_MODERATE`/`_CRITICAL` (`FUN_016f3ef0`), `WO_WEAPON_ABSORB`
  (`FUN_01722810`, an object built per ship by `FUN_01307560`),
  `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` (`FUN_016f0f20`) and `WO_BOMB_RAYS`
  (`FUN_013594d0`, a 107 KB decompile, not named); what raises each is unread, so
  none of HD's seven effect triggers is carried over.

## Keeping it honest

A lane that changes a weapon's law, tuning, AI use, visuals or sound updates
the matching table above **in the same change**, and `just
check-weapon-lineage` has to pass. The rule is in
`.claude/skills/oag-drive/member-rules.md`.

What gets recorded and how:

- **Measured something that was inherited**: change the cell to **M**, link the
  page and, if the engine gained a per-title field, add its row to "Checked
  axes".
- **Added a `Weapons` or `WeaponModels` field**: the script fails until its row
  exists. Give each title's value and provenance.
- **Found a title lacks the weapon**: **A**, with the evidence, not `?`.
- **Never raise a confidence score here.** The score lives on the evidence page;
  a cell that cites one cites it as the page states it.
