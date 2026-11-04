# Omega's 3D audio: the patch's sink, the VR object panner, and what positions a sound

What patch 1.03 added to Omega's sound and what a flat-screen player actually hears from it, read statically off
both executables and the shipped banks. Lane `omega-3d-audio-read`, 2026-11-04. Function addresses are base
(`/omega/eboot-ps4-omega-eu.bin`) with the patch twin beside them; globals and everything marked patch are
`/omega/eboot-ps4-omega-eu-v107.bin`. The held patch is 1.07, so "patch" below means "somewhere up to 1.07": nothing here separates 1.03 from later. The event and bank path
is [wwise-sound.md](wwise-sound.md); the shipped bank container is
[wwise.md](../../../formats/wwise.md). Nothing on this page ran live (shadPS4 has no memory readout), so every score
is capped at 84.

**The headline, four lines:**

1. **The 3D-audio path is VR's.** The object panner sits on 12 `VR_*` busses only, and sound reaches them through
   `Game_VR` switch containers, cockpit camera states and VR-only events, all driven by the headset flag. A flat-screen
   race reaches no audio object. (72)
2. **On a flat screen the patch changes the output device, not the positioning.** Everything goes to a 7.1 bed
   through `libSceAudio3d`, despite the device's name; a sound is still panned by Wwise relative to the camera. (80)
3. **Omega has no rear low-pass in code, and no HRTF of its own.** `RearOcclusion` is parsed and never read on both
   builds. Its front/back design is authored Wwise data: pass-by switches on the built-in Azimuth. (78)
4. **HD and 2048 do have a rear low-pass**, a measured law on two binaries that agree, and ours does not play it.
   That is the ready-to-wire result: [Cross-title](#cross-title). (80)

## The audio device (confidence 80)

`Game_Main` (`0x0163b920`; patch `0x0167a020`) builds Wwise's init settings once. Against the base, the patch adds
two things at the same site (patch `0x0167f33d`..`0x0167f35f`):

```text
sceSysmoduleLoadModule(0xa7)                         ; patch only, gate on the whole init
settings.+0x24 = 512                                 ; both; samples per frame by the SDK's layout
settings.+0x38 = hash("SCE_Audio3d_Ambisonic_Output"); patch only, the audio device share set
```

Default settings are zero at `+0x30`..`+0x38` (patch `FUN_01090080`; its `FUN_010bfae0` clears the 12 bytes). The field
reading (`+0x30` panning rule, `+0x34` channel config, `+0x38` device) is the SDK's shape from memory, fitted by the one
write; so the panning rule stays at its default, not set by the game (confidence 60, no second write found). Among
strings matching `audio3d`, `hrtf`, `binaural`, `headphone`, `surround`, `3d audio`, `spatial`, `listener` or `libSceAudio`, the
share set's name is the only one the patch adds; `AK::SceAudio3dSink` and every `sceAudio3d*` import are in the base
too, linked and unused, because the base `Init.bnk` has no device.

`Init.bnk`, base against patch (HIRC walked to the last byte in both):

| | Base (`data00`) | Patch (`data08`) |
| --- | --- | --- |
| Generator version | 118 | 118 |
| HIRC object type 23 (audio device) | 0 | 1 |
| Plugins `0x950006`, `0x960003`, `0x970007` (`SceAudio3d`) | absent | present |
| Busses (type 8) | 96 | 111 |
| Listing's Audio Devices row | none | `SCE_Audio3d_Ambisonic_Output`, type "SCE Audio3d Bed Output" |

The device's record: plugin `0x970007`, 8 parameter bytes, `00000000`, `0x0063f108`. The sink's init `SceAudio3dSink_Init`
(`0x011f3130`; patch `0x011eeb10`) reads the second word as an `AkChannelConfig`, falls back to the same `0x0063f108` when
its low byte is zero, and takes the ambisonic branch only when `(config & 0xf00) == 0x200`. Low byte 8, type field 1, mask
`0x63f`: **eight channels, standard, 7.1**. So the bed is 7.1 and the ambisonic objects are not reserved. The word
"Ambisonic" in the share set's name does not describe what runs. Which sink runs for the bed path is pinned: `Pump`
thread `FUN_011eef30` (patch), `sceAudio3dBedWrite` at patch `0x011efb45`.

The `brb*` code (`brbDriverBase::Initialize3D`, a second `sceAudio3d` client at patch `0x011c4130`) is in both builds
and reached only through a table (`0x018f5aac`); no caller from game code was found. Hypothesis, confidence 40: linked
and idle. Not read further.

**System side, not read:** what the PS4 does with a bed after `sceAudio3dBedWrite` (the downmix to stereo, the virtual
surround of a headset). That is firmware, not this executable.

## The object panner, VR only (confidence 72)

`SceAudio3dMixer_Execute` (`0x011f1a80`; patch `0x011ed460`) is the plugin `0x950006`, a Wwise mixer attached to a bus. Per
input voice, when the context says it is 3D (`+0x40` true and `+0x50 == 1`, vtable calls, meaning unread) and an object
could be reserved (patch `0x011ed1a0`):

```text
pick the voice's emitter-listener pair with the smallest distance
position = (sin(az) * cos(el), sin(el), cos(az) * cos(el)) * distance * scale   ; attribute 3, 12 bytes
spread   = min(spread * 0.0628318..., 2 * pi)                                    ; attribute 4
priority = 100 - voice priority                                                  ; attribute 2
pcm                                                                              ; attribute 1
sceAudio3dObjectSetAttributes(port, object, 4, attributes)
```

Attribute ids are read off this call (sizes 12 and 4 fit position and spread; gain, id 5, is written once at 1.0 in
`0x011ed1a0`, patch). `scale` is `0x01a72894`, set by a parameter call, unread. Any other voice is mixed to the bed.

The 12 `SCE Audio3d Object Panner (Custom)` records (`06009500`, parameter byte 0) are attached to 12 busses, and to no
other, by exact id over `Init.bnk`:

`VR_WPN_Explo_Blast_PLAYER_3D_OBJ`, `VR_WPN_Explo_Blast_NPC_3D_OBJ`, `VR_WPN_PowerUps_PLAYER`,
`VR_WPN_Projectiles_NPC_3D_OBJ`, `VR_WPN_Projectiles_PLAYER_3D_OBJ`, `VR_WPN_Special_Quake_3D_OBJ`,
`VR_UI_Ingame_HUD_OBJ`, `VR_SHIPS_Passbys_3D_OBJ`, `VR_SHIPS_Gameplay_PLAYER_OBJ`,
`VR_SHIPS_COCKPIT_INTERNAL_AMB_PC_3D_OBJ`, `VR_SHIPS_Engines_NPC_3D_OBJ`, `VR_SHIPS_Engines_PLAYER`.

**How a sound gets there.** `SoundManager_Update` (`0x01731b50`; patch `0x01780190`, from `0x017801e7`) posts
`Set_State__GAME_VR__YES` or `__No` when the headset flag `0x02212d2b` changes, and the headset code (patch `0x017fa100`,
`sceHmd*`) owns that flag. A byte scan of the patch's `Ship_NGP` and `Weapons_NGP` (ancestors followed through container
child lists) finds **46 nodes naming a panner bus**:

| Bank | Nodes | Under a `Game_VR` switch container | The rest |
| --- | --- | --- | --- |
| `Weapons_NGP` | 34 | 34 (33 such containers) | none |
| `Ship_NGP` | 12 | 4 (3 such containers) | 8: every one under `Gameplay_CAMERA_VR` containers and reached by `_Cockpit_Amb_Loop_*`, `VR_Ship_*` or `Flip_VR__*` events |

That the eight stay silent flat rests on `Gameplay_CAMERA_VR` sitting at `None` outside VR and on those events' names; no post
of them was traced. **The switch-to-child tables were not read**, only which containers sit above which node, which is what holds
this at 72. Which sounds: weapons, explosions, power-ups, the Quake, engines, pass-bys, cockpit ambience, the in-race HUD's. In
which space: listener-relative, per voice. It goes no further because VR is not planned.

## What positions a sound on a flat screen (confidence 78)

Every game object is a Wwise emitter; `SoundGroup_Create` registers it with a listener mask of 1 or 3.
`SoundManager_Update` runs once per frame:

| Step | Evidence | Confidence |
| --- | --- | --- |
| `Wwise_SetListenerPosition(pos, 0)` from a view matrix: `0x01a8a100` or `0x01a8a210` on `0x020e8a48`, or the manager's own at `+0x180` when its byte `+0x2b829` is set | `0x01092890` (patch `0x01090c90`), which orthogonalises top against front and normalises both. "View" is inferred from the use | 72 |
| With the headset up, that matrix is multiplied by the head pose first | branch on `0x02212d2b`, patch `FUN_01780190` | 72 |
| Listener 1 when `0x020e8a48` is set, from a second matrix at `0x01a8a140`; two listeners fits split screen, not traced | the same call with index 1 | 70 |
| Each active group: `Wwise_SetPosition(object, pose)` | `0x01092460` (patch `0x01090860`) | 80 |
| Each active group: `CONTROLLERID_DOPPLER` is `distance delta / dt`, a game-set RTPC; a distance gate and an `acosf` cone check | patch `FUN_01780190` | 78 |
| Aux sends `VERB_Interior`, `VERB_Exterior` | `SetGameObjectAuxSendValues` shape, patch `0x01092990` | 70 |

Boot registers `EXTERNAL_MUSIC` on listener mask 4 and `GLOBAL_2D` on mask 1 (`Wwise_RegisterGameObj`, `0x010976a0`,
same arguments in both builds). No listener is positioned by index 2 anywhere (two callers, indices 0 and 1), so music's
listener never moves. Panning into the bed is Wwise's own speaker panner: **SDK internals, not read**.

### Front and back are authored, in Wwise

`Init.bnk`'s `STMG` chunk ties switch groups to parameters; the table parses to the byte, and the parameter ids resolve by
FNV-1 to the names below. `Azimuth` and `Distance` are Wwise's built-ins (no string in the game sets them; `Azimuth`
appears in no `CONTROLLERID_*`). So the engine computes them from the listener above:

| Switch group | Parameter | Points |
| --- | --- | --- |
| `Passby_FrontBack`, `Passby_NPC_FrontBack` | Azimuth | -180 BehindPlayer, -90 InFrontOfPlayer, 90 BehindPlayer, 180 BehindPlayer |
| `Passby_AzimuthThreshold` | Azimuth | -180 BehindPlayer, -120 TriggerZone, -30 InFrontOfPlayer, 0 InFrontOfPlayer, 30 TriggerZone, 120 BehindPlayer |
| `Passby_DistanceThreshold` | Distance | 0 Inside, 35 Outside, 5000 Outside |
| `Passby_DistanceThreshold_Large` | Distance | 0 Inside, 80 Outside |
| `Passby_DistanceThreshold_AmbShips` | Distance | 0 Inside, 100 Outside |
| `Passby_Doppler_Switch_MEDSPEED`, `_HIGHSPEED` | `CONTROLLERID_DOPPLER` | -300 Approaching, 8 and 40 GoingAway |
| `Passby_NPC_Doppler_Switch_MEDSPEED`, `_HIGHSPEED` | `CONTROLLERID_DOPPLER` | -300 Approaching, 10 and 90 GoingAway |
| `Passby_DopplerThreshold`, `_AmbShips` | `CONTROLLERID_DOPPLER` | -300 FastEnough, -75 Off, 90 Off |

`Ship_NGP.bnk` selects on them through switch containers (14 on the two FrontBack groups, 8 on the two Gate groups), so
**a pass-by sample is chosen by front or back, not filtered**. Beside the switches, the built-in Azimuth is
named directly in 6 `Ship_NGP` objects (3 switch, 1 actor-mixer, 2 layer) and Distance in 6 `Weapons_NGP` objects
(a byte scan for the id in each HIRC body: what the binding does on each was not read). `Passby_Gate`'s parameter
`3301119579` and `Passby_NPC_Gate`'s `2916017607` are named in no listing here; both run -48 Open, -30 Close, a shape
of a level in dB, so a hypothesis only: a sidechain.

Those containers are **data**, shared by both modes: VR adds a bus, not the pass-by logic. Omega's race cues beyond the
countdown are silent in the port ([wwise-sound.md](wwise-sound.md)), so none of this plays yet.

## `RearOcclusion` is dead on Omega (confidence 78)

`GlobalAudioConfig.xml` on both archives ships `<RearOcclusion max_frequency="10000.0" />`, with a comment calling it
"the low-pass filter that is applied to sounds, when the sound is behind the listener". `AudioConfig_Load` (`0x01738090`;
patch `0x01786a60`) parses it with `sscanf` into `0x019467a0` (patch `0x019b7fa0`). **Each of those is a single xref in its
build, the parser's**, and the words at `+4` and `+8` have none; the word at `-4` is an unrelated
option global (`0x0194679c`, read and written by the front end). The HD law below reads through a getter, and Omega has no twin:
no `CONTROLLERID`, no RTPC and no filter takes it. A struct-relative load cannot show in an xref list, which is why this is
78 and not more; two builds agreeing is what it stands on.

The cue request's `+0x0c` 23999.0 ([wwise-sound.md](wwise-sound.md), "Emitter groups") is the same number as the HD law's
open cutoff below, which reads as a leftover field; that the request's value does anything on Omega is unread.

## The Speaker Setup option is not surround and not HRTF (confidence 82)

Both builds. `SoundMastering_ApplyPreset` (`0x01777a10`; patch `0x017c91e0`) maps the option to a state and a
`sceAudioOutMastering` block: 0 Normal (`OPT_SURROUND`), 1 TVMedium (`OPT_TV_MED`), 2 TVSmall (`OPT_TV_SMALL`), 3
Headphones (`OPT_HEADPHONES`), 4 NightMode (`OPT_NIGHT_MODE`). It posts `Set_State__SULPHA__<name>` through
`Sound_PostSetState` (`0x017783e0`; patch `0x017c9b60`); the name is hashed, so `Headphones` and the listing's `HeadPhones` meet.
Each case fills a mastering parameter block, and SULPHA is a set of custom states over the mix (EQ and dynamics
effects in `Init.bnk`, `EQ_Sulpha_Presets`, `RTPC_SULPHA_COMPRESSION_RATIO`). **"Headphones" is a dynamics preset.** Boot
applies 1 (TVMedium). Since the option is in the base, it is no part of the 3D-audio patch.

`Sound_SyncSurroundState` (`0x01779510`; patch `0x017c9e30`) reads the port's channel count (`sceAudioOutGetPortState`,
byte 2): 6 posts `SURROUND_STATE__SURR_5_1`, 8 `SURR_7_1`, anything else `Stereo`, when the count changes or the caller
forces it. The sound update calls it per frame (patch `0x01780578`). That is the whole of Omega's output detection, and it names a channel count, not headphones.
What the music's `INSTRU_Front`, `_Back`, `_Surround`, `_Spread` stems do with it is the state's business, unread.

## Cross-title

**HD: checked, differs. The brief's "HD's Wwise path" does not exist.** HD plays `SBlk` through Scream
([wwise-sound.md](wwise-sound.md#cross-title)). It positions with its own law, and the law has a real front/back.

**2048 against Omega (the standing check): checked, differs.** Same XML, same value, opposite outcome: 2048 uses it,
Omega parses and drops it. Evidence below; Omega's half above.

### The rear low-pass (HD and 2048, confidence 80)

Two binaries, same law, one decompile each:

```text
; HD FUN_002fd858 (angle in f1, voice, out struct, last argument smooths); 2048 FUN_81261d14
t  = clamp((radius - distance) / radius, 0, 1)
e  = 1 - (1 - t) * (1 - t)
fc = near_cutoff * e + far_cutoff * (1 - e)
if 90 < angle < 270:
    r    = min(angle - 90, 270 - angle) * 0.05          ; 20 degrees to full, clamped to 1
    rear = r * RearOcclusion.max_frequency + (1 - r) * 23999.0
    fc   = min(fc, rear)
if smooth: fc = 0.05 * fc + 0.95 * previous             ; per update
out = min(fc, 23999.0)
```

- HD reads the value through `FUN_00304600` (one caller, this function; global `0x008c273c`), 2048 through `FUN_81265430`
  (one caller, its twin). `AudioConfig_Load` (HD `0x0030d0b0`) picks `<Stereo>` or `<Surround>` on the output layout byte
  `+0xe8`, as the mix does ([audio-mix.md](../ps3-hdfury-eu/audio-mix.md)).
- **Values, from the shipped `GlobalAudioConfig.xml`:** HD 14,000 Hz stereo and 24,000 Hz surround (above 23,999, so off); 2048 10,000 Hz.
  No track `AudioConfig.xml` overrides it (no `RearOcclusion` in any of 81 extracted copies across the titles).
- **DistanceFilter is open on circuits:** near and far are both 23,999 on 16 of 18 HD files, 28 of 30 2048, so only the
  rear term acts. The rest carry `near_cutoff="0.0" far_cutoff="999.0" low_pass="0"`, not traced.
- **The angle is a full circle:** HD `FUN_002fc5f0` transforms the emitter into listener space, normalises it, and at
  `0x002fc8c8` takes `acosf(dot(dir, (0,0,1)))` in degrees, then `360 - angle` when the cross product's component is
  negative, wrapped to 0..360. So unlike Pulse's right-axis angle, it keeps front from back; the disassembly was hand-decoded
  because the decompiler stops at `0x002fc7f8` (bad instruction data). That `+Z` of this space is the camera's forward is
  inferred from the XML comment and the 90..270 band, not traced to the camera. The 2048 angle source was not read.
- **Pulse and Pure:** no `RearOcclusion` string in either executable. The Pulse law's angle loses front from back
  ([positional-audio.md](../psp-pulse-usa/positional-audio.md)).

**One link unread.** HD's `FUN_002fdc00` passes the 16-byte result to `FUN_003146e8`, which copies it to every sub-voice at
`+0x48`..`+0x54` and sets the dirty byte `+2`. The final consumer, and so the filter's order and shape, is unread. Recipe:
in the HD program, list `lfs` of `+0x50` on the sub-voice array `*(DAT_008b5064 + 0x18)` in `0x00314000`..`0x00316000`
and follow to the `cellMSCore` call. Until then a one-pole is a choice. A second path (`param_2[0x14] != 1`) passes the
same struct in a request; it agrees in shape.

## SPATIAL AUDIO against the originals

`audio.spatial_audio` values, against what each title does. **Ours differs**, and the differences are named:

| Value | Ours | Omega | HD, 2048 | Pulse, Pure |
| --- | --- | --- | --- | --- |
| `original` | Pulse's equal-power pan, no front/back, on every title | **Differs.** 7.1 bed, Wwise's panner, authored pass-by swaps; what a stereo system output does with the bed is firmware, unread | **Differs.** Pan plus the rear low-pass above, which ours does not play | **Same** (bit-identical) |
| `rear_cue` | 2.5 kHz one-pole, gain 0.8, linear over the whole rear half; **chosen** | No equivalent. `RearOcclusion` parsed, unused | Closest thing we have, far stronger than HD's 14 kHz or 2048's 10 kHz, over a 20 degree ramp, with no gain drop | No equivalent |
| `headphones` | SADIE II HRTF per voice; **ours** | **No equivalent outside VR.** In VR, objects to the PS4 system. "Headphones" in its menu is a dynamics preset | None | None |
| `auto` | Headphones on a detected headphone route, else rear cue; **ours** | No such thing: channel count to `SURROUND_STATE` only, never headphones | None | None |

So: the label `original` is accurate on Pulse and Pure only. On Omega, `original` cannot be Omega's until its Wwise pan and
pass-by data play; today it is a stand-in, and it inherits Pulse's law by lineage with no front/back at all, where Omega has some.
On HD and 2048 it should carry the rear low-pass, which is open as a ready-to-wire thread under `handover/audio/`. The `rear_cue` code comment in `oag_audio::mixer` says "the original cannot tell
front from back", which is true of Pulse alone; that comment is left for the same wiring, as this lane changes no `.rs`.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x01777a10` | `SoundMastering_ApplyPreset` | 82 | five cases, `sceAudioOutMasteringSetParam`, the state post; option and boot callers |
| `0x01779510` | `Sound_SyncSurroundState` | 82 | port state channel byte to three state names |
| `0x017783e0` | `Sound_PostSetState` | 75 | builds and posts the name, `SetState fail reason` printf; many callers |
| `0x01731b50` | `SoundManager_Update` | 80 | listeners, emitters, Doppler, sends, per frame |
| `0x01738090` | `AudioConfig_Load` | 85 | the XML path and every element name |
| `0x011f3130` | `SceAudio3dSink_Init` | 82 | `sceAudio3dPortOpen`, channel config, its own failure strings, thread name |
| `0x011f1a80` | `SceAudio3dMixer_Execute` | 75 | object attributes from azimuth, elevation and distance per voice |
| `0x010976a0` | `Wwise_RegisterGameObj` | 80 | id, name, mask; `EXTERNAL_MUSIC`, `GLOBAL_2D`, `SoundGroup_Create` |
| `0x01092890` | `Wwise_SetListenerPosition` | 80 | queues message `0x10` with an orthonormal frame and an index (read on the patch twin) |
| `0x01092460` | `Wwise_SetPosition` | 78 | object id, pose; the emitter loop; `GLOBAL_2D` |

`FUN_011eef30` (patch, the sink's thread) and the `brb*` code stay unnamed.

### Patch (1.07) addresses

| Base | Name | Patch |
| --- | --- | --- |
| `0x0163b920` | `Game_Main` | `0x0167a020` |
| `0x01777a10` | `SoundMastering_ApplyPreset` | `0x017c91e0` |
| `0x01779510` | `Sound_SyncSurroundState` | `0x017c9e30` |
| `0x017783e0` | `Sound_PostSetState` | `0x017c9b60` |
| `0x01731b50` | `SoundManager_Update` | `0x01780190` |
| `0x01738090` | `AudioConfig_Load` | `0x01786a60` |
| `0x011f3130` | `SceAudio3dSink_Init` | `0x011eeb10` |
| `0x011f1a80` | `SceAudio3dMixer_Execute` | `0x011ed460` |
| `0x010976a0` | `Wwise_RegisterGameObj` | `0x010959f0` |
| `0x01092890` | `Wwise_SetListenerPosition` | `0x01090c90` |
| `0x01092460` | `Wwise_SetPosition` | `0x01090860` |

## Open

* **HD's filter consumer**, recipe above: the only unread value the wiring needs.
* **The `Game_VR` switch-to-child tables** and the object panner's scale `0x01a72894`: only worth reading if VR opens.
* **Live:** every claim here is static. A `wipeout_pc` run could log the sink's `sceAudio3dPortOpen` and the absence of
  object reservations in a flat race (`sceAudio3dObjectReserve`, patch import `0x01801050`), which would lift the VR-only
  claim past 84. That launch is the maintainer's.
