# M6 authored lighting: no hardware light slot found enabled

`AmbientLight`/`DirectionalLight` are registered, `PointLight` genuinely is not, and none of the mesh-draw functions read so far enables a GE light slot. The runtime direction vector recovered along the way is useful evidence for [`lighting.md`](../docs/formats/lighting.md)'s open rotation-versus-translation question. Recorded so a future pass does not reopen them: `FUN_08a6b5bc`/`FUN_08a6b5c8` are World/Scene container class stubs, not lighting. Full account on [`psp-pulse-eu/lighting.md`](../docs/ghidra/functions/psp-pulse-eu/lighting.md).

## Open

- No GE light slot has been found enabled by any mesh-draw function read so far
- `lighting.md`'s open rotation-versus-translation question for the runtime direction vector is still unresolved

## Next Steps

- Read the remaining mesh-draw functions to check whether any enables a GE light slot (the search so far is not exhaustive)
