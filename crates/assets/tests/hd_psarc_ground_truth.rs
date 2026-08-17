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
use oag_formats::{ByteOrder, track, vex};

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
