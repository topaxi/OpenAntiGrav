/// The bytes of the NUL-terminated string at `at`, without the NUL.
///
/// `None` when `at` is past the end or no NUL follows it. An empty string is
/// `Some(&[])`.
#[must_use]
pub fn cstr_at(data: &[u8], at: usize) -> Option<&[u8]> {
    let rest = data.get(at..)?;
    let end = rest.iter().position(|&b| b == 0)?;
    Some(&rest[..end])
}

/// Like [`cstr_at`], but a string that runs to the end of `data` without a
/// NUL ends there instead of failing.
#[must_use]
pub fn cstr_at_or_end(data: &[u8], at: usize) -> Option<&[u8]> {
    let rest = data.get(at..)?;
    let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
    Some(&rest[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cstr_at_stops_at_the_nul() {
        assert_eq!(cstr_at(b"ab\0cd\0", 3), Some(&b"cd"[..]));
        assert_eq!(cstr_at(b"ab\0", 2), Some(&b""[..]));
    }

    #[test]
    fn cstr_at_wants_a_nul_and_an_offset_in_range() {
        assert_eq!(cstr_at(b"ab", 0), None);
        assert_eq!(cstr_at(b"ab\0", 4), None);
    }

    #[test]
    fn cstr_at_or_end_accepts_an_unterminated_tail() {
        assert_eq!(cstr_at_or_end(b"ab", 0), Some(&b"ab"[..]));
        assert_eq!(cstr_at_or_end(b"ab", 2), Some(&b""[..]));
        assert_eq!(cstr_at_or_end(b"ab", 3), None);
    }
}
