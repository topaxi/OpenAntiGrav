# A *slot-resolved* record's own field layout

Narrowed 2026-08-12, and the row used to overstate what was missing. The container, the `SYSP` slot table, the names **and the whole emitter tree** are decoded, parsed and corroborated on a second binary (35 PSP files / 76 emitters, 41 PS2 / 90, one unmodified parser). What is still unread is what a *slot* resolves to - 43% are developer texture paths, the rest small float runs - which is what stands between `oag_render::psys`'s procedural falloff and the authored sprites.

2026-08-25: the one specific lead this thread named (xref the preload path-string addresses) was tried and is a clean static dead end - see `docs/formats/pob.md`'s "The preload happens as a batch" section for the full evidence trail (every `ghidra-mcp` xref mechanism, overlay-mirror addresses included, plus a whole-program `lui`/`_lui` sweep, all zero hits). It was also a stale lead: it was written before the 2026-08-01 pass that located the record interpreter through a different route (`ShipCollisionFx_Trigger` → ... → `particle-system.md`), so finding the preload's caller would no longer be the way into the slot-resolved record question even if it had worked. A second, previously-unrecorded 24-entry weapon preload table (`0x08a7c3e4`-`0x08a7c6ac`) turned up in the same search and is now written down in `pob.md`, but adds no new effect name and no new mechanism.

## Open

- What a *slot* resolves to is still unread - 43% are developer texture paths, the rest small float runs.
- This gap is what stands between `oag_render::psys`'s procedural falloff and the authored sprites.
- The preload's own caller is also still unfound, by any static xref mechanism tried so far - not the same question as the slot-record layout, but the same corner of the format.
- Whether this binary's per-function saved-register pool pointers (e.g. `FUN_089156a0`'s own `lui s3,0x2b` / `lw a0,-0x1db8(s3)`) are how such tables get reached is unexamined - a live PPSSPP session found no `$gp`-named register or `$gp`-relative displacement anywhere in this program's disassembly, so that specific sibling mechanism is ruled out, but this one isn't yet checked either way.

## Next Steps

- A live capture is the honest next step now that static search is exhausted: breakpoint on `FUN_088f8e38`'s entry (the fixup loop) during the "loading the race" step with `scripts/ppsspp_debugger.py`, then walk the return-address stack to find what called down into the loader for `WO_SHIP_COLL_SPARK_DAMAGE` et al. This reaches the preload caller, not the slot-record layout directly, but is the most concrete open lead in this corner.
- Separately, and more directly on-topic: decode a resolved slot's own bytes. `WO_SHIP_COLL_SPARK_DAMAGE`'s root emitter has a known texture slot (`0x4c4`) and known non-string float-run targets (e.g. `+0xce0`: `0.98, 0.98, 0.98` after a 12-byte string block) - annotating what those trailing floats mean for a few more resolved targets across the corpus, by hand from the file bytes plus a live read where one is available, does not need the preload caller at all.
