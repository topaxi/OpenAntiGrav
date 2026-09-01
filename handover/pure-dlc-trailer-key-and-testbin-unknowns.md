# Pure's DLC packs decrypt but are not mounted

2026-09-01. Follow-up to the now-closed `pures-dlc-packs-are-genuinely-encrypted`
thread: Wipeout Pure's seven PSN packs (`A7`, `Delta Pack`, `Gamma Pack 1`,
`GamesRadar Pack`, `Oblivion`, `Omega Pack`, `Voice of Cod`, in `data/dlc/`)
decrypt with a public, external algorithm and key table, verified against all
seven by turning each into a file `oag-wad` parses unmodified. Full writeup,
evidence and confidence:
[`docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table`](../docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table).
Keys are at
[`data/keys/pure-dlc-keys.txt`](../data/keys/README.md#pure-dlc-keystxt)
(gitignored research material, not yet in a form the engine can read at
runtime - see Open below).

None of this is wired up. `oag_assets::dlc::open_dir`
([`crates/assets/src/dlc.rs`](../crates/assets/src/dlc.rs)) tries
`Archive::open_file` on every file in a pack directory and silently skips
anything that fails the WAD header check - which is every one of Pure's
`pi.wad` files today, since they are ciphertext until decrypted. Pure's packs
are invisible to `oag_assets::dlc::discover` entirely; nothing mounts them,
and nothing in `oag_game::dlc` unpacks a Pure `.zip` the way it does a Pulse
one.

## Open

- **Where do the per-pack keys live for a shipped build to read them?**
  `data/keys/` is gitignored research material (same as the Vita zRIF table),
  fine for this project's own docs and scripts but not for `oag_assets` at
  runtime - a player's checkout won't have it. The keys are not copyrighted
  game content (128-bit constants from a third-party interoperability tool,
  not anything extracted from the disc), so precedent like
  `scripts/psp-import-names.txt` (a public candidate-name list, tracked in
  git) suggests they can move into tracked source - but that is a call for
  whoever picks this up to make deliberately, not something to default into.
- **The 256-byte trailer's own key** (`keys.txt`'s comment: "encrypted with a
  region-specific key embedded in BOOT.BIN") is still unlocated - a direct
  byte-pattern search in both `BOOT.BIN`s came back empty, and MIPS can
  synthesise a 32-bit constant from split immediates rather than storing it as
  a literal, so this does not rule the routine out. Not needed for reading a
  pack's content (`oag-wad` never touches the trailer), only for round-
  tripping a pack the way the upstream region-converter tool does - out of
  scope unless region conversion becomes a goal.
- **`TEST.bin`**'s actual role is still unknown - not load-bearing for
  decryption (verified: all seven packs decrypt correctly without touching
  it), most likely something the real PSP's savedata-signature check needs
  since `pi.wad` is packaged as savedata.
- **Region-selectable DLC**, raised independently while this thread's evidence
  was being gathered: Pure's packs (and potentially others, per
  [ADR-0021](../docs/architecture/adr/0021-region-independent-dlc.md)'s
  region-independent mounting) may carry region-specific assets - e.g. the
  256-byte trailer being *per-region* rather than per-pack hints the payload
  itself could differ by region too, not just the key. Only EU packs are in
  `data/dlc/` right now, so this project cannot check that claim this
  session - would need JP/US copies of the same pack to compare payloads.
  If confirmed, mounting needs a way to prefer one region's copy over another
  when both are present, rather than pure first-mounted-wins.

## Next Steps

- Decide the key-storage question above, then add an XTEA-8 decoder
  (`crypt_with_key`/`xtea8`, algorithm documented on the doc page) somewhere
  in `oag-formats` or `oag-assets` - it is pure arithmetic, no external
  dependency needed, and small enough that determinism rules apply trivially
  (unsigned wraparound only, no floats).
- Extend `oag_assets::dlc::open_dir` (or a step before it) to recognise a
  Pure-shaped pack directory (`PARAM.sfo` + `pi.wad`, no `PACK*.edat`), read
  the pack's own content ID out of `PARAM.sfo`'s `TITLE` field, try the
  matching key (by trial against the `version == 1` check, **not** by
  trusting the containing folder name - the Gamma pack's own zip folder is
  `UCES00001DGAMMAPACK`, one letter off from the `GAMMAPAK` content ID and key
  table entry, and a name-keyed lookup breaks silently on exactly that), strip
  the 256-byte trailer, and hand the rest to the existing WAD reader
  unmodified.
- A ground-truth test over `data/dlc/`'s seven Pure packs, mirroring
  `dlc_ground_truth.rs`'s Pulse coverage: mount each, assert the entry counts
  in the table on the doc page, and assert `Vanuber`'s `Ship.vex` and
  `handlingstats.xml` are readable and race-usable the way Pulse's four DLC
  teams already are.
- Once mounted, drop the "Not yet wired into `oag_assets::dlc`" lines this
  thread's evidence pages currently carry (`dlc-pack.md`, `data/README.md`).
