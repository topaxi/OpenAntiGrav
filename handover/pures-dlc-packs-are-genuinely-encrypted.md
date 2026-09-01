# Pure's PSN DLC packs are genuinely encrypted, unlike Pulse's

Was "one team's `Ship.vex` does not resolve by name": four of the original
five turned out to be Pulse's own DLC teams (`Auricom`, `Harimau`, `Icaras`,
`Mantis`/Mirage - see [`docs/formats/dlc-pack.md`](../docs/formats/dlc-pack.md)).
The fifth, `Van_Uber`, is not on either Pulse's or Pure's base-disc roster -
checked directly against both `Data\Plugins\PI001\Definition.xml` files, not
inferred from a name-hash miss - so it isn't "unaccounted for" in the sense
the other four were. It's very likely Pure PSN content: `data/dlc/` holds
seven Wipeout Pure packs (`A7`, `Delta Pack`, `Gamma Pack 1`, `GamesRadar
Pack`, `Oblivion`, `Omega Pack`, `Voice of Cod`), each a `PARAM.sfo` +
`pi.wad` pair, and `pi.wad` measures 8.0000 bits/byte across its whole
length - genuinely encrypted or otherwise unstructured, unlike Pulse's
`PACKn.edat` (plaintext WAD under a misleading extension). See
[`docs/formats/pure-status.md`](../docs/formats/pure-status.md#van_uber-is-pure-content-not-a-pure-team-on-this-disc)
and [`data/README.md`](../data/README.md).

External sources (not project evidence - nothing here has read a byte of
`pi.wad`) place a team called Van-Über in the Gamma pack specifically, and
say it originated in Wipeout Fusion (out of this project's scope) before
returning in Pure as DLC. Worth using to prioritise which pack to attack
first, not worth citing as a finding.

Pure's packaging shape is also worth noting: `ICON0.png` + `PARAM.sfo` +
`PIC1.png` + a 16-byte `TEST.bin` + `pi.wad`, closer to a general PSN content
package than Pulse's four-`.edat`-plus-`PARAM.pbp` shape. `TEST.bin`'s
role is unexamined beyond its size and high-entropy-looking bytes.

## Open

- Whether Pure's NpDrm usage matches Pulse's single-argument
  `sceNpDrmEdataSetupKey` (no per-title key) or differs - Pure is a
  different executable from Pulse and was not checked at all.
- What `TEST.bin` is for, and whether it's load-bearing for decryption (a
  license/key-derivation artefact) or unrelated packaging metadata.
- Whether all seven Pure packs share one key/scheme or each is per-title,
  the way Pulse's region-independent mounting depended on checking rather
  than assuming ([ADR-0021](../docs/architecture/adr/0021-region-independent-dlc.md)).

## Next Steps

- Open Pure's `BOOT.BIN` in Ghidra (unencrypted ELF, no decryption step
  needed to get this far - see `docs/reverse-engineering/toolchain.md`) and
  find its NpDrm imports the way `dlc-pack.md`'s Pulse read did:
  `sceNpDrmEdataSetupKey`'s call sites and whether `sceNpDrmSetLicenseeKey`
  is imported at all.
- If Pure does supply a per-title key, that key - not a zRIF - is what
  `pi.wad` needs; the Vita PSN toolchain in `data/README.md` (zRIF ->
  klicensee, `psvpfsparser`) is a different DRM family (Vita PFS) and won't
  apply directly, but is worth reading for the general shape of a PSN
  key-derivation writeup.
- Once one `pi.wad` decrypts, check its content against the disc's own
  `.wad`/`.vex` decoders unmodified, the same way Pulse's `PACKn.edat`
  needed no new parser - the manifest-entry-0 pattern
  (`Data\Plugins\PI001\Definition.xml` fragment) may or may not carry over,
  since Pure's packaging is already shaped differently around it.
