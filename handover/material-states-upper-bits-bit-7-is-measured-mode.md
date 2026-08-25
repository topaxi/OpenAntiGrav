# `Material::state`'s upper bits: bit 7 is measured, mode 2 is not, and neither is wired

2026-08-25, branch `worktree-handover-texcoord-atoc-bit7`. Split off from a
texcoord-orientation thread that resolved cleanly and closed; this is what was
left. Written up in [rcsmodel.md](../docs/formats/rcsmodel.md), "Bit 7 tracks
the texture, not the material name". **Bit 7** was thought to be carried by
exactly the disc's `*_atoc`-suffixed material family - it is not a
material-name family at all, but a per-instance flag: `cf_tree`'s ten resolved
instances on `amphiseum` split cleanly on it, the one sampling a plain bark
texture (`ol_treemonzabrownshowbark_c.gtf`) is the one without the bit, and
the other nine sampling `_atoc`-suffixed textures all carry it. Disc-wide,
across all 16 circuits, 113 of 145 bit-7 records sample an `_atoc` texture and
113 of 115 `_atoc`-textured records carry the bit; every exception is
concentrated in one over-reused material name (`lambert`, 32 records, opaque
gridwork/rail/window textures with no `_atoc` counterpart) or one repeated
material (`wes_billboardholographicscanlines`, 2 records). **Confidence 84**
for that correlation - the same evidence class as the low-two-bits
transparency-mode reading beside it on the page. **What the bit specifically
selects is a separate, weaker claim at confidence 55**: every `_atoc` texture
on the disc is foliage, a distant crowd, a traffic sprite or on-screen text -
textbook alpha-to-coverage content, used to cut out detail without
depth-sorting transparency - but that is an inference from content class, not
a reading of the executable, and no RSX register write has been tied to the
bit. **`Transparency::Mode2`** is unrelated and untouched by this pass: its
six material names (`nr_crowd_bustle`, `jd_alphalambert_test`, `fence_alpha`,
`emissive_alpha_heathaze_test`, `cf_alpha4glow`,
`uv_anim_diffuse_alpha_emissive`) suggest a cutout, and that reading is
refuted by the picture - routing it through the existing alpha-test pipeline
erases the crowd entirely, since `crowd_avatars_22x4.gtf`'s alpha runs 0..255
at a mean of 120 and a 0.5 threshold discards most of it. **HD's render path
can actually use alpha-to-coverage**, checked while at it:
`mesh_render::build`'s `sample_count` is 1 outside a race and the configured
`anti_aliasing` MSAA count inside one, so wiring it would be a real, if
conditional, effect rather than definitionally unwireable. New diagnostic:
`hd_atoc_check` (`crates/render/examples/hd_atoc_check.rs`), alongside
`hd_state_census`'s now-uncapped bit-7 name list and `hd_params`'s
state/bit-7 printout.

## Open

- Bit 7's specific GPU meaning ("alpha-to-coverage" versus something else) is
  a confidence-55 content-class inference, not confirmed against the
  executable.
- `Transparency::Mode2`'s split from mode 1 is unrecovered; the obvious name
  hypothesis (a 0.5 alpha cutout) is refuted by the crowd-alpha experiment.
- Neither bit 7 nor `Transparency::Mode2` is wired into `oag_render`.

## Next Steps

- Tie bit 7 to an RSX method write in `ps3-hdfury-eu`'s Ghidra database, to
  move "alpha-to-coverage" from a content-class inference to a decompiled
  read. The database is a fresh import with no material/RSX-state functions
  named yet, so this starts cold - from method-constant pattern matching in
  the render layer's own address range, not from an existing name to search
  for. See `docs/reverse-engineering/toolchain.md` and the per-function TOC
  trap on PS3 Ghidra work.
- Dump the fragment programs for `Transparency::Mode2`'s six material names
  with `scripts/ps3-microcode.py` and look for a kill/comparison at a
  threshold that isn't 0.5. This is microcode work, unlike bit 7 above: a
  comparison-and-kill is visible in the fragment program itself, where
  fixed-function GPU state like alpha-to-coverage is not.
- Once either of the above resolves what a bit or mode actually selects,
  wire it into `oag_render`. For bit 7 specifically: outside a race
  `sample_count` is 1, so `alpha_to_coverage_enabled` would visibly do
  nothing there - don't mistake that for a wiring bug, check it during a race
  with MSAA on instead.
