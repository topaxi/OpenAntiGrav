# Omega's sound path: banks, event names and posting

How Omega's executable turns a sound request into Wwise media: which banks it loads and when, how it names an
event, and how a cue is posted. Lane `omega-wwise-sfx-re`, 2026-11-02. Read in Ghidra on both executables (base
`/omega/eboot-ps4-omega-eu.bin`, addresses below; the 1.07 patch `/omega/eboot-ps4-omega-eu-v107.bin`, the
build the game boots, addresses in the last table) and checked against the shipped banks with
`crates/formats/tests/wwise_ground_truth.rs` (`posted_events_resolve`). The container is
[`docs/formats/wwise.md`](../../../formats/wwise.md).

**The headline: an event's name is `<cue>[_PC]__<bank>`, hashed whole.** The code holds the cue (`_321_NGP`) and the
bank's slot, and builds the full name only at post time. That is why `_321_NGP` and `Go_NGP` looked undefined
([countdown-voice.md](../vita-2048-eu-v104/countdown-voice.md#omega)): their events are
`_321_NGP__Weapons_NGP` (243270081) and `Go_NGP__Weapons_NGP` (744083536), and both are defined in
`English(US)/Weapons_NGP.bnk`, a bank every race loads.

## The event name (confidence 92)

`SoundManager_PostEventByName` (`0x01779800`; patch `0x017caf30`), prototype `(u32 game_object, char* bank,
char* cue, void** handle, bool player)`:

```text
name = cue
if player: name += "_PC"            ; mov dword [rsp+rdx], 0x43505f
name += "__" + bank                  ; mov word [rsp+rax], 0x5f5f; strcpy
id = PostEvent(name, game_object, flags = handle ? 0x100001 : 0x100000, callback, handle, 0, 0, 0)
if id == 0: printf("Failed Posting Audio Event %s /n", name)
```

`PostEvent` (base `0x01095430`, patch `0x010937d0`) lower-cases the name and hashes it with FNV-1
(`h = 0x811c9dc5; h = (h * 0x1000193) ^ byte`), which is `oag_formats::wwise::name_hash`. The same three
lines (cue, `_PC` when the group's player byte is set, `__`, bank) are inlined in the pause and resume walks
(patch `0x01782420`, `0x0177fd00`, which call `ExecuteActionOnEvent` with action 1 and 2).

Evidence, three independent:

1. **The bank listings agree.** Every bank ships a Wwise `.txt` listing beside it (`Data/audio/sound/*.txt`,
   `English(US)/*.txt`). `Weapons_NGP.txt` lists `243270081 _321_NGP__Weapons_NGP`, `744083536
   Go_NGP__Weapons_NGP`, `3309018609 _321_NGP_PC__Weapons_NGP`, `3733418614 Go_NGP_PC__Weapons_NGP`; FNV-1
   of each name gives its id (checked for the four).
2. **The music call site.** `Set_Music_Track_%d` is posted with the `frontend` slot's bank name and
   `player = 0` (patch `0x0177f2e0`): `Set_Music_Track_N__frontend`, the event names
   [wwise.md](../../../formats/wwise.md#music-a-music_track-state-per-song) already reads off `Music.bnk`.
3. **The trackside census.** Every `track.final.audio` (38 files over the base archives; `u32` version 1,
   `u32` count, then 32-byte entries `{u32, f32, char bank[8], char cue[16]}`) names a legacy bank and a cue.
   Mapped through the alias table (below), with `~` turned into `_` as `SoundGroup_PlayCue` does, **1,212 of
   1,272 entries name an event that a listing contains**. The 60 that do not name a legacy bank the alias table
   lacks (`CHENGOU`, `SEBCLIM`, `GENTRAK` in upper case where the table has `gentrak`, `env_mal`, `general`,
   `techder`, `shipHD`) or an empty cue.

The ground-truth check pins the countdown and the missile against the banks themselves (below).

## The bank slots and what loads them (confidence 88)

`SoundManager_LoadBank` (`0x017361e0`; patch `0x01784a10`) takes `(manager, name, unused)`: it appends `.bnk`,
calls `LoadBank(name, pool -1, &id)` (base `0x01097d20`, patch `0x01096060`), and stores the bank id at
`manager + 0x2e8 + 8 * slot` and the name pointer at `manager + 0x368 + 8 * slot`, in the first free slot of
1-15 (slot 0 is the ship bank). The third argument is never read (no use of `edx` before the call overwrites
`rdx`); `frontend` and `Weapons_NGP` pass 1, the rest 0, so it is not the language flag (`Speech_NGP` and
`speech_fe` sit under `English(US)/` and pass 0).

Who loads what, all eleven call sites of the loader (base and patch alike, every one read):

| When | Caller | Banks |
| --- | --- | --- |
| Sound manager construction (boot) | `SoundManager_Construct` (`0x017359c0`) | `frontend` (slot global `DAT_020f9590`), `Speech_NGP`, `generaltrack`, `speech_fe`, `Ship_NGP` (slot 0, loaded directly, skipped when already loaded), `Weapons_NGP` (`DAT_020f959c`), `crowd`, `Speech_NGP_Grid` |
| Front end entered | `FrontendRoot_Construct` (`0x013c28f0`) and `FUN_01252ad0`, both after the `LoadFESoundBanks numEmitters=%d` printf | `frontend_fliers` |
| Track load | `TrackStartup_Load` (`0x01391130`) | the track's `TrackStartup.xml` `<LoadSoundBank Filename=...>`, `.bnk` stripped; **replaced** by `env0_det` in Detonator (mode 14) and by `env0_zone` in Zone (6), NitroBattle (13) and Multiplayer Nitro Battle (21), and in two more conditions not followed (`DAT_020eb4a8` with `FUN_016b05b0()`, or the current game-mode object's `+0x21c == 0`) |

`Init.bnk` and `Music.bnk` load before the sound manager (the music manager's path, not followed). Mode
numbers are `g_GameModeNames`, read off the patch table at `0x0198f890` (23 names, `Zone` = 6, `NitroBattle` =
13, `Detonator` = 14, `Multiplayer Nitro Battle` = 21).

`TrackStartup.xml` per track (all 26 read, base archives): the eight HD circuits and the four HD extras name
their `envN_*` bank under `Data/audio/sound/`; the ten 2048 circuits name `env_*`, with **`environments2048/mall`
naming `env_tower.bnk`** (nothing loads `env_mall.bnk`; no string `env_mall` in the executable). The four Zone
tracks name `env0_zoneN.bnk`, which is no `Data/audio/sound/` bank (the files of that name are `SBlk` banks in
`data02`'s `environments/zone_N/`); that works only because Zone mode always substitutes `env0_zone`.

**Live, 2026-11-01** (the wipeout_pc native port running the patch eboot to the Amphiseum grid, its open log):
the banks open in exactly the construction order above, each tried under `english(US)/` first and then at the
root (`Init`, `Music`, `generaltrack`, `Ship_NGP`, `crowd`, `Speech_NGP_Grid` log "not found" in the language
directory; `frontend`, `Speech_NGP`, `speech_fe` and `Weapons_NGP` exist there and log nothing), then
`LoadFESoundBanks numEmitters=12` and `frontend_fliers`, then `UnloadFESoundBanks` and `env10_amphesium` for the
race. No `Failed Posting Audio Event` line in the run. One run; the race was not driven past the countdown.

## The stub banks: legacy names, never loaded (confidence 85)

The sixteen 32-byte banks (`weapons`, `weapons_det`, `weapons_nitro`, `weapons_zone`, `shipHD`,
`Ship_NGP_Zone`, `voppler`, `speech`, `speech_det`, `speech_download`, `speech_elim`, `Speech_NGP_Grid`,
`speech_PreRaceChatter`, `speech_results`, `speech_zbattle`, `speech_zone`) hold nothing because their events
live in the loaded banks under the old suffix:

* No loader call names any of them except `Speech_NGP_Grid` (loaded at boot, empty in the shipped build). Their
  strings are referenced only by the alias table's initializer.
* Events suffixed with a stub name sit in a loaded bank: `_voppler__voppler` and four more in `Ship_NGP.bnk`,
  `_AUTOPILOT__weapons` in `Weapons_NGP.bnk`, `_321_GO__speech` in `Speech_NGP.bnk`; 482
  `*__Speech_NGP` events are in `English(US)/frontend.bnk` (the patch's 22.6 MB copy) and 274 more in
  `Speech_NGP.bnk` itself. The bank in a name is a namespace, not where the event is loaded from.

### The alias table (confidence 75)

`SoundBankAliases_Init` (`0x01733e50`, a static initializer registered with `__cxa_atexit`) builds a
`std::map` of 50 `{key, bank}` pairs (inserted with a hint by `0x01734e70`); `SoundBankAliases_Lookup`
(`0x01736430`) returns the mapped bank for a key, or the key itself. Keys are legacy bank names cut to 8 bytes.
Read off the disassembly's stack stores (the decompile drops the high halves):

```text
env_zon->env0_zone   env_vin->env1_vinetak   env_moa->env3_moathermav   env_che->env4_chengou
env_anu->env8_anulphapass   env_alt->env_altima   env_are->env_arena   env_bri->env_bridge
env_cat->env_cathedral   env_par->env_park   env_sol->env_sol   env_squ->env_square   env_sub->env_subwa
env_tow->env_tower   FRNTEND->frontend   frontend->frontend_fliers   gentrak->generaltrack
shipHD->shipHD   Ship_NGP->Ship_NGP   shipHD->Ship_NGP_Zone   SPEECH->speech   speech_f->speech_fe_NGP
Speech_N->Speech_NGP   Speech_N->Speech_NGP_Grid   radios->speech_PreRaceChatter   results->Speech_NGP
zone_vo->speech_zone_NGP   weapons->weapons   Weapons_->Weapons_NGP   base_tra->base_track_bank
crowd->crowd   env_zon->env0_det   env_amp->env10_amphesium   env_mod->env11_modesto
env_tec->env12_techdera   env_met->env2_metropia   env_ube->env5_ubermall   env_seb->env6_sebencoclimb
env_sol->env7_sol2   env_tal->env9_talonsjunction   zone_vo->speech_det   downloa->speech_download
"SPEECH "->speech_elim   FE->speech_fe   zone_vo->speech_zbattle   zone_vo->speech_zone   voppler->voppler
weapons_->weapons_det   weapons_->weapons_nitro   weapons_->weapons_zone
```

Duplicate keys (`env_zon`, `env_sol`, `shipHD`, `Speech_N`, `zone_vo`, `weapons_`) keep the first insert under
`std::map` semantics; that the hinted insert at `0x01734e70` refuses a duplicate is the library's contract, not
traced to its end. Two callers: an emitter builder (`0x01728960`: its record has a bank name at `+0x14` and a cue at `+0x1c`,
the same 8 + 16 byte pair a `track.final.audio` entry carries; which file feeds it was not traced) and the
animated world-object builder (`0x01393610`, `WorldManagerAnimated_%s` groups). The census above used first-insert-wins and reached
1,212 of 1,272; `env_sol -> env_sol` against `env7_sol2` is the case that decides Sol 2's emitters, and
`env7_sol2.txt` lists 26 `*__env_sol` events, which fits first-insert-wins.

## Emitter groups and cue requests (confidence 80)

`SoundGroup_Create` (`0x01736840`; patch `0x017850e0`), `(manager, name, flag, player)`: takes a group from the
free list, registers its game object (`+0x64`) under the name with listener mask 1 or 3, stores `flag` at
`+0xec` and **`player` at `+0xe0`**, the byte the post path reads for `_PC`. `Ship_StartMagstripSound` passes
`player = 1` for `MagStrip_Player`, 0 for `MagStrip_NPC`.

`SoundGroup_PlayCue` (`0x01731480`), `(f32 volume, group, slot, bank or null, cue, handle*, ?, copy)`: turns a
leading `~` into `_`, takes the slot's bank id and name when `bank` is null, and queues a request on the group:
`+0x00` flags (2 = post pending), `+0x08` volume, `+0x0c` 23999.0, `+0x10` 0.0005, `+0x1c` 0.4, `+0x48` the
group, `+0x60` bank id, `+0x68` cue, `+0x70` bank name. The sound update (patch `0x01780190`) posts a pending
request through `SoundManager_PostEventByName(group+0x64, +0x70, +0x68, handle, group+0xe0)`. The 23999.0 is
the value of every 2048 `AudioConfig.xml`'s `DistanceFilter near_cutoff`; that they are the same thing is a
hypothesis. The patch inlines this function into its callers (the countdown and the missile below are inlined
copies); no stand-alone patch twin was found.

## The countdown, end to end (confidence 88)

* **Post** (base): the race-stage setter `FUN_012993b0`, stage 1, called from 45 sites, among them every
  race manager's constructor (`SPArcadeRaceManager_Construct`, `SPZoneRaceManager_Construct`, ...), posts `_321_NGP` through
  `SpeechManager_PlayCue` (`0x0173ac30`) on `DAT_020fad68`, the speech manager, with slot `DAT_020f959c`
  (`Weapons_NGP`), unless the mode is Demo (2) or the game-mode object's `+0xc0` call says so. The go stage
  `FUN_0129a360` posts `Go_NGP` the same way. `SpeechManager_PlayCue` stops the cue still playing on its
  group (flag `0x10`, `ExecuteActionOnEvent` stop with a 4 curve), plays the new cue at volume 1.0 with the
  slot's bank, and keeps the cue name at `+0x88`. The speech manager's group is created in `Game_Main`
  (`0x0163fe9a`) as `SoundGroup_Create(manager, "SpeechManager", 1, 0)`: **player 0, so the name is
  `_321_NGP__Weapons_NGP`**.
* **Patch:** `FUN_012acf10` and `FUN_012afb20` (the same `_321_NGP`/`Go_NGP` string xrefs) inline the request
  on the group at the race manager's `+0x1aff0` instead, slot `DAT_021aaa4c`. That group's player byte was not
  read; it changes nothing measured, because `_321_NGP_PC__Weapons_NGP` is one action of kind `0x2103` whose
  target is `_321_NGP__Weapons_NGP` (and the same for `Go_NGP`), so both names reach the same media.
* **Bank:** `English(US)/Weapons_NGP.bnk` (base 38,077,271 bytes, patch 37,637,137), the only bank defining
  either event, on both archives.
* **What `_321_NGP__Weapons_NGP` does:** 12 actions. Three `Play` on layer containers at 0 ms; `Play` on six
  sounds with property 15 = 400, 400, 1,300, 1,300, 2,150, 2,150; two kind-`0x1e03` actions on sounds at 2,000
  and 500; and a kind-`0x0102` action on an actor-mixer at 3,100 with property 16 = 50. Property 15 reads as a
  delay in ms (the 400 / 1,300 / 2,150 pairs fit "three, two, one") and 16 as a fade time; both are fitted, not
  read out of the SDK.
* **Media reached** (all embedded, following `Play` and relays, `Library::resolve_event`): 13 ids, named by the
  listing: `SFX_WOPS4__COUNTDOWN__VO_ Intro_3_01`, `..._2_01`, `..._1_01`, `UI_Countdown_Layers_01` and its
  `__THREE`, `__TWO`, `__ONE`, `SHIP_Engines_Layer_01`, `SHIP_Mech_Layers_01`, `SHIP_Explosion_Layer_01`,
  `Hits_01`, plus the legacy `Weapons_NGP__321_stereo` (47216879) and `Weapons_NGP__beat1` (478212370). The
  library lists everything below the targets: which children of a random, switch or layer container sound at
  once is not read, so 13 is the candidates, not what plays.
* **`Go_NGP__Weapons_NGP`:** a `SetState` and three `Play` (two layer containers, one sound, delays 100 and 0),
  plus a relay to `xpress_low_PC__Ship_NGP` in `Ship_NGP.bnk`. 6 media: `VO_ Intro_GO_01`,
  `UI_Countdown_Layers_01__GO`, the three ship layers, and the legacy `Weapons_NGP__Go_stereo` (605868029).

## One weapon sound: the missile launch (confidence 85)

The missile launcher (patch `FUN_0137c0c0`; base `FUN_01350c40`, the `NGP_MissileFire` string's only xref)
queues `NGP_MissileFire` on the missile's own group with the `Weapons_NGP` slot, the same inlined request as
the countdown. `NGP_MissileFire__Weapons_NGP` (1097221332) is **one relay**, kind `0x2103`, to
`WPN_MISSILE_Fire` (2120703714, a `___NEW_WEAPONS` event without a bank suffix), which plays one layer
container; `NGP_MissileFire_PC__Weapons_NGP` relays to `WPN_MISSILE_Fire__PLAYER` (a `Play`, a relay and a
`SetState`). Media reached: 20 embedded ids (`SFX_WOPS4__WPN__Missile_Fire_V3_01_Tail`, `_03_Bwoosh`,
`_04_Trigger`, `_06_Trigger`, ...). The legacy `Weapons_NGP__NGP_MissileFire_stereo` (331390639) sits under the
listing's `LEGACY_WEAPONS` and is not reached. So following `0x2103` is required for weapons: with `Play`
alone, `Library::resolve_event` returns nothing for this event.

## Cross-title

**HD and 2048: checked, differs.** HD/Fury's PS3 build and 2048 play `SBlk` banks through Studio Liverpool's
own player ([psp-audio.md](../../../formats/psp-audio.md); 2048's countdown is `321_NGP`/`Go_NGP` cue names in an
`SBlk` speech bank, [countdown-voice.md](../vita-2048-eu-v104/countdown-voice.md)). Omega keeps the cue strings
and the legacy 8-character bank names, and bridges them to Wwise with the alias table and the `__<bank>` suffix:
the cue names match 2048's, the container and the naming do not.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x01779800` | `SoundManager_PostEventByName` | 92 | name built and posted, failure printf; three agreeing call sites |
| `0x017361e0` | `SoundManager_LoadBank` | 88 | `.bnk` appended, `LoadBank`, slot tables; 11 call sites; live open order |
| `0x01095430` | `Wwise_PostEventByName` | 85 | lower-case + FNV-1 of the name, then the id post; the bank ids match |
| `0x01097d20` | `Wwise_LoadBankByName` | 80 | called with `name.bnk`, pool `-1`, out id; live `open` of each bank |
| `0x01731480` | `SoundGroup_PlayCue` | 80 | request layout above; magstrip and trackside callers |
| `0x01736840` | `SoundGroup_Create` | 80 | free list, game object registration, player byte `+0xe0` |
| `0x0173ac30` | `SpeechManager_PlayCue` | 75 | stops the previous cue, plays on `+0x1d0`; countdown caller |
| `0x01733e50` | `SoundBankAliases_Init` | 75 | static initializer, 50 pairs, `__cxa_atexit` |
| `0x01736430` | `SoundBankAliases_Lookup` | 80 | map find, returns the mapped name or the key |

`FUN_012993b0` (race-stage setter) and `FUN_0129a360` (go stage) stay unnamed: what each stage is beyond the
countdown was not read.

## Patch (1.07) addresses

| Base | Name | Patch | Match |
| --- | --- | --- | --- |
| `0x01779800` | `SoundManager_PostEventByName` | `0x017caf30` | the failure printf and the `_PC`/`__` immediates |
| `0x017361e0` | `SoundManager_LoadBank` | `0x01784a10` | the same caller set and slot tables |
| `0x01095430` | `Wwise_PostEventByName` | `0x010937d0` | called by the post wrapper |
| `0x01097d20` | `Wwise_LoadBankByName` | `0x01096060` | called by the loader |
| `0x01736840` | `SoundGroup_Create` | `0x017850e0` | called by the patch's `MagStrip_Player` builder `0x0132b1e0`; player byte `+0xe0` |
| `0x01731480` | `SoundGroup_PlayCue` | - | inlined in the patch's callers |
| `0x012993b0` | (stage setter) | `0x012acf10` | `_321_NGP` string xref |
| `0x0129a360` | (go stage) | `0x012afb20` | `Go_NGP` string xref |

## What the port plays (2026-11-02)

`oag_omega::race::COUNTDOWN_VOICE` names the two events by their full names in `English(US)/Weapons_NGP.bnk`,
on every mode (`event_every_mode`); `oag_sound::sfx` reads a `BKHD` bank's event into the timeline its voices
already play (`crates/sound/src/sfx/wwise_start.rs`): `Play` on sounds and on layer containers (all children),
relays within the bank, action delays, initial delays, node volumes summed up the tree (busses not), the `Stop`
cut with its fade and the seeks spliced into the media. `_321_NGP__Weapons_NGP` loads 17 voices and
`Go_NGP__Weapons_NGP` 6; Go's `SetState` and its relay's seek on a `Ship_NGP` object are reported, not played.
Pinned by `crates/raceplay/tests/omega_countdown_ground_truth.rs`
(`the_start_plays_its_wwise_events_as_timelines`). A rendered Time Trial start (speech only, ours raising Ready
at tick 91 and Go at 271) rises at 1.50 s, then 2.0, 2.9 and 3.6 s (the VO's three delays after Ready) and at
4.6 s for Go; one sample of 768,000 reaches full scale. The Ready-to-Go spacing is Pulse's, lent.

## Open

* **The countdown post, live.** Drive the wipeout_pc run past GO and confirm no `Failed Posting Audio Event`
  line and the Wwise post of 243270081 (a breakpoint on patch `0x017caf30`, or its log).
* **Container rules.** Random/sequence and switch containers decide which child sounds;
  `oag_formats::wwise::hirc` does not read their playlists or switch keys, and the port refuses an event that
  reaches one. The countdown has none (three layer containers, whose children all play).
* **Seek and Stop layouts are fitted** on the countdown's two seeks and one stop (`hirc::Seek`, `prop`):
  confidence 65. `0x1204` (`SetState`) in `Go_NGP` sets a state nothing here reads.
* **Positioning.** The request's 23999.0, 0.0005 and 0.4, and each sound's 3D parameters (unread base
  parameters), decide pan and attenuation. What runs per frame, and what the patch's 3D audio is: [audio-3d.md](audio-3d.md). The countdown plays on the speech manager's group, which no code
  read here positions.
