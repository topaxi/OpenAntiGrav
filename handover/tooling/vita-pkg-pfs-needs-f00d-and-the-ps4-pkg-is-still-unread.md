# A Vita `.pkg` needs the console's F00D; the PS4 `.pkg` is feasible and unread

2026-10-07, lane `direct-read`. What landed: an encrypted PS3 ISO read in place
([ps3-disc.md](../../docs/formats/ps3-disc.md)), the chooser's disc-key prompt,
and a Vita `.vpk` read in place with its patch and DLC
([vita-package.md](../../docs/formats/vita-package.md)). What did not, and why.

## Open

- **Vita `.pkg` + zRIF read in place is blocked, not just unwritten.** The outer
  AES-CTR layer reads (public keys, `pkg2zip`). The PFS layer under it uses a file
  key that `psvpfsparser` gets from the F00D service (`USE_KEYGEN`); the offline
  derivations (key = klicensee with an HMAC-SHA1 mask over a salt or a `dbseed`;
  the SHA-1 `generate_enckeys` form) did not reproduce the known plaintext of the
  264-byte `template.xml`. Confidence 75. If someone can show a derivation that
  does, the reader is: outer CTR, `files.db` B-tree, per-sector AES-CBC with a
  tweak mask, no hash checking. The reader names the refusal today.
- **A real NoNpDrm `.vpk` has not been read.** The `.vpk` reader is proven on ZIPs
  built from the extracted folders with the system `zip`. A real dump may carry a
  layout the stand-ins do not (a different top folder, `work.bin`, a vpk of an
  already patched game with the patch merged into the base).
- **Deflated `.vpk` entries are inflated whole**, 512 MiB LRU, 1 GiB per entry. A
  deflated multi-gigabyte `data.psarc` is refused by name. A checkpointed
  (zran-style) index would lift it; not needed if real dumps store it.
- **The PS4 Omega `.pkg` (base and the day-one patch) is not read in place.** It
  is a fake package (`\x7fCNT`, `EP9000-CUSA05670_00-...`) that `LibOrbisPkg`
  extracts on a PC with no console key, so the keys are public constants and it is
  feasible. Needs: the entry table, the RSA-2048 key entry (a big-integer modexp:
  a dependency or our own), HMAC-SHA256, AES-XTS for `pfs_image.dat`, the PFS inode
  and directory walk, and the PFSC compressed-file layer; the patch's `data09.psarc`
  is required for a front end. Estimate: a day or two with the data on hand, and
  `data/extracted/ps4` is the byte-for-byte ground truth.
- **Wayland paste in the key prompt is unverified**; X11 Ctrl+V was exercised.
- **The `data1` IV in the PS3 key derivation comes from RPCS3's source** and was not
  exercised against a real `data1`; the oracle guards it.

## Next Steps

1. Ask the maintainer for a real NoNpDrm `.vpk` of 2048 and run
   `crates/2048/tests/vpk_ground_truth.rs` shapes against it (15 minutes).
2. PS4 `.pkg`: write `oag_disc::ps4_pkg` against `LibOrbisPkg`'s `PkgReader` and
   `PfsReader` (MIT), comparing every file with `data/extracted/ps4/omega-eu`.
3. If a Vita PFS derivation turns up, add `oag_disc::vita_pfs` behind the same
   `PackageSource` trait the `.vpk` reader uses; `DiscImage` needs no change.
