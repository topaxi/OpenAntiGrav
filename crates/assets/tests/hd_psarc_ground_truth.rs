//! Wipeout HD's circuit, read end to end: disc, PSARC, `.vex`, spline.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The image has to be layer-1 decrypted first - `scripts/ps3iso.py decrypt`,
//! `docs/formats/ps3-disc.md`. Neither the key nor the decrypted image is
//! committed.
//!
//! # What this is for
//!
//! Every layer between a PS3 disc and a drawable spline in one assertion chain,
//! so a break anywhere in it names itself. Nothing above `oag-formats` had ever
//! opened a PS3 asset before this file existed, and each of the four steps was
//! separately measured but never composed:
//!
//! 1. `oag_assets::psarc::Archive` finds the archive inside the image and reads
//!    the manifest, so `track.vex` is addressable **by its real path**.
//! 2. The entry inflates to a `.vex` whose magic is `XXEV` - big-endian.
//! 3. `vex::nodes` walks its tree, closing the pre-order child-count invariant
//!    `docs/formats/hd-status.md` measures over 40 files.
//! 4. `track::parse` reads the `WO Track` payload with **no byte order passed
//!    in**, because the payload's own magic says which way round it is, and
//!    accounts for every byte of it.
//!
//! And the circuit is checked against Pulse's `16_Track`, which is the same
//! circuit in the same world coordinates - the claim that makes an HD run and a
//! Pulse capture directly comparable.

use std::path::{Path, PathBuf};

use oag_assets::psarc::Archive;
use oag_formats::{ByteOrder, collision, pads, track, vex};

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// The archive Talon's Junction lives in. Not predictable from the path - the
/// seven archives do not split by kind - so it is named rather than searched.
const ARCHIVE: &str = "PS3_GAME/USRDIR/DATA00.PSARC";

/// The circuit, by its real path inside the archive.
const TRACK_VEX: &str = "/data/environments/talons_junction/track.vex";

/// The same circuit on Pulse's UMD, for the coordinate comparison.
const PULSE_IMAGE: &str = "pulse-psp-usa.chd";
const PULSE_WAD: &str = "PSP_GAME/USRDIR/Data.wad";
const PULSE_TRACK: &str = r"Data\Environments\16_Track\track.vex";

/// Control points across every path, from `docs/formats/hd-status.md`'s survey.
const HD_POINT_COUNT: usize = 862;

/// Nodes in Talon's Junction's tree. Measured here, and far below Pulse's own
/// count for the same circuit because HD's geometry has left the `.vex`.
const HD_NODE_COUNT: usize = 826;

/// Collision objects and vertices per class, from `docs/formats/hd-status.md`'s
/// survey - which counted them with `scripts/hd-survey.py`, so this is the Rust
/// reader reproducing an independent measurement rather than restating itself.
const HD_COLLISION: [(collision::SurfaceKind, usize, usize); 4] = [
    (collision::SurfaceKind::Floor, 246, 7_588),
    (collision::SurfaceKind::Wall, 122, 4_269),
    (collision::SurfaceKind::MagFloor, 14, 357),
    (collision::SurfaceKind::Reset, 18, 627),
];

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn hd_track_vex() -> Option<Vec<u8>> {
    let path = image(PS3_IMAGE)?;
    let spec = format!("{}:{ARCHIVE}", path.display());
    let mut archive = Archive::open(&spec).expect("the archive opens");
    Some(archive.read_path(TRACK_VEX).expect("the circuit reads"))
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn an_hd_circuit_reads_from_the_disc_and_walks_as_a_big_endian_vex() {
    let Some(data) = hd_track_vex() else { return };

    assert_eq!(&data[12..16], b"XXEV", "the PS3 spelling of the magic");
    assert!(vex::has_magic(&data), "which still counts as a .vex");
    assert_eq!(vex::byte_order(&data), ByteOrder::Big);
    assert_eq!(vex::version(&data).expect("version"), 6);
    assert_eq!(
        vex::FILE_HEADER_LEN + vex::tree_len(&data).unwrap() + vex::texture_len(&data).unwrap(),
        data.len(),
        "the two declared section lengths close on the file length"
    );

    let nodes = vex::nodes(&data).expect("the node tree walks");
    assert_eq!(nodes.len(), HD_NODE_COUNT);
    assert_eq!(
        nodes.iter().map(|n| n.child_count).sum::<usize>(),
        nodes.len() - 1,
        "immediate child counts sum to node count minus one, which is what says \
         the walk did not drift and the count is a u16"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_hd_spline_parses_with_no_byte_order_passed_in() {
    let Some(data) = hd_track_vex() else { return };

    let nodes = vex::nodes(&data).expect("the node tree walks");
    let node = track::find_node(&data, &nodes).expect("a WO Track node");
    let payload = &data[node.payload()];

    assert_eq!(&payload[..4], b"WOtd", "the payload declares its own order");
    assert_eq!(track::byte_order(payload), Some(ByteOrder::Big));

    let ai = track::parse(payload).expect("the spline parses");
    assert_eq!(ai.version, 0x106, "the HD version word");
    assert_eq!(
        ai.encoded_len(),
        payload.len(),
        "and accounts for every byte, which a parse one structure out cannot"
    );
    assert_eq!(ai.point_count(), HD_POINT_COUNT);

    // Frame vectors are unit length: what says the field offsets are right
    // rather than merely that the stride is.
    let worst = ai
        .paths
        .iter()
        .flat_map(|p| &p.points)
        .flat_map(|p| [p.tangent, p.down, p.lateral])
        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        .map(|len| (len - 1.0).abs())
        .fold(0.0f32, f32::max);
    assert!(
        worst < 1e-3,
        "worst frame vector is {worst} off unit length"
    );
}

#[test]
#[ignore = "needs both a decrypted PS3 image and Pulse's UMD in data/images"]
fn talons_junction_is_pulses_16_track_in_the_same_world_coordinates() {
    let Some(hd) = hd_track_vex() else { return };
    let Some(pulse_path) = image(PULSE_IMAGE) else {
        return;
    };

    let spec = format!("{}:{PULSE_WAD}", pulse_path.display());
    let mut wad = oag_assets::Archive::open(&spec).expect("Pulse's archive opens");
    let pulse = wad.read_name(PULSE_TRACK).expect("16_Track reads");

    let spline = |data: &[u8]| {
        let nodes = vex::nodes(data).expect("walk");
        let node = track::find_node(data, &nodes).expect("a WO Track node");
        track::parse(&data[node.payload()]).expect("the spline")
    };
    let (a, b) = (spline(&hd), spline(&pulse));

    assert_eq!(
        a.point_count(),
        b.point_count(),
        "the same circuit, re-exported rather than re-laid-out"
    );

    let worst = a
        .paths
        .iter()
        .flat_map(|p| &p.points)
        .zip(b.paths.iter().flat_map(|p| &p.points))
        .map(|(x, y)| {
            let d = [
                x.pos[0] - y.pos[0],
                x.pos[1] - y.pos[1],
                x.pos[2] - y.pos[2],
            ];
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
        })
        .fold(0.0f32, f32::max);

    // `docs/formats/hd-status.md` measures 0.67 mean / 5.5 max nearest-point
    // distance. This is the stricter point-for-point form, which holds because
    // the two files happen to carry the same count in the same order; the
    // ceiling is loose enough not to be a tolerance test on float noise and
    // tight enough that a wrong pairing - the nearest other candidate scores in
    // the hundreds - could not pass it.
    assert!(
        worst < 20.0,
        "worst point-for-point distance is {worst} world units"
    );
}

/// The collision soup reads byte-swapped, with the counts the survey measured.
///
/// A collision payload is the one shape here that **cannot** declare its own
/// order: its first word is `0xffffffff`, a palindrome. `collision::from_vex`
/// takes it from the containing file's magic, and the check that says it got it
/// right is the same one that settled the layout on the PSP - every node
/// consuming its payload down to the 16-byte alignment padding, which
/// `from_vex` enforces rather than merely offering.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_hd_collision_soup_reads_byte_swapped_with_every_payload_accounted_for() {
    let Some(data) = hd_track_vex() else { return };

    let nodes = collision::from_vex(&data).expect("every collision node decodes");

    for (kind, objects, vertices) in HD_COLLISION {
        let of_kind: Vec<_> = nodes.iter().filter(|n| n.kind == kind).collect();
        assert_eq!(
            of_kind
                .iter()
                .map(|n| n.geometry.meshes.len())
                .sum::<usize>(),
            objects,
            "{kind:?} objects"
        );
        assert_eq!(
            of_kind
                .iter()
                .flat_map(|n| &n.geometry.meshes)
                .map(|m| m.vertices.len())
                .sum::<usize>(),
            vertices,
            "{kind:?} vertices"
        );
    }

    // Read the other way round it does not merely disagree, it does not parse:
    // the chunk headers stop being 1, 2 and 3 and the declared stride stops
    // matching what the type implies.
    let node = vex::nodes(&data).expect("walk")[nodes[0].node_index].clone();
    assert!(
        collision::parse_chunks(&data[node.payload()], ByteOrder::Little).is_err(),
        "a big-endian payload read little-endian must not produce geometry"
    );
}

/// The collision surface lies under the spline it belongs to.
///
/// Cross-format rather than within one: the soup and the spline are separate
/// payloads in separate nodes, so this is what says both were decoded in the
/// same coordinate system rather than each self-consistently in its own.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_hd_collision_floor_bounds_contain_the_circuit_it_belongs_to() {
    let Some(data) = hd_track_vex() else { return };

    let nodes = vex::nodes(&data).expect("walk");
    let node = track::find_node(&data, &nodes).expect("a WO Track node");
    let ai = track::parse(&data[node.payload()]).expect("the spline");

    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for mesh in collision::from_vex(&data)
        .expect("collision")
        .iter()
        .filter(|n| n.kind == collision::SurfaceKind::Floor)
        .flat_map(|n| &n.geometry.meshes)
    {
        for v in &mesh.vertices {
            for axis in 0..3 {
                lo[axis] = lo[axis].min(v[axis]);
                hi[axis] = hi[axis].max(v[axis]);
            }
        }
    }

    // Generous on the vertical axis: a control point sits on the surface, and
    // the floor soup is a shell around a circuit that climbs and dives.
    let slack = [0.0f32, 40.0, 0.0];
    for point in ai.paths.iter().flat_map(|p| &p.points) {
        for axis in 0..3 {
            assert!(
                point.pos[axis] >= lo[axis] - slack[axis]
                    && point.pos[axis] <= hi[axis] + slack[axis],
                "control point {:?} is outside the floor soup {lo:?}..{hi:?}",
                point.pos
            );
        }
    }
}

/// Both pad classes read, and every trigger volume is the right way up.
///
/// A pad's payload is a `Mesh` payload, and HD's `Mesh` payload is where the
/// large structural change is: the batches went to `.rcsmodel` and what stayed
/// behind is the bounding-box pair at `+0x10`/`+0x20` - which is exactly the
/// field a pad's trigger volume is. So the pads survive the change that stops
/// `--mesh` drawing anything, and `docs/formats/hd-status.md` counted them:
/// 416 speedup and 208 weapon pads across the 28 circuits, `min <= max`
/// componentwise on all 624.
///
/// The counts here are Talon's Junction's share of that, measured by this
/// reader. The `min <= max` half is not asserted separately because
/// `PadVolume::parse` already refuses an inverted box - a pad that came back at
/// all passed it.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn both_hd_pad_classes_read_and_sit_on_the_circuit() {
    let Some(data) = hd_track_vex() else { return };
    let nodes = vex::nodes(&data).expect("walk");

    let speedup = pads::volumes(&data, &nodes, vex::CLASS_SPEEDUP_PAD);
    let weapon = pads::volumes(&data, &nodes, vex::CLASS_WEAPON_PAD);
    assert_eq!(speedup.len(), 18, "speedup pads");
    assert_eq!(weapon.len(), 9, "weapon pads");

    // Every pad's centre is near the driveable surface. This is the check that
    // says the *transform chain* composed big-endian too: a pad's own payload
    // is a box in local space, and only the parent chain puts it on a circuit.
    let node = track::find_node(&data, &nodes).expect("a WO Track node");
    let ai = track::parse(&data[node.payload()]).expect("the spline");
    let points: Vec<[f32; 3]> = ai
        .paths
        .iter()
        .flat_map(|p| &p.points)
        .map(|p| p.pos)
        .collect();

    // Horizontal distance, deliberately: `PadVolume::parse` applies the bind's
    // own vertical expansion (2 units down, 8 up), so a pad's centre is lifted
    // above the surface by construction and a 3-D distance would be measuring
    // that as much as the placement.
    let worst = speedup
        .iter()
        .chain(&weapon)
        .map(|pad| {
            let c = pad.centre();
            points
                .iter()
                .map(|p| ((c[0] - p[0]).powi(2) + (c[2] - p[2]).powi(2)).sqrt())
                .fold(f32::MAX, f32::min)
        })
        .fold(0.0f32, f32::max);

    // A circuit's half-widths are of order 20 units and a pad sits within one.
    // What this rules out is the failure that matters: an untransformed pad
    // reads at the origin, hundreds of units from the nearest control point.
    assert!(
        worst < 20.0,
        "furthest pad is {worst} units from the nearest control point, horizontally"
    );
}
