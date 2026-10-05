# A *slot-resolved* record's own field layout

Narrowed 2026-08-12, and the row used to overstate what was missing. The container, the `SYSP` slot table, the names **and the whole emitter tree** are decoded, parsed and corroborated on a second binary (35 PSP files / 76 emitters, 41 PS2 / 90, one unmodified parser). What is still unread is what a *slot* resolves to - 43% are developer texture paths, the rest small float runs - which is what stands between `oag_fx::psys`'s procedural falloff and the authored sprites.

2026-08-25: the one specific lead this thread named (xref the preload path-string addresses) was tried and is a clean static dead end - see `docs/formats/pob.md`'s "The preload happens as a batch" section for the full evidence trail. It was also a stale lead: it was written before the 2026-08-01 pass that located the record interpreter through a different route (`ShipCollisionFx_Trigger` → ... → `particle-system.md`), so finding the preload's caller would no longer be the way into the slot-resolved record question even if it had worked. A second, previously-unrecorded 24-entry weapon preload table (`0x08a7c3e4`-`0x08a7c6ac`) turned up in the same search and is now written down in `pob.md`, but adds no new effect name and no new mechanism.

**Same day, follow-up: the preload's own caller is found, live, and it is not the way into the slot-resolved record question either.** A PPSSPP session under Xvfb (v1.20.4, `pulse-psp-usa.chd`) walked into a Time Trial and broke on `FUN_088f3540`'s entry during "loading the race" - reading `ra` at each hit (rather than searching for the call site's address, which is what the earlier static sweep tried and failed at) found the caller directly: `ShipCollisionFx_Preload` (`0x08924550`, renamed, confidence 85, see `pob.md`), which makes four hardcoded calls for the four known `.pob` files. Its own caller turned out to be generic per-entity vtable dispatch (`FUN_08908f98`), not a name-driven loader - so this whole chain is a fixed four-file preload with no slot resolution in it anywhere, confirming the staleness call above rather than opening a new path. Full writeup with all the live-capture numbers is in `docs/formats/pob.md`.

## Open

- What a *slot* resolves to is still unread - 43% are developer texture paths, the rest small float runs.
- This gap is what stands between `oag_fx::psys`'s procedural falloff and the authored sprites.

## Next Steps

- Decode a resolved slot's own bytes. `WO_SHIP_COLL_SPARK_DAMAGE`'s root emitter has a known texture slot (`0x4c4`) and known non-string float-run targets (e.g. `+0xce0`: `0.98, 0.98, 0.98` after a 12-byte string block) - annotating what those trailing floats mean for a few more resolved targets across the corpus, by hand from the file bytes plus a live read where one is available, is the direct next step.
- If a live session is wanted again: a working recipe for arming a breakpoint at a *specific function's entry* and reading `ra`/`a0` off `cpu.getAllRegs` at the hit - rather than searching for a call site's address - is proven out in this pass and sidesteps this binary's whole PIC-addressing xref-blindness class. Worth reaching for directly next time rather than re-deriving it.
