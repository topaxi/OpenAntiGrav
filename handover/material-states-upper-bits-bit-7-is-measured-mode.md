# `Material::state`'s upper bits: bit 7 is measured, mode 2 is now fully decoded

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
bit. **HD's render path can actually use alpha-to-coverage**, checked while at
it: `mesh_render::build`'s `sample_count` is 1 outside a race and the
configured `anti_aliasing` MSAA count inside one, so wiring it would be a
real, if conditional, effect rather than definitionally unwireable. New
diagnostic: `hd_atoc_check` (`crates/render/examples/hd_atoc_check.rs`),
alongside `hd_state_census`'s now-uncapped bit-7 name list and `hd_params`'s
state/bit-7 printout.

**2026-08-31: `Transparency::Mode2` is fully resolved and has moved to its own
thread.** It was thought unrelated to bit 7 - a separate, still-open question
about the state word's other bits. It turned out to need exactly the same
evidence bit 7 still does (a decompiled RSX register write), and finding
`Material_ApplyRenderState` answered mode 2 completely:
state bit 1 drives `NV4097_SET_ALPHA_TEST_ENABLE`, distinct from bit 0's
`NV4097_SET_BLEND_ENABLE`, with the comparison and reference read from two
newly-decoded material fields. Mode 2 is a plain `GL_GREATER`/`0.5` cutout,
disc-wide - see
[material-state.md](../docs/ghidra/functions/ps3-hdfury-eu/material-state.md)
and [rcsmodel.md](../docs/formats/rcsmodel.md#mode-2-is-a-plain-alpha-test-after-all-2026-08-31)
for the full evidence, including the correction of this thread's own earlier
"refuted by the picture" reading, which turned out to be a mean-vs-distribution
measurement error, not a fact about the mechanism. The remaining "wire it
into `oag_render`" work went to a thread of its own and landed the same day -
see below - and this file stays about bit 7, which is still genuinely
unresolved.

**2026-08-31, later: mode 2 is wired, and it handed this thread a symptom.**
`oag_render` draws `Transparency::Mode2` as a real cutout now (see
[rcsmodel.md](../docs/formats/rcsmodel.md#mode-2-is-wired-and-it-is-not-subtle-on-every-material-2026-08-31)),
and with it comes the classic alpha-test failure at minification. Measured on
`amphiseum` with `OAG_ALBEDO_ONLY=1` and only the change varying: one crowd
slot alone (`OAG_ONLY_SLOT=574`, radius 213) goes 44,313 non-background pixels
blended to 24,290 tested, which is the cutout working; all 25 crowd chunks at
once (radius 902) go **7,640 to 8** - the crowd is gone. The arithmetic that
predicts it is on that page: a minifying surface's filtered sample converges
on its texture's mean alpha, and `nr_crowd_bustle`'s is `0.4858`, just under
the `0.5` reference, so the test fails everywhere at once.

Alpha-to-coverage is the standard fix for exactly that, and **the `_atoc`
textures this thread is about are the crowd, the foliage and the traffic
sprites** - the same surfaces. So bit 7's content-class inference has a
rendering symptom pointing at it now as well as a naming coincidence. Do not
read that as evidence: it is a reason the inference is plausible, and the
register write is still what would settle it.

## Open

- Bit 7's specific GPU meaning ("alpha-to-coverage" versus something else) is
  a confidence-55 content-class inference, not confirmed against the
  executable.
- Bit 7 is not wired into `oag_render`.
- Whether the mode-2 cutout's minification artefact above is what the disc
  uses bit 7 to avoid. Suggestive, not established: nothing has read the bit's
  own register write, and the artefact would equally follow from this
  project's mip and sampler choices rather than from the original's.

## Next Steps

- Tie bit 7 to an RSX method write in `ps3-hdfury-eu`'s Ghidra database, to
  move "alpha-to-coverage" from a content-class inference to a decompiled
  read. `Material_ApplyRenderState` (`0x005d8f68`,
  [material-state.md](../docs/ghidra/functions/ps3-hdfury-eu/material-state.md))
  is now a real starting point rather than a cold search: it already reads
  `Material::state` bits 0 and 1 for blend/alpha-test, and calls
  `Rsx_SetMethod` twice more with registers `0x183c` and `0xa74`, gated on
  state bits 4 and 3 respectively - neither chased yet, and bit 3 is worth
  checking first since it is read in the same function as bit 7's neighbours,
  even though the specific `Rsx_SetMethod(..., 0xa74, ...)` call reads state
  bit 3, not bit 7 itself. Bit 7 proper still needs its own site found, most
  likely elsewhere in the render layer's method-constant patterns now that
  one real caller of the pattern (`Material_ApplyRenderState`) is known and
  can be searched from (its callers, siblings, and the same
  `0x005c0000`-`0x00650000` address span).
- Once resolved, wire it into `oag_render`. Outside a race `sample_count` is
  1, so `alpha_to_coverage_enabled` would visibly do nothing there - don't
  mistake that for a wiring bug, check it during a race with MSAA on
  instead.
