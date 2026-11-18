# oag-trackfile

This project's own custom-track format: a glTF 2.0 binary holding every number
a race reads, plus a TOML manifest beside it. Disc exporters and foreign importers
both target its `Track` model, so the simulation never learns where a track came
from.

No gameplay crate may depend on it. See
[ADR-0063](../../docs/architecture/adr/0063-the-own-track-format-is-glb-plus-a-toml-manifest.md).
