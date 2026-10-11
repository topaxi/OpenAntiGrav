//! Regenerates the committed fixture: `oval.track.toml` and `oval.glb` under
//! `crates/trackfile/tests/fixtures/`.
//!
//! `cargo run -p oag-trackfile --example make_fixture`

fn main() -> Result<(), oag_trackfile::Error> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    oag_trackfile::write(
        &oag_trackfile::fixture::oval(),
        &dir.join("oval.track.toml"),
    )
}
