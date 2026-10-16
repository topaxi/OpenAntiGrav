# Online lobby: Omega's flow, what a server needs, the v1 plan

Research for a first online lobby server (2026-10-16). Part 1 is read off Wipeout
Omega Collection's own front end (PS4, patch v1.07). Part 2 maps it onto
[`oag-net`](README.md). Part 3 is the shadPS4 attempt and the firmware question.
Part 4 is the architecture plan, which is **chosen, not measured**; it has no
confidence scores. Every other claim carries a score from the
[rubric](../reverse-engineering/confidence-rubric.md).

Nothing here is built. No server code changed.

## 1. Omega's online screen flow

### Where it comes from

Omega's front end is HD's `PI001` plugin carried forward ([`omega-status.md`](../formats/omega-status.md)).
The online screens are in `data09.psarc` of the day-one patch, under
`data/plugins/frontend/gui/`: `online_definition.xml` (login and community),
`network_definition.xml` (create, join, lobby, settings) and the lobby widgets named
in them. Read with `scripts/psarc.py cat <archive> <path>` (digest-matched, see
[`psarc.md`](../formats/psarc.md)); the strings are `data/plugins/languages/english/entries.xml`.

**Reachability, and why it matters (confidence 90).** The screen graph has to be read
with XML comments and `<? ... ?>` author notes removed, and with `<aEntry>`/`<aList>`
(a disabled element: the `a` prefix) ignored. Done that way and walked from
`mainmenu_definition.xml`'s `Main Menu`:

- `multiplayer_definition.xml` (`Multiplayer`, `Join Game`, `Lobby`, `Network Sync`,
  `Online Settings`, `Create Single Race` and its siblings) is **not reachable**. It is
  byte-identical to HD's copy (18,226 bytes, both), authored at 480-wide coordinates
  (`x="240"`): Pulse-era legacy, not Omega's online flow. Do not model it.
- **There is no quick match.** `Play Now` (a `UnityPlayNow` item that searches and
  joins) is inside an XML comment in Omega's `network_definition.xml`, with no live
  reference. HD's copy is live. Omega's `OnlineMainMenu` offers exactly three entries
  (CREATE GAME, JOIN GAME, ONLINE STATS); `ONL_CON_EULA`, `ONL_CON_ANNOU`, `ONL_M_BL`
  and `PRO_STATS` are `<aEntry>`, disabled.
- The online flow re-uses the local race-setup screens (`Track Creation`, `Tournament C`,
  `Team Selection`) and diverges at `Team Selection`'s redirect (below).

Some screens are reached only from code (`LobbyCommand` modes, `KickFromGame`,
`FrontendLaunchMPGame`, `ScoreSyncRoot`): XML-unreachable does not mean dead for those.

### The flow (live path)

| Step | Screen (file) | What it does | Evidence | Conf. |
| --- | --- | --- | --- | --- |
| 1 | `Main Menu` ONLINE (`mainmenu_definition.xml`) | `FE_ONLINE` redirects to `NetLoginPage` | the `<Redirect>` entry | 95 |
| 2 | `NetLoginPage` (`online_definition.xml`) | A `<PsnConnection>` item; success goes to `NetConnectionRedirect`, failure back to `Main Menu`. The player sees an INFO dialog "Connecting..." | XML; live: shadPS4 shows exactly this dialog after Main Menu > ONLINE > X (part 3) | 95 |
| 3 | `OnlineMainMenu` | Three entries: CREATE GAME (`ONL_CRGAM`), JOIN GAME (`ONL_JGAM`), ONLINE STATS | XML | 90 |
| 4a | `MP Select` (`network_definition.xml`, a `RaceBox` screen) | The create-game settings page, below | XML | 90 |
| 4b | `Game List` | 4 columns (`RB_HOST`, `FE_GAME_MODE` the content pack, `RC_RT` the mode, `RC_SC` the speed class), `<GameList MaxRows="10" Columns="4">`; the list is filled by code; the title is `ONL_CRJ_GLIST` ("JOIN GAME" list). `FilterList` (a RaceBox screen, not reachable from XML) holds the filters: Game, Mode, Class, availability (`MSC_ANY` on each) | XML | 85 |
| 5 | `CreateMPGame` / `JoinMPGame` | A `<PsnConnection>` item in `create` or `join` mode; success to `Team Selection`, failure to `OnlineMainMenu`; `JoinMPGame` also a `LobbyCommand mode="join"` | XML | 90 |
| 6 | `Team Selection` (`team_selection_definition.xml`) | `TeamRedirect`: `Main Menu->Mode == FE_ONLINE` goes to `GameLobby`, otherwise `Team Launch Transition` (the local path) | XML, three copies | 90 |
| 7 | `GameLobby` | The waiting room, below | XML + strings | 85 |
| 8 | `FrontendLaunchMPGame` | `LobbyCommand mode="loadgame" forward="Launch Game" back="GameLobby"`: the race starts | XML | 85 |
| 9 | After the race | `return_to_lobby` is an EndRace menu block (`FE_RETURNING_TO_LOBBY`); `ScoreSync` goes to `Race Records`; a ranked game ends in `RankedGameEnd` (`LobbyCommand mode="kick"`) | XML and a string in the executable | 70 |

`LeaveGame` (`LobbyCommand mode="leave"`, forward `ModeResolver`) and `LeaveConfirm`
("Are you sure you want to leave this game?") leave from the lobby; `ModeResolver`
sends the player back to `OnlineMainMenu`.

### Create game: `MP Select` options (confidence 90, XML as shipped)

| Option | Values | Default |
| --- | --- | --- |
| Game (`MSC_WOHD`, `msc_wo48`) | the title's own content pack: HD, or 2048 | `MSC_WOHD` |
| Mode | `Arcade` (Multiplayer Single Race), `Eliminator`, `NitroBattle` (Zone), `Tournament`; the 2048 pack's own list is Arcade 2048, Elimination 2048, Tournament 2048 | Arcade |
| Class | venom, flash, rapier, phantom | venom |
| Weapons | on / off | on |
| Weapon hints | on / off | |
| Pure racing (`GreenMode`) | on / off | off |
| Availability (`GameAvailability`) | PUBLIC (0), INVITE ONLY (1); a PARTY-only variant (2) is a separate list | PUBLIC |
| Barrel rolls | on / off | |
| Pilot assist allowed | on / off | |
| Laps | slider 3 to 20 | |
| Eliminations (target) | slider 50 to 600 step 25 | 200 |
| Zone target (`NitroTarget`) | slider 15 to 55 step 5 | 30 |
| Max players | the list is `<aList>`, **disabled**: the room size is fixed in code | 8 (the lobby draws `playerList0..7`, `RankIcon0..7`) |
| Respawns | `<aList>`, disabled | |

A **track** is not on this page: `MP Select`'s redirect goes to `Track Creation` (a
Tournament goes to `Tournament C`), the local track pages. Game name and password
(`GameDetails`, `EnterGameName`: ONL_CG_GAME_NAME, ONL_CG_GAME_PASSWORD) exist but are
not reachable from the live graph (**checked**: no live reference; a password feature
Omega shipped switched off).

### The lobby: `GameLobby` (confidence 85 for the layout, 70 for the behaviours that are strings only)

Widgets: eight player rows (name, flag, rank, team logo, ship, voice icon),
"CURRENT SETUP" (mode, class, weapons, weapon hints, laps icons), an `ONL_GAMEINFO` panel
and a `LobbyCountdown`/`lobbytimer` item. Options (`GameOptions` menu): **SELECT TEAM**
(`ONL_TEAM_SELECT`, goes to `Team Selection`), **EDIT SETTINGS** (`ONL_LOB_EDIT`, goes to
`Edit Settings`, the same page as `MP Select`, then `ApplySettings`), **INVITE FRIENDS**
(`Friend Invites`), **START RACE** (`ONL_LOB_STARTRACE`).

Behaviour, from strings in `entries.xml` (each a **single-source string**, so 70):

- Start gate: "You cannot start this race until at least %d players are in the lobby"
  (`ONL_LOB_STARTFAILED`), "WAITING FOR PLAYERS", "STARTING IN", "STARTING RACE IN:",
  "Waiting for:" (a list of players not yet ready), "EXTEND TIMER".
- Ready is **ship choice, with a timeout**: "Please select a ship or you will leave the game"
  (`ONL_KICK_COUNTDOWN`), "You failed to choose a ship in time" (`ONL_KICK_TEAM`).
- **Host migration**: "%s has been made host of this game", "You have been made host of this game",
  "Lost connection with host", "The host has terminated the game"
  (`ONL_CRJ_HOSTTER`, back to the list).
- Compatibility kicks: "The host has changed the settings and has picked content that you
  do not have" (`ONL_KICK_CONTENT`), "This game has already started" (`ONL_KICK_MISSED_START`),
  blocked-list conflicts both ways, "Cannot join game. Game is full."
- **Spectate**: `ONL_SPECTATE`, "JOIN AS SPECTATOR", "RACE IN PROGRESS" (a lobby can be open
  while a race runs).
- All players leave: "All other players have left the game. Returning to Main Menu."
- Chat: a `Chat Warning` screen and "ONL_CHAT_BAN" exist (text and voice are a PSN feature).

### What the executable adds (confidence in each line)

Searches of Omega's base `eboot.bin` (v1.00; the patch executable is not in the Ghidra
project) for `Frontend\Items\*.cpp` source-path strings:

- `LobbyCommand_Item.cpp`, `LobbyCountdown_Item.cpp`, `LobbyTeamDisplay_Item.cpp`,
  `PsnConnection_Item.cpp` and `UnityCommandPlayNow_Item.cpp` are the item classes the
  XML above names (**95**, same strings). A `WOLobbyStateManager::SetMPSeed` string
  (`0x018327c9`) says the lobby state carries a **shared MP seed** (confidence **70**:
  the name only; nothing read at the call site). It maps onto our `RaceParams::seed`.
- **Rooms are NP Matching2 through NP Toolkit v2** (confidence **80**): 27
  `SCE_TOOLKIT_NP_V2_MATCHING_*` strings (create/join/update "system session", room
  attributes, `..._USER_IS_ALREADY_IN_A_ROOM`, `..._ROOM_DESTROYED`). Those are the
  library's own limits, **not Omega's design** (8 integer search attributes, 512 bytes
  of room attributes).
- **RakNet is linked** (`Code\Network\raknet\RakPeer.cpp`, `ReliabilityLayer.cpp`,
  `RakNetSocket2.cpp`, `Command_StartRakNet`; **90** that it is linked). That the race
  traffic itself runs over it is an **inference (60)**: the PSN room layer would carry
  discovery and signalling, not 8 ships at 60 Hz. The `ID_LOBBY2_*` and `ID_XBOX_LOBBY`
  strings are RakNet's own enum names, compiled in with the library; they say nothing
  about Omega.
- The shared seed and the `loadgame` command (step 8) fit a lobby that **fixes the race
  parameters and the seed, then every machine loads and simulates**; nothing found says
  whether Omega used lockstep or state sync (the networking README's 40-confidence
  inference stands).

### Checked against HD and 2048 (the project rule for Omega front-end findings)

- **HD: differs.** HD's `network_definition.xml` has 50 screens against Omega's 30
  (26 shared), `online_definition.xml` 31 against 20 (17 shared). HD has its own
  account and server layer (`Eula`, `UniverseList`, `ChannelList`, `Login`, `Logout`,
  `PatchGame`, `ServerDataSync`, `OnlineUserName`, `AutoLogin`, `InitAdhoc`, `InitLan`,
  `MPMainMenu` with an ONLINE-or-LAN choice) and a live `Play Now`. Omega dropped them for
  PSN (`NetLoginPage` is a `PsnConnection`), added `NetConnectionRedirect`, `NetLocation`,
  `Reconnect` and four VR warning screens, and commented `Play Now` out. The create page,
  game list, lobby and edit-settings screens are the shared core (**ported by
  inheritance**: Omega copied HD's). Confidence 85.
- **2048: checked, applies in part.** `MP Select`'s Game list names `msc_wo48` and the 2048 mode
  list, so a 2048 pack can be hosted from Omega's lobby; 2048's own (Vita) front end has no
  `network_definition.xml` (touch front end, [`2048-frontend.md`](../formats/2048-frontend.md)).
  Not wired.

## 2. What a lobby server needs (mapped onto this project)

What Omega delegates to PSN and a host console, and what the project has:

| Need | Omega | This project today | Gap |
| --- | --- | --- | --- |
| Room record (settings, members, owner) | PSN room with attributes | `RaceParams` (`oag_net::wire`), fixed at process start | a `Room` that holds the settings before the race is loaded; the settings are the `MP Select` table above, mapped to `RaceParams` fields (`mode`, `class`, `weapons`, `track`, `seed`, `people`) |
| Room list | `Game List`, 4 columns, filters Game/Mode/Class | LAN `discovery` adverts + `compat` | a list message with host, mode, class, members, a join verdict |
| Quick match | none (commented out) | none | **optional**; label chosen |
| Join | `JoinMPGame`, full/started/blocked/content kicks | `Host::join`, `Control::Reject{reason}` | reasons as named rejects: full, started, content, version |
| Ready | choose a ship within a timeout | `Control::Ready` (the client's tick-0 hash) | a lobby-phase ready that is not the tick-0 hash |
| Start | host presses START; min players; countdown; extend timer | `Lobby::take` + `Host::seat` (LAN START) | min-players rule, countdown, extension |
| Host authority | the creator; migrates when they leave | server-authoritative, no host (trust model) | a **room owner** role for settings and START only; the sim authority is the server |
| Seed / content / version | shared MP seed; `ONL_KICK_CONTENT` | tick-0 hash compared at join; `fingerprint` | exchange fingerprint **before** the race loads: image, mods, build |
| Spectate / late join | JOIN AS SPECTATOR | none | later |
| After the race | `return_to_lobby`, rematch | the server exits when everyone finished | a return-to-lobby message and a reset |
| Chat | PSN | none | out of v1 |
| Identity | PSN account | none | **a name and a session token** (chosen) |

The existing `lobby.rs` (waiting connections) and `serve.rs` (race loop) are the LAN
equivalent; online adds rooms and the owner role and keeps the same `Host`.

## 3. ShadPS4 on Omega (2026-10-16)

### Result (measured)

shadPS4 0.20.0 boots Omega (the v1.07 `eboot.bin` and archives, the base `param.sfo`)
to the title screen with **no firmware modules at all** (HLE only), on the AMD
RX 7800 XT with RADV, and a virtual pad walks it to `PRESS ANY BUTTON TO START`,
CAMPAIGN SELECT, MAIN MENU (CAMPAIGN, RACEBOX, ONLINE, OPTIONS, RECORDS), and ONLINE
leads to the `NetLoginPage` INFO dialog "Connecting...", which never resolves (confidence
**90**; the screens agree with the XML above, which makes the live path a measured one
up to step 2). Everything after is gated behind NP sign-in, so steps 3 to 9 are
read, not seen.

The compatibility report ([shadps4-game-compatibility#487](https://github.com/shadps4-compatibility/shadps4-game-compatibility/issues/487),
CUSA05670, "Nothing") describes an **NVIDIA** failure: `vk_scheduler.cpp:155 Device lost
during submit` from v0.6.0 to v0.10.0, with AMD users reporting menus. This result
fits that (an AMD device here), and not more: it does not say the NVIDIA bug is fixed.

### The recipe that worked (and what did not)

- A game folder with `eboot.bin`, the `dataNN.psarc` archives (base `data00` to `data04`
  and the patch's `data05`, `data07`, `data08`, `data09`), `sce_module/`, the `.plt`
  files and `sce_sys/`. **Hard-link or copy**: symlinks that leave the folder fail
  (`/app0/../..` is resolved inside the guest and the executable "does not exist").
- `sce_sys/param.sfo`, `playgo-chunk.dat`, `pic1.png` and friends are **not in the
  extracted folder**; they are plaintext entries (ids `0x1000` to `0x12c0`) of the
  `.pkg`'s entry table. Without `playgo-chunk.dat` the game parks after `scePlayGoOpen`.
- **Xvfb has no DRI3**: RADV cannot present, and lavapipe runs out of device memory
  (`vk::Result=ErrorOutOfDeviceMemory` at the first compute pipeline). What worked is a
  headless wlroots compositor on the render node: `WLR_BACKENDS=headless
  WLR_RENDERER=vulkan WLR_RENDER_DRM_DEVICE=/dev/dri/renderD128 sway -c <conf>`, and
  `SDL_VIDEODRIVER=wayland` for the emulator (`pixman` in the compositor gives
  `ErrorSurfaceLostKHR`).
- **Input**: a uinput pad is seen, but the game ignored it until
  `SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1` was set. `scripts/uinput-pads.py` works.
- Isolation: set `HOME` and the three `XDG_*` variables together, or shadPS4 asks (a
  `zenity` dialog) to migrate saves from "the old location" of the real profile; a
  `zenity` shim on `PATH` that prints `Do nothing` answers it. Silence audio with
  `SDL_AUDIODRIVER=dummy`; **audio was not checked at the mixer** (no `pactl` probe).

### Firmware: what Omega needs, and what the emulator says it has

- `SYSTEM_VER` in the `param.sfo`: base `0x02700000`, patch `0x04700000`: the console
  firmware the title was built for is **2.70 and 4.70** (**95**, read off the SFO).
  The patch's `PUBTOOLVER` is `0x03380000`.
- The log shows the game loading these through `sceSysmoduleLoadModule`: NpTrophy,
  MsgDialog, ErrorDialog, SaveDataDialog, PlayGoDialog, AppContent, DiscMap, PlayGo,
  Audio3d (twice), NpToolkit2 (its own `sce_module/libSceNpToolkit2.prx`, which loads and
  starts), Json2, Hmd, HmdSetupDialog, Remoteplay, ShareUtility, SocialScreen.
- **Stubs** (843 `Stub resolved` lines): 436 are `libSceNpToolkit2` symbols and 285
  `libSceJson2`, then `libSceLibcInternal` 49, `libkernel` 16, ShareUtility 15,
  SocialScreen 10. The NpToolkit2 count is unexplained: the patch's own PRX loaded and
  started and the count is identical with the base's older PRX, so those are imports the
  emulator does not satisfy, not a missing file (cause not isolated; **open**).
  `libSceJson2.sprx` is on the quickstart's list of firmware modules; NpToolkit2 is
  not (the game ships it).
- shadPS4's own HLE implements part of `sceNpMatching2` (create/join/search room, signalling
  info, 1,282 lines), so the room layer is not absent; sign-in (`NpManager`, 6 stubs)
  and Toolkit are what stop the walk.
- The project's guide ([shadps4.net/quickstart](https://shadps4.net/quickstart/))
  lists 26 modules to place in `sys_modules` (the `libSceAudiodec*` family,
  `libSceCesCs`, `libSceFont`, `libSceFontFt`, `libSceFreeType*`, `libSceJpegDec/Enc`,
  `libSceJson`, `libSceJson2`, `libSceLibcInternal`, `libSceNgs2`, `libScePngEnc`,
  `libSceRtc`, `libSceRudp`, `libSceSystemGesture`, `libSceUlt`, `libSceWkFontConfig`,
  `libSceXml`, `libSceAt9Enc`). Omega reached its menus without any of them: they
  improve fidelity (LLE) where HLE is missing, they are not a boot requirement here.
  `libSceJson2` and `libSceLibcInternal` are the two that intersect Omega's stub list.
- **Where the modules come from.** The official guide documents one route: FTP the
  `/system/common/lib/` folder off **your own console**, with a server that decrypts. It
  mentions no firmware update file and no other source, and says nothing on legality.
  **No legitimate route without a console was found**, and none is claimed: an update
  file (`PS4UPDATE.PUP`) is encrypted and nothing found shows shadPS4 reading it.
  Never fetch modules from unofficial mirrors. For the online screens, firmware is the
  smaller problem: PSN sign-in cannot succeed on an emulator, so a lobby test needs
  our own server speaking whatever the title connects to, which is a different project
  (the game's room layer is NP Matching2 on Sony's servers).

## 4. Plan: v1 architecture (chosen, not measured)

**Goal (maintainer, 2026-10-16):** a first spike of a server for a remote host, following
Omega's lobby flow where possible; one binary; one concurrent game, everyone else waits.

### Constraints that decide it

- **One UDP port.** The test host filters inbound UDP except one port (60000), IPv6
  only (`[::]`), glibc 2.35 (the README's "Running a server on another machine").
- **QUIC cannot be split across processes.** `oag_net::wt::Listener::bind` owns the port
  and the identity's certificate; a connection is not transferable to another process.
  So a forked race child cannot accept the players the lobby already holds.
- `Host` is a plain struct, taking `Box<dyn Link>`s (`Host::join`/`seat`); `serve::run`
  is a blocking loop. `lan.rs` already moves lobby connections into a `Host`
  (`Lobby::take`, `Host::seat`), in one process.
- A race is loaded (`race::load`: image, track) **before** the host listens. A lobby must
  defer that to START, or keep a warm process.

### The options for "lobby and game"

1. **Same process, race in a thread (recommended for v1).** One `Listener` on 60000, one
   certificate, one hash. The lobby accepts, collects settings, and at START builds the
   `Race` on a worker thread and moves the seated `Link`s into a `Host` there. Queued
   players stay connected to the same endpoint, so the queue is live. A panic is caught
   at the thread boundary and tears the room down, not the lobby; an abort or OOM takes
   both. Put the runner behind one function (settings in, finished-or-failed out) so (2) or
   (3) can replace it.
2. **Fork or exec a race child that inherits the bound socket.** The parent stops
   reading UDP; the child builds its own endpoint over the inherited descriptor with the
   **same certificate** (passed in), so clients re-handshake and the hash is unchanged. It
   gives the crash isolation the maintainer wants and is the shape of "spawn the game
   elsewhere" later. Cost: **the lobby is deaf while the race runs** (one socket, one
   reader), so queued players are dropped and must reconnect and re-queue (a token makes
   that cheap).
3. **Lobby on TCP, race child owns UDP 60000.** Lobby as an HTTPS or WebSocket service
   behind the maintainer's webserver; on START, it spawns `oag-server` for the race, which
   binds 60000 with a fresh certificate, and the lobby sends each client `address + cert
   hash` (what `README.md` already calls "the handoff"). Queue and room state survive any
   race failure; one slot fits the single open port exactly. Needs a TCP port on the host
   and a second protocol alongside `oag-net`'s.

**The later shape** (containers): the lobby allocates a game per room on its own
address and port (a range, or one address each), and hands out `address + cert hash`. That
is (3), with a spawner behind the same message. (2) does not scale to it; (1) does not
isolate.

### Recommended build, in order (the thread's Next Steps)

1. A `Room` model: owner, settings (the `MP Select` table, with the max people 2 to 8),
   members, state `Open -> Countdown -> Racing -> Finished`.
2. `Control` messages (in `oag_net::wire`, v1 edited in place, no version bump
   pre-release): `ListRooms`, `CreateRoom`, `JoinRoom`, `EditSettings`, `Ready`,
   `Start`, `Leave`, `Kick`, the room snapshot pushed to members, and named rejects
   (`Full`, `Started`, `Content`, `Version`, `Blocked`).
3. The content check **before** the race loads: build, title, image fingerprint, mods.
4. The owner role: settings and START only; migrates to the next member on leave
   (Omega does, evidence above); the race stays server-authoritative.
5. One game slot: a FIFO of rooms or players waiting for the slot; a room that wants to
   start while a race runs shows "RACE IN PROGRESS" and its position.
6. The runner seam (1), a thread, with the failure path: a race that dies returns every
   member to the room list with a reason.
7. A scripted client (the existing `--net-bot`) to join, ready and race to the end; then
   `oag-game` front-end screens modelled on Omega's `OnlineMainMenu`, `Game List`,
   `MP Select` and `GameLobby` (with pointer support, per the project rule).

### Open decisions for the maintainer (ranked)

1. **Is a TCP port available for the lobby on the host (for example behind the
   webserver)?** If yes, option 3 is the best v1 (the lobby survives a race crash, the queue is
   live, the race child owns UDP 60000). If no, option 1. Recommendation: ask; build option 1
   behind the runner seam either way, because it is the least code.
2. **Fork versus thread for the race.** Option 1 is a thread, not the fork the maintainer
   described: a fork cannot take the lobby's QUIC connections (above). Recommendation: thread
   for v1; move to a child process when the host opens a second port.
3. **Identity.** Omega has PSN accounts; we have none. Recommendation (chosen): a
   player-chosen name plus a per-session token, no accounts, no password rooms (Omega ships
   them off). The trust model (no client anti-cheat) already allows it.
4. **Which title or titles the server hosts.** Omega's lobby lists the content pack as a
   room attribute (`Game`: HD or 2048). Recommendation: one title per server process in v1
   (the image it was started with), advertised in the room list.

## Sources

- Omega `data09.psarc` front-end XML and `english/entries.xml`, via `scripts/psarc.py`;
  HD's same files from `hdfury-ps3-eu-dec.iso` (`DATA00`/`DATA02`).
- shadPS4 0.20.0 (`/usr/bin/shadps4`), its [quickstart](https://shadps4.net/quickstart/),
  `src/core/libraries/np/np_matching2/np_matching2.cpp` in the repository, and
  compatibility issue 487 (linked above).
- Omega `eboot.bin` v1.00 in Ghidra (`/omega/eboot-ps4-omega-eu.bin`), string searches only:
  no function was read, no name was recovered, `names.tsv` is unchanged.
