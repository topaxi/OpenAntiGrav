---
status: measure
titles: [all]
updated: 2026-10-20
summary: "Lobby: session takeover re-send, spectate, chat, invite-only and quick match unbuilt; remote runs cover only Pulse"
---

# Online lobby server: the spike is built, what is left

2026-10-16. The flow, the needs and what was built, with the wire messages, ports, run and
deploy steps, are in [`docs/networking/online-lobby.md`](../../docs/networking/online-lobby.md)
(parts 1 to 3 research, part 5 the spike). Server side and a command-line client are done and
tested locally; nothing has been deployed or tried over a real network.

## Open

1. ~~The remote test~~ (done 2026-10-16, "Measured over the internet" in the doc): UDP 60000-60010 reach the host with nothing to open, two clients raced a lean Pulse over the internet with 0 bad hashes and returned to the room. Open still: only Pulse and only `--max-ticks` endings were run remotely (HD/Omega/2048 and their memory on a 1 GB host are not; the finish line and all six titles ran on localhost, part 7), and the host was IPv6-only, so a v4-only player path is untested.
2. ~~**The menu screens.**~~ Landed 2026-10-16 (`docs/networking/online-lobby.md` part 8). Landed 2026-10-16 (`online-menus-titles`): the owner's ROOM SETTINGS and MAX PLAYERS, a typed port and IPv6 literal, a live walk on Pure, HD, 2048 and Omega, the pointer on Pure and HD. Kick landed 2026-10-19 (item 6). Open: the Omega sub-pages that are not settings (friends, invite-only; the settings rows landed, item 4).
3. ~~The build at join~~ (landed 2026-10-16, part 6 of the doc): a hello of another build is refused by name. Open still: `--online-create` hardcodes AI fill off, which only matters in modes without opponents.
4. ~~**Settings Omega has and `RaceParams` lacks**~~ Landed 2026-10-19 (`docs/networking/online-lobby.md` part 9): laps, barrel rolls, Pilot Assist allowed (a level per seat) and the zone target travel through the wire, the lobby, `race::prepare` and the room screen on every title. Open: the eliminations target (not a kill count; an Eliminator room uses the mode's own), and a live pointer click on the new rows on each title.
5. **Session takeover re-sending a pending hand-off.** (The killed-lobby orphan is fixed, part 7.)
   ~~The room note carries the reason, not the places~~ (landed 2026-10-19, part 11 of the doc: the board rides `End` and the room snapshot, the end screens name every person, the room shows each seat's place; RACE AGAIN asks the room for a rematch).
6. **Spectate, chat, invite-only, quick match**: not built, Omega has some of them. ~~Kick~~ landed 2026-10-19 (part 11: the owner's confirm on a seat, refused back by name while the room exists; Omega authors no host kick, so it is ours). Open from that lane: a pad-only walk of the kick; Pure, Omega and 2048 live (their end screens are the built-in table, named, 2048's over its touch pages: **checked, differs** from HD's grid, not walked), Omega's authored 'GO TO NEW LOBBY' / return countdown (`ER_GOTO_NEW_LOBBY`, `ONL_IGM_RTL_COUNTDOWN`) not wired, a ban that outlives the room (none: by design), a person who returns to the room by any route is ready (a READY toggle row would need a pad and pointer row on every title's room screen).
7. **Containers**: the `Runner` seam (`online::race_proc`) is where a spawner per room on its own
   port goes; the hand-off already carries a port and a hash.

8. **Weapons landed 2026-10-19** (ADR-0071, `docs/networking/online-lobby.md` part 10): predicted simulation plus a discrete-state correction trigger, the room's Weapons row, Eliminator open online. Follow-up 2026-10-19 (`online-weapons-polish`): a fire first seen in a quiet step is now shown late (weapon cues, blasts, shield bulges, `oag_raceplay::catch_up`); loss and jitter measured (ADR-0071 follow-up); room stills and a pad and pointer test on the Weapons row. Open: the launch flash and screen flash start fresh rather than aged, the player-visible effect is mostly the sound (no camera-visible blast in the forced scenes), ~~an analog human stream with weapons~~ (a chosen proxy measured 2026-10-19, `docs/networking/README.md` "An analog stick"; a recorded pad stream is still owed); Head to Head online (two seats in a room of eight).

9. **ADR-0069's conditions (2026-10-19, `netcode-conditions`)**: 2 to 4 answered (ADR-0069 follow-up), condition 1 (the Galaxy S24 replay cost) open, deferred by the maintainer. Open from it: a recorded pad stream (`oag_raceplay::hand::Hand` is the stand-in; fly a pack with a controller through `--record-ghost` and replay it through `reconcile_predict_human_sweep`); a shot's late first appearance has a tail under loss and jitter (62, 137 and 152 ticks in 3 of 12 client runs) that nothing caps; a seat-1 client's wall contact effects, countdown cue, reticle, boost kick and flaps now follow its seat (2026-10-19, `seat1-contact-fx`); open: a second local person's sparks and shake (split screen, multi-window), the zone announcement gate, the wreck-ended tick and the engine throttle have no test of their own (the real seat-1 camera bug, the PVS placement and the audio levels are fixed, 2026-10-19).

- **`online::a_client_of_another_build_reads_the_refusal_before_the_close` flaked once
  under load** (2026-10-20, the lead's quick run at load average 71: FAIL in 0.08 s, then 4 of 4
  PASS alone and the whole quick suite green on the rerun). At 0.08 s it is not the 20 s timeout,
  so it is most likely the test's other assertion, "the link closed before the refusal was read":
  the server's close can overtake the refusal on a starved machine. Either the refusal is not
  guaranteed to arrive before the close (a real ordering bug a slow client would also hit) or the
  test polls too narrowly. Not investigated.

## Next Steps


1. ~~The ONLINE screens~~ landed; follow-ups are in item 2 above.
2. The extraction omits Omega's guest roster (2048-era craft need a hull variant in `RaceParams`) and anything DLC; if the server grows either, the extractor's plan grows with it (`oag_net::extract::plan`).
