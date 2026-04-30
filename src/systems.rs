//! Bevy systems for rendering Bezier spline gizmos.

use bevy::prelude::*;

use crate::{
    components::{BezierPath, BezierPathNode, GizmoDrawMode},
    math::sample_cubic_bezier,
};

/// Draws gizmos for every [`BezierPath`] in the world.
///
/// For each path the system:
/// 1. Collects ordered child [`BezierPathNode`] entities.
/// 2. Draws the spline curve (gray) by sampling cubic Bezier segments.
/// 3. Draws waypoint markers and control handles when enabled.
pub fn draw_bezier_spline_gizmos(
    mut gizmos: Gizmos,
    paths: Query<(&BezierPath, &Children)>,
    nodes: Query<(&BezierPathNode, &GlobalTransform)>,
) {
    for (path, children) in &paths {
        // Gather (center, incoming_local, outgoing_local) for each child node.
        let node_data: Vec<(Vec3, Vec3, Vec3)> = children
            .iter()
            .filter_map(|child| {
                nodes
                    .get(child)
                    .ok()
                    .map(|(node, tf)| (tf.translation(), node.incoming, node.outgoing))
            })
            .collect();

        if node_data.len() < 2 {
            continue;
        }

        let draw_spline = matches!(
            path.gizmo_draw_mode,
            GizmoDrawMode::Complete | GizmoDrawMode::SplineOnly
        );
        let draw_supports = matches!(
            path.gizmo_draw_mode,
            GizmoDrawMode::Complete | GizmoDrawMode::WaypointOnly
        );

        let segment_count = if path.closed {
            node_data.len()
        } else {
            node_data.len() - 1
        };

        // Draw spline curve.
        if draw_spline {
            for i in 0..segment_count {
                let next = (i + 1) % node_data.len();
                let (p0, _in0, out0) = node_data[i];
                let (p3, in3, _out3) = node_data[next];
                let p1 = p0 + out0;
                let p2 = p3 + in3;

                let mut prev = p0;
                for point in sample_cubic_bezier(p0, p1, p2, p3, path.subdivisions).skip(1) {
                    gizmos.line(prev, point, Color::srgb(0.5, 0.5, 0.5));
                    prev = point;
                }
            }
        }

        // Draw waypoints and control handles.
        if draw_supports {
            let last = node_data.len() - 1;

            for (i, (center, incoming, outgoing)) in node_data.iter().enumerate() {
                draw_cross(&mut gizmos, *center, 0.125, Color::srgb(1.0, 0.0, 1.0));

                let draw_incoming = i > 0 || path.closed;
                let draw_outgoing = i < last || path.closed;

                if draw_incoming && *incoming != Vec3::ZERO {
                    let ctrl = center + incoming;
                    gizmos.line(*center, ctrl, Color::srgb(0.0, 0.4, 1.0));
                    draw_cross(&mut gizmos, ctrl, 0.06, Color::srgb(0.0, 0.4, 1.0));
                }

                if draw_outgoing && *outgoing != Vec3::ZERO {
                    let ctrl = center + outgoing;
                    gizmos.line(*center, ctrl, Color::srgb(1.0, 0.2, 0.2));
                    draw_cross(&mut gizmos, ctrl, 0.06, Color::srgb(1.0, 0.2, 0.2));
                }
            }

            // Dashed waypoint-to-waypoint connections.
            for i in 0..segment_count {
                let next = (i + 1) % node_data.len();
                draw_dashed_line(
                    &mut gizmos,
                    node_data[i].0,
                    node_data[next].0,
                    0.15,
                    Color::srgba(0.5, 0.5, 0.5, 0.5),
                );
            }
        }
    }
}

/// Draw a small axis-aligned cross at `center` with half-extent `size`.
fn draw_cross(gizmos: &mut Gizmos, center: Vec3, size: f32, color: Color) {
    let h = size * 0.5;
    gizmos.line(center - Vec3::X * h, center + Vec3::X * h, color);
    gizmos.line(center - Vec3::Y * h, center + Vec3::Y * h, color);
    gizmos.line(center - Vec3::Z * h, center + Vec3::Z * h, color);
}

/// Draw a dashed line from `start` to `end` using alternating visible/invisible dashes.
fn draw_dashed_line(gizmos: &mut Gizmos, start: Vec3, end: Vec3, dash_len: f32, color: Color) {
    let dir = end - start;
    let total = dir.length();
    if total < 1e-6 {
        return;
    }
    let unit = dir / total;
    let mut t = 0.0_f32;
    let mut drawing = true;
    while t < total {
        let t_end = (t + dash_len).min(total);
        if drawing {
            gizmos.line(start + unit * t, start + unit * t_end, color);
        }
        t = t_end;
        drawing = !drawing;
    }
}
