# Where a 2048 race reads its particle effects from

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. Read 2026-10-05 for the `v2048-particles` lane.

## `Particles_BuildEffectPath` - `0x812a6a2e`

**Confidence: 85.** One `snprintf` into a shared 128-byte buffer
(`DAT_819641e8`): it asks `FUN_81000930(&DAT_8153fc40, -1)` and formats
`Data/Particles2048/%s` when that returns 0, `Data/Particles/%s` otherwise.
`FUN_81000930` reads the `u32` at `+0xe4` of the global state block and returns
1 when it is below `0x17` (23). `FUN_810018d4`, read beside it, treats
`+0xe4 == 6`, `0xd..0xe` and `0x15` as the Zone-family modes, so `+0xe4` is a
game-mode id. **Which modes are `>= 23`, answered 2026-10-05 (conf. 70):** an event's id is
`FUN_810016ba` of its own `m_name` (`GameModeBase+0xc4`, `FUN_812b0d52`, called from
`CampaignEventCard_HandleInput`). None of the 577 `SP.xml`/`MP.xml` instance names is in the 23-name
table or hashes below 23, so 2048's own events read `Data/Particles2048` and the port's
`oag_2048::race::EFFECT_DIR` is right for them. Only the HD-lineage named modes read `Data/Particles/`,
and the port plays none. Later writers of `state+0xe4` were not walked; a Vita3K read of `0x8153fd24` in a
campaign race would settle it.

The two directories hold the same stems with different bytes on nearly every
one (`docs/formats/pob.md`, "Wipeout 2048").

## `Ship_RegisterEffectSet_q` - `0x81298f58`

**Confidence: 65.** Called from 28 sites with the ship being set up. It adds
effect files to a per-ship set through `FUN_812a6906`, all by literal name:

| Condition | Effects |
| --- | --- |
| not a Zone mode (`FUN_810018d4 == 0`) | `WO_SHIP_COLL_SPARK_DAMAGE`, `WO_SHIP_COLL_SPARK_NODAMAGE`, `WO_DAMAGE_MILD`, `WO_DAMAGE_MODERATE`, `WO_DAMAGE_CRITICAL`, and `WO_FORCE_FIELD` when mode `< 23` |
| a Zone mode | `WO_SHIP_COLL_SPARK_DAMAGE_ZONE`, `WO_SHIP_SPARK_NODAMAGE_ZONE`, `WO_DAMAGE_ELECTRIC`, and `WO_FORCE_FIELD_ZONE` when mode `< 23` |
| always | `WO_SHIP_SPARK_DAMAGE_WEAPON`, `WO_TRAIL_HITSHIP_RED`, `WO_TRAIL_HITSHIP` |
| the hull's name is `nitro` | `WO_NITRO_DEBRIS_SPARKS` |
| any other hull | `WO_DEBRIS_SPARKS`, `WO_DEBRIS_FIRE` |

This is a *preload* list, not a trigger: it names what a ship may play, and
which event plays which entry was not read. It does show that the Zone modes
swap the wall-contact spark for `WO_SHIP_COLL_SPARK_DAMAGE_ZONE`; the port does
not do that swap yet (listed in the lane report).
