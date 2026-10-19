---
categories: [rendering]
---

# Omega and 2048 draw their normal maps as the picture on 18 to 34 circuit models

2026-10-19, `hd-finish-line-purple`. HD's purple start line (a normal and specular map
bound as the road colour, `docs/formats/rcsmaterial.md`, "A normal map is not the
picture") is fixed on HD by reading the resolved fragment program. Omega and 2048 draw
the same files through `psp2::build`, which binds `Material::diffuse_texture` (first
non-lightmap texture) and so still draw it.

## Open

- **Not the cause of Omega's pink road** (`omega-pink-road-sun`, 2026-10-19): that road
  bound an undecodable `R16F` panel map ahead of its `DiffuseMap`, and its circuit was
  not in this census. Naming the diffuse sampler (`Material::diffuse_texture`) did not
  move the counts below; Altima, Mall and Tower now join at 0. Where a family below
  binds a sampler named `NormalMap`/`Normal` ahead of a named diffuse, the new rule is
  already right; the counts are materials where no `Diffuse*` name exists.

- **Counts, drawn colour slots that are normal maps:** Omega 223 over 34 models (Vineta K
  34, Ubermall 33, Metropia 28, Tech De Ra 22 and so on), 2048's Vita package 66 over 18.
  Frozen per circuit in `omega_transparency_ground_truth.rs` (2048 as one constant).
  Families: `constantdiffuse_specular_normal`, `m_diffuse_normal_constantspecular`,
  `ds_booth_glass`, water (`waves2`, `waternormalmap`), rock (`rocks_01_normal_alpha`),
  `jd_*_n`, `amb_glass`.
- **Why not fixed here:** the Vita GXP and PS4 programs are not read for sampler use;
  HD's `Program::samples_as_normal` has no equivalent on them. The sampler hash table on
  `Material::samplers` is shared with HD (`0x3bdc0403`), so the family tables in
  `mesh/rcs/emissive.rs` are a starting point, but a hash alone cannot refuse `0x3bdc0403`.
- **HD families left white times light** (constant parameter not traced):
  `m_diffuse_normal_constantspecular`, `ds_booth_glass`, `metallic`, `dc_constantglow`,
  `cf_constantcolourglow`, `dc_metallic`, `mesh_colour_constant`, `emissive`,
  `dc_glowlightband`. Modesto's `nr_holobowlparallax` picture changed with the rule and is
  unchecked against the original.
- **Original comparison:** done for Talon's start line at the committed grid pose
  (dark thin lines in both, ours darker: the specular/normal term is not applied); not
  done for Modesto's bowl or the other circuits' start lines.
- **The weapon-scene proxies lost margin:** the rocket (`black < 2300`) and cannon
  (`white > 1000`) bounds were re-set from one measured frame each on the dark start
  line; a flat-black rocket may now pass. A control frame needs a pose with no start line
  under the weapon.

## Next Steps

1. Read the Vita GXP's sampler use for `Texture1` on `constantdiffuse_specular_normal`
   and bind no picture there; the census above shrinks.
2. Pose-match an RPCS3 frame on Talon's start line and Modesto's bowl.
