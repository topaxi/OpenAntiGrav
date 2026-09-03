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
// One dimension, because a buffer has one. `render_size` bounds it rather than
// the buffer's length, so a frame drawn into part of a larger allocation clears
// exactly the part it will use.

@group(1) @binding(0) var<storage, read_write> rw_reconstructed_previous_nearest_depth: array<u32>;

@compute @workgroup_size(64, 1, 1)
fn cs_clear_reconstructed_depth(@builtin(global_invocation_id) id: vec3<u32>) {
    let count = u32(render_size().x) * u32(render_size().y);
    if id.x >= count {
        return;
    }
    rw_reconstructed_previous_nearest_depth[id.x] = bitcast<u32>(1.0);
}
