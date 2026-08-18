//! Class IDs, keyed by the file's own version word.
//!
//! # Why the version and not the title
//!
//! A class ID is a **table index, not a stable enum**: the numbering shifted
//! wholesale between format versions, so `0x125` means `Mesh` in a version-6
//! file and something else entirely in a version-4 one. Keying on which disc
//! a file came off would be both wrong and unnecessary, and
//! [ADR-0022](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md)
//! says why: the decoder reads the artifact and picks a table.
//!
//! **This is not a guess dressed up as a design.** The Pulse PSP disc ships
//! its own version-4 file - `Data\Defaults\Skycube.vex`, in an otherwise
//! version-6 archive - and its class IDs are the *older* numbering, not
//! Pulse's: `0x0ee` world, `0x373` texture, `0x378` shape, measured in
//! `crates/formats/tests/skycube_ground_truth.rs`. Its `0x373` is exactly
//! the `Texture` ID `docs/formats/pure-status.md` measured across 156 Pure
//! files. So a version-4 file uses version-4 numbering whichever disc it
//! came from, which is the whole claim this module rests on, and it was
//! testable before any Pure work started.
//!
//! # The older tables are deliberately incomplete
//!
//! [`V6`] is fully recovered. [`V4`] carries only what has been measured,
//! and [`V3`] almost nothing - a `None` here means "not recovered", never
//! "absent from the format", and per `CLAUDE.md`'s naming rules an
//! unrecovered ID gets no entry rather than a plausible one. `docs/formats/pure-status.md`
//! costs out what filling them in would take.

/// Class ID of a `Floor Collision` node.
///
/// One of Pulse's five collision classes, all registered with the same vtable by
/// `Collision_RegisterNodeClasses` (`0x08934d44`). Their payloads are decoded by
/// [`collision`](crate::collision), which also documents what is inferred rather
/// than observed about them.
pub const CLASS_FLOOR_COLLISION: u32 = 0x3b9;

/// Class ID of a `Wall Collision` node.
pub const CLASS_WALL_COLLISION: u32 = 0x3ba;

/// Class ID of a `Reset Collision` node.
pub const CLASS_RESET_COLLISION: u32 = 0x3cd;

/// Class ID of a `Mag Floor Collision` node: the magstrip surface.
pub const CLASS_MAG_FLOOR_COLLISION: u32 = 0x3e6;

/// Class ID of a `Cage Collision` node, which the loader parses and then skips.
pub const CLASS_CAGE_COLLISION: u32 = 0x3e7;

/// Class ID of a `collision_trackwall` node: the barrier along the road, as
/// distinct from [`CLASS_WALL_COLLISION`]'s wider scenery.
///
/// **Only Wipeout HD authors it, and this ID is not from a class table.**
/// Confidence 85, on four measurements over all 16 of HD's circuits -
/// one node each, all parsing, 97 % of their triangles near-vertical, and an
/// extent matching the floor's rather than the wall's. The numbers and what
/// they deliberately do *not* settle about Pulse's own table are in
/// `docs/formats/hd-status.md`; the tests are
/// `crates/game/tests/hd_trackwall_ground_truth.rs`.
pub const CLASS_TRACK_WALL_COLLISION: u32 = 0x3ed;

/// One format version's class-ID assignments.
///
/// `None` means the ID has not been recovered for that version. A decoder
/// that needs one and finds `None` should report that it cannot read the
/// file, not fall back to another version's numbering: the two tables share
/// no values, so a fallback would silently match nothing or, worse, match
/// the wrong node type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Classes {
    /// The format version this table belongs to.
    pub version: u32,
    pub mesh: Option<u32>,
    pub texture: Option<u32>,
    pub transform: Option<u32>,
    pub wo_track: Option<u32>,
    pub section: Option<u32>,
    pub skycube: Option<u32>,
    pub fogcube: Option<u32>,
    pub speedup_pad: Option<u32>,
    pub weapon_pad: Option<u32>,
    pub start_position: Option<u32>,
    pub floor_collision: Option<u32>,
    pub wall_collision: Option<u32>,
    pub reset_collision: Option<u32>,
    pub mag_floor_collision: Option<u32>,
    pub cage_collision: Option<u32>,
    /// The barrier along the road, which only Wipeout HD authors.
    ///
    /// Kept apart from [`Self::wall_collision`] because on HD they are two
    /// different things: this one is co-extensive with the floor and 97 %
    /// near-vertical, that one spans the whole environment. See
    /// [`vex::CLASS_TRACK_WALL_COLLISION`](CLASS_TRACK_WALL_COLLISION)
    /// for the measurements and for what is deliberately not claimed.
    pub track_wall_collision: Option<u32>,
    /// The node whose second child is a mesh's low-detail alternative.
    pub lod_group: Option<u32>,
    /// The keyframed transform that makes trackside scenery move.
    pub anim_transform: Option<u32>,
    /// The `Airbrake` node a flap `Mesh` hangs under.
    pub airbrake: Option<u32>,
    /// The engine flare locator on a ship.
    pub engine_flare: Option<u32>,
    /// The collision-spark locator on a ship.
    pub ship_collision_fx: Option<u32>,
}

/// The table used by every file Pulse ships except one.
///
/// Recovered in full; see each `CLASS_*` constant's own doc comment for the
/// evidence behind it.
pub const V6: Classes = Classes {
    version: 6,
    mesh: Some(super::CLASS_MESH),
    texture: Some(super::CLASS_TEXTURE),
    transform: Some(super::CLASS_TRANSFORM),
    wo_track: Some(super::CLASS_WO_TRACK),
    section: Some(super::CLASS_SECTION),
    skycube: Some(super::CLASS_SKYCUBE),
    fogcube: Some(super::CLASS_FOGCUBE),
    speedup_pad: Some(super::CLASS_SPEEDUP_PAD),
    weapon_pad: Some(super::CLASS_WEAPON_PAD),
    start_position: Some(super::CLASS_START_POSITION),
    floor_collision: Some(CLASS_FLOOR_COLLISION),
    wall_collision: Some(CLASS_WALL_COLLISION),
    reset_collision: Some(CLASS_RESET_COLLISION),
    mag_floor_collision: Some(CLASS_MAG_FLOOR_COLLISION),
    cage_collision: Some(CLASS_CAGE_COLLISION),
    // **The one entry here that no Pulse file exercises.** Version 6 is the
    // format generation, not the title, and HD's circuits are version 6 - so
    // the id belongs in this table, while the *table* is not where it was
    // recovered from. Pulse authors no node of this class at all, which
    // `collision_ground_truth.rs` establishes over every `.vex` on both its
    // discs, so listing it changes nothing Pulse decodes.
    track_wall_collision: Some(CLASS_TRACK_WALL_COLLISION),
    lod_group: Some(super::CLASS_LOD_GROUP),
    anim_transform: Some(super::CLASS_ANIM_TRANSFORM),
    airbrake: Some(super::CLASS_AIRBRAKE),
    engine_flare: Some(super::CLASS_ENGINE_FLARE),
    ship_collision_fx: Some(super::CLASS_SHIP_COLLISION_FX),
};

/// The table Pure's 156 version-4 files use, and Pulse's one legacy
/// `Skycube.vex`.
///
/// `texture` is corroborated twice and independently: `docs/formats/pure-status.md`
/// measured `0x373` across 156 Pure files by closing
/// `sum(clut_size + texel_size)` against each file's declared texture-block
/// length, and `skycube_ground_truth.rs` reads the same `0x373` out of a
/// file on the *Pulse* disc.
///
/// `mesh`, `transform` and `wo_track` come from `pure-status.md` alone, at
/// confidences 92, 90 and 94. `floor_collision` and `wall_collision` were
/// this table's own measurement first; see their comments below.
///
/// **Everything else here is the class-name table index** (2026-08-11), which
/// is a second and independent chain reaching the same `0x36b` and `0x36c`
/// and the reason `reset_collision` can be named at all. `mesh` and
/// `transform` are the exception on purpose - they sit far outside this run's
/// `0x36b..=0x383` span because they belong to a separate registration table
/// whose names are not in the string run, and each was pinned by a property
/// of its own payload instead.
pub const V4: Classes = Classes {
    version: 4,
    mesh: Some(0x11e),
    texture: Some(0x373),
    transform: Some(0x6d),
    wo_track: Some(0x36d),
    // **The eight ids below, and the two collision ids after them, are the
    // table index** - see `the_class_ids_are_the_table_index_with_three_gaps`
    // in `crates/pure/tests/class_table_ground_truth.rs`. Both executables
    // carry the exporter's class-name run in the same order, so an id is its
    // position in that run counted from `Floor Collision`, plus three gaps
    // that Pulse's own recovered constants force rather than fit. 13 of
    // Pulse's 15 land with nothing adjusted and the other 2 set the gaps.
    //
    // The prediction that makes it more than arithmetic: the first gap skips
    // exactly one id, and that id is `Texture` - independently measured at
    // `0x373` across 156 of 156 Pure files by closing
    // `sum(clut_size + texel_size)` against each declared texture-block
    // length, before this table was found.
    section: Some(0x37b),
    skycube: Some(0x378),
    fogcube: None,
    speedup_pad: Some(0x36f),
    weapon_pad: Some(0x370),
    start_position: Some(0x36e),
    // **Recovered 2026-08-12 by matching a facing signature against Pulse's
    // own known classes**, rather than by matching object counts - which is
    // all an earlier pass had, and why these two stayed `None` at 45.
    //
    // The statistic: for every triangle of every candidate class, the
    // area-weighted `|dot(normal, up)|` taken against the nearest sample of
    // the track's own `WO Track` spline. A floor faces along that axis and a
    // wall across it, and the spline is the reference rather than world `+y`
    // because these circuits bank and climb. Calibrated on Pulse first, where
    // the answer is already established at 90, then run over all 16 Pure
    // circuits:
    //
    // | class        | mean abs dot | area within 45 deg of up | area per object |
    // | ------------ | -----------: | -----------------------: | --------------: |
    // | Pulse Floor  |        0.979 |                   98.1 % |           2,348 |
    // | Pure  0x36b  |        0.992 |                   99.4 % |           4,214 |
    // | Pulse Wall   |        0.087 |                    3.7 % |           1,127 |
    // | Pure  0x36c  |        0.104 |                    1.2 % |           3,204 |
    //
    // Each lands on its Pulse counterpart within the spread Pulse's own
    // circuits show, on three statistics that do not follow from one another,
    // and both carry ~31.8 triangles per object against Pulse's 31.9 and
    // 31.7. Confidence **88**: an exact-property match on a discriminator
    // calibrated against a corpus whose answer is known, over 16 circuits,
    // with no emulator verification - which is what holds it below the 90 the
    // Pulse ids carry. Finding the registration function in Pure's own
    // `BOOT.BIN` is what would raise it.
    //
    // `crates/pure/tests/collision_classes_ground_truth.rs` re-derives that
    // table and fails if either id stops matching. See
    // `docs/formats/collision.md`.
    floor_collision: Some(0x36b),
    wall_collision: Some(0x36c),
    // **`Reset`, by the table index, and this overturns a `Cage` guess.**
    //
    // The facing pass above could not name `0x37f` and said so at 45: it
    // matches none of Pulse's four measurable signatures - mixed-facing
    // (0.402, against 0.834 for `Reset` and 0.998 for `Mag Floor`) and
    // 173,803 area units per object, nine times the largest Pulse class. That
    // reads as an enclosing shell, which made `Cage` the obvious guess, and
    // `Cage` is the one Pulse class whose signature cannot be measured at all.
    //
    // The class-name run answers it directly instead of by resemblance:
    // `Reset Collision` sits at index 19, which is this id. The two readings
    // are not in conflict once Pure's geometry is looked at rather than
    // Pulse's assumed - a downward raycast sweep puts `0x37f` under **99.9 %**
    // of the spline at a consistent ~10.7 units below the road, where Pulse's
    // `Reset` covers **5.8 %** at ~21.5 and only where a craft can leave. A
    // continuous under-road surface is what produces both the huge area per
    // object and the mixed facing, so the signature that refuted every Pulse
    // `Reset` was measuring a different *authoring style* for the same class.
    //
    // Confidence **94**: the exporter's own ordering, cross-checked on both
    // executables, with 15 Pulse anchors constraining the run and Pure's own
    // files decoding under it.
    reset_collision: Some(0x37f),
    mag_floor_collision: None,
    cage_collision: None,
    // No version-4 or version-3 file authors one; only HD does, and HD is
    // version 6.
    track_wall_collision: None,
    lod_group: None,
    // `Anim Transform` is index 8 of the same name run that gives the ten ids
    // above, which puts it at `0x372`. It stays `None` for the reason
    // `airbrake` does: no version-4 file has been checked for one and nothing
    // reads it there, so the derivation is recorded without this table
    // asserting it.
    anim_transform: None,
    // `Airbrake` *is* in the name run, at index 11, so the same arithmetic
    // gives `0x377`. It stays `None` because nothing has looked for it in
    // Pure's own files and nothing consumes it yet - the derivation is
    // written down here so the next reader does not redo it, which is not the
    // same as this table asserting the id.
    airbrake: None,
    engine_flare: Some(0x371),
    ship_collision_fx: Some(0x382),
};

/// The table Pure's 15 version-3 files use.
///
/// Nothing is recovered. Version 3 also **shortens the batch header** - it
/// carries no `alternate` pair - so a v3 file needs more than a class table
/// before it decodes; `docs/formats/pure-status.md` records that walk at
/// confidence 65.
pub const V3: Classes = Classes {
    version: 3,
    mesh: None,
    texture: None,
    transform: None,
    wo_track: None,
    section: None,
    skycube: None,
    fogcube: None,
    speedup_pad: None,
    weapon_pad: None,
    start_position: None,
    floor_collision: None,
    wall_collision: None,
    reset_collision: None,
    mag_floor_collision: None,
    cage_collision: None,
    // No version-4 or version-3 file authors one; only HD does, and HD is
    // version 6.
    track_wall_collision: None,
    lod_group: None,
    anim_transform: None,
    airbrake: None,
    engine_flare: None,
    ship_collision_fx: None,
};

/// The class table for a file's version word, or `None` for a version this
/// project has never seen.
///
/// `None` is not a decode failure to swallow: it means the file is a format
/// generation nobody has looked at, and saying so is more useful than
/// guessing at [`V6`].
#[must_use]
pub fn for_version(version: u32) -> Option<Classes> {
    match version {
        6 => Some(V6),
        4 => Some(V4),
        3 => Some(V3),
        _ => None,
    }
}
