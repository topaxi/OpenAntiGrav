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
//! not its content, so the test runs in the normal suite. Raw captures live
//! under `data/scratch/pulse-camera-shake/`, gitignored.

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
        timer: 6.000000238e-01,
        magnitude: 3.000000119e-01,
        mode: 1,
        pre: [
            [6.866559386e-02, 9.425611049e-02, -9.931771159e-01],
            [1.933038607e-02, 9.952142239e-01, 9.578589350e-02],
            [9.974524379e-01, -2.577569336e-02, 6.651496887e-02],
        ],
        post: [
            [1.721699387e-01, 6.114271283e-02, -9.831677079e-01],
            [2.450571395e-02, 9.974969029e-01, 6.632522494e-02],
            [9.847623706e-01, -3.551244736e-02, 1.702407002e-01],
        ],
    },
    Frame {
        capture: "cap5 hit 14",
        timer: 5.331050158e-01,
        magnitude: 3.000000119e-01,
        mode: 1,
        pre: [
            [6.902219355e-02, 9.425374120e-02, -9.931526184e-01],
            [1.933159120e-02, 9.952135086e-01, 9.579283744e-02],
            [9.974278212e-01, -2.581105195e-02, 6.686974317e-02],
        ],
        post: [
            [1.043084934e-01, 1.192412451e-01, -9.873705506e-01],
            [1.661438495e-02, 9.924387336e-01, 1.216085106e-01],
            [9.944059849e-01, -2.908936702e-02, 1.015387326e-01],
        ],
    },
    Frame {
        capture: "cap5 hit 18",
        timer: 4.665269852e-01,
        magnitude: 3.000000119e-01,
        mode: 1,
        pre: [
            [6.937880814e-02, 9.425166249e-02, -9.931280613e-01],
            [1.933278888e-02, 9.952128530e-01, 9.580008686e-02],
            [9.974030256e-01, -2.584643103e-02, 6.722453982e-02],
        ],
        post: [
            [1.372967511e-01, 7.076619565e-02, -9.879986644e-01],
            [2.229927666e-02, 9.969710708e-01, 7.450765371e-02],
            [9.902787209e-01, -3.226131946e-02, 1.353029013e-01],
        ],
    },
    Frame {
        capture: "cap5 hit 22",
        timer: 3.997939825e-01,
        magnitude: 3.000000119e-01,
        mode: 1,
        pre: [
            [6.973640621e-02, 9.424919635e-02, -9.931032062e-01],
            [1.933399774e-02, 9.952121973e-01, 9.580699354e-02],
            [9.973781109e-01, -2.588189021e-02, 6.758029759e-02],
        ],
        post: [
            [8.901844919e-02, 1.103056371e-01, -9.899026752e-01],
            [1.785351336e-02, 9.935122728e-01, 1.123133451e-01],
            [9.958698153e-01, -2.767121606e-02, 8.647160232e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 30",
        timer: 2.663419545e-01,
        magnitude: 3.000000119e-01,
        mode: 1,
        pre: [
            [7.044960558e-02, 9.424477816e-02, -9.930532575e-01],
            [1.933637261e-02, 9.952106476e-01, 9.582130611e-02],
            [9.973278642e-01, -2.595262229e-02, 6.828986108e-02],
        ],
        post: [
            [8.684461564e-02, 1.012442112e-01, -9.910635352e-01],
            [1.868784986e-02, 9.944815636e-01, 1.032309532e-01],
            [9.960464239e-01, -2.748591453e-02, 8.447338641e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 38",
        timer: 1.328749657e-01,
        magnitude: 3.000000119e-01,
        mode: 1,
        pre: [
            [7.116533816e-02, 9.423972666e-02, -9.930027723e-01],
            [1.933868043e-02, 9.952093959e-01, 9.583508968e-02],
            [9.972770214e-01, -2.602349967e-02, 6.900194287e-02],
        ],
        post: [
            [8.166553825e-02, 9.531117231e-02, -9.920918941e-01],
            [1.922790520e-02, 9.950809479e-01, 9.718111157e-02],
            [9.964741468e-01, -2.701219916e-02, 7.943119854e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 50",
        timer: 6.000000238e-01,
        magnitude: 1.500000060e-01,
        mode: 3,
        pre: [
            [7.223559171e-02, 9.423156828e-02, -9.929261804e-01],
            [1.934197731e-02, 9.952074289e-01, 9.585518390e-02],
            [9.972001314e-01, -2.612931095e-02, 7.006676495e-02],
        ],
        post: [
            [1.951484568e-02, 8.053459972e-02, -9.965602160e-01],
            [1.963689364e-02, 9.965271950e-01, 8.091644943e-02],
            [9.996166229e-01, -2.114842460e-02, 1.786562055e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 54",
        timer: 5.332570672e-01,
        magnitude: 1.500000060e-01,
        mode: 3,
        pre: [
            [7.259324193e-02, 9.422884136e-02, -9.929003716e-01],
            [1.934313960e-02, 9.952067733e-01, 9.586193413e-02],
            [9.971740842e-01, -2.616473846e-02, 7.042258978e-02],
        ],
        post: [
            [5.541044846e-02, 1.076775789e-01, -9.926403165e-01],
            [1.862013526e-02, 9.938835502e-01, 1.088518500e-01],
            [9.982898235e-01, -2.451462671e-02, 5.306655169e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 60",
        timer: 4.331550598e-01,
        magnitude: 1.500000060e-01,
        mode: 3,
        pre: [
            [7.312788814e-02, 9.422515333e-02, -9.928615093e-01],
            [1.934493706e-02, 9.952057004e-01, 9.587243944e-02],
            [9.971349835e-01, -2.621779218e-02, 7.095450163e-02],
        ],
        post: [
            [5.822036415e-02, 9.966279566e-02, -9.933161139e-01],
            [1.904944144e-02, 9.947120547e-01, 1.009193808e-01],
            [9.981218576e-01, -2.479769103e-02, 5.601399764e-02],
        ],
    },
    Frame {
        capture: "cap5 hit 70",
        timer: 2.663010359e-01,
        magnitude: 1.500000060e-01,
        mode: 3,
        pre: [
            [7.402113825e-02, 9.421855211e-02, -9.927958846e-01],
            [1.934784837e-02, 9.952038527e-01, 9.588962048e-02],
            [9.970689416e-01, -2.630632371e-02, 7.184319943e-02],
        ],
        post: [
            [6.595097482e-02, 9.813664109e-02, -9.929848909e-01],
            [1.910230517e-02, 9.948449731e-01, 9.958919138e-02],
            [9.976398945e-01, -2.553632110e-02, 6.373639405e-02],
        ],
    },
    Frame {
        capture: "cap2 hit 0",
        timer: 6.000000238e-01,
        magnitude: 4.815574735e-03,
        mode: 3,
        pre: [
            [7.724373788e-02, 5.179183185e-02, -9.956660867e-01],
            [-9.251832962e-02, 9.947131276e-01, 4.456469789e-02],
            [9.927102923e-01, 8.867501467e-02, 8.162706345e-02],
        ],
        post: [
            [7.561513036e-02, 5.116340518e-02, -9.958232045e-01],
            [-9.248191863e-02, 9.947378039e-01, 4.408527538e-02],
            [9.928388000e-01, 8.876214921e-02, 7.994893938e-02],
        ],
    },
    Frame {
        capture: "cap2 hit 90",
        timer: 6.000000238e-01,
        magnitude: 5.498585757e-03,
        mode: 1,
        pre: [
            [9.966482520e-01, -5.859164149e-02, 5.708950385e-02],
            [5.999876559e-02, 9.979276061e-01, -2.325219102e-02],
            [-5.560880154e-02, 2.659955621e-02, 9.980983138e-01],
        ],
        post: [
            [9.965060949e-01, -5.908878148e-02, 5.902291834e-02],
            [6.054644287e-02, 9.978951812e-01, -2.321975492e-02],
            [-5.752666295e-02, 2.671226300e-02, 9.979865551e-01],
        ],
    },
    Frame {
        capture: "cap2 hit 150",
        timer: 1.996760368e-01,
        magnitude: 1.484338893e-03,
        mode: 1,
        pre: [
            [1.846905053e-02, -1.599392891e-01, 9.869540930e-01],
            [1.367720515e-01, 9.782471061e-01, 1.559688300e-01],
            [-9.904304147e-01, 1.321071386e-01, 3.994251788e-02],
        ],
        post: [
            [1.835995167e-02, -1.599438787e-01, 9.869549274e-01],
            [1.367723793e-01, 9.782438278e-01, 1.559878290e-01],
            [-9.904321432e-01, 1.321243048e-01, 3.983642533e-02],
        ],
    },
    Frame {
        capture: "cap2 hit 222",
        timer: 5.667079687e-01,
        magnitude: 9.271764196e-03,
        mode: 1,
        pre: [
            [-9.674724340e-01, 2.032994851e-02, -2.521584034e-01],
            [-3.345263377e-02, 9.777311683e-01, 2.071781904e-01],
            [2.507550418e-01, 2.088745534e-01, -9.452478290e-01],
        ],
        post: [
            [-9.669660330e-01, 2.083208971e-02, -2.540514171e-01],
            [-3.337350115e-02, 9.777291417e-01, 2.071989477e-01],
            [2.527099848e-01, 2.088330090e-01, -9.447361827e-01],
        ],
    },
    Frame {
        capture: "cap2 hit 276",
        timer: 6.000000238e-01,
        magnitude: 6.421708968e-03,
        mode: 1,
        pre: [
            [6.964916736e-02, 9.306978434e-02, -9.932205677e-01],
            [-9.334664047e-02, 9.918779135e-01, 8.639807999e-02],
            [9.931945205e-01, 8.669625223e-02, 7.777122408e-02],
        ],
        post: [
            [7.194083184e-02, 9.262743592e-02, -9.930981994e-01],
            [-9.330044687e-02, 9.919372201e-01, 8.576040715e-02],
            [9.930354357e-01, 8.648687601e-02, 8.000302315e-02],
        ],
    },
    Frame {
        capture: "cap7 hit 10",
        timer: 6.000000238e-01,
        magnitude: 2.000000030e-01,
        mode: 1,
        pre: [
            [1.118597463e-01, 9.386387467e-02, -9.892810583e-01],
            [1.945411786e-02, 9.951313734e-01, 9.661867470e-02],
            [9.935335517e-01, -3.005333245e-02, 1.094890982e-01],
        ],
        post: [
            [1.806509495e-01, 7.161240280e-02, -9.809367061e-01],
            [2.307150513e-02, 9.967627525e-01, 7.701667398e-02],
            [9.832767248e-01, -3.654482216e-02, 1.784139276e-01],
        ],
    },
    Frame {
        capture: "cap7 hit 14",
        timer: 5.334190726e-01,
        magnitude: 2.000000030e-01,
        mode: 1,
        pre: [
            [1.122173369e-01, 9.385973960e-02, -9.892409444e-01],
            [1.945498213e-02, 9.951306581e-01, 9.662549198e-02],
            [9.934931993e-01, -3.008872084e-02, 1.098448783e-01],
        ],
        post: [
            [1.356359571e-01, 1.105026752e-01, -9.845767617e-01],
            [1.708826981e-02, 9.933516979e-01, 1.138416082e-01],
            [9.906113744e-01, -3.226574883e-02, 1.328459531e-01],
        ],
    },
    Frame {
        capture: "cap7 hit 22",
        timer: 3.999581039e-01,
        magnitude: 2.000000030e-01,
        mode: 1,
        pre: [
            [1.129335985e-01, 9.385143220e-02, -9.891602397e-01],
            [1.945671439e-02, 9.951294065e-01, 9.663917124e-02],
            [9.934120774e-01, -3.015961871e-02, 1.105574816e-01],
        ],
        post: [
            [1.256861538e-01, 1.045735031e-01, -9.865428805e-01],
            [1.805380732e-02, 9.940231442e-01, 1.076664776e-01],
            [9.919055700e-01, -3.134304658e-02, 1.230470091e-01],
        ],
    },
    Frame {
        capture: "cap7 hit 30",
        timer: 2.665030658e-01,
        magnitude: 2.000000030e-01,
        mode: 1,
        pre: [
            [1.136483252e-01, 9.384422749e-02, -9.890790582e-01],
            [1.945843361e-02, 9.951278567e-01, 9.665397555e-02],
            [9.933304787e-01, -3.023049235e-02, 1.112685576e-01],
        ],
        post: [
            [1.244842187e-01, 9.853048623e-02, -9.873170853e-01],
            [1.883069798e-02, 9.946432710e-01, 1.016358659e-01],
            [9.920428395e-01, -3.124393895e-02, 1.219620407e-01],
        ],
    },
    Frame {
        capture: "cap7 hit 38",
        timer: 1.328590661e-01,
        magnitude: 2.000000030e-01,
        mode: 1,
        pre: [
            [1.143645644e-01, 9.383567423e-02, -9.889972210e-01],
            [1.946015842e-02, 9.951264262e-01, 9.666751325e-02],
            [9.932482839e-01, -3.030138090e-02, 1.119811460e-01],
        ],
        post: [
            [1.213388816e-01, 9.451767802e-02, -9.881007075e-01],
            [1.935082860e-02, 9.950416088e-01, 9.755789489e-02],
            [9.924225211e-01, -3.095813841e-02, 1.189082563e-01],
        ],
    },
    Frame {
        capture: "cap7 hit 50",
        timer: 6.000000238e-01,
        magnitude: 3.000000119e-01,
        mode: 3,
        pre: [
            [1.154389232e-01, 9.382291883e-02, -9.888736606e-01],
            [1.946272328e-02, 9.951245189e-01, 9.668800980e-02],
            [9.931239486e-01, -3.040773422e-02, 1.130500361e-01],
        ],
        post: [
            [1.012804639e-02, 6.660057604e-02, -9.977281690e-01],
            [1.977541298e-02, 9.975709319e-01, 6.679081172e-02],
            [9.997531772e-01, -2.040695213e-02, 8.786380291e-03],
        ],
    },
    Frame {
        capture: "cap7 hit 54",
        timer: 5.332770348e-01,
        magnitude: 3.000000119e-01,
        mode: 3,
        pre: [
            [1.157970354e-01, 9.381850809e-02, -9.888321757e-01],
            [1.946358755e-02, 9.951237440e-01, 9.669471532e-02],
            [9.930822253e-01, -3.044318408e-02, 1.134063303e-01],
        ],
        post: [
            [8.151801676e-02, 1.208222955e-01, -9.893211126e-01],
            [1.733719558e-02, 9.923025966e-01, 1.226149723e-01],
            [9.965210557e-01, -2.714739367e-02, 7.879584283e-02],
        ],
    },
    Frame {
        capture: "cap7 hit 62",
        timer: 3.998050094e-01,
        magnitude: 3.000000119e-01,
        mode: 3,
        pre: [
            [1.165117696e-01, 9.381109476e-02, -9.887490273e-01],
            [1.946523972e-02, 9.951224327e-01, 9.670951962e-02],
            [9.929986000e-01, -3.051403537e-02, 1.141174063e-01],
        ],
        post: [
            [9.790549427e-02, 1.109458953e-01, -9.889919758e-01],
            [1.783606410e-02, 9.934111834e-01, 1.132073253e-01],
            [9.950357676e-01, -2.872335166e-02, 9.528157860e-02],
        ],
    },
];
