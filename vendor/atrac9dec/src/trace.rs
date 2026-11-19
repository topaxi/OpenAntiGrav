
//! Bit-position tracing for reference comparison against LibAtrac9.
#[inline]
pub fn stage(_name: &str, _bit_pos: usize) {
    #[cfg(feature = "trace")]
    println!("RUST_STAGE|{}|{}", _name, _bit_pos);
}
