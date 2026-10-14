# Head to Head on HD/Fury, and the census on Pure, 2048 and Omega (2026-10-14, `h2h-re`)

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), with the shipped XML of
`hdfury-ps3-eu-dec.iso`. Pulse's Head to Head, single and two-person, is on
[`../psp-pulse-usa/head2head.md`](../psp-pulse-usa/head2head.md) and
[`../ps2-pulse-eu/head2head.md`](../ps2-pulse-eu/head2head.md).

## Answer

**HD/Fury carries Head2Head in its executable and offers it nowhere.** Every shipped
menu that could set the mode lists it as `<aEntry>`, an element the front end's list
reader never reads; no campaign cell is `Head2Head`. So HD has no Head to Head a player
can reach, single or two-person, and the authored `splitscreen_hud` files' `HeadToHeadBar`
is a widget left over from Pulse's layout, not a two-person Head to Head mode. Confidence
85: the XML as shipped across all seven archives, plus the reader's own string table;
not walked live on RPCS3 on this pass.

## The evidence

**The mode is in the executable.** `search_strings` on the EBOOT finds `Head2Head`
(`0x00779068`) and `Multiplayer Head2Head` (`0x00779100`) in the mode-name block Pulse
also has, `HeadToHeadBar` (`0x0077fb00`), `HeadToHead` (`0x0078fb98`, the track
`ModeModifiers` attribute), `Data\FE\Images\head2Head_BW.gtf` (`0x00793258`) and
`/Data/Fe/Images/Head2Head_Icon` (`0x007a0660`). HD's campaign code still names mode 9 in
its difficulty-button set ([race-campaign.md](race-campaign.md)) and its loyalty
multiplier ([endrace-loyalty.md](endrace-loyalty.md)), and `stats_definition.xml`
(`DATA05`) charts a `Head2Head` slice (`StatID="12"`).

**No menu offers it.** `data/plugins/frontend/gui/racebox_definition.xml`, `Single
Player` screen, `<List name="Mode">`:

| Archive | Entries |
| --- | --- |
| `DATA02`, `DATA03` (HD) | `Arcade`, `<aEntry idstring="Head2Head">`, `Time Trial`, `Speed Lap`, `Tournament`, `Zone`, `<aEntry idstring="Ghost Viewer">` |
| `DATA00` (Fury) | `Arcade`, `<aEntry idstring="Head2Head">`, `Time Trial`, `Speed Lap`, `Eliminator`, `Zone`, `NitroBattle`, `Detonator`, `Tournament`, `<aEntry idstring="Ghost Viewer">` |

`network_definition.xml`'s online `Mode` list does the same in `DATA02`, `DATA03` and
`DATA04` (`<aEntry idstring="Head2Head" value="Multiplayer Head2Head">`, beside an
`<aEntry>` Elimination), and Fury's `DATA00` copy drops the line. **The EBOOT has no
`aEntry` string at all** (`search_strings`), while `Entry` appears many times, so the list
reader only ever matches `Entry` and these lines are switched off in the data. The same
RACEBOX page is where HD authors its local two-person race (`NumberOfPlayers`,
`SplitScreen`, [hd-frontend.md](../../../formats/hd-frontend.md#racebox-authors-local-split-screen-a-players-list-and-a-cut)),
so a two-person Head to Head would have been chosen here, and it is not offered.

**No campaign cell.** None of HD's 167 grid cells is `Head2Head` (checked 2026-10-08,
`docs/overview/status.md`).

**What the strings say it was meant to be.** HD's English `MSC_EVENT_HTH` reads "Head to
head: a two player single race of intense rivalry...", where Pulse's speaks of being
"pitted against an opponent". The design note survives; the mode does not ship.

## Census: Pure, 2048, Omega

| Title | Head to Head? | Evidence | Conf |
| --- | --- | --- | ---: |
| **Pure** (`psp-pure-eu` `BOOT.BIN`) | **None, single or multi.** | `strings`: the race managers are `SPArcade`, `SPTimeTrial`, `SPTournament`, `SPZone`, `SPFreePlay`, `Demo`, `AIBatch`, `MPArcade`, `MPTimeTrial`, `MPTournament`; no `Head2Head`, `HeadToHead` or `HeadToHeadBar` string at all | 85 |
| **2048** (`vita-2048-eu-v104`) | **None reachable.** The mode-name strings and `HeadToHeadBar` are inherited, nothing sets the mode. | `search_strings`: `Head2Head` (`0x81419094`), `Multiplayer Head2Head` (`0x814193b4`), `HeadToHeadBar` (`0x8147bce4`); the six `GameMode_*` classes are `ArcadeRace`, `CheckPointRace`, `EliminatorRace`, `SpeedLapRace`, `ZombieRace`, `ZoneRace` ([2048-campaign.md](../../../formats/2048-campaign.md)); 2048 has no racebox; no `Head2Head` cell in the 16 `data/plugins/grids/grid_NN.xml` of `data.psarc` or `data2.psarc` (`mode` census: Race 44, Speed Lap 34, Time Trial 22, Elimination 18, NitroBattle 13, Detonator, Tournament and Zone 12 each); outside the language files only HUD position files name it | 82 |
| **Omega** (`ps4-omega-eu`, patch 1.07) | **None reachable**, HD's case exactly. | `search_strings`: `Multiplayer Head2Head` (`0x0182c0a6`), `Head2Head` (`0x0183085a`), `HeadToHeadBar` (`0x018115c3`); `racebox_definition.xml` in `data00`, `data08` and `data09` all carry `<aEntry idstring="Head2Head" intvalue="1">`; the 20 grid files in each of `data00`, `data08`, `data09` hold the same mode census as 2048's, no `Head2Head` | 82 |

**2048 and Omega checked against each other** (CLAUDE.md's rule): **checked, applies**:
both inherit the mode name and the HUD widget and neither authors a route to the mode,
and their grid files carry the same cell-mode census. Omega's executable
(`/omega/eboot-ps4-omega-eu.bin`, the base package's) has `Entry` (`0x01822c30`) and no
`aEntry` string, as HD's.

**A correction to `docs/overview/status.md`:** its Omega cell said `Head2Head` cells
launch with one opponent. No Omega grid file has a `Head2Head` cell; the test it cited
holds a rule over every cell and is true of zero `Head2Head` cells.
