# Online lobby server: the spike is built, what is left

2026-10-16. The flow, the needs and what was built, with the wire messages, ports, run and
deploy steps, are in [`docs/networking/online-lobby.md`](../../docs/networking/online-lobby.md)
(parts 1 to 3 research, part 5 the spike). Server side and a command-line client are done and
tested locally; nothing has been deployed or tried over a real network.

## Open

1. **The maintainer's remote test.** Build with `scripts/build-server-portable.sh`, open UDP
   60000 (lobby) and UDP 60001 (race child), `oag-server extract` locally and copy the extraction (no disc image goes to the host), run `oag-server lobby` (steps in the doc), connect
   two `oag-game --online` clients from another machine. Not done: no remote host was touched.
2. ~~**The menu screens.**~~ Landed 2026-10-16 (`docs/networking/online-lobby.md` part 7). Open: owner-side room settings editing and a max-players choice (a room is 8 with the RACE page's AI), kick, the Omega sub-pages, a live walk on HD, 2048 and Omega (stills only there), a lobby on a port other than 60000 typed on the pad.
3. ~~The build at join~~ (landed 2026-10-16, part 6 of the doc): a hello of another build is refused by name. Open still: `--online-create` hardcodes AI fill off, which only matters in modes without opponents.
4. **Settings Omega has and `RaceParams` lacks**: laps, eliminations target, zone target,
   barrel rolls, pilot assist allowed. Weapons stay off (ADR-0069, condition 2).
5. **Session takeover re-sending a pending hand-off**, and `PR_SET_PDEATHSIG` so a killed lobby
   takes its race child with it.
6. **Kick, spectate, chat, invite-only, quick match**: not built, Omega has some of them.
7. **Containers**: the `Runner` seam (`online::race_proc`) is where a spawner per room on its own
   port goes; the hand-off already carries a port and a hash.

## Next Steps

1. Maintainer: run the remote test above and report what the host's firewall and IPv6 did.
2. ~~The ONLINE screens~~ landed; follow-ups are in item 2 above.
3. The extraction omits Omega's guest roster (2048-era craft need a hull variant in `RaceParams`) and anything DLC; if the server grows either, the extractor's plan grows with it (`oag_net::extract::plan`).
