---
status: ready-to-wire
titles: [all]
updated: 2026-11-17
summary: "HD taps, setting, enhanced and the planar-reflection read landed; next wire the planar pass, Chenghou/Modesto seas, 2048 and Omega laws"
categories: [rendering, frontend]
size: XL (60)
difficulty: medium (55)
---

# A per-title `Water` setting: what each choice draws, and the lanes that build it

2026-11-17, lane `water-re` (measure). The recovered laws, the census and the
confidence scores are in [water.md](../../docs/rendering/water.md) and
[caustic-pass.md](../../docs/ghidra/functions/psp-pulse-usa/caustic-pass.md).
This thread is the plan. Steps 1 (HD's taps, without the planar term), 2 (the setting) and 10 (own-design `enhanced` on Pulse and Pure, with the row hidden where nothing changes) have landed.

## The maintainer's request

Each title gets a `Water` setting with `off`, `original`, `HD`, `2048`, `Omega`,
default `original`. A choice that needs original assets is offered only when
that disc image is present. An own-design water for Pure and Pulse is optional
and TBD.

## What each choice draws

| Choice | Law | Assets it needs | Offered when |
| --- | --- | --- | --- |
| `off` | the surface's constant colour or texture, lit, no wave normal, no scroll, no reflection, no caustic pass (what is drawn today on every title) | none | always |
| `original` | the title's own law (water.md): Pulse textures plus Vostok Reef's caustic pass; Pure textures; HD taps plus planar or sky reflection; 2048 one scrolled tap; Omega two taps plus Fresnel planar reflection | the title's own disc | always |
| `HD` | HD's sea law: two `dc_waternormalmap` taps at `u + 0.05 t` and `1.15 (-u + 0.05 t)`, sun diffuse and specular, reflection | HD's `dc_waternormalmap.gtf` (and `waves2.gtf` for Vineta K) | the HD image is present, and the title is not HD |
| `2048` | 2048's `fc15` law: one `water_lapping_waves2_n` tap at `u + 0.05 t`, sun diffuse and specular, no reflection | 2048's `water_lapping_waves2_n.gxt` | the 2048 package is present, and the title is not 2048 |
| `Omega` | Omega's `reflectplane_mid` law: `NormalMap` and `NormalMap1`, `Speed`, Fresnel-weighted planar reflection | Omega's `dc_waternormalmap.gnf`, `Water_lapping_waves2_N.gnf` | the Omega package is present, the title is not Omega, **and lane 6 has read the law** |

A choice equal to the title's own `original` is dropped from that title's row,
not shown twice. All three later-title choices borrow a normal map, so none is
"needs no assets"; the own-design option below is the one that would be offered
with no other disc.

### Which surfaces are water on a title that borrows a law

Never by guess. Per title, a table the title package owns and the loader
injects:

- **HD:** by program facts, as `mesh::rcs::water::is_water` does today. The sea
  is "declares `screenSpaceReflectionTex` (`0xec2b3fc2`) and `DiffuseTexture1`
  (`0x06a77e50`), reads both material samplers as normals". Amphiseum's `water`
  is "declares `paraboloidReflectionTex`, `NormalTexture2` (`0x62ae87a7`) and
  `ColourConstant`".
- **2048:** by the program's declared interface (`oag_rcs::gxp::sampler_use`
  already separates the eight lighting-only families): `NormalMap` plus `time`
  plus `ColourConstant` or `Watercolour`.
- **Omega:** by the GCN program's declared names (`oag_rcs::gcn`):
  `screenSpaceReflectionTex` with a `NormalMap*` or `DiffuseTexture1` sampler,
  or 2048's rule on the Zone models.
- **Pulse:** the caustic bit (`Batch::header_flags & 0x04`) for the caustic
  pass; for a borrowed law, the batches whose material binds one of the
  embedded textures the census names (`NewSeaTest`, `sea`, `water`), as a
  per-circuit list in `oag-pulse`. **Lane 8 must confirm each name on a frame
  first** (a census by name says the texture ships, not that it is the sea
  sheet).
- **Pure:** the same, in `oag-pure`: `AAdc_Water`, `AAdc_WaterDark` (Vineta K),
  `Water` (Blue Ridge).

### Where the borrowed law's numbers come from

HD authors `ColourConstant`, `Reflectivity`, `SpecularColour` and
`SpecularPower` per circuit. For a circuit the lender also ships (Moa Therma
between Pulse and HD, Vineta K between Pure and HD, for example; the pairing
table is lane 8's to write), read the lender's own instance for that circuit
off its disc. For a circuit it does not ship, the
values are **chosen, not measured**: the lender's Moa Therma instance, with the
colour replaced by the borrower's own water texture's mean. Say which in the
loader report.

### Reflection on a title with no reflect set

Pulse and Pure author no reflect set and no sky paraboloid. Under `HD` or
`Omega` they reflect the title's own sky cube only, resampled to the dual
paraboloid `mesh::sky_paraboloid` builds (chosen, not measured; Amphiseum's
`water` is the HD precedent for a sea that reflects only the sky). A planar
pass over Pulse geometry is the own-design section's question, not this one's.

## Plumbing

- **Enum:** `oag_display::display::Water { Off, Original, Hd, Vita2048, Omega }`
  in its own file beside `shadows.rs`, serde names `off`, `original`, `hd`,
  `2048`, `omega`.
- **Setting:** `graphics.water` in the per-title table of
  `crates/game/src/settings/render_profile.rs` (where `shadows` lives, for the
  same reason: what a value means depends on the title). Default `original` on
  every title.
- **Menu:** one `[[page.entry]]` `kind = "choice"` in `assets/ui/menu.toml`,
  `string_id = "OAG_GRAPHICS_WATER"`, values `["off", "original", "hd", "2048",
  "omega"]`, with the English label and value texts in
  `assets/ui/strings/english.toml` in the same change (`just check-strings`).
  Other languages name the gap in `[untranslated]`.
- **Availability:** `oag_ui::menu::Definition::drop_unavailable_water(title,
  present)`, modelled on `drop_unavailable_rumble_original`: drops the value
  equal to the title's own, and each lender value whose image the `oag-source`
  registry does not find. A stored value that is no longer offered falls back
  to `original` at load, with a `warn`.
- **Render side:** the law is generic and lives in `oag-mesh`: a `WaterLaw`
  record per material slot (taps: sampler, scale, rate; colour; specular;
  reflection kind `None | Sky | Planar`; weights) in the glow-table style
  `slots::ICE` uses, and one `shade.wesl` branch. `oag-mesh` names no title:
  `oag-raceplay`'s load builds the records from the title package's table and,
  for a borrowed law, from the lender's archive through `oag-source` (rule 3).
  The planar target is a pass in `oag_raceplay`'s `scene`, beside
  `behind_glass`, sharing its half-size target code. The caustic pass belongs
  to the `.vex` draw path with the 16 frames loaded by `oag-raceplay`.
- **Clock:** the taps scroll on the race clock the existing
  `AnimTrack::Scroll` uses, so a replay shows the same water.

## Own-design water for Pulse and Pure (decided 2026-11-17; chosen, not measured; LANDED, `water-enhanced`)

**Decided (maintainer, 2026-11-17): keep the setting, hide the row when it
would change nothing, and build `enhanced` with a procedurally generated normal
map.**

- **`enhanced`**: a sixth value, offered on Pulse and Pure with no other disc
  present. HD's sea law (two opposite-scrolled taps, lighting, specular; sky
  reflection only, as Amphiseum) drawn by our shader over **a normal map this
  project generates procedurally** (tiling, two channels as HD's
  `dc_waternormalmap` layout), with parameters of our own (colour from each
  circuit's own water texture's mean, reflectivity and specular power tuned for
  a good picture), all labelled chosen, not measured. No HD disc involved: the
  shader is ours and the borrowed `hd` choice stays the one that reads HD's own
  normal map and values off its disc.
- **The row is hidden** on a title when nothing it offers changes the picture:
  shown only if `original` differs from `off` on that title (HD once the sea
  law lands, Pulse PS2 once the caustic pass lands), or `enhanced` is offered
  there and drawn, or a lender's law is drawn and its disc is present.
- **Landed** (docs/rendering/water.md, "Enhanced"): `Water::Enhanced`, `oag_title::water` per
  title (`original_differs`, the circuit and texture lists in `oag_pulse::race` and
  `oag_pure::race`), `mesh::vex_water` (tags the batches), `mesh::water_normal` (the generated
  map, pinned), `sky_paraboloid::from_mesh` (the dome cast to the reflection map), `water.wesl`
  family 5, `water_setting::row_changes_picture`. The row is hidden on 2048 and Omega.
- Still open for later: whether Vostok Reef's caustics extend to other
  underwater sections, one look or one per circuit, the sun being fixed in the world (a glint
  shows only on the sun's side), and an `enhanced` for HD, 2048 and Omega (their `original`
  draws or will draw its own; `Title::water.enhanced` empty keeps it unoffered there).

## Open

- The planar reflection is read on HD Moa Therma (water.md, "The planar reflection, read off
  live frames"); unread: Chenghou's and Modesto's seas (recipe there), bit 1 as the mirror, and
  Omega's and 2048's copies of the same passes.
- Omega's `Reflectivity*` combine and `Speed` law are unread; 2048's `fc17`
  edge program is unread.
- Pure's caustic frames on Vineta K are unread; the caustic bit is unread on
  Pure's version-4 batches.
- Pulse's caustic pass is read on the PSP code only and reached only by PS2
  disc data or the PSP download pack.
- Blue Ridge's embedded `Water` texture is bound by no batch; unread.

## Next Steps

Ranked. Hours are for one member on an idle machine.

1. **LANDED (`water-hd`): HD sea without the planar term.** `mesh::rcs::water` classifies
   the sea, Amphiseum's pool and Vineta K's two seas by program facts and, under a law
   that draws the taps (`Reading::Taps`: `original` on HD, and `hd`), lays a
   `PROGRAM_WATER` block (colour, reflection weight, specular, one entry per `Tap`), binds
   the normal map third and `water_noref`'s second map fourth, and sets the chunk's frame
   to `TangentFrame::Raw`; `shade.wesl` and `water.wesl` draw it. `off` keeps the old
   reading to the vertex. Pinned: chunk counts per direction in `hd_water_ground_truth`
   (Moa Therma 31/31, Ubermall 13/7, Chenghou 3/3, Modesto 5/5, Amphiseum 2/2, Vineta K
   2+3 both ways), `mesh::rcs::water::tests` (`0.05`, `1.15`, the opposite `u`). Frames and
   what a player sees: water.md, "What this engine draws today". **Still open from it:**
   - **The RPCS3 pair at a sea pose.** No HD compare pair frames water and the October
     Moa Therma state no longer loads. Rebuild it (`scripts/emu-hd-timetrial-state.sh
     <lane> ...` for Moa Therma, recipes in emulator-recipes.md), drive to a sea view
     (camera `210,40,-380` over the `-z` sea is ours), `rpcs3-hd-pair.py`, then
     `rerender` the pair with `graphics.water` `off` and `original` and quote a sea crop.
   - **`is_drawn(Water::Hd)` is still false**, deliberately: `hd` is never offered on HD
     (it is HD's `original`), and on Pulse, Pure, 2048 and Omega nothing draws the HD law
     until lane 8, so flipping it would offer a choice that changes nothing. Lane 8 flips
     it with the borrower's wiring.
   - **Under `off`, the non-lightmapped seas add `DiffuseTexture1` as a glow** (a normal
     map; `CIRCUIT_SURFACE_MAP_SAMPLERS` refuses only `DiffuseTexture`). Kept, since `off`
     must stay today's look; the taps replace it.
   - `water_noref`'s own glint (`sat(a * 0x697f57e3)`, power `0x2a2dc10b`): the generic
     specular stands in.
   - 2048 and Omega: checked, applies, not wired (water.md, "Lineage"): their seas draw
     through `psp2`, which has no tap reader, and the literals are unread in GXP and GCN.
2. **LANDED: the setting and `off`.** `oag_display::display::Water`;
   `graphics.water` in `RenderProfile`; the WATER row in `menu.toml`
   (`OAG_GRAPHICS_WATER`); `oag_game::water_setting` (`offered`, `resolve`,
   `trim_row`, `for_source`) and `Definition::drop_unavailable_water(&dyn Fn)`
   (the lender's disc is asked through `menu_tree::disc_probe`, the survey the
   campaign check shares; `oag-ui` names no title). `race::Options::water` is the
   resolved law; `mesh::rcs::build_scene_with` / `build_scene_views` /
   `scene_from` carry it to `water::water`, whose `reading(law)` is the hook:
   **lane 1 adds a `Reading` variant per law there**; every value maps to
   `Reading::Constant` today. `Report::water_law` names the law in the loader
   line. Open: the 2048 and Omega `psp2` water path and the front-end capture
   route (`capture.rs`) read no setting yet; 
   **Each choice is hidden until its law is drawn:** `mesh::rcs::water::is_drawn(law)`
   is the one flag; `water_setting::offered` also requires it, so today the row
   holds `off` and `original` only; on HD `original` draws the taps. Lane 8 flips
   `Hd` (lane 1 drew the law on HD only), lane 5 `Vita2048`, lane 6 `Omega` (and adds the
   `Reading` variant) when its law is drawn; the menu test
   `with_every_lender_present_only_drawn_laws_are_offered` then grows its list.
   A saved `hd`/`2048`/`omega` resolves to `original` until then.
   Original wording: (`oag-wire`, sonnet, 3 to 4 h). Enum, per-title
   field, menu row, strings, `drop_unavailable_water`, `off` routing every
   classified water slot to today's constant reading. Acceptance: settings
   round-trip and fallback tests, a menu test with and without each lender
   image (and with `OAG_MODS` pinned empty), `just check-strings`, mouse
   support as every row has. Can run beside lane 1; touches different crates.
3. **LANDED (`water-hdcap`): the HD reflection target read.** One Moa Therma sea frame,
   three frames over two restores: a 960 x 540 R5G6B5 + Z16 target at `0x02a20000`
   cleared opaque black / depth 1, drawn before the main view with the sky and the scenery
   under the main view-projection mirrored by `y' = K - y`, `K = -50` (plane -25; Moa
   Therma authors `Water plane height` -50), front-face culling; the sea reads it at its own
   screen position through `reflectProject` = the main view-projection, normal pushed 40
   world units (`distortion.z` = 40 = `Water distortion`); only the sea material samples it.
   **Left open:** Chenghou's and Modesto's seas (the recipe in water.md names a visibility-set
   scan, then `scripts/hd-sea-place-dump.py` and `hd-sea-reflect-read.py`), and bit 1.
   The RPCS3 sea pair the HD taps lane left open is made (sea-crop luma: original 140, `off`
   139, `original` 105; `original` reads 35 levels darker than the original and worse than
   `off` by MAE until lane 4 lands).
4. **The planar reflection** (`oag-wire`, opus, 8 h; **ready to wire**, lane 3 read the camera, target and constants). A mirrored
   pass over render-flag bit-3 chunks (HD) and `wohdmeshreflect` meshes (2048,
   Omega, `psp2::refraction_only`'s split generalised), sampled through
   `reflectProject` with the `distortion` offset. Acceptance: target set
   equals the flagged set per circuit (HD 257 and 73; Omega 268, 73, 30, 38,
   45, 24; 2048 353 and 52), no target on a circuit with none, frame cost
   quoted at 800p, 1080p and 4K.
5. **2048's own water** (`oag-wire`, opus, 5 h). `fc15`/`fc17` one-tap law;
   read the `fc17` edge program first (`scripts/vita-usse.py disasm`).
   Acceptance: per-circuit counts pinned in `omega_transparency_ground_truth`'s
   walk, a Vita3K frame pair on Altima at two times.
6. **Omega's own water** (`oag-re` then `oag-wire`, opus, 8 h). A GCN
   instruction read of `reflectplane_mid` (`Reflectivity*`, `Speed`), then
   wire it on Omega's 2048 circuits and as the `Omega` choice. Checked against
   2048 in the same change.
7. **Vostok Reef's caustics** (`oag-capture`, sonnet, 2 h, then `oag-wire`,
   sonnet, 4 h). A PCSX2 frame of the underwater floor with and without the
   pass, batch state `0x202`'s blend, then the pass. PS2 inherits the PSP law
   where the frame does not contradict it.
8. **Borrowed laws on Pulse and Pure** (`oag-wire`, opus, 8 h; after 1, 2).
   Confirm each water texture name on a frame, add the per-circuit tables to
   `oag-pulse` and `oag-pure`, borrow the lender's normal map and per-circuit
   numbers through `oag-source`, sky-only reflection. Acceptance: with no
   lender image the row offers `off` and `original` only; with HD present,
   Pulse's Moa Therma under `HD` shows the scrolled taps on the sea batches
   and nothing else changes (capture diff against `original`).
9. **Pure's caustic frames** (`oag-re`, opus, 3 h).
10. **LANDED (`water-enhanced`): own-design water** (see above).
