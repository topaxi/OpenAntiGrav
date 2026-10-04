use super::*;

fn put32(out: &mut [u8], at: usize, v: u32) {
    out[at..at + 4].copy_from_slice(&v.to_be_bytes());
}

fn put16(out: &mut [u8], at: usize, v: u16) {
    out[at..at + 2].copy_from_slice(&v.to_be_bytes());
}

/// One channel with `triggers` triggers and one layer on it.
fn table(triggers: usize) -> Vec<u8> {
    let trigger_at = HEADER_LEN + CHANNEL_LEN;
    let layer_at = trigger_at + triggers * TRIGGER_LEN;
    let table_at = layer_at + LAYER_LEN;
    let mut out = vec![0u8; table_at + 4];
    out[..4].copy_from_slice(&MAGIC);
    put32(&mut out, 4, VERSION_WORD);
    put32(&mut out, 0x0c, 1);
    put32(&mut out, 0x10, 1);
    put32(&mut out, 0x14, HEADER_LEN as u32);
    put32(&mut out, 0x18, table_at as u32);

    let ch = HEADER_LEN;
    put16(&mut out, ch, 100);
    put32(&mut out, ch + 0x08, 0x3333);
    put32(&mut out, ch + 0x18, 0x1_0000);
    put32(&mut out, ch + 0x50, 0x1_0000);
    put16(&mut out, ch + 0x58, triggers as u16);
    put32(&mut out, ch + 0x5c, trigger_at as u32);

    out[layer_at + 1..layer_at + 6].copy_from_slice(b"~jet1");
    out[layer_at + 0x14] = 0;
    put32(&mut out, layer_at + 0x1c, (layer_at + 0x30) as u32);
    put32(&mut out, layer_at + 0x20, (layer_at + 0x430) as u32);
    for x in 0..CURVE_LEN {
        put16(&mut out, layer_at + 0x30 + x * 2, (x * 2) as u16);
        put16(&mut out, layer_at + 0x430 + x * 2, (1000 - x) as u16);
    }
    put32(&mut out, table_at, layer_at as u32);
    out
}

#[test]
fn a_well_formed_table_accounts_for_every_byte() {
    let data = table(2);
    let xfx = Xfx::parse(&data).unwrap();
    assert_eq!(xfx.channels().len(), 1);
    assert_eq!(xfx.layers().len(), 1);
    assert_eq!(xfx.channels()[0].trigger_count(), 2);
    assert_eq!(xfx.accounted_bytes(), data.len());
}

#[test]
fn a_layer_reads_its_name_channel_and_both_curves() {
    let data = table(0);
    let xfx = Xfx::parse(&data).unwrap();
    let layer = &xfx.layers()[0];
    assert_eq!(layer.name(), "~jet1");
    assert_eq!(layer.channel(), 0);
    assert_eq!(layer.gain_at(3), 6);
    assert_eq!(layer.pitch_at(3), 997);
    assert_eq!(layer.gain().count(), CURVE_LEN);
    assert_eq!(layer.pitch().last(), Some(489));
}

#[test]
fn an_index_past_the_curve_clamps_to_its_last_entry() {
    let data = table(0);
    let layer = Xfx::parse(&data).unwrap().layers()[0];
    assert_eq!(layer.gain_at(10_000), layer.gain_at(CURVE_LEN - 1));
}

#[test]
fn a_channel_exposes_its_bands_and_rates() {
    let data = table(0);
    let xfx = Xfx::parse(&data).unwrap();
    let channel = &xfx.channels()[0];
    assert_eq!(channel.band_edges(), [100, 0, 0, 0]);
    assert_eq!(channel.rise_rates(), [0x3333, 0, 0, 0]);
    assert_eq!(channel.fall_rates(), [0x1_0000, 0, 0, 0]);
    assert_eq!(channel.input_scale(), 0x1_0000);
}

#[test]
fn the_loader_s_header_checks_each_refuse() {
    let mut data = table(0);
    data[0] = b'Y';
    assert_eq!(Xfx::parse(&data).unwrap_err(), Error::NotXfdx);

    let mut data = table(0);
    put32(&mut data, 4, 0x0106_0000);
    assert!(matches!(
        Xfx::parse(&data),
        Err(Error::UnsupportedVersion { .. })
    ));

    let mut data = table(0);
    put32(&mut data, 8, 1);
    assert_eq!(Xfx::parse(&data).unwrap_err(), Error::AlreadyRelocated);
}

#[test]
fn an_unread_layer_type_or_a_missing_channel_is_refused() {
    let layer_at = HEADER_LEN + CHANNEL_LEN;
    let mut data = table(0);
    data[layer_at + 0x17] = 1;
    assert!(matches!(
        Xfx::parse(&data),
        Err(Error::UnsupportedLayerType { layer: 0, kind: 1 })
    ));

    let mut data = table(0);
    data[layer_at + 0x14] = 4;
    assert!(matches!(
        Xfx::parse(&data),
        Err(Error::ChannelOutOfRange { channel: 4, .. })
    ));
}

#[test]
fn a_truncated_file_is_an_error_not_a_panic() {
    let data = table(1);
    for cut in [0, 3, HEADER_LEN, HEADER_LEN + 10, data.len() - 1] {
        assert!(Xfx::parse(&data[..cut]).is_err(), "cut at {cut}");
    }
}
