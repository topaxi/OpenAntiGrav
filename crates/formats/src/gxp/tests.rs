//! Synthetic programs only. The real corpus is
//! `crates/formats/tests/gxp_ground_truth.rs`, which needs game content.

use super::*;

/// Where each region of [`program`]'s output starts.
const CODE: usize = 0xA0;
const LITERALS: usize = 0xB0;
const IMAGE: usize = 0xB8;
const CONTAINERS: usize = 0xC0;
const PARAMS: usize = 0xC8;

/// A minimal program that closes: two instructions, one literal, two 4-byte
/// uniform-image words, one container and two named parameters.
fn program(fragment: bool) -> Vec<u8> {
    let mut b = vec![0u8; PARAMS + 2 * PARAMETER_LEN];
    b[..4].copy_from_slice(b"GXP\0");
    b[4] = 1;
    b[5] = 4;
    put16(&mut b, 6, 0x0165);
    put(&mut b, 0x14, u32::from(fragment));
    put(&mut b, 0x24, 2);
    put(&mut b, 0x28, (PARAMS - 0x28) as u32);
    put(&mut b, 0x3C, 2);
    put(&mut b, 0x40, (CODE - 0x40) as u32);
    put(&mut b, 0x48, (CODE - 0x48) as u32);
    put(&mut b, 0x4C, (CODE - 0x4C) as u32);
    put(&mut b, 0x58, 2);
    put(&mut b, 0x70, 1);
    put(&mut b, 0x74, (LITERALS - 0x74) as u32);
    put(&mut b, 0x7C, (CONTAINERS - 0x7C) as u32);
    put(&mut b, 0x84, (CONTAINERS - 0x84) as u32);
    put(&mut b, 0x8C, (CONTAINERS - 0x8C) as u32);
    put(&mut b, 0x90, 1);
    put(&mut b, 0x94, (CONTAINERS - 0x94) as u32);
    // An attribute at unit 0 and a sampler at unit 3, named after the table.
    parameter(&mut b, PARAMS, 0x0400, 1, 0);
    parameter(&mut b, PARAMS + PARAMETER_LEN, 0x0102, 1, 3);
    let names = b.len();
    b.extend_from_slice(b"position\0shadowMap\0");
    put(&mut b, PARAMS, (names - PARAMS) as u32);
    put(
        &mut b,
        PARAMS + PARAMETER_LEN,
        (names + 9 - PARAMS - PARAMETER_LEN) as u32,
    );
    let size = b.len() as u32;
    put(&mut b, 8, size);
    debug_assert_eq!(IMAGE, LITERALS + 8);
    b
}

fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn put16(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn parameter(b: &mut [u8], at: usize, word: u16, array: u32, resource: u32) {
    put16(b, at + 4, word);
    put(b, at + 8, array);
    put(b, at + 12, resource);
}

#[test]
fn a_closing_program_decodes_with_its_parameters() {
    let p = Program::parse(&program(true)).expect("the synthetic program closes");
    assert!(p.is_fragment());
    assert_eq!(p.version, (1, 4));
    assert_eq!(p.primary_instructions, 2);
    assert_eq!(
        p.primary,
        Region {
            start: CODE,
            end: LITERALS,
            what: "primary code"
        }
    );
    assert_eq!(p.literal_count, 1);
    assert_eq!(p.container_count, 1);
    assert_eq!(p.parameters.len(), 2);
    assert_eq!(p.parameters[0].name, "position");
    assert_eq!(p.parameters[0].category, Category::Attribute);
    assert_eq!(p.parameters[0].components, 4);
    let shadow = p.parameter("shadowMap").expect("the sampler is bound");
    assert_eq!(shadow.category, Category::Sampler);
    assert_eq!(shadow.components, 1);
    assert_eq!(shadow.resource, 3);
}

/// Bit 0 of `+0x14` is the whole of the type test, so both values are checked.
#[test]
fn bit_zero_of_the_flags_word_is_the_only_thing_that_says_fragment() {
    assert!(!Program::parse(&program(false)).unwrap().is_fragment());
    assert!(Program::parse(&program(true)).unwrap().is_fragment());
}

/// The check that separates a real reading from a plausible one: the last
/// name's NUL is the program's last byte, and one byte either way is refused.
#[test]
fn a_declared_size_that_does_not_land_on_the_last_name_is_refused() {
    let mut short = program(true);
    let size = u32::from_le_bytes(short[8..12].try_into().unwrap());
    put(&mut short, 8, size - 1);
    short.truncate(size as usize - 1);
    // One byte short takes the last name's NUL with it, so this is refused for
    // being unterminated rather than for not closing. Either way it is refused.
    assert!(matches!(
        Program::parse(&short),
        Err(Error::BadParameterName { .. })
    ));

    let mut long = program(true);
    let size = u32::from_le_bytes(long[8..12].try_into().unwrap());
    long.push(0);
    put(&mut long, 8, size + 1);
    assert!(matches!(
        Program::parse(&long),
        Err(Error::DoesNotClose { .. })
    ));
}

/// A one-instruction error in the primary count desynchronises every table
/// after it. This is why the count and the width are not separately assumed.
#[test]
fn a_wrong_instruction_count_breaks_the_table_chain() {
    let mut b = program(true);
    put(&mut b, 0x3C, 3);
    assert!(matches!(
        Program::parse(&b),
        Err(Error::DoesNotClose { .. })
    ));
}

/// A zero name offset means *unnamed*, not a name at byte zero. Reading it as
/// an offset points the name back at the parameter's own entry.
#[test]
fn a_zero_name_offset_is_an_unnamed_parameter() {
    let mut b = program(true);
    // Drop the second name and give its parameter no name at all.
    let size = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
    b.truncate(size - 10);
    put(&mut b, 8, (size - 10) as u32);
    put(&mut b, PARAMS + PARAMETER_LEN, 0);
    let p = Program::parse(&b).expect("an unnamed parameter still closes");
    assert_eq!(p.parameters[1].name, "");
    assert_eq!(p.parameters[0].name, "position");
}

/// A chance `GXP\0` in unrelated bytes is not a program that failed to decode.
#[test]
fn a_stray_magic_is_reported_as_not_a_container() {
    let mut junk = vec![0xAA; 0x200];
    junk[0x40..0x44].copy_from_slice(b"GXP\0");
    put(&mut junk, 0x48, 0x0044_1122);
    let found = programs(&junk);
    assert_eq!(found.len(), 1);
    assert!(matches!(found[0].1, Err(Error::NotAContainer { .. })));
}

/// Two programs back to back, the way a `.rcsmaterial` stores them.
#[test]
fn programs_walks_a_run_of_containers() {
    let mut blob = program(false);
    blob.extend_from_slice(&program(true));
    let found = programs(&blob);
    assert_eq!(found.len(), 2);
    assert!(!found[0].1.as_ref().unwrap().is_fragment());
    assert!(found[1].1.as_ref().unwrap().is_fragment());
}
