//! Cubic Bezier math utilities.

use bevy::math::Vec3;

/// Evaluate a cubic Bezier curve at parameter `t` ∈ \[0, 1\].
///
/// * `p0` – start point
/// * `p1` – first control point (outgoing handle of the start node)
/// * `p2` – second control point (incoming handle of the end node)
/// * `p3` – end point
pub fn cubic_bezier(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let it = 1.0 - t;
    let it2 = it * it;
    let it3 = it2 * it;
    let t2 = t * t;
    let t3 = t2 * t;
    it3 * p0 + 3.0 * t * it2 * p1 + 3.0 * t2 * it * p2 + t3 * p3
}

/// Sample `subdivisions + 1` points along a cubic Bezier segment (including both endpoints).
pub fn sample_cubic_bezier(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    subdivisions: u32,
) -> impl Iterator<Item = Vec3> {
    let n = subdivisions.max(1);
    let inv = 1.0 / n as f32;
    (0..=n).map(move |i| cubic_bezier(p0, p1, p2, p3, i as f32 * inv))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_exact() {
        let p0 = Vec3::ZERO;
        let p1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(2.0, 0.0, 0.0);
        let p3 = Vec3::new(3.0, 0.0, 0.0);

        assert!((cubic_bezier(p0, p1, p2, p3, 0.0) - p0).length() < 1e-6);
        assert!((cubic_bezier(p0, p1, p2, p3, 1.0) - p3).length() < 1e-6);
    }

    #[test]
    fn midpoint_of_collinear_segment() {
        let p0 = Vec3::ZERO;
        let p3 = Vec3::new(6.0, 0.0, 0.0);
        let p1 = p0 + (p3 - p0) / 3.0;
        let p2 = p0 + (p3 - p0) * 2.0 / 3.0;

        let mid = cubic_bezier(p0, p1, p2, p3, 0.5);
        assert!((mid - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-5);
    }
}
