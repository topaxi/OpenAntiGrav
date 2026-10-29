---
status: blocked-maintainer
titles: [all]
updated: 2026-10-29
summary: "A campaign may race another mod's track (`<mod id>:<path>`), but no manifest declares it: dependencies, optional ones and a reach rule are open"
size: M (8)
difficulty: medium (25)
---

# A mod names no dependencies yet

Since 2026-10-29 a campaign event may race a track another installed mod carries:
`file = "oag.hillside:tracks/hillside.track.toml"` (`oag_game::campaign::custom`,
`track_file_in`; `docs/formats/custom-campaign.md`, "Track file"). The example mods
use it: `example-campaign/` races Hillside out of the track-only `hillside/`
(github.com/OpenAntiGrav/mod-examples).

The reference is the only dependency. Nothing in `oag-mod.toml` says that
`oag.example-campaign` needs `oag.hillside`. That has three consequences.

1. **Breakage is found per campaign, late.** A missing mod, or one the player turned
   off on the MODS page (ADR-0078, 2026-10-29), leaves the campaign out when it
   compiles for a title; the log names the mod and the campaign mod's MODS entry says
   `NEEDS HILLSIDE, OFF` or `NEEDS <id>, MISSING` (`oag_game::mods::note_left_out`).
   Nothing installs or names the missing mod beforehand, and the whole campaign goes,
   not only the events racing the missing track.
2. **Any mod can reach any other.** An event may name any installed mod's file. When
   that mod changes its layout or is removed, every mod pointing into it partly
   breaks, and no declaration says which mods are affected.
3. **No version.** A track mod that changes its simulation (Hillside's pads did, on
   2026-10-29) silently re-times every campaign event racing it.

The maintainer raised this on 2026-10-29 and asked for a thread rather than machinery
now.

## Open

- A manifest key, e.g. `[dependencies] "oag.hillside" = ">=1.0"` (semver, as `oag` is
  already), and optional dependencies whose absence leaves only the events using them
  out rather than the whole campaign.
- A reach rule: may a `<mod id>:` reference name only a declared dependency? A
  reference to an undeclared mod would then be a shape error when the mod is read
  (`oag_mod::campaign`), not a compile-time miss.
- What the MODS page shows for a mod with an unmet dependency beyond today's NEEDS
  line, whether turning a mod off should offer to turn its dependents off too, and
  whether a dependency's `titles` must cover the dependent's.
- Load order and conflicts: none today, since mods only add. They matter once a mod
  can replace another's file.
- Other asset kinds: pilots are per mod (`pilots/`); a cross-mod pilot reference would
  need the same rule.

- From the MODS on/off lane (ADR-0078, 2026-10-29): a campaign needing an off or missing mod's
  track is left out whole; leaving out only the events that need it is open (it pairs with
  optional dependencies above). `just mods-list` ignores the game's `[mods] disabled` setting and
  lists every mod as on. `--menu-page` applies only some of the boot's title trims, so a captured
  MODS page can differ from the live one (that drift predates the lane).

## Next Steps

1. Decide the manifest shape (key, semver, optional) and the reach rule with the
   maintainer; an ADR, since it changes what a mod may say.
2. `oag_mod::manifest`: read and check the key; `oag_mod::load`: report an unmet
   dependency per mod, as an unmet `oag` requirement is reported.
3. `track_file_in`: refuse a reference to an undeclared mod, once the rule says so.
4. Declare `oag.hillside` in `mod-examples/example-campaign/oag-mod.toml`.
