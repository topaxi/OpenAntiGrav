use super::*;

#[test]
fn truncation_reports_the_offset_of_the_read() {
    let mut r = Reader::le(&[1, 2, 3, 4, 5, 6]);
    r.skip(4).unwrap();
    assert_eq!(
        r.u32(),
        Err(Truncated {
            at: 4,
            wanted: 4,
            available: 2
        })
    );
    assert_eq!(r.position(), 4, "a failed read leaves the position alone");
}

#[test]
fn each_order_reads_its_own_way() {
    let bytes = [0x12, 0x34, 0x56, 0x78];
    assert_eq!(Reader::be(&bytes).u32(), Ok(0x1234_5678));
    assert_eq!(Reader::le(&bytes).u32(), Ok(0x7856_3412));
    assert_eq!(Reader::be(&bytes).i16(), Ok(0x1234));
}

#[test]
fn the_order_can_change_mid_stream() {
    let mut r = Reader::be(&[0, 1, 1, 0]);
    assert_eq!(r.u16(), Ok(1));
    let mut r = r.into_endian::<Le>();
    assert_eq!(r.position(), 2);
    assert_eq!(r.u16(), Ok(1));
}

#[test]
fn slices_outlive_the_reader() {
    let data = [1u8, 2, 3];
    let head = {
        let mut r = Reader::le(&data);
        r.take(2).unwrap()
    };
    assert_eq!(head, [1, 2]);
}

#[test]
fn seek_and_rest() {
    let mut r = Reader::le(&[1, 2, 3, 4]);
    r.seek(3).unwrap();
    assert_eq!(r.rest(), [4]);
    assert!(r.seek(5).is_err());
    assert_eq!(r.position(), 3);
    r.seek(4).unwrap();
    assert!(r.is_empty());
}

#[test]
fn align_is_relative_to_the_start() {
    let mut r = Reader::le(&[0; 8]);
    r.skip(1).unwrap();
    r.align(4).unwrap();
    assert_eq!(r.position(), 4);
    r.align(4).unwrap();
    assert_eq!(r.position(), 4);
    assert!(Reader::le(&[0; 1]).align(4).is_ok());
    let mut short = Reader::le(&[0; 2]);
    short.skip(1).unwrap();
    assert!(short.align(4).is_err());
}

#[test]
fn cstr_consumes_the_nul() {
    let mut r = Reader::le(b"ab\0cd");
    assert_eq!(r.cstr(), Ok(&b"ab"[..]));
    assert_eq!(r.position(), 3);
    assert!(r.cstr().is_err());
    assert_eq!(r.position(), 3);
}

#[test]
fn varint_round_trips_and_refuses_overlong() {
    let mut r = Reader::le(&[0xac, 0x02, 0x7f]);
    assert_eq!(r.varint(), Ok(300));
    assert_eq!(r.varint(), Ok(127));

    let mut max = vec![0xff; 9];
    max.push(0x01);
    assert_eq!(Reader::le(&max).varint(), Ok(u64::MAX));

    let mut over = vec![0xff; 9];
    over.push(0x02);
    let mut r = Reader::le(&over);
    assert!(r.varint().is_err());
    assert_eq!(r.position(), 0);

    let mut r = Reader::le(&[0x80, 0x80]);
    assert!(r.varint().is_err());
    assert_eq!(r.position(), 0);
}
