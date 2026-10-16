# Online lobby server: the spike is built, what is left

2026-10-16. The flow, the needs and what was built, with the wire messages, ports, run and
deploy steps, are in [`docs/networking/online-lobby.md`](../../docs/networking/online-lobby.md)
(parts 1 to 3 research, part 5 the spike). Server side and a command-line client are done and
tested locally; nothing has been deployed or tried over a real network.

## Open

1. ~~The remote test~~ (done 2026-10-16, "Measured over the internet" in the doc): UDP 60000-60010 reach the host with nothing to open, two clients raced a lean Pulse over the internet with 0 bad hashes and returned to the room. Open still: only Pulse and only `--max-ticks` endings were run remotely (HD/Omega/2048 and their memory on a 1 GB host are not; the finish line and all six titles ran on localhost, part 7), and the host was IPv6-only, so a v4-only player path is untested.
2. **The menu screens.** `--online` is a flag path. Omega's order is ONLINE, CREATE or JOIN,
   settings (`MP Select`) or the game list, team, the lobby; each needs `menu.toml` rows with
   `string_id`s and English text, and pointer support. `oag_game::online::run` is the flow to
   split into a per-frame state like `session::lan`'s.
3. ~~The build at join~~ (landed 2026-10-16, part 6 of the doc): a hello of another build is refused by name. Open still: `--online-create` hardcodes AI fill off, which only matters in modes without opponents.
4. **Settings Omega has and `RaceParams` lacks**: laps, eliminations target, zone target,
   barrel rolls, pilot assist allowed. Weapons stay off (ADR-0069, condition 2).
5. **Session takeover re-sending a pending hand-off.** (The killed-lobby orphan is fixed, part 7.)
   Also open: the lobby's room note carries the reason of a race, not the places; carrying the board in `End` would let the room screen show it.
6. **Kick, spectate, chat, invite-only, quick match**: not built, Omega has some of them.
7. **Containers**: the `Runner` seam (`online::race_proc`) is where a spawner per room on its own
   port goes; the hand-off already carries a port and a hash.

## Next Steps

1. A lane for the ONLINE screens in `oag-ui-screens` over `oag_net::online::client::LobbyClient`.
2. The extraction omits Omega's guest roster (2048-era craft need a hull variant in `RaceParams`) and anything DLC; if the server grows either, the extractor's plan grows with it (`oag_net::extract::plan`).
