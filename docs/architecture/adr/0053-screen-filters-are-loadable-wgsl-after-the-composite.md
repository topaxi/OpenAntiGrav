# ADR-0053: Screen filters are loadable WGSL files, run after the UI composites and before the grade

## Status

Accepted, 2026-09-16. Extends
[ADR-0036](0036-ui-composites-at-presentation-resolution.md) (where the UI
lands) and [ADR-0041](0041-one-row-for-what-resolves-the-frame.md) (what the
`RECONSTRUCTION` row is *not* for). Relies on
[ADR-0020](0020-gamma-authoritative-colour-space.md) for the space the filter
reads and writes in.

## Context

The titles this engine runs were seen through two very different displays: a
PSP's 480x272 LCD - three panels across the 1000, 2000 and 3000, with
different persistence, contrast and subpixel structure - and, for the PS2
port, an interlaced consumer television over a composite cable. A modern
monitor shows neither, and a player who remembers the 3000's combing during
motion or the smear of a 2000 is remembering the display as much as the game.

Emulators answer this with a post-processing shader system: PPSSPP ships a
`shaders.ini` of `.fsh` files with a previous-frame binding, RetroArch has its
`.slangp` presets. Both are user-extensible, and the popular collections built
on them are largely GPL - which this MIT/Apache-2.0 tree cannot vendor, and
which is the same constraint `oag_post::fxaa`'s module doc records for
NVIDIA's FXAA.

Three questions had to be settled before anything was written, because each
one is a contract with a player's own files:

1. **Where in the frame the pass runs.** The scene-resolution ladder in
   `oag_game::upscale::Framebuffer::resolve_scene` sees the 3D scene alone -
   ADR-0036 and ADR-0038 put the HUD, the menus and the front end after it.
   A display showed all of those. The grade (brightness, gamma) in
   `composite` sees everything, but it is a calibration of the *player's*
   monitor and belongs last.
2. **What a filter is.** A format of this project's own, or a loader for an
   existing one.
3. **Where the setting lives.** `[display]` is "applied after the game has
   finished drawing", which fits; `[render_profiles.<title>]` is "what a value
   means depends on the title", which also fits, and only one can hold it.

## Decision

**A screen filter is one fullscreen WGSL pass over the presentation target,
run inside `Framebuffer::composite` after the UI has composited and before the
grade.** The order is presentation target, filter, grade, surface. The filter
*is* the picture - a different display drawn onto this one - and the grade is
the monitor. The performance overlay, drawn onto the surface after `composite`,
stays outside the filter, on purpose: it is an instrument.

**A filter is a `.wgsl` file in this project's own contract**, not a RetroArch
or PPSSPP loader: a `//!` TOML header naming it and up to sixteen tunables,
and a body defining `fn screen_filter(uv, pixel) -> vec3<f32>` against a
prelude this crate owns - the frame, a bilinear and a nearest sampler, **last
frame's own output**, the output size, the title's authored grid
(`Space::size`), a frame counter, a clock and the tunables. The prelude is
`crates/post/src/screen.wgsl`; the contract in full is
[screen-filters.md](../../rendering/screen-filters.md). Built-ins live in
`assets/shaders/screen/` and are compiled into the binary; a player's own live
in `<config dir>/oag/shaders/`, a file with a built-in's stem replaces it, and
the directory is polled once a second so a saved edit shows without a restart.
A player's file is validated with naga before the device sees it and the
pipeline is built inside an error scope, so a broken file is a logged line
and an unfiltered frame rather than a crash.

**The setting is per title, in `[render_profiles.<title>]`**, as
`screen_filter` (a string: `off` or a preset id) and `screen_filter_strength`
(a percentage mixed in by the prelude, so it means the same for every
preset). The `[display]` reading lost on the second question: the right
filter for a PSP disc is its LCD, for the PS2 port an interlaced television,
and for Wipeout HD nothing, so one shared value would be wrong on two of the
three titles a player switches between. The menu rows sit on the GRAPHICS
page beside SHADOWS, which is in the same table for the same reason.

**The built-ins are this project's own text**, written from the techniques -
gaussian beam profiles, phosphor masks, barrel warp, field alternation with
phosphor decay, YIQ chroma bandwidth, LCD grids and persistence - and not
transliterated from any published shader. The user directory is where a
GPL-licensed shader goes, ported by whoever wants it, and the contract is
deliberately simple enough that porting one is an afternoon.

## Alternatives considered

**A rung on the reconstruction ladder.** Rejected: it would filter the scene
and leave the HUD and the menus untouched, which for a display simulation is
wrong on its face, and ADR-0041 settled that the `RECONSTRUCTION` row answers
one question - what resolves the frame - which this is not.

**After the grade, or inside it.** Rejected: a monitor calibration applied to
a simulated display's output would be graded twice from the player's point of
view - the simulated panel's black level and the monitor's brightness are
different knobs - and putting the filter inside the grade's shader would make
a player's file responsible for reproducing the grade.

**A RetroArch `.slangp` or PPSSPP `.fsh` loader.** Rejected for now. Both are
GLSL, which naga can read, but the value is in the preset systems around them
- multi-pass chains, per-pass scale and filter settings, parameter UIs - and a
partial loader that ran one pass of a chain would look like support and not
be it. The contract here keeps the door open: a converter from either format
to this one is a script, not an architectural change.

**One shared `[display] screen_filter`.** Rejected on the per-title argument
above. A maintainer directive on 2026-09-16 settled it the same way.

**A parameter row per tunable in the menu.** Deferred rather than rejected.
The header's `min`/`max`/`step` exist for it and are validated, but a menu
that regenerates rows from a file's header is a change to `menu.toml`'s
static-definition model that this ADR does not need to make. Tuning today is
copying the built-in out and editing the defaults.

## Consequences

- **One more presentation-sized pass and two presentation-sized textures
  while a preset is selected**, and nothing at all when it is `off` -
  `set_screen_filter(None)` drops the pass. The two textures are the history
  ping-pong that persistence and field alternation read from.
- **A player's file is code this project runs.** naga validation and the
  error scope contain what a bad file can do to the process; they do not
  contain what a slow one can do to the frame rate, and a preset with a
  hundred taps per pixel will show as one. That is the same footing every
  shader-modding system is on and the doc says so.
- **The uniform is an ABI.** `Screen`'s field order and the five bindings are
  what every player's file is written against; changing them breaks those
  files. Additions go on the end of the struct, behind the sixteen tunables.
- **A still capture of a temporal preset shows one frame of it.**
  `--presented --screenshot` composites once with a black history, so
  `crt-interlaced` shows one field and a persistence preset shows the first
  frame's fade-in. The doc states it; the alternative - a warm-up loop in the
  capture - would make a comparison capture depend on how many frames it ran.
- **`native_size` is read off whatever is in hand.** The menu shell's grid
  when the run has menus, the race HUD's on the `--race` route, and the PSP's
  when a run has neither - a `--race` on a source whose HUD did not load is
  the one case that reads a PS2 disc as 272 rows.
- **Nothing here is calibrated.** The PSP presets are a plausible model of
  what contemporary reports describe, with their starting values taken from a
  design discussion rather than from a measured panel; the doc records the
  provenance and the confidence, and no preset claims to be a measurement.
