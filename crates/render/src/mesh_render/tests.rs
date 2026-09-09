//! What `mesh_render`'s own builders are asserted to do.
//!
//! Its own file rather than an inline `#[cfg(test)] mod`, per the 200-line
//! rule in `scripts/check-file-size.py` - and it takes `mesh_render.rs` back
//! under the 1,000-line one at the same time.

use super::*;

/// A model with no geometry must write a frame rather than panic.
///
/// `wgpu::Buffer::slice` panics on a zero-length buffer, which is how
/// `oag-view --collision` died on any `.vex` with no recognised collision
/// class - see `docs/formats/pure-status.md`. Worth knowing if this ever
/// regresses: `create_buffer(size: 0)` and `write_buffer(&[])` both
/// *succeed*, so the death is two frames later at `set_vertex_buffer`, and
/// clamping the buffer to a nonzero size is the fix that looks right and
/// still crashes.
///
/// Deliberately not `#[ignore]`d, unlike `tests/collision_capture.rs`: that
/// one exists to produce a picture, this one guards a regression, and an
/// `#[ignore]`d regression test is a test nobody runs. The adapter probe is
/// the pattern `post::fxaa` and `post::fsr1` already use, so a machine
/// without a GPU skips instead of failing.
#[test]
fn an_empty_model_captures_a_frame_instead_of_panicking() {
    // Probed here rather than left to `capture_from`, which reports a
    // missing adapter as an error - indistinguishable, from the test's
    // side, from the guard not working.
    let instance = wgpu::Instance::default();
    if pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        .is_err()
    {
        eprintln!("no GPU adapter: skipping");
        return;
    }

    // Built the way the bug arrived rather than hand-assembled: a `.vex`
    // with no recognised collision class decodes to zero nodes, and
    // `build_model` over zero nodes is what reached the render pass.
    let model =
        crate::collision::build_model("empty", &[], crate::collision::Style::Wireframe, true);
    assert!(model.vertices.is_empty() && model.indices.is_empty());

    let path = std::env::temp_dir().join("oag-empty-model.png");
    capture_from(&model, &path, 64, 64, 0.9, 0.85, Anisotropy::default(), 0.0)
        .expect("capturing an empty model");

    // Checked through the PNG header rather than the pixels: reaching this
    // line at all is the regression, since the old code panicked inside the
    // render pass and never wrote a file. Byte length carries no signal -
    // `oag_texture::png` emits stored deflate blocks, so every 64x64 frame
    // is the same ~16 KB whatever is in it.
    let bytes = std::fs::read(&path).expect("reading the capture back");
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
    assert_eq!(
        &bytes[16..24],
        &[0, 0, 0, 64, 0, 0, 0, 64],
        "wrong IHDR size"
    );
}
