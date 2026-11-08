# Texture packs

**Status: fetch and deploy work; the engine does not draw a pack yet.** Two fan
texture packs for PPSSPP replace Wipeout Pulse's and Pure's textures with larger
ones. This page is how to get them, where they live, and how they travel to a
device. What a pack is and how its keys are computed:
[ppsspp-texture-pack.md](../formats/ppsspp-texture-pack.md).

| Pack | Repository | Needs |
| --- | --- | --- |
| Pulse | `reTokyo/WipEout-Pulse-Upscale-Version` (about 910 MB) | Pulse EU or USA |
| Pure | `reTokyo/WipEout-Pure-HD-Texture-Pack` (about 9 MB, user interface only) | Pure EU, USA, Japan or a demo |
| Pure + Pulse 2026 | one `textures.zip` (553 MB) in NR74W's `WipEout Mods` download; **not fetched by the script** | either game |

The two repositories are early 2020 snapshots; the 2026 `textures.zip` is the maintained
pack and reuses their keys. Put it in
`data/texture-packs/WipEout-Pure-Pulse-PPSSPP-Pack-2026-07-25/textures.zip` by hand
(or point `OAG_TEXTURE_PACK_ZIP` at it); `just push-data` carries any folder there that holds a
`textures.ini` or a `textures.zip`.

**Both repositories are unlicensed fan derivatives of the games' textures.** They
are never committed here: they live under the gitignored `data/` and `just
audit-leakage` stays green with them present.

## Fetch

```sh
just fetch-texture-packs                  # both, at the commit scripts/texture-packs.pin names
just fetch-texture-packs WipEout-Pure-HD-Texture-Pack   # one
just fetch-texture-packs --update         # move the pin to each repository's head, then commit the pin
```

A shallow clone of one commit into `data/texture-packs/<name>/`, detached at the
pin. A checkout with local edits stops the script instead of being replaced; it
deletes nothing. `OAG_TEXTURE_PACKS=<dir>` puts the checkouts elsewhere.
`scripts/texture-packs.pin` is one `name url commit` row per pack and is the only
tracked file about them.

In a git worktree, `just link-data` links the main checkout's `data/texture-packs`.

## Where a pack is looked for

A pack folder is PPSSPP's own layout (`textures.ini` and the PNGs it names, or a
`textures.zip`), so the same folder works in two places:

1. `data/texture-packs/<name>/` here, and on a device
   `<XDG_DATA_HOME>/oag/texture-packs/<name>/` (see below).
2. A PPSSPP install: `<memory stick>/PSP/TEXTURES/<game serial>/`, which is
   `~/.config/ppsspp/PSP/TEXTURES/<serial>/` on Linux. A player's existing PPSSPP
   texture folder is the second search location, so a pack installed for PPSSPP
   needs no second copy.

```sh
just fetch-texture-packs --install-ppsspp ~/.config/ppsspp          # symlinks, one per serial in [games]
just fetch-texture-packs --install-ppsspp ~/.config/ppsspp --copy   # copies instead (without .git)
```

The script refuses to replace a folder that is not its own symlink.

Both locations are on the mod search path: a pack found there is a mod that starts off, switched
on on the MODS page and drawn in a Pulse race (see [mods.md](mods.md), "Texture packs", and "Wired"
in [ppsspp-texture-pack.md](../formats/ppsspp-texture-pack.md#wired)).

## Deploy to a Steam Deck or an Android phone

```sh
just push-data deck                       # the picker lists texture-packs/<name> rows
just push-data android 'texture-packs/*'  # or by glob
```

Each pack is one picker row, marked "on device", "partial" or "-" like a disc
image, and lands in `<XDG_DATA_HOME>/oag/texture-packs/<name>/` (the app's
`files/data/oag/texture-packs/` on Android). Only files that are missing or differ
in size are copied; `.git` is left behind. The Pulse pack is 1,941 files, so the
first copy is slow.

## Checking a pack against a disc

```sh
just test-data    # crates/texture/tests/ppsspp_pack_ground_truth.rs
```

It skips when a pack is absent or not at the pinned commit. The result per pack and
directory is in [ppsspp-texture-pack.md](../formats/ppsspp-texture-pack.md#what-was-loaded-the-census).
