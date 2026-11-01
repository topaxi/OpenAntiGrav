# ADR-0081: Screen layouts are files, and a track may carry its loading screen

## Status

**Accepted** (maintainer, 2026-11-01).

Builds on [ADR-0064](0064-project-toml-files-declare-a-toml-schema.md) (TOML schema headers),
[ADR-0063](0063-the-own-track-format-is-glb-plus-a-toml-manifest.md) and
[ADR-0076](0076-a-track-file-carries-its-art-as-an-optional-section.md) (the track manifest: edited in
place, nothing is released) and the asset layers
([`asset-layers.md`](../asset-layers.md)). Nothing is superseded.

## Context

HD's loading screen was drawn by about forty `const`s in `oag_game::loading`, chosen in the PSP's 480x272
grid with every face scaled against a PSP line height. The original draws it in its own 1920x1080 grid
with named fonts at stated scales, from code (its `Definition.xml` plugin was cut), so there was no file
to read and no file to override; the share of pixels differing from an RPCS3 frame was 10.5 %. The
split-screen HUD already had a TOML layout file of this project's (`split-hud.tosd`), read through the
asset layers, but it could place only a title's own named widgets.

## Decision

1. **One file format for the screens this project lays out from a file**, `schemas/v1/screen-layout.tosd`
   (the old `split-hud.tosd`, renamed and extended in v1, no version bump, no shim). A file's top-level
   `screen` says which screen it lays out: `split-hud` (named widgets, as before) or `loading`
   (primitives: images, text, labels, brackets, a dot grid, a clip, in a `grid` of the source's own pixels).
2. **A layout holds numbers and where they came from, nothing else.** A number read off the executable
   carries its address and confidence in a comment; one that was chosen says so. The drawing code knows
   what a source name means and which colour a role is, never a coordinate. An element a title cannot
   resolve is skipped and reported; a file that will not load draws the ground alone. Nothing is
   substituted.
3. **The file ships in the fallback layer and a mod overrides it by path**, as every other fallback file.
   No `oag-mod.toml` key.
4. **A track carries its own loading screen in its manifest**, an optional `[loading]` table (a layout
   file, a caption, a picture, three feature texts), files inside the track's own folder. Each thing falls
   back on its own to the title's; naming any feature key makes the track the feature.
5. **The renderer is given the layout's grid** and the Safe Area is applied in the layout's draw, so
   nothing converts between grids.
6. **`--loading-draw N` pins the feature** the original draws at random, so a pair names the draw its
   frame showed.

## Alternatives rejected

- *A second kind (`loading-layout.tosd`) beside `split-hud.tosd`.* Both are "where things go", share the
  anchor/aspect/modes selection and the asset-layer lookup, and a mod author learns one format.
- *Fractions of the screen for the loading layout.* The numbers come off the executable in pixels of a
  1920x1080 grid; fractions would put a rounding between each number and its address.
- *`oag-mod.toml [loading]`.* The path override already replaces a title's screen for every race, and a
  per-track screen belongs with the track.
- *Keeping the code layout as a fallback for a bad file.* That is the stand-in this project's rule
  forbids; a visible empty screen and a report line say the file is wrong.

## Consequences

- HD's race load went from 10.52 % to under 1 % of pixels over 40 grey against the same original frame
  (`hd-loading`); what is left is glyph rasterisation and the clip and bar phases.
- Pulse's and Pure's loading screens are still code in the PSP's grid (no file yet); the code path is the
  centred screen both have always drawn. Moving them is a file each, by the same format.
- The wrap rule of the paragraph (a word breaks when the line and the space after it pass the width) is
  inferred from one frame's two lines, not read from the executable.
- A track's layout can resolve only the strings the title's layout names; a literal `text` is the way to
  anything else.
- A format version bump will one day need a migration for track manifests; none is written now because
  nothing is released.
