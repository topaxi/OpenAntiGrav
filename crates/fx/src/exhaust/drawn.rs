/// What the last upload left to draw, as a value: a split screen records its views at once, so
/// each view keeps its own while a later view's upload refills the pipeline. `draw` draws it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Drawn {
    pub(super) count: u32,
    pub(super) trail_count: u32,
}

impl Drawn {
    /// The ribbon alone. The original queues every `Trail` at the constant key `0x4d000000`,
    /// ahead of every list-B mesh, so the ribbon draws earlier than the flare. See `exhaust.md`,
    /// "The ribbon is queued first".
    #[must_use]
    pub fn trail_only(self) -> Self {
        Self { count: 0, ..self }
    }

    /// The flare quads alone, for the draw that follows [`Self::trail_only`]'s.
    #[must_use]
    pub fn without_trail(self) -> Self {
        Self {
            trail_count: 0,
            ..self
        }
    }
}
