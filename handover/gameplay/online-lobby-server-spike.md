# Online lobby server: the spike is built, what is left

2026-10-16. The flow, the needs and what was built, with the wire messages, ports, run and
deploy steps, are in [`docs/networking/online-lobby.md`](../../docs/networking/online-lobby.md)
(parts 1 to 3 research, part 5 the spike). Server side and a command-line client are done and
tested locally; nothing has been deployed or tried over a real network.

## Open

1. **The maintainer's remote test.** Build with `scripts/build-server-portable.sh`, open UDP
   60000 (lobby) and UDP 60001 (race child), run `oag-server lobby` (steps in the doc), connect
   two `oag-game --online` clients from another machine. Not done: no remote host was touched.
2. **The menu screens.** `--online` is a flag path. Omega's order is ONLINE, CREATE or JOIN,
   settings (`MP Select`) or the game list, team, the lobby; each needs `menu.toml` rows with
   `string_id`s and English text, and pointer support. `oag_game::online::run` is the flow to
   split into a per-frame state like `session::lan`'s.
3. **The build at join.** The hello carries title, platform and mods (the room list and joins use
   `discovery::differs`), not the build: the standalone server has no build string. A client of
   another build is caught by the race child's tick-0 hash only after the countdown, as a join
   timeout. Give `oag-server` a build identity (an env var at build time) and compare it.
   Also: `--online-create` hardcodes AI fill off, which only matters in modes without opponents.
4. **Settings Omega has and `RaceParams` lacks**: laps, eliminations target, zone target,
   barrel rolls, pilot assist allowed. Weapons stay off (ADR-0069, condition 2).
5. **Session takeover re-sending a pending hand-off**, and `PR_SET_PDEATHSIG` so a killed lobby
   takes its race child with it.
6. **Kick, spectate, chat, invite-only, quick match**: not built, Omega has some of them.
7. **Containers**: the `Runner` seam (`online::race_proc`) is where a spawner per room on its own
   port goes; the hand-off already carries a port and a hash.

## Next Steps

1. Maintainer: run the remote test above and report what the host's firewall and IPv6 did.
2. A lane for the ONLINE screens in `oag-ui-screens` over `oag_net::online::client::LobbyClient`.
3. The build at join (open 3).
