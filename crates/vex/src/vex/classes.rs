//! Class IDs, keyed by the file's own version word.
//!
//! # Why the version and not the title
//!
//! A class ID is a **table index, not a stable enum**: numbering shifted
//! wholesale between format versions, so `0x125` is `Mesh` in version 6 and
//! something else in version 4. The decoder reads the artifact and picks a
//! table, per
//! [ADR-0022](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md).
//!
//! Measured: Pulse's PSP disc ships one version-4 file, `Data\Defaults\Skycube.vex`,
//! in a version-6 archive, with the *older* IDs (`0x0ee` world, `0x373` texture,
//! `0x378` shape; `crates/vex/tests/skycube_ground_truth.rs`). Its `0x373` is the
//! `Texture` ID `docs/formats/pure-status.md` measured across 156 Pure files, so
//! a version-4 file uses version-4 numbering whichever disc it came from.
//!
//! # The older tables are deliberately incomplete
//!
//! [`V6`] is fully recovered, [`V4`] carries only what has been measured, [`V3`]
//! almost nothing. `None` means "not recovered", never "absent from the
//! format", and an unrecovered ID gets no entry rather than a plausible one.
//! `docs/formats/pure-status.md` costs out filling them in.

/// Class ID of a `Floor Collision` node.
///
/// One of Pulse's five collision classes, all registered with one vtable by
/// `Collision_RegisterNodeClasses` (`0x08934d44`); payloads are decoded by
/// [`collision`](crate::collision).
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
/// **Only Wipeout HD authors it, and this ID is not from a class table**
/// (Pulse's table, read to its terminator, stops three entries short of HD's,
/// and this is one of the three). Confidence 85, on four measurements over all
/// 16 of HD's circuits: one node each, all parsing, 97 % of triangles
/// near-vertical, extent matching the floor's rather than the wall's. See
/// `docs/formats/hd-status.md`; tests in
/// `crates/game/tests/hd_trackwall_ground_truth.rs`.
pub const CLASS_TRACK_WALL_COLLISION: u32 = 0x3ed;

/// Class ID of an `absorb` node: where Wipeout HD plays `WO_WEAPON_ABSORB` when
/// a craft absorbs a pickup.
///
/// **From HD's executable, not a Pulse table.** `FUN_002d8bb8` in
/// `/hdfury/EBOOT-ps3-hdfury-eu.elf` registers class `0x3ee` against
/// `ShipAbsorbNode_Importer.h`; `FUN_002d8e58`, the constructor, stamps the type
/// token the absorb feedback's collector matches on. 37 of 39 HD `Locators.vex`
/// author six (`Absorb_1`..`Absorb_6`); Detonator's and Zone's none. Confidence
/// 85; see `docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md`.
pub const CLASS_ABSORB: u32 = 0x3ee;

/// Class ID of a `cannon_flash` node: the two locators (`cannon_flash_left`,
/// `cannon_flash_right`) Wipeout HD hangs its Cannon's muzzle flash on.
///
/// **From HD's executable.** `ShipCannonFlash_Importer` registers it; the attach
/// step `FUN_002d8ec0` stores the node on the craft at `+0x5f38` when its
/// lowercased name contains `left`, `+0x5f3c` otherwise, the two muzzles
/// `CannonManager` alternates between. Confidence 85; see
/// `docs/ghidra/functions/ps3-hdfury-eu/cannon.md`.
pub const CLASS_CANNON_FLASH: u32 = 0x3eb;

/// One format version's class-ID assignments.
///
/// `None` means not recovered. A decoder that needs one and finds `None` should
/// report it cannot read the file, not fall back to another version's numbering:
/// the tables share no values, so a fallback matches nothing or the wrong type.
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
    /// Apart from [`Self::wall_collision`]: on HD this one is co-extensive with
    /// the floor and 97 % near-vertical, that one spans the whole environment.
    /// See [`CLASS_TRACK_WALL_COLLISION`].
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
    /// The locator Wipeout HD plays its weapon-absorb burst at ([`CLASS_ABSORB`]).
    pub absorb: Option<u32>,
    /// The road-span table a Quake ripples - see [`crate::quake`].
    pub quake: Option<u32>,
}

/// The table used by every file Pulse ships except one; recovered in full (see
/// each `CLASS_*` constant).
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
    // **The one entry no Pulse file exercises.** Version 6 is the format
    // generation, not the title, and HD's circuits are version 6; Pulse authors
    // no node of this class (`collision_ground_truth.rs`, every `.vex` on both
    // discs), so listing it changes nothing Pulse decodes.
    track_wall_collision: Some(CLASS_TRACK_WALL_COLLISION),
    lod_group: Some(super::CLASS_LOD_GROUP),
    anim_transform: Some(super::CLASS_ANIM_TRANSFORM),
    airbrake: Some(super::CLASS_AIRBRAKE),
    engine_flare: Some(super::CLASS_ENGINE_FLARE),
    ship_collision_fx: Some(super::CLASS_SHIP_COLLISION_FX),
    // Same reason as `track_wall_collision`: HD's registration, no Pulse file.
    absorb: Some(CLASS_ABSORB),
    quake: Some(crate::quake::CLASS_QUAKE),
};

/// The table Pure's 156 version-4 files use, and Pulse's one legacy
/// `Skycube.vex`.
///
/// `texture` is corroborated twice: `docs/formats/pure-status.md` measured
/// `0x373` across 156 Pure files (closing `sum(clut_size + texel_size)` against
/// the declared texture-block length), and `skycube_ground_truth.rs` reads it
/// from a file on the *Pulse* disc. `mesh`, `transform` and `wo_track` come from
/// `pure-status.md` alone (confidences 92, 90, 94).
///
/// Most of the rest is the class-name table index (a second, independent chain
/// reaching the same `0x36b` and `0x36c`); `mesh`, `transform`, `lod_group` and
/// `fogcube` sit outside the `0x36b..=0x383` run because they are generic Maya
/// classes recovered from their own payload or node shape.
pub const V4: Classes = Classes {
    version: 4,
    mesh: Some(0x11e),
    texture: Some(0x373),
    transform: Some(0x6d),
    wo_track: Some(0x36d),
    // **These ids and the two collision ids after them are the table index**
    // (`the_class_ids_are_the_table_index_with_three_gaps`, `crates/pure/tests/
    // class_table_ground_truth.rs`). Both executables carry the exporter's
    // class-name run in the same order, so an id is its position counted from
    // `Floor Collision` plus three gaps Pulse's recovered constants force: 13 of
    // Pulse's 15 land unadjusted and the other 2 set the gaps. The first gap skips
    // exactly `Texture`, independently measured at `0x373` on 156 of 156 Pure
    // files before the table was found.
    section: Some(0x37b),
    skycube: Some(0x378),
    // **`fogCube`, structural** (a generic Maya class absent from the run). Seven
    // of Pure's 16 circuits carry `fogCubeShapeN` under a `fogCube3`-style
    // `Transform`, class `0x385`, payload exactly [`crate::fog::PAYLOAD_LEN`]
    // (128 bytes). All ten decoded volumes carry `edge == 500.0`, the figure
    // `crate::fog` records for Pulse, an invariant this reproduces rather than
    // assumes. Confidence **94**: exact length and `edge` across every instance,
    // not runtime-verified.
    fogcube: Some(0x385),
    speedup_pad: Some(0x36f),
    weapon_pad: Some(0x370),
    start_position: Some(0x36e),
    // **Floor and wall, by a facing signature matched against Pulse's classes**
    // rather than object counts (all an earlier pass had, hence `None` at 45).
    // For every triangle, the area-weighted `|dot(normal, up)|` against the
    // nearest sample of the track's own `WO Track` spline (circuits bank and
    // climb, so not world `+y`), calibrated on Pulse, then run over all 16 Pure
    // circuits:
    //
    // | class        | mean abs dot | area within 45 deg of up | area per object |
    // | ------------ | -----------: | -----------------------: | --------------: |
    // | Pulse Floor  |        0.979 |                   98.1 % |           2,348 |
    // | Pure  0x36b  |        0.992 |                   99.4 % |           4,214 |
    // | Pulse Wall   |        0.087 |                    3.7 % |           1,127 |
    // | Pure  0x36c  |        0.104 |                    1.2 % |           3,204 |
    //
    // Each lands on its Pulse counterpart on three independent statistics, both
    // with ~31.8 triangles per object against Pulse's 31.9 and 31.7. Confidence
    // **88**: exact-property match on a discriminator calibrated against a known
    // corpus, no emulator verification; finding the registration function in
    // Pure's `BOOT.BIN` would raise it.
    // `crates/pure/tests/collision_classes_ground_truth.rs` re-derives the table;
    // see `docs/formats/collision.md`.
    floor_collision: Some(0x36b),
    wall_collision: Some(0x36c),
    // **`Reset`, by the table index, overturning a `Cage` guess.** The facing
    // pass could not name `0x37f` (mixed-facing 0.402 against 0.834 for `Reset`
    // and 0.998 for `Mag Floor`; 173,803 area units per object, nine times
    // Pulse's largest), which suggested an enclosing `Cage`. The class-name run
    // answers directly: `Reset Collision` is index 19, this id. A downward
    // raycast sweep agrees: `0x37f` lies under **99.9 %** of the spline ~10.7
    // units below the road (Pulse's `Reset`: **5.8 %** at ~21.5), a continuous
    // under-road surface that explains both signatures, a different authoring
    // style for the same class. Confidence **94**: the exporter's own ordering,
    // cross-checked on both executables, 15 Pulse anchors constraining the run.
    reset_collision: Some(0x37f),
    mag_floor_collision: None,
    cage_collision: None,
    // Only HD authors one, and HD is version 6.
    track_wall_collision: None,
    // **`LodGroup`, structural** (a generic Maya class absent from the run): each
    // of Pure's six reachable team ships carries one `lodGroup1`, class `0x2de`,
    // with exactly two `Transform` children, the shape the `oag-render` reader
    // assumes. `crates/pure/tests/class_table_ground_truth.rs` re-derives it.
    // Confidence **88**: exact name and child-count structure on six ships, not
    // runtime-verified.
    lod_group: Some(0x2de),
    // **`Anim Transform`**, index 8 of the run, `0x372`. Every one of Pure's 16
    // circuits carries at least one, 332 in total, and **every one** parses as a
    // valid [`crate::vex::anim_transform`] payload (a decoder that fails loudly
    // on a key array past the payload end). Confidence **94**: an exact
    // invariant (332 of 332 close under the decoder's bounds checks); the rubric
    // caps the band as not runtime-verified.
    anim_transform: Some(0x372),
    // **`Airbrake`**, index 11 of the run, `0x377`. Six of Pure's eight team
    // ships are reachable on this pressing (two hash to nothing under the
    // Pulse-derived directory names), each carrying exactly two `0x377` nodes
    // named `Airbrake_Left`/`Airbrake_Right` (case varies by team) with a single
    // `Mesh` (`0x11e`) child under a `Transform` parent, the shape
    // `crates/mesh/src/mesh.rs` assumes. Before this Pure's flaps rendered
    // undeflected. Confidence **90**: exact names and structure on six of six
    // ships; not runtime-verified.
    airbrake: Some(0x377),
    engine_flare: Some(0x371),
    ship_collision_fx: Some(0x382),
    absorb: None,
    quake: None,
};

/// The table Pure's 15 version-3 files use. Nothing is recovered; version 3 also
/// **shortens the batch header** (no `alternate` pair), so it needs more than a
/// class table (`docs/formats/pure-status.md`, confidence 65).
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
    // Only HD authors one, and HD is version 6.
    track_wall_collision: None,
    lod_group: None,
    anim_transform: None,
    airbrake: None,
    engine_flare: None,
    ship_collision_fx: None,
    absorb: None,
    quake: None,
};

/// The class table for a file's version word, or `None` for a version this
/// project has never seen (a format generation nobody has looked at, which is
/// more useful said than guessed at as [`V6`]).
#[must_use]
pub fn for_version(version: u32) -> Option<Classes> {
    match version {
        6 => Some(V6),
        4 => Some(V4),
        3 => Some(V3),
        _ => None,
    }
}
