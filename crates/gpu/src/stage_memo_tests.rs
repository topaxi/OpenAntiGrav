//! The vendored `wgpu-hal` keeps the SPIR-V of a shader stage per device and
//! hands it to the next pipeline built with the same stage
//! (`docs/architecture/vendored-wgpu-hal.md`). These tests prove that two
//! pipelines whose SPIR-V must differ never share it: the pipeline layout
//! renumbers bindings, override constants are baked in, and a destroyed
//! module's entries must not answer for a new one.
//!
//! Only a module with an `override` reaches the memo (without one, `wgpu-hal`
//! writes SPIR-V when the module is created), so every shader here has one.
//! The memo is Vulkan's; the tests run on every adapter and assert the same
//! results everywhere.

const SHADER: &str = "
override VALUE: u32 = 1u;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(1) fn main() { out[0] = VALUE + OFFSET; }
";

fn shader(offset: u32) -> String {
    SHADER.replace("OFFSET", &format!("{offset}u"))
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn storage_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// A layout with one storage buffer at each of `bindings`, in group 0. With
/// `[1]` the shader's binding 1 is Vulkan binding 0; with `[0, 1]` it is 1.
fn layout(gpu: &Gpu, bindings: &[u32]) -> (wgpu::BindGroupLayout, wgpu::PipelineLayout) {
    let entries: Vec<_> = bindings.iter().map(|&b| storage_entry(b)).collect();
    let group = gpu
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("stage memo test"),
            entries: &entries,
        });
    let pipeline = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("stage memo test"),
            bind_group_layouts: &[Some(&group)],
            immediate_size: 0,
        });
    (group, pipeline)
}

fn pipeline(
    gpu: &Gpu,
    module: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    value: f64,
) -> wgpu::ComputePipeline {
    gpu.device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("stage memo test"),
            layout: Some(layout),
            module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[("VALUE", value)],
                ..Default::default()
            },
            cache: None,
        })
}

/// Runs `pipeline` once with a fresh zeroed buffer at each of `bindings` and
/// returns the first word of each.
fn run(
    gpu: &Gpu,
    pipeline: &wgpu::ComputePipeline,
    group_layout: &wgpu::BindGroupLayout,
    bindings: &[u32],
) -> Vec<u32> {
    use wgpu::util::DeviceExt as _;
    let buffers: Vec<wgpu::Buffer> = bindings
        .iter()
        .map(|_| {
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("stage memo test"),
                    contents: &[0; 4],
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                })
        })
        .collect();
    let entries: Vec<_> = bindings
        .iter()
        .zip(&buffers)
        .map(|(&binding, buffer)| wgpu::BindGroupEntry {
            binding,
            resource: buffer.as_entire_binding(),
        })
        .collect();
    let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("stage memo test"),
        layout: group_layout,
        entries: &entries,
    });
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("stage memo test"),
        size: 4 * buffers.len() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    for (i, buffer) in buffers.iter().enumerate() {
        encoder.copy_buffer_to_buffer(buffer, 0, &readback, 4 * i as u64, 4);
    }
    gpu.queue.submit([encoder.finish()]);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("mapping the readback"));
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("waiting for the readback");
    let words = readback
        .slice(..)
        .get_mapped_range()
        .expect("reading the readback");
    words
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
        .collect()
}

/// One device per adapter, or none (the tests then skip, as the other GPU
/// tests in this crate do).
fn devices() -> Vec<(String, Gpu)> {
    let instance = wgpu::Instance::default();
    crate::test_adapter::ordered(&instance)
        .into_iter()
        .filter_map(|adapter| {
            let info = adapter.get_info();
            let (device, queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    label: Some("stage memo test"),
                    ..Default::default()
                }))
                .ok()?;
            device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
            Some((
                format!("{} ({:?})", info.name, info.backend),
                Gpu { device, queue },
            ))
        })
        .collect()
}

fn module(gpu: &Gpu, offset: u32) -> wgpu::ShaderModule {
    gpu.device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("stage memo test"),
            source: wgpu::ShaderSource::Wgsl(shader(offset).into()),
        })
}

/// Same module, same entry point and constants, two layouts that put the
/// shader's binding 1 at different Vulkan bindings: each pipeline writes the
/// buffer its own layout names. A memo keyed without the binding map would
/// hand the second pipeline the first one's SPIR-V, which writes the decoy at
/// binding 0.
#[test]
fn two_layouts_of_one_module_write_through_their_own_bindings() {
    for (name, gpu) in devices() {
        let module = module(&gpu, 0);
        let (group_a, layout_a) = layout(&gpu, &[1]);
        let (group_b, layout_b) = layout(&gpu, &[0, 1]);
        let a = pipeline(&gpu, &module, &layout_a, 7.0);
        let b = pipeline(&gpu, &module, &layout_b, 7.0);
        assert_eq!(run(&gpu, &a, &group_a, &[1]), [7], "{name}: layout A");
        assert_eq!(
            run(&gpu, &b, &group_b, &[0, 1]),
            [0, 7],
            "{name}: layout B reused layout A's SPIR-V"
        );
    }
}

/// Same module and layout, two override values: each pipeline writes its own.
#[test]
fn two_override_values_of_one_stage_stay_apart() {
    for (name, gpu) in devices() {
        let module = module(&gpu, 0);
        let (group, layout) = layout(&gpu, &[1]);
        let seven = pipeline(&gpu, &module, &layout, 7.0);
        let nine = pipeline(&gpu, &module, &layout, 9.0);
        let seven_again = pipeline(&gpu, &module, &layout, 7.0);
        assert_eq!(run(&gpu, &seven, &group, &[1]), [7], "{name}");
        assert_eq!(run(&gpu, &nine, &group, &[1]), [9], "{name}");
        assert_eq!(run(&gpu, &seven_again, &group, &[1]), [7], "{name}");
    }
}

/// A module created after another was dropped never gets the dropped one's
/// SPIR-V, even with the same entry point, constants and layout.
#[test]
fn a_new_module_does_not_inherit_a_dropped_ones_stages() {
    for (name, gpu) in devices() {
        let (group, layout) = layout(&gpu, &[1]);
        {
            let first = module(&gpu, 0);
            let p = pipeline(&gpu, &first, &layout, 7.0);
            assert_eq!(run(&gpu, &p, &group, &[1]), [7], "{name}: first module");
        }
        let second = module(&gpu, 100);
        let p = pipeline(&gpu, &second, &layout, 7.0);
        assert_eq!(run(&gpu, &p, &group, &[1]), [107], "{name}: second module");
    }
}
