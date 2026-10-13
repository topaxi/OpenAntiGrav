# `scripts/psarc.py` returns the wrong file from Omega's PS4 archives

2026-10-13, found while reading the original's split-screen wording. **A trap, not
yet fixed.** `just psarc cat` on an Omega `dataNN.psarc` can hand back a different
entry under the requested path: the script maps PS4 archive paths to entries by
position, and the PS4 family's manifest order does not match its table of contents
the way the PS3 archives' does ([psarc.md](../../docs/formats/psarc.md), "The PS4 Omega
Collection family").

Seen twice in one archive (`data09`):

- `racebox_definition.xml` came back as a 5,475-byte `<PI_Grid>` file; the real one
  (15,805 bytes) has HD's `NumberOfPlayers` and `SplitScreen` lists. A doc claim that
  Omega's race box had "neither list" rested on it and is corrected
  ([hd-frontend.md](../../docs/formats/hd-frontend.md#racebox-authors-local-split-screen-a-players-list-and-a-cut)).
- The English `entries.xml` came back as a shader blob, and the real string table sits
  under another file's name.

The Rust reader matches paths by their digest and returns the right file:
`cargo build -p oag-assets --example psarc_cat`, then
`target/debug/examples/psarc_cat <psarc> <path>`. The game itself reads through the
Rust reader, so this is a tooling bug, not a runtime one.

## Open

- **Every Omega claim built on `just psarc` output is unchecked** until re-read with the
  Rust reader. Re-read 2026-10-13 (`split-entry-mirror-hd`): the race box's split lists, the
  Ship Select screens (`Team Selection Player 1`/`2` exist in `data09`) and Omega's rows of the
  split-screen, selection-screens and campaign-screens pages. Still worth a sweep:
  `docs/formats/omega-status.md` and the rest of `docs/formats/hd-frontend.md`'s Omega rows.
- Fix or retire the script: match by digest as the Rust reader does, or make
  `just psarc cat` call the Rust example for PS4 archives and refuse positional lookup.

## Next Steps

1. Make `just psarc cat` digest-matched (or route it to `psarc_cat`). About an hour.
2. Re-read the Omega claims listed above with it. Half a day.
