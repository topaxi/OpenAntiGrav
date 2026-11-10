# The files beside a `.vex`

A PS3, Vita or PS4 circuit or ship is a `.vex` scene tree whose geometry,
animation and visibility live in files beside it. **One resolver names them**:
`oag_mesh::mesh::rcs::siblings::Siblings` (`crates/mesh/src/mesh/rcs/siblings.rs`).
Before it, four functions in two files and a private one in `oag_raceplay`
each knew a piece of the rule.

## The rule

Every companion of a `.vex` shares one stem, and the stem has one of three
spellings. Each companion is the stem plus its extension: `.rcsmodel`,
`.rcsskeleton`, `.rcsanimclip`, `.pvs`.

| spelling | stem | where | evidence |
| --- | --- | --- | --- |
| plain | `<dir>/track` | every ship; HD; 2048; Omega's four `zone_N` circuits | a `.vex` and its `.rcsmodel` are one path with another extension on every pair on the disc |
| cooked | `<dir>/track.final` | Omega's 22 other circuits, forward and reversed | [omega-status.md](omega-status.md): `track.rcsmodel` does not exist beside them, and no directory holds both spellings |
| zone | `<dir>/trackZone` | 2048's Zone-mode model | a model of its own (37 MB against the race model's 17 on `altima`) with its own skeleton, clip and PVS; the reversed circuit ships none, so both directions name the same file ([2048-animation.md](2048-animation.md)) |

`.vex` and `.rcsmodel` are matched case-insensitively, both containers being so.
HD ships no `.rcsskeleton` or `.rcsanimclip` (its scenery animation is in the
`.vex`); the resolver names them anyway and a caller reads them where they
resolve.

## Why a function in `oag-mesh` and not a `Title` axis

ADR-0058 makes per-title behaviour `Title` data. This is not per title: Omega
uses both the plain and the cooked spelling, one per circuit, and which one a
circuit has is a fact about the archive. A title key could only say "Omega may
use `.final`", and the caller would still probe to find out which, so the axis
would add a table without removing the probe. The rules are therefore data in
one module (`Spelling`, the extension constants, `ZONE_STEM`), and the resolver
answers with **candidates in the order a loader tries them**:

- `Siblings::probe(vex, zone)`: zone (when asked), plain, cooked. The caller
  owns the archive and takes the first whose model reads. The Zone content
  check (`is_psp2` and not `is_ps4`, so Omega's PS4 Zone models are passed
  over) stays with the caller, where the blob is read
  (`oag_raceplay::load::geometry::sibling_model`).
- `Siblings::beside(model)`: the set of a model already found, whatever its
  spelling. The loaders carry the model's name, so the skeleton, clip and PVS
  follow the model that was actually found.
- `Siblings::plain(vex)` and `oag_mesh::mesh::rcs::sibling_name`, which is its
  `.model`: for the callers that want only the plain model (a ship's hull, a
  weapon, a preview, the gantry). They keep exactly the probe they had; none of
  them tries `.final`, so no preview or gantry reads a name it did not read
  before.

`sibling_name` stays as a one-line wrapper because about twenty callers in
`oag-livery`, `oag-game` and `oag-raceplay` want only the plain model.

## Census

`crates/mesh/tests/rcs_siblings_ground_truth.rs` walks the table of contents of
every archive of each title (no blob read) and, for every `.vex`, in a Zone and
a non-Zone race, compares the resolver's model, skeleton, clip and PVS names
with what the old rules answered (taken before they were deleted), asserts they
are identical, and now asserts that every companion a model names exists where
the title ships one. Measured on this machine:

| title | `.vex` entries | with a model | skeleton | clip | pvs | Zone model answers |
| --- | --- | --- | --- | --- | --- | --- |
| HD (7 PSARCs, decrypted disc) | 721 | 623 | 0 | 0 | 28 | 0 |
| 2048 (base, patch v1.04, DLC1, DLC2; EU) | 1,182 | 974 | 58 | 58 | 38 | 40 |
| Omega (base and patch, EU) | 1,385 | 1,183 | 58 | 58 | 34 | 50 |

Old and new answers were identical on every `.vex` of all three titles, in
both modes.

One deliberate narrowing: the old `animation_names_beside` matched the
`.rcsmodel` suffix case-sensitively and the old PVS rewrite case-insensitively;
the resolver matches it case-insensitively. A loader only ever passes a name
the resolver itself produced (lower-case extension), so no real input differs.

## 2048 and Omega

**Ported.** One resolver serves both, and the census runs on both: Omega's
`.final` set and 2048's Zone set were the two rules that existed for them.
HD uses the plain set and ships no skeleton or clip.

## Not covered here

- Texture names and decoding: a separate lane.
- `oag_vex::kdcol::sibling_name` (`track.vex` to `track_col.col`, the
  collision) and `oag_raceplay::load::environment::psp2_sky_sibling_name`
  ([2048-sky.md](2048-sky.md)) are other rewrites on a `.vex` name and are
  untouched; they pair different files.
