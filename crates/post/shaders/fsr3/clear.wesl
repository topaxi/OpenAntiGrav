// Clearing the reconstructed-previous-depth buffer, which is the one FSR 3.1
// resource this port holds as a buffer rather than a texture.
//
// **Not a transliteration - upstream has no such shader**, because it clears an
// `R32_UINT` UAV through a backend clear job. `wgpu::CommandEncoder::clear_buffer`
// only clears to zero, and zero is the *nearest* possible depth: leave it there
// and `atomicMin` would keep it, so every texel would read back as touching the
// near plane. The value written is upstream's own clear value for a
// non-inverted projection - `1.0`, the far plane - as its bit pattern, because
// that is what the scatter compares against.
//
// **Dispatched in two dimensions even though a buffer has one**, and that is
// not cosmetic. A flat dispatch over `width * height` threads needs
// `texels / 64` workgroups, and wgpu caps a dispatch at **65535 per
// dimension** - which a 2880x1800 render extent passes at 81000, panicking in
// the frame loop. Walking the grid the buffer *represents* keeps each dimension
// at `extent / 8`, which no texture this port can allocate comes close to
// exceeding.
//
// `render_size` bounds it rather than the buffer's length, so a frame drawn
// into part of a larger allocation clears exactly the part it will use - and
// the index below is `prepare_inputs`'s own, which is what makes "the part it
// will use" the same set of words in both.
import package::fsr3::common::{
    is_on_screen,
    render_size,
};

@group(1) @binding(0) var<storage, read_write> rw_reconstructed_previous_nearest_depth: array<u32>;

@compute @workgroup_size(8, 8, 1)
fn cs_clear_reconstructed_depth(@builtin(global_invocation_id) id: vec3<u32>) {
    let px_pos = vec2<i32>(id.xy);
    if !is_on_screen(px_pos, render_size()) {
        return;
    }
    let index = u32(px_pos.y) * u32(render_size().x) + u32(px_pos.x);
    rw_reconstructed_previous_nearest_depth[index] = bitcast<u32>(1.0);
}
