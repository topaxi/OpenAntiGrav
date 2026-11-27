# Shader census: which fragment programs are modelled

Every material an ordinary race draws on HD/Fury, Omega and 2048, grouped by the fragment
program it draws with. Read lanes are drawn from the ranked tables at the end. Measured
2026-11-27 by `crates/mesh/tests/shader_census_ground_truth.rs`; Pulse and Pure are
fixed-function and out of scope.

## What each status means

| status | meaning | confidence in the label |
|---|---|---|
| `matcher:<names>` | a family matcher in `oag-mesh` claimed the material and drew its reading. The reading may be partial: `hull_normal` claims the normal map only | 95, that the matcher ran |
| `generic` | no family matcher claimed it: the default picture, lightmap, lighting and additive readings draw it. Not evidence the program is understood | 95 |
| `unread` | a matcher recognised it and drew without its reading (`unread:<family>`), the material ships no row for the lit-race key (`unread:no-variant`, HD), or the material file is in no archive (`unread:no-file`) | 95 |

The "generic, reading more" tables are the generic programs whose declaration names an input
outside the list of standard ones (`STANDARD` in `scripts/shader-census-tables.py`: albedo,
lightmap, lights, fog, shadow maps and the Zone variant's own). It is a ranking heuristic, not
a reading: an input in the list is not proof it is drawn correctly.

## Method

- **The key** (`oag_rcs::program_key`) is FNV-1a over code bytes only.
  - HD: the fragment block at the lit-race variant `skin::variants` resolves, with every
    16-byte slot a parameter patches zeroed. Against the files' own `program_hash` it never
    splits one hash; it merges 662 hashes into 593 keys.
  - 2048: every fragment `GXP` program of the `.rcsmaterial` (code and literal table), as one
    set key. Which of them a lit race draws is not read, so the file's whole set is the unit.
  - Omega: every pixel-stage `Shdr` program of the `.rcsmaterial`, the same way.
  - A key is not comparable across titles.
- **The status** comes from `oag_mesh::mesh::rcs::census`: each matcher notes the materials it
  claims beside the counter it already keeps. Nothing draws differently.
- **A chunk** is, on HD, one surface of one `.rcsmodel` mesh (checked per model against
  `Report::variant_chunks`); on 2048 and Omega, one shown submesh of the model as the loader
  builds it. Each `.rcsmodel` counts once, so craft (every team, skin and LOD) weigh more than
  one race draws and circuits are not multiplied by laps or visibility.

## Population

| title | models | where from |
|---|---|---|
| HD/Fury | 499 | `DATA00`-`DATA06` of the EU disc image |
| Omega | 1104 (36 HD-lineage circuit, 50 2048 circuit, 1018 other) | `data05`, `07`-`09` of the patch over `data00`-`04` of the base |
| 2048 | 975 | EU `data2`, `data1` over `data`, then `dlc1`, `dlc2` |

A path held by more than one archive is read once, from the first in that order. For HD that
is the same copy `Archives` serves (`DATA00`, `DATA02`, then the rest): `DATA01`, which the
engine mounts ahead of `DATA03`-`06`, holds no model or material. The disc updates
(`data07`, `data08`) hold no `.rcsmodel` or `.rcsmaterial`
([`hd-updates.md`](../formats/hd-updates.md)), so they are not mounted here.

The setup arguments are a race's: `graphics.water` at `original` (the default) and the
lightmapped emissive families on, as craft and circuit builds pass them. The pad pass
(`mesh::rcs::pads`) builds with them off; its chunks are counted from the whole model's setup.

Left out, as not part of an ordinary race: `/fe/` models, the `zone_N` environments, the Zone
and Detonator craft, Zone battle HUD and pads, `new_mode_pads`, `pvsblocker` occluders (never
drawn) and `aurora`. Zone-mode program variants are not censused: HD's is the lit-race key's
row only. The HD PSN download edition (`data/extracted/ps3/hd-psn-eu`) is not censused; its
`.rcsmodel`/`.rcsmaterial` entries differ from the disc's in `data02` and `data03`.

## Totals by chunks

| title | programs | chunks | matcher | generic, reads more | generic, plain | unread |
|---|---|---|---|---|---|---|
| HD/Fury | 608 | 61823 | 11679 (18.9%) | 22544 (36.5%) | 26058 (42.1%) | 1542 (2.5%) |
| Omega | 679 | 188304 | 76613 (40.7%) | 68158 (36.2%) | 43413 (23.1%) | 120 (0.1%) |
| 2048 | 418 | 204666 | 72015 (35.2%) | 42982 (21.0%) | 89668 (43.8%) | 1 (0.0%) |

## Chunks by matcher

- **HD/Fury**: hull_normal 5642, alpha_emissive 1175, vertex_scroll 1105, window_glass 820, lightmapped_emissive 742, mag_wave 390, holo_billboard 301, hull_normal+probe_lit 277
- **Omega**: fc01_placeholder 35498, zone_colour 28337, crowd 3372, hull_colour 2604, emissive_add+pad_bars 1832, alpha_emissive 1333, glow_layer 1057, emissive_add 874
- **2048**: fc01_placeholder 35498, zone_colour 28337, emissive_add 1964, emissive_add+pad_bars 1852, alpha_emissive 1849, glow_layer 1101, vertex_blend 882, lit_alpha 158

Two simple readings make most of the matcher share on the 2048-lineage circuits:
`fc01_placeholder` (35,498 submeshes drawn as the fog colour) and `zone_colour` (28,337 drawn
in the authored Zone colour). Without them the matcher share is 10.3% on Omega and 5.8% on
2048.

Read as a yes or no (generic counting as not modelled), the modelled share by chunks is the
matcher column: HD 18.9%, Omega 40.7%, 2048 35.2%.

HD's `unread:no-variant` billboards and ribbon effects may draw through another pass
(billboards are rendered to a texture, ribbons by the effect renderer); whether the original
does is not read, so they are not a counted defect yet. `defuse_occulsion_vert_col_tint` on
Modesto Heights, Tech de Ra and Talon's Junction is the one circuit family in that state.

The 2048/Omega rule (a finding on one is checked against the other): **ported**, both are
censused by the same test.

## Regenerate

```sh
OAG_CENSUS_OUT=<dir> OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh \
    --run-ignored all -E 'test(/^shader_census_ground_truth::/)' --no-fail-fast
python3 scripts/shader-census-tables.py <dir>
```

A change that moves a program's status fails the test naming the key; copy the new
`<dir>/<name>.tsv` over `crates/mesh/tests/shader_census/` once the change is intended, and
update this page.

## HD/Fury: unread (17 programs)

| program | status | chunks | material families | circuits (chunks) | file |
|---|---|---|---|---|---|
| `nokey:defuse_occulsion_vert_col_tint` | unread:no-variant | 666 | defuse_occulsion_vert_col_tint | modesto_heights 352, tech_de_ra 206, talons_junction 108 | `DATA00:data/environments/modesto_heights/materials/defuse_occulsion_vert_col_tint.rcsmaterial` |
| `nokey:simpletexture` | unread:no-variant | 311 | simpletexture | billboards 311 | `DATA00:data/materials/billboards/simpletexture.rcsmaterial` |
| `nokey:simpletextureandtexturealphauvoffsetscale` | unread:no-variant | 251 | simpletextureandtexturealphauvoffsetscale | billboards 251 | `DATA00:data/materials/billboards/simpletextureandtexturealphauvoffsetscale.rcsmaterial` |
| `2cddeede6503a35f` | unread:probe_lit | 153 | nitro_perspex_new | craft 153 | `DATA06:data/materials/ships/nitro_perspex_new.rcsmaterial @0x4250` |
| `nokey:simpletextureuvoffsetscale` | unread:no-variant | 50 | simpletextureuvoffsetscale | billboards 50 | `DATA02:data/materials/billboards/simpletextureuvoffsetscale.rcsmaterial` |
| `nokey:simpletextureandtexturealpha` | unread:no-variant | 47 | simpletextureandtexturealpha | billboards 47 | `DATA00:data/materials/billboards/simpletextureandtexturealpha.rcsmaterial` |
| `nokey:constantcolour` | unread:no-variant | 45 | constantcolour | billboards 45 | `DATA02:data/materials/billboards/constantcolour.rcsmaterial` |
| `nokey:cf_fx350` | unread:no-variant | 7 | cf_fx350 | billboards 7 | `DATA02:data/materials/billboards/cf_fx350.rcsmaterial` |
| `nokey:vertexcolourandtexturealpha` | unread:no-variant | 4 | vertexcolourandtexturealpha | billboards 4 | `DATA02:data/materials/billboards/vertexcolourandtexturealpha.rcsmaterial` |
| `8c9534915b807d6d` | unread:probe_lit | 1 | glass_colour_spec_trans | craft 1 | `DATA02:data/materials/ships/glass_colour_spec_trans.rcsmaterial @0x3bf0` |
| `nokey:hd_enginetrail` | unread:no-variant | 1 | hd_enginetrail | ribboneffects 1 | `DATA02:data/ribboneffects/materials/hd_enginetrail.rcsmaterial` |
| `nokey:hd_enginetrail_bluered` | unread:no-variant | 1 | hd_enginetrail_bluered | ribboneffects 1 | `DATA06:data/ribboneffects/materials/hd_enginetrail_bluered.rcsmaterial` |
| `nokey:hd_leachbeam` | unread:no-variant | 1 | hd_leachbeam | ribboneffects 1 | `DATA02:data/ribboneffects/materials/hd_leachbeam.rcsmaterial` |
| `nokey:hd_rockettrail` | unread:no-variant | 1 | hd_rockettrail | ribboneffects 1 | `DATA02:data/ribboneffects/materials/hd_rockettrail.rcsmaterial` |
| `nokey:hd_rockettrail_shadow` | unread:no-variant | 1 | hd_rockettrail_shadow | ribboneffects 1 | `DATA02:data/ribboneffects/materials/hd_rockettrail_shadow.rcsmaterial` |
| `nokey:hd_waketrail` | unread:no-variant | 1 | hd_waketrail | ribboneffects 1 | `DATA02:data/ribboneffects/materials/hd_waketrail.rcsmaterial` |
| `nokey:vertexcolourandtexture` | unread:no-variant | 1 | vertexcolourandtexture | billboards 1 | `DATA02:data/materials/billboards/vertexcolourandtexture.rcsmaterial` |

## HD/Fury: generic, reading more than the standard inputs (372 programs, top 30)

| program | status | chunks | material families | circuits (chunks) | file | reads beyond the standard inputs |
|---|---|---|---|---|---|---|
| `cb4fa5f985e2d984` | generic | 1966 | nr_crowd_bustle | 04_chenghou_project 732, 10_sebenco_climb 628, 01_vineta_k 452, +3 | `DATA02:data/environments/01_vineta_k/materials/nr_crowd_bustle.rcsmaterial @0x2630` | noiseScale, tex:0379ee32 |
| `48929cc84561b734` | generic | 1846 | base_diffusespecular, cf_diff_spec | modesto_heights 1348, amphiseum 498 | `DATA00:data/environments/amphiseum/materials/base_diffusespecular.rcsmaterial @0x2b60` | 2924e4ad, SpecularColour, SpecularTexture |
| `c6f387aa871dd7c6` | generic | 1620 | base_diffusespecular, cf_diff_spec | modesto_heights 874, amphiseum 746 | `DATA00:data/environments/amphiseum/materials/base_diffusespecular.rcsmaterial @0x7810` | 2924e4ad, SpecularColour, SpecularTexture |
| `e91f1da8734a664b` | generic | 844 | diffuse_with_specular_from_alpha, tech_de_ra_diffuse_spec, tech_de_ra_diffuse_spec_test | talons_junction 362, tech_de_ra 334, modesto_heights 140, +1 | `DATA00:data/environments/amphiseum/materials/diffuse_with_specular_from_alpha.rcsmaterial @0x2960` | SpecScale, tex:fb17503f |
| `fc951c3ae3e54662` | generic | 750 | cf_billboard1, emissivetexture | amphiseum 596, talons_junction 134, modesto_heights 18, +1 | `DATA00:data/environments/amphiseum/materials/emissivetexture.rcsmaterial @0x13b0` | EmissiveTexture |
| `2a7d9f6baa26162a` | generic | 503 | blimphulloneuv, diffuse_with_specular_from_alpha, hd_bomb, +1 | 03_track 192, 12_sol_2 116, 01_vineta_k 68, +5 | `DATA02:data/environments/01_vineta_k/materials/hd_bomb.rcsmaterial @0x1540` | SpecularPower, tex:9d55a89a |
| `04f5e76af37c29b8` | generic | 424 | reflectplane_dc_seawateredging | 03_track 424 | `DATA02:data/environments/03_track/materials/reflectplane_dc_seawateredging.rcsmaterial @0x3d10` | 906b67ba, SpecularColour, SpecularPower, tex:fabde27e, tex:fc52b822 |
| `bac49c53cba5b824` | generic | 412 | art2_basic | tech_de_ra 412 | `DATA00:data/environments/tech_de_ra/materials/art2_basic.rcsmaterial @0x28d0` | 7611a2d8 |
| `711011fbad467376` | generic | 402 | d_cs, dn_diffuse_specular | 02_track 398, 05_ubermall 4 | `DATA02:data/environments/02_track/materials/dn_diffuse_specular.rcsmaterial @0xd800` | tex:832aa927, tex:f2ef8543 |
| `f332830c01bfe129` | generic | 402 | j_alphascalar | modesto_heights 402 | `DATA00:data/environments/modesto_heights/materials/j_alphascalar.rcsmaterial @0x1530` | Emissive, Scaler |
| `1d4f947481ab4996` | generic | 396 | simplefogdiffuse | talons_junction 326, modesto_heights 70 | `DATA00:data/environments/modesto_heights/materials/simplefogdiffuse.rcsmaterial @0x2e70` | 19158085, 74df0ab9, bf3290fb, c45093a3, fe7f3ef4 |
| `9f80484e7f6a1e5e` | generic | 376 | glass_reflect | 05_ubermall 342, 04_chenghou_project 16, 01_vineta_k 12, +1 | `DATA02:data/environments/01_vineta_k/materials/glass_reflect.rcsmaterial @0x2d60` | 7480de6d, paraboloidReflectionTex, tex:c7c97729 |
| `f49c6b15c0bad756` | generic | 348 | diffuse_with_specular_from_alpha, tech_de_ra_diffuse_spec, tech_de_ra_diffuse_spec_test | talons_junction 150, modesto_heights 140, tech_de_ra 48, +1 | `DATA00:data/environments/amphiseum/materials/diffuse_with_specular_from_alpha.rcsmaterial @0x7410` | SpecScale, tex:fb17503f |
| `45d12a910559bc7e` | generic | 342 | detonator_diffuse_vcol, diffuse_vcol | craft 342 | `DATA02:data/materials/ships/diffuse_vcol.rcsmaterial @0x32e0` | 4c13d3af, tex:fb17503f |
| `66ea6071c4550d9c` | generic | 333 | flame_test | craft 333 | `DATA00:data/materials/ships/engines/flame_test.rcsmaterial @0x3370` | 17d9b3d3, 4c13d3af, 906b67ba, 92fc84bf, Speed, min1, power1, scale1, +1 |
| `6acf0ea4df519ffe` | generic | 318 | pb_diffusealpha4spec | 02_track 294, 12_sol_2 18, 05_ubermall 6 | `DATA02:data/environments/02_track/materials/pb_diffusealpha4spec.rcsmaterial @0xe000` | 7611a2d8, tex:6e1426b8 |
| `1e988447fe68e471` | generic | 254 | mt_diffuse_glow_specular_01 | 15_anulpha_pass 254 | `DATA02:data/environments/15_anulpha_pass/materials/mt_diffuse_glow_specular_01.rcsmaterial @0x3ce0` | 2d7c53b5, AlphaTexture |
| `f3d1a080068d0b41` | generic | 252 | nr_crowd_bustle | modesto_heights 134, talons_junction 58, amphiseum 50, +1 | `DATA00:data/environments/amphiseum/materials/nr_crowd_bustle.rcsmaterial @0x2810` | noiseScale, tex:0379ee32 |
| `d813d0a7393a2477` | generic | 250 | dn_diffuse_specular | 02_track 236, 05_ubermall 14 | `DATA02:data/environments/02_track/materials/dn_diffuse_specular.rcsmaterial @0x64e0` | tex:832aa927, tex:f2ef8543 |
| `a7580810130998cc` | generic | 242 | bluemetal | talons_junction 242 | `DATA00:data/environments/talons_junction/materials/bluemetal.rcsmaterial @0x2fd0` | 901d9fa8, SpecularTexture, VScale, da67aa34, ef18f362, f9db3a85, tex:35281c78 |
| `48ba28926d7625be` | generic | 232 | d_s_n_r, d_s_n_r_uvflip | 05_ubermall 232 | `DATA02:data/environments/05_ubermall/materials/d_s_n_r.rcsmaterial @0xab10` | 7480de6d, Colour, Normal, paraboloidReflectionTex, tex:68a741d2 |
| `739de6f974602bf6` | generic | 214 | mt_diffuse_specular | 15_anulpha_pass 214 | `DATA02:data/environments/15_anulpha_pass/materials/mt_diffuse_specular.rcsmaterial @0x75b0` | SpecularScalar, tex:152839b8, tex:fb17503f |
| `e922e5c6a4ec3c91` | generic | 213 | holographic_test | craft 213 | `DATA02:data/materials/ships/holographic_test.rcsmaterial @0x4470` | 4c13d3af, Colour_Tint, Transparency, min, power, scale, tex:15b908fd, tex:1b2a3d71 |
| `20792413a02562e5` | generic | 198 | lambert_spec_mult, outersurround01 | talons_junction 126, tech_de_ra 72 | `DATA00:data/environments/talons_junction/materials/outersurround01.rcsmaterial @0x2610` | 7611a2d8, Colour, Spec |
| `a9147781e85bb615` | generic | 190 | mt_uvanim_diffuse_emissive, mt_uvanim_diffuse_emissive2, mt_uvanim_diffuse_emissive3tokey | 15_anulpha_pass 150, 01_vineta_k 30, 02_track 10 | `DATA02:data/environments/01_vineta_k/materials/mt_uvanim_diffuse_emissive2.rcsmaterial @0x1f10` | 78256a45, 78787596, 906b67ba, EmissiveTexture, e8bcd7f5 |
| `89c45a2f494699b1` | generic | 181 | glass_reflect_opacity_normal | 05_ubermall 145, 02_track 36 | `DATA02:data/environments/02_track/materials/glass_reflect_opacity_normal.rcsmaterial @0xc5d0` | 7480de6d, Texture2, paraboloidReflectionTex |
| `196e1352c60d5221` | generic | 178 | defuse_occulsion_vert_col_tint | amphiseum 178 | `DATA00:data/environments/amphiseum/materials/defuse_occulsion_vert_col_tint.rcsmaterial @0x1630` | b51ff3f0, ebecee0f, tex:fb17503f |
| `c29ab0a82b03040e` | generic | 174 | cf_constantcolourglow, dc_constantglow | 02_track 88, 01_vineta_k 68, 03_track 16, +1 | `DATA02:data/environments/01_vineta_k/materials/cf_constantcolourglow.rcsmaterial @0x1fe0` | 7611a2d8 |
| `2f8e696ff2472fb1` | generic | 162 | j_diffuse_colour_tint_spec, wes_diffuse_colourtint_specular | amphiseum 140, modesto_heights 22 | `DATA00:data/environments/amphiseum/materials/j_diffuse_colour_tint_spec.rcsmaterial @0x2b60` | ColourTint, SpecularTexture, spec_power |
| `92e988b24fad6691` | generic | 158 | cf_constantcolourglow, dc_constantglow | 03_track 114, 02_track 34, 05_ubermall 8, +1 | `DATA02:data/environments/01_vineta_k/materials/cf_constantcolourglow.rcsmaterial @0x6360` | 7611a2d8 |

## Omega: unread (1 programs)

| program | status | chunks | material families | circuits (chunks) | file |
|---|---|---|---|---|---|
| `4887736fca031911` | unread:panel_map | 120 | road_alphaspec_normal_emissive_panelmap1_mask | altima 77, tower 27, square 16 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/road_alphaspec_normal_emissive_panelmap1_mask.rcsmaterial` |

## Omega: generic, reading more than the standard inputs (547 programs, top 30)

| program | status | chunks | material families | circuits (chunks) | file | reads beyond the standard inputs |
|---|---|---|---|---|---|---|
| `cb7b1741fae4c317` | generic | 4671 | diffuse_normal_specular | square 810, mall 781, subway 682, +7 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/diffuse_normal_specular.rcsmaterial` | Diffuse_AlphaSpecMaskSampler, NormalSampler, SpecStrength, SpecularPower |
| `616ccd31d0ac5b96` | generic | 4334 | 01_normal_diffuse_specularonalpha, diffuse_normal_specular, diffuse_normalrgb_specular | 01_vineta_k 2653, 10_sebenco_climb 468, 03_moa_therma 356, +8 | `data08.psarc:data/environments/01_vineta_k/materials_dlc/diffuse_normal_specular.rcsmaterial` | Texture2Sampler |
| `1794cf7ff399751f` | generic | 3402 | base_diffusespecular, cf_diff_spec | modesto_heights 2276, amphiseum 1126 | `data08.psarc:data/environments/amphiseum/materials/base_diffusespecular.rcsmaterial` | SpecularColour, SpecularPowerConstant, SpecularTextureSampler |
| `be2ed27a50a9540c` | generic | 3171 | diffuse_alphareflect_blur_normal_fresnel | bridge 737, subway 536, mall 408, +7 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/diffuse_alphareflect_blur_normal_fresnel.rcsmaterial` | DiffuseAlphaReflectMaskSampler, NormalMapSampler, ReflectBlur, ReflectivityMin, ReflectivityPower, ReflectivityScale, ReflectivityStrength, Spec_Strength, +3 |
| `d5372b21a0d33090` | generic | 1737 | zonefc06_lambert_emissive_scalar_1, zonefc06_lambert_emissive_scalar_3 | arena 483, bridge 407, cathedral 291, +7 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/zonematerials/zonefc06_lambert_emissive_scalar_3.rcsmaterial` | Zone_WindowGradient3Sampler, Zone_WindowGradient3_Emissive |
| `2ebcb144879f44d1` | generic | 1590 | diffuse_normal_specular | 05_ubermall 1590 | `data08.psarc:data/environments/05_ubermall/materials/diffuse_normal_specular.rcsmaterial` | Texture2Sampler |
| `32e7a1b05c5cde23` | generic | 1559 | fc07_lambert_emmisive | mall 487, cathedral 474, square 278, +3 | `data08.psarc:data/environments2048/bridge/materials/fc07_lambert_emmisive.rcsmaterial` | EmissiveMapSampler, Tint |
| `851ce3ccf1b30591` | generic | 1513 | fc01_emissive_simple | mall 295, square 241, tower 189, +7 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/fc01_emissive_simple.rcsmaterial` | EmissiveMapSampler, EmissiveStrength |
| `350daa8b22a1c612` | generic | 1500 | blimphulloneuv, diffuse_with_specular_from_alpha, hd_bomb, +1 | talons_junction 522, modesto_heights 286, tech_de_ra 242, +7 | `data08.psarc:data/environments/01_vineta_k/materials_dlc/blimphulloneuv.rcsmaterial` | Constant1 |
| `fae7062668ada002` | generic | 1461 | tree_wind_new | park 418, square 187, tower 177, +6 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/tree_wind_new.rcsmaterial` | Ambient, FresnelMin, FresnelPower, FresnelScale, NormalMapSampler, Shadowed, SpecPower, SpecStrength |
| `a0ac9e5349c7ecd6` | generic | 1259 | zonefc07_cube_animation_1 | tower 220, sol 202, park 186, +7 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/zonematerials/zonefc07_cube_animation_1.rcsmaterial` | Zone_CubeAnim_Emissive_1, Zone_CubeAnim_GradientTex_1Sampler |
| `2f60f83679e962f7` | generic | 962 | zonefc06_lambert_emissive_scalar_2 | sol 295, bridge 211, arena 181, +7 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/zonematerials/zonefc06_lambert_emissive_scalar_2.rcsmaterial` | Zone_WindowGradient2Sampler, Zone_WindowGradient2_Emissive |
| `8d161dddbc05328a` | generic | 946 | zonefc09_eq_gradient | altima 318, bridge 296, square 77, +7 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/zonematerials/zonefc09_eq_gradient.rcsmaterial` | ZoneEQ_BGColour_A, ZoneEQ_BGColour_B, ZoneEQ_BGColour_C, ZoneEQ_Colour_A, ZoneEQ_Colour_B, ZoneEQ_Colour_C, ZoneVisPalette_BlockySampler, ZoneVisPalette_SmoothSampler |
| `a16ea690c930d19d` | generic | 906 | startanim_fc07_lambert_emmisive | bridge 185, arena 156, subway 153, +3 | `data08.psarc:data/environments2048/altima/materials/startanim_fc07_lambert_emmisive.rcsmaterial` | EmissiveMapSampler, Tint |
| `5dcdf57714bac987` | generic | 881 | diffuse_normal_specular, fc16_phong_alpha_normal_specpow | park 562, bridge 150, tower 70, +3 | `data08.psarc:data/environments2048/arena/materials/diffuse_normal_specular.rcsmaterial` | Diffuse_AlphaSpecMaskSampler, NormalSampler, SpecularPower |
| `ca07111b2e501668` | generic | 837 | cf_billboard1, cf_uvanim_emssive, emissivetexture | amphiseum 596, talons_junction 134, 05_ubermall 106, +1 | `data08.psarc:data/environments/02_metropia/materials_dlc/cf_billboard1.rcsmaterial` | EmissiveSampler |
| `d017dff15990bb8d` | generic | 796 | tree_wind_new | 03_moa_therma 626, modesto_heights 104, 04_chenghou_project 50, +1 | `data08.psarc:data/environments/03_moa_therma/materials/tree_wind_new.rcsmaterial` | Ambient, FresnelMin, FresnelPower, FresnelScale, NormalMapSampler, Shadowed, SpecPower, SpecStrength |
| `55aa5f3cf145ec06` | generic | 779 | diffuse_normal_specular_emissive, fc16_phong_alphaspec_normal_emissive | altima 120, square 112, arena 86, +7 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/fc16_phong_alphaspec_normal_emissive.rcsmaterial` | EmissiveMapSampler, NormalMapSampler, SpecPower |
| `3076c444762a466a` | generic | 753 | diffuse_normal_specular_displacement2out, diffuse_normal_specular_panelmap2 | 04_chenghou_project 568, 12_sol_2 185 | `data08.psarc:data/environments/04_chenghou_project/materials/diffuse_normal_specular_displacement2out.rcsmaterial` | Cg_PanelDisplacement_2Out1Aspect, Cg_PanelDisplacement_2Out1Depth, Cg_PanelDisplacement_2Out1DepthPc2, Cg_PanelDisplacement_2Out1SlopeWidth1, Cg_PanelDisplacement_2Out1SlopeWidth2, Cg_PanelDisplacement_2Out1panel_mapSampler, NormalSampler |
| `e946bf95393c2339` | generic | 744 | diffuse_car | mall 174, arena 173, square 150, +4 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/diffuse_car.rcsmaterial` | DiffuseAlphaReflectMaskSampler, EmissiveStrength, EmissiveTextureSampler, NormalMapSampler, ReflectiveBlur, ReflectivityMin, ReflectivityPower, ReflectivityScale, +5 |
| `f49a2521c5a86697` | generic | 676 | diffuse_specular, dn_diffuse_specular | 02_metropia 612, tech_de_ra 50, 05_ubermall 14 | `data08.psarc:data/environments/02_metropia/materials/dn_diffuse_specular.rcsmaterial` | diffuse_TEXTURESampler, specular_TEXTURESampler |
| `4c0681919664c090` | generic | 674 | defuse_occulsion_vert_col_tint | modesto_heights 352, tech_de_ra 188, talons_junction 134 | `data08.psarc:data/environments/modesto_heights/materials/defuse_occulsion_vert_col_tint.rcsmaterial` | Spec_power, colour_TINT |
| `94afedcfcfa12992` | generic | 615 | diffuse_alphareflect_blur_normal_fresnel_interior | square 353, mall 145, subway 52, +3 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/diffuse_alphareflect_blur_normal_fresnel_interior.rcsmaterial` | DiffuseAlphaReflectMaskSampler, InteriorTextureSampler, NormalMapSampler, ReflectiveBlur, ReflectivityMin, ReflectivityPower, ReflectivityScale, ReflectivityStrength, +4 |
| `92280f82f57d9f7b` | generic | 604 | glass_reflect | 05_ubermall 354, 03_moa_therma 118, tech_de_ra 34, +6 | `data08.psarc:data/environments/01_vineta_k/materials_dlc/glass_reflect.rcsmaterial` | Glass_reflectSampler, cubeDistanceTexSampler, cubeReflectionTexSampler |
| `cc39f8dddd81b780` | generic | 596 | 2048_engine_additive, 2048_engine_additive_unique | craft 596 | `data08.psarc:data/art/published/ships/materials/2048_engine_additive.rcsmaterial` | Flame_TextureSampler, Speed, facing_gradientSampler, rgb_Scale, time |
| `ab001e63c8e11a1a` | generic | 582 | lambert_simple, simpletexture, simpletextureuvoffsetscale | billboards 526, weapons 28, weapons2048 28 | `data08.psarc:data/materials/billboards/simpletextureuvoffsetscale.rcsmaterial` | diffuseTextureSampler |
| `f3264151ac78c906` | generic | 544 | startanim_fc07_lambert_emmisive | square 152, tower 140, sol 127, +1 | `data08.psarc:data/environments2048/mall/materials/startanim_fc07_lambert_emmisive.rcsmaterial` | EmissiveMapSampler, Tint |
| `4931cf5064bfcb4f` | generic | 541 | fc01_effects_vscroll_emissive_alpha, fc01_effects_vscroll_emissive_alpha_glow, fc01_emissive_simple, +1 | mall 266, altima 176, tower 80, +7 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/fc01_effects_vscroll_emissive_alpha.rcsmaterial` | EmissiveAlphaMapSampler |
| `c1ef4fe96298347e` | generic | 534 | fc06_lambert_emmissive_alpha | sol 143, mall 129, subway 92, +5 | `data08.psarc:data/environments2048/shared/objectlibrary/materials/fc06_lambert_emmissive_alpha.rcsmaterial` | Tint |
| `74c7c4f7b32babb0` | generic | 524 | cf_alpha4glow | 10_sebenco_climb 524 | `data08.psarc:data/environments/10_sebenco_climb/materials/cf_alpha4glow.rcsmaterial` | EmissveBloom |

## 2048: unread (1 programs)

| program | status | chunks | material families | circuits (chunks) | file |
|---|---|---|---|---|---|
| `nokey:fc06_lambert_simple` | unread:no-file | 1 | fc06_lambert_simple | startanim 1 | `?:data/startanim/materials/fc06_lambert_simple.rcsmaterial` |

## 2048: generic, reading more than the standard inputs (324 programs, top 30)

| program | status | chunks | material families | circuits (chunks) | file | reads beyond the standard inputs |
|---|---|---|---|---|---|---|
| `ddf9428625f9109e` | generic | 3651 | fc07_lambert_emmisive | mall 775, cathedral 672, square 664, +7 | `data2.psarc:data/art/published/environments/altima/materials/fc07_lambert_emmisive.rcsmaterial` | EmissiveMap, Tint |
| `b2851135c91d45f6` | generic | 2699 | zonefc06_lambert_emissive_scalar_1, zonefc06_lambert_emissive_scalar_2, zonefc06_lambert_emissive_scalar_3 | arena 664, bridge 618, sol 575, +7 | `data2.psarc:data/art/published/environments/shared/objectlibrary/materials/zonematerials/zonefc06_lambert_emissive_scalar_3.rcsmaterial` | Zone_WindowGradient3, Zone_WindowGradient3_Emissive |
| `4b3e0367e830183a` | generic | 2213 | fc06_lambert_emmissive_alpha | bridge 782, arena 439, mall 263, +7 | `data2.psarc:data/art/published/environments/altima/materials/fc06_lambert_emmissive_alpha.rcsmaterial` | Tint |
| `9bcced5b67ba4a96` | generic | 1775 | cf_billboard1, cf_uvanim_emssive, emissivetexture, +1 | amphiseum 814, modesto_heights 302, talons_junction 208, +3 | `dlc2.psarc:data/art/published/dlc1/environments/amphiseum/materials/emissivetexture.rcsmaterial` | EmissiveTexture |
| `00b742c321dbfa6d` | generic | 1727 | startanim_fc07_lambert_emmisive | bridge 220, subway 181, arena 180, +7 | `data2.psarc:data/art/published/environments/altima/materials/startanim_fc07_lambert_emmisive.rcsmaterial` | EmissiveMap, Tint |
| `873b61afea57c26e` | generic | 1605 | fc01_effects_vscroll_emissive, fc01_emissive_simple, fc01_video_emissive | mall 580, tower 328, altima 170, +7 | `data2.psarc:data/art/published/environments/shared/objectlibrary/materials/fc01_emissive_simple.rcsmaterial` | EmissiveMap |
| `cbd4b95ad06dcd00` | generic | 1460 | pb_diffusealpha4spec | metropia 1450, sol_2 10 | `dlc1.psarc:data/art/published/dlc1/environments/metropia/materials/pb_diffusealpha4spec.rcsmaterial` | Constant1, DiffuseAlphaSpecularTexture |
| `074df0106eab4ab0` | generic | 1259 | zonefc07_cube_animation_1 | tower 220, sol 202, park 186, +7 | `data2.psarc:data/art/published/environments/shared/objectlibrary/materials/zonematerials/zonefc07_cube_animation_1.rcsmaterial` | Zone_CubeAnim_Emissive_1, Zone_CubeAnim_GradientTex_1 |
| `e6d1f521b74be851` | generic | 1200 | fc07_lambert_emmisive | talons_junction 1176, modesto_heights 24 | `dlc2.psarc:data/art/published/dlc1/environments/modesto_heights/materials/fc07_lambert_emmisive.rcsmaterial` | EmissiveMap, Tint |
| `7f17dd0a9445790e` | generic | 1084 | 2048_ship_carbon_fibre_shiny, 2048_ship_paint_shiny, 2048_ship_paint_shiny_final, +2 | craft 1084 | `data2.psarc:data/art/published/materials/ships/diffuse_with_specular_from_alpha_n_vcol.rcsmaterial` | Normal_Map, Refbrightness, SpecScale, paraboloidReflectionTex |
| `7ed0d4a03aa782fe` | generic | 1026 | fc06_lambert_scalar_emissive_alpha | mall 319, tower 188, park 185, +6 | `data2.psarc:data/art/published/environments/altima/materials/fc06_lambert_scalar_emissive_alpha.rcsmaterial` | Constant1 |
| `717d8402d5986711` | generic | 946 | zonefc09_eq_gradient | altima 318, bridge 296, square 77, +7 | `data2.psarc:data/art/published/environments/shared/objectlibrary/materials/zonematerials/zonefc09_eq_gradient.rcsmaterial` | ZoneEQ_BGColour_A, ZoneEQ_BGColour_B, ZoneEQ_BGColour_C, ZoneEQ_Colour_A, ZoneEQ_Colour_B, ZoneEQ_Colour_C, ZoneVisPalette_Blocky, ZoneVisPalette_Smooth |
| `90763226aa3cc288` | generic | 880 | speed_pads | bridge 166, square 110, mall 94, +7 | `data2.psarc:data/art/published/environments/altima/materials/speed_pads.rcsmaterial` | Texture2, W_Cycle |
| `e5ba12b805b85407` | generic | 712 | glass_2nduv_reflect_glow | metropia 702, sebenco_climb 10 | `dlc1.psarc:data/art/published/dlc1/environments/metropia/materials/glass_2nduv_reflect_glow.rcsmaterial` | Glow_colour_strength2, Texture2_alpha_for_glow, paraboloidReflectionTex, texture1_alpha_for_reflect |
| `ceb150981dc5cea8` | generic | 703 | fc07_diffuse_glow_asemissive_colour | anulpha_pass 703 | `data1.psarc:data/art/published/dlc1/environments/anulpha_pass/materials/fc07_diffuse_glow_asemissive_colour.rcsmaterial` | AlphaTexture, EmmisiveColour |
| `6bfef69690e91d25` | generic | 694 | fc06_lambert_emmissive_alpha | moa_therma 373, vineta_k 321 | `data1.psarc:data/art/published/dlc1/environments/moa_therma/materials/fc06_lambert_emmissive_alpha.rcsmaterial` | Tint |
| `9d35c5c6421f7fd7` | generic | 678 | glass_2uv_rgb_reflect_glow | metropia 678 | `dlc1.psarc:data/art/published/dlc1/environments/metropia/materials/glass_2uv_rgb_reflect_glow.rcsmaterial` | Glow_colour_strength2, Texture2_alpha_for_glow, paraboloidReflectionTex, texture1_alpha_for_reflect |
| `89800f7e0b722605` | generic | 641 | dc_constantglow, fc01_colour_emissive | amphiseum 178, moa_therma 167, ubermall 162, +1 | `dlc2.psarc:data/art/published/dlc1/environments/amphiseum/materials/fc01_colour_emissive.rcsmaterial` | Colour |
| `d720f7caa22a1370` | generic | 631 | fc16_phong_alpha_normal_specpow | tower 215, mall 121, bridge 109, +2 | `data2.psarc:data/art/published/environments/shared/objectlibrary/materials/fc16_phong_alpha_normal_specpow.rcsmaterial` | NormalMap, SpecPower |
| `f2d5a69f404c9b12` | generic | 594 | diffuse_normal_specular, diffuse_normal_specularnew | chenghou_project 313, vineta_k 278, anulpha_pass 3 | `data1.psarc:data/art/published/dlc1/environments/anulpha_pass/materials/diffuse_normal_specular.rcsmaterial` | Texture2 |
| `abeabf2c8d8e36c1` | generic | 534 | fc01_emissive_alpha_emistint | park 150, sol 80, bridge 74, +7 | `data2.psarc:data/art/published/environments/shared/objectlibrary/materials/fc01_emissive_alpha_emistint.rcsmaterial` | emisiveTint |
| `28d84f8cee90f020` | generic | 517 | 2048_engine_additive | craft 517 | `data2.psarc:data/art/published/ships/materials/2048_engine_additive.rcsmaterial` | Flame_Texture, Speed, facing_gradient, rgb_Scale, time |
| `e0e4d7d0a2b916da` | generic | 505 | 01_normal_diffuse_specularonalpha, diffuse_normal_specular | sol_2 349, sebenco_climb 54, metropia 52, +2 | `dlc2.psarc:data/art/published/dlc1/environments/amphiseum/materials/01_normal_diffuse_specularonalpha.rcsmaterial` | Texture2 |
| `fc2f5ff58082c9d2` | generic | 472 | j_alphascalar | modesto_heights 472 | `dlc2.psarc:data/art/published/dlc1/environments/modesto_heights/materials/j_alphascalar.rcsmaterial` | Emissive, Scaler |
| `61b123ce3d49fd5c` | generic | 459 | fc02_effects_uscroll_scanlines_emissive, videoscreen_fc02_patternoverlay, videoscreen_fc02_patternoverlay_scrolling | square 182, tower 72, subway 38, +7 | `data2.psarc:data/art/published/environments/shared/objectlibrary/materials/videoscreen_fc02_patternoverlay.rcsmaterial` | ScreenCellTexture, VideoScreenTexture |
| `b640da461fea0e11` | generic | 426 | fc18_phong_alpha_tintspec_normal_specpow | cathedral 126, arena 112, altima 84, +6 | `data2.psarc:data/art/published/environments/shared/objectlibrary/materials/fc18_phong_alpha_tintspec_normal_specpow.rcsmaterial` | NormalMap, SpecPower, SpecTint |
| `9c107da3f6f1d911` | generic | 422 | cf_blimplights_anim, cf_laserrail_cap | sol_2 316, sebenco_climb 104, metropia 2 | `dlc1.psarc:data/art/published/dlc1/environments/metropia/materials/cf_blimplights_anim.rcsmaterial` | Texture2 |
| `9d3429108641f44f` | generic | 383 | 2048_ship_tech | craft 383 | `data2.psarc:data/art/published/ships/materials/2048_ship_tech.rcsmaterial` | Normal_Map, SpecScale |
| `ca4d361925073e3a` | generic | 371 | fc05_colour_lambert | amphiseum 268, metropia 34, sebenco_climb 34, +4 | `dlc2.psarc:data/art/published/dlc1/environments/amphiseum/materials/fc05_colour_lambert.rcsmaterial` | ColourDiffuse |
| `b950c855d51aef9a` | generic | 333 | defuse_occulsion_vert_col_tint | modesto_heights 322, tech_de_ra 6, talons_junction 5 | `dlc2.psarc:data/art/published/dlc1/environments/modesto_heights/materials/defuse_occulsion_vert_col_tint.rcsmaterial` | Spec_power, colour_TINT |
