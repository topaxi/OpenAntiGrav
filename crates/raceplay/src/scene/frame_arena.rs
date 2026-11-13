//! A bump arena for a view's draw list: stable `std::alloc::Allocator` (Rust 1.100).
//!
//! One block, handed out front to back; the offset returns to zero when the last allocation
//! in it is freed, so a frame's lists reuse the same bytes without the scene keeping a list
//! whose element type borrows the frame. Past its capacity it falls through to the global
//! allocator.
#![allow(unsafe_code)] // an `Allocator` impl is `unsafe impl` by definition

use std::alloc::{AllocError, Allocator, Global, Layout};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};

const ALIGN: usize = 16;

/// The block, its size, and `live << 32 | offset` in one word so a reset and a bump never race.
#[derive(Debug)]
pub(super) struct FrameArena {
    base: NonNull<u8>,
    capacity: usize,
    state: AtomicU64,
}

// SAFETY: `base` is owned by the arena and only ever handed out in disjoint ranges claimed by
// a compare-and-swap on `state`.
unsafe impl Send for FrameArena {}
// SAFETY: as above; every shared mutation goes through the atomic `state`.
unsafe impl Sync for FrameArena {}

impl FrameArena {
    pub(super) fn new(capacity: usize) -> Self {
        let capacity = u32::try_from(capacity).map_or(0, |c| c as usize);
        let base = Layout::from_size_align(capacity, ALIGN)
            .ok()
            .filter(|_| capacity > 0)
            .and_then(|layout| Global.allocate(layout).ok())
            .map_or(NonNull::dangling(), NonNull::cast);
        let capacity = if base == NonNull::dangling() {
            0
        } else {
            capacity
        };
        Self {
            base,
            capacity,
            state: AtomicU64::new(0),
        }
    }

    fn owns(&self, ptr: NonNull<u8>) -> bool {
        let at = ptr.as_ptr() as usize;
        let start = self.base.as_ptr() as usize;
        self.capacity > 0 && at >= start && at < start + self.capacity
    }
}

impl Drop for FrameArena {
    fn drop(&mut self) {
        if self.capacity > 0 {
            // SAFETY: allocated in `new` with this layout; every borrower holds `&self`, so
            // none outlives this drop.
            unsafe {
                Global.deallocate(
                    self.base,
                    Layout::from_size_align_unchecked(self.capacity, ALIGN),
                );
            }
        }
    }
}

// SAFETY: a range is handed out once (compare-and-swap) and stays valid until it is freed: the
// offset only returns to zero when no allocation in the block is live. Anything that does not
// fit, or wants more alignment than the block, comes from `Global` and goes back to it.
unsafe impl Allocator for &FrameArena {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if layout.size() == 0 || layout.align() > ALIGN {
            return Global.allocate(layout);
        }
        let mut state = self.state.load(Ordering::Relaxed);
        loop {
            let (live, offset) = (state >> 32, (state & 0xffff_ffff) as usize);
            let start = offset.next_multiple_of(layout.align());
            let end = start + layout.size();
            if end > self.capacity {
                return Global.allocate(layout);
            }
            let next = ((live + 1) << 32) | end as u64;
            match self
                .state
                .compare_exchange_weak(state, next, Ordering::AcqRel, Ordering::Relaxed)
            {
                Ok(_) => {
                    // SAFETY: `start + size <= capacity`, inside the block.
                    let ptr = unsafe { self.base.add(start) };
                    return Ok(NonNull::slice_from_raw_parts(ptr, layout.size()));
                }
                Err(now) => state = now,
            }
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if !self.owns(ptr) {
            // SAFETY: not from the block, so from `Global` with this layout.
            unsafe { Global.deallocate(ptr, layout) };
            return;
        }
        let mut state = self.state.load(Ordering::Relaxed);
        loop {
            let live = (state >> 32) - 1;
            let next = if live == 0 {
                0
            } else {
                (live << 32) | (state & 0xffff_ffff)
            };
            match self
                .state
                .compare_exchange_weak(state, next, Ordering::AcqRel, Ordering::Relaxed)
            {
                Ok(_) => return,
                Err(now) => state = now,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_block_is_reused_once_every_list_is_freed() {
        let arena = FrameArena::new(1024);
        let first = {
            let mut v: Vec<u64, _> = Vec::with_capacity_in(8, &arena);
            v.push(1);
            v.as_ptr()
        };
        let mut a: Vec<u64, _> = Vec::with_capacity_in(8, &arena);
        let b: Vec<u64, _> = Vec::with_capacity_in(8, &arena);
        assert_eq!(a.as_ptr(), first);
        assert_ne!(a.as_ptr(), b.as_ptr());
        a.extend(0..1000);
        assert_eq!(a.iter().sum::<u64>(), 499_500);
    }
}
