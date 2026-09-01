# Pure's DLC packs decrypt and mount

2026-09-01. Follow-up to the closed `pures-dlc-packs-are-genuinely-encrypted`
thread: Wipeout Pure's seven PSN packs (`A7`, `Delta Pack`, `Gamma Pack 1`,
`GamesRadar Pack`, `Oblivion`, `Omega Pack`, `Voice of Cod`, in `data/dlc/`)
decrypt with a public, external algorithm and key table, and are now wired
all the way through to the boot path. Full writeup, evidence and confidence:
[`docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table`](../docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table).

**Decryption** is `oag_formats::pure_dlc` (`crypt_with_key`/`xtea8`, plus the
key-table parser). **Mounting** is `oag_game::dlc::ensure_extracted` trying
`pure_dlc::decrypt_pack` on anything that fails the plain-WAD sniff, and
`oag_pure::open_with_packs` mounting the result the way
`oag_pulse::open_with_packs` mounts Pulse's own - see
[ADR-0033](../docs/architecture/adr/0033-external-key-material-for-decryption.md)
for the decrypt-in-process decision this needed, and
`crates/game/tests/pure_dlc_ground_truth.rs` for ground-truth coverage
(needs `data/dlc/` and `data/keys/pure-dlc-keys.txt`, so `#[ignore]`d).
Confirmed live: `just play pure --team Vanuber` races on the Gamma pack's
`Vanuber` team, ship and handling stats both loaded from the decrypted pack.

**The one thing this took that Pulse's own DLC path never needed**: Pure's
packs and Pulse's packs are discovered from two separate subtrees of the DLC
cache (`<cache>/pure/` vs `<cache>/`, decided in
`oag_game::dlc::ensure_extracted` per member, by whether it needed decrypting)
rather than one shared `oag_assets::dlc::discover` walk. Before that split, a
run that had decrypted Pure's packs once left them on disk as ordinary WADs,
indistinguishable from Pulse's own to a later walk with no key table at all -
so they mounted behind *Wipeout Pulse* on every later boot, keys or not. See
`oag_game::dlc`'s module docs and `crate::title::open_source`'s.

## Open

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
- **`data/dlc/` is one flat folder for every title in the lineage.** Working,
  but a maintainer's own download folder mixes Pulse's and Pure's zips by
  eye only; a per-title subdirectory convention (`data/dlc/pulse/`,
  `data/dlc/pure/`) was raised as worth considering. `dlc_search_path` and
  its `MAX_DEPTH = 2` walk already support that layout with no code change -
  this is a `data/README.md` convention decision, not an implementation task.

## Next Steps

- Whoever chases the region-conversion or savedata-signature questions above
  needs the upstream tool's own region-converter path re-read alongside a
  disassembly of `sceNpDrm`'s savedata check in `BOOT.BIN` - out of scope
  for a casual follow-up.
- If JP/US copies of any pack turn up, diff their decrypted payload against
  the EU one to settle the region-selectable-DLC question one way or the
  other before building any UI for it.
