---
status: re-first
titles: [2048, omega]
updated: 2026-11-11
summary: "pipe strip and Chenghou emmissivenew wired (2048 41 to 3, Omega 149 to 67); left: water edge, temp_diffuse, Omega jd_*_n/rocks/amb_glass"
categories: [rendering]
---

# Omega and 2048 still draw some normal maps as the picture

2026-10-19, `hd-finish-line-purple`. HD's purple start line (a normal and specular map
bound as the road colour, `docs/formats/rcsmaterial.md`, "A normal map is not the
picture") is fixed on HD by reading the resolved fragment program. Omega and 2048 draw
the same files through `psp2::build`, which binds `Material::diffuse_texture` (first
non-lightmap texture) and so still draw it.

## Open

- **Done 2026-10-19 (`normal-maps-as-colour`):** 2048 reads each material's own fragment
  programs (declared interface only, `oag_rcs::gxp::sampler_use`) and binds no picture from a
  sampler they read for lighting alone, drawing the constant colour instead: 66 -> 41 slots.
  Omega takes 2048's answer for the three families whose instance record is 2048's exactly
  (`oag_rcs::rcsmodel::psp2::lighting_only`): 223 -> 149. Evidence and the rule:
  `docs/formats/gxp.md`, "Which sampler carries the colour"; `docs/formats/rcsmaterial.md`.
  Ratchets per circuit in `omega_transparency_ground_truth.rs`.
- **Still drawn as colour, 2048 (41 slots) and Omega (149):** `m_diffuse_normal_constantspecular`
  (and `New`; 82 Omega materials, the biggest) samples its map in the ambient-only
  permutation too, so the programs do not call it lighting-only; reading what it does with
  it needs the USSE `SMP` result's use (or a Vita3K `.dsm` of a DLC circuit with that
  material, e.g. Anulpha Pass), and on Omega the GCN. Also: the water edge
  (`fc17_..._wateredge_vertexcol`), `rocks_01_normal_alpha`, `jd_*_n`, `amb_glass`, glass
  families on Omega (`glass_2uv_rgb_reflect_normal_glow`, `pb_window`, `j_rgb_colourtintnormalreflect`)
  whose Omega programs are unread, and `temp_diffuse` on 2048's Square and Subway.
- **The water now draws its constant colour, flat, on 2048 and Omega.** HD's water has its own
  pass (`water.rs`); here there is none, so a sea is one colour times light. Not compared
  against a Vita3K frame (no matched pose was captured); the start-line fix is judged
  against HD's RPCS3 frame only.
- **HD families left white times light** (constant parameter not traced):
  `m_diffuse_normal_constantspecular`, `ds_booth_glass`, `metallic`, `dc_constantglow`,
  `cf_constantcolourglow`, `dc_metallic`, `mesh_colour_constant`, `emissive`,
  `dc_glowlightband`. Modesto's `nr_holobowlparallax` picture changed with the rule and is
  unchecked against the original. (`ds_booth_glass`'s constant is `Constant1` on 2048 too,
  and now bound there; HD's is still white.)
- **Original comparison:** done for Talon's start line at the committed grid pose
  (dark thin lines in both, ours darker: the specular/normal term is not applied); not
  done for Modesto's bowl, the other circuits' start lines, or any 2048 or Omega pose.
- **The weapon-scene proxies lost margin:** the rocket (`black < 2300`) and cannon
  (`white > 1000`) bounds were re-set from one measured frame each on the dark start
  line; a flat-black rocket may now pass. A control frame needs a pose with no start line
  under the weapon. (HD only; this lane's change reaches 2048 and Omega.)

- **2026-10-20 (`normal-maps-2`), no census change (2048 41, Omega 149).** The family is the
  start line's pipe strip (`ds_pipe_nc`, Vineta K, Moa, Anulpha, Chenghou); its ambient
  program uses the texel's alpha, and Vita3K never compiled its programs in two Vineta K laps
  (the original may not draw it there). Evidence and what closes it: `docs/formats/gxp.md`,
  "second pass". Not done: the water against Vita3K (the seawater lit program is fully
  labelled in the dictionary: two scrolled normal samples, no colour sample, specular from
  `ColourConstant`; no matched frame taken), the water edge, `rocks_01`, `jd_*_n`,
  `amb_glass`, `temp_diffuse`, Omega.

- **2026-10-31 (`2048-normal-maps-re`): `m_diffuse_normal_constantspecular` read and wired.**
  Its colour is `Constant1 * T.a` (rgb a normal) on 2048's USSE programs and Omega's GCN ones
  (Omega fetches alpha alone, `dmask:0x8`); `mesh::rcs::psp2::alpha_product` binds that product,
  unorm bytes (chosen; it must move together with `psp2.rs`'s `constant_colour`, and HD measured
  unorm on this same family too dark against the original, 24/23/2 against 50/61/21, sRGB-encoded
  46/58/15, `docs/formats/rcsmaterial.md`, "Scenery that is a constant"). Census 2048 41 -> 10, Omega 149 -> 67, ratchets re-set. Evidence:
  `docs/formats/gxp.md`, "`m_diffuse_normal_constantspecular` colours by `Texture1`'s alpha".
  Not compared against an original frame; whether the original draws the pipe strip in a race at
  all (Vita3K never compiled its programs in two Vineta K laps) is still open.
- **What is left.** 2048 (10): Chenghou's `diffuse_normal_specular_emmissivenew`
  (`ds_sf_ne`, `ds_sfline_trench_ne`, 7 slots; its ambient program is the next to list with
  `scripts/vita-usse.py disasm`), the Altima water edge (1), `temp_diffuse` on Square and Subway
  (2). Omega (67): `jd_ubermall_gold_n` 16, `rocks_01_normal_alpha` 12, `amb_glass` 8 and twelve
  files of 1 to 4 (`omega_transparency_ground_truth.rs --no-capture` lists them); Omega's
  programs are now listable with `cargo run -p oag-rcs --example gcn_dump`, so a `dmask` read
  per family is the cheap first step.

- **2026-11-11 (`w2048-normal-colour`): Chenghou's `diffuse_normal_specular_emmissivenew` read and
  wired.** Colour `Texture1` (`_cs`, squared) plus `Norm`'s alpha times `Constant1` after the light;
  the material lists the `_ne` map first, which is why it was drawn as the picture. 2048 census
  10 to 3 (the 7 Chenghou slots). Omega ships no `new` file; its plain
  `diffuse_normal_specular_emmissive` ambient GCN program has the same two terms and `emissive_add`
  now admits it on Omega too (red gantry on Chenghou, blue panels on Vineta K). Omega's lit programs
  of the plain family and its `_emmissive_alpha` are unread. Evidence: `docs/formats/gxp.md`,
  "`diffuse_normal_specular_emmissivenew` colours by `Texture1`...".
  Left on 2048 (3): the Altima water edge (1), `temp_diffuse` on Square and Subway (2).

## Next Steps

1. Omega's `jd_ubermall_gold_n` (16), `rocks_01_normal_alpha` (12), `amb_glass` (8) and the twelve
   small ones: a `gcn_dump` `dmask` read per family, the way `m_diffuse_normal_constantspecular`
   was read (`docs/formats/gxp.md`); then 2048's water edge and `temp_diffuse`.
2. Pose-match an RPCS3 frame on Talon's start line and Modesto's bowl; a Vita3K frame of a
   2048 DLC start line for the dark strip.
