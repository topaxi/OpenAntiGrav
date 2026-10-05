use super::*;

/// `None` when there is no adapter, the guard the rest of this crate's GPU
/// tests use.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
}

fn rgba(width: u32, height: u32) -> ModelTexture {
    ModelTexture::rgba8(
        "sink test".into(),
        width,
        height,
        vec![200; (width * height * 4) as usize],
        None,
    )
}

#[test]
fn with_no_scope_a_texture_keeps_its_texels() {
    let texture = offer(rgba(4, 4));
    assert!(matches!(texture.texels, Texels::Rgba8(_)));
    assert_eq!(texture.cpu_bytes(), 64);
}

#[test]
fn under_a_scope_a_texture_goes_up_and_keeps_no_texels() {
    let Some((device, queue)) = device() else {
        return;
    };
    let scope = Scope::open(&device, &queue);
    let texture = offer(rgba(4, 4));
    // 4x4, 2x2 and 1x1 at four bytes a texel: the box-filtered chain.
    assert!(
        matches!(&texture.texels, Texels::Uploaded { gpu_bytes: 84, .. }),
        "{:?}",
        texture.texels
    );
    assert_eq!(texture.cpu_bytes(), 0);
    assert_eq!((texture.width, texture.height), (4, 4));
    assert_eq!(
        scope.stats(),
        Stats {
            textures: 1,
            block_compressed: 0,
            gpu_bytes: 84,
            cpu_bytes_streamed: 64,
        }
    );
    // Offered again it passes straight through: no second upload.
    let again = offer(texture);
    assert_eq!(scope.stats().textures, 1);
    assert!(matches!(again.texels, Texels::Uploaded { .. }));
    // The uploader hands back the view it carries rather than uploading.
    let _ = texture::upload(&device, &queue, &again, false);
    let before = scope.stats();
    let _ = texture::upload(&device, &queue, &again, false);
    assert_eq!(scope.stats(), before);
    drop(scope);
    assert!(matches!(offer(rgba(2, 2)).texels, Texels::Rgba8(_)));
}
