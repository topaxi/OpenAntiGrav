mod reader;
mod slice;

pub use reader::{Be, ByteOrder, Endian, Le, Reader, Truncated, VarintError};
pub use slice::{be, cstr_at, cstr_at_or_end, f32_at, le, u16_at, u32_at, u64_at};
