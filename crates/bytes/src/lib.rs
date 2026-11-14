mod reader;
mod slice;

pub use reader::{Be, ByteOrder, Endian, Le, Reader, Truncated, VarintError};
pub use slice::{cstr_at, cstr_at_or_end};
