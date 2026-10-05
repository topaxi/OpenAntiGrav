# ADR-0033: External key material lives under `data/keys/`, gitignored, and its absence is never an error

## Status

Accepted.

## Context

Wipeout Pure's seven PSN DLC packs are genuinely encrypted, unlike Pulse's
`.edat` packs (plaintext under a misleading extension). The algorithm is a
hand-rolled 8-round XTEA stream cipher the game implements itself, and the
per-pack keys are public - not extracted from any disc, not console- or
account-bound, published by a third party
([`wipeout-pure-dlc2dlc`](https://gitlab.com/thp/wipeout-pure-dlc2dlc), ISC
license). See
[`docs/formats/dlc-pack.md`](../../formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table).

This is a new *shape* of dependency for the engine, not just a new format.
Every previous case where this project handles a key or a license artefact -
the PS3 disc's own `.dkey`, the Vita klicensee derived from a zRIF in
`data/keys/vita-zrif.tsv` - is **preprocessing**: a one-time step, run by a
maintainer with a tool in `scripts/`, that turns an encrypted dump into
something Ghidra or the game can read directly. Nothing downstream of that
step ever asks "do I have a key" at runtime.

Pure's DLC is different: **the shipped engine itself**, on every boot, has to
decide whether it can decrypt a pack it just found, using key material nothing
in this repository provides. That is a runtime question, not a one-time prep
step, and it needed a decision on where that material lives and what happens
when it does not - and, separately, whether decryption happening *in the
engine at all* was the right shape, given that every existing precedent for a
key this project handles was preprocessing, not a runtime code path. That
second question had not been decided anywhere before this: `oag_source::dlc`
already unpacks a Pulse `.zip` into `data/cache/dlc/` once and reuses the
result, so "does Pure's decryption belong in that same step, or in a separate
offline tool the way `scripts/vita-self-decrypt.py` works" was a real, open
choice, not something inherited from an earlier decision.

Two things this project has already settled and this decision inherits rather
than reopens:

- **No copyrighted content in the repository** ([ADR-0006](0006-no-copyrighted-content.md)).
  Not directly in point - a key is not game content - but the same instinct
  applies: this project has not established that *redistributing* a
  third-party key table is permitted, and the cost of being wrong is higher
  than the cost of asking a player to source it themselves, the same way disc
  images already work.
- **A player mounts content they own** ([ADR-0021](0021-region-independent-dlc.md)).
  That ADR's packs needed no key at all. This one needs one, but the
  ownership story is the same: the engine reads what a player already has,
  and ships none of it.

## Decision

**External key material is user-supplied data, exactly like a disc image or a
DLC zip - not a build artefact and not embedded in a crate.**

Concretely:

1. It lives under `data/keys/`, which is entirely gitignored except its own
   `README.md` (the same rule `/data/*` already applies to disc images and
   DLC packs). `data/keys/README.md` documents provenance and format for
   each table it describes; the table itself is never committed.
2. A crate that needs to *use* key material takes it as a parameter -
   `oag_formats::pure_dlc::decrypt_pack(data, keys: &[DlcKey])` - and never
   reads a path itself. Only the composition root
   (`oag_source::dlc::load_pure_dlc_keys`) knows where the file lives and what
   happens if it is not there.
3. **A missing key file is not an error, anywhere in the chain.** It parses
   to an empty table, which makes every pack that needs a key fail to
   decrypt, which is reported exactly the way a corrupt or irrelevant zip
   already is - one line, not a hard failure. A checkout with no
   `data/keys/pure-dlc-keys.txt` sees Pure's packs exactly as it did before
   this feature existed.
4. The algorithm and the format decoder it produces are ordinary,
   documented, versioned code - what is kept out of the repository is
   specifically the *key values*, not the ability to use them.

This generalises past Pure's DLC. Any future title, pack, or asset that needs
externally-sourced key material to decrypt follows the same shape: a
gitignored table under `data/keys/` with a tracked `README.md` explaining
where it came from, a pure function that takes the parsed keys as an
argument, and graceful, silent absence rather than a hard requirement.

### Decryption happens in-process, in the existing unpack-to-cache step

**Not in a new standalone tool.** `oag_source::dlc::ensure_extracted` already
turns a Pulse `.zip` into plain files under `data/cache/dlc/` once, on
whichever boot first needs it, and reuses that result afterwards. Pure's
packs take the same path: `ensure_extracted` tries the plain WAD-header sniff
first, and only if that fails does it try `pure_dlc::decrypt_pack` against
whatever `keys` names, writing the **decrypted** bytes to the cache in place
of the ciphertext. From `oag_assets::dlc` downward - everything that actually
mounts and reads a pack - there is no difference between a Pulse `.edat` and
a decrypted Pure `pi.wad`; both are plain files by the time anything gets
there.

This is deliberately **not** the shape of `scripts/vita-self-decrypt.py`,
which is a standalone script a maintainer runs once, by hand, outside the
game, to prepare something for Ghidra. That shape did not fit here: this
decryption's output is not RE tooling input, it is an asset the running game
needs to mount, on whatever machine a player has, without a maintainer's
involvement. Reusing the cache step that already exists for exactly that
reason - "the game needs a plain file on disk, derived once from something
the player supplies" - was the smaller change, not a new subsystem.

## Alternatives considered

**Embed the keys in tracked source**, the way `scripts/psp-import-names.txt`
(a public PSP SDK candidate-name list) is committed. Rejected for now: that
file is unambiguously public technical data with no plausible ownership
claim; a third party's DLC decryption key table has not been vetted the same
way, and getting this wrong is a one-way door a maintainer cannot undo by
deleting a file - history remembers it. Nothing here rules embedding back in
later if that vetting happens; it would be a narrower ADR, not a reversal of
this one.

**Fetch keys at runtime from the network** (the source repository, or a
mirror). Rejected: a build that phones home on boot is a bigger change than
this feature needs, and every other piece of user-supplied data in this
project - discs, DLC, save state - already works offline from `data/`.

**Decrypt with a standalone offline tool**, run once by a maintainer, in the
shape of `scripts/vita-self-decrypt.py` or `scripts/resolve-psp-imports.py` -
producing plain `pi.wad` files that `oag_assets::dlc` would then find with no
engine-side decryption at all. Rejected, but closer than the other two
alternatives: this project's RE tools already work this way, and it would
have kept `oag_formats`/`oag_game` free of any crypto code whatsoever. What
it costs is a manual step outside the game for every player who owns Pure's
DLC, for content this project has already decided a player mounts by pointing
`--dlc` at a folder they own ([ADR-0021](0021-region-independent-dlc.md)) -
adding "and run this script first" to that story is a worse experience for a
gain (no crypto in the engine) that the code size does not justify: the
algorithm is under sixty lines of integer arithmetic with no dependency.
The existing `data/cache/dlc/` unpack-once step already gives the same
"pay the cost once, not every boot" property a separate tool would, without
a second thing to remember to run.

## Consequences

**A checkout works identically with or without the key table.** Nothing
regresses for a maintainer who never sources it; Pure's packs are found,
fail to decrypt, and are reported the same way an unrelated zip already was
before this feature existed.

**The key table can vanish and the reason is documented, not mysterious.**
The same property `data/keys/vita-zrif.tsv`'s absence already has: a
maintainer who deletes `data/` and re-populates it from `data/README.md`
gets image and DLC zips back but not this table, and `data/keys/README.md`
says why and where to get it again.

**A test that needs decryption to actually run needs two independent
optional inputs, not one.** `data/dlc/` and `data/keys/pure-dlc-keys.txt` can
each be present or absent independently, so a ground-truth test gates on
both - see `crates/game/tests/pure_dlc_ground_truth.rs`.

**`oag_formats` and `oag_game` now carry decryption code**, which
`docs/formats/dlc-pack.md`'s own framing of Pulse's packs ("there is no
decryption step to write") no longer holds for the project as a whole. It is
small, dependency-free and unit-tested, but it is a real precedent: the next
format that turns out to be encrypted will reach for this same shape rather
than for a new offline tool, on the reasoning above - so getting this
decision wrong here would have been wrong more than once.

**This does not solve the harder version of the same question.** `pi.wad`'s
own 256-byte trailer is, per its own source's comment, "encrypted with a
region-specific key embedded in `BOOT.BIN`" - a key this project would
recover by reverse engineering the disc's own executable, not source
externally. That case is not decided here, because it has not arisen yet:
nothing in this engine reads the trailer at all. If it does, it is a
`docs/ghidra/` recovery with a confidence score, not a `data/keys/` table -
the two are different provenances and this ADR only speaks to the second.
