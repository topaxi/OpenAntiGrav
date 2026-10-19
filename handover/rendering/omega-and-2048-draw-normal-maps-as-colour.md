---
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

## Next Steps

1. Read what `m_diffuse_normal_constantspecular` does with `Norm`/`Texture1` in its
   ambient-only permutation: a Vita3K `.dsm` of the `Anulpha_Pass` material, or the USSE
   `SMP` consumer. The family is most of the 41 2048 slots and 82 of Omega's materials.
2. Pose-match an RPCS3 frame on Talon's start line and Modesto's bowl; a Vita3K frame of a
   2048 DLC start line for the dark strip.
