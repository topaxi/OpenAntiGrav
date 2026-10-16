# Online lobby server: the spike is built, what is left

2026-10-16. The flow, the needs and what was built, with the wire messages, ports, run and
deploy steps, are in [`docs/networking/online-lobby.md`](../../docs/networking/online-lobby.md)
(parts 1 to 3 research, part 5 the spike). Server side and a command-line client are done and
tested locally; nothing has been deployed or tried over a real network.

## Open

1. ~~The remote test~~ (done 2026-10-16, "Measured over the internet" in the doc): UDP 60000-60010 reach the host with nothing to open, two clients raced a lean Pulse over the internet with 0 bad hashes and returned to the room. Open still: only Pulse and only `--max-ticks` endings were run remotely (a finish line, HD/Omega/2048 and their memory on a 1 GB host are not), and the host was IPv6-only, so a v4-only player path is untested.
2. ~~**The menu screens.**~~ Landed 2026-10-16 (`docs/networking/online-lobby.md` part 7). Landed 2026-10-16 (`online-menus-titles`): the owner's ROOM SETTINGS and MAX PLAYERS, a typed port and IPv6 literal, a live walk on all five titles, the pointer on Pure and HD. Open: kick, laps in `RoomSettings`, the Omega sub-pages.
3. ~~The build at join~~ (landed 2026-10-16, part 6 of the doc): a hello of another build is refused by name. Open still: `--online-create` hardcodes AI fill off, which only matters in modes without opponents.
4. **Settings Omega has and `RaceParams` lacks**: laps, eliminations target, zone target,
   barrel rolls, pilot assist allowed. Weapons stay off (ADR-0069, condition 2).
5. **Session takeover re-sending a pending hand-off**, and `PR_SET_PDEATHSIG` so a killed lobby
   takes its race child with it.
6. **Kick, spectate, chat, invite-only, quick match**: not built, Omega has some of them.
7. **Containers**: the `Runner` seam (`online::race_proc`) is where a spawner per room on its own
   port goes; the hand-off already carries a port and a hash.

## Next Steps


1. ~~The ONLINE screens~~ landed; follow-ups are in item 2 above.
2. The extraction omits Omega's guest roster (2048-era craft need a hull variant in `RaceParams`) and anything DLC; if the server grows either, the extractor's plan grows with it (`oag_net::extract::plan`).
