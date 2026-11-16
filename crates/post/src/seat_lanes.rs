//! What a split screen keeps per seat, built when a seat is first drawn.

use std::cell::{Cell, Ref, RefCell};

/// The seats' lanes, built when a split first asks for them, and how many builds that took
/// (a lane forgotten and asked for again is built again, which a live count cannot show).
#[derive(Debug)]
pub(crate) struct SeatLanes<L>(RefCell<Vec<Option<L>>>, Cell<u64>);

impl<L> Default for SeatLanes<L> {
    fn default() -> Self {
        Self(RefCell::new(Vec::new()), Cell::new(0))
    }
}

impl<L> SeatLanes<L> {
    /// How many seats hold a lane right now.
    pub(crate) fn built(&self) -> usize {
        self.0.borrow().iter().flatten().count()
    }

    /// How many lanes have been built so far.
    pub(crate) fn builds(&self) -> u64 {
        self.1.get()
    }

    pub(crate) fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    pub(crate) fn retire(&self, seat: usize) {
        if let Some(lane) = self.0.borrow_mut().get_mut(seat) {
            lane.take();
        }
    }

    pub(crate) fn get_or_build(&self, seat: usize, build: impl FnOnce() -> L) -> Ref<'_, L> {
        {
            let mut lanes = self.0.borrow_mut();
            if lanes.len() <= seat {
                lanes.resize_with(seat + 1, || None);
            }
            if lanes[seat].is_none() {
                lanes[seat] = Some(build());
                self.1.set(self.1.get() + 1);
            }
        }
        Ref::map(self.0.borrow(), |lanes| {
            lanes[seat].as_ref().expect("built above")
        })
    }
}
