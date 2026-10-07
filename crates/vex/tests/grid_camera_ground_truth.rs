//! The pre-race flyby, read off `start_grid.vex`, against the running original.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What the samples are
//!
//! Eight camera poses read off Pulse PSP (PPSSPP v1.20.4, `UCUS98712`) on `03_Track`, Time
//! Trial, with `scripts/psp-flyby.py`: at each, the grid-camera animation's own clock
//! (`grid_camera1`'s `+0x40`, in seconds) and the camera node's published world matrix. The
//! clock is the original's own - it is not fitted to the pose - so the test is not circular.
//!
//! **The pose lags the clock by one frame.** The camera node composes its world matrix from the
//! animation's local matrix as of the *previous* update, so a pose is compared against the clock
//! read one frame **before** it. Against the clock of the same frame the median error over the
//! whole flyby is `0.74` units; one frame back it is `5e-5` (p99 `3e-4`).

use oag_vex::grid_camera::GridCamera;
use std::path::PathBuf;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn grid(archives: &mut oag_assets::source::Archives, circuit: u32) -> Option<GridCamera> {
    let name = format!(r"Data\Environments\{circuit:02}_Track\start_grid.vex");
    GridCamera::read(&archives.read_name(&name).ok()?)
}

/// `(clock one frame before, eye, right row, up row, back row)`, `03_Track`.
#[allow(clippy::type_complexity, clippy::excessive_precision)]
const SAMPLES: &[(f32, [f32; 3], [f32; 3], [f32; 3], [f32; 3])] = &[
    (
        0.616943,
        [-368.90509, -9.64131, 892.10944],
        [0.09424, 0.0, -0.99555],
        [0.36300, 0.93116, 0.03430],
        [0.92701, -0.36461, 0.08778],
    ),
    (
        2.952194,
        [-402.80350, -16.44973, 787.96179],
        [-0.30059, 0.0, -0.95375],
        [0.54280, 0.82225, -0.17111],
        [0.78423, -0.56913, -0.24714],
    ),
    (
        5.287323,
        [-436.70123, -23.25835, 683.82843],
        [-0.63355, 0.0, -0.77370],
        [0.51030, 0.75163, -0.41791],
        [0.58156, -0.65958, -0.47617],
    ),
    (
        7.122391,
        [-41.52916, 40.92772, -272.30597],
        [0.41365, 0.0, 0.91043],
        [-0.14089, 0.98795, 0.06400],
        [-0.89947, -0.15475, 0.40867],
    ),
    (
        11.292496,
        [501.27814, 60.71626, -153.20154],
        [-0.59138, 0.0, -0.80639],
        [0.44707, 0.83223, -0.32791],
        [0.67112, -0.55444, -0.49214],
    ),
    (
        14.628876,
        [518.47742, 45.96870, -142.70447],
        [-0.51399, 0.0, -0.85780],
        [0.23859, 0.96053, -0.14301],
        [0.82395, -0.27817, -0.49369],
    ),
    (
        19.633606,
        [-389.16635, 2.35172, -10.83780],
        [-0.79670, 0.0, 0.60437],
        [-0.33136, 0.83627, -0.43687],
        [-0.50544, -0.54832, -0.66624],
    ),
    (
        24.638031,
        [-392.90955, 2.35172, 214.75879],
        [0.86084, 0.0, 0.50888],
        [-0.17962, 0.93564, 0.30384],
        [-0.47612, -0.35297, 0.80543],
    ),
];

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_flyby_reproduces_the_originals_published_camera() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    let grid = grid(&mut archives, 3).expect("03_Track's start_grid.vex");
    for &(seconds, eye, right, up, back) in SAMPLES {
        let pose = grid.pose_at(seconds);
        for k in 0..3 {
            assert!(
                (pose.eye[k] - eye[k]).abs() < 5e-3,
                "eye {k} at {seconds}: {} against {}",
                pose.eye[k],
                eye[k]
            );
            assert!(
                (pose.rows[0][k] - right[k]).abs() < 1e-3,
                "right {k} at {seconds}"
            );
            assert!(
                (pose.rows[1][k] - up[k]).abs() < 1e-3,
                "up {k} at {seconds}"
            );
            assert!(
                (pose.rows[2][k] - back[k]).abs() < 1e-3,
                "back {k} at {seconds}"
            );
        }
    }
}

/// How long each circuit's flyby plays: `AnimEnd` on `grid_camera1`, in seconds.
///
/// Read from the files; the original's exit was measured against it on `16_Track` and
/// `03_Track` alone (`25.0` s on both, to a frame). The two shorter ones, `01_Track` and
/// `14_Track`, have not been watched to the end.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_circuit_authors_a_flyby_of_its_own_length() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    let mut lengths = Vec::new();
    for circuit in [1, 2, 3, 4, 5, 6, 7, 9, 10, 13, 14, 16] {
        let grid = grid(&mut archives, circuit).expect("start_grid.vex");
        lengths.push((circuit, (grid.length() * 60.0).round() as u32));
        assert!(
            !grid.cut_frames().is_empty() || circuit == 2,
            "{circuit} has no cut"
        );
    }
    assert_eq!(
        lengths,
        [
            (1, 1080),
            (2, 1620),
            (3, 1500),
            (4, 1500),
            (5, 1500),
            (6, 1500),
            (7, 1500),
            (9, 1500),
            (10, 1500),
            (13, 1500),
            (14, 1078),
            (16, 1500)
        ]
    );
}

/// **Every flyby on Wipeout HD's disc looks where its own `gridCamera` leaf says, and keeps the
/// sky up.** The leaf stores the point the camera looks at (`+0x10`, as a `Camera`'s), which is
/// the one thing in the file the animation's rotation can be checked against.
///
/// The eight circuits carried over from Pulse author their rotation as whole `f32` quaternions,
/// and read `(x, y, z, w)` they were upside down and up to 130 degrees off the aim; the order is
/// `(w, x, y, z)`. HD's own four circuits author `s16` keys and were right all along, so the sweep
/// holds both forms to the one rule.
#[test]
#[ignore]
fn every_hd_flyby_looks_at_its_aim_point_with_the_sky_up() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut checked = 0;
    for archive in 0..7 {
        let spec = format!("{}:PS3_GAME/USRDIR/DATA{archive:02}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.contains("start_grid") && p.ends_with(".vex"))
            .cloned()
            .collect();
        for path in paths {
            let bytes = open.read_path(&path).expect("reads");
            let Some(grid) = GridCamera::read(&bytes) else {
                continue;
            };
            let nodes = oag_vex::vex::nodes(&bytes).expect("nodes");
            let leaf = nodes
                .iter()
                .find(|n| n.class_id == oag_vex::grid_camera::CLASS_GRID_CAMERA)
                .expect("leaf");
            let at = leaf.payload().start;
            let aim: [f32; 3] = std::array::from_fn(|k| {
                f32::from_be_bytes(bytes[at + 0x10 + k * 4..][..4].try_into().unwrap())
            });
            let pose = grid.pose_at(0.0);
            let to: [f32; 3] = std::array::from_fn(|k| aim[k] - pose.eye[k]);
            let len = to.iter().map(|v| v * v).sum::<f32>().sqrt();
            let facing: f32 = (0..3).map(|k| -pose.rows[2][k] * to[k] / len).sum();
            assert!(
                facing > 0.99,
                "{path}: looks {facing} of the way at its aim"
            );
            let steps = 100;
            for step in 0..steps {
                let t = grid.length() * step as f32 / steps as f32;
                let up = grid.pose_at(t).rows[1][1];
                assert!(up > 0.0, "{path}: upside down at {t} s (up.y {up})");
            }
            checked += 1;
        }
    }
    assert!(checked >= 12 * 4, "only {checked} flybys read");
}
