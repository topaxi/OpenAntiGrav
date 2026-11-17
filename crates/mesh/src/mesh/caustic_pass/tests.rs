use super::*;

#[test]
fn the_frame_steps_thirty_times_a_second_through_sixteen() {
    assert_eq!(frame(0.0), 0);
    assert_eq!(frame(1.0 / 30.0 + 0.001), 1);
    assert_eq!(frame(0.5), 15);
    assert_eq!(frame(15.5 / 30.0 + 0.001), 15);
    assert_eq!(frame(16.0 / 30.0 + 0.001), 0);
    assert_eq!(frame(1.0), 14);
    assert_eq!(frame(100.0), (3000 & 15) as usize);
}

#[test]
fn the_pass_names_the_odd_files() {
    assert_eq!(frame_name(0), r"Data\Tex\caustics\save.01.mip");
    assert_eq!(frame_name(15), r"Data\Tex\caustics\save.31.mip");
}

#[test]
fn the_tint_is_the_authored_triple_times_the_mesh_byte() {
    assert_eq!(colour(255), [51.0 / 255.0, 1.0, 204.0 / 255.0, 1.0]);
    assert_eq!(colour(204), [40.0 / 255.0, 204.0 / 255.0, 163.0 / 255.0, 1.0]);
    assert_eq!(colour(0), [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn a_tile_is_seventy_one_and_a_bit_units_and_height_moves_nothing() {
    let tile = 1.0 / UV_SCALE;
    let at = uv([tile, 3.0, 0.0], [0.0; 3]);
    assert!((at[0] - 1.0).abs() < 1e-6 && at[1] == 0.0);
    assert_eq!(uv([5.0, 0.0, 7.0], [0.0; 3]), uv([5.0, 90.0, 7.0], [0.0; 3]));
    assert_eq!(uv([5.0, 0.0, 7.0], [1.0, 2.0, 3.0]), uv([4.0, 9.0, 4.0], [0.0; 3]));
    assert_eq!(UV_SCALE, 0.014);
}
