# The `.vex` class table also exists here, spot-checked

2026-09-15. Data in `eboot.bin` (WipEout: Omega Collection, PS4, `CUSA05670`,
EU), image base `0x01000000`. **Not a full read** - see "What this is not"
below. Checked while working on [`billboards.md`](billboards.md), on the
theory that a table this project already recovered whole on
`ps3-hdfury-eu` ([`vex-classes.md`](../ps3-hdfury-eu/vex-classes.md),
`g_VexClassTable`, confidence 95, 866 records) is worth a cheap spot-check
here rather than assumed identical or assumed absent.

## What was checked

`search_strings("Floor Collision")` and `search_strings("cloudCube")` both
find their class-name strings verbatim on this binary. `get_xrefs_to` on
`"Floor Collision"`'s address (`0x0183363b`) names exactly one reader, a
data reference from `0x019413e8` - the same "one xref, from a table slot"
shape `ps3-hdfury-eu`'s own reading used to find its table.

`inspect_memory_content` around that address resolves the record layout as
**24-byte stride**, `{id (8 bytes, low 4 used), name pointer (8 bytes),
runtime slot (8 bytes)}` - the 64-bit-pointer analogue of HD's own 12-byte
`{id, name, ptr}` stride, and the same "one shared runtime-slot value across
every record" shape HD's page documents (every slot read here holds
`0x02053c68`, unvaried):

| Address | id | Name (read live) | Matches `ps3-hdfury-eu`? |
| --- | --- | --- | --- |
| `0x019413e0` | `0x3b9` | `Floor Collision` | Yes - HD's own `0x3b9` |
| `0x019413f8` | `0x3ba` | `Wall Collision` | Yes - HD's own `0x3ba` |

Both id/name pairs match `ps3-hdfury-eu/vex-classes.md`'s own table exactly.
**Confidence 82** for the claim "this binary carries the same
`g_VexClassTable` shape and content, at least for these two records":
decompilation-free, direct-memory evidence, but only two of 866 records
checked, and the table's own base address (where `id == 0`, `"Invalid"`,
begins) was not located - `0x019413e0` is just where these two records
happen to sit, not necessarily the table's start.

## What this is not

This is **not** a re-run of `ps3-hdfury-eu`'s own whole-table recovery. That
page's confidence-95 reading rests on all 866 records resolving with no
repeated id and a terminator found; this page checked exactly two. It also
does not locate this table's own base address, size, or terminator, and adds
no `names.tsv` row - nothing here was renamed, since a two-record spot-check
does not clear the bar a `g_VexClassTable`-shaped data symbol would need.
**Next step, if this ever matters**: search backward/forward from
`0x019413e0` at 24-byte stride for the `id == 0` sentinel, the same way
`ps3-hdfury-eu`'s own table was bounded, and diff against
`oag_vex::vex::class_names` the way `vex_class_ground_truth.rs` already
does for the other binaries.
