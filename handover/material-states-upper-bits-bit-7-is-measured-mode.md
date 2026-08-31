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

**2026-08-31: the microcode route for `Transparency::Mode2` is closed, empty.**
All 86 fragment blocks across its six materials were disassembled with
`scripts/ps3-microcode.py` and checked for a `KIL` or a mode-2-specific
comparison. `KIL` never appears anywhere in the six; the only comparisons
present (`SLT` against `zoneColourTint`, an `ADD_SAT ..., -0.5` range-compress)
are ordinary shared boilerplate, confirmed present identically in a mode-0
control (`track_surface`) and a mode-1 control (`glass_texture`). Full writeup
at [rcsmodel.md](../docs/formats/rcsmodel.md), "Mode 2's six materials carry
no fragment-program kill". **This isn't a narrower miss - it retires the
whole microcode avenue for mode 2**: a programmable discard would show
directly in the fragment program, and it doesn't, on any of the six. Mode 2
is fixed-function RSX state, the same class of mechanism bit 7 is suspected
of selecting, and the two open items below now point at exactly the same
missing evidence.

## Open

- Bit 7's specific GPU meaning ("alpha-to-coverage" versus something else) is
  a confidence-55 content-class inference, not confirmed against the
  executable.
- `Transparency::Mode2`'s split from mode 1 is unrecovered, and is now known
  **not** to be a shader-side cutout (the microcode has no kill in any of its
  six materials - see above). What's left is fixed-function GPU state.
- Neither bit 7 nor `Transparency::Mode2` is wired into `oag_render`.

## Next Steps

- Tie bit 7 **and** mode 2 to RSX method writes in `ps3-hdfury-eu`'s Ghidra
  database - they now need the identical evidence, so one pass through the
  render layer's method-constant patterns can look for both at once. The
  database is a fresh import with no material/RSX-state functions named yet,
  so this starts cold - from method-constant pattern matching in the render
  layer's own address range, not from an existing name to search for. See
  `docs/reverse-engineering/toolchain.md` and the per-function TOC trap on PS3
  Ghidra work. This is the only remaining path for either: the microcode
  route for mode 2 is retired (above), and bit 7 was never a microcode
  question to begin with.
- Once either resolves what a bit or mode actually selects, wire it into
  `oag_render`. For bit 7 specifically: outside a race `sample_count` is 1,
  so `alpha_to_coverage_enabled` would visibly do nothing there - don't
  mistake that for a wiring bug, check it during a race with MSAA on
  instead.
