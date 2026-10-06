// The scene's velocity and depth attachments, which are different WGSL *types*
// when MSAA is on. `prepare_inputs` imports these two names and is otherwise
// the same pass in both builds: `textureLoad(t, p, 0)` is spelled the same
// either way, the `0` being a mip level on the single-sampled texture and a
// *sample index* on the multisampled one. Sample 0 rather than a resolve, for
// the reason `post::motion_blur` gives.
//
// `build.rs` compiles `prepare_inputs` twice, with the `msaa` feature off and
// on.
@if(msaa) @group(1) @binding(0) var r_input_motion_vectors: texture_multisampled_2d<f32>;
@else @group(1) @binding(0) var r_input_motion_vectors: texture_2d<f32>;
@if(msaa) @group(1) @binding(1) var r_input_depth: texture_multisampled_2d<f32>;
@else @group(1) @binding(1) var r_input_depth: texture_2d<f32>;
