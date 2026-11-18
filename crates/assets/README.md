# oag-assets

Runtime asset access: open a WAD or a PSARC archive by path or straight out of
a disc image, then find an entry by index, name or name hash.

It sits between the byte-level crates (`oag-disc`, `oag-formats`) and everything that
wants a file by name. It reads a title's vocabulary (`oag-title`) but holds no title's
data.
