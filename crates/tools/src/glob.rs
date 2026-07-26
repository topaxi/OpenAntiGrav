//! A minimal glob matcher for filtering disc listings.
//!
//! Supports `*` (any run of characters, including `/`) and `?` (one
//! character), matched case-insensitively. That is enough for `*.WAD` or
//! `PSP_GAME/*`, and it avoids a dependency for twenty lines of matching.
//!
//! `*` deliberately crosses `/`. On a disc listing, `*.wad` almost always means
//! "every WAD anywhere", and requiring `**/*.wad` for that would be a papercut
//! with no upside.

/// Matches `text` against `pattern`, case-insensitively.
#[must_use]
pub fn matches(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_ascii_lowercase().chars().collect();
    let t: Vec<char> = text.to_ascii_lowercase().chars().collect();
    match_from(&p, &t)
}

/// Iterative backtracking match, so a pathological pattern cannot blow the
/// stack the way a naive recursive version can.
fn match_from(pattern: &[char], text: &[char]) -> bool {
    let (mut p, mut t) = (0usize, 0usize);
    // Where to resume if the current `*` turns out to have consumed too little.
    let mut star: Option<(usize, usize)> = None;

    while t < text.len() {
        match pattern.get(p) {
            Some('*') => {
                star = Some((p, t));
                p += 1;
            }
            Some('?') => {
                p += 1;
                t += 1;
            }
            Some(&c) if c == text[t] => {
                p += 1;
                t += 1;
            }
            _ => match star {
                Some((sp, st)) => {
                    // Let the star swallow one more character and retry.
                    p = sp + 1;
                    t = st + 1;
                    star = Some((sp, st + 1));
                }
                None => return false,
            },
        }
    }

    // Trailing stars may match nothing.
    while pattern.get(p) == Some(&'*') {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_a_literal() {
        assert!(matches("EBOOT.BIN", "EBOOT.BIN"));
        assert!(!matches("EBOOT.BIN", "BOOT.BIN"));
    }

    #[test]
    fn is_case_insensitive() {
        assert!(matches("*.wad", "DATA/WIPEOUT.WAD"));
        assert!(matches("*.WAD", "data/wipeout.wad"));
    }

    #[test]
    fn star_crosses_directory_separators() {
        assert!(matches("*.wad", "A/B/C/D.wad"));
        assert!(matches("PSP_GAME/*", "PSP_GAME/SYSDIR/EBOOT.BIN"));
    }

    #[test]
    fn question_mark_matches_exactly_one() {
        assert!(matches("A?C", "ABC"));
        assert!(!matches("A?C", "AC"));
        assert!(!matches("A?C", "ABBC"));
    }

    #[test]
    fn star_matches_the_empty_string() {
        assert!(matches("*", ""));
        assert!(matches("A*", "A"));
        assert!(matches("*A*", "A"));
    }

    #[test]
    fn handles_multiple_stars() {
        assert!(matches("*GAME*EBOOT*", "PSP_GAME/SYSDIR/EBOOT.BIN"));
        assert!(!matches("*GAME*EBOOT*", "PSP_GAME/SYSDIR/BOOT.BIN"));
    }

    #[test]
    fn backtracks_rather_than_matching_greedily_and_giving_up() {
        // A greedy `*` that never backtracks fails this: it eats the whole
        // string and then cannot find the trailing "ab".
        assert!(matches("*ab", "aaab"));
        assert!(matches("a*b*c", "axxbyyc"));
    }

    #[test]
    fn a_pathological_pattern_terminates() {
        // Would be exponential with naive recursion.
        let text = "a".repeat(64);
        assert!(!matches("*a*a*a*a*a*a*b", &text));
    }
}
