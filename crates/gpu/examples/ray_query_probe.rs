//! Reads every adapter's ray-tracing features and limits and requests nothing.
//!
//! ```sh
//! cargo run -q -p oag-gpu --example ray_query_probe
//! ```
//!
//! The question is in `docs/rendering/ray-tracing.md`: which adapters could run
//! a hardware ray query under the workspace's wgpu. No device is created, so
//! the `unsafe` experimental-features token is never taken here.

fn main() {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    if adapters.is_empty() {
        println!("no adapters");
    }
    for adapter in adapters {
        let info = adapter.get_info();
        let features = adapter.features();
        let limits = adapter.limits();
        println!(
            "{} | {:?} | {:?} | driver {} {}",
            info.name, info.backend, info.device_type, info.driver, info.driver_info
        );
        for (name, flag) in [
            (
                "EXPERIMENTAL_RAY_QUERY",
                wgpu::Features::EXPERIMENTAL_RAY_QUERY,
            ),
            (
                "EXPERIMENTAL_RAY_HIT_VERTEX_RETURN",
                wgpu::Features::EXPERIMENTAL_RAY_HIT_VERTEX_RETURN,
            ),
            (
                "EXPERIMENTAL_RAY_TRACING_PIPELINES",
                wgpu::Features::EXPERIMENTAL_RAY_TRACING_PIPELINES,
            ),
            (
                "EXTENDED_ACCELERATION_STRUCTURE_VERTEX_FORMATS",
                wgpu::Features::EXTENDED_ACCELERATION_STRUCTURE_VERTEX_FORMATS,
            ),
            (
                "ACCELERATION_STRUCTURE_BINDING_ARRAY",
                wgpu::Features::ACCELERATION_STRUCTURE_BINDING_ARRAY,
            ),
        ] {
            println!("  {name}: {}", features.contains(flag));
        }
        println!(
            "  max_blas_primitive_count {} | max_blas_geometry_count {} | max_tlas_instance_count {} | max_acceleration_structures_per_shader_stage {}",
            limits.max_blas_primitive_count,
            limits.max_blas_geometry_count,
            limits.max_tlas_instance_count,
            limits.max_acceleration_structures_per_shader_stage
        );
    }
}
