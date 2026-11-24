//! Bit-position tracing for reference comparison against LibAtrac9.
#[inline]
pub fn stage(_name: &str, _bit_pos: usize) {
    #[cfg(feature = "trace")]
    println!("RUST_STAGE|{}|{}", _name, _bit_pos);
}

/// [`stage`] for a per-channel name, `<prefix><channel>`. The name is built only when the
/// `trace` feature prints it, so a decode that does not trace formats nothing.
#[inline]
pub fn channel_stage(_prefix: &str, _channel: usize, _bit_pos: usize) {
    #[cfg(feature = "trace")]
    println!("RUST_STAGE|{}{}|{}", _prefix, _channel, _bit_pos);
}
