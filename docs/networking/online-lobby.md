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
| `oag-game --online` | `crates/game/src/online.rs`, `crates/game/src/main/cli/extra.rs` |

`oag-server` with no subcommand is the standalone race server it always was. Rule 6 of
`just check-deps` holds: the lobby is `oag-net` without the `client` feature, and the one new
dependency is `ring`'s random source, already linked.

### The wire messages

Client to lobby: `Hello { protocol, name, titles, token }` (magic `OAGL`), `ListRooms`,
`CreateRoom(settings)`, `JoinRoom { room }`, `Edit(settings)` (owner), `Ready { team, ready }`
(a pick is the ready, as Omega's ship choice is), `Start` (owner), `Leave`.

Lobby to client: `Welcome { token, name, titles }`, `Rooms(list)`, `Room(snapshot)` (pushed on
every change, owner first), `Left { reason }`, `Handoff { port, cert_hash, token, name }`,
`Rejected { code, text }`.

Named refusals (`Refusal`): `Full`, `Started`, `Content`, `Version`, `NoSuchRoom`, `NotOwner`,
`NeedPlayers`, `NotReady`, `BadName`, `NotInRoom`, `InRoom`, `BadSettings`.

Room settings (`RoomSettings`): title, track, class, mode, max players (2 to 8), AI fill.
Weapons are not a setting: a network race holds them off (ADR-0069, condition 2). Omega's laps,
barrel rolls, pilot-assist-allowed, eliminations and zone targets are not carried: `RaceParams`
has no field for them yet.

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
(`RaceJob`: the `RaceParams` plus `(name, token)` per seat) off **standard input**, not argv,
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

The deploy steps above, run for real on a remote host (x86-64 Linux, glibc 2.35, **IPv6 only**, 1 core,
957 MB RAM, no swap), with the musl build and a lean Pulse (PSP, EU) extraction (257 MB on disk, copied in 4 s).
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
  over first (`a_refusal_that_arrives_with_the_close_is_still_read`).
- Everything was removed from the host afterwards (processes killed by pid, files deleted, listing checked).

### Tests

- `wire::online` and `online::rooms` unit tests (round trips, every rule above).
- `tests/online.rs`, no disc, over real WebTransport on localhost: two rooms race one after the
  other through one slot (the second is told it is queued, then hands off only after the first
  race is over), reconnecting in the wrong order still seats by token, 0 hash mismatches, the room
  returns open with the reason; a child that crashes after coming up; a child that never comes up
  is killed; a title with no image refuses; a game client on the lobby port is refused by name.
- `net_ground_truth::the_lobby_forks_a_real_race_...` (`#[ignore]`, `just test-data`): the real
  `oag-server lobby` forks the real `oag-server race` on a real circuit, one room. **Coverage is
  honest, not complete:** every race in these tests ends at its `--max-ticks` limit, not at a
  finish line, and the second queued room races only through `ThreadRunner`; the real child is
  covered by the one-room test.
- `discovery` unit tests: `differs` and `content_fingerprint`; `online::rooms`: a PS2 pressing or
  other mods is hidden from the list and refused.

### Open

- **Menu screens: landed** (part 7). Open there: owner-side editing of a room's settings, a
  max-players choice, kick, and the sub-pages Omega has beyond the list.
- **EU and USA pressings of one platform's title are not told apart before the race** (the
  same rule as LAN); the tick-0 hash catches them, as a join timeout.
- **A session takeover does not re-send a pending hand-off**, so a client that drops during the
  race's loading and reconnects to the lobby misses it; it times out at the child and returns.
- **A SIGKILLed lobby orphans its running race child**, which exits on its own at the end of the
  race or the join timeout. No `PR_SET_PDEATHSIG` (it needs `libc`).
- **No kick, no spectators, no chat, no quick match, no invite-only**; Omega has some of them.
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

**What it loads.** The lobby fixes difficulty (`elite`), scheme (`veteran`) and weapons (off) of
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

## 7. The online menus (2026-10-16)

**Ours, chosen, not measured** except where it says Omega's. Omega's order, with our login:
MULTIPLAYER, ONLINE, a connect step, CREATE GAME or JOIN GAME, the settings (CREATE) or the room
list (JOIN), team selection, the room. Code: `crates/game/src/main/session/online.rs` (the state
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
| CREATE GAME | the RACE page, Track Select, Ship Select, then the room | Eliminator and Head to Head are dimmed (a network race holds weapons off, and Head to Head is two people in a room of up to eight); a stored one becomes a Single Race. Max players is 8, AI fill follows the RACE page |
| JOIN GAME | Ship Select, then the list: owner left, title and head count right, the circuit, class and mode (the title's own circuit name) under the title while the cursor is on it | rows the server marks not joinable are dimmed with the reason; the empty row makes a room instead; the list is asked for again every second |
| the room | one seat per person, `NAME (HOST)  READY` or `PICKING`, START RACE for the host, BACK leaves | the pick is the ready (sent once in the room, and again if an edit clears it); the line under the title is the room's note, then where the room is: the host's prompt, "number N in line" while another game races (`Phase::Queued`), the countdown |
| the race | the ordinary network race (`Pending`, tick-0 check, `launch_race`) with the seat token | the lobby link is polled in every stage and kept open for the whole race |
| after the race | the room again, with the room's note ("--max-ticks reached", a crashed child, ...) | `Session::online_menus_opened`, called as the menus reopen |

**Notices** (all on screen, none only in the log): lobby unreachable or build refused (the
connect keyboard, with the text), a join refused (room full, started, no such room: the line under
the room list for five seconds), a refused START (not enough people, someone has not picked), the
host kicked or the room gone (`Left { reason }`: a screen of its own with BACK), the lobby closing
mid-flow (the same), the race child failing to come up (the room's note after the return).

**Not in the menus:** the keyboard grid has no colon (it stays four rows), so a lobby on another
port than 60000 is written into `[online] address` once; `--online` stays the headless
command-line flow of part 5 (it does not open the screens).

**Walked on 2026-10-16** (Pulse PSP EU, a debug build, `oag-server lobby` on localhost, two
windowed `oag-game` clients each in its own Xvfb on lavapipe, driven by xdotool, mouse for the
first screens and held keys after): connect (including the refused build, with both builds named),
the hub, CREATE (RACE page, Track Select, Ship Select), JOIN (Ship Select, the list with the room
and its circuit line, the room), the host's START, the hand-off, the race to `--max-ticks 300`,
and both clients back in the room with "--max-ticks reached". Stills of the room and the list on
Pulse and HD (`--menu-page online-room:3`, `online-list:3:0`): HD draws its own strip look.

## Sources

- Omega `data09.psarc` front-end XML and `english/entries.xml`, via `scripts/psarc.py`;
  HD's same files from `hdfury-ps3-eu-dec.iso` (`DATA00`/`DATA02`).
- shadPS4 0.20.0 (`/usr/bin/shadps4`), its [quickstart](https://shadps4.net/quickstart/),
  `src/core/libraries/np/np_matching2/np_matching2.cpp` in the repository, and
  compatibility issue 487 (linked above).
- Omega `eboot.bin` v1.00 in Ghidra (`/omega/eboot-ps4-omega-eu.bin`), string searches only:
  no function was read, no name was recovered, `names.tsv` is unchanged.