//! Pins `oag_render::camera::shake` against the original running, frame by frame.
//!
//! Every [`Frame`] below is one call of Pulse PSP's `Camera_SubmitScene`
//! (`0x08878874`, PSP USA) read through PPSSPP v1.20.4's debugger on
//! 2026-09-30: `pre` is the camera's basis rows `+0x40/+0x50/+0x60` at the
//! function's entry, `post` the same rows at `0x08878af0`, after the shake block
//! has rotated them in place and before the copy-out, `timer`/`magnitude`/`mode`
//! the shake fields at entry (`+0xe4`, `+0x11c`, `+0x118`). Rows are
//! `(left, up, forward)` in world coordinates.
//!
//! Three captures, two boots (`cap2` and `cap5` from the first, `cap7` from a fresh
//! boot with a different heap layout):
//!
//! - `cap2`: a Time Trial on Talon's Junction held on `cross`+`left`, so the
//!   shake is the real wall-scrape one (severity `0.005`-`0.08`, magnitude
//!   `0.0009`-`0.025`), armed every frame. A small rotation, but it is the game's
//!   own arming.
//! - `cap5`: the same race stationary, with the shake fields **forced** at the
//!   entry breakpoint (magnitude `0.3`, mode 1, and magnitude `0.15`, mode 3,
//!   timer `0.6`, the envelope table `Camera_ArmShake` writes) so the whole
//!   `0.6` s decay and a full-strength rotation are measured. A forced arm
//!   measures the apply side only, which is what this file pins.
//! - `cap7`: the same forced arm on the second boot, magnitude `0.2` mode 1 and
//!   magnitude `0.3` mode 3.
//!
//! A rotation from the wrong model is many orders of magnitude off this
//! tolerance: over these frames the fit error of the model pinned here is
//! `3e-7`, a sine oscillator `1e-2`, the right-handed sense `5e-2`, and keeping
//! the pre-shake forward row as the second axis `3e-3`.
//!
//! No disc image is needed: these are measurements of the original's behaviour,
//! not its content, so the test runs in the normal suite.

use oag_core::Rng;
use oag_core::math::{Mat4, Vec3};
use oag_render::camera::shake::{DURATION_SECONDS, MAGNITUDE_SCALE, Shake, Side};

struct Frame {
    capture: &'static str,
    timer: f32,
    magnitude: f32,
    mode: u32,
    pre: [[f32; 3]; 3],
    post: [[f32; 3]; 3],
}

const TOLERANCE: f32 = 2e-6;

fn shake_of(frame: &Frame) -> Shake {
    let mut shake = Shake::new();
    let side = if frame.mode == 3 {
        Side::Ahead
    } else {
        Side::Elsewhere
    };
    shake.arm(frame.magnitude / MAGNITUDE_SCALE, side, &mut Rng::new(1));
    shake.advance(DURATION_SECONDS - frame.timer);
    shake
}

#[test]
fn the_rotation_reproduces_the_shaken_basis_the_original_wrote() {
    for frame in FRAMES {
        let shake = shake_of(frame);
        let rows = frame.pre.map(Vec3::from);
        let rotation = shake.rotation(rows[1], rows[2]);
        for (k, row) in rows.iter().enumerate() {
            let got = rotation * *row;
            let want = Vec3::from(frame.post[k]);
            let error = (got - want).abs().max_element();
            assert!(
                error < TOLERANCE,
                "{} row {k}: got {got}, original {want}, error {error}",
                frame.capture
            );
        }
    }
}

/// The same through the view matrix `Race::view` hands the renderer: a view
/// built from the original's basis, shaken with [`Shake::apply`], has the
/// original's shaken basis for its up and forward, and the eye does not move.
#[test]
fn apply_turns_a_view_the_way_the_original_turns_its_basis() {
    let eye = Vec3::new(-192.8, 41.05, 201.25);
    for frame in FRAMES {
        let shake = shake_of(frame);
        let [left, up, forward] = frame.pre.map(Vec3::from);
        // The original's rows are (left, up, forward) and this engine's view
        // rows are (right, up, back): the same two axes, differently named.
        let view = view_from(-left, up, -forward, eye);
        let shaken = shake.apply(view);
        let moved = shaken.inverse().w_axis.truncate();
        assert!(
            (moved - eye).length() < 1e-3,
            "{}: eye {moved}",
            frame.capture
        );
        let got_up = shaken.row(1).truncate();
        let got_forward = -shaken.row(2).truncate();
        let [_, want_up, want_forward] = frame.post.map(Vec3::from);
        for (name, got, want) in [
            ("up", got_up, want_up),
            ("forward", got_forward, want_forward),
        ] {
            let error = (got - want).abs().max_element();
            assert!(
                error < 5e-5,
                "{} {name}: got {got}, original {want}, error {error}",
                frame.capture
            );
        }
    }
}

fn view_from(right: Vec3, up: Vec3, back: Vec3, eye: Vec3) -> Mat4 {
    let rotation = Mat4::from_cols(
        right.extend(0.0),
        up.extend(0.0),
        back.extend(0.0),
        oag_core::math::Vec4::W,
    )
    .transpose();
    rotation * Mat4::from_translation(-eye)
}

const FRAMES: &[Frame] = &[
    Frame {
        capture: "cap5 hit 10",
        timer: 6e-01,
        magnitude: 3e-01,
        mode: 1,
        pre: [
            [6.8665594e-02, 9.425611e-02, -9.931771e-01],
            [1.9330386e-02, 9.952142e-01, 9.578589e-02],
            [9.9745244e-01, -2.5775693e-02, 6.651497e-02],
        ],
        post: [
            [1.7216994e-01, 6.1142713e-02, -9.831677e-01],
            [2.4505714e-02, 9.974969e-01, 6.6325225e-02],
            [9.847624e-01, -3.5512447e-02, 1.702407e-01],
        ],
    },
    Frame {
        capture: "cap5 hit 14",
        timer: 5.33105e-01,
        magnitude: 3e-01,
        mode: 1,
        pre: [
            [6.902219e-02, 9.425374e-02, -9.931526e-01],
            [1.9331591e-02, 9.952135e-01, 9.579284e-02],
            [9.974278e-01, -2.5811052e-02, 6.686974e-02],
        ],
        post: [
            [1.0430849e-01, 1.19241245e-01, -9.8737055e-01],
            [1.6614385e-02, 9.9243873e-01, 1.2160851e-01],
            [9.94406e-01, -2.9089367e-02, 1.0153873e-01],
        ],
    },
    Frame {
        capture: "cap5 hit 18",
        timer: 4.66527e-01,
        magnitude: 3e-01,
        mode: 1,
        pre: [
            [6.937881e-02, 9.425166e-02, -9.9312806e-01],
            [1.9332789e-02, 9.9521285e-01, 9.580009e-02],
            [9.97403e-01, -2.5846431e-02, 6.722454e-02],
        ],
        post: [
            [1.3729675e-01, 7.0766196e-02, -9.8799866e-01],
            [2.2299277e-02, 9.969711e-01, 7.450765e-02],
            [9.902787e-01, -3.226132e-02, 1.353029e-01],
        ],
    },
    Frame {
        capture: "cap5 hit 22",
        timer: 3.9979398e-01,
        magnitude: 3e-01,
        mode: 1,
        pre: [
            [6.9736406e-02, 9.42492e-02, -9.931032e-01],
            [1.9333998e-02, 9.952122e-01, 9.580699e-02],
            [9.973781e-01, -2.588189e-02, 6.75803e-02],
        ],
        post: [
            [8.901845e-02, 1.1030564e-01, -9.899027e-01],
            [1.7853513e-02, 9.935123e-01, 1.12313345e-01],
            [9.958698e-01, -2.7671216e-02, 8.64716e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 30",
        timer: 2.6634195e-01,
        magnitude: 3e-01,
        mode: 1,
        pre: [
            [7.0449606e-02, 9.424478e-02, -9.9305326e-01],
            [1.9336373e-02, 9.9521065e-01, 9.5821306e-02],
            [9.9732786e-01, -2.5952622e-02, 6.828986e-02],
        ],
        post: [
            [8.6844616e-02, 1.0124421e-01, -9.9106354e-01],
            [1.868785e-02, 9.9448156e-01, 1.0323095e-01],
            [9.960464e-01, -2.7485915e-02, 8.447339e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 38",
        timer: 1.3287497e-01,
        magnitude: 3e-01,
        mode: 1,
        pre: [
            [7.116534e-02, 9.423973e-02, -9.930028e-01],
            [1.933868e-02, 9.952094e-01, 9.583509e-02],
            [9.97277e-01, -2.60235e-02, 6.900194e-02],
        ],
        post: [
            [8.166554e-02, 9.531117e-02, -9.920919e-01],
            [1.9227905e-02, 9.9508095e-01, 9.718111e-02],
            [9.9647415e-01, -2.70122e-02, 7.94312e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 50",
        timer: 6e-01,
        magnitude: 1.5e-01,
        mode: 3,
        pre: [
            [7.223559e-02, 9.423157e-02, -9.929262e-01],
            [1.9341977e-02, 9.952074e-01, 9.5855184e-02],
            [9.9720013e-01, -2.6129311e-02, 7.0066765e-02],
        ],
        post: [
            [1.9514846e-02, 8.05346e-02, -9.965602e-01],
            [1.9636894e-02, 9.965272e-01, 8.091645e-02],
            [9.996166e-01, -2.1148425e-02, 1.786562e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 54",
        timer: 5.3325707e-01,
        magnitude: 1.5e-01,
        mode: 3,
        pre: [
            [7.259324e-02, 9.422884e-02, -9.929004e-01],
            [1.934314e-02, 9.952068e-01, 9.5861934e-02],
            [9.971741e-01, -2.6164738e-02, 7.042259e-02],
        ],
        post: [
            [5.541045e-02, 1.0767758e-01, -9.926403e-01],
            [1.8620135e-02, 9.9388355e-01, 1.0885185e-01],
            [9.982898e-01, -2.4514627e-02, 5.306655e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 60",
        timer: 4.3315506e-01,
        magnitude: 1.5e-01,
        mode: 3,
        pre: [
            [7.312789e-02, 9.422515e-02, -9.928615e-01],
            [1.9344937e-02, 9.952057e-01, 9.587244e-02],
            [9.97135e-01, -2.6217792e-02, 7.09545e-02],
        ],
        post: [
            [5.8220364e-02, 9.9662796e-02, -9.933161e-01],
            [1.9049441e-02, 9.9471205e-01, 1.0091938e-01],
            [9.9812186e-01, -2.4797691e-02, 5.6013998e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 70",
        timer: 2.6630104e-01,
        magnitude: 1.5e-01,
        mode: 3,
        pre: [
            [7.402114e-02, 9.421855e-02, -9.927959e-01],
            [1.9347848e-02, 9.9520385e-01, 9.588962e-02],
            [9.9706894e-01, -2.6306324e-02, 7.18432e-02],
        ],
        post: [
            [6.5950975e-02, 9.813664e-02, -9.929849e-01],
            [1.9102305e-02, 9.94845e-01, 9.958919e-02],
            [9.976399e-01, -2.5536321e-02, 6.3736394e-02],
        ],
    },
    Frame {
        capture: "cap2 hit 0",
        timer: 6e-01,
        magnitude: 4.8155747e-03,
        mode: 3,
        pre: [
            [7.724374e-02, 5.179183e-02, -9.956661e-01],
            [-9.251833e-02, 9.947131e-01, 4.4564698e-02],
            [9.927103e-01, 8.8675015e-02, 8.162706e-02],
        ],
        post: [
            [7.561513e-02, 5.1163405e-02, -9.958232e-01],
            [-9.248192e-02, 9.947378e-01, 4.4085275e-02],
            [9.928388e-01, 8.876215e-02, 7.994894e-02],
        ],
    },
    Frame {
        capture: "cap2 hit 90",
        timer: 6e-01,
        magnitude: 5.4985858e-03,
        mode: 1,
        pre: [
            [9.9664825e-01, -5.859164e-02, 5.7089504e-02],
            [5.9998766e-02, 9.979276e-01, -2.3252191e-02],
            [-5.56088e-02, 2.6599556e-02, 9.980983e-01],
        ],
        post: [
            [9.965061e-01, -5.908878e-02, 5.902292e-02],
            [6.0546443e-02, 9.978952e-01, -2.3219755e-02],
            [-5.7526663e-02, 2.6712263e-02, 9.9798656e-01],
        ],
    },
    Frame {
        capture: "cap2 hit 150",
        timer: 1.9967604e-01,
        magnitude: 1.4843389e-03,
        mode: 1,
        pre: [
            [1.846905e-02, -1.5993929e-01, 9.869541e-01],
            [1.3677205e-01, 9.782471e-01, 1.5596883e-01],
            [-9.904304e-01, 1.3210714e-01, 3.9942518e-02],
        ],
        post: [
            [1.8359952e-02, -1.5994388e-01, 9.869549e-01],
            [1.3677238e-01, 9.782438e-01, 1.5598783e-01],
            [-9.9043214e-01, 1.321243e-01, 3.9836425e-02],
        ],
    },
    Frame {
        capture: "cap2 hit 222",
        timer: 5.6670797e-01,
        magnitude: 9.271764e-03,
        mode: 1,
        pre: [
            [-9.6747243e-01, 2.0329949e-02, -2.521584e-01],
            [-3.3452634e-02, 9.7773117e-01, 2.0717819e-01],
            [2.5075504e-01, 2.0887455e-01, -9.452478e-01],
        ],
        post: [
            [-9.6696603e-01, 2.083209e-02, -2.5405142e-01],
            [-3.33735e-02, 9.7772914e-01, 2.0719895e-01],
            [2.5270998e-01, 2.0883301e-01, -9.447362e-01],
        ],
    },
    Frame {
        capture: "cap2 hit 276",
        timer: 6e-01,
        magnitude: 6.421709e-03,
        mode: 1,
        pre: [
            [6.964917e-02, 9.3069784e-02, -9.9322057e-01],
            [-9.334664e-02, 9.918779e-01, 8.639808e-02],
            [9.931945e-01, 8.669625e-02, 7.7771224e-02],
        ],
        post: [
            [7.194083e-02, 9.2627436e-02, -9.930982e-01],
            [-9.330045e-02, 9.919372e-01, 8.576041e-02],
            [9.9303544e-01, 8.6486876e-02, 8.000302e-02],
        ],
    },
    Frame {
        capture: "cap7 hit 10",
        timer: 6e-01,
        magnitude: 2e-01,
        mode: 1,
        pre: [
            [1.1185975e-01, 9.3863875e-02, -9.8928106e-01],
            [1.9454118e-02, 9.951314e-01, 9.6618675e-02],
            [9.9353355e-01, -3.0053332e-02, 1.094891e-01],
        ],
        post: [
            [1.8065095e-01, 7.16124e-02, -9.809367e-01],
            [2.3071505e-02, 9.9676275e-01, 7.7016674e-02],
            [9.832767e-01, -3.6544822e-02, 1.7841393e-01],
        ],
    },
    Frame {
        capture: "cap7 hit 14",
        timer: 5.334191e-01,
        magnitude: 2e-01,
        mode: 1,
        pre: [
            [1.1221734e-01, 9.385974e-02, -9.8924094e-01],
            [1.9454982e-02, 9.9513066e-01, 9.662549e-02],
            [9.934932e-01, -3.008872e-02, 1.0984488e-01],
        ],
        post: [
            [1.3563596e-01, 1.10502675e-01, -9.8457676e-01],
            [1.708827e-02, 9.933517e-01, 1.1384161e-01],
            [9.906114e-01, -3.226575e-02, 1.3284595e-01],
        ],
    },
    Frame {
        capture: "cap7 hit 22",
        timer: 3.999581e-01,
        magnitude: 2e-01,
        mode: 1,
        pre: [
            [1.129336e-01, 9.385143e-02, -9.8916024e-01],
            [1.9456714e-02, 9.951294e-01, 9.663917e-02],
            [9.934121e-01, -3.0159619e-02, 1.1055748e-01],
        ],
        post: [
            [1.2568615e-01, 1.045735e-01, -9.865429e-01],
            [1.8053807e-02, 9.9402314e-01, 1.0766648e-01],
            [9.9190557e-01, -3.1343047e-02, 1.2304701e-01],
        ],
    },
    Frame {
        capture: "cap7 hit 30",
        timer: 2.6650307e-01,
        magnitude: 2e-01,
        mode: 1,
        pre: [
            [1.13648325e-01, 9.384423e-02, -9.8907906e-01],
            [1.9458434e-02, 9.9512786e-01, 9.6653976e-02],
            [9.933305e-01, -3.0230492e-02, 1.1126856e-01],
        ],
        post: [
            [1.2448422e-01, 9.8530486e-02, -9.873171e-01],
            [1.8830698e-02, 9.946433e-01, 1.01635866e-01],
            [9.9204284e-01, -3.1243939e-02, 1.2196204e-01],
        ],
    },
    Frame {
        capture: "cap7 hit 38",
        timer: 1.3285907e-01,
        magnitude: 2e-01,
        mode: 1,
        pre: [
            [1.14364564e-01, 9.3835674e-02, -9.889972e-01],
            [1.9460158e-02, 9.951264e-01, 9.666751e-02],
            [9.932483e-01, -3.030138e-02, 1.11981146e-01],
        ],
        post: [
            [1.2133888e-01, 9.451768e-02, -9.881007e-01],
            [1.9350829e-02, 9.950416e-01, 9.7557895e-02],
            [9.924225e-01, -3.0958138e-02, 1.1890826e-01],
        ],
    },
    Frame {
        capture: "cap7 hit 50",
        timer: 6e-01,
        magnitude: 3e-01,
        mode: 3,
        pre: [
            [1.1543892e-01, 9.382292e-02, -9.8887366e-01],
            [1.9462723e-02, 9.951245e-01, 9.668801e-02],
            [9.9312395e-01, -3.0407734e-02, 1.13050036e-01],
        ],
        post: [
            [1.0128046e-02, 6.6600576e-02, -9.9772817e-01],
            [1.9775413e-02, 9.9757093e-01, 6.679081e-02],
            [9.997532e-01, -2.0406952e-02, 8.78638e-03],
        ],
    },
    Frame {
        capture: "cap7 hit 54",
        timer: 5.3327703e-01,
        magnitude: 3e-01,
        mode: 3,
        pre: [
            [1.15797035e-01, 9.381851e-02, -9.888322e-01],
            [1.9463588e-02, 9.9512374e-01, 9.6694715e-02],
            [9.930822e-01, -3.0443184e-02, 1.1340633e-01],
        ],
        post: [
            [8.151802e-02, 1.20822296e-01, -9.893211e-01],
            [1.7337196e-02, 9.923026e-01, 1.2261497e-01],
            [9.9652106e-01, -2.7147394e-02, 7.879584e-02],
        ],
    },
    Frame {
        capture: "cap7 hit 62",
        timer: 3.99805e-01,
        magnitude: 3e-01,
        mode: 3,
        pre: [
            [1.1651177e-01, 9.3811095e-02, -9.88749e-01],
            [1.946524e-02, 9.9512243e-01, 9.670952e-02],
            [9.929986e-01, -3.0514035e-02, 1.1411741e-01],
        ],
        post: [
            [9.7905494e-02, 1.10945895e-01, -9.88992e-01],
            [1.7836064e-02, 9.934112e-01, 1.13207325e-01],
            [9.9503577e-01, -2.8723352e-02, 9.528158e-02],
        ],
    },
];
