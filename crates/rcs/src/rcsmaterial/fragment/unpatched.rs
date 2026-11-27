//! The code bytes of a fragment block, less what the engine writes into them.

use super::Program;

impl Program {
    /// The code of the block at `at`, with every 16-byte slot a parameter patches at draw
    /// time zeroed, so a file's default values do not tell two copies of one program apart.
    #[must_use]
    pub fn unpatched_code(data: &[u8], at: usize) -> Option<Vec<u8>> {
        let program = Self::parse(data, at)?;
        let program_at = usize::from(u16::from_be_bytes([
            *data.get(at + 0x16)?,
            *data.get(at + 0x17)?,
        ]));
        let head = at + program_at;
        let len = u32::from_be_bytes(data.get(head..head + 4)?.try_into().ok()?) as usize;
        let offset =
            u32::from_be_bytes(data.get(head + 0x10..head + 0x14)?.try_into().ok()?) as usize;
        let mut code = data.get(head + offset..head + offset + len)?.to_vec();
        for &(slot, _) in &program.parameter_patches {
            let start = usize::from(slot) * 16;
            if let Some(bytes) = code.get_mut(start..start + 16) {
                bytes.fill(0);
            }
        }
        Some(code)
    }
}
