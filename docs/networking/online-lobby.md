# Online lobby: Omega's flow, what a server needs, the v1 plan

Research for a first online lobby server (2026-10-16). Part 1 is read off Wipeout
Omega Collection's own front end (PS4, patch v1.07). Part 2 maps it onto
[`oag-net`](README.md). Part 3 is the shadPS4 attempt and the firmware question.
Part 4 is the architecture plan, which is **chosen, not measured**; it has no
confidence scores. Part 5 is what the first spike built, after the maintainer's decisions
of 2026-10-16, which replace part 4's recommendations. Every other claim carries a score from the
[rubric](../reverse-engineering/confidence-rubric.md).

Parts 1 to 4 are research and changed no code. Part 5 is the spike that followed.

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
| Barrel rolls (`OnlineBarrelRolls`, label `MAN_2_BR`) | `FE_ON` / `FE_OFF`, On listed first, no `default` attribute | not authored |
| Pilot assist allowed (`OnlinePilotAssist`, label `FE_PILOT_ASSIST_ALLOWED`) | `FE_ON` / `FE_OFF`, On listed first, no `default` | not authored |
| Laps (`NumberOfOnlineLaps`, `RC_LAPS`) | slider 3 to 20 step 1 | HD's `MP Select` says 3, Omega's does not say |
| Eliminations (target) (`NumberOfEliminations`, label `IG_HUD_TARGET`) | slider 50 to 600 step 25 | 200 |
| Zone target (`NitroTarget`, global `NumberOfNitroLevels`, label `FE_ZONE_TARGET`) | slider 15 to 55 step 5 | 30 |
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

- **HD's `MP Select` rows, read 2026-10-19** (`DATA00.PSARC`, `DATA05.PSARC`, `DATA06.PSARC` of the
  EU disc): the same `OnlineBarrelRolls`, `OnlinePilotAssist`, `OnlineRespawns` (disabled
  `aList`) and `NumberOfOnlineLaps` rows with the same ids and values; `DATA00` also holds
  `Eliminations` and `NitroTarget` (50 to 600 step 25, 15 to 55 step 5 default 30, the same as
  Omega's), `DATA05` and `DATA06` do not. **Checked, same rows**: Omega's room settings are HD's.

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

## 4. Plan: v1 architecture (chosen, not measured; superseded by part 5)

**Superseded (2026-10-16):** the maintainer chose a WebTransport lobby, a forked race child
the clients reconnect to, name plus token identity, every title in one process and exactly
one game slot. Part 5 is what was built; the options below stay as the reasoning that led
to the question.

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

## 5. The spike (2026-10-16, built)

Everything here is **chosen, not measured**: Omega's lobby is the model for the order of the
screens and for the rules, the bytes and the numbers are ours.

### The decisions it follows (maintainer, 2026-10-16)

1. The lobby speaks **WebTransport**, the transport the race already uses: UDP, not TCP.
2. The race is a **forked child** (`oag-server race`, exec of the same binary, not `fork()`:
   forking a process with a live tokio runtime is unsafe) and the **clients reconnect** to it.
   The lobby hands each client the race's **port and certificate hash** (not an address: the
   lobby, bound to `[::]`, does not know its public one; the client combines the host it
   dialled with the port).
3. **Identity** is a chosen name (1 to 24 characters, unique, case-blind) plus a session token
   the lobby issues in its welcome. No accounts. A hello carrying a connected session's token
   and name takes that session over (a reconnect after a dropped link).
4. **All titles in one process**: `--image` is repeatable, each opened with
   `oag_source::title::open_source`. A room is in one title. A client's hello carries, per title
   it can race, an `Edition` (title, the image's platform, a fingerprint of its mods), and the
   **room list shows only rooms it can join**: its edition against the server's by
   `oag_net::discovery::differs`, the one decision LAN discovery's `compat` also makes. A join or
   a room into a title or platform it lacks, or with other mods, is refused `Content` and says
   which. The server's own mods fingerprint is `discovery::NO_MODS` (it loads none);
   `Mods::content_fingerprint` moved onto `discovery::content_fingerprint` so both ends hash alike.
   **The build is compared too** (6 below): a hello from another build is refused by name.
5. **Exactly one game slot.** A room whose owner presses START queues FIFO and is told its
   place (`Queued { position }`, 1 is next). When the slot frees, the front room is
   re-validated (still at the minimum, still everyone ready) and counts down; a room that no
   longer qualifies is skipped with a note, not left blocking.

**After a race the players return to their room** (chosen; the menus were the alternative). The
client's lobby link stays open for the whole race (`oag-game --online` hands the link to a thread
that polls it and prints "back in the room: REASON"), so the lobby can push the queue and the end
of the race, and everyone is back in their room with picks cleared and the race's reason in
the room's note ("every person has finished", "--max-ticks reached", "the race process exited
with exit status: 2: error text", "timed out after 60s waiting for ...").

### Where it lives

| Piece | Path |
| --- | --- |
| Wire messages (tags `0x20..`, disjoint from `Control` 1 to 8 and `Datagram` `0x10`/`0x11`) | `crates/net/src/wire/online.rs` |
| Rooms: every rule, no I/O | `crates/net/src/online/rooms.rs` |
| The service over WebTransport | `crates/net/src/online/server.rs` |
| The race behind a seam: `Runner`, `ChildRunner` (exec), `ThreadRunner` (tests), `run_race` | `crates/net/src/online/race_proc.rs` |
| A lobby client over any `Link` | `crates/net/src/online/client.rs` |
| `oag-server lobby`, `oag-server race` | `crates/net/src/bin/oag_server/` |
| `oag-game --online` | `crates/game/src/online.rs`, `crates/game/src/app/cli/extra.rs` |

`oag-server` with no subcommand is the standalone race server it always was. Rule 6 of
`just check-deps` holds: the lobby is `oag-net` without the `client` feature, and the one new
dependency is `ring`'s random source, already linked.

### The wire messages

Client to lobby: `Hello { protocol, name, titles, token }` (magic `OAGL`), `ListRooms`,
`CreateRoom(settings)`, `JoinRoom { room }`, `Edit(settings)` (owner), `Ready { team, ready, assist }`
(a pick is the ready, as Omega's ship choice is; `assist` is the Pilot Assist level this person flies
with, an index into `Level::ALL`), `Start` (owner), `Leave`.

Lobby to client: `Welcome { token, name, titles }`, `Rooms(list)`, `Room(snapshot)` (pushed on
every change, owner first), `Left { reason }`, `Handoff { host, port, cert_hash, token, name }`
(`host` is empty for the lobby's own host), `Rejected { code, text }`.

Named refusals (`Refusal`): `Full`, `Started`, `Content`, `Version`, `NoSuchRoom`, `NotOwner`,
`NeedPlayers`, `NotReady`, `BadName`, `NotInRoom`, `InRoom`, `BadSettings`, `Kicked`, `Banned`,
`Unreachable`, `BadHost`. The messages of a player-hosted room are in part 12.

Room settings (`RoomSettings`): title, track, class, mode, max players (2 to 8), AI fill, laps,
weapons, barrel rolls, Pilot Assist allowed and the zone target (part 9). Weapons are Omega's
`Weapons` row (`RB_WEAP`, default on) and are predicted simulation ([ADR-0071](../architecture/adr/0071-weapons-are-predicted-simulation-with-a-discrete-state-correction-trigger.md)).
Omega's eliminations target is not carried (part 9).

A game client that dials the lobby (`--connect`) is refused by name ("this is the online
lobby, not a race"); a lobby client that dials a race port gets the race host's own malformed
refusal.

The race hello gained a **seat token** (`Control::Hello.token`, v1 edited in place, no version
bump). A race child started with a job (`Host::expect_seats`) seats each hello by its token, so
seat 0 is the room's owner whatever order the clients reconnect in, and a stranger without a
token is refused by name.

### The rules (rooms.rs, each with a unit test)

- The owner is the room's creator and the next in join order when they leave ("... has been
  made host of this game" is the room's note, Omega's string). Only the owner edits settings
  and presses START.
- START needs 2 people (`--min-players`; Omega's lobby says "at least %d players" without the
  number) and every pick made; a refusal names who is being waited for.
- An edit that changes the settings clears every pick (as `ApplySettings` re-opens the lobby)
  and removes a member who lacks the new title ("picked content that you do not have", Omega's
  `ONL_KICK_CONTENT`).
- Taking a pick back, or dropping below the minimum, while queued or counting down cancels the
  start and returns the room to `Open` with the reason.
- The countdown is `--countdown-ms` (3000 by default); the room's snapshot carries the milliseconds
  left once a second.
- A room whose members all left while its race runs stays until the race is reported over, so the
  slot is not freed early.

### The race child

`oag-server race --image I --listen ADDR --join-timeout S --max-ticks N` reads one hex line
(`RaceJob`: the `RaceParams` plus `(name, token)` per seat) off **standard input**, not argv (and
keeps reading it: the lobby holds the pipe open and its end of file stops the race, part 7, so a
hand-piped `echo LINE | oag-server race` stops at once),
where any local user could read the tokens. It loads the race, binds, prints `listening on ADDR`
and `cert-hash HEX`, seats the people by token, waits for them to load (**`--join-timeout`, 60
s: a no-show ends the race with the names missing, it never holds the only slot**), runs the
race, prints `race-over REASON` and exits 0; a failure prints `error: WHY` on stderr and exits 2.
The lobby reads both pipes on threads for the whole race (a full pipe would stall the child and
look like a frozen race), kills a child that does not come up within `--load-timeout` (120 s) and
kills the child when its handle drops. A race is stopped at `--max-ticks` (30 minutes by default).

### Ports

Two UDP ports, both configurable, both listening on `[::]` (dual stack): the lobby
(`--listen`, **60000**) and the race child (`--race-listen`, **60001**). One race runs at a time,
so one race port. Nothing else is needed; no TCP.

### Run it locally

```sh
cargo build --release -p oag-net --bin oag-server
cargo build --release -p oag-game
target/release/oag-server lobby --image data/images/pulse-psp-eu.chd \
  --listen 127.0.0.1:60000 --race-listen 127.0.0.1:60001 --hash-out /tmp/lobby.hash
# prints: hosting Wipeout Pulse (PSP) ..., lobby listening on 127.0.0.1:60000, race port 60001,
#         lobby cert-hash <64 hex digits>

# Ada makes a room and starts when two are ready; Bob joins the first joinable room.
target/release/oag-game data/images/pulse-psp-eu.chd --no-audio --net-bot \
  --online 127.0.0.1:60000 --lobby-hash @/tmp/lobby.hash --online-name Ada --online-team Feisar \
  --online-create --online-track 'Data\Environments\01_Track\track.vex' \
  --screenshot /tmp/a.png --ticks 600
target/release/oag-game data/images/pulse-psp-eu.chd --no-audio --net-bot \
  --online 127.0.0.1:60000 --lobby-hash @/tmp/lobby.hash --online-name Bob --online-team Qirex \
  --screenshot /tmp/b.png --ticks 600
```

(Drop `--no-audio --net-bot --screenshot --ticks` to play in a window.)

Measured on 2026-10-16 with those commands (debug builds, the lobby run with `--max-ticks 300`) on localhost: both clients welcomed, room made and
joined, countdown 500 ms, the child forked, both reconnected with their tokens as seats 0 and 1,
0 bad state hashes, the race stopped at 300 ticks, and both clients printed "back in the room: --max-ticks reached" while the lobby freed the slot. `--online-room N` joins a named room; `--online-max`, `--online-class`, `--online-mode`,
`--online-start-at` set the room.

### Deploying to a remote host (not done by the spike)

The host needs the binary and one extraction per title, no disc image (part 6).

1. Build: `scripts/build-server-portable.sh musl target/portable/oag-server` (static, any x86-64
   Linux) or `scripts/build-server-portable.sh 2.35 ...` (glibc 2.35). The lobby execs itself for
   races, so the one binary is all that travels.
2. Extract on your own machine, from your own images (part 6):
   `oag-server extract --image data/images/pulse-psp-eu.chd --image data/extracted/vita ... --out data/server`.
   Copy each `data/server/<name>/` folder to the host.
3. Open **UDP 60000** (the lobby) and **UDP 60001** (the race child) inbound. Nothing else.
4. Run `./oag-server lobby --image <extraction> [--image <extraction> ...] --listen '[::]:60000'
   --race-listen '[::]:60001' --hash-out lobby.hash` with the four `XDG_*` variables in a
   directory of its own. The race child inherits the environment. A whole image or an unpacked
   folder works in place of an extraction, so a host that has the disc can use it.
5. Give the players the host name, `60000` and the contents of `lobby.hash`. The lobby's hash
   changes on every restart; the race child mints its own and the lobby passes it on.

### Measured over the internet (2026-10-16)

The deploy steps above, run for real on a remote x86-64 host (1 core, 1 GB) reached over **IPv6 only**, with the musl build and a lean Pulse (PSP, EU) extraction (257 MB on disk, copied in 4 s).
From here the round trip is about 10 ms.

- **Ports.** Echo listeners run as an ordinary user on `[::]`: inbound UDP **60000, 60001, 60002 and 60010
  arrive; 4433, 8443, 40000 and 50000 do not**. The lobby on 60000 and the race child on 60001 need nothing
  opened. The filter is upstream of the host (an unprivileged user cannot read its rules), and a 60000-range
  allow is the likely shape.
- **Race.** Two `oag-game --online --net-bot` clients from here, an IPv6 literal as the host: welcomed, room
  made and joined, 3 s countdown, child forked, both seated by token on 60001, **0 bad hashes** over 600
  ticks and over a 1,800-tick run to the lobby's `--max-ticks`, after which both printed "back in the room:
  --max-ticks reached" and the lobby freed the slot. QUIC rtt 9.3 to 16.8 ms (mostly 10-11), loss 0.0%,
  jitter 1-6 ms, rx 49 KB/s and tx 10 KB/s per client at steady state, 0 to 2 corrections a minute. The first
  second stalls (confirm latency 2 s at tick 30, 70-100 ms by tick 335) while the debug-profile clients
  finish loading.
- **Server cost** (2 people, no opponents, no weapons): the race child 9.5 MB resident and 15-20 % of the one
  core, the lobby 6 MB and 1-4 %. One race at a time fits easily on this host.
- **Build string.** A client built with another `OAG_GIT_HASH` is refused with the named text. This found a
  bug: the lobby hangs up right after a refusal, `LobbyClient::poll` returned the close before the caller read
  `refused`, and the client printed "the lobby closed: ... bye" with no reason. `poll` now hands the refusal
  over first (`a_refusal_that_arrives_with_the_close_is_still_read`). The flake that remained on a real
  socket was in the transport: a dropped `WtLink` closed the session at once and discarded the refusal
  still in flight, and the datagram reader could report the close ahead of the reliable queue. A link now
  finishes its stream and waits up to 1 s for the peer to acknowledge it before closing, only the reliable
  reader reports the close, and a poll reads the close before it drains the queue.
- Everything was removed from the host afterwards (processes killed by pid, files deleted, listing checked).

### Measured over the internet (2026-11-17)

The same deploy and the same measurements as above, a month later, on `main` at `f442d0dc5` (which has the `net-v1`
limits: a seat buffers under 120 future inputs, one resend answer a second, a silent race child stopped after 5 s).
The remote host is the same kind (x86-64, 1 core, about 1 GB, shared with other services) and was reached by its **host
name**, which resolves to the host's own IPv4 and IPv6 addresses (no CDN in between, `server-roadmap.md`, "The
connection path and network floods"). The server was the musl build (zig 0.17.1 from the `ziglang` wheel as the C
compiler) with the same lean Pulse (PSP, EU) extraction (184 MB), run as in the deploy steps with `--max-ticks 1800`.
From here the round trip is about 10 ms.

- **Race.** Six pairs of `oag-game --online --net-bot` clients (a lobby, a room, a 3 s countdown, a child on 60001, each
  seat by token) through the host name on UDP 60000 and 60001: four pairs over **IPv6** (the client picks the AAAA
  record first when this machine has IPv6, shown with `strace` as `sendto` to `AF_INET6`) and two over the host's
  **IPv4 literal** (a name cannot be forced to IPv4 from here). **0 bad hashes in all twelve clients**, all six rooms
  freed their slot at `--max-ticks reached` (the client prints "back in the room" in only some runs because it exits at
  its own `--ticks`; the lobby log is the record). The lobby log has no warning, refusal or limit line over the six races.
- **Network** (steady state from tick 300, per client): QUIC rtt 9.1 to 17.9 ms (mostly 10-11), loss 0.0 %, jitter 0.0 to
  11 ms (a client with 7-11 ms in two of twelve, the rest 0 to 6), rx 50.1 KB/s and tx 8.1 to 10.3 KB/s, 0 to 6
  corrections a minute (cumulated over a 30 s race, so coarse). The first second stalls as in October (confirm latency
  about 0.9 s at tick 30). IPv4 and IPv6 rows are the same within the spread.
- **Against October** (rtt 9.3-16.8 ms, rx 49 KB/s, tx 10 KB/s, 0-2 corrections a minute, loss 0.0 %): the same row for
  row, so the limits cost nothing measurable on a well-behaved pair. rx is 2 % higher and tx slightly lower; the
  corrections range is wider because the October figure was read from the periodic lines, not cumulated.
- **Server cost** (2 people, no opponents, weapons on): the race child 8.1 to 9.0 MB resident and 11 to 15 % of the one
  core averaged over the race (peaks of 26 to 43 % while it loads), the lobby 2.5 to 3.4 MB and 1 % (October: 9.5 MB,
  15-20 %; 6 MB, 1-4 %). Sampled with `top` every 5 s on the host.
- **A name behind the CDN proxy does not reach the game ports.** A client pointed at a CDN-proxied name on port 60000
  prints `Error: connecting to <proxied name>:60000: no answer in 5s` after 5.1 and 5.4 s (two tries).
- **Client build.** The clients were debug-profile builds with `debug-assertions` off. With them on, a client in any
  online race panics at about tick 30 in `debug_assert_eq!(recording.base + recording.poses.len() as u64, tick + 1)`
  (`crates/raceplay/src/replay/recording.rs`, the end of the lap recorder), both seats, on a local lobby on localhost too (12 vs 13
  there, 33 vs 68 against the remote host): a hypothesis (not read out of the code, about 55 %) is that the recorder
  assumes contiguous ticks and the online client, which stalls and replays ticks, does not give it that. Not a network
  finding; the simulation hashes are unaffected (0 bad). A release build never evaluates the assertion.
- Everything was removed from the host afterwards (the one lobby killed by pid, its directory and a stray pid file deleted,
  the home listing checked).

### Measured over the internet, with the limits (2026-11-17)

The same deploy and measurements a third time, on `main` at `091a66175`, which has `net-v1b`'s limits (the lobby's 64
connections and 16 per address, the race host's 48 waiting for a hello, bounded transport queues, a datagram budget of
120 a second with a burst of 120 per link and direction, one repeated warning a second). The remote host is the same kind
as before (x86-64, 1 core, about 1 GB, shared with other services), reached by its host name over the same path; the
server is the musl build with the lean Pulse (PSP, EU) extraction (184 MB), run with `--max-ticks 1800`. The clients were
**release** builds (`oag-game`, `--no-audio --net-bot`, one `--release` build with `debug-assertions` off).
From here the round trip is about 10 ms.

- **Race.** Five pairs and one four-person room of `oag-game --online --net-bot`, name by name through UDP 60000 and
  60001: **0 bad hashes in all fourteen clients** (every `net:` line, `bad 0`), all six rooms ended at `--max-ticks reached`
  and freed the slot. The lobby log (the race child writes into it too) has 16 lines in all: the start banner and, per
  race, one `races:` and one `race is over:` line. **No warning, refusal, budget, cap or queue line.**
- **Network** (per client, `net:` lines from tick 300, one a second-ish): QUIC rtt 9.2 to 14.8 ms (mostly 10-12), loss 0.0 %,
  jitter 0.0 to 11.1 ms, rx 50.1 KB/s and tx 7.8 to 10.1 KB/s in a pair; in the four-person room rx 77.9 KB/s and tx
  9.6 to 9.7 KB/s. Corrections cumulate to 1 or 2 over a 30 s race in a pair and 1 to 4 in the room of four. The first
  second stalls as before (confirm latency about 0.9 s at tick 30, 66-83 ms by tick 200 and 80-100 ms after).
- **Against the earlier rows** (2026-11-17, before the limits: rtt 9.1-17.9 ms, loss 0.0 %, rx 50.1 KB/s, tx 8.1-10.3 KB/s,
  0-6 corrections a minute): the same row for row, so the limits cost nothing measurable on a well-behaved pair.
- **Datagram rate against the budget.** The client prints UDP datagrams a second (`rx ... d/s`, `tx ... d/s`, the transport's
  packet counters, so an upper bound on the application datagrams the budget counts: it includes acknowledgement-only
  packets). Over the 14 clients: **tx mean 60 to 73, peak 79 a second; rx mean 61 to 71, peak 92 a second** (the peaks are
  the first seconds, while the client catches up). Against 120 a second that is **at least 34 % headroom on tx and 23 % on rx
  at the sampled peak, and about 45 % at the steady mean** in a pair. In the room of four the means are rx 66 to 71 and tx 63
  to 68, peaks rx 77 and tx 77: **four people cost no more datagrams than two** (a snapshot carries more bytes, not more
  datagrams), so the headroom at 4 is the same, rx 78 KB/s instead of 50. The sampling is one reading per second, and a
  burst inside a second is absorbed by the bucket's 120. An over-budget datagram is dropped **silently** (`wt.rs`, the
  reader task has no counter and no log), so the lobby log's silence alone does not prove none was dropped; the proof is
  the client counters above (the budget is read on both ends) together with 0 bad hashes and no stalls after the first second.
- **Server cost** (sampled with `top` every 4 s and `/proc/<pid>/status` every 0.25 s on the host): the race child 9 to 13 MB
  resident and 11 to 17 % of the one core averaged over the samples of a race in a pair, 19 % in the room of four (peaks of 67 to 81 %
  of the core in the first seconds, while it loads), the lobby 3.4 to 5.3 MB and 1 to 1.5 %. **Two load transients are larger than the
  steady figure: the race child peaked at 542 MB resident (`VmHWM`; 462 MB at a `top` sample 0.7 s after it started) while
  it loaded the circuit, and the lobby's `VmHWM` was 380 MB** (its start, reading the extraction). The host showed about 640 MB available
  with 2 GB of swap and was never short, but a second load on top of a busy host would be. **Both peaks were the
  extraction's sparse archives read whole, fixed the same day: 18.5 MB race load and 9.7 MB lobby start**
  (part 6, "Server memory while loading").
- **A name stays taken for a while after a race.** A pair that reused the previous pair's `--online-name` within about a
  minute was refused `BadName: that name is in use` (twice: the held session, `SEAT_GRACE` 30 s, plus the lobby's time to
  free it, is the likely cause, a hypothesis of about 55 %, not read out of the code). Not a limit firing on a pair; the
  runs that followed used one name per run.
- Everything was removed from the host afterwards (the one lobby killed by pid, its directory deleted, the home listing
  and the free ports checked).

### Tests

- `wire::online` and `online::rooms` unit tests (round trips, every rule above).
- `tests/online.rs`, no disc, over real WebTransport on localhost: two rooms race one after the
  other through one slot (the second is told it is queued, then hands off only after the first
  race is over), reconnecting in the wrong order still seats by token, 0 hash mismatches, the room
  returns open with the reason; a child that crashes after coming up (the fake child waits at a gate file the test opens once it has
  seen the hand-off, not a `sleep`: a child that exits before the lobby's poll reads it is `Over`, never `Up`, so
  a pump stalled past the sleep lost the hand-off - it failed once under load on 2026-10-19; and the test waits for both members' view of the returned room, not only the owner's, before reading the guest's note; with the gate
  removed the test now fails every time, which is what proves it is wired); a child that never comes up
  is killed; a title with no image refuses; a game client on the lobby port is refused by name.
- `net_ground_truth::the_lobby_forks_a_real_race_...` (`#[ignore]`, `just test-data`): the real
  `oag-server lobby` forks the real `oag-server race` on a real circuit, one room. **Coverage is
  honest, not complete:** this part's races ended at their `--max-ticks` limit, not at a
  finish line (part 7 races one to the line), and the second queued room races only through `ThreadRunner`; the real child is
  covered by the one-room test.
- `discovery` unit tests: `differs` and `content_fingerprint`; `online::rooms`: a PS2 pressing or
  other mods is hidden from the list and refused.

### Open

- **Menu screens: landed** (part 8), walked live on Pure, HD, 2048 and Omega (Pulse by the earlier
  lane, plus its certificate keyboard and room stills on this build), with the owner's room settings
  and size. Kick landed in part 11; open there: the other
  sub-pages Omega has beyond the list (part 9 lists which of them are settings and are done).
- **EU and USA pressings of one platform's title are not told apart before the race** (the
  same rule as LAN); the tick-0 hash catches them, as a join timeout.
- ~~A session takeover does not re-send a pending hand-off~~ (fixed, V1 of the server roadmap: the
  lobby keeps where the race listens and sends the hand-off again to a session taken over after it
  went out; `online::a_client_that_drops_while_its_race_loads_and_takes_its_session_over_receives_the_handoff`).
- ~~A SIGKILLed lobby orphans its running race child~~ (fixed, part 7).
- **No spectators, no chat, no quick match, no invite-only**; Omega has some of them (kick is part 11).
- **One race slot is per server**, so a second title waits behind the first. The containers shape
  (a game per room on its own port) is the same `Runner` seam with a spawner behind it.
- **AI fill** (`RaceParams.opponents`) only matters in modes without opponents: the grid is
  full when the flag is set or the mode has opponents (`oag-raceplay` `load.rs`, `full_grid`), so
  a `single_race` room always has AI whatever the room says. `--online-create` hardcodes `false`.
- Omega and 2048: not a format finding. The lobby carries a title's name, so those titles are
  hostable as soon as a race on them loads through `oag_net::race::load`; Omega's race is
  incomplete (`omega-status.md`), so that stays **checked, applies, not wired**.

## 6. The server extraction and the build string (2026-10-16)

### The extractor

`oag-server extract --image <image or folder> [--image ...] --out data/server` keeps only what the
races a server can offer read of a source, one folder per image (named after it). The folder opens
in place of the source wherever the server takes `--image` (`oag-server lobby`, `oag-server race`,
`oag-server`), so a host needs the extraction and never the disc. **Server only**: a client
(`oag-game`) neither opens nor recognises one (`oag_source::title::open_server_source`, reached
through `Options::server_source`, which only `oag_net::race` sets). **Never commit one**: it is the
source's own content, sliced (`docs/overview/legal.md`); it lives under `data/`, which is ignored.

**How it records.** Every archive reader sits on one lazy `(offset, len)` source
(`oag_assets::blob_source`), so noting the ranges there sees every read a load makes, by every
path: entry reads, hash reads, table-of-contents parses, the presence probes (a probe reads the
table). `oag_assets::record` does that, off by default (one relaxed atomic load per read), and
`oag_assets::sparse` is the stand-in file: the archive's full length and the bytes of just the
recorded ranges, under the archive's own relative path. A reader opened on it behaves as on the
whole archive for every recorded read and fails **by name** (`NotExtracted`) for any other; it
never answers with zeros. The same reasoning is why the format is not a pruned WAD or PSARC: a
pruned archive would change the answers to "is X here" and "list the entries", and would need
writers for both formats.

The identity files that tell a folder's console apart (`UMD_DATA.BIN`, `SYSTEM.CNF`, `PARAM.SFO`)
and any loose file a load reads are copied whole. `oag-extract.txt` (tab separated) names the
title, platform, serial, source file name, the source's SHA-256, the extractor's build string, what
was swept and every file with its length, kept bytes and SHA-256. `oag_source::title::open_source`
reads the title out of it: a Pure extraction would otherwise open as Pulse, the two ship archives of
identical names and a folder has no serial.

**What it loads.** The lobby fixes difficulty (`elite`), scheme (`veteran`) and, by default, weapons (on) of
every race, so the axes are a room's circuit, class and mode, the number of people and each seat's
team. `oag_net::extract::plan` walks each once with the rest at defaults (every circuit, every Zone
circuit, every team alone and as a full grid of eight in rotation, no opponents), then crosses every
class with every mode on two circuits. The offer is read from the title's own definitions
(`catalogue::documents`, `catalogue::raceable_teams`, the one team filter the menu also uses). After
writing, `extract` loads the **whole plan again from the extraction** and requires the same tick-0
state hash for every race that loaded from the source; a mismatch is a `GAP` and a non-zero exit.

**Not offered, so not extracted:** downloadable content (the server's race options carry no pack
folder) and the player override layer; a team or race the source itself refuses is listed as such
(Omega's guest roster of 2048-era craft needs a hull variant `RaceParams` cannot carry yet, and
the Zone variant of a circuit that has none).

**Sizes** (the 6 EU sources on this machine, 2026-10-16; what is kept of what the source holds):

| Source | Kept | Of | Races loaded from the extraction | Time |
| --- | --- | --- | --- | --- |
| Pulse PSP (`pulse-psp-eu.chd`) | 182 MiB | 307 MiB | 109, 0 gaps | 69 s |
| Pulse PS2 (`pulse-ps2-eu.chd`) | 109 MiB | 361 MiB | 183, 0 gaps | 81 s |
| Pure PSP (`pure-psp-eu.chd`) | 56 MiB | 235 MiB | 102, 0 gaps | 53 s |
| HD (`hdfury-ps3-eu.iso`) | 41 MiB | 1,879 MiB | 189, 0 gaps | 30 s |
| 2048 (`data/extracted/vita`) | 5 MiB | 2,303 MiB | 109, 0 gaps | 19 s |
| Omega (`data/extracted/ps4`) | 40 MiB | 47,091 MiB | 237, 0 gaps (40 more are races the source refuses too) | 45 s |

The AI's teams are drawn from the title's whole declared pool by the race's seed, and the
lobby's seed is arbitrary, so the plan includes seeds that put every pool team on a grid slot
(`covering_seeds`); without them a race with another seed read an AI team's handling and engine
sound the extraction lacked (found by the ground truth, which loud-fails on any such read:
`record::take_misses`).

**Build it lean, or the extraction is not** (`oag-server extract` refuses to run in a build with `view` on, naming the right command). These numbers are from `cargo build -p oag-net --bin
oag-server` alone. Built in one cargo invocation with the crate's tests or the game, the
workspace's unified features turn `oag-raceplay`'s `view` on in the same binary, the load then
reads textures, meshes, animation clips and sound banks, and the extraction grows: HD to 1,072 MiB
(540 MiB `.gtf` lightmaps and crowd textures, 392 MiB `.rcsmodel` track meshes, 105 MiB `.vex`,
28 MiB sound banks) and 2048 to 288 MiB (100 MiB `.gxt`, 79 MiB `.rcsanimclip`, 48 MiB sound
banks, 30 MiB materials, 20 MiB `.pvs`); an Omega race load then decodes `.gnf` textures (1.3 to
2.6 s and 1.7 GB a race). `scripts/build-server-portable.sh` builds the binary alone. The
ground-truth tests run with `view` on, so their extractions are supersets of a lean server's.

A race reads whole entries (a circuit's `.vex` carries its geometry, collision and textures in one
compressed blob that the lean load reads whole), so the two PSP titles and the PS2 keep most of
their circuits and ships. Reading only the collision, spline and pad sections of a `.vex` is a
change to the load path, a follow-up, not to the extractor.

### Server memory while loading (2026-11-17)

**A race child or lobby opened on an extraction held each sparse archive whole in memory.**
`BlobSource::open_file` read the whole `Data.wad` stand-in (192.7 MB for Pulse PSP EU) with
`std::fs::read`, and `Sparse::parse` copied its data section out of that buffer, so opening it
cost twice the file at once and kept one copy for the life of the process. The lobby opens a
title's archives once at start (to name the title), the race load opened `Data.wad` twice:
**383 MB lobby, 572 MB race child**, against 13 MB and 24 MB opened on the `.chd` itself. That
is the 380 MB and 542 MB measured on the remote host (part 5, "Measured over the internet, with
the limits"). It also broke `oag_assets::blob_source`'s own rule that no container is slurped.

**Now `Sparse` keeps only the header and range table** and reads each range from the file when
asked, like the plain-file source; a read outside the recorded ranges still fails by name
(`NotExtracted`). Nothing a reader is answered changes: the tick-0 state hash of the default
Pulse race is `0xc4430b77e8e80ed0` before and after, and the same from the `.chd`.

Measured with a lean release `oag-server` (`cargo build --release -p oag-net --bin oag-server`
alone, glibc), on a lean Pulse PSP EU extraction (184 MiB), `VmHWM` from `/proc/<pid>/status`
read once the process printed its ready line, three runs each (load average about 30, which does
not move a memory figure: runs agree within 0.4 MB):

| Process | Before | After |
| --- | --- | --- |
| Race load (`oag-server --image <extraction>`, default circuit, 8 craft) | 571.8 to 572.1 MB | **18.4 to 18.8 MB** |
| Lobby at start (`oag-server lobby --image <extraction>`) | 382.5 to 383.0 MB | **9.7 to 9.8 MB** |
| Race load from the `.chd`, for reference | 23.8 MB | 24.1 MB |

The standalone `oag-server --image` and the lobby's race child (`oag-server race`) load through
the same `oag_net::race::load`. heaptrack before: 385.5 MB peak from `sparse.rs` (the data copy,
two archives) and 192.7 MB from the `std::fs::read` in `blob_source.rs`, 578 MB peak heap in all;
after: 11.4 MB peak heap for the race load and 0.2 MB for the lobby. Most of what is left is
the circuit's `.vex` entry (5.1 MB, read whole, see "The extractor" above), held twice at once:
`oag_raceplay`'s track load reads it once for the AI line and again for the collision
(`load/track_inputs.rs`), a further 5 MB a later change could save. The musl build
the host runs allocates the same buffers; its figures were not re-measured remotely.

Held by `oag_assets::sparse`'s `an_archive_on_a_sparse_file_reads_the_file_at_each_read`: it
rewrites an entry's bytes on disk under an open archive and requires the next read to see them,
which a copy taken at open cannot.

### The build string

`oag_net::BUILD_VERSION` (`0.1.0 (abc1234)`) is the one build string, from `crates/net/build.rs`
(`OAG_GIT_HASH`, else `git`, else `unknown`; `OAG_VERSION_SUFFIX`), moved there from `oag-game`'s build
script so the server and the client read one definition; `oag-game` takes `oag_net::BUILD_VERSION`
and `oag_net::GIT_HASH`. `just test-data` sets `OAG_GIT_HASH=test-data`, so a commit does not rebuild
every binary. The lobby's `Edition` carries it (`oag-server lobby` states its own, `oag-game --online`
its own), and `Rooms::connect` refuses a hello whose build differs on a title the server hosts, by name
and with both builds: *"this server is build X, yours is Y: races only match between the same build"*
(`Refusal::Version`), where it was a 60 s join timeout at the race child's tick-0 hash. `discovery::differs`
(which also decides LAN `compat`) compares it, so rooms and joins use the same rule. `oag-server
--version` prints it, and the extraction manifest records the extractor's.

## 7. Racing to the finish line, on every title (2026-10-16)

Everything here is **chosen, not measured** unless it says otherwise: the original has no network race.

### What was already true, and what was missing

The race child already ended a race when every person had finished (`serve::run`, `Host::finished`)
and every client already derived the results board from its own confirmed state, but every test
raced to a tick limit, so nothing had ever run a finish line. Running one found three things, all
fixed.

1. **No lap count.** A room had no laps, so the shortest race was the mode's own (3 to 5 laps).
   `RoomSettings.laps` and `RaceParams.laps` (v1 edited in place; `0` is the mode's own) reach
   `Options::laps_override` through `oag_net::race::options`, the one function both ends use, and
   **only in a mode that ends on laps** (`race::mode_has_laps`: single race, tournament, head to head,
   time trial; Speed Lap, Zone and Eliminator never end on laps and `laps_override` must not be set
   for them). A room's value is capped at 20 (Omega's slider) and zeroed in the other modes. `oag-server
   --laps`, `oag-game --online-laps`.
2. **A leaver held the finish open.** A seat whose person left kept repeating its last input and
   `RaceSim::finished` still counted it as a person, so the race ran to the tick limit (30 minutes). Now
   `TickRecord.retired` (a cumulative seat mask, one byte a record) carries the departure: from the
   record it first appears in, **both ends** set that slot's controller to `Away`, the opponent
   driver below racing pace with the pickup kept (`race::retire`; a returning person is handed the
   craft back, `docs/networking/server-roadmap.md`, "V1") before the tick runs, so the controllers (which are hashed) agree and the end rule
   counts the remaining people only. The owner (seat 0) and a joiner both tested. A seat that sends
   nothing for 8 s while racing is dropped the same way (`Host::drop_silent`): a vanished player sends
   no close frame, and QUIC's idle timeout alone is 30 s; the dropped seat is sent an `End` with the
   reason (a player who only stalled is taken back to the menus). The retired craft **is flown by
   the AI driver** (`net_ground_truth::a_retired_person_s_craft_is_flown_by_the_ai`: an unretired
   person who sends nothing sits at speed 0, the same craft retired at tick 300 is at 137 u/s 600
   ticks later). The client applies the newest mask to its present simulation too, because the game
   asks the present whether the race is finished (`loopback::a_person_who_leaves_...`).
3. **A server running from an extraction raced differently from the clients after the finish.** Pulse
   PSP's track stats were opened with `"<source>:<archive>"`, the image form, which a folder (an
   extraction) cannot answer: the read failed with an i/o error, the AI skill scale fell back from 3.10
   to 2.00, and the **finished** craft's thrust cap (the post-finish law, `finished_thrust`) came out at
   62.3 instead of 68.9, so the server's state hash disagreed with both clients' from about 4 s before the
   end, and for ever after. No tick-0 hash could see it, and the extractor's miss list is silent about
   it (an i/o error is not a `NotExtracted`). `track_stats::read` (and the campaign's skill lookup) now
   build the spec with `oag_assets::source::archive_spec`, which joins a folder with a separator. The
   extraction ground truth (`extract_ground_truth.rs`) flies a third race on every title, one person
   for one lap with the autopilot standing in for the clients, **to the finish line**, from the source
   and from the extraction, and compares every tick; it failed on Pulse PSP before the fix and passes
   on all six after it. The race child now also names any extraction read it lacks as it happens
   (`warning: the extraction lacks a read the race made`, on its log and standard error).

### The standings

The results the clients show are the **confirmed board**: each client builds it from its own
confirmed state, which the tick hash proves equal to the server's, and `Client::advance_confirmed`
recaptures it from the confirmed state the first time it is finished. Nothing is sent: the server's
`End` carries only the reason. Both clients' boards were identical in every run below. What a client
shows, **seen** in a windowed `--online --net-bot` client (Xvfb, software Vulkan, Pulse PSP, one
lap): the in-race view to the line, then the built-in results table ("race complete", eight
rows with the two people marked P1 and P2, laps, best lap, time, "x or escape leaves the race"),
the one a `--race`-style launch shows (no EndRace screens are built on this path, which has no
front end); `--net-bot` headless prints the board. The 1280x720 window clips the table's last
column, which is the results table's own layout. Returning to the room is the
menus lane's (the `--online` flag path prints `back in the room: REASON`). The room note carries
the reason; the places travel too since part 11 (`End` carries the board, the room screen shows it).

### The orphaned race child

The lobby now keeps the child's standard input open for the whole race (it was closed right after
the job line), and the child treats that pipe's end of file as "the lobby is gone" and stops
(`oag-server race`: `lobby_watch`; it prints `error: the lobby stopped the race` and exits 2). The
kernel closes the pipe when the lobby dies for any reason, SIGKILL included, so no `libc` call is
needed. Measured: a lobby SIGKILLed 200 ticks into a race, its child gone within 3 s (without it the
child ran on to the end of the race or the join timeout), so the race port is free for a restarted
lobby. `net_ground_truth::a_race_child_stops_when_its_lobby_is_gone`.

### Every title, localhost

Per title: the real `oag-server lobby` on a **lean extraction** of the server's image (build `oag-server`
alone, `oag-server extract`), two `oag-game --online --net-bot` clients on the **full image** making
and joining a room (one lap, the title's default circuit, class VENOM), the lobby forking the real race
child. Debug builds; 6 titles. "ok/bad" is the sum of the clients' per-second hash counters.

| Title (client image, server extraction) | Tick-0 hash, both clients | ok / bad (client A, B) | Humans crossed at | Race ended at tick | Extraction |
| --- | --- | --- | --- | --- | --- |
| Pulse PSP EU | equal | 2853 / 0, 2853 / 0 | 2540, 2601 | ~2857 | 182 MiB |
| Pulse PS2 EU | equal | 2801 / 0, 2800 / 0 | 2559, 2582 | ~2805 | 109 MiB |
| Pure PSP EU | equal | 2817 / 0, 2818 / 0 | 2566, 2590 | ~2820 | 56 MiB |
| HD (PS3 EU) | equal | 2876 / 0, 2874 / 0 | 2638, 2647 | ~2881 | 41 MiB |
| 2048 (Vita folder) | equal | 4866 / 0, 4869 / 0 | 4058, 4651 | ~4871 | 5.6 MiB |
| Omega (PS4 folder) | equal | 2822 / 0, 2823 / 0 | 2568, 2579 | ~2826 | 41 MiB |

All six ended with `every person has finished`, both clients printed an identical board and `back in
the room: every person has finished`, and the lobby freed the slot. Pulse PSP's first attempt in this
table had 49 bad hashes (item 3 above); the row is the rerun after the fix from a fresh extraction.
The AI field fills the grid whatever the room says (the `opponents` note in part 5): the boards list
eight craft. 2048's circuit is slow (a lap is about 4,000 ticks).

Leavers, Pulse PSP, same setup (the leaver's seat is dropped by the server after the 8 s silence): the joiner killed (SIGKILL, no close frame) at tick 600 and, in
another run, the owner killed: the race ended `every person has finished` when the remaining person
crossed the line (tick ~2800), 0 bad hashes on the survivor, the leaver's craft on the board unfinished.

### Tests

- `loopback::a_person_who_leaves_is_retired_on_both_ends_and_the_hashes_agree` (no disc): the record
  carries the seat, both controllers flip, the survivor's hashes agree on every confirmed tick.
- `loopback::a_silent_seat_is_dropped_and_told` (no disc): `drop_silent` drops, tells and retires.
- `race::laps_are_set_only_for_a_mode_that_ends_on_laps`, `rooms::laps_are_kept_only_for_a_mode_that_ends_on_laps`.
- `net_ground_truth::the_lobby_forks_a_real_race_...` now races **one lap to the finish line**:
  both clients' state is finished with a one-lap board, the phase is `Over("every person has
  finished")`, the room returns open with that note (43 s). Before, every lobby test ended at a limit.
- `extract_ground_truth` (above) and `net_ground_truth::a_race_child_stops_when_its_lobby_is_gone`.
- A finish line needs a real circuit, so there is no disc-free end-to-end test of one; the disc-free
  layers are the retirement and the lap gating above.

### Open

- ~~Session takeover does not re-send a pending hand-off~~ (fixed, part 5).
- ~~The room note carries the reason only~~ (part 11: the board travels in `End` and the room).
- The windowed client's return from the results table to the room is the menus lane's: the table
  says "x or escape leaves the race" and the `--online` flag path has no room screen to go back to.
- Omega's laps slider is 3 to 20; a room here may ask for 1 (chosen, for a short race).
- 2048 and Omega: this is not a format finding. The finish and retirement code is title-blind and
  ran on both (**ported**, with the table above as the ground truth); the track-stats fix concerns
  Pulse PSP's `FEData.wad` only (`track_stats::read` reads nothing on any other title,
  **checked, differs**).

## 8. The online menus (2026-10-16)

**Ours, chosen, not measured** except where it says Omega's. Omega's order, with our login:
MULTIPLAYER, ONLINE, a connect step, CREATE GAME or JOIN GAME, the settings (CREATE) or the room
list (JOIN), team selection, the room. Code: `crates/game/src/app/session/online.rs` (the state
machine, native only; `online_web.rs` is the browser's no-op), the rows in
`crates/game/src/online_screen.rs` (shared with the stills), the screens
`oag_ui_screens::join` in its list and lobby forms (the slot LAN uses, `stage.lan`, only one of
the two is ever live), the pages in `assets/ui/menu.toml` (`online`).

| Step | What the player sees | Notes |
| --- | --- | --- |
| ONLINE (MULTIPLAYER page) | three on-screen keyboards in turn: LOBBY ADDRESS, LOBBY CERTIFICATE, YOUR NAME | remembered in `settings.toml` `[online]` once the lobby welcomes, so the next visit is OK, OK, OK; the name starts as the login name |
| connecting | CONNECTING... on the list screen with only BACK | the dial runs off the frame thread (`oag_thread::Task`) |
| refused or unreachable | the address keyboard again, a line under the text | the server's own text, e.g. the refused build with both builds named; `Rejected` is read before the closing link so it is never lost to "the lobby closed" |
| ONLINE page | CREATE GAME, JOIN GAME, BACK | pushed on the welcome; BACK drops the connection |
| CREATE GAME | the RACE page, Track Select, Ship Select, then the room | Head to Head is dimmed (two people in a room of up to eight) and a stored one becomes a Single Race; Eliminator is open since weapons are online (2026-10-19). A new room seats 8; its owner changes that in the room. AI fill follows the RACE page. A title with no Ship Select (2048) has the pick on the RACE page, and a joiner who never saw one sends the title's own default team (`oag_title::RaceDefaults::team`) |
| JOIN GAME | Ship Select, then the list: owner left, title and head count right, the circuit, class and mode (the title's own circuit name) under the title while the cursor is on it | rows the server marks not joinable are dimmed with the reason; the empty row makes a room instead; the list is asked for again every second |
| the room | one seat per person up to the room's size, `NAME (HOST)  READY` or `PICKING`, START RACE for the host, BACK leaves; a seat says the room's circuit, class and mode while the cursor is on it. **The owner also has ROOM SETTINGS** (the RACE page, Track Select and Ship Select again, seeded with the room's own mode, class and circuit; START sends `Edit` and the room is back; BACK leaves it as it was) **and MAX PLAYERS** (confirm or a click one more, left and right one either way, wrapping between the people already seated and 8; the server holds the number and asks the latest to join to go if it ever shrinks below the head count). An edit clears every ready and the pick is sent again by itself | the pick is the ready (sent once in the room, and again if an edit clears it); the line under the title is the room's note, then where the room is: the host's prompt, "number N in line" while another game races (`Phase::Queued`), the countdown |
| the race | the ordinary network race (`Pending`, tick-0 check, `launch_race`) with the seat token | the lobby link is polled in every stage and kept open for the whole race |
| after the race | the room again, with the room's note ("--max-ticks reached", a crashed child, ...) | `Session::online_menus_opened`, called as the menus reopen |

**Notices** (all on screen, none only in the log): lobby unreachable or build refused (the
connect keyboard, with the text), a join refused (room full, started, no such room: the line under
the room list for five seconds), a refused START (not enough people, someone has not picked), the
host kicked or the room gone (`Left { reason }`: a screen of its own with BACK), the lobby closing
mid-flow (the same), the race child failing to come up (the room's note after the return).

**The address keyboard has a colon and brackets** (`ADDRESS_KEYS` gained `:[]`, a fifth row, shared
with LAN's address keyboard), so `host:60500`, `[::1]:60500` and a bare `::1` (bracketed for the
dial) are typed on the pad; no port means 60000. **Chosen, not measured:** the room's size is
2 to 8 for every title (`oag_net::wire::online::MAX_PLAYERS`, Omega's eight rows); no title
crate carries a measured grid size, so none is capped lower. Weapons, laps, barrel rolls, Pilot Assist and the zone
target are rows on the room screen (part 9).
A 64-digit certificate is drawn on two lines, readable on every face, where it ran off the panel
on every title. `--online` stays the headless command-line flow of part 5 (it does not open the
screens).

**Walked live, 2026-10-16** (`online-menus-titles`): Pure, HD, 2048 and Omega
each from a fresh profile's front end through the three keyboards, CREATE, JOIN, ready, START, the
hand-off, the race to `--max-ticks 600` and both clients back in the room (Omega's clients crawl
on software Vulkan and took minutes to reach the end; they did). Found and fixed on the way: the
certificate ran off the keyboard panel on every title; 2048's joiner never became ready (its
stored team is not in the roster and there is no Ship Select to pick one); HD and Omega's room
said "PLAYER 2 PRESS START TO JOIN" for a seat somebody arrives at over the network (the disc's
local-split line, `FE_GUEST_JOIN`; **LAN's host lobby had the same line and loses it too**). Checked, not
changed: a fresh profile's MODE is Time Trial, which with two people is Pulse's measured multiplayer
Time Trial (`oag_race::Mode::is_multiplayer_time_trial`), so it is a legitimate room; circuit names
on 2048 (`square`) and Omega (`16_Track`) are the raw ids because the RACE and Track Select pages
of those titles show the same ids (the room line and the race loaded the same circuit); 2048's
TEAM row shows a clipped raw craft id, a general front-end matter. The pointer was walked on Pure and
HD: the list, the room, MAX PLAYERS, ROOM SETTINGS and BACK. **A full room is twelve rows for its
owner** (eight seats, ROOM SETTINGS, MAX PLAYERS, START, BACK), which ran START and BACK off the
picture on both PSP titles: the join screen now closes its rows up to fit above the footer
(`Join::layout`, shared by the drawing and the pointer's hit-test, down to 0.4 of the usual
size), checked in stills of an eight-person room on Pulse, Pure, HD and 2048 and in a test
that clicks BACK. The room's note wins over the START line when the two do not fit one line,
because the note is where a failed race says why.

**Earlier, 2026-10-16** (Pulse PSP EU, a debug build, `oag-server lobby` on localhost, two
windowed `oag-game` clients each in its own Xvfb on lavapipe, driven by xdotool, mouse for the
first screens and held keys after): connect (including the refused build, with both builds named),
the hub, CREATE (RACE page, Track Select, Ship Select), JOIN (Ship Select, the list with the room
and its circuit line, the room), the host's START, the hand-off, the race to `--max-ticks 300`,
and both clients back in the room with "--max-ticks reached". Stills of the room and the list on
Pulse and HD (`--menu-page online-room:3`, `online-list:3:0`): HD draws its own strip look.

## 9. Omega's room settings, on every title (2026-10-19)

Omega's `MP Select` (the create page, and `Edit Settings` in the lobby) is HD's, carried forward
(part 1, "Checked against HD"). Each row below was read from `data09.psarc`'s
`network_definition.xml` and `languages/english/entries.xml`; HD's rows from `DATA00`, `DATA05`
and `DATA06`. **What is carried is Omega's (the rows, ranges and labels); that the settings behave
as the engine implements them online is ours, chosen, not measured**: the original has no network
race to measure.

| Omega row | Values and default | Verdict |
| --- | --- | --- |
| Laps | 3 to 20 (HD default 3) | **Carried** (part 7); a room may ask for 1 (chosen) and `0` is the mode's own |
| Barrel rolls (`MAN_2_BR`, "BARREL ROLL") | On, Off; no default authored | **Carried** as `RoomSettings.no_barrel_rolls`; a new room has rolls on |
| Pilot assist allowed (`FE_PILOT_ASSIST_ALLOWED`) | On, Off; no default authored | **Carried** as `RoomSettings.pilot_assist`; a new room allows it (chosen: the list names On first) |
| Zone target (`FE_ZONE_TARGET`, `NumberOfNitroLevels`) | 15 to 55 step 5, default 30 | **Carried** as `RoomSettings.zone_target`, Zone rooms only |
| Eliminations (`IG_HUD_TARGET`, `NumberOfEliminations`) | 50 to 600 step 25, default 200 | **Not carried, checked, differs**: the unit is not a kill count (Pulse's kill target is 5 to 10, `Options::eliminator_kill_target`); an Eliminator room uses the mode's own target |
| Weapons (`RB_WEAP`) | On/Off, default On | **Carried** as `RoomSettings.weapons` (2026-10-19, ADR-0071); an Eliminator room is always on and shows no row. Label read from the title's own `RB_WEAP` where its disc has it (Omega, HD), else `OAG_ONLINE_WEAPONS` |
| Weapon hints, Pure racing | On/Off | Not rows |
| Availability (PUBLIC, INVITE ONLY) | 0, 1 | Not built (no invite-only, item 6) |
| Respawns | `aList`, disabled in the XML | Not a setting in Omega either |
| Max players | `aList`, disabled, 8 | Ours is the room's `max_players` (part 8) |

**How each reaches the race.** `RoomSettings` (the owner's) grew `no_barrel_rolls`, `pilot_assist`
and `zone_target`; `RaceParams` (what both ends load) grew `no_barrel_rolls`, `assists` (one Pilot
Assist level per seat, an index into `oag_physics::pilot_assist::Level::ALL`) and `zone_target`.
Wire v1 was edited in place. The lobby validates (`Rooms::validate`: a zone target only in Zone,
raised or lowered into 15 to 55) and builds the params at launch (`launch_for`). Both ends then
call `oag_net::race::prepare`, the one function that already forced Pilot Assist off, which now:

- sets each seat's level from `assists` (`RaceSim::set_pilot_assist_seat`; the level is per slot in the
  sim now, `Sim::pilot_assist_levels`), which is the lobby's gate: with the room's assist off,
  every entry is `0`;
- turns barrel rolls off for every craft, the AI included (`RaceSim::set_barrel_rolls`, read as
  `Environment::barrel_rolls` in `oag_physics::forces`; the gesture is never read, so no roll arms
  and no shield is spent);
- gives a Zone race its ending (`RaceSim::set_zone_target`, `RaceState::zone_target`: the race ends
  on the step that reaches the target zone; a zone is 600 ticks). **Chosen, not measured**:
  Omega's slider is the only zone target a race ever has, and the original's own Zone ends only on
  destruction. `RaceState::zone_target` is hashed only while set, so every race without one keeps
  its committed hash and two ends that disagree about it differ at tick 0.

A person's own level (`controls.pilot_assist` in their settings, or `--online-assist`) travels in
their `Ready` (a pick is the ready, so it is sent with the pick and again after every edit).
`oag-server` (`--no-barrel-rolls`, `--zone-target`, `--assists off,normal`) and `oag-game`
(`--online-no-barrel-rolls`, `--online-no-pilot-assist`, `--online-zone-target`, `--online-assist`)
set them from the command line.

**The room screen.** The owner's LAPS row became a generic `Setting` row
(`oag_ui_screens::join::Setting`: Laps, BarrelRolls, PilotAssist, ZoneTarget), so one mechanism
holds all four: a click or the pad's left and right steps it, it is a pointer row like MAX PLAYERS,
and the rows that do not apply (laps outside a lap mode, the zone target outside Zone) are not
there. An edit clears every ready as before. Labels read the title's own string first
(`MAN_2_BR` on HD, 2048 and Omega; `FE_PILOT_ASSIST_ALLOWED` and `FE_ZONE_TARGET` on 2048 and Omega;
`FE_ON`, `FE_OFF`), else this project's own (`OAG_ONLINE_BARREL_ROLLS`, `OAG_ONLINE_PILOT_ASSIST`,
`OAG_ONLINE_ZONE_TARGET`, `OAG_ONLINE_ON`, `OAG_ONLINE_OFF`, `OAG_ONLINE_ZONE_NONE`, in all four
language files, machine translations `human = false`), which is what Pulse and Pure show. The zone
row says NONE for `0`, the original's Zone, which is a choice Omega's slider does not offer. The
mirror rule needed nothing of its own: the screen is shared, Pure shows Pulse's strings and 2048
shows HD's via Omega (`MAN_2_BR`) or its own.

**Walked, 2026-10-19.** Stills (`--menu-page online-room:8`) of the fullest room (eight seats, the
five owner rows, START, BACK: fifteen rows, all on the picture) on Pulse, Pure, HD, 2048 and Omega
(each draws its own face; HD, 2048 and Omega say their own "BARREL ROLL", Pulse and Pure this
project's "BARREL ROLLS"; Pure's START RACE and BACK sit tight against each other at fifteen rows
but do not overlap). Localhost two-client races, the real `oag-server lobby` forking the race
child, `--net-bot` clients, debug builds. Every client log was searched whole for `bad [1-9]`:
none, in any run below. What each run **exercised**:

| Run | Setting reached the race | Exercised |
| --- | --- | --- |
| Pulse PSP EU, Zone, target 15, 12000-tick cap | zone target | both clients and the server ended the race on tick 9271 (15 zones of 600 ticks plus the countdown), `every person has finished`, 0 bad hashes |
| Pulse PSP EU, single race, 900 ticks, rolls off, assists `[extreme, normal]` | rolls, assists | the lobby's `RaceParams` carried both; Pulse's `Laws` may hold none for a level (not checked), so this run shows agreement, not the assist acting |
| HD (PS3 EU) and 2048 (Vita folder), single race, 900 ticks, same settings | rolls, assists | the same, on the titles that author the laws; 0 bad hashes |
| Pure PSP EU (600 ticks), HD, 2048, Omega (PS4 folder) (400 ticks), Zone, rolls off, assists, zone target 20 | carried (the server's `RaceParams` line) | **the target was never reached** (20 zones is 12000 ticks), and `pilot_assist_input` turns the assist off in Zone, so these four prove the fields travel and the ends agree, nothing more |

The disc-backed `room_settings_ground_truth` test is what proves each reaches the race: it loads Pulse
with the settings, reads each seat's level, the roll switch and the target back off the `Race`, and
runs a one-zone Zone race to its end next to one with no target. `ship_dynamics`'s
`a_room_with_barrel_rolls_off_never_arms_one` proves the roll gate itself (a craft that arms and
pays with rolls on, and neither with them off).

**Who wins a zone-target race.** No craft has a standing finish in Zone (`Standing::update` ends on laps
only), so the board ranks by `oag_race::places`'s racers rule: distance covered at the end, ties by
slot. Every person reaches the target zone on the same tick; the first place is whoever got
furthest round, not the owner.

**Replays and ghosts.** An online race records neither: a ghost is a one-person Time Trial
(`race_stage/ghost.rs`), and no replay recording was found on the online path (searched, not
proven absent). A replay of a networked race would need the room's settings in its header.

**Open.** The room screen's new rows are walked as stills, not clicked live on every title (the
pointer is covered by the row tests); Omega's remaining sub-pages (kick, friends, invite-only) and
the Eliminations target is not carried; LAN keeps the originals' race (rolls on, no assist,
Zone's own ending); a Zone room cannot be given a target above 55 or between the fives (the slider's
own steps).

## Sources

- Omega `data09.psarc` front-end XML and `english/entries.xml`, via `scripts/psarc.py`;
  HD's same files from `hdfury-ps3-eu-dec.iso` (`DATA00`/`DATA02`).
- shadPS4 0.20.0 (`/usr/bin/shadps4`), its [quickstart](https://shadps4.net/quickstart/),
  `src/core/libraries/np/np_matching2/np_matching2.cpp` in the repository, and
  compatibility issue 487 (linked above).
- Omega `eboot.bin` v1.00 in Ghidra (`/omega/eboot-ps4-omega-eu.bin`), string searches only:
  no function was read, no name was recovered, `names.tsv` is unchanged.
## 10. Weapons (2026-10-19)

Weapons are predicted simulation, not server events: the client already simulates the whole
`RaceSim`, a fire is a button edge in the wire's input, and the one thing missing was a correction
trigger for discrete state. Decision, tables and the 2 and 4 client numbers are in
[ADR-0071](../architecture/adr/0071-weapons-are-predicted-simulation-with-a-discrete-state-correction-trigger.md);
the code is `oag_net::weapon_sync`, the test `crates/net/tests/weapons_ground_truth.rs`.

- The room's `weapons` byte reaches `RaceParams::weapons` at launch (`launch_for`); Eliminator forces
  it on (`Rooms::validate`). `oag-server` takes `--no-weapons`; `oag-game --online-create` takes
  `--online-no-weapons`. A bot (`--net-bot`) fires what it holds.
- The room screen has a `Setting::Weapons` row (not in an Eliminator room). Eliminator is no longer
  dimmed on the ONLINE RACE page; Head to Head still is (`OAG_ONLINE_MODE_TWO_ONLY`).
- How late another person's shot first shows (at most 15 ticks at 5 percent loss and 30 ms
  jitter, capped at 33) and what a fire seen late looks like are in
  [the networking page](README.md#a-shots-first-appearance-2026-11-11); the two-person
  weapons race there also ran against a remote host over IPv4 alone, which the runs above
  (IPv6 only) had not covered.
- **Checked against the other titles' rows:** the label id `RB_WEAP` is Omega's and HD's `MP Select`
  (read from `network_definition.xml`); 2048 and Pure/Pulse show this project's own words. The
  weapon rules themselves are each title's (Pulse's law where unmeasured, per the project's rule).

## 11. The end screen, a rematch and kick (2026-10-19)

**Ours, chosen, not measured**, except where Omega's own strings are named. Code: `crates/net/src/wire/board.rs`,
`online/rooms.rs` (`rematch`, `kick`, `race_over`), `crates/game/src/net_play.rs`,
`crates/game/src/app/session/online.rs`, `crates/game/src/online_screen.rs`; tests in `rooms/tests/after_race.rs`,
`tests/online.rs` and `tests/net_ground_truth.rs`.

### The board

The server's own final board (`wire::board::Board`: per craft the place, slot, the person's name or empty for an AI
craft, laps, finish ticks, best lap and the Eliminator's eliminations) travels three ways:

1. **Race child to lobby.** `oag-server race` prints `race-board <hex>` (the same hex-line shape as the job) before
   `race-over`. `ChildHandle` reads it from the tail of the child's output with the reason; `Status::Over` carries an
   `Ended { reason, board }`. A child that crashed, timed out or never came up has no board line and the board is empty.
2. **Race child to every client.** `Control::End { reason, board }`. The board is `Host::board()`, built by
   `oag_net::race::final_board` from `race.results()`. It is empty when the race ended with nobody placed (everyone left,
   a limit before a finish, a dropped seat's `End`).
3. **Lobby to every member, through the room.** `Rooms::race_over(room, reason, board)` stores it on the room and
   `Snapshot.board` carries it on every later push, so a member who joins after the race also sees it.

The clients still derive their own board from their confirmed state (the tick hash proves it equal), and **both agree**:
`NetPlay::check_server_board` compares the two (`Board::same_race_as`, ignoring names, which only the server knows)
once the race is over and logs `net: the server's board matches this client's`, or an error naming both.
`net_ground_truth::the_lobby_forks_a_real_race_...` asserts on a real Pulse race that each client's `End` board is the
client's own board, names both people (slot 0 Ada, slot 1 Bob), is the same on both, and is what the room holds
afterwards.

**What a client's end screen shows.** The title's own EndRace screens (`session/endrace.rs`): on Pulse and Pure the
multiplayer table (`PRO_POS`, `PRO_NAME`, `PRO_TOT_TIME`, the same `PeopleResults` a local split screen on a PSP pressing
gets), on HD and Omega the field grid, in Eliminator and Zone the mode's own tables. **A network race is not a split screen
for anything else**: the loyalty, the medal and the menu rows are what they were before, and the PS2's two stacked
`EndRace Results Split` tables stay a local split screen's (they say only a place, not who). **The name column is a
person's name** where the race knows one (`SimRace::set_person_names`, set from the room's members in seat order at the hand-off;
`scoreboard::Row.name`), else `PLAYER n`. That the original's multiplayer table prints the person's name in `PRO_NAME` is
what `endrace/people.rs` already records for Pulse's network-play branch; HD's `PI001` fills its name cell the same way, so the
name is the authored behaviour, the seat fallback is ours. Opponents are not named by the HD grid (its authored column).
The built-in results table (no `EndRace_Definition.xml`) prints the name upper-cased in the craft column. **Which route each title takes** (read from the code, `oag_game::endrace::dialect` and `endrace_entry`): Pulse (its own
`EndRace_Definition.xml`, the `PeopleResults` table) and HD (the field grid) were walked; **Pure and Omega name no
`EndRace_Definition.xml` (`endrace_entry: None`)**, so a network race there draws the built-in results table, which now
names each person (`scoreboard::craft_label`, tested); **2048 is the touch dialect**, whose pages summarise one racer, so a
network race of several people keeps the built-in table over them, as a split screen already did (`build_endrace_touch`,
**checked, differs**: 2048 and Omega do not use HD's grid). None of Pure, Omega or 2048 was walked live. The mode/team fix
below is in `build_endrace` only: the touch route reads neither. Walked live on Pulse
PSP: the first two screens a person sees after the line were the two stacked split tables before this choice (4TH PLACE and
8TH PLACE, nobody named), which is why the table is the named one.

**The room screen after a race** says where each person came in, on their seat: `ADA (HOST)  READY  #2 3.03.00` (the place,
and the time for a craft that finished), `online_screen::seat_line`. Stills: `--menu-page online-room-raced:4:2`.

### Rematch

`Message::Rematch` (client to lobby, `LobbyClient::rematch`). RACE AGAIN on an online end screen sends it and then leaves
the finished race for the room, which is where the menus always come back to (`MenuOption::RaceAgain` on a network race).
Server rule (`Rooms::rematch`, `try_rematch`):

- The sender is ready with the team they last picked, **now** if the room is open, or **when the race ends** if it is
  still running (`Occupant.again`; a person who finishes first presses RACE AGAIN while the others are still racing).
  Picks are otherwise cleared at the end of a race as before.
- From the **owner** it also arms the room's start (`Room.armed`): the start fires by itself, as the owner's `Start`, the
  moment the room is open, has enough people and everybody is ready. Until then the room's note keeps the last race's
  reason and appends the wait (`every person has finished - rematch, waiting for: bob`), and `Snapshot.rematch` is set. A guest's
  ask puts "NAME wants to race again" in the note.
- **The same settings**: a rematch never touches `RoomSettings`; the launch is the room's own (a new seed, `launches` is
  mixed in on purpose). Changing the track is the owner's ROOM SETTINGS row, which the room already has and which clears
  every ready and disarms the rematch; there is no second change-track path.
- **Someone leaves meanwhile**: through `leave()`. If the owner leaves, the request dies with them (the new owner presses
  RACE AGAIN themselves); if the room drops below `MIN_PLAYERS` the request is disarmed with the room's note saying who
  left; a leaver who was ready simply is not seated. A person who is back in the room is ready again by the existing rule
  (the pick is the ready, sent by itself once the room is open), so **returning to the room by any route means ready**; BACK
  leaves it.
- A pick sent while the room is still racing was refused and never resent; the client now sends the pick only when the
  room is open.

**Omega's own words** (`endrace_definition.xml` in `data09.psarc`, and HD's `DATA02.PSARC` copy): the multiplayer end menu
is two blocks, `return_to_lobby` (label `IG_PAUSE_QUIT`, goes to `Kill Game Multiplayer`) and `view_MP_again`
(`ER_VIEW_AGAIN`); the single-player block `race_again` is not in it. The string tables of HD and Omega also hold
`ER_HOST_RACE_AGAIN` ("THE HOST HAS DECIDED TO RACE AGAIN."), `ER_GOTO_NEW_LOBBY` ("GO TO NEW LOBBY"),
`ER_RETURN_L` ("RETURN TO LOBBY") and `FE_RETURNING_TO_LOBBY`, `ONL_IGM_RTL_COUNTDOWN` ("You will be returned to the lobby in
%d second(s)"). So **Omega's rematch is the host's decision**, announced to the others; ours keeps the owner as the one who
starts it and a guest asking is a ready-up, and the room tells a guest the owner has asked (`Snapshot.rematch`) with
`ER_HOST_RACE_AGAIN` (the disc's own text where its table has it, which HD's and Omega's do; ours, `OAG_ONLINE_HOST_RACE_AGAIN`,
elsewhere). **Not wired from the disc:** the menu rows (our RACE AGAIN
row is the title's own `ER_RACE_AGAIN`, which the MP block does not author), the 'GO TO NEW LOBBY' flow (a host who
changes the settings) and the countdown to the return.

### Kick

`Message::Kick { name }`, owner only (`Rooms::kick` goes through `owned()`), in an open or queued room (not once the
countdown or the race is running: refused `Started`). The kicked person is sent `Left` with the reason ("the host removed
you from this game", shown as a screen of its own with BACK, the existing `Left` handling), the others see the room without
them and the note "NAME was removed by the host".

**Rejoin rule (chosen):** the room remembers the name (case-blind) for as long as the room exists. A join into that room is
refused `Refusal::Kicked` ("the host removed you from this game"), the room list shows the room as not joinable to that
person, and a new session under the same name is refused the same way (the name is the identity here: names are unique per
session, ADR trust model). Any other room is open to them. When the room closes the ban goes with it; the owner leaving
does not lift it. Not a ban of the person's address (online is server-authoritative with a trust model that has no client
anti-cheat; a determined person rejoins under another name).

On the screen the owner's cursor on another person's seat says `CONFIRM TO REMOVE THIS PERSON FROM THE GAME`
(`OAG_ONLINE_KICK_HINT`, with its German, French and Brazilian Portuguese text) and a confirm or a click on that seat
(`Event::Leave(seat)` in the room form, the same event the room list reads as "chosen") sends the kick; the owner's own seat
does nothing. **Pointer: the click is the confirm, the same hit-test as every row.** Omega authors no host kick: its
`KickFromGame` screen is reached from code (`LobbyCommand mode="kick"`, ranked game end and server errors, one dialog with
`TRC_ONL_SERVER`) and the `ONL_KICK_*` strings are the compatibility kicks (content, started, blocked list, no ship picked in
time). **Not in Omega, so ours, chosen.**

### Tests

- Disc-free: `wire` round trips (the board in `End` and the room snapshot, `Rematch`, `Kick`, `Refusal::Kicked`);
  `rooms/tests/after_race.rs` (the board reaches every member; a rematch from the owner fires once everyone is ready under
  the same settings, taking its picks from the last race; a member who asks while racing is ready at the end; a rematch dies
  with its owner or below the minimum; only the owner kicks, by case-blind name, never themselves, never during a start,
  and the kicked person is refused back by the join and in the list, also under a new session); `tests/online.rs` over real
  sockets with a fake race child that prints `race-board` (the board arrives at both members, a second hand-off arrives
  after both rematch and the settings are equal, a guest cannot kick, the owner can, the kicked person is told and refused
  on rejoin). The fake child waits at a gate file (the `online-crash-flake` pattern), with a marker file so a second run waits
  at its own gate.
- Disc-backed: `net_ground_truth::the_lobby_forks_a_real_race_...` (Pulse PSP, a real child, one lap): both clients'
  `End` board equals their own, names both people and equals the room's, then both ask for a rematch and a second hand-off
  arrives under the same settings.
- `online_screen/tests.rs`: the seat line, the kick hint for the owner only. `results_tables` and the HD draw list: the
  name in place of `PLAYER n`.

### Walked live (2026-10-19)

Two windowed `oag-game --net-bot` clients (Ada owns the room, Bob joins) on Xvfb `:97` with software Vulkan, a real
`oag-server lobby` on loopback (lobby 60510, race 60511), one lap each, over the menus as a player sees them: connect (the
remembered address, certificate and name), CREATE GAME, Track Select, Ship Select, the room (LAPS stepped to 1 with a click),
JOIN GAME and the room list, START RACE, the race, the end screens, the room again. **Pulse PSP EU** and **HD (PS3 EU)**.

- **The board.** Both clients logged `net: the server's board matches this client's (8 rows)` after every race (Pulse 3 races,
  HD 3 races), 0 bad hashes. Pulse's end screen after the line is the named table (`RESULTS`, `POS`, `NAME`, `TOTAL RACE TIME`,
  ADA and BOB on the two highlighted rows, the AI craft by team); HD's is the field grid with `Ada` and `Bob` on the two
  highlighted rows and the headline `7TH PLACE`. The first Pulse walk showed the PS2's stacked split tables (`4TH PLACE`,
  `8TH PLACE`, nobody named), which is why a network race no longer takes that route.
- **A bug the walk found, fixed:** a person who **joined** a room read the end screen's mode (and the team their loyalty is
  banked for) from their own RACE page instead of the server's race, so on HD the joiner's end screen was
  `TIME TRIAL COMPLETE!` over a broken table while the owner's was `7TH PLACE`; the room's owner never saw it because their page
  is the room's. `build_endrace` now reads a network race's mode from the world and its team from the race's roster. The
  same joiner on the rebuilt binary: `7TH PLACE`, `Ada`, `Bob`.
- **The room afterwards**: the seats say `#3`, `#6 0.42.73`; the owner's
  note is `EVERY PERSON HAS FINISHED`, then `... - REMATCH, WAITING FOR: BOB` once the owner asked. The first Pulse still put
  the time into the `PLAYER n` label on a guest's room screen (the widest face); the line now drops the time first
  (`SEAT_GLYPHS`).
- **Rematch**: Ada's RACE AGAIN took her to the room with the rematch armed; Bob's return to the room (RETURN TO MENUS, which
  is a ready by the rule above) started the second race by itself under the same settings, on Pulse and on HD (a third race
  after a second joiner). Both clients matched the server's board each time.
- **Kick**: the cursor on Bob's seat read `CONFIRM TO REMOVE THIS PERSON FROM THE GAME`, a click on it sent the kick, Bob's
  client showed `THE HOST REMOVED YOU FROM THIS GAME` with BACK, and in JOIN GAME the room was listed dimmed (not joinable)
  and could not be chosen.
- **Not walked:** a pad-only kick (the confirm path is the same `Event::Leave` as the click and is read by the pad's
  `confirm()`, covered by the room form's own tests), and Pure, Omega, 2048 and the PS2 pressing: their routes are above
  (**checked, differs** from HD's grid: Pure and Omega the built-in table, 2048 the built-in table over its touch pages);
  the board, the names and all the lobby code are title-blind.

## 12. Player-hosted races (2026-11-17)

Design and reasons: [ADR-0092](../architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md). This part is what is built and how to
run it. **Chosen, not measured**, like the rest of the lobby.

### What a player does

HOST GAME on the ONLINE page (or `--online-create --online-host`) walks CREATE GAME's pages and
then offers this machine as the room's race host. The game opens its hosting port (UDP 4435,
`[online] host_port` in `settings.toml` or `--online-host-port`), the lobby dials that port from
its side, and only then lists the room, marked **PLAYER-HOSTED** beside the host's name. The
port has to be reachable from the lobby: open, or forwarded to this machine by the router. A
refusal names the port. Everybody else joins the room as they join any other; at START the race
runs on the host's machine and the lobby hands each member the host's address.

The mark's one line is "PLAYER-HOSTED: RUN BY A PLAYER, NOT THE SERVER - UNOFFICIAL" (the list row
under the cursor, and a guest's room). Not on the line, and true: latency favours the host, no
record is kept, and the host and its guests see each other's addresses.

### The messages

- Client to lobby: `HostRoom { settings, offer }` with `offer = { port, cert_hash, key,
  signature }`; `HostUp { room }` once the race is loaded and again once a second while it runs;
  `HostOver { room, reason, board }`.
- Lobby to the host: `Launch { room, job }`, the `RaceJob` a race child reads from its standard
  input.
- `RoomInfo` and `Snapshot` carry `player_hosted`.

The signature is Ed25519 (`ring`) over `"OAG-HOST-1" | lobby certificate hash | session token |
port | cert_hash` (`HostOffer::statement`). The key pair is made on first use and kept in
`identity.toml` in the configuration directory (`oag_game::identity`, mode 0600).

### Where it lives

| Piece | Where |
| --- | --- |
| The key pair and the offer | `oag_net::online::hostkey`, `oag_game::identity` |
| The lobby's side: signature, bans, limits, probe, launch, hand-off | `crates/net/src/online/server/hosting.rs` |
| The ban file | `oag_net::online::moderation` |
| A hosted room's rules (no queue, the owner's edition, closes with its owner) | `oag_net::online::rooms` |
| The host thread on the player's machine | `oag_net::online::player_host` |
| The game's side, menus and `--online` | `oag_game::online::Hosting`, `session::online` |

### Limits on what a host controls

| Limit | Value |
| --- | --- |
| Player-hosted rooms in all, probes included | 32 (`MAX_HOSTED_ROOMS`) |
| Player-hosted rooms per address | 2 (`MAX_HOSTED_PER_ADDRESS`) |
| Probes under way | 4 (`MAX_PROBES`), each one WebTransport handshake of at most 5 s |
| Hosting requests per connection | one every 5 s (`HOST_ASK_INTERVAL`), one probe at a time |
| The address probed and handed out | the one the request came from, never one it names |
| A host that is up and silent | 5 s (`QUIET_TIMEOUT`), then its room is closed |
| A host that does not load the race | 120 s |
| A hosted race the lobby hears no end of | 35 minutes |

### Moderation

`oag-server lobby --ban-file PATH` (`oag_net::online::moderation`): one entry a line, `#` for a
comment, every date a UTC stamp as the log writes one (`2026-11-18T12:34:56.000Z`):

| Line | Meaning | Written by |
| --- | --- | --- |
| `key <64 hex> <since> [reason]` | the key may not host, permanently | the operator |
| `key <64 hex> <since> until <stamp> [reason]` | the key may not host until then | the lobby, or the operator |
| `addr <ip> <since> [until <stamp>] [reason]` | the same for an address | the operator |
| `strike <host key> <when> race <16 hex> by <reporter key> [reason]` | one case the lobby proved; `race` is the first 8 bytes of the race's params hash | the lobby |

A proven case (anti-cheat below) appends a `strike` line and bans nobody. When a key's strikes
reach `STRIKES_TO_BAN` (2), counted within `STRIKE_WINDOW` (7 days), after its last timed ban,
on no race a ban was already made on (another guest's late report on that race is recorded but
counts for nothing), and only over distinct races each reported by a different player (a maximum matching, so the
order they came in does not matter; reporters with no key count as one), the lobby appends a
timed `key` line. Its n-th timed ban lasts `FIRST_BAN` (1 h) doubled n - 1 times, at most
`LONGEST_BAN` (30 days): 1 h, 2 h, 4 h ... The lines are never removed by the lobby, so the
count survives a restart and an expired ban still makes the next one longer. A refused host is
told the reason and the time left (`you may not host on this lobby: caught cheating in 2 races,
... (59 min left)`). Constants and the reasoning behind each: the module's doc comments. Address
bans are unchanged apart from taking the same optional `until`.

A line that does not parse (no date, a bad key, `until` with no stamp after it, a strike missing
`race` or `by`) is reported by line number in the log and skipped, never a ban. To lift a ban,
delete its `key` line; to forgive a strike, delete its `strike` line. The file is read again
within 2 s of a change; a listed room whose host is now banned is closed with the reason. Every registration, refusal, launch and end is a
`host-log:` line in the lobby's log with the first 16 digits of the key, the address, the name,
the room and its settings.

### Anti-cheat (built 2026-11-18)

ADR-0092 decision 7, items 1 to 4. A player host is the authority, so the lobby, which issued
the race, is the referee. Every value here is **chosen, not measured**; the costs are measured in
[the server roadmap](server-roadmap.md#anti-cheat-what-it-costs).

1. **The race the lobby issued.** The lobby draws a hosted race's seed at random. The hand-off
   carries `params`, SHA-256 of the job's `RaceParams` wire bytes (`oag_net::fair::params_hash`),
   and `host_key`, the host's public key (all zero for a server race). A client whose host
   welcomes it to another race (another seed, circuit or setting) refuses it at the welcome:
   `the host offered a race the lobby did not issue (circuit ..., seed ...): refused`.
2. **Signed checkpoints.** Host and client fold every confirmed record (state hash, away mask,
   inputs) into a running SHA-256 (`fair::Chain`, seeded with the params hash and the tick-0
   hash). Once a second, and at the end, the host sends `Control::Checkpoint { ticks, chain,
   signature }`, signed with its player key over `"OAG-CHECK-1" | params | ticks | chain`. A
   client checks the signature and its own chain; a checkpoint signed over records other than
   the ones it was sent is logged, as evidence only.
3. **Fraud proof.** A client keeps every confirmed record of a player-hosted race. Once a
   verified checkpoint covers the first tick whose state hash it could not reproduce, it sends
   the lobby `Report { room, part, parts, bytes }`: the records from tick 0 to the checkpoint and
   the checkpoint (`fair::Proof`), in 128 KiB parts, one a lobby poll so the link's 1 MiB send
   queue never fills. The lobby checks the length against the ticks the race can have run before
   it reads a record, checks the signature and folds the chain again, then re-simulates the
   records from the job on a thread of its own (`fair::judge`, the client's confirmed path:
   away mask, held-stream edges, one simulation tick, the hash). A hash the host's own inputs do
   not give is proven: the host's key gets a dated `strike` line in the moderation file (above;
   two from distinct races and reporters ban it), its room is closed with `the lobby closed this
   game: its host was caught cheating (...)`, and a `host-log:` line names the reporter, the key,
   the address and the strike count (`gets a strike (1 of 2 counted)`, plus `, banned for 1 h`
   when it bans). A start state the lobby cannot
   reproduce (content it lacks, mods) or a re-simulation out of time is "not checkable", kept as
   evidence, never a ban. A report that reproduces is dismissed.

   **The build guard.** A desync report on a host whose build (the `Edition::build` of its
   hello, `crate::BUILD_VERSION`) differs from the lobby's own is not checkable, decided before
   any re-simulation is spent: `the host runs another build (<host> vs <lobby>)`, a `host-log:`
   line, evidence only, no strike. The lobby's build is its editions' (`Rooms::own_build`); the
   host's is recorded when the race launches (`RaceCase::host_build`). No second identity: it is
   the string `Rooms::connect` already compares. That hello check refuses another build of a
   title and pressing the lobby races, so the guard decides only what it lets through: a host of
   a pressing the lobby does not race (another platform of one title) whose build differs. A
   same-build host is judged as before. A forged-input challenge (item 4) is not guarded: a
   missing signed datagram is proof on its own, needs no re-simulation, and a build difference
   cannot explain it, so it still strikes. Tests: `crates/net/tests/anticheat.rs`
   (`a_host_of_another_build_than_the_lobby_is_not_judged_by_a_re_simulation_and_not_struck`,
   `a_host_of_another_build_that_hides_a_signed_input_is_still_struck`; the same-build strike is
   `a_host_that_alters_its_simulation_is_proven_...`).
4. **Signed inputs.** A hosted race's job names each seat's player key (from the lobby hello's
   `key`). Each client signs every input datagram (`Datagram::Inputs` with a signature, tag
   `0x12`, over `"OAG-INPUT-1" | params | seat | first | inputs`); the host drops one not signed
   by the seat's key, and keeps the signed datagram behind each input it applied for two
   minutes (`fair::Ledger`). A client signs whenever it has a key, server races included (the
   server ignores it), so a game that came back to a player-hosted race from its record, which
   keeps no host key, is still heard; it signs for the race its welcome names. From the first tick it sent, a client that sees an input confirmed for its
   seat that is neither what it sent nor a repeat of the tick before (a host repeats a seat's
   last input when none arrived) reports it once a checkpoint covers it. The lobby checks it is
   not a repeat in the signed records, and asks the host `ShowInput { room, seat, tick }`; the
   host answers `InputShown { room, seat, tick, datagram }`. A datagram signed by the seat's key
   with that input at that tick dismisses the report; none, a wrong one, or no answer in 10 s is
   proven, with the same strike. What would convict an honest host is ruled out: a claim on a tick
   older than half the ledger, counted from when the host said its race was up
   (`fair::too_old_to_challenge`), is not checkable, and so is a host that left the lobby before
   the answer was due (an honest `--online` host exits after its race).

Bounds on what a reporter controls: one report a seat a race (the host's own seat reports
nothing), two assembling and four waiting at once, at most 64 MiB of proofs held in all
(`MAX_HELD_BYTES`, a decoded one counted at twice its bytes), one re-simulation at a time with
60 s to finish, a report taken up to 60 s after its race ended. A client sends a report's next
part only once its link has written all but one part of the last (`Link::unsent`). A
re-simulation over an extraction that missed a read the race made is not checkable, never
proven (`oag_assets::record::take_misses`, as the race child warns of). Item 5 (withheld inputs) is not built:
the client already measures how long its inputs take to come back confirmed
(`Stats::confirm_sum_ms`), but nothing reports it.

The test cheats are `oag_net::host::Cheat`, compiled only with `oag-net`'s `test-cheats`
feature, which only dev-dependencies turn on (`cargo tree -p oag-game -e normal,features -i
oag-net` shows `client` alone), and armed by `OAG_TEST_CHEAT=speed|forge|seed`. The same
feature carries one staged scenario that is not a cheat: `OAG_TEST_WRECK=<slot>` wrecks that
seat's craft at the same tick on every peer (`oag_net::race::presence`).

### Run it locally

```sh
target/debug/oag-server lobby --image data/images/pulse-psp-eu.chd \
  --listen 127.0.0.1:60020 --race-listen 127.0.0.1:60021 --hash-out /tmp/lobby.hash \
  --ban-file /tmp/bans.txt
# Ada hosts; port 0 lets the system pick one, which is only any use on one machine.
target/debug/oag-game data/images/pulse-psp-eu.chd --no-audio --net-bot \
  --online 127.0.0.1:60020 --lobby-hash @/tmp/lobby.hash --online-name ada --online-team Feisar \
  --online-create --online-host --online-host-port 0 --online-laps 1 \
  --online-track 'Data\Environments\01_Track\track.vex' --screenshot /tmp/a.png --ticks 20000
# Bob joins the first joinable room and is handed Ada's address.
target/debug/oag-game data/images/pulse-psp-eu.chd --no-audio --net-bot \
  --online 127.0.0.1:60020 --lobby-hash @/tmp/lobby.hash --online-name bob --online-team Feisar \
  --screenshot /tmp/b.png --ticks 20000
```

Measured on 2026-11-17 by `net_hosted_ground_truth` (debug builds, localhost, Pulse EU circuit
01, one lap, two bots): the lobby logged the registration and sent the launch to the host; the
guest printed `online: room 1 (player-hosted): 2 in it` and `the race is on 127.0.0.1:<the
host's port>`; both finished with 0 bad hashes over more than 1,500 checked ticks and a board
equal to the host's; the lobby logged `room 1's hosted race is over: every person has finished`
and ran no race child. A host killed mid-race: the lobby logged `host went quiet` and the guest
printed `the lobby closed the room: the host left: a player-hosted race ends with its host` and
exited. A host whose profile key was in the ban file exited with `the lobby refused: Banned: you
may not host on this lobby: altered a race`.

**A `--screenshot` race ends on the server's `End`, never on this client's own predicted
finish** (2026-11-26). With seat 1 wrecked, seat 0's prediction finishes the race at its own
line with nothing left to run on; the capture loop used to stop stepping the client there, so
the `End` and its board never arrived and the finisher printed no board line. This was the
likely cause of the occasional `board is the host's` failure in the gate: forcing the wreck
reproduces that message every time, and CPU load alone never did in 27 runs. The two failing
gate runs kept no game logs, so that it was a wreck in them is inferred (65). The windowed
game steps its client every frame and was not affected.

### Tests

- `oag-net` `player_host::` (no disc, real sockets): the signed offer whose port answers is
  listed with the mark and the host's name; a guest races the host to the end through the
  hand-off with no bad hash; the host leaving the race ends it for the guest with the reason; a
  host that vanishes, and one that goes quiet, close the room with the reason; a wrong signature,
  a second request too soon and a closed port are refused by name; an address hosts only its
  cap; a banned key is refused and a ban added later delists the room.
- `oag-net` `hostkey`, `moderation` (every line form and each malformed one refused by name; a
  timed ban's time left and its end; strikes counted over distinct races and reporters in any
  order; one reporter or one race is one strike's worth; the window; bans of 1 h, 2 h, 4 h across
  restarts with injected time, the 30-day cap, a ban using up its strikes), `oag-log` `stamp`
  (parse round trip), the wire round trip; `oag-game` `identity` (made on first
  use, the same key after, owner-only, a broken file left alone) and `online_screen` (the mark
  and its line).
- `oag-game` `net_hosted_ground_truth` (`#[ignore]`d, real processes): the three runs above,
  and a host whose game cheats (`OAG_TEST_CHEAT=speed`) reported by the guest, proven by the
  lobby's re-simulation ("after tick 300 the host claimed state ..."), its key given a dated
  `strike` line and no `key` line (one case bans nobody), and the guest told why, in about 16 s;
  and a race whose guest is wrecked (`OAG_TEST_WRECK=1`), where both games still print a board
  equal to the host's, in about 48 s.
- `oag-net` `anticheat::` (no disc, real sockets): an altered simulation proven and struck, the
  host free to host again at once; a forged input the host cannot show signed, proven without a
  re-simulation; two proven cases in two lobby runs over one file (a restart between), reported
  by two players, ban the host with the reason and `59 min left`, the reporter not banned; the
  same two by one reporter are one strike's worth; a host racing
  another seed refused at the welcome; an honest race with no report, every checkpoint verified
  and every input signed; a malformed and an oversized report refused without judging, the race
  going on; a lying guest that claims a forgery of an input it sent is dismissed when the host
  shows the signed datagram, banning nobody. `fair::tests`: the chain, the proof codec and its
  refusals, the judge (honest, altered, another start, out of time), the forgery rule and its
  age limit, the client's watch, a rejoined game's signatures.

### Open

- HOST GAME from the menus is wired and compiled; no test drives it end to end (the real-process
  test uses `--online-host`).
- Nothing helps a host behind a router beyond the refusal: UPnP, hole punching and a relay are
  ADR-0092's later steps.
- A host whose lobby link drops loses its room; the host's own record of the race is not kept
  apart from a server race's (no records are shared yet either way).
- Anti-cheat: a game that came back to its race in a new process (REJOIN RACE) signs its inputs
  but keeps no watch on the host (the record holds no params hash or host key); decision 8's
  resync is not built; a
  report has no row on the room screen; item 5 (withheld inputs) is not reported; the HOST GAME
  path of the menus is wired but only `--online-host` is driven end to end.

