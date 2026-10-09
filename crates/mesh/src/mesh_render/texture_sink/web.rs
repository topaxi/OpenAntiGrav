//! The web build's texture sink: none. A race load runs on a Web Worker there,
//! and wgpu's web types may not leave the page's thread, so every texture
//! stays on the CPU until the scene uploads it (docs/tools/web.md, "Threads").
//! The same API as the native module, so no caller branches.

use crate::mesh::ModelTexture;

/// What a scope has uploaded so far: always nothing here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Textures uploaded.
    pub textures: usize,
    /// Of those, kept as the disc's own blocks.
    pub block_compressed: usize,
    /// Bytes the uploads occupy on the GPU, mip chains included.
    pub gpu_bytes: u64,
    /// Bytes of decoded texels that were never held past their upload.
    pub cpu_bytes_streamed: u64,
}

/// A scope that uploads nothing.
#[derive(Debug)]
pub struct Scope {
    _private: (),
}

impl Scope {
    /// Opens nothing: textures stay on the CPU.
    #[must_use]
    pub fn open(_device: &wgpu::Device, _queue: &wgpu::Queue) -> Self {
        Self { _private: () }
    }

    /// Always empty.
    #[must_use]
    pub fn stats(&self) -> Stats {
        Stats::default()
    }
}

/// Runs `f`: there is no scope to set aside.
pub fn without_scope<T>(f: impl FnOnce() -> T) -> T {
    f()
}

/// Hands `texture` back unchanged.
pub(crate) fn offer(texture: ModelTexture) -> ModelTexture {
    texture
}
