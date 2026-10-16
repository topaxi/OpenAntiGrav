# Online lobby server spike: build steps

2026-10-16, from the research lane `online-lobby-research`. The flow, the needs and the
plan, with evidence and confidences, are in
[`docs/networking/online-lobby.md`](../../docs/networking/online-lobby.md). Nothing is built.

The maintainer's goal: a first spike of a server for a remote host, following Omega's lobby
where possible; one binary; one game at a time, the rest queued; later a lobby with games
spawned in containers.

## Open

Four decisions for the maintainer, ranked, with the recommendation (full text in the page):

1. Is a TCP port available for the lobby (behind the webserver)? Yes: lobby over TCP and the
   race child owns UDP 60000 (queue and rooms survive a crash). No: lobby and race in one
   process, one UDP endpoint, race in a thread.
2. Fork or thread: a forked child cannot take the QUIC connections the lobby holds, so a
   thread in v1 behind a runner seam; a child when a second port exists.
3. Identity: a chosen name plus a session token, no accounts.
4. One title per server process in v1, advertised in the room list.

Also open: why 436 `libSceNpToolkit2` imports resolve to shadPS4 stubs although the
game's own PRX loads (a shadPS4 question, not needed for the spike), and the walk past
`NetLoginPage` (needs PSN sign-in, so the screens after it are read, not seen).

## Next Steps

1. `Room` model and state machine (`Open`, `Countdown`, `Racing`, `Finished`), settings from
   Omega's `MP Select` table mapped onto `RaceParams`; unit tests that fail if a setting is
   dropped on the way to the race.
2. Lobby messages in `oag_net::wire` (edit v1 in place): list, create, join, edit settings,
   ready, start, leave, kick, a room snapshot pushed to members, named rejects.
3. Content and version check before the race loads, using the existing fingerprint.
4. Room owner (settings and START only, migrates on leave); the race stays server-authoritative.
5. One game slot with a waiting queue and a "race in progress" state.
6. The runner seam: a function from settings to a finished race, a thread first; a dead
   race returns its members to the room list with a reason.
7. Drive it with `--net-bot` clients end to end on localhost, then against the remote host
   (UDP 60000, `[::]`, glibc 2.35 build) when the maintainer says to deploy.
8. Front-end screens modelled on Omega's `OnlineMainMenu`, `Game List`, `MP Select`,
   `GameLobby`, with pointer support.
