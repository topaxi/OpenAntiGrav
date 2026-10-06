//! Moving the racing line sideways toward chosen points, inside its corridor.
//!
//! **Ours, chosen, not measured.** The authored line is the artists' line; a
//! driver that wants something the line does not pass over - a speed pad a jump
//! needs - has to leave it, and doing that once, at load, by moving the line,
//! keeps every consumer (the steering, the speed plan, the progress index)
//! reading one line rather than a driver steering away from the line it
//! measures itself against.

use oag_core::math::Vec3;

use super::Frame;

/// A lateral target: reach `offset` (signed, positive to the driver's right, as
/// [`Frame::lateral`] points) across the line at sample `index`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub index: usize,
    /// How far across, before the corridor clamps it.
    pub offset: f32,
}

/// Moves `points` toward each target in `targets` and rebases `corridor` onto
/// the moved line.
///
/// The move is full within `hold` units of travel either side of a target's
/// sample and eases to nothing over `ease` units beyond that, on a smoothstep,
/// so the line's heading changes gently. Where two targets' reaches overlap the
/// larger move wins. A target is clamped to `margin` inside the corridor, and
/// a corridor of zero width at the target leaves the line where it is.
///
/// `points` and `corridor` must be the same length; a mismatch moves nothing.
pub fn toward(
    points: &mut [Vec3],
    corridor: &mut [Frame],
    targets: &[Target],
    hold: f32,
    ease: f32,
    margin: f32,
) {
    let n = points.len();
    if n < 3 || corridor.len() != n || targets.is_empty() {
        return;
    }
    let spacing: Vec<f32> = (0..n)
        .map(|i| points[i].distance(points[(i + 1) % n]))
        .collect();
    let mut shift = vec![0.0f32; n];
    for target in targets {
        let at = target.index % n;
        let frame = corridor[at];
        let offset = target.offset.clamp(
            (frame.left + margin).min(0.0),
            (frame.right - margin).max(0.0),
        );
        if offset == 0.0 {
            continue;
        }
        let reach = hold + ease;
        // Forward from the target, then backward, each until the reach runs out.
        for forward in [true, false] {
            let mut travelled = 0.0f32;
            let mut i = at;
            for _ in 0..n {
                let weight = ease_weight(travelled, hold, ease);
                let moved = offset * weight;
                if moved.abs() > shift[i].abs() {
                    shift[i] = moved;
                }
                let next = if forward {
                    (i + 1) % n
                } else {
                    (i + n - 1) % n
                };
                travelled += if forward { spacing[i] } else { spacing[next] };
                if travelled > reach || next == at {
                    break;
                }
                i = next;
            }
        }
    }
    for i in 0..n {
        let s = shift[i];
        if s == 0.0 {
            continue;
        }
        let frame = corridor[i];
        let s = s.clamp(frame.left.min(0.0), frame.right.max(0.0));
        points[i] += frame.lateral * s;
        corridor[i] = Frame {
            lateral: frame.lateral,
            left: (frame.left - s).min(0.0),
            right: (frame.right - s).max(0.0),
        };
    }
}

/// One inside `hold`, easing to zero across the next `ease`, smoothstep.
fn ease_weight(distance: f32, hold: f32, ease: f32) -> f32 {
    if distance <= hold {
        return 1.0;
    }
    if ease <= 0.0 || distance >= hold + ease {
        return 0.0;
    }
    let t = 1.0 - (distance - hold) / ease;
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests;
