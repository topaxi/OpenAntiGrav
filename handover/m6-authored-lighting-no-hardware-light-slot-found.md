# M6 authored lighting: no hardware light slot found enabled

`AmbientLight`/`DirectionalLight` are registered, `PointLight` genuinely is not, and none of the mesh-draw functions read so far enables a GE light slot. The runtime direction vector recovered along the way is useful evidence for [`lighting.md`](../docs/formats/lighting.md)'s open rotation-versus-translation question. Recorded so a future pass does not reopen them: `FUN_08a6b5bc`/`FUN_08a6b5c8` are World/Scene container class stubs, not lighting. A fourth pass (2026-08-31) closed off `World_LoadTrack_q`'s own body as `DirectionalLight`'s consumer - disassembly-verified, not just decompiled - and found its name is doubtful (it's called as a generic small object constructor, not handed "the world"); it also hit, and cross-referenced, an already-documented `jal` relocation wart on a second binary. Full account on [`psp-pulse-eu/lighting.md`](../docs/ghidra/functions/psp-pulse-eu/lighting.md).

## Open

- No GE light slot has been found enabled by any mesh-draw function read so far
- `lighting.md`'s open rotation-versus-translation question for the runtime direction vector is still unresolved
- `DirectionalLight`'s final consumer (a reader of `world+0x40`/`+0x7c` after `World_CollectMarkerLists` populates them) is still unfound after four passes; `World_LoadTrack_q`'s own body is now ruled out specifically, narrowing the remaining candidates to its caller `FUN_08886950` (read in full but on an unconfirmed pointer identity - see the page) or a later per-frame function
- `World_LoadTrack_q`'s own name is doubtful: it's called from `FUN_08886950` as a generic 60-byte object constructor, not literally handed "the world" - no replacement is anywhere near the 50-confidence floor to act on, so it stays unrenamed and open

## Next Steps

- Confirm whether `FUN_08886950`'s `param_1` and the object `World_LoadTrack_q`'s tag-lookup resolves to (the one passed into `World_CollectMarkerLists`) are the same object; if so, a per-frame function elsewhere is the only remaining lead, if not, re-check `FUN_08886950`'s own body against the right pointer
- The original mesh-draw-function sweep this thread started with is not the bottleneck anymore - the `Mesh_ApplyMaterialLighting`/`Mesh_ApplyShinemapReflection_q` pair and all seven sibling display-list builders are already read in full per the page; further work belongs on the `World_LoadTrack_q`/`FUN_08886950` lead above, not a fresh mesh-draw pass
