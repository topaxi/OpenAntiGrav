//! Resolving a front-end `Image`'s `src` when it is a `hash:`-prefixed spec
//! rather than a name.
//!
//! Its own file rather than a few lines inside `boot.rs`: that file is
//! baselined by `scripts/check-file-size.py` and may shrink but not grow, and
//! this arrived as a *net addition*. See `crates/game/src/boot/provenance.rs`
//! for the same trade made once already.
//!
//! Reuses [`super::movies::EntryRef`]'s own `hash:` spelling rather than
//! inventing a second one for images - a `src`-less widget backed by a
//! content-scan find is the same shape of gap a boot movie with no recovered
//! name already has.

use super::movies::EntryRef;

/// The hash `src` addresses, if it uses the `hash:` spelling.
///
/// `None` for an ordinary name, which [`super::sprites::read_front_end_first`]'s
/// existing FE-then-Data search handles unchanged - this is consulted first
/// and only short-circuits that search when it matches.
pub(super) fn hash_spec(src: &str) -> Option<u32> {
    match EntryRef::parse(src) {
        EntryRef::Hash(hash) => Some(hash),
        EntryRef::Name(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hash_prefixed_spec_parses_to_its_hash() {
        assert_eq!(hash_spec("hash:3af18d90"), Some(0x3af1_8d90));
    }

    #[test]
    fn an_ordinary_name_is_not_a_hash_spec() {
        assert_eq!(hash_spec(r"Data\FE\Images\Real.mip"), None);
    }
}
