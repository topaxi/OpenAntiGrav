# Omega lacks HD's projectile bodies, hit sparks and adverts: ranked wiring list

Scout lane `omega-from-hd-scout`, 2026-10-26. Question: which HD-wired behaviour
or asset could Omega take cheaply. Read-only; no code changed.

Cost tiers: **A** one `Title` field or table reference, no new code. **B**
small wiring, but the provenance or the render needs a check first. **C** not
cheap: it needs reverse-engineering or a capture first.

Provenance rule for every entry: inherited from Wipeout HD unless Omega's own
data proves it. **Census** means the entry names of an Omega archive were read
directly from the PSARC name table: `data00`, `data02`, `data03`, `data04` and
the patch's `data05` and `data09`. `data01`, `data07` and `data08` were not read.

## Open

### Worth a lane now (tier A or B, ranked by player impact)

1. **Projectile bodies: the Omega table is empty (tier A, bodies only).**
   - Where: `crates/omega/src/lib.rs:149-155` sets `weapon_models` to
     `..WeaponModels::EMPTY` plus `ghost_static`. HD fills the bodies at
     `crates/hd/src/race.rs:640-712`: rocket, mine, bomb, plasma ball, the plasma
     blast trio, the bomb blast set, the missile blast and the LeachBeam ball.
   - What happens: `crates/raceplay/src/load/weapon_models.rs:181-189`
     (`load_optional`) returns `None` for an absent entry. `crates/raceplay/src/options.rs:908-911`
     says a round with no model "draws as nothing at all". So on Omega a rocket,
     a mine, a bomb, a plasma bolt and a missile blast draw nothing in any race
     with a weapon. The exception is the cannon round, which falls back to
     Pulse's bolt quads (`weapon_models.rs:268-274`), so its body is missing but
     not the round.
   - Stale comment: `crates/hd/src/race.rs:684` says Omega's table "is still
     `EMPTY` (it does not race)". Omega races from its own data since 2026-09-29
     (`crates/omega/src/race.rs`, module doc).
   - Census: the body files exist in `data00`: `Data/weapons/HD_rocket.vex`,
     `HD_mine.vex`, `HD_bomb.vex`, `HD_plasma_ball.vex`, `HD_plasma_sphere.vex`,
     `HD_missile_explosion.vex`, `hd_muzzleflash.vex`, `HD_leachbeam_ball.vex`,
     `HD_bomb_sphere.vex`, and the rest of HD's names.
   - Rocket path, with its limit: `docs/ghidra/functions/ps4-omega-eu/weapons.md:107-115`
     records that `Rocket_Construct` picks `Data/Weapons` or `Data/Weapons2048`
     by `DAT_01f999e4 < 0x17`. The page does not say which branch an HD-lineage
     race takes; that mapping is this lane's inference from the lineage byte
     (`crates/omega/src/race.rs`, `ORIGINAL_DRIVER` comment). `WeaponModels`
     holds one path per weapon, with no per-circuit field, so a `environments2048`
     circuit would draw HD's rocket where the original draws 2048's. That is a
     choice to make, not a finding.
   - Expected win (estimate): every projectile visible on Omega.
   - Verify: the shadPS4 pair (`scripts/shadps4-omega-pair.py`) on a start grid
     plus a headless `oag-game` race with a fire input; the load report names
     each body as loaded, none as missing. Template for the HD side:
     `crates/raceplay/tests` weapon-scene tests.
   - Risk: render only; the simulation reads no model.

2. **Texture fields on the same struct (tier B, per-field census).**
   - `magstrip_wake` (`crates/hd/src/race.rs:686-690`): the atlas is at
     `Data/particles/Tex/HD_electric_arc_8x8.gnf` on Omega, not HD's `Data\Tex\`
     path. Contact texture not checked.
   - `leach_strip` (`crates/hd/src/race.rs:703-706`): noise texture is
     `Data/art/published/ships/weapons/textures/HD_waketrail_clouds.gnf` in the census.
   - `rocket_trail` (`crates/hd/src/race.rs:696-700`): HD's own comment says
     Omega ships the textures but no opacity ramp, so it stays unwired. Leave it.
   - `cannon_look` (`crates/hd/src/race.rs:709`): HD names `.gtf`; Omega's
     spelling is not read.

3. **HD's hit sparks, damage smoke and bomb rays are not triggered on Omega
   (tier B, one decision first).**
   - Where: HD adds them at `crates/hd/src/effects.rs:37-70`: `WeaponSpark`,
     `DamageMild`, `DamageModerate`, `DamageCritical`, `LeachHitSpark`,
     `ShieldAbsorb`, `BombRays`. Omega's `EFFECTS` at `crates/omega/src/lib.rs:208-217`
     only removes Pulse's triggers. Its comment at `lib.rs:157-162` says the
     spark and absorb triggers are left off because their wiring is unread.
   - Census: the `.pob` files exist: `Data/particles/WO_DAMAGE_MILD.pob`,
     `WO_DAMAGE_MODERATE.pob`, `WO_DAMAGE_CRITICAL.pob`,
     `WO_SHIP_SPARK_DAMAGE_WEAPON.pob`, `WO_SHIP_SPARK_DAMAGE_LEACHBEAM.pob`,
     `WO_WEAPON_ABSORB.pob`, `WO_BOMB_RAYS.pob` (`data00` and patch `data05`).
   - Why it is B, not A: `CLAUDE.md` says an effect with no recovered trigger
     stays unwired. The cheap pre-check is a cross-reference search for
     `WO_DAMAGE_MILD` and `WO_SHIP_SPARK_DAMAGE_WEAPON` in Omega's `eboot.bin`
     (headless Ghidra, `docs/reverse-engineering/toolchain.md`). If the xrefs
     show the call sites, this becomes a tier A `.with(...)` per effect.
     Otherwise it is a lead's decision.
   - Expected win (estimate): a struck craft sparks and smokes as on HD.
   - Verify: a still of a damaged craft, Omega against HD, the same state.

4. **Adverts pass is off on Omega (tier B).**
   - Where: `crates/omega/src/lib.rs:190` `adverts: None`. HD's block is at
     `crates/hd/src/lib.rs:217-239`: target 512x256, `near` 0.5, `far` 5500,
     `gantry_card`, `zone_gantry` at `/Data/Billboards/HD_Adverts/321Go/321Go_Zone.vex`.
   - Census: `Data/billboards/HD_Adverts/321Go/` is in `data00`, with
     `321Go_Zone.vex` present (case-insensitive match). `Billboard_ConstructResource`
     and the `Type` attribute are read on Omega
     (`docs/ghidra/functions/ps4-omega-eu/billboards.md`).
   - Provenance: HD's block is measured on RPCS3 (`lib.rs:217-223` comment).
     On Omega it stays inherited until a still confirms card placement.
   - Verify: the shadPS4 pair on the start grid, against HD at the same camera.

5. **Engine light and front-face flags on the plume (tier B, render-critical).**
   - Where: Omega `crates/omega/src/lib.rs:111-113` sets `engine_light: false`
     and `front_faces_only: false`, with the comment "Off, chosen, not
     measured, as on 2048". HD sets both `true` at `crates/hd/src/lib.rs:157-158`.
   - Why it matters: the maintainer's render priority puts the hull and plume
     first (`docs/` render notes, memory "Render fidelity priorities").
   - Provenance: HD measured (its comment is not on those two lines, so the
     measured claim is read from the struct's neighbours; confirm before flipping).
   - Verify: the shadPS4 pair on a grid still at the plume, Omega against HD.

6. **Opponents fly the player's variant on HD, not on Omega (tier B, RE check).**
   - Where: `crates/omega/src/race.rs:180` `opponents_match_player_variant: false`,
     and the comment at `race.rs:159-161` says it is a choice because Omega's
     `Ship_Construct` was not read. HD's `true` was read off HD's executable.
   - Verify first: read Omega's `Ship_Construct`. Then flip if it matches.

7. **Render laws HD measured on RPCS3 are off on Omega (tier B, risky).**
   - Where: `crates/omega/src/lib.rs:163-166` `looks: Looks::unread(...)`.
     HD's `crates/hd/src/effects.rs:110-150` sets `absorb_shell` (measured),
     `material_render_state`, `blends_encoded`, `lit_output_halved` and
     `skips_language_picker`.
   - Why it is risky: these were read off RSX frames. Omega is PS4 with
     recompiled shaders (`docs/formats/omega-status.md`, re-shipped note).
     `absorb_shell` and `material_render_state` are the two to try. Leave
     `lit_output_halved` alone.
   - Verify: the shadPS4 pair on a hit hull, against HD.

8. **Shadow tier is `Unbuilt` on Omega, though HD's mechanism is built (tier B).**
   - Where: `crates/omega/src/lib.rs:133` `OriginalShadows::Unbuilt`;
     `crates/hd/src/lib.rs:172` `Built`. The enum's doc (`crates/title/src/shadow.rs:9-24`)
     calls Omega's state an "untried carry-over from HD".
   - Unverified: whether Omega's occluder hulls and coverage data match HD's.
     Not censused in this lane.
   - Verify: a census of the shadow hulls (`docs/rendering/shadows.md`) first,
     then a shadPS4 still with `original` shadows.

### Already covered by a thread, not re-reported

- **Zone speed ladder and handling classes:** `handover/gameplay/zone-handling-needs-a-speed-law-per-title-and-a-ladder-as-data.md`.
  The `speed_classes` gap on Omega (`crates/omega/src/race.rs:125` `None`) is in that thread.
- **Trackside ambience is silent on Omega** (0 of 32 emitters on Tech De Ra):
  `handover/audio/2048-and-omega-circuits-play-no-trackside-ambience.md`.
- **Omega's end-race screens:** `docs/formats/hd-endrace-screens.md:324` ("Omega
  (checked, applies, not wired)"), with `crates/omega/src/frontend.rs:258-264`
  leaving `endrace_entry: None`. The dead end is `EndRace Menu` drawing no
  option blocks.
- **Omega's campaign selection overlay:** `crates/hd/src/psn.rs:76` says the
  ids `selection_strings` overlays are `DATA06`'s alone. Omega has no `DATA06`,
  so `false` is correct.
- **Omega's pt-BR disc text:** `docs/ui/project-languages.md:28-29` and `:47` say
  Omega ships its own `portuguesebr`, so `disc_strings: None` is correct.

### Tier C: not cheap, listed not ranked

- **Grid hover and hover rig.** `crates/omega/src/race.rs:98-100` sets
  `launch_hover`, `hover_rig` and `zone_hover_rig` to `None`; HD sets them at
  `crates/hd/src/race.rs:57-59`. The lead's `launch_hover` example is not a
  field flip. HD's value is read from its craft update (`crates/hd/src/race.rs:54-56`
  comment), and Omega's craft update is not read in any doc. The shadPS4 pair
  can measure the grid clamp, but the law needs the update read first.
  `docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md:136-138` and `:246-252`
  say Omega's spring and probe offsets are unread.
- **Zone mode.** `crates/omega/src/race.rs:94-120` sets `zone_palette`, `zone_stages`,
  `zone_transition`, `zoom_ring`, `zone_stage_textures`, `zone_sky`,
  `zone_announcer` and `zone_handling` to `None`. Zone is unmeasured on Omega.
- **Pre-race flyby.** HD's `crates/hd/src/pre_race.rs:15` is
  `on: Platforms::Only(&[Platform::Ps3])`. Census: about 22 `start_grid.vex` entries (case variants included)
  exist in Omega's archives. The flyby's timing is unread on Omega. The
  shadPS4 pair is the way to read it.
- **Prompt glyphs.** `crates/omega/src/lib.rs:123-124` is `Prompts::UNREAD`; HD
  is read (`crates/hd/src/lib.rs:165`).
- **Loading screen.** `crates/omega/src/lib.rs:129` `loading: None`; HD's
  `crates/hd/src/lib.rs:168` is read. No Omega row in `docs/formats/omega-status.md`.
- **Fresh profile variant.** `crates/omega/src/race.rs:124` `None`; HD's
  `crates/hd/src/race.rs:102` is `Chosen`. A product choice, not a finding.

## Doc drift found while checking (fix in the wire lane)

- `docs/gameplay/weapon-lineage.md:249` says Omega's rocket has "no model named";
  `:255` says the mine is "not drawn". Both are wrong against item 1.
- `crates/hd/src/race.rs:681-684`: "Omega's `WEAPON_MODELS` is still `EMPTY` (it
  does not race)" is out of date.
- `docs/formats/ai-tables.md:77` says Omega's `original_opponent` is `false`.
  `crates/omega/src/race.rs:111` is `true`.
- `docs/formats/omega-status.md:33`, `:197`, `:554` and `:1449` say no PS4 emulator
  exists in this toolchain. `scripts/shadps4-omega-pair.py` now drives shadPS4 for
  Omega, so those statements need an update.

## Next Steps

1. Wire item 1 (bodies) in one change: fill `crates/omega/src/lib.rs`
   `weapon_models` with the body files above. Cite the rocket row to
   `weapons.md:107-115` and label the lineage mapping as inference. Update
   `docs/gameplay/weapon-lineage.md` rows 249 and 255 in the same change, and run
   `just check-weapon-lineage`.
2. Item 3's xref pre-check (Omega's `eboot.bin`, headless Ghidra). It decides
   whether the sparks and smoke are a tier A wire or a lead's decision.
3. Items 4, 5 and 7 each need one shadPS4 still pair, Omega against HD, on the
   same circuit and state. Start with item 4 (census proof exists).
4. Item 6 needs `Ship_Construct` read in Ghidra before any flip.
5. Item 8 needs the shadow census first.

Commands: `just play [--screenshot out.png] [--until "text"] [--press ...]` for
headless stills (the options in `CLAUDE.md`). For the shadPS4 pair, see the
header of `scripts/shadps4-omega-pair.py`. Run both under the member rules: own
profile dir via `XDG_CONFIG_HOME`, no audio, own Xvfb or headless.
