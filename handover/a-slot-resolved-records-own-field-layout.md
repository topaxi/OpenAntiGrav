# A *slot-resolved* record's own field layout

Narrowed 2026-08-12, and the row used to overstate what was missing. The container, the `SYSP` slot table, the names **and the whole emitter tree** are decoded, parsed and corroborated on a second binary (35 PSP files / 76 emitters, 41 PS2 / 90, one unmodified parser). What is still unread is what a *slot* resolves to - 43% are developer texture paths, the rest small float runs - which is what stands between `oag_render::psys`'s procedural falloff and the authored sprites. The preload path-string addresses `0x08a886f8`-`0x08a8877c` are the untried xref target.

## Open

- What a *slot* resolves to is still unread - 43% are developer texture paths, the rest small float runs.
- This gap is what stands between `oag_render::psys`'s procedural falloff and the authored sprites.

## Next Steps

- Try the xref on the preload path-string addresses `0x08a886f8`-`0x08a8877c` (untried).
