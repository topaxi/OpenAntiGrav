//! A track's data classes: `TrackDataStorage`, `Section`, `Tile`,
//! `RacingLineContainer`, `SceneReferences` and the `BallisticMeshCollider`
//! markers, read through [`oag_unity::value::Value`] into typed structs.
//!
//! Every field here was read off the 32 Workshop tracks; what a field *means*
//! is labelled where it is declared (measured against the files, or inferred
//! from names and counts, with no second source).

use std::collections::HashMap;

use oag_unity::assets::Assets;
use oag_unity::classes::{GameObject, Transform};
use oag_unity::serialized::class_id;
use oag_unity::value::{PPtr, Value};

use crate::error::{Error, Result};
use crate::read::{array, flag, float, get, index, int, pptr, text, vec3, vec3_array};

/// An object of the bundle: the file it is in and its pointer there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRef {
    /// Index into [`Assets::files`].
    pub file: usize,
    /// The pointer, relative to that file.
    pub pptr: PPtr,
}

/// What a `Tile` is (`Tile.Type`). The pairing with the pad textures is exact
/// on every Lion's Den tile (`TrackDataStorage.*TileIndex` are atlas indices);
/// the names are inferred from that, not authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    /// Plain floor or wall (0).
    Plain,
    /// Speed pad (1), atlas index `SpeedTileIndex`.
    Speed,
    /// Weapon pad (2), atlas index `WeaponTileIndex`.
    Weapon,
    /// Recharge strip (3), atlas index `RechargeTileIndex`; inferred from its
    /// place in the pit chain.
    Recharge,
    /// Start-grid slot (4), atlas index `SpawnTileIndex`.
    Grid,
    /// A type this reader has no meaning for.
    Unknown(i64),
}

impl TileKind {
    fn from_id(id: i64) -> Self {
        match id {
            0 => Self::Plain,
            1 => Self::Speed,
            2 => Self::Weapon,
            3 => Self::Recharge,
            4 => Self::Grid,
            n => Self::Unknown(n),
        }
    }
}

/// One floor or wall tile: six vertices of the track's triangle lists.
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    /// `Tile.Index`.
    pub index: i64,
    /// `Tile.Type`.
    pub kind: TileKind,
    /// `Tile.Position`: scaled space for a floor tile, unscaled mesh space for a
    /// wall tile (measured on Lion's Den, see the format page).
    pub position: [f32; 3],
    /// `Tile.Indices`: the tile's vertices in the floor or wall arrays.
    pub indices: Vec<usize>,
    /// The section it belongs to (floor tiles only), as an array position.
    pub section: Option<usize>,
    /// `Tile.Flipped`.
    pub flipped: bool,
    /// `Tile.AtlasIndex`.
    pub atlas_index: usize,
    /// `Tile.IsWet`.
    pub wet: bool,
    /// `Tile.IsMagLock`.
    pub mag_lock: bool,
}

/// One authored track section.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// `Section.Index`; equal to its position in [`Track::sections`].
    pub index: usize,
    /// `Section.Type`: 0 plain, 2 and 3 mark a branch's first and last section
    /// (measured on Lion's Den and Stanza Inter, inferred in general).
    pub kind: i64,
    /// Position in the scaled space (the 1.15 of `[ Track Configuration ]` is in).
    pub position: [f32; 3],
    /// Rotation, `(x, y, z, w)`; forward is `rot * (0,0,1)`.
    pub rotation: [f32; 4],
    /// The surface normal (up).
    pub normal: [f32; 3],
    /// Half-width out to the wall top (inferred).
    pub width: f32,
    /// Wall height.
    pub height: f32,
    /// `Section.Next`, as an array position. `None` ends a chain.
    pub next: Option<usize>,
    /// `Section.Junction`: the first section of a branch leaving here.
    pub junction: Option<usize>,
    /// `Section.PitlaneEntance` (sic): the junction leads to the pit lane.
    pub pit_entrance: bool,
    /// Section flags this layer does not act on. Nonzero ones are reported by
    /// name so a track that uses them is not silently simplified.
    pub special: Vec<&'static str>,
}

/// One section of an AI line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RacingLinePoint {
    /// `SectionArrayIndex`.
    pub section: usize,
    /// `SectionNextArrayIndex`.
    pub next: usize,
    /// `Offset`: lateral, in the section's local x (measured on Lion's Den).
    pub offset: f32,
}

/// What a `BallisticMeshCollider` marks (`Type`). 0 and 2 are the track floor
/// and wall (the `Track Floor` and `Track Wall` objects); 1 is other solid
/// scenery (inferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColliderKind {
    /// 0.
    Floor,
    /// 1.
    Scenery,
    /// 2.
    Wall,
    /// Another value.
    Unknown(i64),
}

/// A `BallisticMeshCollider` and the object it sits on.
#[derive(Debug, Clone, PartialEq)]
pub struct Collider {
    /// The object's name.
    pub name: String,
    /// `Type`.
    pub kind: ColliderKind,
    /// The object's pose in the scene, composed up its transform chain.
    pub placement: Placement,
}

/// A pose in Unity's frame: a 3x3 matrix (rotation times scale, rows) and a
/// translation. Composed exactly from the transform chain; nothing is rounded
/// or assumed uniform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Row-major linear part.
    pub matrix: [[f32; 3]; 3],
    /// Translation.
    pub translation: [f32; 3],
}

impl Placement {
    /// The identity pose.
    pub const IDENTITY: Self = Self {
        matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation: [0.0; 3],
    };

    fn local(t: &Transform) -> Self {
        let [x, y, z, w] = t.rotation;
        let r = [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - z * w),
                2.0 * (x * z + y * w),
            ],
            [
                2.0 * (x * y + z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - x * w),
            ],
            [
                2.0 * (x * z - y * w),
                2.0 * (y * z + x * w),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ];
        let mut matrix = [[0.0; 3]; 3];
        for (row, out) in r.iter().zip(&mut matrix) {
            for (c, o) in out.iter_mut().enumerate() {
                *o = row[c] * t.scale[c];
            }
        }
        Self {
            matrix,
            translation: t.position,
        }
    }

    /// `self` applied to a point.
    #[must_use]
    pub fn apply(&self, p: [f32; 3]) -> [f32; 3] {
        let m = &self.matrix;
        std::array::from_fn(|i| {
            m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2] + self.translation[i]
        })
    }

    /// `parent` after `self`: the pose of a child placed in `parent`.
    #[must_use]
    pub fn within(&self, parent: &Self) -> Self {
        let mut matrix = [[0.0; 3]; 3];
        for (i, row) in matrix.iter_mut().enumerate() {
            for (j, o) in row.iter_mut().enumerate() {
                *o = (0..3)
                    .map(|k| parent.matrix[i][k] * self.matrix[k][j])
                    .sum();
            }
        }
        Self {
            matrix,
            translation: parent.apply(self.translation),
        }
    }

    /// Whether this is a uniform scale by `s` with no rotation or translation
    /// (to within `1e-6`), and `s`.
    #[must_use]
    pub fn uniform_scale(&self) -> Option<f32> {
        let s = self.matrix[0][0];
        let off = |a: f32, b: f32| (a - b).abs() > 1e-6;
        let ok = (0..3)
            .all(|i| (0..3).all(|j| !off(self.matrix[i][j], if i == j { s } else { 0.0 })))
            && self.translation.iter().all(|&t| t.abs() <= 1e-6);
        ok.then_some(s)
    }
}

/// The fields of `SceneReferences` this layer reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// `PhysicsMode`.
    pub physics_mode: i64,
    /// `PointToPointTrack`.
    pub point_to_point: bool,
    /// `AiSpeedMultiplier`.
    pub ai_speed_multiplier: f32,
    /// `AiLookAhead{Toxic,Apex,Halberd,Spectre,Zen}`.
    pub ai_look_ahead: [i64; 5],
    /// `TrackLocation`.
    pub location: String,
    /// `TrackDescription`.
    pub description: String,
    /// `UseFog`.
    pub use_fog: bool,
    /// `FogEndDistance`.
    pub fog_end: f32,
    /// How many weapons the track blocks.
    pub blocked_weapons: usize,
}

/// A chain of sections other than the main loop, entered by a `Junction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// The section (on the main chain or on another branch) whose `Junction` leads in.
    pub from: usize,
    /// Its sections in chain order.
    pub sections: Vec<usize>,
    /// The already-classified section the last one's `Next` leads to, if any.
    pub joins: Option<usize>,
    /// Whether `from` has `PitlaneEntance` set.
    pub pit: bool,
}

/// How the `Next` chain from the start section ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopEnd {
    /// Back at the start section.
    Closed,
    /// Into a loop that does not include the start section: `tail` sections,
    /// then a cycle through `rejoins`.
    Rho {
        /// The section the chain first revisits.
        rejoins: usize,
        /// How many sections precede it.
        tail: usize,
    },
    /// At a section with no `Next`.
    Open,
}

/// The section graph, classified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chains {
    /// The main chain from `StartSectionIndex`, in `Next` order.
    pub main: Vec<usize>,
    /// How it ends.
    pub end: LoopEnd,
    /// Each chain entered by a main-loop `Junction`.
    pub branches: Vec<Branch>,
    /// Sections on no chain this classifier reached. Zero on a fully
    /// understood track.
    pub unreached: Vec<usize>,
}

impl Chains {
    /// The sections a lap runs round, in `Next` order: the whole main chain
    /// when it closes on its start, the cycle it runs into otherwise. Empty
    /// for an open chain, which has no lap.
    #[must_use]
    pub fn lap_loop(&self) -> &[usize] {
        match self.end {
            LoopEnd::Closed => &self.main,
            LoopEnd::Rho { tail, .. } => &self.main[tail..],
            LoopEnd::Open => &[],
        }
    }
}

/// A track read out of its bundle.
#[derive(Debug, Clone)]
pub struct Track {
    /// Floor triangle list, in the mesh's own space (not yet scaled).
    pub floor_vertices: Vec<[f32; 3]>,
    /// Per-vertex floor normals.
    pub floor_normals: Vec<[f32; 3]>,
    /// Wall triangle list.
    pub wall_vertices: Vec<[f32; 3]>,
    /// Per-vertex wall normals.
    pub wall_normals: Vec<[f32; 3]>,
    /// `SpeedTileIndex`.
    pub speed_atlas: usize,
    /// `WeaponTileIndex`.
    pub weapon_atlas: usize,
    /// `RechargeTileIndex`.
    pub recharge_atlas: usize,
    /// `SpawnTileIndex`.
    pub spawn_atlas: usize,
    /// `AtlasRects` as `(x, y, width, height)` in UV space.
    pub atlas_rects: Vec<[f32; 4]>,
    /// `TrackDiffuseTexture`: the atlas the tiles' `AtlasIndex` cuts rects from.
    pub diffuse: Option<ObjectRef>,
    /// `TrackIllumTexture`.
    pub illum: Option<ObjectRef>,
    /// `TrackReflectionTexture`.
    pub reflection: Option<ObjectRef>,
    /// `StartSectionIndex`.
    pub start_section: usize,
    /// `StartLineIndex`: the section the lap line is at.
    pub start_line: usize,
    /// `MidLineIndex`: the section of the mid-lap checkpoint.
    pub mid_line: usize,
    /// The sections, in `Index` order.
    pub sections: Vec<Section>,
    /// Floor tiles.
    pub floor_tiles: Vec<Tile>,
    /// Wall tiles.
    pub wall_tiles: Vec<Tile>,
    /// The AI lines (empty when the track has no `RacingLineContainer`).
    pub racing_lines: Vec<Vec<RacingLinePoint>>,
    /// `SceneReferences`, when the track has one.
    pub scene: Option<Scene>,
    /// Every `BallisticMeshCollider`.
    pub colliders: Vec<Collider>,
    /// The section graph.
    pub chains: Chains,
    /// `MonoBehaviour`s whose script could not be resolved, so were not
    /// classified (a track with a null `m_Script`).
    pub unresolved_scripts: usize,
}

/// The fields of each class this layer reads, by class name. The field census
/// (`tests/field_census.rs`) holds the other fields of the 32 tracks to a frozen
/// list with their non-default counts, so a field the tools add is noticed.
pub const READ_FIELDS: &[(&str, &[&str])] = &[
    (
        "TrackDataStorage",
        &[
            "AtlasRects",
            "FloorNormals",
            "FloorTiles",
            "FloorVertices",
            "MidLineIndex",
            "RechargeTileIndex",
            "Sections",
            "SpawnTileIndex",
            "SpeedTileIndex",
            "StartLineIndex",
            "StartSectionIndex",
            "TrackDiffuseTexture",
            "TrackIllumTexture",
            "TrackReflectionTexture",
            "WallNormals",
            "WallTiles",
            "WallVertices",
            "WeaponTileIndex",
        ],
    ),
    (
        "Section",
        &[
            "AiAlwaysWallAvoid",
            "AiIgnoreJunction",
            "AllowOutOfBounds",
            "ForTeleporter",
            "Height",
            "Index",
            "Junction",
            "LockOntoUntilClose",
            "Next",
            "NoHoverPositionCorrection",
            "NoMaglockSounds",
            "NoTiltLock",
            "Normal",
            "PitlaneEntance",
            "Position",
            "ResetFlyBackwards",
            "Rotation",
            "RotationMultiplier",
            "Type",
            "Width",
        ],
    ),
    (
        "Tile",
        &[
            "AtlasIndex",
            "Flipped",
            "Index",
            "Indices",
            "IsMagLock",
            "IsWet",
            "Position",
            "Section",
            "Type",
        ],
    ),
];

struct Found {
    file: usize,
    path_id: i64,
}

#[derive(Default)]
struct Classes {
    storage: Vec<Found>,
    racing: Vec<Found>,
    scene: Vec<Found>,
    colliders: Vec<Found>,
    unresolved: usize,
}

fn classify(assets: &Assets) -> Result<Classes> {
    let mut names: HashMap<(usize, i64, i32), Option<String>> = HashMap::new();
    let mut out = Classes::default();
    for (fi, sf) in assets.files().iter().enumerate() {
        for o in &sf.objects {
            if o.class_id != class_id::MONO_BEHAVIOUR {
                continue;
            }
            let ty = &sf.types[o.type_index];
            let Some(&(file_id, path_id)) = usize::try_from(ty.script_type_index)
                .ok()
                .and_then(|i| sf.script_types.get(i))
            else {
                out.unresolved += 1;
                continue;
            };
            let name = names.entry((fi, path_id, file_id)).or_insert_with(|| {
                let p = PPtr { file_id, path_id };
                let (_, script) = assets.read(fi, p).ok()??;
                script.get("m_ClassName")?.as_str().map(str::to_string)
            });
            let Some(name) = name else {
                out.unresolved += 1;
                continue;
            };
            let found = Found {
                file: fi,
                path_id: o.path_id,
            };
            match name.as_str() {
                "TrackDataStorage" => out.storage.push(found),
                "RacingLineContainer" => out.racing.push(found),
                "SceneReferences" => out.scene.push(found),
                "BallisticMeshCollider" => out.colliders.push(found),
                _ => {}
            }
        }
    }
    Ok(out)
}

fn read_found(assets: &Assets, f: &Found) -> Result<Value> {
    let p = PPtr {
        file_id: 0,
        path_id: f.path_id,
    };
    assets
        .read(f.file, p)?
        .map(|(_, v)| v)
        .ok_or(Error::Dangling("a classified object"))
}

fn only<'a>(v: &'a [Found], class: &'static str) -> Result<&'a Found> {
    match v {
        [one] => Ok(one),
        [] => Err(Error::MissingClass(class)),
        _ => Err(Error::DuplicateClass(class)),
    }
}

impl Track {
    /// Reads a track from its opened bundle. Refuses, by name, a track with no
    /// or several `TrackDataStorage`, a pointer that leads nowhere, or section
    /// indices that are not their array positions.
    pub fn read(assets: &Assets) -> Result<Self> {
        let classes = classify(assets)?;
        let storage = only(&classes.storage, "TrackDataStorage")?;
        let sv = read_found(assets, storage)?;
        let sf = storage.file;
        const C: &str = "TrackDataStorage";

        let section_ptrs = array(&sv, C, "Sections")?;
        let mut position_of: HashMap<i64, usize> = HashMap::with_capacity(section_ptrs.len());
        for (i, p) in section_ptrs.iter().enumerate() {
            let p = p.as_pptr().ok_or(Error::Field(C, "Sections"))?;
            if p.file_id != 0 || position_of.insert(p.path_id, i).is_some() {
                return Err(Error::Dangling("a section pointer"));
            }
        }
        let resolve_section = |p: PPtr| -> Result<Option<usize>> {
            if p.is_null() {
                return Ok(None);
            }
            if p.file_id != 0 {
                return Err(Error::Dangling("a section pointer into another file"));
            }
            position_of
                .get(&p.path_id)
                .copied()
                .map(Some)
                .ok_or(Error::Dangling("a section pointer"))
        };

        let mut sections = Vec::with_capacity(section_ptrs.len());
        for (i, p) in section_ptrs.iter().enumerate() {
            let (_, v) = assets
                .read(sf, p.as_pptr().expect("checked above"))?
                .ok_or(Error::Dangling("a section"))?;
            let s = read_section(&v, &resolve_section)?;
            if s.index != i {
                return Err(Error::Unsupported(
                    "Section.Index is not its array position",
                ));
            }
            sections.push(s);
        }

        let read_tiles = |name: &'static str| -> Result<Vec<Tile>> {
            array(&sv, C, name)?
                .iter()
                .map(|p| {
                    let p = p.as_pptr().ok_or(Error::Field(C, name))?;
                    let (_, v) = assets.read(sf, p)?.ok_or(Error::Dangling("a tile"))?;
                    read_tile(&v, &resolve_section)
                })
                .collect()
        };
        let floor_tiles = read_tiles("FloorTiles")?;
        let wall_tiles = read_tiles("WallTiles")?;

        let atlas_rects = array(&sv, C, "AtlasRects")?
            .iter()
            .map(|r| {
                Some([
                    r.get("x")?.as_f32()?,
                    r.get("y")?.as_f32()?,
                    r.get("width")?.as_f32()?,
                    r.get("height")?.as_f32()?,
                ])
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(Error::Field(C, "AtlasRects"))?;

        let texture_ref = |name: &'static str| -> Result<Option<ObjectRef>> {
            let p = pptr(&sv, C, name)?;
            if p.is_null() {
                return Ok(None);
            }
            let l = assets.resolve(sf, p).ok_or(Error::Dangling(name))?;
            Ok(Some(ObjectRef {
                file: l.file,
                pptr: PPtr {
                    file_id: 0,
                    path_id: p.path_id,
                },
            }))
        };
        let diffuse = texture_ref("TrackDiffuseTexture")?;
        let illum = texture_ref("TrackIllumTexture")?;
        let reflection = texture_ref("TrackReflectionTexture")?;

        let mut racing_lines = Vec::new();
        if let Some(r) = classes.racing.first() {
            if classes.racing.len() > 1 {
                return Err(Error::DuplicateClass("RacingLineContainer"));
            }
            racing_lines = read_racing_lines(&read_found(assets, r)?, sections.len())?;
        }

        let scene = match classes.scene.as_slice() {
            [] => None,
            [one] => Some(read_scene(&read_found(assets, one)?)?),
            _ => return Err(Error::DuplicateClass("SceneReferences")),
        };

        let mut colliders = Vec::new();
        for f in &classes.colliders {
            colliders.push(read_collider(assets, f)?);
        }

        let start_section = index(&sv, C, "StartSectionIndex")?;
        let start_line = index(&sv, C, "StartLineIndex")?;
        let mid_line = index(&sv, C, "MidLineIndex")?;
        for (n, i) in [
            ("StartSectionIndex", start_section),
            ("StartLineIndex", start_line),
            ("MidLineIndex", mid_line),
        ] {
            if i >= sections.len() {
                return Err(Error::Dangling(n));
            }
        }
        let chains = classify_chains(&sections, start_section)?;

        let track = Self {
            floor_vertices: vec3_array(&sv, C, "FloorVertices")?,
            floor_normals: vec3_array(&sv, C, "FloorNormals")?,
            wall_vertices: vec3_array(&sv, C, "WallVertices")?,
            wall_normals: vec3_array(&sv, C, "WallNormals")?,
            speed_atlas: index(&sv, C, "SpeedTileIndex")?,
            weapon_atlas: index(&sv, C, "WeaponTileIndex")?,
            recharge_atlas: index(&sv, C, "RechargeTileIndex")?,
            spawn_atlas: index(&sv, C, "SpawnTileIndex")?,
            atlas_rects,
            diffuse,
            illum,
            reflection,
            start_section,
            start_line,
            mid_line,
            sections,
            floor_tiles,
            wall_tiles,
            racing_lines,
            scene,
            colliders,
            chains,
            unresolved_scripts: classes.unresolved,
        };
        track.check_triangle_lists()?;
        Ok(track)
    }

    fn check_triangle_lists(&self) -> Result<()> {
        for (v, n, tiles) in [
            (&self.floor_vertices, &self.floor_normals, &self.floor_tiles),
            (&self.wall_vertices, &self.wall_normals, &self.wall_tiles),
        ] {
            if v.len() % 3 != 0 || v.len() != n.len() {
                return Err(Error::Unsupported("a triangle list that is not whole"));
            }
            if tiles.iter().flat_map(|t| &t.indices).any(|&i| i >= v.len()) {
                return Err(Error::Dangling("a tile vertex index"));
            }
        }
        Ok(())
    }

    /// The floor's triangles as vertex triples, mesh space.
    #[must_use]
    pub fn floor_triangles(&self) -> Vec<[[f32; 3]; 3]> {
        triangles(&self.floor_vertices)
    }

    /// The wall's triangles as vertex triples, mesh space.
    #[must_use]
    pub fn wall_triangles(&self) -> Vec<[[f32; 3]; 3]> {
        triangles(&self.wall_vertices)
    }

    /// Where the floor object sits in the scene, from its collider marker, or
    /// `None` when the track marks no floor or marks several that disagree. The wall object's is
    /// [`Self::wall_placement`]; the arrays are in the mesh's own space.
    #[must_use]
    pub fn floor_placement(&self) -> Option<Placement> {
        self.placement_of(ColliderKind::Floor)
    }

    /// Where the wall object sits in the scene.
    #[must_use]
    pub fn wall_placement(&self) -> Option<Placement> {
        self.placement_of(ColliderKind::Wall)
    }

    fn placement_of(&self, kind: ColliderKind) -> Option<Placement> {
        let mut it = self.colliders.iter().filter(|c| c.kind == kind);
        let first = it.next()?.placement;
        it.all(|c| c.placement == first).then_some(first)
    }
}

fn triangles(v: &[[f32; 3]]) -> Vec<[[f32; 3]; 3]> {
    v.as_chunks::<3>().0.to_vec()
}

fn read_section(v: &Value, resolve: &dyn Fn(PPtr) -> Result<Option<usize>>) -> Result<Section> {
    const C: &str = "Section";
    let flags = [
        "AllowOutOfBounds",
        "NoTiltLock",
        "ResetFlyBackwards",
        "ForTeleporter",
        "LockOntoUntilClose",
        "AiIgnoreJunction",
        "NoHoverPositionCorrection",
        "NoMaglockSounds",
    ];
    let mut special = Vec::new();
    for f in flags {
        if flag(v, C, f)? {
            special.push(f);
        }
    }
    // Present on 15 of the 32 tracks (an older tools version lacks it).
    if v.get("AiAlwaysWallAvoid").is_some() && flag(v, C, "AiAlwaysWallAvoid")? {
        special.push("AiAlwaysWallAvoid");
    }
    if float(v, C, "RotationMultiplier")? != 1.0 {
        special.push("RotationMultiplier");
    }
    Ok(Section {
        index: index(v, C, "Index")?,
        kind: int(v, C, "Type")?,
        position: vec3(v, C, "Position")?,
        rotation: get(v, C, "Rotation")?
            .as_vec4()
            .ok_or(Error::Field(C, "Rotation"))?,
        normal: vec3(v, C, "Normal")?,
        width: float(v, C, "Width")?,
        height: float(v, C, "Height")?,
        next: resolve(pptr(v, C, "Next")?)?,
        junction: resolve(pptr(v, C, "Junction")?)?,
        pit_entrance: flag(v, C, "PitlaneEntance")?,
        special,
    })
}

fn read_tile(v: &Value, resolve: &dyn Fn(PPtr) -> Result<Option<usize>>) -> Result<Tile> {
    const C: &str = "Tile";
    let indices = array(v, C, "Indices")?
        .iter()
        .map(|i| i.as_i64().and_then(|i| usize::try_from(i).ok()))
        .collect::<Option<Vec<_>>>()
        .ok_or(Error::Field(C, "Indices"))?;
    Ok(Tile {
        index: int(v, C, "Index")?,
        kind: TileKind::from_id(int(v, C, "Type")?),
        position: vec3(v, C, "Position")?,
        indices,
        section: resolve(pptr(v, C, "Section")?)?,
        flipped: flag(v, C, "Flipped")?,
        atlas_index: index(v, C, "AtlasIndex")?,
        wet: flag(v, C, "IsWet")?,
        mag_lock: flag(v, C, "IsMagLock")?,
    })
}

fn read_racing_lines(v: &Value, sections: usize) -> Result<Vec<Vec<RacingLinePoint>>> {
    const C: &str = "RacingLineContainer";
    let mut lines = Vec::new();
    for line in array(v, C, "RacingLines")? {
        let mut points = Vec::new();
        for p in array(line, "RacingLine", "Points")? {
            const P: &str = "RacingLinePoint";
            let pt = RacingLinePoint {
                section: index(p, P, "SectionArrayIndex")?,
                next: index(p, P, "SectionNextArrayIndex")?,
                offset: float(p, P, "Offset")?,
            };
            if pt.section >= sections || pt.next >= sections {
                return Err(Error::Dangling("a racing line point"));
            }
            points.push(pt);
        }
        lines.push(points);
    }
    Ok(lines)
}

fn read_scene(v: &Value) -> Result<Scene> {
    const C: &str = "SceneReferences";
    Ok(Scene {
        physics_mode: int(v, C, "PhysicsMode")?,
        point_to_point: flag(v, C, "PointToPointTrack")?,
        ai_speed_multiplier: float(v, C, "AiSpeedMultiplier")?,
        ai_look_ahead: [
            int(v, C, "AiLookAheadToxic")?,
            int(v, C, "AiLookAheadApex")?,
            int(v, C, "AiLookAheadHalberd")?,
            int(v, C, "AiLookAheadSpectre")?,
            int(v, C, "AiLookAheadZen")?,
        ],
        location: text(v, C, "TrackLocation")?,
        description: text(v, C, "TrackDescription")?,
        use_fog: flag(v, C, "UseFog")?,
        fog_end: float(v, C, "FogEndDistance")?,
        blocked_weapons: array(v, C, "BlockedWeapons")?.len(),
    })
}

fn read_collider(assets: &Assets, f: &Found) -> Result<Collider> {
    let v = read_found(assets, f)?;
    let kind = match int(&v, "BallisticMeshCollider", "Type")? {
        0 => ColliderKind::Floor,
        1 => ColliderKind::Scenery,
        2 => ColliderKind::Wall,
        n => ColliderKind::Unknown(n),
    };
    let go_ptr = pptr(&v, "BallisticMeshCollider", "m_GameObject")?;
    let (l, gov) = assets
        .read(f.file, go_ptr)?
        .ok_or(Error::Dangling("a collider's GameObject"))?;
    let go = GameObject::from_value(&gov)?;
    let mut placement = Placement::IDENTITY;
    for c in &go.components {
        let Some((l2, tv)) = assets.read(l.file, *c)? else {
            continue;
        };
        if l2.info.class_id != class_id::TRANSFORM {
            continue;
        }
        let mut cur = (l2.file, Transform::from_value(&tv)?);
        placement = Placement::local(&cur.1);
        let mut depth = 0;
        while !cur.1.father.is_null() {
            depth += 1;
            if depth > 4096 {
                return Err(Error::Unsupported(
                    "a transform parent chain that does not end",
                ));
            }
            let Some((l3, fv)) = assets.read(cur.0, cur.1.father)? else {
                break;
            };
            cur = (l3.file, Transform::from_value(&fv)?);
            placement = placement.within(&Placement::local(&cur.1));
        }
    }
    Ok(Collider {
        name: go.name,
        kind,
        placement,
    })
}

fn follow(sections: &[Section], from: usize, covered: &[bool]) -> (Vec<usize>, Option<usize>) {
    let mut seen = vec![false; sections.len()];
    let mut chain = Vec::new();
    let mut cur = from;
    loop {
        if (covered[cur] && cur != from) || seen[cur] {
            return (chain, Some(cur));
        }
        seen[cur] = true;
        chain.push(cur);
        match sections[cur].next {
            Some(n) => cur = n,
            None => return (chain, None),
        }
    }
}

fn classify_chains(sections: &[Section], start: usize) -> Result<Chains> {
    let none = vec![false; sections.len()];
    let (main, back) = follow(sections, start, &none);
    let end = match back {
        Some(b) if b == start => LoopEnd::Closed,
        Some(b) => LoopEnd::Rho {
            rejoins: b,
            tail: main.iter().position(|&s| s == b).unwrap_or(0),
        },
        None => LoopEnd::Open,
    };
    let mut covered = vec![false; sections.len()];
    for &s in &main {
        covered[s] = true;
    }
    let mut branches = Vec::new();
    let mut queue: Vec<usize> = main.clone();
    while let Some(s) = queue.pop() {
        let Some(j) = sections[s].junction else {
            continue;
        };
        if covered[j] {
            return Err(Error::Unsupported(
                "a junction into a section already on a chain",
            ));
        }
        let (chain, joins) = follow(sections, j, &covered);
        if let Some(j) = joins
            && !covered[j]
        {
            return Err(Error::Unsupported("a branch that loops on itself"));
        }
        for &c in &chain {
            covered[c] = true;
        }
        queue.extend(chain.iter().copied());
        branches.push(Branch {
            from: s,
            sections: chain,
            joins,
            pit: sections[s].pit_entrance,
        });
    }
    branches.sort_by_key(|b| b.from);
    let unreached = (0..sections.len()).filter(|&i| !covered[i]).collect();
    Ok(Chains {
        main,
        end,
        branches,
        unreached,
    })
}
