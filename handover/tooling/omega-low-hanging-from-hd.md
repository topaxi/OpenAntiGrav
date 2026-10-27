---
status: ready-to-wire
titles: [omega]
updated: 2026-10-26
summary: "Omega adverts: reader landed, pass off until PS4 material curves are read; plume flags, hit-effect triggers, Looks, shadows still open"
difficulty: hard (35)
---

# Omega lacks HD's adverts, plume flags and hit effects: what is left of the wiring list

Scout lane `omega-from-hd-scout` ranked the list on 2026-10-26; wire lane
`omega-from-hd-wire` landed its first two items the same day. What it found and
decided is in `docs/formats/omega-status.md` ("Weapon bodies") and
`docs/gameplay/weapon-lineage.md`; this thread holds only what is still open.

Cost tiers: **A** one `Title` field or table reference, no new code. **B**
small wiring, but the provenance or the render needs a check first. **C** not
cheap: it needs reverse-engineering or a capture first.

Provenance rule for every entry: inherited from Wipeout HD unless Omega's own
data proves it.

## Open

### Landed by the wire lane (nothing left to do)

- **Projectile bodies:** the rocket, mine, bomb, the cannon's muzzle flash, the
  plasma blast trio and the missile blast load from Omega's `data00` through the
  Vita/PS4 container branch of the weapon loader. The rocket root is decided by
  the mode (`DAT_01f999e4 < 0x17`), not the circuit, so HD's root serves every
  race the port launches.
- **`magstrip_wake`:** both textures are executable literals, `.gnf`; the wake
  draws on contact. `rocket_trail` stays unwired, as HD's own comment says.
- **Doc drift:** the weapon-lineage cells, the stale comment in
  `crates/hd/src/race.rs`, `ai-tables.md`'s Omega `original_opponent` and the
  "no PS4 emulator" sentences in `omega-status.md` are corrected.

### Held back inside the weapon table (tier C, needs the PS4 shading read)

1. **Plasma bolt head, bomb blast set and halo, LeachBeam ball
   (`plasma_ball`, `bomb_blast_hd`, `leachbeam_ball` all `None`).** The bodies
   load and build; they draw wrong because HD's `RIM_EDGE`, `RIM_GLOW` and
   `BOMB_FIRE` bits come from fingerprinting RSX fragment programs
   (`oag_mesh::mesh::rcs::rim_glow`) and the PS4 builder sets none. The cheap
   route is a name-keyed inheritance table in the style of
   `psp2::lineage_blend`; the honest one is reading the PS4 shader programs.
   Flip a field only with a still pair, Omega against HD, that matches.
2. **`leach_strip`.** Omega's `leachbeam_triangle.rcsmodel` names `hd_leechbeam`
   (128x32) and `hd_waketrail_clouds`, where HD's names `hd_leechbeam_glow`
   (256x256), a file no Omega archive holds. Read the PS4 `HD_Leachbeam`
   material's samplers before wiring. 2048 ships the same `hd_leechbeam.gxt`.
3. **2048's own bodies.** 2048 ships the same body names, only under
   `Data/Weapons2048/` and with its own bytes, so it cannot borrow Omega's table.
   Its paths come off 2048's executable. Checked, applies, not wired.

### Worth a lane now (tier B)

4. **Adverts pass is off on Omega: the reader landed, the look did not.**
   `gantry::build` reads Omega's `321Go_*.vex` through `mesh::rcs::psp2::build_with_vex`
   (landed 2026-10-26, `start_gantry_omega_ground_truth.rs`). `adverts: None` stays
   because the Vineta K start-grid pair against shadPS4 does not match: placement does,
   the look does not. Open: (a) read the PS4 materials' texture curves (the countdown
   digits; the GO-edge finder reports no animated texture), tier C, `crates/mesh` and
   `oag_rcs::rcsmodel::psp2`; (b) add `.gnf` spellings of `billboard1-8` and `fx350_nomip`
   to `oag_render::gantry` (`SLOT_PLACEHOLDER_TEXTURES`, `mount()`, `FX350_TEXTURE`), which
   also strips Omega's `?` icons, so only together with the pass; (c) set `adverts` to HD's
   values (inherited) and repeat the pair. Details: `docs/formats/omega-status.md`, "Adverts".
5. **Engine light and front-face flags on the plume.** `crates/omega/src/lib.rs`
   sets `engine_light: false` and `front_faces_only: false` ("Off, chosen, not
   measured, as on 2048"); HD sets both `true`. Omega reads `EngineLightData.xml`
   in `FUN_0130e090` (`Distance` to `ship+0xbc`, `Radius` to `ship+0xc0`); the
   consumer of those two fields is unread. Needs it read, then a thrust frame
   pair, Omega against HD, on the same circuit. With the light on a grid still
   gains a warm cast at the nozzle; with `front_faces_only` on nothing changed at
   the two ticks checked, because the flame is not visible there.
6. **HD's hit sparks, damage smoke and bomb rays are not triggered on Omega.**
   The names are located, the triggers are not (no wiring on a guess):
   `WO_SHIP_SPARK_DAMAGE_WEAPON` in `ShipCollisionFx_Trigger` (`0x01723ae0`,
   `kind 1`), `WO_DAMAGE_MILD`, `_MODERATE`, `_CRITICAL` in `FUN_016f3ef0`,
   `WO_WEAPON_ABSORB` in `FUN_01722810` (an object built per ship by
   `FUN_01307560`), `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` in `FUN_016f0f20`,
   `WO_BOMB_RAYS` in `FUN_013594d0` (a 107 KB decompile). Read what raises each,
   then each is one `.with(...)` in `crates/omega/src/lib.rs` `EFFECTS`. The `.pob`
   files are in `data00` and the patch's `data05`.
7. **Opponents fly the player's variant on HD, not on Omega.**
   `crates/omega/src/race.rs` `opponents_match_player_variant: false` is a
   choice, because Omega's `Ship_Construct` was not read. Read it, then flip if
   it matches.
8. **Render laws HD measured on RPCS3 are off on Omega.** `crates/omega/src/lib.rs`
   `looks: Looks::unread(...)`; HD's `crates/hd/src/effects.rs` sets
   `absorb_shell` (measured), `material_render_state`, `blends_encoded`,
   `lit_output_halved` and `skips_language_picker`. Risky: they were read off RSX
   frames and Omega's shaders are recompiled. Try `absorb_shell` and
   `material_render_state` against a hit-hull still pair; leave `lit_output_halved`.
   The rocket and missile bodies on Omega draw darker than HD's (no lit-output or
   weapon-light law), which this item would also move.
9. **Shadow tier is `Unbuilt` on Omega, though HD's mechanism is built.**
   `crates/title/src/shadow.rs` calls Omega's state an "untried carry-over from
   HD". Census the shadow hulls (`docs/rendering/shadows.md`) first, then a
   shadPS4 still with `original` shadows.

### Already covered by a thread, not re-reported

- **Zone speed ladder and handling classes:** `handover/gameplay/zone-handling-needs-a-speed-law-per-title-and-a-ladder-as-data.md`.
- **Trackside ambience is silent on Omega:** `handover/audio/2048-and-omega-circuits-play-no-trackside-ambience.md`.
- **Omega's end-race screens:** `docs/formats/hd-endrace-screens.md` ("Omega
  (checked, applies, not wired)"); `endrace_entry: None` in `crates/omega/src/frontend.rs`.
- **Omega's campaign selection overlay:** `DATA06` is HD's alone; `false` is right.
- **Omega's pt-BR disc text:** Omega ships its own `portuguesebr`; `disc_strings: None` is right.

### Tier C: not cheap, listed not ranked

- **Grid hover and hover rig.** `launch_hover`, `hover_rig`, `zone_hover_rig` are
  `None` on Omega; HD's value is read from its craft update and Omega's is not.
- **Zone mode.** `zone_palette`, `zone_stages`, `zone_transition`, `zoom_ring`,
  `zone_stage_textures`, `zone_sky`, `zone_announcer`, `zone_handling` are `None`;
  Zone is unmeasured on Omega.
- **Pre-race flyby.** HD's is `Platforms::Only(&[Platform::Ps3])`; about 22
  `start_grid.vex` entries exist in Omega's archives; its timing is unread.
- **Prompt glyphs** (`Prompts::UNREAD`), **loading screen** (`loading: None`) and
  **fresh profile variant** (`None`, a product choice).

## Next Steps

1. Item 4: read the PS4 material curves, add the `.gnf` spellings, then repeat the
   Vineta K pair and set `adverts`.
2. Item 6: read the call sites of the six functions above, in the order
   `ShipCollisionFx_Trigger`, `FUN_016f3ef0`, `FUN_01722810`.
3. Item 5: read the consumer of `ship+0xbc`/`ship+0xc0`, then the thrust frame pair.
4. Item 1: decide between the name-keyed table and reading the PS4 programs.
5. Items 7, 8 and 9 each need their read or census first.

Commands: `just play [--screenshot out.png] [--until "text"] [--press ...]` for
headless stills (the options in `CLAUDE.md`). For the shadPS4 pair, see the
header of `scripts/shadps4-omega-pair.py`. Run both under the member rules: own
profile dir via `XDG_CONFIG_HOME`, no audio, own Xvfb or headless.
