---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-10-29
summary: "Track art leftovers: Bake operator, sun on HD hosts, moving-sun gaps, glb sky images, track-file respawn, re-timing"
size: L (60)
difficulty: medium (50)
categories: [tooling, rendering]
---

# Track art: what is left after landings 1, 2a, 2b, motion, pads, own sky and Zone

Everything ADR-0076 decided up to decision 16 is on main (2026-10-29): the art section, linear
`COLOR_0`, layers and alpha refs, the PSP hull rig from the file's light, the Blender export and
import, own sky (`[art.skybox]`), keyframed motion, pad plates (`oag_pad`), `[art] draw_collision`,
the host pin and `titles` filter with auto-remix, the computed racing line, and Zone on a track
file. `docs/formats/track-file.md` and `docs/tools/blender.md` are the reference; this thread
holds only what is open. Hillside, the demonstration track, lives in the mod-examples repository
(`hillside/`, builder `hillside/hillside.py`), mounted at `assets/mods/examples`.

## Open

- **ADR-0076 is still Proposed.** The maintainer accepts it (or edits it) now that every landing
  it names is in.
- **Bake operator.** The vertex-colour bake lives in Hillside's builder script only.
- **Sun on HD-family hosts.** The sun is not fed to `Light::authored`; ships on an HD, Omega or
  2048 host are not lit by a track file's sun.
- **Moving sun gaps.** The hull rig keeps the sun's time-zero pose (built once at load,
  `loaded.light.hull`); a flare's own `direction` does not turn with the sun; `oag_shadow` is not
  honoured by moving casters; a disc export writes `Anim Transform` geometry at time zero.
- **Own sky:** images inside the glb have no texture form in 1.x; a Blender world Sky Texture or
  Environment Texture is not mapped (the add-on's panel colours and images are).
- **Track-file respawn.** With the autopilot on a track file, HD, Pure and PS2 Pulse respawn once at
  about tick 44, in Time Trial as well as Zone (seen by the Zone lane on the oval fixture). Not
  investigated.
- **Racing line.** The computed line stays in the middle 40% of the AI corridor because a line
  nearer the bounds left the opponents' speed plan unlearnable (fails at inset 0.1 and 0.2, works
  from 0.25). The cause (lane room) is likely, not traced. The gain is small (0.4 to 1.2% AI lap).
- **Re-timing.** NIGHT RUN's Hillside Sprint medals (62/68/78 s) were set before Hillside took the
  computed racing line; the Octagon Zone medals (24/18/12 zones) are chosen, not measured (the
  autopilot never wrecks before zone 626).
- **Pad plates on Hillside**: weapon plates hiding in Time Trial was proven on the fixture by
  screenshot, not on Hillside (the frames taken there show no pad).
- **rcsmodel art** for HD, 2048 and Omega disc exports: *checked, applies, not wired*.
- A foreign track with **no** art lost its hull-lights line in the loader report (one-line fix in
  `pulse_psp::light_hulls`' caller).
- A lit art mesh's colour is a gamma float, not a byte; the add-on exports `oag_lit` meshes
  knowing that.
- **Test budget under load.** On the merge gate of 2026-10-29 (four sessions building), the suite
  read 1290 s and `autopilot_original_ground_truth::an_opponent_flies_its_autopilot_by_class_and_difficulty`
  360 s (ceiling 300 s); the PS2 track-file proof slice grew to 146 s with pad plates. Re-measure
  on an idle machine before splitting anything.

## Next Steps

1. Maintainer: accept ADR-0076.
2. Bake operator in the add-on: lift Hillside's bake (albedo stand-in materials, collision hidden,
   `DIFFUSE` direct + indirect to `VERTEX_COLORS`, report vertices per area, warn on an animated
   sun). Half a day.
3. The hull rig follows a turning sun: re-derive directional 0 per frame from `SceneryFx::sun`.
   Two hours.
4. HD-family host: the sun into `Light::authored`, with a screenshot check that nothing else
   relights. Half a day.
5. Trace the track-file respawn at tick 44 on HD, Pure and PS2 Pulse (the oval fixture, Time
   Trial). An hour to find.
6. Re-time Hillside Sprint with the autopilot on the current Hillside (mod-examples).
