# `/pulse/BOOT-psp-pulse-eu.BIN` - exact opcode-hash transfer from `psp-pulse-usa`

**2026-09-07, following [ADR-0048](../../../architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md).**
All four PSP databases were reimported with the Allegrex relocation patch the
same day (`docs/ghidra/workflow.md`, "The full set is relocated as of
2026-09-07"), which is what makes this pass possible: `get_bulk_function_hashes`
needed real function bodies to hash, and a program with zero relocations
applied hashes garbage.

## Method

**Exact match on a normalized opcode hash, not a fuzzy score.** Per the
maintainer's own instruction for this pass: "exact body-hash matches only,
unless you can defend something looser" - `bulk_fuzzy_match` was not used at
all here, on purpose. Four steps, every one of them a mechanical check rather
than a judgement call:

1. `get_bulk_function_hashes` pulled every function's normalized opcode hash
   from both `/pulse/BOOT-psp-pulse-usa.BIN` (10,711 functions) and
   `/pulse/BOOT-psp-pulse-eu.BIN` (10,672 functions), over the bridge's plain HTTP
   endpoint rather than one tool call per page, to keep the ~21,000-row pull
   out of the conversation itself.
2. For each of `psp-pulse-usa/names.tsv`'s named functions, the hash was
   required to be **unique on both sides** - appearing exactly once among all
   ~10,700 USA functions and exactly once among all ~10,700 EU functions -
   before being accepted as a candidate. A hash shared by more than one
   function on either side is the signature of a small, generic, structurally
   identical stub (the same collision this project's fuzzy sweeps have hit
   repeatedly - `Xml_AttributeNameIs`/`Xml_AttributeValueIs`/`Xml_ElementNameIs`
   all collapsing onto one EU target is the worked example in
   [corroboration.md](corroboration.md)), and is excluded rather than guessed
   at. 16 USA-named functions failed the source-side uniqueness check and 157
   EU targets were already named (carried over by an earlier fuzzy sweep, so
   nothing to add); neither counts against the candidates below.
3. **Every one of the 115 surviving candidates was individually verified with
   `diff_functions` before being trusted** - a normalized hash is strong
   evidence, but this project's own rule is that nothing gets renamed off a
   score alone, hash or fuzzy. All 115 came back with **zero added and zero
   removed instructions** - a fully identical body on both sides, the
   strongest match category this project's confidence rubric recognises. Where
   `similarity_score` still reads below 1.0 (as low as 0.7), the entire gap is
   `calls_only_in_a`/`calls_only_in_b` naming asymmetry: the callee is the same
   logical function on both sides, just already named (sometimes with a
   different confidence tier's `_q`) on one side and not yet on the other -
   the exact "callee-only difference, 100% instruction-identical body" category
   [corroboration.md](corroboration.md)'s deep sweep already established, never
   a real control-flow or instruction difference. No string divergence and no
   instruction-count mismatch turned up on any of the 115 either.
4. **A sample was additionally read by hand in the decompiler** (not just the
   raw diff) before applying anything, spanning small (`Entity_GetPosition`,
   4 instructions), medium (`Collision_BoxAgainstBox`, 700 instructions, a
   dense VFPU routine) and large (`Body_ResolveContactPair`, 503 instructions)
   functions across collision, rigid-body and GE state-cache subsystems - the
   same subsystems the fuzzy sweeps' own worked examples came from. All read
   as the same function, callee-naming asymmetry aside.

## Confidence

**EU confidence = USA confidence minus 5**, applied uniformly. The `-5` itself
is **chosen for this pass, not derived from the evidence** - the direction
(lighter than the fuzzy tiers) follows from the evidence, the exact number
does not. This is a
tighter (less punitive) discount than [corroboration.md](corroboration.md)'s
own fuzzy-based "clean" tier (USA confidence minus 8, for a diff ratio under
2%), because every row here has a diff ratio of exactly zero and a match
method (hash equality across two independent ~10,700-function corpora, both
directions unique) that is strictly stronger evidence than any fuzzy score.
The normal project rules still apply on top of the subtraction: below 70 gets
`_q` (7 of the 115 land there), below 50 is left un-renamed (none did here).
This pass's own floor case is `Quake_SpanIntensityAt` (USA 55, EU computes to
exactly 50 - the qualified-but-`_q` floor `apply-ghidra-names.py` accepts, not
the below-50 floor it refuses - so it is applied as `Quake_SpanIntensityAt_q`
rather than left un-renamed).

## Applied (115 rows)

| EU address | Name | EU confidence | USA address | USA confidence | USA evidence page |
| --- | --- | --- | --- | --- | --- |
| `0x08984570` | `__kernel_cosf` | 83 | `0x08984d14` | 88 | [particle-system.md](../psp-pulse-usa/particle-system.md) |
| `0x08984edc` | `__kernel_sinf` | 83 | `0x08985680` | 88 | [particle-system.md](../psp-pulse-usa/particle-system.md) |
| `0x0892d4d8` | `Airbrake_Update` | 85 | `0x0892d9fc` | 90 | [ship-parts.md](../psp-pulse-usa/ship-parts.md) |
| `0x088ff588` | `AnimTransform_EvalScale` | 85 | `0x088ffc08` | 90 | [anim-transform.md](../psp-pulse-usa/anim-transform.md) |
| `0x088fe6c4` | `AnimTransform_EvalTranslation` | 85 | `0x088fed44` | 90 | [anim-transform.md](../psp-pulse-usa/anim-transform.md) |
| `0x088fdd80` | `AnimTransform_Evaluate` | 85 | `0x088fe400` | 90 | [anim-transform.md](../psp-pulse-usa/anim-transform.md) |
| `0x088fda28` | `AnimTransform_Update` | 85 | `0x088fe0a8` | 90 | [anim-transform.md](../psp-pulse-usa/anim-transform.md) |
| `0x0893a414` | `Audio_SetSfxFadeTarget` | 80 | `0x0893a938` | 85 | [audio-levels.md](../psp-pulse-usa/audio-levels.md) |
| `0x0893a74c` | `Audio_UpdateGroupVolumes` | 77 | `0x0893ac70` | 82 | [audio-levels.md](../psp-pulse-usa/audio-levels.md) |
| `0x08861290` | `Autopilot_Update` | 80 | `0x08861404` | 85 | [autopilot.md](../psp-pulse-usa/autopilot.md) |
| `0x08907350` | `Bloom_BuildBlurHorizontalList` | 80 | `0x089079d0` | 85 | [bloom.md](../psp-pulse-usa/bloom.md) |
| `0x08907240` | `Bloom_BuildBrightPassList` | 80 | `0x089078c0` | 85 | [bloom.md](../psp-pulse-usa/bloom.md) |
| `0x08907474` | `Bloom_BuildCompositeList` | 80 | `0x08907af4` | 85 | [bloom.md](../psp-pulse-usa/bloom.md) |
| `0x0884d8b0` | `Body_ClearAccumulators` | 80 | `0x0884da24` | 85 | [engine.md](../psp-pulse-usa/engine.md) |
| `0x0884edbc` | `Body_ResolveContactPair` | 75 | `0x0884ef30` | 80 | [contact-response.md](../psp-pulse-usa/contact-response.md) |
| `0x08864748` | `Cannon_Init` | 75 | `0x088648ec` | 80 | [cannon-quake-leachbeam.md](../psp-pulse-usa/cannon-quake-leachbeam.md) |
| `0x0881576c` | `Collision_AddBoxCollider` | 75 | `0x0881585c` | 80 | [collision.md](../psp-pulse-usa/collision.md) |
| `0x08816f3c` | `Collision_BoxAgainstBox` | 87 | `0x0881702c` | 92 | [collision.md](../psp-pulse-usa/collision.md) |
| `0x08817a2c` | `Collision_RaycastBox` | 80 | `0x08817b1c` | 85 | [collision.md](../psp-pulse-usa/collision.md) |
| `0x0897d88c` | `cosf` | 85 | `0x0897e030` | 90 | [particle-system.md](../psp-pulse-usa/particle-system.md) |
| `0x08859b60` | `Entity_GetPosition` | 85 | `0x08859cd4` | 90 | [contact-response.md](../psp-pulse-usa/contact-response.md) |
| `0x0891da24` | `Gfx_AcquireBatchStateList` | 80 | `0x0891df48` | 85 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0891e5c8` | `Gfx_AllocDrawScratch` | 77 | `0x0891eaec` | 82 | [particle-system.md](../psp-pulse-usa/particle-system.md) |
| `0x0891f36c` | `Gfx_BuildBatchStateList` | 85 | `0x0891f890` | 90 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0891db30` | `Gfx_CompileDirtyBatchStateLists` | 85 | `0x0891e054` | 90 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0891e3a4` | `Gfx_ResetTexTransform` | 83 | `0x0891e8c8` | 88 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08811724` | `Gu_AlphaFunc` | 87 | `0x08811814` | 92 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x088111d8` | `Gu_Color` | 83 | `0x088112c8` | 88 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x088116dc` | `Gu_ColorFunc` | 87 | `0x088117cc` | 92 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08810ef0` | `Gu_CopyImage` | 80 | `0x08810fe0` | 85 | [bloom.md](../psp-pulse-usa/bloom.md) |
| `0x08906a8c` | `Gu_DrawTiledSprites` | 77 | `0x0890710c` | 82 | [bloom.md](../psp-pulse-usa/bloom.md) |
| `0x088125bc` | `Gu_Material` | 85 | `0x088126ac` | 90 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x088117a8` | `Gu_PixelMask` | 83 | `0x08811898` | 88 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08810998` | `Gu_SpecularCoef` | 77 | `0x08810a88` | 82 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08811300` | `Gu_TexFilter` | 83 | `0x088113f0` | 88 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x088114c0` | `Gu_TexImage` | 83 | `0x088115b0` | 88 | [bloom.md](../psp-pulse-usa/bloom.md) |
| `0x088113c0` | `Gu_TexLevelMode` | 75 | `0x088114b0` | 80 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08811418` | `Gu_TexMapMode` | 83 | `0x08811508` | 88 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08811468` | `Gu_TexProjMapMode` | 80 | `0x08811558` | 85 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0883b208` | `Hud_ResolveLockTarget` | 75 | `0x0883b358` | 80 | [lock-sight.md](../psp-pulse-usa/lock-sight.md) |
| `0x0881f534` | `Hud_UpdateCountdownFade_q` | 65 | `0x0881f624` | 70 | [countdown-widgets.md](../psp-pulse-usa/countdown-widgets.md) |
| `0x08873b98` | `LeachBeam_InitLocked` | 77 | `0x08873d3c` | 82 | [cannon-quake-leachbeam.md](../psp-pulse-usa/cannon-quake-leachbeam.md) |
| `0x08872c04` | `LeachBeam_InitUnlocked` | 75 | `0x08872da8` | 80 | [cannon-quake-leachbeam.md](../psp-pulse-usa/cannon-quake-leachbeam.md) |
| `0x08859764` | `MagFloorFx_Hide` | 83 | `0x088598d8` | 88 | [magfloor-fx.md](../psp-pulse-usa/magfloor-fx.md) |
| `0x08859738` | `MagFloorFx_Show` | 83 | `0x088598ac` | 88 | [magfloor-fx.md](../psp-pulse-usa/magfloor-fx.md) |
| `0x0890d400` | `Mesh_BeginTransparentPass` | 80 | `0x0890d904` | 85 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0892e3cc` | `Mesh_BuildBatchDrawCommands` | 83 | `0x0892e8f0` | 88 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08911b14` | `Mesh_BuildModelDrawData` | 77 | `0x08912018` | 82 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0890e2a4` | `Mesh_CountBatchesPerList` | 83 | `0x0890e7a8` | 88 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0890d71c` | `Mesh_EndTransparentPass` | 77 | `0x0890dc20` | 82 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0890e3b0` | `Mesh_InitBatch` | 77 | `0x0890e8b4` | 82 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0890c348` | `Mesh_RegisterBatches` | 75 | `0x0890c84c` | 80 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0890dc5c` | `Mesh_UpdateTextureTransforms` | 85 | `0x0890e160` | 90 | [texture-animation.md](../psp-pulse-usa/texture-animation.md) |
| `0x0886886c` | `MissilePool_DestroyRemote` | 73 | `0x08868a10` | 78 | [missile.md](../psp-pulse-usa/missile.md) |
| `0x0893bce4` | `MusicPlayer_ComputeCallbackGain` | 77 | `0x0893c208` | 82 | [audio-levels.md](../psp-pulse-usa/audio-levels.md) |
| `0x08937b6c` | `MusicPlayer_ComputeStreamGain` | 80 | `0x08938090` | 85 | [audio-levels.md](../psp-pulse-usa/audio-levels.md) |
| `0x08910ff8` | `Node_SetAnimTimeTree` | 70 | `0x089114fc` | 75 | [texture-animation.md](../psp-pulse-usa/texture-animation.md) |
| `0x089167dc` | `ParticleSystem_DrawCappedStreak` | 83 | `0x08916d00` | 88 | [particle-system.md](../psp-pulse-usa/particle-system.md) |
| `0x089160ec` | `ParticleSystem_DrawRotatedSprite` | 83 | `0x08916610` | 88 | [particle-system.md](../psp-pulse-usa/particle-system.md) |
| `0x088f7334` | `ParticleSystem_InitParticleFields` | 83 | `0x088f79b4` | 88 | [particle-system.md](../psp-pulse-usa/particle-system.md) |
| `0x0885c558` | `Plasma_Update` | 80 | `0x0885c6cc` | 85 | [plasma.md](../psp-pulse-usa/plasma.md) |
| `0x0891c2bc` | `Quake_ProximityToCraft_q` | 55 | `0x0891c7e0` | 60 | [cannon-quake-leachbeam.md](../psp-pulse-usa/cannon-quake-leachbeam.md) |
| `0x0891c308` | `Quake_SpanIntensityAt_q` | 50 | `0x0891c82c` | 55 | [cannon-quake-leachbeam.md](../psp-pulse-usa/cannon-quake-leachbeam.md) |
| `0x08827080` | `Race_ResetCraftBoosts_q` | 55 | `0x088271b4` | 60 | [engine.md](../psp-pulse-usa/engine.md) |
| `0x08827320` | `RaceMode_SetSubstate` | 75 | `0x08827454` | 80 | [zone-mode.md](../psp-pulse-usa/zone-mode.md) |
| `0x0886dcbc` | `RocketPool_Update` | 77 | `0x0886de60` | 82 | [missile.md](../psp-pulse-usa/missile.md) |
| `0x0887a34c` | `SceneLight_BuildLightingList_q` | 63 | `0x0887a4f0` | 68 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0887a08c` | `SceneLight_CallLightingList` | 73 | `0x0887a230` | 78 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x0887a0c4` | `SceneLight_CallMaterialList` | 73 | `0x0887a268` | 78 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08879e74` | `SceneLight_InitLists_q` | 60 | `0x0887a018` | 65 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08879970` | `SceneLight_Rebuild` | 70 | `0x08879b14` | 75 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08991ac0` | `Scream_FindBankByName` | 75 | `0x08992264` | 80 | [sound.md](../psp-pulse-usa/sound.md) |
| `0x08991b60` | `Scream_FindSoundInBank` | 80 | `0x08992304` | 85 | [sound.md](../psp-pulse-usa/sound.md) |
| `0x0898d9d4` | `Scream_OpAlternate` | 83 | `0x0898e178` | 88 | [sound.md](../psp-pulse-usa/sound.md) |
| `0x0898debc` | `Scream_OpGoto` | 83 | `0x0898e660` | 88 | [sound.md](../psp-pulse-usa/sound.md) |
| `0x089952f8` | `Scream_PanVolumePair` | 89 | `0x08995a9c` | 94 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x0898f0c0` | `Scream_StartSound` | 80 | `0x0898f864` | 85 | [sound.md](../psp-pulse-usa/sound.md) |
| `0x0883e3f4` | `Shield_Activate` | 79 | `0x0883e544` | 84 | [shield-pickup.md](../psp-pulse-usa/shield-pickup.md) |
| `0x0883e470` | `Shield_Deactivate` | 75 | `0x0883e5c0` | 80 | [shield-pickup.md](../psp-pulse-usa/shield-pickup.md) |
| `0x0883efec` | `Ship_ApplyPendingWeaponDamage` | 77 | `0x0883f13c` | 82 | [shield-pickup.md](../psp-pulse-usa/shield-pickup.md) |
| `0x0883e4fc` | `Ship_State` | 79 | `0x0883e64c` | 84 | [shield.md](../psp-pulse-usa/shield.md) |
| `0x0885e04c` | `ShipShield_Deactivate` | 79 | `0x0885e1c0` | 84 | [shield-pickup.md](../psp-pulse-usa/shield-pickup.md) |
| `0x08877a38` | `Shuriken_Update` | 80 | `0x08877bdc` | 85 | [shuriken.md](../psp-pulse-usa/shuriken.md) |
| `0x0897db5c` | `sinf` | 85 | `0x0897e300` | 90 | [particle-system.md](../psp-pulse-usa/particle-system.md) |
| `0x08938d8c` | `Sound_Play` | 75 | `0x089392b0` | 80 | [sound.md](../psp-pulse-usa/sound.md) |
| `0x089396dc` | `SoundEmitter_ComputeVolumeAndAngle` | 85 | `0x08939c00` | 90 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x08938ca0` | `SoundEmitter_Init` | 85 | `0x089391c4` | 90 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x08938fb8` | `SoundEmitter_ServiceRequests` | 80 | `0x089394dc` | 85 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x089391fc` | `SoundEmitter_Update` | 85 | `0x08939720` | 90 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x08939660` | `SoundInstance_StillPlaying` | 75 | `0x08939b84` | 80 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x08939934` | `SoundInstance_UpdateSpatial` | 81 | `0x08939e58` | 86 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x0893a2c4` | `SoundManager_BuildVolumeCurve` | 89 | `0x0893a7e8` | 94 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x0893a0c8` | `SoundManager_LinkEmitter` | 80 | `0x0893a5ec` | 85 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x08939d8c` | `SoundManager_Update` | 83 | `0x0893a2b0` | 88 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x0893a358` | `SoundManager_VolumeCurve` | 89 | `0x0893a87c` | 94 | [positional-audio.md](../psp-pulse-usa/positional-audio.md) |
| `0x0897418c` | `strtod` | 77 | `0x08974930` | 82 | [xml-reader.md](../psp-pulse-usa/xml-reader.md) |
| `0x08926e34` | `TexAnim_CompileTransformList` | 85 | `0x08927358` | 90 | [texture-animation.md](../psp-pulse-usa/texture-animation.md) |
| `0x08926b10` | `TexAnim_EvalKeyframes` | 85 | `0x08927034` | 90 | [texture-animation.md](../psp-pulse-usa/texture-animation.md) |
| `0x08926ce0` | `TexAnim_UpdateTransform` | 85 | `0x08927204` | 90 | [texture-animation.md](../psp-pulse-usa/texture-animation.md) |
| `0x08928828` | `Texture_BuildBindList` | 80 | `0x08928d4c` | 85 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08911384` | `Texture_Downsample4bpp` | 79 | `0x08911888` | 84 | [ship-skin.md](../psp-pulse-usa/ship-skin.md) |
| `0x089114fc` | `Texture_UploadPaletted` | 81 | `0x08911a00` | 86 | [ship-skin.md](../psp-pulse-usa/ship-skin.md) |
| `0x08911e54` | `Vex_UpdateLightLists_q` | 60 | `0x08912358` | 65 | [mesh-draw.md](../psp-pulse-usa/mesh-draw.md) |
| `0x08900204` | `VexCamera_Submit` | 80 | `0x08900884` | 85 | [exhaust.md](../psp-pulse-usa/exhaust.md) |
| `0x089257cc` | `VexSound_SampleRadiusCurve` | 87 | `0x08925cf0` | 92 | [track-sound-emitters.md](../psp-pulse-usa/track-sound-emitters.md) |
| `0x08925728` | `VexSound_Update` | 80 | `0x08925c4c` | 85 | [track-sound-emitters.md](../psp-pulse-usa/track-sound-emitters.md) |
| `0x0886a6c4` | `Weapon_FirePlasma` | 83 | `0x0886a868` | 88 | [plasma.md](../psp-pulse-usa/plasma.md) |
| `0x08862bf8` | `Weapon_RequestFire` | 87 | `0x08862d9c` | 92 | [missile.md](../psp-pulse-usa/missile.md) |
| `0x088513dc` | `WeaponAi_Update` | 80 | `0x08851550` | 85 | [weapon-ai.md](../psp-pulse-usa/weapon-ai.md) |
| `0x08851bc8` | `WeaponAiStats_Reset` | 85 | `0x08851d3c` | 90 | [ai-stats.md](../psp-pulse-usa/ai-stats.md) |
| `0x08868418` | `WeaponPickup_ArmMissile` | 75 | `0x088685bc` | 80 | [missile.md](../psp-pulse-usa/missile.md) |
| `0x0886df54` | `WeaponPickup_ArmRocket` | 75 | `0x0886e0f8` | 80 | [missile.md](../psp-pulse-usa/missile.md) |
| `0x0895337c` | `Xml_AttributeAsFloatLibc` | 80 | `0x089538a4` | 85 | [xml-reader.md](../psp-pulse-usa/xml-reader.md) |
| `0x089534b0` | `Xml_AttributeAsOwnedString` | 80 | `0x089539d8` | 85 | [xml-reader.md](../psp-pulse-usa/xml-reader.md) |
| `0x0895323c` | `Xml_AttributeValueIs` | 83 | `0x08953764` | 88 | [xml-reader.md](../psp-pulse-usa/xml-reader.md) |

## What this does not close

This pass only ever attempted an exact hash match - it is one technique out of
several the maintainer laid out. Counted on **functions only** (this method
cannot touch a `data`-kind row at all): `psp-pulse-usa/names.tsv` named 581
functions and `psp-pulse-eu/names.tsv` named 284 of them before this pass, a
297-function gap. This page closes 115 of those, leaving **182**. Of the
remainder, 16 can never resolve by hash matching alone regardless of technique
- their USA hash is shared by more than one USA function, so hash equality
alone cannot tell which one an EU match belongs to - and the other 166 either
had no EU function anywhere sharing their USA hash, or that hash's EU target
was already named by an earlier fuzzy sweep and so was not a gap to begin
with. (The 355-row figure this project's ADR-0048 and its accompanying
handover thread cite is the **total row** gap - functions and data combined,
656 against 301 - a different, coarser count than the function-only one this
page works with.) The descending-threshold `bulk_fuzzy_match` sweep and the
structural/positional techniques (address adjacency, callee-diff
correspondence, offset interpolation) that closed the *previous* 213 rows in
[corroboration.md](corroboration.md) still apply to the 182 that remain, and
are the next step - the project's open-thread tracking names the exact method
and its documented failure modes for the remainder, and should be updated
with this pass's own count before anyone resumes it.
