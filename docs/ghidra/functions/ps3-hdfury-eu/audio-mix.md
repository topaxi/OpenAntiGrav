# The authored mix: where HD's group volumes come from and where they land

2026-10-05. Lane `hd-mix-level`. The measurement and the law are on
[`hd-xfx.md`](../../../formats/hd-xfx.md) "The authored mix"; this page is the code. Addresses
below `0x32d5e0` resolve through the one TOC `0x008ad4d8`
([race-manager.md](race-manager.md) has the boundary). RPCS3 run, EU disc, Fury default walk
(Feisar, Talons Junction), GDB stub on the shipped EBOOT.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0030d0b0` | `AudioConfig_Load` | 85 |
| `0x000242a0` | `FrontEnd_ApplyAudioOptions` | 78 |
| `0x00307850` | `SoundSystem_SetMusicSlider` | 80 |
| `0x003084c0` | `SoundSystem_UpdateOutputGains` | 58 |

## `AudioConfig_Load` (`0x0030d0b0`)

Opens a path (`%s\AudioConfig.xml` at `0x00780500`, and `Data\sound\GlobalAudioConfig.xml` at
`0x007a4590`), parses the XML and walks its children by name: `AudioConfig` (root), then
`RearOcclusion`, `TunnelReverb`, `EdgeProximityReverb`, `EnvironmentalReverb`, `DistanceFilter`,
`EmitterRadii`, `Gamma`, `DuckingTemplates`, `DuckerEvents`, `ShieldAttenuation`, `VoiceRanges`
and **`ParameterMaps`**. Under `ParameterMaps` each of `ParameterMapFrontEnd`,
`PreRace`, `Countdown`, `RaceNormal`, `RaceCriticalEnergy`, `PlayerDead`, `PostRace`,
`DisplayResults`, `SplitScreen` and `DemoMode` is stored at `g_sound_system + 0x6a0 + 0xb8 *
state` (`0x6a0, 0x758, 0x810, 0x8c8, 0x980, 0xa38, 0xaf0, 0xba8, 0xc60, 0xd18` in the code).
A map is `transition_speed` (`+0`, clamped), then the layout branch: `g_sound_system + 0xe8 == 2`
reads the **`Stereo`** child and anything else the `Surround` one. Inside it `GroupVolumes` writes
`music` to `+4` and `user1..user12` to `+8 .. +0x34` (`user_d` format string at `0x008b4f1c`);
`Music` writes the two reverb send levels, `Scream` the LFE send, `MasterCompressor` and
`MasterEQ` the rest.

Evidence: the decompile above; **live**, 2026-10-05, `g_sound_system = 0x00b6cc90`: ten rows read
back at those offsets equal `GlobalAudioConfig.xml`'s Stereo rows (`PreRace` music `0.0`, user7
`0.4`; `RaceNormal` music `0.65`, user7 `0.68`, user8 `0.8`) and `+0xe8` read `2`. The copy read is
the `DATA00.PSARC` one: the live critical-energy row held music `0.45` and user7 `0.7`, which
`DATA01.PSARC`'s copy changes to `0.5` and `0.8`. `crates/game/tests/hd_mix_ground_truth.rs`
asserts the same from the disc.

## `SoundSystem_UpdateOutputGains_q` (`0x003084c0`)

The per-frame output stage, called from `SoundSystem_AudioThread` (`0x00309230`). Its music line
is `1.0 * (g_sound_system + 0x308) * (cfg + 0x1c)`: the live **music group** times the **music
slider** (`cfg = *0x008b4c58 = 0x008c2724`, set by `0x00307850`). It also runs the smoothing of
the sound system's own master envelopes (`+0x45c` chasing `+0x8`, steps from `cfg + 0x288/0x28c`),
which is not traced. The live group array `+0x308 .. +0x33c` (thirteen floats, music then
`user1..12`) chases the active state's row; one reading is a group falling from `1.1` to `0.001`
in about 1.5 s. Name at 58: the music line and the audio-thread caller are read, the rest is not.

## `FrontEnd_ApplyAudioOptions` (`0x000242a0`) and `SoundSystem_SetMusicSlider` (`0x00307850`)

The options screen's apply step. `"Music Volume"` (`0x0077a5b0`) and `"SFX Volume"` (`0x0077a5c0`)
are looked up by hash in the screen's value list and parsed as integers `0..100`
(`FUN_00676958(node, 0, 10)`); each is multiplied by the float at `0x008a5b74`, which reads
`0x3c23d70a` = **0.01**, and stored: music through `0x002faf20 -> 0x00307850` into
`cfg + 0x1c` (clamped `0..1`), SFX through `0x002ff060` into the SFX object's `+0xe0` (and `+0x120`
when it exceeds a threshold). `"Auto Volume"` (`0x0077a650`) sets one flag byte at `0x008c26ec` through
`0x002f6078`; its seventeen readers sit in the settings block at `0x002f6000 .. 0x002fb000` beside
`MusicTrackResume`, none read as a leveler. **Not traced**, and not modelled.

Evidence: live, the profile read `cfg + 0x1c = 0.8` and the SFX object's `+0xe0 = 0.8`, both the
authored default: `Slider "Music Volume" ... default="80%"` and `Slider "SFX Volume" ...
default="80%"` in `data/plugins/frontend/gui/additional_definition.xml` (and
`ingame_definition.xml`), `DATA0x.PSARC`; `Auto Volume` is `default="FE_ON"`.

## Not done

- The cue-to-group assignment other than the two measured (the engine is group 7, read off the
  player's own voices; the circuit's emitters are group 8, the only group audible with the rest
  zeroed). The `USER1..12` comment in the `DATA00` copy lists the intent and is not traced.
- The `MasterCompressor` (`ratio 0.2`, `threshold -6 dB`, split `100 Hz`) and the ducking
  templates, which change peaks and transients and are left unmodelled.
- Which state is live when: the port uses the grid and race rows only.
