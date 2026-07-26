# Front-end XML

**Status: understood.** Implemented in
[`oag-formats::fexml`](../../crates/formats/src/fexml.rs).

Screens, widgets, 3D model previews, menu transitions and
[handling stats](handling-stats.md) are all XML. The front end is data-driven,
so much of the "menu code" is really menu *data*.

## Name shortening

Element and attribute names are replaced by one- or two-letter codes, with a
`<code>` element at the top of each file giving the mapping. An attribute
`Xs="Name"` maps the short name `X` to `Name`.

```xml
<code as="Values" bs="Screen" cs="Mode3D" ds="name" es="Model" ls="Src"></code>
<b d="MPLobby Info">
  <c><e d="Ship"><a l="Data\Ships\AG_Systems\ship_FE.vex"/></e></c>
</b>
```

expands to

```xml
<Screen name="MPLobby Info">
  <Mode3D><Model name="Ship"><Values Src="Data\Ships\AG_Systems\ship_FE.vex"/></Model></Mode3D>
</Screen>
```

**The dictionary is per file.** Across the 64 blobs in `FEData.wad`, every one
of the 18 short codes carries more than one meaning: `a` is `Values` in one file
and `Class` in another, `b` is `Screen` or `Difficulty`. A shared table would
silently corrupt most files, so the expander uses each file's own.

This is a size optimisation, not encryption.

## Reading it

```sh
oag-wad cat <archive> 'Data\Ships\Feisar\handlingstats.xml' --expand
```

Identified by the leading `<code`. Files that begin `<?xml` are plain and need
no expansion.

## Why it matters beyond the front end

**This is the richest source of real asset names in the game.** Most names are
built at runtime from templates like `%s\%strack%s.vex`, so they never appear in
the executable and cannot be recovered by scanning strings. The XML contains
them written out.

`fexml::asset_paths` collects them, and feeding those to
[`wad::hash_name`](wad.md#the-name-hash) resolves archive entries that binary
strings alone cannot. That is how all eight `handlingstats.xml` files were
located.

## Known elements

Recovered from expanded files, not exhaustive:

| Element | Role |
| --- | --- |
| `Screen` | A named screen or sub-screen |
| `Values` | Attribute carrier for its parent |
| `Mode3D`, `Viewport` | 3D preview embedded in a menu |
| `Model` | A `.vex` model, with `Src`, position and rotation |
| `Image` | A texture |
| `Movie` | Video playback; see [frontend-video](../ghidra/functions/psp-pulse/frontend-video.md) |
| `Redirect`, `goto` | State transitions |
| `Entry`, `Class`, `Difficulty` | Menu and mode data |
| `LoadXML` | Includes another XML file |

## Open questions

- The full element and widget set. Each widget registers a name and a vtable in
  the binary, so the complete list is recoverable from the registration sites.
- Whether the PS2 release uses the same schema. Its archives decompress now, and
  the first PS2 blob decoded happened to be a `<Screen name="Top">`, so at least
  the shape matches.
- How layout and anchoring are expressed.
