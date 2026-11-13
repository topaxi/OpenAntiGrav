# `TrackStartup_Read`: where a circuit's `<LoadSoundBank>` is loaded from

Wipeout 2048, Vita EU, patch v1.04 (`/2048/eboot-vita-2048-eu-v104.elf`, image
base `0x81000000`). One function reads a circuit's `trackstartup.xml`; this page
records the one branch that settles which sound bank a circuit loads.

| Address | Name | Confidence | Role |
| --- | --- | --- | --- |
| `0x8121b688` | `TrackStartup_Read` | 85 | Walks the `TrackStartup` element: `LevelFx` (`UnderwaterSound`, `WindSound`), `LoadSoundBank` and `Billboard` children. Takes the parent object (`param_1`) and the XML cursor (`param_2`). |

Evidence is the function's own element-name literals (`"TrackStartup"`,
`"LevelFx"`, `"UnderwaterSound"`, `"WindSound"`, `"LoadSoundBank"`,
`"Billboard"`, `"Filename"`, `"Location"`, `"321Go_StartFinish.vex"`) and the
three child branches, one per element.

## The `LoadSoundBank` branch (confidence 85)

Decompiled, with the ordinary-race arm (`FUN_810018d4(&DAT_8153fc40) == 0`):

```c
FUN_81230e70(attr, filename, 0x400);                  /* Filename="env_x.bnk" */
SceLibc_7449B359(path, "data/audio/sound/%s", filename);   /* snprintf */
if (FUN_81014336(path) == 0) {                         /* virtual call, +0x34 */
    SceLibc_7449B359(path, "data/audio/DLC1/%s", filename);
    DAT_818c4e04 = FUN_81262b54(DAT_81521638, path);   /* load the bank */
} else {
    DAT_818c4e04 = FUN_81262b54(DAT_81521638, path);
}
```

The two format strings sit at `0x814a9074` (`data/audio/sound/%s`) and
`0x814a9088` (`data/audio/DLC1/%s`), directly after the `"LoadSoundBank"`
literal at `0x814a9058`, and each has exactly one cross-reference, in this
function. The Zone arm (`FUN_810018d4 != 0`) loads the literals
`data/audio/DLC1/env0_det.bnk` (when `DAT_8153fd24 == 0xe`, Detonator) or
`data/audio/sound/env0_zone.bnk`.

What this fixes:

1. **The bank is never read beside the track.** There is no
   `<track directory>/%s` format anywhere in the branch. The copies shipped
   beside `DLC1\environments\<X>\track.vex` are dead data.
2. **`data/audio/sound/` first, `data/audio/DLC1/` second.** `FUN_81014336` is a
   one-line virtual call (`(**(code **)(*DAT_815192f4 + 0x34))`) used at 32
   sites; its role as "does this path exist" is read from this branch, where a
   zero result sends the load to the other directory. That reading is the part
   short of 90: the call's target was not followed into the filesystem object.
3. Four of the twelve downloadable circuits (`Vineta_K`, `Anulpha_Pass`,
   `Chenghou_Project`, `Moa_Therma`) name a bank that **also** exists under
   `data/audio/sound/`, so the base copy wins there; the other eight read
   `DLC1`.

`TrackBanks`/`CircuitBanks::Directories` carries this as title data
(`crates/2048/src/race.rs`); the ground truth is
`track_audio_2048_ground_truth::the_downloadable_circuits_read_sound_first_then_dlc1_never_beside_the_track`.

## The bank names around it (for the shared list)

The executable names these as literals, which is what puts them in the shared
list rather than a guess: `generaltrack.bnk` (`0x814c50a8`, with the front-end
and results banks), and `crowd_NGP.bnk` (`0x814a9308`) with
`speech_PreRaceChatter.bnk` (`0x814a9438`) among the race banks. No
`voppler.bnk` literal exists on 2048 (HD has one), so 2048's `voppler` nodes
dangle: see [`2048-audio.md`](../../../formats/2048-audio.md).

## The `Billboard` arm substitutes the start board by mode (2026-11-10, `omega-billboards`)

**Confidence: 80** (decompile only; the same substitution, with the same file
names, is read at the instruction level on `ps4-omega-eu`'s
`TrackStartup_Load`, `0x01391130`).

After a `Billboard` element's attributes are read, `TrackStartup_Read`
(`0x8121b688`) tests the `Location` string:

```c
if (GameMode_Current() == 0) {
    if (GameMode_IsZoneFamily_q(&DAT_8153fc40)
        && (p = strstr(location, "321Go_StartFinish.vex")) != 0) {
        if (FUN_81001982(&DAT_8153fc40) != 0) strcpy(p, "321Go_HD_Zone_Battle.vex");
        else if (DAT_8153fd24 == 0xe)         strcpy(p, "321go_hd_detonator.vex");
        else                                  strcpy(p, "321Go_Zone.vex");
    }
} else {
    p = strstr(location, "321Go_2048.vex");
    if (FUN_812b0880(mode, 1) != 0 && p != 0) strcpy(p, "321Go_2048_Combat.vex");
}
```

So a Zone race on an HD-ported circuit shows `321Go_Zone.vex` on slot 8, a
Zone Battle `321Go_HD_Zone_Battle.vex` and a Detonator race
`321go_hd_detonator.vex`, as on Omega. `FUN_81001982` (the Zone Battle test)
and `FUN_812b0880` (the Combat test) are unread and keep their addresses.
`oag_2048::TITLE.adverts.mode_gantries` lists all four boards; Zone and Detonator are
selected (`oag_race::Mode::Detonator` since 2026-11-13); Zone Battle and Combat have no
`Mode`.
