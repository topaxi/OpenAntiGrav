use super::*;

/// Every entry hashes to its own key, and nothing else - a typo'd string here
/// would silently name the wrong hash rather than fail to compile.
#[test]
fn every_known_sampler_name_looks_itself_up() {
    for name in KNOWN_SAMPLER_NAMES {
        assert_eq!(
            sampler_name(name_hash(name)),
            Some(*name),
            "{name} did not round-trip through sampler_name"
        );
    }
}

#[test]
fn every_known_parameter_name_looks_itself_up() {
    for name in KNOWN_PARAMETER_NAMES {
        assert_eq!(
            parameter_name(name_hash(name)),
            Some(*name),
            "{name} did not round-trip through parameter_name"
        );
    }
}

/// No two names in the same table collide on the same hash - if they did,
/// [`sampler_name`]/[`parameter_name`] would silently pick the first one and
/// hide an ambiguity that a preimage sweep must report instead.
#[test]
fn no_two_known_sampler_names_share_a_hash() {
    let mut seen = std::collections::HashMap::new();
    for name in KNOWN_SAMPLER_NAMES {
        let hash = name_hash(name);
        if let Some(prior) = seen.insert(hash, *name) {
            panic!("{prior} and {name} both hash to {hash:#010x}");
        }
    }
}

#[test]
fn no_two_known_parameter_names_share_a_hash() {
    let mut seen = std::collections::HashMap::new();
    for name in KNOWN_PARAMETER_NAMES {
        let hash = name_hash(name);
        if let Some(prior) = seen.insert(hash, *name) {
            panic!("{prior} and {name} both hash to {hash:#010x}");
        }
    }
}

/// An unrecognised hash reports absence rather than a false positive.
#[test]
fn an_unknown_hash_names_nothing() {
    assert_eq!(sampler_name(0xdead_beef), None);
    assert_eq!(parameter_name(0xdead_beef), None);
}
