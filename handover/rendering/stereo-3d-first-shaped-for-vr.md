---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-10-13
summary: "Build the multi-view camera and anaglyph output first, then side-by-side packing; five VR-ready design constraints"
size: XL (45)
difficulty: hard (40)
---

# Stereo 3D first, shaped so VR plugs in later

2026-10-13, queued by the maintainer. No work started. HD/Fury shipped
stereoscopic 3D for 3D TVs, and Omega shipped PlayStation VR. Neither title's
3D or VR code path has been looked at in this project; nothing here is
recovered from an executable.

**Decision (maintainer, 2026-10-13): one stereo layer serves both, built in two
steps.** Stereo 3D lands first. VR is not built alongside it, but the stereo
lane is shaped so VR is purely additive. Doing both at once would hold stereo
back behind OpenXR frame timing, which only a headset can verify. Both modes
are render-side only, so they backport to every title, Pulse and Pure
included, behind the opt-in rule for later-title features on earlier titles.

## How the two differ

| | HD/Fury stereo 3D (PS3, 3D TV) | Omega VR (PS4, PSVR) |
| --- | --- | --- |
| View source | The game camera, offset per eye | Head-tracked eye poses |
| Projection | Symmetric, with a convergence distance | Asymmetric (off-axis) per eye, from the runtime |
| Output | Frame-packed stereo to the TV | Per-eye images, lens warp and reprojection by the headset's runtime |
| Rate | The display's | 90-120 Hz, low latency |
| Comfort | Depth strength, HUD depth | Horizon lock against roll, cockpit view, HUD in the world |

That table is general knowledge of how the two shipped, not a measurement.

## Open

The stereo lane takes these design constraints, so VR is additive:

1. **The camera returns a list of views, each with its own view and projection
   matrix**, not one camera plus an eye offset. A hard-coded symmetric frustum
   shifted sideways would force VR to rewrite the camera.
2. **The view source is swappable**: mono, stereo around the game camera, and
   later OpenXR poses. The renderer consumes views and knows no source.
3. **Each view renders into its own target or texture layer, and output is a
   separate step.** Stereo packs side-by-side, top-bottom or anaglyph in
   `oag-present`; VR hands the views to the runtime. wgpu multiview (one draw
   broadcast to two layers) is the efficient path where the backend has it.
4. **The HUD is drawn at a depth**, not stamped flat on the final image. Stereo
   needs it anyway (a flat HUD sits at screen depth and fights the scene), and
   it is the step toward VR's in-world HUD panel.
5. **The render rate is separate from the 60 Hz tick.** Stereo does not need
   this. VR's 90-120 Hz needs the renderer to draw poses between ticks, and
   since 2026-10-13 it does: a renderer sets `Race::set_draw_alpha` from its
   own frame clock before it draws and the race's camera and model accessors
   blend the last two ticks (`docs/rendering/between-ticks.md`). A headset's
   predicted display time maps onto the same alpha. What is still stepped is
   `drawing-between-ticks-what-is-still-stepped.md` in this directory.

Out of the stereo lane: OpenXR, head tracking, lens warp and reprojection
(the runtime owns those), comfort options and a cockpit view.

The simulation sees none of this: no state hash moves, and `oag-render` and
`oag-present` are outside the determinism scan.

Per-title presentation work, for both modes:

- **HUD depth or panel**, per title's HUD layout.
- **Comfort camera** (horizon lock): a camera choice, labelled chosen, not
  measured, unless recovered from Omega.
- **Cockpit view**: HD/Fury and Omega ship cockpit models; Pulse and Pure have
  none, so VR there is the chase camera or a bare nose camera.

Fidelity reference, unread: HD's 3D settings (eye separation, convergence, HUD
depth) and Omega's VR settings (comfort options, cockpit, HUD placement) live
in their executables and front-end data.

## Next Steps

1. Multi-view camera plus anaglyph output, honouring constraints 1-4. About a
   day; testable on any screen.
2. Side-by-side and top-bottom packing for 3D TVs and a settings entry, with a
   `string_id` for each new menu row. An hour or two on top of step 1.
3. RE pass on HD/Fury's stereo settings and Omega's VR settings, so the
   defaults are the originals' rather than chosen. Half a day each for a census.
4. VR through OpenXR: render interpolation (constraint 5, landed), runtime submission,
   comfort camera. A week or more; needs a headset to verify.
