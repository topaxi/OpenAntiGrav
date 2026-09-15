# Pure's DLC packs decrypt and mount

2026-09-01. Follow-up to the closed `pures-dlc-packs-are-genuinely-encrypted`
thread: Wipeout Pure's seven PSN packs (`A7`, `Delta Pack`, `Gamma Pack 1`,
`GamesRadar Pack`, `Oblivion`, `Omega Pack`, `Voice of Cod`, in `data/dlc/`)
decrypt with a public, external algorithm and key table, and are now wired
all the way through to the boot path. Full writeup, evidence and confidence:
[`docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table`](../../docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table).

**Decryption** is `oag_formats::pure_dlc` (`crypt_with_key`/`xtea8`, plus the
key-table parser). **Mounting** is `oag_game::dlc::ensure_extracted` trying
`pure_dlc::decrypt_pack` on anything that fails the plain-WAD sniff, and
`oag_pure::open_with_packs` mounting the result the way
`oag_pulse::open_with_packs` mounts Pulse's own - see
[ADR-0033](../../docs/architecture/adr/0033-external-key-material-for-decryption.md)
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
  scope unless region conversion becomes a goal. **2026-09-15**: the
  `get_xrefs_to` gap below that blocked this is now fixed project-wide (see
  next bullet), and chasing it further found the game's own XTEA cipher in
  code (`Xtea_EncryptBlock`/`Xtea_CryptBuffer`, `pure-eu` only) plus a
  function shaped exactly like a `[payload][256-byte trailer]` reader with a
  key sourced from an unidentified object. **The obvious offline test of that
  object's key candidate (`0x08aa64fc`) came back negative** - not the raw
  key `Xtea_CryptBuffer` uses on the trailer, at either half or offset
  convention - which narrows but does not close the lead: full trace,
  what's ruled out versus what's still open, in
  [dlc-download-check.md](../../docs/ghidra/functions/psp-pure-eu/dlc-download-check.md).
  The `sceUtilitySavedataInitStart` chain mentioned in earlier versions of
  this bullet *was* a dead end (ghost-replay/profile savedata, not DLC) but
  is no longer the only lead.
- **`TEST.bin`**'s actual role: **narrowed, 2026-09-15, not fully closed.**
  `BOOT.BIN`'s own `WowDownload_VerifyPackFiles`
  (`psp-pure-eu` `0x08955494`, `psp-pure-usa` `0x08955b44`) checks that
  `TEST.BIN` exists alongside `PI.WAD`/`ICON0.PNG`/`PARAM.SFO`/`PIC1.PNG`
  before treating a PSN pack download as complete - a plain file-existence
  probe, never a content read. See
  [dlc-download-check.md](../../docs/ghidra/functions/psp-pure-eu/dlc-download-check.md)
  for the full trace. This is real, evidenced use of the file, but it
  **weakens rather than confirms** the savedata-signature hypothesis below:
  the code that manages this exact five-file bundle only ever asks whether
  `TEST.bin` exists, never opens it. Whichever code (if any) reads its 16
  bytes is a different, still-unlocated call site - possibly none at all in
  `BOOT.BIN`, since neither it nor Pure's PRXs import any NpDrm or KIRK
  primitive (`docs/formats/dlc-pack.md`), so real PSN content validation may
  be entirely system-side.

  **The previous "gap" this bullet described was a relocation bug, not an
  import-stub convention problem - closed 2026-09-15.** The 2026-09-04 pass
  recorded here found zero callers of `sceUtilitySavedataInitStart`'s stub
  and zero callers of a known-must-be-called control
  (`sceCtrlReadBufferPositive`), on both `pure-eu` and `pure-usa`, and read
  that as `get_xrefs_to` failing to see through this binary's import-stub
  calling convention. It was actually the pre-2026-09-07 Allegrex relocation
  bug documented in `docs/ghidra/workflow.md` ("Why a PSP import silently
  loses every relocation") - already known to have broken `psp-pulse-eu`/
  `psp-pulse-usa` the same way, but not until now checked against Pure's own
  two databases. Both are confirmed fixed: `get_xrefs_to` on
  `sceUtilitySavedataInitStart`'s stub now returns a real caller on the first
  try, on both regions, with no workaround needed. **The stub addresses this
  bullet used to cite (e.g. `0x08a76dfc`) were never Pure's** -
  `data/ghidra/psp-imports.tsv` is `psp-pulse-usa`-addressed only
  (`scripts/apply-ghidra-names.py`'s `BINARY_PROGRAMS` mapping); Pure's own
  stub is at `0x08a42798` (`pure-eu`), found by running
  `scripts/resolve-psp-imports.py` directly against
  `data/extracted/psp/pure-eu/PSP_GAME/SYSDIR/BOOT.BIN`. The
  `SceUtilitySavedataParam.key[16]` idea from the old text is neither
  confirmed nor ruled out - on `pure-eu`, the caller chain from that stub
  goes to an unrelated feature (see the trailer bullet above), so it was
  never reached; `pure-usa`'s own chain exists (its stub has a caller too)
  but was not traced.
- **Region-selectable DLC**, raised independently while this thread's evidence
  was being gathered: Pure's packs (and potentially others, per
  [ADR-0021](../../docs/architecture/adr/0021-region-independent-dlc.md)'s
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

- **The `sceUtilitySavedataInitStart` lead is a dead end for this thread on
  `pure-eu`**, chased 2026-09-15: its caller there belongs to a ghost-replay/
  profile savedata dialog, not the DLC download path. `pure-usa`'s equivalent
  chain exists (its stub has a caller too) but was not chased - lower
  priority now that a better lead exists (next bullet). See
  [dlc-download-check.md](../../docs/ghidra/functions/psp-pure-eu/dlc-download-check.md)
  for the trace, and note its addresses supersede this thread's - the
  `0x08a76dfc`-style stub addresses recorded above are `psp-pulse-usa`'s, not
  Pure's own.
- **The game's own XTEA cipher is now found in code** (`Xtea_EncryptBlock`
  `0x088a9c64`, `Xtea_CryptBuffer` `0x088aa204`, `pure-eu` only - see
  dlc-download-check.md), matching `oag_formats::pure_dlc::crypt_with_key`'s
  algorithm structurally. Its caller `FUN_088a3118` handles a buffer shaped
  exactly like `[payload][256-byte trailer]` and reads a key from an object
  populated by twelve unnamed calls fed by a 32-byte high-entropy blob at
  `0x08aa64fc` - a real, unresolved lead, not the dead end the previous
  version of this bullet described everything as being.
  **The offline check this bullet used to point at is now done, and came
  back negative** (dlc-download-check.md's Open section has the full
  write-up): `0x08aa64fc`, either half, is not the direct `Xtea_CryptBuffer`
  key for a shipped pack's trailer at either offset convention. That rules
  out the simplest version of the hypothesis, not the lead itself - **the
  next step is decompiling `FUN_088a6824` through `FUN_088a7018`** (the
  twelve calls that build and verify the object `FUN_088a3118` reads its key
  from) to find what they actually do with the trailer and with
  `0x08aa64fc`, which needs a working Ghidra session against `pure-eu`.
  `pure-usa`'s counterparts to both named functions were not found - the
  Ghidra program for `/psp-pure-usa/BOOT.BIN` became unreachable
  (`"Disk quota exceeded"`, then `"Program not found"`) partway through the
  2026-09-15 session and had not recovered as of that session's end; check
  its state fresh before resuming.
- If JP/US copies of any pack turn up, diff their decrypted payload against
  the EU one to settle the region-selectable-DLC question one way or the
  other before building any UI for it.
