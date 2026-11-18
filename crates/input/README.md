# oag-input

Maps real devices (keyboard, gamepad) onto the abstract button layer and into an
`InputSnapshot`, the only thing the simulation reads.

The dependency points into `oag-gameplay` and never back, so a replay or a test can
drive a race with no device attached. No gameplay crate may depend on it.
