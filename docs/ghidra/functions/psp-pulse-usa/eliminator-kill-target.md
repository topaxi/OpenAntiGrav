# The Eliminator kill target and who is credited with a kill

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`. Read and measured 2026-09-30
on PPSSPP v1.20.4, `pulse-psp-usa.chd`, fresh profile.

**Status:** where a non-campaign Eliminator's target comes from is settled; how a kill is
credited is read in full.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x088e5ff4` | `RaceBox_ApplySetupGlobals` | 78 |
| `0x08b30fb0` | `g_eliminator_kill_target` | 82 |

`Ship_Damage` (`0x088439ac`, [shield.md](shield.md)) and `Eliminator_UpdateKillTarget`
(`0x0882ce18`, [race-campaign.md](race-campaign.md)) are already named; this page adds what
they do with a kill.

## The target is the race box's `KILLS` row, and a fresh profile shows 5

`RaceBox_ApplySetupGlobals` (`0x088e5ff4`) is the race box's commit. It writes the screen's
lists into the named globals (`Mode`, `Class`, `Opponents`, `Weapons`, `SkillLevel`,
`Locked`, `Track`), and its one numeric line is:

```c
if (*(int *)(param_1 + 0x120) != 0)
    DAT_08b30fb0 = atoi(*(char **)(*(int *)(param_1 + 0x120) + 0x84));
```

`DAT_08b30fb0` has exactly one writer (this line) and five readers, among them
`Eliminator_UpdateKillTarget` and `Hud_BindWidgets` (`KILLS (%d)`). The list at `+0x120` is
the `Eliminations` list of `RaceBox_Definition.xml`: `5`, `10`, `15`, `20`, `25`, the only
numeric list on the screen. Confidence 78 for the identification of the widget (by
elimination and by the live frame below, not by a name string).

**Live, fresh profile (PPSSPP, own `HOME`, nothing pressed on the row):** the Racebox with
`RACE TYPE ELIMINATOR` shows `KILLS 5`, every other row on its first entry too (`AI DIFFICULTY
EASY`). Frame: `rb-settings.png` (gitignored).
The previous lane's Racebox Eliminator drew `KILLS (5)` in the HUD and ended at exactly five
kills. Confidence 90 that a Racebox Eliminator on a fresh profile uses 5.

**The XML says otherwise and the screen ignores it.** The list is authored
`<List name="Eliminations" ... Default="10">`, and it shows its first entry (5). Every list
row on the page also showed its first entry, so `Default` is not applied to this list; why is
**not determined** (a loader that reads it as an index, which 10 cannot be for five entries,
would fall back to entry 0, but that is a guess). The other four lists author string defaults
that agree with their first entries anyway.

Two sources therefore feed the target, and only one is a fallback:

1. a campaign cell's gold figure (`cell+0xa0`), when `DAT_08b30ffc` is set;
2. otherwise the `KILLS` list's selected entry: 5, 10, 15, 20 or 25, **5 on a fresh profile**.

`oag_race::Mode::ELIMINATOR_KILL_TARGET_DEFAULT` is now 5, the list's first entry. This build's
RACE page has the `KILLS` row since 2026-09-30 (see [race-setup.md](../../../formats/race-setup.md)),
so the other four can be picked.

## Who is credited with a kill (`Ship_Damage`, confidence 85)

On the blow that takes a shield to zero (`param_1 <= 0.0`), in game mode 8, **and only when the
damage source (`param_3`) is 2, a weapon**: the attacker recorded on the victim
(`*(*(victim+0x4c)+0x13c)`) has its kill counter (`entity+0x8d8`) raised by one, unless it is
the victim itself. The credit is immediate, on the fatal blow, not at the respawn. A wall or
other source that finishes a craft off credits nobody, and a wall scrape before a later
rocket does not erase the attacker id (nothing read clears it).

Two things this build had wrong, both corrected in `oag_raceplay::eliminator`: a wall
scrape cleared the credit, and a blast credited only the craft it struck directly (splash
credited nobody). Measured with the player parked on `16_Track`, 3 of 9 deaths credited
nobody.

## Every weapon credits through the same field (2026-10-02, confidence 80)

Three independent writers put the shooter's craft index into the victim's `+0x13c`, the field
`Ship_Damage` credits from, and each reaches `Ship_Damage` with source 2 (a weapon):

| Weapon | Writer of `+0x13c` | Damage reaches `Ship_Damage` through |
| --- | --- | --- |
| Cannon | `Cannon_ApplyCraftDamage` (`0x08857e90`), tag 3 | `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`) |
| Leech Beam | `LeachBeam_Drain` (`0x08866804`), tag 7, on the target | the same consumer |
| Quake | the wave block of `FUN_088418e0` (`0x08841e60`), tag 5 | a direct `Ship_Damage(damage, s2, 2, 5, 0)` |

Evidence: [cannon-quake-leachbeam.md](cannon-quake-leachbeam.md) (static, two independent call
paths per weapon). Not confirmed live on PPSSPP. One reading to keep in mind: the Quake block
that page transcribes twice names the written record `entity` once and `owner_craft` once; the
first reading (the struck craft's own record, `s2`) is the one consistent with the shooter
and shield tests beside it, and is what is ported.

So a Beam or Quake kill **is** credited, and a wall that finishes a craft off after a weapon hit
that left the shield standing credits nobody (the fatal blow's source must be 2).
`oag_raceplay::eliminator::note_weapon_hit` and `record_pending_hit` port this.

## What is still open

- ~~Beam, Cannon and Quake credit~~ settled above; this build now credits them.
- Whether the original's opponents die to walls as often as this build's: in a 330 s
  parked-player run 9 of 20 deaths here were wall-fatal (no weapon hit in the last second),
  where the original's results table read kills 21, deaths 20 in 85 s.
- `Default="10"` not being applied to the `Eliminations` list.
- How often the original's AI fires: `WeaponAi_DecideFireOrAbsorb` rolls the
  Eliminator table (`0.001` to `0.1`) times `useAgainst*` times 5, indexed by a skill score
  that is `0` unless a craft is ahead within 100, so a craft with nobody near ahead fires about
  once in 45 s. The original fires rarely; what it has is a dense field ([weapon-ai.md](weapon-ai.md)).
