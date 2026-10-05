# 2048's cruising engine is a gearbox beside the crossfade

2026-10-05, lane `v2048-engine`. The crossfade half is wired ([2048-xfx.md](../../docs/formats/2048-xfx.md)): a 2048 craft sounds on the grid and at low speed. At cruising speed the crossfade's layers are silent by their own curves, and the original plays the rest as a **gearbox of named cues**.

## Open

- **The gearbox is not wired.** `ShipTeam_UpdateEngineCrossfade` (`0x812cd154`, [crossfader.md](../../docs/ghidra/functions/vita-2048-eu-v104/crossfader.md)) plays, out of `Ship_NGP.bnk`, `~Ship<N>_<Low|Mid|High|vHigh><Acc|Dec>_<1..3>` and the same with `_G_`, plus `Ship_Gear`, `Ship_Gear_up`, `INT_gearchange`, `Land_antigrav`, `flaps_l`/`flaps_r`, `xpress`/`xrelease`, `c_AfterburnerIg`. `N` is 1 `Feisar2048`, 2 `Qirex2048`, 3 `Piranha2048`, 4 `Auricom2048`, 5 `AG_Systems2048`. 251 of the 254 names built that way resolve by the FNV-1 hash (`Bank::cue_named`). The decompile of the state machine (`ship[+0x76b0..]`, `+0x1d92`/`+0x1d93` the two voices' fade, `+0x1da0..+0x1da2` the pedal and its last value, `+0x1dac` the gear 1 to 5, `+0x76ad` a one-shot latch) is read: an Acc cue plays to its end, then the gear goes up; the pedal released plays a Dec; the speed band picks Low/Mid/High/vHigh; the `_G_` voice cross-fades with the plain one by the pedal's change.
- **Channel 4's source and the class word.** `ship[+0x6098]` (what `70 * x` feeds channel 4) and the writer of `DAT_8153fd18` (the 1.25/1.15/0.95/0.8 factor on channel 0) are unfound. The port feeds the throttle and holds the factor at 1.0, both labelled chosen.
- **2048's other cues read HD's banks.** `SOUND_BANKS` points `weapons`, `speech` and `ship` at `weapons.bnk`, `speech.bnk` and `shipHD.bnk` ("chosen" by an earlier lane). With the hash lookup a 2048 race's report went from 42 cues not loading to 6, so those HD-named cues now play. `Weapons_NGP.bnk`, `Speech_NGP.bnk` and `Ship_NGP.bnk` are the 2048-named banks, with names like `NGP_PICKUP_Missile`.
- **Banks that do not parse**: `speech_zone_NGP.bnk`, `speech_fe_NGP.bnk`, `Speech_NGP.bnk` (the waveform section is not whole PS-ADPCM blocks) and the eight `env_*` banks (section framing).
- **Which circuit takes which ship bank.** `FUN_8121bce2`'s selection field `+400` picks `Ship_NGP.bnk` or `Ship_NGP_Zone.bnk`; this port picks by Zone or not.
- Nothing is heard against a Vita3K capture.

## Next Steps

1. Port the gearbox: a `GearBox` beside `oag_sound::sfx::xfade::Craft`, fed the pedal and `speed_field`, loading the 251 cues through `load_named_cue`. Prove it with a ground-truth test over the real package and a `--dump-audio` run.
2. Capture channel 4 and the class word on Vita3K (a watchpoint on `ship+0x6098`) to turn the two chosen terms into measured ones.
3. Move the 2048 `SoundBanks` to the NGP banks and their cue names.
