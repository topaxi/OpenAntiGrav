use super::*;

/// The test input: `(i * 7 + 3) mod 256`.
fn input(n: usize) -> Vec<u8> {
    (0..n).map(|i| ((i * 7 + 3) & 0xff) as u8).collect()
}

/// `(length, seed, xxh32, xxh64)` from the reference implementation (`xxhash`
/// for Python, libxxhash 0.8) over [`input`]. Lengths straddle every branch:
/// empty, the 1/4/8-byte tails, the 16-byte and 32-byte bulk loops.
const REFERENCE: &[(usize, u32, u32, u64)] = &[
    (0, 0x0, 0x2cc5d05, 0xef46db3751d8e999),
    (0, 0xbacd7814, 0xd81d1328, 0xd0d2cf1c5037efff),
    (0, 0xc0108888, 0x8c64d961, 0x48138b9840a4c6b6),
    (1, 0x0, 0x21ae663a, 0x1f25c8d0bc1f4bb6),
    (1, 0xbacd7814, 0xc3217864, 0xab427284eec91137),
    (1, 0xc0108888, 0x3bb87c5c, 0x8844738b81f01d62),
    (3, 0x0, 0xcd3d2242, 0x31d2363f52e564c9),
    (3, 0xbacd7814, 0xac9f9039, 0x57f4d4d37ec93cee),
    (3, 0xc0108888, 0x1a3e4277, 0x69eaef14886af923),
    (4, 0x0, 0x725479f2, 0x9bb64b7d66ee9fda),
    (4, 0xbacd7814, 0xb8a5dced, 0x7fb30838a41d90b8),
    (4, 0xc0108888, 0xba644e3b, 0xc802228d1d3d9e26),
    (7, 0x0, 0xf3a90eb0, 0x9a7b149959ce60d8),
    (7, 0xbacd7814, 0xe1566f18, 0x1dd28390ecae29ad),
    (7, 0xc0108888, 0xa1dc04db, 0xa862a286bd0d0554),
    (8, 0x0, 0x29b5025a, 0xdab99d95c6f90092),
    (8, 0xbacd7814, 0xc41cb7b6, 0x5fc17ad073e6bdfa),
    (8, 0xc0108888, 0xa3eb895a, 0x8c3f5604d38ec9de),
    (15, 0x0, 0x82d3c84d, 0x1b47cb8243cc8e32),
    (15, 0xbacd7814, 0xfd574948, 0xea06187cc6f08293),
    (15, 0xc0108888, 0x9dba891e, 0xef968d8f35a3be3c),
    (16, 0x0, 0x8845547d, 0x434850232b787be2),
    (16, 0xbacd7814, 0x84bbb8bd, 0x1d638d927af8ec2d),
    (16, 0xc0108888, 0xd665a4c8, 0x88573bce7c40a167),
    (17, 0x0, 0x450e5f84, 0x1efa7025f1b97a7a),
    (17, 0xbacd7814, 0x6f25bdf0, 0xce09dafe7366c3be),
    (17, 0xc0108888, 0x63984488, 0x1ead5b17b5f3b257),
    (31, 0x0, 0x1f73d3c7, 0xa2aa5f33cc4a6119),
    (31, 0xbacd7814, 0x2859de7a, 0xad8a0aeaeee7b01),
    (31, 0xc0108888, 0xbcccc62, 0x3091c4bd710617c),
    (32, 0x0, 0xe988ccb7, 0x23c3c17ef790fd97),
    (32, 0xbacd7814, 0xfcbd4aeb, 0x7451a9c2b2693a4),
    (32, 0xc0108888, 0x61271633, 0xf92c5fd5a2ca302d),
    (33, 0x0, 0x94f536d5, 0x50a7cfc7ba588784),
    (33, 0xbacd7814, 0xe4308285, 0xd8ba217f918594ca),
    (33, 0xc0108888, 0xfc5ed529, 0xcdd35de89ade743c),
    (63, 0x0, 0xbedc2f7, 0x5e3e54b431c7493c),
    (63, 0xbacd7814, 0x854c07d7, 0x365275f5126f3ea9),
    (63, 0xc0108888, 0xff57f97f, 0xb2fc2c2bfa501cce),
    (64, 0x0, 0xecd2ac44, 0xeb64b3ef6eeb01f),
    (64, 0xbacd7814, 0x5715f8e7, 0xed6fdde9e2a9e147),
    (64, 0xc0108888, 0xee975c70, 0x3cabc8fdba44ed0d),
    (100, 0x0, 0x73091a4d, 0xa61f8d4c170fe531),
    (100, 0xbacd7814, 0x477db80e, 0x81775a978aeeb381),
    (100, 0xc0108888, 0xaba25773, 0xa97488ca12b4315f),
    (1000, 0x0, 0xc85a5152, 0x5f235fa033f1a3fb),
    (1000, 0xbacd7814, 0x3c7e31d5, 0x8d2275c99623a01e),
    (1000, 0xc0108888, 0x1b6952a7, 0xd75ea8b109b1f187),
];

#[test]
fn matches_the_reference_implementation() {
    for &(len, seed, want32, want64) in REFERENCE {
        let data = input(len);
        assert_eq!(xxh32(&data, seed), want32, "xxh32 len {len} seed {seed:#x}");
        assert_eq!(
            xxh64(&data, u64::from(seed)),
            want64,
            "xxh64 len {len} seed {seed:#x}"
        );
    }
}
