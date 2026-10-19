//! A bind group that is rebuilt only when the texture view it reads changes.
//!
//! A post pass that samples the frame's scene target used to create its group on
//! every call. The scene target is the same texture frame after frame, so the group
//! is kept and the view it was built for is kept beside it: the next call compares
//! the view it is handed (`wgpu::TextureView` compares by identity) and builds again
//! only when it differs. Holding the view also keeps its texture alive, so an
//! identity can never be reused by a different texture while the group is cached.
//!
//! **What the key does not cover is fixed by the owner**: the layout, sampler and
//! buffers the group also binds must live as long as the cache does. A pass that
//! rebuilds those on a resize holds the cache inside the object it rebuilds, so the
//! cache goes with them.

use std::cell::{Cell, RefCell};

/// One cached group keyed on one texture view.
#[derive(Debug, Default)]
pub(crate) struct ViewGroup {
    slot: RefCell<Option<(wgpu::TextureView, wgpu::BindGroup)>>,
    builds: Cell<u64>,
}

impl ViewGroup {
    /// The group for `view`, from `build` only when `view` is not the one the held
    /// group was built for.
    pub(crate) fn get(
        &self,
        view: &wgpu::TextureView,
        build: impl FnOnce() -> wgpu::BindGroup,
    ) -> wgpu::BindGroup {
        let mut slot = self.slot.borrow_mut();
        if let Some((held, group)) = slot.as_ref()
            && held == view
        {
            return group.clone();
        }
        let group = build();
        self.builds.set(self.builds.get() + 1);
        *slot = Some((view.clone(), group.clone()));
        group
    }

    /// How many times `build` has run, for the tests that prove a group is kept.
    #[cfg(test)]
    pub(crate) fn builds(&self) -> u64 {
        self.builds.get()
    }
}

#[cfg(test)]
mod tests;
