# ADR-0062: Unity content is a generic reader with a content-specific layer on top

## Status

Accepted. Relates to [ADR-0061](0061-two-asset-layers-around-the-disc.md), whose
override layer this decision deliberately does not use for tracks, and to
[ADR-0050](0050-format-crates-split-by-format-family.md), whose split by format
family it follows. Format layout and evidence: [`unity-bundles.md`](../../formats/unity-bundles.md).

## Context

A player can have Unity content on disk that is a natural fit for this engine:
Steam Workshop tracks and ships made for a Unity anti-gravity racer. A probe
showed the format side is unusually easy: UnityFS bundles whose SerializedFiles
carry full typetrees, so a track's data reads with no game assemblies at all.
Three questions followed.

1. **Where does the format knowledge live?** Unity's container, typetree and
   built-in classes are generic; what a `Section`, a `Tile` or a ship `Stats`
   block means is one game's schema.
2. **How does a foreign track enter the engine?** ADR-0061's override layer
   answers by *entry name inside a title's archives*, replacing one asset. A
   Unity track is not an entry of any title's archive: it is a whole circuit
   with its own source file, spline, collision and pads.
3. **How is the framing kept honest?** The feature is "Unity content support".
   It must not read as a reader of one game, and the generic crate must not be
   able to grow game knowledge by accident.

## Decision

1. **`oag-unity` is a generic Unity reader and nothing else.** UnityFS, the
   SerializedFile, the typetree-driven value reader, and the built-in classes
   (`GameObject`, `Transform`, `Mesh`, `Texture2D`, `Material`, and
   `MonoBehaviour` as a script pointer plus the uninterpreted value tree). No
   game's class names, no wrapper formats such as `.wtrk`, no tile or pad
   meanings, no title.
2. **A boundary the build enforces.** `scripts/check-dependency-rules.py` rule 4
   holds `oag-unity` to an **allowlist of workspace dependencies, empty today**,
   modelled on rule 3. A `cargo add` of any title package, game-specific crate or
   anything above it fails `just check-deps`; widening the list is a deliberate edit
   with a reason beside it. An allowlist rather than a denylist, so a crate created
   next year is forbidden to it by default.
3. **The game-specific layer is a separate crate that depends on `oag-unity`**
   (phase 2, not built): it reads the schema's `MonoBehaviour`s through the generic
   value tree, unwraps the `.wtrk` and `.csf` wrappers, and converts into this
   engine's track and ship models. The dependency never runs the other way.
4. **A foreign track is its own source beside `oag-source`**, selected by a
   player directory the way `data/dlc` is, reported in the loader log and never
   part of a hashed asset set. It is not an asset-override-layer entry: the override
   layer stays what ADR-0061 made it, a by-name replacement of one disc entry. A
   foreign track may *borrow* from the layers (HUD, sounds, skybox fallbacks).
5. **Dependencies.** LZ4 is hand-written (a block decoder is about forty lines and
   is fuzzed against damaged input). LZMA uses `lzma-rs-perf-exp`, already in the
   lockfile through `chd`, pinned `=0.2.1`. Block-format texture decoding is not in
   the crate: it returns the raw pixels and format, and a consumer decodes through
   `oag-texture` (whose `bcn` block decoders were made `pub` for this); `oag-unity`
   reaches `oag-texture` from its tests only.
6. **Untrusted input.** Every count and size is bounded by what the bytes can hold
   or a stated limit; every failure is a named `Error`. The reader decodes or
   refuses, and a typetree walk that does not consume exactly an object's bytes is
   an error.
7. **Legal posture.** The crate reads files a player already has, as the engine
   reads disc images. No Unity content, and no byte extracted from it, is ever
   committed; the cross-check against another implementation is a script plus an
   environment variable, not a checked-in table.

## Consequences

- Phase 2 is a new crate and a new source; neither touches `oag-unity`.
- A second Unity-based content family would reuse `oag-unity` unchanged.
- `oag-unity` is outside the determinism-bound crates: it decodes assets and
  feeds nothing that reaches a state hash.
- The framing rule is now mechanical for the part that can be: nothing in
  `oag-unity` can name a game without a dependency the gate refuses, and its docs
  name the test content as the source of files, not as the subject.
