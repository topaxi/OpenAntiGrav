//! The XML's own `0xAARRGGBB` colour spelling, and the RGBA floats the
//! renderer wants instead - split out of `screen.rs` under the 1,000-line
//! rule (`scripts/check-file-size.py`), the same shape `reveal.rs`/`fade.rs`
//! already split their own seams into. A move, not a behaviour change.

/// Parses `0xAARRGGBB`, as every colour in the XML is written.
#[must_use]
pub fn parse_argb(value: &str) -> Option<u32> {
    let text = value.trim();
    let hex = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    u32::from_str_radix(hex, 16).ok()
}

/// Splits ARGB into linear-ish RGBA floats for the renderer.
#[must_use]
pub fn argb_to_rgba(argb: u32) -> [f32; 4] {
    let a = ((argb >> 24) & 0xff) as f32 / 255.0;
    let r = ((argb >> 16) & 0xff) as f32 / 255.0;
    let g = ((argb >> 8) & 0xff) as f32 / 255.0;
    let b = (argb & 0xff) as f32 / 255.0;
    [r, g, b, a]
}
