# oag-ballisticng

BallisticNG's Workshop content, read on top of `oag-unity`: the `.wtrk` and
`.csf` wrappers and a track's sections, tiles and AI line as typed structs. It
reads. Mapping a track into this engine is a different decision and lives in
`oag-bng-import`.

Reaches `oag-unity`, `oag-texture` and `oag-bytes` only (rule 5). No gameplay
crate may depend on it.
