# oag-formats

Asset containers and what every decoder needs before it starts: identify a blob,
pull it out of its archive (WAD, PSARC), decompress it, undo the console's byte
layout. What the bytes then mean is the next crate's job (`oag-vex`, `oag-texture`,
`oag-tables` and the other format crates, per
[ADR-0050](../../docs/architecture/adr/0050-format-crates-split-by-format-family.md)).

A decoder reads a version word from the file itself, so this crate must never depend
on a title package. It is in the determinism scan.
